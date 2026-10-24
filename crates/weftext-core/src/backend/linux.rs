//! Linux controlled-service profile. This deliberately requires administrator
//! enrollment and a dedicated OS identity; flock is only instance coordination.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CString, OsStr, OsString};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::sync::{Arc, Mutex, Weak};

const RESOLVE: u64 = 0x01 | 0x02 | 0x04 | 0x08; // NO_XDEV, NO_MAGICLINKS, NO_SYMLINKS, BENEATH

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LinuxDeviceAuthority {
    pub machine: String,
    pub enrollment: String,
    pub service_uid: u32,
    pub root: FileIdentity,
}

pub(super) struct ControlledRoot {
    directory: File,
    authority: weftext_enrollment::ValidatedEnrollment,
    pub(super) path: PathBuf,
    identity: FileIdentity,
    service_uid: u32,
    registry_lock: File,
    leases: Mutex<BTreeMap<FileIdentity, (usize, bool)>>,
    namespace_readers: Mutex<Vec<Weak<EntryBinding>>>,
    pub(super) device_authority: LinuxDeviceAuthority,
    pub(super) device_key: zeroize::Zeroizing<Vec<u8>>,
}

pub(super) struct FileLease {
    root: Arc<ControlledRoot>,
    identity: FileIdentity,
    write: bool,
}
impl FileLease {
    fn acquire(root: &Arc<ControlledRoot>, file: &File, write: bool) -> Result<Self> {
        let identity = opaque_identity(file)?;
        let mut leases = root
            .leases
            .lock()
            .map_err(|_| error("busy", UiMessage::error_core_posix_file_scope_unavailable()))?;
        let state = leases.entry(identity.clone()).or_default();
        if state.1 || (write && state.0 != 0) {
            return Err(error(
                "busy",
                UiMessage::error_core_posix_incompatible_file_scope(),
            ));
        }
        state.0 += 1;
        state.1 = write;
        Ok(Self {
            root: root.clone(),
            identity,
            write,
        })
    }
}
impl Drop for FileLease {
    fn drop(&mut self) {
        if let Ok(mut leases) = self.root.leases.lock()
            && let Some(state) = leases.get_mut(&self.identity)
        {
            state.0 -= 1;
            if self.write {
                state.1 = false;
            }
            if state.0 == 0 {
                leases.remove(&self.identity);
            }
        }
    }
}

fn error(code: &str, message: UiMessage) -> CoreError {
    CoreError::new(code, message)
}
fn os_error() -> CoreError {
    std::io::Error::last_os_error().into()
}
fn cstring(name: &OsStr) -> Result<CString> {
    name.to_str()
        .ok_or_else(|| error("invalidPath", UiMessage::error_core_posix_non_utf8_name()))?;
    CString::new(name.as_bytes())
        .map_err(|_| error("invalidPath", UiMessage::error_core_posix_nul_name()))
}
fn basename(name: &OsStr) -> Result<CString> {
    if name.is_empty() || name == "." || name == ".." || name.as_bytes().contains(&b'/') {
        return Err(error(
            "invalidPath",
            UiMessage::error_core_posix_basename_required(),
        ));
    }
    cstring(name)
}
#[repr(C)]
struct OpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}
fn open_at(parent: &File, name: &OsStr, flags: i32, mode: u32) -> Result<File> {
    let name = cstring(name)?;
    let how = OpenHow {
        flags: (flags | libc::O_CLOEXEC | libc::O_NOFOLLOW) as u64,
        mode: mode as u64,
        resolve: RESOLVE,
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
fn enrollment_error(value: weftext_enrollment::EnrollmentError) -> CoreError {
    use weftext_enrollment::{EnrollmentError, EnrollmentRejection as Reject};
    let reason = match value {
        EnrollmentError::Io(value) => return value.into(),
        EnrollmentError::Format(value) => return value.into(),
        EnrollmentError::Rejected(reason) => reason,
    };
    let (code, message) = match reason {
        Reject::AbsoluteAdministratorPathRequired => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_absolute_enrollment_required(),
        ),
        Reject::PathTraversal => (
            "invalidPath",
            UiMessage::error_core_posix_traversal_forbidden(),
        ),
        Reject::InvalidName => (
            "invalidPath",
            UiMessage::error_core_posix_exact_utf8_basename_required(),
        ),
        Reject::UntrustedAdministratorPath => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_administrator_enrollment_required(),
        ),
        Reject::FreshRootBirthRequired => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_fresh_root_birth_required(),
        ),
        Reject::DistinctIdentitiesRequired => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_distinct_os_identities_required(),
        ),
        Reject::ServiceAccountRequired => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_non_login_service_required(),
        ),
        Reject::WorkerAccountRequired => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_worker_account_required(),
        ),
        Reject::ExclusiveServiceGroupRequired => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_exclusive_service_group_required(),
        ),
        Reject::EnrollmentIdentityRequired => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_enrollment_uuid_required(),
        ),
        Reject::MachineMismatch => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_foreign_machine(),
        ),
        Reject::KeyPermissions => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_device_key_protection_required(),
        ),
        Reject::ExtendedKeyAcl => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_extended_key_acl_forbidden(),
        ),
        Reject::KeyInsideRoot => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_device_key_outside_profile_required(),
        ),
        Reject::RootPath => (
            "invalidPath",
            UiMessage::error_core_posix_root_path_required(),
        ),
        Reject::RootIdentity => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_physical_root_mismatch(),
        ),
        Reject::RootPermissions => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_root_permissions_required(),
        ),
        Reject::AuthorityChanged => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_authority_changed(),
        ),
        Reject::EnrollmentChanged => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_enrollment_changed(),
        ),
        Reject::ServiceCredentials => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_service_credentials_required(),
        ),
        Reject::PrivilegeGainAllowed => (
            "controlledRootAuthority",
            UiMessage::error_core_posix_privilege_boundary_required(),
        ),
        Reject::UnsupportedFilesystem => (
            "unsupportedVolume",
            UiMessage::error_core_posix_native_filesystem_required(),
        ),
        Reject::OpaqueHandleSize => (
            "unsupportedVolume",
            UiMessage::error_core_posix_opaque_handle_size_unsupported(),
        ),
    };
    error(code, message)
}
fn fsid(file: &File) -> Result<[i32; 2]> {
    weftext_enrollment::filesystem_id(file).map_err(enrollment_error)
}
fn opaque_identity(file: &File) -> Result<FileIdentity> {
    weftext_enrollment::opaque_identity(file).map_err(enrollment_error)
}
impl ControlledRoot {
    pub(super) fn enrollment(&self) -> &weftext_enrollment::ValidatedEnrollment {
        &self.authority
    }
    pub(super) fn admit(enrollment: &Path) -> Result<Arc<Self>> {
        let authority =
            weftext_enrollment::inspect_enrollment(enrollment).map_err(enrollment_error)?;
        authority
            .verify_service_process()
            .map_err(enrollment_error)?;
        let directory = authority.root_directory().try_clone()?;
        let device_key = authority.read_service_key().map_err(enrollment_error)?;
        // Lock the physical directory itself: separate private data directories
        // cannot create independent writers for this same library authority.
        let registry_lock = directory.try_clone()?;
        if unsafe { libc::flock(registry_lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } < 0 {
            let mut e = os_error();
            e.code = "instanceBusy".into();
            return Err(e);
        }
        let device_authority = LinuxDeviceAuthority {
            machine: authority.machine_id().to_owned(),
            enrollment: authority.enrollment_id().to_owned(),
            service_uid: authority.service_uid(),
            root: authority.root_identity().clone(),
        };
        let root = Arc::new(Self {
            directory,
            path: authority.root().to_path_buf(),
            identity: authority.root_identity().clone(),
            service_uid: authority.service_uid(),
            authority,
            registry_lock,
            leases: Mutex::new(BTreeMap::new()),
            namespace_readers: Mutex::new(Vec::new()),
            device_authority,
            device_key,
        });
        root.check_fence()?;
        // Admission validates the existing complete namespace. It never treats
        // unenumerated entries or partial scans as proof of exclusive authority.
        root.validate_tree(&root.directory, 0, &mut 0, Path::new(""))?;
        Ok(root)
    }
    pub(super) fn check_fence(&self) -> Result<()> {
        self.authority
            .verify_service_process()
            .map_err(enrollment_error)?;
        self.authority.check_current().map_err(enrollment_error)?;
        let meta = self.directory.metadata()?;
        if meta.uid() != self.service_uid
            || meta.mode() & 0o777 != 0o700
            || opaque_identity(&self.directory)? != self.identity
        {
            return Err(error(
                "controlledRootAuthority",
                UiMessage::error_core_posix_authority_changed(),
            ));
        }
        if unsafe {
            libc::flock(
                self.registry_lock.as_raw_fd(),
                libc::LOCK_EX | libc::LOCK_NB,
            )
        } < 0
        {
            return Err(os_error());
        }
        Ok(())
    }
    pub(super) fn check_path(&self, path: &Path) -> Result<()> {
        self.check_fence()?;
        if !path.is_absolute()
            || !path.starts_with(&self.path)
            || path.components().any(|part| {
                !matches!(
                    part,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
        {
            return Err(error(
                "outsideRoot",
                UiMessage::error_core_posix_outside_controlled_root(),
            ));
        }
        cstring(path.as_os_str())?;
        Ok(())
    }
    fn verify_object(&self, file: &File, directory: bool) -> Result<()> {
        self.check_fence()?;
        let m = file.metadata()?;
        let expected = if directory {
            m.is_dir()
        } else {
            m.is_file() && m.nlink() == 1
        };
        if !expected
            || m.uid() != self.service_uid
            || m.mode() & 0o077 != 0
            || m.mode() & 0o7000 != 0
        {
            return Err(error(
                "unsupportedPath",
                UiMessage::error_core_posix_private_ordinary_entry_required(),
            ));
        }
        if fsid(file)?
            != match &self.identity {
                FileIdentity::Linux { filesystem, .. } => *filesystem,
                _ => unreachable!(),
            }
        {
            return Err(error(
                "crossVolume",
                UiMessage::error_core_posix_filesystem_changed(),
            ));
        }
        opaque_identity(file)?;
        Ok(())
    }
    fn check_namespace_mutation(&self, file: &BoundFile) -> Result<()> {
        let identity = opaque_identity(file)?;
        let mut readers = self.namespace_readers.lock().map_err(|_| {
            error(
                "busy",
                UiMessage::error_core_posix_namespace_reads_unavailable(),
            )
        })?;
        readers.retain(|reader| reader.strong_count() != 0);
        for reader in readers.iter().filter_map(Weak::upgrade) {
            if Arc::ptr_eq(&reader, &file.binding) {
                continue;
            }
            let mut entry = Some(reader);
            while let Some(current) = entry {
                if opaque_identity(&current.file)? == identity {
                    return Err(error(
                        "busy",
                        UiMessage::error_core_posix_namespace_read_conflict(),
                    ));
                }
                entry = current
                    .parent
                    .lock()
                    .map_err(|_| {
                        error(
                            "busy",
                            UiMessage::error_core_posix_read_parent_unavailable(),
                        )
                    })?
                    .clone();
            }
        }
        Ok(())
    }
    fn validate_tree(&self, directory: &File, depth: usize, used: &mut usize, relative: &Path) -> Result<()> {
        self.verify_object(directory, true)?;
        if depth > 128 {
            return Err(error("budget", UiMessage::error_core_posix_depth_budget()));
        }
        let (entries, complete) = directory_entries(directory, 5000usize.saturating_sub(*used))?;
        if !complete {
            return Err(error(
                "budget",
                UiMessage::error_core_posix_admission_budget(),
            ));
        }
        *used += entries.len();
        for (name, entry) in entries {
            if entry.is_unsupported() {
                return Err(error(
                    "unsupportedPath",
                    UiMessage::error_core_posix_symbolic_or_special_entry(),
                ));
            }
            let file = open_at(
                directory,
                &name,
                libc::O_RDONLY
                    | if entry.is_directory() {
                        libc::O_DIRECTORY
                    } else {
                        0
                    },
                0,
            )?;
            self.verify_object(&file, entry.is_directory())?;
            if entry.is_directory() {
                let next = relative.join(&name);
                // Only the canonical flat journal's 256 exact hexadecimal
                // buckets receive a separate 5000-entry admission allowance.
                // They are still fully traversed and ownership-verified.
                let journal_bucket = relative.file_name() == Some(OsStr::new("journal"))
                    && name.to_str().is_some_and(|name| name.len() == 2
                        && name.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
                if journal_bucket {
                    let mut bucket_count = 0;
                    self.validate_tree(&file, depth + 1, &mut bucket_count, &next)?;
                } else {
                    self.validate_tree(&file, depth + 1, used, &next)?;
                }
            }
        }
        Ok(())
    }
}
fn bound_root(root: &Arc<ControlledRoot>) -> Result<BoundFile> {
    root.check_fence()?;
    let mut file = BoundFile::bind(root.directory.try_clone()?, None, root.path.as_os_str())?;
    Arc::get_mut(&mut file.binding).unwrap().root = Some(root.clone());
    Ok(file)
}
fn root_for(file: &BoundFile) -> Result<&Arc<ControlledRoot>> {
    file.binding.root.as_ref().ok_or_else(|| {
        error(
            "controlledRootRequired",
            UiMessage::error_core_posix_descriptor_authority_required(),
        )
    })
}
fn fenced_fact<T>(file: &BoundFile, read: impl FnOnce(&File) -> Result<T>) -> Result<T> {
    let root = root_for(file)?;
    root.check_fence()?;
    let value = read(file)?;
    root.check_fence()?;
    Ok(value)
}
pub(super) fn create_private_directory(root: &Arc<ControlledRoot>, path: &Path) -> Result<()> {
    root.check_path(path)?;
    let mut current = bound_root(root)?;
    for part in path.strip_prefix(&root.path).unwrap().components() {
        let std::path::Component::Normal(name) = part else {
            return Err(error(
                "invalidPath",
                UiMessage::error_core_posix_private_traversal(),
            ));
        };
        current = match open_bound(&current, name, true, false, false) {
            Ok(directory) => directory,
            Err(e) if e.code == "missing" => open_bound(&current, name, true, true, true)?,
            Err(e) => return Err(e),
        };
    }
    Ok(())
}
fn open_bound(
    parent: &BoundFile,
    name: &OsStr,
    directory: bool,
    write: bool,
    create: bool,
) -> Result<BoundFile> {
    open_bound_role(parent, name, directory, write, create, false)
}
fn open_bound_role(
    parent: &BoundFile,
    name: &OsStr,
    directory: bool,
    write: bool,
    create: bool,
    placement: bool,
) -> Result<BoundFile> {
    let root = root_for(parent)?;
    root.check_fence()?;
    let c = basename(name)?;
    // Inspect through O_PATH before opening data access. Opening a FIFO or
    // device to discover its type could block or cause a device side effect.
    let original = if !create && !directory {
        let probe = open_at(parent, name, libc::O_PATH, 0)?;
        root.verify_object(&probe, false)?;
        Some(opaque_identity(&probe)?)
    } else {
        None
    };
    if directory && create {
        if unsafe { libc::mkdirat(parent.as_raw_fd(), c.as_ptr(), 0o700) } < 0 {
            return Err(os_error());
        }
        parent.sync_all()?;
    }
    let flags = if placement {
        libc::O_PATH
    } else if directory {
        libc::O_RDONLY | libc::O_DIRECTORY
    } else {
        (if write { libc::O_RDWR } else { libc::O_RDONLY })
            | libc::O_NONBLOCK
            | libc::O_NOCTTY
            | if create {
                libc::O_CREAT | libc::O_EXCL
            } else {
                0
            }
    };
    let file = open_at(
        parent,
        name,
        flags,
        if !directory && create { 0o600 } else { 0 },
    )?;
    root.verify_object(&file, directory)?;
    if let Some(identity) = original
        && opaque_identity(&file)? != identity
    {
        return Err(error(
            "thirdState",
            UiMessage::error_core_posix_original_entry_changed_before_open(),
        ));
    }
    let mut bound = BoundFile::bind(file, Some(parent), name)?;
    if !directory && !placement {
        bound.lease = Some(FileLease::acquire(root, &bound, write)?);
        if !write {
            root.namespace_readers
                .lock()
                .map_err(|_| {
                    error(
                        "busy",
                        UiMessage::error_core_posix_namespace_reads_unavailable(),
                    )
                })?
                .push(Arc::downgrade(&bound.binding));
        }
    }
    if create {
        bound.sync_all()?;
        parent.sync_all()?;
    }
    Ok(bound)
}
pub(crate) fn stamp(file: &BoundFile) -> Result<FileStamp> {
    fenced_fact(file, raw_stamp)
}
fn raw_stamp(file: &File) -> Result<FileStamp> {
    let m = file.metadata()?;
    let seconds = u64::try_from(m.mtime()).map_err(|_| {
        error(
            "unsupportedPath",
            UiMessage::error_core_posix_negative_modified_time(),
        )
    })?;
    let modified = seconds
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(m.mtime_nsec() as u64))
        .ok_or_else(|| {
            error(
                "unsupportedPath",
                UiMessage::error_core_posix_modified_time_range(),
            )
        })?;
    Ok(FileStamp {
        identity: opaque_identity(file)?,
        modified,
        size: m.len(),
    })
}
pub(crate) fn attributes(file: &BoundFile) -> Result<EntryMetadata> {
    fenced_fact(file, raw_attributes)
}
fn raw_attributes(file: &File) -> Result<EntryMetadata> {
    let m = file.metadata()?;
    Ok(EntryMetadata {
        kind: if m.is_dir() {
            EntryKind::Directory
        } else if m.is_file() {
            EntryKind::File
        } else {
            EntryKind::Unsupported
        },
        qualified: m.is_dir() || (m.is_file() && m.nlink() == 1),
        native: NativeMetadata::Linux {
            mode: m.mode(),
            uid: m.uid(),
            gid: m.gid(),
            links: m.nlink(),
        },
    })
}
pub(crate) fn file_coordinates(file: &BoundFile) -> Result<PathBuf> {
    fenced_fact(file, raw_file_coordinates)
}
fn raw_file_coordinates(file: &File) -> Result<PathBuf> {
    let path = std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))?;
    cstring(path.as_os_str())?;
    if !path.is_absolute() || file.metadata()?.nlink() == 0 {
        return Err(error(
            "identityPending",
            UiMessage::error_core_posix_current_coordinate_unavailable(),
        ));
    }
    Ok(path)
}
fn directory_entries(
    directory: &File,
    max: usize,
) -> Result<(Vec<(OsString, EntryMetadata)>, bool)> {
    // A separate open-file description prevents concurrent enumerations sharing
    // or exhausting one descriptor's seek position.
    let scan = open_at(
        directory,
        OsStr::new("."),
        libc::O_RDONLY | libc::O_DIRECTORY,
        0,
    )?;
    let mut buffer = [0u8; 32768];
    let mut entries = Vec::new();
    loop {
        let n = unsafe {
            libc::syscall(
                libc::SYS_getdents64,
                scan.as_raw_fd(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        };
        if n < 0 {
            return Err(os_error());
        }
        if n as usize > buffer.len() {
            return Err(error(
                "inventoryUnavailable",
                UiMessage::error_core_posix_invalid_directory_entry(),
            ));
        }
        if n == 0 {
            return Ok((entries, true));
        }
        let mut offset = 0;
        while offset < n as usize {
            if offset + 20 > n as usize {
                return Err(error(
                    "inventoryUnavailable",
                    UiMessage::error_core_posix_truncated_directory_entry(),
                ));
            }
            let len =
                u16::from_ne_bytes(buffer[offset + 16..offset + 18].try_into().unwrap()) as usize;
            if len < 20 || !len.is_multiple_of(8) || offset + len > n as usize {
                return Err(error(
                    "inventoryUnavailable",
                    UiMessage::error_core_posix_invalid_directory_entry(),
                ));
            }
            let name = &buffer[offset + 19..offset + len];
            let end = name.iter().position(|b| *b == 0).ok_or_else(|| {
                error(
                    "inventoryUnavailable",
                    UiMessage::error_core_posix_unterminated_directory_entry(),
                )
            })?;
            let name = OsString::from_vec(name[..end].to_vec());
            offset += len;
            if name == "." || name == ".." {
                continue;
            }
            basename(&name)?;
            if entries.len() == max {
                return Ok((entries, false));
            }
            // O_PATH lets us inspect a symlink/special object without opening
            // its target, blocking on a FIFO or executing device behavior.
            let c = basename(&name)?;
            let fd = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    c.as_ptr(),
                    libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(os_error());
            }
            let file = unsafe { File::from_raw_fd(fd) };
            let metadata = raw_attributes(&file)?;
            if metadata.is_unsupported() {
                return Err(error(
                    "unsupportedPath",
                    UiMessage::error_core_posix_ordinary_entry_required(),
                ));
            }
            let qualified = open_at(
                directory,
                &name,
                libc::O_PATH
                    | if metadata.is_directory() {
                        libc::O_DIRECTORY
                    } else {
                        0
                    },
                0,
            )?;
            opaque_identity(&qualified)?;
            entries.push((name, metadata));
        }
    }
}

pub(crate) struct ParentGuard {
    dirs: Vec<BoundFile>,
    pub name: OsString,
}
impl ParentGuard {
    pub fn acquire(backend: &BackendContext, path: &Path) -> Result<Self> {
        let absolute = crate::absolute_path(path)?;
        backend.check_path(&absolute)?;
        let root = backend.root.clone();
        let relative = absolute.strip_prefix(&root.path).map_err(|_| {
            error(
                "outsideRoot",
                UiMessage::error_core_posix_path_outside_root(),
            )
        })?;
        let name = relative
            .file_name()
            .ok_or_else(|| error("invalidPath", UiMessage::error_core_posix_leaf_required()))?
            .to_owned();
        basename(&name)?;
        let mut dirs = vec![bound_root(&root)?];
        for component in relative.parent().unwrap().components() {
            let std::path::Component::Normal(component) = component else {
                return Err(error(
                    "invalidPath",
                    UiMessage::error_core_posix_path_traversal(),
                ));
            };
            let next = open_bound(dirs.last().unwrap(), component, true, false, false)?;
            dirs.push(next);
        }
        Ok(Self { dirs, name })
    }
    fn parent(&self) -> &BoundFile {
        self.dirs.last().unwrap()
    }
    pub fn directory_identity(&self) -> Result<FileIdentity> {
        Ok(stamp(self.parent())?.identity)
    }
    pub(crate) fn directory_coordinates(&self) -> Result<PathBuf> {
        file_coordinates(self.parent())
    }
    pub fn directory_entries(&self, max: usize) -> Result<(Vec<(OsString, EntryMetadata)>, bool)> {
        fenced_fact(self.parent(), |file| directory_entries(file, max))
    }
    pub fn open_file(&self, write: bool, create: bool) -> Result<BoundFile> {
        open_bound(self.parent(), &self.name, false, write, create)
    }
    pub(crate) fn open_retiring_file(&self, _: bool) -> Result<BoundFile> {
        self.open_file(false, false)
    }
    pub(crate) fn open_placement_file(&self) -> Result<BoundFile> {
        open_bound_role(self.parent(), &self.name, false, false, false, true)
    }
    pub(crate) fn open_read_guard_file(&self) -> Result<BoundFile> {
        self.open_file(false, false)
    }
    pub fn create_directory(&self) -> Result<BoundFile> {
        open_bound(self.parent(), &self.name, true, true, true)
    }
    pub fn child_file(directory: &BoundFile, name: &str) -> Result<BoundFile> {
        open_bound(directory, OsStr::new(name), false, true, true)
    }
    pub fn child_directory(directory: &BoundFile, name: &str) -> Result<BoundFile> {
        open_bound(directory, OsStr::new(name), true, true, true)
    }
}

fn rename_bound(
    file: &BoundFile,
    target_parent: &BoundFile,
    name: &OsStr,
    replace: bool,
) -> Result<()> {
    let root = root_for(file)?;
    if !Arc::ptr_eq(root, root_for(target_parent)?) {
        return Err(error(
            "crossVolume",
            UiMessage::error_core_posix_cross_root_move(),
        ));
    }
    root.check_fence()?;
    root.check_namespace_mutation(file)?;
    let parent = file
        .binding
        .parent
        .lock()
        .map_err(|_| {
            error(
                "busy",
                UiMessage::error_core_posix_entry_parent_unavailable(),
            )
        })?
        .clone()
        .ok_or_else(|| {
            error(
                "invalidPath",
                UiMessage::error_core_posix_root_move_forbidden(),
            )
        })?;
    let mut old_name = file.binding.name.lock().map_err(|_| {
        error(
            "busy",
            UiMessage::error_core_posix_entry_binding_unavailable(),
        )
    })?;
    let old = basename(&old_name)?;
    let new = basename(name)?;
    // Service DAC isolation plus the retained root writer fence excludes
    // noncooperative namespace writers; the retained entry is never guessed
    // from inode coordinates. Verify the original opaque entry immediately.
    let original = open_at(&parent.file, &old_name, libc::O_PATH, 0)?;
    if opaque_identity(&original)? != opaque_identity(file)? {
        return Err(error(
            "thirdState",
            UiMessage::error_core_posix_original_entry_changed(),
        ));
    }
    if unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            parent.file.as_raw_fd(),
            old.as_ptr(),
            target_parent.as_raw_fd(),
            new.as_ptr(),
            if replace { 0 } else { libc::RENAME_NOREPLACE },
        )
    } < 0
    {
        return Err(os_error());
    }
    *file.binding.parent.lock().map_err(|_| {
        error(
            "busy",
            UiMessage::error_core_posix_entry_parent_unavailable(),
        )
    })? = Some(target_parent.binding.clone());
    *old_name = name.to_owned();
    // A successful syscall changes the retained binding even if a following
    // fsync reports an unknown durable outcome. Recovery keeps the operation.
    target_parent.sync_all()?;
    parent.file.sync_all()?;
    Ok(())
}
pub(crate) fn publish_private_create(
    backend: &BackendContext,
    from: &Path,
    to: &Path,
) -> Result<()> {
    publish_private(backend, from, to, false)
}
pub(crate) fn publish_private_replace(
    backend: &BackendContext,
    from: &Path,
    to: &Path,
) -> Result<()> {
    publish_private(backend, from, to, true)
}
fn publish_private(backend: &BackendContext, from: &Path, to: &Path, replace: bool) -> Result<()> {
    let source = ParentGuard::acquire(backend, from)?;
    let target = ParentGuard::acquire(backend, to)?;
    let file = source.open_file(true, false)?;
    file.sync_all()?;
    rename_bound(&file, target.parent(), &target.name, replace)
}

pub(crate) struct NamespaceGuard {
    root: Arc<ControlledRoot>,
    dirs: BTreeMap<PathBuf, BoundFile>,
    source: PathBuf,
    target_parent: PathBuf,
}
impl NamespaceGuard {
    pub fn acquire(backend: &BackendContext, source: &Path, target_parent: &Path) -> Result<Self> {
        Ok(MultiNamespaceGuard::acquire(backend, &[(source, target_parent)])?.inner)
    }
    pub fn source_identity(&self) -> Result<FileIdentity> {
        Ok(stamp(&self.dirs[&self.source])?.identity)
    }
    pub(crate) fn source_coordinates(&self) -> Result<PathBuf> {
        file_coordinates(&self.dirs[&self.source])
    }
    pub fn parent_identity(&self) -> Result<FileIdentity> {
        Ok(stamp(&self.dirs[&self.target_parent])?.identity)
    }
    pub fn open_source_file(&self, name: &str, _: bool) -> Result<BoundFile> {
        open_bound(
            &self.dirs[&self.source],
            OsStr::new(name),
            false,
            false,
            false,
        )
    }
    fn source_directory(&mut self, relative: &Path) -> Result<&BoundFile> {
        self.root.check_fence()?;
        let mut path = self.source.clone();
        for component in relative.components() {
            let std::path::Component::Normal(name) = component else {
                return Err(error(
                    "invalidPath",
                    UiMessage::error_core_posix_relative_traversal(),
                ));
            };
            let next = path.join(name);
            if !self.dirs.contains_key(&next) {
                let file = open_bound(&self.dirs[&path], name, true, false, false)?;
                self.dirs.insert(next.clone(), file);
            }
            if file_coordinates(&self.dirs[&next])?
                != file_coordinates(&self.dirs[&path])?.join(name)
            {
                return Err(error(
                    "thirdState",
                    UiMessage::error_core_backend_source_descendant_placement_changed(),
                ));
            }
            path = next;
        }
        self.root.check_fence()?;
        Ok(&self.dirs[&path])
    }
    pub fn source_directory_snapshot(
        &mut self,
        relative: &Path,
        max: usize,
    ) -> Result<DirectorySnapshot> {
        let file = self.source_directory(relative)?;
        fenced_fact(file, |file| {
            let (entries, complete) = directory_entries(file, max)?;
            Ok((raw_stamp(file)?.identity, entries, complete))
        })
    }
    pub fn open_source_descendant(&mut self, relative: &Path) -> Result<BoundFile> {
        self.open_source_output(relative, false, false)
    }
    pub(crate) fn open_source_placement(&mut self, relative: &Path) -> Result<BoundFile> {
        let name = relative.file_name().ok_or_else(|| {
            error(
                "invalidPath",
                UiMessage::error_core_posix_source_basename_required(),
            )
        })?;
        open_bound_role(
            self.source_directory(relative.parent().unwrap())?,
            name,
            false,
            false,
            false,
            true,
        )
    }
    pub(crate) fn source_parent_identity(&mut self, relative: &Path) -> Result<FileIdentity> {
        Ok(
            stamp(self.source_directory(relative.parent().ok_or_else(|| {
                error("invalidPath", UiMessage::error_core_posix_parent_required())
            })?)?)?
            .identity,
        )
    }
    pub fn open_source_output(
        &mut self,
        relative: &Path,
        write: bool,
        create: bool,
    ) -> Result<BoundFile> {
        let name = relative.file_name().ok_or_else(|| {
            error(
                "invalidPath",
                UiMessage::error_core_posix_source_basename_required(),
            )
        })?;
        open_bound(
            self.source_directory(relative.parent().unwrap())?,
            name,
            false,
            write,
            create,
        )
    }
    pub(crate) fn open_source_retiring(&mut self, relative: &Path, _: bool) -> Result<BoundFile> {
        self.open_source_output(relative, false, false)
    }
    pub fn release_source_descendants(&mut self) {
        self.dirs
            .retain(|path, _| path == &self.source || !path.starts_with(&self.source));
    }
    pub fn rename_leaf(&self, file: &BoundFile, name: &str) -> Result<()> {
        rename_bound(file, &self.dirs[&self.source], OsStr::new(name), false)
    }
    pub fn move_source(&self, name: &str) -> Result<()> {
        rename_bound(
            &self.dirs[&self.source],
            &self.dirs[&self.target_parent],
            OsStr::new(name),
            false,
        )
    }
    pub fn open_moved_file(&self, name: &str) -> Result<BoundFile> {
        self.open_source_file(name, false)
    }
}
pub(crate) struct MultiNamespaceGuard {
    inner: NamespaceGuard,
    sources: Vec<PathBuf>,
    parents: Vec<PathBuf>,
}
impl MultiNamespaceGuard {
    pub(crate) fn acquire(
        backend: &BackendContext,
        participants: &[(&Path, &Path)],
    ) -> Result<Self> {
        Self::acquire_planned(backend, participants, &[])
    }
    pub(crate) fn acquire_planned(
        backend: &BackendContext,
        participants: &[(&Path, &Path)],
        planned: &[PathBuf],
    ) -> Result<Self> {
        Self::acquire_with_ancestors(backend, participants, planned, &[])
    }
    pub(crate) fn acquire_with_ancestors(
        backend: &BackendContext,
        participants: &[(&Path, &Path)],
        planned: &[PathBuf],
        ancestors: &[PathBuf],
    ) -> Result<Self> {
        Self::acquire_union(backend, participants, planned, ancestors, false)
    }
    pub(crate) fn acquire_forest_with_ancestors(
        backend: &BackendContext,
        participants: &[(&Path, &Path)],
        planned: &[PathBuf],
        ancestors: &[PathBuf],
    ) -> Result<Self> {
        Self::acquire_union(backend, participants, planned, ancestors, true)
    }
    pub(crate) fn acquire_anchors(
        backend: &BackendContext,
        ancestors: &[PathBuf],
        planned: &[PathBuf],
    ) -> Result<Self> {
        Self::acquire_union(backend, &[], planned, ancestors, true)
    }
    fn acquire_union(
        backend: &BackendContext,
        participants: &[(&Path, &Path)],
        planned: &[PathBuf],
        ancestors: &[PathBuf],
        forest: bool,
    ) -> Result<Self> {
        if (participants.is_empty() && ancestors.is_empty())
            || participants.len() + planned.len() + ancestors.len() > 5000
        {
            return Err(error(
                "budget",
                UiMessage::error_core_posix_participant_budget(),
            ));
        }
        let sources = participants
            .iter()
            .map(|(path, _)| crate::absolute_path(path))
            .collect::<Result<Vec<_>>>()?;
        let parents = participants
            .iter()
            .map(|(_, path)| crate::absolute_path(path))
            .collect::<Result<Vec<_>>>()?;
        let root = backend.root.clone();
        for path in sources
            .iter()
            .chain(&parents)
            .chain(ancestors)
            .chain(planned)
        {
            root.check_path(path)?;
        }
        if !forest
            && sources.iter().enumerate().any(|(i, source)| {
                parents.iter().any(|parent| parent.starts_with(source))
                    || sources
                        .iter()
                        .enumerate()
                        .any(|(j, other)| i != j && other.starts_with(source))
            })
        {
            return Err(error(
                "recursiveMove",
                UiMessage::error_core_posix_independent_roots_required(),
            ));
        }
        let mut dirs = BTreeMap::new();
        dirs.insert(root.path.clone(), bound_root(&root)?);
        let planned: BTreeSet<_> = planned.iter().cloned().collect();
        for leaf in sources.iter().chain(&parents).chain(ancestors) {
            let mut current = root.path.clone();
            for part in leaf.strip_prefix(&root.path).unwrap().components() {
                let std::path::Component::Normal(name) = part else {
                    return Err(error(
                        "invalidPath",
                        UiMessage::error_core_posix_namespace_traversal(),
                    ));
                };
                let next = current.join(name);
                if !dirs.contains_key(&next) {
                    match open_bound(&dirs[&current], name, true, false, false) {
                        Ok(file) => {
                            dirs.insert(next.clone(), file);
                        }
                        Err(e) if e.code == "missing" && planned.contains(&next) => break,
                        Err(e) => return Err(e),
                    }
                }
                current = next;
            }
        }
        Ok(Self {
            inner: NamespaceGuard {
                root: root.clone(),
                dirs,
                source: sources
                    .first()
                    .cloned()
                    .unwrap_or_else(|| root.path.clone()),
                target_parent: parents
                    .first()
                    .cloned()
                    .unwrap_or_else(|| root.path.clone()),
            },
            sources,
            parents,
        })
    }
    pub(crate) fn select(&mut self, index: usize) -> Result<&mut NamespaceGuard> {
        self.inner.root.check_fence()?;
        self.inner.source = self
            .sources
            .get(index)
            .ok_or_else(|| {
                error(
                    "invalidPlan",
                    UiMessage::error_core_posix_participant_missing(),
                )
            })?
            .clone();
        self.inner.target_parent = self.parents[index].clone();
        self.inner.root.check_fence()?;
        Ok(&mut self.inner)
    }
    pub(crate) fn release_source_descendants(&mut self, index: usize) -> Result<()> {
        let source = self.sources.get(index).ok_or_else(|| {
            error(
                "invalidPlan",
                UiMessage::error_core_posix_participant_missing(),
            )
        })?;
        self.inner
            .dirs
            .retain(|path, _| self.sources.contains(path) || !path.starts_with(source));
        Ok(())
    }
    pub(crate) fn finish_source_move(
        &mut self,
        index: usize,
        name: &str,
        canonical: &str,
        expected: &FileIdentity,
    ) -> Result<()> {
        let source = self
            .sources
            .get(index)
            .ok_or_else(|| {
                error(
                    "invalidPlan",
                    UiMessage::error_core_posix_participant_missing(),
                )
            })?
            .clone();
        let parent = self.parents[index].clone();
        let original = &self.inner.dirs[&source];
        let witness = open_bound(original, OsStr::new(canonical), false, false, false)?;
        if &stamp(&witness)?.identity != expected {
            return Err(error(
                "thirdState",
                UiMessage::error_core_posix_canonical_witness_changed(),
            ));
        }
        let reopened = open_bound(
            &self.inner.dirs[&parent],
            OsStr::new(name),
            true,
            false,
            false,
        )?;
        if stamp(&reopened)?.identity != stamp(original)?.identity {
            return Err(error(
                "thirdState",
                UiMessage::error_core_posix_directory_witness_changed(),
            ));
        }
        self.inner.dirs.insert(source, reopened);
        Ok(())
    }
    pub(crate) fn move_source_to(&self, index: usize, target: &Path) -> Result<()> {
        let source = self
            .sources
            .get(index)
            .and_then(|p| self.inner.dirs.get(p))
            .ok_or_else(|| {
                error(
                    "invalidPlan",
                    UiMessage::error_core_posix_moving_root_missing(),
                )
            })?;
        self.move_file_to(source, target)
    }
    pub(crate) fn move_file_to(&self, file: &BoundFile, target: &Path) -> Result<()> {
        let parent = self.directory_file(target.parent().ok_or_else(|| {
            error(
                "invalidPath",
                UiMessage::error_core_posix_moving_parent_required(),
            )
        })?)?;
        rename_bound(
            file,
            parent,
            target.file_name().ok_or_else(|| {
                error(
                    "invalidPath",
                    UiMessage::error_core_posix_moving_basename_required(),
                )
            })?,
            false,
        )
    }
    pub(crate) fn source_parent_identity(&self, index: usize) -> Result<FileIdentity> {
        let parent = self
            .sources
            .get(index)
            .and_then(|p| p.parent())
            .ok_or_else(|| {
                error(
                    "invalidPlan",
                    UiMessage::error_core_posix_source_parent_missing(),
                )
            })?;
        Ok(stamp(self.directory_file(parent)?)?.identity)
    }
    pub(crate) fn directory_identity(&self, path: &Path) -> Result<Option<FileIdentity>> {
        self.inner.root.check_fence()?;
        let identity = self
            .inner
            .dirs
            .get(path)
            .map(|f| stamp(f).map(|stamp| stamp.identity))
            .transpose()?;
        self.inner.root.check_fence()?;
        Ok(identity)
    }
    pub(crate) fn directory_file(&self, path: &Path) -> Result<&BoundFile> {
        self.inner.root.check_fence()?;
        let file = self.inner.dirs.get(path).ok_or_else(|| {
            error(
                "missing",
                UiMessage::error_core_posix_directory_not_retained(),
            )
        })?;
        self.inner.root.check_fence()?;
        Ok(file)
    }
    pub(crate) fn retain_directory(&mut self, path: &Path) -> Result<&BoundFile> {
        self.inner.root.check_fence()?;
        if !self.inner.dirs.contains_key(path) {
            let parent = path.parent().ok_or_else(|| {
                error(
                    "invalidPath",
                    UiMessage::error_core_posix_directory_parent_required(),
                )
            })?;
            self.retain_directory(parent)?;
            let file = open_bound(
                &self.inner.dirs[parent],
                path.file_name().unwrap(),
                true,
                false,
                false,
            )?;
            self.inner.dirs.insert(path.to_owned(), file);
        }
        self.directory_file(path)
    }
    pub(crate) fn retain_target_parent(&mut self, index: usize) -> Result<()> {
        let parent = self
            .parents
            .get(index)
            .ok_or_else(|| {
                error(
                    "invalidPlan",
                    UiMessage::error_core_posix_target_parent_missing(),
                )
            })?
            .clone();
        self.retain_directory(&parent)?;
        Ok(())
    }
    pub(crate) fn create_directory(&mut self, path: &Path) -> Result<&BoundFile> {
        self.inner.root.check_fence()?;
        if self.inner.dirs.contains_key(path) {
            return Err(error(
                "exists",
                UiMessage::error_core_posix_directory_already_retained(),
            ));
        }
        let parent = self.directory_file(path.parent().ok_or_else(|| {
            error(
                "invalidPath",
                UiMessage::error_core_posix_created_parent_required(),
            )
        })?)?;
        let file = open_bound(parent, path.file_name().unwrap(), true, true, true)?;
        self.inner.dirs.insert(path.to_owned(), file);
        self.directory_file(path)
    }
    pub(crate) fn target_parent_identity(&self, index: usize) -> Result<Option<FileIdentity>> {
        self.directory_identity(self.parents.get(index).ok_or_else(|| {
            error(
                "invalidPlan",
                UiMessage::error_core_posix_target_parent_missing(),
            )
        })?)
    }
    pub(crate) fn open_output(&self, path: &Path, write: bool, create: bool) -> Result<BoundFile> {
        open_bound(
            self.directory_file(path.parent().ok_or_else(|| {
                error(
                    "invalidPath",
                    UiMessage::error_core_posix_output_parent_required(),
                )
            })?)?,
            path.file_name().ok_or_else(|| {
                error(
                    "invalidPath",
                    UiMessage::error_core_posix_output_basename_required(),
                )
            })?,
            false,
            write,
            create,
        )
    }
    pub(crate) fn directory_snapshot(&self, path: &Path, max: usize) -> Result<DirectorySnapshot> {
        let file = self.directory_file(path)?;
        fenced_fact(file, |file| {
            let (entries, complete) = directory_entries(file, max)?;
            Ok((raw_stamp(file)?.identity, entries, complete))
        })
    }
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;
