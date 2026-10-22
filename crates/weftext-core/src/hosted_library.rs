//! Original actor-record provenance. This is host-selected context, not a grant.
use super::*;
use std::io::{Seek, SeekFrom};

const MAX_LIBRARY_HEADER: usize = 2048;
const MAX_HEADER_PREFIX: usize = 4096;
pub(crate) type MetadataFields = Vec<(String, Vec<u8>)>;

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
pub(crate) fn read_actor_library_header(
    file: &mut (impl Read + Seek),
) -> Result<ActorLibraryHeader> {
    match read_first_metadata(file, "library")? {
        Some(value) => {
            let binding: ActorLibraryBinding = serde_json::from_slice(&value)?;
            binding.validate()?;
            Ok(ActorLibraryHeader::Bound(binding))
        }
        None => Ok(ActorLibraryHeader::LegacyUnbound),
    }
}

pub(crate) fn read_first_metadata(
    file: &mut (impl Read + Seek),
    expected_key: &str,
) -> Result<Option<Vec<u8>>> {
    Ok(
        read_metadata_prefix(file, &[(expected_key, false)], MAX_HEADER_PREFIX)?
            .map(|mut fields| fields.remove(0).1),
    )
}

/// Read only the declared ordered metadata fields. No payload value is skipped
/// or buffered, and the same handle is restored before qualified body reading.
pub(crate) fn read_metadata_prefix(
    file: &mut (impl Read + Seek),
    fields: &[(&str, bool)],
    maximum: usize,
) -> Result<Option<MetadataFields>> {
    let original = file.stream_position()?;
    file.seek(SeekFrom::Start(0))?;
    let result = (|| {
        let mut count = 0usize;
        let mut byte = || -> Result<u8> {
            if count == maximum {
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
        if next_nonspace(&mut byte)? != b'{' {
            return Err(CoreError::new(
                "privateEvidence",
                "Invalid actor metadata header",
            ));
        }
        let mut found = Vec::new();
        let mut pending_key = None;
        for (expected, optional) in fields {
            let key = match pending_key.take() {
                Some(key) => key,
                None => {
                    if !found.is_empty() && next_nonspace(&mut byte)? != b',' {
                        return Err(CoreError::new(
                            "privateEvidence",
                            "Incomplete private metadata prefix",
                        ));
                    }
                    let raw = metadata_value(&mut byte)?;
                    let key: String = serde_json::from_slice(&raw)?;
                    if next_nonspace(&mut byte)? != b':' {
                        return Err(CoreError::new(
                            "privateEvidence",
                            "Invalid private metadata separator",
                        ));
                    }
                    key
                }
            };
            if key != *expected {
                if *optional {
                    pending_key = Some(key);
                    continue;
                }
                if found.is_empty() {
                    return Ok(None);
                }
                return Err(CoreError::new(
                    "unsupportedInputScope",
                    "Private record lacks the required ordered original metadata; original bytes remain unchanged",
                ));
            }
            found.push((key, metadata_value(&mut byte)?));
        }
        Ok(Some(found))
    })();
    file.seek(SeekFrom::Start(original))?;
    result
}

fn metadata_value(next: &mut impl FnMut() -> Result<u8>) -> Result<Vec<u8>> {
    let first = next_nonspace(next)?;
    if !matches!(first, b'{' | b'[' | b'"') {
        return Err(CoreError::new(
            "privateEvidence",
            "Invalid private metadata value",
        ));
    }
    let string = first == b'"';
    let mut value = vec![first];
    let mut depth = usize::from(!string);
    let mut quoted = string;
    let mut escaped = false;
    loop {
        let b = next()?;
        value.push(b);
        if value.len() > MAX_LIBRARY_HEADER {
            return Err(CoreError::new(
                "budget",
                "Private metadata value exceeds its budget",
            ));
        }
        if quoted {
            if b == b'"' && !escaped {
                quoted = false;
                if string {
                    break;
                }
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
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(value)
}

pub(crate) fn decode_metadata<T: serde::de::DeserializeOwned>(fields: MetadataFields) -> Result<T> {
    let mut bytes = vec![b'{'];
    for (index, (key, value)) in fields.into_iter().enumerate() {
        if index != 0 {
            bytes.push(b',');
        }
        bytes.extend(serde_json::to_vec(&key)?);
        bytes.push(b':');
        bytes.extend(value);
    }
    bytes.push(b'}');
    Ok(serde_json::from_slice(&bytes)?)
}

fn next_nonspace(next: &mut impl FnMut() -> Result<u8>) -> Result<u8> {
    loop {
        let b = next()?;
        if !b.is_ascii_whitespace() {
            return Ok(b);
        }
    }
}

/// A foreign classification or qualification error returns before any body
/// read. Production callers retain namespace/read guards around this handle.
pub(crate) fn read_qualified_private_record<R: Read + Seek, T>(
    file: &mut R,
    qualify: impl FnOnce(&mut R) -> Result<Option<(T, usize)>>,
) -> Result<Option<(T, Vec<u8>)>> {
    let Some((metadata, maximum)) = qualify(file)? else {
        return Ok(None);
    };
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(CoreError::new(
            "budget",
            "Qualified private record exceeds its body budget",
        ));
    }
    Ok(Some((metadata, bytes)))
}

impl ActorLibraryBinding {
    pub(super) fn validate(&self) -> Result<()> {
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

    pub(crate) fn validate_unbound_actor_backup_record(
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
        allow_unbound_authority: bool,
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
            self.actor_operation_bytes(&record)?;
            if !matches!(record.scope, ActorInputScope::Qualified { .. }) {
                return Err(CoreError::new(
                    "privateEvidence",
                    "A no-input device fence has no library backup owner",
                ));
            }
            let categories = usize::from(record.field.is_some())
                + usize::from(record.feature.is_some())
                + usize::from(record.definition.is_some())
                + usize::from(record.history.is_some())
                + usize::from(record.owner_backup.is_some());
            let unbound_authority = allow_unbound_authority && record.library.is_none();
            if unbound_authority
                && !matches!(&record.scope, ActorInputScope::Qualified {
                    scope: QualifiedInputScope::Library { binding: original }
                } if Some(&original.workspace_id) == binding.workspace_id.as_ref()
                    && original.root_identity == binding.root_identity)
            {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Unbound actor input has a different original backup owner",
                ));
            }
            if (record.library.as_ref() != Some(binding) && !unbound_authority)
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
            if let Some(plan) = self.operations.get(&record.core_operation_id) {
                if matches!(plan.kind.as_str(), "save" | "saveAs") {
                    self.validate_actor_file_input(&record)?;
                } else if let Some(workspace) = &plan.workspace_id
                    && !matches!(&record.scope, ActorInputScope::Qualified { scope: QualifiedInputScope::Library { binding } } if &binding.workspace_id == workspace)
                {
                    return Err(CoreError::new(
                        "privateEvidence",
                        "Original actor Plan and input owners differ",
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

    pub(crate) fn draft_library_for_root(
        &self,
        root: &Path,
    ) -> Result<Option<ActorLibraryBinding>> {
        if self.actor.is_some() {
            return self.actor_library_for_root(root);
        }
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

    pub(super) fn actor_operation_bytes(&self, record: &ActorOperation) -> Result<Vec<u8>> {
        self.validate_actor_input_record(record)?;
        let bytes = serde_json::to_vec(record)?;
        let metadata = read_actor_operation_metadata(&mut std::io::Cursor::new(&bytes))?;
        if metadata != ActorOperationMetadata::from(record) {
            return Err(CoreError::new(
                "privateEvidence",
                "Actor producer metadata differs",
            ));
        }
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
