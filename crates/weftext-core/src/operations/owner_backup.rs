//! Scope-first capture of the original private recovery records. The only
//! persisted routing witness is one digest in the original frozen operation.
use super::*;
use crate::capture_draft::{self, CaptureLibrary};
use crate::hosted::{ActorLibraryBinding, ActorLibraryHeader, read_actor_library_header};
use crate::journal::Plan;
use std::io::{Seek, SeekFrom};

const MAX_ROUTING_MEMBERS: usize = 5000;
const PRIVATE_ROOTS: [&str; 7] = [
    "journal",
    "drafts",
    "draft-history",
    "actors",
    "sync",
    "capture-drafts",
    crate::device_state::DEVICE_STATE_ROOT,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum RecordKind {
    Journal,
    Draft,
    Actor,
    Output,
    Capture,
}
fn record_kind(core: &Core, path: &Path) -> Result<RecordKind> {
    let relative = path
        .strip_prefix(&core.data_dir)
        .map_err(|_| private_evidence())?;
    let parts = relative
        .components()
        .map(|p| p.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(private_evidence)?;
    match parts.as_slice() {
        ["journal", _] => Ok(RecordKind::Journal),
        ["drafts" | "draft-history", _] => Ok(RecordKind::Draft),
        ["actors", _, "drafts" | "draft-history", _] => Ok(RecordKind::Draft),
        ["actors", _, "operations", _] => Ok(RecordKind::Actor),
        ["actors", _, "operations", "outputs", _] => Ok(RecordKind::Output),
        ["capture-drafts", _] | ["actors", _, "capture-drafts", _] => Ok(RecordKind::Capture),
        _ => Err(private_evidence()),
    }
}

pub(crate) struct PrivateCapture {
    pub digest: String,
    selected_files: Vec<(PathBuf, backend::BoundFile)>,
    pub guards: Vec<ParentGuard>,
    pub handles: Vec<backend::BoundFile>,
}

#[derive(Serialize)]
struct DirectoryRoute {
    path: String,
    identity: FileIdentity,
    entries: Vec<String>,
}
#[derive(Serialize)]
struct FileRoute {
    path: String,
    attributes: backend::EntryMetadata,
    stamp: FileStamp,
    route: String,
}

fn selected_attempt_scope(
    scope: Option<&ActorLibraryBinding>,
    expected: &ActorLibraryBinding,
) -> Result<bool> {
    let Some(scope) = scope else {
        return Ok(false);
    };
    if scope == expected {
        return Ok(true);
    }
    if scope.workspace_id == expected.workspace_id
        || scope.root.eq_ignore_ascii_case(&expected.root)
        || scope.root_identity == expected.root_identity
    {
        return Err(CoreError::new(
            "scopeChanged",
            "Original reservation library identity differs from this backup",
        ));
    }
    Ok(false)
}
fn qualified_journal_suffix<'a>(id: &str, name: &'a str) -> Result<&'a str> {
    let suffix = name
        .strip_prefix(&format!("{id}."))
        .and_then(|name| name.strip_suffix(".json"))
        .ok_or_else(private_evidence)?;
    let fixed = matches!(
        suffix,
        "plan"
            | "install"
            | "commit"
            | "resolution"
            | "birth"
            | "births"
            | "observations"
            | "resource-births"
            | "definition-reservation"
            | "definition-reservations"
            | "revision-reservation"
            | "tag-reservation"
            | "directory-reservation"
    );
    let object = suffix
        .strip_prefix("object-")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()));
    let recovery = suffix
        .strip_prefix("recovery-observations-")
        .is_some_and(|id| valid_operation_id(id).is_ok());
    if !fixed && !object && !recovery {
        return Err(private_evidence());
    }
    Ok(suffix)
}
fn route_device_state(
    core: &Core,
    digest: &mut Sha256,
    capture: &mut PrivateCapture,
) -> Result<()> {
    let guard = core.device_namespace(false)?.ok_or_else(private_evidence)?;
    let folder = core.data_dir.join(crate::device_state::DEVICE_STATE_ROOT);
    let (mut entries, complete) = guard.directory_entries(128)?;
    if !complete {
        return Err(backup_budget());
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut inspected_bytes = 0u64;
    for (name, attrs) in &entries {
        let kind = name
            .to_str()
            .and_then(crate::device_state::device_member_kind)
            .ok_or_else(private_evidence)?;
        if !attrs.is_file() {
            return Err(private_evidence());
        }
        let path = folder.join(name);
        let file_guard = ParentGuard::acquire(&core.backend, &path)?;
        let mut file = file_guard.open_read_guard_file()?;
        let actual = stamp(&file)?;
        inspected_bytes = inspected_bytes
            .checked_add(actual.size)
            .filter(|size| *size <= 32 * 1024 * 1024)
            .ok_or_else(backup_budget)?;
        if actual.size > kind.max_bytes() as u64 {
            return Err(backup_budget());
        }
        let mut bytes = Vec::new();
        std::io::Read::by_ref(&mut file)
            .take(kind.max_bytes() as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > kind.max_bytes() || stamp(&file)? != actual {
            return Err(private_evidence());
        }
        crate::device_state::validate_backup_record(kind, &bytes)?;
        // Keep the same handles used for validation; no device bytes become
        // CapturedFile, selected input, or an author's exported backup payload.
        capture.guards.push(file_guard);
        capture.handles.push(file);
    }
    // Device-global content is excluded, so a resident setting or reminder
    // ledger update must not invalidate an author's frozen library backup.
    // Revalidate the closed namespace and strict records above on every pass;
    // bind only this physical root and its explicit non-library classification.
    digest_json(
        digest,
        &(
            display(&folder),
            guard.directory_identity()?,
            "deviceGlobal:v1",
        ),
    )?;
    capture.guards.push(guard);
    Ok(())
}

impl Core {
    pub(super) fn capture_owner_backup_private(
        &self,
        root: &Path,
        workspace_id: &str,
        files: &mut Vec<CapturedFile>,
        directories: &mut Vec<CapturedDirectory>,
    ) -> Result<PrivateCapture> {
        let mut capture =
            self.route_owner_backup_private(root, workspace_id, None, files, false)?;
        let mut total = files.iter().map(|f| f.bytes.len()).sum::<usize>();
        // Selected records were read from the very handle used to classify
        // their header; retain them until the complete capture is formed.
        for (path, handle) in std::mem::take(&mut capture.selected_files) {
            let mut handle = handle;
            let actual = stamp(&handle)?;
            if actual.size > MAX_OPERATION_BYTES.saturating_sub(total) as u64 {
                return Err(backup_budget());
            }
            let bytes = read_blob(&mut handle)?;
            validate_selected_record(self, &path, &bytes, root, workspace_id)?;
            total += bytes.len();
            files.push(CapturedFile {
                path: display(&path),
                bytes,
                stamp: actual,
            });
            if files.len() > MAX_OPERATION_FILES {
                return Err(backup_budget());
            }
        }
        let sync = self.data_dir.join("sync").join(workspace_id);
        if self.backend.entry_exists(&sync)? {
            snapshot_tree(&self.backend, &sync, files, directories, false)?;
            // snapshot_tree records selected actual empty directories as well.
        }
        capture.guards.clear();
        capture.handles.clear();
        Ok(capture)
    }

    pub(crate) fn validate_owner_backup_private(
        &self,
        plan: &Plan,
        protected_inputs: bool,
    ) -> Result<Option<PrivateCapture>> {
        let Some(t) = &plan.transaction else {
            return Ok(None);
        };
        if !matches!(t.request.intent, OperationIntent::BackupLibrary { .. }) {
            return Ok(None);
        }
        let expected = t.backup_private_digest.as_ref().ok_or_else(|| {
            CoreError::new(
                "privateEvidence",
                "This legacy backup has no complete private routing witness",
            )
        })?;
        let workspace = self.require_workspace(Path::new(&t.request.root))?;
        let mut capture = self.route_owner_backup_private(
            Path::new(&t.request.root),
            &workspace.workspace_id,
            Some(plan),
            &t.files,
            protected_inputs,
        )?;
        if &capture.digest != expected {
            return Err(CoreError::new(
                "scopeChanged",
                "Original private recovery membership or routing changed",
            ));
        }
        // The normal transaction reader immediately acquires the exact
        // CapturedFiles; retaining these read handles would self-conflict.
        capture.selected_files.clear();
        Ok(Some(capture))
    }

    fn route_owner_backup_private(
        &self,
        root: &Path,
        workspace_id: &str,
        own: Option<&Plan>,
        selected_inputs: &[CapturedFile],
        protected_inputs: bool,
    ) -> Result<PrivateCapture> {
        let root_guard =
            ParentGuard::acquire(&self.backend, &root.join(".backup-root-prospective"))?;
        let binding = ActorLibraryBinding {
            root: display(&root_guard.directory_coordinates()?),
            root_identity: root_guard.directory_identity()?,
            workspace_id: Some(workspace_id.into()),
        };
        let mut digest = Sha256::new();
        let mut selected_bytes = selected_inputs
            .iter()
            .filter(|f| !Path::new(&f.path).starts_with(&self.data_dir))
            .map(|f| f.bytes.len())
            .sum::<usize>();
        // Malformed selected or foreign decisions are retained errors, never
        // silently treated as an absence or assigned to a different library.
        for plan in self.operations.values() {
            self.resolution_record(&plan.operation_id)?;
        }
        let mut selected_attempts = Vec::new();
        for entry in self.backend.read_dir(self.data_dir.join("journal"))? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(id) = name.strip_suffix(".resolution.json")
                && !self.operations.contains_key(id)
            {
                let scope = self.unregistered_attempt_library(
                    id,
                    false,
                    if protected_inputs {
                        selected_inputs
                    } else {
                        &[]
                    },
                )?;
                if selected_attempt_scope(scope.as_ref(), &binding)? {
                    selected_attempts.push(id.to_owned());
                }
            }
        }
        let selected_plans = self
            .operations
            .values()
            .filter(|p| {
                !library_creation::is_cancellation(p)
                    && own.is_none_or(|own| p.operation_id != own.operation_id)
                    && (p.workspace_id.as_deref() == Some(workspace_id)
                        || locator_within(Path::new(&p.path), root))
            })
            .map(|p| p.operation_id.as_str())
            .chain(selected_attempts.iter().map(String::as_str))
            .collect::<BTreeSet<_>>();
        let mut capture = PrivateCapture {
            digest: String::new(),
            selected_files: Vec::new(),
            guards: vec![root_guard],
            handles: Vec::new(),
        };
        let base_guard = ParentGuard::acquire(
            &self.backend,
            &self.data_dir.join(".backup-private-prospective"),
        )?;
        let (base_entries, complete) = base_guard.directory_entries(MAX_ROUTING_MEMBERS)?;
        if !complete {
            return Err(backup_budget());
        }
        digest_json(&mut digest, &base_guard.directory_identity()?)?;
        // Credentials, policy, instance lock and unrelated roots are not backup
        // input. Device-global state has its own closed, strictly validated
        // route and is never selected as author-library backup payload.
        let mut base_names = base_entries
            .into_iter()
            .filter(|(name, _)| {
                PRIVATE_ROOTS
                    .iter()
                    .any(|p| name.to_string_lossy().eq_ignore_ascii_case(p))
            })
            .collect::<Vec<_>>();
        base_names.sort_by(|a, b| a.0.cmp(&b.0));
        digest_json(
            &mut digest,
            &sorted_entries(base_names.clone(), &self.data_dir),
        )?;
        capture.guards.push(base_guard);
        let mut queue = Vec::new();
        for (name, attrs) in base_names {
            if !PRIVATE_ROOTS.iter().any(|p| name == *p) {
                return Err(private_evidence());
            }
            if attrs.is_unsupported() || !attrs.is_directory() {
                return Err(private_evidence());
            }
            let path = self.data_dir.join(&name);
            if name == crate::device_state::DEVICE_STATE_ROOT {
                route_device_state(self, &mut digest, &mut capture)?;
            } else if name == "sync" {
                // Its other workspace bodies are never opened. Fix the selected
                // workspace's presence and parent identity; its subtree uses
                // the existing complete CapturedDirectory/CapturedFile path.
                let guard =
                    ParentGuard::acquire(&self.backend, &path.join(".backup-sync-prospective"))?;
                let (entries, complete) = guard.directory_entries(MAX_ROUTING_MEMBERS)?;
                if !complete {
                    return Err(backup_budget());
                }
                let selected = entries
                    .into_iter()
                    .filter(|(n, _)| n == workspace_id)
                    .collect::<Vec<_>>();
                digest_json(
                    &mut digest,
                    &DirectoryRoute {
                        path: display(&path),
                        identity: guard.directory_identity()?,
                        entries: sorted_entries(selected, &path),
                    },
                )?;
                capture.guards.push(guard);
            } else {
                queue.push(path);
            }
        }
        let mut members = 0usize;
        while let Some(folder) = queue.pop() {
            members += 1;
            if members > MAX_ROUTING_MEMBERS {
                return Err(backup_budget());
            }
            let guard =
                ParentGuard::acquire(&self.backend, &folder.join(".backup-private-prospective"))?;
            let own_limit = own.map_or(0, own_suffix_limit);
            let (mut entries, complete) =
                guard.directory_entries(MAX_ROUTING_MEMBERS + own_limit)?;
            if !complete {
                return Err(backup_budget());
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut routed_entries = Vec::new();
            for (name, attrs) in entries {
                let path = folder.join(&name);
                if attrs.is_unsupported() {
                    return Err(private_evidence());
                }
                if folder == self.data_dir.join("journal")
                    && own.is_some_and(|p| {
                        name.to_string_lossy()
                            .starts_with(&format!("{}.", p.operation_id))
                    })
                {
                    let own = own.unwrap();
                    let (own_guard, own_file) =
                        validate_own_suffix(&self.backend, own, &path, attrs)?;
                    capture.guards.push(own_guard);
                    capture.handles.push(own_file);
                    continue;
                }
                members += 1;
                if members > MAX_ROUTING_MEMBERS {
                    return Err(backup_budget());
                }
                routed_entries.push((name.clone(), attrs));
                if attrs.is_directory() {
                    if !path.starts_with(self.data_dir.join("actors")) {
                        return Err(private_evidence());
                    }
                    queue.push(path);
                    continue;
                }
                record_kind(self, &path)?;
                let protected = protected_inputs
                    .then(|| selected_inputs.iter().find(|f| Path::new(&f.path) == path))
                    .flatten();
                let (actual, route, selected, handle) = if let Some(original) = protected {
                    // The caller holds the original exact-byte handle. Reopening
                    // its share-zero leaf would self-conflict; use that proven
                    // CapturedFile while checking the current parent namespace.
                    let route = route_selected_bytes(
                        self,
                        &path,
                        &original.bytes,
                        root,
                        &binding,
                        &selected_plans,
                    )?;
                    (original.stamp.clone(), route, true, None)
                } else {
                    let file_guard = ParentGuard::acquire(&self.backend, &path)?;
                    let mut file = file_guard.open_read_guard_file()?;
                    let actual = stamp(&file)?;
                    let (selected, route) = if folder == self.data_dir.join("journal") {
                        let text = name.to_string_lossy();
                        let id = text.split('.').next().ok_or_else(private_evidence)?;
                        let suffix = qualified_journal_suffix(id, &text)?;
                        if !self.operations.contains_key(id) {
                            let scope = self.unregistered_attempt_library(
                                id,
                                false,
                                if protected_inputs {
                                    selected_inputs
                                } else {
                                    &[]
                                },
                            )?;
                            let selected = selected_attempt_scope(scope.as_ref(), &binding)?;
                            let mut decision = self
                                .backend
                                .open_read(self.journal_path(id, "resolution"))?;
                            let mut bytes = Vec::new();
                            Read::by_ref(&mut decision)
                                .take(4097)
                                .read_to_end(&mut bytes)?;
                            let record = journal::decode_resolution(id, &bytes)?;
                            if record.action != "cancelPrepared" {
                                return Err(private_evidence());
                            }
                            if suffix == "resolution" && !selected {
                                digest_json(&mut digest, &hash(&bytes))?;
                            } else if suffix != "resolution"
                                && !matches!(
                                    suffix,
                                    "definition-reservation"
                                        | "revision-reservation"
                                        | "tag-reservation"
                                        | "directory-reservation"
                                )
                            {
                                return Err(private_evidence());
                            }
                            (
                                selected,
                                if selected {
                                    "selectedJournal".into()
                                } else {
                                    "deviceOrForeignAttempt".into()
                                },
                            )
                        } else {
                            let selected = selected_plans
                                .iter()
                                .any(|id| name.to_string_lossy().starts_with(&format!("{id}.")));
                            (
                                selected,
                                if selected {
                                    "selectedJournal".into()
                                } else {
                                    "foreignJournal".into()
                                },
                            )
                        }
                    } else {
                        classify_record(
                            self,
                            &path,
                            &mut file,
                            root,
                            &binding,
                            MAX_OPERATION_BYTES.saturating_sub(selected_bytes),
                            actual.size,
                        )?
                    };
                    capture.guards.push(file_guard);
                    (actual, route, selected, Some(file))
                };
                digest_json(
                    &mut digest,
                    &FileRoute {
                        path: display(&path),
                        attributes: attrs,
                        stamp: actual.clone(),
                        route,
                    },
                )?;
                if selected {
                    selected_bytes = selected_bytes
                        .checked_add(usize::try_from(actual.size).map_err(|_| backup_budget())?)
                        .filter(|total| *total <= MAX_OPERATION_BYTES)
                        .ok_or_else(backup_budget)?;
                }
                if let Some(handle) = handle {
                    if selected {
                        capture.selected_files.push((path, handle));
                    } else {
                        capture.handles.push(handle);
                    }
                }
            }
            digest_json(
                &mut digest,
                &DirectoryRoute {
                    path: display(&folder),
                    identity: guard.directory_identity()?,
                    entries: sorted_entries(routed_entries, &folder),
                },
            )?;
            capture.guards.push(guard);
        }
        // Paths are temporary capture bookkeeping, not persisted routing facts.
        capture.digest = format!("{:x}", digest.finalize());
        Ok(capture)
    }
}

fn digest_json(digest: &mut Sha256, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    Ok(())
}
fn backup_budget() -> CoreError {
    CoreError::new(
        "budget",
        "Complete library backup exceeds 64 MiB or 5000 members",
    )
}
fn private_evidence() -> CoreError {
    CoreError::new(
        "privateEvidence",
        "Original private evidence cannot be scoped completely without reading another library",
    )
}

fn classify_record(
    core: &Core,
    path: &Path,
    file: &mut backend::BoundFile,
    root: &Path,
    expected: &ActorLibraryBinding,
    remaining: usize,
    size: u64,
) -> Result<(bool, String)> {
    if record_kind(core, path)? == RecordKind::Capture {
        let header = capture_draft::read_capture_header(file)?;
        capture_draft::validate_capture_header_path(path, &core.data_dir, &header)?;
        let selected = match &header.library {
            CaptureLibrary::DeviceUnassigned => false,
            CaptureLibrary::Bound { binding } => {
                if binding != expected
                    && (binding.workspace_id == expected.workspace_id
                        || binding.root.eq_ignore_ascii_case(&expected.root))
                {
                    return Err(CoreError::new(
                        "scopeChanged",
                        "Capture original library identity differs",
                    ));
                }
                binding == expected
            }
        };
        if selected
            && (size > remaining as u64 || size > capture_draft::MAX_CAPTURE_RECORD_BYTES as u64)
        {
            return Err(backup_budget());
        }
        return Ok((selected, serde_json::to_string(&header)?));
    }
    match read_actor_library_header(file)? {
        ActorLibraryHeader::Bound(binding) => {
            let selected = binding == *expected;
            if !selected
                && (binding.workspace_id == expected.workspace_id
                    || binding.root.eq_ignore_ascii_case(&expected.root))
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    "A private record's original library identity differs",
                ));
            }
            Ok((selected, serde_json::to_string(&binding)?))
        }
        ActorLibraryHeader::LegacyUnbound => {
            file.seek(SeekFrom::Start(0))?;
            let is_draft = record_kind(core, path)? == RecordKind::Draft;
            if is_draft {
                // Identify the primary path before a legacy author string. A
                // selected record's complete size is charged before scanning
                // its alternatives, so routed legacy bodies cannot multiply
                // the whole-backup read budget.
                let mut prefix = Tokens::new(file, 4096);
                prefix.expect(b'{')?;
                match prefix.key()?.as_str() {
                    "path" => {
                        let path = prefix.string()?;
                        if !locator_within(Path::new(&path), root) {
                            return Err(private_evidence());
                        }
                        if size > remaining as u64 {
                            return Err(backup_budget());
                        }
                    }
                    "identity" => {
                        // Legacy migration contains routing metadata only.
                        // Its ordinary producer remains bounded by the same
                        // private record byte budget; never parse unknown fields.
                        if size > MAX_OPERATION_BYTES as u64 {
                            return Err(backup_budget());
                        }
                    }
                    _ => return Err(private_evidence()),
                }
                file.seek(SeekFrom::Start(0))?;
                let route = legacy_draft_route(file, root)?;
                return Ok((route.0, route.1));
            }
            if record_kind(core, path)? == RecordKind::Output {
                return Err(private_evidence());
            }
            let id = legacy_actor_core_id(file)?;
            let plan = core.operations.get(&id).ok_or_else(private_evidence)?;
            let selected = plan.workspace_id.as_deref() == expected.workspace_id.as_deref()
                || locator_within(Path::new(&plan.path), root);
            Ok((selected, format!("legacyPlan:{id}:{}", plan.fingerprint)))
        }
    }
}

fn route_selected_bytes(
    core: &Core,
    path: &Path,
    bytes: &[u8],
    root: &Path,
    expected: &ActorLibraryBinding,
    selected_plans: &BTreeSet<&str>,
) -> Result<String> {
    if record_kind(core, path)? == RecordKind::Capture {
        let header = capture_draft::read_capture_header(&mut std::io::Cursor::new(bytes))?;
        capture_draft::validate_capture_header_path(path, &core.data_dir, &header)?;
        if header.library
            != (CaptureLibrary::Bound {
                binding: expected.clone(),
            })
        {
            return Err(private_evidence());
        }
        return Ok(serde_json::to_string(&header)?);
    }
    if record_kind(core, path)? == RecordKind::Journal {
        if !selected_plans.iter().any(|id| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!("{id}."))
        }) {
            return Err(private_evidence());
        }
        let name = path.file_name().unwrap().to_string_lossy();
        let id = name.split('.').next().ok_or_else(private_evidence)?;
        qualified_journal_suffix(id, &name)?;
        return Ok("selectedJournal".into());
    }
    let value: UniqueValue = serde_json::from_slice(bytes)?;
    if let Some(library) = value.0.get("library") {
        let binding: ActorLibraryBinding = serde_json::from_value(library.clone())?;
        if binding != *expected {
            return Err(private_evidence());
        }
        return Ok(serde_json::to_string(&binding)?);
    }
    if record_kind(core, path)? == RecordKind::Draft {
        return legacy_draft_route(&mut std::io::Cursor::new(bytes), root).map(|r| r.1);
    }
    let id = value.0["coreOperationId"]
        .as_str()
        .ok_or_else(private_evidence)?;
    // The full Plan fingerprint was fixed by capture; find it through the
    // current Core in the live routing path rather than trusting actor payload.
    // Protected input routing below substitutes this exact captured route.
    if !selected_plans.contains(id) {
        return Err(private_evidence());
    }
    let plan = core.operations.get(id).ok_or_else(private_evidence)?;
    Ok(format!("legacyPlan:{id}:{}", plan.fingerprint))
}

fn validate_selected_record(
    core: &Core,
    path: &Path,
    bytes: &[u8],
    root: &Path,
    workspace_id: &str,
) -> Result<()> {
    let kind = record_kind(core, path)?;
    if kind == RecordKind::Journal {
        let name = path.file_name().unwrap().to_string_lossy();
        let id = name.split('.').next().ok_or_else(private_evidence)?;
        let suffix = qualified_journal_suffix(id, &name)?;
        if !core.operations.contains_key(id) {
            if suffix == "resolution" {
                let record = journal::decode_resolution(id, bytes)?;
                if record.action != "cancelPrepared" {
                    return Err(private_evidence());
                }
            } else {
                let scope = core.validate_preplan_reservation(id, suffix, bytes)?;
                let guard =
                    ParentGuard::acquire(&core.backend, &root.join(".reservation-backup-root"))?;
                if scope.workspace_id.as_deref() != Some(workspace_id)
                    || scope.root_identity != guard.directory_identity()?
                {
                    return Err(private_evidence());
                }
            }
        }
        return Ok(());
    }
    if kind == RecordKind::Capture {
        let draft = capture_draft::validate_capture_record(path, &core.data_dir, bytes)?;
        let guard = ParentGuard::acquire(&core.backend, &root.join(".backup-capture-prospective"))?;
        let expected = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: Some(workspace_id.into()),
        };
        if draft.scope.library != (CaptureLibrary::Bound { binding: expected }) {
            return Err(private_evidence());
        }
        return Ok(());
    }
    let value: UniqueValue = serde_json::from_slice(bytes)?;
    if let Some(library) = value.0.get("library") {
        let binding: ActorLibraryBinding = serde_json::from_value(library.clone())?;
        let guard = ParentGuard::acquire(&core.backend, &root.join(".backup-record-prospective"))?;
        if binding.root != display(&guard.directory_coordinates()?)
            || binding.root_identity != guard.directory_identity()?
            || binding.workspace_id.as_deref() != Some(workspace_id)
        {
            return Err(private_evidence());
        }
        if matches!(kind, RecordKind::Actor | RecordKind::Output) {
            core.validate_scoped_actor_record(path, bytes, &binding)?;
        }
    } else if kind == RecordKind::Actor {
        let guard = ParentGuard::acquire(&core.backend, &root.join(".backup-record-prospective"))?;
        let binding = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: Some(workspace_id.into()),
        };
        core.validate_legacy_actor_backup_record(path, bytes, &binding)?;
    }
    if kind == RecordKind::Draft {
        if value.0.get("consumed").is_some() {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
            struct Migration {
                #[serde(default)]
                library: Option<ActorLibraryBinding>,
                identity: FileIdentity,
                consumed: Vec<Reference>,
            }
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
            struct Reference {
                #[serde(default)]
                library: Option<ActorLibraryBinding>,
                path: String,
                fingerprint: String,
            }
            let migration: Migration = serde_json::from_slice(bytes)?;
            if migration.consumed.is_empty() {
                return Err(private_evidence());
            }
            for reference in &migration.consumed {
                if !locator_within(Path::new(&reference.path), root)
                    || reference.library != migration.library
                    || reference.fingerprint.len() != 64
                {
                    return Err(private_evidence());
                }
            }
            let _ = migration.identity;
        } else {
            let draft: Draft = serde_json::from_slice(bytes)?;
            if !locator_within(Path::new(&draft.path), root)
                || draft
                    .alternatives
                    .iter()
                    .any(|d| !locator_within(Path::new(&d.path), root))
            {
                return Err(private_evidence());
            }
        }
        for value in std::iter::once(&value.0)
            .chain(value.0["alternatives"].as_array().into_iter().flatten())
            .chain(value.0["consumed"].as_array().into_iter().flatten())
        {
            if let Some(p) = value["path"].as_str()
                && !locator_within(Path::new(p), root)
            {
                return Err(private_evidence());
            }
        }
    }
    Ok(())
}

// Strict first metadata readers. These never buffer a foreign author payload.
struct Tokens<'a, R: Read> {
    reader: &'a mut R,
    look: Option<u8>,
    count: usize,
    limit: usize,
}
impl<'a, R: Read> Tokens<'a, R> {
    fn new(reader: &'a mut R, limit: usize) -> Self {
        Self {
            reader,
            look: None,
            count: 0,
            limit,
        }
    }
    fn byte(&mut self) -> Result<u8> {
        if let Some(byte) = self.look.take() {
            return Ok(byte);
        }
        if self.count >= self.limit {
            return Err(backup_budget());
        }
        let mut b = [0];
        self.reader
            .read_exact(&mut b)
            .map_err(|_| private_evidence())?;
        self.count += 1;
        Ok(b[0])
    }
    fn next(&mut self) -> Result<u8> {
        loop {
            let b = self.byte()?;
            if !b.is_ascii_whitespace() {
                return Ok(b);
            }
        }
    }
    fn expect(&mut self, expected: u8) -> Result<()> {
        if self.next()? != expected {
            return Err(private_evidence());
        }
        Ok(())
    }
    fn string(&mut self) -> Result<String> {
        self.expect(b'"')?;
        let mut bytes = vec![b'"'];
        let mut escape = false;
        loop {
            let b = self.byte()?;
            bytes.push(b);
            if b == b'"' && !escape {
                break;
            }
            escape = b == b'\\' && !escape;
            if b != b'\\' {
                escape = false;
            }
        }
        serde_json::from_slice(&bytes).map_err(|_| private_evidence())
    }
    fn key(&mut self) -> Result<String> {
        let key = self.string()?;
        self.expect(b':')?;
        Ok(key)
    }
    fn scalar(&mut self) -> Result<()> {
        let first = self.next()?;
        if first == b'"' {
            self.look = Some(first);
            self.string()?;
            return Ok(());
        }
        let mut bytes = vec![first];
        loop {
            let b = self.byte()?;
            if matches!(b, b',' | b'}' | b']') || b.is_ascii_whitespace() {
                self.look = Some(b);
                break;
            }
            bytes.push(b);
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| private_evidence())?;
        if value.is_array() || value.is_object() {
            return Err(private_evidence());
        }
        Ok(())
    }
    fn identity(&mut self) -> Result<()> {
        let b = self.next()?;
        if b == b'n' {
            self.look = Some(b);
            return self.scalar();
        }
        if b != b'{' {
            return Err(private_evidence());
        }
        let mut bytes = vec![b'{'];
        let mut quoted = false;
        let mut escaped = false;
        let mut depth = 1;
        while depth != 0 {
            let byte = self.byte()?;
            bytes.push(byte);
            if bytes.len() > 2048 {
                return Err(private_evidence());
            }
            if quoted {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    quoted = false;
                }
            } else {
                match byte {
                    b'"' => quoted = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => depth -= 1,
                    _ => {}
                }
            }
        }
        let unique: UniqueValue = serde_json::from_slice(&bytes).map_err(|_| private_evidence())?;
        let _: FileIdentity = serde_json::from_value(unique.0).map_err(|_| private_evidence())?;
        Ok(())
    }
}
fn legacy_actor_core_id(file: &mut impl Read) -> Result<String> {
    let mut t = Tokens::new(file, 4096);
    t.expect(b'{')?;
    for expected in ["actor", "resource", "fingerprint", "coreOperationId"] {
        if t.key()? != expected {
            return Err(private_evidence());
        }
        if expected == "coreOperationId" {
            let id = t.string()?;
            valid_operation_id(&id)?;
            return Ok(id);
        }
        t.string()?;
        t.expect(b',')?;
    }
    Err(private_evidence())
}
fn legacy_draft_route(reader: &mut impl Read, root: &Path) -> Result<(bool, String)> {
    let mut t = Tokens::new(reader, MAX_OPERATION_BYTES);
    t.expect(b'{')?;
    let first = t.key()?;
    if first == "identity" {
        // This old record has no author strings. A bounded metadata-only join
        // remains possible, while an arbitrarily large unscoped list does not.
        t.limit = 16 * 1024;
        t.identity()?;
        t.expect(b',')?;
        if t.key()? != "consumed" {
            return Err(private_evidence());
        }
        t.expect(b'[')?;
        let mut paths = Vec::new();
        loop {
            let b = t.next()?;
            if b == b']' {
                break;
            }
            if b != b'{' {
                return Err(private_evidence());
            }
            if t.key()? != "path" {
                return Err(private_evidence());
            }
            let path = t.string()?;
            paths.push(path);
            t.expect(b',')?;
            if t.key()? != "fingerprint" {
                return Err(private_evidence());
            }
            t.string()?;
            t.expect(b'}')?;
            match t.next()? {
                b']' => break,
                b',' => {}
                _ => return Err(private_evidence()),
            }
        }
        t.expect(b'}')?;
        if paths.is_empty() {
            return Err(private_evidence());
        }
        let selected = paths.iter().all(|p| locator_within(Path::new(p), root));
        if !selected && paths.iter().any(|p| locator_within(Path::new(p), root)) {
            return Err(private_evidence());
        }
        return Ok((
            selected,
            format!("legacyMigration:{}", serde_json::to_string(&paths)?),
        ));
    }
    if first != "path" {
        return Err(private_evidence());
    }
    let path = t.string()?;
    if !locator_within(Path::new(&path), root) {
        return Err(private_evidence());
    }
    t.expect(b',')?;
    draft_fields(&mut t, root, false)?;
    Ok((true, format!("legacyDraft:{path}")))
}
fn draft_fields<R: Read>(t: &mut Tokens<'_, R>, root: &Path, alternative: bool) -> Result<()> {
    let mut keys = BTreeSet::new();
    loop {
        let key = t.key()?;
        if !keys.insert(key.clone()) {
            return Err(private_evidence());
        }
        match key.as_str() {
            "baseSource" | "source" => {
                t.string()?;
            }
            "baseKnown" => t.scalar()?,
            "identity" if !alternative => t.identity()?,
            "alternatives" if !alternative => {
                t.expect(b'[')?;
                loop {
                    let b = t.next()?;
                    if b == b']' {
                        break;
                    }
                    if b != b'{' {
                        return Err(private_evidence());
                    }
                    if t.key()? != "path" {
                        return Err(private_evidence());
                    }
                    let path = t.string()?;
                    if !locator_within(Path::new(&path), root) {
                        return Err(private_evidence());
                    }
                    t.expect(b',')?;
                    draft_fields(t, root, true)?;
                    match t.next()? {
                        b']' => break,
                        b',' => {}
                        _ => return Err(private_evidence()),
                    }
                }
            }
            _ => return Err(private_evidence()),
        }
        match t.next()? {
            b'}' => return Ok(()),
            b',' => {}
            _ => return Err(private_evidence()),
        }
    }
}

// serde_json::Value accepts duplicate map keys; recovery provenance must not.
struct UniqueValue(serde_json::Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "JSON with unique object fields")
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueValue(n.into()))
                    .ok_or_else(|| E::custom("Invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(serde_json::Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut v = vec![];
                while let Some(UniqueValue(e)) = a.next_element()? {
                    v.push(e);
                }
                Ok(UniqueValue(v.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut v = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if v.contains_key(&k) {
                        return Err(serde::de::Error::custom("Duplicate private metadata field"));
                    }
                    let UniqueValue(e) = a.next_value()?;
                    v.insert(k, e);
                }
                Ok(UniqueValue(v.into()))
            }
        }
        d.deserialize_any(V)
    }
}

fn own_suffix_limit(plan: &Plan) -> usize {
    if let Some(FrozenOperation {
        mutation: Mutation::CreateTree {
            directories, files, ..
        },
        ..
    }) = &plan.transaction
    {
        directories.len() + files.len() + 6
    } else {
        6
    }
}
fn validate_own_suffix(
    backend: &crate::BackendContext,
    plan: &Plan,
    path: &Path,
    attrs: backend::EntryMetadata,
) -> Result<(ParentGuard, backend::BoundFile)> {
    if !attrs.is_file() {
        return Err(private_evidence());
    }
    let name = path.file_name().unwrap().to_string_lossy();
    let suffix = name
        .strip_prefix(&format!("{}.", plan.operation_id))
        .and_then(|p| p.strip_suffix(".json"))
        .ok_or_else(private_evidence)?;
    let guard = ParentGuard::acquire(backend, path)?;
    let mut file = guard.open_read_guard_file()?;
    if suffix == "plan" {
        // A Plan may encode two copies of byte arrays. Compare a streamed exact
        // serialization without allocating another copy or widening read_blob.
        struct HashWriter(Sha256, u64);
        impl Write for HashWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.update(b);
                self.1 += b.len() as u64;
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut expected = HashWriter(Sha256::new(), 0);
        serde_json::to_writer(&mut expected, plan)?;
        if stamp(&file)?.size != expected.1 {
            return Err(private_evidence());
        }
        let mut actual = Sha256::new();
        let mut buf = [0; 8192];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            actual.update(&buf[..n]);
        }
        if actual.finalize() != expected.0.finalize() {
            return Err(private_evidence());
        }
        return Ok((guard, file));
    }
    if stamp(&file)?.size > 4096 {
        return Err(private_evidence());
    }
    let bytes = read_blob(&mut file)?;
    let UniqueValue(value) = serde_json::from_slice(&bytes)?;
    match suffix {
        "install" => {
            if value
                != serde_json::json!({"operationId":plan.operation_id,"fingerprint":plan.fingerprint})
            {
                return Err(private_evidence());
            }
        }
        "observations" => {
            if value != serde_json::json!([]) {
                return Err(private_evidence());
            }
        }
        "commit" => {
            if value["operationId"] != plan.operation_id
                || value["fingerprint"] != plan.fingerprint
                || value["receipt"] != serde_json::to_value(&plan.receipt)?
                || value["storageProfile"] != STORAGE_PROFILE
                || value["commitSequence"].as_u64().is_none()
                || !value.as_object().is_some_and(|v| v.len() == 6)
            {
                return Err(private_evidence());
            }
        }
        "resolution" => {
            if value["operationId"] != plan.operation_id
                || !matches!(
                    value["action"].as_str(),
                    Some("keepCurrent" | "createRejected" | "cancelPrepared")
                )
                || value["message"]
                    != "Current files were preserved; original plan remains retained"
                || !value.as_object().is_some_and(|v| v.len() == 3)
            {
                return Err(private_evidence());
            }
        }
        object if object.starts_with("object-") => {
            let Some(FrozenOperation {
                mutation:
                    Mutation::CreateTree {
                        target,
                        directories,
                        files,
                        ..
                    },
                ..
            }) = &plan.transaction
            else {
                return Err(private_evidence());
            };
            let recorded = value["path"].as_str().ok_or_else(private_evidence)?;
            let allowed = recorded == target
                || directories
                    .iter()
                    .any(|d| display(&Path::new(target).join(d)) == recorded)
                || files.iter().any(|f| f.path == recorded);
            if !allowed
                || object != format!("object-{}", hash(recorded.as_bytes()))
                || !value.as_object().is_some_and(|v| v.len() == 2)
            {
                return Err(private_evidence());
            }
            let identity: FileIdentity = serde_json::from_value(value["identity"].clone())?;
            let directory = recorded == target
                || directories
                    .iter()
                    .any(|d| display(&Path::new(target).join(d)) == recorded);
            let actual = if directory {
                ParentGuard::acquire(
                    backend,
                    &Path::new(recorded).join(".backup-object-prospective"),
                )?
                .directory_identity()?
            } else {
                let placed =
                    ParentGuard::acquire(backend, Path::new(recorded))?.open_placement_file()?;
                stamp(&placed)?.identity
            };
            if actual != identity {
                return Err(private_evidence());
            }
        }
        _ => return Err(private_evidence()),
    }
    Ok((guard, file))
}

#[cfg(all(test, windows))]
mod tests;
