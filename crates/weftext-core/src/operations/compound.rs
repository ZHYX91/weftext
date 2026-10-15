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
    directory_keys: BTreeMap<String, PathBuf>,
}

fn steps(t: &FrozenOperation) -> Result<&[RelocationStep]> {
    match &t.mutation {
        Mutation::Compound { relocations, .. }
            if !relocations.is_empty() && relocations.len() <= MAX_OPERATION_FILES =>
        {
            Ok(relocations)
        }
        _ => Err(CoreError::new(
            "invalidPlan",
            "Compound requires bounded independent original placements",
        )),
    }
}
fn creation_directories(t: &FrozenOperation) -> &[String] {
    match &t.mutation {
        Mutation::Compound {
            created_directories,
            ..
        } => created_directories,
        _ => &[],
    }
}
pub(crate) fn validate_compound_namespace(t: &FrozenOperation) -> Result<()> {
    // Component ordering puts a path immediately before its descendants, so a
    // file can never secretly become the parent of a later output. Exact update
    // paths coalesce; distinct Windows-equivalent namespace slots do not.
    let mut slots = BTreeMap::<PathBuf, bool>::new();
    for file in t.relocated_files() {
        slots.insert(PathBuf::from(file.path), false);
    }
    for directory in &t.directories {
        let path = PathBuf::from(t.relocated_path(&directory.path));
        if slots.insert(path, true).is_some() {
            return Err(CoreError::new(
                "invalidPlan",
                "A compound file and directory share a final namespace slot",
            ));
        }
    }
    for directory in creation_directories(t) {
        if slots.insert(PathBuf::from(directory), true).is_some() {
            return Err(CoreError::new(
                "invalidPlan",
                "A create-only directory overlaps an original final namespace slot",
            ));
        }
    }
    let mut slots = slots.into_iter().collect::<Vec<_>>();
    let mut failure = None;
    slots.sort_by(|(left, _), (right, _)| {
        match crate::backend::namespace_path_compare(left, right) {
            Ok(order) => order,
            Err(error) => {
                failure = Some(error);
                std::cmp::Ordering::Equal
            }
        }
    });
    if let Some(error) = failure {
        return Err(error);
    }
    for adjacent in slots.windows(2) {
        let (left, directory) = &adjacent[0];
        let (right, _) = &adjacent[1];
        if crate::backend::namespace_path_compare(left, right)? == std::cmp::Ordering::Equal
            || (!directory && crate::backend::namespace_relative_path(right, left)?.is_some())
        {
            return Err(CoreError::new(
                "invalidPlan",
                "Compound final namespace slots collide or a file is used as a directory",
            ));
        }
    }
    Ok(())
}
fn physical_path(t: &FrozenOperation, path: &str, phases: &[StepPhase]) -> PathBuf {
    t.relocations()
        .iter()
        .zip(phases)
        .find_map(|(step, phase)| {
            relative(path, step.target).map(|relative| {
                Path::new(if *phase == StepPhase::After {
                    step.target
                } else {
                    step.source
                })
                .join(relative)
            })
        })
        .unwrap_or_else(|| PathBuf::from(path))
}
fn directory_keys(t: &FrozenOperation, phases: &[StepPhase]) -> BTreeMap<String, PathBuf> {
    creation_directories(t)
        .iter()
        .map(|path| (path.clone(), physical_path(t, path, phases)))
        .collect()
}
fn expected_parent(
    core: &Core,
    plan: &Plan,
    step: &RelocationStep,
) -> Result<Option<FileIdentity>> {
    match &step.target_parent_identity {
        RelocationParent::Existing(identity) => Ok(Some(identity.clone())),
        RelocationParent::Created { created_directory } => {
            if Path::new(&step.target).parent() != Some(Path::new(created_directory))
                || !creation_directories(plan.transaction.as_ref().unwrap())
                    .contains(created_directory)
            {
                return Err(CoreError::new(
                    "invalidPlan",
                    "Created target parent is not in this original Plan",
                ));
            }
            core.materialized_identity(&plan.operation_id, Path::new(created_directory))
        }
    }
}
fn verify_created_membership(
    t: &FrozenOperation,
    union: &MultiNamespaceGuard,
    keys: &BTreeMap<String, PathBuf>,
) -> Result<()> {
    for (path, key) in keys {
        if union.directory_identity(key)?.is_none() {
            continue;
        }
        let (_, entries, complete) = union.directory_snapshot(key, MAX_OPERATION_FILES)?;
        if !complete {
            return Err(CoreError::new(
                "scopeChanged",
                "Created directory membership exceeds its bounded proof",
            ));
        }
        let mut allowed = BTreeSet::new();
        for child in creation_directories(t)
            .iter()
            .chain(t.created_files().iter().map(|file| &file.path))
        {
            if Path::new(child).parent() == Some(Path::new(path)) {
                allowed.insert(
                    Path::new(child)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        if let Mutation::Compound { updates, .. } = &t.mutation {
            for file in updates.iter().filter(|file| file.before.is_none()) {
                if Path::new(&file.path).parent() == Some(Path::new(path)) {
                    allowed.insert(
                        Path::new(&file.path)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        for step in t.relocations() {
            if Path::new(step.target).parent() == Some(Path::new(path)) {
                allowed.insert(
                    Path::new(step.target)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        if entries
            .iter()
            .any(|(name, _)| !allowed.contains(name.to_string_lossy().as_ref()))
        {
            return Err(CoreError::new(
                "thirdState",
                "Created directory has an unplanned member",
            ));
        }
    }
    Ok(())
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
        if include_creations
            && let Mutation::Compound {
                updates,
                created_files,
                created_directories,
                ..
            } = &t.mutation
        {
            let final_directory = Path::new(&step.target).join(&path);
            for change in updates.iter().chain(created_files).filter(|change| {
                change.before.is_none()
                    && Path::new(&change.path).parent() == Some(final_directory.as_path())
            }) {
                let name = Path::new(&change.path).file_name().unwrap();
                // Missing is a known create-only before state; existing names
                // remain subject to original materialization/bytes checks.
                entries.retain(|(actual, _)| actual != name);
            }
            for directory in created_directories.iter().filter(|directory| {
                Path::new(directory).parent() == Some(final_directory.as_path())
            }) {
                let name = Path::new(directory).file_name().unwrap();
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
    core: &Core,
    plan: &Plan,
    union: &mut MultiNamespaceGuard,
    index: usize,
    step: &RelocationStep,
    phase: StepPhase,
) -> Result<()> {
    let parent = union.source_parent_identity(index)?;
    let expected = expected_parent(core, plan, step)?;
    let target_parent = union.target_parent_identity(index)?;
    let namespace = union.select(index)?;
    if namespace.source_identity()? != step.source_identity
        || target_parent != expected
        || (phase == StepPhase::After && expected.as_ref() != Some(&parent))
        || (phase != StepPhase::After && parent != step.source_parent_identity)
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
    check_membership(namespace, t, step, phase, phase == StepPhase::After)?;
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
    if let Mutation::Compound {
        updates,
        created_files,
        ..
    } = &t.mutation
    {
        for change in updates
            .iter()
            .chain(created_files)
            .filter(|change| change.before.is_none())
        {
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
    pub(crate) fn validate_compound_pending(&self, plan: &Plan) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let relocations = steps(t)?;
        let mut roots = relocations
            .iter()
            .flat_map(|step| [PathBuf::from(&step.source), PathBuf::from(&step.target)])
            .collect::<Vec<_>>();
        roots.extend(creation_directories(t).iter().map(PathBuf::from));
        let except = Some(plan.operation_id.as_str());
        for file in &t.files {
            self.check_pending_except(Path::new(&file.path), except)?;
            self.check_pending_except(Path::new(&t.relocated_path(&file.path)), except)?;
            if relocations
                .iter()
                .any(|step| relative(&file.path, &step.source).is_some())
            {
                self.check_pending_identity_except(&file.stamp.identity, except)?;
            }
        }
        let phases = vec![StepPhase::Before; relocations.len()];
        for change in t.relocated_files() {
            self.check_pending_except(Path::new(&change.path), except)?;
            self.check_pending_except(&physical_path(t, &change.path, &phases), except)?;
        }
        for root in &roots {
            self.check_pending_except(root, except)?;
        }
        // A pending child can still be absent and therefore absent from this
        // Plan's captured tree. Compare the original unfinished writer's frozen
        // addresses against both namespace scopes, rather than scanning current
        // files or treating absent children as unowned slots.
        for other in self.operations.values().filter(|other| {
            other.operation_id != plan.operation_id
                && !self.committed(&other.operation_id)
                && !self.resolved(&other.operation_id)
                && (other.transaction.is_none()
                    || self.journal_path(&other.operation_id, "install").exists())
        }) {
            let mut paths = Vec::new();
            if let Some(transaction) = &other.transaction {
                paths.extend(
                    transaction
                        .files
                        .iter()
                        .map(|file| PathBuf::from(&file.path)),
                );
                paths.extend(
                    transaction
                        .created_files()
                        .iter()
                        .map(|file| PathBuf::from(&file.path)),
                );
                paths.extend(
                    transaction
                        .relocated_files()
                        .iter()
                        .map(|file| PathBuf::from(&file.path)),
                );
                paths.extend(
                    transaction
                        .relocations()
                        .iter()
                        .flat_map(|step| [PathBuf::from(step.source), PathBuf::from(step.target)]),
                );
                match &transaction.mutation {
                    Mutation::Update { files } => {
                        paths.extend(files.iter().map(|file| PathBuf::from(&file.path)))
                    }
                    Mutation::CreateTree {
                        target,
                        existing_target,
                        directories,
                        updates,
                        ..
                    } => {
                        if !existing_target {
                            paths.push(PathBuf::from(target));
                        }
                        paths.extend(
                            directories
                                .iter()
                                .map(|directory| Path::new(target).join(directory)),
                        );
                        paths.extend(updates.iter().map(|file| PathBuf::from(&file.path)));
                    }
                    Mutation::Compound {
                        created_directories,
                        ..
                    } => paths.extend(created_directories.iter().map(PathBuf::from)),
                    Mutation::Relocate { .. } => {}
                }
            } else {
                paths.push(PathBuf::from(&other.path));
            }
            for path in paths {
                for root in &roots {
                    if crate::backend::namespace_relative_path(&path, root)?.is_some()
                        || crate::backend::namespace_relative_path(root, &path)?.is_some()
                    {
                        return Err(CoreError::new(
                            "recoveryPending",
                            "An original unfinished writer overlaps this compound namespace; inspect its retained operation before any effect",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    fn materialize_compound_directories(
        &self,
        plan: &Plan,
        union: &mut MultiNamespaceGuard,
        keys: &BTreeMap<String, PathBuf>,
    ) -> Result<()> {
        let mut paths = keys.iter().collect::<Vec<_>>();
        paths.sort_by_key(|(_, path)| path.components().count());
        for (final_path, key) in paths {
            let expected = self.materialized_identity(&plan.operation_id, Path::new(final_path))?;
            match union.directory_identity(key)? {
                Some(actual) if expected.as_ref() == Some(&actual) => {}
                Some(_) => {
                    return Err(CoreError::new(
                        "thirdState",
                        "Created directory is not its original materialized object",
                    ));
                }
                None if expected.is_some() => {
                    return Err(CoreError::new(
                        "thirdState",
                        "Original created directory disappeared",
                    ));
                }
                None => {
                    let directory = union.create_directory(key)?;
                    self.record_materialized(&plan.operation_id, Path::new(final_path), directory)?;
                    compound_failpoint(&format!(
                        "directory:{}",
                        Path::new(final_path).file_name().unwrap().to_string_lossy()
                    ))?;
                }
            }
        }
        Ok(())
    }
    fn retain_compound_directories(
        &self,
        plan: &Plan,
        union: &mut MultiNamespaceGuard,
        keys: &BTreeMap<String, PathBuf>,
    ) -> Result<()> {
        let mut paths = keys.iter().collect::<Vec<_>>();
        paths.sort_by_key(|(_, path)| path.components().count());
        for (final_path, key) in paths {
            let file = union.retain_directory(key)?;
            if self
                .materialized_identity(&plan.operation_id, Path::new(final_path))?
                .as_ref()
                != Some(&stamp(file)?.identity)
            {
                return Err(CoreError::new(
                    "thirdState",
                    "Created directory changed across its original namespace step",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn execute_compound(&mut self, plan: &Plan) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let relocations = steps(t)?;
        validate_compound_namespace(t)?;
        self.validate_compound_pending(plan)?;
        for (index, step) in relocations.iter().enumerate() {
            for (other, another) in relocations.iter().enumerate() {
                let within = |path: &str, root: &str| {
                    crate::backend::namespace_relative_path(Path::new(path), Path::new(root))
                        .map(|relative| relative.is_some())
                };
                if index != other
                    && (within(&step.source, &another.source)?
                        || within(&step.target, &another.target)?)
                    || crate::backend::namespace_relative_path(
                        Path::new(&step.target).parent().unwrap(),
                        Path::new(&another.source),
                    )?
                    .is_some()
                {
                    return Err(CoreError::new(
                        "invalidPlan",
                        "Compound namespace targets overlap under the Windows ordinal name comparison",
                    ));
                }
            }
        }
        for step in relocations {
            self.check_subtree_boundary(Path::new(&t.request.root), Path::new(&step.source))?;
            if matches!(step.target_parent_identity, RelocationParent::Existing(_)) {
                self.owned_parent(
                    Path::new(&t.request.root),
                    Path::new(&step.target).parent().unwrap(),
                )?;
            }
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
        let initial_phases = vec![StepPhase::Before; relocations.len()];
        let keys = directory_keys(t, &initial_phases);
        let mut union = if keys.is_empty() {
            MultiNamespaceGuard::acquire(&pairs)?
        } else {
            MultiNamespaceGuard::acquire_planned(
                &pairs,
                &keys.values().cloned().collect::<Vec<_>>(),
            )?
        };
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
            check_step_identity(self, plan, &mut union, index, step, StepPhase::Before)?;
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
        let Mutation::Compound {
            updates,
            created_files,
            ..
        } = &t.mutation
        else {
            unreachable!()
        };
        for change in updates.iter().chain(created_files).filter(|change| {
            change.before.is_none()
                && !relocations
                    .iter()
                    .any(|step| relative(&change.path, &step.target).is_some())
        }) {
            self.check_pending(Path::new(&change.path))?;
            ensure_absent(&physical_path(t, &change.path, &initial_phases))?;
        }
        for (path, key) in &keys {
            self.check_pending(Path::new(path))?;
            if union.directory_identity(key)?.is_some() {
                return Err(CoreError::new(
                    "exists",
                    "Create-only directory is occupied",
                ));
            }
        }
        self.install_record(plan)?;
        compound_failpoint("installed")?;
        self.materialize_compound_directories(plan, &mut union, &keys)?;
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
            // Source descendants had to be released for NTFS rename. Rebind
            // only through the same original root and retained materialization.
            self.retain_compound_directories(plan, &mut union, &keys)?;
            let namespace = union.select(index)?;
            tree_handles.extend(open_tree(namespace, t, step, StepPhase::After)?);
            compound_failpoint(&format!("move{index}"))?;
        }
        let mut created = Vec::new();
        for change in updates
            .iter()
            .chain(created_files)
            .filter(|change| change.before.is_none())
        {
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
            compound_failpoint(&format!(
                "materialize:{}",
                Path::new(&change.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
            ))?;
            write_blob(&mut file, &change.after)?;
            compound_failpoint(&format!(
                "create:{}",
                Path::new(&change.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
            ))?;
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
        verify_created_membership(t, &union, &keys)?;
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
        validate_compound_namespace(t)?;
        self.validate_compound_pending(plan)?;
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
        let keys = directory_keys(t, &phases);
        let mut union = if keys.is_empty() {
            MultiNamespaceGuard::acquire(&pairs)?
        } else {
            MultiNamespaceGuard::acquire_planned(
                &pairs,
                &keys.values().cloned().collect::<Vec<_>>(),
            )?
        };
        for (path, key) in &keys {
            let expected = self.materialized_identity(&plan.operation_id, Path::new(path))?;
            let actual = union.directory_identity(key)?;
            let mut directory = member(
                path,
                "write",
                None,
                Some(b"createdDirectory"),
                expected.as_ref(),
            );
            directory.kind = "directory".into();
            directory.target_path = Some(display(key));
            if actual.is_some() && actual == expected {
                directory.matches_after = true;
                directory.state = "knownAfter".into();
                directory.identity_binding = "materialized".into();
                directory.current_identity = actual.as_ref().map(RecoveryIdentity::from);
            } else if actual.is_none() && expected.is_none() {
                directory.matches_before = true;
                directory.state = "knownBefore".into();
                directory.identity_binding = "missing".into();
            } else {
                return Err(CoreError::new(
                    "thirdState",
                    "Create-only directory lost its exact original materialization",
                ));
            }
            directory.message =
                "Only this Plan's create-only directory may supply the planned parent".into();
            capture.members.push(directory);
        }
        verify_created_membership(t, &union, &keys)?;
        capture.changes = t.relocated_files();
        for (index, step) in relocations.iter().enumerate() {
            let phase = phases[index];
            check_step_identity(self, plan, &mut union, index, step, phase)?;
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
            if let Mutation::Compound {
                updates,
                created_files,
                ..
            } = &t.mutation
            {
                for change in updates.iter().chain(created_files).filter(|change| {
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
            directory_keys: keys,
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
        self.materialize_compound_directories(
            plan,
            &mut compound.namespace,
            &compound.directory_keys,
        )?;
        for directory in capture
            .members
            .iter_mut()
            .filter(|member| member.kind == "directory" && member.role == "write")
        {
            directory.matches_after = true;
        }
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
            self.retain_compound_directories(
                plan,
                &mut compound.namespace,
                &compound.directory_keys,
            )?;
            let namespace = compound.namespace.select(index)?;
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
        verify_created_membership(t, &compound.namespace, &compound.directory_keys)?;
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
        Ok(Some(compound.namespace.open_output(
            Path::new(path),
            true,
            true,
        )?))
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
