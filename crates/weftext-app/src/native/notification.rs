//! Trusted OS adapter. Only the desktop coordinator owns these entry points;
//! neither HTTP clients nor activation payloads can manufacture a Core permit.
use std::{
    io,
    path::PathBuf,
    time::{Instant, SystemTime},
};
use weftext_core::reminder_device_state::{
    ReminderDeliveryPermit, ReminderDeliveryStatus, ReminderNotificationRequest,
    ReminderOriginalNotification, ReminderOsCallResult, ReminderSubmissionObservation,
    ReminderSubmissionOutcome,
};
use windows::{
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{
        NotificationMirroring, NotificationSetting, ToastNotification, ToastNotificationManager,
        ToastNotifier,
    },
    Win32::Foundation::{E_ACCESSDENIED, E_INVALIDARG, E_UNEXPECTED},
    core::{Error, HSTRING, Result},
};

pub(super) use crate::desktop_api::NotificationState as State;
#[derive(Clone, Debug)]
pub(super) struct Capability {
    pub state: State,
    pub app_id: Option<String>,
}
fn state(value: NotificationSetting) -> State {
    match value {
        NotificationSetting::Enabled => State::Enabled,
        NotificationSetting::DisabledForApplication => State::DisabledForApplication,
        NotificationSetting::DisabledForUser => State::DisabledForUser,
        NotificationSetting::DisabledByGroupPolicy => State::DisabledByPolicy,
        NotificationSetting::DisabledByManifest => State::DisabledByManifest,
        _ => State::Unknown,
    }
}
fn notifier(expected: &str) -> Result<ToastNotifier> {
    let identity = super::package::current()?.ok_or_else(|| Error::from(E_ACCESSDENIED))?;
    if identity.app_id != expected {
        return Err(Error::from(E_ACCESSDENIED));
    }
    // A packaged desktop uses its actual application identity. Never hand an
    // arbitrary supplied AUMID to CreateToastNotifierWithId as qualification.
    ToastNotificationManager::CreateToastNotifier()
}
pub(super) async fn capability() -> Capability {
    super::winrt::call(|| {
        let Some(identity) = super::package::current()? else {
            return Ok(Capability {
                state: State::Unavailable,
                app_id: None,
            });
        };
        Ok::<_, Error>(Capability {
            state: state(notifier(&identity.app_id)?.Setting()?),
            app_id: Some(identity.app_id),
        })
    })
    .await
    .unwrap_or(Capability {
        state: State::Unknown,
        app_id: None,
    })
}

fn text(value: &str) -> Result<String> {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            character
                if character == '\t'
                    || character == '\n'
                    || character == '\r'
                    || (character >= ' ' && character != '\u{fffe}' && character != '\u{ffff}') =>
            {
                escaped.push(character)
            }
            _ => return Err(Error::from(E_INVALIDARG)),
        }
    }
    Ok(escaped)
}
fn document(request: &ReminderNotificationRequest) -> Result<XmlDocument> {
    if !crate::broker::canonical_uuid(&request.activation_key)
        || request.tag.len() != 64
        || !request
            .tag
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || request.group != "weftext.reminders.v1"
        || request.title.len() > 1024
        || request.body.len() > 2048
    {
        return Err(Error::from(E_INVALIDARG));
    }
    let xml = format!(
        "<toast launch=\"{}\" activationType=\"foreground\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        request.activation_key,
        text(&request.title)?,
        text(&request.body)?,
    );
    let document = XmlDocument::new()?;
    document.LoadXml(&HSTRING::from(xml))?;
    Ok(document)
}
struct Prepared {
    notifier: ToastNotifier,
    toast: ToastNotification,
}
impl Prepared {
    fn new(request: &ReminderNotificationRequest) -> Result<Self> {
        let notifier = notifier(&request.app_id)?;
        if state(notifier.Setting()?) != State::Enabled {
            return Err(Error::from(E_ACCESSDENIED));
        }
        let toast = ToastNotification::CreateToastNotification(&document(request)?)?;
        toast.SetTag(&HSTRING::from(&request.tag))?;
        toast.SetGroup(&HSTRING::from(&request.group))?;
        toast.SetNotificationMirroring(NotificationMirroring::Disabled)?;
        // Default priority/popup/audio respect system settings and Do Not
        // Disturb. There is no scheduled toast, wake request or focus action.
        Ok(Self { notifier, toast })
    }
    fn show(&self) -> Result<()> {
        self.notifier.Show(&self.toast)
    }
}

#[derive(Clone)]
pub(super) struct Profile {
    path: PathBuf,
    physical: String,
}
impl Profile {
    pub fn from_qualified(profile: &weftext_core::QualifiedProfile) -> Self {
        Self {
            path: profile.path().into(),
            physical: profile.physical_identity().into(),
        }
    }
    fn qualify(&self) -> io::Result<weftext_core::QualifiedProfile> {
        let profile =
            weftext_core::qualify_existing_profile(&self.path).map_err(io::Error::other)?;
        if profile.physical_identity() != self.physical {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "The original notification profile changed",
            ));
        }
        Ok(profile)
    }
}
/// An accepted permit is owned by the blocking tail until its actual OS call
/// and durable finish have completed. Caller cancellation never drops this tail.
pub(super) async fn submit(
    state: &crate::AppState,
    profile: Profile,
    permit: ReminderDeliveryPermit,
) -> io::Result<ReminderDeliveryStatus> {
    let core = state.core.clone();
    crate::work::spawn_blocking(move || {
        let prepared = (|| -> io::Result<_> {
            let apartment = super::winrt::Apartment::enter().map_err(io::Error::other)?;
            let prepared = Prepared::new(permit.request()).map_err(io::Error::other)?;
            let profile = profile.qualify()?;
            let device = core.lock().map_err(|_| io::Error::other("The original Core is unavailable"))?
                .device_identity().map_err(io::Error::other)?;
            crate::host_state::Root::qualify(false)?.register(
                &profile, &device, &permit.request().activation_key,
                permit.execution_key(), &permit.request().app_id,
            )?;
            Ok((apartment, prepared, profile))
        })();
        let mut observation = None;
        let outcome = match prepared {
            Err(error) => ReminderSubmissionOutcome::FailedBeforeCall(weftext_core::CoreDescription::from(weftext_core::CoreError::new("notificationPreparationFailed", weftext_messages::UiMessage::error_notification_preparation_failed()).caused_by(weftext_core::CoreError::from(error)))),
            Ok((_apartment, prepared, _profile)) => {
                let admitted = core.lock().map_err(|_| io::Error::other("The original Core is unavailable"))?
                    .check_reminder_permit(&permit);
                match admitted {
                    Err(error) if error.code == "reminderPermitUsed" => ReminderSubmissionOutcome::Unknown(
                        weftext_core::CoreDescription::from(error)),
                    Err(error) => ReminderSubmissionOutcome::FailedBeforeCall(weftext_core::CoreDescription::from(error)),
                    Ok(()) => {
                        // No await, preparation, catalog write or retained Core
                        // lock occurs between final one-use admission and Show.
                        let started_at = SystemTime::now();
                        let started = Instant::now();
                        let result = prepared.show();
                        let elapsed = started.elapsed();
                        let returned_at = SystemTime::now();
                        let os_result = match &result {
                            Ok(()) => ReminderOsCallResult::ReturnedSuccess,
                            Err(error) => ReminderOsCallResult::ReturnedFailure { hresult: error.code().0 },
                        };
                        // Convert only after the call. A missing clock observation
                        // cannot turn an actual OS result into a pre-call failure.
                        observation = ReminderSubmissionObservation::from_system_times(
                            started_at, returned_at, elapsed, os_result,
                        ).ok();
                        match result {
                            Ok(()) => ReminderSubmissionOutcome::SubmittedToSystem,
                            Err(error) => ReminderSubmissionOutcome::Unknown(weftext_core::CoreDescription::from(weftext_core::CoreError::new("notificationResultUnknown", weftext_messages::UiMessage::error_notification_submission_unknown()).caused_by(weftext_core::CoreError::from(std::io::Error::other(error))))),
                        }
                    }
                }
            }
        };
        core.lock().map_err(|_| io::Error::other("The original notification result could not be stored"))?
            .finish_reminder_delivery(&permit, outcome, observation).map_err(io::Error::other)
    }).await.map_err(io::Error::other)?
}

/// Presence is useful only if the exact app, tag, group and full normalized XML
/// equal the durable original request. Absence is never a no-call/retry proof.
pub(super) async fn in_history(original: &ReminderOriginalNotification) -> io::Result<bool> {
    let request = original.request.clone();
    super::winrt::call(move || {
        let _notifier = notifier(&request.app_id)?;
        let expected = document(&request)?.GetXml()?.to_string();
        let history = ToastNotificationManager::History()?.GetHistory()?;
        if history.Size()? > 256 {
            return Err(Error::from(E_UNEXPECTED));
        }
        for index in 0..history.Size()? {
            let item = history.GetAt(index)?;
            if item.Tag()? != request.tag || item.Group()? != request.group {
                continue;
            }
            let xml = item.Content()?.GetXml()?.to_string();
            if xml.len() > 16 * 1024 {
                return Err(Error::from(E_UNEXPECTED));
            }
            if xml == expected {
                return Ok(true);
            }
        }
        Ok(false)
    })
    .await
}
pub(super) async fn withdraw(request: &ReminderNotificationRequest) -> io::Result<()> {
    let request = request.clone();
    super::winrt::call(move || {
        let _notifier = notifier(&request.app_id)?;
        ToastNotificationManager::History()?
            .RemoveGroupedTag(&HSTRING::from(&request.tag), &HSTRING::from(&request.group))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xml_content_escapes_payload_and_refuses_invalid_control_characters() {
        assert_eq!(text("<&\"'>").unwrap(), "&lt;&amp;&quot;&apos;&gt;");
        assert!(text("before\0after").is_err());
        assert_eq!(text("中文\n\t").unwrap(), "中文\n\t");
    }
}
