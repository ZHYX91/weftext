//! Intermediate locations belong to the original Compound Plan. All roots
//! retain their original directory/file identities; parking never copies data.
use super::*;
use weftext_messages::UiMessage;

pub(super) fn coordinate(step: &RelocationStep, phase: StepPhase) -> &Path {
    Path::new(match phase {
        StepPhase::After => &step.target,
        StepPhase::Parked => step.parking.as_ref().expect("validated original parking"),
        _ => &step.source,
    })
}

pub(super) fn forest(steps: &[RelocationStep]) -> bool {
    steps.iter().any(|step| {
        steps.iter().any(|other| {
            relative(&step.source, &other.source)
                .is_some_and(|relative| !relative.as_os_str().is_empty())
                || relative(&step.target, &other.target)
                    .is_some_and(|relative| !relative.as_os_str().is_empty())
        })
    })
}

pub(super) fn owns(t: &FrozenOperation, path: &str, step: &RelocationStep) -> bool {
    let Ok(steps) = steps(t) else {
        return false;
    };
    steps
        .iter()
        .filter(|candidate| relative(path, &candidate.source).is_some())
        .max_by_key(|candidate| Path::new(&candidate.source).components().count())
        .is_some_and(|owner| owner.source == step.source)
}

pub(super) fn owns_final(t: &FrozenOperation, path: &str, step: &RelocationStep) -> bool {
    let Ok(steps) = steps(t) else {
        return false;
    };
    steps
        .iter()
        .filter(|candidate| relative(path, &candidate.target).is_some())
        .max_by_key(|candidate| Path::new(&candidate.target).components().count())
        .is_some_and(|owner| owner.source == step.source)
}

pub(super) fn target_parent_path(
    steps: &[RelocationStep],
    phases: &[StepPhase],
    step: &RelocationStep,
) -> PathBuf {
    let parent = Path::new(&step.target).parent().unwrap();
    steps
        .iter()
        .zip(phases)
        .filter_map(|(ancestor, phase)| {
            relative(&display(parent), &ancestor.target).map(|relative| {
                (
                    Path::new(&ancestor.target).components().count(),
                    coordinate(ancestor, *phase).join(relative),
                )
            })
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, path)| path)
        .unwrap_or_else(|| parent.into())
}

pub(super) fn source_parent_path(
    steps: &[RelocationStep],
    phases: &[StepPhase],
    step: &RelocationStep,
) -> PathBuf {
    let parent = Path::new(&step.source).parent().unwrap();
    steps
        .iter()
        .zip(phases)
        .filter_map(|(ancestor, phase)| {
            relative(&display(parent), &ancestor.source).map(|relative| {
                (
                    Path::new(&ancestor.source).components().count(),
                    coordinate(ancestor, *phase).join(relative),
                )
            })
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, path)| path)
        .unwrap_or_else(|| parent.into())
}

fn source_slot(steps: &[RelocationStep], phases: &[StepPhase], step: &RelocationStep) -> PathBuf {
    source_parent_path(steps, phases, step).join(Path::new(&step.source).file_name().unwrap())
}

pub(super) fn order(steps: &[RelocationStep], parking: bool) -> Vec<usize> {
    let mut order = (0..steps.len()).collect::<Vec<_>>();
    if steps.iter().any(|step| step.parking.is_some()) {
        if parking {
            order.sort_by_key(|index| {
                std::cmp::Reverse(Path::new(&steps[*index].source).components().count())
            });
        } else {
            order.sort_by_key(|index| Path::new(&steps[*index].target).components().count());
        }
    }
    order
}

pub(super) fn validate(t: &FrozenOperation) -> Result<()> {
    let steps = steps(t)?;
    let enabled = steps.iter().any(|step| step.parking.is_some());
    if !enabled {
        return Ok(());
    }
    if steps.iter().any(|step| step.parking.is_none()) {
        return Err(CoreError::new(
            "invalidPlan",
            UiMessage::error_core_operations_compound_parking_must_cover_complete_participant_set(),
        ));
    }
    for (index, step) in steps.iter().enumerate() {
        let path = Path::new(step.parking.as_ref().unwrap());
        let parking_parent = if forest(steps) {
            Path::new(&t.request.root)
        } else {
            Path::new(&step.source).parent().unwrap()
        };
        let expected =
            parking_parent.join(format!("WeftextMove-{}-{index}", t.request.operation_id));
        if crate::backend::namespace_path_compare(path, &expected)? != std::cmp::Ordering::Equal {
            return Err(CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_parking_not_original_operation_s_frozen_sibling_slot(),
            ));
        }
        for other in steps {
            for root in [&other.source, &other.target] {
                if crate::backend::namespace_relative_path(path, Path::new(root))?.is_some()
                    || crate::backend::namespace_relative_path(Path::new(root), path)?.is_some()
                {
                    return Err(CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_operations_parking_overlaps_original_or_final_participant(),
                    ));
                }
            }
        }
        for file in t
            .files
            .iter()
            .map(|file| file.path.as_str())
            .chain(t.absent_dependencies.iter().map(|file| file.path.as_str()))
            .chain(t.relocated_files().iter().map(|file| file.path.as_str()))
        {
            if crate::backend::namespace_relative_path(Path::new(file), path)?.is_some()
                || crate::backend::namespace_relative_path(path, Path::new(file))?.is_some()
            {
                return Err(CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_operations_parking_overlaps_original_read_or_content_slot(
                    ),
                ));
            }
        }
        for directory in creation_directories(t) {
            if crate::backend::namespace_relative_path(Path::new(directory), path)?.is_some()
                || crate::backend::namespace_relative_path(path, Path::new(directory))?.is_some()
            {
                return Err(CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_operations_parking_overlaps_create_only_directory(),
                ));
            }
        }
    }
    Ok(())
}

fn slots(steps: &[RelocationStep]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::<PathBuf>::new();
    for step in steps {
        for path in [&step.source, &step.target]
            .into_iter()
            .chain(step.parking.iter())
        {
            let path = PathBuf::from(path);
            let mut found = false;
            for previous in &paths {
                if crate::backend::namespace_path_compare(previous, &path)?
                    == std::cmp::Ordering::Equal
                {
                    found = true;
                }
            }
            if !found {
                paths.push(path)
            }
        }
    }
    Ok(paths)
}

pub(super) fn phases(steps: &[RelocationStep]) -> Result<Vec<StepPhase>> {
    if steps.iter().all(|step| step.parking.is_none()) {
        return steps.iter().map(phase).collect();
    }
    let observed = slots(steps)?
        .into_iter()
        .map(|path| {
            let identity = bound_directory(&path)?;
            Ok((path, identity))
        })
        .collect::<Result<Vec<_>>>()?;
    let phases = steps
        .iter()
        .map(|step| {
            let mut locations = observed
                .iter()
                .filter(|(_, identity)| identity.as_ref() == Some(&step.source_identity));
            let (path, _) = locations.next().ok_or_else(|| {
                CoreError::new("thirdState", UiMessage::error_core_operations_original_moving_directory_missing())
            })?;
            if locations.next().is_some() {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_operations_original_moving_directory_occupies_ambiguous_slots(),
                ));
            }
            let same = |other: &str| {
                crate::backend::namespace_path_compare(path, Path::new(other))
                    .map(|order| order == std::cmp::Ordering::Equal)
            };
            let state = if same(&step.source)? {
                StepPhase::Before
            } else if step
                .parking
                .as_ref()
                .map(|parking| same(parking))
                .transpose()?
                .unwrap_or(false)
            {
                StepPhase::Parked
            } else if same(&step.target)? {
                StepPhase::After
            } else {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_operations_original_directory_outside_own_frozen_positions(),
                ));
            };
            let old = capture_optional(&path.join(&step.old_leaf))?;
            let new = if step.old_leaf == step.new_leaf {
                None
            } else {
                capture_optional(&path.join(&step.new_leaf))?
            };
            if step.old_leaf == step.new_leaf {
                return if old.is_some_and(|file| file.stamp.identity == step.leaf_identity) {
                    Ok(state)
                } else {
                    Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_original_canonical_file_missing_or_replaced(),
                    ))
                };
            }
            match (state, old, new) {
                (StepPhase::Before, Some(file), None)
                    if file.stamp.identity == step.leaf_identity =>
                {
                    Ok(StepPhase::Before)
                }
                (StepPhase::Before, None, Some(file))
                    if file.stamp.identity == step.leaf_identity =>
                {
                    Ok(StepPhase::LeafRenamed)
                }
                (state, None, Some(file)) if file.stamp.identity == step.leaf_identity => Ok(state),
                _ => Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_operations_original_canonical_leaf_outside_exact_frozen_phase(),
                )),
            }
        })
        .collect::<Result<Vec<_>>>()?;
    for path in phase_slots(steps, &phases) {
        if bound_directory(&path)?
            .as_ref()
            .is_some_and(|identity| !steps.iter().any(|step| &step.source_identity == identity))
        {
            return Err(CoreError::new(
                "thirdState",
                UiMessage::error_core_operations_original_compound_namespace_slot_has_foreign_directory(),
            ));
        }
    }
    Ok(phases)
}

fn phase_slots(steps: &[RelocationStep], phases: &[StepPhase]) -> Vec<PathBuf> {
    steps
        .iter()
        .flat_map(|step| {
            [
                Some(source_slot(steps, phases, step)),
                Some(
                    target_parent_path(steps, phases, step)
                        .join(Path::new(&step.target).file_name().unwrap()),
                ),
                step.parking.as_ref().map(PathBuf::from),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}

pub(super) fn verify_vacancies(steps: &[RelocationStep], phases: &[StepPhase]) -> Result<()> {
    for path in phase_slots(steps, phases) {
        let mut owned = false;
        for (step, phase) in steps.iter().zip(phases) {
            if crate::backend::namespace_path_compare(&path, coordinate(step, *phase))?
                == std::cmp::Ordering::Equal
            {
                owned = true;
            }
        }
        if !owned {
            ensure_absent(&path)?;
        }
    }
    Ok(())
}

pub(super) fn acquire(
    steps: &[RelocationStep],
    phases: &[StepPhase],
    keys: &BTreeMap<String, PathBuf>,
) -> Result<MultiNamespaceGuard> {
    if steps.is_empty() {
        let planned = keys.values().cloned().collect::<BTreeSet<_>>();
        let mut anchors = BTreeSet::new();
        for path in &planned {
            let mut anchor = path.as_path();
            while planned.contains(anchor) {
                anchor = anchor.parent().ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_operations_preservation_directory_lacks_original_ancestor(),
                    )
                })?;
            }
            anchors.insert(anchor.to_path_buf());
        }
        return MultiNamespaceGuard::acquire_anchors(
            &anchors.into_iter().collect::<Vec<_>>(),
            &planned.into_iter().collect::<Vec<_>>(),
        );
    }
    let parents = steps
        .iter()
        .map(|step| target_parent_path(steps, phases, step))
        .collect::<Vec<_>>();
    let pairs = steps
        .iter()
        .zip(phases)
        .zip(&parents)
        .map(|((step, phase), parent)| (coordinate(step, *phase), parent.as_path()))
        .collect::<Vec<_>>();
    let ancestors = steps
        .iter()
        .filter(|step| step.parking.is_some())
        .flat_map(|step| {
            [
                source_parent_path(steps, phases, step),
                Path::new(step.parking.as_ref().unwrap())
                    .parent()
                    .unwrap()
                    .into(),
            ]
        })
        .collect::<BTreeSet<PathBuf>>()
        .into_iter()
        .collect::<Vec<_>>();
    if ancestors.is_empty() {
        return if keys.is_empty() {
            MultiNamespaceGuard::acquire(&pairs)
        } else {
            MultiNamespaceGuard::acquire_planned(
                &pairs,
                &keys.values().cloned().collect::<Vec<_>>(),
            )
        };
    }
    if forest(steps) {
        return MultiNamespaceGuard::acquire_forest_with_ancestors(
            &pairs,
            &keys.values().cloned().collect::<Vec<_>>(),
            &ancestors,
        );
    }
    MultiNamespaceGuard::acquire_with_ancestors(
        &pairs,
        &keys.values().cloned().collect::<Vec<_>>(),
        &ancestors,
    )
}
