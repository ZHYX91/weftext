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
    protection: DeviceProtection,
    sealed_binding: Vec<u8>,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum DeviceProtection {
    #[serde(rename = "windows-dpapi-local-machine-v1")]
    WindowsDpapi,
    #[serde(rename = "linux-controlled-service-hmac-sha256-v1")]
    LinuxControlledService,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "platform",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Binding {
    Windows {
        schema_version: u32,
        id: String,
        user_sid: String,
        profile: FileIdentity,
    },
    Linux {
        schema_version: u32,
        id: String,
        authority: LinuxAuthority,
        profile: FileIdentity,
    },
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LinuxAuthority {
    machine: String,
    enrollment: String,
    service_uid: u32,
    root: FileIdentity,
}

impl Core {
    /// This identity belongs to this physical Core profile and its qualified
    /// platform authority. An unreadable/foreign seal returns is_current=false and
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
        let binding = self.current_device_binding(&id)?;
        let (protection, sealed_binding) =
            self.protect_device_binding(&serde_json::to_vec(&binding)?)?;
        let record = IdentityRecord {
            schema_version: 2,
            id: id.clone(),
            protection,
            sealed_binding,
        };
        let bytes = serde_json::to_vec(&record)?;
        self.compare_and_write_device_record(DeviceRecordKind::Identity, None, &bytes)?;
        self.qualify_device_identity(&bytes)
    }

    /// Reads and qualifies the original identity without creating a namespace,
    /// identity, ledger or replacement device ID.
    pub fn existing_device_identity(&self) -> Result<DeviceIdentity> {
        let (bytes, _) = self
            .read_device_record(DeviceRecordKind::Identity, 64 * 1024)?
            .ok_or_else(|| evidence("Existing device identity is unavailable"))?;
        self.qualify_device_identity(&bytes)
    }

    fn qualify_device_identity(&self, bytes: &[u8]) -> Result<DeviceIdentity> {
        let record = validate_identity_record(bytes)?;
        let Some(plaintext) =
            self.unprotect_device_binding(record.protection, &record.sealed_binding)?
        else {
            return Ok(DeviceIdentity {
                id: record.id,
                is_current: false,
            });
        };
        let binding: Binding = serde_json::from_slice(&plaintext).map_err(|_| {
            evidence("Device binding plaintext is not a strict current physical record")
        })?;
        let expected = self.current_device_binding(&record.id)?;
        Ok(DeviceIdentity {
            id: record.id,
            is_current: binding == expected,
        })
    }

    fn current_device_binding(&self, id: &str) -> Result<Binding> {
        let profile = self._data_guard.directory_identity()?;
        #[cfg(windows)]
        {
            Ok(Binding::Windows {
                schema_version: 2,
                id: id.to_owned(),
                user_sid: platform::current_user_sid()?,
                profile,
            })
        }
        #[cfg(target_os = "linux")]
        {
            let actual = self.backend.linux_device_authority()?;
            Ok(Binding::Linux {
                schema_version: 2,
                id: id.to_owned(),
                authority: LinuxAuthority {
                    machine: actual.machine,
                    enrollment: actual.enrollment,
                    service_uid: actual.service_uid,
                    root: actual.root,
                },
                profile,
            })
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = (id, profile);
            Err(evidence(
                "Device binding requires a qualified platform authority",
            ))
        }
    }
    fn protect_device_binding(&self, bytes: &[u8]) -> Result<(DeviceProtection, Vec<u8>)> {
        #[cfg(windows)]
        {
            Ok((DeviceProtection::WindowsDpapi, platform::seal(bytes)?))
        }
        #[cfg(target_os = "linux")]
        {
            Ok((
                DeviceProtection::LinuxControlledService,
                self.backend.seal_linux_device(bytes)?,
            ))
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = bytes;
            Err(evidence(
                "Device protection is unavailable on this platform",
            ))
        }
    }
    fn unprotect_device_binding(
        &self,
        protection: DeviceProtection,
        bytes: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        match protection {
            DeviceProtection::WindowsDpapi => {
                #[cfg(windows)]
                {
                    platform::unseal(bytes)
                }
                #[cfg(not(windows))]
                {
                    let _ = bytes;
                    Ok(None)
                }
            }
            DeviceProtection::LinuxControlledService => {
                #[cfg(target_os = "linux")]
                {
                    self.backend.unseal_linux_device(bytes)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = bytes;
                    Ok(None)
                }
            }
        }
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
        let guard = ParentGuard::acquire(&self.backend, &path)?;
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
            // Their owners perform strict typed validation before this shared
            // primitive. Backup will reject them until their real validator is linked.
            DeviceRecordKind::Reminders | DeviceRecordKind::ClientPreferences => {}
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
        let guard = ParentGuard::acquire(&self.backend, &temporary)?;
        let mut file = guard.open_file(true, true)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        if expected_digest.is_some() {
            crate::backend::publish_private_replace(&self.backend, &temporary, &path)?;
        } else {
            crate::backend::publish_private_create(&self.backend, &temporary, &path)?;
        }
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
        let guard = match ParentGuard::acquire(&self.backend, &prospective) {
            Ok(guard) => guard,
            Err(error) if error.code == "missing" && !create => return Ok(None),
            Err(error) if error.code == "missing" => {
                let parent = ParentGuard::acquire(&self.backend, &root)?;
                let directory = parent.create_directory()?;
                let guard = ParentGuard::acquire(&self.backend, &prospective)?;
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
            if !attributes.is_file() || device_member_kind(name).is_none() {
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
    if record.schema_version != 2
        || id.to_string() != record.id
        || id.get_version_num() != 4
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
        DeviceRecordKind::Reminders | DeviceRecordKind::ClientPreferences => Err(evidence(
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn device_v2_rejects_old_physical_shell_and_unknown_providers() {
        let id = fresh();
        for original in [
            serde_json::json!({"schemaVersion":1,"id":id,"protection":"windows-dpapi-local-machine-v1","sealedBinding":[1]}),
            serde_json::json!({"schemaVersion":2,"id":id,"protection":"unknown-provider","sealedBinding":[1]}),
        ] {
            let bytes = serde_json::to_vec(&original).unwrap();
            assert!(validate_identity_record(&bytes).is_err());
        }
        let foreign = serde_json::json!({"schemaVersion":2,"id":id,"protection":"windows-dpapi-local-machine-v1","sealedBinding":[1]});
        assert!(validate_identity_record(&serde_json::to_vec(&foreign).unwrap()).is_ok());
    }
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
