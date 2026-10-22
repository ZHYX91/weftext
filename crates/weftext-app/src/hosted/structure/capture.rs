//! Capture recovery belongs to the authenticated actor. Effects and result
//! delivery reuse the original structure producer and its current grant checks.
use super::*;
use weftext_core::{
    CaptureAuthorInput, CaptureDraft, CaptureInput, CaptureMutationCommand, FrozenCaptureRequest,
    MAX_CAPTURE_RECORD_BYTES,
};
#[cfg(all(test, windows))]
#[path = "capture_tests.rs"]
mod tests;

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/capture/draft/create", post(create))
        .route("/api/hosted/capture/draft/list", post(list))
        .route("/api/hosted/capture/draft/read", post(read))
        .route("/api/hosted/capture/draft/input", post(input))
        .route(
            "/api/hosted/capture/draft/input-mutation",
            post(input_mutation),
        )
        .route("/api/hosted/capture/draft/author-input", post(author_input))
        .route("/api/hosted/capture/draft/checkpoint", post(checkpoint))
        .route(
            "/api/hosted/capture/draft/author-mutation",
            post(author_mutation),
        )
        .route("/api/hosted/capture/draft/mutation", post(mutation))
        .route("/api/hosted/capture/draft/freeze", post(freeze))
        .route("/api/hosted/capture/draft/advance", post(advance))
        .route("/api/hosted/capture/draft/submit", post(submit))
        .route("/api/hosted/capture/draft/operation", post(operation))
        .route("/api/hosted/capture/draft/settle", post(settle))
        .route("/api/hosted/capture/draft/commit", post(commit))
        .route("/api/hosted/capture/draft/flush", post(flush))
        .layer(DefaultBodyLimit::max(MAX_CAPTURE_RECORD_BYTES + 16 * 1024))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftId {
    draft_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Flush {
    draft_id: String,
    record_revision: u64,
    input_revision: u64,
    last_mutation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Create {
    draft_id: String,
    mutation_id: String,
    slot: String,
    input: CaptureInput,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    draft_id: String,
    mutation_id: String,
    expected_revision: u64,
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
    mutation_id: String,
    expected_revision: u64,
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
struct Freeze {
    draft_id: String,
    mutation_id: String,
    expected_revision: u64,
    request: FrozenCaptureRequest,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Advance {
    draft_id: String,
    mutation_id: String,
    expected_revision: u64,
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
        slot: String,
        input: CaptureInput,
    },
    Input {
        expected_revision: u64,
        input: CaptureInput,
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
impl MutationCommand {
    fn core(self) -> Result<CaptureMutationCommand, ApiError> {
        Ok(match self {
            Self::Checkpoint { expected_revision } => {
                CaptureMutationCommand::Checkpoint { expected_revision }
            }
            Self::Create { .. } => return Err(changed()),
            MutationCommand::Input {
                expected_revision,
                input,
            } => CaptureMutationCommand::Input {
                expected_revision,
                input,
            },
            MutationCommand::Freeze {
                expected_revision,
                request,
            } => CaptureMutationCommand::Freeze {
                expected_revision,
                request,
            },
            MutationCommand::Advance {
                expected_revision,
                operation_id,
                reset_input,
            } => CaptureMutationCommand::Advance {
                expected_revision,
                operation_id,
                reset_input,
            },
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    draft_id: String,
    plan_id: String,
    accept_losses: bool,
}

fn ordinary(slot: &str) -> Prepare {
    Prepare {
        resource: slot.into(),
        operation_id: String::new(),
        kind: "create".into(),
        source: None,
        destination: Some(slot.into()),
        trash_operation_id: None,
        template: None,
        parameters: BTreeMap::new(),
        inputs: vec![],
        capture: None,
    }
}
fn current_target(inner: &mut HostedInner, actor: &str, slot: &str) -> Result<PathBuf, ApiError> {
    authorize(inner, actor, &ordinary(slot))?;
    let configured = &inner.config.structure_slots[slot];
    let parent = &inner.config.directories[&configured.parent];
    let path = inner.config.root.join(&parent.path);
    inner
        .core
        .qualified_query_scope_at(&inner.config.root, &path, &parent.reference, "live")
        .map_err(|_| unavailable())?;
    Ok(path)
}
fn private_current(
    inner: &mut HostedInner,
    actor: &str,
    id: &str,
) -> Result<CaptureDraft, ApiError> {
    let owner = scope(inner, actor);
    let root = inner.config.root.clone();
    let draft = inner
        .core
        .with_actor_for_root(&owner, &root, |core| core.read_capture_draft(id))
        .map_err(core_error)?;
    let slot = draft
        .target
        .as_ref()
        .and_then(|target| target.hosted_slot.as_deref())
        .ok_or_else(unavailable)?;
    let actual_parent = current_target(inner, actor, slot)?;
    let actual = inner
        .core
        .with_actor_for_root(&owner, &root, |core| core.resolve_capture_target(id))
        .map_err(core_error)?;
    if Path::new(&actual.root) != root || Path::new(&actual.parent) != actual_parent {
        return Err(unavailable());
    }
    Ok(draft)
}
fn active_view(inner: &mut HostedInner, actor: &str, id: &str) -> Result<Value, ApiError> {
    let draft = private_current(inner, actor, id)?;
    let slot = draft.target.as_ref().unwrap().hosted_slot.as_ref().unwrap();
    let input = &draft.current_input;
    if !input.fields.is_empty()
        || !input.type_ids.is_empty()
        || !input.raw_inputs.is_empty()
        || !input.detached_raw_inputs.is_empty()
    {
        let _catalog = typed_catalog(inner, actor, slot)?;
        let configured = &inner.config.structure_slots[slot];
        if input
            .type_ids
            .iter()
            .any(|id| !configured.types.contains(id))
            || input
                .fields
                .iter()
                .map(|field| &field.field_id)
                .chain(input.raw_inputs.iter().map(|raw| &raw.field_id))
                .chain(
                    input
                        .detached_raw_inputs
                        .iter()
                        .map(|raw| &raw.input.field_id),
                )
                .any(|field_id| {
                    !creation_grants(inner, actor, slot)
                        .ok()
                        .and_then(|rights| rights.fields.get(field_id))
                        .is_some_and(|right| right.read && right.write)
                })
        {
            return Err(unavailable());
        }
        let document = inner
            .core
            .read_config(&inner.config.root, ConfigRole::Schema)
            .map_err(core_error)?;
        let witness = inner
            .core
            .take_snapshot_witness(&document.base_id)
            .map_err(core_error)?;
        if witness.retained_bytes() > MAX_BYTES
            || document.bytes.is_some() && document.revision.is_none()
        {
            return Err(changed());
        }
        let schema = match &document.bytes {
            Some(bytes) => {
                Schema::from_author_json(std::str::from_utf8(bytes).map_err(|_| changed())?)
            }
            None => Schema::bundled(),
        }
        .map_err(|_| changed())?;
        let digest = digest(&serde_json::to_vec(&schema).map_err(|_| changed())?);
        if input.form_basis.as_ref().is_none_or(|basis| {
            basis.schema_ref != document.role_ref || basis.semantic_digest != digest
        }) {
            return Err(changed());
        }
        inner
            .core
            .validate_snapshot_witness(&inner.config.root, Path::new(&document.path), &witness)
            .map_err(core_error)?;
    }
    if let Some(template_id) = &input.template_id {
        let configured = inner
            .config
            .structure_templates
            .get(template_id)
            .ok_or_else(unavailable)?;
        let mut request = ordinary(slot);
        request.kind = "template".into();
        request.template = Some(template_id.clone());
        request.source = Some(configured.source.clone());
        authorize(inner, actor, &request)?;
    }
    Ok(
        json!({"slot":slot,"textInputs":{"name":input.name,"title":input.title,"body":input.body,"rootInput":input.root_input,"parentInput":input.parent_input,"parametersText":input.parameters_text},"bodyMode":input.body_mode,"typeIds":input.type_ids,"fields":input.fields,"formBasis":input.form_basis,"templateId":input.template_id,"rawInputs":input.raw_inputs,"detachedRawInputs":input.detached_raw_inputs}),
    )
}
fn view(inner: &mut HostedInner, actor: &str, id: &str) -> Result<Value, ApiError> {
    let owner = scope(inner, actor);
    let recovery = inner
        .core
        .with_actor(&owner, |core| core.read_capture_author_recovery(id))
        .map_err(core_error)?;
    // Same actor identity alone cannot renew the former resource capabilities.
    let active = if recovery.input_cut == weftext_core::CaptureInputCut::Full {
        active_view(inner, actor, id).ok()
    } else {
        None
    };
    Ok(json!({"recovery":recovery,"active":active}))
}
async fn create(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Create>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let parent = current_target(inner, &actor, &req.slot)?;
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.create_targeted_capture_draft(
                        &req.draft_id,
                        &req.mutation_id,
                        req.input,
                        &root,
                        &parent,
                        Some(req.slot),
                    )
                })
                .map_err(core_error)?;
            view(inner, &actor, &req.draft_id)
        })
        .await?,
    ))
}
async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<DraftId>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            view(inner, &actor, &req.draft_id)
        })
        .await?,
    ))
}
async fn flush(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Flush>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            private_current(inner, &actor, &req.draft_id)?;
            active_view(inner, &actor, &req.draft_id)?;
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            let receipt = inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.capture_flush_receipt(
                        &req.draft_id,
                        req.record_revision,
                        req.input_revision,
                        &req.last_mutation_id,
                    )
                })
                .map_err(core_error)?;
            value(receipt)
        })
        .await?,
    ))
}
async fn list(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(_req): Json<Empty>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let owner = scope(inner, &actor);
            let drafts = inner
                .core
                .with_actor(&owner, |core| core.list_capture_author_recovery())
                .map_err(core_error)?;
            let mut result = vec![];
            for draft in drafts {
                result.push(view(inner, &actor, &draft.draft_id)?);
            }
            Ok(json!(result))
        })
        .await?,
    ))
}
async fn input(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Input>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            private_current(inner, &actor, &req.draft_id)?;
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.update_capture_input(
                        &req.draft_id,
                        req.expected_revision,
                        &req.mutation_id,
                        req.input,
                    )
                })
                .map_err(core_error)?;
            view(inner, &actor, &req.draft_id)
        })
        .await?,
    ))
}
async fn author_input(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<AuthorInput>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let owner = scope(inner, &actor);
            inner
                .core
                .with_actor(&owner, |core| {
                    core.update_capture_author_input(
                        &req.draft_id,
                        req.expected_revision,
                        &req.mutation_id,
                        req.input,
                    )
                })
                .map_err(core_error)?;
            view(inner, &actor, &req.draft_id)
        })
        .await?,
    ))
}
async fn checkpoint(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Checkpoint>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let owner = scope(inner, &actor);
            inner
                .core
                .with_actor(&owner, |core| {
                    core.checkpoint_capture_input(
                        &req.draft_id,
                        req.expected_revision,
                        &req.mutation_id,
                    )
                })
                .map_err(core_error)?;
            view(inner, &actor, &req.draft_id)
        })
        .await?,
    ))
}
async fn input_mutation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<InputMutation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let owner = scope(inner, &actor);
            let (status, _) = inner
                .core
                .with_actor(&owner, |core| {
                    core.capture_input_mutation_status(
                        &req.draft_id,
                        &req.mutation_id,
                        req.expected_revision,
                        &req.fingerprint,
                    )
                })
                .map_err(core_error)?;
            Ok(json!({"status":status,"draft":view(inner,&actor,&req.draft_id)?}))
        })
        .await?,
    ))
}
async fn author_mutation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<AuthorInput>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let owner = scope(inner, &actor);
            let (status, _recovery) = inner
                .core
                .with_actor(&owner, |core| {
                    core.capture_author_mutation_status(
                        &req.draft_id,
                        &req.mutation_id,
                        req.expected_revision,
                        &req.input,
                    )
                })
                .map_err(core_error)?;
            Ok(json!({"status":status,"draft":view(inner, &actor, &req.draft_id)?}))
        })
        .await?,
    ))
}
async fn mutation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Mutation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let owner = scope(inner, &actor);
            let (status, _recovery) = match req.command {
                MutationCommand::Create { slot, input } => inner
                    .core
                    .with_actor(&owner, |core| {
                        core.capture_hosted_create_status(
                            &req.draft_id,
                            &req.mutation_id,
                            &slot,
                            input,
                        )
                    })
                    .map_err(core_error)?,
                command => {
                    let command = command.core()?;
                    inner
                        .core
                        .with_actor(&owner, |core| {
                            core.capture_author_command_status(
                                &req.draft_id,
                                &req.mutation_id,
                                &command,
                            )
                        })
                        .map_err(core_error)?
                }
            };
            Ok(json!({"status":status,"draft":view(inner, &actor, &req.draft_id)?}))
        })
        .await?,
    ))
}
async fn freeze(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Freeze>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            private_current(inner, &actor, &req.draft_id)?;
            if req.request.request_kind != "hostedStructure" {
                return Err(ApiError::bad(
                    weftext_messages::UiMessage::error_api_capture_capture_request_kind_differs(),
                ));
            }
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.freeze_capture_submission(
                        &req.draft_id,
                        req.expected_revision,
                        &req.mutation_id,
                        req.request,
                    )
                })
                .map_err(core_error)?;
            view(inner, &actor, &req.draft_id)
        })
        .await?,
    ))
}
async fn advance(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Advance>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            private_current(inner, &actor, &req.draft_id)?;
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.advance_capture_submission(
                        &req.draft_id,
                        req.expected_revision,
                        &req.mutation_id,
                        &req.operation_id,
                        req.reset_input,
                    )
                })
                .map_err(core_error)?;
            view(inner, &actor, &req.draft_id)
        })
        .await?,
    ))
}
fn pending(
    inner: &mut HostedInner,
    auth: &Auth,
    id: &str,
) -> Result<(String, String, FrozenCaptureRequest), ApiError> {
    let actor = inner.actor(auth)?;
    let owner = scope(inner, &actor);
    let root = inner.config.root.clone();
    let draft = inner
        .core
        .with_actor_for_root(&owner, &root, |core| core.read_capture_draft(id))
        .map_err(core_error)?;
    let slot = draft
        .target
        .and_then(|target| target.hosted_slot)
        .ok_or_else(changed)?;
    // Original-result recovery precedes any attempt to reinterpret today's
    // configured parent or Schema. The existing safe-status filter still owns
    // all result delivery and its current resource permissions.
    anchor_access(inner, &actor, &slot)?;
    let request = draft.pending.ok_or_else(changed)?;
    if request.request_kind != "hostedStructure" {
        return Err(changed());
    }
    Ok((actor, slot, request))
}
fn require_full_input(inner: &mut HostedInner, actor: &str, id: &str) -> Result<(), ApiError> {
    let owner = scope(inner, actor);
    let draft = inner
        .core
        .with_actor(&owner, |core| core.read_capture_author_recovery(id))
        .map_err(core_error)?;
    if draft.input_cut != weftext_core::CaptureInputCut::Full {
        return Err(changed());
    }
    Ok(())
}
async fn submit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<DraftId>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, slot, original) = pending(inner, &auth, &req.draft_id)?;
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            if let Some(status) = inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.actor_feature_status(&slot, &original.operation_id)
                })
                .map_err(core_error)?
            {
                return safe_status(inner, &actor, status);
            }
            require_full_input(inner, &actor, &req.draft_id)?;
            let request: Prepare = serde_json::from_str(&original.wire)
                .map_err(|_| ApiError::bad(weftext_messages::UiMessage::error_api_capture_the_original_capture_request_contains_unfinished_author_input_original_text_and()))?;
            if !matches!(request.kind.as_str(), "create" | "template")
                || request.resource != slot
                || request.destination.as_deref() != Some(slot.as_str())
                || request.operation_id != original.operation_id
            {
                return Err(changed());
            }
            prepare_inner(inner, &auth, request)
        })
        .await?,
    ))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<DraftId>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, slot, original) = pending(inner, &auth, &req.draft_id)?;
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            match inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.actor_feature_status(&slot, &original.operation_id)
                })
                .map_err(core_error)?
            {
                Some(status) => safe_status(inner, &actor, status),
                None => Ok(
                    json!({"operationId":original.operation_id,"resource":slot,"status":"unknown"}),
                ),
            }
        })
        .await?,
    ))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<DraftId>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, slot, original) = pending(inner, &auth, &req.draft_id)?;
            let owner = scope(inner, &actor);
            let root = inner.config.root.clone();
            let status = inner
                .core
                .with_actor_for_root(&owner, &root, |core| {
                    core.actor_feature_settle(&slot, &original.operation_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, status)
        })
        .await?,
    ))
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Commit>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, slot, original) = pending(inner, &auth, &req.draft_id)?;
            require_full_input(inner, &actor, &req.draft_id)?;
            commit_inner(
                inner,
                &auth,
                super::Commit {
                    resource: slot,
                    operation_id: original.operation_id,
                    plan_id: req.plan_id,
                    accept_losses: req.accept_losses,
                },
            )
        })
        .await?,
    ))
}
