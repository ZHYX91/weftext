//! One original sync namespace/content decision, including its interrupted steps.
use super::*;
use crate::backend::MultiNamespaceGuard;
use crate::journal::{Plan, RecoveryBlob, RecoveryIdentity, RecoveryMember};
use crate::transaction_recovery::RecoveryCapture;
use weftext_messages::UiMessage;

#[path = "compound_placement.rs"]
mod placement;
#[path = "compound_raw.rs"]
mod raw;
#[path = "compound_retirement.rs"]
mod retirement;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepPhase {
    Before,
    LeafRenamed,
    Parked,
    After,
}
pub(crate) struct CompoundCapture {
    namespace: MultiNamespaceGuard,
    phases: Vec<StepPhase>,
    directory_keys: BTreeMap<String, PathBuf>,
    retirement_guards: Vec<ParentGuard>,
}

fn steps(t: &FrozenOperation) -> Result<&[RelocationStep]> {
    match &t.mutation {
        Mutation::Compound { relocations, .. }
            if (!relocations.is_empty() || !t.retired_members().is_empty())
                && relocations.len() <= MAX_OPERATION_FILES =>
        {
            Ok(relocations)
        }
        _ => Err(CoreError::new(
            "invalidPlan",
            UiMessage::error_core_operations_compound_requires_bounded_independent_original_placements(),
        )),
    }
}
/// Coordinates from the original frozen namespace phases only. Consumers must
/// independently retain the returned directory and compare its original FI.
pub(crate) fn compound_membership_positions(
    t: &FrozenOperation,
) -> Result<BTreeMap<String, PathBuf>> {
    let steps = steps(t)?;
    let phases = placement::phases(steps)?;
    Ok(steps
        .iter()
        .zip(phases)
        .map(|(step, phase)| {
            (
                step.source.clone(),
                placement::coordinate(step, phase).to_path_buf(),
            )
        })
        .collect())
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
    placement::validate(t)?;
    retirement::validate(t)?;
    // Component ordering puts a path immediately before its descendants, so a
    // file can never secretly become the parent of a later output. Exact update
    // paths coalesce; distinct Windows-equivalent namespace slots do not.
    let mut slots = BTreeMap::<PathBuf, bool>::new();
    for file in t.relocated_files() {
        slots.insert(PathBuf::from(file.path), false);
    }
    for retired in t.retired_members() {
        slots.insert(PathBuf::from(&retired.target), false);
    }
    for directory in &t.directories {
        let path = PathBuf::from(t.relocated_path(&directory.path));
        if slots.insert(path, true).is_some() {
            return Err(CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_compound_file_and_directory_share_final_namespace_slot(),
            ));
        }
    }
    for directory in creation_directories(t) {
        if slots.insert(PathBuf::from(directory), true).is_some() {
            return Err(CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_create_only_directory_overlaps_original_final_namespace_slot(),
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
                UiMessage::error_core_operations_compound_final_namespace_slots_collide_or_file_used_as_directory(),
            ));
        }
    }
    Ok(())
}
fn physical_path(t: &FrozenOperation, path: &str, phases: &[StepPhase]) -> PathBuf {
    let Mutation::Compound { relocations, .. } = &t.mutation else {
        return PathBuf::from(path);
    };
    relocations
        .iter()
        .zip(phases)
        .filter_map(|(step, phase)| {
            relative(path, &step.target).map(|relative| {
                (
                    Path::new(&step.target).components().count(),
                    placement::coordinate(step, *phase).join(relative),
                )
            })
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, path)| path)
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
                    UiMessage::error_core_operations_created_target_parent_not_in_original_plan(),
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
                UiMessage::error_core_operations_created_directory_membership_exceeds_bounded_proof(
                ),
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
        for retired in t
            .retired_members()
            .iter()
            .filter(|retired| Path::new(&retired.target).parent() == Some(Path::new(path)))
        {
            allowed.insert(
                Path::new(&retired.target)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        if entries
            .iter()
            .any(|(name, _)| !allowed.contains(name.to_string_lossy().as_ref()))
        {
            return Err(CoreError::new(
                "thirdState",
                UiMessage::error_core_operations_created_directory_has_unplanned_member(),
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
        description: None,
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
                    UiMessage::error_core_operations_canonical_namespace_not_original_before_intermediate_state(),
                )),
            }
        }
        (None, Some(identity)) if identity == step.source_identity => Ok(StepPhase::After),
        _ => Err(CoreError::new(
            "thirdState",
            UiMessage::error_core_operations_original_moving_root_missing_replaced_or_other_location_occupied(),
        )),
    }
}
fn source_relative(
    t: &FrozenOperation,
    file: &CapturedFile,
    step: &RelocationStep,
    phase: StepPhase,
) -> Option<PathBuf> {
    if !placement::owns(t, &file.path, step) {
        return None;
    }
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
    raw_budget: &mut raw::Budget,
) -> Result<()> {
    let mut additions = Vec::new();
    for directory in &t.directories {
        if !placement::owns(t, &directory.path, step) {
            continue;
        }
        let Some(path) = relative(&directory.path, &step.source) else {
            continue;
        };
        let (identity, mut entries, complete) =
            namespace.source_directory_snapshot(&path, MAX_OPERATION_FILES)?;
        let mut expected = directory.entries.clone();
        for retired in t.retired_members().iter().filter(|retired| {
            Path::new(&retired.source).parent() == Some(Path::new(&directory.path))
        }) {
            let name = Path::new(&retired.source).file_name().unwrap();
            let prefix = format!("{}:", name.to_string_lossy());
            expected.retain(|entry| !entry.starts_with(&prefix));
            entries.retain(|(entry, _)| entry != name);
        }
        for child in steps(t)?.iter().filter(|child| {
            child.source != step.source
                && Path::new(&child.source).parent() == Some(Path::new(&directory.path))
        }) {
            let name = Path::new(&child.source).file_name().unwrap();
            let prefix = format!("{}:", name.to_string_lossy());
            expected.retain(|entry| !entry.starts_with(&prefix));
            entries.retain(|(entry, _)| entry != name);
        }
        let final_directory = Path::new(&step.target).join(&path);
        for child in steps(t)?.iter().filter(|child| {
            child.source != step.source
                && Path::new(&child.target).parent() == Some(final_directory.as_path())
        }) {
            let name = Path::new(&child.target).file_name().unwrap();
            let prefix = format!("{}:", name.to_string_lossy());
            expected.retain(|entry| !entry.starts_with(&prefix));
            entries.retain(|(entry, _)| entry != name);
        }
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
        let actual = sorted_entries(entries.clone(), Path::new(&directory.path));
        if !complete
            || identity != directory.identity
            || expected
                .iter()
                .any(|entry| actual.binary_search(entry).is_err())
        {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_core_operations_original_compound_directory_identity_or_captured_member_changed(),
            ));
        }
        for (name, attributes) in entries {
            if expected
                .binary_search(&format!("{}:{attributes}", name.to_string_lossy()))
                .is_err()
            {
                raw_budget.reserve(1)?;
                additions.push((path.clone(), name, attributes));
            }
        }
    }
    raw::validate_additions(namespace, additions, raw_budget)
}
fn check_step_identity(
    core: &Core,
    plan: &Plan,
    union: &mut MultiNamespaceGuard,
    index: usize,
    step: &RelocationStep,
    phase: StepPhase,
    phases: &[StepPhase],
) -> Result<()> {
    let t = plan.transaction.as_ref().unwrap();
    let parent = union.source_parent_identity(index)?;
    let parking_parent = if let Some(parking) = &step.parking {
        if placement::forest(steps(t)?) {
            Some(
                t.write_parents
                    .iter()
                    .find(|(path, _)| locator_equal(path, parking))
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_original_parking_parent_evidence_missing(),
                        )
                    })?
                    .1
                    .clone(),
            )
        } else {
            Some(step.source_parent_identity.clone())
        }
    } else {
        None
    };
    let expected_current_parent = match phase {
        StepPhase::After => expected_parent(core, plan, step)?,
        StepPhase::Parked => parking_parent.clone(),
        _ => Some(step.source_parent_identity.clone()),
    };
    let expected = expected_parent(core, plan, step)?;
    let target_parent = union.target_parent_identity(index)?;
    let namespace = union.select(index)?;
    if namespace.source_identity()? != step.source_identity
        || target_parent != expected
        || expected_current_parent.as_ref() != Some(&parent)
    {
        return Err(CoreError::new(
            "thirdState",
            UiMessage::error_core_operations_compound_original_directory_or_parent_identity_changed(
            ),
        ));
    }
    // Both address slots are part of the same namespace closure. Equal bytes
    // elsewhere never license a second object occupying the vacated slot.
    if step.parking.is_none() {
        ensure_absent(Path::new(if phase == StepPhase::After {
            &step.source
        } else {
            &step.target
        }))?;
    } else if union
        .directory_identity(&placement::source_parent_path(steps(t)?, phases, step))?
        .as_ref()
        != Some(&step.source_parent_identity)
        || step.parking.as_ref().is_some_and(|parking| {
            union
                .directory_identity(Path::new(parking).parent().unwrap())
                .map(|actual| actual != parking_parent)
                .unwrap_or(true)
        })
    {
        return Err(CoreError::new(
            "thirdState",
            UiMessage::error_core_operations_original_parking_parent_was_replaced(),
        ));
    }
    Ok(())
}
fn open_tree(
    namespace: &mut NamespaceGuard,
    t: &FrozenOperation,
    step: &RelocationStep,
    phase: StepPhase,
    raw_budget: &mut raw::Budget,
) -> Result<Vec<(usize, File)>> {
    check_membership(
        namespace,
        t,
        step,
        phase,
        phase != StepPhase::Before,
        raw_budget,
    )?;
    let mut handles = Vec::new();
    for (index, file) in t.files.iter().enumerate() {
        let Some(path) = source_relative(t, file, step, phase) else {
            continue;
        };
        let write =
            update(t, file).is_some_and(|change| change.before.as_ref() != Some(&change.after));
        let mut handle = if t.is_retired_member(&file.path) {
            namespace.open_source_retiring(&path, true)?
        } else {
            namespace.open_source_output(&path, write, false)?
        };
        if let Some(retired) = t
            .retired_members()
            .iter()
            .find(|retired| locator_equal(&retired.source, &file.path))
            && crate::backend::attributes(&handle)? != retired.attributes
        {
            return Err(CoreError::new(
                "conflict",
                UiMessage::error_core_operations_retired_member_attributes_changed(),
            ));
        }
        if stamp(&handle)? != file.stamp || read_blob(&mut handle)? != file.bytes {
            return Err(CoreError::new(
                "conflict",
                UiMessage::error_core_operations_compound_member_no_longer_has_original_stamp_and_bytes(),
            ));
        }
        handles.push((index, handle));
    }
    for entry in &step.members {
        let original_path = display(&Path::new(&step.source).join(&entry.relative_path));
        if !placement::owns(t, &original_path, step) {
            continue;
        }
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
                    UiMessage::error_core_operations_placement_member_lacks_same_original_captured_file(),
                )
            })?;
        if captured.stamp.identity != entry.identity
            || namespace.source_parent_identity(Path::new(&entry.relative_path))? != entry.parent
        {
            return Err(CoreError::new(
                "thirdState",
                UiMessage::error_core_operations_compound_original_placement_member_or_parent_changed(),
            ));
        }
    }
    Ok(handles)
}

fn move_final(
    core: &Core,
    plan: &Plan,
    union: &mut MultiNamespaceGuard,
    index: usize,
    step: &RelocationStep,
) -> Result<()> {
    union.retain_target_parent(index)?;
    if union.target_parent_identity(index)? != expected_parent(core, plan, step)? {
        return Err(CoreError::new(
            "thirdState",
            UiMessage::error_core_operations_original_final_parent_changed_before_placement(),
        ));
    }
    let name = Path::new(&step.target)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_original_final_directory_name_invalid(),
            )
        })?;
    union.select(index)?.move_source(name)?;
    protect_final_source(union, index, plan.transaction.as_ref().unwrap(), step)
}

fn protect_final_source(
    union: &mut MultiNamespaceGuard,
    index: usize,
    t: &FrozenOperation,
    step: &RelocationStep,
) -> Result<()> {
    let original = t
        .files
        .iter()
        .find(|file| {
            locator_equal(
                &file.path,
                &display(&Path::new(&step.source).join(&step.old_leaf)),
            )
        })
        .ok_or_else(|| {
            CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_final_source_canonical_input_missing(),
            )
        })?;
    let name = Path::new(&step.target)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_original_final_directory_name_invalid(),
            )
        })?;
    union.finish_source_move(index, name, &step.new_leaf, &original.stamp.identity)
}

fn rename_original_leaf(
    namespace: &NamespaceGuard,
    t: &FrozenOperation,
    step: &RelocationStep,
) -> Result<()> {
    let mut leaf = namespace.open_source_file(&step.old_leaf, step.old_leaf != step.new_leaf)?;
    let original = t
        .files
        .iter()
        .find(|file| {
            locator_equal(
                &file.path,
                &display(&Path::new(&step.source).join(&step.old_leaf)),
            )
        })
        .ok_or_else(|| {
            CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_compound_canonical_before_image_missing(),
            )
        })?;
    if stamp(&leaf)? != original.stamp || read_blob(&mut leaf)? != original.bytes {
        return Err(CoreError::new(
            "recoveryPending",
            UiMessage::error_core_operations_original_canonical_input_changed_before_namespace_step(
            ),
        ));
    }
    if step.old_leaf != step.new_leaf {
        namespace.rename_leaf(&leaf, &step.new_leaf)?;
    }
    Ok(())
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
                    UiMessage::error_core_operations_renamed_canonical_filename_occupied(),
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
        for change in updates.iter().chain(created_files).filter(|change| {
            change.before.is_none() && placement::owns_final(t, &change.path, step)
        }) {
            if let Some(path) = relative(&change.path, &step.target) {
                match namespace.open_source_descendant(&path) {
                    Err(error) if error.code == "missing" => {}
                    Err(error) => return Err(error),
                    Ok(_) => {
                        return Err(CoreError::new(
                            "exists",
                            UiMessage::error_core_operations_compound_create_only_participant_occupied(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

impl Core {
    /// Read-only validation for consumers of this same retained Compound Plan.
    /// Raw additions are handled by the identical original namespace validator
    /// used by execution and recovery, without treating them as product inputs.
    pub(crate) fn validate_compound_captured_membership(&self, plan: &Plan) -> Result<()> {
        let t = plan.transaction.as_ref().ok_or_else(|| {
            CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_original_compound_transaction_missing(),
            )
        })?;
        validate_compound_namespace(t)?;
        let relocations = steps(t)?;
        let phases = placement::phases(relocations)?;
        let keys = directory_keys(t, &phases);
        let mut union = placement::acquire(relocations, &phases, &keys)?;
        placement::verify_vacancies(relocations, &phases)?;
        drop(retirement::capture(self, plan, &mut union, &phases, false)?);
        let mut raw_budget = raw::Budget::new(t)?;
        for (index, step) in relocations.iter().enumerate() {
            check_step_identity(self, plan, &mut union, index, step, phases[index], &phases)?;
            check_membership(
                union.select(index)?,
                t,
                step,
                phases[index],
                true,
                &mut raw_budget,
            )?;
        }
        raw_budget.validate(self, t)
    }

    pub(crate) fn validate_compound_pending(&self, plan: &Plan) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let relocations = steps(t)?;
        let mut roots = relocations
            .iter()
            .flat_map(|step| {
                [
                    Some(PathBuf::from(&step.source)),
                    Some(PathBuf::from(&step.target)),
                    step.parking.as_ref().map(PathBuf::from),
                ]
                .into_iter()
                .flatten()
            })
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
        for other in self.operations.values() {
            if other.operation_id == plan.operation_id
                || self.committed(&other.operation_id)
                || self.resolved(&other.operation_id)?
                || (other.transaction.is_some()
                    && !self.journal_path(&other.operation_id, "install").exists())
            {
                continue;
            }
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
                        relocations,
                        ..
                    } => {
                        paths.extend(created_directories.iter().map(PathBuf::from));
                        paths.extend(
                            relocations
                                .iter()
                                .filter_map(|step| step.parking.as_ref())
                                .map(PathBuf::from),
                        );
                    }
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
                            UiMessage::error_core_operations_original_unfinished_writer_overlaps_compound_namespace_inspect_retained_operation_before_any_effect(),
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
                        UiMessage::error_core_operations_created_directory_not_original_materialized_object(),
                    ));
                }
                None if expected.is_some() => {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_original_created_directory_disappeared(),
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
                    UiMessage::error_core_operations_created_directory_changed_across_original_namespace_step(),
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
        let initial_phases = vec![StepPhase::Before; relocations.len()];
        for (index, step) in relocations.iter().enumerate() {
            let original_parent = placement::target_parent_path(relocations, &initial_phases, step);
            let moving_parent = crate::backend::namespace_path_compare(
                &original_parent,
                Path::new(&step.target).parent().unwrap(),
            )? != std::cmp::Ordering::Equal;
            for (other, another) in relocations.iter().enumerate() {
                if index != other
                    && (crate::backend::namespace_path_compare(
                        Path::new(&step.source),
                        Path::new(&another.source),
                    )? == std::cmp::Ordering::Equal
                        || crate::backend::namespace_path_compare(
                            Path::new(&step.target),
                            Path::new(&another.target),
                        )? == std::cmp::Ordering::Equal)
                    || (!moving_parent
                        && crate::backend::namespace_relative_path(
                            Path::new(&step.target).parent().unwrap(),
                            Path::new(&another.source),
                        )?
                        .is_some())
                {
                    return Err(CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_operations_compound_namespace_targets_overlap_under_windows_ordinal_name_comparison(),
                    ));
                }
            }
        }
        for step in relocations {
            self.check_subtree_boundary(Path::new(&t.request.root), Path::new(&step.source))?;
            if matches!(step.target_parent_identity, RelocationParent::Existing(_))
                && Path::new(&step.target).parent()
                    != Some(Path::new(&t.request.root).join(".weftext-trash").as_path())
            {
                self.owned_parent(
                    Path::new(&t.request.root),
                    &placement::target_parent_path(relocations, &initial_phases, step),
                )?;
            }
        }
        let keys = directory_keys(t, &initial_phases);
        let mut union = placement::acquire(relocations, &initial_phases, &keys)?;
        placement::verify_vacancies(relocations, &initial_phases)?;
        drop(retirement::capture(
            self,
            plan,
            &mut union,
            &initial_phases,
            false,
        )?);
        let mut outside_guards = Vec::new();
        let mut outside_handles = Vec::new();
        for (path, expected) in &t.write_parents {
            let guard = ParentGuard::acquire(Path::new(path))?;
            if guard.directory_identity()? != *expected {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_compound_external_write_parent_changed(),
                ));
            }
            outside_guards.push(guard);
        }
        for absent in &t.absent_dependencies {
            let guard = ParentGuard::acquire(Path::new(&absent.path))?;
            if guard.directory_identity()? != absent.parent {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_compound_absent_dependency_parent_changed(),
                ));
            }
            ensure_absent(Path::new(&absent.path))?;
            outside_guards.push(guard);
        }
        let mut tree_handles = Vec::new();
        let mut raw_budget = raw::Budget::new(t)?;
        for (index, step) in relocations.iter().enumerate() {
            check_step_identity(
                self,
                plan,
                &mut union,
                index,
                step,
                StepPhase::Before,
                &initial_phases,
            )?;
            let namespace = union.select(index)?;
            tree_handles.extend(open_tree(
                namespace,
                t,
                step,
                StepPhase::Before,
                &mut raw_budget,
            )?);
            verify_creation_absence(namespace, t, step)?;
        }
        raw_budget.validate(self, t)?;
        for (index, file) in t.files.iter().enumerate().filter(|(_, file)| {
            !relocations
                .iter()
                .any(|step| relative(&file.path, &step.source).is_some())
        }) {
            self.check_pending(Path::new(&file.path))?;
            let guard = ParentGuard::acquire(Path::new(&file.path))?;
            let write =
                update(t, file).is_some_and(|change| change.before.as_ref() != Some(&change.after));
            let mut handle = if t.is_retired_member(&file.path) {
                guard.open_retiring_file(true)?
            } else if !write
                && operation_read_control(Path::new(&t.request.root), Path::new(&file.path))
            {
                guard.open_read_guard_file()?
            } else {
                guard.open_file(write, false)?
            };
            if stamp(&handle)? != file.stamp || read_blob(&mut handle)? != file.bytes {
                return Err(CoreError::new(
                    "conflict",
                    UiMessage::error_core_operations_compound_external_dependency_changed(),
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
                    UiMessage::error_core_operations_create_only_directory_occupied(),
                ));
            }
        }
        self.install_record(plan)?;
        compound_failpoint("installed")?;
        self.materialize_compound_directories(plan, &mut union, &keys)?;
        let mut current_phases = initial_phases.clone();
        for index in placement::order(relocations, true) {
            let step = &relocations[index];
            if step.parking.is_none() {
                continue;
            }
            placement::verify_vacancies(relocations, &current_phases)?;
            tree_handles.retain(|(file, _)| !placement::owns(t, &t.files[*file].path, step));
            union.release_source_descendants(index)?;
            let namespace = union.select(index)?;
            rename_original_leaf(namespace, t, step)?;
            compound_failpoint(&format!("leaf{index}"))?;
            union.move_source_to(index, Path::new(step.parking.as_ref().unwrap()))?;
            current_phases[index] = StepPhase::Parked;
            placement::verify_vacancies(relocations, &current_phases)?;
            self.retain_compound_directories(plan, &mut union, &keys)?;
            let mut step_raw_budget = raw::Budget::new(t)?;
            tree_handles.extend(open_tree(
                union.select(index)?,
                t,
                step,
                StepPhase::Parked,
                &mut step_raw_budget,
            )?);
            step_raw_budget.validate(self, t)?;
            compound_failpoint(&format!("park{index}"))?;
        }
        if relocations.iter().any(|step| step.parking.is_some()) {
            compound_failpoint("parked")?;
        }
        for index in placement::order(relocations, false) {
            let step = &relocations[index];
            placement::verify_vacancies(relocations, &current_phases)?;
            // Releasing only this root's descendants is required by NTFS parent
            // rename. All other roots and the union ancestor closure stay held.
            tree_handles.retain(|(file, _)| !placement::owns(t, &t.files[*file].path, step));
            union.release_source_descendants(index)?;
            let namespace = union.select(index)?;
            if step.parking.is_none() {
                rename_original_leaf(namespace, t, step)?;
                compound_failpoint(&format!("leaf{index}"))?;
            }
            move_final(self, plan, &mut union, index, step)?;
            current_phases[index] = StepPhase::After;
            placement::verify_vacancies(relocations, &current_phases)?;
            // Source descendants had to be released for NTFS rename. Rebind
            // only through the same original root and retained materialization.
            self.retain_compound_directories(plan, &mut union, &keys)?;
            let namespace = union.select(index)?;
            let mut step_raw_budget = raw::Budget::new(t)?;
            tree_handles.extend(open_tree(
                namespace,
                t,
                step,
                StepPhase::After,
                &mut step_raw_budget,
            )?);
            step_raw_budget.validate(self, t)?;
            compound_failpoint(&format!("move{index}"))?;
        }
        let mut preserved = Vec::new();
        for (index, retired) in t.retired_members().iter().enumerate() {
            let original = t
                .files
                .iter()
                .position(|file| locator_equal(&file.path, &retired.source))
                .unwrap();
            let mut file = if let Some(position) = tree_handles
                .iter()
                .position(|(member, _)| *member == original)
            {
                tree_handles.remove(position).1
            } else {
                let position = outside_handles
                    .iter()
                    .position(|(member, _)| *member == original)
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_retirement_lacks_original_guarded_file(
                            ),
                        )
                    })?;
                outside_handles.remove(position).1
            };
            retirement::check_file(t, retired, &mut file)?;
            union.move_file_to(&file, Path::new(&retired.target))?;
            retirement::verify_held(self, plan, &union, retired, &mut file)?;
            preserved.push((index, file));
            compound_failpoint(&format!("retire{index}"))?;
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
                .filter(|(_, step)| relative(&change.path, &step.target).is_some())
                .max_by_key(|(_, step)| Path::new(&step.target).components().count())
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
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_operations_compound_mapped_participant_missing(),
                    )
                })?;
            if change.before.as_ref() != Some(&change.after) {
                write_blob(file, &change.after)?;
            }
            if read_blob(file)? != change.after {
                return Err(CoreError::new(
                    "recoveryPending",
                    UiMessage::error_core_operations_compound_after_image_verification_failed(),
                ));
            }
            materialized.push((path, stamp(file)?));
            compound_failpoint(&format!("content{index}"))?;
        }
        for (path, file) in &mut created {
            materialized.push((path.clone(), stamp(file)?));
        }
        let mut raw_budget = raw::Budget::new(t)?;
        for (index, step) in relocations.iter().enumerate() {
            check_membership(
                union.select(index)?,
                t,
                step,
                StepPhase::After,
                true,
                &mut raw_budget,
            )?;
        }
        raw_budget.validate(self, t)?;
        verify_created_membership(t, &union, &keys)?;
        for (index, file) in &mut preserved {
            retirement::verify_held(self, plan, &union, &t.retired_members()[*index], file)?;
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
        validate_compound_namespace(t)?;
        self.validate_compound_pending(plan)?;
        let relocations = steps(t)?;
        let phases = placement::phases(relocations)?;
        let keys = directory_keys(t, &phases);
        let mut union = placement::acquire(relocations, &phases, &keys)?;
        placement::verify_vacancies(relocations, &phases)?;
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
                    UiMessage::error_core_operations_create_only_directory_lost_exact_original_materialization(),
                ));
            }
            directory.description = Some(UiMessage::recovery_compound_directory_parent().into());
            capture.members.push(directory);
        }
        verify_created_membership(t, &union, &keys)?;
        let mut retirement_guards = Vec::new();
        for proof in retirement::capture(self, plan, &mut union, &phases, write)? {
            capture.files.push((capture.members.len(), proof.file));
            capture.members.push(proof.member);
            if let Some(guard) = proof.guard {
                retirement_guards.push(guard);
            }
        }
        capture.changes = t.relocated_files();
        let mut raw_budget = raw::Budget::new(t)?;
        for (index, step) in relocations.iter().enumerate() {
            let phase = phases[index];
            check_step_identity(self, plan, &mut union, index, step, phase, &phases)?;
            if phase == StepPhase::After {
                // Recovery reacquired every moving root with DELETE, including
                // already placed parents whose remaining children must enter.
                protect_final_source(&mut union, index, t, step)?;
            }
            let namespace = union.select(index)?;
            check_membership(namespace, t, step, phase, true, &mut raw_budget)?;
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
                StepPhase::Parked => "knownIntermediate",
                StepPhase::After => "knownAfter",
            }
            .into();
            root.description = Some(UiMessage::recovery_compound_namespace_retained().into());
            capture.members.push(root);
            for original in &t.files {
                if t.is_retired_member(&original.path) {
                    continue;
                }
                let Some(path) = source_relative(t, original, step, phase) else {
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
                m.target_path = Some(display(&placement::coordinate(step, phase).join(&path)));
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
                m.description = Some(UiMessage::recovery_compound_file_retained().into());
                if !before && !after {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_compound_file_neither_original_before_nor_exact_after_object(),
                    ));
                }
                capture.files.push((capture.members.len(), held));
                capture.members.push(m);
                // Retain an explicit index, never zip independently filtered
                // sources and handles. Every original file is represented once.
            }
            for entry in &step.members {
                let original_path = display(&Path::new(&step.source).join(&entry.relative_path));
                if !placement::owns(t, &original_path, step) || t.is_retired_member(&original_path)
                {
                    continue;
                }
                if namespace.source_parent_identity(Path::new(&entry.relative_path))?
                    != entry.parent
                {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_original_compound_member_parent_changed(),
                    ));
                }
                if !t.files.iter().any(|file| {
                    file.stamp.identity == entry.identity
                        && relative(&file.path, &step.source).as_ref()
                            == Some(&PathBuf::from(&entry.relative_path))
                }) {
                    return Err(CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_operations_compound_placement_member_lacks_original_physical_input(),
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
                    change.before.is_none() && placement::owns_final(t, &change.path, step)
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
                                UiMessage::error_core_operations_original_created_compound_object_disappeared(),
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
                                    UiMessage::error_core_operations_compound_create_only_output_not_original_materialized_before_after_object(),
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
                    m.description = Some(UiMessage::recovery_compound_create_output().into());
                    capture.members.push(m);
                }
            }
        }
        // Namespace-root handles stay owned by the recovery capture, including
        // before/intermediate roots that an explicit original continuation moves.
        raw_budget.validate(self, t)?;
        capture.compound = Some(CompoundCapture {
            namespace: union,
            phases,
            directory_keys: keys,
            retirement_guards,
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
        {
            let compound = capture.compound.as_mut().ok_or_else(|| {
                CoreError::new(
                    "recoveryConflict",
                    UiMessage::error_core_operations_original_compound_namespace_proof_unavailable(
                    ),
                )
            })?;
            self.materialize_compound_directories(
                plan,
                &mut compound.namespace,
                &compound.directory_keys,
            )?;
        }
        for directory in capture
            .members
            .iter_mut()
            .filter(|member| member.kind == "directory" && member.role == "write")
        {
            directory.matches_after = true;
        }
        // Complete every original parking step before filling any final slot.
        // A restart keeps the same frozen locations and directory identities.
        for next in [StepPhase::Parked, StepPhase::After] {
            for index in placement::order(relocations, next == StepPhase::Parked) {
                let step = &relocations[index];
                let phase = capture.compound.as_ref().unwrap().phases[index];
                if phase == StepPhase::After
                    || (next == StepPhase::Parked
                        && (step.parking.is_none() || phase == StepPhase::Parked))
                {
                    continue;
                }
                self.advance_compound_recovery_step(plan, capture, index, next)?;
            }
        }
        retirement::refresh(self, plan, capture)?;
        for (index, retired) in t.retired_members().iter().enumerate() {
            let path = t.relocated_path(&retired.source);
            let member = capture
                .members
                .iter()
                .position(|member| member.role == "memberRetirement" && member.path == path)
                .unwrap();
            if capture.members[member].matches_after {
                continue;
            }
            let file = &mut capture
                .files
                .iter_mut()
                .find(|(index, _)| *index == member)
                .unwrap()
                .1;
            let compound = capture.compound.as_ref().unwrap();
            compound
                .namespace
                .move_file_to(file, Path::new(&retired.target))?;
            retirement::verify_held(self, plan, &compound.namespace, retired, file)?;
            capture.members[member].matches_before = false;
            capture.members[member].matches_after = true;
            capture.members[member].state = "knownAfter".into();
            capture.members[member].target_path = Some(retired.target.clone());
            compound_failpoint(&format!("retire{index}"))?;
        }
        Ok(())
    }

    fn advance_compound_recovery_step(
        &self,
        plan: &Plan,
        capture: &mut RecoveryCapture,
        index: usize,
        next: StepPhase,
    ) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let step = &steps(t)?[index];
        let compound = capture.compound.as_mut().unwrap();
        let phase = compound.phases[index];
        let current = placement::coordinate(step, phase);
        capture.files.retain(|(member_index, _)| {
            !capture.members[*member_index]
                .target_path
                .as_ref()
                .is_some_and(|path| relative(path, &display(current)).is_some())
        });
        placement::verify_vacancies(steps(t)?, &compound.phases)?;
        compound.namespace.release_source_descendants(index)?;
        let namespace = compound.namespace.select(index)?;
        if phase == StepPhase::Before {
            rename_original_leaf(namespace, t, step)?;
        }
        let target = placement::coordinate(step, next);
        if next == StepPhase::After {
            move_final(self, plan, &mut compound.namespace, index, step)?;
        } else {
            compound.namespace.move_source_to(index, target)?;
        }
        compound.phases[index] = next;
        placement::verify_vacancies(steps(t)?, &compound.phases)?;
        self.retain_compound_directories(plan, &mut compound.namespace, &compound.directory_keys)?;
        let namespace = compound.namespace.select(index)?;
        let mut step_raw_budget = raw::Budget::new(t)?;
        check_membership(namespace, t, step, next, true, &mut step_raw_budget)?;
        step_raw_budget.validate(self, t)?;
        for original in &t.files {
            if t.is_retired_member(&original.path) {
                continue;
            }
            let Some(path) = source_relative(t, original, step, next) else {
                continue;
            };
            let final_path = t.relocated_path(&original.path);
            let member_index = capture
                .members
                .iter()
                .position(|member| {
                    member.role == "write" && locator_equal(&member.path, &final_path)
                })
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
                    UiMessage::error_core_operations_compound_original_file_changed_during_namespace_continuation(),
                ));
            }
            capture.members[member_index].matches_after =
                next == StepPhase::After && bytes == change.after;
            capture.members[member_index].target_path = Some(display(&target.join(&path)));
            capture.files.push((member_index, file));
        }
        let root = capture
            .members
            .iter_mut()
            .find(|member| member.role == "placement" && member.path == step.source)
            .unwrap();
        root.matches_before = false;
        root.matches_after = next == StepPhase::After;
        root.state = if next == StepPhase::After {
            "knownAfter"
        } else {
            "knownIntermediate"
        }
        .into();
        retirement::refresh(self, plan, capture)?;
        Ok(())
    }
    pub(crate) fn verify_compound_final(
        &self,
        plan: &Plan,
        capture: &mut RecoveryCapture,
    ) -> Result<()> {
        let t = plan.transaction.as_ref().unwrap();
        let compound = capture.compound.as_mut().ok_or_else(|| {
            CoreError::new(
                "recoveryConflict",
                UiMessage::error_core_operations_original_namespace_proof_missing(),
            )
        })?;
        let mut raw_budget = raw::Budget::new(t)?;
        for (index, step) in steps(t)?.iter().enumerate() {
            if compound.phases[index] != StepPhase::After {
                return Err(CoreError::new(
                    "recoveryConflict",
                    UiMessage::error_core_operations_original_namespace_incomplete(),
                ));
            }
            check_membership(
                compound.namespace.select(index)?,
                t,
                step,
                StepPhase::After,
                true,
                &mut raw_budget,
            )?;
        }
        raw_budget.validate(self, t)?;
        verify_created_membership(t, &compound.namespace, &compound.directory_keys)?;
        for retired in t.retired_members() {
            let path = t.relocated_path(&retired.source);
            let index = capture
                .members
                .iter()
                .position(|member| member.role == "memberRetirement" && member.path == path)
                .unwrap();
            let file = &mut capture
                .files
                .iter_mut()
                .find(|(member, _)| *member == index)
                .ok_or_else(|| {
                    CoreError::new(
                        "recoveryConflict",
                        UiMessage::error_core_operations_original_retired_file_proof_missing(),
                    )
                })?
                .1;
            retirement::verify_held(self, plan, &compound.namespace, retired, file)?;
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
            if !placement::owns_final(t, path, step) {
                continue;
            }
            if let Some(relative) = relative(path, &step.target) {
                if compound.phases[index] != StepPhase::After {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        UiMessage::error_core_operations_create_only_output_s_original_placement_incomplete(),
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
            UiMessage::error_core_operations_interrupted_original_compound_operation(),
        ));
    }
    let _ = name;
    Ok(())
}
