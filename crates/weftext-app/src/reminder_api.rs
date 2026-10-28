//! Local owner's reminder intent/settings/status UI. External notification
//! submission and OS-history confirmation have no HTTP entry point.
use crate::*;
use std::path::Path;
use weftext_core::reminder_device_state::{
    ReminderDeliveryStatus, ReminderEligibility, ReminderScheduleCursor, ReminderScheduleIssue,
    ReminderSettings, TaskReminderCandidate,
};
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
        .route("/api/reminders/activation", post(activation))
        .route("/api/reminders/activation/open", post(open_activation))
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
struct ActivationRequest {
    activation_key: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenActivationRequest {
    activation_key: String,
    owner: NodeRef,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageRequest {
    after: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScheduleRequest {
    cursor: Option<ReminderScheduleCursor>,
}
/// The local owner's response retains the actual Core failure and all of its
/// typed details. No language rendering participates in reminder identity,
/// settings mutation fingerprints or external-delivery state.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Inspection {
    candidate: TaskReminderCandidate,
    settings: ReminderSettings,
    eligibility: Option<ReminderEligibility>,
    delivery: Option<ReminderDeliveryStatus>,
    issue: Option<CoreError>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleEntry {
    candidate: TaskReminderCandidate,
    eligibility: Option<ReminderEligibility>,
    issue: Option<CoreError>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SchedulePage {
    entries: Vec<ScheduleEntry>,
    issues: Vec<ReminderScheduleIssue>,
    complete: bool,
    next: Option<ReminderScheduleCursor>,
}
async fn settings(State(state): State<AppState>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, |core| core.reminder_settings()).await?,
    )?))
}
async fn inspect(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<SessionRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        let candidate = core.task_reminder_for_session(&request.session_id)?;
        let settings = core.reminder_settings()?;
        let delivery = core.reminder_delivery_for_saved_task(&candidate)?;
        let (eligibility, issue) = match core.reminder_eligibility(&candidate) {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error)),
        };
        Ok(Inspection {
            candidate,
            settings,
            eligibility,
            delivery,
            issue,
        })
    })
    .await?;
    Ok(Json(value(result)?))
}
async fn library(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<LibraryRequest>,
) -> ApiResult {
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
    RequestJson(request): RequestJson<SubscriptionRequest>,
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
async fn status(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<KeyRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.reminder_delivery_status(&request.execution_key)
        })
        .await?,
    )?))
}
async fn records(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<PageRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.reminder_records_page(request.after.as_deref())
        })
        .await?,
    )?))
}
async fn schedule(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<ScheduleRequest>,
) -> ApiResult {
    let page = core_call(&state, move |core| {
        let page = core.reminder_schedule_page(request.cursor.as_ref())?;
        let entries = page
            .candidates
            .into_iter()
            .map(|candidate| {
                let (eligibility, issue) = match core.reminder_eligibility(&candidate) {
                    Ok(eligibility) => (Some(eligibility), None),
                    Err(error) => (None, Some(error)),
                };
                ScheduleEntry {
                    candidate,
                    eligibility,
                    issue,
                }
            })
            .collect();
        Ok(SchedulePage {
            entries,
            issues: page.issues,
            complete: page.complete,
            next: page.next,
        })
    })
    .await?;
    Ok(Json(value(page)?))
}
async fn open(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<KeyRequest>,
) -> ApiResult {
    let document = value(
        core_call(&state, move |core| {
            core.open_reminder_record(&request.execution_key)
        })
        .await?,
    )?;
    remember(&state, &document)?;
    Ok(Json(document))
}
async fn activation(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<ActivationRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.resolve_reminder_activation(&request.activation_key)
        })
        .await?,
    )?))
}

/// The original opaque notification selects the permitted members. A browser
/// never supplies a path, replacement effect or cached authorization to open.
fn open_activation_member(
    core: &mut weftext_core::Core,
    activation_key: &str,
    owner: &NodeRef,
) -> weftext_core::Result<weftext_core::OpenedDocument> {
    let unavailable = || {
        CoreError::new(
            "reminderUnavailable",
            weftext_messages::UiMessage::error_reminder_activation_unavailable(),
        )
    };
    let members = core.resolve_reminder_activation(activation_key)?;
    let member = members
        .candidates
        .iter()
        .find(|candidate| &candidate.owner == owner)
        .ok_or_else(unavailable)?;
    let record = core
        .reminder_delivery_for_saved_task(member)?
        .ok_or_else(unavailable)?;
    let opened = core.open_reminder_record(&record.execution_key)?;
    let qualified = (|| {
        let current = core.resolve_reminder_activation(activation_key)?;
        let member = current
            .candidates
            .iter()
            .find(|candidate| &candidate.owner == owner)
            .ok_or_else(unavailable)?;
        let current = core
            .reminder_delivery_for_saved_task(member)?
            .ok_or_else(unavailable)?;
        if current.execution_key != record.execution_key {
            return Err(unavailable());
        }
        Ok(())
    })();
    if let Err(error) = qualified {
        core.close_session(&opened.session_id)?;
        return Err(error);
    }
    Ok(opened)
}
async fn open_activation(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<OpenActivationRequest>,
) -> ApiResult {
    let document = value(
        core_call(&state, move |core| {
            open_activation_member(core, &request.activation_key, &request.owner)
        })
        .await?,
    )?;
    remember(&state, &document)?;
    Ok(Json(document))
}
async fn handle(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<KeyRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.handle_reminder(&request.execution_key)
        })
        .await?,
    )?))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::local_service::{LocalService, LocalServiceOptions};

    #[test]
    fn notification_open_is_bound_to_the_original_current_member_and_disabled_library_is_rejected()
    {
        use weftext_core::reminder_device_state::{
            ReminderNotificationOptions, ReminderSummaryPreparation,
        };
        use weftext_core::{Core, OperationIntent, OperationRequest};
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("library");
        let mut core = Core::new(directory.path().join("profile")).unwrap();
        let library = core.create_library(&root).unwrap();
        let mut owners = Vec::new();
        let mut revision = core
            .set_reminder_library(&root, true, 0, &uuid::Uuid::new_v4().to_string())
            .unwrap()
            .settings_revision;
        for name in ["First", "Second"] {
            let intent = uuid::Uuid::new_v4().to_string();
            let operation_id = uuid::Uuid::new_v4().to_string();
            let preview = core.prepare_operation(OperationRequest {
                operation_id: operation_id.clone(), root: root.to_string_lossy().into_owned(),
                intent: OperationIntent::NewNode { parent: root.to_string_lossy().into_owned(), name: name.into(), source: Some(format!("#metadata((types: (\"task\",), status: \"open\", reminder: (id: \"{intent}\", kind: \"absolute\", at: \"2000-01-01T09:00:00Z\"))) <weftext>\n= {name}\n原任务")) },
            }).unwrap();
            let owner = NodeRef {
                workspace_id: library.workspace_id.clone(),
                node_id: preview.new_node_id.clone().unwrap(),
            };
            core.execute_operation(&operation_id, &preview.plan_id)
                .unwrap();
            revision = core
                .set_reminder_subscription(
                    &root,
                    &owner,
                    &intent,
                    true,
                    revision,
                    &uuid::Uuid::new_v4().to_string(),
                )
                .unwrap()
                .settings_revision;
            owners.push(owner);
        }
        // A real durable original summary, without any simulated OS acceptance.
        let ReminderSummaryPreparation::Started { permit, .. } = core
            .prepare_reminder_summary(ReminderNotificationOptions {
                app_id: "Weftext.Activation.Tests".into(),
                title: "Reminder".into(),
                body: "Open tasks".into(),
            })
            .unwrap()
        else {
            panic!("expected original missed-summary permit");
        };
        let activation = permit.request().activation_key.clone();
        let other = NodeRef {
            workspace_id: library.workspace_id.clone(),
            node_id: uuid::Uuid::new_v4().to_string(),
        };
        assert!(open_activation_member(&mut core, &activation, &other).is_err());
        assert!(
            open_activation_member(&mut core, &uuid::Uuid::new_v4().to_string(), &owners[0])
                .is_err()
        );
        for owner in &owners {
            let opened = open_activation_member(&mut core, &activation, owner).unwrap();
            assert_eq!(opened.node_id.as_deref(), Some(owner.node_id.as_str()));
            assert_eq!(
                opened.workspace_id.as_deref(),
                Some(owner.workspace_id.as_str())
            );
            core.close_session(&opened.session_id).unwrap();
        }
        let records = core.reminder_records_page(None).unwrap().records;
        core.handle_reminder(&records[0].status.execution_key)
            .unwrap();
        assert!(open_activation_member(&mut core, &activation, &records[0].owner).is_err());
        core.disable_reminder_library(
            &library.workspace_id,
            revision,
            &uuid::Uuid::new_v4().to_string(),
        )
        .unwrap();
        for owner in &owners {
            assert!(open_activation_member(&mut core, &activation, owner).is_err());
        }
        assert_eq!(core.reminder_records_page(None).unwrap().records.len(), 2);
    }

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
        let original_identity = std::fs::read(&identity).unwrap();
        let failure = client.post(endpoint.replace("/settings", "/library"))
            .header("x-weftext-token", token)
            .json(&serde_json::json!({"action":"disable","workspaceId":uuid::Uuid::new_v4(),"expectedRevision":0,"mutationId":uuid::Uuid::new_v4()}))
            .send().await.unwrap();
        assert_eq!(failure.status(), reqwest::StatusCode::CONFLICT);
        let failure: Value = failure.json().await.unwrap();
        assert_eq!(failure["error"]["code"], "reminderState");
        assert!(failure["error"].get("message").is_none());
        let _:ErrorDescription=serde_json::from_value(serde_json::json!({"explanation":failure["error"]["explanation"],"diagnostics":failure["error"]["diagnostics"],"causes":failure["error"]["causes"]})).unwrap();
        assert_eq!(failure["error"]["details"]["kind"], "core");
        assert_eq!(failure["error"]["details"]["code"], "reminderState");
        assert_eq!(std::fs::read(&identity).unwrap(), original_identity);
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

#[cfg(test)]
mod typed_contract_tests {
    use super::*;
    use weftext_core::reminder_device_state::{ReminderDeliveryState, ReminderNotificationRequest};
    use weftext_core::{CoreDescription, CoreErrorDetails, ErrorDetailGroup};

    #[test]
    fn schedule_issue_keeps_original_owner_error_and_all_parallel_details() {
        let reminder_id = uuid::Uuid::new_v4().to_string();
        let description = ErrorDescription::new(UiMessage::common_operation(
            weftext_messages::params::CommonOperation {
                operation_id: "original-reminder".into(),
            },
        ));
        let mut error = CoreError::from_product(
            "reminderState",
            Some("originalFailure".into()),
            description.clone(),
            vec![description; 48],
        );
        error.current_source = Some("Exact local owner draft 😀\r\n".into());
        let original = serde_json::to_value(&error).unwrap();
        let owner = NodeRef {
            workspace_id: uuid::Uuid::new_v4().to_string(),
            node_id: uuid::Uuid::new_v4().to_string(),
        };
        let page = SchedulePage {
            entries: vec![],
            issues: vec![ReminderScheduleIssue {
                owner: owner.clone(),
                reminder_id: reminder_id.clone(),
                error,
            }],
            complete: false,
            next: None,
        };
        let wire = value(page).unwrap();
        assert_eq!(
            wire["issues"][0]["owner"],
            serde_json::to_value(owner).unwrap()
        );
        assert_eq!(wire["issues"][0]["reminderId"], reminder_id);
        assert!(wire["issues"][0].get("message").is_none());
        for key in [
            "code",
            "explanation",
            "diagnostics",
            "causes",
            "details",
            "currentSource",
        ] {
            assert_eq!(wire["issues"][0][key], original[key]);
        }
        let groups = wire["issues"][0]["details"]["groups"].as_array().unwrap();
        assert!(groups.iter().any(|group| {
            group["diagnostics"]
                .as_array()
                .is_some_and(|items| items.len() == 48)
        }));
        assert_eq!(wire["complete"], false);
        assert!(wire["next"].is_null());
    }

    #[test]
    fn unknown_delivery_keeps_typed_diagnostics_and_original_notification_text_independent_of_display()
     {
        let mut description = CoreDescription::new(UiMessage::common_operation(
            weftext_messages::params::CommonOperation {
                operation_id: "original-delivery".into(),
            },
        ));
        description.details = Some(CoreErrorDetails::Aggregate {
            groups: vec![ErrorDetailGroup::Product {
                code: Some("originalFailure".into()),
                diagnostics: vec![
                    ErrorDescription::new(UiMessage::common_operation(
                        weftext_messages::params::CommonOperation {
                            operation_id: "exact detail".into()
                        }
                    ));
                    48
                ],
            }],
        });
        let original = ReminderNotificationRequest {
            app_id: "Weftext.Test".into(),
            tag: "original-tag".into(),
            group: "original-group".into(),
            activation_key: uuid::Uuid::new_v4().to_string(),
            title: "原系统通知标题 😀".into(),
            body: "Exact original system notification body\r\n".into(),
        };
        let unknown = ReminderDeliveryState::Unknown {
            description: description.clone(),
        };
        let before = serde_json::to_value((&unknown, &original)).unwrap();
        assert_eq!(before[0]["state"], "unknown");
        assert!(before[0].get("diagnostic").is_none());
        assert_eq!(
            before[0]["description"]["details"]["groups"][0]["diagnostics"]
                .as_array()
                .unwrap()
                .len(),
            48
        );
        assert_ne!(
            description
                .explanation
                .render(weftext_messages::UiLocale::ZhCn),
            description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
        );
        assert_eq!(serde_json::to_value((&unknown, &original)).unwrap(), before);
        assert_eq!(before[1]["title"], "原系统通知标题 😀");
        assert_eq!(
            before[1]["body"],
            "Exact original system notification body\r\n"
        );
        let decoded: ReminderDeliveryState = serde_json::from_value(before[0].clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), before[0]);
    }
}
