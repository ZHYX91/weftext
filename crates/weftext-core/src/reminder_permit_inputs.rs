//! Original read-only witnesses carried by one short-lived, non-wire permit.
use super::*;
use std::cell::RefCell;

#[derive(PartialEq)]
struct JournalRecordWitness {
    parent: FileIdentity,
    stamp: FileStamp,
    digest: String,
}

pub(super) struct ReminderJournalFence {
    _directory: ParentGuard,
    directory_identity: FileIdentity,
    data_dir: PathBuf,
    operations: BTreeMap<String, String>,
    records: BTreeMap<PathBuf, JournalRecordWitness>,
    handles: BTreeMap<PathBuf, RefCell<backend::BoundFile>>,
    record_order: Vec<PathBuf>,
    committed_order: Vec<String>,
}
impl ReminderJournalFence {
    pub(super) fn capture(core: &Core) -> Result<Self> {
        let journal = core.data_dir.join("journal");
        let directory = ParentGuard::acquire(&core.backend, &journal.join(".reminder-proof"))?;
        let directory_identity = directory.directory_identity()?;
        let operation_order = core.operations.keys().cloned().collect::<Vec<_>>();
        let operations = core
            .operations
            .iter()
            .map(|(id, plan)| (id.clone(), plan.fingerprint.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut records = BTreeMap::new();
        let mut handles = BTreeMap::new();
        let mut record_order = Vec::new();
        let mut total_bytes = 0u64;
        for entry in core.backend.read_dir(&journal)? {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let Some((operation, role)) = name.split_once('.') else {
                continue;
            };
            if !operations.contains_key(operation)
                || !(matches!(
                    role,
                    "commit.json" | "birth.json" | "births.json" | "observations.json"
                ) || (role.starts_with("recovery-observations-") && role.ends_with(".json")))
            {
                continue;
            }
            let guard = ParentGuard::acquire(&core.backend, &path)?;
            let parent = guard.directory_identity()?;
            if parent != directory_identity {
                return Err(changed());
            }
            let mut file = guard.open_read_guard_file()?;
            let original = stamp(&file)?;
            total_bytes = total_bytes
                .checked_add(original.size)
                .ok_or_else(|| err(UiMessage::error_reminder_input_byte_overflow()))?;
            if total_bytes > MAX_INPUT_BYTES as u64 {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_reminder_input_guard_byte_budget(),
                ));
            }
            let bytes = read_blob(&mut file)?;
            if stamp(&file)? != original || bytes.len() as u64 != original.size {
                return Err(changed());
            }
            records.insert(
                path.clone(),
                JournalRecordWitness {
                    parent,
                    stamp: original,
                    digest: hash(&bytes),
                },
            );
            record_order.push(path.clone());
            handles.insert(path, RefCell::new(file));
        }
        let mut committed_order = Vec::new();
        for operation in operation_order {
            if records.contains_key(&core.journal_path(&operation, "commit")) {
                if !core.committed(&operation)? {
                    return Err(changed());
                }
                committed_order.push(operation);
            }
        }
        Ok(Self {
            _directory: directory,
            directory_identity,
            data_dir: core.data_dir.clone(),
            operations,
            records,
            handles,
            record_order,
            committed_order,
        })
    }
    pub(super) fn validate(&self, core: &Core) -> Result<()> {
        let current = Self::capture(core)?;
        if self.directory_identity != current.directory_identity
            || self.data_dir != current.data_dir
            || self.operations != current.operations
            || self.records != current.records
            || self.committed_order != current.committed_order
        {
            return Err(changed());
        }
        Ok(())
    }
    fn read_record(&self, path: &Path) -> Result<Option<Vec<u8>>> {
        let Some(handle) = self.handles.get(path) else {
            return Ok(None);
        };
        let original = &self.records[path];
        let mut file = handle.borrow_mut();
        if stamp(&file)? != original.stamp {
            return Err(changed());
        }
        let bytes = read_blob(&mut file)?;
        if stamp(&file)? != original.stamp || hash(&bytes) != original.digest {
            return Err(changed());
        }
        Ok(Some(bytes))
    }
    fn verify_author_history(
        &self,
        core: &Core,
        sources: &[(&TaskReminderCandidate, &operations::CapturedFile)],
    ) -> Result<()> {
        let plans = self
            .committed_order
            .iter()
            .map(|id| core.operations.get(id).ok_or_else(changed))
            .collect::<Result<Vec<_>>>()?;
        let mut original_births = BTreeMap::new();
        for plan in &plans {
            let single = self.read_record(&core.journal_path(&plan.operation_id, "birth"))?;
            let multiple = self.read_record(&core.journal_path(&plan.operation_id, "births"))?;
            original_births.insert(
                plan.operation_id.clone(),
                core.decode_committed_births_in_cut(
                    &plan.operation_id,
                    single.as_deref(),
                    multiple.as_deref(),
                )?,
            );
        }
        let mut births = Vec::new();
        for (candidate, source) in sources {
            let birth = plans
                .iter()
                .flat_map(|plan| &original_births[&plan.operation_id])
                .find(|birth| birth.identity == source.stamp.identity)
                .ok_or_else(|| err(UiMessage::error_reminder_live_task_unavailable()))?;
            if birth.node_id != candidate.owner.node_id
                || birth.workspace_id != candidate.owner.workspace_id
            {
                return Err(changed());
            }
            births.push(birth.clone());
        }
        let placements =
            core.node_placement_paths_in_cut(&plans, &births, None, Some(&original_births))?;
        for ((candidate, source), places) in sources.iter().zip(placements) {
            let path = Path::new(&candidate.source_path);
            core.check_history_placement_paths(path, &places)?;
            let guard = ParentGuard::acquire(&core.backend, path)?;
            let file = guard.open_read_guard_file()?;
            if stamp(&file)? != source.stamp {
                return Err(changed());
            }
            let coordinate = backend::file_coordinates(&file)?;
            core.check_query_node_placement_paths(
                Path::new(&candidate.root),
                &coordinate,
                &places,
            )?;
        }
        let files = sources
            .iter()
            .map(|(_, source)| *source)
            .collect::<Vec<_>>();
        core.accepted_revisions_with_facts(&plans, &files, |id| {
            let paths = self.record_order.iter().filter(|path| {
                operations::observation_record_name_matches(
                    &path.file_name().unwrap_or_default().to_string_lossy(),
                    id,
                )
            });
            core.decode_original_observation_records(
                id,
                paths.map(|path| self.read_record(path)?.ok_or_else(changed)),
                || Ok(true),
            )
        })?;
        Ok(())
    }
}

pub(super) struct ReminderPermitInputs {
    pub(super) members: BTreeMap<String, TaskReminderCandidate>,
    pub(super) roots: BTreeMap<String, Vec<SnapshotWitness>>,
    journal: ReminderJournalFence,
}
impl std::fmt::Debug for ReminderPermitInputs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReminderPermitInputs")
            .field("members", &self.members.len())
            .field(
                "input_paths",
                &self.roots.values().map(Vec::len).sum::<usize>(),
            )
            .field("journal_records", &self.journal.records.len())
            .finish()
    }
}
impl ReminderPermitInputs {
    // Producers call this inside the original input guards, after the complete
    // capture and fence recheck. Borrow first so a shared large Schema/control
    // is cloned only once, rather than once for every summary member.
    pub(super) fn new<'a>(
        core: &Core,
        journal: ReminderJournalFence,
        members: impl IntoIterator<Item = (&'a str, &'a TaskReminderCandidate, &'a [SnapshotWitness])>,
    ) -> Result<Self> {
        let mut admitted = BTreeMap::new();
        let mut inputs = BTreeMap::<&str, (&str, &SnapshotWitness)>::new();
        let mut budget = InputBudget::default();
        for (key, candidate, witnesses) in members {
            if admitted.len() >= MAX_SUMMARY_MEMBERS
                || admitted.insert(key.to_owned(), candidate.clone()).is_some()
            {
                return Err(err(UiMessage::error_reminder_summary_members_invalid()));
            }
            if !witnesses.iter().any(|witness| {
                witness.binding.path == candidate.source_path && witness.binding.captured.is_some()
            }) {
                return Err(err(
                    UiMessage::error_reminder_qualified_task_source_missing(),
                ));
            }
            for witness in witnesses {
                let basis = &witness.binding;
                if let Some((_, previous)) = inputs.get(basis.path.as_str()) {
                    let previous = &previous.binding;
                    if previous.parent != basis.parent
                        || previous
                            .captured
                            .as_ref()
                            .map(|file| (&file.stamp, &file.bytes))
                            != basis
                                .captured
                                .as_ref()
                                .map(|file| (&file.stamp, &file.bytes))
                    {
                        return Err(CoreError::new(
                            "staleBase",
                            UiMessage::error_reminder_dependency_conflict(),
                        ));
                    }
                    continue;
                }
                budget.include(
                    &basis.path,
                    basis.captured.as_ref().map_or(0, |file| file.bytes.len()),
                )?;
                inputs.insert(&basis.path, (&candidate.root, witness));
            }
        }
        if admitted.is_empty() {
            return Err(err(UiMessage::error_reminder_summary_members_invalid()));
        }
        let sources = admitted
            .values()
            .map(|candidate| {
                let (_, witness) = inputs
                    .get(candidate.source_path.as_str())
                    .ok_or_else(changed)?;
                let source = witness.binding.captured.as_ref().ok_or_else(changed)?;
                Ok((candidate, source))
            })
            .collect::<Result<Vec<_>>>()?;
        journal.verify_author_history(core, &sources)?;
        journal.validate(core)?;
        let mut roots = BTreeMap::<String, Vec<SnapshotWitness>>::new();
        for (root, witness) in inputs.into_values() {
            roots
                .entry(root.to_owned())
                .or_default()
                .push(witness.clone());
        }
        Ok(Self {
            members: admitted,
            roots,
            journal,
        })
    }
    pub(super) fn validate_journal(&self, core: &Core) -> Result<()> {
        let sources = self
            .members
            .values()
            .map(|candidate| {
                let source = self
                    .roots
                    .values()
                    .flatten()
                    .find(|witness| witness.binding.path == candidate.source_path)
                    .and_then(|witness| witness.binding.captured.as_ref())
                    .ok_or_else(changed)?;
                Ok((candidate, source))
            })
            .collect::<Result<Vec<_>>>()?;
        self.journal.verify_author_history(core, &sources)?;
        self.journal.validate(core)
    }
}

fn changed() -> CoreError {
    CoreError::new(
        "staleBase",
        UiMessage::error_reminder_input_changed_before_decision(),
    )
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn fixed_journal_cut_never_borrows_restored_required_observations() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let library = core.create_library(&root).unwrap();
        core.set_reminder_library(&root, true, 0, &fresh()).unwrap();
        let operation = fresh();
        let preview = core.prepare_operation(OperationRequest {
            operation_id: operation.clone(), root: display(&root),
            intent: OperationIntent::NewNode {
                parent: display(&root), name: "Task".into(),
                source: Some(format!("#metadata((types: (\"task\",), status: \"open\", due: \"2026-10-23T09:30:00Z\", reminder: (id: \"{}\", kind: \"absolute\", at: \"2026-10-23T09:00:00Z\"))) <weftext>\n= Task\n", fresh())),
            },
        }).unwrap();
        let owner = NodeRef {
            workspace_id: library.workspace_id,
            node_id: preview.new_node_id.clone().unwrap(),
        };
        core.execute_operation(&operation, &preview.plan_id)
            .unwrap();
        let observations = core.journal_path(&operation, "observations");
        let original = fs::read(&observations).unwrap();
        let withheld = temp.path().join("withheld-observations.json");
        let ledger_before = core
            .read_device_record(DeviceRecordKind::Reminders, MAX_BYTES)
            .unwrap();
        fs::rename(&observations, &withheld).unwrap();
        let original_cut = ReminderJournalFence::capture(&core).unwrap();
        assert!(original_cut.committed_order.contains(&operation));
        assert!(!original_cut.records.contains_key(&observations));
        fs::rename(&withheld, &observations).unwrap();
        // The real live reader succeeds after restoring the original producer's
        // record; the old cut must still decode its required-record absence.
        let (candidate, witnesses) = core.capture_reminder_task(&root, &owner).unwrap();
        let (state, _) = core.read_reminders().unwrap();
        let (id, definition, trigger) = ready(&candidate).unwrap();
        let key = execution_key(&state.device_id, &owner, id, definition, trigger).unwrap();
        let failure = core
            .with_reminder_inputs(&root, &witnesses, |core| {
                ReminderPermitInputs::new(
                    core,
                    original_cut,
                    [(key.as_str(), &candidate, witnesses.as_slice())],
                )
            })
            .unwrap_err();
        assert_eq!(failure.code, "identityPending");
        assert_eq!(
            failure.explanation,
            UiMessage::error_core_backend_committed_observations_missing()
        );
        let current_cut = ReminderJournalFence::capture(&core).unwrap();
        core.with_reminder_inputs(&root, &witnesses, |core| {
            ReminderPermitInputs::new(
                core,
                current_cut,
                [(key.as_str(), &candidate, witnesses.as_slice())],
            )
        })
        .unwrap();
        assert_eq!(fs::read(&observations).unwrap(), original);
        assert_eq!(
            core.read_device_record(DeviceRecordKind::Reminders, MAX_BYTES)
                .unwrap(),
            ledger_before
        );
    }

    #[test]
    fn fixed_journal_cut_never_adopts_a_new_commit_to_qualify_its_original_absence() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let library = core.create_library(&root).unwrap();
        core.set_reminder_library(&root, true, 0, &fresh()).unwrap();
        let operation = fresh();
        let preview = core.prepare_operation(OperationRequest {
            operation_id: operation.clone(), root: display(&root),
            intent: OperationIntent::NewNode {
                parent: display(&root), name: "Task".into(),
                source: Some(format!("#metadata((types: (\"task\",), status: \"open\", due: \"2026-10-23T09:30:00Z\", reminder: (id: \"{}\", kind: \"absolute\", at: \"2026-10-23T09:00:00Z\"))) <weftext>\n= Task\n", fresh())),
            },
        }).unwrap();
        let owner = NodeRef {
            workspace_id: library.workspace_id,
            node_id: preview.new_node_id.clone().unwrap(),
        };
        let original_cut = ReminderJournalFence::capture(&core).unwrap();
        assert!(!original_cut.committed_order.contains(&operation));
        core.execute_operation(&operation, &preview.plan_id)
            .unwrap();
        let (candidate, witnesses) = core.capture_reminder_task(&root, &owner).unwrap();
        assert_eq!(candidate.resolution.state, TaskReminderState::Ready);
        let source = witnesses
            .iter()
            .find(|witness| witness.binding.path == candidate.source_path)
            .and_then(|witness| witness.binding.captured.as_ref())
            .unwrap();
        // The real current reader proves this task, but its new commit/Birth
        // was absent from the old cut and cannot be borrowed into that proof.
        assert!(
            original_cut
                .verify_author_history(&core, &[(&candidate, source)])
                .is_err()
        );
        let current_cut = ReminderJournalFence::capture(&core).unwrap();
        assert!(current_cut.committed_order.contains(&operation));
        current_cut
            .verify_author_history(&core, &[(&candidate, source)])
            .unwrap();
    }
}
