use std::collections::VecDeque;
use std::fmt;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};
use weftext_agent::{
    AgentRuntimeController, AgentRuntimeEvent, AgentSessionStatus, HarnessHandshake,
};

use crate::protocol::{DshCompatibilityPolicy, DshInitialize, DshPrompt, DshPromptReceipt};

const STDERR_TAIL_LIMIT: usize = 100;

/// Fail-closed errors produced by the first-party DSH bridge.
#[derive(Debug)]
pub enum DshError {
    Spawn(std::io::Error),
    MissingPipe(&'static str),
    Io(std::io::Error),
    Encode(serde_json::Error),
    Timeout,
    TransportClosed,
    AlreadyInitialized,
    NotInitialized,
    InvalidConfiguration(String),
    InvalidResponse(String),
    Rpc {
        code: i64,
        message: String,
        data: Option<Value>,
    },
    IncompatibleRuntimeName {
        expected: String,
        actual: String,
    },
    UnsupportedRuntimeVersion {
        actual: String,
        supported: Vec<String>,
    },
    ProcessTermination(String),
    Controller(String),
}

impl fmt::Display for DshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(error) => write!(formatter, "failed to launch DSH runtime: {error}"),
            Self::MissingPipe(name) => write!(formatter, "DSH runtime has no piped {name}"),
            Self::Io(error) => write!(formatter, "DSH transport I/O failed: {error}"),
            Self::Encode(error) => {
                write!(formatter, "failed to encode DSH JSON-RPC frame: {error}")
            }
            Self::Timeout => formatter.write_str("DSH request timed out"),
            Self::TransportClosed => formatter.write_str("DSH transport closed"),
            Self::AlreadyInitialized => formatter.write_str("DSH runtime is already initialized"),
            Self::NotInitialized => formatter.write_str("DSH runtime is not initialized"),
            Self::InvalidConfiguration(message) | Self::InvalidResponse(message) => {
                formatter.write_str(message)
            }
            Self::Rpc {
                code,
                message,
                data,
            } => write!(formatter, "DSH JSON-RPC error {code}: {message} ({data:?})"),
            Self::IncompatibleRuntimeName { expected, actual } => write!(
                formatter,
                "incompatible DSH runtime name {actual:?}; expected {expected:?}"
            ),
            Self::UnsupportedRuntimeVersion { actual, supported } => write!(
                formatter,
                "unsupported DSH runtime version {actual:?}; supported versions: {}",
                supported.join(", ")
            ),
            Self::ProcessTermination(message) => formatter.write_str(message),
            Self::Controller(message) => {
                write!(
                    formatter,
                    "trusted agent controller rejected a runtime event: {message}"
                )
            }
        }
    }
}

impl std::error::Error for DshError {}

enum Incoming {
    Frame(Value),
    Eof,
    Io(String),
}

/// Synchronous owner of one DSH SDK runtime subprocess.
pub struct DshClient {
    child: Option<Child>,
    writer: Option<BufWriter<ChildStdin>>,
    incoming: mpsc::Receiver<Incoming>,
    notifications: VecDeque<AgentRuntimeEvent>,
    stderr_tail: Arc<Mutex<VecDeque<String>>>,
    next_request_id: u64,
    request_timeout: Duration,
    initialize_attempted: bool,
    initialized: bool,
}

impl DshClient {
    /// Launches a DSH SDK runtime with piped protocol streams.
    ///
    /// # Errors
    ///
    /// Returns an error when the process cannot start or its standard streams
    /// cannot be reserved for JSON-RPC.
    pub fn spawn(mut command: Command, request_timeout: Duration) -> Result<Self, DshError> {
        if request_timeout.is_zero() {
            return Err(DshError::InvalidConfiguration(
                "DSH request timeout must be positive".to_owned(),
            ));
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(DshError::Spawn)?;
        let stdin = child.stdin.take().ok_or(DshError::MissingPipe("stdin"))?;
        let stdout = child.stdout.take().ok_or(DshError::MissingPipe("stdout"))?;
        let stderr = child.stderr.take().ok_or(DshError::MissingPipe("stderr"))?;

        let (sender, incoming) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if let Ok(frame) = serde_json::from_str::<Value>(&line)
                            && sender.send(Incoming::Frame(frame)).is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Incoming::Io(error.to_string()));
                        return;
                    }
                }
            }
            let _ = sender.send(Incoming::Eof);
        });

        let stderr_tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL_LIMIT)));
        let stderr_reader_tail = Arc::clone(&stderr_tail);
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Ok(mut tail) = stderr_reader_tail.lock() {
                    if tail.len() == STDERR_TAIL_LIMIT {
                        tail.pop_front();
                    }
                    tail.push_back(line);
                }
            }
        });

        Ok(Self {
            child: Some(child),
            writer: Some(BufWriter::new(stdin)),
            incoming,
            notifications: VecDeque::new(),
            stderr_tail,
            next_request_id: 1,
            request_timeout,
            initialize_attempted: false,
            initialized: false,
        })
    }

    /// Performs the one allowed process-wide DSH initialization and validates compatibility.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid parameters, transport/RPC failure, a repeated
    /// initialization, or an unsupported DSH runtime identity/version.
    pub fn initialize(
        &mut self,
        parameters: &DshInitialize,
        policy: &DshCompatibilityPolicy,
    ) -> Result<HarnessHandshake, DshError> {
        if self.initialize_attempted {
            return Err(DshError::AlreadyInitialized);
        }
        let params = parameters.to_params()?;
        self.initialize_attempted = true;
        let result = self.request("initialize", Some(params))?;
        let initialize: InitializeResult = serde_json::from_value(result)
            .map_err(|error| DshError::InvalidResponse(error.to_string()))?;
        let handshake =
            policy.validate(initialize.server_info.name, initialize.server_info.version)?;
        self.initialized = true;
        Ok(handshake)
    }

    /// Enqueues one user message in a DSH session.
    ///
    /// # Errors
    ///
    /// Returns an error when initialization has not succeeded, prompt parameters
    /// are invalid, or the runtime rejects or fails to acknowledge the request.
    pub fn prompt(&mut self, prompt: &DshPrompt) -> Result<DshPromptReceipt, DshError> {
        if !self.initialized {
            return Err(DshError::NotInitialized);
        }
        let result = self.request("session/prompt", Some(prompt.to_params()?))?;
        serde_json::from_value(result).map_err(|error| DshError::InvalidResponse(error.to_string()))
    }

    /// Returns the next normalized runtime event, or `None` when the timeout elapses.
    ///
    /// # Errors
    ///
    /// Returns an error when the transport closes or emits an invalid known notification.
    pub fn next_event(&mut self, timeout: Duration) -> Result<Option<AgentRuntimeEvent>, DshError> {
        if let Some(event) = self.notifications.pop_front() {
            return Ok(Some(event));
        }
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            DshError::InvalidConfiguration("event timeout is too large".to_owned())
        })?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        match self.incoming.recv_timeout(remaining) {
            Ok(Incoming::Frame(frame)) => {
                if let Some(event) = normalize_notification(&frame)? {
                    return Ok(Some(event));
                }
                Err(DshError::InvalidResponse(
                    "unexpected DSH response while waiting for an event".to_owned(),
                ))
            }
            Ok(Incoming::Eof) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(DshError::TransportClosed)
            }
            Ok(Incoming::Io(message)) => Err(DshError::InvalidResponse(message)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
        }
    }

    /// Reads and forwards one normalized event into the trusted co-located
    /// controller. Runtime payloads remain transient; the controller's durable
    /// audit accepts only its closed redacted event schema.
    ///
    /// # Errors
    ///
    /// Returns a transport/protocol error or a redacted controller rejection.
    pub fn forward_next_event(
        &mut self,
        timeout: Duration,
        controller: &impl AgentRuntimeController,
    ) -> Result<Option<AgentRuntimeEvent>, DshError> {
        match self.next_event(timeout) {
            Ok(Some(event)) => {
                controller
                    .ingest_runtime_event(event.clone())
                    .map_err(DshError::Controller)?;
                Ok(Some(event))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                if let Some(error_code) = adapter_crash_code(&error) {
                    controller
                        .record_adapter_crash(error_code)
                        .map_err(DshError::Controller)?;
                }
                Err(error)
            }
        }
    }

    /// Returns recent DSH stderr lines without exposing them to the protocol stream.
    #[must_use]
    pub fn stderr_tail(&self) -> Vec<String> {
        self.stderr_tail
            .lock()
            .map_or_else(|_| Vec::new(), |tail| tail.iter().cloned().collect())
    }

    /// Gracefully requests runtime shutdown, then force-terminates on timeout.
    ///
    /// # Errors
    ///
    /// Returns an error when the shutdown request or process termination fails.
    pub fn shutdown(&mut self) -> Result<(), DshError> {
        if self.child.is_none() {
            return Ok(());
        }
        let request_result = self.request("shutdown", None).map(|_| ());
        self.writer.take();
        let wait_result = self.wait_or_kill();
        request_result.and(wait_result)
    }

    /// Cancels in-flight DSH work by terminating the entire runtime process.
    ///
    /// DSH preview wire version `0.0.1` has no prompt-cancel or session-close method.
    ///
    /// # Errors
    ///
    /// Returns an error when process termination fails.
    pub fn terminate_for_cancellation(&mut self) -> Result<(), DshError> {
        self.writer.take();
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        child.kill().map_err(|error| {
            DshError::ProcessTermination(format!("failed to terminate DSH runtime: {error}"))
        })?;
        child.wait().map_err(|error| {
            DshError::ProcessTermination(format!("failed to reap DSH runtime: {error}"))
        })?;
        Ok(())
    }

    /// Terminates DSH and records the exact wire limitation in the trusted
    /// controller. Wire `0.0.1` has no prompt cancel, close, or resume method.
    ///
    /// # Errors
    ///
    /// Returns a process-termination failure or a redacted controller rejection.
    pub fn terminate_for_cancellation_with(
        &mut self,
        controller: &impl AgentRuntimeController,
    ) -> Result<(), DshError> {
        match self.terminate_for_cancellation() {
            Ok(()) => controller
                .record_runtime_terminated_for_cancellation()
                .map_err(DshError::Controller),
            Err(error) => {
                controller
                    .record_adapter_crash("runtime_termination_failed")
                    .map_err(DshError::Controller)?;
                Err(error)
            }
        }
    }

    fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value, DshError> {
        let request_id = self.next_request_id;
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or_else(|| DshError::InvalidConfiguration("request ID overflow".to_owned()))?;
        let mut frame = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
        });
        if let Some(params) = params {
            frame["params"] = params;
        }
        self.write_frame(&frame)?;

        let deadline = Instant::now()
            .checked_add(self.request_timeout)
            .ok_or_else(|| {
                DshError::InvalidConfiguration("request timeout is too large".to_owned())
            })?;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(DshError::Timeout);
            }
            match self.incoming.recv_timeout(remaining) {
                Ok(Incoming::Frame(frame)) => {
                    if let Some(event) = normalize_notification(&frame)? {
                        self.notifications.push_back(event);
                        continue;
                    }
                    if frame.get("id").and_then(Value::as_u64) != Some(request_id) {
                        return Err(DshError::InvalidResponse(
                            "DSH returned an unexpected JSON-RPC response ID".to_owned(),
                        ));
                    }
                    if let Some(error) = frame.get("error") {
                        return Err(parse_rpc_error(error));
                    }
                    return frame.get("result").cloned().ok_or_else(|| {
                        DshError::InvalidResponse(
                            "DSH JSON-RPC response contains neither result nor error".to_owned(),
                        )
                    });
                }
                Ok(Incoming::Eof) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(DshError::TransportClosed);
                }
                Ok(Incoming::Io(message)) => return Err(DshError::InvalidResponse(message)),
                Err(mpsc::RecvTimeoutError::Timeout) => return Err(DshError::Timeout),
            }
        }
    }

    fn write_frame(&mut self, frame: &Value) -> Result<(), DshError> {
        let writer = self.writer.as_mut().ok_or(DshError::TransportClosed)?;
        serde_json::to_writer(&mut *writer, frame).map_err(DshError::Encode)?;
        writer.write_all(b"\n").map_err(DshError::Io)?;
        writer.flush().map_err(DshError::Io)
    }

    fn wait_or_kill(&mut self) -> Result<(), DshError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let deadline = Instant::now()
            .checked_add(self.request_timeout)
            .ok_or_else(|| {
                DshError::InvalidConfiguration("shutdown timeout is too large".to_owned())
            })?;
        loop {
            if child
                .try_wait()
                .map_err(|error| DshError::ProcessTermination(error.to_string()))?
                .is_some()
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                child.kill().map_err(|error| {
                    DshError::ProcessTermination(format!(
                        "failed to terminate DSH after shutdown timeout: {error}"
                    ))
                })?;
                child.wait().map_err(|error| {
                    DshError::ProcessTermination(format!("failed to reap DSH runtime: {error}"))
                })?;
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn adapter_crash_code(error: &DshError) -> Option<&'static str> {
    match error {
        DshError::TransportClosed => Some("runtime_transport_closed"),
        DshError::Io(_) => Some("runtime_io_failed"),
        DshError::InvalidResponse(_) | DshError::Rpc { .. } => Some("runtime_protocol_failed"),
        DshError::ProcessTermination(_) => Some("runtime_termination_failed"),
        DshError::Spawn(_)
        | DshError::MissingPipe(_)
        | DshError::Encode(_)
        | DshError::Timeout
        | DshError::AlreadyInitialized
        | DshError::NotInitialized
        | DshError::InvalidConfiguration(_)
        | DshError::IncompatibleRuntimeName { .. }
        | DshError::UnsupportedRuntimeVersion { .. }
        | DshError::Controller(_) => None,
    }
}

impl Drop for DshClient {
    fn drop(&mut self) {
        self.writer.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeResult {
    server_info: ServerInfo,
}

#[derive(Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
}

fn normalize_notification(frame: &Value) -> Result<Option<AgentRuntimeEvent>, DshError> {
    let Some(method) = frame.get("method").and_then(Value::as_str) else {
        return Ok(None);
    };
    if frame.get("id").is_some() {
        return Err(DshError::InvalidResponse(
            "server-to-client JSON-RPC requests are not supported by this DSH wire version"
                .to_owned(),
        ));
    }
    let params = frame.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "session.event" => {
            let session_id = required_string(&params, "sessionId", method)?;
            let event = params.get("event").cloned().ok_or_else(|| {
                DshError::InvalidResponse("session.event is missing event".to_owned())
            })?;
            Ok(Some(AgentRuntimeEvent::SessionEvent { session_id, event }))
        }
        "session.status" => {
            let session_id = required_string(&params, "sessionId", method)?;
            let status = match required_string(&params, "status", method)?.as_str() {
                "idle" => AgentSessionStatus::Idle,
                "running" => AgentSessionStatus::Running,
                value => {
                    return Err(DshError::InvalidResponse(format!(
                        "session.status has unknown status {value:?}"
                    )));
                }
            };
            Ok(Some(AgentRuntimeEvent::SessionStatus {
                session_id,
                status,
            }))
        }
        "subagent.started" => Ok(Some(AgentRuntimeEvent::SubagentStarted {
            parent_session_id: required_string(&params, "parentSessionId", method)?,
            child_session_id: required_string(&params, "childSessionId", method)?,
        })),
        "subagent.finished" => Ok(Some(AgentRuntimeEvent::SubagentFinished {
            payload: params,
        })),
        _ => Ok(Some(AgentRuntimeEvent::Unknown {
            method: method.to_owned(),
            params,
        })),
    }
}

fn required_string(params: &Value, field: &str, method: &str) -> Result<String, DshError> {
    params
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            DshError::InvalidResponse(format!("{method} is missing string field {field}"))
        })
}

fn parse_rpc_error(error: &Value) -> DshError {
    DshError::Rpc {
        code: error.get("code").and_then(Value::as_i64).unwrap_or(-32_603),
        message: error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown DSH JSON-RPC error")
            .to_owned(),
        data: error.get("data").cloned(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::process::Command;
    use std::sync::Mutex;
    use std::time::Duration;

    use serde_json::{Value, json};
    use weftext_agent::{
        AgentRuntimeController, AgentRuntimeEvent, AgentSessionStatus, CancellationMode,
    };

    use super::DshClient;
    use crate::{DshCompatibilityPolicy, DshError, DshInitialize, DshPrompt};

    const FAKE_RUNTIME_ENV: &str = "WEFTEXT_DSH_FAKE_RUNTIME";
    const FAKE_VERSION_ENV: &str = "WEFTEXT_DSH_FAKE_VERSION";

    #[derive(Default)]
    struct RecordingController {
        events: Mutex<Vec<AgentRuntimeEvent>>,
        crashes: Mutex<Vec<String>>,
        terminated: Mutex<u32>,
    }

    impl AgentRuntimeController for RecordingController {
        fn ingest_runtime_event(&self, event: AgentRuntimeEvent) -> Result<(), String> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }

        fn record_adapter_crash(&self, error_code: &str) -> Result<(), String> {
            self.crashes.lock().unwrap().push(error_code.to_owned());
            Ok(())
        }

        fn record_runtime_terminated_for_cancellation(&self) -> Result<(), String> {
            *self.terminated.lock().unwrap() += 1;
            Ok(())
        }
    }

    #[test]
    fn fake_runtime_process() {
        if std::env::var_os(FAKE_RUNTIME_ENV).is_none() {
            return;
        }
        let version = std::env::var(FAKE_VERSION_ENV).unwrap_or_else(|_| "0.0.1".to_owned());
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in BufReader::new(stdin.lock()).lines().map_while(Result::ok) {
            let Ok(frame) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let id = frame["id"].clone();
            match frame["method"].as_str() {
                Some("initialize") => write_json(
                    &mut stdout,
                    &json!({"jsonrpc":"2.0","id":id,"result":{"serverInfo":{"name":"deepseek-harness-sdk-runtime","version":version}}}),
                ),
                Some("session/prompt") => {
                    let session_id = frame["params"]["sessionId"].clone();
                    write_json(
                        &mut stdout,
                        &json!({"jsonrpc":"2.0","method":"session.status","params":{"sessionId":session_id,"status":"running"}}),
                    );
                    write_json(
                        &mut stdout,
                        &json!({"jsonrpc":"2.0","id":id,"result":{"messageId":"message-1"}}),
                    );
                    write_json(
                        &mut stdout,
                        &json!({"jsonrpc":"2.0","method":"session.event","params":{"sessionId":session_id,"event":{"type":"assistant.delta","text":"ok"}}}),
                    );
                    write_json(
                        &mut stdout,
                        &json!({"jsonrpc":"2.0","method":"session.status","params":{"sessionId":session_id,"status":"idle"}}),
                    );
                }
                Some("shutdown") => {
                    write_json(&mut stdout, &json!({"jsonrpc":"2.0","id":id,"result":{}}));
                    return;
                }
                _ => write_json(
                    &mut stdout,
                    &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}),
                ),
            }
        }
    }

    #[test]
    fn sdk_process_handshake_prompt_events_and_shutdown() {
        let mut client = spawn_fake_runtime("0.0.1");
        let handshake = client
            .initialize(&initialize_parameters(), &DshCompatibilityPolicy::default())
            .unwrap();
        assert_eq!(handshake.runtime_name, "deepseek-harness-sdk-runtime");
        assert_eq!(handshake.runtime_version, "0.0.1");
        assert_eq!(
            handshake.capabilities.cancellation,
            CancellationMode::RuntimeTermination
        );
        assert!(!handshake.capabilities.approval_requests);

        let receipt = client
            .prompt(&DshPrompt::text("session-1", "hello"))
            .unwrap();
        assert_eq!(receipt.message_id, "message-1");
        assert_eq!(
            client.next_event(Duration::from_secs(1)).unwrap(),
            Some(AgentRuntimeEvent::SessionStatus {
                session_id: "session-1".to_owned(),
                status: AgentSessionStatus::Running,
            })
        );
        assert!(matches!(
            client.next_event(Duration::from_secs(1)).unwrap(),
            Some(AgentRuntimeEvent::SessionEvent { .. })
        ));
        assert_eq!(
            client.next_event(Duration::from_secs(1)).unwrap(),
            Some(AgentRuntimeEvent::SessionStatus {
                session_id: "session-1".to_owned(),
                status: AgentSessionStatus::Idle,
            })
        );
        client.shutdown().unwrap();
    }

    #[test]
    fn incompatible_preview_version_fails_closed() {
        let mut client = spawn_fake_runtime("9.9.9");
        let error = client
            .initialize(&initialize_parameters(), &DshCompatibilityPolicy::default())
            .unwrap_err();
        assert!(matches!(error, DshError::UnsupportedRuntimeVersion { .. }));
        let retry_error = client
            .initialize(&initialize_parameters(), &DshCompatibilityPolicy::default())
            .unwrap_err();
        assert!(matches!(retry_error, DshError::AlreadyInitialized));
        client.terminate_for_cancellation().unwrap();
    }

    #[test]
    fn prompt_before_handshake_is_refused_locally() {
        let mut client = spawn_fake_runtime("0.0.1");
        let error = client
            .prompt(&DshPrompt::text("session-1", "hello"))
            .unwrap_err();
        assert!(matches!(error, DshError::NotInitialized));
        client.terminate_for_cancellation().unwrap();
    }

    #[test]
    fn repeated_initialize_is_refused_locally() {
        let mut client = spawn_fake_runtime("0.0.1");
        client
            .initialize(&initialize_parameters(), &DshCompatibilityPolicy::default())
            .unwrap();
        let error = client
            .initialize(&initialize_parameters(), &DshCompatibilityPolicy::default())
            .unwrap_err();
        assert!(matches!(error, DshError::AlreadyInitialized));
        client.terminate_for_cancellation().unwrap();
    }

    #[test]
    fn cancellation_terminates_the_preview_runtime() {
        let mut client = spawn_fake_runtime("0.0.1");
        client
            .initialize(&initialize_parameters(), &DshCompatibilityPolicy::default())
            .unwrap();
        client
            .prompt(&DshPrompt::text("session-1", "keep running"))
            .unwrap();

        client.terminate_for_cancellation().unwrap();
    }

    #[test]
    fn controller_receives_typed_events_and_runtime_termination_cancellation() {
        let mut client = spawn_fake_runtime("0.0.1");
        client
            .initialize(&initialize_parameters(), &DshCompatibilityPolicy::default())
            .unwrap();
        client
            .prompt(&DshPrompt::text("session-1", "keep running"))
            .unwrap();
        let controller = RecordingController::default();
        let event = client
            .forward_next_event(Duration::from_secs(1), &controller)
            .unwrap();
        assert!(matches!(
            event,
            Some(AgentRuntimeEvent::SessionStatus {
                status: AgentSessionStatus::Running,
                ..
            })
        ));
        assert_eq!(controller.events.lock().unwrap().len(), 1);
        client.terminate_for_cancellation_with(&controller).unwrap();
        assert_eq!(*controller.terminated.lock().unwrap(), 1);
        assert!(controller.crashes.lock().unwrap().is_empty());
    }

    fn spawn_fake_runtime(version: &str) -> DshClient {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["fake_runtime_process", "--nocapture"])
            .env(FAKE_RUNTIME_ENV, "1")
            .env(FAKE_VERSION_ENV, version);
        DshClient::spawn(command, Duration::from_secs(2)).unwrap()
    }

    fn initialize_parameters() -> DshInitialize {
        DshInitialize {
            cwd: std::env::current_dir().unwrap(),
            provider: "deepseek-official".to_owned(),
            model: "test-model".to_owned(),
            max_tokens: Some(1024),
        }
    }

    fn write_json(writer: &mut impl Write, value: &Value) {
        serde_json::to_writer(&mut *writer, value).unwrap();
        writer.write_all(b"\n").unwrap();
        writer.flush().unwrap();
    }
}
