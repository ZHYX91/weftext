//! One Core-backed local HTTP service, independent of a browser or GUI runtime.
use crate as api;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};
use std::{
    collections::HashMap,
    future::Future,
    io,
    net::Ipv4Addr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// An explicit document locator; Core still qualifies identity and restores Draft.
#[derive(Clone, Debug)]
pub struct InitialDocument {
    pub path: PathBuf,
    pub root: Option<PathBuf>,
}

/// Input for a host-owned local service. No tray, browser or scheduler is started.
#[derive(Clone, Debug)]
pub struct LocalServiceOptions {
    pub data_dir: PathBuf,
    pub port: u16,
    pub initial_document: Option<InitialDocument>,
}

impl LocalServiceOptions {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            port: 0,
            initial_document: None,
        }
    }
}

/// Owns the listener and router, including their shared single Core lease.
/// A native host can keep its GUI thread separate and run `serve_until` on Tokio.
/// Each executable hosting this service must also dispatch `--render-worker` and
/// `--notes-edit-worker` before GUI initialization or Core acquisition: isolated
/// workers start through the current executable. The existing console dispatch
/// is [`crate::launch::run`].
pub struct LocalService {
    listener: tokio::net::TcpListener,
    router: Router,
    browser_url: String,
}

impl LocalService {
    /// Core failures retain `CoreError` in `io::Error::get_ref`; a host may
    /// inspect its code without treating paths or diagnostic text as authority.
    pub async fn bind(options: LocalServiceOptions) -> io::Result<Self> {
        let (mut core, data) = api::host_environment::open_core(options.data_dir)?;
        let initial = options
            .initial_document
            .map(|document| {
                let path = std::path::absolute(document.path)?;
                let root = document.root.map(std::path::absolute).transpose()?;
                let opened = core.open(path, root).map_err(io::Error::other)?;
                serde_json::to_value(opened).map(Arc::new).map_err(|error| {
                    let mut error = api::CoreError::from(error);
                    error.explanation =
                        weftext_messages::UiMessage::error_local_service_document_encoding_failed();
                    io::Error::other(error)
                })
            })
            .transpose()?;
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, options.port))
            .await
            .map_err(|error| {
                let kind = error.kind();
                let mut error = api::CoreError::from(error);
                error.explanation = weftext_messages::UiMessage::error_local_service_bind_failed(
                    weftext_messages::params::ErrorLocalServiceBindFailed {
                        port: options.port.to_string(),
                    },
                );
                io::Error::new(kind, error)
            })?;
        let port = listener.local_addr()?.port();
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let state = api::AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from(token.as_str()),
            port,
            locations: Arc::new(Mutex::new(HashMap::new())),
            renders: Arc::new(Mutex::new(HashMap::new())),
            editor_versions: Arc::new(Mutex::new(HashMap::new())),
            exports: Arc::new(Mutex::new(api::export_api::ExportStore::with_legacy_root(
                data.join("legacy-office"),
            ))),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(api::remote_sync::Store::default())),
            initial,
        };
        if let Some(document) = state.initial.as_ref() {
            api::remember(&state, document).map_err(io::Error::other)?;
        }
        let router = router(state);
        Ok(Self {
            listener,
            router,
            browser_url: format!("http://127.0.0.1:{port}/#token={token}"),
        })
    }

    /// Contains the local session credential; hand it only to the selected UI.
    pub fn browser_url(&self) -> &str {
        &self.browser_url
    }

    /// The caller owns shutdown. Returning drains HTTP requests and drops Core.
    pub async fn serve_until(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> io::Result<()> {
        axum::serve(self.listener, self.router)
            .with_graceful_shutdown(shutdown)
            .await
    }
}

fn router(state: api::AppState) -> Router {
    Router::new()
        .merge(api::writing_api::router())
        .merge(api::client_preferences_api::router())
        .merge(api::builtin_module_api::router())
        .merge(api::data_api::router())
        .merge(api::embedded_query_api::router())
        .merge(api::history_api::router())
        .merge(api::knowledge_api::router())
        .merge(api::processing_api::router())
        .merge(api::recognition_api::router())
        .merge(api::media_api::router())
        .merge(api::navigation_api::router())
        .merge(api::sync_api::router())
        .merge(api::remote_sync::router())
        .merge(api::export_api::router())
        .route("/api/bootstrap", get(api::bootstrap))
        .route("/api/open", post(api::open))
        .route("/api/library/open", post(api::inventory))
        .route("/api/library/create", post(api::create_library))
        .route(
            "/api/library/create-defaults",
            get(api::library_creation_defaults),
        )
        .route(
            "/api/library/create-cancel",
            post(api::cancel_library_creation),
        )
        .route("/api/note/create", post(api::create_note))
        .route("/api/save", post(api::save))
        .route("/api/save-as", post(api::save_as))
        .route("/api/draft", post(api::draft))
        .route("/api/draft/resolve", post(api::resolve_draft))
        .route("/api/render", post(api::render))
        .route("/api/editor/preview", post(api::proposal_preview))
        .route("/api/paged", post(api::paged))
        .route("/api/edit-literal", post(api::edit_literal))
        .route("/api/link/resolve", post(api::link_resolve))
        .route("/api/recovery", get(api::recovery))
        .route("/api/recovery/inspect", post(api::inspect_recovery))
        .route("/api/recovery/resolve", post(api::resolve_recovery))
        .fallback(api::assets)
        .layer(DefaultBodyLimit::max(96 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), api::boundary))
        .with_state(state)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn authenticated_preference_json_rejections_use_typed_wire_and_preserve_original_state() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("state");
        let service = LocalService::bind(LocalServiceOptions::new(data.clone()))
            .await
            .unwrap();
        let address = url::Url::parse(service.browser_url()).unwrap();
        let token = address
            .fragment()
            .unwrap()
            .strip_prefix("token=")
            .unwrap()
            .to_owned();
        let endpoint = format!(
            "http://127.0.0.1:{}/api/client-preferences/update",
            address.port().unwrap()
        );
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let host = tokio::spawn(service.serve_until(async {
            let _ = stopped.await;
        }));
        let client = reqwest::Client::new();
        let saved = client
            .post(&endpoint)
            .header("x-weftext-token", &token)
            .json(&serde_json::json!({"expectedRevision":0,"preferences":{"uiLocale":"en-US"}}))
            .send()
            .await
            .unwrap();
        assert!(saved.status().is_success());
        assert_eq!(
            saved.json::<serde_json::Value>().await.unwrap()["revision"],
            1
        );
        let path = data.join("device-state/client-preferences.json");
        let original = std::fs::read(&path).unwrap();
        for (credential, content_type, body, status, key) in [
            (
                None,
                Some("application/json"),
                "{",
                reqwest::StatusCode::FORBIDDEN,
                "error.localSessionInvalid",
            ),
            (
                Some(token.as_str()),
                Some("application/json"),
                "{",
                reqwest::StatusCode::BAD_REQUEST,
                "error.api.jsonSyntaxInvalid",
            ),
            (
                Some(token.as_str()),
                None,
                "{",
                reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "error.api.jsonContentTypeRequired",
            ),
            (
                Some(token.as_str()),
                Some("application/json"),
                r#"{"expectedRevision":1,"preferences":{"uiLocale":"en"}}"#,
                reqwest::StatusCode::UNPROCESSABLE_ENTITY,
                "error.api.jsonDataInvalid",
            ),
            (
                Some(token.as_str()),
                Some("application/json"),
                r#"{"expectedRevision":1,"preferences":{"uiLocale":{"en-US":null}}}"#,
                reqwest::StatusCode::UNPROCESSABLE_ENTITY,
                "error.api.jsonDataInvalid",
            ),
        ] {
            let mut request = client.post(&endpoint).body(body);
            if let Some(credential) = credential {
                request = request.header("x-weftext-token", credential);
            }
            if let Some(content_type) = content_type {
                request = request.header("content-type", content_type);
            }
            let response = request.send().await.unwrap();
            assert_eq!(response.status(), status);
            let response: serde_json::Value = response.json().await.unwrap();
            assert_eq!(response["error"]["explanation"]["key"], key);
            assert!(response["error"].get("message").is_none());
            assert!(response["error"]["causes"].is_array());
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
        assert!(!data.join("device-state/identity.json").exists());
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), host)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn host_shutdown_releases_the_only_core_lease_and_listener() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("state");
        let service = LocalService::bind(LocalServiceOptions::new(data.clone()))
            .await
            .unwrap();
        let address = url::Url::parse(service.browser_url()).unwrap();
        let token = address
            .fragment()
            .unwrap()
            .strip_prefix("token=")
            .unwrap()
            .to_owned();
        let endpoint = format!("http://127.0.0.1:{}", address.port().unwrap());
        let duplicate = LocalService::bind(LocalServiceOptions::new(data.clone()))
            .await
            .err()
            .expect("duplicate Core accepted");
        let original = duplicate
            .get_ref()
            .and_then(|error| error.downcast_ref::<weftext_core::CoreError>())
            .expect("Core error type was discarded");
        assert_eq!(original.code, "instanceBusy");
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let host = tokio::spawn(service.serve_until(async {
            let _ = stopped.await;
        }));
        let client = reqwest::Client::new();
        let denied = client
            .get(format!("{endpoint}/api/bootstrap"))
            .send()
            .await
            .unwrap();
        assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
        let allowed = client
            .get(format!("{endpoint}/api/bootstrap"))
            .header("x-weftext-token", token)
            .send()
            .await
            .unwrap();
        assert!(allowed.status().is_success());
        drop(allowed);
        drop(denied);
        drop(client);
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), host)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let again = LocalService::bind(LocalServiceOptions {
            data_dir: data,
            port: address.port().unwrap(),
            initial_document: None,
        })
        .await
        .unwrap();
        drop(again);
    }

    #[tokio::test]
    async fn occupied_port_is_an_error_and_does_not_retain_the_core_lease() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("state");
        let occupied = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let options = LocalServiceOptions {
            data_dir: data.clone(),
            port: occupied.local_addr().unwrap().port(),
            initial_document: None,
        };
        let error = match LocalService::bind(options).await {
            Ok(_) => panic!("occupied port accepted"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
        let original = error
            .get_ref()
            .and_then(|error| error.downcast_ref::<api::CoreError>())
            .expect("Bind error discarded its typed Core/OS facts");
        assert_eq!(
            original.explanation,
            weftext_messages::UiMessage::error_local_service_bind_failed(
                weftext_messages::params::ErrorLocalServiceBindFailed {
                    port: occupied.local_addr().unwrap().port().to_string()
                },
            )
        );
        assert_eq!(original.diagnostics.len(), 1);
        assert_eq!(
            original.diagnostics[0].origin,
            weftext_messages::DiagnosticOrigin::Os
        );
        let retry = LocalService::bind(LocalServiceOptions::new(data))
            .await
            .unwrap();
        drop(retry);
    }
}
