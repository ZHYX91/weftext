use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, TempDir};
use uuid::Uuid;
use weftext_agent::{
    AgentAuditConfig, AgentBrokerConfig, AgentCapability, AgentOrigin, CancellationMode,
    CapabilityGrant, HarnessHandshake,
};
use weftext_agent_dsh::{DshClient, DshCompatibilityPolicy, DshInitialize};
use weftext_agent_mcp::{
    SupervisedMcpAgentEndpoint, SupervisedMcpAuthority, SupervisedMcpController,
    SupervisedMcpSession, SupervisedRuntimeContract, SupervisedSessionConfig,
};
use weftext_core::{scan_workspace, NodeId, WorkspaceNodeProjection, WorkspaceReadScope};

const CONTROL_DIRECTORY: &str = "agent-control-v1";
const SESSIONS_DIRECTORY: &str = "sessions";
const AUDIT_DIRECTORY: &str = "audit";
const SESSION_DESCRIPTOR: &str = "session.json";
const SESSION_REVOCATION: &str = "revoked.json";
const SESSION_REVOCATION_PENDING: &str = "revoked.pending";
const RUNTIME_CONFIG_FILE: &str = "agent-dsh-runtime-v1.json";
const RUNTIME_CONFIG_SCHEMA: &str = "weftext.desktop.dsh-runtime-config.v1";
const SESSION_DESCRIPTOR_SCHEMA: &str = "weftext.desktop.agent-session.v1";
const SESSION_REVOCATION_SCHEMA: &str = "weftext.desktop.agent-session-revocation.v1";
const CAPABILITY_SCHEMA: &str = "weftext.desktop.supervised-agent-capability.v1";
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_DESCRIPTOR_BYTES: u64 = 64 * 1024;
const MAX_REVOCATION_BYTES: u64 = 16 * 1024;
const MAX_SCOPED_NODES: usize = 256;
const MAX_RUNTIME_ARGUMENTS: usize = 64;
const MAX_RUNTIME_ARGUMENT_BYTES: usize = 4 * 1024;
const MAX_RUNTIME_ID_BYTES: usize = 256;
const MIN_RUNTIME_TIMEOUT_MILLIS: u64 = 100;
const MAX_RUNTIME_TIMEOUT_MILLIS: u64 = 30_000;
const MAX_EVENT_POLL_MILLIS: u64 = 250;
const LOCAL_HUMAN_ACTOR_ID: &str = "desktop-local-human";
const DESKTOP_AGENT_TOOL_ALLOWLIST: &[&str] =
    &["workspace_inventory", "read_document", "mutation_status"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct StartSessionRequest {
    scope_node_ids: Vec<NodeId>,
    delegated_capabilities: Vec<AgentCapability>,
    probe_dsh_runtime: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RecoverSessionRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CapabilityUpdateRequest {
    delegated_capabilities: Vec<AgentCapability>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct EventRequest {
    cursor: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RuntimePollRequest {
    timeout_millis: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DshRuntimeConfiguration {
    schema: String,
    executable: PathBuf,
    executable_sha256: String,
    #[serde(default)]
    arguments: Vec<String>,
    provider: String,
    model: String,
    max_tokens: Option<u64>,
    request_timeout_millis: u64,
}

#[derive(Clone, Debug)]
struct VerifiedDshRuntime {
    configuration: DshRuntimeConfiguration,
    executable: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AgentSessionDescriptor {
    schema: String,
    session_id: String,
    workspace_key: String,
    human_actor_id: String,
    delegated_client_id: String,
    workspace_scope_id: String,
    scope_node_ids: Vec<NodeId>,
    actor_capabilities: Vec<AgentCapability>,
    delegated_capability_ceiling: Vec<AgentCapability>,
    workspace_policy_capabilities: Vec<AgentCapability>,
    harness: String,
    adapter_version: String,
    runtime_wire_version: String,
    resume_supported: bool,
    cancellation: CancellationMode,
    runtime_was_handshaken: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AgentSessionRevocation {
    schema: String,
    session_id: String,
    workspace_key: String,
    workspace_scope_id: String,
    delegated_capability_ceiling: Vec<AgentCapability>,
    descriptor_sha256: String,
}

impl AgentSessionRevocation {
    fn for_descriptor(descriptor: &AgentSessionDescriptor) -> Result<Self, String> {
        Ok(Self {
            schema: SESSION_REVOCATION_SCHEMA.to_owned(),
            session_id: descriptor.session_id.clone(),
            workspace_key: descriptor.workspace_key.clone(),
            workspace_scope_id: descriptor.workspace_scope_id.clone(),
            delegated_capability_ceiling: descriptor.delegated_capability_ceiling.clone(),
            descriptor_sha256: descriptor_sha256(descriptor)?,
        })
    }

    fn validate(&self, descriptor: &AgentSessionDescriptor) -> Result<(), String> {
        let expected = Self::for_descriptor(descriptor)?;
        if self.schema != expected.schema
            || self.session_id != expected.session_id
            || self.workspace_key != expected.workspace_key
            || self.workspace_scope_id != expected.workspace_scope_id
            || self.delegated_capability_ceiling != expected.delegated_capability_ceiling
            || self.descriptor_sha256 != expected.descriptor_sha256
        {
            return Err(
                "agent session revocation marker is invalid or belongs elsewhere".to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentSessionBinding<'a> {
    schema: &'a str,
    session_id: &'a str,
    workspace_key: &'a str,
    human_actor_id: &'a str,
    delegated_client_id: &'a str,
    scope_node_ids: &'a [NodeId],
    actor_capabilities: &'a [AgentCapability],
    delegated_capability_ceiling: &'a [AgentCapability],
    workspace_policy_capabilities: &'a [AgentCapability],
    harness: &'a str,
    adapter_version: &'a str,
    runtime_wire_version: &'a str,
    resume_supported: bool,
    cancellation: CancellationMode,
    runtime_was_handshaken: bool,
}

impl AgentSessionDescriptor {
    fn binding(&self) -> AgentSessionBinding<'_> {
        AgentSessionBinding {
            schema: &self.schema,
            session_id: &self.session_id,
            workspace_key: &self.workspace_key,
            human_actor_id: &self.human_actor_id,
            delegated_client_id: &self.delegated_client_id,
            scope_node_ids: &self.scope_node_ids,
            actor_capabilities: &self.actor_capabilities,
            delegated_capability_ceiling: &self.delegated_capability_ceiling,
            workspace_policy_capabilities: &self.workspace_policy_capabilities,
            harness: &self.harness,
            adapter_version: &self.adapter_version,
            runtime_wire_version: &self.runtime_wire_version,
            resume_supported: self.resume_supported,
            cancellation: self.cancellation,
            runtime_was_handshaken: self.runtime_was_handshaken,
        }
    }

    fn expected_scope_id(&self) -> Result<String, String> {
        let bytes = serde_json::to_vec(&self.binding())
            .map_err(|_| "agent session binding could not be encoded".to_owned())?;
        Ok(format!("desktop-scope-v1:{:x}", Sha256::digest(bytes)))
    }

    fn validate(&self, workspace_root: &Path) -> Result<(), String> {
        let runtime_contract_is_valid = if self.runtime_was_handshaken {
            self.harness == "dsh"
                && self.runtime_wire_version == "0.0.1"
                && !self.resume_supported
                && self.cancellation == CancellationMode::RuntimeTermination
        } else {
            self.harness == "desktop-private-mcp"
                && self.runtime_wire_version == "none"
                && !self.resume_supported
                && self.cancellation == CancellationMode::Unsupported
        };
        if self.schema != SESSION_DESCRIPTOR_SCHEMA
            || !is_exact_lowercase_uuid(&self.session_id)
            || self.workspace_key != workspace_key(workspace_root)?
            || self.human_actor_id != LOCAL_HUMAN_ACTOR_ID
            || self.delegated_client_id != format!("desktop-agent:{}", self.session_id)
            || self.workspace_scope_id != self.expected_scope_id()?
            || !runtime_contract_is_valid
        {
            return Err("agent session descriptor is invalid or belongs elsewhere".to_owned());
        }
        validate_scope_node_ids(&self.scope_node_ids)?;
        let actor = validate_capabilities(&self.actor_capabilities)?;
        let ceiling = validate_capabilities(&self.delegated_capability_ceiling)?;
        let policy = validate_capabilities(&self.workspace_policy_capabilities)?;
        if !ceiling.is_subset(&actor) || !ceiling.is_subset(&policy) {
            return Err("agent session capability authority is invalid".to_owned());
        }
        Ok(())
    }

    fn authority(&self, delegated: CapabilityGrant) -> SupervisedMcpAuthority {
        SupervisedMcpAuthority {
            human_actor_id: self.human_actor_id.clone(),
            delegated_client_id: self.delegated_client_id.clone(),
            workspace_scope_id: self.workspace_scope_id.clone(),
            origin: AgentOrigin {
                harness: self.harness.clone(),
                adapter_version: self.adapter_version.clone(),
                session_id: self.session_id.clone(),
            },
            actor_capabilities: grant(&self.actor_capabilities),
            delegated_session_capabilities: delegated,
            workspace_policy_capabilities: grant(&self.workspace_policy_capabilities),
        }
    }
}

struct ActiveAgentSession {
    descriptor: AgentSessionDescriptor,
    session_directory: PathBuf,
    #[allow(dead_code)] // Reserved behind the filtered private transport seam.
    endpoint: DesktopPrivateAgentEndpoint,
    controller: SupervisedMcpController,
    delegated_capabilities: Vec<AgentCapability>,
    runtime: Option<DshClient>,
    runtime_working_directory: Option<TempDir>,
    handshake: Option<HarnessHandshake>,
    recovered: bool,
    revoked: bool,
}

struct DesktopPrivateAgentEndpoint {
    inner: SupervisedMcpAgentEndpoint,
}

impl DesktopPrivateAgentEndpoint {
    // The packaged runtime attachment remains unavailable, but this is the only transport seam it
    // may receive. Its positive allowlist keeps every controller operation out of discovery and
    // direct invocation even if the shared MCP catalog grows later.
    #[allow(dead_code)]
    fn serve(&self, input: impl BufRead, mut output: impl Write) -> Result<(), String> {
        for line in input.lines() {
            let line = line.map_err(|_| "agent endpoint input failed".to_owned())?;
            if line.trim().is_empty() {
                continue;
            }
            let request: Value = serde_json::from_str(&line)
                .map_err(|_| "agent endpoint received invalid JSON".to_owned())?;
            if request["method"] == "tools/call"
                && !request["params"]["name"]
                    .as_str()
                    .is_some_and(|name| DESKTOP_AGENT_TOOL_ALLOWLIST.contains(&name))
            {
                serde_json::to_writer(
                    &mut output,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": request.get("id").cloned().unwrap_or(Value::Null),
                        "error": {"code": -32601, "message": "tool is not available"},
                    }),
                )
                .map_err(|_| "agent endpoint output failed".to_owned())?;
                output
                    .write_all(b"\n")
                    .and_then(|()| output.flush())
                    .map_err(|_| "agent endpoint output failed".to_owned())?;
                continue;
            }
            let mut inner_output = Vec::new();
            self.inner.serve(
                BufReader::new(Cursor::new(format!("{line}\n").into_bytes())),
                &mut inner_output,
            )?;
            if inner_output.is_empty() {
                continue;
            }
            let mut response: Value = serde_json::from_slice(&inner_output)
                .map_err(|_| "agent endpoint returned invalid JSON".to_owned())?;
            if request["method"] == "tools/list" {
                if let Some(tools) = response["result"]["tools"].as_array_mut() {
                    tools.retain(|tool| {
                        tool["name"]
                            .as_str()
                            .is_some_and(|name| DESKTOP_AGENT_TOOL_ALLOWLIST.contains(&name))
                    });
                }
            }
            serde_json::to_writer(&mut output, &response)
                .map_err(|_| "agent endpoint output failed".to_owned())?;
            output
                .write_all(b"\n")
                .and_then(|()| output.flush())
                .map_err(|_| "agent endpoint output failed".to_owned())?;
        }
        Ok(())
    }
}

pub(crate) struct DesktopAgentLifecycle {
    config_dir: PathBuf,
    active: Option<ActiveAgentSession>,
}

impl DesktopAgentLifecycle {
    pub(crate) fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            active: None,
        }
    }

    pub(crate) fn close_for_workspace_change(&mut self) -> Result<(), String> {
        if self.active.is_some() {
            self.revoke_active()?;
            self.active = None;
        }
        Ok(())
    }

    pub(crate) fn capability(&self, workspace_root: &Path) -> Value {
        let runtime_status = runtime_status(&self.config_dir, workspace_root);
        let active_workspace_key = workspace_key(workspace_root).ok();
        let active = self.active.as_ref().filter(|session| {
            active_workspace_key.as_ref() == Some(&session.descriptor.workspace_key)
        });
        let (configured, caller_digest_verified, probe_available, reason_code) =
            match runtime_status {
                RuntimeStatus::Missing => (false, false, false, "dsh_runtime_not_configured"),
                RuntimeStatus::Invalid(code) => (true, false, false, code),
                RuntimeStatus::Verified => (true, true, true, "runtime_probe_only"),
            };
        json!({
            "schema": CAPABILITY_SCHEMA,
            "sessionLifecycleAvailable": true,
            "agentExecutionAvailable": false,
            "trustedControllerOnly": true,
            "rawWorkspaceAccessGranted": false,
            "osSandboxEnforced": false,
            "approvalAndCommitAgentCallable": false,
            "runtimeMcpAttachmentAvailable": false,
            "blockerCodes": [
                reason_code,
                "dsh_runtime_not_packaged_by_weftext",
                "runtime_mcp_attachment_unavailable",
                "os_sandbox_not_enforced"
            ],
            "dsh": {
                "configured": configured,
                "callerPackageDigestVerified": caller_digest_verified,
                "probeAvailable": probe_available,
                "wireVersion": "0.0.1",
                "resumeSupported": false,
                "cancellation": "runtime_termination",
                "active": active.is_some_and(|session| session.runtime.is_some()),
            },
            "activeSession": active.map(session_summary),
        })
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn request(
        &mut self,
        route: &str,
        body: Option<Value>,
        workspace_root: &Path,
    ) -> Result<Value, String> {
        if route_requires_active_workspace(route) {
            self.require_active_workspace(workspace_root)?;
        }
        match route {
            "/api/agent/capability" => {
                require_no_body(body.as_ref(), "agent capability")?;
                Ok(json!({"ok": true, "agent": self.capability(workspace_root)}))
            }
            "/api/agent/session/start" => {
                let request = parse_body::<StartSessionRequest>(body, "agent session start")?;
                self.start_session(workspace_root, request)
            }
            "/api/agent/session/recover" => {
                let request = parse_body::<RecoverSessionRequest>(body, "agent session recovery")?;
                self.recover_session(workspace_root, &request.session_id)
            }
            "/api/agent/session/grants" => {
                let request =
                    parse_body::<CapabilityUpdateRequest>(body, "agent capability update")?;
                self.update_grants(request.delegated_capabilities)
            }
            "/api/agent/session/revoke" => {
                require_no_body(body.as_ref(), "agent revocation")?;
                self.revoke_active()?;
                Ok(json!({
                    "ok": true,
                    "session": self.active.as_ref().map(session_summary),
                }))
            }
            "/api/agent/session/events" => {
                let request = parse_body::<EventRequest>(body, "agent events")?;
                let events = self
                    .active_controller()?
                    .events_after(request.cursor)
                    .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "events": events}))
            }
            "/api/agent/session/recovery" => {
                require_no_body(body.as_ref(), "agent recovery state")?;
                let recovery = self
                    .active_controller()?
                    .recovery_states()
                    .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "recovery": recovery}))
            }
            "/api/agent/runtime/poll" => {
                let request = parse_body::<RuntimePollRequest>(body, "agent runtime poll")?;
                self.poll_runtime(request.timeout_millis)
            }
            "/api/agent/runtime/cancel" => {
                require_no_body(body.as_ref(), "agent runtime cancellation")?;
                self.cancel_runtime()
            }
            _ => Err(format!("未知桌面 agent 路径：{route}")),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn start_session(
        &mut self,
        workspace_root: &Path,
        request: StartSessionRequest,
    ) -> Result<Value, String> {
        self.clear_terminal_active()?;
        if self.active.is_some() {
            return Err("an agent session is already active; revoke it first".to_owned());
        }
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(|_| "agent workspace is unavailable".to_owned())?;
        validate_scope_node_ids(&request.scope_node_ids)?;
        let requested = validate_capabilities(&request.delegated_capabilities)?;
        if request.probe_dsh_runtime && !requested.contains(&AgentCapability::ExternalEgress) {
            return Err(
                "DSH runtime probing requires an explicit external_egress grant".to_owned(),
            );
        }
        if !request.probe_dsh_runtime && requested.contains(&AgentCapability::ExternalEgress) {
            return Err("external_egress is unavailable without an explicit DSH probe".to_owned());
        }
        let scope = build_scope(&workspace_root, &request.scope_node_ids)?;
        let control_root = prepare_control_root(&self.config_dir, &workspace_root)?;
        let session_id = new_session_id(&control_root)?;

        let mut runtime = None;
        let mut runtime_working_directory = None;
        let mut handshake = None;
        if request.probe_dsh_runtime {
            let verified = load_verified_runtime(&self.config_dir, &workspace_root)?;
            let working_directory = tempfile::Builder::new()
                .prefix("runtime-probe-")
                .tempdir_in(&control_root)
                .map_err(|_| "could not create private DSH runtime state".to_owned())?;
            set_private_directory_permissions(working_directory.path())?;
            let mut command = Command::new(&verified.executable);
            command
                .args(&verified.configuration.arguments)
                .current_dir(working_directory.path());
            restrict_runtime_environment(&mut command, working_directory.path(), &workspace_root);
            let timeout = Duration::from_millis(verified.configuration.request_timeout_millis);
            let mut client = DshClient::spawn(command, timeout)
                .map_err(|_| "configured DSH runtime could not be launched".to_owned())?;
            let observed = client
                .initialize(
                    &DshInitialize {
                        cwd: working_directory.path().to_path_buf(),
                        provider: verified.configuration.provider.clone(),
                        model: verified.configuration.model.clone(),
                        max_tokens: verified.configuration.max_tokens,
                    },
                    &DshCompatibilityPolicy::default(),
                )
                .map_err(|error| format!("configured DSH runtime is incompatible: {error}"))?;
            runtime = Some(client);
            runtime_working_directory = Some(working_directory);
            handshake = Some(observed);
        }

        let runtime_identity = handshake.as_ref();
        let mut actor_capabilities = base_host_capabilities();
        if requested.contains(&AgentCapability::ExternalEgress) {
            actor_capabilities.push(AgentCapability::ExternalEgress);
        }
        sort_capabilities(&mut actor_capabilities);
        let mut delegated_capabilities = request.delegated_capabilities;
        sort_capabilities(&mut delegated_capabilities);
        let mut descriptor = AgentSessionDescriptor {
            schema: SESSION_DESCRIPTOR_SCHEMA.to_owned(),
            session_id: session_id.clone(),
            workspace_key: workspace_key(&workspace_root)?,
            human_actor_id: LOCAL_HUMAN_ACTOR_ID.to_owned(),
            delegated_client_id: format!("desktop-agent:{session_id}"),
            workspace_scope_id: String::new(),
            scope_node_ids: sorted_node_ids(request.scope_node_ids),
            actor_capabilities: actor_capabilities.clone(),
            delegated_capability_ceiling: delegated_capabilities.clone(),
            workspace_policy_capabilities: actor_capabilities,
            harness: runtime_identity.map_or_else(
                || "desktop-private-mcp".to_owned(),
                |value| value.harness.clone(),
            ),
            adapter_version: runtime_identity.map_or_else(
                || env!("CARGO_PKG_VERSION").to_owned(),
                |value| value.adapter_version.clone(),
            ),
            runtime_wire_version: if handshake.is_some() { "0.0.1" } else { "none" }.to_owned(),
            resume_supported: false,
            cancellation: if handshake.is_some() {
                CancellationMode::RuntimeTermination
            } else {
                CancellationMode::Unsupported
            },
            runtime_was_handshaken: handshake.is_some(),
        };
        descriptor.workspace_scope_id = descriptor.expected_scope_id()?;
        descriptor.validate(&workspace_root)?;

        let session_directory = create_session_directory(&control_root, &session_id)?;
        write_descriptor(&session_directory, &descriptor)?;
        let audit_root = checked_directory(&session_directory, AUDIT_DIRECTORY)?;
        let session = SupervisedMcpSession::open(
            &workspace_root,
            scope,
            descriptor.authority(grant(&delegated_capabilities)),
            audit_root,
            session_config(&descriptor),
        )
        .map_err(|error| error.to_string())?;
        let (endpoint, controller) = session.into_parts();
        self.active = Some(ActiveAgentSession {
            descriptor,
            session_directory,
            endpoint: DesktopPrivateAgentEndpoint { inner: endpoint },
            controller,
            delegated_capabilities,
            runtime,
            runtime_working_directory,
            handshake,
            recovered: false,
            revoked: false,
        });
        Ok(json!({
            "ok": true,
            "session": self.active.as_ref().map(session_summary),
            "agent": self.capability(&workspace_root),
        }))
    }

    fn recover_session(
        &mut self,
        workspace_root: &Path,
        session_id: &str,
    ) -> Result<Value, String> {
        self.clear_terminal_active()?;
        if self.active.is_some() {
            return Err("an agent session is already active; revoke it first".to_owned());
        }
        if !is_exact_lowercase_uuid(session_id) {
            return Err("agent session identity is invalid".to_owned());
        }
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(|_| "agent workspace is unavailable".to_owned())?;
        let control_root = prepare_control_root(&self.config_dir, &workspace_root)?;
        let session_directory = existing_session_directory(&control_root, session_id)?;
        let descriptor = read_descriptor(&session_directory)?;
        descriptor.validate(&workspace_root)?;
        if descriptor.session_id != session_id {
            return Err("agent session descriptor identity does not match".to_owned());
        }
        refuse_revoked_session(&session_directory, &descriptor)?;
        let scope = build_scope(&workspace_root, &descriptor.scope_node_ids)?;
        let audit_root = existing_checked_directory(&session_directory, AUDIT_DIRECTORY)?;
        let _records = existing_checked_directory(&audit_root, "records")?;
        let _seals = existing_checked_directory(&audit_root, "seals")?;
        let session = SupervisedMcpSession::open(
            &workspace_root,
            scope,
            descriptor.authority(CapabilityGrant::default()),
            audit_root,
            session_config(&descriptor),
        )
        .map_err(|error| error.to_string())?;
        let (endpoint, controller) = session.into_parts();
        self.active = Some(ActiveAgentSession {
            descriptor,
            session_directory,
            endpoint: DesktopPrivateAgentEndpoint { inner: endpoint },
            controller,
            delegated_capabilities: Vec::new(),
            runtime: None,
            runtime_working_directory: None,
            handshake: None,
            recovered: true,
            revoked: false,
        });
        let recovery = self
            .active_controller()?
            .recovery_states()
            .map_err(|error| error.to_string())?;
        Ok(json!({
            "ok": true,
            "session": self.active.as_ref().map(session_summary),
            "recovery": recovery,
            "requiresExplicitRegrant": true,
            "runtimeResumed": false,
        }))
    }

    fn update_grants(&mut self, mut capabilities: Vec<AgentCapability>) -> Result<Value, String> {
        let requested = validate_capabilities(&capabilities)?;
        let active = self.active.as_mut().ok_or("no active agent session")?;
        if active.revoked {
            return Err("agent session has been revoked".to_owned());
        }
        let ceiling = active
            .descriptor
            .delegated_capability_ceiling
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if !requested.is_subset(&ceiling) {
            return Err("delegated capabilities exceed the approved session ceiling".to_owned());
        }
        if requested.contains(&AgentCapability::ExternalEgress) && active.runtime.is_none() {
            return Err(
                "external_egress requires an active digest-verified DSH runtime".to_owned(),
            );
        }
        let runtime_termination =
            if active.runtime.is_some() && !requested.contains(&AgentCapability::ExternalEgress) {
                let mut runtime = active
                    .runtime
                    .take()
                    .ok_or("no compatible DSH runtime is active")?;
                let termination = runtime
                    .terminate_for_cancellation_with(&active.controller)
                    .map_err(|_| "DSH runtime egress revocation failed".to_owned());
                active.runtime_working_directory = None;
                termination
            } else {
                Ok(())
            };
        sort_capabilities(&mut capabilities);
        let grant_update = apply_delegated_grants(active, capabilities);
        runtime_termination?;
        grant_update?;
        Ok(json!({"ok": true, "session": session_summary(active)}))
    }

    fn revoke_active(&mut self) -> Result<(), String> {
        let Some(active) = self.active.as_mut() else {
            return Err("no active agent session".to_owned());
        };
        if active.revoked {
            return require_revocation_marker(&active.session_directory, &active.descriptor);
        }
        let persistence = write_revocation_marker(&active.session_directory, &active.descriptor);
        active.delegated_capabilities.clear();
        active.revoked = true;
        let runtime_termination = if let Some(mut runtime) = active.runtime.take() {
            runtime
                .terminate_for_cancellation_with(&active.controller)
                .map_err(|_| "DSH runtime cancellation failed".to_owned())
        } else {
            Ok(())
        };
        active.runtime_working_directory = None;
        let controller_revocation = active
            .controller
            .revoke()
            .map_err(|error| error.to_string());
        persistence?;
        runtime_termination?;
        controller_revocation?;
        Ok(())
    }

    fn poll_runtime(&mut self, timeout_millis: u64) -> Result<Value, String> {
        if timeout_millis > MAX_EVENT_POLL_MILLIS {
            return Err("agent runtime poll exceeds the bounded timeout".to_owned());
        }
        let active = self.active.as_mut().ok_or("no active agent session")?;
        if active.revoked {
            return Err("agent session has been revoked".to_owned());
        }
        let runtime = active
            .runtime
            .as_mut()
            .ok_or("no compatible DSH runtime is active")?;
        let event =
            runtime.forward_next_event(Duration::from_millis(timeout_millis), &active.controller);
        if let Ok(event) = event {
            Ok(json!({"ok": true, "event": event}))
        } else {
            active.runtime = None;
            active.runtime_working_directory = None;
            let capabilities = without_external_egress(&active.delegated_capabilities);
            apply_delegated_grants(active, capabilities)?;
            Err("DSH runtime event transport failed".to_owned())
        }
    }

    fn cancel_runtime(&mut self) -> Result<Value, String> {
        let active = self.active.as_mut().ok_or("no active agent session")?;
        let Some(mut runtime) = active.runtime.take() else {
            let capabilities = without_external_egress(&active.delegated_capabilities);
            apply_delegated_grants(active, capabilities)?;
            return Ok(json!({"ok": true, "cancelRequested": false}));
        };
        let termination = runtime
            .terminate_for_cancellation_with(&active.controller)
            .map_err(|_| "DSH runtime cancellation failed".to_owned());
        active.runtime_working_directory = None;
        let capabilities = without_external_egress(&active.delegated_capabilities);
        let grant_update = apply_delegated_grants(active, capabilities);
        termination?;
        grant_update?;
        Ok(json!({"ok": true, "cancelRequested": true}))
    }

    fn active_controller(&self) -> Result<&SupervisedMcpController, String> {
        self.active
            .as_ref()
            .map(|active| &active.controller)
            .ok_or_else(|| "no active agent session".to_owned())
    }

    fn clear_terminal_active(&mut self) -> Result<(), String> {
        let Some(active) = self.active.as_ref() else {
            return Ok(());
        };
        if !active.revoked {
            return Ok(());
        }
        require_revocation_marker(&active.session_directory, &active.descriptor)?;
        self.active = None;
        Ok(())
    }

    fn require_active_workspace(&self, workspace_root: &Path) -> Result<(), String> {
        let active = self.active.as_ref().ok_or("no active agent session")?;
        active.descriptor.validate(workspace_root)
    }

    #[cfg(test)]
    fn serve_agent_frames(&self, frames: &str) -> Result<Vec<Value>, String> {
        let active = self.active.as_ref().ok_or("no active agent session")?;
        let mut output = Vec::new();
        active
            .endpoint
            .serve(BufReader::new(Cursor::new(frames.as_bytes())), &mut output)?;
        String::from_utf8(output)
            .map_err(|_| "agent endpoint returned non-UTF-8 output".to_owned())?
            .lines()
            .map(|line| {
                serde_json::from_str(line)
                    .map_err(|_| "agent endpoint returned invalid JSON".to_owned())
            })
            .collect()
    }

    #[cfg(test)]
    fn active_session_directory(&self) -> Result<PathBuf, String> {
        let active = self.active.as_ref().ok_or("no active agent session")?;
        Ok(active.session_directory.clone())
    }
}

fn route_requires_active_workspace(route: &str) -> bool {
    matches!(
        route,
        "/api/agent/session/grants"
            | "/api/agent/session/events"
            | "/api/agent/session/recovery"
            | "/api/agent/runtime/poll"
    )
}

fn session_summary(session: &ActiveAgentSession) -> Value {
    json!({
        "schema": SESSION_DESCRIPTOR_SCHEMA,
        "sessionId": session.descriptor.session_id,
        "humanActorId": session.descriptor.human_actor_id,
        "delegatedClientId": session.descriptor.delegated_client_id,
        "workspaceScopeId": session.descriptor.workspace_scope_id,
        "scopeNodeIds": session.descriptor.scope_node_ids,
        "actorCapabilities": session.descriptor.actor_capabilities,
        "delegatedCapabilities": session.delegated_capabilities,
        "delegatedCapabilityCeiling": session.descriptor.delegated_capability_ceiling,
        "workspacePolicyCapabilities": session.descriptor.workspace_policy_capabilities,
        "runtime": {
            "wireVersion": session.descriptor.runtime_wire_version,
            "resumeSupported": session.descriptor.resume_supported,
            "cancellation": session.descriptor.cancellation,
            "previouslyHandshaken": session.descriptor.runtime_was_handshaken,
            "active": session.runtime.is_some(),
            "handshake": session.handshake,
        },
        "recovered": session.recovered,
        "revoked": session.revoked,
        "approvalAndCommitAgentCallable": false,
    })
}

fn session_config(descriptor: &AgentSessionDescriptor) -> SupervisedSessionConfig {
    SupervisedSessionConfig {
        broker: AgentBrokerConfig::default(),
        audit: AgentAuditConfig::default(),
        runtime: SupervisedRuntimeContract {
            wire_version: descriptor.runtime_wire_version.clone(),
            resume_supported: descriptor.resume_supported,
            cancellation: descriptor.cancellation,
        },
        max_buffered_events: 1_024,
    }
}

fn base_host_capabilities() -> Vec<AgentCapability> {
    vec![
        AgentCapability::ReadWorkspace,
        AgentCapability::SearchWorkspace,
    ]
}

fn build_scope(workspace_root: &Path, node_ids: &[NodeId]) -> Result<WorkspaceReadScope, String> {
    let inventory = scan_workspace(workspace_root);
    if !inventory.is_valid() {
        return Err("agent scope requires a valid Weftext workspace".to_owned());
    }
    let projections = node_ids
        .iter()
        .map(|node_id| {
            let matches = inventory
                .nodes
                .iter()
                .filter(|node| node.id == Some(*node_id))
                .count();
            if matches != 1 {
                return Err("agent scope contains an unavailable or ambiguous node".to_owned());
            }
            Ok(WorkspaceNodeProjection::new(
                *node_id,
                None,
                format!("node-{node_id}"),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    WorkspaceReadScope::new(projections).map_err(|_| "agent scope is invalid".to_owned())
}

fn validate_scope_node_ids(node_ids: &[NodeId]) -> Result<(), String> {
    if node_ids.is_empty() || node_ids.len() > MAX_SCOPED_NODES {
        return Err("agent scope node count is outside the bounded contract".to_owned());
    }
    let unique = node_ids.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != node_ids.len() {
        return Err("agent scope contains duplicate node identities".to_owned());
    }
    Ok(())
}

fn validate_capabilities(
    capabilities: &[AgentCapability],
) -> Result<BTreeSet<AgentCapability>, String> {
    if capabilities.len() > 5 {
        return Err("agent capability list exceeds the closed contract".to_owned());
    }
    let unique = capabilities.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != capabilities.len() {
        return Err("agent capability list contains duplicates".to_owned());
    }
    if unique.contains(&AgentCapability::ProposeMutation)
        || unique.contains(&AgentCapability::CommitApprovedMutation)
    {
        return Err(
            "Desktop agent mutation capabilities are unavailable without a typed IR contract"
                .to_owned(),
        );
    }
    Ok(unique)
}

fn grant(capabilities: &[AgentCapability]) -> CapabilityGrant {
    CapabilityGrant::new(capabilities.iter().copied())
}

fn without_external_egress(capabilities: &[AgentCapability]) -> Vec<AgentCapability> {
    capabilities
        .iter()
        .copied()
        .filter(|capability| *capability != AgentCapability::ExternalEgress)
        .collect()
}

fn apply_delegated_grants(
    active: &mut ActiveAgentSession,
    capabilities: Vec<AgentCapability>,
) -> Result<(), String> {
    match active.controller.update_capability_grants(
        grant(&active.descriptor.actor_capabilities),
        grant(&capabilities),
        grant(&active.descriptor.workspace_policy_capabilities),
    ) {
        Ok(()) => {
            active.delegated_capabilities = capabilities;
            Ok(())
        }
        Err(error) => {
            active.delegated_capabilities.clear();
            let _ = active.controller.revoke();
            Err(error.to_string())
        }
    }
}

fn sort_capabilities(capabilities: &mut [AgentCapability]) {
    capabilities.sort_unstable();
}

fn sorted_node_ids(mut node_ids: Vec<NodeId>) -> Vec<NodeId> {
    node_ids.sort_unstable();
    node_ids
}

fn workspace_key(workspace_root: &Path) -> Result<String, String> {
    let canonical = workspace_root
        .canonicalize()
        .map_err(|_| "agent workspace is unavailable".to_owned())?;
    let normalized = normalized_path_key(&canonical);
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(normalized.as_bytes())
    ))
}

fn prepare_control_root(config_dir: &Path, workspace_root: &Path) -> Result<PathBuf, String> {
    if !config_dir.is_absolute()
        || config_dir
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(
            "desktop agent control state requires an absolute device-local path".to_owned(),
        );
    }
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|_| "agent workspace is unavailable".to_owned())?;
    let prospective_config = prospective_canonical_path(config_dir)?;
    if path_is_within(&prospective_config, &workspace_root)
        || path_is_within(&workspace_root, &prospective_config)
    {
        return Err("desktop agent control state and workspace must be disjoint".to_owned());
    }
    fs::create_dir_all(config_dir)
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
    let metadata = fs::symlink_metadata(config_dir)
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err("desktop agent control directory is linked or invalid".to_owned());
    }
    let config_dir = config_dir
        .canonicalize()
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
    if path_is_within(&config_dir, &workspace_root) || path_is_within(&workspace_root, &config_dir)
    {
        return Err("desktop agent control state and workspace must be disjoint".to_owned());
    }
    let control_root = checked_directory(&config_dir, CONTROL_DIRECTORY)?;
    let _sessions = checked_directory(&control_root, SESSIONS_DIRECTORY)?;
    Ok(control_root)
}

fn checked_directory(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&path)
                .map_err(|_| "desktop agent control directory could not be created".to_owned())?;
        }
        Err(_) => return Err("desktop agent control directory is unavailable".to_owned()),
    }
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err("desktop agent control directory is linked or invalid".to_owned());
    }
    set_private_directory_permissions(&path)?;
    let canonical = path
        .canonicalize()
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
    if canonical.parent() != Some(parent) {
        return Err("desktop agent control directory escaped its reviewed parent".to_owned());
    }
    Ok(canonical)
}

fn existing_checked_directory(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let path = parent.join(name);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err("desktop agent control directory is linked or invalid".to_owned());
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
    if canonical.parent() != Some(parent) {
        return Err("desktop agent control directory escaped its reviewed parent".to_owned());
    }
    Ok(canonical)
}

fn create_session_directory(control_root: &Path, session_id: &str) -> Result<PathBuf, String> {
    let sessions = existing_checked_directory(control_root, SESSIONS_DIRECTORY)?;
    let directory = sessions.join(session_id);
    fs::create_dir(&directory)
        .map_err(|_| "desktop agent session directory could not be created".to_owned())?;
    let metadata = fs::symlink_metadata(&directory)
        .map_err(|_| "desktop agent session directory is unavailable".to_owned())?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err("desktop agent session directory is linked or invalid".to_owned());
    }
    set_private_directory_permissions(&directory)?;
    let canonical = directory
        .canonicalize()
        .map_err(|_| "desktop agent session directory is unavailable".to_owned())?;
    if canonical.parent() != Some(sessions.as_path()) {
        return Err("desktop agent session directory escaped its reviewed parent".to_owned());
    }
    Ok(canonical)
}

fn existing_session_directory(control_root: &Path, session_id: &str) -> Result<PathBuf, String> {
    let sessions = existing_checked_directory(control_root, SESSIONS_DIRECTORY)?;
    let directory = sessions.join(session_id);
    let metadata = fs::symlink_metadata(&directory)
        .map_err(|_| "desktop agent session is unavailable".to_owned())?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err("desktop agent session directory is linked or invalid".to_owned());
    }
    let canonical = directory
        .canonicalize()
        .map_err(|_| "desktop agent session is unavailable".to_owned())?;
    if canonical.parent() != Some(sessions.as_path()) {
        return Err("desktop agent session directory escaped its reviewed parent".to_owned());
    }
    Ok(canonical)
}

fn prospective_canonical_path(path: &Path) -> Result<PathBuf, String> {
    let mut existing = path;
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(existing) {
            Ok(_) => {
                let mut resolved = existing
                    .canonicalize()
                    .map_err(|_| "desktop agent control directory is unavailable".to_owned())?;
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = existing
                    .file_name()
                    .ok_or_else(|| "desktop agent control directory is unavailable".to_owned())?;
                missing.push(name.to_os_string());
                existing = existing
                    .parent()
                    .ok_or_else(|| "desktop agent control directory is unavailable".to_owned())?;
            }
            Err(_) => {
                return Err("desktop agent control directory is unavailable".to_owned());
            }
        }
    }
}

fn new_session_id(control_root: &Path) -> Result<String, String> {
    let sessions = control_root.join(SESSIONS_DIRECTORY);
    for _ in 0..8 {
        let candidate = Uuid::new_v4().to_string();
        if !sessions.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("could not allocate an agent session identity".to_owned())
}

fn write_descriptor(directory: &Path, descriptor: &AgentSessionDescriptor) -> Result<(), String> {
    let bytes = serde_json::to_vec(descriptor)
        .map_err(|_| "agent session descriptor could not be encoded".to_owned())?;
    if bytes.len() > usize::try_from(MAX_DESCRIPTOR_BYTES).unwrap_or(usize::MAX) {
        return Err("agent session descriptor exceeds its byte bound".to_owned());
    }
    let path = directory.join(SESSION_DESCRIPTOR);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "agent session descriptor could not be created".to_owned())?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "agent session descriptor could not be persisted".to_owned())
}

fn read_descriptor(directory: &Path) -> Result<AgentSessionDescriptor, String> {
    let path = directory.join(SESSION_DESCRIPTOR);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "agent session descriptor is unavailable".to_owned())?;
    if !metadata.is_file() || is_link_or_reparse(&metadata) || metadata.len() > MAX_DESCRIPTOR_BYTES
    {
        return Err("agent session descriptor is linked, invalid, or oversized".to_owned());
    }
    let bytes = fs::read(path).map_err(|_| "agent session descriptor is unreadable".to_owned())?;
    serde_json::from_slice(&bytes)
        .map_err(|_| "agent session descriptor has an invalid closed schema".to_owned())
}

fn descriptor_sha256(descriptor: &AgentSessionDescriptor) -> Result<String, String> {
    let bytes = serde_json::to_vec(descriptor)
        .map_err(|_| "agent session descriptor could not be encoded".to_owned())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn write_revocation_marker(
    directory: &Path,
    descriptor: &AgentSessionDescriptor,
) -> Result<(), String> {
    let marker_path = directory.join(SESSION_REVOCATION);
    match fs::symlink_metadata(&marker_path) {
        Ok(_) => {
            read_revocation_marker(directory, descriptor)?;
            return remove_completed_revocation_pending(directory);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("agent session revocation marker is unavailable".to_owned()),
    }

    let pending_path = directory.join(SESSION_REVOCATION_PENDING);
    let mut pending = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending_path)
        .map_err(|_| "agent session revocation could not be reserved".to_owned())?;
    pending
        .write_all(b"weftext.desktop.agent-session-revocation.pending.v1\n")
        .and_then(|()| pending.sync_all())
        .map_err(|_| "agent session revocation reservation could not be persisted".to_owned())?;
    sync_control_directory(directory)?;

    let marker = AgentSessionRevocation::for_descriptor(descriptor)?;
    let bytes = serde_json::to_vec(&marker)
        .map_err(|_| "agent session revocation marker could not be encoded".to_owned())?;
    if bytes.len() > usize::try_from(MAX_REVOCATION_BYTES).unwrap_or(usize::MAX) {
        return Err("agent session revocation marker exceeds its byte bound".to_owned());
    }
    let mut temporary = NamedTempFile::new_in(directory)
        .map_err(|_| "agent session revocation marker could not be staged".to_owned())?;
    temporary
        .write_all(&bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|_| "agent session revocation marker could not be persisted".to_owned())?;
    let persisted = temporary
        .persist_noclobber(&marker_path)
        .map_err(|_| "agent session revocation marker could not be published".to_owned())?;
    persisted
        .sync_all()
        .map_err(|_| "agent session revocation marker could not be persisted".to_owned())?;
    sync_control_directory(directory)?;
    remove_completed_revocation_pending(directory)
}

fn remove_completed_revocation_pending(directory: &Path) -> Result<(), String> {
    let pending_path = directory.join(SESSION_REVOCATION_PENDING);
    match fs::symlink_metadata(&pending_path) {
        Ok(metadata) => {
            if !metadata.is_file() || is_link_or_reparse(&metadata) {
                return Err("agent session revocation reservation is invalid".to_owned());
            }
            fs::remove_file(&pending_path).map_err(|_| {
                "agent session revocation reservation could not be cleared".to_owned()
            })?;
            sync_control_directory(directory)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("agent session revocation reservation is unavailable".to_owned()),
    }
}

fn refuse_revoked_session(
    directory: &Path,
    descriptor: &AgentSessionDescriptor,
) -> Result<(), String> {
    let pending_path = directory.join(SESSION_REVOCATION_PENDING);
    match fs::symlink_metadata(pending_path) {
        Ok(_) => {
            return Err("agent session revocation is pending; recovery is refused".to_owned());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("agent session revocation state is unavailable".to_owned()),
    }
    let marker_path = directory.join(SESSION_REVOCATION);
    match fs::symlink_metadata(marker_path) {
        Ok(_) => {
            read_revocation_marker(directory, descriptor)?;
            Err("agent session has been terminally revoked".to_owned())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("agent session revocation state is unavailable".to_owned()),
    }
}

fn require_revocation_marker(
    directory: &Path,
    descriptor: &AgentSessionDescriptor,
) -> Result<(), String> {
    let pending_path = directory.join(SESSION_REVOCATION_PENDING);
    match fs::symlink_metadata(pending_path) {
        Ok(_) => return Err("agent session revocation is pending".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("agent session revocation state is unavailable".to_owned()),
    }
    read_revocation_marker(directory, descriptor).map(|_| ())
}

fn read_revocation_marker(
    directory: &Path,
    descriptor: &AgentSessionDescriptor,
) -> Result<AgentSessionRevocation, String> {
    let path = directory.join(SESSION_REVOCATION);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "agent session revocation marker is unavailable".to_owned())?;
    if !metadata.is_file() || is_link_or_reparse(&metadata) || metadata.len() > MAX_REVOCATION_BYTES
    {
        return Err("agent session revocation marker is linked, invalid, or oversized".to_owned());
    }
    let bytes =
        fs::read(path).map_err(|_| "agent session revocation marker is unreadable".to_owned())?;
    let marker: AgentSessionRevocation = serde_json::from_slice(&bytes)
        .map_err(|_| "agent session revocation marker has an invalid closed schema".to_owned())?;
    marker.validate(descriptor)?;
    Ok(marker)
}

#[cfg(unix)]
fn sync_control_directory(directory: &Path) -> Result<(), String> {
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|_| "agent session control directory could not be persisted".to_owned())
}

#[cfg(not(unix))]
fn sync_control_directory(directory: &Path) -> Result<(), String> {
    let metadata = fs::metadata(directory)
        .map_err(|_| "agent session control directory is unavailable".to_owned())?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err("agent session control directory is invalid".to_owned())
    }
}

enum RuntimeStatus {
    Missing,
    Invalid(&'static str),
    Verified,
}

fn runtime_status(config_dir: &Path, workspace_root: &Path) -> RuntimeStatus {
    let path = config_dir.join(RUNTIME_CONFIG_FILE);
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return RuntimeStatus::Missing;
        }
        Ok(_) => {}
        Err(_) => return RuntimeStatus::Invalid("dsh_runtime_configuration_invalid"),
    }
    match load_verified_runtime(config_dir, workspace_root) {
        Ok(_) => RuntimeStatus::Verified,
        Err(_) => RuntimeStatus::Invalid("dsh_runtime_configuration_invalid"),
    }
}

fn load_verified_runtime(
    config_dir: &Path,
    workspace_root: &Path,
) -> Result<VerifiedDshRuntime, String> {
    let path = config_dir.join(RUNTIME_CONFIG_FILE);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "configured DSH runtime is unavailable".to_owned())?;
    if !metadata.is_file() || is_link_or_reparse(&metadata) || metadata.len() > MAX_CONFIG_BYTES {
        return Err("configured DSH runtime metadata is invalid".to_owned());
    }
    let bytes =
        fs::read(path).map_err(|_| "configured DSH runtime metadata is unreadable".to_owned())?;
    let configuration: DshRuntimeConfiguration = serde_json::from_slice(&bytes)
        .map_err(|_| "configured DSH runtime metadata has an invalid closed schema".to_owned())?;
    validate_runtime_configuration(&configuration)?;
    let executable = configuration
        .executable
        .canonicalize()
        .map_err(|_| "configured DSH executable is unavailable".to_owned())?;
    let executable_metadata = fs::symlink_metadata(&configuration.executable)
        .map_err(|_| "configured DSH executable is unavailable".to_owned())?;
    if !executable_metadata.is_file() || is_link_or_reparse(&executable_metadata) {
        return Err("configured DSH executable is linked or invalid".to_owned());
    }
    let workspace = workspace_root
        .canonicalize()
        .map_err(|_| "agent workspace is unavailable".to_owned())?;
    if path_is_within(&executable, &workspace) {
        return Err("configured DSH executable cannot be workspace content".to_owned());
    }
    let actual_digest = sha256_file(&executable)?;
    if actual_digest != configuration.executable_sha256 {
        return Err("configured DSH executable digest does not match".to_owned());
    }
    let workspace_key = normalized_path_key(&workspace);
    if configuration
        .arguments
        .iter()
        .map(String::as_str)
        .chain([
            configuration.provider.as_str(),
            configuration.model.as_str(),
        ])
        .any(|value| normalized_text_key(value).contains(&workspace_key))
    {
        return Err("configured DSH metadata cannot grant a raw workspace path".to_owned());
    }
    Ok(VerifiedDshRuntime {
        configuration,
        executable,
    })
}

fn validate_runtime_configuration(configuration: &DshRuntimeConfiguration) -> Result<(), String> {
    if configuration.schema != RUNTIME_CONFIG_SCHEMA
        || !configuration.executable.is_absolute()
        || configuration
            .executable
            .components()
            .any(|component| component == Component::ParentDir)
        || !is_lower_hex_digest(&configuration.executable_sha256)
        || configuration.arguments.len() > MAX_RUNTIME_ARGUMENTS
        || configuration.arguments.iter().any(|argument| {
            argument.is_empty()
                || argument.len() > MAX_RUNTIME_ARGUMENT_BYTES
                || argument.chars().any(char::is_control)
        })
        || invalid_runtime_id(&configuration.provider)
        || invalid_runtime_id(&configuration.model)
        || configuration.max_tokens == Some(0)
        || !(MIN_RUNTIME_TIMEOUT_MILLIS..=MAX_RUNTIME_TIMEOUT_MILLIS)
            .contains(&configuration.request_timeout_millis)
    {
        return Err("configured DSH runtime metadata is outside the bounded contract".to_owned());
    }
    Ok(())
}

fn invalid_runtime_id(value: &str) -> bool {
    value.trim().is_empty()
        || value.len() > MAX_RUNTIME_ID_BYTES
        || value.chars().any(char::is_control)
}

fn restrict_runtime_environment(
    command: &mut Command,
    private_temporary: &Path,
    workspace_root: &Path,
) {
    #[cfg(windows)]
    const ALLOWED: &[&str] = &["SystemRoot", "WINDIR", "COMSPEC", "PATHEXT"];
    #[cfg(not(windows))]
    const ALLOWED: &[&str] = &["PATH", "LANG", "LC_ALL"];
    command.env_clear();
    let workspace_key = normalized_path_key(workspace_root);
    for name in ALLOWED {
        if let Some(value) = std::env::var_os(name) {
            if !normalized_text_key(&value.to_string_lossy()).contains(&workspace_key) {
                command.env(name, value);
            }
        }
    }
    #[cfg(windows)]
    command
        .env("TEMP", private_temporary)
        .env("TMP", private_temporary);
    #[cfg(not(windows))]
    command.env("TMPDIR", private_temporary);
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|_| "configured DSH executable is unreadable".to_owned())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "configured DSH executable is unreadable".to_owned())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn parse_body<T>(body: Option<Value>, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(body.ok_or_else(|| format!("{label} requires a typed body"))?)
        .map_err(|_| format!("{label} body is outside the closed typed contract"))
}

fn require_no_body(body: Option<&Value>, label: &str) -> Result<(), String> {
    if body.is_some() {
        Err(format!("{label} does not accept a body"))
    } else {
        Ok(())
    }
}

fn is_exact_lowercase_uuid(value: &str) -> bool {
    Uuid::parse_str(value)
        .is_ok_and(|parsed| parsed.get_version_num() == 4 && parsed.to_string() == value)
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn path_is_within(path: &Path, ancestor: &Path) -> bool {
    let path = normalized_path_key(path);
    let ancestor = normalized_path_key(ancestor);
    if path == ancestor {
        return true;
    }
    let separator = std::path::MAIN_SEPARATOR;
    path.starts_with(&format!(
        "{}{separator}",
        ancestor.trim_end_matches(separator)
    ))
}

fn normalized_path_key(path: &Path) -> String {
    normalized_text_key(&path.to_string_lossy())
}

fn normalized_text_key(value: &str) -> String {
    #[cfg(windows)]
    {
        let normalized = value.replace('/', "\\");
        normalized
            .strip_prefix("\\\\?\\")
            .unwrap_or(&normalized)
            .to_lowercase()
    }
    #[cfg(not(windows))]
    {
        value.to_owned()
    }
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| {
        "desktop agent control directory permissions could not be restricted".to_owned()
    })
}

#[cfg(not(unix))]
fn set_private_directory_permissions(path: &Path) -> Result<(), String> {
    if fs::metadata(path)
        .map_err(|_| "desktop agent control directory is unavailable".to_owned())?
        .is_dir()
    {
        Ok(())
    } else {
        Err("desktop agent control directory is invalid".to_owned())
    }
}

#[cfg(windows)]
fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::{json, Value};
    use tempfile::{tempdir, TempDir};
    use weftext_agent::AgentCapability;
    use weftext_core::{create_child_node, create_workspace, NodeId};

    use super::{
        load_verified_runtime, sha256_file, DesktopAgentLifecycle, AUDIT_DIRECTORY,
        CONTROL_DIRECTORY, RUNTIME_CONFIG_FILE, RUNTIME_CONFIG_SCHEMA, SESSIONS_DIRECTORY,
        SESSION_DESCRIPTOR, SESSION_REVOCATION, SESSION_REVOCATION_PENDING,
        SESSION_REVOCATION_SCHEMA,
    };

    struct Fixture {
        temporary: TempDir,
        workspace: PathBuf,
        config: PathBuf,
        node_id: NodeId,
    }

    impl Fixture {
        fn new() -> Self {
            let temporary = tempdir().expect("temporary root");
            let workspace = temporary.path().join("Notes");
            let config = temporary.path().join("device-config");
            create_workspace(&workspace).expect("workspace");
            let child = create_child_node(&workspace, "Child").expect("child");
            Self {
                temporary,
                workspace,
                config,
                node_id: child.id,
            }
        }

        fn manager(&self) -> DesktopAgentLifecycle {
            DesktopAgentLifecycle::new(self.config.clone())
        }
    }

    fn delegated_capabilities() -> Vec<AgentCapability> {
        vec![
            AgentCapability::ReadWorkspace,
            AgentCapability::SearchWorkspace,
        ]
    }

    #[cfg(unix)]
    fn create_directory_alias(target: &Path, alias: &Path) {
        std::os::unix::fs::symlink(target, alias).expect("control symlink");
    }

    #[cfg(windows)]
    fn create_directory_alias(target: &Path, alias: &Path) {
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(alias)
            .arg(target)
            .status()
            .expect("launch mklink");
        assert!(status.success(), "create control junction");
    }

    fn start_session(manager: &mut DesktopAgentLifecycle, fixture: &Fixture) -> Value {
        manager
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [fixture.node_id],
                    "delegatedCapabilities": delegated_capabilities(),
                    "probeDshRuntime": false,
                })),
                &fixture.workspace,
            )
            .expect("start session")
    }

    fn initialize_endpoint(manager: &DesktopAgentLifecycle) -> Vec<String> {
        let responses = manager
            .serve_agent_frames(
                &[
                    json!({
                        "jsonrpc":"2.0",
                        "id":1,
                        "method":"initialize",
                        "params":{
                            "protocolVersion":"2025-06-18",
                            "capabilities":{},
                            "clientInfo":{"name":"desktop-agent-test","version":"1"}
                        }
                    })
                    .to_string(),
                    json!({
                        "jsonrpc":"2.0",
                        "method":"notifications/initialized"
                    })
                    .to_string(),
                    json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string(),
                ]
                .join("\n"),
            )
            .expect("initialize endpoint");
        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[0]["result"]["serverInfo"]["name"],
            "weftext-supervised"
        );
        responses[1]["result"]["tools"]
            .as_array()
            .expect("tool catalog")
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name").to_owned())
            .collect()
    }

    fn read_tree_text(root: &Path) -> String {
        let mut collected = String::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(path) = pending.pop() {
            for entry in std::fs::read_dir(path).expect("read control tree") {
                let entry = entry.expect("control entry");
                if entry.file_type().expect("entry type").is_dir() {
                    pending.push(entry.path());
                } else {
                    let bytes = std::fs::read(entry.path()).expect("read control file");
                    collected.push_str(&String::from_utf8_lossy(&bytes));
                }
            }
        }
        collected
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn private_endpoint_is_read_only_and_all_mutation_surfaces_fail_closed() {
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        let started = start_session(&mut manager, &fixture);
        assert_eq!(started["agent"]["agentExecutionAvailable"], false);
        assert_eq!(started["session"]["approvalAndCommitAgentCallable"], false);

        let tools = initialize_endpoint(&manager);
        assert_eq!(
            tools,
            ["workspace_inventory", "read_document", "mutation_status"]
        );
        let inventory = manager
            .serve_agent_frames(
                &(json!({
                    "jsonrpc":"2.0",
                    "id":3,
                    "method":"tools/call",
                    "params":{"name":"workspace_inventory","arguments":{}}
                })
                .to_string()
                    + "\n"),
            )
            .expect("scoped inventory");
        assert_eq!(inventory[0]["result"]["isError"], false);
        let read = manager
            .serve_agent_frames(
                &(json!({
                    "jsonrpc":"2.0",
                    "id":4,
                    "method":"tools/call",
                    "params":{"name":"read_document","arguments":{"nodeId":fixture.node_id}}
                })
                .to_string()
                    + "\n"),
            )
            .expect("scoped read");
        assert_eq!(read[0]["result"]["isError"], false);
        let status = manager
            .serve_agent_frames(
                &(json!({
                    "jsonrpc":"2.0",
                    "id":5,
                    "method":"tools/call",
                    "params":{"name":"mutation_status","arguments":{"requestId":"absent"}}
                })
                .to_string()
                    + "\n"),
            )
            .expect("read-only status");
        assert!(status[0].get("result").is_some());
        assert!(status[0].get("error").is_none());

        for (id, blocked) in [
            "propose_document_edits",
            "approve_mutation",
            "commit_mutation",
            "cancel_mutation",
            "revoke_session",
        ]
        .into_iter()
        .enumerate()
        {
            let response = manager
                .serve_agent_frames(
                    &(json!({
                        "jsonrpc":"2.0",
                        "id":id + 6,
                        "method":"tools/call",
                        "params":{"name":blocked,"arguments":{"requestId":"x"}}
                    })
                    .to_string()
                        + "\n"),
                )
                .expect("filtered endpoint");
            assert!(response[0].get("error").is_some(), "tool leaked: {blocked}");
        }
        for route in [
            "/api/agent/session/preview",
            "/api/agent/session/approve",
            "/api/agent/session/deny",
            "/api/agent/session/commit",
            "/api/agent/session/cancel",
        ] {
            assert!(
                manager.request(route, None, &fixture.workspace).is_err(),
                "retired Desktop mutation route remained callable: {route}"
            );
        }

        let session_directory = manager
            .active_session_directory()
            .expect("session directory");
        let control_text = read_tree_text(&session_directory);
        assert!(!control_text.contains(fixture.workspace.to_string_lossy().as_ref()));
        manager
            .request("/api/agent/session/revoke", None, &fixture.workspace)
            .expect("revoke read-only session");
        let mutation_error = manager
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [fixture.node_id],
                    "delegatedCapabilities": ["propose_mutation"],
                    "probeDshRuntime": false,
                })),
                &fixture.workspace,
            )
            .expect_err("raw document mutation capability must stay retired");
        assert!(mutation_error.contains("typed IR contract"));
    }

    #[test]
    fn terminal_revocation_and_regrant_expansion_fail_closed() {
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        start_session(&mut manager, &fixture);
        manager
            .request("/api/agent/session/revoke", None, &fixture.workspace)
            .expect("revoke");
        assert!(manager
            .request(
                "/api/agent/session/grants",
                Some(json!({"delegatedCapabilities":delegated_capabilities()})),
                &fixture.workspace,
            )
            .is_err());
    }

    #[test]
    fn trusted_controller_routes_reject_a_foreign_workspace_binding() {
        let fixture = Fixture::new();
        let foreign_workspace = fixture
            .workspace
            .parent()
            .expect("workspace parent")
            .join("Foreign");
        create_workspace(&foreign_workspace).expect("foreign workspace");
        let mut manager = fixture.manager();
        start_session(&mut manager, &fixture);
        assert!(manager
            .request(
                "/api/agent/session/grants",
                Some(json!({"delegatedCapabilities":delegated_capabilities()})),
                &foreign_workspace,
            )
            .is_err());
        let foreign_capability = manager
            .request("/api/agent/capability", None, &foreign_workspace)
            .expect("foreign capability response");
        assert!(foreign_capability["agent"]["activeSession"].is_null());

        manager
            .request("/api/agent/session/revoke", None, &foreign_workspace)
            .expect("shutdown remains available from the trusted host");
    }

    #[test]
    fn restart_recovery_is_body_free_and_descriptor_or_audit_tamper_blocks_reopen() {
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        let started = start_session(&mut manager, &fixture);
        let session_id = started["session"]["sessionId"]
            .as_str()
            .expect("session id")
            .to_owned();
        let session_directory = manager
            .active_session_directory()
            .expect("session directory");
        drop(manager);

        let mut restarted = fixture.manager();
        let recovered = restarted
            .request(
                "/api/agent/session/recover",
                Some(json!({"sessionId":session_id})),
                &fixture.workspace,
            )
            .expect("verified recovery");
        assert_eq!(recovered["requiresExplicitRegrant"], true);
        assert_eq!(recovered["runtimeResumed"], false);
        assert!(recovered["recovery"]
            .as_array()
            .expect("recovery array")
            .is_empty());
        let descriptor_path = session_directory.join(SESSION_DESCRIPTOR);
        let original_descriptor = std::fs::read(&descriptor_path).expect("descriptor");
        drop(restarted);

        let mut tampered: Value =
            serde_json::from_slice(&original_descriptor).expect("descriptor JSON");
        tampered["scopeNodeIds"] = json!([]);
        std::fs::write(
            &descriptor_path,
            serde_json::to_vec(&tampered).expect("tampered descriptor"),
        )
        .expect("write descriptor tamper");
        let mut refused = fixture.manager();
        assert!(refused
            .request(
                "/api/agent/session/recover",
                Some(json!({"sessionId":session_id})),
                &fixture.workspace,
            )
            .is_err());

        std::fs::write(&descriptor_path, original_descriptor).expect("restore descriptor");
        let first_record = session_directory
            .join(AUDIT_DIRECTORY)
            .join("records")
            .join("00000000000000000001.json");
        let mut record: Value =
            serde_json::from_slice(&std::fs::read(&first_record).expect("audit record"))
                .expect("audit JSON");
        record["timestampMillis"] = json!(1);
        std::fs::write(
            first_record,
            serde_json::to_vec(&record).expect("tampered audit"),
        )
        .expect("write audit tamper");
        let mut audit_refused = fixture.manager();
        assert!(audit_refused
            .request(
                "/api/agent/session/recover",
                Some(json!({"sessionId":session_id})),
                &fixture.workspace,
            )
            .is_err());
    }

    #[test]
    fn terminal_revocation_is_descriptor_bound_and_survives_restart() {
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        let started = start_session(&mut manager, &fixture);
        let session_id = started["session"]["sessionId"]
            .as_str()
            .expect("session id")
            .to_owned();
        let session_directory = manager
            .active_session_directory()
            .expect("session directory");
        manager
            .request("/api/agent/session/revoke", None, &fixture.workspace)
            .expect("terminal revocation");

        assert!(!session_directory.join(SESSION_REVOCATION_PENDING).exists());
        let marker: Value = serde_json::from_slice(
            &std::fs::read(session_directory.join(SESSION_REVOCATION)).expect("revocation marker"),
        )
        .expect("revocation JSON");
        assert_eq!(marker["schema"], SESSION_REVOCATION_SCHEMA);
        assert_eq!(marker["sessionId"], session_id);
        assert_eq!(
            marker["workspaceScopeId"],
            started["session"]["workspaceScopeId"]
        );
        assert_eq!(
            marker["delegatedCapabilityCeiling"],
            started["session"]["delegatedCapabilityCeiling"]
        );
        assert_eq!(marker["descriptorSha256"].as_str().map(str::len), Some(64));
        let replacement = start_session(&mut manager, &fixture);
        assert_ne!(replacement["session"]["sessionId"], session_id);
        drop(manager);

        let mut restarted = fixture.manager();
        let error = restarted
            .request(
                "/api/agent/session/recover",
                Some(json!({"sessionId":session_id.clone()})),
                &fixture.workspace,
            )
            .expect_err("revoked session must not recover");
        assert!(error.contains("terminally revoked"));

        let mut tampered_marker = marker;
        tampered_marker["descriptorSha256"] = json!("0".repeat(64));
        std::fs::write(
            session_directory.join(SESSION_REVOCATION),
            serde_json::to_vec(&tampered_marker).expect("tampered revocation marker"),
        )
        .expect("write revocation tamper");
        let tamper_error = restarted
            .request(
                "/api/agent/session/recover",
                Some(json!({"sessionId":session_id})),
                &fixture.workspace,
            )
            .expect_err("tampered revocation must fail closed");
        assert!(tamper_error.contains("invalid or belongs elsewhere"));
    }

    #[test]
    fn interrupted_revocation_publication_fails_closed_on_recovery() {
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        let started = start_session(&mut manager, &fixture);
        let session_id = started["session"]["sessionId"]
            .as_str()
            .expect("session id")
            .to_owned();
        let session_directory = manager
            .active_session_directory()
            .expect("session directory");
        drop(manager);
        std::fs::write(
            session_directory.join(SESSION_REVOCATION_PENDING),
            b"interrupted",
        )
        .expect("pending revocation");

        let mut restarted = fixture.manager();
        let error = restarted
            .request(
                "/api/agent/session/recover",
                Some(json!({"sessionId":session_id})),
                &fixture.workspace,
            )
            .expect_err("partial revocation must fail closed");
        assert!(error.contains("revocation is pending"));
    }

    #[test]
    fn disjoint_control_preflight_never_creates_workspace_content() {
        let fixture = Fixture::new();
        let forbidden_config = fixture.workspace.join("device-agent-control");
        let mut manager = DesktopAgentLifecycle::new(forbidden_config.clone());
        let error = manager
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [fixture.node_id],
                    "delegatedCapabilities": delegated_capabilities(),
                    "probeDshRuntime": false,
                })),
                &fixture.workspace,
            )
            .expect_err("workspace-local control state must be refused");
        assert!(error.contains("must be disjoint"));
        assert!(
            !forbidden_config.exists(),
            "control preflight wrote portable workspace content before refusing it"
        );

        let enclosing_config = fixture.temporary.path().join("enclosing-device-state");
        std::fs::create_dir(&enclosing_config).expect("enclosing config");
        let nested_workspace = enclosing_config.join("NestedWorkspace");
        create_workspace(&nested_workspace).expect("nested workspace");
        let nested_node_id = weftext_core::read_node_document(&nested_workspace)
            .expect("nested root document")
            .node_id;
        let mut reverse_manager = DesktopAgentLifecycle::new(enclosing_config.clone());
        reverse_manager
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [nested_node_id],
                    "delegatedCapabilities": delegated_capabilities(),
                    "probeDshRuntime": false,
                })),
                &nested_workspace,
            )
            .expect_err("workspace nested inside control state must be refused");
        assert!(
            !enclosing_config.join(CONTROL_DIRECTORY).exists(),
            "reverse-overlap preflight created agent control content"
        );
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn linked_or_reparse_control_child_is_rejected_without_following_it() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(&fixture.config).expect("config directory");
        let outside = fixture
            .config
            .parent()
            .expect("config parent")
            .join("outside-control-target");
        std::fs::create_dir(&outside).expect("outside target");
        create_directory_alias(&outside, &fixture.config.join(CONTROL_DIRECTORY));

        let mut manager = fixture.manager();
        let error = manager
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [fixture.node_id],
                    "delegatedCapabilities": delegated_capabilities(),
                    "probeDshRuntime": false,
                })),
                &fixture.workspace,
            )
            .expect_err("linked control state must be refused");
        assert!(error.contains("linked or invalid"));
        assert!(
            !outside.join(SESSIONS_DIRECTORY).exists(),
            "control setup followed a linked or reparse-point child"
        );
    }

    #[test]
    fn runtime_metadata_cannot_smuggle_the_workspace_path_in_provider_or_model() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(&fixture.config).expect("config directory");
        let executable = std::env::current_exe()
            .expect("current test executable")
            .canonicalize()
            .expect("canonical test executable");
        let executable_sha256 = sha256_file(&executable).expect("test executable digest");
        std::fs::write(
            fixture.config.join(RUNTIME_CONFIG_FILE),
            serde_json::to_vec(&json!({
                "schema": RUNTIME_CONFIG_SCHEMA,
                "executable": executable,
                "executableSha256": executable_sha256,
                "arguments": [],
                "provider": "fixture-provider",
                "model": fixture.workspace,
                "maxTokens": 128,
                "requestTimeoutMillis": 5_000,
            }))
            .expect("runtime config"),
        )
        .expect("write runtime config");
        assert!(
            load_verified_runtime(&fixture.config, &fixture.workspace).is_err(),
            "no DSH initialization field may receive the raw workspace path"
        );
    }

    #[cfg(windows)]
    #[test]
    #[allow(clippy::too_many_lines)]
    fn configured_digest_bound_dsh_probe_forwards_events_and_revokes_egress_honestly() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(&fixture.config).expect("config directory");
        let script = fixture
            .config
            .parent()
            .expect("config parent")
            .join("fake-dsh-runtime.ps1");
        std::fs::write(
            &script,
            r"
$ErrorActionPreference = 'Stop'
while ($null -ne ($line = [Console]::In.ReadLine())) {
    try { $frame = $line | ConvertFrom-Json } catch { continue }
    if ($frame.method -eq 'initialize') {
        @{ jsonrpc = '2.0'; method = 'runtime.ready'; params = @{} } | ConvertTo-Json -Compress -Depth 10 | Write-Output
        @{ jsonrpc = '2.0'; id = $frame.id; result = @{ serverInfo = @{ name = 'deepseek-harness-sdk-runtime'; version = '0.0.1' } } } | ConvertTo-Json -Compress -Depth 10 | Write-Output
    } elseif ($frame.method -eq 'shutdown') {
        @{ jsonrpc = '2.0'; id = $frame.id; result = @{} } | ConvertTo-Json -Compress -Depth 10 | Write-Output
        break
    }
}
",
        )
        .expect("fake runtime script");
        let powershell = PathBuf::from(std::env::var_os("SystemRoot").expect("Windows SystemRoot"))
            .join("System32/WindowsPowerShell/v1.0/powershell.exe")
            .canonicalize()
            .expect("Windows PowerShell");
        let executable_sha256 = sha256_file(&powershell).expect("PowerShell digest");
        std::fs::write(
            fixture.config.join(RUNTIME_CONFIG_FILE),
            serde_json::to_vec(&json!({
                "schema": RUNTIME_CONFIG_SCHEMA,
                "executable": powershell,
                "executableSha256": executable_sha256,
                "arguments": [
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    script,
                ],
                "provider": "fixture-provider",
                "model": "fixture-model",
                "maxTokens": 128,
                "requestTimeoutMillis": 5_000,
            }))
            .expect("runtime config"),
        )
        .expect("write runtime config");

        let mut manager = fixture.manager();
        let capability = manager
            .request("/api/agent/capability", None, &fixture.workspace)
            .expect("capability");
        assert_eq!(capability["agent"]["agentExecutionAvailable"], false);
        assert_eq!(capability["agent"]["dsh"]["configured"], true);
        assert_eq!(
            capability["agent"]["dsh"]["callerPackageDigestVerified"],
            true
        );

        let mut capabilities = delegated_capabilities();
        capabilities.push(AgentCapability::ExternalEgress);
        let started = manager
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [fixture.node_id],
                    "delegatedCapabilities": capabilities,
                    "probeDshRuntime": true,
                })),
                &fixture.workspace,
            )
            .expect("start DSH-backed supervised session");
        assert_eq!(started["session"]["runtime"]["active"], true);
        assert_eq!(
            started["session"]["runtime"]["handshake"]["runtimeName"],
            "deepseek-harness-sdk-runtime"
        );
        assert_eq!(started["agent"]["agentExecutionAvailable"], false);

        let polled = manager
            .request(
                "/api/agent/runtime/poll",
                Some(json!({"timeoutMillis":1})),
                &fixture.workspace,
            )
            .expect("forward queued runtime event");
        assert_eq!(polled["event"]["type"], "unknown");
        assert_eq!(polled["event"]["method"], "runtime.ready");
        let narrowed = manager
            .request(
                "/api/agent/session/grants",
                Some(json!({"delegatedCapabilities":delegated_capabilities()})),
                &fixture.workspace,
            )
            .expect("withdraw external egress");
        assert_eq!(narrowed["session"]["runtime"]["active"], false);
        assert!(!narrowed["session"]["delegatedCapabilities"]
            .as_array()
            .expect("delegated capabilities")
            .iter()
            .any(|capability| capability == "external_egress"));
        let capability = manager
            .request("/api/agent/capability", None, &fixture.workspace)
            .expect("capability after egress withdrawal");
        assert_eq!(capability["agent"]["agentExecutionAvailable"], false);
        assert_eq!(capability["agent"]["dsh"]["active"], false);
        let mut egress_regrant = delegated_capabilities();
        egress_regrant.push(AgentCapability::ExternalEgress);
        let error = manager
            .request(
                "/api/agent/session/grants",
                Some(json!({"delegatedCapabilities":egress_regrant})),
                &fixture.workspace,
            )
            .expect_err("terminated runtime must not regain external egress");
        assert!(error.contains("active digest-verified DSH runtime"));
        let events = manager
            .request(
                "/api/agent/session/events",
                Some(json!({"cursor":0})),
                &fixture.workspace,
            )
            .expect("merged events");
        let event_list = events["events"]["events"].as_array().expect("event list");
        assert!(event_list.iter().any(|event| {
            event["event"]["type"] == "runtime"
                && event["event"]["event"]["method"] == "runtime.ready"
        }));
        assert!(event_list.iter().any(|event| {
            event["event"]["type"] == "runtime_terminated_for_cancellation"
                && event["event"]["resume_supported"] == false
        }));

        manager
            .request("/api/agent/session/revoke", None, &fixture.workspace)
            .expect("revoke first runtime session");
        let mut second_capabilities = delegated_capabilities();
        second_capabilities.push(AgentCapability::ExternalEgress);
        manager
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [fixture.node_id],
                    "delegatedCapabilities": second_capabilities,
                    "probeDshRuntime": true,
                })),
                &fixture.workspace,
            )
            .expect("start second DSH-backed session");
        let cancelled = manager
            .request("/api/agent/runtime/cancel", None, &fixture.workspace)
            .expect("cancel DSH runtime");
        assert_eq!(cancelled["cancelRequested"], true);
        let capability = manager
            .request("/api/agent/capability", None, &fixture.workspace)
            .expect("capability after runtime cancellation");
        let delegated = capability["agent"]["activeSession"]["delegatedCapabilities"]
            .as_array()
            .expect("delegated capabilities after cancellation");
        assert!(!delegated.iter().any(|value| value == "external_egress"));
        assert_eq!(capability["agent"]["dsh"]["active"], false);
    }
}
