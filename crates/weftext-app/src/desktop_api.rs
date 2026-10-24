//! Desktop intent and actual document-draft readback on the same local Core.
//! No OS permit, notification submission or arbitrary host command is an HTTP API.
use crate::{ApiResult, AppState, RequestJson, core_call, value};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NotificationState {
    #[default]
    Unavailable,
    Unknown,
    Enabled,
    DisabledForApplication,
    DisabledForUser,
    DisabledByPolicy,
    DisabledByManifest,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Scheduling {
    #[default]
    Inactive,
    Running,
    Paused,
    Restricted,
    Stopping,
}
/// Ephemeral observation from this actual desktop owner. It is neither task
/// data nor a second delivery ledger; headless services stay explicitly inactive.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostStatus {
    pub desktop_active: bool,
    pub notification_state: NotificationState,
    pub scheduling: Scheduling,
    pub scan_complete: bool,
    pub scope_issue_count: usize,
}
pub(crate) fn set_status(state: &AppState, status: HostStatus) -> std::io::Result<()> {
    *state
        .native_status
        .lock()
        .map_err(|_| std::io::Error::other("The desktop observation is unavailable"))? = status;
    Ok(())
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/desktop/preferences", get(preferences))
        .route("/api/desktop/preferences/update", post(update_preferences))
        .route("/api/desktop/status", get(status))
        .route("/api/desktop/document-flush", post(document_flush))
        .route("/api/session/release", post(release_session))
}
async fn status(State(state): State<AppState>) -> ApiResult {
    let observed = state
        .native_status
        .lock()
        .map_err(|_| crate::ApiError::internal(weftext_messages::UiMessage::error_api_desktop_api_the_actual_desktop_status_is_unavailable()))?
        .clone();
    Ok(Json(value(observed)?))
}

async fn preferences(State(state): State<AppState>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, |core| core.desktop_preferences()).await?,
    )?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdatePreferences {
    expected_revision: u64,
    preferences: weftext_core::DesktopPreferences,
}

async fn update_preferences(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<UpdatePreferences>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.update_desktop_preferences(request.expected_revision, request.preferences)
        })
        .await?,
    )?))
}

async fn document_flush(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<weftext_core::DocumentDraftFlush>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.flush_document_draft(request)).await?,
    )?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReleaseSession {
    session_id: String,
}
async fn release_session(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<ReleaseSession>,
) -> ApiResult {
    if !crate::broker::canonical_uuid(&request.session_id) {
        return Err(crate::ApiError::bad(
            weftext_messages::UiMessage::error_api_desktop_api_the_exact_original_session_i_d_is_required(),
        ));
    }
    let id = request.session_id.clone();
    core_call(&state, move |core| core.close_session(&id)).await?;
    crate::forget_session_projection(&state, &request.session_id)?;
    // Closing only an unadopted OpenResult is deliberately not a flush or a
    // native surface close permit. Its Draft and operation facts are untouched.
    Ok(Json(
        serde_json::json!({"released":true,"sessionId":request.session_id}),
    ))
}

#[cfg(all(test, windows))]
mod tests {
    use crate::local_service::{LocalService, LocalServiceOptions};

    #[tokio::test]
    async fn release_is_exact_authenticated_and_keeps_original_author_drafts() {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let source = project.join("note.typ");
        std::fs::write(&source, "original source").unwrap();
        let data = directory.path().join("core");
        let service = LocalService::bind(LocalServiceOptions::new(data.clone()))
            .await
            .unwrap();
        let state = service.authority();
        let address = url::Url::parse(service.browser_url()).unwrap();
        let token = address
            .fragment()
            .unwrap()
            .strip_prefix("token=")
            .unwrap()
            .to_owned();
        let base = format!("http://127.0.0.1:{}", address.port().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let host = tokio::spawn(service.serve_until(async {
            let _ = stopped.await;
        }));
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{base}/api/open"))
            .header("x-weftext-token", &token)
            .json(&serde_json::json!({"path":source,"root":project}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let opened: serde_json::Value = response.json().await.unwrap();
        let id = opened["sessionId"].as_str().unwrap();
        let response = client
            .post(format!("{base}/api/draft"))
            .header("x-weftext-token", &token)
            .json(&serde_json::json!({"sessionId":id,"source":"retained author input","inputRevision":1}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let drafts = || {
            std::fs::read_dir(data.join("drafts"))
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    (entry.file_name(), std::fs::read(entry.path()).unwrap())
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let original = drafts();
        assert!(!original.is_empty());
        let request = serde_json::json!({"sessionId":id});
        for credential in [None, Some("wrong")] {
            let mut request = client
                .post(format!("{base}/api/session/release"))
                .json(&request);
            if let Some(credential) = credential {
                request = request.header("x-weftext-token", credential);
            }
            assert_eq!(
                request.send().await.unwrap().status(),
                reqwest::StatusCode::FORBIDDEN
            );
        }
        assert!(state.locations.lock().unwrap().contains_key(id));
        let mut foreign = weftext_core::Core::new(directory.path().join("other-core")).unwrap();
        let other = foreign.open(&source, Some(project.clone())).unwrap();
        let response = client
            .post(format!("{base}/api/session/release"))
            .header("x-weftext-token", &token)
            .json(&serde_json::json!({"sessionId":other.session_id}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
        foreign
            .persist_draft(&other.session_id, "foreign owner remains open")
            .unwrap();
        let response = client
            .post(format!("{base}/api/session/release"))
            .header("x-weftext-token", &token)
            .json(&serde_json::json!({"sessionId":id,"proof":true}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        let response = client
            .post(format!("{base}/api/session/release"))
            .header("x-weftext-token", &token)
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(!state.locations.lock().unwrap().contains_key(id));
        assert!(!state.renders.lock().unwrap().contains_key(id));
        assert!(!state.editor_versions.lock().unwrap().contains_key(id));
        assert_eq!(drafts(), original);
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "original source");
        let response = client
            .post(format!("{base}/api/session/release"))
            .header("x-weftext-token", &token)
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            reqwest::StatusCode::CONFLICT,
            "missing never proves which owner released a session"
        );
        assert_eq!(drafts(), original);
        drop(client);
        drop(state);
        drop(foreign);
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), host)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
