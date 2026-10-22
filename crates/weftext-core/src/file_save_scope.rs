//! Original file-save ownership is part of its frozen Plan, not a later locator.
use super::*;
use hosted::ActorLibraryBinding;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum QualifiedInputScope {
    DeviceUnassigned,
    Library { binding: QualifiedLibrary },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QualifiedLibrary {
    pub root: String,
    pub root_identity: FileIdentity,
    pub workspace_id: String,
}
impl QualifiedLibrary {
    pub(crate) fn actor_binding(&self) -> ActorLibraryBinding {
        ActorLibraryBinding {
            root: self.root.clone(),
            root_identity: self.root_identity.clone(),
            workspace_id: Some(self.workspace_id.clone()),
        }
    }
    pub(crate) fn validate(&self) -> Result<()> {
        valid_operation_id(&self.workspace_id)?;
        if serde_json::to_vec(self)?.len() > 1536 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_file_save_scope_qualified_metadata_budget_exceeded(),
            ));
        }
        if !Path::new(&self.root).is_absolute() {
            return Err(CoreError::new(
                "privateEvidence",
                UiMessage::error_core_file_save_scope_original_library_root_not_absolute(),
            ));
        }
        Ok(())
    }
}
impl QualifiedInputScope {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Self::Library { binding } = self {
            binding.validate()?;
        }
        Ok(())
    }
}
pub(crate) fn read_input_scope(
    file: &mut (impl Read + std::io::Seek),
) -> Result<QualifiedInputScope> {
    let value = hosted::read_first_metadata(file, "scope")?.ok_or_else(|| {
        CoreError::new(
            "unsupportedInputScope",
            UiMessage::error_core_file_save_scope_retained_input_scope_unsupported(),
        )
    })?;
    let scope: QualifiedInputScope = serde_json::from_slice(&value)?;
    scope.validate()?;
    Ok(scope)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FileSaveData {
    pub version: u8,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_id: Option<String>,
    pub scope: QualifiedInputScope,
}
impl FileSaveData {
    pub(crate) fn new(
        session_id: String,
        base_id: Option<String>,
        scope: QualifiedInputScope,
    ) -> Self {
        Self {
            version: 1,
            session_id,
            base_id,
            scope,
        }
    }
}
pub(crate) fn unsupported_scope() -> CoreError {
    CoreError::new(
        "unsupportedFileSaveEvidence",
        UiMessage::error_core_file_save_scope_retained_file_save_scope_unsupported(),
    )
}
pub(crate) fn fingerprint(
    kind: &str,
    path: &Path,
    source: &str,
    data: &FileSaveData,
    restore: Option<&history::RestoreBinding>,
) -> Result<String> {
    let mut intent =
        serde_json::json!({"kind":kind,"path":display(path),"source":source,"fileSave":data});
    if let Some(binding) = restore {
        intent["historyRestore"] = serde_json::to_value(binding)?;
    }
    Ok(hash(&serde_json::to_vec(&intent)?))
}
fn same_locator(left: &str, right: &str) -> bool {
    left.replace('\\', "/")
        .eq_ignore_ascii_case(&right.replace('\\', "/"))
}
impl Core {
    /// Validate all original ownership coordinates from the same admission;
    /// a later namespace or path reuse cannot qualify damaged metadata.
    pub(crate) fn validate_original_input_scope(&self, scope: &QualifiedInputScope) -> Result<()> {
        scope.validate()?;
        if let QualifiedInputScope::Library { binding } = scope {
            let original = self.validate_original_reservation_library(&binding.actor_binding())?;
            if !same_locator(&binding.root, &original) {
                return Err(CoreError::new(
                    "privateEvidence",
                    UiMessage::error_core_file_save_scope_original_locator_contradicts_admission(),
                ));
            }
        }
        Ok(())
    }
    /// The nearest qualified library is a discovery boundary. Observe real
    /// ancestors even when an ordinary open selected only its immediate parent.
    pub(crate) fn capture_file_save_scope(&self, path: &Path) -> Result<QualifiedInputScope> {
        let parent = path.parent().ok_or_else(|| {
            CoreError::new(
                "invalidPath",
                UiMessage::error_core_file_save_scope_file_parent_missing(),
            )
        })?;
        let _parent_guard = ParentGuard::acquire(&self.backend, path)?;
        for candidate in parent.ancestors() {
            if let Some(workspace) = self.workspace(candidate)? {
                let guard = ParentGuard::acquire(
                    &self.backend,
                    &candidate.join(".file-scope-prospective"),
                )?;
                let root_identity = guard.directory_identity()?;
                let original_root =
                    self.validate_original_reservation_library(&ActorLibraryBinding {
                        root: display(candidate),
                        root_identity: root_identity.clone(),
                        workspace_id: Some(workspace.workspace_id.clone()),
                    })?;
                return Ok(QualifiedInputScope::Library {
                    binding: QualifiedLibrary {
                        root: original_root,
                        root_identity,
                        workspace_id: workspace.workspace_id,
                    },
                });
            }
            for marker in [
                ".weftext-workspace.json",
                ".weftext-format",
                ".weftext-mvp-commit.json",
            ] {
                if self.backend.entry_exists(candidate.join(marker))? {
                    return Err(CoreError::new(
                        "unadmittedLibrary",
                        UiMessage::error_core_file_save_scope_ancestor_owner_unproven(),
                    ));
                }
            }
        }
        Ok(QualifiedInputScope::DeviceUnassigned)
    }
    pub(crate) fn validate_current_file_save_scope(
        &self,
        scope: &QualifiedInputScope,
        path: &Path,
    ) -> Result<()> {
        if self.capture_file_save_scope(path)? != *scope {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_core_file_save_scope_physical_owner_changed(),
            ));
        }
        Ok(())
    }
    pub(crate) fn validate_original_file_save_scope(
        &self,
        scope: &QualifiedInputScope,
        path: &Path,
    ) -> Result<()> {
        self.validate_original_input_scope(scope)?;
        match scope {
            QualifiedInputScope::Library { binding } => {
                if history::history_relative_path(path, Path::new(&binding.root)).is_none() {
                    return Err(CoreError::new(
                        "invalidState",
                        UiMessage::error_core_file_save_scope_target_contradicts_scope(),
                    ));
                }
            }
            QualifiedInputScope::DeviceUnassigned => {}
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn ordinary_nested_open_and_save_as_freeze_the_actual_target_owner_not_ui_root_or_source() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let a = temp.path().join("A");
        let b = temp.path().join("B");
        core.create_library(&a).unwrap();
        core.create_library(&b).unwrap();
        let nested = a.join("nested/deeper");
        fs::create_dir_all(&nested).unwrap();
        let path = nested.join("ordinary.typ");
        fs::write(&path, "ordinary input").unwrap();
        let opened = core.open(&path, None).unwrap();
        assert_eq!(opened.kind, "ordinary");
        assert_eq!(opened.root, display(&nested));
        let original = core.sessions[&opened.session_id]
            .file_scope
            .clone()
            .unwrap();
        assert!(
            matches!(&original, QualifiedInputScope::Library { binding } if binding.root == display(&a) && binding.workspace_id == core.require_workspace(&a).unwrap().workspace_id)
        );
        let target = b.join("output.typ");
        let id = fresh();
        core.save_as(SaveAsRequest {
            session_id: opened.session_id.clone(),
            path: display(&target),
            source: "B original result".into(),
            operation_id: id.clone(),
        })
        .unwrap();
        let plan = &core.operations[&id];
        let data = plan.file_save.as_ref().unwrap();
        assert!(
            matches!(&data.scope, QualifiedInputScope::Library { binding } if binding.root == display(&b) && binding.workspace_id == core.require_workspace(&b).unwrap().workspace_id)
        );
        core.validate_file_save_plan(plan).unwrap();
        assert_eq!(
            core.sessions[&opened.session_id]
                .file_scope
                .as_ref()
                .unwrap(),
            &original
        );
        assert!(core.committed(&id).unwrap());
        let old_plan = fs::read(core.journal_path(&id, "plan")).unwrap();
        core.sessions.clear();
        let (_, delivered) = core.reopen_local_save(&id).unwrap();
        assert_eq!(delivered.source, "B original result");
        assert_eq!(fs::read(core.journal_path(&id, "plan")).unwrap(), old_plan);
    }
    #[test]
    fn physical_library_aliases_share_original_scope_and_one_current_draft_key() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let path = root.join("ordinary.typ");
        fs::write(&path, "B").unwrap();
        let first = core.open(&path, None).unwrap();
        core.persist_draft(&first.session_id, "original input Y")
            .unwrap();
        let scope = core.sessions[&first.session_id].file_scope.clone().unwrap();
        let primary = core.draft_path(&path);
        let bytes = fs::read(&primary).unwrap();
        let alias = PathBuf::from(display(&path).replace('\\', "/").to_uppercase());
        let second = core.open(&alias, None).unwrap();
        assert_eq!(
            core.sessions[&second.session_id]
                .file_scope
                .as_ref()
                .unwrap(),
            &scope
        );
        assert_eq!(second.recovered_draft.as_deref(), Some("original input Y"));
        assert_eq!(core.draft_path(&alias), primary);
        assert_eq!(fs::read(&primary).unwrap(), bytes);
        let file_count = fs::read_dir(core.draft_directory("drafts"))
            .unwrap()
            .count();
        core.persist_draft(&second.session_id, "next input")
            .unwrap();
        assert_eq!(
            fs::read_dir(core.draft_directory("drafts"))
                .unwrap()
                .count(),
            file_count
        );
        assert_eq!(
            core.open(&path, None).unwrap().recovered_draft.as_deref(),
            Some("next input")
        );
    }
    #[test]
    fn original_participant_fingerprint_rejects_scope_tampering_and_old_missing_schema_is_not_current()
     {
        for variant in ["scope", "version", "missing"] {
            let temp = tempfile::tempdir().unwrap();
            let data = temp.path().join("private");
            let path = temp.path().join("ordinary.typ");
            fs::write(&path, "B").unwrap();
            let mut core = Core::new(&data).unwrap();
            let doc = core.open(&path, None).unwrap();
            let id = fresh();
            core.save(SaveRequest {
                session_id: doc.session_id,
                base_id: doc.base_id,
                source: "S".into(),
                operation_id: id.clone(),
            })
            .unwrap();
            let plan_path = core.journal_path(&id, "plan");
            let commit_path = core.journal_path(&id, "commit");
            let original_commit = fs::read(&commit_path).unwrap();
            let mut changed = core.operations[&id].clone();
            match variant {
                "scope" => changed.file_save.as_mut().unwrap().session_id = fresh(),
                "version" => changed.file_save.as_mut().unwrap().version = 2,
                _ => changed.file_save = None,
            }
            fs::write(&plan_path, serde_json::to_vec(&changed).unwrap()).unwrap();
            core.operations.insert(id.clone(), changed);
            let error = core
                .validate_file_save_plan(&core.operations[&id])
                .unwrap_err();
            assert_eq!(
                error.code,
                if variant == "scope" {
                    "invalidState"
                } else {
                    "unsupportedFileSaveEvidence"
                }
            );
            assert!(core.local_operation_status(&id).is_err());
            assert!(core.reopen_local_save(&id).is_err());
            assert!(
                core.saved_outcome_for_request(&SaveRequest {
                    session_id: fresh(),
                    base_id: fresh(),
                    source: "S".into(),
                    operation_id: id.clone()
                })
                .is_err()
            );
            assert_eq!(fs::read(commit_path).unwrap(), original_commit);
            assert_eq!(fs::read_to_string(path).unwrap(), "S");
        }
    }
    #[test]
    fn damaged_or_unadmitted_ancestor_is_never_device_unassigned() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let path = root.join("ordinary.typ");
        fs::write(&path, "readable input").unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let device = core.open(&path, None).unwrap();
        assert_eq!(
            core.sessions[&device.session_id]
                .file_scope
                .as_ref()
                .unwrap(),
            &QualifiedInputScope::DeviceUnassigned
        );
        fs::write(root.join(".weftext-workspace.json"), "damaged controls").unwrap();
        let readable = core.open(&path, None).unwrap();
        assert_eq!(readable.source, "readable input");
        let scope_error = core.sessions[&readable.session_id]
            .file_scope
            .as_ref()
            .unwrap_err();
        assert_eq!(scope_error.code, "unadmittedLibrary");
        assert_eq!(
            scope_error.explanation,
            UiMessage::error_core_file_save_scope_ancestor_owner_unproven()
        );
        assert!(scope_error.diagnostics.is_empty());
        assert_eq!(
            core.persist_draft(&readable.session_id, "Y")
                .unwrap_err()
                .code,
            "unadmittedLibrary"
        );
        assert!(core.reopen_local_session(&device.session_id).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "readable input");
    }
}
