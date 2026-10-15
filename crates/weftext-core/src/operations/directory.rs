//! Placement of an admitted non-Node container uses the original Relocate
//! writer. Its identity marker keeps its name and exact content revision.
use super::*;

fn validate_target_name(name: &str) -> Result<()> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.is_empty()
        || name.len() > 240
        || matches!(name, "." | "..")
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || name.to_ascii_lowercase().starts_with(".weftext-")
        || name.eq_ignore_ascii_case(".git")
        || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
    {
        return Err(CoreError::new(
            "invalidName",
            "Use a safe directory basename outside reserved library controls",
        ));
    }
    Ok(())
}

impl Core {
    pub(crate) fn qualified_directory_marker(
        &self,
        root: &Path,
        source: &Path,
    ) -> Result<(CapturedFile, FileIdentity)> {
        if source == root {
            return Err(CoreError::new(
                "invalidTarget",
                "The workspace root cannot be relocated",
            ));
        }
        self.owned_parent(root, source)?;
        self.check_subtree_boundary(root, source)?;
        let target = self.capture_navigation_target(root, source)?;
        if target.kind != "directory" || target.status != "managed" {
            return Err(CoreError::new(
                "identityConflict",
                "Relocation requires the original admitted non-Node directory",
            ));
        }
        let reference = target.target_ref.ok_or_else(|| {
            CoreError::new(
                "identityConflict",
                "The directory has no qualified reference",
            )
        })?;
        let (_, workspace, id) = crate::directory_identity::parse_ref(&reference)?;
        let marker = source.join("weftext.directory.json");
        self.check_pending(&marker)?;
        let captured = capture_optional(&marker)?.ok_or_else(|| {
            CoreError::new(
                "identityConflict",
                "The admitted directory marker is missing",
            )
        })?;
        let identity = ParentGuard::acquire(&marker)?.directory_identity()?;
        if workspace != self.require_workspace(root)?.workspace_id
            || !self.directory_admitted(workspace, id, &identity)?
            || serde_json::from_slice::<serde_json::Value>(&captured.bytes)?["directoryId"] != id
        {
            return Err(CoreError::new(
                "identityConflict",
                "The directory identity binding changed",
            ));
        }
        self.check_pending_identity(&captured.stamp.identity)?;
        Ok((captured, identity))
    }

    pub(crate) fn prepare_directory_relocation(
        &self,
        root: &Path,
        source: &Path,
        target: &Path,
        files: &mut Vec<CapturedFile>,
        preview: &mut OperationPreview,
    ) -> Result<Mutation> {
        self.prepare_directory_relocation_with_target(root, source, target, files, preview, None)
    }

    pub(crate) fn prepare_directory_relocation_with_target(
        &self,
        root: &Path,
        source: &Path,
        target: &Path,
        files: &mut Vec<CapturedFile>,
        preview: &mut OperationPreview,
        occupant: Option<&FileIdentity>,
    ) -> Result<Mutation> {
        validate_target_name(
            target
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    CoreError::new("invalidName", "The target directory needs a UTF-8 basename")
                })?,
        )?;
        let (captured, expected_directory) = self.qualified_directory_marker(root, source)?;
        let workspace = self.require_workspace(root)?;
        self.owned_parent(root, target.parent().unwrap())?;
        let mut namespace = NamespaceGuard::acquire(source, target.parent().unwrap())?;
        if source != target {
            qualify_relocation_target(target, occupant)?;
        }
        let source_identity = namespace.source_identity()?;
        if source_identity != expected_directory {
            return Err(CoreError::new(
                "conflict",
                "The admitted directory changed during planning",
            ));
        }
        let target_parent_identity = namespace.parent_identity()?;
        let leaf = "weftext.directory.json";
        let mut held = namespace.open_source_file(leaf, false)?;
        if stamp(&held)? != captured.stamp || read_blob(&mut held)? != captured.bytes {
            return Err(CoreError::new(
                "conflict",
                "The original directory marker changed during planning",
            ));
        }
        preview
            .changes
            .push(change(source, "relocateFrom", captured.bytes.len()));
        preview
            .changes
            .push(change(target, "relocateTo", captured.bytes.len()));
        let members =
            self.capture_relocation_members(&mut namespace, source, leaf, &workspace.workspace_id)?;
        let leaf_identity = captured.stamp.identity.clone();
        files.push(captured);
        Ok(Mutation::Relocate {
            source: display(source),
            target: display(target),
            source_identity,
            target_parent_identity,
            create_trash_parent: false,
            old_leaf: leaf.into(),
            new_leaf: leaf.into(),
            leaf_identity,
            members,
            updates: vec![],
        })
    }
}

#[cfg(all(test, windows))]
#[path = "directory_tests.rs"]
mod tests;
