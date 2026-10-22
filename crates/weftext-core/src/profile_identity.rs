//! Physical profile qualification before native owner/broker discovery. This
//! holds no Core lease and opens no journal or author payload.
use crate::{CoreError, ParentGuard, Result, absolute_path, hash};
use std::path::{Path, PathBuf};
use weftext_messages::UiMessage;

pub struct QualifiedProfile {
    path: PathBuf,
    identity: String,
    _guard: ParentGuard,
}

impl QualifiedProfile {
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Stable for the same physical directory, including rename; differs when
    /// a path is replaced. It is neither an authentication key nor a device ID.
    pub fn physical_identity(&self) -> &str {
        &self.identity
    }

    /// Compare directory handles, including ancestors, rather than Windows
    /// pathname spelling/case. Both qualifiers keep their parent chains held.
    pub fn contains_profile(&self, other: &QualifiedProfile) -> Result<bool> {
        let identity = self._guard.directory_identity()?;
        for ancestor in other.path.ancestors() {
            let backend = crate::BackendContext::local()?;
            let guard = ParentGuard::acquire(&backend, &ancestor.join(".profile-prospective"))?;
            if guard.directory_identity()? == identity {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub fn qualify_profile(path: impl AsRef<Path>) -> Result<QualifiedProfile> {
    let path = absolute_path(path.as_ref())?;
    let backend = crate::BackendContext::local()?;
    let guard = qualify_directory(&backend, &path)?;
    qualified(path, guard)
}

/// Existing-owner control must not create a new profile while locating the
/// original. A missing/replaced directory is a rejection, never a fresh owner.
pub fn qualify_existing_profile(path: impl AsRef<Path>) -> Result<QualifiedProfile> {
    let path = absolute_path(path.as_ref())?;
    let backend = crate::BackendContext::local()?;
    let guard = ParentGuard::acquire(&backend, &path.join(".profile-prospective"))?;
    qualified(path, guard)
}

fn qualified(path: PathBuf, guard: ParentGuard) -> Result<QualifiedProfile> {
    let identity = format!(
        "weftext.profile.physical.v2:{}",
        hash(&serde_json::to_vec(&guard.directory_identity()?)?)
    );
    Ok(QualifiedProfile {
        path,
        identity,
        _guard: guard,
    })
}

fn qualify_directory(backend: &crate::BackendContext, path: &Path) -> Result<ParentGuard> {
    let prospective = path.join(".profile-prospective");
    match ParentGuard::acquire(backend, &prospective) {
        Ok(guard) => Ok(guard),
        Err(error) if error.code == "missing" => {
            let parent = path.parent().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_profile_identity_profile_has_no_qualified_parent(),
                )
            })?;
            let _parent_guard = qualify_directory(backend, parent)?;
            let leaf = ParentGuard::acquire(backend, path)?;
            match leaf.create_directory() {
                Ok(directory) => {
                    let guard = ParentGuard::acquire(backend, &prospective)?;
                    drop(directory);
                    Ok(guard)
                }
                // Another first-start process may have created this exact
                // directory. Qualify it; do not recreate, remove or overwrite.
                Err(error) if error.code == "exists" => ParentGuard::acquire(backend, &prospective),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn profile_ancestry_uses_actual_directory_identity_and_existing_control_never_creates() {
        let temp = tempfile::tempdir().unwrap();
        let root = qualify_profile(temp.path().join("CoreState")).unwrap();
        let child = qualify_profile(root.path().join("child")).unwrap();
        let sibling = qualify_profile(temp.path().join("cache")).unwrap();
        assert!(root.contains_profile(&root).unwrap());
        assert!(root.contains_profile(&child).unwrap());
        assert!(!child.contains_profile(&root).unwrap());
        assert!(!sibling.contains_profile(&root).unwrap());
        let spelling = PathBuf::from(root.path().to_str().unwrap().to_uppercase());
        let alias = qualify_existing_profile(spelling).unwrap();
        assert_eq!(alias.physical_identity(), root.physical_identity());
        assert!(alias.contains_profile(&child).unwrap());
        let missing = temp.path().join("not-created");
        assert!(qualify_existing_profile(&missing).is_err());
        assert!(!missing.exists());
    }
}
