//! Explicit recovery of the original frozen decision. Inspection never writes
//! user files; continuation holds every original participant until commit.
use super::*;
use journal::{Plan, RecoveryBlob, RecoveryIdentity, RecoveryMember};
use operations::{FileChange, Mutation, read_blob, write_blob};
use std::collections::BTreeSet;
use weftext_messages::UiMessage;

pub(crate) struct RecoveryCapture {
    pub(crate) members: Vec<RecoveryMember>,
    pub(crate) guards: Vec<ParentGuard>,
    pub(crate) files: Vec<(usize, File)>,
    pub(crate) placement_files: Vec<File>,
    pub(crate) changes: Vec<FileChange>,
    pub(crate) coherence: String,
    pub(crate) compound: Option<operations::compound::CompoundCapture>,
}
impl RecoveryCapture {
    fn complete(&mut self) {
        if self.members.iter().any(|m| m.state == "thirdState") {
            self.coherence = "thirdState".into();
        } else if self.members.iter().any(|m| m.state == "unavailable") {
            self.coherence = "unavailable".into();
        } else if self.members.iter().all(|m| m.matches_before) {
            self.coherence = "knownBefore".into();
        } else if self.members.iter().all(|m| m.matches_after) {
            self.coherence = "knownAfter".into();
        } else {
            self.coherence = "mixedKnown".into();
        }
    }
}
impl Core {
    pub(crate) fn inspect_transaction_recovery(&self, plan: &Plan) -> Result<RecoveryInfo> {
        if plan.sync_setup.is_some() || plan.sync_application.is_some() {
            self.sync_local_owner()?;
        }
        let t = plan.transaction.as_ref().unwrap();
        let capture = self.capture_transaction_recovery(plan, false)?;
        let installing = self.journal_path(&plan.operation_id, "install").exists();
        let status = if self.committed(&plan.operation_id) {
            "committed"
        } else if self.resolved(&plan.operation_id)? {
            "resolved"
        } else if !installing {
            "prepared"
        } else {
            match capture.coherence.as_str() {
                "knownBefore" => "before",
                "knownAfter" => "after",
                "mixedKnown" => "mixedKnown",
                "thirdState" => "thirdState",
                _ => "unavailable",
            }
        };
        let mut allowed_actions = Vec::new();
        if !self.committed(&plan.operation_id) && !self.resolved(&plan.operation_id)? {
            if capture.coherence == "knownBefore" {
                allowed_actions.push("keepCurrent".into());
            }
            if installing && capture.members.iter().all(|m| m.matches_after) {
                allowed_actions.push("adoptAfterImages".into());
            }
            if installing
                && (capture.coherence == "mixedKnown"
                    || (capture.coherence == "knownBefore"
                        && matches!(t.mutation, Mutation::Compound { .. })))
                && can_resume_content(t, &capture)
            {
                allowed_actions.push("resumeOriginalPlan".into());
            }
        }
        Ok(RecoveryInfo {
            operation_id: plan.operation_id.clone(),
            path: plan.path.clone(),
            status: status.into(),
            before_source: None,
            after_source: String::new(),
            current_source: None,
            description: UiMessage::recovery_frozen_plan_retained(
                weftext_messages::params::RecoveryFrozenPlanRetained {
                    plan_id: t.preview.plan_id.clone(),
                },
            )
            .into(),
            coherence: capture.coherence,
            members: capture.members,
            allowed_actions,
        })
    }
    pub(crate) fn resolve_transaction_recovery(
        &mut self,
        plan: &Plan,
        action: &str,
    ) -> Result<RecoveryInfo> {
        if plan.sync_setup.is_some() || plan.sync_application.is_some() {
            self.sync_local_owner()?;
        }
        if !matches!(
            action,
            "keepCurrent" | "adoptAfterImages" | "resumeOriginalPlan"
        ) {
            return Err(CoreError::new(
                "unsupportedRecovery",
                UiMessage::error_core_transaction_recovery_choose_action_permitted_by_inspected_original_plan(),
            ));
        }
        if self.committed(&plan.operation_id) || self.resolved(&plan.operation_id)? {
            return self.inspect_transaction_recovery(plan);
        }
        let t = plan.transaction.as_ref().unwrap();
        let mut capture =
            self.capture_transaction_recovery(plan, action == "resumeOriginalPlan")?;
        let _private_capture = self.validate_owner_backup_private(plan, true)?;
        let installing = self.journal_path(&plan.operation_id, "install").exists();
        if action == "keepCurrent" {
            if capture.coherence != "knownBefore" {
                return Err(CoreError::new(
                    "recoveryConflict",
                    UiMessage::error_core_transaction_recovery_keeping_current_permitted_only_when_all_original_members_coherently_at_their_verified_before_state(),
                ));
            }
            self.resolve_record(&plan.operation_id, action)?;
        } else {
            let eligible = installing
                && match action {
                    "adoptAfterImages" => capture.members.iter().all(|m| m.matches_after),
                    "resumeOriginalPlan" => {
                        (capture.coherence == "mixedKnown"
                            || (capture.coherence == "knownBefore"
                                && matches!(t.mutation, Mutation::Compound { .. })))
                            && can_resume_content(t, &capture)
                    }
                    _ => false,
                };
            if !eligible {
                return Err(CoreError::new(
                    "recoveryConflict",
                    UiMessage::error_core_transaction_recovery_original_plan_has_unavailable_changed_or_incoherent_members_preserve_current_and_original_before_after_images_for_explicit_repair(),
                ));
            }
            if matches!(t.request.intent, OperationIntent::CloneSync { .. }) {
                // The create-only replica has no committed workspace until
                // this original transaction commits. Complete exact files and
                // materialized identities above are its activation proof.
                self.sync_local_owner()?;
                if plan
                    .sync_setup
                    .as_ref()
                    .is_none_or(|s| Some(&s.identity.workspace_id) != plan.workspace_id.as_ref())
                {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        UiMessage::error_core_transaction_recovery_replica_setup_binding_unavailable(),
                    ));
                }
            } else {
                let workspace = self.workspace(Path::new(&t.request.root))?.ok_or_else(|| {
                    CoreError::new(
                        "recoveryConflict",
                        UiMessage::error_core_transaction_recovery_original_committed_workspace_binding_unavailable(),
                    )
                })?;
                if plan.workspace_id.as_ref() != Some(&workspace.workspace_id) {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        UiMessage::error_core_transaction_recovery_original_workspace_identity_changed(),
                    ));
                }
            }
            if plan.sync_application.is_some() {
                self.validate_sync_recovery_cut(plan)?;
            }
            if action == "resumeOriginalPlan" {
                if matches!(t.mutation, Mutation::Compound { .. }) {
                    self.resume_compound_namespaces(plan, &mut capture)
                        .map_err(|e| recovery_failure(plan, e))?;
                }
                // The read set and every existing write member are protected
                // before the first continuation write. No new ID/plan is made.
                for change in capture.changes.clone() {
                    let index = capture
                        .members
                        .iter()
                        .position(|m| m.role == "write" && m.path == change.path)
                        .ok_or_else(|| {
                            CoreError::new("invalidPlan", UiMessage::error_core_transaction_recovery_frozen_change_lacks_recovery_member())
                        })?;
                    if capture.members[index].matches_after {
                        continue;
                    }
                    if let Some((_, file)) = capture.files.iter_mut().find(|(i, _)| *i == index) {
                        write_blob(file, &change.after).map_err(|e| recovery_failure(plan, e))?;
                    } else {
                        let compound_file = if matches!(t.mutation, Mutation::Compound { .. }) {
                            self.create_compound_recovery_output(plan, &mut capture, &change.path)?
                        } else {
                            None
                        };
                        let mut new_guard = None;
                        let mut file = if let Some(file) = compound_file {
                            file
                        } else {
                            let guard = ParentGuard::acquire(Path::new(&change.path))?;
                            let file = guard
                                .open_file(true, true)
                                .map_err(|e| recovery_failure(plan, e))?;
                            new_guard = Some(guard);
                            file
                        };
                        self.record_materialized(
                            &plan.operation_id,
                            Path::new(&change.path),
                            &file,
                        )
                        .map_err(|e| recovery_failure(plan, e))?;
                        write_blob(&mut file, &change.after)
                            .map_err(|e| recovery_failure(plan, e))?;
                        capture.files.push((index, file));
                        if let Some(guard) = new_guard {
                            capture.guards.push(guard);
                        }
                    }
                    capture.members[index].matches_after = true;
                }
            }
            let mut materialized = Vec::new();
            for (index, file) in &mut capture.files {
                let member = &capture.members[*index];
                if member.role == "write"
                    || (matches!(
                        t.mutation,
                        Mutation::Relocate { .. } | Mutation::Compound { .. }
                    ) && t.preview.revisions.contains_key(&member.path))
                {
                    let change = capture
                        .changes
                        .iter()
                        .find(|c| c.path == member.path)
                        .ok_or_else(|| {
                            CoreError::new(
                                "invalidPlan",
                                UiMessage::error_core_transaction_recovery_recovery_write_has_no_exact_after_image(),
                            )
                        })?;
                    if read_blob(file)? != change.after {
                        return Err(CoreError::new(
                            "recoveryConflict",
                            UiMessage::error_core_transaction_recovery_after_verification_changed_before_original_decision(),
                        ));
                    }
                    materialized.push((member.path.clone(), stamp(file)?));
                }
            }
            if plan.sync_application.is_some() {
                self.validate_sync_recovery_cut(plan)?;
            }
            if matches!(t.mutation, Mutation::Compound { .. }) {
                self.verify_compound_final(plan, &mut capture)
                    .map_err(|e| recovery_failure(plan, e))?;
            }
            self.publish_recovery_facts(plan, &materialized)
                .map_err(|e| recovery_failure(plan, e))?;
            self.commit(plan).map_err(|e| recovery_failure(plan, e))?;
            for session in self.sessions.values_mut().filter(|s| t.affects(&s.path)) {
                session.base_id = fresh();
            }
        }
        // Release all guards only after the durable decision, then re-inspect.
        drop(capture);
        self.inspect_transaction_recovery(plan)
    }
    pub(crate) fn capture_transaction_recovery(
        &self,
        plan: &Plan,
        write: bool,
    ) -> Result<RecoveryCapture> {
        let t = plan.transaction.as_ref().unwrap();
        let mut capture = RecoveryCapture {
            members: Vec::new(),
            guards: Vec::new(),
            files: Vec::new(),
            placement_files: Vec::new(),
            changes: Vec::new(),
            coherence: String::new(),
            compound: None,
        };
        if !self.committed(&plan.operation_id)
            && !self.resolved(&plan.operation_id)?
            && let Err(error) = self.validate_operation_pending(plan)
        {
            capture.members.push(RecoveryMember {
                kind: "barrier".into(),
                role: "readDependency".into(),
                path: t.request.root.clone(),
                target_path: None,
                before: RecoveryBlob::from_bytes(None),
                after: RecoveryBlob::from_bytes(None),
                current: RecoveryBlob::from_bytes(None),
                state: "unavailable".into(),
                matches_before: false,
                matches_after: false,
                identity_binding: "unavailable".into(),
                expected_identity: None,
                current_identity: None,
                description: Some(error.into()),
            });
        }
        if plan.sync_application.is_some()
            && !self.committed(&plan.operation_id)
            && !self.resolved(&plan.operation_id)?
            && let Err(error) = self.validate_sync_recovery_cut(plan)
        {
            capture.members.push(RecoveryMember {
                kind: "syncCut".into(),
                role: "readDependency".into(),
                path: t.request.root.clone(),
                target_path: None,
                before: RecoveryBlob::from_bytes(None),
                after: RecoveryBlob::from_bytes(None),
                current: RecoveryBlob::from_bytes(None),
                state: "unavailable".into(),
                matches_before: false,
                matches_after: false,
                identity_binding: "unavailable".into(),
                expected_identity: None,
                current_identity: None,
                description: Some(
                    CoreError::new(
                        "syncCutUnavailable",
                        UiMessage::recovery_replica_cut_unqualified(),
                    )
                    .caused_by(error)
                    .into(),
                ),
            });
        }
        if let Err(error) = self.validate_tag_plan_changes(plan) {
            capture.members.push(RecoveryMember {
                kind: "tagRegistry".into(),
                role: "readDependency".into(),
                path: Path::new(&t.request.root)
                    .join(".weftext-tags.json")
                    .to_string_lossy()
                    .into_owned(),
                target_path: None,
                before: RecoveryBlob::from_bytes(None),
                after: RecoveryBlob::from_bytes(None),
                current: RecoveryBlob::from_bytes(None),
                state: "thirdState".into(),
                matches_before: false,
                matches_after: false,
                identity_binding: "mismatch".into(),
                expected_identity: None,
                current_identity: None,
                description: Some(
                    CoreError::new(
                        "tagRoleUnavailable",
                        UiMessage::recovery_tags_role_unqualified(),
                    )
                    .caused_by(error)
                    .into(),
                ),
            });
        }
        if let Err(error) = self.validate_recovery_scope(t) {
            let third = matches!(
                error.code.as_str(),
                "scopeChanged" | "identityConflict" | "thirdState"
            );
            capture.members.push(RecoveryMember {
                kind: "scope".into(),
                role: "readDependency".into(),
                path: t.request.root.clone(),
                target_path: None,
                before: RecoveryBlob::from_bytes(Some(&serde_json::to_vec(
                    &t.expected_membership,
                )?)),
                after: RecoveryBlob::from_bytes(Some(&serde_json::to_vec(&t.expected_membership)?)),
                current: RecoveryBlob::from_bytes(None),
                state: if third { "thirdState" } else { "unavailable" }.into(),
                matches_before: false,
                matches_after: false,
                identity_binding: if third { "mismatch" } else { "unavailable" }.into(),
                expected_identity: None,
                current_identity: None,
                description: Some(error.into()),
            });
        }
        match self.validate_owner_backup_private(plan, false) {
            Ok(Some(private)) => {
                capture.guards.extend(private.guards);
                capture.placement_files.extend(private.handles);
            }
            Ok(None) => {}
            Err(error) => {
                capture.members.push(RecoveryMember {
                    kind: "privateScope".into(),
                    role: "readDependency".into(),
                    path: t.request.root.clone(),
                    target_path: None,
                    before: RecoveryBlob::from_bytes(
                        t.backup_private_digest.as_deref().map(str::as_bytes),
                    ),
                    after: RecoveryBlob::from_bytes(
                        t.backup_private_digest.as_deref().map(str::as_bytes),
                    ),
                    current: RecoveryBlob::from_bytes(None),
                    state: "thirdState".into(),
                    matches_before: false,
                    matches_after: false,
                    identity_binding: "mismatch".into(),
                    expected_identity: None,
                    current_identity: None,
                    description: Some(error.into()),
                });
            }
        }
        match &t.mutation {
            Mutation::Compound { .. } => {
                if let Err(error) = self.capture_compound_recovery(plan, &mut capture, write) {
                    let third = matches!(
                        error.code.as_str(),
                        "thirdState" | "scopeChanged" | "exists" | "conflict"
                    );
                    capture.members.push(RecoveryMember {
                        kind: "barrier".into(),
                        role: "readDependency".into(),
                        path: t.request.root.clone(),
                        target_path: None,
                        before: RecoveryBlob::from_bytes(None),
                        after: RecoveryBlob::from_bytes(None),
                        current: RecoveryBlob::from_bytes(None),
                        state: if third { "thirdState" } else { "unavailable" }.into(),
                        matches_before: false,
                        matches_after: false,
                        identity_binding: "unavailable".into(),
                        expected_identity: None,
                        current_identity: None,
                        description: Some(error.into()),
                    });
                    capture.compound = None;
                    capture.complete();
                    return Ok(capture);
                }
            }
            Mutation::Update { files } => capture.changes = files.clone(),
            Mutation::CreateTree {
                target,
                existing_target,
                directories,
                files,
                updates,
                ..
            } => {
                capture.changes = files.iter().chain(updates).cloned().collect();
                if !existing_target {
                    self.capture_created_directory(plan, target, &mut capture)?;
                }
                for relative in directories {
                    self.capture_created_directory(
                        plan,
                        &display(&Path::new(target).join(relative)),
                        &mut capture,
                    )?;
                }
            }
            Mutation::Relocate { .. } => {
                capture.changes = t.relocated_files();
                self.capture_relocation(plan, &mut capture, write)?;
                capture.complete();
                return Ok(capture);
            }
        }
        for (path, expected_parent) in &t.write_parents {
            let parent = Path::new(path).parent().ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_transaction_recovery_frozen_write_target_lacks_parent(),
                )
            })?;
            self.capture_bound_directory(&display(parent), expected_parent, &mut capture)?;
        }
        for change in &capture.changes {
            if capture
                .members
                .iter()
                .any(|member| member.role == "write" && member.path == change.path)
            {
                continue;
            }
            let is_created_output = t
                .created_files()
                .iter()
                .any(|file| file.path == change.path)
                || matches!(&t.mutation, Mutation::Compound { created_directories, .. }
                    if change.before.is_none() && created_directories.iter().any(|directory| Path::new(directory) == Path::new(&change.path).parent().unwrap()));
            if !is_created_output && !t.write_parents.iter().any(|(path, _)| path == &change.path) {
                capture.members.push(RecoveryMember {
                    kind: "barrier".into(),
                    role: "readDependency".into(),
                    path: change.path.clone(),
                    target_path: None,
                    before: RecoveryBlob::from_bytes(None),
                    after: RecoveryBlob::from_bytes(None),
                    current: RecoveryBlob::from_bytes(None),
                    state: "unavailable".into(),
                    matches_before: false,
                    matches_after: false,
                    identity_binding: "unavailable".into(),
                    expected_identity: None,
                    current_identity: None,
                    description: Some(UiMessage::recovery_write_parent_binding_missing().into()),
                });
            }
        }
        for change in capture.changes.clone() {
            if capture
                .members
                .iter()
                .any(|member| member.role == "write" && member.path == change.path)
            {
                continue;
            }
            let original = t
                .files
                .iter()
                .find(|f| f.path == change.path)
                .map(|f| &f.stamp.identity);
            self.capture_recovery_file(
                plan,
                &change.path,
                "write",
                change.before.as_deref(),
                Some(&change.after),
                original,
                write && change.before.as_ref() != Some(&change.after),
                &mut capture,
            )?;
        }
        for file in &t.files {
            if t.is_retired_member(&file.path) {
                continue;
            }
            if !capture
                .changes
                .iter()
                .any(|c| c.path == t.relocated_path(&file.path))
            {
                self.capture_recovery_file(
                    plan,
                    &file.path,
                    "readDependency",
                    Some(&file.bytes),
                    Some(&file.bytes),
                    Some(&file.stamp.identity),
                    false,
                    &mut capture,
                )?;
            }
        }
        for absent in &t.absent_dependencies {
            if !capture.changes.iter().any(|c| c.path == absent.path) {
                self.capture_recovery_file(
                    plan,
                    &absent.path,
                    "readDependency",
                    None,
                    None,
                    None,
                    false,
                    &mut capture,
                )?;
                let member = capture.members.last_mut().unwrap();
                match ParentGuard::acquire(Path::new(&absent.path)) {
                    Ok(guard) => {
                        if guard.directory_identity()? != absent.parent {
                            member.state = "thirdState".into();
                            member.matches_before = false;
                            member.matches_after = false;
                            member.identity_binding = "mismatch".into();
                            member.description =
                                Some(UiMessage::recovery_absent_parent_replaced().into());
                        }
                        capture.guards.push(guard);
                    }
                    Err(error) => {
                        member.state = "unavailable".into();
                        member.matches_before = false;
                        member.matches_after = false;
                        member.identity_binding = "unavailable".into();
                        member.description = Some(
                            CoreError::new(
                                "dependencyParentUnavailable",
                                UiMessage::recovery_absent_parent_unavailable(),
                            )
                            .caused_by(error)
                            .into(),
                        );
                    }
                }
            }
        }
        for directory in &t.directories {
            if matches!(t.mutation, Mutation::Compound { .. })
                && t.relocations().iter().any(|step| {
                    crate::history::history_relative_path(
                        Path::new(&directory.path),
                        Path::new(step.source),
                    )
                    .is_some()
                })
            {
                continue;
            }
            let mut member = directory_member(
                &directory.path,
                "readDependency",
                Some(&directory.identity),
                Some(&directory.identity),
            );
            match ParentGuard::acquire(&Path::new(&directory.path).join(".recovery-prospective")) {
                Ok(guard) => {
                    let identity = guard.directory_identity()?;
                    let (entries, complete) = guard.directory_entries(5000)?;
                    let names = operations::sorted_entries(entries, Path::new(&directory.path));
                    let same = complete
                        && identity == directory.identity
                        && crate::author_order::navigation_order_directory_entries(
                            t,
                            Path::new(&directory.path),
                            names,
                        ) == crate::author_order::navigation_order_directory_entries(
                            t,
                            Path::new(&directory.path),
                            directory.entries.clone(),
                        );
                    member.current = directory_blob(Some(&identity));
                    member.current_identity = Some((&identity).into());
                    member.matches_before = same;
                    member.matches_after = same;
                    member.state = if same { "knownBefore" } else { "thirdState" }.into();
                    member.identity_binding = if identity == directory.identity {
                        "original"
                    } else {
                        "mismatch"
                    }
                    .into();
                    member.description = Some(
                        if same {
                            UiMessage::recovery_directory_matches()
                        } else {
                            UiMessage::recovery_directory_changed()
                        }
                        .into(),
                    );
                    capture.guards.push(guard);
                }
                Err(e) => {
                    member.state = "unavailable".into();
                    member.description = Some(e.into());
                }
            }
            capture.members.push(member);
        }
        capture.complete();
        Ok(capture)
    }
    #[allow(clippy::too_many_arguments)]
    fn capture_recovery_file(
        &self,
        plan: &Plan,
        path: &str,
        role: &str,
        before: Option<&[u8]>,
        after: Option<&[u8]>,
        original: Option<&FileIdentity>,
        write: bool,
        capture: &mut RecoveryCapture,
    ) -> Result<()> {
        let materialized = self.materialized_identity(&plan.operation_id, Path::new(path))?;
        let expected = original.or(materialized.as_ref());
        let mut member = RecoveryMember {
            kind: "file".into(),
            role: role.into(),
            path: path.into(),
            target_path: None,
            before: RecoveryBlob::from_bytes(before),
            after: RecoveryBlob::from_bytes(after),
            current: RecoveryBlob::from_bytes(None),
            state: "unavailable".into(),
            matches_before: false,
            matches_after: false,
            identity_binding: "unavailable".into(),
            expected_identity: expected.map(RecoveryIdentity::from),
            current_identity: None,
            description: None,
        };
        match ParentGuard::acquire(Path::new(path)) {
            Ok(guard) => {
                let opened = if !write
                    && plan.transaction.as_ref().is_some_and(|t| {
                        matches!(t.request.intent, OperationIntent::BackupLibrary { .. })
                            || crate::operations::operation_read_control(
                                Path::new(&t.request.root),
                                Path::new(path),
                            )
                    }) {
                    guard.open_read_guard_file()
                } else {
                    guard.open_file(write, false)
                };
                match opened {
                    Ok(mut file) => {
                        let observed: Result<(FileStamp, Vec<u8>)> =
                            (|| Ok((stamp(&file)?, read_blob(&mut file)?)))();
                        match observed {
                            Ok((observed_stamp, bytes)) => {
                                let identity = observed_stamp.identity.clone();
                                let bound = expected == Some(&identity);
                                let exact_before_stamp = plan
                                    .transaction
                                    .as_ref()
                                    .filter(|t| matches!(t.mutation, Mutation::Compound { .. }))
                                    .and_then(|t| t.files.iter().find(|f| f.path == path))
                                    .is_none_or(|f| f.stamp == observed_stamp);
                                member.current = RecoveryBlob::from_bytes(Some(&bytes));
                                member.current_identity = Some((&identity).into());
                                member.identity_binding = if !bound {
                                    "mismatch"
                                } else if original.is_some() {
                                    "original"
                                } else {
                                    "materialized"
                                }
                                .into();
                                let empty_materialization = original.is_none()
                                    && materialized.is_some()
                                    && before.is_none()
                                    && bytes.is_empty()
                                    && plan.transaction.as_ref().is_some_and(|t| {
                                        matches!(t.mutation, Mutation::Compound { .. })
                                            || matches!(
                                                t.request.intent,
                                                OperationIntent::CreateLibrary { .. }
                                            )
                                    });
                                member.matches_before = bound
                                    && exact_before_stamp
                                    && (before == Some(bytes.as_slice()) || empty_materialization);
                                member.matches_after = bound
                                    && (before != after || exact_before_stamp)
                                    && after == Some(bytes.as_slice());
                                member.state = if member.matches_before {
                                    "knownBefore"
                                } else if member.matches_after {
                                    "knownAfter"
                                } else {
                                    "thirdState"
                                }
                                .into();
                                member.description = Some(
                                    if bound {
                                        UiMessage::recovery_current_bytes_guarded()
                                    } else {
                                        UiMessage::recovery_object_binding_unproven()
                                    }
                                    .into(),
                                );
                            }
                            Err(e) => member.description = Some(e.into()),
                        }
                        capture.files.push((capture.members.len(), file));
                    }
                    Err(e) if e.code == "missing" => {
                        member.matches_before =
                            before.is_none() && (original.is_some() || materialized.is_none());
                        member.matches_after = after.is_none();
                        member.state = if member.matches_before {
                            "knownBefore"
                        } else if member.matches_after {
                            "knownAfter"
                        } else {
                            "thirdState"
                        }
                        .into();
                        member.identity_binding = "missing".into();
                        member.description = Some(
                            if materialized.is_some() && original.is_none() {
                                UiMessage::recovery_materialized_object_missing()
                            } else {
                                UiMessage::recovery_parent_has_no_member()
                            }
                            .into(),
                        );
                    }
                    Err(e) => member.description = Some(e.into()),
                }
                capture.guards.push(guard);
            }
            Err(e) if e.code == "missing" && (before.is_none() || after.is_none()) => {
                // Create-tree absence is additionally proven by its guarded root
                // member. No arbitrary replacement parent is followed here.
                member.matches_before =
                    before.is_none() && (original.is_some() || materialized.is_none());
                member.matches_after = after.is_none();
                member.state = if member.matches_before {
                    "knownBefore"
                } else if member.matches_after {
                    "knownAfter"
                } else {
                    "thirdState"
                }
                .into();
                member.identity_binding = "missing".into();
                member.description = Some(UiMessage::recovery_member_parent_absent().into());
            }
            Err(e) => member.description = Some(e.into()),
        }
        capture.members.push(member);
        Ok(())
    }
    fn capture_created_directory(
        &self,
        plan: &Plan,
        path: &str,
        capture: &mut RecoveryCapture,
    ) -> Result<()> {
        let expected = self.materialized_identity(&plan.operation_id, Path::new(path))?;
        let mut member = directory_member(path, "write", None, expected.as_ref());
        if expected.is_none() {
            member.after = RecoveryBlob::from_bytes(Some(
                b"{\"kind\":\"directory\",\"identity\":\"allocatedByCreateOnly\"}",
            ));
        }
        match ParentGuard::acquire(&Path::new(path).join(".recovery-prospective")) {
            Ok(guard) => {
                let identity = guard.directory_identity()?;
                let bound = expected.as_ref() == Some(&identity);
                member.current = directory_blob(Some(&identity));
                member.current_identity = Some((&identity).into());
                member.matches_after = bound;
                member.state = if bound { "knownAfter" } else { "thirdState" }.into();
                member.identity_binding = if bound { "materialized" } else { "mismatch" }.into();
                member.description =
                    Some(UiMessage::recovery_created_directory_binding_required().into());
                capture.guards.push(guard);
            }
            Err(e) if e.code == "missing" => {
                member.matches_before = expected.is_none();
                member.state = if expected.is_none() {
                    "knownBefore"
                } else {
                    "thirdState"
                }
                .into();
                member.identity_binding = "missing".into();
                member.description = Some(
                    if expected.is_none() {
                        UiMessage::recovery_planned_directory_absent()
                    } else {
                        UiMessage::recovery_materialized_directory_missing()
                    }
                    .into(),
                );
                if let Ok(parent) = ParentGuard::acquire(Path::new(path)) {
                    capture.guards.push(parent);
                }
            }
            Err(e) => member.description = Some(e.into()),
        }
        capture.members.push(member);
        Ok(())
    }
    fn capture_relocation(
        &self,
        plan: &Plan,
        capture: &mut RecoveryCapture,
        write: bool,
    ) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let Mutation::Relocate {
            source,
            target,
            source_identity,
            old_leaf,
            new_leaf,
            leaf_identity,
            target_parent_identity,
            create_trash_parent,
            updates,
            members,
        } = &t.mutation
        else {
            unreachable!()
        };
        let canonical = t
            .files
            .iter()
            .find(|f| Path::new(&f.path) == Path::new(source).join(old_leaf))
            .ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_transaction_recovery_missing_relocation_canonical_preimage(),
                )
            })?;
        if source == target {
            self.capture_bound_directory(source, source_identity, capture)?;
            self.capture_recovery_file(
                plan,
                &canonical.path,
                "namespace",
                Some(&canonical.bytes),
                Some(&canonical.bytes),
                Some(leaf_identity),
                false,
                capture,
            )?;
            return Ok(());
        }
        let target_parent = Path::new(target).parent().unwrap();
        if *create_trash_parent {
            self.capture_bound_directory(
                &display(target_parent.parent().unwrap()),
                target_parent_identity,
                capture,
            )?;
            self.capture_created_directory(plan, &display(target_parent), capture)?;
        } else {
            self.capture_bound_directory(&display(target_parent), target_parent_identity, capture)?;
        }
        let creations_at = |path: &Path| -> BTreeSet<String> {
            updates
                .iter()
                .filter(|c| c.before.is_none() && Path::new(&c.path).parent() == Some(path))
                .map(|c| {
                    Path::new(&c.path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        };
        let root_entries = t
            .directories
            .iter()
            .find(|d| Path::new(&d.path) == Path::new(source))
            .map(|d| &d.entries);
        let renamed_entries = root_entries.map(|entries| {
            let mut entries = entries.clone();
            for entry in &mut entries {
                if let Some(attrs) = entry.strip_prefix(&format!("{old_leaf}:")) {
                    *entry = format!("{new_leaf}:{attrs}");
                }
            }
            entries.sort();
            entries
        });
        self.capture_namespace_directory(
            source,
            Some(source_identity),
            None,
            root_entries,
            &BTreeSet::new(),
            capture,
        )?;
        self.capture_namespace_directory(
            target,
            None,
            Some(source_identity),
            renamed_entries.as_ref(),
            &creations_at(Path::new(target)),
            capture,
        )?;
        let moved = capture.members.last().is_some_and(|m| m.matches_after);
        for member in members {
            let locator =
                Path::new(if moved { target } else { source }).join(&member.relative_path);
            let mut fact = directory_member(
                &display(&locator),
                "placement",
                Some(&member.identity),
                Some(&member.identity),
            );
            fact.kind = "file".into();
            match ParentGuard::acquire(&locator).and_then(|g| {
                if g.directory_identity()? != member.parent {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_transaction_recovery_placement_member_parent_changed(
                        ),
                    ));
                }
                let h = g.open_placement_file()?;
                if stamp(&h)?.identity != member.identity {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_transaction_recovery_placement_member_object_changed(
                        ),
                    ));
                }
                Ok((g, h))
            }) {
                Ok((g, h)) => {
                    fact.matches_before = !moved;
                    fact.matches_after = moved;
                    fact.state = if moved { "knownAfter" } else { "knownBefore" }.into();
                    fact.identity_binding = "original".into();
                    capture.guards.push(g);
                    capture.placement_files.push(h);
                }
                Err(e) => {
                    fact.state = if e.code == "thirdState" {
                        "thirdState"
                    } else {
                        "unavailable"
                    }
                    .into();
                    fact.description = Some(e.into());
                }
            }
            capture.members.push(fact);
        }
        for directory in t
            .directories
            .iter()
            .filter(|d| Path::new(&d.path) != Path::new(source))
        {
            let relative = Path::new(&directory.path)
                .strip_prefix(source)
                .map_err(|_| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_transaction_recovery_directory_outside_original_move(
                        ),
                    )
                })?;
            let destination = Path::new(target).join(relative);
            self.capture_namespace_directory(
                &directory.path,
                Some(&directory.identity),
                None,
                Some(&directory.entries),
                &BTreeSet::new(),
                capture,
            )?;
            self.capture_namespace_directory(
                &display(&destination),
                None,
                Some(&directory.identity),
                Some(&directory.entries),
                &creations_at(&destination),
                capture,
            )?;
        }
        for (path, identity) in &t.write_parents {
            self.capture_bound_directory(
                &display(Path::new(path).parent().unwrap()),
                identity,
                capture,
            )?;
        }
        for file in &t.files {
            let final_path = t.relocated_path(&file.path);
            let update = updates.iter().find(|c| c.path == final_path);
            let after = update.map_or(file.bytes.as_slice(), |c| c.after.as_slice());
            let changed = after != file.bytes;
            if !Path::new(&file.path).starts_with(source) {
                self.capture_recovery_file(
                    plan,
                    &file.path,
                    if changed { "write" } else { "readDependency" },
                    Some(&file.bytes),
                    Some(after),
                    Some(&file.stamp.identity),
                    write && changed,
                    capture,
                )?;
                continue;
            }
            self.capture_recovery_file(
                plan,
                &file.path,
                "namespace",
                Some(&file.bytes),
                None,
                Some(&file.stamp.identity),
                false,
                capture,
            )?;
            self.capture_recovery_file(
                plan,
                &final_path,
                if moved && changed {
                    "write"
                } else {
                    "namespace"
                },
                if moved && changed {
                    Some(&file.bytes)
                } else {
                    None
                },
                Some(after),
                Some(&file.stamp.identity),
                write && moved && changed,
                capture,
            )?;
        }
        for change in updates.iter().filter(|c| c.before.is_none()) {
            self.capture_recovery_file(
                plan,
                &change.path,
                "write",
                None,
                Some(&change.after),
                None,
                write,
                capture,
            )?;
        }
        if old_leaf != new_leaf {
            self.capture_recovery_file(
                plan,
                &display(&Path::new(source).join(new_leaf)),
                "namespaceStep",
                None,
                None,
                Some(leaf_identity),
                false,
                capture,
            )?;
            let intermediate = capture.members.last_mut().unwrap();
            if intermediate.current_identity.as_ref().is_some_and(|i| {
                i.volume == leaf_identity.volume
                    && i.index == leaf_identity.index.to_string()
                    && i.created == leaf_identity.created.to_string()
            }) && intermediate.current.sha256.as_deref() == Some(hash(&canonical.bytes).as_str())
            {
                intermediate.state = "knownIntermediate".into();
                intermediate.description =
                    Some(UiMessage::recovery_canonical_rename_intermediate().into());
            }
        }
        Ok(())
    }
    fn capture_namespace_directory(
        &self,
        path: &str,
        before: Option<&FileIdentity>,
        after: Option<&FileIdentity>,
        entries: Option<&Vec<String>>,
        introduced: &BTreeSet<String>,
        capture: &mut RecoveryCapture,
    ) -> Result<()> {
        let expected = before.or(after).unwrap();
        let mut member = directory_member(path, "namespace", before, after);
        match ParentGuard::acquire(&Path::new(path).join(".recovery-prospective")) {
            Ok(guard) => {
                let identity = guard.directory_identity()?;
                member.current = directory_blob(Some(&identity));
                member.current_identity = Some((&identity).into());
                member.matches_before = before == Some(&identity);
                member.matches_after = after == Some(&identity);
                member.state = if member.matches_before {
                    "knownBefore"
                } else if member.matches_after {
                    "knownAfter"
                } else {
                    "thirdState"
                }
                .into();
                member.identity_binding = if &identity == expected {
                    "original"
                } else {
                    "mismatch"
                }
                .into();
                if let Some(entries) = entries {
                    let (mut actual, complete) = guard.directory_entries(5000)?;
                    actual
                        .retain(|(name, _)| !introduced.contains(name.to_string_lossy().as_ref()));
                    if !complete || operations::sorted_entries(actual, Path::new(path)) != *entries
                    {
                        member.matches_before = false;
                        member.matches_after = false;
                        member.state = "thirdState".into();
                        member.description =
                            Some(UiMessage::recovery_relocation_membership_changed().into());
                    }
                }
                capture.guards.push(guard);
            }
            Err(e) if e.code == "missing" => {
                member.matches_before = before.is_none();
                member.matches_after = after.is_none();
                member.state = if member.matches_before {
                    "knownBefore"
                } else {
                    "knownAfter"
                }
                .into();
                member.identity_binding = "missing".into();
                if let Ok(parent) = ParentGuard::acquire(Path::new(path)) {
                    capture.guards.push(parent);
                }
            }
            Err(e) => member.description = Some(e.into()),
        }
        capture.members.push(member);
        Ok(())
    }
    fn capture_bound_directory(
        &self,
        path: &str,
        expected: &FileIdentity,
        capture: &mut RecoveryCapture,
    ) -> Result<()> {
        let mut member = directory_member(path, "readDependency", Some(expected), Some(expected));
        match ParentGuard::acquire(&Path::new(path).join(".recovery-prospective")) {
            Ok(guard) => {
                let identity = guard.directory_identity()?;
                let bound = &identity == expected;
                member.current = directory_blob(Some(&identity));
                member.current_identity = Some((&identity).into());
                member.matches_before = bound;
                member.matches_after = bound;
                member.state = if bound { "knownBefore" } else { "thirdState" }.into();
                member.identity_binding = if bound { "original" } else { "mismatch" }.into();
                capture.guards.push(guard);
            }
            Err(e) => member.description = Some(e.into()),
        }
        capture.members.push(member);
        Ok(())
    }
}
fn can_resume_content(t: &operations::FrozenOperation, capture: &RecoveryCapture) -> bool {
    match &t.mutation {
        Mutation::Update { .. } => true,
        Mutation::Compound { .. } => {
            capture.compound.is_some()
                && !capture
                    .members
                    .iter()
                    .any(|member| matches!(member.state.as_str(), "thirdState" | "unavailable"))
        }
        Mutation::Relocate { updates, .. } if !updates.is_empty() => {
            capture.members.iter().all(|m| {
                if m.role == "write" {
                    m.matches_before || m.matches_after
                } else {
                    m.matches_after
                }
            })
        }
        _ => false,
    }
}
fn directory_blob(identity: Option<&FileIdentity>) -> RecoveryBlob {
    let bytes = identity.map(|i| {
        serde_json::to_vec(&serde_json::json!({"kind":"directory","identity":i})).unwrap()
    });
    RecoveryBlob::from_bytes(bytes.as_deref())
}
fn recovery_failure(plan: &Plan, error: CoreError) -> CoreError {
    CoreError::new(
        "recoveryPending",
        UiMessage::recovery_operation_unresolved(
            weftext_messages::params::RecoveryOperationUnresolved {
                operation_id: plan.operation_id.clone(),
            },
        ),
    )
    .caused_by(error)
}
fn directory_member(
    path: &str,
    role: &str,
    before: Option<&FileIdentity>,
    after: Option<&FileIdentity>,
) -> RecoveryMember {
    RecoveryMember {
        kind: "directory".into(),
        role: role.into(),
        path: path.into(),
        target_path: None,
        before: directory_blob(before),
        after: if after.is_some() {
            directory_blob(after)
        } else {
            RecoveryBlob::from_bytes(None)
        },
        current: RecoveryBlob::from_bytes(None),
        state: "unavailable".into(),
        matches_before: false,
        matches_after: false,
        identity_binding: "unavailable".into(),
        expected_identity: before.or(after).map(RecoveryIdentity::from),
        current_identity: None,
        description: None,
    }
}
