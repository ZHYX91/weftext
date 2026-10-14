//! A bounded, original-preserving native content package. This is deliberately
//! separate from a Core backup: local admission, operation journals and execution
//! responsibility are never portable package members. No member is executed or
//! extracted while reading a package.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const NATIVE_FORMAT: &str = "weftext.native-package";
pub const NATIVE_VERSION: u32 = 1;
pub const MAX_PACKAGE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_PACKAGE_FILES: usize = 5000;
const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
const MANIFEST_PATH: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NativeFileRole {
    Source,
    Resource,
    Style,
    ResourceManifest,
    Annotations,
    Schema,
    Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeEntry {
    pub role: NativeFileRole,
    pub bytes: usize,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}
impl NativeEntry {
    pub fn new(role: NativeFileRole, bytes: &[u8]) -> Self {
        Self {
            role,
            bytes: bytes.len(),
            sha256: digest(bytes),
            node_id: None,
            owner_node_id: None,
            resource_id: None,
            revision: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeLoss {
    pub path: String,
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeManifest {
    pub format: String,
    pub version: u32,
    pub package_id: String,
    pub workspace_id: String,
    /// v1 freezes the complete admitted live library, not an arbitrary raw tree.
    pub scope: String,
    pub schema_path: String,
    #[serde(deserialize_with = "unique_entries")]
    pub entries: BTreeMap<String, NativeEntry>,
    #[serde(default)]
    pub losses: Vec<NativeLoss>,
}

#[derive(Debug, Clone)]
pub struct NativePackage {
    pub manifest: NativeManifest,
    pub files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct NativeError {
    pub code: String,
    pub message: String,
}
impl NativeError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for NativeError {}
type Result<T> = std::result::Result<T, NativeError>;

fn unique_entries<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, NativeEntry>, D::Error> {
    struct Entries;
    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = BTreeMap<String, NativeEntry>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a native entry map with unique member names")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut output = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, NativeEntry>()? {
                if output.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("Duplicate native manifest member"));
                }
                if output.len() > MAX_PACKAGE_FILES {
                    return Err(serde::de::Error::custom(
                        "Native manifest member budget exceeded",
                    ));
                }
            }
            Ok(output)
        }
    }
    deserializer.deserialize_map(Entries)
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Canonical, cross-platform file paths only. Checking the spelling before any
/// extraction also rejects Windows drive/ADS/device aliases and case collisions.
pub fn validate_member_path(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 1024 || path.starts_with('/') || path.contains('\\') {
        return Err(NativeError::new(
            "packagePath",
            "Noncanonical native member path",
        ));
    }
    for part in path.split('/') {
        let device = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.len() > 240
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
            || matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || matches!(
                device.as_str(),
                "COM¹" | "COM²" | "COM³" | "LPT¹" | "LPT²" | "LPT³"
            )
            || (device.len() == 4
                && (device.starts_with("COM") || device.starts_with("LPT"))
                && device.as_bytes()[3].is_ascii_digit())
        {
            return Err(NativeError::new(
                "packagePath",
                format!("Unsafe native member path: {path}"),
            ));
        }
    }
    Ok(())
}

fn uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|u| u.to_string() == value)
}

/// Validates a closed exact byte map and the only supported package version.
pub fn validate_native_package(package: &NativePackage) -> Result<()> {
    let manifest = &package.manifest;
    if manifest.format != NATIVE_FORMAT
        || manifest.version != NATIVE_VERSION
        || manifest.scope != "library"
    {
        return Err(NativeError::new(
            "packageVersion",
            "Unsupported native package format/version/scope",
        ));
    }
    if !uuid(&manifest.package_id) || !uuid(&manifest.workspace_id) {
        return Err(NativeError::new(
            "packageIdentity",
            "Package/workspace identities must be canonical UUIDs",
        ));
    }
    if package.files.len() != manifest.entries.len()
        || package.files.is_empty()
        || package.files.len() > MAX_PACKAGE_FILES
        || package
            .files
            .values()
            .map(Vec::len)
            .try_fold(0usize, usize::checked_add)
            .is_none_or(|n| n > MAX_PACKAGE_BYTES)
    {
        return Err(NativeError::new(
            "packageBudget",
            "Native package is incomplete or exceeds the member budget",
        ));
    }
    let mut paths = BTreeSet::new();
    let mut nodes = BTreeSet::new();
    let mut node_directories = BTreeMap::new();
    let mut resources = BTreeSet::new();
    for (path, entry) in &manifest.entries {
        validate_member_path(path)?;
        if path == MANIFEST_PATH || !paths.insert(path.to_lowercase()) {
            return Err(NativeError::new(
                "packagePath",
                "Duplicate/colliding native member path",
            ));
        }
        let bytes = package
            .files
            .get(path)
            .ok_or_else(|| NativeError::new("packageManifest", "Missing declared native member"))?;
        if entry.bytes != bytes.len() || entry.sha256 != digest(bytes) {
            return Err(NativeError::new(
                "packageDigest",
                format!("Native member differs from its manifest: {path}"),
            ));
        }
        for id in [
            &entry.node_id,
            &entry.owner_node_id,
            &entry.resource_id,
            &entry.revision,
        ]
        .into_iter()
        .flatten()
        {
            if !uuid(id) {
                return Err(NativeError::new(
                    "packageIdentity",
                    "Native identities/revisions require canonical UUIDs",
                ));
            }
        }
        let leaf = path.rsplit('/').next().unwrap();
        match entry.role {
            NativeFileRole::Source => {
                let id = entry.node_id.as_ref().ok_or_else(|| {
                    NativeError::new("packageIdentity", "Source has no Node identity")
                })?;
                let parts: Vec<_> = path.split('/').collect();
                if !nodes.insert(id)
                    || parts.len() < 2
                    || leaf != format!("{}.typ", parts[parts.len() - 2])
                    || entry.owner_node_id.is_some()
                    || entry.resource_id.is_some()
                    || bytes.len() > 2 * 1024 * 1024
                    || std::str::from_utf8(bytes).is_err()
                {
                    return Err(NativeError::new(
                        "packageSource",
                        "Source identity/path/encoding is not canonical",
                    ));
                }
                node_directories.insert(id.clone(), path.rsplit_once('/').unwrap().0);
            }
            NativeFileRole::Resource | NativeFileRole::Style => {
                let id = entry.resource_id.as_ref().ok_or_else(|| {
                    NativeError::new("packageIdentity", "Resource has no identity")
                })?;
                if entry.node_id.is_some()
                    || entry.owner_node_id.is_none()
                    || !resources.insert(id)
                    || matches!(leaf, "weftext.resources.json" | "weftext.annotations.json")
                {
                    return Err(NativeError::new(
                        "packageResource",
                        "Resource identity/owner is not canonical",
                    ));
                }
            }
            NativeFileRole::Schema | NativeFileRole::Settings => {
                let expected = if entry.role == NativeFileRole::Schema {
                    ".weftext-schema.json"
                } else {
                    ".weftext-settings.json"
                };
                if path != expected
                    || entry.node_id.is_some()
                    || entry.owner_node_id.is_some()
                    || entry.resource_id.is_some()
                {
                    return Err(NativeError::new(
                        "packageControl",
                        "Invalid portable config path",
                    ));
                }
            }
            NativeFileRole::Annotations | NativeFileRole::ResourceManifest => {
                let expected = if entry.role == NativeFileRole::Annotations {
                    "weftext.annotations.json"
                } else {
                    "weftext.resources.json"
                };
                if leaf != expected
                    || entry.node_id.is_some()
                    || entry.owner_node_id.is_none()
                    || entry.resource_id.is_some()
                {
                    return Err(NativeError::new(
                        "packageControl",
                        "Invalid owned document path",
                    ));
                }
            }
        }
        // There are no local admission/control/journal slots in this format.
        if path != ".weftext-schema.json"
            && path != ".weftext-settings.json"
            && path.split('/').any(|p| p.starts_with(".weftext-"))
        {
            return Err(NativeError::new(
                "packageControl",
                "Local admission/history is not a portable member",
            ));
        }
    }
    for entry in manifest.entries.values() {
        if entry
            .owner_node_id
            .as_ref()
            .is_some_and(|owner| !nodes.contains(owner))
        {
            return Err(NativeError::new(
                "packageOwner",
                "Resource/control owner is outside the selected native package",
            ));
        }
    }
    for (path, entry) in &manifest.entries {
        if let Some(owner) = &entry.owner_node_id {
            let directory = node_directories[owner];
            let nearest = node_directories
                .iter()
                .filter(|(_, parent)| path.starts_with(&format!("{parent}/")))
                .max_by_key(|(_, parent)| parent.len())
                .map(|(id, _)| id.as_str());
            if nearest != Some(owner.as_str())
                || (matches!(
                    entry.role,
                    NativeFileRole::Annotations | NativeFileRole::ResourceManifest
                ) && path.rsplit_once('/').map(|(parent, _)| parent) != Some(directory))
            {
                return Err(NativeError::new(
                    "packageOwner",
                    "A native member crosses its exact Node ownership boundary",
                ));
            }
        }
    }
    if manifest.schema_path != ".weftext-schema.json"
        || !manifest
            .entries
            .get(&manifest.schema_path)
            .is_some_and(|e| e.role == NativeFileRole::Schema)
    {
        return Err(NativeError::new(
            "packageSchema",
            "The frozen native schema is unavailable",
        ));
    }
    // A file cannot also be an ancestor directory, including Windows aliases.
    for path in &paths {
        let mut ancestor = path.as_str();
        while let Some((parent, _)) = ancestor.rsplit_once('/') {
            if paths.contains(parent) || parent == MANIFEST_PATH {
                return Err(NativeError::new(
                    "packagePath",
                    "A file overlaps another member's directory",
                ));
            }
            ancestor = parent;
        }
    }
    Ok(())
}

/// Writes deterministic stored ZIP members: exact author bytes are not converted.
pub fn write_native_package(package: &NativePackage) -> Result<Vec<u8>> {
    validate_native_package(package)?;
    let manifest = serde_json::to_vec_pretty(&package.manifest)
        .map_err(|e| NativeError::new("packageManifest", e.to_string()))?;
    if manifest.len() > MAX_MANIFEST_BYTES {
        return Err(NativeError::new(
            "packageBudget",
            "Native manifest exceeds its byte budget",
        ));
    }
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o600);
    writer
        .start_file(MANIFEST_PATH, options)
        .map_err(|e| NativeError::new("packageZip", e.to_string()))?;
    writer
        .write_all(&manifest)
        .map_err(|e| NativeError::new("packageZip", e.to_string()))?;
    for (path, bytes) in &package.files {
        writer
            .start_file(path, options)
            .map_err(|e| NativeError::new("packageZip", e.to_string()))?;
        writer
            .write_all(bytes)
            .map_err(|e| NativeError::new("packageZip", e.to_string()))?;
    }
    let bytes = writer
        .finish()
        .map_err(|e| NativeError::new("packageZip", e.to_string()))?
        .into_inner();
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(NativeError::new(
            "packageBudget",
            "Native ZIP exceeds its total byte budget",
        ));
    }
    Ok(bytes)
}

/// Safely reads into a bounded byte map, without creating any filesystem object.
pub fn read_native_package(bytes: &[u8]) -> Result<NativePackage> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(NativeError::new(
            "packageBudget",
            "Native ZIP exceeds its byte budget",
        ));
    }
    // Bound declared central-directory allocation before ZipArchive parses it.
    // Native v1 fits classic single-disk ZIP; ZIP64/multidisk metadata is not an
    // accepted input variant and cannot supply an attacker-sized entry reserve.
    let tail_start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let eocd = bytes[tail_start..]
        .windows(4)
        .rposition(|part| part == b"PK\x05\x06")
        .map(|offset| tail_start + offset)
        .ok_or_else(|| NativeError::new("packageZip", "Native ZIP has no complete end record"))?;
    let end = bytes
        .get(eocd..eocd + 22)
        .ok_or_else(|| NativeError::new("packageZip", "Native ZIP end record is truncated"))?;
    let field = |offset: usize| u16::from_le_bytes([end[offset], end[offset + 1]]) as usize;
    let count = field(10);
    if field(4) != 0
        || field(6) != 0
        || field(8) != count
        || count == u16::MAX as usize
        || end[12..16] == [255; 4]
        || end[16..20] == [255; 4]
        || eocd + 22 + field(20) != bytes.len()
    {
        return Err(NativeError::new(
            "packageZip",
            "Native v1 requires one complete classic single-disk ZIP",
        ));
    }
    if !(2..=MAX_PACKAGE_FILES + 1).contains(&count) {
        return Err(NativeError::new(
            "packageBudget",
            "Declared native ZIP member count exceeds the budget",
        ));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| NativeError::new("packageZip", e.to_string()))?;
    if zip.len() < 2 || zip.len() > MAX_PACKAGE_FILES + 1 {
        return Err(NativeError::new(
            "packageBudget",
            "Native ZIP member count exceeds the budget",
        ));
    }
    let mut files = BTreeMap::new();
    let mut collision = BTreeSet::new();
    let mut total = 0usize;
    for index in 0..zip.len() {
        let mut member = zip
            .by_index(index)
            .map_err(|e| NativeError::new("packageZip", e.to_string()))?;
        let name = std::str::from_utf8(member.name_raw())
            .map_err(|_| NativeError::new("packagePath", "ZIP member names must be UTF-8"))?
            .to_string();
        validate_member_path(&name)?;
        let max = if name == MANIFEST_PATH {
            MAX_MANIFEST_BYTES
        } else {
            MAX_PACKAGE_BYTES
        };
        let kind = member.unix_mode().unwrap_or(0) & 0o170000;
        if member.is_dir()
            || !matches!(kind, 0 | 0o100000)
            || !collision.insert(name.to_lowercase())
            || member.size() > max as u64
            || !matches!(
                member.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
            || (member.size() > 1024 * 1024
                && member.size() > member.compressed_size().saturating_mul(100))
        {
            return Err(NativeError::new(
                "packageZip",
                "ZIP has duplicate, special, oversized or excessively compressed members",
            ));
        }
        total = total
            .checked_add(member.size() as usize)
            .ok_or_else(|| NativeError::new("packageBudget", "Native byte count overflow"))?;
        if total > MAX_PACKAGE_BYTES {
            return Err(NativeError::new(
                "packageBudget",
                "Expanded native package exceeds its byte budget",
            ));
        }
        let mut output = Vec::new();
        (&mut member)
            .take(max as u64 + 1)
            .read_to_end(&mut output)
            .map_err(|e| NativeError::new("packageZip", e.to_string()))?;
        if output.len() != member.size() as usize || output.len() > max {
            return Err(NativeError::new(
                "packageZip",
                "ZIP member size/checksum is invalid",
            ));
        }
        files.insert(name, output);
    }
    let manifest_bytes = files
        .remove(MANIFEST_PATH)
        .ok_or_else(|| NativeError::new("packageManifest", "Native manifest is missing"))?;
    let manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| NativeError::new("packageManifest", e.to_string()))?;
    let package = NativePackage { manifest, files };
    validate_native_package(&package)?;
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> NativePackage {
        let source = b"#metadata((id: \"00000000-0000-4000-8000-000000000001\", types: ())) <weftext>\n= Exact\r\n".to_vec();
        let mut source_entry = NativeEntry::new(NativeFileRole::Source, &source);
        source_entry.node_id = Some("00000000-0000-4000-8000-000000000001".into());
        let schema = b"{\"version\":1}".to_vec();
        NativePackage {
            manifest: NativeManifest {
                format: NATIVE_FORMAT.into(),
                version: NATIVE_VERSION,
                package_id: "00000000-0000-4000-8000-000000000002".into(),
                workspace_id: "00000000-0000-4000-8000-000000000003".into(),
                scope: "library".into(),
                schema_path: ".weftext-schema.json".into(),
                losses: vec![],
                entries: BTreeMap::from([
                    ("Note/Note.typ".into(), source_entry),
                    (
                        ".weftext-schema.json".into(),
                        NativeEntry::new(NativeFileRole::Schema, &schema),
                    ),
                ]),
            },
            files: BTreeMap::from([
                ("Note/Note.typ".into(), source),
                (".weftext-schema.json".into(), schema),
            ]),
        }
    }
    #[test]
    fn exact_native_bytes_round_trip_deterministically() {
        let package = fixture();
        let first = write_native_package(&package).unwrap();
        let decoded = read_native_package(&first).unwrap();
        assert_eq!(decoded.files, package.files);
        assert_eq!(write_native_package(&decoded).unwrap(), first);
    }
    #[test]
    fn manifest_tampering_and_private_admission_are_rejected() {
        let mut package = fixture();
        package.files.get_mut("Note/Note.typ").unwrap().push(b'x');
        assert_eq!(
            validate_native_package(&package).unwrap_err().code,
            "packageDigest"
        );
        let mut package = fixture();
        let bytes = b"forged birth".to_vec();
        package.manifest.entries.insert(
            ".weftext-mvp-commit.json".into(),
            NativeEntry::new(NativeFileRole::Settings, &bytes),
        );
        package
            .files
            .insert(".weftext-mvp-commit.json".into(), bytes);
        assert_eq!(
            validate_native_package(&package).unwrap_err().code,
            "packageControl"
        );
    }
    #[test]
    fn unsafe_zip_paths_and_bombs_are_rejected_without_extraction() {
        for path in [
            "../outside",
            "/outside",
            "C:/outside",
            "a\\b",
            "A/con.txt",
            "A/file:stream",
            "A/../B",
            "A/x.",
        ] {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            writer
                .start_file(path, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"bad").unwrap();
            writer
                .start_file("manifest.json", SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"{}").unwrap();
            let bytes = writer.finish().unwrap().into_inner();
            assert_eq!(
                read_native_package(&bytes).unwrap_err().code,
                "packagePath",
                "{path}"
            );
        }
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "huge.bin",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(&vec![0; 2 * 1024 * 1024]).unwrap();
        writer
            .start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"{}").unwrap();
        assert_eq!(
            read_native_package(&writer.finish().unwrap().into_inner())
                .unwrap_err()
                .code,
            "packageZip"
        );
    }
    #[test]
    fn case_and_file_directory_collisions_are_rejected() {
        let mut package = fixture();
        let entry = package.manifest.entries["Note/Note.typ"].clone();
        let bytes = package.files["Note/Note.typ"].clone();
        package
            .manifest
            .entries
            .insert("note/note.typ".into(), entry);
        package.files.insert("note/note.typ".into(), bytes);
        assert_eq!(
            validate_native_package(&package).unwrap_err().code,
            "packagePath"
        );
    }

    #[test]
    fn central_directory_allocation_is_bounded_before_zip_parsing() {
        let mut bytes = write_native_package(&fixture()).unwrap();
        let end = bytes.len() - 22;
        bytes[end + 8..end + 10].copy_from_slice(&6000u16.to_le_bytes());
        bytes[end + 10..end + 12].copy_from_slice(&6000u16.to_le_bytes());
        assert_eq!(
            read_native_package(&bytes).unwrap_err().code,
            "packageBudget"
        );
        bytes[end + 8..end + 12].fill(255);
        assert_eq!(read_native_package(&bytes).unwrap_err().code, "packageZip");
    }
}
