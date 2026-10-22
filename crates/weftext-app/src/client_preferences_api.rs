//! Authenticated, profile-local presentation settings; no author mutation or OS startup effect.
use crate::RequestJson;
use crate::{ApiResult, AppState, core_call, value};
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use weftext_core::ClientPreferences;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/client-preferences/read", post(read))
        .route("/api/client-preferences/update", post(update))
}

async fn read(State(state): State<AppState>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, |core| core.client_preferences()).await?,
    )?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Update {
    expected_revision: u64,
    preferences: ClientPreferences,
}

async fn update(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<Update>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.update_client_preferences(request.expected_revision, request.preferences)
        })
        .await?,
    )?))
}
