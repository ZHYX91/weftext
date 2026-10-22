use super::security;
use super::*;
use crate::{
    client_protocol::{CallKind, LocalMethod, MAX_REPLY_BYTES, MAX_WIRE_BYTES},
    local_client::{
        LocalCall, LocalClientError, LocalReply, OwnerIdentity, OwnerRole, ProfileLocator,
        RemoteSyncSecret, ShutdownReceipt,
    },
};
use std::{
    collections::HashSet,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer},
    sync::{Semaphore, mpsc, oneshot, watch},
    task::JoinSet,
};
use weftext_core::QualifiedProfile;

const HANDSHAKE_LIMIT: Duration = Duration::from_secs(15);
const QUALIFICATION_LIMIT: Duration = Duration::from_secs(60);
const CALL_LIMIT: Duration = Duration::from_secs(180);

pub(crate) struct HostRequest {
    pub command: Command,
    pub reply: oneshot::Sender<io::Result<Vec<u8>>>,
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
fn pipe_name(profile: &QualifiedProfile, sid: &str) -> String {
    format!(
        r"\\.\pipe\Weftext.Owner.v1.{}",
        digest(format!("{sid}\0{}", profile.physical_identity()).as_bytes())
    )
}

fn owner_catalog() -> io::Result<&'static crate::artifact_catalog::Catalog> {
    crate::artifact_catalog::Catalog::get()
}

pub struct Connection {
    pipe: NamedPipeClient,
    _profile: QualifiedProfile,
    _peer: security::Peer,
    welcome: Welcome,
    sent: HashSet<String>,
    poisoned: bool,
}

impl Connection {
    pub fn profile_identity(&self) -> &str {
        &self.welcome.profile_identity
    }
    pub fn owner_identity(&self) -> OwnerIdentity {
        OwnerIdentity {
            pid: self._peer.pid,
            created: self._peer.created,
            incarnation: self.welcome.owner_incarnation.clone(),
            profile_identity: self.welcome.profile_identity.clone(),
            role: self.welcome.owner_role,
        }
    }

    pub async fn call(&mut self, call: LocalCall) -> Result<LocalReply, LocalClientError> {
        if self.welcome.owner_role != OwnerRole::Local
            || call.kind.local_route().is_none()
            || call.body.len() > MAX_WIRE_BYTES
        {
            return Err(LocalClientError::not_submitted(
                "invalidLocalCall",
                "The known operation or original wire exceeds the local contract",
            ));
        }
        let sideband = matches!(
            call.kind,
            CallKind::RemoteSyncInspect | CallKind::RemoteSyncStart
        );
        if sideband != call.secret.is_some() {
            return Err(LocalClientError::not_submitted(
                "invalidSecretSideband",
                "Only the two fixed remote-sync calls require a transient credential",
            ));
        }
        self.request(
            call.request_id,
            Command::Business { kind: call.kind },
            &call.body,
            call.secret.as_ref(),
        )
        .await
    }

    pub(crate) async fn control(
        &mut self,
        command: Command,
    ) -> Result<LocalReply, LocalClientError> {
        self.request(uuid::Uuid::new_v4().to_string(), command, &[], None)
            .await
    }

    async fn request(
        &mut self,
        request_id: String,
        command: Command,
        body: &[u8],
        secret: Option<&RemoteSyncSecret>,
    ) -> Result<LocalReply, LocalClientError> {
        if self.poisoned
            || !canonical_uuid(&request_id)
            || self.sent.len() >= 4096
            || !self.sent.insert(request_id.clone())
        {
            return Err(LocalClientError::not_submitted(
                "invalidLocalSession",
                "The original owner connection cannot accept this request",
            ));
        }
        // From the first attempted write onward, errors never authorize resending
        // an effect. The connection is poisoned on every incomplete roundtrip.
        self.poisoned = true;
        let result = tokio::time::timeout(CALL_LIMIT, async {
            write_header(&mut self.pipe, &RequestHeader {request_id:request_id.clone(),owner_incarnation:self.welcome.owner_incarnation.clone(),command,body_bytes:body.len() as u64,secret_bytes:secret.map_or(0, |secret| secret.bytes().len() as u32)}).await?;
            self.pipe.write_all(body).await?;
            if let Some(secret) = secret { self.pipe.write_all(secret.bytes()).await?; }
            self.pipe.flush().await?;
            let reply: ReplyHeader = read_header(&mut self.pipe).await?;
            if reply.request_id != request_id || reply.owner_incarnation != self.welcome.owner_incarnation
                || !(100..=599).contains(&reply.http_status) || reply.content_type.len() > 256 {
                return Err(invalid(weftext_messages::UiMessage::error_api_windows_the_owner_result_does_not_match_the_original_request()));
            }
            let body = read_body(&mut self.pipe, reply.body_bytes, MAX_REPLY_BYTES).await?;
            Ok(LocalReply {request_id:reply.request_id,owner_incarnation:reply.owner_incarnation,http_status:reply.http_status,content_type:reply.content_type,body})
        }).await.map_err(|_| LocalClientError::uncertain("The original call timed out; retain its business operation ID and query its original result"))?
            .map_err(|_| LocalClientError::uncertain("The original owner response was not completely received; no request was resent"))?;
        self.poisoned = false;
        Ok(result)
    }
}

pub(crate) async fn shutdown_existing(
    profile: &ProfileLocator,
    expected: &OwnerIdentity,
    limit: Duration,
) -> Result<ShutdownReceipt, LocalClientError> {
    let qualified = weftext_core::qualify_existing_profile(&profile.data_dir).map_err(|error| {
        LocalClientError::not_submitted("profileQualification", error.to_string())
    })?;
    if qualified.physical_identity() != expected.profile_identity {
        return Err(LocalClientError::not_submitted(
            "ownerChanged",
            "The original physical profile differs",
        ));
    }
    let sid = security::current_sid()
        .map_err(|error| LocalClientError::not_submitted("ownerIdentity", error.to_string()))?;
    let catalog = owner_catalog().map_err(|error| {
        LocalClientError::not_submitted("desktopRoleMissing", error.to_string())
    })?;
    let pipe = ClientOptions::new()
        .open(pipe_name(&qualified, &sid))
        .map_err(|error| LocalClientError::not_submitted("ownerUnavailable", error.to_string()))?;
    let mut connection = establish(pipe, qualified, catalog, &sid).await?;
    if connection.owner_identity() != *expected {
        return Err(LocalClientError::not_submitted(
            "ownerChanged",
            "The verified original owner process/incarnation differs",
        ));
    }
    let reply = connection.control(Command::Shutdown).await?;
    let value: serde_json::Value = serde_json::from_slice(&reply.body).map_err(|_| {
        LocalClientError::uncertain("The original shutdown acknowledgement is invalid")
    })?;
    if reply.http_status != 200 || value.get("accepted") != Some(&serde_json::Value::Bool(true)) {
        return Err(LocalClientError::not_submitted(
            "shutdownBlocked",
            "The original owner did not accept shutdown",
        ));
    }
    // Disconnect this lease before waiting: the owner must not wait for the
    // shutdown caller to vanish while that caller waits for process exit.
    let Connection {
        pipe,
        _profile,
        _peer,
        welcome: _,
        sent: _,
        poisoned: _,
    } = connection;
    drop(pipe);
    let exit_code = _peer.wait(limit).await.map_err(|_| {
        LocalClientError::uncertain(
            "Shutdown was accepted but the exact original process exit is not confirmed",
        )
    })?;
    drop(_profile);
    Ok(ShutdownReceipt {
        owner: expected.clone(),
        exit_code,
    })
}

pub(crate) async fn inspect_existing(
    profile: &ProfileLocator,
) -> Result<OwnerIdentity, LocalClientError> {
    let catalog = owner_catalog().map_err(|error| {
        LocalClientError::not_submitted("desktopRoleMissing", error.to_string())
    })?;
    let qualified = weftext_core::qualify_existing_profile(&profile.data_dir).map_err(|error| {
        LocalClientError::not_submitted("profileQualification", error.to_string())
    })?;
    let sid = security::current_sid()
        .map_err(|error| LocalClientError::not_submitted("ownerIdentity", error.to_string()))?;
    let pipe = ClientOptions::new()
        .open(pipe_name(&qualified, &sid))
        .map_err(|error| LocalClientError::not_submitted("ownerUnavailable", error.to_string()))?;
    let connection = establish(pipe, qualified, catalog, &sid).await?;
    Ok(connection.owner_identity())
}

/// Existing-owner activation never launches a fallback owner or opens Core.
pub(crate) async fn activate_existing(
    profile: &ProfileLocator,
    command: Command,
    limit: Duration,
) -> Result<(), LocalClientError> {
    if matches!(&command, Command::Business { .. } | Command::Shutdown) {
        return Err(LocalClientError::not_submitted(
            "invalidActivation",
            "Only a fixed desktop activation is accepted",
        ));
    }
    tokio::time::timeout(limit,async{
        let qualified=weftext_core::qualify_existing_profile(&profile.data_dir).map_err(|_|LocalClientError::not_submitted("profileQualification","The existing physical profile could not be qualified"))?;
        let mut connection=connect_qualified(qualified,limit,Start::Never).await?;
        let reply=connection.control(command).await?;
        let accepted=serde_json::from_slice::<serde_json::Value>(&reply.body).is_ok_and(|value|value.get("accepted")==Some(&serde_json::Value::Bool(true)));
        if reply.http_status==200&&accepted{Ok(())}else{Err(LocalClientError::uncertain("The original desktop activation was not confirmed"))}
    }).await.map_err(|_|LocalClientError::uncertain("The original existing-owner activation timed out; no new owner or repeat activation was attempted"))?
}

/// The only activation allowed to start a missing owner names no caller path:
/// the child independently requalifies this original opaque routing record.
pub(crate) async fn activate_original_toast(
    qualified: QualifiedProfile,
    opaque: &str,
    limit: Duration,
) -> Result<(), LocalClientError> {
    if !canonical_uuid(opaque) {
        return Err(LocalClientError::not_submitted(
            "invalidActivation",
            "Invalid original activation",
        ));
    }
    let mut connection = connect_qualified(qualified, limit, Start::OriginalToast(opaque)).await?;
    // No enclosing timeout drops a request after submission. The one common
    // control roundtrip retains its original unknown boundary and call budget.
    let reply = connection
        .control(Command::ToastActivation {
            opaque: opaque.into(),
        })
        .await?;
    if reply.http_status == 200
        && serde_json::from_slice::<serde_json::Value>(&reply.body)
            .is_ok_and(|value| value.get("accepted") == Some(&serde_json::Value::Bool(true)))
    {
        Ok(())
    } else {
        Err(LocalClientError::uncertain(
            "The original notification activation was not confirmed",
        ))
    }
}

pub async fn connect_or_start(
    profile: &ProfileLocator,
    limit: Duration,
) -> Result<Connection, LocalClientError> {
    let qualified = crate::host_paths::qualify_core(&profile.data_dir).map_err(|error| {
        LocalClientError::not_submitted("profileQualification", error.to_string())
    })?;
    connect_qualified(qualified, limit, Start::Service).await
}

#[derive(Clone, Copy)]
enum Start<'a> {
    Never,
    Service,
    OriginalToast(&'a str),
}
async fn connect_qualified(
    qualified: QualifiedProfile,
    limit: Duration,
    start: Start<'_>,
) -> Result<Connection, LocalClientError> {
    let sid = security::current_sid()
        .map_err(|error| LocalClientError::not_submitted("ownerIdentity", error.to_string()))?;
    let name = pipe_name(&qualified, &sid);
    if limit.is_zero() {
        return Err(LocalClientError::not_submitted(
            "ownerTimeout",
            "The owner connection requires a positive time budget",
        ));
    }
    let catalog = owner_catalog().map_err(|error| {
        LocalClientError::not_submitted("desktopRoleMissing", error.to_string())
    })?;
    let (executable, _desktop_image) = catalog.desktop_executable().map_err(|error| {
        LocalClientError::not_submitted("desktopRoleMissing", error.to_string())
    })?;
    let deadline = tokio::time::Instant::now() + limit.min(Duration::from_secs(60));
    let mut spawned = false;
    let mut launched = None;
    let pipe = loop {
        match ClientOptions::new().open(&name) {
            Ok(pipe) => break pipe,
            Err(error)
                if error.raw_os_error() == Some(2)
                    && !spawned
                    && !matches!(start, Start::Never) =>
            {
                let mut child = tokio::process::Command::new(&executable);
                match start {
                    Start::Service => {
                        child
                            .arg("--service-owner")
                            .arg("--data-dir")
                            .arg(qualified.path());
                    }
                    Start::OriginalToast(opaque) => {
                        child.arg("--toast-owner").arg(opaque);
                    }
                    Start::Never => unreachable!(),
                }
                child
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .kill_on_drop(false)
                    .creation_flags(0x08000000);
                let child = child.spawn().map_err(|error| {
                    LocalClientError::not_submitted("ownerLaunchFailed", error.to_string())
                })?;
                // This is the shared owner, not a child the caller may kill on
                // Drop. Core/pipe election still decides the single winner.
                launched = Some(child);
                spawned = true;
            }
            Err(error)
                if matches!(error.raw_os_error(), Some(2 | 231))
                    && tokio::time::Instant::now() < deadline => {}
            Err(error) => {
                return Err(LocalClientError::not_submitted(
                    "ownerUnavailable",
                    error.to_string(),
                ));
            }
        }
        if let Some(child) = &mut launched
            && child
                .try_wait()
                .map_err(|_| {
                    LocalClientError::not_submitted(
                        "ownerStartingUnknown",
                        "The one launch process state could not be checked",
                    )
                })?
                .is_some()
        {
            return Err(LocalClientError::not_submitted(
                "ownerLaunchExited",
                "The one owner launch exited before a qualified connection; no repeat launch was sent",
            ));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(LocalClientError::not_submitted(
                "ownerStartingUnknown",
                "The one owner launch was not confirmed; no second launch or Core was attempted",
            ));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let connection = establish(pipe, qualified, catalog, &sid).await?;
    if connection.welcome.owner_role != OwnerRole::Local {
        return Err(LocalClientError::not_submitted(
            "profileHasHostedOwner",
            "This physical profile is owned by the hosted role; no local business call was sent",
        ));
    }
    Ok(connection)
}

async fn establish(
    mut pipe: NamedPipeClient,
    profile: QualifiedProfile,
    executables: &'static crate::artifact_catalog::Catalog,
    sid: &str,
) -> Result<Connection, LocalClientError> {
    let observed = security::inspect_server(&pipe, sid, executables).map_err(|error| {
        LocalClientError::not_submitted("ownerIdentityMismatch", error.to_string())
    })?;
    let expected = HelloPeer {
        profile: profile.physical_identity().into(),
        pid: observed.pid,
        created: observed.created,
    };
    // Only identification crosses the pipe before full product qualification.
    // Hashing a large, retained executable cannot consume the peer's Hello
    // deadline, and it never blocks Tokio's I/O thread.
    let (welcome, peer) = identify_then_qualify(
        &mut pipe,
        expected,
        (HANDSHAKE_LIMIT, QUALIFICATION_LIMIT),
        move || async move {
            crate::work::spawn_blocking(move || observed.qualify(executables))
                .await
                .map_err(io::Error::other)?
        },
    )
    .await?;
    Ok(Connection {
        pipe,
        _profile: profile,
        _peer: peer,
        welcome,
        sent: HashSet::new(),
        poisoned: false,
    })
}

struct HelloPeer {
    profile: String,
    pid: u32,
    created: u64,
}

/// Shared framing and admission boundary. The qualification closure has no
/// transport access; callers receive no usable connection until it succeeds.
async fn identify_then_qualify<S, P, Q, F>(
    pipe: &mut S,
    peer: HelloPeer,
    limits: (Duration, Duration),
    qualify: Q,
) -> Result<(Welcome, P), LocalClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    Q: FnOnce() -> F,
    F: std::future::Future<Output = io::Result<P>>,
{
    let challenge = uuid::Uuid::new_v4().to_string();
    let welcome = tokio::time::timeout(limits.0, async {
        write_header(pipe, &Hello {
            version: VERSION, profile_identity: peer.profile.clone(), challenge: challenge.clone(),
        }).await?;
        pipe.flush().await?;
        let welcome: Welcome = read_header(pipe).await?;
        if welcome.version != VERSION || welcome.profile_identity != peer.profile
            || welcome.challenge != challenge || welcome.owner_pid != peer.pid
            || welcome.owner_created != peer.created || !canonical_uuid(&welcome.owner_incarnation)
        {
            return Err(invalid(weftext_messages::UiMessage::error_api_windows_the_verified_owner_handshake_does_not_match_this_physical_profile_and()));
        }
        Ok(welcome)
    }).await.map_err(|_| LocalClientError::not_submitted("ownerHandshakeUnknown", "Owner readiness was not confirmed; no business request was sent"))?
      .map_err(|_| LocalClientError::not_submitted("ownerHandshakeRejected", "The owner handshake was rejected; no business request was sent"))?;
    let qualified = tokio::time::timeout(limits.1, qualify()).await
        .map_err(|_| LocalClientError::not_submitted("ownerIdentityUnknown", "Original owner executable qualification did not finish; no business request was sent"))?
        .map_err(|error| LocalClientError::not_submitted("ownerIdentityMismatch", error.to_string()))?;
    Ok((welcome, qualified))
}

pub(crate) struct OwnerPipe {
    first: NamedPipeServer,
    profile: QualifiedProfile,
    sid: String,
    name: String,
    incarnation: String,
    created: u64,
    role: OwnerRole,
}

impl OwnerPipe {
    /// Must succeed before LocalService/Core is opened. Never remove the first
    /// flag or change the name after an occupied/unknown election result.
    pub(crate) fn acquire(profile: QualifiedProfile, role: OwnerRole) -> io::Result<Self> {
        let sid = security::current_sid()?;
        let name = pipe_name(&profile, &sid);
        let first = security::create_server(&name, &sid, true)?;
        Ok(Self {
            first,
            profile,
            sid,
            name,
            incarnation: uuid::Uuid::new_v4().to_string(),
            created: security::self_created()?,
            role,
        })
    }

    pub(crate) fn identity(&self) -> OwnerIdentity {
        OwnerIdentity {
            pid: std::process::id(),
            created: self.created,
            incarnation: self.incarnation.clone(),
            profile_identity: self.profile.physical_identity().into(),
            role: self.role,
        }
    }

    /// Keep this first-instance object alive independently of listener/task
    /// retirement, until its supervisor has actually released Core. During
    /// drain another launch sees occupied/busy, never an absent owner name.
    pub(crate) fn retain_name(&self) -> io::Result<OwnedHandle> {
        use ::windows::Win32::{
            Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE},
            System::Threading::GetCurrentProcess,
        };
        let mut duplicate = HANDLE::default();
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(self.first.as_raw_handle()),
                GetCurrentProcess(),
                &mut duplicate,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(io::Error::other)?;
        Ok(unsafe { OwnedHandle::from_raw_handle(duplicate.0) })
    }

    pub(crate) async fn serve(
        self,
        browser_url: Option<String>,
        requests: mpsc::Sender<HostRequest>,
        leases: Arc<AtomicUsize>,
        continuation: Arc<super::lifetime::ServiceContinuation>,
        mut stopping: watch::Receiver<bool>,
    ) -> io::Result<()> {
        let bridge = match (self.role, browser_url) {
            (OwnerRole::Local, Some(browser_url)) => {
                let parsed = url::Url::parse(&browser_url).map_err(io::Error::other)?;
                let endpoint = format!(
                    "http://127.0.0.1:{}",
                    parsed
                        .port()
                        .ok_or_else(|| invalid(weftext_messages::UiMessage::error_api_windows_local_listener_has_no_port()))?
                );
                let token = parsed
                    .fragment()
                    .and_then(|fragment| fragment.strip_prefix("token="))
                    .ok_or_else(|| invalid(weftext_messages::UiMessage::error_api_windows_local_owner_credential_is_unavailable()))?
                    .to_owned();
                let http = reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .retry(reqwest::retry::never())
                    .build()
                    .map_err(io::Error::other)?;
                Some(LocalBridge {
                    endpoint,
                    token,
                    http,
                })
            }
            (OwnerRole::Hosted, None) => None,
            _ => return Err(invalid(weftext_messages::UiMessage::error_api_windows_owner_role_and_local_service_authority_differ())),
        };
        let authority = Arc::new(Authority {
            profile: self.profile.physical_identity().into(),
            incarnation: self.incarnation,
            created: self.created,
            sid: self.sid,
            bridge,
            requests,
            role: self.role,
            continuation,
        });
        let capacity = Arc::new(Semaphore::new(4));
        let mut tasks = JoinSet::new();
        let mut listener = self.first;
        let (close_connections, connections_closing) = watch::channel(false);
        let outcome = loop {
            if *stopping.borrow() {
                break Ok(());
            }
            tokio::select! {
                biased;
                result = stopping.changed() => { if result.is_err() || *stopping.borrow() { break Ok(()); } }
                Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                    match result { Ok(Ok(()))=>{},Ok(Err(error))=>break Err(error),Err(error)=>break Err(io::Error::other(error)) }
                }
                result = listener.connect() => {
                    if let Err(error) = result { break Err(error); }
                    // Keep the connected original alive until its replacement
                    // already owns the name; there is never a last-handle gap.
                    let next = match security::create_server(&self.name, &authority.sid, false) {Ok(next)=>next,Err(error)=>break Err(error)};
                    let connected = std::mem::replace(&mut listener, next);
                    let Ok(permit) = capacity.clone().try_acquire_owned() else { drop(connected); continue; };
                    let authority = authority.clone();
                    let leases = leases.clone();
                    let closing = connections_closing.clone();
                    let stopping = stopping.clone();
                    tasks.spawn(async move { let _permit = permit; connection(connected,authority,leases,closing,stopping).await });
                }
            }
        };
        // Close idle sessions, but never abort a task carrying an already
        // accepted request. Even an owner-loop fault converges on real joins.
        let _ = close_connections.send(true);
        drop(listener);
        let mut task_error = None;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    task_error.get_or_insert(error);
                }
                Err(error) => {
                    task_error.get_or_insert(io::Error::other(error));
                }
            }
        }
        drop(authority);
        drop(self.profile);
        outcome.and(task_error.map_or(Ok(()), Err))
    }
}

struct Authority {
    profile: String,
    incarnation: String,
    created: u64,
    sid: String,
    role: OwnerRole,
    bridge: Option<LocalBridge>,
    requests: mpsc::Sender<HostRequest>,
    continuation: Arc<super::lifetime::ServiceContinuation>,
}
struct LocalBridge {
    endpoint: String,
    token: String,
    http: reqwest::Client,
}

struct Lease(Arc<AtomicUsize>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

async fn connection(
    mut pipe: NamedPipeServer,
    owner: Arc<Authority>,
    leases: Arc<AtomicUsize>,
    mut stopping: watch::Receiver<bool>,
    mut global_stopping: watch::Receiver<bool>,
) -> io::Result<()> {
    let hello =
        match tokio::time::timeout(HANDSHAKE_LIMIT, read_header::<_, Hello>(&mut pipe)).await {
            Ok(Ok(hello)) => hello,
            _ => return Ok(()),
        };
    if hello.version != VERSION
        || hello.profile_identity != owner.profile
        || !canonical_uuid(&hello.challenge)
    {
        return Ok(());
    }
    // Read identification context before examining or allocating business data.
    match security::authenticate_client(&pipe, owner.sid.clone()).await {
        Ok(()) => {}
        Err(security::AuthenticationError::Rejected) => return Ok(()),
        Err(security::AuthenticationError::Fatal(error)) => return Err(error),
    }
    let welcome = Welcome {
        version: VERSION,
        profile_identity: owner.profile.clone(),
        challenge: hello.challenge,
        owner_incarnation: owner.incarnation.clone(),
        owner_pid: std::process::id(),
        owner_created: owner.created,
        owner_role: owner.role,
    };
    if !matches!(
        tokio::time::timeout(HANDSHAKE_LIMIT, async {
            write_header(&mut pipe, &welcome).await?;
            pipe.flush().await
        })
        .await,
        Ok(Ok(()))
    ) {
        return Ok(());
    }
    leases.fetch_add(1, Ordering::AcqRel);
    let _lease = Lease(leases);
    let mut seen = HashSet::new();
    loop {
        if *stopping.borrow() || *global_stopping.borrow() {
            return Ok(());
        }
        let header = tokio::select! {
            biased;
            _ = global_stopping.changed() => return Ok(()),
            result = stopping.changed() => { if result.is_err() || *stopping.borrow() { return Ok(()); } else {continue;} }
            header = tokio::time::timeout(CALL_LIMIT,read_header::<_,RequestHeader>(&mut pipe)) => match header {Ok(Ok(header))=>header,_=>return Ok(())},
        };
        if header.owner_incarnation != owner.incarnation
            || !canonical_uuid(&header.request_id)
            || seen.len() >= 4096
            || !seen.insert(header.request_id.clone())
            || header.body_bytes > MAX_WIRE_BYTES as u64
            || header.secret_bytes > 512
        {
            return Ok(());
        }
        let incoming = tokio::time::timeout(CALL_LIMIT, async {
            let body = read_body(&mut pipe, header.body_bytes, MAX_WIRE_BYTES).await?;
            let secret = if header.secret_bytes == 0 {
                None
            } else {
                Some(RemoteSyncSecret::receive(&mut pipe, header.secret_bytes).await?)
            };
            Ok::<_, io::Error>((body, secret))
        });
        let (body, secret) = tokio::select! {
            biased;
            _ = global_stopping.changed() => return Ok(()),
            _ = stopping.changed() => return Ok(()),
            value = incoming => match value {Ok(Ok(value))=>value,_=>return Ok(())},
        };
        // Full framing precedes admission. Cancellation of an incomplete frame
        // cannot become a late new effect after closing began.
        if *stopping.borrow() || *global_stopping.borrow() {
            return Ok(());
        }
        // This read excludes known shutdown. For business calls, dispatch then
        // atomically admits service continuation after fixed kind/secret checks,
        // linearizing it against a last-window idle shutdown. The accepted
        // connection is already retained by the owner's JoinSet; a later stop
        // must join its dispatch, rather than aborting it or relying on the
        // asynchronously relayed idle-connection cancellation signal.
        // Once a complete request is accepted, do not select cancellation over
        // its result. HTTP/Core and durable bookkeeping must finish independently
        // of a later pipe disconnect or host shutdown signal.
        let (status, content_type, output) =
            match dispatch(&owner, header.command, body, secret).await {
                Ok(value) => value,
                Err(error) => {
                    let error = crate::ApiError::from(
                        weftext_core::CoreError::new(
                            "ownerResultUnknown",
                            weftext_messages::UiMessage::error_api_broker_original_result_unknown(),
                        )
                        .caused_by(weftext_core::CoreError::from(error)),
                    );
                    (
                        500,
                        "application/json".into(),
                        serde_json::to_vec(&serde_json::json!({"error": error}))
                            .map_err(io::Error::other)?,
                    )
                }
            };
        if output.len() > MAX_REPLY_BYTES {
            return Err(invalid(weftext_messages::UiMessage::error_api_windows_host_result_exceeds_the_fixed_output_budget()));
        }
        let reply = ReplyHeader {
            request_id: header.request_id,
            owner_incarnation: owner.incarnation.clone(),
            http_status: status,
            content_type,
            body_bytes: output.len() as u64,
        };
        if !matches!(
            tokio::time::timeout(CALL_LIMIT, async {
                write_header(&mut pipe, &reply).await?;
                pipe.write_all(&output).await?;
                pipe.flush().await
            })
            .await,
            Ok(Ok(()))
        ) {
            return Ok(());
        }
        if *stopping.borrow() {
            return Ok(());
        }
    }
}

async fn dispatch(
    owner: &Authority,
    command: Command,
    body: Vec<u8>,
    secret: Option<RemoteSyncSecret>,
) -> io::Result<(u16, String, Vec<u8>)> {
    match command {
        Command::Business { kind } => {
            if owner.role != OwnerRole::Local {
                return Err(invalid(weftext_messages::UiMessage::error_api_windows_hosted_owner_has_no_local_business_bridge()));
            }
            let bridge = owner.bridge.as_ref().ok_or_else(|| {
                invalid(
                    weftext_messages::UiMessage::error_api_windows_local_authority_is_unavailable(),
                )
            })?;
            let (method, route) = kind.local_route().ok_or_else(|| {
                invalid(weftext_messages::UiMessage::error_api_windows_unknown_local_operation())
            })?;
            let body = inject_secret(kind, body, secret)?;
            if !owner.continuation.admit_cli() {
                return Ok((
                    503,
                    "application/json".into(),
                    br#"{"error":{"code":"hostClosing","message":"The owner is closing; this business call was not admitted"}}"#.to_vec(),
                ));
            }
            let method = match method {
                LocalMethod::Get => reqwest::Method::GET,
                LocalMethod::Post => reqwest::Method::POST,
            };
            let mut response = bridge
                .http
                .request(method, format!("{}{route}", bridge.endpoint))
                .header("X-Weftext-Token", &bridge.token)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await
                .map_err(io::Error::other)?;
            let status = response.status().as_u16();
            let mime = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_owned();
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(io::Error::other)? {
                if chunk.len() > MAX_REPLY_BYTES.saturating_sub(bytes.len()) {
                    return Err(invalid(weftext_messages::UiMessage::error_api_windows_original_reply_exceeds_the_fixed_output_budget()));
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok((status, mime, bytes))
        }
        command => {
            if owner.continuation.is_closing() {
                return Ok((
                    503,
                    "application/json".into(),
                    br#"{"error":{"code":"hostClosing","message":"The owner is closing; this control was not admitted"}}"#.to_vec(),
                ));
            }
            if owner.role == OwnerRole::Hosted && !matches!(&command, Command::Shutdown) {
                return Err(invalid(
                    weftext_messages::UiMessage::error_api_windows_hosted_owner_accepts_only_its_exact_lifecycle_control(),
                ));
            }
            if !body.is_empty() || secret.is_some() {
                return Err(invalid(
                    weftext_messages::UiMessage::error_api_windows_host_control_does_not_accept_business_or_secret_bytes(),
                ));
            }
            let (reply, result) = oneshot::channel();
            owner
                .requests
                .send(HostRequest { command, reply })
                .await
                .map_err(|_| io::Error::other("Native coordinator is unavailable"))?;
            let body = result.await.map_err(|_| {
                io::Error::other("Native coordinator did not confirm the original command")
            })??;
            Ok((200, "application/json".into(), body))
        }
    }
}

fn inject_secret(
    kind: CallKind,
    body: Vec<u8>,
    secret: Option<RemoteSyncSecret>,
) -> io::Result<Vec<u8>> {
    if !matches!(
        kind,
        CallKind::RemoteSyncInspect | CallKind::RemoteSyncStart
    ) {
        if secret.is_some() {
            return Err(invalid(weftext_messages::UiMessage::error_api_windows_secret_sideband_is_forbidden_for_this_operation()));
        }
        return Ok(body);
    }
    let secret = secret.ok_or_else(|| {
        invalid(
            weftext_messages::UiMessage::error_api_windows_transient_sync_credential_is_missing(),
        )
    })?;
    let mut value: serde_json::Value = serde_json::from_slice(&body).map_err(|_| {
        invalid(weftext_messages::UiMessage::error_api_windows_sync_pure_input_is_invalid())
    })?;
    let connection = if kind == CallKind::RemoteSyncStart {
        value.get_mut("connection")
    } else {
        Some(&mut value)
    }
    .and_then(serde_json::Value::as_object_mut)
    .ok_or_else(|| {
        invalid(
            weftext_messages::UiMessage::error_api_windows_sync_connection_pure_input_is_missing(),
        )
    })?;
    if connection.contains_key("token") {
        return Err(invalid(weftext_messages::UiMessage::error_api_windows_sync_pure_input_cannot_contain_credentials()));
    }
    connection.insert(
        "token".into(),
        serde_json::Value::String(
            std::str::from_utf8(secret.bytes())
                .map_err(|_| {
                    invalid(
                        weftext_messages::UiMessage::error_api_windows_credential_is_not_a_s_c_i_i(
                        ),
                    )
                })?
                .into(),
        ),
    );
    serde_json::to_vec(&value).map_err(io::Error::other)
}

#[cfg(test)]
mod continuation_tests {
    use super::*;

    fn local_authority() -> Authority {
        let (requests, _received) = mpsc::channel(1);
        Authority {
            profile: "unit-profile-not-an-OS-qualification".into(),
            incarnation: uuid::Uuid::new_v4().to_string(),
            created: 0,
            sid: String::new(),
            role: OwnerRole::Local,
            bridge: Some(LocalBridge {
                endpoint: "http://127.0.0.1:1".into(),
                token: String::new(),
                http: reqwest::Client::builder().no_proxy().build().unwrap(),
            }),
            requests,
            continuation: Arc::new(super::super::lifetime::ServiceContinuation::new(false)),
        }
    }

    #[tokio::test]
    async fn wrong_role_unknown_route_and_invalid_secret_do_not_admit_cli() {
        let mut owner = local_authority();
        owner.role = OwnerRole::Hosted;
        assert!(
            dispatch(
                &owner,
                Command::Business {
                    kind: CallKind::Capabilities
                },
                vec![],
                None
            )
            .await
            .is_err()
        );
        assert!(!owner.continuation.cli_active());
        owner.role = OwnerRole::Local;
        assert!(
            dispatch(
                &owner,
                Command::Business {
                    kind: CallKind::Documents
                },
                vec![],
                None
            )
            .await
            .is_err()
        );
        assert!(!owner.continuation.cli_active());
        assert!(
            dispatch(
                &owner,
                Command::Business {
                    kind: CallKind::RemoteSyncStart
                },
                br#"{"connection":{}}"#.to_vec(),
                None
            )
            .await
            .is_err()
        );
        assert!(!owner.continuation.cli_active());
        // No request above reaches the HTTP client or creates a Core/listener.
    }

    #[tokio::test]
    async fn idle_exit_winner_rejects_complete_business_without_http_dispatch() {
        let owner = local_authority();
        assert!(owner.continuation.end_unclaimed_bootstrap());
        let (status, _, body) = dispatch(
            &owner,
            Command::Business {
                kind: CallKind::Capabilities,
            },
            vec![],
            None,
        )
        .await
        .unwrap();
        assert_eq!(status, 503);
        assert!(String::from_utf8(body).unwrap().contains("hostClosing"));
        assert!(!owner.continuation.cli_active());
    }
    #[tokio::test]
    async fn expired_unclaimed_bootstrap_does_not_deliver_late_surface_control() {
        let (requests, mut received) = mpsc::channel(1);
        let mut owner = local_authority();
        owner.requests = requests;
        assert!(owner.continuation.end_unclaimed_bootstrap());
        for command in [
            Command::ShowLibrary { document: None },
            Command::ShowCapture,
            Command::LoginStartup,
        ] {
            let (status, _, _) = dispatch(&owner, command, vec![], None).await.unwrap();
            assert_eq!(status, 503);
            assert!(received.try_recv().is_err());
        }
        assert!(!owner.continuation.has_takeover());
    }
}

#[cfg(test)]
mod handshake_tests {
    use super::*;
    use crate::local_client::SubmissionEffect;
    fn expected() -> HelloPeer {
        HelloPeer {
            profile: "fixture-physical-profile".into(),
            pid: 17,
            created: 9,
        }
    }
    fn welcome(hello: Hello) -> Welcome {
        Welcome {
            version: VERSION,
            profile_identity: hello.profile_identity,
            challenge: hello.challenge,
            owner_incarnation: uuid::Uuid::new_v4().to_string(),
            owner_pid: 17,
            owner_created: 9,
            owner_role: OwnerRole::Local,
        }
    }
    #[tokio::test]
    async fn slow_image_qualification_never_occupies_hello_budget_or_sends_early_control() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let (started, waiting) = oneshot::channel();
        let (release, released) = oneshot::channel();
        let task = tokio::spawn(async move {
            let (welcome, ()) = identify_then_qualify(
                &mut client,
                expected(),
                (Duration::from_millis(250), Duration::from_secs(5)),
                || async {
                    started.send(()).unwrap();
                    released.await.unwrap();
                    Ok(())
                },
            )
            .await
            .unwrap();
            write_header(
                &mut client,
                &RequestHeader {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    owner_incarnation: welcome.owner_incarnation,
                    command: Command::Shutdown,
                    body_bytes: 0,
                    secret_bytes: 0,
                },
            )
            .await
            .unwrap();
        });
        let hello: Hello = read_header(&mut server).await.unwrap();
        write_header(&mut server, &welcome(hello)).await.unwrap();
        waiting.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(500), server.read_u8())
                .await
                .is_err()
        );
        release.send(()).unwrap();
        let request: RequestHeader = read_header(&mut server).await.unwrap();
        assert!(matches!(request.command, Command::Shutdown));
        assert_eq!(request.body_bytes, 0);
        assert_eq!(request.secret_bytes, 0);
        task.await.unwrap();
        let mut tail = Vec::new();
        server.read_to_end(&mut tail).await.unwrap();
        assert!(tail.is_empty());
    }
    #[tokio::test]
    async fn failed_or_expired_qualification_never_sends_any_business_bytes() {
        for expires in [false, true] {
            let (mut client, mut server) = tokio::io::duplex(4096);
            let task = tokio::spawn(async move {
                identify_then_qualify(
                    &mut client,
                    expected(),
                    (Duration::from_secs(2), Duration::from_millis(20)),
                    || async {
                        if expires {
                            std::future::pending::<()>().await;
                        }
                        Err::<(), _>(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "fixture image rejection",
                        ))
                    },
                )
                .await
                .err()
                .unwrap()
            });
            let hello: Hello = read_header(&mut server).await.unwrap();
            write_header(&mut server, &welcome(hello)).await.unwrap();
            let error = task.await.unwrap();
            assert_eq!(error.effect, SubmissionEffect::NotSubmitted);
            assert_eq!(
                error.code,
                if expires {
                    "ownerIdentityUnknown"
                } else {
                    "ownerIdentityMismatch"
                }
            );
            let mut bytes = Vec::new();
            server.read_to_end(&mut bytes).await.unwrap();
            assert!(bytes.is_empty());
        }
    }
    #[tokio::test]
    async fn untrusted_welcome_cannot_replace_original_os_tuple_or_start_qualification() {
        for field in [
            "version",
            "profile",
            "challenge",
            "pid",
            "created",
            "incarnation",
        ] {
            let (mut client, mut server) = tokio::io::duplex(4096);
            let task = tokio::spawn(async move {
                identify_then_qualify(
                    &mut client,
                    expected(),
                    (Duration::from_secs(2), Duration::from_secs(2)),
                    || async {
                        Err::<(), _>(io::Error::other(
                            "invalid Welcome must never enter qualification",
                        ))
                    },
                )
                .await
                .err()
                .unwrap()
            });
            let hello: Hello = read_header(&mut server).await.unwrap();
            let mut reply = welcome(hello);
            match field {
                "version" => reply.version += 1,
                "profile" => reply.profile_identity.push('x'),
                "challenge" => reply.challenge = uuid::Uuid::new_v4().to_string(),
                "pid" => reply.owner_pid += 1,
                "created" => reply.owner_created += 1,
                "incarnation" => reply.owner_incarnation = "not-an-original-incarnation".into(),
                _ => unreachable!(),
            }
            write_header(&mut server, &reply).await.unwrap();
            let error = task.await.unwrap();
            assert_eq!(error.code, "ownerHandshakeRejected");
            assert_eq!(error.effect, SubmissionEffect::NotSubmitted);
            let mut bytes = Vec::new();
            server.read_to_end(&mut bytes).await.unwrap();
            assert!(bytes.is_empty());
        }
    }
}
