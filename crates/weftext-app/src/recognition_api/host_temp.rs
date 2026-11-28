//! Recognition caller TEMP only. Retained inputs/results never move here.
use serde_json::json;
use std::{
    ffi::c_void,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        FILE_DISPOSITION_INFO, FILE_ID_INFO, FileDispositionInfo, FileIdInfo,
        GetFileInformationByHandleEx, SetFileInformationByHandle,
    },
};

// GetTempPathW appends '\'. GetTempFileNameW limits its directory argument
// to MAX_PATH-14; retain a separate NUL unit as an additional conservative margin.
// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-gettemppathw
// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-gettempfilenamew
const MAX_PATH_UNITS: usize = 260;
const TEMP_NAME_RESERVE: usize = 14;
fn bounded(path: &Path) -> bool {
    let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
    !units.contains(&0) && units.len().saturating_add(1 + TEMP_NAME_RESERVE + 1) <= MAX_PATH_UNITS
}
fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Recognition host TEMP is not qualified",
    )
}
fn ordinary(directory: &File) -> io::Result<()> {
    let metadata = directory.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err(refused());
    }
    Ok(())
}
fn open_directory(path: &Path, access: u32, share: u32) -> io::Result<File> {
    let file = OpenOptions::new()
        .access_mode(access)
        .share_mode(share)
        .custom_flags(0x02000000 | 0x00200000) // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
        .open(path)?;
    ordinary(&file)?;
    Ok(file)
}
fn identity(directory: &File) -> io::Result<(u64, [u8; 16])> {
    let mut id = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(directory.as_raw_handle()),
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .map_err(io::Error::other)?;
    Ok((id.VolumeSerialNumber, id.FileId.Identifier))
}

// Same documented native FILE_CREATE directory operation and ABI as Core's
// backend, limited here to one ASCII execution leaf under a protected parent.
// CreateDirectoryW followed by Open would not retain the original creation.
// https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntcreatefile
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}
#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root: *mut c_void,
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
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtCreateFile(
        handle: *mut *mut c_void,
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
}
fn create_leaf(parent: &File, name: &str) -> io::Result<File> {
    let mut wide = name.encode_utf16().collect::<Vec<_>>();
    let mut name = UnicodeString {
        length: (wide.len() * 2) as u16,
        maximum_length: (wide.len() * 2) as u16,
        buffer: wide.as_mut_ptr(),
    };
    let mut attributes = ObjectAttributes {
        length: std::mem::size_of::<ObjectAttributes>() as u32,
        root: parent.as_raw_handle(),
        name: &mut name,
        attributes: 0x40,
        security: null_mut(),
        quality: null_mut(),
    };
    let mut handle = null_mut();
    let mut result = IoStatus {
        status: 0,
        information: 0,
    };
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            1 | 0x80 | 0x10000 | 0x100000, // LIST | READ_ATTRIBUTES | DELETE | SYNCHRONIZE
            &mut attributes,
            &mut result,
            null(),
            0x10,
            3,                     // SHARE_READ | SHARE_WRITE; deny rename/delete while retained
            2,                     // FILE_CREATE: never adopt a preexisting file or directory
            1 | 0x20 | 0x00200000, // DIRECTORY | SYNCHRONOUS_NONALERT | OPEN_REPARSE_POINT
            null(),
            0,
        )
    };
    if status < 0 {
        return Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        ));
    }
    let file = unsafe { File::from_raw_handle(handle) };
    ordinary(&file)?;
    Ok(file)
}

pub(super) struct Workspace {
    path: PathBuf,
    original: (u64, [u8; 16]),
    directory: Option<File>,
    _parents: weftext_core::QualifiedProfile,
    record: File,
    attempted: bool,
    finished: bool,
}
impl Workspace {
    pub(super) fn create(runtime: &Path, retained: &Path, tag: &str) -> io::Result<Self> {
        if !runtime.is_absolute()
            || !uuid::Uuid::parse_str(tag)
                .is_ok_and(|id| id.to_string() == tag && id.get_version_num() == 4)
        {
            return Err(refused());
        }
        let leaf = format!("host-{tag}");
        if !bounded(&runtime.join("jobs").join(&leaf)) {
            return Err(refused());
        }
        let runtime = weftext_core::qualify_existing_profile(runtime).map_err(io::Error::other)?;
        let jobs = runtime.path().join("jobs");
        match fs::create_dir(&jobs) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        let parents = weftext_core::qualify_existing_profile(&jobs).map_err(io::Error::other)?;
        let path = parents.path().join(&leaf);
        if !bounded(&path) {
            return Err(refused());
        }
        let parent = open_directory(parents.path(), 1 | 0x80 | 0x100000, 3)?;
        let mut record = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(retained.join(format!("host-temp-{tag}.jsonl")))?;
        let attempt = json!({"version":1,"stage":"create-attempt","path":path.to_str(),"pathUtf16":path.as_os_str().encode_wide().collect::<Vec<_>>()});
        writeln!(record, "{attempt}")?;
        record.sync_all()?;
        let directory = create_leaf(&parent, &leaf)?;
        let original = identity(&directory)?;
        let mut workspace = Self {
            path,
            original,
            directory: Some(directory),
            _parents: parents,
            record,
            attempted: false,
            finished: false,
        };
        workspace.record("created", None)?;
        Ok(workspace)
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    fn record(&mut self, stage: &str, error: Option<i32>) -> io::Result<()> {
        let value = json!({"version":1,"stage":stage,"timestampUnixMillis":super::now(),"identity":self.original,"spawnAttempted":self.attempted,"terminalConfirmed":self.finished,"osError":error});
        writeln!(self.record, "{value}")?;
        self.record.sync_all()
    }
    pub(super) fn spawn_attempted(&mut self) {
        self.attempted = true;
        let _ = self.record("spawn-attempted", None);
    }
    // Call only after owned Child::try_wait returned terminal (includes Job0),
    // or owned Child::kill completed its original process/Job wait.
    pub(super) fn confirmed_terminal(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let result = self.remove_empty();
        let (stage, error) = match result {
            Ok(stage) => (stage, None),
            Err(error) => ("retained-cleanup-unavailable", error.raw_os_error()),
        };
        let _ = self.record(stage, error);
    }
    fn remove_empty(&mut self) -> io::Result<&'static str> {
        let Some(directory) = self.directory.as_ref() else {
            return Ok("retained-original-handle-unavailable");
        };
        ordinary(directory)?;
        let named = open_directory(&self.path, 0x80, 7)?;
        if identity(directory)? != self.original || identity(&named)? != self.original {
            return Ok("retained-identity-changed");
        }
        if fs::read_dir(&self.path)?.next().transpose()?.is_some() {
            return Ok("retained-nonempty");
        }
        drop(named);
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        unsafe {
            SetFileInformationByHandle(
                HANDLE(directory.as_raw_handle()),
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        }
        .map_err(io::Error::other)?;
        self.directory.take();
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok("removed-empty"),
            _ => Ok("removal-unconfirmed"),
        }
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        // Reporting only: no deletion, no guessed child/Job terminal state.
        if !self.finished {
            let stage = if self.attempted {
                "retained-terminal-unconfirmed"
            } else {
                "retained-no-spawn"
            };
            let _ = self.record(stage, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let runtime = temp.path().join("runtime");
        let retained = temp.path().join("retained");
        fs::create_dir(&runtime).unwrap();
        fs::create_dir(&retained).unwrap();
        (temp, runtime, retained)
    }
    fn tag() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    fn stages(retained: &Path, tag: &str) -> Vec<String> {
        fs::read_to_string(retained.join(format!("host-temp-{tag}.jsonl")))
            .unwrap()
            .lines()
            .map(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["stage"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn utf16_budget_includes_separator_temp_file_reserve_and_nul() {
        let maximum = MAX_PATH_UNITS - 1 - TEMP_NAME_RESERVE - 1;
        let at_limit = PathBuf::from(format!("D:\\{}", "x".repeat(maximum - 3)));
        assert!(bounded(&at_limit));
        assert!(!bounded(&PathBuf::from(format!("{}x", at_limit.display()))));
        // A supplementary character occupies two native Windows units.
        assert!(bounded(Path::new(&format!("D:\\{}", "😀".repeat(120)))));
        assert!(!bounded(Path::new(&format!("D:\\{}", "😀".repeat(121)))));
        assert!(!bounded(Path::new(&format!(
            "\\\\?\\{}",
            at_limit.display()
        ))));
    }

    #[test]
    fn overlong_runtime_is_refused_before_creating_jobs_or_temp() {
        let (_temp, runtime, retained) = fixture();
        let selected = runtime.join("x".repeat(220));
        fs::create_dir(&selected).unwrap();
        assert!(Workspace::create(&selected, &retained, &tag()).is_err());
        assert!(!selected.join("jobs").exists());
        assert_eq!(fs::read_dir(retained).unwrap().count(), 0);
    }

    #[test]
    fn unicode_runtime_uses_native_units_and_no_spawn_retains_empty_temp() {
        let (_temp, runtime, retained) = fixture();
        let unicode = runtime.join("识别-😀");
        fs::create_dir(&unicode).unwrap();
        let id = tag();
        let workspace = Workspace::create(&unicode, &retained, &id).unwrap();
        let path = workspace.path().to_owned();
        assert!(path.starts_with(unicode.join("jobs")));
        assert!(bounded(&path));
        assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
        drop(workspace);
        assert!(path.is_dir());
        assert!(stages(&retained, &id).contains(&"retained-no-spawn".into()));
    }

    #[test]
    fn existing_jobs_file_and_execution_leaves_are_never_adopted() {
        let (_temp, runtime, retained) = fixture();
        let jobs = runtime.join("jobs");
        fs::write(&jobs, b"original jobs file").unwrap();
        assert!(Workspace::create(&runtime, &retained, &tag()).is_err());
        assert_eq!(fs::read(&jobs).unwrap(), b"original jobs file");
        fs::remove_file(&jobs).unwrap();
        fs::create_dir(&jobs).unwrap();
        for directory in [false, true] {
            let id = tag();
            let path = jobs.join(format!("host-{id}"));
            if directory {
                fs::create_dir(&path).unwrap();
                fs::write(path.join("retained"), b"original leaf bytes").unwrap();
            } else {
                fs::write(&path, b"original leaf bytes").unwrap();
            }
            assert!(Workspace::create(&runtime, &retained, &id).is_err());
            assert_eq!(
                fs::read(if directory {
                    path.join("retained")
                } else {
                    path
                })
                .unwrap(),
                b"original leaf bytes"
            );
        }
    }

    #[test]
    fn unknown_or_nonempty_temp_is_retained_without_recursive_removal() {
        let (_temp, runtime, retained) = fixture();
        let unknown_tag = tag();
        let mut unknown = Workspace::create(&runtime, &retained, &unknown_tag).unwrap();
        let unknown_path = unknown.path().to_owned();
        assert_eq!(
            fs::read_dir(&unknown_path).unwrap().count(),
            0,
            "No TEMP marker"
        );
        unknown.spawn_attempted();
        drop(unknown);
        assert!(unknown_path.is_dir());
        assert!(stages(&retained, &unknown_tag).contains(&"retained-terminal-unconfirmed".into()));

        let nonempty_tag = tag();
        let mut nonempty = Workspace::create(&runtime, &retained, &nonempty_tag).unwrap();
        let original = nonempty.path().join("producer-file");
        fs::write(&original, b"preserved producer bytes").unwrap();
        nonempty.confirmed_terminal();
        assert_eq!(fs::read(original).unwrap(), b"preserved producer bytes");
        assert!(stages(&retained, &nonempty_tag).contains(&"retained-nonempty".into()));
    }

    #[test]
    fn original_handle_blocks_replacement_and_changed_identity_is_retained() {
        let (_temp, runtime, retained) = fixture();
        let id = tag();
        let mut workspace = Workspace::create(&runtime, &retained, &id).unwrap();
        let path = workspace.path().to_owned();
        let replaced = runtime.join("jobs/retained-original");
        assert!(
            fs::rename(&path, &replaced).is_err(),
            "Original handle denies rename"
        );
        // Simulate a lost guard and external replacement, retaining the recorded
        // original identity. A new handle must never acquire deletion authority.
        drop(workspace.directory.take());
        fs::rename(&path, &replaced).unwrap();
        fs::create_dir(&path).unwrap();
        workspace.directory = Some(open_directory(&path, 0x80 | 0x10000, 3).unwrap());
        workspace.confirmed_terminal();
        assert!(path.is_dir() && replaced.is_dir());
        assert!(stages(&retained, &id).contains(&"retained-identity-changed".into()));
    }

    // Same unprivileged junction fixture as Core's backend tests.
    fn junction(path: &Path, target: &Path) {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn DeviceIoControl(
                handle: *mut c_void,
                code: u32,
                input: *const c_void,
                input_len: u32,
                output: *mut c_void,
                output_len: u32,
                returned: *mut u32,
                overlapped: *mut c_void,
            ) -> i32;
        }
        fs::create_dir(path).unwrap();
        let handle = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(7)
            .custom_flags(0x02000000 | 0x00200000)
            .open(path)
            .unwrap();
        let substitute = format!("\\??\\{}", target.display())
            .encode_utf16()
            .collect::<Vec<_>>();
        let printable = target.as_os_str().encode_wide().collect::<Vec<_>>();
        let mut payload = Vec::new();
        payload.extend_from_slice(&0xA0000003u32.to_le_bytes());
        payload.extend_from_slice(
            &((8 + (substitute.len() + 1 + printable.len() + 1) * 2) as u16).to_le_bytes(),
        );
        for value in [
            0,
            0,
            (substitute.len() * 2) as u16,
            ((substitute.len() + 1) * 2) as u16,
            (printable.len() * 2) as u16,
        ] {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        for value in substitute
            .into_iter()
            .chain([0])
            .chain(printable)
            .chain([0])
        {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        let mut returned = 0;
        assert_ne!(
            unsafe {
                DeviceIoControl(
                    handle.as_raw_handle(),
                    0x900A4,
                    payload.as_ptr().cast(),
                    payload.len() as u32,
                    null_mut(),
                    0,
                    &mut returned,
                    null_mut(),
                )
            },
            0,
            "{}",
            io::Error::last_os_error()
        );
    }

    #[test]
    fn runtime_or_ancestor_junction_cannot_redirect_temp_creation() {
        let (temp, _runtime, retained) = fixture();
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::create_dir(outside.join("nested-runtime")).unwrap();
        fs::write(outside.join("original"), b"unchanged").unwrap();
        let linked = temp.path().join("linked");
        junction(&linked, &outside);
        for runtime in [linked.clone(), linked.join("nested-runtime")] {
            assert!(Workspace::create(&runtime, &retained, &tag()).is_err());
        }
        assert!(!outside.join("jobs").exists());
        assert!(!outside.join("nested-runtime/jobs").exists());
        assert_eq!(fs::read(outside.join("original")).unwrap(), b"unchanged");
        fs::remove_dir(linked).unwrap();
    }

    #[tokio::test]
    async fn confirmed_original_child_and_job_terminal_allow_empty_removal() {
        let (temp, runtime, retained) = fixture();
        let id = tag();
        let mut workspace = Workspace::create(&runtime, &retained, &id).unwrap();
        let path = workspace.path().to_owned();
        let system = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let mut command = crate::owned_process::Command::new(
            PathBuf::from(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"),
        );
        command
            .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
            .env_clear()
            .env("SystemRoot", system)
            .env("TEMP", &path)
            .env("TMP", &path);
        workspace.spawn_attempted();
        let mut child = match command.spawn().await {
            Ok(child) => child,
            Err(error) => {
                let kept = temp.keep();
                panic!(
                    "Original spawn outcome unknown; retained {}: {error}",
                    kept.display()
                );
            }
        };
        let status =
            match tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await {
                Ok(Ok(status)) => status,
                error => {
                    if child.kill().await.is_err() {
                        let kept = temp.keep();
                        panic!(
                            "Original child retirement unknown; retained {}: {error:?}",
                            kept.display()
                        );
                    }
                    panic!("Original child did not finish in fixture deadline: {error:?}");
                }
            };
        assert!(status.success());
        workspace.confirmed_terminal();
        assert!(!path.exists());
        assert!(stages(&retained, &id).contains(&"removed-empty".into()));
    }
}
