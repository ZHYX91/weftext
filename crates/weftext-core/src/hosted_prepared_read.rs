//! Read-only delivery qualification of an original actor Plan's shared settings.
//! No current capture can replace this retained physical dependency.
use super::*;
use crate::backend::namespace_path_compare;
use crate::operations::read_blob;

impl Core {
    /// Recheck only the exact settings read dependency of a prepared actor
    /// feature. The original actor/library record supplies its parent identity;
    /// the same immutable Plan supplies file identity, stamp and bytes. This
    /// returns no snapshot ID, author path or write capability.
    pub fn validate_actor_feature_settings_dependency(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<()> {
        let fail = |message| CoreError::new("staleBase", message);
        let record = self
            .actor_feature_record(resource, operation)?
            .ok_or_else(|| fail(UiMessage::error_core_actor_feature_unavailable()))?;
        let library = record
            .library
            .as_ref()
            .ok_or_else(|| fail(UiMessage::error_core_actor_library_unbound()))?;
        let plan = self
            .operations
            .get(&record.core_operation_id)
            .ok_or_else(|| fail(UiMessage::error_core_actor_feature_not_prepared()))?;
        if record.cancelled
            || self.committed(&plan.operation_id)
            || self.resolved(&plan.operation_id)?
            || self.journal_path(&plan.operation_id, "install").exists()
        {
            return Err(fail(UiMessage::error_core_actor_feature_not_prepared()));
        }
        let transaction = plan
            .transaction
            .as_ref()
            .ok_or_else(|| fail(UiMessage::error_core_actor_feature_plan_unavailable()))?;
        let root = absolute_path(Path::new(&transaction.request.root))?;
        if plan.workspace_id != library.workspace_id {
            return Err(fail(
                UiMessage::error_core_actor_feature_workspace_mismatch(),
            ));
        }
        self.actor_library_for_root(&root)?;
        let workspace = self.require_workspace(&root)?;
        if library.workspace_id.as_ref() != Some(&workspace.workspace_id) {
            return Err(fail(UiMessage::error_core_actor_library_workspace_changed()));
        }
        // This is one closed Core configuration role, never an HTTP-selected
        // filename or a path/exclusion supplied by the caller.
        let path = root.join(".weftext-settings.json");
        let mut original = None;
        for file in &transaction.files {
            if namespace_path_compare(Path::new(&file.path), &path)?.is_eq() {
                if original.is_some() {
                    return Err(fail(UiMessage::error_core_competing_settings_inputs()));
                }
                original = Some(file);
            }
        }
        let original =
            original.ok_or_else(|| fail(UiMessage::error_core_missing_settings_dependency()))?;
        self.check_pending_except(&path, Some(&plan.operation_id))?;
        let guard = ParentGuard::acquire(&path)?;
        let mut file = guard.open_read_guard_file()?;
        if namespace_path_compare(&guard.directory_coordinates()?, Path::new(&library.root))?
            .is_ne()
            || guard.directory_identity()? != library.root_identity
            || stamp(&file)? != original.stamp
            || read_blob(&mut file)? != original.bytes
        {
            return Err(fail(UiMessage::error_core_original_settings_changed()));
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn prepared_settings_dependency_uses_original_actor_plan_object() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let settings = root.join(".weftext-settings.json");
        fs::write(&settings, br#"{"version":1,"processing":{"fixture":1}}"#).unwrap();
        let config = core
            .read_config(&root, ConfigRole::WorkspaceSettings)
            .unwrap();
        assert!(config.revision.is_none());
        let annotation = Path::new(&note.path)
            .parent()
            .unwrap()
            .join("weftext.annotations.json");
        let snapshot = core
            .read_snapshot(&root, &[display(&annotation)])
            .unwrap()
            .files
            .remove(0);
        let operation = fresh();
        let request = OperationRequest {
            operation_id: operation.clone(),
            root: display(&root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: display(&annotation),
                    base_id: Some(snapshot.base_id),
                    before: None,
                    after: br#"{"version":1,"records":[]}"#.to_vec(),
                }],
            },
        };
        core.with_actor_for_root("hosted:alice:review", &root, |core| {
            core.actor_feature_prepare(
                "Note",
                &operation,
                &"a".repeat(64),
                request,
                serde_json::json!({"kind":"annotationProofread"}),
                &[config.base_id],
            )?;
            core.validate_actor_feature_settings_dependency("Note", &operation)
        })
        .unwrap();
        let raw = fs::read(&settings).unwrap();
        fs::rename(&settings, temp.path().join("retained-settings.json")).unwrap();
        fs::write(&settings, &raw).unwrap();
        assert!(
            core.with_actor_for_root("hosted:alice:review", &root, |core| {
                core.validate_actor_feature_settings_dependency("Note", &operation)
            })
            .is_err()
        );
        drop(core);
        let mut core = Core::new(&data).unwrap();
        assert!(
            core.with_actor_for_root("hosted:alice:review", &root, |core| {
                core.validate_actor_feature_settings_dependency("Note", &operation)
            })
            .is_err()
        );
        assert!(
            core.with_actor_for_root("hosted:bob:review", &root, |core| {
                core.validate_actor_feature_settings_dependency("Note", &operation)
            })
            .is_err()
        );
        assert!(!annotation.exists());
        assert_eq!(fs::read(settings).unwrap(), raw);
    }
}
