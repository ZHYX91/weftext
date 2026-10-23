//! Explicit hosted client with fixed routes and no ambient proxy or retry.
use super::input::Target;
use crate::client_protocol::{CallKind, LocalMethod, MAX_REPLY_BYTES, MAX_WIRE_BYTES};
use serde_json::Value;
use std::time::Duration;

pub(crate) struct Remote {
    client: reqwest::Client,
    origin: url::Url,
}

#[derive(Debug)]
pub(crate) struct TransportError {
    pub code: &'static str,
    pub may_have_been_accepted: bool,
}

pub(crate) struct Reply {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl Remote {
    pub fn from_environment(
        endpoint: &str,
        variable: &str,
        allow_loopback_http: bool,
    ) -> Result<Self, TransportError> {
        let mut bytes = read_credential(variable)?;
        let result = std::str::from_utf8(&bytes)
            .map_err(|_| failure("credentialFormat", false))
            .and_then(|secret| Self::new(endpoint, secret, allow_loopback_http));
        bytes.fill(0);
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
        result
    }

    fn new(
        endpoint: &str,
        secret: &str,
        allow_loopback_http: bool,
    ) -> Result<Self, TransportError> {
        let origin = validate_endpoint(endpoint, allow_loopback_http)?;
        if !(32..=512).contains(&secret.len())
            || !secret.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
        {
            return Err(failure("credentialFormat", false));
        }
        let mut authorization = reqwest::header::HeaderValue::from_str(&format!("Bearer {secret}"))
            .map_err(|_| failure("credentialFormat", false))?;
        authorization.set_sensitive(true);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::AUTHORIZATION, authorization);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()
            .map_err(|_| failure("remoteClient", false))?;
        Ok(Self { client, origin })
    }

    pub async fn target(&self) -> Result<Target, TransportError> {
        let reply = self.call(CallKind::Capabilities, b"{}").await?;
        if reply.status == 401 || reply.status == 403 {
            return Err(failure("authentication", false));
        }
        if !(200..300).contains(&reply.status) {
            return Err(failure("remoteIdentity", false));
        }
        if !reply
            .content_type
            .split(';')
            .next()
            .is_some_and(|v| v.trim() == "application/json")
        {
            return Err(failure("remoteIdentity", false));
        }
        let value =
            super::input::parse_json(&reply.body).map_err(|_| failure("remoteIdentity", false))?;
        let name = |key: &str| {
            value[key]
                .as_str()
                .filter(|v| !v.is_empty() && v.len() <= 256)
                .map(str::to_owned)
                .ok_or_else(|| failure("remoteIdentity", false))
        };
        Ok(Target::Remote {
            origin: self.origin.as_str().to_owned(),
            actor: name("actor")?,
            deployment: name("deployment")?,
        })
    }

    pub async fn call(&self, kind: CallKind, wire: &[u8]) -> Result<Reply, TransportError> {
        let (method, route) = kind
            .remote_route()
            .ok_or_else(|| failure("unsupportedRemote", false))?;
        validate_remote_body(kind, wire)?;
        let url = self
            .origin
            .join(route)
            .map_err(|_| failure("invalidEndpoint", false))?;
        // A static absolute path is resolved against the already validated root.
        if url.origin() != self.origin.origin() {
            return Err(failure("invalidEndpoint", false));
        }
        let builder = match method {
            LocalMethod::Get => self.client.get(url),
            LocalMethod::Post => self
                .client
                .post(url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(wire.to_vec()),
        };
        let mut response = builder
            .send()
            .await
            .map_err(|_| failure("remoteConnection", true))?;
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        if response
            .content_length()
            .is_some_and(|n| n > MAX_REPLY_BYTES as u64)
        {
            return Err(failure("replyBudget", true));
        }
        let mut body = Vec::new();
        while let Some(part) = response
            .chunk()
            .await
            .map_err(|_| failure("remoteResponse", true))?
        {
            if part.len() > MAX_REPLY_BYTES.saturating_sub(body.len()) {
                return Err(failure("replyBudget", true));
            }
            body.extend_from_slice(&part);
        }
        if body.is_empty() {
            return Err(failure("emptyReply", true));
        }
        Ok(Reply {
            status,
            content_type,
            body,
        })
    }
}

fn failure(code: &'static str, may_have_been_accepted: bool) -> TransportError {
    TransportError {
        code,
        may_have_been_accepted,
    }
}

pub(crate) fn validate_endpoint(
    value: &str,
    allow_loopback_http: bool,
) -> Result<url::Url, TransportError> {
    let url = url::Url::parse(value).map_err(|_| failure("invalidEndpoint", false))?;
    let literal_loopback = url.host_str().is_some_and(|host| {
        host.trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    });
    if !(url.scheme() == "https"
        || allow_loopback_http && url.scheme() == "http" && literal_loopback)
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err(failure("invalidEndpoint", false));
    }
    Ok(url)
}
pub(crate) fn read_credential(variable: &str) -> Result<Vec<u8>, TransportError> {
    if variable.is_empty()
        || variable.len() > 128
        || !variable
            .bytes()
            .enumerate()
            .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || i > 0 && b.is_ascii_digit())
    {
        return Err(failure("credentialSource", false));
    }
    std::env::var(variable)
        .map(String::into_bytes)
        .map_err(|_| failure("credentialsRequired", false))
}

/// Remote endpoints never receive local disk locators or client-selected actors.
fn validate_remote_body(kind: CallKind, bytes: &[u8]) -> Result<(), TransportError> {
    if bytes.len() > MAX_WIRE_BYTES {
        return Err(failure("inputBudget", false));
    }
    let body = super::input::parse_json(bytes).map_err(|_| failure("inputJson", false))?;
    validate_remote_value(kind, &body)
}

pub(super) fn validate_remote_value(kind: CallKind, body: &Value) -> Result<(), TransportError> {
    let object = body
        .as_object()
        .ok_or_else(|| failure("inputShape", false))?;
    if object.keys().any(|key| {
        matches!(
            key.as_str(),
            "root"
                | "path"
                | "parent"
                | "directory"
                | "dataDir"
                | "actor"
                | "token"
                | "authorization"
                | "endpoint"
        )
    }) {
        return Err(failure("remoteLocalPath", false));
    }
    // A path buried in a Query expression or author source is authored data, not
    // an OS capability. The actual hosted DTO still validates every allowed key.
    if kind == CallKind::Open
        && (object.len() != 1 || object.get("resource").and_then(Value::as_str).is_none())
    {
        return Err(failure("remoteSelector", false));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    const TEST_CREDENTIAL: &str = "public-fixture-credential-000000000000";
    async fn request(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let n = stream.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0);
            bytes.extend_from_slice(&buffer[..n]);
            assert!(bytes.len() < 1024 * 1024);
            if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                let header = std::str::from_utf8(&bytes[..end]).unwrap();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.split_once(':')
                            .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                            .map(|(_, v)| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + length {
                    return bytes;
                }
            }
        }
    }
    async fn fixture(response: Vec<u8>) -> (String, tokio::task::JoinHandle<(Vec<u8>, bool)>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let bytes = request(&mut stream).await;
            stream.write_all(&response).await.unwrap();
            stream.shutdown().await.unwrap();
            drop(stream);
            let retried = tokio::time::timeout(Duration::from_millis(150), listener.accept())
                .await
                .is_ok();
            (bytes, retried)
        });
        (endpoint, task)
    }
    #[test]
    fn explicit_origin_and_literal_loopback_only() {
        for endpoint in [
            "http://notes.test/",
            "https://user:secret@notes.test/",
            "https://notes.test/base/",
            "https://notes.test/?token=x",
            "https://notes.test/#x",
        ] {
            assert!(validate_endpoint(endpoint, true).is_err());
        }
        assert!(validate_endpoint("http://localhost/", true).is_err());
        assert!(validate_endpoint("http://127.0.0.1/", false).is_err());
        assert!(validate_endpoint("http://127.0.0.1/", true).is_ok());
        assert!(validate_endpoint("https://notes.test/", false).is_ok());
    }
    #[test]
    fn credentials_reject_whitespace_controls_and_non_ascii_before_http() {
        for secret in [
            "public fixture credential 0000000000000",
            "public-fixture-credential-0000000000\n",
            "public-fixture-credential-0000000000\r",
            "public-fixture-credential-0000000000é",
        ] {
            assert!(Remote::new("https://notes.test/", secret, false).is_err());
        }
    }
    #[test]
    fn remote_source_is_opaque_but_disk_capabilities_are_rejected() {
        assert!(
            validate_remote_body(
                CallKind::Save,
                br#"{"source":"author mentions C:\\Notes","operationId":"id"}"#
            )
            .is_ok()
        );
        assert!(
            validate_remote_body(CallKind::Open, br#"{"path":"C:\\Notes\\private.typ"}"#).is_err()
        );
        assert!(
            validate_remote_body(CallKind::QueryRun, br#"{"root":"C:\\Notes","plan":{}}"#).is_err()
        );
    }
    #[tokio::test]
    async fn actual_transport_preserves_first_wire_bytes_and_does_not_retry() {
        let response=b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}".to_vec();
        let (endpoint, server) = fixture(response).await;
        let client = Remote::new(&endpoint, TEST_CREDENTIAL, true).unwrap();
        let wire="{ \"source\": \"中文🙂\\r\\n\", \"operationId\": \"00000000-0000-4000-8000-000000000001\" }".as_bytes();
        let reply = client.call(CallKind::Save, wire).await.unwrap();
        assert_eq!(reply.status, 200);
        let (request, retried) = server.await.unwrap();
        assert!(!retried);
        assert!(request.ends_with(wire));
        assert!(request.starts_with(b"POST /api/hosted/save HTTP/1.1\r\n"));
    }
    #[tokio::test]
    async fn redirects_are_not_followed_with_the_authorization_credential() {
        let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let response=format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{}/stolen\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}",destination.local_addr().unwrap()).into_bytes();
        let (endpoint, server) = fixture(response).await;
        let client = Remote::new(&endpoint, TEST_CREDENTIAL, true).unwrap();
        let reply = client.call(CallKind::Save, b"{}").await.unwrap();
        assert_eq!(reply.status, 307);
        assert!(!server.await.unwrap().1);
        assert!(
            tokio::time::timeout(Duration::from_millis(150), destination.accept())
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn a_truncated_response_preserves_uncertainty_and_never_reposts() {
        let (endpoint,server)=fixture(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{}".to_vec()).await;
        let client = Remote::new(&endpoint, TEST_CREDENTIAL, true).unwrap();
        let error = client.call(CallKind::Save, b"{}").await.err().unwrap();
        assert_eq!(error.code, "remoteResponse");
        assert!(error.may_have_been_accepted);
        assert!(!server.await.unwrap().1);
    }
    #[tokio::test]
    async fn declared_oversized_replies_are_bounded_before_allocation() {
        let (endpoint,server)=fixture(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",MAX_REPLY_BYTES+1).into_bytes()).await;
        let client = Remote::new(&endpoint, TEST_CREDENTIAL, true).unwrap();
        let error = client.call(CallKind::Save, b"{}").await.err().unwrap();
        assert_eq!(error.code, "replyBudget");
        assert!(error.may_have_been_accepted);
        assert!(!server.await.unwrap().1);
    }
}
