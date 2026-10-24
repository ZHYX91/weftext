//! Scope-first capture of the original private recovery records. The only
//! persisted routing witness is one digest in the original frozen operation.
use super::*;
use crate::capture_draft::{self, CaptureLibrary};
use crate::hosted::{ActorLibraryBinding, ActorLibraryHeader, read_actor_library_header};
use crate::journal::Plan;
use std::io::{Seek, SeekFrom};
use weftext_messages::UiMessage;

// The 256 shard directory entries are metadata overhead rather than new
// author payload; original selected-file budgets remain enforced separately.
const MAX_ROUTING_MEMBERS: usize = 5256;
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
fn journal_bucket_folder(core: &Core, path: &Path) -> bool {
    path.parent() == Some(core.data_dir.join("journal").as_path())
        && path.file_name().and_then(std::ffi::OsStr::to_str).is_some_and(|name| {
            name.len() == 2 && name.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
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
        ["journal", _, _] if path.parent().is_some_and(|parent| journal_bucket_folder(core, parent)) => Ok(RecordKind::Journal),
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

fn selected_library_scope(
    scope: Option<&ActorLibraryBinding>,
    expected: &ActorLibraryBinding,
) -> Result<bool> {
    let Some(scope) = scope else {
        return Ok(false);
    };
    if scope == expected {
        return Ok(true);
    }
    // An explicit different workspace at a different physical root is foreign,
    // including when its old locator has since been reused. A logical/physical
    // identity contradiction remains an error; a path cannot override either.
    if scope.root_identity == expected.root_identity || scope.workspace_id == expected.workspace_id
    {
        return Err(CoreError::new(
            "scopeChanged",
            UiMessage::error_core_original_scope_backup_library_identity_differs(),
        ));
    }
    if scope.workspace_id.is_some() && expected.workspace_id.is_some() {
        return Ok(false);
    }
    if scope.root.eq_ignore_ascii_case(&expected.root) {
        return Err(CoreError::new(
            "scopeChanged",
            UiMessage::error_core_original_scope_backup_unqualified_scope_differs(),
        ));
    }
    Ok(false)
}
fn selected_input_scope(
    core: &Core,
    scope: &QualifiedInputScope,
    expected: &ActorLibraryBinding,
) -> Result<bool> {
    core.validate_original_input_scope(scope)?;
    match scope {
        QualifiedInputScope::DeviceUnassigned => Ok(false),
        QualifiedInputScope::Library { binding } => {
            if binding.workspace_id.as_str()
                == expected
                    .workspace_id
                    .as_deref()
                    .ok_or_else(private_evidence)?
                && binding.root_identity == expected.root_identity
            {
                return Ok(true);
            }
            if binding.root_identity == expected.root_identity
                || Some(&binding.workspace_id) == expected.workspace_id.as_ref()
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_original_scope_backup_logical_physical_identity_conflict(
                    ),
                ));
            }
            Ok(false)
        }
    }
}
fn selected_plan_scope(
    core: &Core,
    plan: &Plan,
    workspace_id: &str,
    expected: &ActorLibraryBinding,
) -> Result<bool> {
    if matches!(plan.kind.as_str(), "save" | "saveAs") {
        core.validate_file_save_plan(plan)?;
        return selected_input_scope(core, &plan.file_save.as_ref().unwrap().scope, expected);
    }
    let original = plan.workspace_id.as_ref().ok_or_else(private_evidence)?;
    Uuid::parse_str(original).map_err(|_| private_evidence())?;
    Ok(original == workspace_id)
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
                UiMessage::error_core_operations_legacy_backup_has_no_complete_private_routing_witness(),
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
                UiMessage::error_core_operations_original_private_recovery_membership_or_routing_changed(),
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
                if selected_library_scope(scope.as_ref(), &binding)? {
                    selected_attempts.push(id.to_owned());
                }
            }
        }
        let mut capture = PrivateCapture {
            digest: String::new(),
            selected_files: Vec::new(),
            guards: vec![root_guard],
            handles: Vec::new(),
        };
        let mut selected_plans = BTreeSet::new();
        for plan in self.operations.values() {
            if !library_creation::is_cancellation(plan)
                && own.is_none_or(|own| plan.operation_id != own.operation_id)
                && selected_plan_scope(self, plan, workspace_id, &binding)?
            {
                selected_plans.insert(plan.operation_id.as_str());
            }
        }
        selected_plans.extend(selected_attempts.iter().map(String::as_str));
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
                if journal_bucket_folder(self, &folder)
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
                    if folder == self.data_dir.join("journal") && journal_bucket_folder(self, &path) {
                        queue.push(path);
                        continue;
                    }
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
                    let (selected, route) = if journal_bucket_folder(self, &folder) {
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
                            let selected = selected_library_scope(scope.as_ref(), &binding)?;
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
                            (
                                MAX_OPERATION_BYTES.saturating_sub(selected_bytes),
                                actual.size,
                            ),
                            &selected_plans,
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
        UiMessage::error_core_operations_complete_library_backup_exceeds64_mi_b_or5000_members(),
    )
}
fn private_evidence() -> CoreError {
    CoreError::new(
        "privateEvidence",
        UiMessage::error_core_operations_original_private_evidence_cannot_scoped_completely_without_reading_another_library(),
    )
}

fn classify_record(
    core: &Core,
    path: &Path,
    file: &mut backend::BoundFile,
    _root: &Path,
    expected: &ActorLibraryBinding,
    bounds: (usize, u64),
    _selected_plans: &BTreeSet<&str>,
) -> Result<(bool, String)> {
    let (remaining, size) = bounds;
    if record_kind(core, path)? == RecordKind::Capture {
        let prefix = capture_draft::read_capture_prefix(file)?;
        capture_draft::validate_capture_header_path(path, &core.data_dir, &prefix.scope)?;
        let selected = match &prefix.scope.library {
            CaptureLibrary::DeviceUnassigned => false,
            CaptureLibrary::Bound { binding } => selected_library_scope(Some(binding), expected)?,
        };
        if selected {
            prefix.require_supported_version()?;
            if size > remaining as u64 || size > capture_draft::MAX_CAPTURE_RECORD_BYTES as u64 {
                return Err(backup_budget());
            }
        }
        return Ok((selected, serde_json::to_string(&prefix)?));
    }
    if record_kind(core, path)? == RecordKind::Draft {
        let scope = file_save_scope::read_input_scope(file)?;
        return Ok((
            selected_input_scope(core, &scope, expected)?,
            serde_json::to_string(&scope)?,
        ));
    }
    if record_kind(core, path)? == RecordKind::Actor {
        let scope = crate::hosted::read_actor_input_header(file)?;
        let selected = match &scope {
            crate::hosted::ActorInputScope::NoInput => {
                if size > 4096 {
                    return Err(backup_budget());
                }
                let prior = file.stream_position()?;
                file.seek(SeekFrom::Start(0))?;
                let mut bytes = Vec::new();
                Read::by_ref(file).take(4097).read_to_end(&mut bytes)?;
                file.seek(SeekFrom::Start(prior))?;
                core.validate_no_input_actor_fence(path, &bytes)?;
                false
            }
            crate::hosted::ActorInputScope::Qualified { scope } => {
                selected_input_scope(core, scope, expected)?
            }
        };
        return Ok((selected, serde_json::to_string(&scope)?));
    }
    match read_actor_library_header(file)? {
        ActorLibraryHeader::Bound(binding) => Ok((
            selected_library_scope(Some(&binding), expected)?,
            serde_json::to_string(&binding)?,
        )),
        ActorLibraryHeader::LegacyUnbound => Err(private_evidence()),
    }
}

fn route_selected_bytes(
    core: &Core,
    path: &Path,
    bytes: &[u8],
    _root: &Path,
    expected: &ActorLibraryBinding,
    selected_plans: &BTreeSet<&str>,
) -> Result<String> {
    if record_kind(core, path)? == RecordKind::Capture {
        let prefix = capture_draft::read_capture_prefix(&mut std::io::Cursor::new(bytes))?;
        capture_draft::validate_capture_header_path(path, &core.data_dir, &prefix.scope)?;
        if prefix.scope.library
            != (CaptureLibrary::Bound {
                binding: expected.clone(),
            })
        {
            return Err(private_evidence());
        }
        prefix.require_supported_version()?;
        return Ok(serde_json::to_string(&prefix)?);
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
    if record_kind(core, path)? == RecordKind::Draft {
        let scope = file_save_scope::read_input_scope(&mut std::io::Cursor::new(bytes))?;
        if !selected_input_scope(core, &scope, expected)? {
            return Err(private_evidence());
        }
        return Ok(serde_json::to_string(&scope)?);
    }
    let value = private_record_json(bytes)?;
    if record_kind(core, path)? == RecordKind::Actor {
        let scope = crate::hosted::read_actor_input_header(&mut std::io::Cursor::new(bytes))?;
        if !matches!(&scope, crate::hosted::ActorInputScope::Qualified { scope } if selected_input_scope(core, scope, expected)?)
        {
            return Err(private_evidence());
        }
        return Ok(serde_json::to_string(&scope)?);
    }
    let binding: ActorLibraryBinding = serde_json::from_value(value["library"].clone())?;
    if binding != *expected {
        return Err(private_evidence());
    }
    Ok(serde_json::to_string(&binding)?)
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
        if let Some(plan) = core.operations.get(id) {
            if suffix == "plan" {
                let retained: Plan = serde_json::from_slice(bytes)?;
                if serde_json::to_vec(&retained)? != serde_json::to_vec(plan)? {
                    return Err(private_evidence());
                }
                if matches!(retained.kind.as_str(), "save" | "saveAs") {
                    core.validate_file_save_plan(&retained)?;
                }
            } else if suffix == "commit" {
                journal::validate_original_commit_bytes(plan, bytes)?;
            }
        }
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
        let guard = ParentGuard::acquire(&core.backend, &root.join(".backup-capture-prospective"))?;
        let expected = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: Some(workspace_id.into()),
        };
        let prefix = capture_draft::read_capture_prefix(&mut std::io::Cursor::new(bytes))?;
        capture_draft::validate_capture_header_path(path, &core.data_dir, &prefix.scope)?;
        if prefix.scope.library != (CaptureLibrary::Bound { binding: expected }) {
            return Err(private_evidence());
        }
        prefix.require_supported_version()?;
        capture_draft::validate_capture_record(path, &core.data_dir, bytes)?;
        return Ok(());
    }
    let value = private_record_json(bytes)?;
    if kind == RecordKind::Draft {
        let draft: Draft = serde_json::from_slice(bytes)?;
        draft.validate_producers()?;
        let guard = ParentGuard::acquire(&core.backend, &root.join(".backup-draft-root"))?;
        let expected = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: Some(workspace_id.into()),
        };
        if !selected_input_scope(core, &draft.scope, &expected)?
            || !locator_within(Path::new(&draft.path), root)
            || draft
                .alternatives
                .iter()
                .any(|d| !locator_within(Path::new(&d.path), root))
        {
            return Err(private_evidence());
        }
        core.validate_original_file_save_scope(&draft.scope, Path::new(&draft.path))?;
        for input in std::iter::once(draft.proposal()).chain(draft.alternatives.iter().cloned()) {
            core.qualify_committed_baseline(&draft.scope, &draft.identity, &input)?;
        }
        return Ok(());
    }
    let guard = ParentGuard::acquire(&core.backend, &root.join(".backup-record-prospective"))?;
    let expected = ActorLibraryBinding {
        root: display(&guard.directory_coordinates()?),
        root_identity: guard.directory_identity()?,
        workspace_id: Some(workspace_id.into()),
    };
    if kind == RecordKind::Actor {
        let input = crate::hosted::read_actor_input_header(&mut std::io::Cursor::new(bytes))?;
        if !matches!(&input, crate::hosted::ActorInputScope::Qualified { scope } if selected_input_scope(core, scope, &expected)?)
        {
            return Err(private_evidence());
        }
    }
    let file_plan = value["coreOperationId"]
        .as_str()
        .and_then(|id| core.operations.get(id))
        .filter(|plan| matches!(plan.kind.as_str(), "save" | "saveAs"));
    if let Some(plan) = file_plan {
        core.validate_file_save_plan(plan)?;
        if !selected_input_scope(core, &plan.file_save.as_ref().unwrap().scope, &expected)? {
            return Err(private_evidence());
        }
    }
    if let Some(library) = value.get("library") {
        let binding: ActorLibraryBinding = serde_json::from_value(library.clone())?;
        if kind != RecordKind::Actor && binding != expected {
            return Err(private_evidence());
        }
        core.validate_scoped_actor_record(path, bytes, &binding)?;
    } else if kind == RecordKind::Actor {
        core.validate_unbound_actor_backup_record(path, bytes, &expected)?;
    } else {
        return Err(private_evidence());
    }
    Ok(())
}

// Complete records reach this decoder only after scope-first classification
// and exact selected-byte or own-suffix budget checks. Foreign routing remains streaming.
fn private_record_json(bytes: &[u8]) -> Result<serde_json::Value> {
    weftext_data::parse_unique_json(bytes)
        .map_err(|error| CoreError::from_data("invalidState", error))
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
    let value = private_record_json(&bytes)?;
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
            journal::validate_original_commit_bytes(plan, &bytes)?;
            if value["commitSequence"].as_u64().is_none() {
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

#[cfg(all(test, windows))]
mod parse_boundary_tests {
    use super::*;

    #[test]
    fn selected_private_metadata_rejects_duplicates_with_typed_data_classification() {
        for (bytes, duplicate) in [
            (br#"{"library":{},"library":{}}"#.as_slice(), "library"),
            (
                br#"{"nested":{"duplicate":1,"duplicate":2}}"#.as_slice(),
                "duplicate",
            ),
        ] {
            let original = bytes.to_vec();
            let error = private_record_json(bytes).unwrap_err();
            assert_eq!(error.code, "invalidState");
            assert_eq!(
                error.explanation,
                UiMessage::error_data_schema_json_key_duplicate(
                    weftext_messages::params::ErrorDataSchemaJsonKeyDuplicate {
                        key: duplicate.into()
                    }
                )
            );
            assert!(error.diagnostics.is_empty());
            assert!(
                matches!(error.details, Some(crate::CoreErrorDetails::Aggregate { groups })
                if groups.iter().any(|group| matches!(group, crate::ErrorDetailGroup::Data {
                    classification: weftext_data::DataErrorKind::Invalid, diagnostics
                } if diagnostics.is_empty())))
            );
            assert_eq!(bytes, original);
        }
        let malformed = br#"{"nested":[}"#;
        let raw = serde_json::from_slice::<serde_json::Value>(malformed)
            .unwrap_err()
            .to_string();
        let error = private_record_json(malformed).unwrap_err();
        assert_eq!(error.code, "invalidState");
        assert_eq!(error.diagnostics.len(), 1);
        assert_eq!(error.diagnostics[0].origin, DiagnosticOrigin::Json);
        assert_eq!(error.diagnostics[0].text, raw);
    }

    #[test]
    fn foreign_actor_classification_still_stops_at_header_before_malformed_author_body() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let mut core = Core::new(&data).unwrap();
        let selected = temp.path().join("selected");
        let foreign = temp.path().join("foreign");
        core.create_library(&selected).unwrap();
        core.create_library(&foreign).unwrap();
        let binding = |root: &Path| {
            let QualifiedInputScope::Library { binding } = core
                .capture_file_save_scope(&root.join("header.typ"))
                .unwrap()
            else {
                panic!("Expected admitted library")
            };
            binding.actor_binding()
        };
        let expected = binding(&selected);
        let foreign_binding = binding(&foreign);
        let scope = crate::hosted::ActorInputScope::Qualified {
            scope: QualifiedInputScope::Library {
                binding: file_save_scope::QualifiedLibrary {
                    root: foreign_binding.root.clone(),
                    root_identity: foreign_binding.root_identity.clone(),
                    workspace_id: foreign_binding.workspace_id.clone().unwrap(),
                },
            },
        };
        let path = core
            .data_dir
            .join("actors")
            .join(hash(b"foreign-actor"))
            .join("operations")
            .join(format!("{}.json", fresh()));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = format!(
            "{{\"scope\":{},\"library\":{},\"body\":NOT_JSON_\"\\ud800",
            serde_json::to_string(&scope).unwrap(),
            serde_json::to_string(&foreign_binding).unwrap()
        );
        fs::write(&path, &original).unwrap();
        let mut file = core.backend.open_read(&path).unwrap();
        let (included, route) = classify_record(
            &core,
            &path,
            &mut file,
            &selected,
            &expected,
            (0, original.len() as u64),
            &BTreeSet::new(),
        )
        .unwrap();
        assert!(!included);
        assert_eq!(route, serde_json::to_string(&scope).unwrap());
        assert_eq!(file.stream_position().unwrap(), 0);
        assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
        assert!(private_record_json(original.as_bytes()).is_err());
    }
}
