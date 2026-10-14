//! Exact ordinary-file dependency capture for output. No admission is invented.
use super::*;
use std::collections::BTreeMap;

impl Core {
    pub fn ordinary_export_files(
        &self,
        session_id: &str,
        base_id: &str,
        root: &Path,
    ) -> Result<BTreeMap<String, Vec<u8>>> {
        self.check_session_base(session_id, base_id)?;
        let root = absolute_path(root)?;
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the document first"))?;
        if !session.path.starts_with(&root) {
            return Err(CoreError::new(
                "outsideRoot",
                "The source is outside its selected dependency root",
            ));
        }
        let mut pending = vec![root.clone()];
        let mut directories = Vec::new();
        let mut files = Vec::new();
        let mut guards = Vec::new();
        let mut total = 0usize;
        let mut count = 0usize;
        while let Some(directory) = pending.pop() {
            let guard = ParentGuard::acquire(&directory.join(".weftext-export-scope"))?;
            let (mut entries, complete) = guard.directory_entries(5000)?;
            if !complete {
                return Err(CoreError::new(
                    "budget",
                    "Export dependency scope exceeds 5,000 entries",
                ));
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, attrs) in &entries {
                count += 1;
                if count > 5000 {
                    return Err(CoreError::new(
                        "budget",
                        "Export dependency scope exceeds 5,000 entries",
                    ));
                }
                if attrs & 0x400 != 0 {
                    return Err(CoreError::new(
                        "unsupportedPath",
                        "Export dependencies cannot traverse reparse points",
                    ));
                }
                let path = directory.join(name);
                if attrs & 0x10 != 0 {
                    pending.push(path);
                    continue;
                }
                self.check_pending(&path)?;
                let leaf = ParentGuard::acquire(&path)?;
                let mut handle = leaf.open_file(false, false)?;
                let stamp = stamp(&handle)?;
                self.check_pending_identity(&stamp.identity)?;
                total = total
                    .checked_add(stamp.size as usize)
                    .ok_or_else(|| CoreError::new("budget", "Export input too large"))?;
                if total > 64 * 1024 * 1024 {
                    return Err(CoreError::new(
                        "budget",
                        "Export dependency scope exceeds 64 MiB; choose a narrower root",
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
                    "Export input membership changed",
                ));
            }
        }
        let mut captured = BTreeMap::new();
        for (path, mut file, original_stamp, bytes) in files {
            if stamp(&file)? != original_stamp || operations::read_blob(&mut file)? != bytes {
                return Err(CoreError::new(
                    "scopeChanged",
                    "Export dependency changed during capture",
                ));
            }
            let key = path
                .strip_prefix(&root)
                .map_err(|_| CoreError::new("outsideRoot", "Export dependency escaped its root"))?
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
