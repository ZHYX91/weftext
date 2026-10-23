use crate::{EnrollmentError, EnrollmentRejection as Reject, PhysicalIdentity, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

fn rejected(reason: Reject) -> EnrollmentError {
    EnrollmentError::Rejected(reason)
}
fn os_error() -> EnrollmentError {
    std::io::Error::last_os_error().into()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Enrollment {
    version: u32,
    root_birth: String,
    root: PathBuf,
    identity: PhysicalIdentity,
    service_uid: u32,
    client_uid: u32,
    worker_uid: u32,
    enrollment_id: String,
    machine_id: String,
    key_file: PathBuf,
}

/// Inspected administrator authority. Fields are private so an untrusted caller
/// cannot construct an admitted identity from path/UID assertions.
pub struct ValidatedEnrollment {
    record: Enrollment,
    path: PathBuf,
    enrollment_sha256: String,
    service_gid: u32,
    worker_gid: u32,
    root_directory: File,
    root_parent: File,
    root_parent_identity: PhysicalIdentity,
    key_file: File,
}
impl ValidatedEnrollment {
    pub fn enrollment_path(&self) -> &Path {
        &self.path
    }
    pub fn enrollment_sha256(&self) -> &str {
        &self.enrollment_sha256
    }
    pub fn enrollment_id(&self) -> &str {
        &self.record.enrollment_id
    }
    pub fn machine_id(&self) -> &str {
        &self.record.machine_id
    }
    pub fn root(&self) -> &Path {
        &self.record.root
    }
    pub fn root_identity(&self) -> &PhysicalIdentity {
        &self.record.identity
    }
    pub fn service_uid(&self) -> u32 {
        self.record.service_uid
    }
    pub fn service_gid(&self) -> u32 {
        self.service_gid
    }
    pub fn worker_uid(&self) -> u32 {
        self.record.worker_uid
    }
    pub fn worker_gid(&self) -> u32 {
        self.worker_gid
    }
    pub fn client_uid(&self) -> u32 {
        self.record.client_uid
    }

    /// A read-only retained root descriptor; this does not take the Core writer
    /// lock and does not authorize the process to act as the service.
    pub fn root_directory(&self) -> &File {
        &self.root_directory
    }

    /// Revalidates configured coordinates against original physical anchors.
    /// A retained FD alone would survive a privileged rename or ancestor swap.
    pub fn check_current(&self) -> Result<()> {
        let bytes = read_administrator_file(&self.path)?;
        if digest(&bytes) != self.enrollment_sha256 {
            return Err(rejected(Reject::EnrollmentChanged));
        }
        let parent_path = self
            .record
            .root
            .parent()
            .ok_or_else(|| rejected(Reject::RootPath))?;
        let current_parent = open_administrator_directory(parent_path)?;
        if opaque_identity(&current_parent)? != self.root_parent_identity
            || opaque_identity(&self.root_parent)? != self.root_parent_identity
        {
            return Err(rejected(Reject::AuthorityChanged));
        }
        let current_root = open_root(
            &current_parent,
            self.record
                .root
                .file_name()
                .ok_or_else(|| rejected(Reject::RootPath))?,
        )?;
        verify_root(&current_root, &self.record)?;
        if opaque_identity(&self.root_directory)? != self.record.identity {
            return Err(rejected(Reject::AuthorityChanged));
        }
        let current_key = open_administrator_file(&self.record.key_file, 32)?;
        verify_key(&current_key, self.service_gid)?;
        // The key's bytes remain in its original retained descriptor. Changes
        // by an administrator require service restart and never key adoption.
        verify_key(&self.key_file, self.service_gid)?;
        Ok(())
    }

    pub fn verify_service_process(&self) -> Result<()> {
        let mut actual = [0; 3];
        let pointer = actual.as_mut_ptr();
        if unsafe { libc::getresuid(pointer, pointer.add(1), pointer.add(2)) } < 0 {
            return Err(os_error());
        }
        let mut groups = [0; 3];
        let pointer = groups.as_mut_ptr();
        if unsafe { libc::getresgid(pointer, pointer.add(1), pointer.add(2)) } < 0 {
            return Err(os_error());
        }
        let status = std::fs::read_to_string("/proc/self/status")?;
        let caps_clear = ["CapInh:", "CapPrm:", "CapEff:", "CapAmb:"]
            .iter()
            .all(|key| {
                status
                    .lines()
                    .find_map(|line| line.strip_prefix(key))
                    .and_then(|value| u64::from_str_radix(value.trim(), 16).ok())
                    == Some(0)
            });
        let supplementary = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
        if supplementary < 0 {
            return Err(os_error());
        }
        let bounding_empty = status
            .lines()
            .find_map(|line| line.strip_prefix("CapBnd:"))
            .and_then(|value| u64::from_str_radix(value.trim(), 16).ok())
            == Some(0);
        let no_privilege_gain = status
            .lines()
            .find_map(|line| line.strip_prefix("NoNewPrivs:"))
            .is_some_and(|value| value.trim() == "1");
        if !bounding_empty || !no_privilege_gain {
            return Err(rejected(Reject::PrivilegeGainAllowed));
        }
        if actual != [self.service_uid(); 3]
            || groups != [self.service_gid; 3]
            || !caps_clear
            || supplementary != 0
        {
            return Err(rejected(Reject::ServiceCredentials));
        }
        Ok(())
    }

    /// Only the actual enrolled unprivileged service may obtain key material.
    /// Root-side policy inspection never calls this and receives no secret.
    pub fn read_service_key(&self) -> Result<zeroize::Zeroizing<Vec<u8>>> {
        self.verify_service_process()?;
        self.check_current()?;
        let file = self.key_file.try_clone()?;
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        // pread avoids sharing offsets across repeated admissions/inspections.
        use std::os::unix::fs::FileExt;
        let mut buffer = zeroize::Zeroizing::new([0u8; 32]);
        file.read_exact_at(&mut *buffer, 0)?;
        bytes.extend_from_slice(&*buffer);
        Ok(bytes)
    }
}

/// Validates metadata and retains real descriptor anchors. This function can
/// run as root for broker policy admission, or as the already separated service.
/// It does not read authentication-key content or acquire the Core writer lock.
pub fn inspect_enrollment(path: &Path) -> Result<ValidatedEnrollment> {
    let bytes = read_administrator_file(path)?;
    let record: Enrollment = serde_json::from_slice(&bytes)?;
    if record.version != 1 || record.root_birth != "administrator-created-empty-v1" {
        return Err(rejected(Reject::FreshRootBirthRequired));
    }
    if record.service_uid == 0
        || record.client_uid == 0
        || record.worker_uid == 0
        || record.service_uid == record.client_uid
        || record.service_uid == record.worker_uid
        || record.client_uid == record.worker_uid
    {
        return Err(rejected(Reject::DistinctIdentitiesRequired));
    }
    let (service_gid, worker_gid) = accounts(&record)?;
    let id = uuid::Uuid::parse_str(&record.enrollment_id)
        .map_err(|_| rejected(Reject::EnrollmentIdentityRequired))?;
    if id.to_string() != record.enrollment_id || id.get_version_num() != 4 {
        return Err(rejected(Reject::EnrollmentIdentityRequired));
    }
    let machine = read_administrator_file(Path::new("/etc/machine-id"))?;
    if digest(&machine) != record.machine_id {
        return Err(rejected(Reject::MachineMismatch));
    }
    if !record.root.is_absolute()
        || record
            .root
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(rejected(Reject::RootPath));
    }
    if record.key_file.starts_with(&record.root) {
        return Err(rejected(Reject::KeyInsideRoot));
    }
    let key_file = open_administrator_file(&record.key_file, 32)?;
    verify_key(&key_file, service_gid)?;
    let parent = record
        .root
        .parent()
        .ok_or_else(|| rejected(Reject::RootPath))?;
    let root_parent = open_administrator_directory(parent)?;
    let root_parent_identity = opaque_identity(&root_parent)?;
    let root_directory = open_root(
        &root_parent,
        record
            .root
            .file_name()
            .ok_or_else(|| rejected(Reject::RootPath))?,
    )?;
    verify_root(&root_directory, &record)?;
    Ok(ValidatedEnrollment {
        record,
        path: path.to_path_buf(),
        enrollment_sha256: digest(&bytes),
        service_gid,
        worker_gid,
        root_directory,
        root_parent,
        root_parent_identity,
        key_file,
    })
}

fn accounts(record: &Enrollment) -> Result<(u32, u32)> {
    let text = std::fs::read_to_string("/etc/passwd")?;
    let accounts = text
        .lines()
        .map(|line| line.split(':').collect::<Vec<_>>())
        .filter(|parts| parts.len() == 7)
        .collect::<Vec<_>>();
    let account = |uid| {
        let found = accounts
            .iter()
            .filter(|parts| parts[2].parse::<u32>().ok() == Some(uid))
            .collect::<Vec<_>>();
        if found.len() == 1 {
            Some(found[0].as_slice())
        } else {
            None
        }
    };
    let service =
        account(record.service_uid).ok_or_else(|| rejected(Reject::ServiceAccountRequired))?;
    let worker =
        account(record.worker_uid).ok_or_else(|| rejected(Reject::WorkerAccountRequired))?;
    let service_gid = service[3]
        .parse::<u32>()
        .map_err(|_| rejected(Reject::ServiceAccountRequired))?;
    let worker_gid = worker[3]
        .parse::<u32>()
        .map_err(|_| rejected(Reject::WorkerAccountRequired))?;
    if service_gid == 0
        || !matches!(
            service[6],
            "/usr/sbin/nologin" | "/sbin/nologin" | "/bin/false"
        )
    {
        return Err(rejected(Reject::ServiceAccountRequired));
    }
    if worker_gid == 0
        || !matches!(
            worker[6],
            "/usr/sbin/nologin" | "/sbin/nologin" | "/bin/false"
        )
    {
        return Err(rejected(Reject::WorkerAccountRequired));
    }
    if account(record.client_uid).is_none() {
        return Err(rejected(Reject::DistinctIdentitiesRequired));
    }
    let exclusive_primary = accounts.iter().all(|parts| {
        parts[3].parse::<u32>().ok() != Some(service_gid)
            || parts[2].parse::<u32>().ok() == Some(record.service_uid)
    });
    let groups = std::fs::read_to_string("/etc/group")?;
    let exclusive_membership = groups.lines().all(|line| {
        let parts = line.split(':').collect::<Vec<_>>();
        parts.len() != 4
            || parts[2].parse::<u32>().ok() != Some(service_gid)
            || parts[3]
                .split(',')
                .all(|name| name.is_empty() || name == service[0])
    });
    if !exclusive_primary || !exclusive_membership {
        return Err(rejected(Reject::ExclusiveServiceGroupRequired));
    }
    Ok((service_gid, worker_gid))
}

fn verify_root(directory: &File, record: &Enrollment) -> Result<()> {
    let meta = directory.metadata()?;
    if !meta.is_dir() || meta.uid() != record.service_uid || meta.mode() & 0o7777 != 0o700 {
        return Err(rejected(Reject::RootPermissions));
    }
    if opaque_identity(directory)? != record.identity {
        return Err(rejected(Reject::RootIdentity));
    }
    Ok(())
}
fn verify_key(file: &File, group: u32) -> Result<()> {
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.uid() != 0
        || meta.mode() & 0o7777 != 0o440
        || meta.gid() != group
        || meta.len() != 32
        || meta.nlink() != 1
    {
        return Err(rejected(Reject::KeyPermissions));
    }
    for attribute in [c"system.posix_acl_access", c"system.posix_acl_default"] {
        let value = unsafe {
            libc::fgetxattr(
                file.as_raw_fd(),
                attribute.as_ptr(),
                std::ptr::null_mut(),
                0,
            )
        };
        if value >= 0 {
            return Err(rejected(Reject::ExtendedKeyAcl));
        }
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::ENODATA) {
            return Err(os_error());
        }
    }
    Ok(())
}

fn read_administrator_file(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    open_administrator_file(path, 4096)?
        .take(4097)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(rejected(Reject::UntrustedAdministratorPath));
    }
    Ok(bytes)
}
/// Returns a retained read-only file after no-symlink traversal and owner/mode/
/// single-link/length checks. Ancestors are validated at the open boundary;
/// callers needing continuing coordinates must reopen/revalidate them.
pub fn open_administrator_file(path: &Path, maximum_bytes: u64) -> Result<File> {
    administrator_path(path, Some(maximum_bytes))
}
pub fn open_administrator_directory(path: &Path) -> Result<File> {
    administrator_path(path, None)
}
fn administrator_path(path: &Path, file_limit: Option<u64>) -> Result<File> {
    if !path.is_absolute() {
        return Err(rejected(Reject::AbsoluteAdministratorPathRequired));
    }
    let mut directory = File::open("/")?;
    let mut components = path.components().skip(1).peekable();
    if components.peek().is_none() && file_limit.is_some() {
        return Err(rejected(Reject::UntrustedAdministratorPath));
    }
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            return Err(rejected(Reject::PathTraversal));
        };
        let last = components.peek().is_none();
        let name = exact_name(name)?;
        if last && let Some(limit) = file_limit {
            // Inspect without opening a FIFO/device for data access. Only a
            // trusted administrator can change this root-owned directory entry
            // between the probe and the no-follow read-only open below.
            let probe = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if probe < 0 {
                return Err(os_error());
            }
            let probe = unsafe { File::from_raw_fd(probe) };
            let meta = probe.metadata()?;
            if !meta.is_file()
                || meta.uid() != 0
                || meta.mode() & 0o022 != 0
                || meta.nlink() != 1
                || meta.len() > limit
            {
                return Err(rejected(Reject::UntrustedAdministratorPath));
            }
        }
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_CLOEXEC
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_NOCTTY
                    | libc::O_RDONLY
                    | if last && file_limit.is_some() {
                        0
                    } else {
                        libc::O_DIRECTORY
                    },
            )
        };
        if fd < 0 {
            return Err(os_error());
        }
        let opened = unsafe { File::from_raw_fd(fd) };
        let meta = opened.metadata()?;
        if meta.uid() != 0
            || meta.mode() & 0o022 != 0
            || (last
                && file_limit.is_some_and(|limit| {
                    !meta.is_file() || meta.nlink() != 1 || meta.len() > limit
                }))
            || ((!last || file_limit.is_none()) && !meta.is_dir())
        {
            return Err(rejected(Reject::UntrustedAdministratorPath));
        }
        directory = opened;
    }
    let meta = directory.metadata()?;
    if meta.uid() != 0 || meta.mode() & 0o022 != 0 {
        return Err(rejected(Reject::UntrustedAdministratorPath));
    }
    Ok(directory)
}
fn exact_name(name: &OsStr) -> Result<CString> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.to_str().is_none()
        || name.as_bytes().contains(&b'/')
    {
        return Err(rejected(Reject::InvalidName));
    }
    CString::new(name.as_bytes()).map_err(|_| rejected(Reject::InvalidName))
}
#[repr(C)]
struct OpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}
fn open_root(parent: &File, name: &OsStr) -> Result<File> {
    let name = exact_name(name)?;
    let how = OpenHow {
        flags: (libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW) as u64,
        mode: 0,
        resolve: 0x02 | 0x04 | 0x08,
    };
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            parent.as_raw_fd(),
            name.as_ptr(),
            &how,
            std::mem::size_of::<OpenHow>(),
        )
    };
    if fd < 0 {
        return Err(os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}
pub fn filesystem_id(file: &File) -> Result<[i32; 2]> {
    let mut value = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(file.as_raw_fd(), value.as_mut_ptr()) } < 0 {
        return Err(os_error());
    }
    let value = unsafe { value.assume_init() };
    if !matches!(value.f_type, 0xef53 | 0x58465342) {
        return Err(rejected(Reject::UnsupportedFilesystem));
    }
    Ok(unsafe {
        std::ptr::read_unaligned((&value.f_fsid as *const libc::fsid_t).cast::<[i32; 2]>())
    })
}
pub fn opaque_identity(file: &File) -> Result<PhysicalIdentity> {
    #[repr(C)]
    struct Handle {
        bytes: u32,
        kind: i32,
        payload: [u8; 128],
    }
    let mut handle = Handle {
        bytes: 128,
        kind: 0,
        payload: [0; 128],
    };
    let mut mount = 0;
    if unsafe {
        libc::syscall(
            libc::SYS_name_to_handle_at,
            file.as_raw_fd(),
            c"".as_ptr(),
            &mut handle,
            &mut mount,
            libc::AT_EMPTY_PATH,
        )
    } < 0
    {
        return Err(os_error());
    }
    if handle.bytes == 0 || handle.bytes > 128 {
        return Err(rejected(Reject::OpaqueHandleSize));
    }
    Ok(PhysicalIdentity::Linux {
        filesystem: filesystem_id(file)?,
        handle_type: handle.kind,
        handle: handle.payload[..handle.bytes as usize].to_vec(),
    })
}
