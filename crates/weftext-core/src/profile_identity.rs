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
}

pub fn qualify_profile(path: impl AsRef<Path>) -> Result<QualifiedProfile> {
    let path = absolute_path(path.as_ref())?;
    let guard = qualify_directory(&path)?;
    let identity = format!(
        "weftext.profile.ntfs.v1:{}",
        hash(&serde_json::to_vec(&guard.directory_identity()?)?)
    );
    Ok(QualifiedProfile {
        path,
        identity,
        _guard: guard,
    })
}

fn qualify_directory(path: &Path) -> Result<ParentGuard> {
    let prospective = path.join(".profile-prospective");
    match ParentGuard::acquire(&prospective) {
        Ok(guard) => Ok(guard),
        Err(error) if error.code == "missing" => {
            let parent = path.parent().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_profile_identity_profile_has_no_qualified_parent(),
                )
            })?;
            let _parent_guard = qualify_directory(parent)?;
            let leaf = ParentGuard::acquire(path)?;
            match leaf.create_directory() {
                Ok(directory) => {
                    let guard = ParentGuard::acquire(&prospective)?;
                    drop(directory);
                    Ok(guard)
                }
                // Another first-start process may have created this exact
                // directory. Qualify it; do not recreate, remove or overwrite.
                Err(error) if error.code == "exists" => ParentGuard::acquire(&prospective),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}
