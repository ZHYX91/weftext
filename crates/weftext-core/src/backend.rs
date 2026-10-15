//! Narrow Windows NTFS backend. All user writes retain the parent closure and the
//! same share-zero leaf handle through verification, write, flush and decision.
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileIdentity {
    pub volume: u32,
    pub index: u64,
    pub created: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FileStamp {
    pub identity: FileIdentity,
    pub modified: u64,
    pub size: u64,
}

type DirectorySnapshot = (FileIdentity, Vec<(std::ffi::OsString, u32)>, bool);

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
    fn native_error(status: i32) -> CoreError {
        let code = unsafe { RtlNtStatusToDosError(status) };
        let kind = match code {
            32 | 33 | 1224 => "busy",
            80 | 183 => "exists",
            2 | 3 => "missing",
            5 => "permissionDenied",
            _ => "io",
        };
        CoreError::new(
            kind,
            format!(
                "Windows file protection failed: {}",
                std::io::Error::from_raw_os_error(code as i32)
            ),
        )
    }
    pub(crate) fn publish_private_create(from: &Path, to: &Path) -> Result<()> {
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
        // MOVEFILE_WRITE_THROUGH, without REPLACE_EXISTING or COPY_ALLOWED.
        if unsafe { MoveFileExW(from_wide.as_ptr(), to_wide.as_ptr(), 8) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    fn nt_open(
        parent: Option<&File>,
        name: &std::ffi::OsStr,
        directory: bool,
        write: bool,
        create: bool,
    ) -> Result<File> {
        nt_open_access(parent, name, directory, write, create, false)
    }
    fn nt_open_access(
        parent: Option<&File>,
        name: &std::ffi::OsStr,
        directory: bool,
        write: bool,
        create: bool,
        delete: bool,
    ) -> Result<File> {
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
        parent: Option<&File>,
        name: &std::ffi::OsStr,
        directory: bool,
        write: bool,
        create: bool,
        delete: bool,
        profile: AccessProfile,
    ) -> Result<File> {
        let mut wide: Vec<u16> = name.encode_wide().collect();
        if wide.len() > 16000 {
            return Err(CoreError::new("invalidPath", "Path is too long"));
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
        Ok(file)
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
                "Reparse, offline or cloud recall objects are not qualified for this backend",
            ));
        }
        if (i.attrs & 0x10 != 0) != directory {
            return Err(CoreError::new(
                "unsupportedPath",
                "File object has the wrong type",
            ));
        }
        if !directory && i.links != 1 {
            return Err(CoreError::new(
                "hardLink",
                "Files with hard-link aliases cannot be written by this backend",
            ));
        }
        Ok(())
    }
    pub(crate) fn stamp(file: &File) -> Result<FileStamp> {
        let i = info(file)?;
        Ok(FileStamp {
            identity: FileIdentity {
                volume: i.volume,
                index: ((i.index_high as u64) << 32) | i.index_low as u64,
                created: ((i.created_high as u64) << 32) | i.created_low as u64,
            },
            modified: ((i.written_high as u64) << 32) | i.written_low as u64,
            size: ((i.size_high as u64) << 32) | i.size_low as u64,
        })
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
                "Cannot obtain a stable volume GUID path",
            ));
        }
        Ok(PathBuf::from(String::from_utf16_lossy(
            &buffer[..n as usize],
        )))
    }
    fn directory_entries(
        directory: &File,
        max: usize,
    ) -> Result<(Vec<(std::ffi::OsString, u32)>, bool)> {
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
                    "Directory enumeration returned incomplete data",
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
                        "Truncated directory entry",
                    ));
                }
                let u32_at = |pos| u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
                let next = u32_at(offset) as usize;
                let attrs = u32_at(offset + 56);
                let name_len = u32_at(offset + 60) as usize;
                if !name_len.is_multiple_of(2) || offset + 64 + name_len > bytes.len() {
                    return Err(CoreError::new(
                        "inventoryUnavailable",
                        "Invalid directory entry length",
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
                    names.push((name, attrs));
                }
                if next == 0 {
                    break;
                }
                if next < 64 || !next.is_multiple_of(8) {
                    return Err(CoreError::new(
                        "inventoryUnavailable",
                        "Invalid directory entry offset",
                    ));
                }
                offset = offset.checked_add(next).ok_or_else(|| {
                    CoreError::new("inventoryUnavailable", "Directory entry overflow")
                })?;
            }
        }
        Ok((names, true))
    }
    pub(crate) struct ParentGuard {
        dirs: Vec<File>,
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
        ) -> Result<(Vec<(std::ffi::OsString, u32)>, bool)> {
            directory_entries(self.dirs.last().unwrap(), max)
        }
        pub fn acquire(path: &Path) -> Result<Self> {
            let path = crate::absolute_path(path)?;
            let text = path.to_string_lossy();
            if text.starts_with("\\\\") || text.as_bytes().get(1) != Some(&b':') {
                return Err(CoreError::new(
                    "unsupportedVolume",
                    "Only explicit local drive paths are supported",
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
                    "The selected local volume cannot be qualified",
                ));
            }
            let end = fsname.iter().position(|x| *x == 0).unwrap_or(fsname.len());
            if String::from_utf16_lossy(&fsname[..end]) != "NTFS" {
                return Err(CoreError::new(
                    "unsupportedVolume",
                    "Writing is limited to local NTFS volumes",
                ));
            }
            let mut dirs = vec![root];
            let relative = path.strip_prefix(&drive).map_err(|_| {
                CoreError::new("invalidPath", "Path is outside its protected volume")
            })?;
            let mut parts = relative.components().collect::<Vec<_>>();
            let name = parts
                .pop()
                .ok_or_else(|| CoreError::new("invalidPath", "A leaf path is required"))?
                .as_os_str()
                .to_os_string();
            for part in parts {
                let child = nt_open(dirs.last(), part.as_os_str(), true, false, false)?;
                if stamp(&child)?.identity.volume != stamp(&dirs[0])?.identity.volume {
                    return Err(CoreError::new(
                        "unsupportedVolume",
                        "A directory crossed the protected volume",
                    ));
                }
                dirs.push(child);
            }
            Ok(Self { dirs, name })
        }
        pub fn open_file(&self, write: bool, create: bool) -> Result<File> {
            nt_open(self.dirs.last(), &self.name, false, write, create)
        }
        pub(crate) fn open_placement_file(&self) -> Result<File> {
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
        pub(crate) fn open_read_guard_file(&self) -> Result<File> {
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
        pub fn create_directory(&self) -> Result<File> {
            nt_open(self.dirs.last(), &self.name, true, true, true)
        }
        pub fn child_file(directory: &File, name: &str) -> Result<File> {
            nt_open(
                Some(directory),
                std::ffi::OsStr::new(name),
                false,
                true,
                true,
            )
        }
        pub fn child_directory(directory: &File, name: &str) -> Result<File> {
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
        dirs: std::collections::BTreeMap<PathBuf, File>,
        source: PathBuf,
        target_parent: PathBuf,
        canonical_keys: bool,
    }
    impl NamespaceGuard {
        pub fn acquire(source: &Path, target_parent: &Path) -> Result<Self> {
            let source = crate::absolute_path(source)?;
            let target_parent = crate::absolute_path(target_parent)?;
            let source_text = source.to_string_lossy();
            let target_text = target_parent.to_string_lossy();
            if !source_text[..2].eq_ignore_ascii_case(&target_text[..2]) {
                return Err(CoreError::new(
                    "crossVolume",
                    "Same-object moves require one local NTFS volume",
                ));
            }
            if target_text.eq_ignore_ascii_case(&source_text)
                || target_text
                    .to_lowercase()
                    .starts_with(&(source_text.to_lowercase() + "\\"))
            {
                return Err(CoreError::new(
                    "recursiveMove",
                    "A node cannot be moved into itself or a descendant",
                ));
            }
            // Reuse the already qualified volume-root and ancestor opening logic.
            let qualified = ParentGuard::acquire(&source)?;
            drop(qualified);
            let drive = PathBuf::from(format!("{}\\", &source_text[..2]));
            let root_nt = format!("\\??\\{}", drive.display());
            let root = nt_open(None, std::ffi::OsStr::new(&root_nt), true, false, false)?;
            let volume = stamp(&root)?.identity.volume;
            let mut dirs = std::collections::BTreeMap::new();
            dirs.insert(drive.clone(), root);
            for leaf in [&source, &target_parent] {
                let mut current = drive.clone();
                for part in leaf
                    .strip_prefix(&drive)
                    .map_err(|_| CoreError::new("invalidPath", "Invalid volume path"))?
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
                        if stamp(&file)?.identity.volume != volume {
                            return Err(CoreError::new(
                                "crossVolume",
                                "Directory crossed the protected volume",
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
        pub fn parent_identity(&self) -> Result<FileIdentity> {
            Ok(stamp(self.dirs.get(&self.target_parent).unwrap())?.identity)
        }
        pub fn open_source_file(&self, name: &str, delete: bool) -> Result<File> {
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
        fn source_directory(&mut self, relative: &Path) -> Result<&File> {
            let mut path = self.source.clone();
            for part in relative.components() {
                let std::path::Component::Normal(name) = part else {
                    return Err(CoreError::new(
                        "invalidPath",
                        "Expected relative source descendant",
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
        pub fn open_source_descendant(&mut self, relative: &Path) -> Result<File> {
            self.open_source_output(relative, false, false)
        }
        pub(crate) fn open_source_placement(&mut self, relative: &Path) -> Result<File> {
            let name = relative
                .file_name()
                .ok_or_else(|| CoreError::new("invalidPath", "Missing placement filename"))?;
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
        ) -> Result<File> {
            let name = relative
                .file_name()
                .ok_or_else(|| CoreError::new("invalidPath", "Missing descendant filename"))?;
            let directory = self.source_directory(relative.parent().unwrap())?;
            nt_open(Some(directory), name, false, write, create)
        }
        pub fn release_source_descendants(&mut self) {
            self.dirs
                .retain(|path, _| path == &self.source || !path.starts_with(&self.source));
        }
        pub fn rename_leaf(&self, file: &File, name: &str) -> Result<()> {
            rename_handle(file, self.dirs.get(&self.source).unwrap(), name)
        }
        pub fn move_source(&self, name: &str) -> Result<()> {
            rename_handle(
                self.dirs.get(&self.source).unwrap(),
                self.dirs.get(&self.target_parent).unwrap(),
                name,
            )
        }
        pub fn open_moved_file(&self, name: &str) -> Result<File> {
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
            let text = component
                .as_os_str()
                .to_str()
                .ok_or_else(|| CoreError::new("invalidPath", "Non-UTF8 namespace component"))?;
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
        pub(crate) fn acquire(participants: &[(&Path, &Path)]) -> Result<Self> {
            if participants.is_empty() || participants.len() > 5000 {
                return Err(CoreError::new(
                    "invalidPlan",
                    "Invalid namespace participant count",
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
            let text = actual[0].0.to_string_lossy();
            if text.starts_with("\\\\") || text.as_bytes().get(1) != Some(&b':') {
                return Err(CoreError::new(
                    "unsupportedVolume",
                    "Explicit local NTFS paths required",
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
                        "Compound placement requires one NTFS volume",
                    ));
                }
                for (other, root) in sources.iter().enumerate() {
                    if parents[index].starts_with(root)
                        || (index != other && sources[index].starts_with(root))
                    {
                        return Err(CoreError::new(
                            "recursiveMove",
                            "Compound roots and target parents must be independent",
                        ));
                    }
                }
                // Establish the existing NTFS qualification without retaining a
                // second closure that would block the union's DELETE handles.
                drop(ParentGuard::acquire(source)?);
                drop(ParentGuard::acquire(
                    &parent.join(".namespace-prospective"),
                )?);
            }
            let drive = PathBuf::from(format!("{}\\", &text[..2]));
            let drive_key = namespace_key(&drive)?;
            let root_nt = format!("\\??\\{}", drive.display());
            let root = nt_open(None, std::ffi::OsStr::new(&root_nt), true, false, false)?;
            let volume = stamp(&root)?.identity.volume;
            let mut dirs = std::collections::BTreeMap::new();
            dirs.insert(drive_key.clone(), root);
            for (source, parent) in &actual {
                for leaf in [source, parent] {
                    let mut current = drive_key.clone();
                    for part in leaf.components().skip(2) {
                        let next = namespace_key(&current.join(part.as_os_str()))?;
                        if !dirs.contains_key(&next) {
                            let file = nt_open_access(
                                dirs.get(&current),
                                part.as_os_str(),
                                true,
                                false,
                                false,
                                sources.contains(&next),
                            )?;
                            if stamp(&file)?.identity.volume != volume {
                                return Err(CoreError::new(
                                    "crossVolume",
                                    "Compound directory crossed the protected volume",
                                ));
                            }
                            dirs.insert(next.clone(), file);
                        }
                        current = next;
                    }
                }
            }
            Ok(Self {
                inner: NamespaceGuard {
                    dirs,
                    source: sources[0].clone(),
                    target_parent: parents[0].clone(),
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
                .ok_or_else(|| CoreError::new("invalidPlan", "Missing namespace participant"))?
                .clone();
            self.inner.target_parent = self.parents[index].clone();
            Ok(&mut self.inner)
        }
        pub(crate) fn source_parent_identity(&self, index: usize) -> Result<FileIdentity> {
            let parent = self
                .sources
                .get(index)
                .and_then(|source| source.parent())
                .ok_or_else(|| CoreError::new("invalidPlan", "Missing source parent"))?;
            Ok(stamp(
                self.inner.dirs.get(parent).ok_or_else(|| {
                    CoreError::new("invalidPlan", "Source parent is not retained")
                })?,
            )?
            .identity)
        }
    }
    fn rename_handle(file: &File, parent: &File, name: &str) -> Result<()> {
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
            (*ptr).parent = parent.as_raw_handle();
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
            NtSetInformationFile(file.as_raw_handle(), &mut io, ptr.cast(), length as u32, 10)
        };
        if status < 0 {
            return Err(native_error(status));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    pub(crate) struct ParentGuard;
    impl ParentGuard {
        pub fn directory_entries(
            &self,
            _: usize,
        ) -> Result<(Vec<(std::ffi::OsString, u32)>, bool)> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn acquire(_: &Path) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "This MVP write backend requires Windows local NTFS",
            ))
        }
        pub fn open_file(&self, _: bool, _: bool) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub(crate) fn open_placement_file(&self) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub(crate) fn open_read_guard_file(&self) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn create_directory(&self) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn child_file(_: &File, _: &str) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn child_directory(_: &File, _: &str) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub(crate) fn directory_coordinates(&self) -> Result<PathBuf> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn directory_identity(&self) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
    }
    pub(crate) struct NamespaceGuard;
    impl NamespaceGuard {
        pub fn acquire(_: &Path, _: &Path) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn source_identity(&self) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn parent_identity(&self) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn open_source_file(&self, _: &str, _: bool) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn source_directory_snapshot(
            &mut self,
            _: &Path,
            _: usize,
        ) -> Result<DirectorySnapshot> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn open_source_descendant(&mut self, _: &Path) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub(crate) fn open_source_placement(&mut self, _: &Path) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub(crate) fn source_parent_identity(&mut self, _: &Path) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn open_source_output(&mut self, _: &Path, _: bool, _: bool) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn release_source_descendants(&mut self) {}
        pub fn rename_leaf(&self, _: &File, _: &str) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn move_source(&self, _: &str) -> Result<()> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub fn open_moved_file(&self, _: &str) -> Result<File> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
    }
    pub(crate) struct MultiNamespaceGuard;
    impl MultiNamespaceGuard {
        pub(crate) fn acquire(_: &[(&Path, &Path)]) -> Result<Self> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub(crate) fn select(&mut self, _: usize) -> Result<&mut NamespaceGuard> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
        pub(crate) fn source_parent_identity(&self, _: usize) -> Result<FileIdentity> {
            Err(CoreError::new(
                "unsupportedPlatform",
                "Windows backend required",
            ))
        }
    }
    pub(crate) fn file_coordinates(_: &File) -> Result<PathBuf> {
        Err(CoreError::new(
            "unsupportedPlatform",
            "Windows backend required",
        ))
    }
    pub(crate) fn stamp(_: &File) -> Result<FileStamp> {
        Err(CoreError::new(
            "unsupportedPlatform",
            "Windows backend required",
        ))
    }
    pub(crate) fn publish_private_create(_: &Path, _: &Path) -> Result<()> {
        Err(CoreError::new(
            "unsupportedPlatform",
            "Windows backend required",
        ))
    }
}
pub(crate) use platform::{
    MultiNamespaceGuard, NamespaceGuard, ParentGuard, file_coordinates, publish_private_create,
    stamp,
};
pub(crate) fn same_directory(a: &Path, b: &Path) -> Result<bool> {
    let a = ParentGuard::acquire(&a.join(".directory-identity-prospective"))?;
    let b = ParentGuard::acquire(&b.join(".directory-identity-prospective"))?;
    Ok(a.directory_identity()? == b.directory_identity()?
        && a.directory_coordinates()? == b.directory_coordinates()?)
}
