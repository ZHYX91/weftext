//! A short-lived ticket binds one user cancellation to one Query execution.
use super::*;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::watch;
use uuid::Uuid;

const TICKET_LIFETIME: Duration = Duration::from_secs(60);

#[derive(Clone, Default)]
struct Tickets(Arc<Mutex<HashMap<Uuid, Ticket>>>);

#[derive(Clone)]
pub(super) struct Owner(pub String);

struct Ticket {
    owner: String,
    created: Instant,
    started: bool,
    run: Arc<Run>,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Reading,
    Cancelling,
    Delivering,
}

pub(crate) struct Run {
    phase: Mutex<Phase>,
    stop: Arc<AtomicBool>,
    changed: watch::Sender<bool>,
    done: watch::Sender<Option<bool>>,
    pub(crate) jobs: crate::work::Registry,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            phase: Mutex::new(Phase::Reading),
            stop: Default::default(),
            changed: watch::channel(false).0,
            done: watch::channel(None).0,
            jobs: Default::default(),
        }
    }
}

tokio::task_local! { pub(crate) static CURRENT: Arc<Run>; }

impl Run {
    fn cancel(&self) -> bool {
        let mut phase = self.phase.lock().unwrap();
        if *phase == Phase::Delivering {
            return false;
        }
        *phase = Phase::Cancelling;
        self.stop.store(true, Ordering::Release);
        self.changed.send_replace(true);
        true
    }

    pub(crate) fn read<T>(&self, f: impl FnOnce() -> T) -> T {
        weftext_data::with_query_stop(self.stop.clone(), f)
    }
}

pub(crate) fn check() -> Result<(), ApiError> {
    if CURRENT
        .try_with(|r| r.stop.load(Ordering::Acquire))
        .unwrap_or(false)
    {
        Err(cancelled())
    } else {
        Ok(())
    }
}

pub(crate) async fn cancellation() {
    let Ok(mut changed) = CURRENT.try_with(|r| r.changed.subscribe()) else {
        return std::future::pending().await;
    };
    let _ = changed.wait_for(|v| *v).await;
}

pub(crate) fn publish() -> Result<(), ApiError> {
    CURRENT
        .try_with(|r| {
            let mut phase = r.phase.lock().unwrap();
            if *phase == Phase::Cancelling {
                return Err(cancelled());
            }
            *phase = Phase::Delivering;
            Ok(())
        })
        .unwrap_or(Ok(()))
}

pub(crate) fn cancelled() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "queryCancelled",
        UiMessage::error_api_data_query_cancelled(),
    )
}

fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "queryRunUnavailable",
        UiMessage::error_api_data_query_run_unavailable(),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TicketId {
    run_id: Uuid,
}

type MaybeOwner = Option<axum::Extension<Owner>>;
fn owner(auth: &MaybeOwner) -> &str {
    auth.as_ref().map_or("", |a| a.0.0.as_str())
}

async fn prepare(State(tickets): State<Tickets>, auth: MaybeOwner) -> ApiResult {
    let mut store = tickets.0.lock().unwrap();
    store.retain(|_, t| t.started || t.created.elapsed() < TICKET_LIFETIME);
    let owner = owner(&auth);
    if store.len() >= 256 || store.values().filter(|t| t.owner == owner).count() >= 32 {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "queryRunLimit",
            UiMessage::error_api_data_query_run_limit(),
        ));
    }
    let id = Uuid::new_v4();
    store.insert(
        id,
        Ticket {
            owner: owner.to_owned(),
            created: Instant::now(),
            started: false,
            run: Default::default(),
        },
    );
    Ok(Json(json!({"runId": id})))
}

async fn cancel(
    State(tickets): State<Tickets>,
    auth: MaybeOwner,
    RequestJson(id): RequestJson<TicketId>,
) -> ApiResult {
    let run = {
        let mut store = tickets.0.lock().unwrap();
        let ticket = store
            .get_mut(&id.run_id)
            .filter(|t| {
                t.owner == owner(&auth) && (t.started || t.created.elapsed() < TICKET_LIFETIME)
            })
            .ok_or_else(unavailable)?;
        if !ticket.run.cancel() {
            return Err(unavailable());
        }
        if !ticket.started {
            ticket.run.done.send_replace(Some(true));
        }
        ticket.run.clone()
    };
    let mut done = run.done.subscribe();
    let completed = *done
        .wait_for(Option::is_some)
        .await
        .map_err(|_| unavailable())?;
    if completed != Some(true) {
        return Err(unavailable());
    }
    Ok(Json(json!({"status":"cancelled"})))
}

async fn execute(State(tickets): State<Tickets>, request: Request, next: Next) -> Response {
    let Some(header) = request.headers().get("x-weftext-query-run") else {
        return next.run(request).await;
    };
    let Some(id) = header.to_str().ok().and_then(|v| Uuid::parse_str(v).ok()) else {
        return unavailable().into_response();
    };
    let path = request.uri().path();
    if !matches!(
        path,
        "/api/query/run"
            | "/api/query/definition/run"
            | "/api/query/embedded/run"
            | "/api/hosted/query"
            | "/api/hosted/query/embedded/run"
    ) {
        return unavailable().into_response();
    }
    let owner = request
        .extensions()
        .get::<Owner>()
        .map_or("", |v| v.0.as_str());
    let run = {
        let mut store = tickets.0.lock().unwrap();
        let Some(ticket) = store
            .get_mut(&id)
            .filter(|t| t.owner == owner && !t.started && t.created.elapsed() < TICKET_LIFETIME)
        else {
            return unavailable().into_response();
        };
        if ticket.run.stop.load(Ordering::Acquire) {
            store.remove(&id);
            return cancelled().into_response();
        }
        ticket.started = true;
        ticket.run.clone()
    };
    // A disconnected HTTP client must still leave an owner to release cuts and
    // reap children. The cancellation acknowledgement waits for that owner.
    struct OnDisconnect(Arc<Run>);
    impl Drop for OnDisconnect {
        fn drop(&mut self) {
            self.0.cancel();
        }
    }
    let _disconnect = OnDisconnect(run.clone());
    let task = crate::work::spawn(async move {
        struct Completion {
            tickets: Tickets,
            id: Uuid,
            run: Arc<Run>,
            clean: bool,
        }
        impl Drop for Completion {
            fn drop(&mut self) {
                self.tickets.0.lock().unwrap().remove(&self.id);
                self.run.done.send_replace(Some(self.clean));
            }
        }
        let mut completion = Completion {
            tickets,
            id,
            run: run.clone(),
            clean: false,
        };
        let response = CURRENT.scope(run.clone(), next.run(request)).await;
        let clean = run.jobs.drain().await.is_ok();
        let stopped = run.stop.load(Ordering::Acquire);
        completion.clean = clean;
        if !clean {
            ApiError::internal(UiMessage::error_api_worker_process_exit_unknown()).into_response()
        } else if stopped {
            cancelled().into_response()
        } else {
            response
        }
    });
    task.await.unwrap_or_else(|_| unavailable().into_response())
}

pub(super) fn attach<S: Clone + Send + Sync + 'static>(
    router: Router<S>,
    prefix: &str,
) -> Router<S> {
    let tickets = Tickets::default();
    let controls = Router::new()
        .route(&format!("{prefix}/prepare"), post(prepare))
        .route(&format!("{prefix}/cancel"), post(cancel))
        .with_state(tickets.clone());
    router
        .merge(controls)
        .layer(middleware::from_fn_with_state(tickets, execute))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accepted_cancellation_crosses_blocking_and_nested_worker_boundaries() {
        let run = Arc::new(Run::default());
        let worker = CURRENT
            .scope(run.clone(), async {
                crate::work::spawn(async {
                    cancellation().await;
                    crate::work::spawn_blocking(|| {
                        assert!(weftext_data::query_checkpoint().is_err());
                        assert!(publish().is_err());
                    })
                    .await
                    .unwrap();
                })
            })
            .await;
        assert!(run.cancel());
        worker.await.unwrap();
        run.jobs.drain().await.unwrap();
        assert!(weftext_data::query_checkpoint().is_ok());
    }

    #[tokio::test]
    async fn a_lost_nested_task_cannot_acknowledge_quiescence() {
        let run = Arc::new(Run::default());
        let worker = CURRENT
            .scope(run.clone(), async {
                crate::work::spawn(std::future::pending::<()>())
            })
            .await;
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
        assert!(run.jobs.drain().await.is_err());
        assert!(Run::default().jobs.drain().await.is_ok());
    }

    #[tokio::test]
    async fn publication_and_cancellation_have_one_winner() {
        let delivered = Arc::new(Run::default());
        CURRENT
            .scope(delivered.clone(), async {
                publish().unwrap();
            })
            .await;
        assert!(!delivered.cancel());
        let cancelled_run = Arc::new(Run::default());
        cancelled_run.cancel();
        CURRENT
            .scope(cancelled_run, async {
                assert!(publish().is_err());
            })
            .await;
    }
}
