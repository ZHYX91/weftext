use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::{ImportError, ImportErrorCode, PortablePath};

const ROOT_MARKER: &[u8] = b"weftext.import-temp-root.v1\n";
const ROOT_MARKER_FILE: &str = ".weftext-import-temp-root";
const SESSION_MARKER: &[u8] = b"weftext.import-temp-session.v1\n";
const SESSION_MARKER_FILE: &str = ".weftext-import-temp-session";
const SESSION_PREFIX: &str = "session-";
const ROOT_MARKER_PUBLISH_RETRIES: usize = 1_000;
const ROOT_MARKER_PUBLISH_RETRY_DELAY: Duration = Duration::from_millis(1);

#[derive(Clone, Debug)]
pub struct ImportTempRoot {
    path: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CleanupReport {
    pub removed: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

impl ImportTempRoot {
    /// Initializes or reopens a dedicated marked import-temp authority.
    ///
    /// # Errors
    ///
    /// Returns an error for links, non-directories, unknown markers, non-empty
    /// unowned directories, or filesystem failures.
    pub fn initialize(path: impl Into<PathBuf>) -> Result<Self, ImportError> {
        let path = path.into();
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_dir() || is_symlink_or_reparse(&metadata) {
                    return temp_error("import temporary root must be a regular directory");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&path)
                    .map_err(|error| ImportError::io("create import temporary root", &error))?;
            }
            Err(error) => {
                return Err(ImportError::io("inspect import temporary root", &error));
            }
        }

        let marker = path.join(ROOT_MARKER_FILE);
        match fs::read(&marker) {
            Ok(bytes) if bytes == ROOT_MARKER => {}
            Ok(bytes) if ROOT_MARKER.starts_with(&bytes) => {
                wait_for_exact_root_marker(&marker)?;
            }
            Ok(_) => return temp_error("import temporary root marker has an unknown version"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let directory_is_nonempty = fs::read_dir(&path)
                    .map_err(|error| ImportError::io("inspect import temporary root", &error))?
                    .next()
                    .is_some();
                if directory_is_nonempty {
                    // Another process may have initialized the dedicated root after our first
                    // marker read. Its create-new marker is visible before its tiny payload is
                    // fully published, so wait only for an exact known prefix to finish. A
                    // non-empty unmarked directory still fails closed.
                    if wait_for_exact_root_marker(&marker).is_err() {
                        return temp_error(
                            "an existing non-empty directory cannot be adopted as an import temporary root",
                        );
                    }
                } else if let Err(create_error) =
                    write_new_file(&marker, ROOT_MARKER, "create import temporary root marker")
                {
                    // Another process may have won the create-new race. Accept only the exact
                    // marker it published.
                    if wait_for_exact_root_marker(&marker).is_err() {
                        return Err(create_error);
                    }
                }
            }
            Err(error) => return Err(ImportError::io("read import temporary root marker", &error)),
        }

        let canonical = path
            .canonicalize()
            .map_err(|error| ImportError::io("resolve import temporary root", &error))?;
        Ok(Self { path: canonical })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Creates one uniquely named, marked temporary import session.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid identifier, unsafe root, name collision,
    /// or filesystem failure.
    pub fn start_session(&self, job_id: &str) -> Result<TempSession, ImportError> {
        validate_job_id(job_id)?;
        self.verify_root()?;
        let path = self.path.join(format!("{SESSION_PREFIX}{job_id}"));
        fs::create_dir(&path)
            .map_err(|error| ImportError::io("create import temporary session", &error))?;
        if let Err(error) = write_new_file(
            &path.join(SESSION_MARKER_FILE),
            SESSION_MARKER,
            "create import temporary session marker",
        ) {
            let _ = fs::remove_dir(&path);
            return Err(error);
        }
        Ok(TempSession {
            root: self.clone(),
            path,
            cleanup_on_drop: true,
        })
    }

    /// Removes only verified abandoned sessions and reports unowned entries.
    /// Call this during process startup, before any import session is active.
    ///
    /// # Errors
    ///
    /// Returns an error when root verification, inspection, or safe removal fails.
    pub fn recover_abandoned(&self) -> Result<CleanupReport, ImportError> {
        self.verify_root()?;
        let mut report = CleanupReport::default();
        let entries = fs::read_dir(&self.path)
            .map_err(|error| ImportError::io("enumerate import temporary root", &error))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| ImportError::io("inspect import temporary entry", &error))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                report.skipped.push(entry.path());
                continue;
            };
            if name == ROOT_MARKER_FILE {
                continue;
            }
            let Some(job_id) = name.strip_prefix(SESSION_PREFIX) else {
                report.skipped.push(entry.path());
                continue;
            };
            if validate_job_id(job_id).is_err()
                || !safe_session_directory(&self.path, &entry.path())?
            {
                report.skipped.push(entry.path());
                continue;
            }
            remove_session_directory(&self.path, &entry.path())?;
            report.removed.push(entry.path());
        }
        Ok(report)
    }

    fn verify_root(&self) -> Result<(), ImportError> {
        let metadata = fs::symlink_metadata(&self.path)
            .map_err(|error| ImportError::io("inspect import temporary root", &error))?;
        if !metadata.is_dir()
            || is_symlink_or_reparse(&metadata)
            || fs::read(self.path.join(ROOT_MARKER_FILE))
                .map_err(|error| ImportError::io("read import temporary root marker", &error))?
                != ROOT_MARKER
        {
            return temp_error("import temporary root authority is missing or unsafe");
        }
        Ok(())
    }
}

fn wait_for_exact_root_marker(marker: &Path) -> Result<(), ImportError> {
    for _ in 0..ROOT_MARKER_PUBLISH_RETRIES {
        match fs::read(marker) {
            Ok(bytes) if bytes == ROOT_MARKER => return Ok(()),
            Ok(bytes) if ROOT_MARKER.starts_with(&bytes) => {
                thread::sleep(ROOT_MARKER_PUBLISH_RETRY_DELAY);
            }
            Ok(_) => return temp_error("import temporary root marker has an unknown version"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                thread::sleep(ROOT_MARKER_PUBLISH_RETRY_DELAY);
            }
            Err(error) => {
                return Err(ImportError::io(
                    "read concurrently published import temporary root marker",
                    &error,
                ));
            }
        }
    }
    temp_error("import temporary root marker publication did not complete")
}

#[derive(Debug)]
pub struct TempSession {
    root: ImportTempRoot,
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl TempSession {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes one bounded, create-new worker input below this session.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe locators, links, collisions, over-limit bytes,
    /// lost session authority, or filesystem failure.
    pub fn write_file(
        &self,
        locator: &PortablePath,
        bytes: &[u8],
        maximum_bytes: u64,
    ) -> Result<(), ImportError> {
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "temporary worker input exceeds its byte limit",
            ));
        }
        self.root.verify_root()?;
        if !safe_session_directory(&self.root.path, &self.path)? {
            return temp_error("import temporary session authority is missing or unsafe");
        }
        let mut parent = self.path.clone();
        let mut components = locator.as_str().split('/').peekable();
        while let Some(component) = components.next() {
            if components.peek().is_none() {
                parent.push(component);
                write_new_file(&parent, bytes, "write bounded worker input")?;
                return Ok(());
            }
            parent.push(component);
            match fs::symlink_metadata(&parent) {
                Ok(metadata) => {
                    if !metadata.is_dir() || is_symlink_or_reparse(&metadata) {
                        return temp_error("worker input parent is not a regular directory");
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&parent).map_err(|error| {
                        ImportError::io("create worker input directory", &error)
                    })?;
                }
                Err(error) => return Err(ImportError::io("inspect worker input parent", &error)),
            }
        }
        temp_error("worker input locator is empty")
    }

    #[cfg(test)]
    pub(crate) fn abandon(mut self) -> PathBuf {
        self.cleanup_on_drop = false;
        self.path.clone()
    }

    pub(crate) fn preserve_for_recovery(&mut self) {
        self.cleanup_on_drop = false;
    }
}

impl Drop for TempSession {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = remove_session_directory(&self.root.path, &self.path);
        }
    }
}

fn safe_session_directory(root: &Path, session: &Path) -> Result<bool, ImportError> {
    let metadata = match fs::symlink_metadata(session) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(ImportError::io("inspect import temporary session", &error)),
    };
    if !metadata.is_dir() || is_symlink_or_reparse(&metadata) {
        return Ok(false);
    }
    let canonical = session
        .canonicalize()
        .map_err(|error| ImportError::io("resolve import temporary session", &error))?;
    if canonical.parent() != Some(root) {
        return Ok(false);
    }
    if fs::read(session.join(SESSION_MARKER_FILE)).ok().as_deref() != Some(SESSION_MARKER) {
        return Ok(false);
    }
    Ok(!contains_link_or_reparse(session)?)
}

fn contains_link_or_reparse(path: &Path) -> Result<bool, ImportError> {
    for entry in fs::read_dir(path)
        .map_err(|error| ImportError::io("inspect import temporary session tree", &error))?
    {
        let entry = entry
            .map_err(|error| ImportError::io("inspect import temporary session entry", &error))?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            ImportError::io("inspect import temporary session metadata", &error)
        })?;
        if is_symlink_or_reparse(&metadata) {
            return Ok(true);
        }
        if metadata.is_dir() && contains_link_or_reparse(&entry.path())? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn remove_session_directory(root: &Path, session: &Path) -> Result<(), ImportError> {
    if !safe_session_directory(root, session)? {
        return temp_error("refusing to remove an unverified import temporary session");
    }
    fs::remove_dir_all(session)
        .map_err(|error| ImportError::io("remove import temporary session", &error))
}

fn write_new_file(path: &Path, bytes: &[u8], operation: &str) -> Result<(), ImportError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| ImportError::io(operation, &error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| ImportError::io(operation, &error))
}

fn validate_job_id(value: &str) -> Result<(), ImportError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return temp_error("temporary job identifiers must use bounded lowercase ASCII");
    }
    Ok(())
}

fn is_symlink_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn temp_error<T>(message: &str) -> Result<T, ImportError> {
    Err(ImportError::new(ImportErrorCode::TemporaryStorage, message))
}

#[cfg(test)]
mod tests {
    use super::ImportTempRoot;
    use std::sync::{Arc, Barrier};

    #[test]
    fn abandoned_sessions_are_removed_but_unowned_entries_are_not() {
        let base = tempfile::tempdir().expect("temp directory");
        let root_path = base.path().join("imports");
        let root = ImportTempRoot::initialize(&root_path).expect("initialize root");
        let abandoned = root
            .start_session("crashed-1")
            .expect("start session")
            .abandon();
        std::fs::write(root.path().join("do-not-remove.txt"), b"user evidence")
            .expect("write unowned evidence");

        let report = root.recover_abandoned().expect("recover abandoned");

        assert_eq!(report.removed, vec![abandoned.clone()]);
        assert!(!abandoned.exists());
        assert!(root.path().join("do-not-remove.txt").exists());
        assert_eq!(report.skipped.len(), 1);
    }

    #[test]
    fn concurrent_initialization_accepts_only_the_exact_shared_marker() {
        let base = tempfile::tempdir().expect("temp directory");
        let root_path = base.path().join("imports");
        let barrier = Arc::new(Barrier::new(16));
        let handles = (0..16)
            .map(|_| {
                let path = root_path.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    ImportTempRoot::initialize(path)
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            handle
                .join()
                .expect("initializer thread")
                .expect("concurrent initialization");
        }
        ImportTempRoot::initialize(root_path).expect("reopen exact initialized root");
    }
}
