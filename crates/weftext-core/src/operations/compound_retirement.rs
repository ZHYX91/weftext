//! Original Resource leaves move to their frozen preservation slots. This is
//! part of the enclosing Compound decision, not a second writer or admission.
use super::*;
use weftext_messages::UiMessage;

pub(super) struct Proof {
    pub member: RecoveryMember,
    pub file: backend::BoundFile,
    pub guard: Option<ParentGuard>,
}

pub(super) fn validate(t: &FrozenOperation) -> Result<()> {
    let mut identities = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for (index, retired) in t.retired_members().iter().enumerate() {
        let expected = Path::new(&t.request.root)
            .join(".weftext-trash")
            .join(format!("{}-members", t.request.operation_id))
            .join(format!("{index}-{}", retired.resource_id));
        let original = t
            .files
            .iter()
            .find(|file| locator_equal(&file.path, &retired.source))
            .ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_operations_retirement_lacks_original_captured_file(),
                )
            })?;
        if !matches!(retired.object, crate::sync::ObjectKey::Node(_))
            || retired.identity != original.stamp.identity
            || retired.digest != hash(&original.bytes)
            || crate::backend::namespace_path_compare(&expected, Path::new(&retired.target))?
                != std::cmp::Ordering::Equal
            || !creation_directories(t)
                .iter()
                .any(|path| Path::new(path) == expected.parent().unwrap())
            || !identities.insert(serde_json::to_string(&retired.identity)?)
            || !sources.insert(retired.source.clone())
        {
            return Err(CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_retirement_differs_from_exact_original_plan(),
            ));
        }
        valid_operation_id(&retired.resource_id)?;
        valid_operation_id(&retired.file_revision)?;
    }
    if t.retired_members().len() > MAX_OPERATION_FILES {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_core_operations_too_many_retired_members(),
        ));
    }
    Ok(())
}

fn optional_file(result: Result<backend::BoundFile>) -> Result<Option<backend::BoundFile>> {
    match result {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.code == "missing" => Ok(None),
        Err(error) => Err(error),
    }
}

pub(super) fn check_file(
    t: &FrozenOperation,
    retired: &RetiredMember,
    file: &mut backend::BoundFile,
) -> Result<Vec<u8>> {
    let original = t
        .files
        .iter()
        .find(|file| locator_equal(&file.path, &retired.source))
        .unwrap();
    let bytes = read_blob(file)?;
    if stamp(file)? != original.stamp
        || bytes != original.bytes
        || crate::backend::attributes(file)? != retired.attributes
    {
        return Err(CoreError::new(
            "thirdState",
            UiMessage::error_core_operations_retired_member_changed_original_identity_metadata_or_bytes(),
        ));
    }
    Ok(bytes)
}

pub(super) fn capture(
    core: &Core,
    plan: &Plan,
    union: &mut MultiNamespaceGuard,
    phases: &[StepPhase],
    write: bool,
) -> Result<Vec<Proof>> {
    let t = plan.transaction.as_ref().unwrap();
    validate(t)?;
    let mut proofs = Vec::new();
    for retired in t.retired_members() {
        let mut source_guard = None;
        let source = if let Some((index, step)) = steps(t)?
            .iter()
            .enumerate()
            .find(|(_, step)| placement::owns(t, &retired.source, step))
        {
            let relative = relative(&retired.source, &step.source).unwrap();
            let namespace = union.select(index)?;
            if namespace.source_parent_identity(&relative)? != retired.parent {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_operations_retired_member_original_parent_changed(),
                ));
            }
            optional_file(namespace.open_source_retiring(&relative, write))?
        } else {
            let guard = ParentGuard::acquire(&core.backend, Path::new(&retired.source))?;
            if guard.directory_identity()? != retired.parent {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_operations_retired_member_original_parent_changed(),
                ));
            }
            let file = optional_file(guard.open_retiring_file(write))?;
            source_guard = Some(guard);
            file
        };
        let expected_parent = core.materialized_identity(
            &plan.operation_id,
            Path::new(&retired.target).parent().unwrap(),
        )?;
        let mut target_guard = None;
        let target = match ParentGuard::acquire(&core.backend, Path::new(&retired.target)) {
            Ok(guard) => {
                if Some(guard.directory_identity()?) != expected_parent {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_retirement_preservation_parent_lacks_original_materialization(),
                    ));
                }
                let file = optional_file(guard.open_retiring_file(write))?;
                target_guard = Some(guard);
                file
            }
            Err(error) if error.code == "missing" && expected_parent.is_none() => None,
            Err(error) => return Err(error),
        };
        let (mut file, after, guard) = match (source, target) {
            (Some(file), None) => (file, false, source_guard),
            (None, Some(file)) => (file, true, target_guard),
            _ => {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_operations_retired_member_missing_or_occupies_both_original_and_preservation_slots(),
                ));
            }
        };
        let bytes = check_file(t, retired, &mut file)?;
        let path = t.relocated_path(&retired.source);
        let mut member = member(
            &path,
            "memberRetirement",
            Some(&bytes),
            Some(&bytes),
            Some(&retired.identity),
        );
        member.target_path = Some(if after {
            retired.target.clone()
        } else {
            display(&physical_path(t, &path, phases))
        });
        member.current = RecoveryBlob::from_bytes(Some(&bytes));
        member.current_identity = Some((&retired.identity).into());
        member.identity_binding = "original".into();
        member.matches_before = !after;
        member.matches_after = after;
        member.state = if after { "knownAfter" } else { "knownBefore" }.into();
        member.description = Some(UiMessage::recovery_retired_resource_retained().into());
        proofs.push(Proof {
            member,
            file,
            guard,
        });
    }
    Ok(proofs)
}

pub(super) fn refresh(core: &Core, plan: &Plan, capture: &mut RecoveryCapture) -> Result<()> {
    capture
        .files
        .retain(|(index, _)| capture.members[*index].role != "memberRetirement");
    let compound = capture.compound.as_mut().unwrap();
    compound.retirement_guards.clear();
    let proofs = self::capture(core, plan, &mut compound.namespace, &compound.phases, true)?;
    for proof in proofs {
        let index = capture
            .members
            .iter()
            .position(|member| {
                member.role == "memberRetirement" && member.path == proof.member.path
            })
            .unwrap();
        capture.members[index] = proof.member;
        capture.files.push((index, proof.file));
        if let Some(guard) = proof.guard {
            compound.retirement_guards.push(guard);
        }
    }
    Ok(())
}

pub(super) fn verify_held(
    core: &Core,
    plan: &Plan,
    union: &mut MultiNamespaceGuard,
    phases: &[StepPhase],
    retired: &RetiredMember,
    file: &mut backend::BoundFile,
) -> Result<()> {
    let t = plan.transaction.as_ref().unwrap();
    check_file(t, retired, file)?;
    let parent = Path::new(&retired.target).parent().unwrap();
    if union.directory_identity(parent)?
        != core.materialized_identity(&plan.operation_id, parent)?
    {
        return Err(CoreError::new(
            "thirdState",
            UiMessage::error_core_operations_retirement_preservation_parent_changed(),
        ));
    }
    let coordinate = crate::backend::file_coordinates(file)?;
    let expected = crate::backend::file_coordinates(union.directory_file(parent)?)?
        .join(Path::new(&retired.target).file_name().unwrap());
    if crate::backend::namespace_path_compare(&coordinate, &expected)? != std::cmp::Ordering::Equal
    {
        return Err(CoreError::new(
            "thirdState",
            UiMessage::error_core_operations_original_retired_member_outside_frozen_preservation_slot(),
        ));
    }
    placement::ensure_absent(
        &core.backend,
        union,
        steps(t)?,
        phases,
        Path::new(&t.relocated_path(&retired.source)),
    )?;
    Ok(())
}
