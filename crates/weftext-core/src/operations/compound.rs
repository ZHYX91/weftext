//! One original sync namespace/content decision, including its interrupted steps.
use super::*;
use crate::backend::MultiNamespaceGuard;
use crate::journal::{Plan, RecoveryBlob, RecoveryIdentity, RecoveryMember};
use crate::transaction_recovery::RecoveryCapture;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepPhase {
    Before,
    LeafRenamed,
    After,
}
pub(crate) struct CompoundCapture {
    namespace: MultiNamespaceGuard,
    phases: Vec<StepPhase>,
}

fn steps(t: &FrozenOperation) -> Result<&[RelocationStep]> {
    match &t.mutation {
        Mutation::Compound { relocations, .. }
            if relocations.len() >= 2 && relocations.len() <= MAX_OPERATION_FILES =>
        {
            Ok(relocations)
        }
        _ => Err(CoreError::new(
            "invalidPlan",
            "Compound requires bounded independent original placements",
        )),
    }
}
fn relative(path: &str, root: &str) -> Option<PathBuf> {
    crate::history::history_relative_path(Path::new(path), Path::new(root))
}
fn member(
    path: &str,
    role: &str,
    before: Option<&[u8]>,
    after: Option<&[u8]>,
    expected: Option<&FileIdentity>,
) -> RecoveryMember {
    RecoveryMember {
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
    }
}
fn bound_directory(path: &Path) -> Result<Option<FileIdentity>> {
    match ParentGuard::acquire(&path.join(".compound-directory-prospective")) {
        Ok(guard) => Ok(Some(guard.directory_identity()?)),
        Err(error) if error.code == "missing" => Ok(None),
        Err(error) => Err(error),
    }
}
fn phase(step: &RelocationStep) -> Result<StepPhase> {
    let source = Path::new(&step.source);
    let target = Path::new(&step.target);
    match (bound_directory(source)?, bound_directory(target)?) {
        (Some(identity), None) if identity == step.source_identity => {
            if step.old_leaf == step.new_leaf {
                return Ok(StepPhase::Before);
            }
            let old = capture_optional(&source.join(&step.old_leaf))?;
            let new = capture_optional(&source.join(&step.new_leaf))?;
            match (old, new) {
                (Some(file), None) if file.stamp.identity == step.leaf_identity => {
                    Ok(StepPhase::Before)
                }
                (None, Some(file)) if file.stamp.identity == step.leaf_identity => {
                    Ok(StepPhase::LeafRenamed)
                }
                _ => Err(CoreError::new(
                    "thirdState",
                    "Canonical namespace is not an original before/intermediate state",
                )),
            }
        }
        (None, Some(identity)) if identity == step.source_identity => Ok(StepPhase::After),
        _ => Err(CoreError::new(
            "thirdState",
            "Original moving root is missing, replaced, or its other location is occupied",
        )),
    }
}
fn source_relative(
    file: &CapturedFile,
    step: &RelocationStep,
    phase: StepPhase,
) -> Option<PathBuf> {
    let path = relative(&file.path, &step.source)?;
    Some(
        if phase != StepPhase::Before && path == Path::new(&step.old_leaf) {
            PathBuf::from(&step.new_leaf)
        } else {
            path
        },
    )
}
fn update<'a>(t: &'a FrozenOperation, file: &CapturedFile) -> Option<&'a FileChange> {
    let path = t.relocated_path(&file.path);
    match &t.mutation {
        Mutation::Compound { updates, .. } => updates
            .iter()
            .find(|change| locator_equal(&change.path, &path)),
        _ => None,
    }
}
fn check_membership(
    namespace: &mut NamespaceGuard,
    t: &FrozenOperation,
    step: &RelocationStep,
    phase: StepPhase,
    include_creations: bool,
) -> Result<()> {
    for directory in &t.directories {
        let Some(path) = relative(&directory.path, &step.source) else {
            continue;
        };
        let (identity, mut entries, complete) =
            namespace.source_directory_snapshot(&path, MAX_OPERATION_FILES)?;
        let mut expected = directory.entries.clone();
        if phase != StepPhase::Before
            && path.as_os_str().is_empty()
            && step.old_leaf != step.new_leaf
        {
            for entry in &mut expected {
                if let Some(attrs) = entry.strip_prefix(&format!("{}:", step.old_leaf)) {
                    *entry = format!("{}:{attrs}", step.new_leaf);
                }
            }
            expected.sort();
        }
        if include_creations && let Mutation::Compound { updates, .. } = &t.mutation {
            let final_directory = Path::new(&step.target).join(&path);
            for change in updates.iter().filter(|change| {
                change.before.is_none()
                    && Path::new(&change.path).parent() == Some(final_directory.as_path())
            }) {
                let name = Path::new(&change.path).file_name().unwrap();
                // Missing is a known create-only before state; existing names
                // remain subject to original materialization/bytes checks.
                entries.retain(|(actual, _)| actual != name);
            }
        }
        if !complete
            || identity != directory.identity
            || sorted_entries(entries, Path::new(&directory.path)) != expected
        {
            return Err(CoreError::new(
                "scopeChanged",
                "Compound directory identity or complete membership changed",
            ));
        }
    }
    Ok(())
}
fn check_step_identity(
    union: &mut MultiNamespaceGuard,
    index: usize,
    step: &RelocationStep,
    phase: StepPhase,
) -> Result<()> {
    let parent = union.source_parent_identity(index)?;
    let namespace = union.select(index)?;
    if namespace.source_identity()? != step.source_identity
        || namespace.parent_identity()? != step.target_parent_identity
        || parent
            != if phase == StepPhase::After {
                step.target_parent_identity.clone()
            } else {
                step.source_parent_identity.clone()
            }
    {
        return Err(CoreError::new(
            "thirdState",
            "Compound original directory or parent identity changed",
        ));
    }
    // Both address slots are part of the same namespace closure. Equal bytes
    // elsewhere never license a second object occupying the vacated slot.
    ensure_absent(Path::new(if phase == StepPhase::After {
        &step.source
    } else {
        &step.target
    }))?;
    Ok(())
}
fn open_tree(
    namespace: &mut NamespaceGuard,
    t: &FrozenOperation,
    step: &RelocationStep,
    phase: StepPhase,
) -> Result<Vec<(usize, File)>> {
    check_membership(namespace, t, step, phase, false)?;
    let mut handles = Vec::new();
    for (index, file) in t.files.iter().enumerate() {
        let Some(path) = source_relative(file, step, phase) else {
            continue;
        };
        let write =
            update(t, file).is_some_and(|change| change.before.as_ref() != Some(&change.after));
        let mut handle = namespace.open_source_output(&path, write, false)?;
        if stamp(&handle)? != file.stamp || read_blob(&mut handle)? != file.bytes {
            return Err(CoreError::new(
                "conflict",
                "Compound member no longer has its original stamp and bytes",
            ));
        }
        handles.push((index, handle));
    }
    for entry in &step.members {
        let captured = t
            .files
            .iter()
            .find(|file| {
                relative(&file.path, &step.source).as_ref()
                    == Some(&PathBuf::from(&entry.relative_path))
            })
            .ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    "Placement member lacks the same original captured file",
                )
            })?;
        if captured.stamp.identity != entry.identity
            || namespace.source_parent_identity(Path::new(&entry.relative_path))? != entry.parent
        {
            return Err(CoreError::new(
                "thirdState",
                "Compound original placement member or parent changed",
            ));
        }
    }
    Ok(handles)
}
fn verify_creation_absence(
    namespace: &mut NamespaceGuard,
    t: &FrozenOperation,
    step: &RelocationStep,
) -> Result<()> {
    if step.old_leaf != step.new_leaf {
        match namespace.open_source_descendant(Path::new(&step.new_leaf)) {
            Err(error) if error.code == "missing" => {}
            Err(error) => return Err(error),
            Ok(_) => {
                return Err(CoreError::new(
                    "exists",
                    "Renamed canonical filename is occupied",
                ));
            }
        }
    }
    if let Mutation::Compound { updates, .. } = &t.mutation {
        for change in updates.iter().filter(|change| change.before.is_none()) {
            if let Some(path) = relative(&change.path, &step.target) {
                match namespace.open_source_descendant(&path) {
                    Err(error) if error.code == "missing" => {}
                    Err(error) => return Err(error),
                    Ok(_) => {
                        return Err(CoreError::new(
                            "exists",
                            "Compound create-only participant is occupied",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

impl Core {
    pub(crate) fn execute_compound(&mut self, plan: &Plan) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let relocations = steps(t)?;
        for step in relocations {
            self.check_subtree_boundary(Path::new(&t.request.root), Path::new(&step.source))?;
            self.owned_parent(
                Path::new(&t.request.root),
                Path::new(&step.target).parent().unwrap(),
            )?;
        }
        let pairs = relocations
            .iter()
            .map(|step| {
                (
                    Path::new(&step.source),
                    Path::new(&step.target).parent().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut union = MultiNamespaceGuard::acquire(&pairs)?;
        let mut outside_guards = Vec::new();
        let mut outside_handles = Vec::new();
        for (path, expected) in &t.write_parents {
            let guard = ParentGuard::acquire(Path::new(path))?;
            if guard.directory_identity()? != *expected {
                return Err(CoreError::new(
                    "scopeChanged",
                    "Compound external write parent changed",
                ));
            }
            outside_guards.push(guard);
        }
        for absent in &t.absent_dependencies {
            let guard = ParentGuard::acquire(Path::new(&absent.path))?;
            if guard.directory_identity()? != absent.parent {
                return Err(CoreError::new(
                    "scopeChanged",
                    "Compound absent dependency parent changed",
                ));
            }
            ensure_absent(Path::new(&absent.path))?;
            outside_guards.push(guard);
        }
        let mut tree_handles = Vec::new();
        for (index, step) in relocations.iter().enumerate() {
            check_step_identity(&mut union, index, step, StepPhase::Before)?;
            let namespace = union.select(index)?;
            tree_handles.extend(open_tree(namespace, t, step, StepPhase::Before)?);
            verify_creation_absence(namespace, t, step)?;
        }
        for (index, file) in t.files.iter().enumerate().filter(|(_, file)| {
            !relocations
                .iter()
                .any(|step| relative(&file.path, &step.source).is_some())
        }) {
            self.check_pending(Path::new(&file.path))?;
            let guard = ParentGuard::acquire(Path::new(&file.path))?;
            let write =
                update(t, file).is_some_and(|change| change.before.as_ref() != Some(&change.after));
            let mut handle = if !write
                && operation_read_control(Path::new(&t.request.root), Path::new(&file.path))
            {
                guard.open_read_guard_file()?
            } else {
                guard.open_file(write, false)?
            };
            if stamp(&handle)? != file.stamp || read_blob(&mut handle)? != file.bytes {
                return Err(CoreError::new(
                    "conflict",
                    "Compound external dependency changed",
                ));
            }
            outside_handles.push((index, handle));
            outside_guards.push(guard);
        }
        let Mutation::Compound { updates, .. } = &t.mutation else {
            unreachable!()
        };
        for change in updates.iter().filter(|change| {
            change.before.is_none()
                && !relocations
                    .iter()
                    .any(|step| relative(&change.path, &step.target).is_some())
        }) {
            ensure_absent(Path::new(&change.path))?;
        }
        self.install_record(plan)?;
        compound_failpoint("installed")?;
        for (index, step) in relocations.iter().enumerate() {
            // Releasing only this root's descendants is required by NTFS parent
            // rename. All other roots and the union ancestor closure stay held.
            tree_handles.retain(|(file, _)| relative(&t.files[*file].path, &step.source).is_none());
            let namespace = union.select(index)?;
            namespace.release_source_descendants();
            let mut leaf =
                namespace.open_source_file(&step.old_leaf, step.old_leaf != step.new_leaf)?;
            let captured = t
                .files
                .iter()
                .find(|file| {
                    locator_equal(
                        &file.path,
                        &display(&Path::new(&step.source).join(&step.old_leaf)),
                    )
                })
                .ok_or_else(|| {
                    CoreError::new("invalidPlan", "Compound canonical before image missing")
                })?;
            if stamp(&leaf)? != captured.stamp || read_blob(&mut leaf)? != captured.bytes {
                return Err(CoreError::new(
                    "recoveryPending",
                    "Compound canonical input changed before its original step",
                ));
            }
            if step.old_leaf != step.new_leaf {
                namespace.rename_leaf(&leaf, &step.new_leaf)?;
            }
            drop(leaf);
            compound_failpoint(&format!("leaf{index}"))?;
            namespace.move_source(
                Path::new(&step.target)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
            )?;
            tree_handles.extend(open_tree(namespace, t, step, StepPhase::After)?);
            compound_failpoint(&format!("move{index}"))?;
        }
        let mut created = Vec::new();
        for change in updates.iter().filter(|change| change.before.is_none()) {
            let mut file = if let Some((index, step)) = relocations
                .iter()
                .enumerate()
                .find(|(_, step)| relative(&change.path, &step.target).is_some())
            {
                union.select(index)?.open_source_output(
                    &relative(&change.path, &step.target).unwrap(),
                    true,
                    true,
                )?
            } else {
                let guard = ParentGuard::acquire(Path::new(&change.path))?;
                let file = guard.open_file(true, true)?;
                outside_guards.push(guard);
                file
            };
            self.record_materialized(&plan.operation_id, Path::new(&change.path), &file)?;
            write_blob(&mut file, &change.after)?;
            created.push((change.path.clone(), file));
        }
        let changes = t.relocated_files();
        let mut materialized = Vec::new();
        for (index, file) in tree_handles.iter_mut().chain(&mut outside_handles) {
            let path = t.relocated_path(&t.files[*index].path);
            let change = changes
                .iter()
                .find(|change| locator_equal(&change.path, &path))
                .ok_or_else(|| {
                    CoreError::new("invalidPlan", "Compound mapped participant missing")
                })?;
            if change.before.as_ref() != Some(&change.after) {
                write_blob(file, &change.after)?;
            }
            if read_blob(file)? != change.after {
                return Err(CoreError::new(
                    "recoveryPending",
                    "Compound after image verification failed",
                ));
            }
            materialized.push((path, stamp(file)?));
            compound_failpoint(&format!("content{index}"))?;
        }
        for (path, file) in &mut created {
            materialized.push((path.clone(), stamp(file)?));
        }
        for (index, step) in relocations.iter().enumerate() {
            check_membership(union.select(index)?, t, step, StepPhase::After, true)?;
        }
        self.publish_recovery_facts(plan, &materialized)?;
        compound_failpoint("facts")?;
        self.commit(plan)?;
        Ok(())
    }

    pub(crate) fn capture_compound_recovery(
        &self,
        plan: &Plan,
        capture: &mut RecoveryCapture,
        write: bool,
    ) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let relocations = steps(t)?;
        let phases = relocations.iter().map(phase).collect::<Result<Vec<_>>>()?;
        let pairs = relocations
            .iter()
            .zip(&phases)
            .map(|(step, phase)| {
                (
                    Path::new(if *phase == StepPhase::After {
                        &step.target
                    } else {
                        &step.source
                    }),
                    Path::new(&step.target).parent().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut union = MultiNamespaceGuard::acquire(&pairs)?;
        capture.changes = t.relocated_files();
        for (index, step) in relocations.iter().enumerate() {
            let phase = phases[index];
            check_step_identity(&mut union, index, step, phase)?;
            let namespace = union.select(index)?;
            check_membership(namespace, t, step, phase, true)?;
            let evidence = serde_json::to_vec(&step.source_identity)?;
            let mut root = member(
                &step.source,
                "placement",
                Some(&evidence),
                Some(&evidence),
                Some(&step.source_identity),
            );
            root.kind = "directory".into();
            root.target_path = Some(step.target.clone());
            root.current = RecoveryBlob::from_bytes(Some(&evidence));
            root.current_identity = Some((&step.source_identity).into());
            root.identity_binding = "original".into();
            root.matches_before = phase == StepPhase::Before;
            root.matches_after = phase == StepPhase::After;
            root.state = match phase {
                StepPhase::Before => "knownBefore",
                StepPhase::LeafRenamed => "knownIntermediate",
                StepPhase::After => "knownAfter",
            }
            .into();
            root.message = "Exact original namespace root, parents and membership retained".into();
            capture.members.push(root);
            for original in &t.files {
                let Some(path) = source_relative(original, step, phase) else {
                    continue;
                };
                let final_path = t.relocated_path(&original.path);
                let change = capture
                    .changes
                    .iter()
                    .find(|change| locator_equal(&change.path, &final_path))
                    .unwrap();
                let mut held = namespace.open_source_output(
                    &path,
                    write && change.before.as_ref() != Some(&change.after),
                    false,
                )?;
                let observed = stamp(&held)?;
                let bytes = read_blob(&mut held)?;
                let bound = observed.identity == original.stamp.identity;
                let unchanged = change.before.as_ref() == Some(&change.after);
                let before = bound && bytes == original.bytes && observed == original.stamp;
                let after =
                    bound && bytes == change.after && (!unchanged || observed == original.stamp);
                let mut m = member(
                    &final_path,
                    "write",
                    Some(&original.bytes),
                    Some(&change.after),
                    Some(&original.stamp.identity),
                );
                m.target_path = Some(display(
                    &Path::new(if phase == StepPhase::After {
                        &step.target
                    } else {
                        &step.source
                    })
                    .join(&path),
                ));
                m.current = RecoveryBlob::from_bytes(Some(&bytes));
                m.current_identity = Some((&observed.identity).into());
                m.matches_before = before;
                m.matches_after = phase == StepPhase::After && after;
                m.state = if before {
                    "knownBefore"
                } else if after {
                    "knownAfter"
                } else {
                    "thirdState"
                }
                .into();
                m.identity_binding = if bound { "original" } else { "mismatch" }.into();
                m.message = "Exact original compound file and before/after bytes".into();
                if !before && !after {
                    return Err(CoreError::new(
                        "thirdState",
                        "Compound file is neither its original before nor exact after object",
                    ));
                }
                capture.files.push((capture.members.len(), held));
                capture.members.push(m);
                // Retain an explicit index, never zip independently filtered
                // sources and handles. Every original file is represented once.
            }
            for entry in &step.members {
                if namespace.source_parent_identity(Path::new(&entry.relative_path))?
                    != entry.parent
                {
                    return Err(CoreError::new(
                        "thirdState",
                        "Original compound member parent changed",
                    ));
                }
                if !t.files.iter().any(|file| {
                    file.stamp.identity == entry.identity
                        && relative(&file.path, &step.source).as_ref()
                            == Some(&PathBuf::from(&entry.relative_path))
                }) {
                    return Err(CoreError::new(
                        "invalidPlan",
                        "Compound placement member lacks original physical input",
                    ));
                }
            }
            if let Mutation::Compound { updates, .. } = &t.mutation {
                for change in updates.iter().filter(|change| {
                    change.before.is_none() && relative(&change.path, &step.target).is_some()
                }) {
                    let path = relative(&change.path, &step.target).unwrap();
                    let materialized =
                        self.materialized_identity(&plan.operation_id, Path::new(&change.path))?;
                    let mut m = member(
                        &change.path,
                        "write",
                        None,
                        Some(&change.after),
                        materialized.as_ref(),
                    );
                    match namespace.open_source_output(&path, write, false) {
                        Err(error) if error.code == "missing" && materialized.is_none() => {
                            m.matches_before = true;
                            m.state = "knownBefore".into();
                            m.identity_binding = "missing".into();
                        }
                        Err(error) if error.code == "missing" => {
                            return Err(CoreError::new(
                                "thirdState",
                                "Original created compound object disappeared",
                            ));
                        }
                        Err(error) => return Err(error),
                        Ok(mut file) => {
                            let observed = stamp(&file)?;
                            let bytes = read_blob(&mut file)?;
                            if phase != StepPhase::After
                                || materialized.as_ref() != Some(&observed.identity)
                                || (bytes != change.after && !bytes.is_empty())
                            {
                                return Err(CoreError::new(
                                    "thirdState",
                                    "Compound create-only output is not the original materialized before/after object",
                                ));
                            }
                            m.matches_before = bytes.is_empty();
                            m.matches_after = bytes == change.after;
                            m.state = if m.matches_after {
                                "knownAfter"
                            } else {
                                "knownBefore"
                            }
                            .into();
                            m.current = RecoveryBlob::from_bytes(Some(&bytes));
                            m.current_identity = Some((&observed.identity).into());
                            m.identity_binding = "materialized".into();
                            capture.files.push((capture.members.len(), file));
                        }
                    }
                    m.message = "Original compound create-only output; only its original materialization may resume".into();
                    capture.members.push(m);
                }
            }
        }
        // Namespace-root handles stay owned by the recovery capture, including
        // before/intermediate roots that an explicit original continuation moves.
        capture.compound = Some(CompoundCapture {
            namespace: union,
            phases,
        });
        Ok(())
    }

    pub(crate) fn resume_compound_namespaces(
        &self,
        plan: &Plan,
        capture: &mut RecoveryCapture,
    ) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let relocations = steps(t)?;
        let compound = capture.compound.as_mut().ok_or_else(|| {
            CoreError::new(
                "recoveryConflict",
                "Original compound namespace proof is unavailable",
            )
        })?;
        for (index, step) in relocations.iter().enumerate() {
            let phase = compound.phases[index];
            if phase == StepPhase::After {
                continue;
            }
            capture.files.retain(|(member_index, _)| {
                !capture.members[*member_index]
                    .target_path
                    .as_ref()
                    .is_some_and(|path| relative(path, &step.source).is_some())
            });
            let namespace = compound.namespace.select(index)?;
            namespace.release_source_descendants();
            if phase == StepPhase::Before && step.old_leaf != step.new_leaf {
                let mut leaf = namespace.open_source_file(&step.old_leaf, true)?;
                let original = t
                    .files
                    .iter()
                    .find(|file| {
                        locator_equal(
                            &file.path,
                            &display(&Path::new(&step.source).join(&step.old_leaf)),
                        )
                    })
                    .unwrap();
                if stamp(&leaf)? != original.stamp || read_blob(&mut leaf)? != original.bytes {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        "Canonical original changed before continuation",
                    ));
                }
                namespace.rename_leaf(&leaf, &step.new_leaf)?;
            }
            namespace.move_source(
                Path::new(&step.target)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
            )?;
            compound.phases[index] = StepPhase::After;
            // Open through the same original root, then bind each held file to
            // its original recovery member. No target-path namesake is authority.
            check_membership(namespace, t, step, StepPhase::After, true)?;
            for original in &t.files {
                let Some(path) = source_relative(original, step, StepPhase::After) else {
                    continue;
                };
                let final_path = t.relocated_path(&original.path);
                let member_index = capture
                    .members
                    .iter()
                    .position(|m| m.role == "write" && locator_equal(&m.path, &final_path))
                    .unwrap();
                let change = capture
                    .changes
                    .iter()
                    .find(|change| locator_equal(&change.path, &final_path))
                    .unwrap();
                let mut file = namespace.open_source_output(
                    &path,
                    change.before.as_ref() != Some(&change.after),
                    false,
                )?;
                let observed = stamp(&file)?;
                let bytes = read_blob(&mut file)?;
                if observed.identity != original.stamp.identity
                    || (bytes != original.bytes && bytes != change.after)
                    || (bytes == original.bytes && observed != original.stamp)
                {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        "Compound original file changed during namespace continuation",
                    ));
                }
                capture.members[member_index].matches_after = bytes == change.after;
                capture.files.push((member_index, file));
            }
            let root = capture
                .members
                .iter_mut()
                .find(|m| m.role == "placement" && m.path == step.source)
                .unwrap();
            root.matches_after = true;
        }
        Ok(())
    }
    pub(crate) fn verify_compound_final(
        &self,
        plan: &Plan,
        capture: &mut RecoveryCapture,
    ) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let compound = capture.compound.as_mut().ok_or_else(|| {
            CoreError::new("recoveryConflict", "Original namespace proof missing")
        })?;
        for (index, step) in steps(t)?.iter().enumerate() {
            if compound.phases[index] != StepPhase::After {
                return Err(CoreError::new(
                    "recoveryConflict",
                    "Original namespace is incomplete",
                ));
            }
            check_membership(
                compound.namespace.select(index)?,
                t,
                step,
                StepPhase::After,
                true,
            )?;
        }
        Ok(())
    }
    pub(crate) fn create_compound_recovery_output(
        &self,
        plan: &Plan,
        capture: &mut RecoveryCapture,
        path: &str,
    ) -> Result<Option<File>> {
        let t = plan.transaction.as_ref().unwrap();
        let Some(compound) = capture.compound.as_mut() else {
            return Ok(None);
        };
        for (index, step) in steps(t)?.iter().enumerate() {
            if let Some(relative) = relative(path, &step.target) {
                if compound.phases[index] != StepPhase::After {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        "Create-only output's original placement is incomplete",
                    ));
                }
                return Ok(Some(
                    compound
                        .namespace
                        .select(index)?
                        .open_source_output(&relative, true, true)?,
                ));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
thread_local! { static COMPOUND_FAILPOINT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
pub(crate) fn set_compound_failpoint(name: &str) {
    COMPOUND_FAILPOINT.with(|point| *point.borrow_mut() = Some(name.into()));
}
fn compound_failpoint(name: &str) -> Result<()> {
    #[cfg(test)]
    if COMPOUND_FAILPOINT.with(|point| point.borrow().as_deref() == Some(name)) {
        COMPOUND_FAILPOINT.with(|point| *point.borrow_mut() = None);
        return Err(CoreError::new(
            "testInterrupted",
            "Interrupted original compound operation",
        ));
    }
    let _ = name;
    Ok(())
}
