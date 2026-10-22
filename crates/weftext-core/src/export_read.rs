//! Exact ordinary-file dependency capture for output. No admission is invented.
use super::*;
use std::collections::BTreeMap;
use weftext_messages::UiMessage;

impl Core {
    pub fn ordinary_export_files(
        &self,
        session_id: &str,
        base_id: &str,
        root: &Path,
    ) -> Result<BTreeMap<String, Vec<u8>>> {
        self.backend.check_path(&self.data_dir)?;
        self.check_session_base(session_id, base_id)?;
        let root = absolute_path(root)?;
        let session = self.sessions.get(session_id).ok_or_else(|| {
            CoreError::new(
                "sessionMissing",
                UiMessage::error_core_export_read_open_document_first(),
            )
        })?;
        if !session.path.starts_with(&root) {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_export_read_source_outside_selected_dependency_root(),
            ));
        }
        let mut pending = vec![root.clone()];
        let mut directories = Vec::new();
        let mut files = Vec::new();
        let mut guards = Vec::new();
        let mut total = 0usize;
        let mut count = 0usize;
        while let Some(directory) = pending.pop() {
            let guard =
                ParentGuard::acquire(&self.backend, &directory.join(".weftext-export-scope"))?;
            let (mut entries, complete) = guard.directory_entries(5000)?;
            if !complete {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_export_read_export_dependency_scope_exceeds5000_entries(),
                ));
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, attrs) in &entries {
                count += 1;
                if count > 5000 {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_export_read_export_dependency_scope_exceeds5000_entries(),
                    ));
                }
                if attrs.is_unsupported() {
                    return Err(CoreError::new(
                        "unsupportedPath",
                        UiMessage::error_core_export_read_export_dependencies_cannot_traverse_reparse_points(),
                    ));
                }
                let path = directory.join(name);
                if attrs.is_directory() {
                    pending.push(path);
                    continue;
                }
                self.check_pending(&path)?;
                let leaf = ParentGuard::acquire(&self.backend, &path)?;
                let mut handle = leaf.open_file(false, false)?;
                let stamp = stamp(&handle)?;
                self.check_pending_identity(&stamp.identity)?;
                total = total.checked_add(stamp.size as usize).ok_or_else(|| {
                    CoreError::new(
                        "budget",
                        UiMessage::error_core_export_read_export_input_too_large(),
                    )
                })?;
                if total > 64 * 1024 * 1024 {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_export_read_export_dependency_scope_exceeds64_mi_b_choose_narrower_root(),
                    ));
                }
                let bytes = operations::read_blob(&mut handle)?;
                files.push((path, handle, stamp, bytes));
                guards.push(leaf);
            }
            directories.push((guard, entries));
        }
        for (guard, entries) in &directories {
            let (mut current, complete) = guard.directory_entries(5000)?;
            current.sort_by(|a, b| a.0.cmp(&b.0));
            if !complete || &current != entries {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_export_read_export_input_membership_changed(),
                ));
            }
        }
        let mut captured = BTreeMap::new();
        for (path, mut file, original_stamp, bytes) in files {
            if stamp(&file)? != original_stamp || operations::read_blob(&mut file)? != bytes {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_export_read_export_dependency_changed_during_capture(),
                ));
            }
            let key = path
                .strip_prefix(&root)
                .map_err(|_| {
                    CoreError::new(
                        "outsideRoot",
                        UiMessage::error_core_export_read_export_dependency_escaped_root(),
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            captured.insert(key, bytes);
        }
        Ok(captured)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn ordinary_capture_preserves_dependency_bytes_without_admission() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ordinary");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("assets")).unwrap();
        fs::write(root.join("main.typ"), "#include \"other.typ\"").unwrap();
        fs::write(root.join("other.typ"), "中文依赖").unwrap();
        fs::write(root.join("assets/a.bin"), [0, 255, 7]).unwrap();
        let mut core = Core::new(dir.path().join("state")).unwrap();
        let doc = core
            .open(root.join("main.typ"), Some(root.clone()))
            .unwrap();
        let files = core
            .ordinary_export_files(&doc.session_id, &doc.base_id, &root)
            .unwrap();
        assert_eq!(files["assets/a.bin"], [0, 255, 7]);
        assert_eq!(files["other.typ"], "中文依赖".as_bytes());
        assert!(!root.join(".weftext-workspace.json").exists());
        fs::write(root.join("main.typ"), "External edit").unwrap();
        assert!(
            core.ordinary_export_files(&doc.session_id, &doc.base_id, &root)
                .is_err()
        );
        assert!(
            core.ordinary_export_files(
                &doc.session_id,
                &doc.base_id,
                &dir.path().join("elsewhere")
            )
            .is_err()
        );
    }
}
