use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use crate::{ExportError, ExportErrorCode};

pub(crate) fn normalize_external_new_path(
    workspace: &Path,
    target: &Path,
    expected_extensions: &[&str],
) -> Result<PathBuf, ExportError> {
    let canonical_workspace = fs::canonicalize(workspace)
        .map_err(|error| io_error("resolve workspace root for external export", &error))?;
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    reject_linked_ancestors(parent)?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| io_error("resolve external export parent", &error))?;
    if canonical_parent.starts_with(&canonical_workspace) {
        return Err(ExportError::new(
            ExportErrorCode::UnsafeDestination,
            "export artifacts and bundles must remain outside the workspace",
        ));
    }
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ExportError::new(
                ExportErrorCode::UnsafeDestination,
                "external export filename must be UTF-8",
            )
        })?;
    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.trim() != file_name
        || file_name.ends_with('.')
        || file_name.chars().any(char::is_control)
    {
        return Err(ExportError::new(
            ExportErrorCode::UnsafeDestination,
            "external export filename is empty or non-portable",
        ));
    }
    let valid_extension = target
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            expected_extensions
                .iter()
                .any(|expected| extension.eq_ignore_ascii_case(expected))
        });
    if !valid_extension {
        return Err(ExportError::new(
            ExportErrorCode::UnsafeDestination,
            "external export destination has an unsupported extension",
        ));
    }
    let normalized = canonical_parent.join(file_name);
    if normalized.exists() {
        return Err(ExportError::new(
            ExportErrorCode::DestinationExists,
            format!(
                "external export destination already exists: {}",
                normalized.display()
            ),
        ));
    }
    Ok(normalized)
}

pub(crate) fn publish_create_new(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    let parent = path.parent().ok_or_else(|| {
        ExportError::new(
            ExportErrorCode::UnsafeDestination,
            "external export destination has no parent",
        )
    })?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ExportError::new(
                ExportErrorCode::UnsafeDestination,
                "external export filename must be UTF-8",
            )
        })?;
    let temporary = parent.join(format!(
        ".{name}.weftext-export-{}.tmp",
        uuid::Uuid::new_v4()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("create external export staging file", &error))?;
        file.write_all(bytes)
            .map_err(|error| io_error("write external export staging file", &error))?;
        file.sync_all()
            .map_err(|error| io_error("sync external export staging file", &error))?;
        fs::hard_link(&temporary, path)
            .map_err(|error| io_error("publish external export without overwrite", &error))?;
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| io_error("verify published external export", &error))?;
        if !metadata.is_file() || linked_or_reparse(&metadata) {
            return Err(ExportError::new(
                ExportErrorCode::Io,
                "published external export is not one regular non-link file",
            ));
        }
        Ok(())
    })();
    let _ = fs::remove_file(&temporary);
    result
}

pub(crate) fn read_regular_file_bounded(
    path: &Path,
    maximum_bytes: u64,
) -> Result<Vec<u8>, ExportError> {
    let file = open_regular_file_nofollow(path)?;
    let metadata = file
        .metadata()
        .map_err(|error| io_error("inspect external export file", &error))?;
    if !metadata.is_file() || linked_or_reparse(&metadata) {
        return Err(ExportError::new(
            ExportErrorCode::Io,
            "external export input is not a regular non-link file",
        ));
    }
    if metadata.len() > maximum_bytes {
        return Err(ExportError::new(
            ExportErrorCode::LimitExceeded,
            "external export input exceeds its byte limit",
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    file.take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read external export file", &error))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
        return Err(ExportError::new(
            ExportErrorCode::LimitExceeded,
            "external export input grew beyond its byte limit",
        ));
    }
    Ok(bytes)
}

fn reject_linked_ancestors(path: &Path) -> Result<(), ExportError> {
    for ancestor in path.ancestors() {
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(io_error("inspect external export path", &error)),
        };
        if linked_or_reparse(&metadata) {
            return Err(ExportError::new(
                ExportErrorCode::UnsafeDestination,
                "external export path cannot traverse a link or reparse point",
            ));
        }
    }
    Ok(())
}

fn open_regular_file_nofollow(path: &Path) -> Result<File, ExportError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options
        .open(path)
        .map_err(|error| io_error("open external export file", &error))
}

fn linked_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}

fn io_error(context: &str, error: &std::io::Error) -> ExportError {
    let code = if error.kind() == std::io::ErrorKind::AlreadyExists {
        ExportErrorCode::DestinationExists
    } else {
        ExportErrorCode::Io
    };
    ExportError::new(code, format!("{context}: {error}"))
}
