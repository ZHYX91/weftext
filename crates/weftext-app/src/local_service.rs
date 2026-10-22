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
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// An explicit document locator; Core still qualifies identity and restores Draft.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
/// is [`crate::launch::dispatch_worker`].
pub struct LocalService {
    listener: tokio::net::TcpListener,
    router: Router,
    browser_url: String,
    state: api::AppState,
    admitting: Arc<AtomicBool>,
}
/// The HTTP tail has stopped, but the original Core lease is still owned.
/// A native supervisor can continue preserving its exposed inputs after an
/// HTTP failure, then release this exact lease after its other owners drain.
pub(crate) struct ServiceStopped {
    pub core: Arc<Mutex<weftext_core::Core>>,
    pub outcome: io::Result<()>,
}

impl LocalService {
    /// Core failures retain `CoreError` in `io::Error::get_ref`; a host may
    /// inspect its code without treating paths or diagnostic text as authority.
    pub async fn bind(options: LocalServiceOptions) -> io::Result<Self> {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, options.port)).await?;
        Self::bind_with_listener(options, listener).await
    }

    /// A native-attachable owner supplies its already reserved stable-origin
    /// listener after acquiring the sole physical-profile owner lease.
    pub(crate) async fn bind_with_listener(
        options: LocalServiceOptions,
        listener: tokio::net::TcpListener,
    ) -> io::Result<Self> {
        Self::bind_core(options, listener, None).await
    }

    #[cfg(windows)]
    pub(crate) async fn bind_existing_with_listener(
        options: LocalServiceOptions,
        listener: tokio::net::TcpListener,
        physical_identity: &str,
        device_id: &str,
    ) -> io::Result<Self> {
        Self::bind_core(options, listener, Some((physical_identity, device_id))).await
    }

    async fn bind_core(
        options: LocalServiceOptions,
        listener: tokio::net::TcpListener,
        existing: Option<(&str, &str)>,
    ) -> io::Result<Self> {
        let bound = listener.local_addr()?;
        if bound.ip() != std::net::IpAddr::V4(Ipv4Addr::LOCALHOST)
            || bound.port() == 0
            || (options.port != 0 && options.port != bound.port())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The owned listener does not match the exact local bind contract",
            ));
        }
        let (mut core, data) = match existing {
            #[cfg(windows)]
            Some((physical, device)) => {
                api::host_environment::open_existing_core(options.data_dir, physical, device)?
            }
            #[cfg(not(windows))]
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "This local owner authority is unavailable",
                ));
            }
            None => api::host_environment::open_core(options.data_dir)?,
        };
        let initial = options
            .initial_document
            .map(|document| {
                let path = std::path::absolute(document.path)?;
                let root = document.root.map(std::path::absolute).transpose()?;
                let opened = core.open(path, root).map_err(io::Error::other)?;
                serde_json::to_value(opened)
                    .map(Arc::new)
                    .map_err(io::Error::other)
            })
            .transpose()?;
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
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(api::remote_sync::Store::default())),
            initial,
        };
        if let Some(document) = state.initial.as_ref() {
            api::remember(&state, document).map_err(|error| io::Error::other(error.message))?;
        }
        let admitting = Arc::new(AtomicBool::new(true));
        let router = router(state.clone()).layer(middleware::from_fn_with_state(
            admitting.clone(),
            admit_request,
        ));
        Ok(Self {
            listener,
            router,
            browser_url: format!("http://127.0.0.1:{port}/#token={token}"),
            state,
            admitting,
        })
    }

    /// Contains the local session credential; hand it only to the selected UI.
    pub fn browser_url(&self) -> &str {
        &self.browser_url
    }

    /// Only trusted in-process native/broker coordinators receive this handle;
    /// clients never obtain Core, session credentials or arbitrary HTTP access.
    pub(crate) fn authority(&self) -> api::AppState {
        self.state.clone()
    }

    /// Stop new requests, drain accepted HTTP and all registered async/blocking
    /// work. In-process authority handles must also be dropped by the host
    /// before it can claim that the final Core lease has been released.
    pub async fn serve_until(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> io::Result<()> {
        let stopped = self.serve_retaining_until(shutdown).await;
        let core = Arc::try_unwrap(stopped.core).map_err(|_| {
            io::Error::other("Core authority is still retained after accepted work drained")
        })?;
        drop(
            core.into_inner()
                .map_err(|_| io::Error::other("Core authority was poisoned during shutdown"))?,
        );
        stopped.outcome
    }

    pub(crate) async fn serve_retaining_until(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> ServiceStopped {
        let admitting = self.admitting;
        let stopping_state = self.state.clone();
        let result = axum::serve(self.listener, self.router)
            .with_graceful_shutdown(async move {
                shutdown.await;
                admitting.store(false, Ordering::Release);
                close_jobs(&stopping_state);
            })
            .await;
        close_jobs(&self.state);
        let drained = api::work::registry().drain().await;
        ServiceStopped {
            core: self.state.core,
            outcome: result.and(drained),
        }
    }
}

fn close_jobs(state: &api::AppState) {
    match state.exports.lock() {
        Ok(mut exports) => exports.close(),
        Err(_) => api::work::fault(),
    }
    match state.sync_remote.lock() {
        Ok(mut sync) => sync.close(),
        Err(_) => api::work::fault(),
    }
}

pub(crate) async fn admit_request(
    axum::extract::State(admitting): axum::extract::State<Arc<AtomicBool>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if !admitting.load(Ordering::Acquire) {
        use axum::response::IntoResponse;
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "error": {"code":"hostClosing", "message":"The owner is draining accepted work"}
            })),
        )
            .into_response();
    }
    next.run(request).await
}

fn router(state: api::AppState) -> Router {
    Router::new()
        .merge(api::writing_api::router())
        .merge(api::builtin_module_api::router())
        .merge(api::data_api::router())
        .merge(api::desktop_api::router())
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
        .route("/api/draft/resume", post(api::resume_draft))
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
    async fn native_http_tail_hands_back_the_same_core_before_final_authority_release() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("core");
        let service = LocalService::bind(LocalServiceOptions::new(data.clone()))
            .await
            .unwrap();
        let state = service.authority();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let host = tokio::spawn(service.serve_retaining_until(async {
            let _ = stopped.await;
        }));
        stop.send(()).unwrap();
        let returned = tokio::time::timeout(std::time::Duration::from_secs(10), host)
            .await
            .unwrap()
            .unwrap();
        assert!(returned.outcome.is_ok());
        assert!(Arc::ptr_eq(&returned.core, &state.core));
        assert!(weftext_core::Core::new(&data).is_err());
        drop(returned.core);
        assert!(weftext_core::Core::new(&data).is_err());
        drop(state);
        drop(weftext_core::Core::new(&data).unwrap());
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
        assert!(error.to_string().contains("Cannot bind Weftext"));
        let retry = LocalService::bind(LocalServiceOptions::new(data))
            .await
            .unwrap();
        drop(retry);
    }
}
