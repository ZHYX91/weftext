//! Rebuildable desktop-only traversal. Core remains the sole task, consent,
//! original-effect and delivery authority. Pausing stops new permit admission;
//! an already accepted permit always reaches its actual OS/result tail.
use super::notification;
use crate::desktop_api::{HostStatus, Scheduling};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::watch;
use weftext_core::reminder_device_state::{
    ReminderDeliveryStart, ReminderDeliveryState, ReminderNotificationOptions,
    ReminderOriginalNotification, ReminderScheduleCursor, ReminderSummaryPreparation,
    ReminderTiming,
};
use weftext_messages::UiMessage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Running,
    Paused,
    Stopped,
}
pub(super) struct Scheduler {
    mode: watch::Sender<Mode>,
    admitting: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<io::Result<()>>,
    pub fault: watch::Receiver<bool>,
}
struct Completion(watch::Sender<bool>);
impl Drop for Completion {
    fn drop(&mut self) {
        self.0.send_replace(true);
    }
}
impl Scheduler {
    pub fn start(state: crate::AppState, profile: &weftext_core::QualifiedProfile) -> Self {
        let (mode, changes) = watch::channel(Mode::Running);
        let admitting = Arc::new(AtomicBool::new(true));
        let profile = notification::Profile::from_qualified(profile);
        let accepted = admitting.clone();
        let (finished, fault) = watch::channel(false);
        let task = tokio::spawn(async move {
            let _completion = Completion(finished);
            run(state, profile, changes, accepted).await
        });
        Self {
            mode,
            admitting,
            task,
            fault,
        }
    }
    pub fn pause(&self, paused: bool) {
        self.admitting.store(!paused, Ordering::Release);
        let next = if paused { Mode::Paused } else { Mode::Running };
        let previous = *self.mode.borrow();
        if previous != next {
            self.mode.send_replace(next);
        }
    }
    pub fn refresh(&self) {
        let current = *self.mode.borrow();
        if current == Mode::Running {
            self.mode.send_replace(current);
        }
    }
    pub async fn stop(self) -> io::Result<()> {
        self.admitting.store(false, Ordering::Release);
        self.mode.send_replace(Mode::Stopped);
        self.task.await.map_err(io::Error::other)?
    }
}
const FALLBACK_RECHECK: Duration = Duration::from_secs(5);

fn retain_earliest_recheck(earliest: &mut Option<Instant>, hint: Option<Instant>) {
    if let Some(hint) = hint {
        *earliest = Some(earliest.map_or(hint, |previous| previous.min(hint)));
    }
}
fn recheck_deadline(earliest: Option<Instant>, after_sweep: Instant) -> Instant {
    // Preserve the original no-near-deadline pause after a completed sweep.
    // Core's absolute hint alone consumes elapsed traversal/submission time.
    let fallback = after_sweep + FALLBACK_RECHECK;
    earliest.map_or(fallback, |hint| hint.min(fallback))
}
async fn wait_for_recheck(mode: &mut watch::Receiver<Mode>, deadline: Instant) -> bool {
    tokio::select! {
        _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => true,
        changed = mode.changed() => changed.is_ok(),
    }
}

async fn run(
    state: crate::AppState,
    profile: notification::Profile,
    mut mode: watch::Receiver<Mode>,
    admitting: Arc<AtomicBool>,
) -> io::Result<()> {
    let mut status = HostStatus {
        desktop_active: true,
        ..Default::default()
    };
    let mut originals_after: Option<String> = None;
    loop {
        let current = *mode.borrow_and_update();
        match current {
            Mode::Stopped => break,
            Mode::Paused => {
                status.scheduling = Scheduling::Paused;
                crate::desktop_api::set_status(&state, status.clone())?;
                if mode.changed().await.is_err() {
                    break;
                }
                continue;
            }
            Mode::Running => {}
        }
        // Rebuilt every running sweep; pause/resume or OS clock refresh never
        // restores an old wake hint as saved task state or delivery permission.
        let mut next_recheck = None;
        let capability = notification::capability().await;
        status.notification_state = capability.state;
        status.scan_complete = false;
        status.scope_issue_count = 0;
        if capability.state == notification::State::Enabled {
            let result = sweep(
                &state,
                &profile,
                &admitting,
                &mut originals_after,
                capability.app_id.as_deref().ok_or_else(|| {
                    io::Error::other("The qualified notification identity is absent")
                })?,
                &mut status,
                &mut next_recheck,
            )
            .await;
            if result.is_err() {
                // The original effect record remains retained even if finish
                // could not publish. Never use this error to reissue its Show.
                status.scheduling = Scheduling::Restricted;
                status.scan_complete = false;
                status.scope_issue_count = status.scope_issue_count.saturating_add(1);
            }
        } else {
            status.scheduling = Scheduling::Restricted;
        }
        crate::desktop_api::set_status(&state, status.clone())?;
        if !wait_for_recheck(&mut mode, recheck_deadline(next_recheck, Instant::now())).await {
            break;
        }
    }
    status.scheduling = Scheduling::Stopping;
    crate::desktop_api::set_status(&state, status)?;
    Ok(())
}
async fn sweep(
    state: &crate::AppState,
    profile: &notification::Profile,
    admitting: &Arc<AtomicBool>,
    originals_after: &mut Option<String>,
    app_id: &str,
    status: &mut HostStatus,
    next_recheck: &mut Option<Instant>,
) -> io::Result<()> {
    status.scheduling = Scheduling::Running;
    // A bounded original page advances independently of current schedule
    // candidates. A withdrawn task must still have its old OS item rechecked.
    let after = originals_after.clone();
    let originals = crate::core_call(state, move |core| {
        core.reminder_original_notifications(after.as_deref())
    })
    .await
    .map_err(|_| io::Error::other("Original notification records are unavailable"))?;
    *originals_after = if originals.len() == 32 {
        originals.last().map(|item| item.execution_key.clone())
    } else {
        None
    };
    for original in originals {
        if !admitting.load(Ordering::Acquire) {
            return Ok(());
        }
        if reconcile_original(state, original, app_id).await.is_err() {
            // An unavailable old OS address is still retained and restricted;
            // it must not starve newly due, independently qualified reminders.
            status.scope_issue_count = status.scope_issue_count.saturating_add(1);
        }
    }
    let locale = crate::core_call(state, |core| core.client_preferences())
        .await
        .map_err(|_| io::Error::other("The trusted notification language is unavailable"))?
        .preferences
        .ui_locale;
    let options = ReminderNotificationOptions {
        app_id: app_id.into(),
        title: UiMessage::reminder_label().render(locale),
        body: UiMessage::reminder_generic_body().render(locale),
    };
    let mut cursor: Option<ReminderScheduleCursor> = None;
    loop {
        if !admitting.load(Ordering::Acquire) {
            return Ok(());
        }
        let current = cursor.take();
        let page = crate::core_call(state, move |core| {
            core.reminder_schedule_page(current.as_ref())
        })
        .await
        .map_err(|_| io::Error::other("The saved reminder scope is incomplete"))?;
        status.scope_issue_count = status.scope_issue_count.saturating_add(page.issues.len());
        for candidate in page.candidates {
            if !admitting.load(Ordering::Acquire) {
                return Ok(());
            }
            let expected = candidate.clone();
            let eligibility =
                crate::core_call(state, move |core| core.reminder_eligibility(&expected)).await;
            let Ok(eligibility) = eligibility else {
                status.scope_issue_count = status.scope_issue_count.saturating_add(1);
                continue;
            };
            retain_earliest_recheck(next_recheck, eligibility.next_recheck_at());
            match eligibility.timing {
                ReminderTiming::Upcoming => {}
                ReminderTiming::Missed => {
                    // Only Core's complete missed-set admission below may
                    // create an original summary, never a page-sized prefix.
                }
                ReminderTiming::Due => {
                    if eligibility.retained.as_ref().is_some_and(|retained| {
                        !matches!(
                            retained.delivery,
                            weftext_core::reminder_device_state::ReminderRecordedDelivery::FailedBeforeCall { .. }
                        )
                    }) {
                        continue;
                    }
                    let options = options.clone();
                    let admission = admitting.clone();
                    let started = crate::core_call(state, move |core| {
                        if !admission.load(Ordering::Acquire) {
                            return Err(weftext_core::CoreError::new(
                                "schedulerPaused",
                                weftext_messages::UiMessage::error_api_scheduler_the_desktop_stopped_accepting_notification_submissions(),
                            ));
                        }
                        core.begin_reminder_delivery(&candidate, options)
                    })
                    .await;
                    match started {
                        Ok(ReminderDeliveryStart::Started(permit)) => {
                            notification::submit(state, profile.clone(), permit).await?;
                        }
                        Ok(ReminderDeliveryStart::Retained(_)) => {}
                        Err(_) => {
                            status.scope_issue_count = status.scope_issue_count.saturating_add(1);
                        }
                    }
                }
            }
        }
        if page.complete {
            status.scan_complete = true;
            break;
        }
        cursor = page.next;
        if cursor.is_none() {
            return Err(io::Error::other(
                "The incomplete reminder page has no continuation",
            ));
        }
        tokio::task::yield_now().await;
    }
    if admitting.load(Ordering::Acquire) {
        let admission = admitting.clone();
        let summary = crate::core_call(state, move |core| {
            if !admission.load(Ordering::Acquire) {
                return Err(weftext_core::CoreError::new(
                    "schedulerPaused",
                    UiMessage::error_api_scheduler_the_desktop_stopped_accepting_notification_submissions(),
                ));
            }
            core.prepare_reminder_summary(options)
        }).await.map_err(io::Error::other)?;
        match summary {
            ReminderSummaryPreparation::Started {
                permit,
                issue_count,
                ..
            } => {
                status.scope_issue_count = status.scope_issue_count.saturating_add(issue_count);
                notification::submit(state, profile.clone(), *permit).await?;
            }
            ReminderSummaryPreparation::NoNewMissed { issue_count, .. } => {
                status.scope_issue_count = status.scope_issue_count.saturating_add(issue_count);
            }
            ReminderSummaryPreparation::CoolingDown { .. } => {}
        }
    }
    if status.scope_issue_count != 0 {
        status.scheduling = Scheduling::Restricted;
    }
    Ok(())
}

async fn reconcile_original(
    state: &crate::AppState,
    original: ReminderOriginalNotification,
    app_id: &str,
) -> io::Result<()> {
    let key = original.execution_key;
    let qualified = crate::core_call(state, move |core| core.refresh_reminder_notification(&key))
        .await
        .map_err(|_| io::Error::other("The original notification qualification is unavailable"))?;
    if qualified.original.request.app_id != app_id {
        return Err(io::Error::other(
            "The original OS notification identity differs from this actual package",
        ));
    }
    if !qualified.eligible {
        notification::withdraw(&qualified.original.request).await?;
    } else if matches!(
        qualified.original.delivery,
        ReminderDeliveryState::Submitting | ReminderDeliveryState::Unknown { .. }
    ) && notification::in_history(&qualified.original).await?
    {
        let original = qualified.original;
        crate::core_call(state, move |core| {
            core.confirm_reminder_in_system_history(&original.execution_key, &original.request)
        })
        .await
        .map_err(|_| io::Error::other("The exact OS history observation could not be stored"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poll_recheck_once(
        mode: &mut watch::Receiver<Mode>,
        deadline: Instant,
    ) -> std::task::Poll<bool> {
        let mut waiting = std::pin::pin!(wait_for_recheck(mode, deadline));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        std::future::Future::poll(waiting.as_mut(), &mut context)
    }

    #[test]
    fn recheck_deadline_consumes_scan_time_but_preserves_the_fallback_pause() {
        let observed = Instant::now();
        let hint = observed + Duration::from_millis(750);
        let mut earliest = None;
        retain_earliest_recheck(&mut earliest, Some(observed + Duration::from_secs(30)));
        retain_earliest_recheck(&mut earliest, None);
        retain_earliest_recheck(&mut earliest, Some(hint));
        retain_earliest_recheck(&mut earliest, Some(observed + Duration::from_secs(2)));
        assert_eq!(
            recheck_deadline(earliest, observed + Duration::from_millis(500)),
            hint
        );
        let finished = observed + Duration::from_secs(1);
        assert!(recheck_deadline(earliest, finished) < finished);
        // A long scan without an imminent hint still receives its old pause.
        let finished = observed + Duration::from_secs(20);
        assert_eq!(
            recheck_deadline(None, finished),
            finished + FALLBACK_RECHECK
        );
        assert_eq!(
            recheck_deadline(Some(observed + Duration::from_secs(120)), finished),
            finished + FALLBACK_RECHECK
        );
    }

    #[tokio::test]
    async fn elapsed_hint_and_refresh_pause_stop_wake_the_real_wait_helper() {
        let (mode, mut observed) = watch::channel(Mode::Running);
        assert!(wait_for_recheck(&mut observed, Instant::now() - Duration::from_secs(1)).await);
        for next in [Mode::Running, Mode::Paused, Mode::Running, Mode::Stopped] {
            mode.send_replace(next);
            assert!(matches!(
                poll_recheck_once(&mut observed, Instant::now() + Duration::from_secs(60)),
                std::task::Poll::Ready(true)
            ));
            assert_eq!(*observed.borrow_and_update(), next);
        }
        drop(mode);
        assert!(matches!(
            poll_recheck_once(&mut observed, Instant::now() + Duration::from_secs(60)),
            std::task::Poll::Ready(false)
        ));
    }

    #[tokio::test]
    async fn stop_keeps_waiting_for_its_already_accepted_task_tail() {
        let (mode, changes) = watch::channel(Mode::Running);
        let admitting = Arc::new(AtomicBool::new(true));
        let accepted = admitting.clone();
        let (finished, fault) = watch::channel(false);
        let (release, held) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let _completion = Completion(finished);
            held.await.map_err(io::Error::other)
        });
        let scheduler = Scheduler {
            mode,
            admitting,
            task,
            fault: fault.clone(),
        };
        let mut stopped = Box::pin(scheduler.stop());
        let initial = std::future::poll_fn(|context| {
            std::task::Poll::Ready(std::future::Future::poll(stopped.as_mut(), context))
        })
        .await;
        assert!(initial.is_pending());
        assert!(!accepted.load(Ordering::Acquire));
        assert_eq!(*changes.borrow(), Mode::Stopped);
        assert!(!*fault.borrow());
        release.send(()).unwrap();
        stopped.await.unwrap();
        assert!(*fault.borrow());
        // This is the production stop/join contract, not simulated OS delivery.
    }
}
