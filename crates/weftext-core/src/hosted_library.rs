//! Original actor-record provenance. This is host-selected context, not a grant.
use super::*;
use std::io::{Seek, SeekFrom};

const MAX_LIBRARY_HEADER: usize = 2048;
const MAX_HEADER_PREFIX: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActorLibraryBinding {
    pub root: String,
    pub root_identity: FileIdentity,
    pub workspace_id: Option<String>,
}

pub(crate) enum ActorLibraryHeader {
    Bound(ActorLibraryBinding),
    LegacyUnbound,
}

/// Read only the first metadata value, with no buffering or payload skipping.
/// New scoped records put library first; old records remain unscoped. The same
/// protected file handle must be retained by the caller through classification
/// and any subsequent full capture. Restore its original position on all paths.
pub(crate) fn read_actor_library_header(file: &mut File) -> Result<ActorLibraryHeader> {
    let original = file.stream_position()?;
    file.seek(SeekFrom::Start(0))?;
    let result = (|| {
        let mut count = 0usize;
        let mut byte = || -> Result<u8> {
            if count == MAX_HEADER_PREFIX {
                return Err(CoreError::new(
                    "budget",
                    "Actor library header exceeds its budget",
                ));
            }
            count += 1;
            let mut b = [0];
            file.read_exact(&mut b)
                .map_err(|_| CoreError::new("privateEvidence", "Truncated actor metadata"))?;
            Ok(b[0])
        };
        let first = next_nonspace(&mut byte)?;
        if first != b'{' || next_nonspace(&mut byte)? != b'"' {
            return Err(CoreError::new(
                "privateEvidence",
                "Invalid actor metadata header",
            ));
        }
        let mut key = vec![b'"'];
        let mut escaped = false;
        loop {
            let b = byte()?;
            key.push(b);
            if b == b'"' && !escaped {
                break;
            }
            escaped = b == b'\\' && !escaped;
            if b != b'\\' {
                escaped = false;
            }
        }
        let key: String = serde_json::from_slice(&key)?;
        if next_nonspace(&mut byte)? != b':' {
            return Err(CoreError::new(
                "privateEvidence",
                "Invalid actor metadata separator",
            ));
        }
        if key != "library" {
            return Ok(ActorLibraryHeader::LegacyUnbound);
        }
        let first = next_nonspace(&mut byte)?;
        if first != b'{' {
            return Err(CoreError::new(
                "privateEvidence",
                "Invalid scoped actor metadata",
            ));
        }
        let mut value = vec![first];
        let mut depth = 1usize;
        let mut quoted = false;
        let mut escaped = false;
        while depth != 0 {
            let b = byte()?;
            value.push(b);
            if value.len() > MAX_LIBRARY_HEADER {
                return Err(CoreError::new(
                    "budget",
                    "Actor library metadata exceeds its budget",
                ));
            }
            if quoted {
                if b == b'"' && !escaped {
                    quoted = false;
                }
                escaped = b == b'\\' && !escaped;
                if b != b'\\' {
                    escaped = false;
                }
            } else {
                match b {
                    b'"' => {
                        quoted = true;
                        escaped = false;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => depth -= 1,
                    _ => {}
                }
            }
        }
        let binding: ActorLibraryBinding = serde_json::from_slice(&value)?;
        binding.validate()?;
        Ok(ActorLibraryHeader::Bound(binding))
    })();
    file.seek(SeekFrom::Start(original))?;
    result
}

fn next_nonspace(next: &mut impl FnMut() -> Result<u8>) -> Result<u8> {
    loop {
        let b = next()?;
        if !b.is_ascii_whitespace() {
            return Ok(b);
        }
    }
}

impl ActorLibraryBinding {
    fn validate(&self) -> Result<()> {
        if !Path::new(&self.root).is_absolute()
            || self
                .workspace_id
                .as_ref()
                .is_some_and(|id| Uuid::parse_str(id).is_err())
            || serde_json::to_vec(self)?.len() > MAX_LIBRARY_HEADER
        {
            return Err(CoreError::new(
                "privateEvidence",
                "Invalid original actor library binding",
            ));
        }
        Ok(())
    }
}

impl Core {
    /// Validate only a selected, already guarded actor record. The caller first
    /// classifies its bounded library header; this never opens a foreign root or
    /// parses foreign actor payloads in order to discover a grant.
    pub(crate) fn validate_scoped_actor_record(
        &self,
        path: &Path,
        bytes: &[u8],
        binding: &ActorLibraryBinding,
    ) -> Result<()> {
        self.validate_actor_backup_record(path, bytes, binding, false)
    }

    pub(crate) fn validate_legacy_actor_backup_record(
        &self,
        path: &Path,
        bytes: &[u8],
        binding: &ActorLibraryBinding,
    ) -> Result<()> {
        self.validate_actor_backup_record(path, bytes, binding, true)
    }

    fn validate_actor_backup_record(
        &self,
        path: &Path,
        bytes: &[u8],
        binding: &ActorLibraryBinding,
        allow_legacy: bool,
    ) -> Result<()> {
        let relative = crate::history::history_relative_path(path, &self.data_dir.join("actors"))
            .ok_or_else(|| {
            CoreError::new("privateEvidence", "Actor record is outside private scope")
        })?;
        let parts = relative
            .components()
            .map(|part| part.as_os_str().to_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| CoreError::new("privateEvidence", "Invalid actor namespace"))?;
        if !matches!(parts.len(), 3 | 4)
            || parts[1] != "operations"
            || (parts.len() == 4 && parts[2] != "outputs")
        {
            return Err(CoreError::new(
                "privateEvidence",
                "Unknown actor record namespace",
            ));
        }
        let name = parts.last().unwrap().strip_suffix(".json").ok_or_else(|| {
            CoreError::new("privateEvidence", "Invalid original operation filename")
        })?;
        valid_operation_id(name)?;
        let actor = if parts.len() == 4 {
            if bytes.len() > 4096 {
                return Err(CoreError::new(
                    "budget",
                    "Output claim exceeds its read budget",
                ));
            }
            let claim: feature::OutputClaim = serde_json::from_slice(bytes)?;
            if claim.library.as_ref() != Some(binding) {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Output scope differs from its header",
                ));
            }
            feature::valid_fingerprint(&claim.fingerprint)?;
            claim.actor
        } else {
            let record: ActorOperation = serde_json::from_slice(bytes)?;
            Self::actor_operation_bytes(&record)?;
            let categories = usize::from(record.field.is_some())
                + usize::from(record.feature.is_some())
                + usize::from(record.definition.is_some())
                + usize::from(record.history.is_some())
                + usize::from(record.owner_backup.is_some());
            let legacy = allow_legacy && record.library.is_none();
            if (record.library.as_ref() != Some(binding) && !legacy)
                || categories > 1
                || (categories != 0 && !record.save_definition_reservations.is_empty())
            {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Original actor record categories or scope differ",
                ));
            }
            valid_operation_id(&record.core_operation_id)?;
            feature::valid_resource(&record.resource)?;
            if !(record.cancelled && record.fingerprint.is_empty()) {
                feature::valid_fingerprint(&record.fingerprint)?;
            }
            if let Some(plan) = self.operations.get(&record.core_operation_id)
                && plan.workspace_id.is_some()
                && plan.workspace_id != binding.workspace_id
            {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Original actor plan belongs to another library",
                ));
            }
            if legacy {
                let plan = self
                    .operations
                    .get(&record.core_operation_id)
                    .ok_or_else(|| {
                        CoreError::new(
                            "privateEvidence",
                            "Legacy actor record has no original Plan",
                        )
                    })?;
                let exact_workspace =
                    binding.workspace_id.is_some() && plan.workspace_id == binding.workspace_id;
                let exact_birth = plan
                    .identity
                    .as_ref()
                    .and_then(|identity| self.admitted_binding(identity))
                    .is_some_and(|birth| {
                        Some(&birth.workspace_id) == binding.workspace_id.as_ref()
                    });
                let exact_original_path = binding.workspace_id.as_ref().is_some_and(|workspace| {
                    self.operations.values().any(|original| {
                        self.committed(&original.operation_id)
                            && matches!(original.kind.as_str(), "createLibrary" | "cloneSync")
                            && original.workspace_id.as_ref() == Some(workspace)
                            && crate::history::history_relative_path(
                                Path::new(&plan.path),
                                Path::new(&original.path),
                            )
                            .is_some()
                    })
                });
                if !exact_workspace && !exact_birth && !exact_original_path {
                    return Err(CoreError::new(
                        "privateEvidence",
                        "Legacy original Plan does not prove this library",
                    ));
                }
            }
            record.actor
        };
        if actor.is_empty() || actor.len() > 256 || hash(actor.as_bytes()) != parts[0] {
            return Err(CoreError::new(
                "privateEvidence",
                "Actor namespace hash differs from its original record",
            ));
        }
        Ok(())
    }

    pub(crate) fn draft_session_library(
        &self,
        session_id: &str,
    ) -> Result<Option<ActorLibraryBinding>> {
        if self.actor.is_some() {
            return self.actor_library_for_session(session_id);
        }
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?;
        let root = &session.root;
        let guard = ParentGuard::acquire(&root.join(".draft-root-prospective"))?;
        let workspace = self.workspace(root)?;
        if workspace.is_none()
            && [
                ".weftext-workspace.json",
                ".weftext-format",
                ".weftext-mvp-commit.json",
            ]
            .iter()
            .any(|name| root.join(name).exists())
        {
            // Preserve local ordinary-file behavior, without claiming that
            // damaged/unadmitted controls are an ordinary qualified library.
            return Ok(None);
        }
        let binding = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: workspace.map(|w| w.workspace_id),
        };
        binding.validate()?;
        Ok(Some(binding))
    }

    /// Trusted Server entry point. The Server passes its configured root, never
    /// a client-selected path. Root namespace protection survives the closure;
    /// file qualification still belongs to each original guarded operation.
    pub fn with_actor_for_root<T>(
        &mut self,
        actor: &str,
        root: &Path,
        action: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        if self.actor.is_some() || self.actor_library.is_some() {
            return Err(CoreError::new(
                "invalidActor",
                "Actor scopes cannot be nested",
            ));
        }
        let root = absolute_path(root)?;
        let guard = ParentGuard::acquire(&root.join(".actor-root-prospective"))?;
        let workspace = self.workspace(&root)?;
        if workspace.is_none()
            && [
                ".weftext-workspace.json",
                ".weftext-format",
                ".weftext-mvp-commit.json",
            ]
            .iter()
            .any(|name| root.join(name).exists())
        {
            return Err(CoreError::new(
                "unadmittedLibrary",
                "Existing library controls are not qualified",
            ));
        }
        let binding = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: workspace.map(|w| w.workspace_id),
        };
        binding.validate()?;
        self.with_actor(actor, |core| {
            core.actor_library_path = Some(root);
            core.actor_library = Some(binding);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action(core)));
            core.actor_library = None;
            core.actor_library_path = None;
            match result {
                Ok(result) => result,
                Err(panic) => {
                    core.actor = None;
                    std::panic::resume_unwind(panic)
                }
            }
        })
    }

    pub(crate) fn actor_library_for_root(
        &self,
        root: &Path,
    ) -> Result<Option<ActorLibraryBinding>> {
        let Some(binding) = &self.actor_library else {
            return Ok(None);
        };
        let root = absolute_path(root)?;
        let guard = ParentGuard::acquire(&root.join(".actor-root-prospective"))?;
        if guard.directory_identity()? != binding.root_identity
            || self.workspace(&root)?.map(|w| w.workspace_id) != binding.workspace_id
        {
            return Err(CoreError::new(
                "operationConflict",
                "The trusted actor library differs",
            ));
        }
        Ok(Some(binding.clone()))
    }

    pub(crate) fn actor_library_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<ActorLibraryBinding>> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?;
        self.actor_library_for_root(&session.root)
    }

    pub(crate) fn check_actor_library(&self, library: &Option<ActorLibraryBinding>) -> Result<()> {
        if let (Some(original), Some(current)) = (library, &self.actor_library)
            && original != current
        {
            return Err(CoreError::new(
                "operationConflict",
                "The original actor library differs",
            ));
        }
        Ok(())
    }

    pub(super) fn actor_operation_bytes(record: &ActorOperation) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(record)?;
        let maximum = if record.history.is_some() {
            16 * 1024 * 1024
        } else if record.definition.is_some() {
            2 * MAX_SOURCE_BYTES + 64 * 1024
        } else {
            MAX_SOURCE_BYTES + 4096
        };
        if bytes.len() > maximum {
            return Err(CoreError::new(
                "budget",
                "Actor operation exceeds its durable read budget",
            ));
        }
        Ok(bytes)
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_library_tests.rs"]
mod tests;
