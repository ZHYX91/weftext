//! Lossless ZIP delivery of an already committed Core recovery backup.
//! This is not the native content interchange format and does not restore or
//! reinterpret private records. The caller supplies only its original Plan cut.
use crate::{ExportError, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Seek, SeekFrom, Write};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub const MAX_BACKUP_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_BACKUP_MEMBERS: usize = 5000;
pub const MAX_BACKUP_ZIP_BYTES: usize = 80 * 1024 * 1024;

#[derive(Deserialize)]
struct Manifest {
    version: u32,
    kind: String,
    files: Vec<Member>,
}
#[derive(Deserialize)]
struct Member {
    path: String,
    bytes: usize,
    sha256: String,
}

fn invalid() -> ExportError {
    ExportError("Recovery backup does not match its original bounded v1 manifest".into())
}

/// Bound actual ZIP construction, including long member names and directory
/// headers, before Vec allocation rather than only checking the finished ZIP.
#[derive(Default)]
struct BackupZipCursor(Cursor<Vec<u8>>);
impl Write for BackupZipCursor {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .0
            .position()
            .checked_add(bytes.len() as u64)
            .is_none_or(|end| end > MAX_BACKUP_ZIP_BYTES as u64)
        {
            return Err(std::io::Error::other(
                "Recovery ZIP exceeds its byte budget",
            ));
        }
        self.0.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
impl Seek for BackupZipCursor {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let before = self.0.position();
        let next = self.0.seek(position)?;
        if next > MAX_BACKUP_ZIP_BYTES as u64 {
            self.0.set_position(before);
            return Err(std::io::Error::other(
                "Recovery ZIP exceeds its byte budget",
            ));
        }
        Ok(next)
    }
}

fn member_path(path: &str) -> Result<String> {
    // Core v1 records native relative separators. Preserve the manifest bytes
    // and translate only ZIP member spelling, never the backed-up file bytes.
    let normalized = path.replace('\\', "/");
    // Full recovery data is not constrained by the narrower native interchange
    // authoring profile. NTFS names count UTF-16 units, not UTF-8 bytes; a valid
    // long Chinese name must not fail only after Core has made its backup.
    if normalized.is_empty() || normalized.len() > u16::MAX as usize {
        return Err(invalid());
    }
    for part in normalized.split('/') {
        let device = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if part.is_empty()
            || matches!(part, "." | "..")
            || part.ends_with(['.', ' '])
            || part.encode_utf16().count() > 255
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
            || matches!(
                device.as_str(),
                "CON" | "PRN" | "AUX" | "NUL" | "COM¹" | "COM²" | "COM³" | "LPT¹" | "LPT²" | "LPT³"
            )
            || (device.len() == 4
                && (device.starts_with("COM") || device.starts_with("LPT"))
                && device.as_bytes()[3].is_ascii_digit())
        {
            return Err(invalid());
        }
    }
    if normalized != "backup.json"
        && normalized != "library"
        && normalized != "private"
        && !normalized.starts_with("library/")
        && !normalized.starts_with("private/")
    {
        return Err(invalid());
    }
    Ok(normalized)
}

/// Encode original after-images, including empty directories, without opening
/// any source/spool path. Stored entries and a fixed timestamp keep the same
/// original cut byte-identical for the lifetime of this format implementation.
pub fn encode_recovery_backup<'a>(
    directories: &[String],
    files: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Result<Vec<u8>> {
    let mut dirs = BTreeSet::new();
    let mut members = BTreeMap::new();
    let mut spellings = BTreeSet::new();
    let mut total = 0usize;
    for directory in directories {
        let name = member_path(directory)?;
        // The original Core CreateTree may repeat exact output parents. That
        // is one directory; a different case spelling is still an alias error.
        if dirs.contains(&name) {
            continue;
        }
        if name == "backup.json" || !spellings.insert(name.to_lowercase()) {
            return Err(invalid());
        }
        dirs.insert(name);
        if dirs.len() > MAX_BACKUP_MEMBERS {
            return Err(invalid());
        }
    }
    for (path, bytes) in files {
        let name = member_path(path)?;
        if matches!(name.as_str(), "library" | "private") || !spellings.insert(name.to_lowercase())
        {
            return Err(invalid());
        }
        total = total.checked_add(bytes.len()).ok_or_else(invalid)?;
        members.insert(name, bytes);
        if total > MAX_BACKUP_BYTES || members.len() + dirs.len() > MAX_BACKUP_MEMBERS {
            return Err(invalid());
        }
    }
    // Every intermediate directory must be represented by the original tree.
    // In particular, a file cannot also be a parent or have a case alias parent.
    for path in dirs.iter().chain(members.keys()) {
        let mut current = path.as_str();
        while let Some((parent, _)) = current.rsplit_once('/') {
            if !dirs.contains(parent) {
                return Err(invalid());
            }
            current = parent;
        }
    }
    if !dirs.contains("library") || !dirs.contains("private") {
        return Err(invalid());
    }
    let manifest_bytes = members.get("backup.json").ok_or_else(invalid)?;
    let manifest: Manifest = serde_json::from_slice(manifest_bytes).map_err(|_| invalid())?;
    if manifest.version != 1
        || manifest.kind != "weftext.local-backup.v1"
        || manifest.files.len() + 1 != members.len()
    {
        return Err(invalid());
    }
    let mut listed = BTreeSet::new();
    for expected in manifest.files {
        let name = member_path(&expected.path)?;
        if name == "backup.json" || !listed.insert(name.clone()) {
            return Err(invalid());
        }
        let actual = members.get(&name).ok_or_else(invalid)?;
        if actual.len() != expected.bytes
            || format!("{:x}", Sha256::digest(actual)) != expected.sha256
        {
            return Err(invalid());
        }
    }
    let mut archive = ZipWriter::new(BackupZipCursor::default());
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    for directory in dirs {
        archive.add_directory(format!("{directory}/"), options)?;
    }
    for (name, bytes) in members {
        archive.start_file(name, options)?;
        archive.write_all(bytes)?;
    }
    let bytes = archive.finish()?.0.into_inner();
    if bytes.len() > MAX_BACKUP_ZIP_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    #[test]
    fn zip_budget_refuses_overflow_before_allocating_header_or_payload_bytes() {
        let mut output = BackupZipCursor::default();
        output.write_all(b"original").unwrap();
        let position = MAX_BACKUP_ZIP_BYTES as u64 - 1;
        output.seek(SeekFrom::Start(position)).unwrap();
        assert!(output.write_all(b"oversized header").is_err());
        assert_eq!(output.0.get_ref(), b"original");
        assert_eq!(output.0.position(), position);
        assert!(output.seek(SeekFrom::Current(2)).is_err());
        assert_eq!(output.0.position(), position);
        output.seek(SeekFrom::Start(0)).unwrap();
        output.write_all(b"verified").unwrap();
        assert_eq!(output.0.into_inner(), b"verified");
    }

    fn fixture() -> (Vec<String>, BTreeMap<String, Vec<u8>>) {
        let dirs = ["library", "library/空目录", "private", "private/actors"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::from([
            (
                "library/笔记.typ".into(),
                "准确草稿\r\n".as_bytes().to_vec(),
            ),
            (
                "private/actors/fence.json".into(),
                b"{\"cancelled\":true}".to_vec(),
            ),
        ]);
        files.insert(
            format!("library/{}.txt", "长".repeat(100)),
            b"long NTFS name".to_vec(),
        );
        let manifest = serde_json::json!({"version":1,"kind":"weftext.local-backup.v1",
            "workspaceId":"original", "restore":"Preserve originals",
            "files":files.iter().map(|(path,bytes)|serde_json::json!({"path":path.replace('/',"\\"),"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(bytes))})).collect::<Vec<_>>()});
        files.insert("backup.json".into(), serde_json::to_vec(&manifest).unwrap());
        (dirs, files)
    }
    fn encode(dirs: &[String], files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>> {
        encode_recovery_backup(dirs, files.iter().map(|(p, b)| (p.as_str(), b.as_slice())))
    }
    #[test]
    fn original_v1_files_and_empty_directories_roundtrip_without_native_roles() {
        let (dirs, files) = fixture();
        let bytes = encode(&dirs, &files).unwrap();
        assert_eq!(bytes, encode(&dirs, &files).unwrap());
        let mut repeated_parents = dirs.clone();
        repeated_parents.extend(dirs.iter().cloned());
        assert_eq!(bytes, encode(&repeated_parents, &files).unwrap());
        let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(zip.len(), dirs.len() + files.len());
        assert!(zip.by_name("library/空目录/").unwrap().is_dir());
        for (name, expected) in files {
            let mut actual = Vec::new();
            zip.by_name(&name)
                .unwrap()
                .read_to_end(&mut actual)
                .unwrap();
            assert_eq!(actual, expected);
        }
        assert!(zip.by_name("manifest.json").is_err());
    }
    #[test]
    fn wrong_bytes_missing_extra_aliases_and_unsafe_names_fail_before_output() {
        let (dirs, files) = fixture();
        let mut changed = files.clone();
        changed
            .get_mut("private/actors/fence.json")
            .unwrap()
            .push(b' ');
        assert!(encode(&dirs, &changed).is_err());
        for path in [
            "../foreign",
            "library/../foreign",
            "private/C:secret",
            "library/CON",
            "other/file",
            "LIBRARY/笔记.typ",
            "library/笔记.typ/child",
        ] {
            let mut changed = files.clone();
            changed.insert(path.into(), b"foreign".to_vec());
            assert!(encode(&dirs, &changed).is_err(), "{path}");
        }
        let mut missing = files.clone();
        missing.remove("private/actors/fence.json");
        assert!(encode(&dirs, &missing).is_err());
        let mut duplicate = dirs.clone();
        duplicate.push("PRIVATE".into());
        assert!(encode(&duplicate, &files).is_err());
        let mut missing_parent = dirs.clone();
        missing_parent.retain(|p| p != "private/actors");
        assert!(encode(&missing_parent, &files).is_err());
    }
}
