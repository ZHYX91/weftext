//! Three independent desktop intents in the unique Core's private device state.
//! Login intent is not proof of an enabled OS startup registration.
use crate::device_state::{DeviceRecordKind, evidence};
use crate::{Core, DeviceIdentity, Result};
use serde::{Deserialize, Serialize};
use weftext_messages::UiMessage;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopPreferences {
    pub resident: bool,
    pub open_library_on_launch: bool,
    pub login_startup_requested: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreferencesSnapshot {
    pub device: DeviceIdentity,
    pub revision: u64,
    pub preferences: DesktopPreferences,
}

impl DesktopPreferencesSnapshot {
    /// Retained foreign preferences are inspectable but never become local OS
    /// effects just because a profile directory was copied to this machine.
    pub fn effective_preferences(&self) -> DesktopPreferences {
        if self.device.is_current {
            self.preferences
        } else {
            DesktopPreferences::default()
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreferencesRecord {
    schema_version: u32,
    device_id: String,
    revision: u64,
    preferences: DesktopPreferences,
}

impl Core {
    pub fn desktop_preferences(&mut self) -> Result<DesktopPreferencesSnapshot> {
        let device = self.device_identity()?;
        let record = self.read_device_record(DeviceRecordKind::DesktopPreferences, 16 * 1024)?;
        let (revision, preferences) = match record {
            None => (0, DesktopPreferences::default()),
            Some((bytes, _)) => {
                let record = parse_record(&bytes)?;
                if record.device_id != device.id {
                    return Err(evidence(
                        UiMessage::error_core_desktop_preferences_desktop_preferences_belong_to_another_retained_device_identity(),
                    ));
                }
                (record.revision, record.preferences)
            }
        };
        Ok(DesktopPreferencesSnapshot {
            device,
            revision,
            preferences,
        })
    }

    pub fn update_desktop_preferences(
        &mut self,
        expected_revision: u64,
        preferences: DesktopPreferences,
    ) -> Result<DesktopPreferencesSnapshot> {
        let device = self.device_identity()?;
        if !device.is_current {
            return Err(crate::CoreError::new(
                "deviceNotCurrent",
                UiMessage::error_core_retain_foreign_device_state(),
            ));
        }
        let original = self.read_device_record(DeviceRecordKind::DesktopPreferences, 16 * 1024)?;
        let original_revision = match original.as_ref() {
            Some((bytes, _)) => {
                let record = parse_record(bytes)?;
                if record.device_id != device.id {
                    return Err(evidence(
                        UiMessage::error_core_desktop_preferences_desktop_preferences_belong_to_another_retained_device_identity(),
                    ));
                }
                record.revision
            }
            None => 0,
        };
        if original_revision != expected_revision {
            return Err(crate::CoreError::new(
                "deviceStateConflict",
                UiMessage::error_core_desktop_preference_revision_changed(),
            ));
        }
        let revision = expected_revision.checked_add(1).ok_or_else(|| {
            evidence(
                UiMessage::error_core_desktop_preferences_desktop_preference_revision_exhausted(),
            )
        })?;
        let record = PreferencesRecord {
            schema_version: 1,
            device_id: device.id,
            revision,
            preferences,
        };
        let bytes = serde_json::to_vec(&record)?;
        self.compare_and_write_device_record(
            DeviceRecordKind::DesktopPreferences,
            original.as_ref().map(|(_, digest)| digest.as_str()),
            &bytes,
        )?;
        let readback = self.desktop_preferences()?;
        if readback.revision != revision
            || readback.preferences != preferences
            || !readback.device.is_current
        {
            return Err(evidence(
                UiMessage::error_core_desktop_preferences_desktop_preference_readback_differs_retain_original_mutation(),
            ));
        }
        Ok(readback)
    }
}

fn parse_record(bytes: &[u8]) -> Result<PreferencesRecord> {
    if bytes.len() > 16 * 1024 {
        return Err(evidence(
            UiMessage::error_core_desktop_preferences_desktop_preference_byte_budget_exceeded(),
        ));
    }
    let record: PreferencesRecord = serde_json::from_slice(bytes)?;
    let id = uuid::Uuid::parse_str(&record.device_id).map_err(|_| {
        evidence(UiMessage::error_core_desktop_preferences_invalid_preference_device_u_u_i_d())
    })?;
    if record.schema_version != 1
        || record.revision == 0
        || id.to_string() != record.device_id
        || id.get_version_num() != 4
    {
        return Err(evidence(
            UiMessage::error_core_desktop_preferences_unsupported_desktop_preference_record(),
        ));
    }
    Ok(record)
}

pub(crate) fn validate_record(bytes: &[u8]) -> Result<()> {
    parse_record(bytes).map(|_| ())
}
