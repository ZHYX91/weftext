//! The one product builder freezes GUI, CLI and this catalog as one batch.
//! File IDs qualify this observation, never a portable installation identity.
//! https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
    sync::OnceLock,
};
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        FILE_ID_INFO, FILE_SHARE_READ, FileIdInfo, GetFileInformationByHandleEx,
    },
};

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "The executable is not a complete qualified Weftext build",
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Spec {
    schema_version: u32,
    build_id: String,
    source_sha256: String,
    host_contract_sha256: String,
    architecture: String,
    roles: Roles,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Roles {
    desktop: Role,
    cli: Role,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Role {
    filename: String,
    sha256: String,
    bytes: u64,
    pe_subsystem: u16,
}

pub(crate) struct Catalog {
    spec: Spec,
    directory: PathBuf,
    _image: VerifiedImage,
}
impl Catalog {
    pub(crate) fn build_id(&self) -> &str {
        &self.spec.build_id
    }
}
pub(crate) struct VerifiedImage {
    _file: File,
    _identity: (u64, [u8; 16]),
}

fn sha(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn canonical_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn identity(file: &File) -> io::Result<(u64, [u8; 16])> {
    let mut id = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .map_err(io::Error::other)?;
    Ok((id.VolumeSerialNumber, id.FileId.Identifier))
}

fn pe_subsystem(file: &mut File, bytes: u64) -> io::Result<u16> {
    let mut dos = [0u8; 64];
    file.read_exact(&mut dos)?;
    if &dos[..2] != b"MZ" {
        return Err(invalid());
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if !(64..=1024 * 1024).contains(&offset) || offset + 24 + 70 > bytes {
        return Err(invalid());
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut pe = [0u8; 24 + 70];
    file.read_exact(&mut pe)?;
    if &pe[..4] != b"PE\0\0"
        || u16::from_le_bytes([pe[4], pe[5]]) != 0x8664
        || u16::from_le_bytes([pe[20], pe[21]]) < 70
        || u16::from_le_bytes([pe[24], pe[25]]) != 0x20b
    {
        return Err(invalid());
    }
    Ok(u16::from_le_bytes([pe[24 + 68], pe[24 + 69]]))
}

impl Spec {
    fn validate(&self) -> io::Result<()> {
        let contract = crate::host_contract::get()?;
        if self.schema_version != 1
            || self.architecture != "x64"
            || ![
                &self.build_id,
                &self.source_sha256,
                &self.host_contract_sha256,
                &self.roles.desktop.sha256,
                &self.roles.cli.sha256,
            ]
            .iter()
            .all(|v| canonical_sha(v))
            || self.host_contract_sha256
                != sha(include_bytes!(
                    "../../../packaging/windows/host-contract.json"
                ))
            || self.roles.desktop.filename != contract.roles.desktop.filename
            || self.roles.cli.filename != contract.roles.cli.filename
            || self.roles.desktop.pe_subsystem != 2
            || self.roles.cli.pe_subsystem != 3
            || [&self.roles.desktop, &self.roles.cli]
                .iter()
                .any(|r| r.bytes == 0 || r.bytes > 4 * 1024 * 1024 * 1024)
        {
            return Err(invalid());
        }
        let material = format!(
            "{}\n{}\n{}\n{}",
            self.source_sha256,
            self.host_contract_sha256,
            self.roles.desktop.sha256,
            self.roles.cli.sha256
        );
        if self.build_id != sha(material.as_bytes()) {
            return Err(invalid());
        }
        Ok(())
    }

    fn verify(&self, path: &Path) -> io::Result<VerifiedImage> {
        // Deny write/delete sharing for the lifetime of the observation. The
        // process handle independently binds PID/creation time; an argv path,
        // a same-folder name or a peer's claimed version is never authority.
        let mut file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .open(path)?;
        let before = identity(&file)?;
        let meta = file.metadata()?;
        if !meta.is_file() {
            return Err(invalid());
        }
        let subsystem = pe_subsystem(&mut file, meta.len())?;
        let candidates = [&self.roles.desktop, &self.roles.cli]
            .into_iter()
            .filter(|r| r.bytes == meta.len() && r.pe_subsystem == subsystem)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Err(invalid());
        }
        file.seek(SeekFrom::Start(0))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut length = 0u64;
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            length += count as u64;
            if length > meta.len() {
                return Err(invalid());
            }
            hasher.update(&buffer[..count]);
        }
        let digest = format!("{:x}", hasher.finalize());
        if length != meta.len()
            || identity(&file)? != before
            || !candidates.iter().any(|r| r.sha256 == digest)
        {
            return Err(invalid());
        }
        Ok(VerifiedImage {
            _file: file,
            _identity: before,
        })
    }
}

impl Catalog {
    pub(crate) fn get() -> io::Result<&'static Self> {
        static CATALOG: OnceLock<Result<Catalog, String>> = OnceLock::new();
        CATALOG
            .get_or_init(|| {
                (|| -> io::Result<Catalog> {
                    let executable = std::env::current_exe()?;
                    let directory = executable.parent().ok_or_else(invalid)?.to_path_buf();
                    let file = File::open(directory.join("artifact-catalog.json"))?;
                    if file.metadata()?.len() > 16 * 1024 {
                        return Err(invalid());
                    }
                    let mut bytes = Vec::new();
                    file.take(16 * 1024 + 1).read_to_end(&mut bytes)?;
                    if bytes.len() > 16 * 1024 {
                        return Err(invalid());
                    }
                    let spec: Spec = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
                    spec.validate()?;
                    let image = spec.verify(&executable)?;
                    Ok(Catalog {
                        spec,
                        directory,
                        _image: image,
                    })
                })()
                .map_err(|e| e.to_string())
            })
            .as_ref()
            .map_err(|e| io::Error::other(e.clone()))
    }

    pub(crate) fn verify_image(&self, path: &Path) -> io::Result<VerifiedImage> {
        self.spec.verify(path)
    }

    pub(crate) fn desktop_executable(&self) -> io::Result<(PathBuf, VerifiedImage)> {
        let path = self.directory.join(&self.spec.roles.desktop.filename);
        let image = self.spec.verify(&path)?;
        Ok((path, image))
    }
}
