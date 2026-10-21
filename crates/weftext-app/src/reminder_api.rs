//! Local owner's reminder intent/settings/status UI. External notification
//! submission and OS-history confirmation have no HTTP entry point.
use crate::*;
use std::path::Path;
use weftext_knowledge::NodeRef;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/reminders/settings", post(settings))
        .route("/api/reminders/inspect", post(inspect))
        .route("/api/reminders/library", post(library))
        .route("/api/reminders/subscription", post(subscription))
        .route("/api/reminders/status", post(status))
        .route("/api/reminders/records", post(records))
        .route("/api/reminders/schedule", post(schedule))
        .route("/api/reminders/open", post(open))
        .route("/api/reminders/handle", post(handle))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionRequest {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum LibraryRequest {
    Enable {
        root: String,
        expected_revision: u64,
        mutation_id: String,
    },
    Disable {
        workspace_id: String,
        expected_revision: u64,
        mutation_id: String,
    },
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubscriptionRequest {
    root: String,
    owner: NodeRef,
    reminder_id: String,
    enabled: bool,
    expected_revision: u64,
    mutation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KeyRequest {
    execution_key: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageRequest {
    after: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScheduleRequest {
    cursor: Option<weftext_core::reminder_device_state::ReminderScheduleCursor>,
}
async fn settings(State(state): State<AppState>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, |core| core.reminder_settings()).await?,
    )?))
}
async fn inspect(State(state): State<AppState>, Json(request): Json<SessionRequest>) -> ApiResult {
    let result=core_call(&state,move|core|{
        let candidate=core.task_reminder_for_session(&request.session_id)?;
        let settings=core.reminder_settings()?;
        let delivery=core.reminder_delivery_for_saved_task(&candidate)?;
        let (eligibility,issue)=match core.reminder_eligibility(&candidate){Ok(value)=>(Some(value),None),Err(error)=>(None,Some(error))};
        Ok(json!({"candidate":candidate,"settings":settings,"eligibility":eligibility,"delivery":delivery,"issue":issue}))
    }).await?;
    Ok(Json(result))
}
async fn library(State(state): State<AppState>, Json(request): Json<LibraryRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| match request {
            LibraryRequest::Enable {
                root,
                expected_revision,
                mutation_id,
            } => core.set_reminder_library(Path::new(&root), true, expected_revision, &mutation_id),
            LibraryRequest::Disable {
                workspace_id,
                expected_revision,
                mutation_id,
            } => core.disable_reminder_library(&workspace_id, expected_revision, &mutation_id),
        })
        .await?,
    )?))
}
async fn subscription(
    State(state): State<AppState>,
    Json(request): Json<SubscriptionRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.set_reminder_subscription(
                Path::new(&request.root),
                &request.owner,
                &request.reminder_id,
                request.enabled,
                request.expected_revision,
                &request.mutation_id,
            )
        })
        .await?,
    )?))
}
async fn status(State(state): State<AppState>, Json(request): Json<KeyRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.reminder_delivery_status(&request.execution_key)
        })
        .await?,
    )?))
}
async fn records(State(state): State<AppState>, Json(request): Json<PageRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.reminder_records_page(request.after.as_deref())
        })
        .await?,
    )?))
}
async fn schedule(
    State(state): State<AppState>,
    Json(request): Json<ScheduleRequest>,
) -> ApiResult {
    Ok(Json(core_call(&state,move|core|{
        let page=core.reminder_schedule_page(request.cursor.as_ref())?;
        let entries=page.candidates.into_iter().map(|candidate|{let(eligibility,issue)=match core.reminder_eligibility(&candidate){Ok(eligibility)=>(Some(eligibility),None),Err(error)=>(None,Some(error))};json!({"candidate":candidate,"eligibility":eligibility,"issue":issue})}).collect::<Vec<_>>();
        Ok(json!({"entries":entries,"issues":page.issues,"complete":page.complete,"next":page.next}))
    }).await?))
}
async fn open(State(state): State<AppState>, Json(request): Json<KeyRequest>) -> ApiResult {
    let document = value(
        core_call(&state, move |core| {
            core.open_reminder_record(&request.execution_key)
        })
        .await?,
    )?;
    remember(&state, &document)?;
    Ok(Json(document))
}
async fn handle(State(state): State<AppState>, Json(request): Json<KeyRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.handle_reminder(&request.execution_key)
        })
        .await?,
    )?))
}

#[cfg(all(test, windows))]
mod tests {
    use crate::local_service::{LocalService, LocalServiceOptions};

    #[tokio::test]
    async fn reminder_http_credentials_precede_device_identity_allocation() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("state");
        let service = LocalService::bind(LocalServiceOptions::new(data.clone()))
            .await
            .unwrap();
        let address = url::Url::parse(service.browser_url()).unwrap();
        let token = address.fragment().unwrap().strip_prefix("token=").unwrap();
        let endpoint = format!(
            "http://127.0.0.1:{}/api/reminders/settings",
            address.port().unwrap()
        );
        let identity = data.join("device-state/identity.json");
        let reminders = data.join("device-state/reminders.json");
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let host = tokio::spawn(service.serve_until(async {
            let _ = stopped.await;
        }));
        let client = reqwest::Client::new();
        for (credential, origin) in [
            (None, None),
            (Some("wrong"), None),
            (Some(token), Some("https://example.invalid")),
        ] {
            let mut request = client.post(&endpoint).json(&serde_json::json!({}));
            if let Some(credential) = credential {
                request = request.header("x-weftext-token", credential);
            }
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            let response = request.send().await.unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
            assert!(!identity.exists());
            assert!(!reminders.exists());
        }
        let response = client
            .post(&endpoint)
            .header("x-weftext-token", token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let settings: serde_json::Value = response.json().await.unwrap();
        assert!(
            settings["deviceId"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
        );
        assert!(identity.is_file());
        assert!(!reminders.exists());
        drop(client);
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), host)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
