use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use weftext_backup::{
    SERVER_CONTROL_PLANE_DATABASE_FILE, SERVER_CONTROL_PLANE_LEASE_FILE,
    acquire_server_control_plane_lease,
};
use weftext_core::{
    DocumentEdit, NodeId, TRASH_ITEM_MANIFEST_FILE_NAME, commit_document_edit,
    commit_workspace_transaction, create_child_node, create_workspace, plan_create_child_node,
    plan_document_edit, plan_trash_node_at, prepare_workspace_transaction_recovery_fixture,
    read_node_document, read_workspace_revision, scan_workspace,
};
use weftext_server::{HttpSecurityConfig, ServerConfig, ServerState, StartupError, app};

const OWNER_PASSWORD: &str = "correct horse battery staple";

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    root_id: NodeId,
    child_id: NodeId,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temporary workspace parent");
        let root = temp.path().join("Workspace");
        let created = create_workspace(&root).expect("create workspace");
        let child_plan = plan_create_child_node(&root, created.id, "Child").expect("plan child");
        let child = child_plan.generated_node_ids[0];
        commit_workspace_transaction(&child_plan).expect("commit child");
        replace_document(&root.join("Child"), |source| {
            format!(
                "{}\nNeedle body",
                source.replacen("weftext:\n", "weftext:\n  icon: \"weftext:star\"\n", 1,)
            )
        });
        Self {
            _temp: temp,
            root,
            root_id: created.id,
            child_id: child,
        }
    }
}

struct CitationFixture {
    _temp: TempDir,
    root: PathBuf,
    root_id: NodeId,
    component_id: NodeId,
}

struct TaskFixture {
    _temp: TempDir,
    root: PathBuf,
    root_id: NodeId,
    child_id: NodeId,
}

impl TaskFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("task fixture parent");
        let root = temporary.path().join("Tasks");
        let root_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1"
            .parse()
            .expect("root UUID");
        let child_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2"
            .parse()
            .expect("child UUID");
        std::fs::create_dir_all(root.join("Child")).expect("child directory");
        std::fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n")
            .expect("format marker");
        std::fs::write(
            root.join("Tasks.adoc"),
            format!(
                concat!(
                    "---\nweftext:\n  id: \"{}\"\n---\n= Tasks\n\n",
                    "* [ ] Editable task:[id=11111111-1111-4111-8111-111111111111]\n",
                    "* [ ] Repeat task:[id=33333333-3333-4333-8333-333333333333,due=2026-08-24,rrule=\"FREQ=DAILY;COUNT=2\",repeat-from=due]\n"
                ),
                root_id
            ),
        )
        .expect("root task source");
        std::fs::write(
            root.join("Child/Child.adoc"),
            format!(
                "---\nweftext:\n  id: \"{child_id}\"\n---\n= Child\n\n* [ ] Dependency task:[id=22222222-2222-4222-8222-222222222222]\n"
            ),
        )
        .expect("child task source");
        Self {
            _temp: temporary,
            root,
            root_id,
            child_id,
        }
    }
}

impl CitationFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("citation fixture parent");
        let root = temporary.path().join("Citations");
        let root_id = "11111111-1111-4111-8111-111111111111"
            .parse()
            .expect("root UUID");
        let component_id = "22222222-2222-4222-8222-222222222222"
            .parse()
            .expect("component UUID");
        std::fs::create_dir_all(root.join("Component")).expect("component directory");
        std::fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n")
            .expect("format marker");
        std::fs::write(
            root.join("Citations.adoc"),
            format!("---\nweftext:\n  id: \"{root_id}\"\n---\n= Citations\n"),
        )
        .expect("root source");
        std::fs::write(
            root.join("Component/Component.adoc"),
            format!(
                "---\nweftext:\n  id: \"{component_id}\"\n---\n= Component\n\nEvidence without a resolved reference.\n"
            ),
        )
        .expect("component source");
        Self {
            _temp: temporary,
            root,
            root_id,
            component_id,
        }
    }
}

fn replace_document(node: &Path, update: impl FnOnce(&str) -> String) {
    let snapshot = read_node_document(node).expect("read fixture document");
    let source = update(&snapshot.source);
    let plan = plan_document_edit(
        node,
        &snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: u64::try_from(snapshot.source.len()).expect("source length"),
            replacement: source,
        }],
    )
    .expect("plan fixture edit");
    commit_document_edit(&plan).expect("commit fixture edit");
}

fn convert_node_to_legacy_trash(root: &Path, node_id: NodeId, name: &str) {
    let plan = plan_trash_node_at(root, node_id, "2026-08-24T11:00:00+08:00")
        .expect("item-backed Trash setup plan");
    commit_workspace_transaction(&plan).expect("item-backed Trash setup commit");
    let item = scan_workspace(root)
        .trash_items
        .into_iter()
        .next()
        .expect("one item-backed Trash entry");
    std::fs::rename(&item.payload_path, root.join(".weftext-trash").join(name))
        .expect("simulate historical direct Trash entry");
    std::fs::remove_dir_all(
        root.join(".weftext-trash")
            .join(weftext_core::TRASH_ITEMS_DIRECTORY_NAME),
    )
    .expect("remove item store authority");
}

#[test]
fn server_startup_requires_exact_asciidoc_marker_and_canonical_envelope() {
    for (case, marker) in [
        ("missing", None),
        ("legacy", Some("weftext.markdown.v1\n")),
        ("crlf", Some("weftext.asciidoc.v1\r\n")),
        ("trailing", Some("weftext.asciidoc.v1\n\n")),
    ] {
        let temporary = tempfile::tempdir().expect("startup fixture parent");
        let root = temporary.path().join("Workspace");
        create_workspace(&root).expect("create canonical workspace");
        let marker_path = root.join(".weftext-format");
        if let Some(marker) = marker {
            std::fs::write(&marker_path, marker).expect("replace marker");
        } else {
            std::fs::remove_file(&marker_path).expect("remove marker");
        }
        let control = temporary.path().join("control");
        let result = ServerState::open(
            &root,
            ServerConfig::new(
                control,
                HttpSecurityConfig::loopback("127.0.0.1:8787".parse().unwrap()),
            ),
        );
        assert!(
            matches!(result, Err(StartupError::InvalidWorkspace(_))),
            "case {case} must fail closed"
        );
    }

    let temporary = tempfile::tempdir().expect("legacy envelope fixture parent");
    let root = temporary.path().join("Workspace");
    let created = create_workspace(&root).expect("create canonical workspace");
    let source = std::fs::read_to_string(&created.document_path).expect("read root source");
    std::fs::write(
        &created.document_path,
        source.replacen("weftext:", "_weftext:", 1),
    )
    .expect("write legacy envelope");
    let result = ServerState::open(
        &root,
        ServerConfig::new(
            temporary.path().join("control"),
            HttpSecurityConfig::loopback("127.0.0.1:8787".parse().unwrap()),
        ),
    );
    assert!(matches!(result, Err(StartupError::InvalidWorkspace(_))));

    let temporary = tempfile::tempdir().expect("Markdown-only fixture parent");
    let root = temporary.path().join("Workspace");
    let created = create_workspace(&root).expect("create canonical workspace");
    std::fs::rename(&created.document_path, root.join("Workspace.md"))
        .expect("replace managed document with Markdown");
    let result = ServerState::open(
        &root,
        ServerConfig::new(
            temporary.path().join("control"),
            HttpSecurityConfig::loopback("127.0.0.1:8787".parse().unwrap()),
        ),
    );
    assert!(matches!(result, Err(StartupError::InvalidWorkspace(_))));
}

#[test]
fn server_state_rejects_non_loopback_http_config_even_without_main_binary() {
    let fixture = Fixture::new();
    let control = fixture
        .root
        .parent()
        .expect("workspace parent")
        .join("non-loopback-control");
    let result = ServerState::open(
        &fixture.root,
        ServerConfig::new(
            control,
            HttpSecurityConfig::loopback("0.0.0.0:8787".parse().expect("address")),
        ),
    );
    assert!(matches!(
        result,
        Err(StartupError::AuthenticationRequiredForNonLoopback(_))
    ));
}

#[test]
fn server_control_plane_lease_precedes_sqlite_and_lives_until_every_state_clone_drops() {
    let fixture = Fixture::new();
    let control = fixture
        .root
        .parent()
        .expect("workspace parent")
        .join("leased-control");
    std::fs::create_dir(&control).expect("create control root");
    let external_lease =
        acquire_server_control_plane_lease(&control).expect("hold external control-plane lease");
    let canonical_control = std::fs::canonicalize(&control).expect("canonical control root");
    let lease_path = canonical_control.join(SERVER_CONTROL_PLANE_LEASE_FILE);
    let config = || {
        ServerConfig::new(
            &control,
            HttpSecurityConfig::loopback("127.0.0.1:8787".parse().expect("address")),
        )
    };

    let blocked_before_sqlite = ServerState::open(&fixture.root, config());
    assert!(matches!(
        blocked_before_sqlite,
        Err(StartupError::ControlPlaneInUse(path)) if path == lease_path
    ));
    assert!(
        !control.join(SERVER_CONTROL_PLANE_DATABASE_FILE).exists(),
        "the Server must acquire its lease before opening SQLite"
    );
    drop(external_lease);

    let state = ServerState::open(&fixture.root, config()).expect("first Server state");
    assert!(control.join(SERVER_CONTROL_PLANE_DATABASE_FILE).exists());
    let clone = state.clone();
    let blocked_by_both = ServerState::open(&fixture.root, config());
    assert!(matches!(
        blocked_by_both,
        Err(StartupError::ControlPlaneInUse(path)) if path == lease_path
    ));

    drop(state);
    let blocked_by_clone = ServerState::open(&fixture.root, config());
    assert!(matches!(
        blocked_by_clone,
        Err(StartupError::ControlPlaneInUse(path)) if path == lease_path
    ));

    drop(clone);
    let reopened = ServerState::open(&fixture.root, config())
        .expect("all clone drops release the control-plane lease");
    drop(reopened);
}

#[test]
fn same_host_proxy_configuration_is_exact_https_and_still_loopback_only() {
    let address = "127.0.0.1:8787".parse().expect("loopback address");
    assert!(
        HttpSecurityConfig::same_host_reverse_proxy(address, "https://notes.example.test").is_ok()
    );
    for origin in [
        "http://notes.example.test",
        "https://Notes.example.test",
        "https://notes.example.test/",
        "https://user@notes.example.test",
        "https://notes.example.test/path",
        "https://notes.example.test?query",
        "https://notes.example.test#fragment",
        "https://notes.example.test:0",
    ] {
        assert!(
            HttpSecurityConfig::same_host_reverse_proxy(address, origin).is_err(),
            "origin {origin} must fail closed"
        );
    }
    assert!(
        HttpSecurityConfig::same_host_reverse_proxy(
            "0.0.0.0:8787".parse().expect("wildcard address"),
            "https://notes.example.test",
        )
        .is_err()
    );
}

#[test]
fn startup_recovers_a_prepared_workspace_journal_before_validation() {
    let fixture = Fixture::new();
    let revision = read_workspace_revision(&fixture.root).expect("workspace revision");
    let plan = plan_create_child_node(&fixture.root, fixture.root_id, "PreparedRecovery")
        .expect("plan prepared recovery fixture");
    let transaction = prepare_workspace_transaction_recovery_fixture(&plan)
        .expect("persist authentic prepared journal");
    let control = fixture
        .root
        .parent()
        .expect("workspace parent")
        .join("recovery-control");
    let state = ServerState::open(
        &fixture.root,
        ServerConfig::new(
            control,
            HttpSecurityConfig::loopback("127.0.0.1:8787".parse().expect("address")),
        ),
    )
    .expect("recover prepared transaction on startup");
    assert!(!transaction.exists());
    assert_eq!(
        read_workspace_revision(state.workspace_root()).expect("recovered revision"),
        revision
    );
}

#[test]
fn startup_keeps_invalid_recovery_evidence_and_fails_closed() {
    let fixture = Fixture::new();
    let transaction = fixture
        .root
        .join(".__weftext-transaction-workspace-invalid-evidence");
    std::fs::create_dir(&transaction).expect("invalid transaction directory");
    std::fs::write(transaction.join("journal.json"), b"{}\n").expect("invalid journal evidence");
    let result = ServerState::open(
        &fixture.root,
        ServerConfig::new(
            fixture
                .root
                .parent()
                .expect("workspace parent")
                .join("invalid-recovery-control"),
            HttpSecurityConfig::loopback("127.0.0.1:8787".parse().expect("address")),
        ),
    );
    assert!(matches!(result, Err(StartupError::WorkspaceRecovery(_))));
    assert!(transaction.join("journal.json").exists());
}

struct TestServer {
    address: SocketAddr,
    task: tokio::task::JoinHandle<()>,
    shutdown_state: Option<ServerState>,
    control: TempDir,
    cookie: Option<String>,
}

impl TestServer {
    async fn start(root: &Path) -> Self {
        Self::start_configured(root, |config| config).await
    }

    async fn start_configured(
        root: &Path,
        configure: impl FnOnce(ServerConfig) -> ServerConfig,
    ) -> Self {
        let mut server = Self::start_unbootstrapped_configured(root, configure).await;
        let secret = std::fs::read_to_string(server.bootstrap_secret_path())
            .expect("read bootstrap secret")
            .trim()
            .to_owned();
        let response = server
            .anonymous_request(
                "POST",
                "/api/v1/auth/bootstrap",
                Some(&json!({ "bootstrapSecret": secret, "password": OWNER_PASSWORD })),
            )
            .await;
        assert_eq!(response.status, 200);
        server.cookie = Some(response.session_cookie().expect("session cookie"));
        server
    }

    async fn start_unbootstrapped(root: &Path) -> Self {
        Self::start_unbootstrapped_with_policy(root, weftext_server::SessionPolicy::default()).await
    }

    async fn start_unbootstrapped_with_policy(
        root: &Path,
        policy: weftext_server::SessionPolicy,
    ) -> Self {
        Self::start_unbootstrapped_configured(root, |config| config.with_session_policy(policy))
            .await
    }

    async fn start_unbootstrapped_configured(
        root: &Path,
        configure: impl FnOnce(ServerConfig) -> ServerConfig,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("server address");
        let control =
            tempfile::tempdir_in(root.parent().expect("workspace parent")).expect("control plane");
        let config = configure(ServerConfig::new(
            control.path(),
            HttpSecurityConfig::loopback(address),
        ));
        let state = ServerState::open(root, config).expect("open hosted workspace");
        let shutdown_state = state.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, app(state)).await.expect("serve API");
        });
        Self {
            address,
            task,
            shutdown_state: Some(shutdown_state),
            control,
            cookie: None,
        }
    }

    fn bootstrap_secret_path(&self) -> PathBuf {
        self.control.path().join("bootstrap-secret")
    }

    async fn request(&self, method: &str, path: &str, body: Option<&Value>) -> HttpResponse {
        request_with_context(
            self.address,
            method,
            path,
            body,
            self.cookie.as_deref(),
            Some(&format!("http://{}", self.address)),
            Some("same-origin"),
            None,
        )
        .await
    }

    async fn anonymous_request(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> HttpResponse {
        request_with_context(
            self.address,
            method,
            path,
            body,
            None,
            Some(&format!("http://{}", self.address)),
            Some("same-origin"),
            None,
        )
        .await
    }

    async fn request_as(
        &self,
        cookie: &str,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> HttpResponse {
        request_with_context(
            self.address,
            method,
            path,
            body,
            Some(cookie),
            Some(&format!("http://{}", self.address)),
            Some("same-origin"),
            None,
        )
        .await
    }

    async fn restart(&mut self, root: &Path) {
        let shutdown_state = self
            .shutdown_state
            .take()
            .expect("running test server state");
        shutdown_state.begin_shutdown();
        tokio::task::yield_now().await;
        self.task.abort();
        let _ = (&mut self.task).await;
        drop(shutdown_state);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind restarted test server");
        self.address = listener.local_addr().expect("restarted server address");
        let config = ServerConfig::new(
            self.control.path(),
            HttpSecurityConfig::loopback(self.address),
        );
        let state = ServerState::open(root, config).expect("reopen hosted workspace");
        self.shutdown_state = Some(state.clone());
        self.task = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .await
                .expect("serve restarted API");
        });
    }

    async fn raw_request(
        &self,
        method: &str,
        path: &str,
        content_type: &str,
        body: &[u8],
    ) -> HttpResponse {
        request_raw_with_context(
            self.address,
            method,
            path,
            content_type,
            body,
            self.cookie.as_deref(),
            Some(&format!("http://{}", self.address)),
            Some("same-origin"),
            None,
        )
        .await
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct HttpResponse {
    status: u16,
    headers: String,
    body: Vec<u8>,
}

impl HttpResponse {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("JSON response")
    }

    fn text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("UTF-8 response")
    }

    fn session_cookie(&self) -> Option<String> {
        self.headers.lines().find_map(|line| {
            line.strip_prefix("set-cookie: ")
                .or_else(|| line.strip_prefix("Set-Cookie: "))
                .and_then(|value| value.split(';').next())
                .map(str::to_owned)
        })
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "raw HTTP negative tests need each independent request security input"
)]
async fn request_with_context(
    address: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&Value>,
    cookie: Option<&str>,
    origin: Option<&str>,
    csrf: Option<&str>,
    host: Option<&str>,
) -> HttpResponse {
    let body = body.map_or_else(Vec::new, |value| {
        serde_json::to_vec(value).expect("request JSON")
    });
    request_raw_with_context(
        address,
        method,
        path,
        "application/json",
        &body,
        cookie,
        origin,
        csrf,
        host,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn request_raw_with_context(
    address: SocketAddr,
    method: &str,
    path: &str,
    content_type: &str,
    body: &[u8],
    cookie: Option<&str>,
    origin: Option<&str>,
    csrf: Option<&str>,
    host: Option<&str>,
) -> HttpResponse {
    request_raw_with_headers(
        address,
        method,
        path,
        content_type,
        body,
        cookie,
        origin,
        csrf,
        host,
        "",
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn request_raw_with_headers(
    address: SocketAddr,
    method: &str,
    path: &str,
    content_type: &str,
    body: &[u8],
    cookie: Option<&str>,
    origin: Option<&str>,
    csrf: Option<&str>,
    host: Option<&str>,
    extra_headers: &str,
) -> HttpResponse {
    let mut stream = TcpStream::connect(address).await.expect("connect API");
    let cookie = cookie.map_or_else(String::new, |value| format!("Cookie: {value}\r\n"));
    let origin = origin.map_or_else(String::new, |value| format!("Origin: {value}\r\n"));
    let csrf = csrf.map_or_else(String::new, |value| format!("X-Weftext-CSRF: {value}\r\n"));
    let host = host.unwrap_or("");
    let host = if host.is_empty() {
        address.to_string()
    } else {
        host.to_owned()
    };
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{origin}{csrf}{cookie}{extra_headers}Connection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    stream.write_all(body).await.expect("write body");
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut bytes))
        .await
        .expect("response timeout")
        .expect("read response");
    parse_response(&bytes)
}

fn parse_response(bytes: &[u8]) -> HttpResponse {
    let boundary = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("HTTP header boundary");
    let headers = String::from_utf8(bytes[..boundary].to_vec()).expect("HTTP headers");
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
        .expect("HTTP status");
    HttpResponse {
        status,
        headers,
        body: bytes[boundary + 4..].to_vec(),
    }
}

fn expected_role_capability_map() -> Value {
    json!({
        "owner": {
            "readVisibleContent": true,
            "editDocuments": true,
            "mutateStructure": true,
            "writeAnnotations": true,
            "permanentlyDelete": true,
            "manageMembers": true,
            "manageWorkspace": true
        },
        "admin": {
            "readVisibleContent": true,
            "editDocuments": true,
            "mutateStructure": true,
            "writeAnnotations": true,
            "permanentlyDelete": false,
            "manageMembers": true,
            "manageWorkspace": false
        },
        "editor": {
            "readVisibleContent": true,
            "editDocuments": true,
            "mutateStructure": true,
            "writeAnnotations": true,
            "permanentlyDelete": false,
            "manageMembers": false,
            "manageWorkspace": false
        },
        "commenter": {
            "readVisibleContent": true,
            "editDocuments": false,
            "mutateStructure": false,
            "writeAnnotations": true,
            "permanentlyDelete": false,
            "manageMembers": false,
            "manageWorkspace": false
        },
        "viewer": {
            "readVisibleContent": true,
            "editDocuments": false,
            "mutateStructure": false,
            "writeAnnotations": false,
            "permanentlyDelete": false,
            "manageMembers": false,
            "manageWorkspace": false
        }
    })
}

#[tokio::test]
async fn serves_health_capabilities_and_same_origin_webui() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;

    let health = server.request("GET", "/api/v1/health", None).await;
    assert_eq!(health.status, 200);
    assert_eq!(health.json()["stage"], "canonical-asciidoc-multirole-acl");
    let health_headers = health.headers.to_ascii_lowercase();
    assert!(health_headers.contains("cache-control: no-store"));
    assert!(health_headers.contains("x-content-type-options: nosniff"));
    assert!(health_headers.contains("x-frame-options: deny"));
    assert!(health_headers.contains("referrer-policy: no-referrer"));
    assert!(health_headers.contains("content-security-policy:"));

    let capabilities = server.request("GET", "/api/v1/capabilities", None).await;
    assert_eq!(capabilities.status, 200);
    assert_eq!(capabilities.json()["loopbackOnly"], true);
    assert_eq!(capabilities.json()["deploymentReady"], false);
    assert_eq!(
        capabilities.json()["authentication"],
        "local_account_session"
    );
    assert_eq!(
        capabilities.json()["managedDocumentProfile"],
        "weftext.asciidoc.v1"
    );
    assert_eq!(
        capabilities.json()["referenceRecordWrites"],
        "retired_until_typed_citation_data"
    );
    assert!(
        capabilities.json()["features"]
            .as_array()
            .expect("capability features")
            .iter()
            .any(|feature| feature == "multi_role_authentication")
    );
    assert_eq!(
        capabilities.json()["roleCapabilities"],
        expected_role_capability_map()
    );
    assert!(capabilities.json().get("bootstrapRequired").is_none());
    let owner_session = server.request("GET", "/api/v1/auth/session", None).await;
    assert_eq!(owner_session.status, 200);
    assert_eq!(
        owner_session.json()["capabilities"],
        capabilities.json()["roleCapabilities"]["owner"]
    );

    let webui = server.request("GET", "/", None).await;
    assert_eq!(webui.status, 200);
    assert!(webui.text().contains("文缕 Server"));
    let app_script = server.request("GET", "/app.js", None).await;
    assert!(app_script.text().contains("serverApi.openDocument"));
    assert!(!app_script.text().contains("weftextDesktop"));
    let navigation_script = server.request("GET", "/navigation.js", None).await;
    assert_eq!(navigation_script.status, 200);
    assert!(
        navigation_script
            .text()
            .contains("INITIAL_NAVIGATION_WINDOW")
    );
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end boundary test keeps the proxy security contract auditable"
)]
async fn strict_same_host_proxy_boundary_is_explicit_and_runtime_ready() {
    let fixture = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("proxy-mode listener");
    let address = listener.local_addr().expect("proxy-mode address");
    let control = tempfile::tempdir_in(fixture.root.parent().expect("workspace parent"))
        .expect("proxy control plane");
    let state = ServerState::open(
        &fixture.root,
        ServerConfig::new(
            control.path(),
            HttpSecurityConfig::same_host_reverse_proxy(address, "https://notes.example.test")
                .expect("strict proxy config"),
        ),
    )
    .expect("open proxy-mode Server");
    let secret_path = state
        .reverse_proxy_secret_path()
        .expect("proxy secret path")
        .to_owned();
    assert_eq!(
        secret_path.parent().expect("secret parent"),
        std::fs::canonicalize(control.path()).expect("canonical control path")
    );
    let token = std::fs::read_to_string(&secret_path)
        .expect("proxy secret")
        .trim()
        .to_owned();
    assert_eq!(token.len(), 64);
    let shutdown_state = state.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, app(state))
            .await
            .expect("serve proxy-mode API");
    });
    let host = Some("notes.example.test");
    let proxy_header = format!("X-Weftext-Proxy-Token: {token}\r\n");

    let missing_boundary = request_raw_with_headers(
        address,
        "GET",
        "/api/v1/health/ready",
        "application/json",
        &[],
        None,
        None,
        None,
        host,
        "",
    )
    .await;
    assert_error(&missing_boundary, 403, "proxy_boundary_rejected");

    let live = request_raw_with_headers(
        address,
        "GET",
        "/api/v1/health/live",
        "application/json",
        &[],
        None,
        None,
        None,
        host,
        "",
    )
    .await;
    assert_eq!(live.status, 200);
    assert!(
        live.headers
            .to_ascii_lowercase()
            .contains("strict-transport-security: max-age=31536000")
    );

    let ready = request_raw_with_headers(
        address,
        "GET",
        "/api/v1/health/ready",
        "application/json",
        &[],
        None,
        None,
        None,
        host,
        &proxy_header,
    )
    .await;
    assert_eq!(ready.status, 200);
    assert_eq!(ready.json()["status"], "ready");
    assert_eq!(
        ready.json()["runtimeBoundary"],
        "same_host_tls_reverse_proxy"
    );
    assert!(!ready.headers.contains(&token));

    let forwarded = request_raw_with_headers(
        address,
        "GET",
        "/api/v1/health/ready",
        "application/json",
        &[],
        None,
        None,
        None,
        host,
        &format!("{proxy_header}X-Forwarded-Proto: https\r\n"),
    )
    .await;
    assert_error(&forwarded, 400, "forwarded_header_rejected");

    let capabilities = request_raw_with_headers(
        address,
        "GET",
        "/api/v1/capabilities",
        "application/json",
        &[],
        None,
        None,
        None,
        host,
        &proxy_header,
    )
    .await;
    assert_eq!(capabilities.status, 200);
    assert_eq!(
        capabilities.json()["runtimeBoundary"],
        "same_host_tls_reverse_proxy"
    );
    assert_eq!(
        capabilities.json()["forwardedHeaderPolicy"],
        "rejected_not_trusted"
    );
    assert_eq!(capabilities.json()["deploymentReady"], false);
    assert_eq!(capabilities.json()["publicInternet"], "unsupported");
    assert_eq!(capabilities.json()["realtimeCollaboration"], false);

    let bootstrap_secret = std::fs::read_to_string(control.path().join("bootstrap-secret"))
        .expect("bootstrap secret")
        .trim()
        .to_owned();
    let body = serde_json::to_vec(&json!({
        "bootstrapSecret": bootstrap_secret,
        "password": OWNER_PASSWORD
    }))
    .expect("bootstrap JSON");
    let bootstrap = request_raw_with_headers(
        address,
        "POST",
        "/api/v1/auth/bootstrap",
        "application/json",
        &body,
        None,
        Some("https://notes.example.test"),
        Some("same-origin"),
        host,
        &proxy_header,
    )
    .await;
    assert_eq!(bootstrap.status, 200);
    assert!(bootstrap.headers.contains("Secure"));

    shutdown_state.begin_shutdown();
    let draining = request_raw_with_headers(
        address,
        "GET",
        "/api/v1/health/ready",
        "application/json",
        &[],
        None,
        None,
        None,
        host,
        &proxy_header,
    )
    .await;
    assert_eq!(draining.status, 503);
    assert_eq!(draining.json()["status"], "not_ready");
    let rejected_during_drain = request_raw_with_headers(
        address,
        "GET",
        "/api/v1/capabilities",
        "application/json",
        &[],
        None,
        None,
        None,
        host,
        &proxy_header,
    )
    .await;
    assert_error(&rejected_during_drain, 503, "server_shutting_down");
    task.abort();
}

#[tokio::test]
async fn inventory_open_search_preview_and_commit_reuse_core() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;

    let inventory = server.request("GET", "/api/v1/workspace", None).await;
    assert_eq!(inventory.status, 200);
    let inventory_json = inventory.json();
    assert_eq!(inventory_json["rootNodeId"], fixture.root_id.to_string());
    assert_eq!(
        inventory_json["documentFormat"]["generation"],
        "ascii_doc_v1"
    );
    assert_eq!(inventory_json["nodes"].as_array().expect("nodes").len(), 2);
    let child = inventory_json["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["id"] == fixture.child_id.to_string())
        .expect("child inventory");
    assert_eq!(child["locator"], "Child");
    assert_eq!(child["icon"]["value"], "weftext:star");

    let document_path = format!("/api/v1/documents/{}", fixture.child_id);
    let opened = server.request("GET", &document_path, None).await;
    assert_eq!(opened.status, 200);
    let opened_json = opened.json();
    assert_eq!(opened_json["profile"]["profile"], "ascii_doc_v1");
    assert_eq!(
        opened_json["metadata"]["schema"],
        "weftext.node-metadata.v1"
    );
    assert_eq!(opened_json["metadata"]["id"], fixture.child_id.to_string());
    assert_eq!(opened_json["metadata"]["icon"], "weftext:star");
    assert!(opened_json["metadata"]["adjacentHeadingBody"].is_null());
    assert!(opened_json["properties"]["properties"].is_array());
    assert_eq!(
        opened_json["model"]["blocks"],
        opened_json["view"]["blocks"]
    );
    assert_eq!(
        opened_json["length"],
        opened_json["source"].as_str().unwrap().len()
    );
    let base = opened_json["revision"].as_str().expect("revision");
    let source = format!(
        "{}\nEdited through Server",
        opened_json["source"].as_str().expect("source")
    );

    let search = server.request("GET", "/api/v1/search?q=Needle", None).await;
    assert_eq!(search.status, 200);
    assert_eq!(
        search.json()["results"][0]["id"],
        fixture.child_id.to_string()
    );

    let preview = server
        .request(
            "POST",
            &format!("{document_path}/preview"),
            Some(&json!({ "baseRevision": base, "source": source })),
        )
        .await;
    assert_eq!(preview.status, 200);
    assert_eq!(preview.json()["changed"], true);
    assert_eq!(preview.json()["profile"]["profile"], "ascii_doc_v1");
    assert_eq!(
        preview.json()["model"]["blocks"],
        preview.json()["view"]["blocks"]
    );
    assert_ne!(preview.json()["nextRevision"], base);

    let commit = server
        .request(
            "PUT",
            &document_path,
            Some(&json!({ "baseRevision": base, "source": source })),
        )
        .await;
    assert_eq!(commit.status, 200);
    assert_eq!(commit.json()["changed"], true);
    let core_snapshot =
        read_node_document(fixture.root.join("Child")).expect("Core read after API commit");
    assert_eq!(commit.json()["revision"], core_snapshot.revision.as_str());
    assert!(core_snapshot.source.ends_with("Edited through Server"));
}

async fn mutate_annotation(
    server: &TestServer,
    route: &str,
    state: &Value,
    action: Value,
) -> HttpResponse {
    let mut body = action
        .as_object()
        .expect("annotation action object")
        .clone();
    body.insert(
        "baseWorkspaceRevision".to_owned(),
        state["workspaceRevision"].clone(),
    );
    body.insert("baseRevision".to_owned(), state["revision"].clone());
    body.insert(
        "nodeId".to_owned(),
        Value::String(
            route
                .rsplit('/')
                .next()
                .expect("annotation route node ID")
                .to_owned(),
        ),
    );
    body.entry("timestamp".to_owned())
        .or_insert_with(|| Value::String("2026-08-24T12:00:00+08:00".to_owned()));
    server
        .request("POST", route, Some(&Value::Object(body)))
        .await
}

async fn mutate_annotation_as(
    server: &TestServer,
    cookie: &str,
    route: &str,
    state: &Value,
    action: Value,
) -> HttpResponse {
    let mut body = action
        .as_object()
        .expect("annotation action object")
        .clone();
    body.insert(
        "baseWorkspaceRevision".to_owned(),
        state["workspaceRevision"].clone(),
    );
    body.insert("baseRevision".to_owned(), state["revision"].clone());
    body.insert(
        "nodeId".to_owned(),
        Value::String(
            route
                .rsplit('/')
                .next()
                .expect("annotation route node ID")
                .to_owned(),
        ),
    );
    body.entry("timestamp".to_owned())
        .or_insert_with(|| Value::String("2026-08-24T12:00:00+08:00".to_owned()));
    server
        .request_as(cookie, "POST", route, Some(&Value::Object(body)))
        .await
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn annotation_v3_api_binds_identity_serializes_writes_and_accepts_suggestions_atomically() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;
    let route = format!("/api/v1/annotations/{}", fixture.child_id);
    let document_route = format!("/api/v1/documents/{}", fixture.child_id);

    let opened = server.request("GET", &route, None).await;
    assert_eq!(opened.status, 200);
    let mut state = opened.json();
    assert_eq!(state["store"]["version"], 3);
    assert_eq!(state["store"]["document_id"], fixture.child_id.to_string());
    assert_eq!(state["store"]["annotations"], json!([]));

    let spoofed = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "create",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": null,
            "labels": [],
            "bodySource": "Bound identity",
            "suggestedSource": null,
            "authorId": "99999999-9999-4999-8999-999999999999"
        }),
    )
    .await;
    assert_eq!(spoofed.status, 422, "{}", spoofed.text());
    assert_eq!(spoofed.json()["error"]["code"], "invalid_json");

    for legacy in [
        json!({
            "action": "create",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": null,
            "labels": [],
            "bodySource": "legacy",
            "suggestedSource": null,
            "sourceOffset": 1
        }),
        json!({
            "action": "create",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": null,
            "labels": [],
            "bodySource": "legacy",
            "suggestedSource": null,
            "mark": "highlight",
            "color": "yellow",
            "resolved": false
        }),
        json!({
            "action": "create",
            "kind": "mark",
            "target": {"kind": "block_at", "source_offset": 1},
            "appearance": {"mark": "highlight", "theme": "yellow"},
            "labels": [],
            "bodySource": null,
            "suggestedSource": null
        }),
        json!({
            "action": "create",
            "kind": "mark",
            "target": {"type": "document"},
            "appearance": {"mark": "highlight", "theme": "yellow"},
            "labels": [],
            "bodySource": null,
            "suggestedSource": null
        }),
        json!({
            "action": "create",
            "kind": "mark",
            "target": {"kind": "document", "unexpected": 1},
            "appearance": {"mark": "highlight", "theme": "yellow"},
            "labels": [],
            "bodySource": null,
            "suggestedSource": null
        }),
        json!({
            "action": "create",
            "kind": "mark",
            "target": {"kind": "document"},
            "appearance": {"mark": "highlight", "theme": "yellow", "color": "yellow"},
            "labels": [],
            "bodySource": null,
            "suggestedSource": null
        }),
        json!({
            "action": "create",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": null,
            "labels": [],
            "bodySource": "client must not author replica authority",
            "suggestedSource": null,
            "sidecarCompleteness": "complete_hosted_workspace"
        }),
    ] {
        let rejected = mutate_annotation(&server, &route, &state, legacy).await;
        assert_eq!(rejected.json()["error"]["code"], "invalid_json");
    }
    let action_alias = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "setAppearance",
            "annotationId": "99999999-9999-4999-8999-999999999999",
            "appearance": null
        }),
    )
    .await;
    assert_error(&action_alias, 400, "invalid_request");
    let create_none = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "create",
            "kind": "mark",
            "target": {"kind": "document"},
            "appearance": {"mark": "none"},
            "labels": [],
            "bodySource": null,
            "suggestedSource": null
        }),
    )
    .await;
    assert_error(&create_none, 400, "invalid_request");
    assert_eq!(
        server.request("GET", &route, None).await.json()["store"]["annotations"],
        json!([])
    );

    let created = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "create",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": null,
            "labels": ["review"],
            "bodySource": "Bound identity",
            "suggestedSource": null
        }),
    )
    .await;
    assert_eq!(created.status, 200, "{}", created.text());
    state = created.json();
    assert_eq!(state["auditRecorded"], true);
    let annotation_id = state["store"]["annotations"][0]["id"]
        .as_str()
        .expect("annotation ID")
        .to_owned();
    let actor_id = state["store"]["annotations"][0]["thread"][0]["author_id"]
        .as_str()
        .expect("bound actor ID")
        .to_owned();
    assert_ne!(actor_id, "99999999-9999-4999-8999-999999999999");
    assert_eq!(
        state["store"]["annotations"][0]["thread"][0]["body"]["format"],
        "weftext.asciidoc.inline.v1"
    );

    let replied = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "reply",
            "annotationId": annotation_id,
            "bodySource": "Second message"
        }),
    )
    .await;
    assert_eq!(replied.status, 200, "{}", replied.text());
    state = replied.json();
    let message_id = state["store"]["annotations"][0]["thread"][1]["id"]
        .as_str()
        .expect("reply ID")
        .to_owned();
    assert_eq!(
        state["store"]["annotations"][0]["thread"][1]["author_id"],
        actor_id
    );

    let edited = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "edit_message",
            "annotationId": annotation_id,
            "messageId": message_id,
            "bodySource": "Edited reply"
        }),
    )
    .await;
    assert_eq!(edited.status, 200, "{}", edited.text());
    state = edited.json();
    assert_eq!(
        state["store"]["annotations"][0]["thread"][1]["body"]["source"],
        "Edited reply"
    );

    for action in [
        json!({
            "action": "set_appearance",
            "annotationId": annotation_id,
            "appearance": {"mark": "highlight", "theme": "yellow"}
        }),
        json!({
            "action": "set_labels",
            "annotationId": annotation_id,
            "labels": ["review", "important"]
        }),
        json!({"action": "resolve", "annotationId": annotation_id}),
        json!({"action": "reopen", "annotationId": annotation_id}),
    ] {
        let response = mutate_annotation(&server, &route, &state, action).await;
        assert_eq!(response.status, 200, "{}", response.text());
        state = response.json();
    }
    assert_eq!(state["store"]["annotations"][0]["state"], "open");
    assert_eq!(
        state["store"]["annotations"][0]["labels"],
        json!(["review", "important"])
    );
    let cleared_appearance = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "set_appearance",
            "annotationId": annotation_id,
            "appearance": {"mark": "none"}
        }),
    )
    .await;
    assert_eq!(
        cleared_appearance.status,
        200,
        "{}",
        cleared_appearance.text()
    );
    state = cleared_appearance.json();
    assert!(state["store"]["annotations"][0]["appearance"].is_null());

    let document = server.request("GET", &document_route, None).await.json();
    let source = document["source"].as_str().expect("document source");
    let needle = u64::try_from(source.find("Needle").expect("Needle offset")).expect("offset");
    let anchored = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "create",
            "kind": "mark",
            "target": {"kind": "text_range", "start": needle, "end": needle + 6},
            "appearance": {"mark": "underline", "theme": "blue"},
            "labels": [],
            "bodySource": null,
            "suggestedSource": null
        }),
    )
    .await;
    assert_eq!(anchored.status, 200, "{}", anchored.text());
    state = anchored.json();
    let anchored_id = state["store"]["annotations"][1]["id"]
        .as_str()
        .expect("anchored ID")
        .to_owned();

    let shifted_source = source.replacen("Needle body", "Prelude\n\nNeedle body", 1);
    let shifted = server
        .request(
            "PUT",
            &document_route,
            Some(&json!({
                "baseRevision": document["revision"],
                "source": shifted_source
            })),
        )
        .await;
    assert_eq!(shifted.status, 200, "{}", shifted.text());
    state = server.request("GET", &route, None).await.json();
    let reanchored = mutate_annotation(
        &server,
        &route,
        &state,
        json!({"action": "reanchor", "annotationId": anchored_id}),
    )
    .await;
    assert_eq!(reanchored.status, 200, "{}", reanchored.text());
    state = reanchored.json();
    assert_eq!(state["store"]["annotations"][1]["state"], "open");
    assert_eq!(
        state["store"]["annotations"][1]["target"]["base_revision"],
        state["revision"]
    );

    let current_document = server.request("GET", &document_route, None).await.json();
    let insertion = current_document["source"]
        .as_str()
        .expect("current source")
        .len();
    let suggestion = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "create",
            "kind": "suggestion_insert",
            "target": {"kind": "insertion_point", "position": insertion},
            "appearance": null,
            "labels": [],
            "bodySource": "Append an accepted line",
            "suggestedSource": "\nAccepted suggestion"
        }),
    )
    .await;
    assert_eq!(suggestion.status, 200, "{}", suggestion.text());
    state = suggestion.json();
    let suggestion_id = state["store"]["annotations"][2]["id"]
        .as_str()
        .expect("suggestion ID")
        .to_owned();
    let accepted = mutate_annotation(
        &server,
        &route,
        &state,
        json!({"action": "accept_suggestion", "annotationId": suggestion_id}),
    )
    .await;
    assert_eq!(accepted.status, 200, "{}", accepted.text());
    state = accepted.json();
    assert_eq!(state["store"]["annotations"][2]["resolution"], "accepted");
    assert!(
        read_node_document(fixture.root.join("Child"))
            .expect("accepted document")
            .source
            .ends_with("Accepted suggestion")
    );
    let persisted: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture.root.join("Child/weftext.annotations.json"))
            .expect("persisted annotation sidecar"),
    )
    .expect("valid annotation JSON");
    assert_eq!(persisted["annotations"][2]["resolution"], "accepted");

    let current_document = server.request("GET", &document_route, None).await.json();
    let insertion = current_document["source"]
        .as_str()
        .expect("current source")
        .len();
    let rejected = mutate_annotation(
        &server,
        &route,
        &state,
        json!({
            "action": "create",
            "kind": "suggestion_insert",
            "target": {"kind": "insertion_point", "position": insertion},
            "appearance": null,
            "labels": [],
            "bodySource": null,
            "suggestedSource": "Rejected text"
        }),
    )
    .await;
    assert_eq!(rejected.status, 200, "{}", rejected.text());
    state = rejected.json();
    let rejected_id = state["store"]["annotations"][3]["id"]
        .as_str()
        .expect("rejected suggestion ID")
        .to_owned();
    let rejected = mutate_annotation(
        &server,
        &route,
        &state,
        json!({"action": "reject_suggestion", "annotationId": rejected_id}),
    )
    .await;
    assert_eq!(rejected.status, 200, "{}", rejected.text());
    state = rejected.json();
    assert_eq!(state["store"]["annotations"][3]["resolution"], "rejected");

    let stale_base = state.clone();
    let first = mutate_annotation(
        &server,
        &route,
        &stale_base,
        json!({
            "action": "set_labels",
            "annotationId": annotation_id,
            "labels": ["serialized"]
        }),
    )
    .await;
    assert_eq!(first.status, 200, "{}", first.text());
    let conflict_response = mutate_annotation(
        &server,
        &route,
        &stale_base,
        json!({"action": "resolve", "annotationId": annotation_id}),
    )
    .await;
    assert_error(&conflict_response, 409, "stale_workspace_revision");

    let connection =
        rusqlite::Connection::open(server.control.path().join("control-plane.sqlite3"))
            .expect("open audit database");
    let accepted_events: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM security_events WHERE event_type = 'annotation_suggestion_accepted'",
            [],
            |row| row.get(0),
        )
        .expect("accepted suggestion audit");
    assert_eq!(accepted_events, 1);

    let conflict_name = "weftext.annotations (conflicted copy private-device).json";
    std::fs::write(
        fixture.root.join("Child").join(conflict_name),
        std::fs::read(fixture.root.join("Child/weftext.annotations.json"))
            .expect("current annotation sidecar"),
    )
    .expect("conflict-copy fixture");
    let unresolved = server.request("GET", &route, None).await;
    assert_error(&unresolved, 503, "workspace_invalid");
    let unresolved_body = unresolved.text();
    assert!(!unresolved_body.contains(conflict_name));
    assert!(!unresolved_body.contains("private-device"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn citation_api_keeps_occurrence_authoring_without_legacy_record_write_routes() {
    let fixture = CitationFixture::new();
    let server = TestServer::start(&fixture.root).await;

    let capabilities = server
        .request("GET", "/api/v1/citations/capabilities", None)
        .await;
    assert_eq!(capabilities.status, 200);
    assert_eq!(capabilities.json()["referenceRecordWritesAvailable"], false);
    assert!(
        capabilities.json()["referenceRecordWritesReason"]
            .as_str()
            .expect("retirement reason")
            .contains("typed Citation Data")
    );

    let validation = server
        .request("GET", "/api/v1/citations/validate", None)
        .await;
    assert_eq!(validation.status, 200);
    assert_eq!(validation.json()["valid"], true);
    let search = server
        .request(
            "GET",
            "/api/v1/citations/references?q=anything&limit=10",
            None,
        )
        .await;
    assert_eq!(search.status, 200);
    assert_eq!(search.json()["references"], json!([]));

    let component_path = format!("/api/v1/documents/{}", fixture.component_id);
    let component = server.request("GET", &component_path, None).await.json();
    let source = component["source"].as_str().expect("component source");
    let macro_request = json!({
        "baseRevision": component["revision"],
        "source": source,
        "target": {"kind": "insert", "offset": source.len()},
        "intent": {"kind": "bibliography", "inclusion": "cited"},
    });
    let macro_preview = server
        .request(
            "POST",
            &format!("/api/v1/citations/{}/macros/preview", fixture.component_id),
            Some(&macro_request),
        )
        .await;
    assert_eq!(macro_preview.status, 200);
    assert!(
        macro_preview.json()["proposedSource"]
            .as_str()
            .expect("macro source")
            .ends_with("bibliography::[]")
    );
    let macro_commit = server
        .request(
            "POST",
            &format!("/api/v1/citations/{}/macros/commit", fixture.component_id),
            Some(&macro_request),
        )
        .await;
    assert_eq!(macro_commit.status, 200);
    assert_eq!(macro_commit.json()["auditRecorded"], true);

    let committed_source = std::fs::read_to_string(fixture.root.join("Component/Component.adoc"))
        .expect("committed component source");
    let workspace_revision = server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json()["workspaceRevision"]
        .as_str()
        .expect("workspace revision")
        .to_owned();
    let retired_routes = [
        "/api/v1/citations/references/preview".to_owned(),
        format!(
            "/api/v1/citations/references/{}/edit/preview",
            fixture.root_id
        ),
        format!(
            "/api/v1/citations/references/{}/edit/commit",
            fixture.root_id
        ),
        format!(
            "/api/v1/citations/references/{}/rename/preview",
            fixture.root_id
        ),
        "/api/v1/citations/transactions/44444444-4444-4444-8444-444444444444/commit".to_owned(),
        "/api/v1/citations/recover".to_owned(),
        "/api/v1/citations/rollback".to_owned(),
    ];
    for route in retired_routes {
        let response = server.request("POST", &route, None).await;
        assert_eq!(response.status, 404, "legacy route must not exist: {route}");
        let response_text = String::from_utf8_lossy(&response.body);
        assert!(!response_text.contains("reference_record"));
        assert!(!response_text.contains("typed Citation Data"));
    }
    assert!(!fixture.root.join("Created").exists());
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("Component/Component.adoc"))
            .expect("unchanged component source"),
        committed_source
    );
    assert_eq!(
        server
            .request("GET", "/api/v1/workspace", None)
            .await
            .json()["workspaceRevision"],
        workspace_revision
    );

    let connection =
        rusqlite::Connection::open(server.control.path().join("control-plane.sqlite3"))
            .expect("open audit database");
    let mut statement = connection
        .prepare("SELECT event_type FROM security_events")
        .expect("prepare audit query");
    let event_types = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query audit rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("audit event types");
    let citation_events = event_types
        .iter()
        .filter(|event| event.starts_with("citation_"))
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(citation_events, vec!["citation_macro_edited"]);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn task_api_uses_session_bound_validated_transactions_and_audit() {
    const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
    const TASK_B: &str = "22222222-2222-4222-8222-222222222222";
    const TASK_R: &str = "33333333-3333-4333-8333-333333333333";
    const MISSING: &str = "99999999-9999-4999-8999-999999999999";

    let fixture = TaskFixture::new();
    let server = TestServer::start(&fixture.root).await;
    let validation = server.request("GET", "/api/v1/tasks/validate", None).await;
    assert_eq!(validation.status, 200);
    assert_eq!(validation.json()["valid"], true);
    assert_eq!(
        validation.json()["occurrences"].as_array().unwrap().len(),
        3
    );
    assert!(
        validation.json()["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|occurrence| occurrence["nodeId"] == fixture.child_id.to_string())
    );
    let inspect_path = format!("/api/v1/tasks/nodes/{}", fixture.root_id);
    let inspect = server.request("GET", &inspect_path, None).await;
    assert_eq!(inspect.status, 200);
    assert_eq!(inspect.json()["occurrences"].as_array().unwrap().len(), 2);

    let workspace = server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();
    let document = server
        .request(
            "GET",
            &format!("/api/v1/documents/{}", fixture.root_id),
            None,
        )
        .await
        .json();
    let edit_request = json!({
        "baseWorkspaceRevision": workspace["workspaceRevision"],
        "baseRevision": document["revision"],
        "target": {"kind": "id", "id": TASK_A},
        "intent": {"kind": "set_priority", "priority": "high"},
    });
    let edit_preview = server
        .request(
            "POST",
            &format!("/api/v1/tasks/nodes/{}/edit/preview", fixture.root_id),
            Some(&edit_request),
        )
        .await;
    assert_eq!(edit_preview.status, 200, "{}", edit_preview.text());
    assert!(
        edit_preview.json()["authoring"]["proposedSource"]
            .as_str()
            .unwrap()
            .contains("priority=high")
    );
    assert!(
        !std::fs::read_to_string(fixture.root.join("Tasks.adoc"))
            .unwrap()
            .contains("priority=high")
    );
    let edit_plan_id = edit_preview.json()["planId"].as_str().unwrap().to_owned();

    let second_login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({"login": "owner", "password": OWNER_PASSWORD})),
        )
        .await;
    let second_cookie = second_login.session_cookie().expect("second session");
    let foreign = request_with_context(
        server.address,
        "POST",
        &format!("/api/v1/tasks/transactions/{edit_plan_id}/commit"),
        None,
        Some(&second_cookie),
        Some(&format!("http://{}", server.address)),
        Some("same-origin"),
        None,
    )
    .await;
    assert_error(&foreign, 404, "task_plan_unavailable");
    assert!(!foreign.text().contains(TASK_A));

    let edit_commit = server
        .request(
            "POST",
            &format!("/api/v1/tasks/transactions/{edit_plan_id}/commit"),
            None,
        )
        .await;
    assert_eq!(edit_commit.status, 200, "{}", edit_commit.text());
    assert_eq!(edit_commit.json()["auditRecorded"], true);
    assert_eq!(
        edit_commit.json()["result"]["task"]["metadata"]["priority"],
        "high"
    );
    assert!(
        std::fs::read_to_string(fixture.root.join("Tasks.adoc"))
            .unwrap()
            .contains("priority=high")
    );
    let replay = server
        .request(
            "POST",
            &format!("/api/v1/tasks/transactions/{edit_plan_id}/commit"),
            None,
        )
        .await;
    assert_error(&replay, 404, "task_plan_unavailable");

    let workspace = server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();
    let document = server
        .request(
            "GET",
            &format!("/api/v1/documents/{}", fixture.root_id),
            None,
        )
        .await
        .json();
    let dependency_preview = server
        .request(
            "POST",
            &format!(
                "/api/v1/tasks/nodes/{}/dependencies/preview",
                fixture.root_id
            ),
            Some(&json!({
                "baseWorkspaceRevision": workspace["workspaceRevision"],
                "baseRevision": document["revision"],
                "target": {"kind": "id", "id": TASK_A},
                "dependencies": [TASK_B],
            })),
        )
        .await;
    assert_eq!(
        dependency_preview.status,
        200,
        "{}",
        dependency_preview.text()
    );
    let dependency_plan = dependency_preview.json()["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let dependency_commit = server
        .request(
            "POST",
            &format!("/api/v1/tasks/transactions/{dependency_plan}/commit"),
            None,
        )
        .await;
    assert_eq!(dependency_commit.status, 200);
    assert_eq!(dependency_commit.json()["auditRecorded"], true);
    assert_eq!(
        dependency_commit.json()["result"]["dependencies"][0],
        TASK_B
    );

    let workspace = server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();
    let document = server
        .request(
            "GET",
            &format!("/api/v1/documents/{}", fixture.root_id),
            None,
        )
        .await
        .json();
    let recurrence_preview = server
        .request(
            "POST",
            &format!("/api/v1/tasks/nodes/{}/recurrence/preview", fixture.root_id),
            Some(&json!({
                "baseWorkspaceRevision": workspace["workspaceRevision"],
                "baseRevision": document["revision"],
                "target": {"kind": "id", "id": TASK_R},
                "context": {
                    "completedAt": {"kind": "date", "value": "2026-08-24"},
                    "utcOffsetMinutes": 480,
                },
            })),
        )
        .await;
    assert_eq!(
        recurrence_preview.status,
        200,
        "{}",
        recurrence_preview.text()
    );
    let next_id = recurrence_preview.json()["completion"]["nextTaskId"]
        .as_str()
        .unwrap()
        .to_owned();
    let recurrence_plan = recurrence_preview.json()["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let recurrence_commit = server
        .request(
            "POST",
            &format!("/api/v1/tasks/transactions/{recurrence_plan}/commit"),
            None,
        )
        .await;
    assert_eq!(recurrence_commit.status, 200);
    assert_eq!(recurrence_commit.json()["result"]["nextTaskId"], next_id);
    assert_eq!(recurrence_commit.json()["auditRecorded"], true);

    let workspace = server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();
    let document = server
        .request(
            "GET",
            &format!("/api/v1/documents/{}", fixture.root_id),
            None,
        )
        .await
        .json();
    let stale_preview = server
        .request(
            "POST",
            &format!("/api/v1/tasks/nodes/{}/edit/preview", fixture.root_id),
            Some(&json!({
                "baseWorkspaceRevision": workspace["workspaceRevision"],
                "baseRevision": document["revision"],
                "target": {"kind": "id", "id": TASK_A},
                "intent": {"kind": "set_priority", "priority": "low"},
            })),
        )
        .await;
    assert_eq!(stale_preview.status, 200);
    let stale_plan = stale_preview.json()["planId"].as_str().unwrap().to_owned();
    replace_document(&fixture.root.join("Child"), |source| {
        format!("{source}\nUnrelated external change.\n")
    });
    let stale_commit = server
        .request(
            "POST",
            &format!("/api/v1/tasks/transactions/{stale_plan}/commit"),
            None,
        )
        .await;
    assert_error(&stale_commit, 409, "stale_workspace_revision");
    assert!(!stale_commit.text().contains(TASK_B));

    let stale_repreview = server
        .request(
            "POST",
            &format!("/api/v1/tasks/nodes/{}/edit/preview", fixture.root_id),
            Some(&json!({
                "baseWorkspaceRevision": workspace["workspaceRevision"],
                "baseRevision": document["revision"],
                "target": {"kind": "id", "id": TASK_A},
                "intent": {"kind": "set_priority", "priority": "highest"},
            })),
        )
        .await;
    assert_error(&stale_repreview, 409, "stale_workspace_revision");
    assert!(!stale_repreview.text().contains(TASK_B));

    let current_workspace = server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();
    let current_document = server
        .request(
            "GET",
            &format!("/api/v1/documents/{}", fixture.root_id),
            None,
        )
        .await
        .json();
    let missing = server
        .request(
            "POST",
            &format!(
                "/api/v1/tasks/nodes/{}/dependencies/preview",
                fixture.root_id
            ),
            Some(&json!({
                "baseWorkspaceRevision": current_workspace["workspaceRevision"],
                "baseRevision": current_document["revision"],
                "target": {"kind": "id", "id": TASK_A},
                "dependencies": [MISSING],
            })),
        )
        .await;
    assert_error(&missing, 422, "task_transaction_rejected");
    assert!(!missing.text().contains(MISSING));

    let recovery = server.request("POST", "/api/v1/tasks/recover", None).await;
    assert_eq!(recovery.status, 200);
    assert_eq!(recovery.json()["auditRecorded"], true);

    let connection =
        rusqlite::Connection::open(server.control.path().join("control-plane.sqlite3")).unwrap();
    let event_types = connection
        .prepare("SELECT event_type FROM security_events ORDER BY event_id")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for expected in [
        "task_edited",
        "task_dependencies_replaced",
        "task_recurrence_completed",
        "task_transaction_recovery",
    ] {
        assert!(event_types.iter().any(|event| event == expected));
    }
}

#[tokio::test]
async fn owner_query_api_executes_exact_source_and_returns_invalid_diagnostics_without_writes() {
    let fixture = TaskFixture::new();
    let server = TestServer::start(&fixture.root).await;
    let body = json!({
        "source": "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n....\n",
        "blockIndex": 0,
        "context": {
            "today": {"year": 2026, "month": 8, "day": 24},
            "now": "2026-08-24T09:30:00+08:00",
            "timezone": "Asia/Shanghai",
            "locale": "zh-CN",
            "binding": {"nodeId": fixture.root_id, "heading": null},
        },
    });
    let first = server
        .request("POST", "/api/v1/queries/execute", Some(&body))
        .await;
    let second = server
        .request("POST", "/api/v1/queries/execute", Some(&body))
        .await;
    assert_eq!(first.status, 200, "{}", first.text());
    assert_eq!(first.json(), second.json());
    assert_eq!(first.json()["valid"], true);
    assert_eq!(first.json()["execution"]["result"]["totalBeforeLimit"], 2);
    assert_eq!(
        first.json()["execution"]["result"]["columns"][0]["path"],
        "name"
    );
    assert_eq!(
        first.json()["execution"]["csv"],
        "name,path\r\nTasks,/\r\nChild,/Child\r\n"
    );
    assert_eq!(
        first.json()["execution"]["result"]["rows"][1]["cells"][1]["value"]["value"],
        "/Child"
    );
    assert_eq!(
        first.json()["execution"]["result"]["rows"][0]["identity"]["kind"],
        "node"
    );

    let invalid = server
        .request(
            "POST",
            "/api/v1/queries/execute",
            Some(&json!({
                "source": "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere node.unknown = true\nselect node.id\norder by node.id asc\nlimit 100\n....\n",
                "blockIndex": 0,
                "context": body["context"].clone(),
            })),
        )
        .await;
    assert_eq!(invalid.status, 200);
    assert_eq!(invalid.json()["valid"], false);
    assert!(invalid.json()["execution"]["result"].is_null());
    assert!(
        !invalid.json()["execution"]["analysis"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let mut unknown_today_field = body.clone();
    unknown_today_field["context"]["today"]["extra"] = json!(true);
    let rejected = server
        .request(
            "POST",
            "/api/v1/queries/execute",
            Some(&unknown_today_field),
        )
        .await;
    assert_error(&rejected, 422, "invalid_json");

    let missing_context = server
        .request(
            "POST",
            "/api/v1/queries/execute",
            Some(&json!({
                "source": "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere node.id = this.node.id\nselect node.id\norder by node.id asc\nlimit 10\n....\n",
                "blockIndex": 0,
                "context": {
                    "today": {"year": 2026, "month": 8, "day": 24},
                    "now": "2026-08-24T09:30:00+08:00",
                    "timezone": "Asia/Shanghai",
                    "locale": "zh-CN",
                    "binding": {"nodeId": null, "heading": null}
                }
            })),
        )
        .await;
    assert_error(&missing_context, 422, "missing_context");
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end matrix keeps every task/query route, role, ACL intersection, and non-disclosure comparison in one auditable scenario"
)]
async fn task_and_query_routes_enforce_role_acl_and_non_disclosure_matrix() {
    const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
    const TASK_B: &str = "22222222-2222-4222-8222-222222222222";
    const TASK_R: &str = "33333333-3333-4333-8333-333333333333";
    const MISSING_NODE: &str = "99999999-9999-4999-8999-999999999999";
    const MISSING_TASK: &str = "88888888-8888-4888-8888-888888888888";

    let fixture = TaskFixture::new();
    let server = TestServer::start(&fixture.root).await;
    let owner_cookie = server.cookie.clone().expect("owner cookie");
    let (admin_scope, admin_cookie) =
        create_member_session(&server, "task.admin", "admin password value", "admin").await;
    let (editor_scope, editor_cookie) =
        create_member_session(&server, "task.editor", "editor password value", "editor").await;
    let (_commenter_scope, commenter_cookie) = create_member_session(
        &server,
        "task.commenter",
        "commenter password value",
        "commenter",
    )
    .await;
    let (_viewer_scope, viewer_cookie) =
        create_member_session(&server, "task.viewer", "viewer password value", "viewer").await;

    let root_tasks = format!("/api/v1/tasks/nodes/{}", fixture.root_id);
    let root_document = format!("/api/v1/documents/{}", fixture.root_id);
    let edit_preview_route = format!("{root_tasks}/edit/preview");
    let recurrence_preview_route = format!("{root_tasks}/recurrence/preview");
    let dependencies_preview_route = format!("{root_tasks}/dependencies/preview");
    let query_request = json!({
        "source": "[.weftext-query,version=1,view=table]\n....\nfrom tasks as task\nscope workspace\nwhere true\nselect task.id as task_id, task.owner_node.id as owner_node_id, task.title\norder by task.owner_node.path asc\nlimit 100\n....\n",
        "blockIndex": 0,
        "context": {
            "today": {"year": 2026, "month": 8, "day": 24},
            "now": "2026-08-24T09:30:00+08:00",
            "timezone": "Asia/Shanghai",
            "locale": "zh-CN",
            "binding": {"nodeId": fixture.root_id, "heading": null},
        },
    });
    let roles = [
        ("owner", &owner_cookie, true),
        ("admin", &admin_cookie, true),
        ("editor", &editor_cookie, true),
        ("commenter", &commenter_cookie, false),
        ("viewer", &viewer_cookie, false),
    ];
    let mut owner_plan = None;
    let mut admin_acl_recheck_plan = None;

    for (role, cookie, can_write_tasks) in roles {
        let inspected = server.request_as(cookie, "GET", &root_tasks, None).await;
        assert_eq!(
            inspected.status,
            200,
            "{role} inspect: {}",
            inspected.text()
        );
        assert_eq!(inspected.json()["occurrences"].as_array().unwrap().len(), 2);
        assert!(inspected.text().contains(TASK_A));
        assert!(inspected.text().contains(TASK_R));

        let queried = server
            .request_as(
                cookie,
                "POST",
                "/api/v1/queries/execute",
                Some(&query_request),
            )
            .await;
        assert_eq!(queried.status, 200, "{role} query: {}", queried.text());
        assert_eq!(queried.json()["execution"]["result"]["totalBeforeLimit"], 3);
        assert!(queried.text().contains(TASK_A));
        assert!(queried.text().contains(TASK_B));
        assert!(queried.text().contains(TASK_R));

        let workspace = server
            .request_as(cookie, "GET", "/api/v1/workspace", None)
            .await
            .json();
        let document = server
            .request_as(cookie, "GET", &root_document, None)
            .await
            .json();
        let edit = server
            .request_as(
                cookie,
                "POST",
                &edit_preview_route,
                Some(&json!({
                    "baseWorkspaceRevision": workspace["workspaceRevision"],
                    "baseRevision": document["revision"],
                    "target": {"kind": "id", "id": TASK_A},
                    "intent": {"kind": "toggle"},
                })),
            )
            .await;
        let recurrence = server
            .request_as(
                cookie,
                "POST",
                &recurrence_preview_route,
                Some(&json!({
                    "baseWorkspaceRevision": workspace["workspaceRevision"],
                    "baseRevision": document["revision"],
                    "target": {"kind": "id", "id": TASK_R},
                    "context": {
                        "completedAt": {"kind": "date", "value": "2026-08-24"},
                        "utcOffsetMinutes": 480,
                    },
                })),
            )
            .await;
        let dependencies = server
            .request_as(
                cookie,
                "POST",
                &dependencies_preview_route,
                Some(&json!({
                    "baseWorkspaceRevision": workspace["workspaceRevision"],
                    "baseRevision": document["revision"],
                    "target": {"kind": "id", "id": TASK_A},
                    "dependencies": [TASK_B],
                })),
            )
            .await;
        if can_write_tasks {
            for (name, response) in [
                ("edit", &edit),
                ("recurrence", &recurrence),
                ("dependencies", &dependencies),
            ] {
                assert_eq!(
                    response.status,
                    200,
                    "{role} {name} preview: {}",
                    response.text()
                );
                assert_eq!(response.json()["nodeId"], fixture.root_id.to_string());
                assert_eq!(
                    response.json()["documentChanges"][0]["nodeId"],
                    fixture.root_id.to_string()
                );
            }
            let edit_plan = edit.json()["planId"].as_str().unwrap().to_owned();
            if role == "owner" {
                owner_plan = Some(edit_plan);
            } else if role == "admin" {
                admin_acl_recheck_plan = Some(edit_plan);
            }
        } else {
            for response in [&edit, &recurrence, &dependencies] {
                assert_error(response, 403, "authorization_denied");
                assert!(!response.text().contains(TASK_A));
                assert!(!response.text().contains(TASK_B));
                assert!(!response.text().contains(TASK_R));
            }
        }
    }

    let owner_plan = owner_plan.expect("owner edit plan");
    for (role, cookie) in [("commenter", &commenter_cookie), ("viewer", &viewer_cookie)] {
        let denied = server
            .request_as(
                cookie,
                "POST",
                &format!("/api/v1/tasks/transactions/{owner_plan}/commit"),
                None,
            )
            .await;
        assert_error(&denied, 403, "authorization_denied");
        assert!(
            !denied.text().contains(&owner_plan),
            "{role} learned a plan ID"
        );
        assert!(!denied.text().contains(TASK_A));
    }
    let owner_commit = server
        .request_as(
            &owner_cookie,
            "POST",
            &format!("/api/v1/tasks/transactions/{owner_plan}/commit"),
            None,
        )
        .await;
    assert_eq!(owner_commit.status, 200, "{}", owner_commit.text());
    assert_eq!(owner_commit.json()["auditRecorded"], true);

    for (role, cookie, priority) in [
        ("admin", &admin_cookie, "high"),
        ("editor", &editor_cookie, "highest"),
    ] {
        let workspace = server
            .request_as(cookie, "GET", "/api/v1/workspace", None)
            .await
            .json();
        let document = server
            .request_as(cookie, "GET", &root_document, None)
            .await
            .json();
        let preview = server
            .request_as(
                cookie,
                "POST",
                &edit_preview_route,
                Some(&json!({
                    "baseWorkspaceRevision": workspace["workspaceRevision"],
                    "baseRevision": document["revision"],
                    "target": {"kind": "id", "id": TASK_A},
                    "intent": {"kind": "set_priority", "priority": priority},
                })),
            )
            .await;
        assert_eq!(preview.status, 200, "{role} preview: {}", preview.text());
        let plan_id = preview.json()["planId"].as_str().unwrap().to_owned();
        let committed = server
            .request_as(
                cookie,
                "POST",
                &format!("/api/v1/tasks/transactions/{plan_id}/commit"),
                None,
            )
            .await;
        assert_eq!(committed.status, 200, "{role} commit: {}", committed.text());
        assert_eq!(committed.json()["auditRecorded"], true);
        assert_eq!(
            committed.json()["result"]["task"]["metadata"]["priority"],
            priority
        );
        assert!(
            committed.json()["commit"]["revision"]
                .as_str()
                .expect("projected commit revision")
                .starts_with("actor-v1:")
        );
        assert_eq!(committed.json()["commit"]["pathChanges"], json!([]));
    }

    let owner_recovery = server
        .request_as(&owner_cookie, "POST", "/api/v1/tasks/recover", None)
        .await;
    assert_eq!(owner_recovery.status, 200, "{}", owner_recovery.text());
    for (role, cookie) in [
        ("admin", &admin_cookie),
        ("editor", &editor_cookie),
        ("commenter", &commenter_cookie),
        ("viewer", &viewer_cookie),
    ] {
        let denied = server
            .request_as(cookie, "POST", "/api/v1/tasks/recover", None)
            .await;
        assert_error(&denied, 403, "authorization_denied");
        assert!(
            !denied.text().contains("recovery"),
            "{role} recovery disclosure"
        );
    }

    let admin_read_acl = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": admin_scope,
                "nodeId": fixture.root_id,
                "access": "read",
            })),
        )
        .await;
    assert_eq!(admin_read_acl.status, 200, "{}", admin_read_acl.text());
    let admin_workspace = server
        .request_as(&admin_cookie, "GET", "/api/v1/workspace", None)
        .await
        .json();
    let admin_document = server
        .request_as(&admin_cookie, "GET", &root_document, None)
        .await
        .json();
    let read_only_requests = [
        (
            &edit_preview_route,
            json!({
                "baseWorkspaceRevision": admin_workspace["workspaceRevision"],
                "baseRevision": admin_document["revision"],
                "target": {"kind": "id", "id": TASK_A},
                "intent": {"kind": "toggle"},
            }),
        ),
        (
            &recurrence_preview_route,
            json!({
                "baseWorkspaceRevision": admin_workspace["workspaceRevision"],
                "baseRevision": admin_document["revision"],
                "target": {"kind": "id", "id": TASK_R},
                "context": {
                    "completedAt": {"kind": "date", "value": "2026-08-24"},
                    "utcOffsetMinutes": 480,
                },
            }),
        ),
        (
            &dependencies_preview_route,
            json!({
                "baseWorkspaceRevision": admin_workspace["workspaceRevision"],
                "baseRevision": admin_document["revision"],
                "target": {"kind": "id", "id": TASK_A},
                "dependencies": [TASK_B],
            }),
        ),
    ];
    for (route, body) in read_only_requests {
        let denied = server
            .request_as(&admin_cookie, "POST", route, Some(&body))
            .await;
        assert_error(&denied, 403, "authorization_denied");
        assert!(!denied.text().contains(TASK_A));
        assert!(!denied.text().contains(TASK_B));
        assert!(!denied.text().contains(TASK_R));
    }
    let acl_rechecked_commit = server
        .request_as(
            &admin_cookie,
            "POST",
            &format!(
                "/api/v1/tasks/transactions/{}/commit",
                admin_acl_recheck_plan.expect("admin staged plan")
            ),
            None,
        )
        .await;
    assert_error(&acl_rechecked_commit, 403, "authorization_denied");
    let admin_inspect = server
        .request_as(&admin_cookie, "GET", &root_tasks, None)
        .await;
    assert_eq!(admin_inspect.status, 200);
    let admin_query = server
        .request_as(
            &admin_cookie,
            "POST",
            "/api/v1/queries/execute",
            Some(&query_request),
        )
        .await;
    assert_eq!(admin_query.status, 200);

    let editor_hidden = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": editor_scope,
                "nodeId": fixture.child_id,
                "access": "hidden",
            })),
        )
        .await;
    assert_eq!(editor_hidden.status, 200, "{}", editor_hidden.text());
    let hidden_inspect = server
        .request_as(
            &editor_cookie,
            "GET",
            &format!("/api/v1/tasks/nodes/{}", fixture.child_id),
            None,
        )
        .await;
    let missing_inspect = server
        .request_as(
            &editor_cookie,
            "GET",
            &format!("/api/v1/tasks/nodes/{MISSING_NODE}"),
            None,
        )
        .await;
    assert_error(&hidden_inspect, 404, "node_not_found");
    assert_eq!(hidden_inspect.json(), missing_inspect.json());

    let filtered_query = server
        .request_as(
            &editor_cookie,
            "POST",
            "/api/v1/queries/execute",
            Some(&query_request),
        )
        .await;
    assert_eq!(filtered_query.status, 200, "{}", filtered_query.text());
    assert_eq!(
        filtered_query.json()["execution"]["result"]["totalBeforeLimit"],
        2
    );
    for hidden in [TASK_B, "Dependency task", &fixture.child_id.to_string()] {
        assert!(!filtered_query.text().contains(hidden));
    }
    let filtered_validation = server
        .request_as(&editor_cookie, "GET", "/api/v1/tasks/validate", None)
        .await;
    assert_eq!(
        filtered_validation.status,
        200,
        "{}",
        filtered_validation.text()
    );
    assert!(!filtered_validation.text().contains(TASK_B));
    assert!(!filtered_validation.text().contains("Dependency task"));

    let editor_workspace = server
        .request_as(&editor_cookie, "GET", "/api/v1/workspace", None)
        .await
        .json();
    let editor_document = server
        .request_as(&editor_cookie, "GET", &root_document, None)
        .await
        .json();
    let dependency_request = |dependency: &str| {
        json!({
            "baseWorkspaceRevision": editor_workspace["workspaceRevision"],
            "baseRevision": editor_document["revision"],
            "target": {"kind": "id", "id": TASK_A},
            "dependencies": [dependency],
        })
    };
    let hidden_dependency = server
        .request_as(
            &editor_cookie,
            "POST",
            &dependencies_preview_route,
            Some(&dependency_request(TASK_B)),
        )
        .await;
    let missing_dependency = server
        .request_as(
            &editor_cookie,
            "POST",
            &dependencies_preview_route,
            Some(&dependency_request(MISSING_TASK)),
        )
        .await;
    assert_error(&hidden_dependency, 422, "task_transaction_rejected");
    assert_eq!(hidden_dependency.json(), missing_dependency.json());
    assert!(!hidden_dependency.text().contains(TASK_B));

    let scoped_query = |node_id: &str| {
        json!({
            "source": "[.weftext-query,version=1,view=table]\n....\nfrom tasks as task\nscope subtree(this.node)\nwhere true\nselect task.title\norder by task.title asc\nlimit 100\n....\n",
            "blockIndex": 0,
            "context": {
                "today": {"year": 2026, "month": 8, "day": 24},
                "now": "2026-08-24T09:30:00+08:00",
                "timezone": "Asia/Shanghai",
                "locale": "zh-CN",
                "binding": {"nodeId": node_id, "heading": null},
            },
        })
    };
    let hidden_scope = server
        .request_as(
            &editor_cookie,
            "POST",
            "/api/v1/queries/execute",
            Some(&scoped_query(&fixture.child_id.to_string())),
        )
        .await;
    let missing_scope = server
        .request_as(
            &editor_cookie,
            "POST",
            "/api/v1/queries/execute",
            Some(&scoped_query(MISSING_NODE)),
        )
        .await;
    assert_error(&hidden_scope, 422, "query_rejected");
    assert_eq!(hidden_scope.json(), missing_scope.json());
    assert!(!hidden_scope.text().contains(&fixture.child_id.to_string()));
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn multirole_sessions_node_acl_non_disclosure_restart_and_audit_are_enforced() {
    let fixture = Fixture::new();
    let grandchild_plan = plan_create_child_node(&fixture.root, fixture.child_id, "Grandchild")
        .expect("plan grandchild");
    let grandchild_id = grandchild_plan.generated_node_ids[0];
    commit_workspace_transaction(&grandchild_plan).expect("commit grandchild");
    replace_document(&fixture.root.join("Child/Grandchild"), |source| {
        format!(
            "{source}\nPromotedNeedle\n\n* [ ] Promoted task:[id=44444444-4444-4444-8444-444444444444]\n"
        )
    });
    let mut server = TestServer::start(&fixture.root).await;

    let admin = server
        .request(
            "POST",
            "/api/v1/admin/members",
            Some(&json!({
                "login": "admin.one",
                "password": "admin password value",
                "role": "admin"
            })),
        )
        .await;
    assert_eq!(admin.status, 200, "{}", admin.text());
    let admin_scope = admin.json()["actorScope"]
        .as_str()
        .expect("admin scope")
        .to_owned();
    let editor = server
        .request(
            "POST",
            "/api/v1/admin/members",
            Some(&json!({
                "login": "editor.one",
                "password": "editor password value",
                "role": "editor"
            })),
        )
        .await;
    assert_eq!(editor.status, 200, "{}", editor.text());
    let editor_scope = editor.json()["actorScope"]
        .as_str()
        .expect("editor scope")
        .to_owned();
    let commenter = server
        .request(
            "POST",
            "/api/v1/admin/members",
            Some(&json!({
                "login": "commenter.one",
                "password": "commenter password value",
                "role": "commenter"
            })),
        )
        .await;
    assert_eq!(commenter.status, 200, "{}", commenter.text());
    let commenter_scope = commenter.json()["actorScope"]
        .as_str()
        .expect("commenter scope")
        .to_owned();
    let viewer = server
        .request(
            "POST",
            "/api/v1/admin/members",
            Some(&json!({
                "login": "viewer.one",
                "password": "viewer password value",
                "role": "viewer"
            })),
        )
        .await;
    assert_eq!(viewer.status, 200, "{}", viewer.text());
    let viewer_scope = viewer.json()["actorScope"]
        .as_str()
        .expect("viewer scope")
        .to_owned();
    let owner_scope = server
        .request("GET", "/api/v1/auth/session", None)
        .await
        .json()["actorScope"]
        .as_str()
        .expect("owner scope")
        .to_owned();
    let last_owner_guard = server
        .request(
            "PUT",
            &format!("/api/v1/admin/members/{owner_scope}"),
            Some(&json!({"role": "viewer", "enabled": true})),
        )
        .await;
    assert_error(&last_owner_guard, 409, "last_owner_required");

    let admin_login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({"login": "admin.one", "password": "admin password value"})),
        )
        .await;
    assert_eq!(admin_login.status, 200, "{}", admin_login.text());
    assert_eq!(admin_login.json()["role"], "admin");
    assert_eq!(admin_login.json()["capabilities"]["manageMembers"], true);
    assert_eq!(admin_login.json()["capabilities"]["manageWorkspace"], false);
    let admin_cookie = admin_login.session_cookie().expect("admin session cookie");
    let editor_login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({"login": "editor.one", "password": "editor password value"})),
        )
        .await;
    assert_eq!(editor_login.status, 200, "{}", editor_login.text());
    assert_eq!(editor_login.json()["role"], "editor");
    let editor_cookie = editor_login
        .session_cookie()
        .expect("editor session cookie");
    let commenter_login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({
                "login": "commenter.one",
                "password": "commenter password value"
            })),
        )
        .await;
    assert_eq!(commenter_login.status, 200, "{}", commenter_login.text());
    assert_eq!(commenter_login.json()["role"], "commenter");
    assert_eq!(
        commenter_login.json()["capabilities"]["writeAnnotations"],
        true
    );
    assert_eq!(
        commenter_login.json()["capabilities"]["editDocuments"],
        false
    );
    let commenter_cookie = commenter_login
        .session_cookie()
        .expect("commenter session cookie");
    let viewer_login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({"login": "viewer.one", "password": "viewer password value"})),
        )
        .await;
    assert_eq!(viewer_login.status, 200, "{}", viewer_login.text());
    assert_eq!(viewer_login.json()["role"], "viewer");
    let viewer_cookie = viewer_login
        .session_cookie()
        .expect("viewer session cookie");

    let child_document = format!("/api/v1/documents/{}", fixture.child_id);
    let admin_members = server
        .request_as(&admin_cookie, "GET", "/api/v1/admin/members", None)
        .await;
    assert_eq!(admin_members.status, 200, "{}", admin_members.text());
    let admin_owner_escalation = server
        .request_as(
            &admin_cookie,
            "POST",
            "/api/v1/admin/members",
            Some(&json!({
                "login": "forbidden.owner",
                "password": "forbidden owner password",
                "role": "owner"
            })),
        )
        .await;
    assert_error(&admin_owner_escalation, 403, "authorization_denied");
    let admin_owner_change = server
        .request_as(
            &admin_cookie,
            "PUT",
            &format!("/api/v1/admin/members/{owner_scope}"),
            Some(&json!({"role": "admin", "enabled": true})),
        )
        .await;
    assert_error(&admin_owner_change, 403, "authorization_denied");
    let admin_acl = server
        .request_as(
            &admin_cookie,
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": admin_scope,
                "nodeId": fixture.root_id,
                "access": "write"
            })),
        )
        .await;
    assert_eq!(admin_acl.status, 200, "{}", admin_acl.text());
    let admin_workspace_recovery = server
        .request_as(&admin_cookie, "POST", "/api/v1/tasks/recover", None)
        .await;
    assert_error(&admin_workspace_recovery, 403, "authorization_denied");

    for (role, cookie, can_edit, can_manage_members, can_mutate_structure) in [
        ("admin", &admin_cookie, true, true, true),
        ("editor", &editor_cookie, true, false, true),
        ("commenter", &commenter_cookie, false, false, false),
        ("viewer", &viewer_cookie, false, false, false),
    ] {
        let opened = server
            .request_as(cookie, "GET", &child_document, None)
            .await;
        assert_eq!(opened.status, 200, "{role} read: {}", opened.text());
        let preview = server
            .request_as(
                cookie,
                "POST",
                &format!("{child_document}/preview"),
                Some(&json!({
                    "baseRevision": opened.json()["revision"],
                    "source": opened.json()["source"]
                })),
            )
            .await;
        if can_edit {
            assert_eq!(preview.status, 200, "{role} preview: {}", preview.text());
        } else {
            assert_error(&preview, 403, "authorization_denied");
        }

        for admin_path in [
            "/api/v1/admin/members",
            "/api/v1/admin/node-acl",
            "/api/v1/admin/audit",
        ] {
            let admin_response = server.request_as(cookie, "GET", admin_path, None).await;
            if can_manage_members {
                assert_eq!(
                    admin_response.status,
                    200,
                    "{role} {admin_path}: {}",
                    admin_response.text()
                );
            } else {
                assert_error(&admin_response, 403, "authorization_denied");
            }
        }

        let structure = server
            .request_as(
                cookie,
                "POST",
                "/api/v1/tasks/transactions/not-a-plan/commit",
                None,
            )
            .await;
        if can_mutate_structure {
            assert_error(&structure, 404, "task_plan_unavailable");
        } else {
            assert_error(&structure, 403, "authorization_denied");
        }
    }

    let editor_open = server
        .request_as(&editor_cookie, "GET", &child_document, None)
        .await;
    assert_eq!(editor_open.status, 200);
    let editor_source = format!(
        "{}\nEdited by a real Editor session",
        editor_open.json()["source"]
            .as_str()
            .expect("editor source")
    );
    let editor_commit = server
        .request_as(
            &editor_cookie,
            "PUT",
            &child_document,
            Some(&json!({
                "baseRevision": editor_open.json()["revision"],
                "source": editor_source
            })),
        )
        .await;
    assert_eq!(editor_commit.status, 200, "{}", editor_commit.text());
    let editor_admin = server
        .request_as(&editor_cookie, "GET", "/api/v1/admin/members", None)
        .await;
    assert_error(&editor_admin, 403, "authorization_denied");

    let viewer_open = server
        .request_as(&viewer_cookie, "GET", &child_document, None)
        .await;
    assert_eq!(viewer_open.status, 200);
    let viewer_write = server
        .request_as(
            &viewer_cookie,
            "PUT",
            &child_document,
            Some(&json!({
                "baseRevision": viewer_open.json()["revision"],
                "source": viewer_open.json()["source"]
            })),
        )
        .await;
    assert_error(&viewer_write, 403, "authorization_denied");

    let annotation_route = format!("/api/v1/annotations/{}", fixture.child_id);
    let editor_annotations = server
        .request_as(&editor_cookie, "GET", &annotation_route, None)
        .await;
    assert_eq!(editor_annotations.status, 200);
    let editor_annotation = mutate_annotation_as(
        &server,
        &editor_cookie,
        &annotation_route,
        &editor_annotations.json(),
        json!({
            "action": "create",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": null,
            "labels": [],
            "bodySource": "Editor annotation",
            "suggestedSource": null
        }),
    )
    .await;
    assert_eq!(
        editor_annotation.status,
        200,
        "{}",
        editor_annotation.text()
    );
    let annotation_id = editor_annotation.json()["store"]["annotations"][0]["id"].clone();
    let commenter_inherited_read = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": commenter_scope,
                "nodeId": fixture.root_id,
                "access": "read"
            })),
        )
        .await;
    assert_eq!(
        commenter_inherited_read.status,
        200,
        "{}",
        commenter_inherited_read.text()
    );
    let commenter_read_state = server
        .request_as(&commenter_cookie, "GET", &annotation_route, None)
        .await;
    assert_eq!(commenter_read_state.status, 200);
    let inherited_read_write = mutate_annotation_as(
        &server,
        &commenter_cookie,
        &annotation_route,
        &commenter_read_state.json(),
        json!({
            "action": "reply",
            "annotationId": annotation_id,
            "bodySource": "inherited read must not permit annotation writes"
        }),
    )
    .await;
    assert_error(&inherited_read_write, 403, "authorization_denied");
    let commenter_child_write = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": commenter_scope,
                "nodeId": fixture.child_id,
                "access": "write"
            })),
        )
        .await;
    assert_eq!(commenter_child_write.status, 200);
    for (role, cookie) in [("admin", &admin_cookie), ("commenter", &commenter_cookie)] {
        let annotation_state = server
            .request_as(cookie, "GET", &annotation_route, None)
            .await;
        assert_eq!(
            annotation_state.status,
            200,
            "{role} annotation read: {}",
            annotation_state.text()
        );
        let reply = mutate_annotation_as(
            &server,
            cookie,
            &annotation_route,
            &annotation_state.json(),
            json!({
                "action": "reply",
                "annotationId": annotation_id,
                "bodySource": format!("{role} reply")
            }),
        )
        .await;
        assert_eq!(reply.status, 200, "{role} annotation: {}", reply.text());
    }
    let viewer_annotations = server
        .request_as(&viewer_cookie, "GET", &annotation_route, None)
        .await;
    assert_eq!(viewer_annotations.status, 200);
    let viewer_annotation_write = mutate_annotation_as(
        &server,
        &viewer_cookie,
        &annotation_route,
        &viewer_annotations.json(),
        json!({
            "action": "reply",
            "annotationId": annotation_id,
            "bodySource": "Viewer must not write"
        }),
    )
    .await;
    assert_error(&viewer_annotation_write, 403, "authorization_denied");

    let hidden = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": viewer_scope,
                "nodeId": fixture.child_id,
                "access": "hidden"
            })),
        )
        .await;
    assert_eq!(hidden.status, 200, "{}", hidden.text());
    let hidden_document = server
        .request_as(&viewer_cookie, "GET", &child_document, None)
        .await;
    let missing_document = server
        .request_as(
            &viewer_cookie,
            "GET",
            "/api/v1/documents/99999999-9999-4999-8999-999999999999",
            None,
        )
        .await;
    assert_error(&hidden_document, 404, "node_not_found");
    assert_eq!(hidden_document.json(), missing_document.json());
    let hidden_annotations = server
        .request_as(&viewer_cookie, "GET", &annotation_route, None)
        .await;
    let missing_annotations = server
        .request_as(
            &viewer_cookie,
            "GET",
            "/api/v1/annotations/99999999-9999-4999-8999-999999999999",
            None,
        )
        .await;
    assert_error(&hidden_annotations, 404, "node_not_found");
    assert_eq!(hidden_annotations.json(), missing_annotations.json());
    let hidden_search = server
        .request_as(&viewer_cookie, "GET", "/api/v1/search?q=Needle", None)
        .await;
    assert_eq!(hidden_search.status, 200);
    assert_eq!(hidden_search.json()["results"], json!([]));
    let hidden_inventory = server
        .request_as(&viewer_cookie, "GET", "/api/v1/workspace", None)
        .await;
    assert_eq!(hidden_inventory.status, 200);
    let hidden_body = hidden_inventory.text();
    assert!(!hidden_body.contains("Child"));
    assert!(!hidden_body.contains("Grandchild"));
    let hidden_revision = hidden_inventory.json()["workspaceRevision"]
        .as_str()
        .expect("actor-scoped revision")
        .to_owned();
    assert!(hidden_revision.starts_with("actor-v1:"));
    let owner_inventory = server.request("GET", "/api/v1/workspace", None).await;
    assert_ne!(
        hidden_revision,
        owner_inventory.json()["workspaceRevision"]
            .as_str()
            .expect("Owner authority revision")
    );
    let owner_hidden_document = server.request("GET", &child_document, None).await;
    let hidden_source = format!(
        "{}\nHidden revision mutation",
        owner_hidden_document.json()["source"]
            .as_str()
            .expect("hidden source")
    );
    let hidden_commit = server
        .request(
            "PUT",
            &child_document,
            Some(&json!({
                "baseRevision": owner_hidden_document.json()["revision"],
                "source": hidden_source
            })),
        )
        .await;
    assert_eq!(hidden_commit.status, 200, "{}", hidden_commit.text());
    let hidden_after_unrelated_change = server
        .request_as(&viewer_cookie, "GET", "/api/v1/workspace", None)
        .await;
    assert_eq!(
        hidden_after_unrelated_change.json()["workspaceRevision"],
        hidden_revision
    );

    let filtered_query = server
        .request_as(
            &viewer_cookie,
            "POST",
            "/api/v1/queries/execute",
            Some(&json!({
                "source": "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n....\n",
                "blockIndex": 0,
                "context": {
                    "today": {"year": 2026, "month": 8, "day": 24},
                    "now": "2026-08-24T09:30:00+08:00",
                    "timezone": "Asia/Shanghai",
                    "locale": "zh-CN",
                    "binding": {"nodeId": fixture.root_id, "heading": null},
                }
            })),
        )
        .await;
    assert_eq!(filtered_query.status, 200, "{}", filtered_query.text());
    let query_text = filtered_query.text();
    assert!(
        filtered_query.json()["workspaceRevision"]
            .as_str()
            .expect("query projection revision")
            .starts_with("actor-v1:")
    );
    assert!(!query_text.contains("Child"));
    assert!(!query_text.contains("Grandchild"));

    for route in ["/api/v1/tasks/validate", "/api/v1/citations/validate"] {
        let filtered = server.request_as(&viewer_cookie, "GET", route, None).await;
        assert_eq!(filtered.status, 200, "{}", filtered.text());
        let filtered_text = filtered.text();
        assert!(!filtered_text.contains("Child"));
        assert!(!filtered_text.contains("Grandchild"));
        assert!(!filtered_text.contains(&fixture.child_id.to_string()));
        assert!(!filtered_text.contains(&grandchild_id.to_string()));
    }

    let explicit_grandchild = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": viewer_scope,
                "nodeId": grandchild_id,
                "access": "read"
            })),
        )
        .await;
    assert_eq!(
        explicit_grandchild.status,
        200,
        "{}",
        explicit_grandchild.text()
    );
    let promoted_inventory = server
        .request_as(&viewer_cookie, "GET", "/api/v1/workspace", None)
        .await;
    assert_eq!(promoted_inventory.status, 200);
    let promoted_json = promoted_inventory.json();
    let promoted_text = promoted_inventory.text();
    assert!(promoted_text.contains("Grandchild"));
    assert!(!promoted_text.contains("Child/Grandchild"));
    assert!(!promoted_text.contains(&fixture.child_id.to_string()));
    let promoted_node = promoted_json["navigation"]["hierarchy"]
        .as_array()
        .expect("promoted hierarchy")
        .iter()
        .find(|node| node["nodeId"] == grandchild_id.to_string())
        .expect("promoted node");
    assert!(promoted_node["parentNodeId"].is_null());
    assert_eq!(promoted_node["locator"], "Grandchild");
    assert_eq!(promoted_node["depth"], 0);

    let promoted_search = server
        .request_as(
            &viewer_cookie,
            "GET",
            "/api/v1/search?q=PromotedNeedle",
            None,
        )
        .await;
    assert_eq!(promoted_search.status, 200, "{}", promoted_search.text());
    assert_eq!(
        promoted_search.json()["results"][0]["id"],
        grandchild_id.to_string()
    );
    assert_eq!(promoted_search.json()["results"][0]["path"], "Grandchild");
    assert!(
        !promoted_search
            .text()
            .contains(&fixture.child_id.to_string())
    );

    let promoted_tasks = server
        .request_as(&viewer_cookie, "GET", "/api/v1/tasks/validate", None)
        .await;
    assert_eq!(promoted_tasks.status, 200, "{}", promoted_tasks.text());
    assert!(promoted_tasks.text().contains(&grandchild_id.to_string()));
    assert!(
        !promoted_tasks
            .text()
            .contains(&fixture.child_id.to_string())
    );

    let promoted_citations = server
        .request_as(&viewer_cookie, "GET", "/api/v1/citations/validate", None)
        .await;
    assert_eq!(
        promoted_citations.status,
        200,
        "{}",
        promoted_citations.text()
    );
    assert!(
        promoted_citations
            .text()
            .contains(&grandchild_id.to_string())
    );
    assert!(
        !promoted_citations
            .text()
            .contains(&fixture.child_id.to_string())
    );
    assert!(!promoted_citations.text().contains("Child/Grandchild"));

    let promoted_query = server
        .request_as(
            &viewer_cookie,
            "POST",
            "/api/v1/queries/execute",
            Some(&json!({
                "source": "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n....\n",
                "blockIndex": 0,
                "context": {
                    "today": {"year": 2026, "month": 8, "day": 24},
                    "now": "2026-08-24T09:30:00+08:00",
                    "timezone": "Asia/Shanghai",
                    "locale": "zh-CN",
                    "binding": {"nodeId": fixture.root_id, "heading": null},
                }
            })),
        )
        .await;
    assert_eq!(promoted_query.status, 200, "{}", promoted_query.text());
    assert!(promoted_query.text().contains("Grandchild"));
    assert!(!promoted_query.text().contains("Child/Grandchild"));
    assert!(
        !promoted_query
            .text()
            .contains(&fixture.child_id.to_string())
    );

    let promoted_revision = promoted_json["workspaceRevision"]
        .as_str()
        .expect("promoted actor revision")
        .to_owned();
    let child_document_path = fixture.root.join("Child/Child.adoc");
    let original_child_document = std::fs::read(&child_document_path).expect("child document");
    std::fs::write(&child_document_path, [0xff, 0xfe, 0xfd])
        .expect("make hidden parent body invalid UTF-8");

    let invalid_hidden_inventory = server
        .request_as(&viewer_cookie, "GET", "/api/v1/workspace", None)
        .await;
    assert_eq!(
        invalid_hidden_inventory.status,
        200,
        "{}",
        invalid_hidden_inventory.text()
    );
    assert_eq!(
        invalid_hidden_inventory.json()["workspaceRevision"],
        promoted_revision
    );
    assert!(invalid_hidden_inventory.text().contains("Grandchild"));
    assert!(!invalid_hidden_inventory.text().contains("Child/Grandchild"));
    assert!(
        !invalid_hidden_inventory
            .text()
            .contains("\"locator\":\"Child\"")
    );
    let invalid_hidden_search = server
        .request_as(
            &viewer_cookie,
            "GET",
            "/api/v1/search?q=PromotedNeedle",
            None,
        )
        .await;
    assert_eq!(
        invalid_hidden_search.status,
        200,
        "{}",
        invalid_hidden_search.text()
    );
    assert_eq!(
        invalid_hidden_search.json()["results"][0]["path"],
        "Grandchild"
    );
    let promoted_annotation_route = format!("/api/v1/annotations/{grandchild_id}");
    for route in [
        "/api/v1/tasks/validate",
        "/api/v1/citations/validate",
        promoted_annotation_route.as_str(),
    ] {
        let response = server.request_as(&viewer_cookie, "GET", route, None).await;
        assert_eq!(response.status, 200, "{route}: {}", response.text());
        assert!(!response.text().contains("Child/Grandchild"));
        assert!(!response.text().contains(&fixture.child_id.to_string()));
    }
    let owner_invalid_inventory = server.request("GET", "/api/v1/workspace", None).await;
    assert_error(&owner_invalid_inventory, 503, "workspace_invalid");
    std::fs::write(&child_document_path, &original_child_document)
        .expect("restore hidden parent body");

    let shadow = fixture.root.join("Shadow");
    std::fs::create_dir(&shadow).expect("create hidden duplicate node");
    let child_source = String::from_utf8(original_child_document.clone()).expect("UTF-8 child");
    std::fs::write(
        shadow.join("Shadow.adoc"),
        child_source.replacen("= Child", "= Shadow", 1),
    )
    .expect("write hidden duplicate identity");
    let duplicate_hidden_inventory = server
        .request_as(&viewer_cookie, "GET", "/api/v1/workspace", None)
        .await;
    assert_eq!(
        duplicate_hidden_inventory.status,
        200,
        "{}",
        duplicate_hidden_inventory.text()
    );
    assert_eq!(
        duplicate_hidden_inventory.json()["workspaceRevision"],
        promoted_revision
    );
    assert!(!duplicate_hidden_inventory.text().contains("Shadow"));
    let duplicate_hidden_search = server
        .request_as(
            &viewer_cookie,
            "GET",
            "/api/v1/search?q=PromotedNeedle",
            None,
        )
        .await;
    assert_eq!(
        duplicate_hidden_search.status,
        200,
        "{}",
        duplicate_hidden_search.text()
    );
    let owner_duplicate_inventory = server.request("GET", "/api/v1/workspace", None).await;
    assert_error(&owner_duplicate_inventory, 503, "workspace_invalid");
    std::fs::remove_dir_all(shadow).expect("remove hidden duplicate fixture");

    for (node_id, access) in [(fixture.child_id, "hidden"), (grandchild_id, "write")] {
        let response = server
            .request(
                "PUT",
                "/api/v1/admin/node-acl",
                Some(&json!({
                    "actorScope": editor_scope,
                    "nodeId": node_id,
                    "access": access,
                })),
            )
            .await;
        assert_eq!(response.status, 200, "{}", response.text());
    }
    let editor_workspace = server
        .request_as(&editor_cookie, "GET", "/api/v1/workspace", None)
        .await;
    let editor_grandchild = server
        .request_as(
            &editor_cookie,
            "GET",
            &format!("/api/v1/documents/{grandchild_id}"),
            None,
        )
        .await;
    let editor_task_preview = server
        .request_as(
            &editor_cookie,
            "POST",
            &format!("/api/v1/tasks/nodes/{grandchild_id}/edit/preview"),
            Some(&json!({
                "baseWorkspaceRevision": editor_workspace.json()["workspaceRevision"],
                "baseRevision": editor_grandchild.json()["revision"],
                "target": {
                    "kind": "id",
                    "id": "44444444-4444-4444-8444-444444444444",
                },
                "intent": {"kind": "set_priority", "priority": "high"},
            })),
        )
        .await;
    assert_eq!(
        editor_task_preview.status,
        200,
        "{}",
        editor_task_preview.text()
    );
    assert_eq!(
        editor_task_preview.json()["documentChanges"][0]["path"],
        "Grandchild/Grandchild.adoc"
    );
    assert!(!editor_task_preview.text().contains("Child/Grandchild"));
    let editor_task_commit = server
        .request_as(
            &editor_cookie,
            "POST",
            &format!(
                "/api/v1/tasks/transactions/{}/commit",
                editor_task_preview.json()["planId"]
                    .as_str()
                    .expect("editor task plan")
            ),
            None,
        )
        .await;
    assert_eq!(
        editor_task_commit.status,
        200,
        "{}",
        editor_task_commit.text()
    );
    assert!(
        editor_task_commit.json()["commit"]["baseRevision"]
            .as_str()
            .expect("projected base revision")
            .starts_with("actor-v1:")
    );
    assert!(
        editor_task_commit.json()["commit"]["revision"]
            .as_str()
            .expect("projected commit revision")
            .starts_with("actor-v1:")
    );
    assert!(!editor_task_commit.text().contains("Child/Grandchild"));

    server.restart(&fixture.root).await;
    let restarted_admin_session = server
        .request_as(&admin_cookie, "GET", "/api/v1/auth/session", None)
        .await;
    assert_eq!(restarted_admin_session.status, 200);
    assert_eq!(restarted_admin_session.json()["role"], "admin");
    assert_eq!(
        restarted_admin_session.json()["capabilities"]["manageMembers"],
        true
    );
    let restarted_admin_audit = server
        .request_as(&admin_cookie, "GET", "/api/v1/admin/audit", None)
        .await;
    assert_eq!(
        restarted_admin_audit.status,
        200,
        "{}",
        restarted_admin_audit.text()
    );
    let restarted_commenter_annotations = server
        .request_as(&commenter_cookie, "GET", &annotation_route, None)
        .await;
    assert_eq!(restarted_commenter_annotations.status, 200);
    let persisted_hidden = server
        .request_as(&viewer_cookie, "GET", &child_document, None)
        .await;
    assert_error(&persisted_hidden, 404, "node_not_found");
    let persisted_grandchild = server
        .request_as(
            &viewer_cookie,
            "GET",
            &format!("/api/v1/documents/{grandchild_id}"),
            None,
        )
        .await;
    assert_eq!(persisted_grandchild.status, 200);

    let audit = server.request("GET", "/api/v1/admin/audit", None).await;
    assert_eq!(audit.status, 200, "{}", audit.text());
    let audit_text = audit.text();
    assert!(audit_text.contains("member_created"));
    assert!(audit_text.contains("node_acl_updated"));
    assert!(audit_text.contains("document_edited"));
    assert!(!audit_text.contains("Edited by a real Editor session"));
    let filtered_audit = server
        .request(
            "GET",
            "/api/v1/admin/audit?eventType=node_acl_updated&limit=1",
            None,
        )
        .await;
    assert_eq!(filtered_audit.status, 200, "{}", filtered_audit.text());
    assert_eq!(
        filtered_audit
            .json()
            .as_array()
            .expect("audit receipts")
            .len(),
        1
    );
    assert_eq!(filtered_audit.json()[0]["eventType"], "node_acl_updated");

    let demoted = server
        .request(
            "PUT",
            &format!("/api/v1/admin/members/{editor_scope}"),
            Some(&json!({"role": "viewer", "enabled": true})),
        )
        .await;
    assert_eq!(demoted.status, 200, "{}", demoted.text());
    let revoked_editor = server
        .request_as(&editor_cookie, "GET", "/api/v1/workspace", None)
        .await;
    assert_error(&revoked_editor, 401, "authentication_required");

    let disabled = server
        .request(
            "PUT",
            &format!("/api/v1/admin/members/{viewer_scope}"),
            Some(&json!({"role": "viewer", "enabled": false})),
        )
        .await;
    assert_eq!(disabled.status, 200, "{}", disabled.text());
    let revoked_viewer = server
        .request_as(&viewer_cookie, "GET", "/api/v1/workspace", None)
        .await;
    assert_error(&revoked_viewer, 401, "authentication_required");
}

#[tokio::test]
async fn server_inventory_uses_core_content_boundary_without_ignored_disclosure() {
    let temporary = tempfile::tempdir().expect("temporary fixture copy");
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/content-boundary-v02");
    let root = temporary.path().join("content-boundary-v02");
    copy_tree(&source, &root);
    std::fs::write(
        root.join("weftext.annotations.json"),
        "{\"version\":3,\"document_id\":\"11111111-1111-4111-8111-111111111111\",\"annotations\":[]}\n",
    )
    .expect("root annotation sidecar");
    let server = TestServer::start(&root).await;

    let response = server.request("GET", "/api/v1/workspace", None).await;
    assert_eq!(response.status, 200);
    let body = response.text();
    assert!(!body.contains("ignored"));
    assert!(!body.contains("IgnoredSearchToken"));
    assert!(!body.contains("weftext.annotations.json"));
    let payload: Value = serde_json::from_str(&body).expect("inventory JSON");
    assert_eq!(payload["nodes"].as_array().expect("nodes").len(), 2);
    assert_eq!(payload["navigation"]["version"], 1);
    assert_eq!(
        payload["navigation"]["hierarchy"]
            .as_array()
            .expect("hierarchy")
            .iter()
            .map(|entry| entry["locator"].as_str().expect("locator"))
            .collect::<Vec<_>>(),
        vec!["", "Managed"]
    );
    assert_eq!(
        payload["navigation"]["contents"]
            .as_array()
            .expect("navigation contents")
            .iter()
            .filter(|entry| entry["parentLocator"] == "")
            .map(|entry| entry["locator"].as_str().expect("locator"))
            .collect::<Vec<_>>(),
        vec!["Managed", "Files", "loose.md", "resource.bin"]
    );
    let content = payload["content"].as_array().expect("content");
    assert!(
        content.iter().any(|entry| {
            entry["kind"] == "unmanaged_markdown" && entry["locator"] == "loose.md"
        })
    );
    assert!(content.iter().any(|entry| {
        entry["kind"] == "unmanaged_directory" && entry["locator"] == "Files/nested/LooksLikeNode"
    }));
    assert!(
        content
            .iter()
            .all(|entry| entry["locator"] != "ignored/secret.md")
    );
    let root_node = payload["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["locator"] == "")
        .expect("root node");
    assert_eq!(root_node["displayIcon"]["kind"], "explicit_node");

    for query in ["UnmanagedSearchToken", "IgnoredSearchToken"] {
        let search = server
            .request("GET", &format!("/api/v1/search?q={query}"), None)
            .await;
        assert_eq!(search.status, 200);
        assert!(
            search.json()["results"]
                .as_array()
                .expect("results")
                .is_empty()
        );
    }
}

#[tokio::test]
async fn stale_revision_returns_structured_conflict_without_overwrite() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;
    let path = format!("/api/v1/documents/{}", fixture.child_id);
    let opened = server.request("GET", &path, None).await.json();
    let base = opened["revision"].as_str().expect("base");
    let original = opened["source"].as_str().expect("source");

    let first_source = format!("{original}\nfirst");
    let first = server
        .request(
            "PUT",
            &path,
            Some(&json!({ "baseRevision": base, "source": first_source })),
        )
        .await;
    assert_eq!(first.status, 200);

    let stale = server
        .request(
            "PUT",
            &path,
            Some(&json!({ "baseRevision": base, "source": format!("{original}\nstale") })),
        )
        .await;
    assert_eq!(stale.status, 409);
    let error = stale.json()["error"].clone();
    assert_eq!(error["code"], "stale_revision");
    assert_eq!(error["conflict"]["expectedRevision"], base);
    assert_eq!(
        error["conflict"]["actualRevision"],
        first.json()["revision"]
    );
    assert!(
        read_node_document(fixture.root.join("Child"))
            .expect("Core read")
            .source
            .ends_with("first")
    );
}

#[tokio::test]
async fn uuid_routes_confine_requests_and_commits_to_the_hosted_workspace() {
    let fixture = Fixture::new();
    let outside = fixture
        .root
        .parent()
        .expect("workspace parent")
        .join("outside.txt");
    std::fs::write(&outside, "outside sentinel").expect("outside sentinel");
    let server = TestServer::start(&fixture.root).await;

    let invalid = server
        .request("GET", "/api/v1/documents/..%2Foutside.txt", None)
        .await;
    assert!(matches!(invalid.status, 400 | 404));
    let unknown = server
        .request(
            "GET",
            "/api/v1/documents/550e8400-e29b-41d4-a716-446655440000",
            None,
        )
        .await;
    assert_eq!(unknown.status, 404);
    assert_eq!(
        std::fs::read_to_string(outside).expect("outside remains"),
        "outside sentinel"
    );
}

#[tokio::test]
async fn query_and_json_rejections_share_the_versioned_error_envelope() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;
    let preview_path = format!("/api/v1/documents/{}/preview", fixture.child_id);

    let missing_query = server.request("GET", "/api/v1/search", None).await;
    assert_error(&missing_query, 400, "invalid_query");

    let malformed = server
        .raw_request("POST", &preview_path, "application/json", b"{")
        .await;
    assert_error(&malformed, 400, "invalid_json");

    let missing_field = server.raw_request(
        "POST",
        &preview_path,
        "application/json",
        br#"{"baseRevision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
    )
    .await;
    assert_error(&missing_field, 422, "invalid_json");

    let unknown_field = server.raw_request(
        "POST",
        &preview_path,
        "application/json",
        br#"{"baseRevision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","source":"x","path":"outside"}"#,
    )
    .await;
    assert_error(&unknown_field, 422, "invalid_json");

    let wrong_content_type = server.raw_request(
        "POST",
        &preview_path,
        "text/plain",
        br#"{"baseRevision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","source":"x"}"#,
    )
    .await;
    assert_error(&wrong_content_type, 415, "unsupported_media_type");

    let oversized = vec![b' '; 2 * 1024 * 1024 + 1];
    let too_large = server
        .raw_request("POST", &preview_path, "application/json", &oversized)
        .await;
    assert_error(&too_large, 413, "payload_too_large");
}

#[tokio::test]
async fn router_controlled_4xx_share_the_versioned_error_envelope() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;

    let wrong_method = server.request("POST", "/api/v1/health", None).await;
    assert_error(&wrong_method, 405, "method_not_allowed");

    let unknown_route = server.request("GET", "/api/v1/unknown", None).await;
    assert_error(&unknown_route, 404, "route_not_found");

    let invalid_path = server.request("GET", "/api/v1/documents/%FF", None).await;
    assert_error(&invalid_path, 400, "invalid_path");
}

#[tokio::test]
async fn workspace_scope_is_random_persistent_and_disjoint_for_identical_copies() {
    let fixture = Fixture::new();
    let clone = fixture
        .root
        .parent()
        .expect("fixture parent")
        .join("copy")
        .join("Workspace");
    copy_tree(&fixture.root, &clone);

    let mut first_server = TestServer::start(&fixture.root).await;
    let first = first_server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();
    first_server.restart(&fixture.root).await;
    let restarted = first_server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();
    let clone_server = TestServer::start(&clone).await;
    let copied = clone_server
        .request("GET", "/api/v1/workspace", None)
        .await
        .json();

    assert_eq!(first["rootNodeId"], copied["rootNodeId"]);
    assert_eq!(first["workspaceRevision"], copied["workspaceRevision"]);
    assert_eq!(first["workspaceScope"], restarted["workspaceScope"]);
    assert_ne!(first["workspaceScope"], copied["workspaceScope"]);
    assert_eq!(
        first["workspaceScope"]
            .as_str()
            .expect("opaque scope")
            .len(),
        64
    );
}

#[tokio::test]
async fn startup_recovers_durable_audit_outbox_without_claiming_unconfirmed_commits() {
    let fixture = Fixture::new();
    let mut server = TestServer::start(&fixture.root).await;
    let actor_scope = server
        .request("GET", "/api/v1/auth/session", None)
        .await
        .json()["actorScope"]
        .as_str()
        .expect("Owner actor")
        .to_owned();
    let revision = read_node_document(&fixture.root)
        .expect("root document")
        .revision
        .to_string();
    let database = server.control.path().join("control-plane.sqlite3");
    let connection = rusqlite::Connection::open(&database).expect("open control database");
    connection
        .execute(
            "INSERT INTO audit_outbox(
                 intent_id, created_at, event_type, actor_scope, detail,
                 authority_kind, target, expected_revision
             ) VALUES(?1, 1, ?2, ?3, ?4, 'document', ?5, ?6)",
            rusqlite::params![
                "confirmed-intent",
                "test_commit_recovered",
                actor_scope,
                "test=confirmed",
                fixture.root_id.to_string(),
                revision,
            ],
        )
        .expect("confirmed intent");
    connection
        .execute(
            "INSERT INTO audit_outbox(
                 intent_id, created_at, event_type, actor_scope, detail,
                 authority_kind, target, expected_revision
             ) VALUES(?1, 2, ?2, ?3, ?4, 'document', ?5, ?6)",
            rusqlite::params![
                "unconfirmed-intent",
                "test_commit_not_proven",
                actor_scope,
                "test=unconfirmed",
                fixture.root_id.to_string(),
                "sha256:not-the-current-document",
            ],
        )
        .expect("unconfirmed intent");
    drop(connection);

    server.restart(&fixture.root).await;
    let audit = server.request("GET", "/api/v1/admin/audit", None).await;
    assert_eq!(audit.status, 200, "{}", audit.text());
    assert!(audit.text().contains("test_commit_recovered"));
    assert!(audit.text().contains("auditRecovery=authority_confirmed"));
    assert!(audit.text().contains("audit_intent_recovered"));
    assert!(!audit.text().contains("test=unconfirmed"));
    let connection = rusqlite::Connection::open(database).expect("reopen control database");
    let remaining: i64 = connection
        .query_row("SELECT COUNT(*) FROM audit_outbox", [], |row| row.get(0))
        .expect("outbox count");
    assert_eq!(remaining, 0);
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("create copied workspace directory");
    for entry in std::fs::read_dir(source).expect("read source tree") {
        let entry = entry.expect("source entry");
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry.file_type().expect("entry type").is_dir() {
            copy_tree(&source_path, &target_path);
        } else {
            std::fs::copy(source_path, target_path).expect("copy workspace file");
        }
    }
}

fn assert_error(response: &HttpResponse, status: u16, code: &str) {
    assert_eq!(response.status, status);
    assert!(
        response
            .headers
            .to_ascii_lowercase()
            .contains("content-type: application/json")
    );
    let payload = response.json();
    assert_eq!(payload["error"]["code"], code);
    assert!(payload["error"]["message"].is_string());
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end test proves the live lease, exact pair, audit, clean restore, and drill contract"
)]
async fn owner_backup_restore_and_drill_endpoints_are_quiesced_paired_and_audited() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.root.join("ignored")).expect("ignored directory");
    std::fs::write(
        fixture.root.join(".weftext-rules"),
        b"weftext-content-rules-v1\nignore ignored/\n",
    )
    .expect("rules");
    std::fs::write(
        fixture.root.join("ignored/private.bin"),
        b"ignored bytes are backup authority\0\xff",
    )
    .expect("ignored bytes");
    let trashable = create_child_node(&fixture.root, "TrashBytes").expect("trash payload node");
    std::fs::write(
        trashable.path.join("asset.bin"),
        b"Server pair carries Core Trash bytes\0\xff",
    )
    .expect("Trash payload bytes");
    commit_workspace_transaction(
        &plan_trash_node_at(&fixture.root, trashable.id, "2026-08-24T12:00:00+08:00")
            .expect("Core Trash plan"),
    )
    .expect("Core Trash commit");
    let inventory = scan_workspace(&fixture.root);
    assert!(inventory.is_valid());
    assert_eq!(inventory.trash_items.len(), 1);
    let trash_item = &inventory.trash_items[0];
    let trash_item_locator = trash_item
        .item_path
        .strip_prefix(&fixture.root)
        .expect("Trash item locator")
        .to_path_buf();
    let trash_payload_locator = trash_item
        .payload_path
        .strip_prefix(&fixture.root)
        .expect("Trash payload locator")
        .to_path_buf();
    let trash_manifest_bytes =
        std::fs::read(trash_item.item_path.join(TRASH_ITEM_MANIFEST_FILE_NAME))
            .expect("Core Trash manifest bytes");
    let server = TestServer::start(&fixture.root).await;
    let (_admin_scope, admin_cookie) =
        create_member_session(&server, "backup.admin", "backup admin password", "admin").await;
    let backup_parent = fixture
        .root
        .parent()
        .expect("workspace parent")
        .join("paired-backups");
    let restore_parent = fixture
        .root
        .parent()
        .expect("workspace parent")
        .join("paired-restores");
    std::fs::create_dir(&backup_parent).expect("backup parent");
    std::fs::create_dir(&restore_parent).expect("restore parent");

    let denied = server
        .request_as(
            &admin_cookie,
            "GET",
            "/api/v1/admin/backup/capabilities",
            None,
        )
        .await;
    assert_error(&denied, 403, "authorization_denied");
    let capabilities = server
        .request("GET", "/api/v1/admin/backup/capabilities", None)
        .await;
    assert_eq!(capabilities.status, 200, "{}", capabilities.text());
    assert_eq!(capabilities.json()["exclusiveLease"], true);
    assert_eq!(capabilities.json()["apiQuiescence"], true);

    let preview = server
        .request(
            "POST",
            "/api/v1/admin/backup/preview",
            Some(&json!({"backupParent": backup_parent})),
        )
        .await;
    assert_eq!(preview.status, 200, "{}", preview.text());
    let preview_json = preview.json();
    assert_eq!(preview_json["stage"], "preview");
    assert_eq!(preview_json["quiesced"], true);
    let plan_digest = preview_json["plan"]["planDigest"]
        .as_str()
        .expect("backup plan digest")
        .to_owned();
    let workspace_snapshot = PathBuf::from(
        preview_json["plan"]["workspaceSnapshotDirectory"]
            .as_str()
            .expect("workspace snapshot"),
    );
    let control_snapshot = PathBuf::from(
        preview_json["plan"]["controlPlaneSnapshotDirectory"]
            .as_str()
            .expect("control snapshot"),
    );
    let committed = server
        .request(
            "POST",
            "/api/v1/admin/backup/commit",
            Some(&json!({"planDigest": plan_digest})),
        )
        .await;
    assert_eq!(committed.status, 200, "{}", committed.text());
    assert_eq!(
        committed.json()["receipt"]["verification"]["exactPair"],
        true
    );
    assert_eq!(committed.json()["auditRecorded"], true);
    assert_eq!(
        std::fs::read(
            workspace_snapshot
                .join("content")
                .join("Workspace")
                .join("ignored/private.bin")
        )
        .expect("ignored snapshot bytes"),
        b"ignored bytes are backup authority\0\xff"
    );
    let snapshot_workspace_root = workspace_snapshot.join("content").join("Workspace");
    assert_eq!(
        std::fs::read(
            snapshot_workspace_root
                .join(&trash_item_locator)
                .join(TRASH_ITEM_MANIFEST_FILE_NAME)
        )
        .expect("snapshot Trash manifest bytes"),
        trash_manifest_bytes
    );
    assert_eq!(
        std::fs::read(
            snapshot_workspace_root
                .join(&trash_payload_locator)
                .join("asset.bin")
        )
        .expect("snapshot Trash payload bytes"),
        b"Server pair carries Core Trash bytes\0\xff"
    );

    let verified = server
        .request(
            "POST",
            "/api/v1/admin/backup/verify",
            Some(&json!({
                "workspaceSnapshotDirectory": workspace_snapshot,
                "controlPlaneSnapshotDirectory": control_snapshot,
            })),
        )
        .await;
    assert_eq!(verified.status, 200, "{}", verified.text());
    assert_eq!(verified.json()["verification"]["exactPair"], true);

    let restored_workspace = restore_parent.join("alternate").join("Workspace");
    let restored_control = restore_parent.join("alternate-control");
    std::fs::create_dir(restored_workspace.parent().expect("alternate parent"))
        .expect("alternate parent");
    let restore_preview = server
        .request(
            "POST",
            "/api/v1/admin/restore/preview",
            Some(&json!({
                "workspaceSnapshotDirectory": workspace_snapshot,
                "controlPlaneSnapshotDirectory": control_snapshot,
                "restoredWorkspaceRoot": restored_workspace,
                "restoredControlPlaneRoot": restored_control,
            })),
        )
        .await;
    assert_eq!(restore_preview.status, 200, "{}", restore_preview.text());
    let restore_digest = restore_preview.json()["plan"]["planDigest"]
        .as_str()
        .expect("restore digest")
        .to_owned();
    let restore_commit = server
        .request(
            "POST",
            "/api/v1/admin/restore/commit",
            Some(&json!({"planDigest": restore_digest})),
        )
        .await;
    assert_eq!(restore_commit.status, 200, "{}", restore_commit.text());
    assert_eq!(
        restore_commit.json()["receipt"]["verification"]["exactPair"],
        true
    );
    assert_eq!(restore_commit.json()["auditRecorded"], true);
    assert_eq!(
        std::fs::read(restored_workspace.join("ignored/private.bin"))
            .expect("restored ignored bytes"),
        b"ignored bytes are backup authority\0\xff"
    );
    assert_eq!(
        std::fs::read(
            restored_workspace
                .join(&trash_item_locator)
                .join(TRASH_ITEM_MANIFEST_FILE_NAME)
        )
        .expect("restored Trash manifest bytes"),
        trash_manifest_bytes
    );
    assert_eq!(
        std::fs::read(
            restored_workspace
                .join(&trash_payload_locator)
                .join("asset.bin")
        )
        .expect("restored Trash payload bytes"),
        b"Server pair carries Core Trash bytes\0\xff"
    );

    let restore_verified = server
        .request(
            "POST",
            "/api/v1/admin/restore/verify",
            Some(&json!({
                "workspaceSnapshotDirectory": workspace_snapshot,
                "controlPlaneSnapshotDirectory": control_snapshot,
                "restoredWorkspaceRoot": restored_workspace,
                "restoredControlPlaneRoot": restored_control,
            })),
        )
        .await;
    assert_eq!(restore_verified.status, 200, "{}", restore_verified.text());
    assert_eq!(restore_verified.json()["verification"]["exactPair"], true);

    let occupied_workspace = restore_parent.join("occupied").join("Workspace");
    std::fs::create_dir_all(&occupied_workspace).expect("occupied restore target");
    let occupied = server
        .request(
            "POST",
            "/api/v1/admin/restore/preview",
            Some(&json!({
                "workspaceSnapshotDirectory": workspace_snapshot,
                "controlPlaneSnapshotDirectory": control_snapshot,
                "restoredWorkspaceRoot": occupied_workspace,
                "restoredControlPlaneRoot": restore_parent.join("unused-control"),
            })),
        )
        .await;
    assert_error(&occupied, 409, "restore_target_not_clean");

    let drill_workspace = restore_parent.join("drill").join("Workspace");
    let drill_control = restore_parent.join("drill-control");
    std::fs::create_dir(drill_workspace.parent().expect("drill parent")).expect("drill parent");
    let drill_preview = server
        .request(
            "POST",
            "/api/v1/admin/backup/drill/preview",
            Some(&json!({
                "workspaceSnapshotDirectory": workspace_snapshot,
                "controlPlaneSnapshotDirectory": control_snapshot,
                "drillWorkspaceRoot": drill_workspace,
                "drillControlPlaneRoot": drill_control,
            })),
        )
        .await;
    assert_eq!(drill_preview.status, 200, "{}", drill_preview.text());
    let drill_digest = drill_preview.json()["plan"]["planDigest"]
        .as_str()
        .expect("drill digest")
        .to_owned();
    let drill_commit = server
        .request(
            "POST",
            "/api/v1/admin/backup/drill/commit",
            Some(&json!({"planDigest": drill_digest})),
        )
        .await;
    assert_eq!(drill_commit.status, 200, "{}", drill_commit.text());
    assert_eq!(drill_commit.json()["stage"], "drill_completed");
    assert_eq!(
        drill_commit.json()["receipt"]["verification"]["exactPair"],
        true
    );
    assert_eq!(drill_commit.json()["auditRecorded"], true);
    assert_eq!(scan_workspace(&drill_workspace).trash_items.len(), 1);
    assert_eq!(
        std::fs::read(
            drill_workspace
                .join(&trash_payload_locator)
                .join("asset.bin")
        )
        .expect("drilled Trash payload bytes"),
        b"Server pair carries Core Trash bytes\0\xff"
    );

    let audit = server.request("GET", "/api/v1/admin/audit", None).await;
    assert_eq!(audit.status, 200, "{}", audit.text());
    let audit_json = audit.json();
    let events = audit_json
        .as_array()
        .expect("audit receipts")
        .iter()
        .filter_map(|receipt| receipt["eventType"].as_str())
        .collect::<Vec<_>>();
    assert!(events.contains(&"server_backup_completed"));
    assert!(events.contains(&"server_restore_completed"));
    assert!(events.contains(&"server_restore_drill_completed"));
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end case keeps reviewed Trash plans, session binding, stale rejection, no-overwrite, permanent evidence, and audit behavior together"
)]
async fn workspace_trash_api_is_item_backed_session_bound_and_no_overwrite() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("alpha.bin"), b"alpha resource")
        .expect("write first resource");
    std::fs::write(fixture.root.join("beta.bin"), b"beta resource").expect("write second resource");
    let server = TestServer::start(&fixture.root).await;

    let empty = server.request("GET", "/api/v1/trash", None).await;
    assert_eq!(empty.status, 200, "{}", empty.text());
    assert_eq!(empty.json()["state"], "ready");
    assert_eq!(empty.json()["items"], json!([]));
    let node_preview = server
        .request(
            "POST",
            &format!("/api/v1/trash/nodes/{}/preview", fixture.child_id),
            Some(&json!({
                "baseWorkspaceRevision": empty.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T12:00:00+08:00",
            })),
        )
        .await;
    assert_eq!(node_preview.status, 200, "{}", node_preview.text());
    assert_eq!(
        node_preview.json()["trashItemChanges"][0]["manifest"]["kind"],
        "node"
    );
    assert!(!node_preview.text().contains("itemPath"));
    assert!(!node_preview.text().contains("payloadPath"));
    assert!(!node_preview.text().contains(".weftext-trash"));
    let node_plan = node_preview.json()["planId"]
        .as_str()
        .expect("node Trash plan")
        .to_owned();
    let node_commit = server
        .request(
            "POST",
            &format!("/api/v1/trash/transactions/{node_plan}/commit"),
            None,
        )
        .await;
    assert_eq!(node_commit.status, 200, "{}", node_commit.text());
    assert_eq!(node_commit.json()["auditRecorded"], true);
    assert!(!fixture.root.join("Child").exists());
    let replay = server
        .request(
            "POST",
            &format!("/api/v1/trash/transactions/{node_plan}/commit"),
            None,
        )
        .await;
    assert_error(&replay, 404, "trash_plan_unavailable");

    let with_node_item = server.request("GET", "/api/v1/trash", None).await;
    assert_eq!(with_node_item.status, 200, "{}", with_node_item.text());
    assert_eq!(
        with_node_item.json()["items"]
            .as_array()
            .expect("Trash items")
            .len(),
        1
    );
    let node_item_id = with_node_item.json()["items"][0]["manifest"]["trashItemId"]
        .as_str()
        .expect("node Trash item ID")
        .to_owned();
    let restore_node = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{node_item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": with_node_item.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    assert_eq!(restore_node.status, 200, "{}", restore_node.text());
    let restore_node_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                restore_node.json()["planId"]
                    .as_str()
                    .expect("node restore plan")
            ),
            None,
        )
        .await;
    assert_eq!(
        restore_node_commit.status,
        200,
        "{}",
        restore_node_commit.text()
    );
    assert!(fixture.root.join("Child/Child.adoc").is_file());

    let before_resources = server.request("GET", "/api/v1/trash", None).await;
    let resource_preview = server
        .request(
            "POST",
            "/api/v1/trash/resources/preview",
            Some(&json!({
                "baseWorkspaceRevision": before_resources.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T12:01:00+08:00",
                "resources": [
                    {"ownerNodeId": fixture.root_id, "name": "alpha.bin"},
                    {"ownerNodeId": fixture.root_id, "name": "beta.bin"},
                ],
            })),
        )
        .await;
    assert_eq!(resource_preview.status, 200, "{}", resource_preview.text());
    let resource_changes = resource_preview.json()["trashItemChanges"]
        .as_array()
        .expect("resource item changes")
        .clone();
    assert_eq!(resource_changes.len(), 2);
    assert_eq!(
        resource_changes[0]["manifest"]["operationId"],
        resource_changes[1]["manifest"]["operationId"],
        "one batch must share one operation ID"
    );
    assert_ne!(
        resource_changes[0]["manifest"]["trashItemId"],
        resource_changes[1]["manifest"]["trashItemId"]
    );
    let resource_plan = resource_preview.json()["planId"]
        .as_str()
        .expect("resource plan")
        .to_owned();

    let second_login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({"login": "owner", "password": OWNER_PASSWORD})),
        )
        .await;
    let second_owner = second_login.session_cookie().expect("second Owner session");
    let foreign = server
        .request_as(
            &second_owner,
            "POST",
            &format!("/api/v1/trash/transactions/{resource_plan}/commit"),
            None,
        )
        .await;
    let missing = server
        .request_as(
            &second_owner,
            "POST",
            "/api/v1/trash/transactions/11111111-1111-4111-8111-111111111111/commit",
            None,
        )
        .await;
    assert_eq!(foreign.status, 404);
    assert_eq!(foreign.json(), missing.json());
    let resource_commit = server
        .request(
            "POST",
            &format!("/api/v1/trash/transactions/{resource_plan}/commit"),
            None,
        )
        .await;
    assert_eq!(resource_commit.status, 200, "{}", resource_commit.text());
    assert!(!fixture.root.join("alpha.bin").exists());
    assert!(!fixture.root.join("beta.bin").exists());

    let resources = server.request("GET", "/api/v1/trash", None).await;
    let resource_items = resources.json()["items"]
        .as_array()
        .expect("resource Trash items")
        .clone();
    assert_eq!(resource_items.len(), 2);
    let alpha = resource_items
        .iter()
        .find(|item| item["manifest"]["originalName"] == "alpha.bin")
        .expect("alpha Trash item");
    let alpha_item_id = alpha["manifest"]["trashItemId"]
        .as_str()
        .expect("alpha item ID")
        .to_owned();

    std::fs::write(fixture.root.join("alpha.bin"), b"do not overwrite")
        .expect("write restore conflict");
    let conflict_inventory = server.request("GET", "/api/v1/trash", None).await;
    let conflict = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{alpha_item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": conflict_inventory.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    assert_error(&conflict, 422, "workspace_transaction_rejected");
    assert_eq!(
        std::fs::read(fixture.root.join("alpha.bin")).expect("read conflict"),
        b"do not overwrite"
    );
    std::fs::remove_file(fixture.root.join("alpha.bin")).expect("remove restore conflict");

    let before_stale = server.request("GET", "/api/v1/trash", None).await;
    let stale_preview = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{alpha_item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": before_stale.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    assert_eq!(stale_preview.status, 200, "{}", stale_preview.text());
    let stale_plan = stale_preview.json()["planId"]
        .as_str()
        .expect("stale restore plan")
        .to_owned();
    create_child_node(&fixture.root, "RevisionBump").expect("external workspace mutation");
    let stale_commit = server
        .request(
            "POST",
            &format!("/api/v1/trash/transactions/{stale_plan}/commit"),
            None,
        )
        .await;
    assert_error(&stale_commit, 409, "stale_workspace_revision");
    let stale_replay = server
        .request(
            "POST",
            &format!("/api/v1/trash/transactions/{stale_plan}/commit"),
            None,
        )
        .await;
    assert_error(&stale_replay, 404, "trash_plan_unavailable");

    let fresh = server.request("GET", "/api/v1/trash", None).await;
    let restore_alpha = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{alpha_item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": fresh.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    assert_eq!(restore_alpha.status, 200, "{}", restore_alpha.text());
    let restore_alpha_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                restore_alpha.json()["planId"]
                    .as_str()
                    .expect("fresh restore plan")
            ),
            None,
        )
        .await;
    assert_eq!(
        restore_alpha_commit.status,
        200,
        "{}",
        restore_alpha_commit.text()
    );
    assert_eq!(
        std::fs::read(fixture.root.join("alpha.bin")).expect("restored alpha"),
        b"alpha resource"
    );

    let after_restore = server.request("GET", "/api/v1/trash", None).await;
    let re_trash = server
        .request(
            "POST",
            "/api/v1/trash/resources/preview",
            Some(&json!({
                "baseWorkspaceRevision": after_restore.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T12:02:00+08:00",
                "resources": [{"ownerNodeId": fixture.root_id, "name": "alpha.bin"}],
            })),
        )
        .await;
    assert_eq!(re_trash.status, 200, "{}", re_trash.text());
    let re_trash_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                re_trash.json()["planId"].as_str().expect("re-Trash plan")
            ),
            None,
        )
        .await;
    assert_eq!(re_trash_commit.status, 200, "{}", re_trash_commit.text());

    let before_delete = server.request("GET", "/api/v1/trash", None).await;
    let delete_items = before_delete.json()["items"]
        .as_array()
        .expect("items before permanent delete")
        .iter()
        .map(|item| {
            json!({
                "trashItemId": item["manifest"]["trashItemId"],
                "payloadSha256": item["manifest"]["sha256"],
                "payloadByteLength": item["manifest"]["byteLength"],
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(delete_items.len(), 2);
    let mut wrong_items = delete_items.clone();
    wrong_items[0]["payloadSha256"] = Value::String("0".repeat(64));
    let wrong_delete = server
        .request(
            "POST",
            "/api/v1/trash/permanent-delete/preview",
            Some(&json!({
                "baseWorkspaceRevision": before_delete.json()["workspaceRevision"],
                "items": wrong_items,
            })),
        )
        .await;
    assert_error(&wrong_delete, 400, "invalid_request");
    let delete_preview = server
        .request(
            "POST",
            "/api/v1/trash/permanent-delete/preview",
            Some(&json!({
                "baseWorkspaceRevision": before_delete.json()["workspaceRevision"],
                "items": delete_items,
            })),
        )
        .await;
    assert_eq!(delete_preview.status, 200, "{}", delete_preview.text());
    let delete_plan = delete_preview.json()["planId"]
        .as_str()
        .expect("permanent-delete plan")
        .to_owned();
    let delete_commit = server
        .request(
            "POST",
            &format!("/api/v1/trash/transactions/{delete_plan}/commit"),
            None,
        )
        .await;
    assert_eq!(delete_commit.status, 200, "{}", delete_commit.text());
    assert_eq!(delete_commit.json()["auditRecorded"], true);
    let delete_replay = server
        .request(
            "POST",
            &format!("/api/v1/trash/transactions/{delete_plan}/commit"),
            None,
        )
        .await;
    assert_error(&delete_replay, 404, "trash_plan_unavailable");
    let final_inventory = server.request("GET", "/api/v1/trash", None).await;
    assert_eq!(final_inventory.json()["items"], json!([]));

    let audit = server.request("GET", "/api/v1/admin/audit", None).await;
    let audit_text = audit.text();
    for event in [
        "trash_node_stored",
        "trash_item_restored",
        "trash_resources_stored",
        "trash_items_permanently_deleted",
    ] {
        assert!(audit_text.contains(event), "missing audit event {event}");
    }
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one parent-order case proves the Server preserves separate items and exposes Core's atomic with-ancestors recovery mode"
)]
async fn workspace_trash_api_restores_parent_chain_atomically() {
    let fixture = Fixture::new();
    let grandchild =
        create_child_node(fixture.root.join("Child"), "Grandchild").expect("grandchild node");
    let server = TestServer::start(&fixture.root).await;

    let empty = server.request("GET", "/api/v1/trash", None).await;
    let grandchild_preview = server
        .request(
            "POST",
            &format!("/api/v1/trash/nodes/{}/preview", grandchild.id),
            Some(&json!({
                "baseWorkspaceRevision": empty.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T12:10:00+08:00",
            })),
        )
        .await;
    assert_eq!(
        grandchild_preview.status,
        200,
        "{}",
        grandchild_preview.text()
    );
    let grandchild_item_id =
        grandchild_preview.json()["trashItemChanges"][0]["manifest"]["trashItemId"]
            .as_str()
            .expect("grandchild item")
            .to_owned();
    let grandchild_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                grandchild_preview.json()["planId"]
                    .as_str()
                    .expect("grandchild plan")
            ),
            None,
        )
        .await;
    assert_eq!(
        grandchild_commit.status,
        200,
        "{}",
        grandchild_commit.text()
    );

    let after_grandchild = server.request("GET", "/api/v1/trash", None).await;
    let parent_preview = server
        .request(
            "POST",
            &format!("/api/v1/trash/nodes/{}/preview", fixture.child_id),
            Some(&json!({
                "baseWorkspaceRevision": after_grandchild.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T12:11:00+08:00",
            })),
        )
        .await;
    assert_eq!(parent_preview.status, 200, "{}", parent_preview.text());
    let parent_item_id = parent_preview.json()["trashItemChanges"][0]["manifest"]["trashItemId"]
        .as_str()
        .expect("parent item")
        .to_owned();
    let parent_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                parent_preview.json()["planId"]
                    .as_str()
                    .expect("parent plan")
            ),
            None,
        )
        .await;
    assert_eq!(parent_commit.status, 200, "{}", parent_commit.text());

    let separated = server.request("GET", "/api/v1/trash", None).await;
    assert_eq!(
        separated.json()["items"]
            .as_array()
            .expect("separate parent and child items")
            .len(),
        2
    );
    let child_item = separated.json()["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["manifest"]["trashItemId"] == grandchild_item_id)
        .expect("grandchild projection")
        .clone();
    assert_eq!(child_item["restore"]["originResolution"], "in_trash");
    assert_eq!(child_item["restore"]["originalAvailable"], false);
    assert_eq!(child_item["restore"]["withAncestorsAvailable"], true);
    assert_eq!(
        child_item["restore"]["requiredAncestorItemIds"],
        json!([parent_item_id])
    );
    let implicit = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{grandchild_item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": separated.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    assert_error(&implicit, 422, "workspace_transaction_rejected");
    let chain_preview = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{grandchild_item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": separated.json()["workspaceRevision"],
                "mode": "with_ancestors",
            })),
        )
        .await;
    assert_eq!(chain_preview.status, 200, "{}", chain_preview.text());
    assert_eq!(
        chain_preview.json()["trashItemChanges"]
            .as_array()
            .expect("atomic parent-chain changes")
            .len(),
        2
    );
    let chain_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                chain_preview.json()["planId"]
                    .as_str()
                    .expect("parent-chain plan")
            ),
            None,
        )
        .await;
    assert_eq!(chain_commit.status, 200, "{}", chain_commit.text());
    assert!(fixture.root.join("Child/Child.adoc").is_file());
    assert!(
        fixture
            .root
            .join("Child/Grandchild/Grandchild.adoc")
            .is_file()
    );
    assert_eq!(
        server.request("GET", "/api/v1/trash", None).await.json()["items"],
        json!([])
    );
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one role matrix proves Trash ACL reauthorization, hidden-item non-disclosure, and the configured Admin permanent-delete boundary"
)]
async fn workspace_trash_api_reauthorizes_acl_and_gates_admin_permanent_delete() {
    let fixture = Fixture::new();
    let editable = create_child_node(&fixture.root, "Editable").expect("editable node");
    let server = TestServer::start(&fixture.root).await;
    let (editor_scope, editor_cookie) =
        create_member_session(&server, "trash.editor", "editor password value", "editor").await;
    let (_admin_scope, admin_cookie) =
        create_member_session(&server, "trash.admin", "admin password value", "admin").await;

    let editor_write = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": editor_scope,
                "nodeId": editable.id,
                "access": "write",
            })),
        )
        .await;
    assert_eq!(editor_write.status, 200, "{}", editor_write.text());
    let editor_inventory = server
        .request_as(&editor_cookie, "GET", "/api/v1/trash", None)
        .await;
    assert_eq!(editor_inventory.status, 200, "{}", editor_inventory.text());
    assert!(
        editor_inventory.json()["workspaceRevision"]
            .as_str()
            .expect("editor revision")
            .starts_with("actor-v1:")
    );
    let editor_preview = server
        .request_as(
            &editor_cookie,
            "POST",
            &format!("/api/v1/trash/nodes/{}/preview", editable.id),
            Some(&json!({
                "baseWorkspaceRevision": editor_inventory.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T13:00:00+08:00",
            })),
        )
        .await;
    assert_eq!(editor_preview.status, 200, "{}", editor_preview.text());
    let editor_plan = editor_preview.json()["planId"]
        .as_str()
        .expect("editor Trash plan")
        .to_owned();
    let hide_editable = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": editor_scope,
                "nodeId": editable.id,
                "access": "hidden",
            })),
        )
        .await;
    assert_eq!(hide_editable.status, 200, "{}", hide_editable.text());
    let revoked_commit = server
        .request_as(
            &editor_cookie,
            "POST",
            &format!("/api/v1/trash/transactions/{editor_plan}/commit"),
            None,
        )
        .await;
    assert_error(&revoked_commit, 409, "stale_workspace_revision");
    let revoked_replay = server
        .request_as(
            &editor_cookie,
            "POST",
            &format!("/api/v1/trash/transactions/{editor_plan}/commit"),
            None,
        )
        .await;
    assert_error(&revoked_replay, 404, "trash_plan_unavailable");
    assert!(fixture.root.join("Editable/Editable.adoc").is_file());

    let hide_child = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": editor_scope,
                "nodeId": fixture.child_id,
                "access": "hidden",
            })),
        )
        .await;
    assert_eq!(hide_child.status, 200, "{}", hide_child.text());
    let owner_inventory = server.request("GET", "/api/v1/trash", None).await;
    let child_preview = server
        .request(
            "POST",
            &format!("/api/v1/trash/nodes/{}/preview", fixture.child_id),
            Some(&json!({
                "baseWorkspaceRevision": owner_inventory.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T13:01:00+08:00",
            })),
        )
        .await;
    assert_eq!(child_preview.status, 200, "{}", child_preview.text());
    let child_item_id = child_preview.json()["trashItemChanges"][0]["manifest"]["trashItemId"]
        .as_str()
        .expect("hidden child item ID")
        .to_owned();
    let child_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                child_preview.json()["planId"].as_str().expect("child plan")
            ),
            None,
        )
        .await;
    assert_eq!(child_commit.status, 200, "{}", child_commit.text());

    let editor_after = server
        .request_as(&editor_cookie, "GET", "/api/v1/trash", None)
        .await;
    assert_eq!(editor_after.status, 200, "{}", editor_after.text());
    assert_eq!(editor_after.json()["items"], json!([]));
    assert!(!editor_after.text().contains(&child_item_id));
    let hidden_restore = server
        .request_as(
            &editor_cookie,
            "POST",
            &format!("/api/v1/trash/items/{child_item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": editor_after.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    let missing_restore = server
        .request_as(
            &editor_cookie,
            "POST",
            "/api/v1/trash/items/11111111-1111-4111-8111-111111111111/restore/preview",
            Some(&json!({
                "baseWorkspaceRevision": editor_after.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    assert_eq!(hidden_restore.status, 404);
    assert_eq!(hidden_restore.json(), missing_restore.json());
    assert!(!hidden_restore.text().contains(&child_item_id));

    let admin_inventory = server
        .request_as(&admin_cookie, "GET", "/api/v1/trash", None)
        .await;
    assert_eq!(admin_inventory.status, 200, "{}", admin_inventory.text());
    let owner_item = server.request("GET", "/api/v1/trash", None).await.json()["items"][0].clone();
    let forbidden_admin_delete = server
        .request_as(
            &admin_cookie,
            "POST",
            "/api/v1/trash/permanent-delete/preview",
            Some(&json!({
                "baseWorkspaceRevision": admin_inventory.json()["workspaceRevision"],
                "items": [{
                    "trashItemId": owner_item["manifest"]["trashItemId"],
                    "payloadSha256": owner_item["manifest"]["payloadSha256"],
                    "payloadByteLength": owner_item["manifest"]["payloadByteLength"],
                }],
            })),
        )
        .await;
    assert_error(&forbidden_admin_delete, 403, "authorization_denied");

    let configured_fixture = Fixture::new();
    let configured_server = TestServer::start_configured(&configured_fixture.root, |config| {
        config.with_admin_permanent_delete(true)
    })
    .await;
    let (_configured_admin_scope, configured_admin_cookie) = create_member_session(
        &configured_server,
        "configured.trash.admin",
        "configured admin password",
        "admin",
    )
    .await;
    let configured_empty = configured_server
        .request("GET", "/api/v1/trash", None)
        .await;
    let configured_preview = configured_server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/nodes/{}/preview",
                configured_fixture.child_id
            ),
            Some(&json!({
                "baseWorkspaceRevision": configured_empty.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T13:02:00+08:00",
            })),
        )
        .await;
    let configured_commit = configured_server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                configured_preview.json()["planId"]
                    .as_str()
                    .expect("configured node plan")
            ),
            None,
        )
        .await;
    assert_eq!(
        configured_commit.status,
        200,
        "{}",
        configured_commit.text()
    );
    let configured_admin_inventory = configured_server
        .request_as(&configured_admin_cookie, "GET", "/api/v1/trash", None)
        .await;
    assert_eq!(
        configured_admin_inventory.status,
        200,
        "{}",
        configured_admin_inventory.text()
    );
    let configured_item = configured_admin_inventory.json()["items"][0].clone();
    let configured_delete = configured_server
        .request_as(
            &configured_admin_cookie,
            "POST",
            "/api/v1/trash/permanent-delete/preview",
            Some(&json!({
                "baseWorkspaceRevision": configured_admin_inventory.json()["workspaceRevision"],
                "items": [{
                    "trashItemId": configured_item["manifest"]["trashItemId"],
                    "payloadSha256": configured_item["manifest"]["payloadSha256"],
                    "payloadByteLength": configured_item["manifest"]["payloadByteLength"],
                }],
            })),
        )
        .await;
    assert_eq!(
        configured_delete.status,
        200,
        "{}",
        configured_delete.text()
    );
    let configured_delete_commit = configured_server
        .request_as(
            &configured_admin_cookie,
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                configured_delete.json()["planId"]
                    .as_str()
                    .expect("configured Admin delete plan")
            ),
            None,
        )
        .await;
    assert_eq!(
        configured_delete_commit.status,
        200,
        "{}",
        configured_delete_commit.text()
    );
    assert_eq!(configured_delete_commit.json()["auditRecorded"], true);
    assert_eq!(
        configured_server
            .request("GET", "/api/v1/trash", None)
            .await
            .json()["items"],
        json!([])
    );
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one degraded-mode case proves external-snapshot legacy migration, read-only reconciliation, and explicit unknown-origin recovery"
)]
async fn workspace_trash_api_migrates_legacy_only_after_external_snapshot() {
    let fixture = Fixture::new();
    convert_node_to_legacy_trash(&fixture.root, fixture.child_id, "Child");
    let snapshot_parent = fixture
        .root
        .parent()
        .expect("workspace parent")
        .join("external migration snapshots");
    std::fs::create_dir(&snapshot_parent).expect("external snapshot parent");
    let server = TestServer::start_configured(&fixture.root, |config| {
        config.with_trash_migration_snapshot_parent(&snapshot_parent)
    })
    .await;
    let (_admin_scope, admin_cookie) = create_member_session(
        &server,
        "migration.admin",
        "migration admin password",
        "admin",
    )
    .await;

    let legacy = server.request("GET", "/api/v1/trash", None).await;
    assert_eq!(legacy.status, 200, "{}", legacy.text());
    assert_eq!(legacy.json()["state"], "legacy_migration_required");
    assert_eq!(legacy.json()["legacyMigrationRequired"], true);
    assert_eq!(legacy.json()["items"], json!([]));
    assert!(!legacy.text().contains(".weftext-trash"));
    let active_workspace = server.request("GET", "/api/v1/workspace", None).await;
    assert_eq!(active_workspace.status, 200, "{}", active_workspace.text());
    assert!(!active_workspace.text().contains("Child"));

    let blocked_node = server
        .request(
            "POST",
            &format!("/api/v1/trash/nodes/{}/preview", fixture.root_id),
            Some(&json!({
                "baseWorkspaceRevision": legacy.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T14:00:00+08:00",
            })),
        )
        .await;
    assert_error(&blocked_node, 409, "trash_reconciliation_required");

    let admin_legacy = server
        .request_as(&admin_cookie, "GET", "/api/v1/trash", None)
        .await;
    let admin_migration = server
        .request_as(
            &admin_cookie,
            "POST",
            "/api/v1/trash/migrate-legacy/preview",
            Some(&json!({
                "baseWorkspaceRevision": admin_legacy.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T14:01:00+08:00",
            })),
        )
        .await;
    assert_error(&admin_migration, 403, "authorization_denied");

    let preview = server
        .request(
            "POST",
            "/api/v1/trash/migrate-legacy/preview",
            Some(&json!({
                "baseWorkspaceRevision": legacy.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T14:01:00+08:00",
            })),
        )
        .await;
    assert_eq!(preview.status, 200, "{}", preview.text());
    assert_eq!(
        preview.json()["trashItemChanges"][0]["manifest"]["originStatus"],
        "unknown"
    );
    assert_eq!(
        std::fs::read_dir(&snapshot_parent)
            .expect("snapshot parent")
            .count(),
        1,
        "preview must create one verified external snapshot"
    );
    assert!(
        fixture
            .root
            .join(".weftext-trash/Child/Child.adoc")
            .is_file()
    );
    let migration_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                preview.json()["planId"].as_str().expect("migration plan")
            ),
            None,
        )
        .await;
    assert_eq!(migration_commit.status, 200, "{}", migration_commit.text());
    assert_eq!(migration_commit.json()["auditRecorded"], true);

    let migrated = server.request("GET", "/api/v1/trash", None).await;
    assert_eq!(migrated.status, 200, "{}", migrated.text());
    assert_eq!(migrated.json()["state"], "ready");
    assert_eq!(
        migrated.json()["items"][0]["manifest"]["originStatus"],
        "unknown"
    );
    assert_eq!(
        migrated.json()["items"][0]["restore"]["originResolution"],
        "unknown"
    );
    assert_eq!(
        migrated.json()["items"][0]["restore"]["originalAvailable"],
        false
    );
    let item_id = migrated.json()["items"][0]["manifest"]["trashItemId"]
        .as_str()
        .expect("migrated item ID")
        .to_owned();
    let item_path = scan_workspace(&fixture.root).trash_items[0]
        .item_path
        .clone();
    let manifest_path = item_path.join(TRASH_ITEM_MANIFEST_FILE_NAME);
    let manifest_bytes = std::fs::read(&manifest_path).expect("trusted manifest bytes");
    std::fs::write(&manifest_path, b"[]\n").expect("tamper manifest");

    let reconciliation = server.request("GET", "/api/v1/trash", None).await;
    assert_eq!(reconciliation.status, 200, "{}", reconciliation.text());
    assert_eq!(reconciliation.json()["state"], "reconciliation_required");
    assert_eq!(reconciliation.json()["reconciliation"]["required"], true);
    assert!(
        reconciliation.json()["reconciliation"]["issueCount"]
            .as_u64()
            .expect("issue count")
            > 0
    );
    assert_eq!(reconciliation.json()["items"], json!([]));
    assert!(!reconciliation.text().contains(&item_id));
    assert_ne!(
        reconciliation.json()["workspaceRevision"],
        migrated.json()["workspaceRevision"],
        "Trash-only tampering must change the opaque workspace revision"
    );
    let blocked_reconciliation = server
        .request(
            "POST",
            "/api/v1/trash/resources/preview",
            Some(&json!({
                "baseWorkspaceRevision": reconciliation.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T14:02:00+08:00",
                "resources": [{"ownerNodeId": fixture.root_id, "name": "anything.bin"}],
            })),
        )
        .await;
    assert_error(
        &blocked_reconciliation,
        409,
        "trash_reconciliation_required",
    );
    std::fs::write(&manifest_path, manifest_bytes).expect("restore trusted manifest");

    let ready_again = server.request("GET", "/api/v1/trash", None).await;
    let implicit = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": ready_again.json()["workspaceRevision"],
                "mode": "original",
            })),
        )
        .await;
    assert_error(&implicit, 422, "workspace_transaction_rejected");
    let explicit = server
        .request(
            "POST",
            &format!("/api/v1/trash/items/{item_id}/restore/preview"),
            Some(&json!({
                "baseWorkspaceRevision": ready_again.json()["workspaceRevision"],
                "mode": "existing_target",
                "targetNodeId": fixture.root_id,
                "name": "Recovered Legacy",
            })),
        )
        .await;
    assert_eq!(explicit.status, 200, "{}", explicit.text());
    let explicit_commit = server
        .request(
            "POST",
            &format!(
                "/api/v1/trash/transactions/{}/commit",
                explicit.json()["planId"].as_str().expect("explicit plan")
            ),
            None,
        )
        .await;
    assert_eq!(explicit_commit.status, 200, "{}", explicit_commit.text());
    assert!(
        fixture
            .root
            .join("Recovered Legacy/Recovered Legacy.adoc")
            .is_file()
    );
    let audit = server.request("GET", "/api/v1/admin/audit", None).await;
    assert!(audit.text().contains("trash_legacy_migrated"));
    assert!(audit.text().contains("trash_item_restored"));

    let unavailable_fixture = Fixture::new();
    convert_node_to_legacy_trash(
        &unavailable_fixture.root,
        unavailable_fixture.child_id,
        "Child",
    );
    let unavailable_server = TestServer::start(&unavailable_fixture.root).await;
    let unavailable_inventory = unavailable_server
        .request("GET", "/api/v1/trash", None)
        .await;
    assert_eq!(unavailable_inventory.status, 200);
    let unavailable = unavailable_server
        .request(
            "POST",
            "/api/v1/trash/migrate-legacy/preview",
            Some(&json!({
                "baseWorkspaceRevision": unavailable_inventory.json()["workspaceRevision"],
                "trashedAt": "2026-08-24T14:03:00+08:00",
            })),
        )
        .await;
    assert_error(&unavailable, 503, "trash_migration_backup_unavailable");
}

async fn create_member_session(
    server: &TestServer,
    login: &str,
    password: &str,
    role: &str,
) -> (String, String) {
    let created = server
        .request(
            "POST",
            "/api/v1/admin/members",
            Some(&json!({ "login": login, "password": password, "role": role })),
        )
        .await;
    assert_eq!(created.status, 200, "{}", created.text());
    let actor_scope = created.json()["actorScope"]
        .as_str()
        .expect("member actor scope")
        .to_owned();
    let login_response = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({ "login": login, "password": password })),
        )
        .await;
    assert_eq!(login_response.status, 200, "{}", login_response.text());
    (
        actor_scope,
        login_response
            .session_cookie()
            .expect("member session cookie"),
    )
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one lifecycle test proves persistence across bootstrap, rotation, logout, and revocation"
)]
async fn owner_bootstrap_login_logout_and_revocation_are_session_backed() {
    let invalid_fixture = Fixture::new();
    let invalid_server = TestServer::start_unbootstrapped(&invalid_fixture.root).await;
    let invalid_bootstrap = invalid_server
        .anonymous_request(
            "POST",
            "/api/v1/auth/bootstrap",
            Some(&json!({
                "bootstrapSecret": "0".repeat(64),
                "password": OWNER_PASSWORD,
            })),
        )
        .await;
    assert_error(&invalid_bootstrap, 403, "bootstrap_failed");

    let fixture = Fixture::new();
    let server = TestServer::start_unbootstrapped(&fixture.root).await;
    let secret = std::fs::read_to_string(server.bootstrap_secret_path())
        .expect("bootstrap secret")
        .trim()
        .to_owned();
    let bootstrap_body = json!({
        "bootstrapSecret": secret,
        "password": OWNER_PASSWORD,
    });

    let (first, second) = tokio::join!(
        server.anonymous_request("POST", "/api/v1/auth/bootstrap", Some(&bootstrap_body)),
        server.anonymous_request("POST", "/api/v1/auth/bootstrap", Some(&bootstrap_body)),
    );
    let (success, conflict) = if first.status == 200 {
        (first, second)
    } else {
        (second, first)
    };
    assert_eq!(success.status, 200);
    assert_error(&conflict, 403, "bootstrap_failed");
    assert_eq!(conflict.body, invalid_bootstrap.body);
    assert!(!server.bootstrap_secret_path().exists());
    let bootstrap_cookie = success.session_cookie().expect("bootstrap cookie");
    assert_cookie_attributes(&success.headers, false);

    let session = request_with_context(
        server.address,
        "GET",
        "/api/v1/auth/session",
        None,
        Some(&bootstrap_cookie),
        None,
        None,
        None,
    )
    .await;
    assert_eq!(session.status, 200);
    assert_eq!(session.json()["role"], "owner");

    let wrong_login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({ "login": "missing", "password": "wrong password value" })),
        )
        .await;
    let wrong_password = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({ "login": "owner", "password": "wrong password value" })),
        )
        .await;
    assert_eq!(wrong_login.status, 401);
    assert_eq!(wrong_login.body, wrong_password.body);

    let login = request_with_context(
        server.address,
        "POST",
        "/api/v1/auth/login",
        Some(&json!({ "login": "owner", "password": OWNER_PASSWORD })),
        Some(&bootstrap_cookie),
        Some(&format!("http://{}", server.address)),
        Some("same-origin"),
        None,
    )
    .await;
    assert_eq!(login.status, 200);
    let login_cookie = login.session_cookie().expect("login cookie");
    assert_ne!(bootstrap_cookie, login_cookie);
    let rotated = request_with_context(
        server.address,
        "GET",
        "/api/v1/auth/session",
        None,
        Some(&bootstrap_cookie),
        None,
        None,
        None,
    )
    .await;
    assert_error(&rotated, 401, "authentication_required");

    let logout = request_with_context(
        server.address,
        "POST",
        "/api/v1/auth/logout",
        None,
        Some(&login_cookie),
        Some(&format!("http://{}", server.address)),
        Some("same-origin"),
        None,
    )
    .await;
    assert_eq!(logout.status, 200);
    assert!(logout.headers.contains("Max-Age=0"));
    let logged_out = request_with_context(
        server.address,
        "GET",
        "/api/v1/auth/session",
        None,
        Some(&login_cookie),
        None,
        None,
        None,
    )
    .await;
    assert_error(&logged_out, 401, "authentication_required");

    let relogin = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({ "login": "owner", "password": OWNER_PASSWORD })),
        )
        .await;
    let relogin_cookie = relogin.session_cookie().expect("relogin cookie");
    let revoke = request_with_context(
        server.address,
        "POST",
        "/api/v1/auth/revoke-all",
        None,
        Some(&relogin_cookie),
        Some(&format!("http://{}", server.address)),
        Some("same-origin"),
        None,
    )
    .await;
    assert_eq!(revoke.status, 200);
    let revoked = request_with_context(
        server.address,
        "GET",
        "/api/v1/auth/session",
        None,
        Some(&relogin_cookie),
        None,
        None,
        None,
    )
    .await;
    assert_error(&revoked, 401, "authentication_required");

    let database = rusqlite::Connection::open(server.control.path().join("control-plane.sqlite3"))
        .expect("control database");
    let digests: Vec<Vec<u8>> = database
        .prepare("SELECT token_digest FROM sessions")
        .expect("session query")
        .query_map([], |row| row.get(0))
        .expect("session rows")
        .collect::<Result<_, _>>()
        .expect("session digests");
    assert!(digests.iter().all(|digest| digest.len() == 32));
    assert!(
        !digests
            .iter()
            .any(|digest| String::from_utf8_lossy(digest).contains(&relogin_cookie))
    );
    let event_types: Vec<String> = database
        .prepare("SELECT event_type FROM security_events ORDER BY event_id")
        .expect("event query")
        .query_map([], |row| row.get(0))
        .expect("event rows")
        .collect::<Result<_, _>>()
        .expect("event types");
    for expected in [
        "bootstrap_succeeded",
        "login_failed",
        "login_succeeded",
        "logout",
        "sessions_revoked",
    ] {
        assert!(event_types.iter().any(|value| value == expected));
    }
}

#[tokio::test]
async fn host_origin_csrf_and_auth_guards_fail_closed_without_content_disclosure() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;
    let wrong_host = request_with_context(
        server.address,
        "GET",
        "/api/v1/health",
        None,
        None,
        None,
        None,
        Some("127.0.0.1.attacker.invalid"),
    )
    .await;
    assert_error(&wrong_host, 400, "invalid_host");

    for (origin, csrf) in [
        (None, Some("same-origin")),
        (Some("null"), Some("same-origin")),
        (Some("http://attacker.invalid"), Some("same-origin")),
        (
            Some(&format!("http://{}.attacker.invalid", server.address)),
            Some("same-origin"),
        ),
        (Some(&format!("http://{}", server.address)), None),
        (Some(&format!("http://{}", server.address)), Some("wrong")),
    ] {
        let response = request_with_context(
            server.address,
            "POST",
            "/api/v1/auth/login",
            Some(&json!({ "login": "owner", "password": "wrong password value" })),
            None,
            origin,
            csrf,
            None,
        )
        .await;
        assert_error(&response, 403, "csrf_rejected");
    }

    let protected = [
        ("GET", "/api/v1/workspace", None),
        ("GET", "/api/v1/search?q=Needle", None),
        ("GET", "/api/v1/citations/capabilities", None),
        ("GET", "/api/v1/citations/validate", None),
        ("GET", "/api/v1/citations/references?q=Needle", None),
        ("GET", "/api/v1/tasks/validate", None),
        ("GET", "/api/v1/changes", None),
        ("GET", "/api/v1/auth/session", None),
    ];
    for (method, path, body) in protected {
        let response = server.anonymous_request(method, path, body).await;
        assert_error(&response, 401, "authentication_required");
        let text = response.text();
        assert!(!text.contains("Needle"));
        assert!(!text.contains(&fixture.child_id.to_string()));
    }
    for (method, path) in [
        (
            "POST",
            format!("/api/v1/documents/{}/preview", fixture.child_id),
        ),
        ("PUT", format!("/api/v1/documents/{}", fixture.child_id)),
        (
            "POST",
            format!("/api/v1/citations/{}/analyze", fixture.child_id),
        ),
        (
            "POST",
            format!("/api/v1/tasks/nodes/{}/edit/preview", fixture.child_id),
        ),
        ("POST", "/api/v1/queries/execute".to_owned()),
    ] {
        let response = server
            .anonymous_request(
                method,
                &path,
                Some(&json!({ "baseRevision": "a".repeat(64), "source": "secret" })),
            )
            .await;
        assert_error(&response, 401, "authentication_required");
    }
}

#[tokio::test]
async fn bootstrap_and_login_have_independent_basic_rate_limits() {
    let fixture = Fixture::new();
    let bootstrap_server = TestServer::start_unbootstrapped(&fixture.root).await;
    for attempt in 0..6 {
        let response = bootstrap_server
            .anonymous_request(
                "POST",
                "/api/v1/auth/bootstrap",
                Some(&json!({
                    "bootstrapSecret": "0".repeat(64),
                    "password": OWNER_PASSWORD,
                })),
            )
            .await;
        if attempt < 5 {
            assert_error(&response, 403, "bootstrap_failed");
        } else {
            assert_error(&response, 429, "rate_limited");
        }
    }

    let login_server = TestServer::start(&fixture.root).await;
    for attempt in 0..6 {
        let response = login_server
            .anonymous_request(
                "POST",
                "/api/v1/auth/login",
                Some(&json!({ "login": "owner", "password": "wrong password value" })),
            )
            .await;
        if attempt < 5 {
            assert_error(&response, 401, "authentication_failed");
        } else {
            assert_error(&response, 429, "rate_limited");
        }
    }
}

fn assert_cookie_attributes(headers: &str, secure: bool) {
    assert!(headers.contains("HttpOnly"));
    assert!(headers.contains("SameSite=Strict"));
    assert!(headers.contains("Path=/api/v1"));
    assert!(!headers.contains("Domain="));
    assert_eq!(headers.contains("; Secure"), secure);
}

#[test]
fn server_transport_contains_no_direct_filesystem_mutation_calls() {
    let source = include_str!("../src/lib.rs")
        .split("#[cfg(test)]")
        .next()
        .expect("production source");
    for forbidden in [
        "fs::write(",
        "fs::rename(",
        "File::create(",
        "OpenOptions",
        "remove_file(",
        "remove_dir(",
    ] {
        assert!(
            !source.contains(forbidden),
            "Server transport must not contain direct mutation call {forbidden}"
        );
    }
    assert!(source.contains("plan_document_edit"));
    assert!(source.contains("commit_document_edit"));
    assert!(source.contains("plan_task_edit_transaction"));
    assert!(source.contains("commit_task_edit_transaction"));
}

#[tokio::test]
async fn concurrent_commits_are_serialized_to_one_winner_and_one_conflict() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;
    let path = format!("/api/v1/documents/{}", fixture.child_id);
    let opened = server.request("GET", &path, None).await.json();
    let base = opened["revision"].as_str().expect("base").to_owned();
    let source = opened["source"].as_str().expect("source").to_owned();
    let request_a = json!({ "baseRevision": base, "source": format!("{source}\nA") });
    let request_b = json!({ "baseRevision": base, "source": format!("{source}\nB") });

    let (left, right) = tokio::join!(
        server.request("PUT", &path, Some(&request_a)),
        server.request("PUT", &path, Some(&request_b)),
    );
    let mut statuses = [left.status, right.status];
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 409]);
    let committed = read_node_document(fixture.root.join("Child")).expect("committed document");
    assert!(committed.source.ends_with('A') || committed.source.ends_with('B'));
}

#[tokio::test]
async fn changed_commit_publishes_node_uuid_and_new_revision() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;
    let mut stream = TcpStream::connect(server.address)
        .await
        .expect("connect SSE");
    stream
        .write_all(
            format!(
                "GET /api/v1/changes HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\nAccept: text/event-stream\r\n\r\n",
                server.address,
                server.cookie.as_deref().expect("session cookie")
            )
            .as_bytes(),
        )
        .await
        .expect("subscribe request");
    let mut received = Vec::new();
    read_until_contains(&mut stream, &mut received, b"\r\n\r\n").await;

    let path = format!("/api/v1/documents/{}", fixture.child_id);
    let opened = server.request("GET", &path, None).await.json();
    let source = format!(
        "{}\nnotification",
        opened["source"].as_str().expect("source")
    );
    let commit = server
        .request(
            "PUT",
            &path,
            Some(&json!({ "baseRevision": opened["revision"], "source": source })),
        )
        .await;
    assert_eq!(commit.status, 200);

    read_until_contains(&mut stream, &mut received, b"node-committed").await;
    read_until_contains(
        &mut stream,
        &mut received,
        commit.json()["revision"]
            .as_str()
            .expect("revision")
            .as_bytes(),
    )
    .await;
    let event = String::from_utf8(received).expect("SSE UTF-8");
    assert!(event.contains(&fixture.child_id.to_string()));
    assert!(event.contains("node-committed"));
}

#[tokio::test]
async fn established_change_stream_ends_at_the_session_deadline() {
    let fixture = Fixture::new();
    let mut server = TestServer::start_unbootstrapped_with_policy(
        &fixture.root,
        weftext_server::SessionPolicy {
            absolute_seconds: 1,
            idle_seconds: 1,
        },
    )
    .await;
    let secret = std::fs::read_to_string(server.bootstrap_secret_path())
        .expect("read bootstrap secret")
        .trim()
        .to_owned();
    let bootstrap = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/bootstrap",
            Some(&json!({ "bootstrapSecret": secret, "password": OWNER_PASSWORD })),
        )
        .await;
    server.cookie = Some(bootstrap.session_cookie().expect("session cookie"));

    let mut stream = TcpStream::connect(server.address)
        .await
        .expect("connect SSE");
    stream
        .write_all(
            format!(
                "GET /api/v1/changes HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n",
                server.address,
                server.cookie.as_deref().expect("session cookie")
            )
            .as_bytes(),
        )
        .await
        .expect("subscribe request");
    let mut received = Vec::new();
    read_until_contains(&mut stream, &mut received, b"\r\n\r\n").await;
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut buffer = [0_u8; 1024];
        loop {
            let count = stream.read(&mut buffer).await.expect("read stream");
            if count == 0 {
                break;
            }
        }
    })
    .await
    .expect("expired SSE connection must close");

    let expired = server.request("GET", "/api/v1/changes", None).await;
    assert_error(&expired, 401, "authentication_required");
}

#[tokio::test]
async fn established_change_stream_ends_when_owner_revokes_the_member() {
    let fixture = Fixture::new();
    let server = TestServer::start(&fixture.root).await;
    let viewer = server
        .request(
            "POST",
            "/api/v1/admin/members",
            Some(&json!({
                "login": "stream.viewer",
                "password": "viewer password value",
                "role": "viewer"
            })),
        )
        .await;
    assert_eq!(viewer.status, 200, "{}", viewer.text());
    let viewer_scope = viewer.json()["actorScope"]
        .as_str()
        .expect("viewer actor scope")
        .to_owned();
    let login = server
        .anonymous_request(
            "POST",
            "/api/v1/auth/login",
            Some(&json!({
                "login": "stream.viewer",
                "password": "viewer password value"
            })),
        )
        .await;
    assert_eq!(login.status, 200, "{}", login.text());
    let viewer_cookie = login.session_cookie().expect("viewer session cookie");

    let mut stream = TcpStream::connect(server.address)
        .await
        .expect("connect SSE");
    stream
        .write_all(
            format!(
                "GET /api/v1/changes HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n",
                server.address, viewer_cookie
            )
            .as_bytes(),
        )
        .await
        .expect("subscribe request");
    let mut received = Vec::new();
    read_until_contains(&mut stream, &mut received, b"\r\n\r\n").await;

    let disabled = server
        .request(
            "PUT",
            &format!("/api/v1/admin/members/{viewer_scope}"),
            Some(&json!({"role": "viewer", "enabled": false})),
        )
        .await;
    assert_eq!(disabled.status, 200, "{}", disabled.text());

    tokio::time::timeout(Duration::from_secs(2), async {
        let mut buffer = [0_u8; 1024];
        loop {
            let count = stream.read(&mut buffer).await.expect("read stream");
            if count == 0 {
                break;
            }
        }
    })
    .await
    .expect("revoked SSE connection must close promptly");
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one multi-session case proves the collaboration wire, convergence, ACL, presence, restart, external-edit, and audit boundaries together"
)]
async fn collaboration_protocol_is_linearized_acl_scoped_durable_and_fail_closed() {
    let fixture = Fixture::new();
    let mut server = TestServer::start(&fixture.root).await;
    let (admin_scope, admin_cookie) =
        create_member_session(&server, "collab.admin", "admin password value", "admin").await;
    let (editor_scope, editor_cookie) =
        create_member_session(&server, "collab.editor", "editor password value", "editor").await;
    let (commenter_scope, commenter_cookie) = create_member_session(
        &server,
        "collab.commenter",
        "commenter password value",
        "commenter",
    )
    .await;
    let (_, viewer_cookie) =
        create_member_session(&server, "collab.viewer", "viewer password value", "viewer").await;

    let node_id = fixture.child_id;
    let snapshot_route = format!("/api/v1/collaboration/documents/{node_id}");
    let operation_route = format!("{snapshot_route}/operations");
    let owner_snapshot = server.request("GET", &snapshot_route, None).await;
    assert_eq!(owner_snapshot.status, 200, "{}", owner_snapshot.text());
    let owner_payload = owner_snapshot.json();
    assert_eq!(owner_payload["wireVersion"], "weftext.collaboration.v1");
    assert_eq!(owner_payload["state"]["epoch"], 1);
    assert_eq!(owner_payload["state"]["version"], 0);
    assert_eq!(owner_payload["state"]["frozen"], false);
    let owner_actor = owner_payload["actorId"]
        .as_str()
        .expect("owner wire actor UUID")
        .to_owned();
    let base_revision = owner_payload["state"]["revision"]
        .as_str()
        .expect("base revision")
        .to_owned();
    let base_source = owner_payload["source"]
        .as_str()
        .expect("base source")
        .to_owned();
    let append_at = base_source.len();
    let owner_client = "10000000-0000-4000-8000-000000000001";
    let admin_client = "10000000-0000-4000-8000-000000000002";
    let editor_client = "10000000-0000-4000-8000-000000000003";
    let owner_operation_id = "20000000-0000-4000-8000-000000000001";
    let request = |client_id: &str, operation_id: &str, replacement: &str| {
        json!({
            "wireVersion": "weftext.collaboration.v1",
            "clientId": client_id,
            "operationId": operation_id,
            "epoch": 1,
            "baseVersion": 0,
            "baseRevision": base_revision,
            "operations": [{
                "start": append_at,
                "end": append_at,
                "replacement": replacement
            }]
        })
    };
    let owner_request = request(owner_client, owner_operation_id, "\nowner-live");
    let mut collaboration_stream = TcpStream::connect(server.address)
        .await
        .expect("connect collaboration SSE");
    collaboration_stream
        .write_all(
            format!(
                "GET /api/v1/collaboration/events HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n",
                server.address,
                server.cookie.as_deref().expect("owner session cookie")
            )
            .as_bytes(),
        )
        .await
        .expect("subscribe to collaboration events");
    let mut collaboration_events = Vec::new();
    read_until_contains(
        &mut collaboration_stream,
        &mut collaboration_events,
        b"\r\n\r\n",
    )
    .await;
    let owner_commit = server
        .request("POST", &operation_route, Some(&owner_request))
        .await;
    assert_eq!(owner_commit.status, 200, "{}", owner_commit.text());
    assert_eq!(owner_commit.json()["status"], "accepted");
    assert_eq!(owner_commit.json()["state"]["version"], 1);
    read_until_contains(
        &mut collaboration_stream,
        &mut collaboration_events,
        b"operation-committed",
    )
    .await;
    let first_event = String::from_utf8(collaboration_events.clone()).expect("event UTF-8");
    assert!(first_event.contains(owner_client));
    assert!(first_event.contains(owner_operation_id));

    let admin_request = request(
        admin_client,
        "20000000-0000-4000-8000-000000000002",
        "\nadmin-live",
    );
    let admin_commit = server
        .request_as(
            &admin_cookie,
            "POST",
            &operation_route,
            Some(&admin_request),
        )
        .await;
    assert_eq!(admin_commit.status, 200, "{}", admin_commit.text());
    let admin_payload = admin_commit.json();
    assert_eq!(admin_payload["status"], "accepted");
    assert_eq!(admin_payload["transformed"], true);
    let admin_applied_base_revision = admin_payload["appliedBaseRevision"].clone();
    let admin_applied_base_version = admin_payload["appliedBaseVersion"].clone();

    let editor_request = request(
        editor_client,
        "20000000-0000-4000-8000-000000000003",
        "\neditor-live",
    );
    let editor_commit = server
        .request_as(
            &editor_cookie,
            "POST",
            &operation_route,
            Some(&editor_request),
        )
        .await;
    assert_eq!(editor_commit.status, 200, "{}", editor_commit.text());
    assert_eq!(editor_commit.json()["status"], "accepted");
    assert_eq!(editor_commit.json()["state"]["version"], 3);

    let draft_route = format!("{snapshot_route}/drafts");
    let offline_draft = server
        .request(
            "POST",
            &draft_route,
            Some(&json!({
                "wireVersion": "weftext.collaboration.v1",
                "clientId": "10000000-0000-4000-8000-000000000004",
                "operationId": "20000000-0000-4000-8000-000000000004",
                "epoch": 1,
                "baseVersion": 0,
                "baseRevision": base_revision,
                "source": base_source.replace("Needle body", "offline draft body")
            })),
        )
        .await;
    assert_eq!(offline_draft.status, 200, "{}", offline_draft.text());
    assert_eq!(offline_draft.json()["status"], "accepted");
    assert_eq!(offline_draft.json()["transformed"], true);
    assert_eq!(offline_draft.json()["state"]["version"], 4);

    let canonical = server
        .request("GET", &format!("/api/v1/documents/{node_id}"), None)
        .await;
    let canonical_source = canonical.json()["source"]
        .as_str()
        .expect("canonical source")
        .to_owned();
    for marker in ["owner-live", "admin-live", "editor-live"] {
        assert!(canonical_source.contains(marker), "missing {marker}");
    }
    assert!(canonical_source.contains("offline draft body"));

    for (role, cookie) in [("commenter", &commenter_cookie), ("viewer", &viewer_cookie)] {
        let rejected = server
            .request_as(cookie, "POST", &operation_route, Some(&owner_request))
            .await;
        assert_error(&rejected, 403, "authorization_denied");
        let readable = server
            .request_as(cookie, "GET", &snapshot_route, None)
            .await;
        assert_eq!(readable.status, 200, "{role}: {}", readable.text());
    }

    let replay = server
        .request("POST", &operation_route, Some(&owner_request))
        .await;
    assert_eq!(replay.status, 200, "{}", replay.text());
    assert_eq!(replay.json()["status"], "replayed");
    assert_eq!(replay.json()["transactionId"], owner_operation_id);
    let transformed_replay = server
        .request_as(
            &admin_cookie,
            "POST",
            &operation_route,
            Some(&admin_request),
        )
        .await;
    let transformed_replay_payload = transformed_replay.json();
    assert_eq!(transformed_replay.status, 200);
    assert_eq!(transformed_replay_payload["status"], "replayed");
    assert_eq!(transformed_replay_payload["transformed"], true);
    assert_eq!(
        transformed_replay_payload["appliedBaseRevision"],
        admin_applied_base_revision
    );
    assert_eq!(
        transformed_replay_payload["appliedBaseVersion"],
        admin_applied_base_version
    );
    let changed_replay = request(owner_client, owner_operation_id, "\nnot-the-same-operation");
    let replay_conflict = server
        .request("POST", &operation_route, Some(&changed_replay))
        .await;
    assert_eq!(replay_conflict.status, 409, "{}", replay_conflict.text());
    assert_eq!(
        replay_conflict.json()["errorCode"],
        "collaboration_replay_mismatch"
    );
    assert_eq!(replay_conflict.json()["transactionId"], "");

    let viewer_snapshot = server
        .request_as(&viewer_cookie, "GET", &snapshot_route, None)
        .await;
    let viewer_payload = viewer_snapshot.json();
    let viewer_actor = viewer_payload["actorId"]
        .as_str()
        .expect("viewer actor UUID")
        .to_owned();
    assert_ne!(owner_actor, viewer_actor);
    let viewer_presence = server
        .request_as(
            &viewer_cookie,
            "POST",
            &format!("{snapshot_route}/presence"),
            Some(&json!({
                "wireVersion": "weftext.collaboration.v1",
                "clientId": "30000000-0000-4000-8000-000000000001",
                "epoch": viewer_payload["state"]["epoch"],
                "revision": viewer_payload["state"]["revision"],
                "cursor": 0,
                "selectionStart": 0,
                "selectionEnd": 0
            })),
        )
        .await;
    assert_eq!(viewer_presence.status, 200, "{}", viewer_presence.text());
    assert_eq!(
        viewer_presence.json()["participants"][0]["actorId"],
        viewer_actor
    );
    read_until_contains(
        &mut collaboration_stream,
        &mut collaboration_events,
        b"event: presence",
    )
    .await;
    read_until_contains(
        &mut collaboration_stream,
        &mut collaboration_events,
        viewer_actor.as_bytes(),
    )
    .await;
    assert!(
        String::from_utf8(collaboration_events.clone())
            .expect("presence event UTF-8")
            .contains(&viewer_actor)
    );

    let annotation_route = format!("/api/v1/annotations/{node_id}");
    let annotation_state = server.request("GET", &annotation_route, None).await.json();
    let annotation_commit = mutate_annotation(
        &server,
        &annotation_route,
        &annotation_state,
        json!({
            "action": "create",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": null,
            "labels": ["live"],
            "bodySource": "collaboration event",
            "suggestedSource": null
        }),
    )
    .await;
    assert_eq!(
        annotation_commit.status,
        200,
        "{}",
        annotation_commit.text()
    );
    read_until_contains(
        &mut collaboration_stream,
        &mut collaboration_events,
        b"annotation-mutated",
    )
    .await;

    let restrict_editor = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": editor_scope,
                "nodeId": node_id,
                "access": "read"
            })),
        )
        .await;
    assert_eq!(restrict_editor.status, 200, "{}", restrict_editor.text());
    let editor_read_only = server
        .request_as(
            &editor_cookie,
            "POST",
            &operation_route,
            Some(&editor_request),
        )
        .await;
    assert_error(&editor_read_only, 403, "authorization_denied");
    let restore_editor = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": editor_scope,
                "nodeId": node_id,
                "access": null
            })),
        )
        .await;
    assert_eq!(restore_editor.status, 200, "{}", restore_editor.text());
    let hide_commenter = server
        .request(
            "PUT",
            "/api/v1/admin/node-acl",
            Some(&json!({
                "actorScope": commenter_scope,
                "nodeId": node_id,
                "access": "hidden"
            })),
        )
        .await;
    assert_eq!(hide_commenter.status, 200, "{}", hide_commenter.text());
    let hidden = server
        .request_as(&commenter_cookie, "GET", &snapshot_route, None)
        .await;
    assert_error(&hidden, 404, "node_not_found");

    let before_duplicate = server.request("GET", &snapshot_route, None).await.json();
    let duplicate_source = before_duplicate["source"]
        .as_str()
        .expect("source before duplicate");
    let simultaneous_duplicate = json!({
        "wireVersion": "weftext.collaboration.v1",
        "clientId": "35000000-0000-4000-8000-000000000001",
        "operationId": "35000000-0000-4000-8000-000000000002",
        "epoch": before_duplicate["state"]["epoch"],
        "baseVersion": before_duplicate["state"]["version"],
        "baseRevision": before_duplicate["state"]["revision"],
        "operations": [{
            "start": duplicate_source.len(),
            "end": duplicate_source.len(),
            "replacement": "\nonce-only"
        }]
    });
    let (duplicate_a, duplicate_b) = tokio::join!(
        server.request("POST", &operation_route, Some(&simultaneous_duplicate)),
        server.request("POST", &operation_route, Some(&simultaneous_duplicate)),
    );
    assert_eq!(duplicate_a.status, 200, "{}", duplicate_a.text());
    assert_eq!(duplicate_b.status, 200, "{}", duplicate_b.text());
    let mut duplicate_statuses = [
        duplicate_a.json()["status"]
            .as_str()
            .expect("duplicate A status")
            .to_owned(),
        duplicate_b.json()["status"]
            .as_str()
            .expect("duplicate B status")
            .to_owned(),
    ];
    duplicate_statuses.sort_unstable();
    assert_eq!(
        duplicate_statuses,
        ["accepted".to_owned(), "replayed".to_owned()]
    );

    let before_overlap = server.request("GET", &snapshot_route, None).await.json();
    let overlap_source = before_overlap["source"]
        .as_str()
        .expect("source before overlap");
    let overlap_start = overlap_source
        .find("offline draft body")
        .expect("fixture body");
    let overlapping_request = |client_id: &str, operation_id: &str, replacement: &str| {
        json!({
            "wireVersion": "weftext.collaboration.v1",
            "clientId": client_id,
            "operationId": operation_id,
            "epoch": before_overlap["state"]["epoch"],
            "baseVersion": before_overlap["state"]["version"],
            "baseRevision": before_overlap["state"]["revision"],
            "operations": [{
                "start": overlap_start,
                "end": overlap_start + "offline draft body".len(),
                "replacement": replacement
            }]
        })
    };
    let first_overlap = overlapping_request(
        owner_client,
        "40000000-0000-4000-8000-000000000001",
        "owner chose this",
    );
    let accepted_overlap = server
        .request("POST", &operation_route, Some(&first_overlap))
        .await;
    assert_eq!(accepted_overlap.status, 200, "{}", accepted_overlap.text());
    let second_overlap = overlapping_request(
        editor_client,
        "40000000-0000-4000-8000-000000000002",
        "editor kept locally",
    );
    let conflict = server
        .request_as(
            &editor_cookie,
            "POST",
            &operation_route,
            Some(&second_overlap),
        )
        .await;
    assert_eq!(conflict.status, 409, "{}", conflict.text());
    assert_eq!(conflict.json()["errorCode"], "collaboration_conflict");
    assert_eq!(conflict.json()["state"]["frozen"], true);
    let conflict_state = conflict.json()["state"].clone();
    let after_conflict = server
        .request("GET", &format!("/api/v1/documents/{node_id}"), None)
        .await
        .json();
    assert!(
        after_conflict["source"]
            .as_str()
            .expect("source after conflict")
            .contains("owner chose this")
    );
    assert!(
        !after_conflict["source"]
            .as_str()
            .expect("source after conflict")
            .contains("editor kept locally")
    );
    let resync = server
        .request(
            "POST",
            &format!("{snapshot_route}/resync"),
            Some(&json!({
                "wireVersion": "weftext.collaboration.v1",
                "clientId": owner_client,
                "epoch": conflict_state["epoch"],
                "revision": conflict_state["revision"]
            })),
        )
        .await;
    assert_eq!(resync.status, 200, "{}", resync.text());
    assert_eq!(resync.json()["state"]["frozen"], false);

    replace_document(&fixture.root.join("Child"), |source| {
        format!("{source}\nexternal-filesystem-edit")
    });
    let external = server.request("GET", &snapshot_route, None).await;
    assert_eq!(external.status, 200, "{}", external.text());
    assert_eq!(external.json()["state"]["frozen"], true);
    assert_eq!(external.json()["state"]["reason"], "external_edit");
    assert_ne!(
        external.json()["state"]["comparison"]["expectedRevision"],
        external.json()["state"]["comparison"]["actualRevision"]
    );

    let audit = server.request("GET", "/api/v1/admin/audit", None).await;
    assert_eq!(audit.status, 200, "{}", audit.text());
    let collaboration_audit = audit
        .json()
        .as_array()
        .expect("audit receipts")
        .iter()
        .find(|receipt| receipt["eventType"] == "collaboration_operation_committed")
        .expect("collaboration audit receipt")
        .clone();
    let audit_detail = collaboration_audit["detail"]
        .as_str()
        .expect("collaboration audit detail");
    assert!(audit_detail.contains(&format!("actor={owner_actor}")));
    assert!(audit_detail.contains(&format!("client={owner_client}")));
    assert!(audit_detail.contains(&format!("transaction={owner_operation_id}")));

    drop(collaboration_stream);
    server.restart(&fixture.root).await;
    let restart_replay = server
        .request("POST", &operation_route, Some(&owner_request))
        .await;
    assert_eq!(restart_replay.status, 200, "{}", restart_replay.text());
    assert_eq!(restart_replay.json()["status"], "replayed");
    assert_eq!(restart_replay.json()["transactionId"], owner_operation_id);

    let capabilities = server
        .anonymous_request("GET", "/api/v1/capabilities", None)
        .await
        .json();
    assert_eq!(
        capabilities["collaborationProtocol"],
        "weftext.collaboration.v1"
    );
    assert_eq!(capabilities["realtimeCollaboration"], false);
    assert_eq!(admin_scope.len(), 64);
}

async fn read_until_contains(stream: &mut TcpStream, received: &mut Vec<u8>, needle: &[u8]) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !received
            .windows(needle.len())
            .any(|window| window == needle)
        {
            let mut buffer = [0_u8; 1024];
            let count = stream.read(&mut buffer).await.expect("read stream");
            assert!(count > 0, "stream closed before expected data");
            received.extend_from_slice(&buffer[..count]);
        }
    })
    .await
    .expect("stream data timeout");
}
