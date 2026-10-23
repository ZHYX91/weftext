//! Personal presentation preferences in the Core profile, independent of author content and OS intents.
use crate::device_state::DeviceRecordKind;
use crate::{Core, CoreError, Result};
use serde::{Deserialize, Serialize};
use weftext_messages::{UiInteger, UiLocale, UiMessage};

const MAX_BYTES: usize = 16 * 1024;
const MAX_REVISION: u64 = UiInteger::MAX as u64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientPreferences {
    pub ui_locale: UiLocale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPreferencesSnapshot {
    pub revision: u64,
    pub preferences: ClientPreferences,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreferencesRecord {
    schema_version: u32,
    revision: u64,
    preferences: ClientPreferences,
}

impl Core {
    /// Reading or changing a language does not allocate an OS-bound device identity.
    pub fn client_preferences(&self) -> Result<ClientPreferencesSnapshot> {
        let record = self.read_device_record(DeviceRecordKind::ClientPreferences, MAX_BYTES)?;
        match record {
            None => Ok(ClientPreferencesSnapshot {
                revision: 0,
                preferences: ClientPreferences::default(),
            }),
            Some((bytes, _)) => {
                let record = parse_record(&bytes)?;
                Ok(ClientPreferencesSnapshot {
                    revision: record.revision,
                    preferences: record.preferences,
                })
            }
        }
    }

    pub fn update_client_preferences(
        &mut self,
        expected_revision: u64,
        preferences: ClientPreferences,
    ) -> Result<ClientPreferencesSnapshot> {
        let original = self.read_device_record(DeviceRecordKind::ClientPreferences, MAX_BYTES)?;
        let revision = match original.as_ref() {
            Some((bytes, _)) => parse_record(bytes)?.revision,
            None => 0,
        };
        if revision != expected_revision {
            return Err(CoreError::new(
                "clientPreferencesConflict",
                UiMessage::error_client_preferences_conflict(),
            ));
        }
        let revision = revision
            .checked_add(1)
            .filter(|revision| *revision <= MAX_REVISION)
            .ok_or_else(|| {
                CoreError::new(
                    "deviceStateEvidence",
                    UiMessage::error_client_preferences_revision_exhausted(),
                )
            })?;
        let record = PreferencesRecord {
            schema_version: 1,
            revision,
            preferences,
        };
        let bytes = serde_json::to_vec(&record)?;
        self.compare_and_write_device_record(
            DeviceRecordKind::ClientPreferences,
            original.as_ref().map(|(_, digest)| digest.as_str()),
            &bytes,
        )?;
        let readback = self.client_preferences()?;
        if readback.revision != revision || readback.preferences != preferences {
            return Err(CoreError::new(
                "deviceStateEvidence",
                UiMessage::error_client_preferences_readback_mismatch(),
            ));
        }
        Ok(readback)
    }
}

fn parse_record(bytes: &[u8]) -> Result<PreferencesRecord> {
    if bytes.len() > MAX_BYTES {
        return Err(CoreError::new(
            "deviceStateEvidence",
            UiMessage::error_client_preferences_byte_limit(),
        ));
    }
    let record: PreferencesRecord = serde_json::from_slice(bytes)?;
    if record.schema_version != 1 || record.revision == 0 || record.revision > MAX_REVISION {
        return Err(CoreError::new(
            "deviceStateEvidence",
            UiMessage::error_client_preferences_record_unsupported(),
        ));
    }
    Ok(record)
}

pub(crate) fn validate_record(bytes: &[u8]) -> Result<()> {
    parse_record(bytes).map(|_| ())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn language_defaults_and_reopens_without_creating_device_identity() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("state");
        let mut core = Core::new(&data).unwrap();
        assert_eq!(
            core.client_preferences().unwrap(),
            ClientPreferencesSnapshot {
                revision: 0,
                preferences: ClientPreferences::default()
            }
        );
        assert!(!core.device_record_path(DeviceRecordKind::Identity).exists());
        assert!(
            !core
                .device_record_path(DeviceRecordKind::ClientPreferences)
                .exists()
        );
        let saved = core
            .update_client_preferences(
                0,
                ClientPreferences {
                    ui_locale: UiLocale::EnUs,
                },
            )
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert!(!core.device_record_path(DeviceRecordKind::Identity).exists());
        assert!(
            !core
                .device_record_path(DeviceRecordKind::DesktopPreferences)
                .exists()
        );
        assert!(
            !core
                .device_record_path(DeviceRecordKind::Reminders)
                .exists()
        );
        drop(core);
        let mut reopened = Core::new(&data).unwrap();
        assert_eq!(reopened.client_preferences().unwrap(), saved);
        let changed = reopened
            .update_client_preferences(1, ClientPreferences::default())
            .unwrap();
        assert_eq!(changed.revision, 2);
        assert_eq!(changed.preferences.ui_locale, UiLocale::ZhCn);
    }

    #[test]
    fn stale_revision_preserves_the_exact_original() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        core.update_client_preferences(
            0,
            ClientPreferences {
                ui_locale: UiLocale::EnUs,
            },
        )
        .unwrap();
        let path = core.device_record_path(DeviceRecordKind::ClientPreferences);
        let original = fs::read(&path).unwrap();
        assert_eq!(
            core.update_client_preferences(0, ClientPreferences::default())
                .unwrap_err()
                .code,
            "clientPreferencesConflict"
        );
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn invalid_records_are_preserved_and_rejected_by_read_cas_and_backup() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        core.update_client_preferences(0, ClientPreferences::default())
            .unwrap();
        let path = core.device_record_path(DeviceRecordKind::ClientPreferences);
        for original in [br#"{"schemaVersion":1,"revision":1,"preferences":{"uiLocale":"en"}}"#.as_slice(), br#"{"schemaVersion":1,"revision":0,"preferences":{"uiLocale":"en-US"}}"#.as_slice(), br#"{"schemaVersion":1,"revision":9007199254740992,"preferences":{"uiLocale":"en-US"}}"#.as_slice(), br#"{"schemaVersion":1,"revision":1,"preferences":{"uiLocale":"en-US","documentLocale":"en-US"}}"#.as_slice()] {
            fs::write(&path, original).unwrap(); assert!(core.client_preferences().is_err());
            assert!(core.update_client_preferences(1, ClientPreferences::default()).is_err());
            assert!(crate::device_state::validate_backup_record(DeviceRecordKind::ClientPreferences, original).is_err());
            assert_eq!(fs::read(&path).unwrap(), original);
        }
    }

    #[test]
    fn strict_client_record_is_accepted_by_the_shared_backup_classifier() {
        let bytes = br#"{"schemaVersion":1,"revision":1,"preferences":{"uiLocale":"en-US"}}"#;
        validate_record(bytes).unwrap();
        crate::device_state::validate_backup_record(DeviceRecordKind::ClientPreferences, bytes)
            .unwrap();
    }
}
