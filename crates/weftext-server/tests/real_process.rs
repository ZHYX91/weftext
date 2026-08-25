use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use weftext_core::create_workspace;
#[cfg(debug_assertions)]
use weftext_core::{plan_create_child_node, prepare_workspace_transaction_recovery_fixture};

static REAL_PROCESS_TEST_SERIAL: Mutex<()> = Mutex::new(());

fn serialize_real_process_test() -> MutexGuard<'static, ()> {
    REAL_PROCESS_TEST_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn read_server_startup(child: &mut Child, expected_boundary: &str) -> String {
    let stdout = child.stdout.take().expect("server stdout");
    let mut lines = BufReader::new(stdout).lines();
    let listening = lines
        .next()
        .expect("listening line")
        .expect("read listening line");
    let boundary = lines
        .next()
        .expect("boundary line")
        .expect("read boundary line");
    assert_eq!(
        boundary,
        format!("WEFTEXT_SERVER_BOUNDARY={expected_boundary}")
    );
    listening
        .strip_prefix("WEFTEXT_SERVER_LISTENING=http://")
        .expect("listening marker")
        .to_owned()
}

struct ProcessGuard(Child);

impl ProcessGuard {
    fn terminate_and_wait(&mut self, description: &str) {
        self.0
            .kill()
            .unwrap_or_else(|error| panic!("cannot stop {description}: {error}"));
        let status = self
            .0
            .wait()
            .unwrap_or_else(|error| panic!("cannot wait for {description}: {error}"));
        assert!(
            !status.success(),
            "terminated {description} unexpectedly reported successful exit"
        );
    }

    fn wait_for_exit(&mut self, description: &str, timeout: Duration) -> ExitStatus {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self
                .0
                .try_wait()
                .unwrap_or_else(|error| panic!("cannot poll {description}: {error}"))
            {
                return status;
            }
            if Instant::now() >= deadline {
                let _ = self.0.kill();
                let _ = self.0.wait();
                panic!("{description} did not exit within {timeout:?}");
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn deployment_assets_preserve_the_same_host_loopback_boundary() {
    let dockerfile = include_str!("../deploy/Dockerfile");
    let dockerignore = include_str!("../deploy/Dockerfile.dockerignore");
    let compose = include_str!("../deploy/compose.same-host.yaml");
    let nginx = include_str!("../deploy/nginx.same-host.conf.template");
    let nginx_start = include_str!("../deploy/start-nginx-same-host.sh");
    let verifier = include_str!("../deploy/verify-same-host-deployment.sh");
    let example_environment = include_str!("../deploy/.env.example");
    let (backend, proxy) = compose
        .split_once("  same-host-proxy:\n")
        .expect("same-host proxy service");

    assert!(dockerfile.contains("STOPSIGNAL SIGTERM"));
    assert!(dockerfile.contains("docker build --file crates/weftext-server/deploy/Dockerfile ."));
    assert!(dockerfile.contains("cargo build --locked --release -p weftext-server"));
    assert!(!dockerfile.contains("EXPOSE"));
    assert!(dockerignore.starts_with("**\n"));
    for required_context in [
        "!Cargo.toml",
        "!Cargo.lock",
        "!rust-toolchain.toml",
        "!crates/**",
    ] {
        assert!(dockerignore.lines().any(|line| line == required_context));
    }
    for instruction in dockerfile.lines().filter(|line| line.starts_with("FROM ")) {
        assert!(
            instruction.contains("@sha256:"),
            "container base must be immutable: {instruction}"
        );
    }
    assert_eq!(compose.matches("network_mode: host").count(), 2);
    assert_eq!(compose.matches("restart: unless-stopped").count(), 2);
    assert_eq!(compose.matches("healthcheck:").count(), 2);
    assert_eq!(compose.matches("@sha256:${WEFTEXT_").count(), 2);
    assert_eq!(compose.matches("cap_drop:").count(), 2);
    assert_eq!(compose.matches("cap_add:").count(), 1);
    assert_eq!(compose.matches("create_host_path: false").count(), 5);
    assert_eq!(
        compose
            .matches("user: \"${WEFTEXT_UID:-10001}:${WEFTEXT_GID:-10001}\"")
            .count(),
        2,
        "backend and proxy need one numeric identity to share protected secrets"
    );
    assert!(compose.contains("127.0.0.1:8787"));
    assert!(compose.contains("--same-host-proxy-origin"));
    assert!(!compose.lines().any(|line| line.trim() == "ports:"));
    assert!(!compose.lines().any(|line| line.trim() == "expose:"));
    assert!(!compose.contains("privileged:"));
    assert!(!compose.contains("latest"));
    assert!(compose.contains("WEFTEXT_SERVER_IMAGE_DIGEST:?"));
    assert!(backend.contains("target: /var/lib/weftext"));
    assert!(proxy.contains("WEFTEXT_PROXY_IMAGE_DIGEST:?"));
    assert!(proxy.contains("condition: service_healthy"));
    assert!(proxy.contains("restart: true"));
    assert!(proxy.contains("cap_drop:\n      - ALL"));
    assert!(proxy.contains("cap_add:\n      - NET_BIND_SERVICE"));
    assert!(proxy.contains("no-new-privileges:true"));
    assert!(proxy.contains(
        "source: \"${WEFTEXT_CONTROL_PATH:?set WEFTEXT_CONTROL_PATH}/reverse-proxy-secret\""
    ));
    assert!(proxy.contains("target: /etc/nginx/weftext-reverse-proxy-secret"));
    assert!(!proxy.contains("target: /var/lib/weftext"));
    assert!(proxy.contains("WEFTEXT_TLS_CERTIFICATE_PATH:?"));
    assert!(proxy.contains("WEFTEXT_TLS_PRIVATE_KEY_PATH:?"));
    assert_eq!(
        proxy
            .lines()
            .filter(|line| line.trim() == "read_only: true")
            .count(),
        4,
        "proxy root, proxy secret, certificate, and key must be read-only"
    );
    assert!(proxy.contains("source: nginx_same_host_template"));
    assert!(proxy.contains("source: nginx_same_host_start"));
    assert!(proxy.contains("/etc/nginx/start-nginx-same-host.sh"));
    assert!(proxy.contains("http://127.0.0.1:8788/api/v1/health/ready"));
    assert!(nginx.contains("proxy_set_header X-Weftext-Proxy-Token"));
    assert!(nginx.contains("listen ${WEFTEXT_HTTPS_PORT} ssl;"));
    assert!(nginx.contains("listen 127.0.0.1:8788;"));
    assert!(nginx.contains("proxy_pass http://127.0.0.1:8787;"));
    assert!(nginx.contains("if ($http_host != \"${WEFTEXT_PUBLIC_AUTHORITY}\")"));
    for header in [
        "Forwarded",
        "X-Forwarded-For",
        "X-Forwarded-Host",
        "X-Forwarded-Proto",
        "X-Real-IP",
    ] {
        assert!(nginx.contains(&format!("proxy_set_header {header} \"\";")));
    }
    assert!(nginx_start.contains("${WEFTEXT_PROXY_SECRET_FILE:?"));
    assert!(nginx_start.contains("reverse-proxy-secret must not be accessible"));
    assert!(nginx_start.contains("TLS private key must not be accessible"));
    assert!(nginx_start.contains("nginx -t -q"));
    assert!(nginx_start.contains("unset WEFTEXT_PROXY_TOKEN"));
    assert!(
        example_environment
            .lines()
            .any(|line| line == "WEFTEXT_SERVER_IMAGE_DIGEST=")
    );
    assert!(
        example_environment
            .lines()
            .any(|line| line == "WEFTEXT_PROXY_IMAGE_DIGEST=")
    );
    assert!(
        example_environment
            .lines()
            .any(|line| { line == "WEFTEXT_PROXY_IMAGE_REPOSITORY=docker.io/library/nginx" })
    );
    assert!(!example_environment.contains("latest"));
    assert!(verifier.contains("--proto '=https'"));
    assert!(!verifier.contains("--insecure"));
}

#[test]
fn docker_compose_validates_same_host_manifest_when_available() {
    let Ok(probe) = Command::new("docker").args(["compose", "version"]).output() else {
        eprintln!("skipping compose schema validation: docker compose is unavailable");
        return;
    };
    if !probe.status.success() {
        eprintln!("skipping compose schema validation: docker compose is unavailable");
        return;
    }

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("deploy")
        .join("compose.same-host.yaml");
    let output = Command::new("docker")
        .args(["compose", "--file"])
        .arg(&manifest)
        .args(["config", "--no-interpolate", "--quiet"])
        .output()
        .expect("run docker compose config");
    assert!(
        output.status.success(),
        "docker compose rejected {}: {}",
        manifest.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn real_server_process_serves_api_and_webui() {
    let _serial = serialize_real_process_test();
    let temp = TempDir::new().expect("temporary workspace parent");
    let root = temp.path().join("Workspace");
    create_workspace(&root).expect("create workspace");
    let control = temp.path().join("ControlPlane");
    let mut child = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start real server process");
    let base = read_server_startup(&mut child, "direct-loopback");
    let mut guard = ProcessGuard(child);

    let health = blocking_get(&base, "/api/v1/health", None);
    assert!(health.starts_with("HTTP/1.1 200"));
    assert!(health.contains("canonical-asciidoc-multirole-acl"));
    let webui = blocking_get(&base, "/", None);
    assert!(webui.starts_with("HTTP/1.1 200"));
    assert!(webui.contains("文缕 Server"));

    guard.terminate_and_wait("server");
}

#[test]
fn real_process_control_plane_lease_blocks_a_second_server_and_releases_on_exit() {
    let _serial = serialize_real_process_test();
    let temp = TempDir::new().expect("temporary workspace parent");
    let root = temp.path().join("Workspace");
    create_workspace(&root).expect("create workspace");
    let control = temp.path().join("ControlPlane");
    let mut first = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start first real server");
    read_server_startup(&mut first, "direct-loopback");
    let mut first_guard = ProcessGuard(first);

    let blocked_stderr_path = temp.path().join("blocked-server.stderr");
    let blocked_stderr =
        std::fs::File::create(&blocked_stderr_path).expect("create second server stderr capture");
    let blocked = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .stdout(Stdio::null())
        .stderr(Stdio::from(blocked_stderr))
        .spawn()
        .expect("run second real server");
    let mut blocked_guard = ProcessGuard(blocked);
    let blocked_status = blocked_guard.wait_for_exit("second real server", Duration::from_secs(5));
    assert!(!blocked_status.success());
    let blocked_stderr =
        std::fs::read_to_string(&blocked_stderr_path).expect("second server stderr UTF-8");
    assert!(blocked_stderr.contains("Server control plane is in use"));
    assert!(blocked_stderr.contains(".weftext-server-control-plane.lease"));

    first_guard.terminate_and_wait("first server");

    let mut reopened = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("reopen real server after lease release");
    read_server_startup(&mut reopened, "direct-loopback");
    let mut reopened_guard = ProcessGuard(reopened);
    reopened_guard.terminate_and_wait("reopened server");
}

#[test]
fn real_server_process_wraps_router_and_extractor_rejections() {
    let _serial = serialize_real_process_test();
    let temp = TempDir::new().expect("temporary workspace parent");
    let root = temp.path().join("Workspace");
    let created = create_workspace(&root).expect("create workspace");
    let control = temp.path().join("ControlPlane");
    let mut child = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start real server process");
    let base = read_server_startup(&mut child, "direct-loopback");
    let origin = format!("http://{base}");
    let _guard = ProcessGuard(child);

    let secret = std::fs::read_to_string(control.join("bootstrap-secret"))
        .expect("bootstrap secret")
        .trim()
        .to_owned();
    let bootstrap = blocking_request(
        &base,
        "POST",
        "/api/v1/auth/bootstrap",
        &format!(r#"{{"bootstrapSecret":"{secret}","password":"correct horse battery staple"}}"#),
        "application/json",
        None,
        Some(&origin),
        Some("same-origin"),
        None,
    );
    assert!(bootstrap.starts_with("HTTP/1.1 200"));
    let cookie = response_cookie(&bootstrap).expect("session cookie");

    let missing_query = blocking_request(
        &base,
        "GET",
        "/api/v1/search",
        "",
        "application/json",
        Some(&cookie),
        None,
        None,
        None,
    );
    assert!(missing_query.starts_with("HTTP/1.1 400"));
    assert!(missing_query.contains("application/json"));
    assert!(missing_query.contains("invalid_query"));

    let malformed = blocking_request(
        &base,
        "POST",
        &format!("/api/v1/documents/{}/preview", created.id),
        "{",
        "application/json",
        Some(&cookie),
        Some(&origin),
        Some("same-origin"),
        None,
    );
    assert!(malformed.starts_with("HTTP/1.1 400"));
    assert!(malformed.contains("application/json"));
    assert!(malformed.contains("invalid_json"));

    let wrong_method = blocking_request(
        &base,
        "POST",
        "/api/v1/health",
        "",
        "application/json",
        None,
        Some(&origin),
        Some("same-origin"),
        None,
    );
    assert!(wrong_method.starts_with("HTTP/1.1 405"));
    assert!(wrong_method.contains("application/json"));
    assert!(wrong_method.contains("method_not_allowed"));

    let anonymous = blocking_get(&base, "/api/v1/workspace", None);
    assert!(anonymous.starts_with("HTTP/1.1 401"));
    assert!(anonymous.contains("authentication_required"));
    let wrong_origin = blocking_request(
        &base,
        "POST",
        "/api/v1/auth/login",
        r#"{"login":"owner","password":"wrong password value"}"#,
        "application/json",
        None,
        Some("http://attacker.invalid"),
        Some("same-origin"),
        None,
    );
    assert!(wrong_origin.starts_with("HTTP/1.1 403"));
    assert!(wrong_origin.contains("csrf_rejected"));
}

#[test]
fn real_server_process_rejects_non_loopback_bind() {
    let _serial = serialize_real_process_test();
    let temp = TempDir::new().expect("temporary workspace parent");
    let root = temp.path().join("Workspace");
    create_workspace(&root).expect("create workspace");
    let control = temp.path().join("ControlPlane");
    let output = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "0.0.0.0:0"])
        .output()
        .expect("run server process");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr UTF-8");
    assert!(stderr.contains("AUTHENTICATION_REQUIRED_FOR_NON_LOOPBACK"));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one real-process scenario proves recovery, proxy enforcement, and restart together"
)]
fn real_process_enforces_same_host_proxy_and_recovers_before_ready() {
    let _serial = serialize_real_process_test();
    let temp = TempDir::new().expect("temporary workspace parent");
    let root = temp.path().join("Workspace");
    let workspace = create_workspace(&root).expect("create workspace");
    #[cfg(not(debug_assertions))]
    let _ = &workspace;
    #[cfg(debug_assertions)]
    let transaction = {
        let plan = plan_create_child_node(&root, workspace.id, "RecoveryProbe")
            .expect("plan recovery probe");
        prepare_workspace_transaction_recovery_fixture(&plan).expect("persist prepared journal")
    };
    let control = temp.path().join("ControlPlane");
    let mut child = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .args(["--same-host-proxy-origin", "https://notes.example.test"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start proxy-mode process");
    let base = read_server_startup(&mut child, "same-host-tls-reverse-proxy");
    let mut guard = ProcessGuard(child);
    #[cfg(debug_assertions)]
    assert!(!transaction.exists(), "startup must clean prepared journal");
    let proxy_secret_path = control.join("reverse-proxy-secret");
    let token = std::fs::read_to_string(&proxy_secret_path)
        .expect("reverse proxy secret")
        .trim()
        .to_owned();
    let proxy_header = format!("X-Weftext-Proxy-Token: {token}\r\n");

    let missing = blocking_request_with_extra_headers(
        &base,
        "GET",
        "/api/v1/health/ready",
        "",
        "application/json",
        None,
        None,
        None,
        Some("notes.example.test"),
        "",
    );
    assert!(missing.starts_with("HTTP/1.1 403"));
    assert!(missing.contains("proxy_boundary_rejected"));

    let ready = blocking_request_with_extra_headers(
        &base,
        "GET",
        "/api/v1/health/ready",
        "",
        "application/json",
        None,
        None,
        None,
        Some("notes.example.test"),
        &proxy_header,
    );
    assert!(ready.starts_with("HTTP/1.1 200"));
    assert!(ready.contains("\"status\":\"ready\""));
    assert!(
        ready
            .to_ascii_lowercase()
            .contains("strict-transport-security: max-age=31536000")
    );

    let forwarded = blocking_request_with_extra_headers(
        &base,
        "GET",
        "/api/v1/health/ready",
        "",
        "application/json",
        None,
        None,
        None,
        Some("notes.example.test"),
        &format!("{proxy_header}Forwarded: for=192.0.2.1;proto=https\r\n"),
    );
    assert!(forwarded.starts_with("HTTP/1.1 400"));
    assert!(forwarded.contains("forwarded_header_rejected"));

    let bootstrap_secret = std::fs::read_to_string(control.join("bootstrap-secret"))
        .expect("bootstrap secret")
        .trim()
        .to_owned();
    let bootstrap = blocking_request_with_extra_headers(
        &base,
        "POST",
        "/api/v1/auth/bootstrap",
        &format!(
            r#"{{"bootstrapSecret":"{bootstrap_secret}","password":"correct horse battery staple"}}"#
        ),
        "application/json",
        None,
        Some("https://notes.example.test"),
        Some("same-origin"),
        Some("notes.example.test"),
        &proxy_header,
    );
    assert!(bootstrap.starts_with("HTTP/1.1 200"));
    assert!(bootstrap.contains("Secure"));

    guard.terminate_and_wait("abruptly stopped server");
    let persisted_token = std::fs::read_to_string(&proxy_secret_path)
        .expect("persistent proxy secret")
        .trim()
        .to_owned();
    assert_eq!(persisted_token, token);

    let mut restarted = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .args(["--same-host-proxy-origin", "https://notes.example.test"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("restart proxy-mode process");
    let restart_base = read_server_startup(&mut restarted, "same-host-tls-reverse-proxy");
    let _restart_guard = ProcessGuard(restarted);
    let restarted_ready = blocking_request_with_extra_headers(
        &restart_base,
        "GET",
        "/api/v1/health/ready",
        "",
        "application/json",
        None,
        None,
        None,
        Some("notes.example.test"),
        &proxy_header,
    );
    assert!(restarted_ready.starts_with("HTTP/1.1 200"));
}

#[test]
fn real_process_rejects_non_https_or_ambiguous_proxy_origin() {
    let _serial = serialize_real_process_test();
    let temp = TempDir::new().expect("temporary workspace parent");
    let root = temp.path().join("Workspace");
    create_workspace(&root).expect("create workspace");
    for origin in [
        "http://notes.example.test",
        "https://notes.example.test/path",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
            .arg(&root)
            .arg("--control-plane")
            .arg(temp.path().join("ControlPlane"))
            .args(["--bind", "127.0.0.1:0"])
            .args(["--same-host-proxy-origin", origin])
            .output()
            .expect("run invalid proxy config");
        assert!(!output.status.success());
        assert!(
            String::from_utf8(output.stderr)
                .expect("stderr UTF-8")
                .contains("invalid same-host proxy origin")
        );
    }
}

#[cfg(unix)]
#[test]
fn real_process_sigterm_closes_change_stream_and_exits_cleanly() {
    let _serial = serialize_real_process_test();
    let temp = TempDir::new().expect("temporary workspace parent");
    let root = temp.path().join("Workspace");
    create_workspace(&root).expect("create workspace");
    let control = temp.path().join("ControlPlane");
    let mut child = Command::new(env!("CARGO_BIN_EXE_weftext-server"))
        .arg(&root)
        .arg("--control-plane")
        .arg(&control)
        .args(["--bind", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start real server process");
    let base = read_server_startup(&mut child, "direct-loopback");
    let origin = format!("http://{base}");
    let secret = std::fs::read_to_string(control.join("bootstrap-secret"))
        .expect("bootstrap secret")
        .trim()
        .to_owned();
    let bootstrap = blocking_request(
        &base,
        "POST",
        "/api/v1/auth/bootstrap",
        &format!(r#"{{"bootstrapSecret":"{secret}","password":"correct horse battery staple"}}"#),
        "application/json",
        None,
        Some(&origin),
        Some("same-origin"),
        None,
    );
    let cookie = response_cookie(&bootstrap).expect("session cookie");
    let mut changes = TcpStream::connect(&base).expect("connect change stream");
    changes
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("change stream timeout");
    write!(
        changes,
        "GET /api/v1/changes HTTP/1.1\r\nHost: {base}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"
    )
    .expect("open change stream");
    let mut response_prefix = [0_u8; 512];
    let read = changes
        .read(&mut response_prefix)
        .expect("read change stream headers");
    assert!(String::from_utf8_lossy(&response_prefix[..read]).contains("HTTP/1.1 200"));

    let pid = child.id().to_string();
    let signal = Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .expect("send SIGTERM");
    assert!(signal.success());
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll server exit") {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "Server did not drain after SIGTERM"
        );
        thread::sleep(Duration::from_millis(25));
    };
    assert!(status.success());
    let mut remaining = Vec::new();
    changes
        .read_to_end(&mut remaining)
        .expect("change stream closes during drain");
}

fn blocking_get(address: &str, path: &str, cookie: Option<&str>) -> String {
    blocking_request(
        address,
        "GET",
        path,
        "",
        "application/json",
        cookie,
        None,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn blocking_request(
    address: &str,
    method: &str,
    path: &str,
    body: &str,
    content_type: &str,
    cookie: Option<&str>,
    origin: Option<&str>,
    csrf: Option<&str>,
    host: Option<&str>,
) -> String {
    blocking_request_with_extra_headers(
        address,
        method,
        path,
        body,
        content_type,
        cookie,
        origin,
        csrf,
        host,
        "",
    )
}

#[allow(clippy::too_many_arguments)]
fn blocking_request_with_extra_headers(
    address: &str,
    method: &str,
    path: &str,
    body: &str,
    content_type: &str,
    cookie: Option<&str>,
    origin: Option<&str>,
    csrf: Option<&str>,
    host: Option<&str>,
    extra_headers: &str,
) -> String {
    let cookie = cookie.map_or_else(String::new, |value| format!("Cookie: {value}\r\n"));
    let origin = origin.map_or_else(String::new, |value| format!("Origin: {value}\r\n"));
    let csrf = csrf.map_or_else(String::new, |value| format!("X-Weftext-CSRF: {value}\r\n"));
    let host = host.unwrap_or(address);
    let attempts = if method == "GET" { 3 } else { 1 };
    'request: for attempt in 1..=attempts {
        let mut stream = TcpStream::connect(address).expect("connect process API");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{origin}{csrf}{cookie}{extra_headers}Connection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .expect("write request");
        let mut response = Vec::new();
        let mut read_buffer = [0_u8; 8 * 1024];
        loop {
            match stream.read(&mut read_buffer) {
                Ok(0) => break,
                Ok(read) => {
                    response.extend_from_slice(&read_buffer[..read]);
                    if http_response_is_complete(&response) {
                        break;
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                    ) && http_response_is_complete(&response) =>
                {
                    break;
                }
                Err(error)
                    if attempt < attempts
                        && response.is_empty()
                        && matches!(
                            error.kind(),
                            io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                        ) =>
                {
                    thread::sleep(Duration::from_millis(20));
                    continue 'request;
                }
                Err(error) => panic!(
                    "read {method} {path} response: {error}; partial response: {}",
                    String::from_utf8_lossy(&response)
                ),
            }
        }
        return String::from_utf8(response).expect("response UTF-8");
    }
    unreachable!("request attempt loop always returns or panics")
}

fn http_response_is_complete(response: &[u8]) -> bool {
    let Some(header_end) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let header_end = header_end + 4;
    let Ok(headers) = std::str::from_utf8(&response[..header_end]) else {
        return false;
    };
    let status_has_no_body = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .is_some_and(|code| (100..200).contains(&code) || matches!(code, 204 | 304));
    if status_has_no_body {
        return true;
    }
    let chunked = headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.eq_ignore_ascii_case("transfer-encoding")
            && value
                .split(',')
                .any(|coding| coding.trim().eq_ignore_ascii_case("chunked"))
    });
    if chunked {
        return chunked_body_is_complete(&response[header_end..]);
    }
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    }) == Some(response.len() - header_end)
}

fn chunked_body_is_complete(mut body: &[u8]) -> bool {
    loop {
        let Some(size_end) = body.windows(2).position(|window| window == b"\r\n") else {
            return false;
        };
        let Ok(size_line) = std::str::from_utf8(&body[..size_end]) else {
            return false;
        };
        let Some(size_token) = size_line.split(';').next() else {
            return false;
        };
        let Ok(size) = usize::from_str_radix(size_token.trim(), 16) else {
            return false;
        };
        body = &body[size_end + 2..];
        if size == 0 {
            return body == b"\r\n" || body.ends_with(b"\r\n\r\n");
        }
        let Some(after_chunk) = body.get(size..) else {
            return false;
        };
        if !after_chunk.starts_with(b"\r\n") {
            return false;
        }
        body = &after_chunk[2..];
    }
}

#[test]
fn raw_http_completion_recognizes_fixed_and_chunked_message_boundaries() {
    assert!(http_response_is_complete(
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"
    ));
    assert!(!http_response_is_complete(
        b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nok"
    ));
    assert!(http_response_is_complete(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nok\r\n0\r\n\r\n"
    ));
    assert!(http_response_is_complete(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, chunked\r\n\r\n2;kind=text\r\nok\r\n0\r\nX-Test: yes\r\n\r\n"
    ));
    assert!(!http_response_is_complete(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\no"
    ));
}

fn response_cookie(response: &str) -> Option<String> {
    response.lines().find_map(|line| {
        line.strip_prefix("set-cookie: ")
            .or_else(|| line.strip_prefix("Set-Cookie: "))
            .and_then(|value| value.split(';').next())
            .map(str::to_owned)
    })
}
