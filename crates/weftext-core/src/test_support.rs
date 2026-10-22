//! Test deployments are explicit administrator enrollments. This module never
//! grants production authority to an arbitrary temporary path.
#[cfg(target_os = "linux")]
use crate::PathBuf;
use crate::{Core, Path};

pub(crate) struct FixtureRoots {
    #[cfg(not(target_os = "linux"))]
    temp: tempfile::TempDir,
    #[cfg(target_os = "linux")]
    paths: [PathBuf; 3],
}

impl FixtureRoots {
    pub fn new() -> Self {
        #[cfg(not(target_os = "linux"))]
        {
            Self {
                temp: tempfile::tempdir().unwrap(),
            }
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::DirBuilderExt;
            let paths = enrollments().map(|path| {
                let authority = weftext_enrollment::inspect_enrollment(&path).unwrap();
                authority.verify_service_process().unwrap();
                authority.check_current().unwrap();
                let child = authority
                    .root()
                    .join(format!("sync-case-{}", crate::fresh()));
                std::fs::DirBuilder::new()
                    .mode(0o700)
                    .create(&child)
                    .unwrap();
                std::fs::File::open(authority.root())
                    .unwrap()
                    .sync_all()
                    .unwrap();
                child
            });
            Self { paths }
        }
    }
    pub fn a_path(&self) -> &Path {
        #[cfg(not(target_os = "linux"))]
        {
            self.temp.path()
        }
        #[cfg(target_os = "linux")]
        {
            &self.paths[0]
        }
    }
    pub fn b_path(&self) -> &Path {
        #[cfg(not(target_os = "linux"))]
        {
            self.temp.path()
        }
        #[cfg(target_os = "linux")]
        {
            &self.paths[1]
        }
    }
    pub fn path(&self) -> &Path {
        #[cfg(not(target_os = "linux"))]
        {
            self.temp.path()
        }
        #[cfg(target_os = "linux")]
        {
            &self.paths[2]
        }
    }
}

pub(crate) fn open_core(path: impl AsRef<Path>) -> crate::Result<Core> {
    #[cfg(not(target_os = "linux"))]
    {
        Core::new(path)
    }
    #[cfg(target_os = "linux")]
    {
        let path = crate::absolute_path(path.as_ref())?;
        for enrollment in enrollments() {
            let authority = weftext_enrollment::inspect_enrollment(&enrollment)
                .expect("explicit administrator test enrollment");
            if path.starts_with(authority.root()) && path != authority.root() {
                return Core::with_backend(
                    &path,
                    crate::BackendContext::controlled_service(&enrollment)?,
                );
            }
        }
        Err(crate::CoreError::new(
            "outsideRoot",
            weftext_messages::UiMessage::error_core_posix_outside_controlled_root(),
        ))
    }
}

/// A test-only external transport channel. Copy portable members create-only;
/// existing names must retain exactly the same bytes in either direction.
pub(crate) fn copy_transport(source: &Path, target: &Path) {
    if source == target {
        return;
    }
    use std::io::Write;
    if !target.exists() {
        std::fs::create_dir(target).unwrap();
    }
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let to = target.join(entry.file_name());
        let meta = entry.file_type().unwrap();
        if meta.is_dir() {
            copy_transport(&entry.path(), &to);
        } else {
            assert!(meta.is_file());
            let bytes = std::fs::read(entry.path()).unwrap();
            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&to)
            {
                Ok(mut file) => {
                    file.write_all(&bytes).unwrap();
                    file.sync_all().unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    assert_eq!(std::fs::read(&to).unwrap(), bytes)
                }
                Err(error) => panic!("create-only test transport copy failed: {error}"),
            }
        }
    }
    std::fs::File::open(target).unwrap().sync_all().unwrap();
}

#[cfg(target_os = "linux")]
fn enrollments() -> [PathBuf; 3] {
    ["A", "B", "C"].map(|peer| {
        PathBuf::from(
            std::env::var_os(format!("WEFTEXT_LINUX_TEST_PEER_{peer}"))
                .expect("three explicit independently enrolled test roots are required"),
        )
    })
}

#[cfg(target_os = "linux")]
impl Drop for FixtureRoots {
    fn drop(&mut self) {
        fn retain(paths: &[PathBuf; 3]) -> std::io::Result<()> {
            use std::io::Write;
            use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
            fn receipt(path: &Path, value: &serde_json::Value) -> std::io::Result<()> {
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)?;
                file.write_all(&serde_json::to_vec(value)?)?;
                file.sync_all()?;
                std::fs::File::open(path.parent().unwrap())?.sync_all()
            }
            let retained = PathBuf::from(
                std::env::var_os("WEFTEXT_LINUX_TEST_RETAINED_FIXTURES")
                    .expect("explicit owned retained fixture directory"),
            );
            let meta = std::fs::symlink_metadata(&retained)?;
            if !meta.is_dir()
                || meta.uid() != unsafe { libc::geteuid() }
                || meta.mode() & 0o777 != 0o700
            {
                return Err(std::io::Error::other(
                    "retained fixture directory is not an owned private directory",
                ));
            }
            let retained = std::fs::canonicalize(&retained)?;
            for (path, enrollment) in paths.iter().zip(enrollments()) {
                let authority =
                    weftext_enrollment::inspect_enrollment(&enrollment).map_err(|error| {
                        std::io::Error::other(format!("test enrollment changed: {error:?}"))
                    })?;
                authority.check_current().map_err(|error| {
                    std::io::Error::other(format!("test enrollment changed: {error:?}"))
                })?;
                let source = std::fs::canonicalize(path)?;
                if source.parent() != Some(authority.root())
                    || !source
                        .file_name()
                        .is_some_and(|name| name.as_bytes().starts_with(b"sync-case-"))
                {
                    return Err(std::io::Error::other(
                        "fixture path is outside its exact enrolled test parent",
                    ));
                }
                let target = retained.join(format!(
                    "{}-{}",
                    source.file_name().unwrap().to_string_lossy(),
                    crate::fresh()
                ));
                let receipt_id = crate::fresh();
                let original = serde_json::json!({"source":source,"target":target,"enrollment":enrollment,"rootIdentity":authority.root_identity()});
                receipt(
                    &retained.join(format!("{receipt_id}.planned.json")),
                    &original,
                )?;
                let from = std::ffi::CString::new(source.as_os_str().as_bytes()).unwrap();
                let to = std::ffi::CString::new(target.as_os_str().as_bytes()).unwrap();
                if unsafe {
                    libc::syscall(
                        libc::SYS_renameat2,
                        libc::AT_FDCWD,
                        from.as_ptr(),
                        libc::AT_FDCWD,
                        to.as_ptr(),
                        libc::RENAME_NOREPLACE,
                    )
                } < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                std::fs::File::open(authority.root())?.sync_all()?;
                std::fs::File::open(&retained)?.sync_all()?;
                receipt(
                    &retained.join(format!("{receipt_id}.retained.json")),
                    &original,
                )?;
            }
            Ok(())
        }
        if let Err(error) = retain(&self.paths) {
            if std::thread::panicking() {
                eprintln!(
                    "Fixture retention is incomplete; keep original and retained paths: {error}"
                );
            } else {
                panic!(
                    "Fixture retention is incomplete; keep original and retained paths: {error}"
                );
            }
        }
    }
}
