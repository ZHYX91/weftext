use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;
use weftext_core::{
    Core, CoreError, DocumentDraftEvidence, DraftInputRequest, PendingOrderedDraft,
};
use weftext_messages::UiMessage;

use crate::{ApiError, AppState, core_call};

const MAX_PENDING_DRAFTS: usize = 8;

#[derive(Clone, Default)]
pub(crate) struct DraftLane(Arc<Mutex<Option<VecDeque<PendingOrderedDraft>>>>);

impl DraftLane {
    fn begin(&self) -> QueryDrafts<'_> {
        let previous = self.0.lock().unwrap().replace(VecDeque::new());
        assert!(
            previous.is_none(),
            "the Core lease admits one Query capture"
        );
        QueryDrafts(self)
    }

    fn next(&self) -> Option<PendingOrderedDraft> {
        self.0.lock().unwrap().as_mut()?.pop_front()
    }
}

struct QueryDrafts<'a>(&'a DraftLane);

impl QueryDrafts<'_> {
    fn finish(self, core: &mut Core) {
        // Close the lane before processing its finite tail. New requests use
        // the normal Core lease. A continuous producer cannot prolong Query
        // scope capture, including when the read was cancelled or failed.
        let pending = self.0.0.lock().unwrap().take().unwrap_or_default();
        for PendingOrderedDraft { request, complete } in pending {
            complete(weftext_data::without_query_stop(|| {
                core.persist_draft_ordered(request)
            }));
        }
    }
}

impl Drop for QueryDrafts<'_> {
    fn drop(&mut self) {
        // On panic the Core lease is poisoned, just like a normal Core call.
        // Drop pending senders so waiters get the normal service error.
        self.0.0.lock().unwrap().take();
    }
}

pub(crate) async fn persist(
    state: &AppState,
    request: DraftInputRequest,
) -> Result<DocumentDraftEvidence, ApiError> {
    let reply = {
        let mut lane = state.draft_lane.0.lock().unwrap();
        if let Some(pending) = lane
            .as_mut()
            .filter(|queue| queue.len() < MAX_PENDING_DRAFTS)
        {
            let (send, receive) = oneshot::channel();
            let permit = crate::work::registry().enter();
            pending.push_back(PendingOrderedDraft {
                request,
                complete: Box::new(move |result| {
                    let _permit = permit;
                    let _ = send.send(result);
                }),
            });
            Ok(receive)
        } else {
            Err(request)
        }
    };
    match reply {
        Ok(reply) => reply
            .await
            .map_err(|_| ApiError::internal(UiMessage::error_file_service_incomplete()))?
            .map_err(ApiError::from),
        Err(request) => core_call(state, move |core| core.persist_draft_ordered(request)).await,
    }
}

pub(crate) async fn query_capture<T: Send + 'static>(
    state: &AppState,
    read: impl FnOnce(&mut Core) -> Result<T, CoreError> + Send + 'static,
) -> Result<T, ApiError> {
    let lane = state.draft_lane.clone();
    core_call(state, move |core| {
        let active = lane.begin();
        let poll = lane.clone();
        let result = core.with_query_draft_checkpoints(Arc::new(move || poll.next()), read);
        active.finish(core);
        result
    })
    .await
}
