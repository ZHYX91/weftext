//! Local timeline adapters; all authorization, exact-source reads and restore
//! decisions stay inside the original Core.
use super::{ApiResult, AppState, core_call, value};
use crate::RequestJson;
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use weftext_core::{CoreError, HistoryRestoreRequest, HistoryRestoreResult, HistoryRestoreStatus};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/history/list", post(list))
        .route("/api/history/read", post(read))
        .route("/api/history/compare", post(compare))
        .route("/api/history/restore", post(restore))
        .route("/api/history/status", post(status))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionRequest {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VersionRequest {
    session_id: String,
    version_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StatusRequest {
    operation_id: String,
}
async fn list(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<SessionRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.list_history(&req.session_id)).await?,
    )?))
}
async fn read(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<VersionRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.read_history(&req.session_id, &req.version_id)
        })
        .await?,
    )?))
}
async fn compare(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<VersionRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.compare_history(&req.session_id, &req.version_id)
        })
        .await?,
    )?))
}
async fn restore(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<HistoryRestoreRequest>,
) -> ApiResult {
    let outcome = core_call(&state, move |core| {
        let original = req.clone();
        let result = core.restore_history(req);
        restore_outcome(result, || {
            core.history_restore_status_for_request(&original)
        })
    })
    .await?;
    Ok(Json(value(outcome)?))
}
/// Restore and status have one envelope. A later result-read failure cannot
/// hide the original receipt; a different request still conflicts.
fn restore_outcome(
    result: Result<HistoryRestoreResult, CoreError>,
    lookup: impl FnOnce() -> Result<HistoryRestoreStatus, CoreError>,
) -> Result<HistoryRestoreStatus, CoreError> {
    match result {
        Ok(result) => Ok(HistoryRestoreStatus {
            operation_id: result.receipt.operation_id.clone(),
            status: "committed".into(),
            receipt: Some(result.receipt.clone()),
            result: Some(result),
            result_error: None,
        }),
        Err(error) => {
            let status = lookup()?;
            if status.status == "committed" {
                Ok(status)
            } else {
                Err(error)
            }
        }
    }
}
async fn status(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<StatusRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.history_restore_status(&req.operation_id)
        })
        .await?,
    )?))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::CoreTestFixture;
    use std::{fs, path::PathBuf};
    use weftext_core::{Core, SaveRequest};

    struct Fixture {
        _temp: tempfile::TempDir,
        core: Core,
        request: HistoryRestoreRequest,
        result: HistoryRestoreResult,
        directory: PathBuf,
        parked: PathBuf,
        version_count: usize,
    }
    fn fixture() -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let doc = core.create_note_fixture(&root, &root, "Note").unwrap();
        let selected = core.list_history(&doc.session_id).unwrap().versions[0]
            .version_id
            .clone();
        let saved = core
            .save(SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source: format!("{}\nnewer saved version", doc.source),
                operation_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        let request = HistoryRestoreRequest {
            session_id: doc.session_id.clone(),
            base_id: saved.receipt.base_id,
            version_id: selected,
            operation_id: uuid::Uuid::new_v4().to_string(),
            draft_action: "none".into(),
            draft_source: None,
        };
        let result = core.restore_history(request.clone()).unwrap();
        let version_count = core.list_history(&doc.session_id).unwrap().versions.len();
        Fixture {
            directory: root.join("Note"),
            parked: temp.path().join("retained-original"),
            _temp: temp,
            core,
            request,
            result,
            version_count,
        }
    }
    #[test]
    fn first_actual_core_result_read_failure_then_available_status_has_one_complete_envelope() {
        let mut f = fixture();
        fs::rename(&f.directory, &f.parked).unwrap();
        let first_read = f.core.restore_history(f.request.clone()).unwrap_err();
        assert!(
            f.core
                .history_restore_status(&f.request.operation_id)
                .unwrap()
                .result
                .is_none()
        );
        // Deterministically restore the same physical object between the two
        // reads. This is an adapter boundary test, not a timed filesystem race.
        fs::rename(&f.parked, &f.directory).unwrap();
        let recovered = restore_outcome(Err(first_read), || {
            f.core.history_restore_status_for_request(&f.request)
        })
        .unwrap();
        let normal = restore_outcome(Ok(f.result.clone()), || panic!("no second read")).unwrap();
        assert_eq!(value(recovered).unwrap(), value(normal).unwrap());
        assert_eq!(
            f.core
                .list_history(&f.request.session_id)
                .unwrap()
                .versions
                .len(),
            f.version_count
        );
    }
    #[test]
    fn unavailable_actual_result_preserves_the_original_commit_receipt_in_the_same_envelope() {
        let mut f = fixture();
        fs::rename(&f.directory, &f.parked).unwrap();
        let first_read = f.core.restore_history(f.request.clone()).unwrap_err();
        let outcome = restore_outcome(Err(first_read), || {
            f.core.history_restore_status_for_request(&f.request)
        })
        .unwrap();
        assert_eq!(outcome.status, "committed");
        assert_eq!(outcome.operation_id, f.request.operation_id);
        assert_eq!(outcome.receipt.unwrap().base_id, f.result.receipt.base_id);
        assert!(outcome.result.is_none() && outcome.result_error.is_some());
        assert_eq!(
            fs::read_to_string(f.parked.join("Note.typ")).unwrap(),
            f.result.source
        );
    }
    #[test]
    fn changed_restore_input_still_conflicts_instead_of_delivering_another_requests_commit() {
        let mut f = fixture();
        f.request.version_id = uuid::Uuid::new_v4().to_string();
        let error = f.core.restore_history(f.request.clone()).unwrap_err();
        let error = restore_outcome(Err(error), || {
            f.core.history_restore_status_for_request(&f.request)
        })
        .unwrap_err();
        assert_eq!(error.code, "operationConflict");
    }
}
