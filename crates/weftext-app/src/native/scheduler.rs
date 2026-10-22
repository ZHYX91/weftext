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
    time::Duration,
};
use tokio::sync::watch;
use weftext_core::reminder_device_state::{
    ReminderDeliveryStart, ReminderDeliveryState, ReminderNotificationOptions,
    ReminderOriginalNotification, ReminderScheduleCursor, ReminderTiming,
};
use weftext_messages::UiMessage;

#[derive(Clone, Copy, PartialEq, Eq)]
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
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(5)) => {},
            changed = mode.changed() => { if changed.is_err() {break;} },
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
            match eligibility.timing {
                ReminderTiming::Upcoming => {}
                ReminderTiming::Missed => {
                    // This cannot use repeated slices of 64 as a fake durable
                    // recovery decision. The same Core record's new bounded
                    // summary admission is being integrated in this batch.
                    status.scope_issue_count = status.scope_issue_count.saturating_add(1);
                }
                ReminderTiming::Due => {
                    if eligibility.retained.as_ref().is_some_and(|retained| {
                        !matches!(
                            retained.delivery,
                            ReminderDeliveryState::FailedBeforeCall { .. }
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
                                "The desktop stopped accepting notification submissions",
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
