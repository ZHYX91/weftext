//! Qualified physical filesystem capabilities. Coordinates are never identity
//! or authority; every namespace mutation retains its original entry binding.
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};
use weftext_messages::UiMessage;

pub(crate) use weftext_enrollment::PhysicalIdentity as FileIdentity;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EntryKind {
    File,
    Directory,
    Unsupported,
}

/// Preserved native metadata is tagged. Consumers use kind/qualification rather
/// than interpreting Windows bits on a different operating system.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "platform", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum NativeMetadata {
    Windows {
        attributes: u32,
    },
    Linux {
        mode: u32,
        uid: u32,
        gid: u32,
        links: u64,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EntryMetadata {
    pub kind: EntryKind,
    pub qualified: bool,
    pub native: NativeMetadata,
}

impl EntryMetadata {
    pub fn is_directory(&self) -> bool {
        self.kind == EntryKind::Directory
    }
    pub fn is_file(&self) -> bool {
        self.kind == EntryKind::File && self.qualified
    }
    pub fn is_unsupported(&self) -> bool {
        !self.qualified
    }
    #[cfg(windows)]
    fn windows(attributes: u32) -> Self {
        Self {
            kind: if attributes & 0x10 != 0 {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            qualified: attributes & (0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) == 0,
            native: NativeMetadata::Windows { attributes },
        }
    }
}

impl std::fmt::Display for EntryMetadata {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&serde_json::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}

/// Owns the actual file and its original entry binding. Linux additionally
/// retains parent descriptors; Windows addresses the intrinsic NT file entry
/// handle and its existing guard closure, including the witness handoff.
pub(crate) struct BoundFile {
    file: File,
    binding: std::sync::Arc<EntryBinding>,
    #[cfg(target_os = "linux")]
    lease: Option<platform::FileLease>,
}
struct EntryBinding {
    file: File,
    #[cfg(target_os = "linux")]
    parent: std::sync::Mutex<Option<std::sync::Arc<EntryBinding>>>,
    name: std::sync::Mutex<std::ffi::OsString>,
    #[cfg(target_os = "linux")]
    root: Option<std::sync::Arc<platform::ControlledRoot>>,
}
impl BoundFile {
    fn bind(file: File, parent: Option<&BoundFile>, name: &std::ffi::OsStr) -> Result<Self> {
        let binding = std::sync::Arc::new(EntryBinding {
            file: file.try_clone()?,
            #[cfg(target_os = "linux")]
            parent: std::sync::Mutex::new(parent.map(|p| p.binding.clone())),
            name: std::sync::Mutex::new(name.to_owned()),
            #[cfg(target_os = "linux")]
            root: parent.and_then(|p| p.binding.root.clone()),
        });
        #[cfg(not(target_os = "linux"))]
        let _ = parent;
        Ok(Self {
            file,
            binding,
            #[cfg(target_os = "linux")]
            lease: None,
        })
    }
    fn check_authority(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.binding
                .root
                .as_ref()
                .ok_or_else(|| {
                    CoreError::new(
                        "controlledRootRequired",
                        weftext_messages::UiMessage::error_core_backend_descriptor_authority_required(),
                    )
                })?
                .check_fence()?;
        }
        Ok(())
    }
    pub fn sync_all(&self) -> Result<()> {
        self.check_authority()?;
        Ok(self.file.sync_all()?)
    }
    pub fn set_len(&self, size: u64) -> Result<()> {
        self.check_authority()?;
        Ok(self.file.set_len(size)?)
    }
}

/// Explicit admission authority owned by one Core. Linux admission requires an
/// administrator-enrolled, separately privileged service root.
pub struct BackendContext {
    #[cfg(target_os = "linux")]
    root: std::sync::Arc<platform::ControlledRoot>,
}
impl BackendContext {
    /// Convert an address to the coordinate system reported by retained native
    /// handles. Only the independently qualified volume root is opened here;
    /// descendant qualification still belongs to its original namespace guard.
    pub(crate) fn namespace_coordinate(&self, path: &Path) -> Result<PathBuf> {
        let path = crate::absolute_path(path)?;
        self.check_path(&path)?;
        #[cfg(windows)]
        {
            let drive = path.ancestors().last().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    weftext_messages::UiMessage::error_core_backend_invalid_volume_path(),
                )
            })?;
            let guard = ParentGuard::acquire(self, &drive.join(".coordinate-prospective"))?;
            let relative = path.strip_prefix(drive).map_err(|_| {
                CoreError::new(
                    "invalidPath",
                    weftext_messages::UiMessage::error_core_backend_path_outside_protected_volume(),
                )
            })?;
            Ok(guard.directory_coordinates()?.join(relative))
        }
        #[cfg(not(windows))]
        Ok(path)
    }
    pub(crate) fn ancestors<'a>(&self, path: &'a Path) -> Result<Vec<&'a Path>> {
        self.check_path(path)?;
        Ok(path
            .ancestors()
            .take_while(|ancestor| {
                #[cfg(target_os = "linux")]
                {
                    ancestor.starts_with(&self.root.path)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = ancestor;
                    true
                }
            })
            .collect())
    }
    /// Test presence only after checking this Core's authority and a complete,
    /// qualified parent inventory. Read/permission/unsupported failures remain
    /// failures; only an actually missing path is absence.
    pub(crate) fn entry_exists(&self, path: impl AsRef<Path>) -> Result<bool> {
        let path = crate::absolute_path(path.as_ref())?;
        self.check_path(&path)?;
        #[cfg(target_os = "linux")]
        if path == self.root.path {
            return Ok(true);
        }
        let name = path.file_name().ok_or_else(|| {
            CoreError::new(
                "invalidPath",
                weftext_messages::UiMessage::error_core_backend_entry_leaf_required(),
            )
        })?;
        let parent = path.parent().ok_or_else(|| {
            CoreError::new(
                "invalidPath",
                weftext_messages::UiMessage::error_core_backend_entry_parent_required(),
            )
        })?;
        let guard = match ParentGuard::acquire(self, &parent.join(".private-enumeration")) {
            Ok(guard) => guard,
            Err(error) if error.code == "missing" => return Ok(false),
            Err(error) => return Err(error),
        };
        let (entries, complete) = guard.directory_entries(5000)?;
        qualified_entry_exists(entries, complete, name)
    }
    pub(crate) fn read_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        use std::io::Read;
        let guard = ParentGuard::acquire(self, path.as_ref())?;
        let mut file = guard.open_read_guard_file()?;
        let original = stamp(&file)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        if stamp(&file)? != original || bytes.len() as u64 != original.size {
            return Err(CoreError::new(
                "thirdState",
                weftext_messages::UiMessage::error_core_backend_private_record_changed(),
            ));
        }
        Ok(bytes)
    }
    pub(crate) fn read_optional_json<T: serde::de::DeserializeOwned>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Option<T>> {
        match self.read_bytes(path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(error) if error.code == "missing" => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub(crate) fn open_read(&self, path: impl AsRef<Path>) -> Result<BoundFile> {
        ParentGuard::acquire(self, path.as_ref())?.open_read_guard_file()
    }
    pub(crate) fn read_dir(&self, path: impl AsRef<Path>) -> Result<ScopedDirectoryEntries> {
        let path = path.as_ref();
        let guard = ParentGuard::acquire(self, &path.join(".private-enumeration"))?;
        let (entries, complete) = guard.directory_entries(5000)?;
        if !complete {
            return Err(CoreError::new(
                "budget",
                weftext_messages::UiMessage::error_core_backend_private_namespace_budget(),
            ));
        }
        let entries = entries
            .into_iter()
            .map(|(name, metadata)| {
                if metadata.is_unsupported() {
                    Err(CoreError::new(
                        "unsupportedPath",
                        weftext_messages::UiMessage::error_core_backend_unsupported_private_entry(),
                    ))
                } else {
                    Ok(ScopedDirectoryEntry {
                        path: path.join(&name),
                        name,
                    })
                }
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ScopedDirectoryEntries {
            _guard: guard,
            entries: entries.into_iter(),
        })
    }
    pub fn local() -> Result<Self> {
        #[cfg(windows)]
        {
            Ok(Self {})
        }
        #[cfg(not(windows))]
        {
            Err(CoreError::new(
                "controlledRootRequired",
                weftext_messages::UiMessage::error_core_backend_controlled_root_required(),
            ))
        }
    }
    #[cfg(target_os = "linux")]
    pub fn controlled_service(enrollment: &Path) -> Result<Self> {
        Ok(Self {
            root: platform::ControlledRoot::admit(enrollment)?,
        })
    }
    pub(crate) fn check_path(&self, path: &Path) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.root.check_path(path)?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = path;
        }
        Ok(())
    }
    pub(crate) fn create_private_directory(&self, path: &Path) -> Result<()> {
        self.check_path(path)?;
        #[cfg(target_os = "linux")]
        {
            platform::create_private_directory(&self.root, path)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(std::fs::create_dir_all(path)?)
        }
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn linux_device_authority(&self) -> Result<platform::LinuxDeviceAuthority> {
        self.root.check_fence()?;
        Ok(self.root.device_authority.clone())
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn seal_linux_device(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        use hmac::{Hmac, Mac};
        self.root.check_fence()?;
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(&self.root.device_key).map_err(|_| {
                CoreError::new(
                    "controlledRootAuthority",
                    weftext_messages::UiMessage::error_core_backend_device_authentication_key_unavailable(),
                )
            })?;
        mac.update(b"weftext.device.controlled-service.v1\0");
        mac.update(bytes);
        let mut sealed = mac.finalize().into_bytes().to_vec();
        sealed.extend_from_slice(bytes);
        Ok(sealed)
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn unseal_linux_device(&self, bytes: &[u8]) -> Result<Option<Vec<u8>>> {
        use hmac::{Hmac, Mac};
        self.root.check_fence()?;
        let Some((tag, payload)) = bytes.split_at_checked(32) else {
            return Ok(None);
        };
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(&self.root.device_key).map_err(|_| {
                CoreError::new(
                    "controlledRootAuthority",
                    weftext_messages::UiMessage::error_core_backend_device_authentication_key_unavailable(),
                )
            })?;
        mac.update(b"weftext.device.controlled-service.v1\0");
        mac.update(payload);
        Ok(mac.verify_slice(tag).is_ok().then(|| payload.to_vec()))
    }
}
pub(crate) struct ScopedDirectoryEntry {
    path: PathBuf,
    name: std::ffi::OsString,
}
impl ScopedDirectoryEntry {
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn file_name(&self) -> std::ffi::OsString {
        self.name.clone()
    }
}
pub(crate) struct ScopedDirectoryEntries {
    _guard: ParentGuard,
    entries: std::vec::IntoIter<ScopedDirectoryEntry>,
}
impl Iterator for ScopedDirectoryEntries {
    type Item = Result<ScopedDirectoryEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(Ok)
    }
}
impl std::ops::Deref for BoundFile {
    type Target = File;
    fn deref(&self) -> &File {
        &self.file
    }
}
impl std::ops::DerefMut for BoundFile {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.file
    }
}
impl std::io::Read for BoundFile {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.check_authority().map_err(std::io::Error::other)?;
        std::io::Read::read(&mut self.file, bytes)
    }
}
impl std::io::Write for BoundFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.check_authority().map_err(std::io::Error::other)?;
        std::io::Write::write(&mut self.file, bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.check_authority().map_err(std::io::Error::other)?;
        std::io::Write::flush(&mut self.file)
    }
}
impl std::io::Seek for BoundFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.file, pos)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FileStamp {
    pub identity: FileIdentity,
    pub modified: u64,
    pub size: u64,
}

type DirectorySnapshot = (FileIdentity, Vec<(std::ffi::OsString, EntryMetadata)>, bool);

#[cfg(windows)]
mod platform {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use std::ptr::{null, null_mut};

    type Handle = *mut c_void;
    const READ: u32 = 1;
    const WRITE: u32 = 2;
    const READ_ATTR: u32 = 0x80;
    const SYNCHRONIZE: u32 = 0x100000;
    const SHARE_RW: u32 = 3;
    const DELETE: u32 = 0x10000;
    const DIRECTORY: u32 = 1;
    const NON_DIRECTORY: u32 = 0x40;
    const SYNC: u32 = 0x20;
    const REPARSE_OPEN: u32 = 0x00200000;
    const BAD_ATTR: u32 = 0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000;

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *mut u16,
    }
    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root: Handle,
        name: *mut UnicodeString,
        attributes: u32,
        security: *mut c_void,
        quality: *mut c_void,
    }
    #[repr(C)]
    struct IoStatus {
        status: usize,
        information: usize,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Info {
        attrs: u32,
        created_low: u32,
        created_high: u32,
        accessed_low: u32,
        accessed_high: u32,
        written_low: u32,
        written_high: u32,
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtCreateFile(
            handle: *mut Handle,
            access: u32,
            attrs: *mut ObjectAttributes,
            status: *mut IoStatus,
            allocation: *const i64,
            file_attrs: u32,
            share: u32,
            disposition: u32,
            options: u32,
            ea: *const c_void,
            ea_len: u32,
        ) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
        fn NtQueryDirectoryFile(
            handle: Handle,
            event: Handle,
            apc: *const c_void,
            context: *mut c_void,
            status: *mut IoStatus,
            buffer: *mut c_void,
            length: u32,
            class: u32,
            single: u8,
            name: *const UnicodeString,
            restart: u8,
        ) -> i32;
        fn NtSetInformationFile(
            handle: Handle,
            status: *mut IoStatus,
            information: *const c_void,
            length: u32,
            class: u32,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CompareStringOrdinal(
            left: *const u16,
            left_length: i32,
            right: *const u16,
            right_length: i32,
            ignore_case: i32,
        ) -> i32;
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        fn GetFileInformationByHandle(handle: Handle, info: *mut Info) -> i32;
        fn GetFinalPathNameByHandleW(
            handle: Handle,
            buffer: *mut u16,
            size: u32,
            flags: u32,
        ) -> u32;
        fn GetVolumeInformationW(
            root: *const u16,
            name: *mut u16,
            name_len: u32,
            serial: *mut u32,
            max_component: *mut u32,
            flags: *mut u32,
            filesystem: *mut u16,
            filesystem_len: u32,
        ) -> i32;
    }
    pub(crate) fn namespace_component_compare(
        left: &std::ffi::OsStr,
        right: &std::ffi::OsStr,
    ) -> Result<std::cmp::Ordering> {
        let left = left.encode_wide().collect::<Vec<_>>();
        let right = right.encode_wide().collect::<Vec<_>>();
        let left_length = i32::try_from(left.len()).map_err(|_| {
            CoreError::new(
                "budget",
                UiMessage::error_core_backend_namespace_component_too_long(),
            )
        })?;
        let right_length = i32::try_from(right.len()).map_err(|_| {
            CoreError::new(
                "budget",
                UiMessage::error_core_backend_namespace_component_too_long(),
            )
        })?;
        // Windows ordinal comparison uses the OS uppercase table, without
        // language sorting, normalization or Rust Unicode case expansion.
        match unsafe {
            CompareStringOrdinal(left.as_ptr(), left_length, right.as_ptr(), right_length, 1)
        } {
            0 => Err(std::io::Error::last_os_error().into()),
            1 => Ok(std::cmp::Ordering::Less),
            2 => Ok(std::cmp::Ordering::Equal),
            3 => Ok(std::cmp::Ordering::Greater),
            _ => Err(CoreError::new(
                "io",
                UiMessage::error_core_backend_invalid_windows_ordinal_comparison_result(),
            )),
        }
    }
    fn native_error(status: i32) -> CoreError {
        let code = unsafe { RtlNtStatusToDosError(status) };
        let kind = match code {
            32 | 33 | 1224 => "busy",
            80 | 183 => "exists",
            2 | 3 => "missing",
            5 => "permissionDenied",
            _ => "io",
        };
        let mut error = CoreError::from(std::io::Error::from_raw_os_error(code as i32));
        error.code = kind.into();
        error.explanation =
            weftext_messages::UiMessage::error_core_windows_file_protection_failed();
        error
    }
    pub(crate) fn publish_private_create(
        backend: &BackendContext,
        from: &Path,
        to: &Path,
    ) -> Result<()> {
        publish_private(backend, from, to, 8)
    }
    pub(crate) fn publish_private_replace(
        backend: &BackendContext,
        from: &Path,
        to: &Path,
    ) -> Result<()> {
        // REPLACE_EXISTING | WRITE_THROUGH; caller holds the qualified parent
        // closure and owns the Core lease/CAS for this private record.
        publish_private(backend, from, to, 1 | 8)
    }
    fn publish_private(backend: &BackendContext, from: &Path, to: &Path, flags: u32) -> Result<()> {
        backend.check_path(from)?;
        backend.check_path(to)?;
        let wide = |p: &Path| {
            let absolute = p.as_os_str().to_string_lossy().replace('/', "\\");
            let extended = if absolute.starts_with("\\\\?\\") {
                absolute
            } else {
                format!("\\\\?\\{absolute}")
            };
            extended.encode_utf16().chain([0]).collect::<Vec<_>>()
        };
        let from_wide = wide(from);
        let to_wide = wide(to);
        // Never COPY_ALLOWED: publication cannot silently cross volumes.
        if unsafe { MoveFileExW(from_wide.as_ptr(), to_wide.as_ptr(), flags) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    fn nt_open(
        parent: Option<&BoundFile>,
        name: &std::ffi::OsStr,
        directory: bool,
        write: bool,
        create: bool,
    ) -> Result<BoundFile> {
        nt_open_access(parent, name, directory, write, create, false)
    }
    fn nt_open_access(
        parent: Option<&BoundFile>,
        name: &std::ffi::OsStr,
        directory: bool,
        write: bool,
        create: bool,
        delete: bool,
    ) -> Result<BoundFile> {
        nt_open_mode(
            parent,
            name,
            directory,
            write,
            create,
            delete,
            AccessProfile::Ordinary,
        )
    }
    #[derive(Clone, Copy, PartialEq)]
    enum AccessProfile {
        Ordinary,
        Placement,
        SharedRead,
    }
    fn nt_open_mode(
        parent: Option<&BoundFile>,
        name: &std::ffi::OsStr,
        directory: bool,
        write: bool,
        create: bool,
        delete: bool,
        profile: AccessProfile,
    ) -> Result<BoundFile> {
        let mut wide: Vec<u16> = name.encode_wide().collect();
        if wide.len() > 16000 {
            return Err(CoreError::new(
                "invalidPath",
                UiMessage::error_core_backend_path_too_long(),
            ));
        }
        let mut unicode = UnicodeString {
            length: (wide.len() * 2) as u16,
            maximum_length: (wide.len() * 2) as u16,
            buffer: wide.as_mut_ptr(),
        };
        let mut attrs = ObjectAttributes {
            length: std::mem::size_of::<ObjectAttributes>() as u32,
            root: parent.map_or(null_mut(), |f| f.as_raw_handle()),
            name: &mut unicode,
            attributes: 0x40,
            security: null_mut(),
            quality: null_mut(),
        };
        let mut result = null_mut();
        let mut io = IoStatus {
            status: 0,
            information: 0,
        };
        let access = if profile == AccessProfile::Placement {
            0
        } else {
            READ
        } | READ_ATTR
            | SYNCHRONIZE
            | if write { WRITE } else { 0 }
            | if delete { DELETE } else { 0 };
        let status = unsafe {
            NtCreateFile(
                &mut result,
                access,
                &mut attrs,
                &mut io,
                null(),
                if directory { 0x10 } else { 0x80 },
                if directory || profile == AccessProfile::Placement {
                    SHARE_RW
                } else if profile == AccessProfile::SharedRead {
                    1
                } else {
                    0
                },
                if create { 2 } else { 1 },
                REPARSE_OPEN | SYNC | if directory { DIRECTORY } else { NON_DIRECTORY },
                null(),
                0,
            )
        };
        if status < 0 {
            return Err(native_error(status));
        }
        // SAFETY: NtCreateFile returned an owned successful kernel handle.
        let file = unsafe { File::from_raw_handle(result) };
        verify(&file, directory)?;
        BoundFile::bind(file, parent, name)
    }
    fn info(file: &File) -> Result<Info> {
        let mut result = Info::default();
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut result) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(result)
    }
    fn verify(file: &File, directory: bool) -> Result<()> {
        let i = info(file)?;
        if i.attrs & BAD_ATTR != 0 {
            return Err(CoreError::new(
                "unsupportedPath",
                UiMessage::error_core_backend_reparse_offline_or_cloud_recall_objects_not_qualified_for_backend(),
            ));
        }
        if (i.attrs & 0x10 != 0) != directory {
            return Err(CoreError::new(
                "unsupportedPath",
                UiMessage::error_core_backend_file_object_has_wrong_type(),
            ));
        }
        if !directory && i.links != 1 {
            return Err(CoreError::new(
                "hardLink",
                UiMessage::error_core_backend_files_with_hard_link_aliases_cannot_written_by_backend(),
            ));
        }
        Ok(())
    }
    pub(crate) fn stamp(file: &File) -> Result<FileStamp> {
        let i = info(file)?;
        Ok(FileStamp {
            identity: FileIdentity::Windows {
                volume: i.volume,
                index: ((i.index_high as u64) << 32) | i.index_low as u64,
                created: ((i.created_high as u64) << 32) | i.created_low as u64,
            },
            modified: ((i.written_high as u64) << 32) | i.written_low as u64,
            size: ((i.size_high as u64) << 32) | i.size_low as u64,
        })
    }
    pub(crate) fn attributes(file: &File) -> Result<EntryMetadata> {
        Ok(EntryMetadata::windows(info(file)?.attrs))
    }
    pub(crate) fn file_coordinates(file: &File) -> Result<PathBuf> {
        let mut buffer = vec![0u16; 32768];
        let n = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                1,
            )
        };
        if n == 0 || n as usize >= buffer.len() {
            return Err(CoreError::new(
                "unsupportedVolume",
                UiMessage::error_core_backend_cannot_obtain_stable_volume_g_u_i_d_path(),
            ));
        }
        Ok(PathBuf::from(String::from_utf16_lossy(
            &buffer[..n as usize],
        )))
    }
    fn directory_entries(
        directory: &File,
        max: usize,
    ) -> Result<(Vec<(std::ffi::OsString, EntryMetadata)>, bool)> {
        // Enumerate the actual protected directory handle. A concurrent reparse
        // attribute edit cannot redirect this enumeration through a path string.
        let mut names = Vec::new();
        let mut buffer = vec![0u64; 8192]; // aligned 64 KiB kernel output buffer
        let mut restart = 1;
        loop {
            let mut io = IoStatus {
                status: 0,
                information: 0,
            };
            let status = unsafe {
                NtQueryDirectoryFile(
                    directory.as_raw_handle(),
                    null_mut(),
                    null(),
                    null_mut(),
                    &mut io,
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 8) as u32,
                    1,
                    0,
                    null(),
                    restart,
                )
            };
            restart = 0;
            if status as u32 == 0x80000006 {
                break;
            }
            if status < 0 {
                return Err(native_error(status));
            }
            if io.information == 0 || io.information > buffer.len() * 8 {
                return Err(CoreError::new(
                    "inventoryUnavailable",
                    UiMessage::error_core_backend_directory_enumeration_returned_incomplete_data(),
                ));
            }
            // SAFETY: the output bytes are initialized; lengths are checked below.
            let bytes =
                unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), io.information) };
            let mut offset = 0;
            loop {
                if offset + 64 > bytes.len() {
                    return Err(CoreError::new(
                        "inventoryUnavailable",
                        UiMessage::error_core_backend_truncated_directory_entry(),
                    ));
                }
                let u32_at = |pos| u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
                let next = u32_at(offset) as usize;
                let attrs = u32_at(offset + 56);
                let name_len = u32_at(offset + 60) as usize;
                if !name_len.is_multiple_of(2) || offset + 64 + name_len > bytes.len() {
                    return Err(CoreError::new(
                        "inventoryUnavailable",
                        UiMessage::error_core_backend_invalid_directory_entry_length(),
                    ));
                }
                let wide = bytes[offset + 64..offset + 64 + name_len]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| u16::from_le_bytes([p[0], p[1]]))
                    .collect::<Vec<_>>();
                let name = std::ffi::OsString::from_wide(&wide);
                if name != "." && name != ".." {
                    if names.len() == max {
                        return Ok((names, false));
                    }
                    names.push((name, EntryMetadata::windows(attrs)));
                }
                if next == 0 {
                    break;
                }
                if next < 64 || !next.is_multiple_of(8) {
                    return Err(CoreError::new(
                        "inventoryUnavailable",
                        UiMessage::error_core_backend_invalid_directory_entry_offset(),
                    ));
                }
                offset = offset.checked_add(next).ok_or_else(|| {
                    CoreError::new(
                        "inventoryUnavailable",
                        UiMessage::error_core_backend_directory_entry_overflow(),
                    )
                })?;
            }
        }
        Ok((names, true))
    }
    pub(crate) struct ParentGuard {
        dirs: Vec<BoundFile>,
        pub name: std::ffi::OsString,
    }
    impl ParentGuard {
        pub fn directory_identity(&self) -> Result<FileIdentity> {
            Ok(stamp(self.dirs.last().unwrap())?.identity)
        }
        pub(crate) fn directory_coordinates(&self) -> Result<PathBuf> {
            file_coordinates(self.dirs.last().unwrap())
        }
        pub fn directory_entries(
            &self,
            max: usize,
        ) -> Result<(Vec<(std::ffi::OsString, EntryMetadata)>, bool)> {
            directory_entries(self.dirs.last().unwrap(), max)
        }
        pub fn acquire(backend: &BackendContext, path: &Path) -> Result<Self> {
            let path = crate::absolute_path(path)?;
            backend.check_path(&path)?;
            let text = path.to_string_lossy();
            if text.starts_with("\\\\") || text.as_bytes().get(1) != Some(&b':') {
                return Err(CoreError::new(
                    "unsupportedVolume",
                    UiMessage::error_core_backend_only_explicit_local_drive_paths_supported(),
                ));
            }
            let drive = PathBuf::from(format!("{}\\", &text[..2]));
            let root_nt = format!("\\??\\{}", drive.display());
            let root = nt_open(None, std::ffi::OsStr::new(&root_nt), true, false, false)?;
            let guid = file_coordinates(&root)?;
            let mut guid_wide: Vec<u16> = guid.as_os_str().encode_wide().collect();
            guid_wide.push(0);
            let mut fsname = [0u16; 32];
            if unsafe {
                GetVolumeInformationW(
                    guid_wide.as_ptr(),
                    null_mut(),
                    0,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    fsname.as_mut_ptr(),
                    32,
                )
            } == 0
            {
                return Err(CoreError::new(
                    "unsupportedVolume",
                    UiMessage::error_core_backend_selected_local_volume_cannot_qualified(),
                ));
            }
            let end = fsname.iter().position(|x| *x == 0).unwrap_or(fsname.len());
            if String::from_utf16_lossy(&fsname[..end]) != "NTFS" {
                return Err(CoreError::new(
                    "unsupportedVolume",
                    UiMessage::error_core_backend_writing_limited_to_local_n_t_f_s_volumes(),
                ));
            }
            let mut dirs = vec![root];
            let relative = path.strip_prefix(&drive).map_err(|_| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_backend_path_outside_protected_volume(),
                )
            })?;
            let mut parts = relative.components().collect::<Vec<_>>();
            let name = parts
                .pop()
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPath",
                        UiMessage::error_core_backend_leaf_path_required(),
                    )
                })?
                .as_os_str()
                .to_os_string();
            for part in parts {
                let child = nt_open(dirs.last(), part.as_os_str(), true, false, false)?;
                if stamp(&child)?.identity.windows_volume()
                    != stamp(&dirs[0])?.identity.windows_volume()
                {
                    return Err(CoreError::new(
                        "unsupportedVolume",
                        UiMessage::error_core_backend_directory_crossed_protected_volume(),
                    ));
                }
                dirs.push(child);
            }
            Ok(Self { dirs, name })
        }
        pub fn open_file(&self, write: bool, create: bool) -> Result<BoundFile> {
            nt_open(self.dirs.last(), &self.name, false, write, create)
        }
        pub(crate) fn open_retiring_file(&self, delete: bool) -> Result<BoundFile> {
            nt_open_access(self.dirs.last(), &self.name, false, false, false, delete)
        }
        pub(crate) fn open_placement_file(&self) -> Result<BoundFile> {
            nt_open_mode(
                self.dirs.last(),
                &self.name,
                false,
                false,
                false,
                false,
                AccessProfile::Placement,
            )
        }
        pub(crate) fn open_read_guard_file(&self) -> Result<BoundFile> {
            nt_open_mode(
                self.dirs.last(),
                &self.name,
                false,
                false,
                false,
                false,
                AccessProfile::SharedRead,
            )
        }
        pub fn create_directory(&self) -> Result<BoundFile> {
            nt_open(self.dirs.last(), &self.name, true, true, true)
        }
        pub fn child_file(directory: &BoundFile, name: &str) -> Result<BoundFile> {
            nt_open(
                Some(directory),
                std::ffi::OsStr::new(name),
                false,
                true,
                true,
            )
        }
        pub fn child_directory(directory: &BoundFile, name: &str) -> Result<BoundFile> {
            nt_open(
                Some(directory),
                std::ffi::OsStr::new(name),
                true,
                true,
                true,
            )
        }
    }

    /// One closure, one handle per physical directory. The source directory is
    /// opened with DELETE from the outset, never upgraded after protection.
    pub(crate) struct NamespaceGuard {
        dirs: std::collections::BTreeMap<PathBuf, BoundFile>,
        source: PathBuf,
        target_parent: PathBuf,
        canonical_keys: bool,
    }
    impl NamespaceGuard {
        pub fn acquire(
            backend: &BackendContext,
            source: &Path,
            target_parent: &Path,
        ) -> Result<Self> {
            let source = crate::absolute_path(source)?;
            let target_parent = crate::absolute_path(target_parent)?;
            let source_text = source.to_string_lossy();
            let target_text = target_parent.to_string_lossy();
            if !source_text[..2].eq_ignore_ascii_case(&target_text[..2]) {
                return Err(CoreError::new(
                    "crossVolume",
                    UiMessage::error_core_backend_same_object_moves_require_one_local_n_t_f_s_volume(),
                ));
            }
            if super::namespace_relative_path(&target_parent, &source)?.is_some() {
                return Err(CoreError::new(
                    "recursiveMove",
                    UiMessage::error_core_backend_directory_cannot_moved_into_itself_or_descendant(
                    ),
                ));
            }
            // Reuse the already qualified volume-root and ancestor opening logic.
            let qualified = ParentGuard::acquire(backend, &source)?;
            drop(qualified);
            let drive = PathBuf::from(format!("{}\\", &source_text[..2]));
            let root_nt = format!("\\??\\{}", drive.display());
            let root = nt_open(None, std::ffi::OsStr::new(&root_nt), true, false, false)?;
            let volume = stamp(&root)?.identity.windows_volume();
            let mut dirs = std::collections::BTreeMap::new();
            dirs.insert(drive.clone(), root);
            for leaf in [&source, &target_parent] {
                let mut current = drive.clone();
                for part in leaf
                    .strip_prefix(&drive)
                    .map_err(|_| {
                        CoreError::new(
                            "invalidPath",
                            UiMessage::error_core_backend_invalid_volume_path(),
                        )
                    })?
                    .components()
                {
                    let next = current.join(part.as_os_str());
                    if !dirs.contains_key(&next) {
                        let file = nt_open_access(
                            dirs.get(&current),
                            part.as_os_str(),
                            true,
                            false,
                            false,
                            next == source,
                        )?;
                        if stamp(&file)?.identity.windows_volume() != volume {
                            return Err(CoreError::new(
                                "crossVolume",
                                UiMessage::error_core_backend_directory_crossed_protected_volume(),
                            ));
                        }
                        dirs.insert(next.clone(), file);
                    }
                    current = next;
                }
            }
            Ok(Self {
                dirs,
                source,
                target_parent,
                canonical_keys: false,
            })
        }
        pub fn source_identity(&self) -> Result<FileIdentity> {
            Ok(stamp(self.dirs.get(&self.source).unwrap())?.identity)
        }
        pub(crate) fn source_coordinates(&self) -> Result<PathBuf> {
            file_coordinates(self.dirs.get(&self.source).unwrap())
        }
        pub fn parent_identity(&self) -> Result<FileIdentity> {
            Ok(stamp(self.dirs.get(&self.target_parent).unwrap())?.identity)
        }
        pub fn open_source_file(&self, name: &str, delete: bool) -> Result<BoundFile> {
            nt_open_access(
                self.dirs.get(&self.source),
                std::ffi::OsStr::new(name),
                false,
                false,
                false,
                delete,
            )
        }
        /// Descendants are resolved from the retained original directory handle,
        /// including after its move. Never reopen the old pathname as authority.
        fn source_directory(&mut self, relative: &Path) -> Result<&BoundFile> {
            let mut path = self.source.clone();
            for part in relative.components() {
                let std::path::Component::Normal(name) = part else {
                    return Err(CoreError::new(
                        "invalidPath",
                        UiMessage::error_core_backend_expected_relative_source_descendant(),
                    ));
                };
                let next = if self.canonical_keys {
                    namespace_key(&path.join(name))?
                } else {
                    path.join(name)
                };
                if !self.dirs.contains_key(&next) {
                    let directory = nt_open(self.dirs.get(&path), name, true, false, false)?;
                    self.dirs.insert(next.clone(), directory);
                }
                if namespace_path_compare(
                    &file_coordinates(&self.dirs[&next])?,
                    &file_coordinates(&self.dirs[&path])?.join(name),
                )? != std::cmp::Ordering::Equal
                {
                    return Err(CoreError::new(
                        "thirdState",
                        weftext_messages::UiMessage::error_core_backend_source_descendant_placement_changed(),
                    ));
                }
                path = next;
            }
            Ok(&self.dirs[&path])
        }
        pub fn source_directory_snapshot(
            &mut self,
            relative: &Path,
            max: usize,
        ) -> Result<DirectorySnapshot> {
            let directory = self.source_directory(relative)?;
            let (entries, complete) = directory_entries(directory, max)?;
            Ok((stamp(directory)?.identity, entries, complete))
        }
        pub fn open_source_descendant(&mut self, relative: &Path) -> Result<BoundFile> {
            self.open_source_output(relative, false, false)
        }
        pub(crate) fn open_source_placement(&mut self, relative: &Path) -> Result<BoundFile> {
            let name = relative.file_name().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_backend_missing_placement_filename(),
                )
            })?;
            let directory = self.source_directory(relative.parent().unwrap())?;
            nt_open_mode(
                Some(directory),
                name,
                false,
                false,
                false,
                false,
                AccessProfile::Placement,
            )
        }
        pub(crate) fn source_parent_identity(&mut self, relative: &Path) -> Result<FileIdentity> {
            Ok(stamp(self.source_directory(relative.parent().unwrap())?)?.identity)
        }
        pub fn open_source_output(
            &mut self,
            relative: &Path,
            write: bool,
            create: bool,
        ) -> Result<BoundFile> {
            let name = relative.file_name().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_backend_missing_descendant_filename(),
                )
            })?;
            let directory = self.source_directory(relative.parent().unwrap())?;
            nt_open(Some(directory), name, false, write, create)
        }
        pub(crate) fn open_source_retiring(
            &mut self,
            relative: &Path,
            delete: bool,
        ) -> Result<BoundFile> {
            let name = relative.file_name().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_backend_missing_retirement_filename(),
                )
            })?;
            let directory = self.source_directory(relative.parent().unwrap())?;
            nt_open_access(Some(directory), name, false, false, false, delete)
        }
        pub fn release_source_descendants(&mut self) {
            self.dirs
                .retain(|path, _| path == &self.source || !path.starts_with(&self.source));
        }
        pub fn rename_leaf(&self, file: &BoundFile, name: &str) -> Result<()> {
            // A basename-only rename uses the file's current directory. Supplying
            // our DELETE directory guard as RootDirectory would make Windows
            // reopen that same directory and conflict with the retained guard.
            rename_handle(file, None, name)
        }
        pub fn move_source(&self, name: &str) -> Result<()> {
            rename_handle(
                self.dirs.get(&self.source).unwrap(),
                Some(self.dirs.get(&self.target_parent).unwrap()),
                name,
            )
        }
        pub fn open_moved_file(&self, name: &str) -> Result<BoundFile> {
            nt_open(
                self.dirs.get(&self.source),
                std::ffi::OsStr::new(name),
                false,
                false,
                false,
            )
        }
    }
    fn namespace_key(path: &Path) -> Result<PathBuf> {
        let mut key = PathBuf::new();
        for component in path.components() {
            let text = component.as_os_str().to_str().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_backend_non_u_t_f8_namespace_component(),
                )
            })?;
            key.push(text.to_ascii_lowercase());
        }
        Ok(key)
    }
    /// All moving roots share one ancestor closure. Every moving directory has
    /// DELETE access from the outset; unrelated roots are never reopened while
    /// another participant denies delete on the same ancestor.
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
        /// Retain existing ancestors without inventing identities for absent
        /// create-only directories. Only this exact planned set may be absent.
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
            if (participants.is_empty() && ancestors.is_empty()) || participants.len() > 5000 {
                return Err(CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_backend_invalid_namespace_participant_count(),
                ));
            }
            let actual = participants
                .iter()
                .map(|(source, parent)| {
                    Ok((crate::absolute_path(source)?, crate::absolute_path(parent)?))
                })
                .collect::<Result<Vec<_>>>()?;
            let sources = actual
                .iter()
                .map(|(source, _)| namespace_key(source))
                .collect::<Result<Vec<_>>>()?;
            let parents = actual
                .iter()
                .map(|(_, parent)| namespace_key(parent))
                .collect::<Result<Vec<_>>>()?;
            if planned.len() > 5000 || ancestors.len() > 5000 {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_backend_too_many_planned_directories(),
                ));
            }
            let planned = planned
                .iter()
                .map(|path| crate::absolute_path(path))
                .collect::<Result<Vec<_>>>()?;
            let planned_keys = planned
                .iter()
                .map(|path| namespace_key(path))
                .collect::<Result<std::collections::BTreeSet<_>>>()?;
            let ancestors = ancestors
                .iter()
                .map(|path| crate::absolute_path(path))
                .collect::<Result<Vec<_>>>()?;
            for parent in &ancestors {
                drop(ParentGuard::acquire(
                    backend,
                    &parent.join(".original-placement-parent"),
                )?);
            }
            let text = actual
                .first()
                .map(|(source, _)| source)
                .or_else(|| ancestors.first())
                .unwrap()
                .to_string_lossy();
            if text.starts_with("\\\\") || text.as_bytes().get(1) != Some(&b':') {
                return Err(CoreError::new(
                    "unsupportedVolume",
                    UiMessage::error_core_backend_explicit_local_n_t_f_s_paths_required(),
                ));
            }
            for (index, (source, parent)) in actual.iter().enumerate() {
                let source_text = source.to_string_lossy();
                let parent_text = parent.to_string_lossy();
                if !source_text
                    .get(..2)
                    .is_some_and(|drive| drive.eq_ignore_ascii_case(&text[..2]))
                    || !parent_text
                        .get(..2)
                        .is_some_and(|drive| drive.eq_ignore_ascii_case(&text[..2]))
                {
                    return Err(CoreError::new(
                        "crossVolume",
                        UiMessage::error_core_backend_compound_placement_requires_one_n_t_f_s_volume(),
                    ));
                }
                for (other, root) in sources.iter().enumerate() {
                    if !forest
                        && (parents[index].starts_with(root)
                            || (index != other && sources[index].starts_with(root)))
                    {
                        return Err(CoreError::new(
                            "recursiveMove",
                            UiMessage::error_core_backend_compound_roots_and_target_parents_must_independent(),
                        ));
                    }
                }
                // Establish the existing NTFS qualification without retaining a
                // second closure that would block the union's DELETE handles.
                drop(ParentGuard::acquire(backend, source)?);
                let mut anchor = parent.as_path();
                while planned_keys.contains(&namespace_key(anchor)?) {
                    anchor = anchor.parent().ok_or_else(|| {
                        CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_backend_planned_directory_lacks_anchor(),
                        )
                    })?;
                }
                drop(ParentGuard::acquire(
                    backend,
                    &anchor.join(".namespace-prospective"),
                )?);
            }
            let drive = PathBuf::from(format!("{}\\", &text[..2]));
            let drive_key = namespace_key(&drive)?;
            let root_nt = format!("\\??\\{}", drive.display());
            let root = nt_open(None, std::ffi::OsStr::new(&root_nt), true, false, false)?;
            let volume = stamp(&root)?.identity.windows_volume();
            let mut dirs = std::collections::BTreeMap::new();
            dirs.insert(drive_key.clone(), root);
            for leaf in actual
                .iter()
                .flat_map(|(source, parent)| [source, parent])
                .chain(&planned)
                .chain(&ancestors)
            {
                let mut current = drive_key.clone();
                for part in leaf.components().skip(2) {
                    let next = namespace_key(&current.join(part.as_os_str()))?;
                    if !dirs.contains_key(&next) {
                        let opened = nt_open_access(
                            dirs.get(&current),
                            part.as_os_str(),
                            true,
                            false,
                            false,
                            sources.contains(&next),
                        );
                        let file = match opened {
                            Err(error)
                                if error.code == "missing" && planned_keys.contains(&next) =>
                            {
                                break;
                            }
                            other => other?,
                        };
                        if stamp(&file)?.identity.windows_volume() != volume {
                            return Err(CoreError::new(
                                "crossVolume",
                                UiMessage::error_core_backend_compound_directory_crossed_protected_volume(),
                            ));
                        }
                        dirs.insert(next.clone(), file);
                    }
                    current = next;
                }
            }
            Ok(Self {
                inner: NamespaceGuard {
                    dirs,
                    source: sources
                        .first()
                        .cloned()
                        .unwrap_or_else(|| drive_key.clone()),
                    target_parent: parents
                        .first()
                        .cloned()
                        .unwrap_or_else(|| drive_key.clone()),
                    canonical_keys: true,
                },
                sources,
                parents,
            })
        }
        pub(crate) fn select(&mut self, index: usize) -> Result<&mut NamespaceGuard> {
            self.inner.source = self
                .sources
                .get(index)
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_backend_missing_namespace_participant(),
                    )
                })?
                .clone();
            self.inner.target_parent = self.parents[index].clone();
            Ok(&mut self.inner)
        }
        pub(crate) fn release_source_descendants(&mut self, index: usize) -> Result<()> {
            let source = self.sources.get(index).ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_backend_missing_source_root(),
                )
            })?;
            self.inner
                .dirs
                .retain(|path, _| self.sources.contains(path) || !path.starts_with(source));
            Ok(())
        }
        /// A final source that is also another participant's target parent no
        /// longer needs DELETE access. Retain continuous namespace protection
        /// through its original canonical file while changing the directory
        /// guard; Windows otherwise conflicts with its own target-parent open.
        pub(crate) fn finish_source_move(
            &mut self,
            index: usize,
            name: &str,
            canonical: &str,
            canonical_identity: &FileIdentity,
        ) -> Result<()> {
            self.finish_source_move_with(index, name, canonical, canonical_identity, || {})
        }
        fn finish_source_move_with(
            &mut self,
            index: usize,
            name: &str,
            canonical: &str,
            canonical_identity: &FileIdentity,
            during_handoff: impl FnOnce(),
        ) -> Result<()> {
            let source = self
                .sources
                .get(index)
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_backend_missing_final_source_root(),
                    )
                })?
                .clone();
            if !self.parents.contains(&source) {
                return Ok(());
            }
            for component in [name, canonical] {
                if component.is_empty()
                    || matches!(component, "." | "..")
                    || component.contains(['/', '\\', ':'])
                {
                    return Err(CoreError::new(
                        "invalidPath",
                        UiMessage::error_core_backend_final_guard_needs_basenames(),
                    ));
                }
            }
            let parent = self.parents[index].clone();
            if source == parent || !self.inner.dirs.contains_key(&parent) {
                return Err(CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_backend_final_parent_not_retained(),
                ));
            }
            let original = self.inner.dirs.get(&source).ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_backend_original_final_source_not_retained(),
                )
            })?;
            let identity = stamp(original)?.identity;
            let witness = nt_open(
                Some(original),
                std::ffi::OsStr::new(canonical),
                false,
                false,
                false,
            )?;
            if &stamp(&witness)?.identity != canonical_identity {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_backend_final_source_canonical_witness_not_original_file(
                    ),
                ));
            }
            // The ordinary READ/share0 child prevents NTFS from renaming its
            // parent during this handoff. All ancestor guards remain retained.
            // If reacquisition fails, the caller must stop without more writes.
            drop(self.inner.dirs.remove(&source));
            during_handoff();
            let directory = nt_open(
                self.inner.dirs.get(&parent),
                std::ffi::OsStr::new(name),
                true,
                false,
                false,
            )?;
            if stamp(&directory)?.identity != identity {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_backend_final_source_directory_changed_during_guard_handoff(),
                ));
            }
            // Preserve capture-time keys even though the directory has moved.
            self.inner.dirs.insert(source, directory);
            drop(witness);
            Ok(())
        }
        pub(crate) fn move_source_to(&self, index: usize, target: &Path) -> Result<()> {
            let source = self
                .sources
                .get(index)
                .and_then(|key| self.inner.dirs.get(key))
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_backend_original_moving_root_not_retained(),
                    )
                })?;
            self.move_file_to(source, target)
        }
        pub(crate) fn move_file_to(&self, file: &BoundFile, target: &Path) -> Result<()> {
            let parent = target.parent().ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_backend_moving_target_has_no_parent(),
                )
            })?;
            let parent = self
                .inner
                .dirs
                .get(&namespace_key(parent)?)
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_backend_moving_target_parent_not_retained(),
                    )
                })?;
            rename_handle(
                file,
                Some(parent),
                target
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidPath",
                            UiMessage::error_core_backend_moving_target_has_no_u_t_f8_name(),
                        )
                    })?,
            )
        }
        pub(crate) fn source_parent_identity(&self, index: usize) -> Result<FileIdentity> {
            let parent = self
                .sources
                .get(index)
                .and_then(|source| source.parent())
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_backend_missing_source_parent(),
                    )
                })?;
            Ok(stamp(self.inner.dirs.get(parent).ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_backend_source_parent_not_retained(),
                )
            })?)?
            .identity)
        }
        pub(crate) fn directory_identity(&self, path: &Path) -> Result<Option<FileIdentity>> {
            self.inner
                .dirs
                .get(&namespace_key(path)?)
                .map(|file| Ok(stamp(file)?.identity))
                .transpose()
        }
        pub(crate) fn directory_file(&self, path: &Path) -> Result<&BoundFile> {
            self.inner.dirs.get(&namespace_key(path)?).ok_or_else(|| {
                CoreError::new(
                    "missing",
                    UiMessage::error_core_backend_planned_directory_has_not_been_materialized(),
                )
            })
        }
        pub(crate) fn retain_directory(&mut self, path: &Path) -> Result<&BoundFile> {
            let key = namespace_key(path)?;
            if !self.inner.dirs.contains_key(&key) {
                let parent_key = key
                    .parent()
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_backend_retained_directory_lacks_parent(),
                        )
                    })?
                    .to_path_buf();
                self.retain_directory(&parent_key)?;
                let parent = key
                    .parent()
                    .and_then(|parent| self.inner.dirs.get(parent))
                    .ok_or_else(|| {
                        CoreError::new(
                            "missing",
                            UiMessage::error_core_backend_directory_parent_not_protected(),
                        )
                    })?;
                let file = nt_open(Some(parent), path.file_name().unwrap(), true, false, false)?;
                self.inner.dirs.insert(key.clone(), file);
            }
            Ok(&self.inner.dirs[&key])
        }
        /// Resolve again only beneath a retained original ancestor handle. Its
        /// pathname key remains the capture-time coordinate after that ancestor
        /// moves; callers compare the result to the frozen parent identity.
        pub(crate) fn retain_target_parent(&mut self, index: usize) -> Result<()> {
            let parent = self
                .parents
                .get(index)
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_backend_missing_namespace_parent(),
                    )
                })?
                .clone();
            self.retain_directory(&parent)?;
            Ok(())
        }
        pub(crate) fn target_parent_identity(&self, index: usize) -> Result<Option<FileIdentity>> {
            self.directory_identity(self.parents.get(index).ok_or_else(|| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_backend_missing_namespace_parent(),
                )
            })?)
        }
        pub(crate) fn create_directory(&mut self, path: &Path) -> Result<&BoundFile> {
            let key = namespace_key(path)?;
            if self.inner.dirs.contains_key(&key) {
                return Err(CoreError::new(
                    "exists",
                    UiMessage::error_core_backend_planned_directory_already_occupied(),
                ));
            }
            let parent = key
                .parent()
                .and_then(|parent| self.inner.dirs.get(parent))
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_backend_created_directory_parent_not_protected(),
                    )
                })?;
            let file = ParentGuard::child_directory(
                parent,
                path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidPath",
                            UiMessage::error_core_backend_invalid_directory_name(),
                        )
                    })?,
            )?;
            self.inner.dirs.insert(key.clone(), file);
            Ok(&self.inner.dirs[&key])
        }
        pub(crate) fn open_output(
            &self,
            path: &Path,
            write: bool,
            create: bool,
        ) -> Result<BoundFile> {
            let key = namespace_key(path)?;
            let parent = key
                .parent()
                .and_then(|parent| self.inner.dirs.get(parent))
                .ok_or_else(|| {
                    CoreError::new(
                        "missing",
                        UiMessage::error_core_backend_output_parent_not_protected(),
                    )
                })?;
            nt_open(
                Some(parent),
                path.file_name().unwrap(),
                false,
                write,
                create,
            )
        }
        pub(crate) fn directory_snapshot(
            &self,
            path: &Path,
            max: usize,
        ) -> Result<DirectorySnapshot> {
            let directory = self.directory_file(path)?;
            let (entries, complete) = directory_entries(directory, max)?;
            Ok((stamp(directory)?.identity, entries, complete))
        }
    }
    fn rename_handle(file: &BoundFile, parent: Option<&BoundFile>, name: &str) -> Result<()> {
        #[repr(C)]
        struct RenameInfo {
            replace: u8,
            parent: Handle,
            length: u32,
            name: [u16; 1],
        }
        let wide = name.encode_utf16().collect::<Vec<_>>();
        let offset = std::mem::offset_of!(RenameInfo, name);
        let length = (offset + wide.len() * 2).max(std::mem::size_of::<RenameInfo>());
        let mut data = vec![0usize; length.div_ceil(std::mem::size_of::<usize>())];
        let ptr = data.as_mut_ptr().cast::<RenameInfo>();
        // SAFETY: aligned allocation is large enough for the fixed header and UTF-16 name.
        unsafe {
            (*ptr).replace = 0;
            (*ptr).parent = parent.map_or(null_mut(), |parent| parent.as_raw_handle());
            (*ptr).length = (wide.len() * 2) as u32;
            std::ptr::copy_nonoverlapping(
                wide.as_ptr(),
                (ptr.cast::<u8>()).add(offset).cast::<u16>(),
                wide.len(),
            );
        }
        let mut io = IoStatus {
            status: 0,
            information: 0,
        };
        let status = unsafe {
            NtSetInformationFile(
                file.binding.file.as_raw_handle(),
                &mut io,
                ptr.cast(),
                length as u32,
                10,
            )
        };
        if status < 0 {
            return Err(native_error(status));
        }
        *file.binding.name.lock().map_err(|_| {
            CoreError::new(
                "busy",
                weftext_messages::UiMessage::error_core_backend_entry_binding_unavailable(),
            )
        })? = std::ffi::OsString::from(name);
        Ok(())
    }

    #[cfg(test)]
    mod final_source_guard_tests {
        use super::*;
        use std::fs;
        use std::os::windows::fs::OpenOptionsExt;

        #[test]
        fn source_presence_uses_retained_delete_closure_and_keeps_external_rename_blocked() {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("Original");
            let target = temp.path().join("Target");
            fs::create_dir(&source).unwrap();
            fs::write(source.join("Original.typ"), "original").unwrap();
            let backend = BackendContext::local().unwrap();
            let mut namespace = NamespaceGuard::acquire(&backend, &source, temp.path()).unwrap();
            assert!(
                namespace
                    .source_entry_exists(Path::new("Original.typ"))
                    .unwrap()
            );
            assert!(
                !namespace
                    .source_entry_exists(Path::new("Renamed.typ"))
                    .unwrap()
            );
            assert!(fs::rename(&source, &target).is_err());
            let canonical = namespace.open_source_file("Original.typ", true).unwrap();
            namespace.rename_leaf(&canonical, "Renamed.typ").unwrap();
            drop(canonical);
            assert!(
                !namespace
                    .source_entry_exists(Path::new("Original.typ"))
                    .unwrap()
            );
            assert!(
                namespace
                    .source_entry_exists(Path::new("Renamed.typ"))
                    .unwrap()
            );
            assert!(fs::rename(&source, &target).is_err());
            namespace.move_source("Target").unwrap();
            assert!(
                namespace
                    .source_entry_exists(Path::new("Renamed.typ"))
                    .unwrap()
            );
            assert_eq!(
                fs::read_to_string(target.join("Renamed.typ")).unwrap(),
                "original"
            );
        }

        struct Fixture {
            union: MultiNamespaceGuard,
            final_parent: PathBuf,
            child: PathBuf,
            canonical_identity: FileIdentity,
            parent_identity: FileIdentity,
            _temp: tempfile::TempDir,
        }
        impl Fixture {
            fn new() -> Self {
                let temp = tempfile::tempdir().unwrap();
                let parent = temp.path().join("Parent");
                let child = temp.path().join("Child");
                let container = temp.path().join("Container");
                for path in [&parent, &child, &container] {
                    fs::create_dir(path).unwrap();
                }
                fs::write(parent.join("Parent.typ"), "original parent").unwrap();
                fs::write(child.join("Child.typ"), "original child").unwrap();
                let canonical_identity = stamp(&File::open(parent.join("Parent.typ")).unwrap())
                    .unwrap()
                    .identity;
                let mut union = MultiNamespaceGuard::acquire_forest_with_ancestors(
                    &BackendContext::local().unwrap(),
                    &[(&parent, &container), (&child, &parent)],
                    &[],
                    &[],
                )
                .unwrap();
                let namespace = union.select(0).unwrap();
                let parent_identity = namespace.source_identity().unwrap();
                namespace.move_source("Placed").unwrap();
                Self {
                    _temp: temp,
                    union,
                    final_parent: container.join("Placed"),
                    child,
                    canonical_identity,
                    parent_identity,
                }
            }
        }

        #[test]
        fn final_parent_handoff_keeps_continuous_protection_and_allows_nested_no_replace_move() {
            let mut f = Fixture::new();
            let external = f.final_parent.with_file_name("External");
            fs::create_dir(f.final_parent.join("Occupied")).unwrap();
            fs::write(f.final_parent.join("Occupied/foreign.raw"), "foreign").unwrap();
            f.union
                .finish_source_move_with(0, "Placed", "Parent.typ", &f.canonical_identity, || {
                    assert!(fs::rename(&f.final_parent, &external).is_err());
                    assert!(
                        fs::rename(
                            f.final_parent.join("Parent.typ"),
                            f.final_parent.join("External.typ"),
                        )
                        .is_err()
                    );
                    assert!(f.final_parent.is_dir() && !external.exists());
                })
                .unwrap();
            assert_eq!(
                f.union.select(0).unwrap().source_identity().unwrap(),
                f.parent_identity
            );
            assert!(fs::rename(&f.final_parent, &external).is_err());
            assert!(f.union.select(1).unwrap().move_source("Occupied").is_err());
            assert_eq!(
                fs::read_to_string(f.final_parent.join("Occupied/foreign.raw")).unwrap(),
                "foreign"
            );
            assert_eq!(
                fs::read_to_string(f.child.join("Child.typ")).unwrap(),
                "original child"
            );
            f.union.select(1).unwrap().move_source("Child").unwrap();
            assert!(!f.child.exists());
            assert_eq!(
                fs::read_to_string(f.final_parent.join("Child/Child.typ")).unwrap(),
                "original child"
            );
            assert_eq!(
                stamp(&File::open(f.final_parent.join("Parent.typ")).unwrap())
                    .unwrap()
                    .identity,
                f.canonical_identity
            );
        }

        #[test]
        fn final_parent_handoff_refuses_a_foreign_canonical_before_releasing_directory() {
            let mut f = Fixture::new();
            let wrong = stamp(&File::open(f.child.join("Child.typ")).unwrap())
                .unwrap()
                .identity;
            assert_eq!(
                f.union
                    .finish_source_move(0, "Placed", "Parent.typ", &wrong)
                    .unwrap_err()
                    .code,
                "thirdState"
            );
            assert_eq!(
                f.union.select(0).unwrap().source_identity().unwrap(),
                f.parent_identity
            );
            assert!(
                fs::rename(&f.final_parent, f.final_parent.with_file_name("External")).is_err()
            );
            assert!(f.child.is_dir());
        }

        #[test]
        fn final_parent_handoff_busy_reacquisition_stops_before_child_placement() {
            let mut f = Fixture::new();
            let mut external_guard = None;
            let error = f
                .union
                .finish_source_move_with(0, "Placed", "Parent.typ", &f.canonical_identity, || {
                    external_guard = Some(
                        fs::OpenOptions::new()
                            .access_mode(READ | READ_ATTR | SYNCHRONIZE | DELETE)
                            .share_mode(SHARE_RW)
                            .custom_flags(0x02000000 | 0x00200000)
                            .open(&f.final_parent)
                            .unwrap(),
                    );
                    assert!(
                        fs::rename(&f.final_parent, f.final_parent.with_file_name("External"))
                            .is_err()
                    );
                })
                .unwrap_err();
            assert_eq!(error.code, "busy");
            assert!(f.child.is_dir());
            assert!(!f.final_parent.join("Child").exists());
            assert_eq!(
                fs::read_to_string(f.child.join("Child.typ")).unwrap(),
                "original child"
            );
            assert_eq!(
                stamp(external_guard.as_ref().unwrap()).unwrap().identity,
                f.parent_identity
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[path = "backend/linux.rs"]
mod platform;

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    use super::*;
    pub(crate) struct ParentGuard;
    impl ParentGuard {
        pub fn directory_entries(
            &self,
            _: usize,
        ) -> Result<(Vec<(std::ffi::OsString, EntryMetadata)>, bool)> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn acquire(backend: &BackendContext, _: &Path) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_m_v_p_write_backend_requires_windows_local_n_t_f_s(),
            ))
        }
        pub fn open_file(&self, _: bool, _: bool) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn open_retiring_file(&self, _: bool) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn open_placement_file(&self) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn open_read_guard_file(&self) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn create_directory(&self) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn child_file(_: &BoundFile, _: &str) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn child_directory(_: &BoundFile, _: &str) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn directory_coordinates(&self) -> Result<PathBuf> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn directory_identity(&self) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
    }
    pub(crate) struct NamespaceGuard;
    impl NamespaceGuard {
        pub fn acquire(backend: &BackendContext, _: &Path, _: &Path) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn source_identity(&self) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn source_coordinates(&self) -> Result<PathBuf> {
            Err(CoreError::new(
                "unsupportedPlatform",
                weftext_messages::UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn parent_identity(&self) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn open_source_file(&self, _: &str, _: bool) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn source_directory_snapshot(
            &mut self,
            _: &Path,
            _: usize,
        ) -> Result<DirectorySnapshot> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn open_source_descendant(&mut self, _: &Path) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn open_source_placement(&mut self, _: &Path) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn source_parent_identity(&mut self, _: &Path) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn open_source_output(&mut self, _: &Path, _: bool, _: bool) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn open_source_retiring(&mut self, _: &Path, _: bool) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn release_source_descendants(&mut self) {}
        pub fn rename_leaf(&self, _: &BoundFile, _: &str) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn move_source(&self, _: &str) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub fn open_moved_file(&self, _: &str) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
    }
    pub(crate) struct MultiNamespaceGuard;
    impl MultiNamespaceGuard {
        pub(crate) fn acquire(backend: &BackendContext, _: &[(&Path, &Path)]) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn acquire_planned(
            backend: &BackendContext,
            _: &[(&Path, &Path)],
            _: &[PathBuf],
        ) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn acquire_with_ancestors(
            backend: &BackendContext,
            _: &[(&Path, &Path)],
            _: &[PathBuf],
            _: &[PathBuf],
        ) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn acquire_forest_with_ancestors(
            backend: &BackendContext,
            _: &[(&Path, &Path)],
            _: &[PathBuf],
            _: &[PathBuf],
        ) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn acquire_anchors(
            backend: &BackendContext,
            _: &[PathBuf],
            _: &[PathBuf],
        ) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn move_file_to(&self, _: &BoundFile, _: &Path) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn release_source_descendants(&mut self, _: usize) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn finish_source_move(
            &mut self,
            _: usize,
            _: &str,
            _: &str,
            _: &FileIdentity,
        ) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn move_source_to(&self, _: usize, _: &Path) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn directory_identity(&self, _: &Path) -> Result<Option<FileIdentity>> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn directory_file(&self, _: &Path) -> Result<&BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn retain_directory(&mut self, _: &Path) -> Result<&BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn retain_target_parent(&mut self, _: usize) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn create_directory(&mut self, _: &Path) -> Result<&BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn target_parent_identity(&self, _: usize) -> Result<Option<FileIdentity>> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn open_output(&self, _: &Path, _: bool, _: bool) -> Result<BoundFile> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn directory_snapshot(&self, _: &Path, _: usize) -> Result<DirectorySnapshot> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn select(&mut self, _: usize) -> Result<&mut NamespaceGuard> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
        pub(crate) fn source_parent_identity(&self, _: usize) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                UiMessage::error_core_backend_windows_backend_required(),
            ))
        }
    }
    pub(crate) fn file_coordinates(_: &File) -> Result<PathBuf> {
        Err(CoreError::new(
            "unsupportedPlatform",
            UiMessage::error_core_backend_windows_backend_required(),
        ))
    }
    pub(crate) fn stamp(_: &File) -> Result<FileStamp> {
        Err(CoreError::new(
            "unsupportedPlatform",
            UiMessage::error_core_backend_windows_backend_required(),
        ))
    }
    pub(crate) fn attributes(_: &File) -> Result<EntryMetadata> {
        Err(CoreError::new(
            "unsupportedPlatform",
            UiMessage::error_core_backend_windows_backend_required(),
        ))
    }
    pub(crate) fn publish_private_create(
        backend: &BackendContext,
        _: &Path,
        _: &Path,
    ) -> Result<()> {
        Err(CoreError::new(
            "unsupportedPlatform",
            UiMessage::error_core_backend_windows_backend_required(),
        ))
    }
    pub(crate) fn publish_private_replace(
        backend: &BackendContext,
        _: &Path,
        _: &Path,
    ) -> Result<()> {
        Err(CoreError::new(
            "unsupportedPlatform",
            UiMessage::error_core_backend_windows_backend_required(),
        ))
    }
}
pub(crate) use platform::{
    MultiNamespaceGuard, NamespaceGuard, ParentGuard, attributes, file_coordinates,
    publish_private_create, publish_private_replace, stamp,
};
/// Presence is qualified from the complete retained parent inventory. Unrelated
/// sibling entries do not become authority for this selected entry.
fn qualified_entry_exists(
    entries: Vec<(std::ffi::OsString, EntryMetadata)>,
    complete: bool,
    name: &std::ffi::OsStr,
) -> Result<bool> {
    if !complete {
        return Err(CoreError::new(
            "budget",
            weftext_messages::UiMessage::error_core_backend_private_namespace_budget(),
        ));
    }
    for (actual, metadata) in entries {
        if namespace_path_compare(Path::new(&actual), Path::new(name))? == std::cmp::Ordering::Equal
        {
            if metadata.is_unsupported() {
                return Err(CoreError::new(
                    "unsupportedPath",
                    weftext_messages::UiMessage::error_core_backend_unsupported_private_entry(),
                ));
            }
            return Ok(true);
        }
    }
    Ok(false)
}
impl NamespaceGuard {
    /// Use the already retained source closure, including its original parent
    /// handles. Reopening its pathname conflicts with Windows DELETE ownership
    /// and could substitute a different source after a lawful move.
    pub(crate) fn source_entry_exists(&mut self, relative: &Path) -> Result<bool> {
        let name = relative.file_name().ok_or_else(|| {
            CoreError::new(
                "invalidPath",
                weftext_messages::UiMessage::error_core_backend_entry_leaf_required(),
            )
        })?;
        let (_, entries, complete) = self
            .source_directory_snapshot(relative.parent().unwrap_or_else(|| Path::new("")), 5000)?;
        qualified_entry_exists(entries, complete, name)
    }
}
pub(crate) fn namespace_relative_path(path: &Path, prefix: &Path) -> Result<Option<PathBuf>> {
    let mut path = path.components();
    for expected in prefix.components() {
        let Some(actual) = path.next() else {
            return Ok(None);
        };
        #[cfg(windows)]
        let equal =
            platform::namespace_component_compare(actual.as_os_str(), expected.as_os_str())?
                == std::cmp::Ordering::Equal;
        #[cfg(not(windows))]
        let equal = actual == expected;
        if !equal {
            return Ok(None);
        }
    }
    Ok(Some(path.collect()))
}
pub(crate) fn namespace_path_compare(left: &Path, right: &Path) -> Result<std::cmp::Ordering> {
    let mut left = left.components();
    let mut right = right.components();
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ok(std::cmp::Ordering::Equal),
            (None, Some(_)) => return Ok(std::cmp::Ordering::Less),
            (Some(_), None) => return Ok(std::cmp::Ordering::Greater),
            (Some(left), Some(right)) => {
                #[cfg(windows)]
                let order =
                    platform::namespace_component_compare(left.as_os_str(), right.as_os_str())?;
                #[cfg(not(windows))]
                let order = left.as_os_str().cmp(right.as_os_str());
                if order != std::cmp::Ordering::Equal {
                    return Ok(order);
                }
            }
        }
    }
}
pub(crate) fn same_directory(backend: &BackendContext, a: &Path, b: &Path) -> Result<bool> {
    let a = ParentGuard::acquire(backend, &a.join(".directory-identity-prospective"))?;
    let b = ParentGuard::acquire(backend, &b.join(".directory-identity-prospective"))?;
    Ok(a.directory_identity()? == b.directory_identity()?
        && a.directory_coordinates()? == b.directory_coordinates()?)
}
