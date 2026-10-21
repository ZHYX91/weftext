//! Core-owned machine/profile state. These records are never author-library data.
//!
//! One Core lease serializes CAS. ParentGuard retains the physical private root;
//! a complete, flushed temporary is atomically published before a readback receipt.
use crate::{Core, CoreError, FileIdentity, ParentGuard, Result, fresh, hash};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::PathBuf;

pub(crate) const DEVICE_STATE_ROOT: &str = "device-state";
const MAX_RECORD_BYTES: usize = 16 * 1024 * 1024;
const MAX_NAMESPACE_MEMBERS: usize = 128;
#[cfg(all(test, windows))]
thread_local! {static FAILPOINT:std::cell::Cell<Option<&'static str>>=const{std::cell::Cell::new(None)};}
#[cfg(all(test, windows))]
pub(crate) fn set_test_failpoint(point: &'static str) {
    FAILPOINT.with(|active| active.set(Some(point)));
}
#[cfg(all(test, windows))]
fn test_failpoint(point: &str) -> Result<()> {
    FAILPOINT.with(|active| {
        if active.get() == Some(point) {
            active.set(None);
            Err(CoreError::new(
                "injectedFailure",
                format!("Interrupted device record at {point}"),
            ))
        } else {
            Ok(())
        }
    })
}

/// A retained identity is current only when its OS protection, Windows user and
/// original physical Core directory all match. Copies are never silently adopted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentity {
    pub id: String,
    pub is_current: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeviceRecordKind {
    Identity,
    DesktopPreferences,
    Reminders,
    ClientPreferences,
}

impl DeviceRecordKind {
    pub(crate) fn max_bytes(self) -> usize {
        match self {
            Self::Identity => 64 * 1024,
            Self::DesktopPreferences | Self::ClientPreferences => 16 * 1024,
            Self::Reminders => MAX_RECORD_BYTES,
        }
    }
    pub(crate) fn leaf(self) -> &'static str {
        match self {
            Self::Identity => "identity.json",
            Self::DesktopPreferences => "desktop-preferences.json",
            Self::Reminders => "reminders.json",
            Self::ClientPreferences => "client-preferences.json",
        }
    }

    fn from_leaf(leaf: &str) -> Option<Self> {
        [
            Self::Identity,
            Self::DesktopPreferences,
            Self::Reminders,
            Self::ClientPreferences,
        ]
        .into_iter()
        .find(|kind| kind.leaf() == leaf)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IdentityRecord {
    schema_version: u32,
    id: String,
    protection: String,
    sealed_binding: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    schema_version: u32,
    id: String,
    user_sid: String,
    profile: FileIdentity,
}

impl Core {
    /// This identity belongs to this physical Core profile on this Windows
    /// installation. An unreadable/foreign seal returns is_current=false and
    /// keeps the exact original. It never allocates a replacement device ID.
    pub fn device_identity(&mut self) -> Result<DeviceIdentity> {
        if let Some((bytes, _)) = self.read_device_record(DeviceRecordKind::Identity, 64 * 1024)? {
            return self.qualify_device_identity(&bytes);
        }
        let namespace = self
            .device_namespace(true)?
            .ok_or_else(|| evidence("Device namespace is unavailable"))?;
        let (originals, complete) = namespace.directory_entries(MAX_NAMESPACE_MEMBERS)?;
        if !complete || !originals.is_empty() {
            return Err(evidence(
                "Device identity is missing while original device records remain; do not allocate a replacement",
            ));
        }
        let id = fresh();
        let binding = Binding {
            schema_version: 1,
            id: id.clone(),
            user_sid: platform::current_user_sid()?,
            profile: self._data_guard.directory_identity()?,
        };
        let record = IdentityRecord {
            schema_version: 1,
            id: id.clone(),
            protection: "windows-dpapi-local-machine-v1".into(),
            sealed_binding: platform::seal(&serde_json::to_vec(&binding)?)?,
        };
        let bytes = serde_json::to_vec(&record)?;
        self.compare_and_write_device_record(DeviceRecordKind::Identity, None, &bytes)?;
        self.qualify_device_identity(&bytes)
    }

    fn qualify_device_identity(&self, bytes: &[u8]) -> Result<DeviceIdentity> {
        let record = validate_identity_record(bytes)?;
        let Some(plaintext) = platform::unseal(&record.sealed_binding)? else {
            return Ok(DeviceIdentity {
                id: record.id,
                is_current: false,
            });
        };
        let binding: Binding = serde_json::from_slice(&plaintext)
            .map_err(|_| evidence("Device binding plaintext is not a strict record"))?;
        if binding.schema_version != 1 || binding.id != record.id {
            return Err(evidence("Device identity and sealed binding differ"));
        }
        Ok(DeviceIdentity {
            id: record.id,
            is_current: binding.user_sid == platform::current_user_sid()?
                && binding.profile == self._data_guard.directory_identity()?,
        })
    }

    pub(crate) fn read_device_record(
        &self,
        kind: DeviceRecordKind,
        max_bytes: usize,
    ) -> Result<Option<(Vec<u8>, String)>> {
        let Some(_namespace) = self.device_namespace(false)? else {
            return Ok(None);
        };
        let path = self.device_record_path(kind);
        let guard = ParentGuard::acquire(&path)?;
        let mut file = match guard.open_file(false, false) {
            Ok(file) => file,
            Err(error) if error.code == "missing" => return Ok(None),
            Err(error) => return Err(error),
        };
        let limit = max_bytes.min(kind.max_bytes());
        let before = crate::backend::stamp(&file)?;
        if before.size > limit as u64 {
            return Err(evidence("Device record byte budget exceeded"));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit || crate::backend::stamp(&file)? != before {
            return Err(evidence("Device record changed during the bounded read"));
        }
        let digest = hash(&bytes);
        Ok(Some((bytes, digest)))
    }

    /// None creates only an absent canonical record. Some replaces only the
    /// exact original bytes under this Core's exclusive writer lease. Callers
    /// validate their own strict typed schema before invoking this primitive.
    pub(crate) fn compare_and_write_device_record(
        &mut self,
        kind: DeviceRecordKind,
        expected_digest: Option<&str>,
        bytes: &[u8],
    ) -> Result<String> {
        if bytes.len() > kind.max_bytes() {
            return Err(evidence("Device record byte budget exceeded"));
        }
        match kind {
            DeviceRecordKind::Identity => {
                validate_identity_record(bytes)?;
            }
            DeviceRecordKind::DesktopPreferences => {
                crate::desktop_preferences::validate_record(bytes)?
            }
            DeviceRecordKind::Reminders => {
                crate::reminder_device_state::validate_reminder_device_record(bytes)?
            }
            // Client preferences remain owned by their separate typed consumer.
            DeviceRecordKind::ClientPreferences => {}
        }
        if let Some(expected) = expected_digest
            && (expected.len() != 64
                || !expected
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        {
            return Err(evidence("A canonical SHA256 CAS digest is required"));
        }
        let _namespace = self
            .device_namespace(true)?
            .ok_or_else(|| evidence("Device namespace is unavailable"))?;
        let original = self.read_device_record(kind, MAX_RECORD_BYTES)?;
        if original.as_ref().map(|(_, digest)| digest.as_str()) != expected_digest {
            return Err(CoreError::new(
                "deviceStateConflict",
                "Device state changed; retain the original mutation and reread",
            ));
        }
        let path = self.device_record_path(kind);
        // Strictly named, bounded candidates remain as original evidence if
        // publication is uncertain. No delete/retry/rollback fallback occurs.
        let temporary = path.with_file_name(format!(".{}.{}.pending", kind.leaf(), fresh()));
        let guard = ParentGuard::acquire(&temporary)?;
        let mut file = guard.open_file(true, true)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(all(test, windows))]
        test_failpoint("beforePublish")?;
        if expected_digest.is_some() {
            crate::backend::publish_private_replace(&temporary, &path)?;
        } else {
            crate::backend::publish_private_create(&temporary, &path)?;
        }
        #[cfg(all(test, windows))]
        test_failpoint("afterPublish")?;
        let actual = self
            .read_device_record(kind, MAX_RECORD_BYTES)?
            .ok_or_else(|| evidence("Published device record readback is missing"))?;
        if actual.0 != bytes {
            return Err(evidence(
                "Published device record readback differs; outcome is unknown",
            ));
        }
        Ok(actual.1)
    }

    pub(crate) fn device_record_path(&self, kind: DeviceRecordKind) -> PathBuf {
        self.data_dir.join(DEVICE_STATE_ROOT).join(kind.leaf())
    }

    pub(crate) fn device_namespace(&self, create: bool) -> Result<Option<ParentGuard>> {
        let root = self.data_dir.join(DEVICE_STATE_ROOT);
        let prospective = root.join(".namespace-prospective");
        let guard = match ParentGuard::acquire(&prospective) {
            Ok(guard) => guard,
            Err(error) if error.code == "missing" && !create => return Ok(None),
            Err(error) if error.code == "missing" => {
                let parent = ParentGuard::acquire(&root)?;
                let directory = parent.create_directory()?;
                let guard = ParentGuard::acquire(&prospective)?;
                drop(directory);
                guard
            }
            Err(error) => return Err(error),
        };
        let (entries, complete) = guard.directory_entries(MAX_NAMESPACE_MEMBERS)?;
        if !complete {
            return Err(evidence("Device namespace member budget exceeded"));
        }
        for (name, attributes) in entries {
            let name = name
                .to_str()
                .ok_or_else(|| evidence("Non-Unicode device record name"))?;
            if attributes & (0x10 | 0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) != 0
                || device_member_kind(name).is_none()
            {
                return Err(evidence("Unknown or unsupported device namespace member"));
            }
        }
        Ok(Some(guard))
    }
}

/// The closed grammar includes only canonical leaves and our original staged
/// candidates. An arbitrary prefix/suffix is never a blanket exclusion.
pub(crate) fn device_member_kind(name: &str) -> Option<DeviceRecordKind> {
    if let Some(kind) = DeviceRecordKind::from_leaf(name) {
        return Some(kind);
    }
    let candidate = name.strip_prefix('.')?.strip_suffix(".pending")?;
    let (leaf, id) = candidate.rsplit_once('.')?;
    let parsed = uuid::Uuid::parse_str(id).ok()?;
    if parsed.to_string() != id || parsed.get_version_num() != 4 {
        return None;
    }
    DeviceRecordKind::from_leaf(leaf)
}

fn validate_identity_record(bytes: &[u8]) -> Result<IdentityRecord> {
    if bytes.len() > 64 * 1024 {
        return Err(evidence("Device identity byte budget exceeded"));
    }
    let record: IdentityRecord = serde_json::from_slice(bytes)?;
    let id =
        uuid::Uuid::parse_str(&record.id).map_err(|_| evidence("Invalid device identity UUID"))?;
    if record.schema_version != 1
        || id.to_string() != record.id
        || id.get_version_num() != 4
        || record.protection != "windows-dpapi-local-machine-v1"
        || record.sealed_binding.is_empty()
        || record.sealed_binding.len() > 16 * 1024
    {
        return Err(evidence("Unsupported device identity record"));
    }
    Ok(record)
}

/// Only fully implemented strict consumers may classify a device-global record
/// for exclusion. Adding a namespace leaf is not permission to ignore payloads.
pub(crate) fn validate_backup_record(kind: DeviceRecordKind, bytes: &[u8]) -> Result<()> {
    match kind {
        DeviceRecordKind::Identity => validate_identity_record(bytes).map(|_| ()),
        DeviceRecordKind::DesktopPreferences => crate::desktop_preferences::validate_record(bytes),
        DeviceRecordKind::Reminders => {
            crate::reminder_device_state::validate_reminder_device_record(bytes).map_err(|error| {
                evidence(&format!(
                    "Invalid task reminder device record: {}",
                    error.message
                ))
            })
        }
        DeviceRecordKind::ClientPreferences => Err(evidence(
            "This build has no strict validator for this device record type",
        )),
    }
}

pub(crate) fn evidence(message: &str) -> CoreError {
    CoreError::new("deviceStateEvidence", message)
}

#[cfg(windows)]
#[path = "device_state_windows.rs"]
mod platform;

#[cfg(not(windows))]
mod platform {
    use super::*;
    fn unsupported() -> CoreError {
        CoreError::new(
            "unsupportedPlatform",
            "Device binding requires the qualified Windows backend",
        )
    }
    pub(super) fn current_user_sid() -> Result<String> {
        Err(unsupported())
    }
    pub(super) fn seal(_: &[u8]) -> Result<Vec<u8>> {
        Err(unsupported())
    }
    pub(super) fn unseal(_: &[u8]) -> Result<Option<Vec<u8>>> {
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_namespace_does_not_ignore_arbitrary_files_or_aliases() {
        assert_eq!(
            device_member_kind("reminders.json"),
            Some(DeviceRecordKind::Reminders)
        );
        assert_eq!(
            device_member_kind(".identity.json.00000000-0000-4000-8000-000000000000.pending"),
            Some(DeviceRecordKind::Identity)
        );
        for name in [
            "other.json",
            "Identity.json",
            "reminders.json.extra",
            "subdir",
            ".identity.json.BAD.pending",
            ".reminders.json.00000000000000000000000000000000.pending",
        ] {
            assert!(device_member_kind(name).is_none(), "{name}");
        }
    }
}
