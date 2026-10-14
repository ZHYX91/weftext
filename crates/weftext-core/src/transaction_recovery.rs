//! Explicit recovery of the original frozen decision. Inspection never writes
//! user files; continuation holds every original participant until commit.
use super::*;
use journal::{Plan, RecoveryBlob, RecoveryIdentity, RecoveryMember};
use operations::{FileChange, Mutation, read_blob, write_blob};

struct RecoveryCapture {
    members: Vec<RecoveryMember>,
    guards: Vec<ParentGuard>,
    files: Vec<(usize, File)>,
    changes: Vec<FileChange>,
    coherence: String,
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
        } else if self.resolved(&plan.operation_id) {
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
        if !self.committed(&plan.operation_id) && !self.resolved(&plan.operation_id) {
            if capture.coherence == "knownBefore" {
                allowed_actions.push("keepCurrent".into());
            }
            if installing && capture.members.iter().all(|m| m.matches_after) {
                allowed_actions.push("adoptAfterImages".into());
            }
            if installing
                && capture.coherence == "mixedKnown"
                && matches!(t.mutation, Mutation::Update { .. })
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
            message: format!(
                "Original frozen plan {} retained. Every member must keep its recorded object binding and exact before/after bytes; third states cannot be overwritten or accepted as coherent.",
                t.preview.plan_id
            ),
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
                "Choose an action permitted by the inspected original plan",
            ));
        }
        if self.committed(&plan.operation_id) || self.resolved(&plan.operation_id) {
            return self.inspect_transaction_recovery(plan);
        }
        let t = plan.transaction.as_ref().unwrap();
        let mut capture =
            self.capture_transaction_recovery(plan, action == "resumeOriginalPlan")?;
        let installing = self.journal_path(&plan.operation_id, "install").exists();
        if action == "keepCurrent" {
            if capture.coherence != "knownBefore" {
                return Err(CoreError::new(
                    "recoveryConflict",
                    "Keeping current is permitted only when all original members are coherently at their verified before state",
                ));
            }
            self.resolve_record(&plan.operation_id, action)?;
        } else {
            let eligible = installing
                && match action {
                    "adoptAfterImages" => capture.members.iter().all(|m| m.matches_after),
                    "resumeOriginalPlan" => {
                        capture.coherence == "mixedKnown"
                            && matches!(t.mutation, Mutation::Update { .. })
                    }
                    _ => false,
                };
            if !eligible {
                return Err(CoreError::new(
                    "recoveryConflict",
                    "The original plan has unavailable, changed or incoherent members; preserve current and original before/after images for explicit repair",
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
                        "Replica setup binding is unavailable",
                    ));
                }
            } else {
                let workspace = self.workspace(Path::new(&t.request.root))?.ok_or_else(|| {
                    CoreError::new(
                        "recoveryConflict",
                        "The original committed workspace binding is unavailable",
                    )
                })?;
                if plan.workspace_id.as_ref() != Some(&workspace.workspace_id) {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        "The original workspace identity changed",
                    ));
                }
            }
            if action == "resumeOriginalPlan" {
                // The read set and every existing write member are protected
                // before the first continuation write. No new ID/plan is made.
                for change in &capture.changes {
                    let index = capture
                        .members
                        .iter()
                        .position(|m| m.role == "write" && m.path == change.path)
                        .ok_or_else(|| {
                            CoreError::new("invalidPlan", "A frozen change lacks a recovery member")
                        })?;
                    if capture.members[index].matches_after {
                        continue;
                    }
                    if let Some((_, file)) = capture.files.iter_mut().find(|(i, _)| *i == index) {
                        write_blob(file, &change.after).map_err(|e| recovery_failure(plan, e))?;
                    } else {
                        let guard = ParentGuard::acquire(Path::new(&change.path))?;
                        let mut file = guard
                            .open_file(true, true)
                            .map_err(|e| recovery_failure(plan, e))?;
                        self.record_materialized(
                            &plan.operation_id,
                            Path::new(&change.path),
                            &file,
                        )
                        .map_err(|e| recovery_failure(plan, e))?;
                        write_blob(&mut file, &change.after)
                            .map_err(|e| recovery_failure(plan, e))?;
                        capture.files.push((index, file));
                        capture.guards.push(guard);
                    }
                    capture.members[index].matches_after = true;
                }
            }
            let mut materialized = Vec::new();
            for (index, file) in &mut capture.files {
                let member = &capture.members[*index];
                if member.role == "write"
                    || (matches!(t.mutation, Mutation::Relocate { .. })
                        && t.preview.revisions.contains_key(&member.path))
                {
                    let change = capture
                        .changes
                        .iter()
                        .find(|c| c.path == member.path)
                        .ok_or_else(|| {
                            CoreError::new(
                                "invalidPlan",
                                "A recovery write has no exact after image",
                            )
                        })?;
                    if read_blob(file)? != change.after {
                        return Err(CoreError::new(
                            "recoveryConflict",
                            "After verification changed before the original decision",
                        ));
                    }
                    materialized.push((member.path.clone(), stamp(file)?));
                }
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
    fn capture_transaction_recovery(&self, plan: &Plan, write: bool) -> Result<RecoveryCapture> {
        let t = plan.transaction.as_ref().unwrap();
        let mut capture = RecoveryCapture {
            members: Vec::new(),
            guards: Vec::new(),
            files: Vec::new(),
            changes: Vec::new(),
            coherence: String::new(),
        };
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
                message: error.message,
            });
        }
        for other in self.operations.values().filter(|p| {
            p.operation_id != plan.operation_id
                && !self.committed(&p.operation_id)
                && !self.resolved(&p.operation_id)
                && (p.transaction.is_none()
                    || self.journal_path(&p.operation_id, "install").exists())
        }) {
            let overlap = t.files.iter().any(|f| {
                other.path.eq_ignore_ascii_case(&f.path)
                    || other.identity.as_ref() == Some(&f.stamp.identity)
                    || other.transaction.as_ref().is_some_and(|pending| {
                        pending.affects(Path::new(&f.path))
                            || pending.has_identity(&f.stamp.identity)
                    })
            }) || other.transaction.as_ref().is_some_and(|pending| {
                t.preview
                    .changes
                    .iter()
                    .any(|c| pending.affects(Path::new(&c.path)))
            });
            if overlap {
                capture.members.push(RecoveryMember {
                    kind: "barrier".into(),
                    role: "readDependency".into(),
                    path: other.path.clone(),
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
                    message: format!(
                        "Another original operation {} has unresolved effects in this scope",
                        other.operation_id
                    ),
                });
            }
        }
        match &t.mutation {
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
                self.capture_relocation(plan, &mut capture)?;
                capture.complete();
                return Ok(capture);
            }
        }
        for (path, expected_parent) in &t.write_parents {
            let parent = Path::new(path).parent().ok_or_else(|| {
                CoreError::new("invalidPlan", "A frozen write target lacks a parent")
            })?;
            self.capture_bound_directory(&display(parent), expected_parent, &mut capture)?;
        }
        for change in &capture.changes {
            let is_created_output = matches!(&t.mutation, Mutation::CreateTree { files, .. } if files.iter().any(|c|c.path==change.path));
            if !is_created_output && !t.write_parents.iter().any(|(path, _)| path == &change.path) {
                capture.members.push(RecoveryMember {
                    kind: "barrier".into(), role: "readDependency".into(), path: change.path.clone(), target_path: None,
                    before: RecoveryBlob::from_bytes(None), after: RecoveryBlob::from_bytes(None), current: RecoveryBlob::from_bytes(None),
                    state: "unavailable".into(), matches_before: false, matches_after: false,
                    identity_binding: "unavailable".into(), expected_identity: None, current_identity: None,
                    message: "The original plan has no durable write-parent binding; path names or equal leaf bytes cannot supply it".into(),
                });
            }
        }
        for change in capture.changes.clone() {
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
            if !capture.changes.iter().any(|c| c.path == file.path) {
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
                if let Ok(guard) = ParentGuard::acquire(Path::new(&absent.path)) {
                    if guard.directory_identity()? != absent.parent {
                        member.state = "thirdState".into();
                        member.matches_before = false;
                        member.matches_after = false;
                        member.identity_binding = "mismatch".into();
                        member.message = "Original absent dependency parent was replaced".into();
                    }
                    capture.guards.push(guard);
                } else {
                    member.state = "unavailable".into();
                    member.matches_before = false;
                    member.matches_after = false;
                    member.identity_binding = "unavailable".into();
                    member.message = "Original absent dependency parent is unavailable".into();
                }
            }
        }
        for directory in &t.directories {
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
                    let mut names = operations::sorted_entries(entries, Path::new(&directory.path));
                    if matches!(t.request.intent, OperationIntent::BackupLibrary { .. })
                        && Path::new(&directory.path) == self.data_dir.join("journal")
                    {
                        names.retain(|name| !name.starts_with(&format!("{}.", plan.operation_id)));
                    }
                    let same =
                        complete && identity == directory.identity && names == directory.entries;
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
                    member.message = if same {
                        "Original guarded directory object and complete entries match"
                    } else {
                        "Frozen directory object or complete membership changed"
                    }
                    .into();
                    capture.guards.push(guard);
                }
                Err(e) => {
                    member.state = "unavailable".into();
                    member.message = e.message;
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
            message: String::new(),
        };
        match ParentGuard::acquire(Path::new(path)) {
            Ok(guard) => {
                match guard.open_file(write, false) {
                    Ok(mut file) => {
                        let observed: Result<(FileIdentity, Vec<u8>)> =
                            (|| Ok((stamp(&file)?.identity, read_blob(&mut file)?)))();
                        match observed {
                            Ok((identity, bytes)) => {
                                let bound = expected == Some(&identity);
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
                                member.matches_before = bound && before == Some(bytes.as_slice());
                                member.matches_after = bound && after == Some(bytes.as_slice());
                                member.state = if member.matches_before {
                                    "knownBefore"
                                } else if member.matches_after {
                                    "knownAfter"
                                } else {
                                    "thirdState"
                                }
                                .into();
                                member.message = if bound { "Exact current bytes read through the retained original object guard" } else { "Existing object has no matching original or create-only materialization record" }.into();
                            }
                            Err(e) => member.message = e.message,
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
                        member.message = if materialized.is_some() && original.is_none() {
                            "The original materialized object is now missing; a new object cannot replace its retained binding"
                        } else { "Actual protected parent has no member with this name" }.into();
                    }
                    Err(e) => member.message = e.message,
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
                member.message =
                    "Planned member parent is absent; the root before-state must also be verified"
                        .into();
            }
            Err(e) => member.message = e.message,
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
                member.message =
                    "Created directory must match its durable create-only object record".into();
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
                member.message = if expected.is_none() { "Planned directory is absent" } else { "The originally materialized directory is missing; replacement cannot supply its original binding" }.into();
                if let Ok(parent) = ParentGuard::acquire(Path::new(path)) {
                    capture.guards.push(parent);
                }
            }
            Err(e) => member.message = e.message,
        }
        capture.members.push(member);
        Ok(())
    }
    fn capture_relocation(&self, plan: &Plan, capture: &mut RecoveryCapture) -> Result<()> {
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
            ..
        } = &t.mutation
        else {
            unreachable!()
        };
        let bytes = t
            .files
            .iter()
            .find(|f| f.path == display(&Path::new(source).join(old_leaf)))
            .ok_or_else(|| CoreError::new("invalidPlan", "Missing relocation canonical preimage"))?
            .bytes
            .as_slice();
        if source == target {
            self.capture_bound_directory(source, source_identity, capture)?;
            self.capture_recovery_file(
                plan,
                &display(&Path::new(source).join(old_leaf)),
                "namespace",
                Some(bytes),
                Some(bytes),
                Some(leaf_identity),
                false,
                capture,
            )?;
            return Ok(());
        }
        let target_parent = Path::new(target)
            .parent()
            .ok_or_else(|| CoreError::new("invalidPlan", "Missing relocation target parent"))?;
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
            capture,
        )?;
        self.capture_namespace_directory(
            target,
            None,
            Some(source_identity),
            renamed_entries.as_ref(),
            capture,
        )?;
        for directory in t
            .directories
            .iter()
            .filter(|d| Path::new(&d.path) != Path::new(source))
        {
            let relative = Path::new(&directory.path)
                .strip_prefix(source)
                .map_err(|_| CoreError::new("invalidPlan", "Directory outside original move"))?;
            self.capture_namespace_directory(
                &directory.path,
                Some(&directory.identity),
                None,
                Some(&directory.entries),
                capture,
            )?;
            self.capture_namespace_directory(
                &display(&Path::new(target).join(relative)),
                None,
                Some(&directory.identity),
                Some(&directory.entries),
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
            if !Path::new(&file.path).starts_with(source) {
                self.capture_recovery_file(
                    plan,
                    &file.path,
                    "readDependency",
                    Some(&file.bytes),
                    Some(&file.bytes),
                    Some(&file.stamp.identity),
                    false,
                    capture,
                )?;
                continue;
            }
            let relative = Path::new(&file.path)
                .strip_prefix(source)
                .map_err(|_| CoreError::new("invalidPlan", "File outside original move"))?;
            if relative == Path::new(old_leaf) {
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
                &display(&Path::new(target).join(relative)),
                "namespace",
                None,
                Some(&file.bytes),
                Some(&file.stamp.identity),
                false,
                capture,
            )?;
        }
        self.capture_recovery_file(
            plan,
            &display(&Path::new(source).join(old_leaf)),
            "namespace",
            Some(bytes),
            None,
            Some(leaf_identity),
            false,
            capture,
        )?;
        self.capture_recovery_file(
            plan,
            &display(&Path::new(target).join(new_leaf)),
            "namespace",
            None,
            Some(bytes),
            Some(leaf_identity),
            false,
            capture,
        )?;
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
            }) && intermediate.current.sha256.as_deref() == Some(hash(bytes).as_str())
            {
                intermediate.state = "knownIntermediate".into();
                intermediate.message = "The original canonical object is at the intermediate leaf name; namespace recovery remains pending and no replay is permitted".into();
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
                    let (actual, complete) = guard.directory_entries(5000)?;
                    if !complete || operations::sorted_entries(actual, Path::new(path)) != *entries
                    {
                        member.matches_before = false;
                        member.matches_after = false;
                        member.state = "thirdState".into();
                        member.message =
                            "Relocation directory membership differs from the original frozen tree"
                                .into();
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
            Err(e) => member.message = e.message,
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
            Err(e) => member.message = e.message,
        }
        capture.members.push(member);
        Ok(())
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
        format!(
            "Original operation {} remains unresolved after continuation; preserve its plan and inspect its members. Underlying {}: {}",
            plan.operation_id, error.code, error.message
        ),
    )
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
        message: String::new(),
    }
}
