//! Device-local capture recovery. These routes store author input, not effects.
//! Hosted consumers must use authenticated actor/library wrappers, not this API.
use super::*;
use weftext_core::{
    CaptureAuthorInput, CaptureInput, CaptureMutationCommand, CaptureMutationTarget,
    FrozenCaptureRequest, MAX_CAPTURE_RECORD_BYTES,
};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/capture/identity", post(identity))
        .route("/api/capture/draft/create", post(create))
        .route("/api/capture/draft/read", post(read))
        .route("/api/capture/draft/list", post(list))
        .route("/api/capture/draft/storage", post(storage))
        .route("/api/capture/draft/input", post(input))
        .route("/api/capture/draft/input-mutation", post(input_mutation))
        .route("/api/capture/draft/author-input", post(author_input))
        .route("/api/capture/draft/author-mutation", post(author_mutation))
        .route("/api/capture/draft/checkpoint", post(checkpoint))
        .route("/api/capture/draft/target", post(target))
        .route("/api/capture/draft/resolve", post(resolve))
        .route("/api/capture/draft/freeze", post(freeze))
        .route("/api/capture/draft/advance", post(advance))
        .route("/api/capture/draft/submit", post(submit))
        .route("/api/capture/draft/open-result", post(open_result))
        .route("/api/capture/draft/mutation", post(mutation))
        .route("/api/capture/draft/flush", post(flush))
        .layer(DefaultBodyLimit::max(MAX_CAPTURE_RECORD_BYTES + 16 * 1024))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TargetSelection {
    root: String,
    parent: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Create {
    draft_id: String,
    mutation_id: String,
    input: CaptureInput,
    target: Option<TargetSelection>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Read {
    draft_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenResult {
    draft_id: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    draft_id: String,
    expected_revision: u64,
    mutation_id: String,
    input: CaptureInput,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InputMutation {
    draft_id: String,
    expected_revision: u64,
    mutation_id: String,
    fingerprint: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthorInput {
    draft_id: String,
    expected_revision: u64,
    mutation_id: String,
    input: CaptureAuthorInput,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Checkpoint {
    draft_id: String,
    expected_revision: u64,
    mutation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    draft_id: String,
    expected_revision: u64,
    mutation_id: String,
    target: TargetSelection,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Freeze {
    draft_id: String,
    expected_revision: u64,
    mutation_id: String,
    request: FrozenCaptureRequest,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Advance {
    draft_id: String,
    expected_revision: u64,
    mutation_id: String,
    operation_id: String,
    reset_input: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Mutation {
    draft_id: String,
    mutation_id: String,
    command: MutationCommand,
}
// The public local proof has the same allowed author fields as each write route.
// Hosted slots and actors are always selected by their trusted wrapper instead.
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum MutationCommand {
    Checkpoint {
        expected_revision: u64,
    },
    Create {
        input: CaptureInput,
        target: Option<TargetSelection>,
    },
    Input {
        expected_revision: u64,
        input: CaptureInput,
    },
    Target {
        expected_revision: u64,
        target: TargetSelection,
    },
    Freeze {
        expected_revision: u64,
        request: FrozenCaptureRequest,
    },
    Advance {
        expected_revision: u64,
        operation_id: String,
        reset_input: bool,
    },
}
impl From<TargetSelection> for CaptureMutationTarget {
    fn from(target: TargetSelection) -> Self {
        Self {
            root: target.root,
            parent: target.parent,
            hosted_slot: None,
        }
    }
}
impl From<MutationCommand> for CaptureMutationCommand {
    fn from(command: MutationCommand) -> Self {
        match command {
            MutationCommand::Checkpoint { expected_revision } => {
                Self::Checkpoint { expected_revision }
            }
            MutationCommand::Create { input, target } => Self::Create {
                input,
                target: target.map(Into::into),
            },
            MutationCommand::Input {
                expected_revision,
                input,
            } => Self::Input {
                expected_revision,
                input,
            },
            MutationCommand::Target {
                expected_revision,
                target,
            } => Self::Target {
                expected_revision,
                target: target.into(),
            },
            MutationCommand::Freeze {
                expected_revision,
                request,
            } => Self::Freeze {
                expected_revision,
                request,
            },
            MutationCommand::Advance {
                expected_revision,
                operation_id,
                reset_input,
            } => Self::Advance {
                expected_revision,
                operation_id,
                reset_input,
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Flush {
    draft_id: String,
    record_revision: u64,
    input_revision: u64,
    last_mutation_id: String,
}

async fn create(State(state): State<AppState>, RequestJson(req): RequestJson<Create>) -> ApiResult {
    let draft = core_call(&state, move |core| match req.target {
        Some(target) => core.create_targeted_capture_draft(
            &req.draft_id,
            &req.mutation_id,
            req.input,
            &PathBuf::from(target.root),
            &PathBuf::from(target.parent),
            None,
        ),
        None => core.create_capture_draft(&req.draft_id, &req.mutation_id, req.input),
    })
    .await?;
    Ok(Json(value(draft)?))
}
async fn identity(State(state): State<AppState>, Json(_req): Json<Empty>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            let identity = core.device_identity()?;
            if !identity.is_current {
                return Err(weftext_core::CoreError::new(
                    "deviceIdentityUnavailable",
                    weftext_messages::UiMessage::error_api_capture_api_the_retained_device_identity_is_not_qualified_for_this_core_profile(),
                ));
            }
            Ok(identity)
        })
        .await?,
    )?))
}
async fn author_input(State(state): State<AppState>, Json(req): Json<AuthorInput>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.update_capture_author_input(
                &req.draft_id,
                req.expected_revision,
                &req.mutation_id,
                req.input,
            )?;
            core.read_capture_draft(&req.draft_id)
        })
        .await?,
    )?))
}
async fn input_mutation(
    State(state): State<AppState>,
    Json(req): Json<InputMutation>,
) -> ApiResult {
    Ok(Json(
        core_call(&state, move |core| {
            let (status, _) = core.capture_input_mutation_status(
                &req.draft_id,
                &req.mutation_id,
                req.expected_revision,
                &req.fingerprint,
            )?;
            Ok(json!({"status":status,"draft":core.read_capture_draft(&req.draft_id)?}))
        })
        .await?,
    ))
}
async fn author_mutation(State(state): State<AppState>, Json(req): Json<AuthorInput>) -> ApiResult {
    Ok(Json(
        core_call(&state, move |core| {
            let (status, _) = core.capture_author_mutation_status(
                &req.draft_id,
                &req.mutation_id,
                req.expected_revision,
                &req.input,
            )?;
            Ok(json!({"status":status,"draft":core.read_capture_draft(&req.draft_id)?}))
        })
        .await?,
    ))
}
async fn checkpoint(State(state): State<AppState>, Json(req): Json<Checkpoint>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.checkpoint_capture_input(&req.draft_id, req.expected_revision, &req.mutation_id)?;
            core.read_capture_draft(&req.draft_id)
        })
        .await?,
    )?))
}
async fn read(State(state): State<AppState>, Json(req): Json<Read>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.read_capture_draft(&req.draft_id)).await?,
    )?))
}
async fn storage(
    State(state): State<AppState>,
    RequestJson(_req): RequestJson<Empty>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.capture_storage_status()).await?,
    )?))
}
async fn resolve(State(state): State<AppState>, RequestJson(req): RequestJson<Read>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.resolve_capture_target(&req.draft_id)
        })
        .await?,
    )?))
}
async fn list(State(state): State<AppState>, RequestJson(_req): RequestJson<Empty>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.list_capture_drafts()).await?,
    )?))
}
async fn input(State(state): State<AppState>, RequestJson(req): RequestJson<Input>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.update_capture_input(
                &req.draft_id,
                req.expected_revision,
                &req.mutation_id,
                req.input,
            )
        })
        .await?,
    )?))
}
async fn target(State(state): State<AppState>, RequestJson(req): RequestJson<Target>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.select_capture_target(
                &req.draft_id,
                req.expected_revision,
                &req.mutation_id,
                &PathBuf::from(req.target.root),
                &PathBuf::from(req.target.parent),
                None,
            )
        })
        .await?,
    )?))
}
async fn freeze(State(state): State<AppState>, RequestJson(req): RequestJson<Freeze>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.freeze_capture_submission(
                &req.draft_id,
                req.expected_revision,
                &req.mutation_id,
                req.request,
            )
        })
        .await?,
    )?))
}
async fn mutation(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<Mutation>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.capture_mutation_status(&req.draft_id, &req.mutation_id, &req.command.into())
        })
        .await?,
    )?))
}
async fn advance(State(state): State<AppState>, Json(req): Json<Advance>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.advance_capture_submission(
                &req.draft_id,
                req.expected_revision,
                &req.mutation_id,
                &req.operation_id,
                req.reset_input,
            )
        })
        .await?,
    )?))
}
async fn submit(State(state): State<AppState>, Json(req): Json<Read>) -> ApiResult {
    let (_preview, receipt) = core_call(&state, move |core| {
        let draft = core.read_capture_draft(&req.draft_id)?;
        if draft.input_cut!=weftext_core::CaptureInputCut::Full {return Err(CoreError::new("captureInputIncomplete",weftext_messages::UiMessage::error_api_capture_api_only_an_author_recovery_checkpoint_is_durable_query_the_original_operation()));}
        let original = draft.pending.as_ref().ok_or_else(|| {
            CoreError::new(
                "captureRequestMissing",
                weftext_messages::UiMessage::error_api_capture_api_no_durable_original_capture_request(),
            )
        })?;
        if original.request_kind != "capture" {
            return Err(CoreError::new(
                "captureRequestInvalid",
                weftext_messages::UiMessage::error_api_capture_api_this_route_only_dispatches_the_original_local_capture_request(),
            ));
        }
        // A committed/prepared original is recovered before current-path
        // interpretation. A fresh attempt must still name the same admitted
        // target container that was frozen, even if its old path was replaced.
        if core.local_operation_status(&original.operation_id)?["status"] == "unknown" {
            let actual = core.resolve_capture_target(&req.draft_id)?;
            let target = draft.target.as_ref().ok_or_else(|| CoreError::new("captureTargetMissing", weftext_messages::UiMessage::error_core_capture_draft_capture_target_is_unavailable()))?;
            let parent = match &target.parent {
                weftext_core::CaptureParent::LibraryRoot => &target.root_hint,
                weftext_core::CaptureParent::Managed { path_hint, .. } => path_hint,
            };
            if actual.root != target.root_hint || actual.parent != *parent {
                return Err(CoreError::new("captureTargetChanged", weftext_messages::UiMessage::error_api_capture_api_original_target_moved_after_freezing_retain_and_settle_this_original_attempt()));
            }
        }
        crate::data_api::capture_frozen_wire(core, &original.wire)
    })
    .await?;
    // Capture effects and document delivery are separate actions. Save-next
    // does not create an unused Session, and delivery cannot change this fact.
    Ok(Json(value(receipt)?))
}
async fn open_result(State(state): State<AppState>, Json(req): Json<OpenResult>) -> ApiResult {
    let document = value(
        core_call(&state, move |core| {
            core.open_capture_result(&req.draft_id, &req.operation_id)
        })
        .await?,
    )?;
    remember(&state, &document)?;
    Ok(Json(document))
}
async fn flush(State(state): State<AppState>, Json(req): Json<Flush>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.capture_flush_receipt(
                &req.draft_id,
                req.record_revision,
                req.input_revision,
                &req.last_mutation_id,
            )
        })
        .await?,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_utf16_transport_is_explicit_and_cannot_select_an_actor_or_private_path() {
        let mut input = serde_json::to_value(CaptureInput::default()).unwrap();
        input["body"] = json!({"utf16":[55296,0,56320,55357,56832]});
        let request = json!({"draftId":"00000000-0000-4000-8000-000000000001","mutationId":"00000000-0000-4000-8000-000000000002","input":input,"target":null});
        let decoded: Create = serde_json::from_value(request.clone()).unwrap();
        assert_eq!(
            decoded.input.body.utf16,
            [0xd800, 0, 0xdc00, 0xd83d, 0xde00]
        );
        let mut forged = request;
        forged["actor"] = json!("another-user");
        assert!(serde_json::from_value::<Create>(forged).is_err());
    }

    #[cfg(windows)]
    struct Host {
        base: String,
        token: String,
        client: reqwest::Client,
        stop: tokio::sync::oneshot::Sender<()>,
        task: tokio::task::JoinHandle<std::io::Result<()>>,
    }
    #[cfg(windows)]
    impl Host {
        async fn start(data: PathBuf) -> Self {
            let service = crate::local_service::LocalService::bind(
                crate::local_service::LocalServiceOptions::new(data),
            )
            .await
            .unwrap();
            let url = url::Url::parse(service.browser_url()).unwrap();
            let token = url
                .fragment()
                .unwrap()
                .strip_prefix("token=")
                .unwrap()
                .to_owned();
            let base = format!("http://127.0.0.1:{}", url.port().unwrap());
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(service.serve_until(async {
                let _ = stopped.await;
            }));
            Self {
                base,
                token,
                client: reqwest::Client::new(),
                stop,
                task,
            }
        }
        async fn request(&self, route: &str, body: &Value) -> reqwest::Response {
            self.client
                .post(format!("{}/api/capture/draft/{route}", self.base))
                .header("x-weftext-token", &self.token)
                .json(body)
                .send()
                .await
                .unwrap()
        }
        async fn json(&self, route: &str, body: &Value) -> Value {
            let response = self.request(route, body).await;
            assert!(
                response.status().is_success(),
                "{route}: {}",
                response.text().await.unwrap()
            );
            response.json().await.unwrap()
        }
        async fn close(self) {
            self.stop.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(5), self.task)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn real_http_restart_recovers_exact_input_and_pending_by_original_command() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("state");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        drop(core);
        let id = uuid::Uuid::new_v4().to_string();
        let create_id = uuid::Uuid::new_v4().to_string();
        let update_id = uuid::Uuid::new_v4().to_string();
        let freeze_id = uuid::Uuid::new_v4().to_string();
        let operation_id = uuid::Uuid::new_v4().to_string();
        let host = Host::start(data.clone()).await;
        let mut raw = serde_json::to_value(CaptureInput::default()).unwrap();
        raw["body"] = json!({"utf16":[55296,0,56320,55357,56832]});
        let create = json!({"draftId":id,"mutationId":create_id,"input":raw,"target":{"root":root,"parent":root}});
        let denied = host
            .client
            .post(format!("{}/api/capture/draft/create", host.base))
            .json(&create)
            .send()
            .await
            .unwrap();
        assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
        let created = host.json("create", &create).await;
        let mut updated_raw = raw.clone();
        updated_raw["title"] = json!({"utf16":[55296,65]});
        let update = json!({"draftId":id,"mutationId":update_id,"expectedRevision":created["revision"],"input":updated_raw});
        let reply = host.request("input", &update).await;
        assert!(reply.status().is_success());
        // Controlled lost delivery: the HTTP handler completed; its body is not
        // installed by a consumer before the service/Core is closed and restarted.
        drop(reply);
        host.close().await;

        let host = Host::start(data.clone()).await;
        let query = json!({"draftId":id,"mutationId":update_id,"command":{"kind":"input","expectedRevision":created["revision"],"input":updated_raw}});
        let recovered = host.json("mutation", &query).await;
        assert_eq!(recovered["status"], "applied");
        assert_eq!(recovered["draft"]["currentInput"], updated_raw);
        let record = data.join("capture-drafts").join(format!("{id}.json"));
        let original = std::fs::read(&record).unwrap();
        let mut wrong = query.clone();
        wrong["command"]["expectedRevision"] = json!(99);
        assert_eq!(
            host.json("mutation", &wrong).await["status"],
            "differentCommand"
        );
        let mut reused = update;
        reused["input"]["body"] = json!({"utf16":[66]});
        assert!(!host.request("input", &reused).await.status().is_success());
        assert_eq!(std::fs::read(&record).unwrap(), original);

        let root = recovered["draft"]["target"]["rootHint"].as_str().unwrap();
        let wire = format!(
            "{{\"operationId\":{},\"root\":{},\"parent\":{},\"body\":\"unfinished\\ud800\"}}",
            serde_json::to_string(&operation_id).unwrap(),
            serde_json::to_string(root).unwrap(),
            serde_json::to_string(root).unwrap()
        );
        use sha2::{Digest, Sha256};
        let request = json!({"operationId":operation_id,"requestKind":"capture","wireDigest":format!("{:x}", Sha256::digest(wire.as_bytes())),"wire":wire,"inputRevision":recovered["draft"]["inputRevision"]});
        let freeze = json!({"draftId":id,"mutationId":freeze_id,"expectedRevision":recovered["draft"]["revision"],"request":request});
        let frozen = host.json("freeze", &freeze).await;
        host.close().await;
        let host = Host::start(data).await;
        let pending = host.json("mutation", &json!({"draftId":id,"mutationId":freeze_id,"command":{"kind":"freeze","expectedRevision":recovered["draft"]["revision"],"request":request}})).await;
        assert_eq!(pending["status"], "applied");
        assert_eq!(pending["draft"]["pending"]["wire"], wire);
        let ack = host.json("flush", &json!({"draftId":id,"recordRevision":frozen["revision"],"inputRevision":frozen["inputRevision"],"lastMutationId":freeze_id})).await;
        assert_eq!(ack["pendingOperationId"], operation_id);
        let mut forged = create;
        forged["actor"] = json!("other-author");
        assert!(!host.request("create", &forged).await.status().is_success());
        let before_budget = std::fs::read(&record).unwrap();
        for malformed in [
            json!({"kind":"text","value":"own","schema":{}}),
            json!({"kind":"record","value":[{"key":"a","value":{"kind":"none"},"extra":true}]}),
        ] {
            let mut bad = updated_raw.clone();
            bad["fields"] =
                json!([{"fieldId":"field","valueState":{"kind":"valid","value":malformed}}]);
            let response=host.request("input",&json!({"draftId":id,"mutationId":uuid::Uuid::new_v4().to_string(),"expectedRevision":frozen["revision"],"input":bad})).await;
            assert!(!response.status().is_success());
            assert_eq!(std::fs::read(&record).unwrap(), before_budget);
        }
        let mut oversized = serde_json::to_vec(&json!({"draftId":id,"mutationId":uuid::Uuid::new_v4().to_string(),"expectedRevision":frozen["revision"],"input":updated_raw})).unwrap();
        oversized.resize(MAX_CAPTURE_RECORD_BYTES + 16 * 1024 + 1, b' ');
        let denied = host
            .client
            .post(format!("{}/api/capture/draft/input", host.base))
            .header("x-weftext-token", &host.token)
            .header("content-type", "application/json")
            .body(oversized)
            .send()
            .await
            .unwrap();
        assert_eq!(denied.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(std::fs::read(&record).unwrap(), before_budget);
        host.close().await;
    }
}
