//! Small shared administrator enrollment authority. It does not depend on Core,
//! the rendering engine, a worker launcher, or an application error catalog.
use serde::{Deserialize, Serialize};

/// Native, platform-tagged physical identity. Linux uses the filesystem's
/// qualified opaque handle, never device/inode/ctime coordinates.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(tag = "platform", rename_all = "camelCase", deny_unknown_fields)]
pub enum PhysicalIdentity {
    Windows {
        volume: u32,
        index: u64,
        created: u64,
    },
    Linux {
        filesystem: [i32; 2],
        handle_type: i32,
        handle: Vec<u8>,
    },
}
impl PhysicalIdentity {
    #[cfg(windows)]
    pub fn windows_volume(&self) -> u32 {
        match self {
            Self::Windows { volume, .. } => *volume,
            _ => unreachable!("Windows handle identity"),
        }
    }
}

/// Closed machine reasons. Consumers choose their typed, localized summary;
/// no authored prose is presented as an operating-system diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnrollmentRejection {
    AbsoluteAdministratorPathRequired,
    PathTraversal,
    InvalidName,
    UntrustedAdministratorPath,
    FreshRootBirthRequired,
    DistinctIdentitiesRequired,
    ServiceAccountRequired,
    WorkerAccountRequired,
    ExclusiveServiceGroupRequired,
    EnrollmentIdentityRequired,
    MachineMismatch,
    KeyPermissions,
    ExtendedKeyAcl,
    KeyInsideRoot,
    RootPath,
    RootIdentity,
    RootPermissions,
    AuthorityChanged,
    EnrollmentChanged,
    ServiceCredentials,
    UnsupportedFilesystem,
    OpaqueHandleSize,
}

#[derive(Debug)]
pub enum EnrollmentError {
    Io(std::io::Error),
    Format(serde_json::Error),
    Rejected(EnrollmentRejection),
}
impl From<std::io::Error> for EnrollmentError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for EnrollmentError {
    fn from(value: serde_json::Error) -> Self {
        Self::Format(value)
    }
}
pub type Result<T> = std::result::Result<T, EnrollmentError>;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{
    ValidatedEnrollment, filesystem_id, inspect_enrollment, opaque_identity,
    open_administrator_directory, open_administrator_file,
};

#[cfg(test)]
mod tests;
