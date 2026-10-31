//! Mutable private activation routing, separate from immutable build artifacts
//! and from every Core profile. It carries locators/identity metadata, never
//! author input, bearer credentials, delivery outcomes or a second Core journal.
use crate::launch::{LocalOriginDiagnostic, LocalOriginPhase};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions, TryLockError},
    io::{self, Read, Write},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{HANDLE, HLOCAL, LocalFree},
        Security::{
            Authorization::{
                ConvertSecurityDescriptorToStringSecurityDescriptorW,
                ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
                SE_FILE_OBJECT,
            },
            DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
            SECURITY_ATTRIBUTES,
        },
        Storage::FileSystem::{
            CreateDirectoryW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_SHARE_READ, FILE_SHARE_WRITE, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            MoveFileExW, READ_CONTROL,
        },
    },
    core::{HSTRING, PWSTR},
};

const LIMIT: usize = 16 * 1024 * 1024;
const LEAF: &str = "host-routing.json";
const LOCK_WAIT: Duration = Duration::from_secs(5);
// Historical nonzero u16 ports are never reassigned. The actual namespace
// and the independent 16 MiB serialized budget bound this lifetime catalog;
// temporary test profiles belong in an isolated OS guest, not this user root.
const MAX_ORIGINS: usize = u16::MAX as usize;
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "The private activation routing evidence is not qualified",
    )
}

fn select_new_origin<T>(
    reserved: &BTreeSet<u16>,
    requested: u16,
    mut bind: impl FnMut(u16) -> io::Result<T>,
    mut address: impl FnMut(&T) -> io::Result<u16>,
) -> Result<T, LocalOriginDiagnostic> {
    use LocalOriginPhase as Phase;
    if requested != 0 && reserved.contains(&requested) {
        return Err(LocalOriginDiagnostic::from_io(
            Phase::RequestedPortReserved,
            &invalid(),
        ));
    }
    if reserved.len() >= MAX_ORIGINS {
        return Err(LocalOriginDiagnostic::from_io(
            Phase::OriginCapacity,
            &io::Error::other("The retained local-origin catalog is at capacity"),
        ));
    }
    // Windows may allocate consecutive ephemeral ports. A fixed retry count
    // can end inside a contiguous retained range despite other free ports.
    // Keep collisions bound so the kernel cannot offer them again: with N
    // reserved ports, N+1 distinct successful binds must include a new port.
    // These temporary listeners retire on every return, before publication.
    let mut collisions = Vec::new();
    for _ in 0..=reserved.len() {
        let candidate = bind(requested)
            .map_err(|error| LocalOriginDiagnostic::from_io(Phase::CandidateBind, &error))?;
        let port = address(&candidate)
            .map_err(|error| LocalOriginDiagnostic::from_io(Phase::CandidateAddress, &error))?;
        if !reserved.contains(&port) {
            return Ok(candidate);
        }
        collisions.push(candidate);
    }
    Err(LocalOriginDiagnostic::from_io(
        Phase::CandidateExhausted,
        &invalid(),
    ))
}
struct Descriptor(PSECURITY_DESCRIPTOR);
impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0.0)));
        }
    }
}
fn descriptor(text: &str) -> io::Result<Descriptor> {
    let mut value = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(text),
            1,
            &mut value,
            None,
        )
    }
    .map_err(io::Error::other)?;
    Ok(Descriptor(value))
}
fn sddl(value: PSECURITY_DESCRIPTOR) -> io::Result<String> {
    let mut text = PWSTR::null();
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            value,
            1,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut text,
            None,
        )
    }
    .map_err(io::Error::other)?;
    let result = unsafe { text.to_string() }.map_err(io::Error::other);
    unsafe {
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
    }
    result
}
pub(crate) struct Root {
    pub profile: weftext_core::QualifiedProfile,
    _directory: File,
}
impl Root {
    pub fn qualify(create: bool) -> io::Result<Self> {
        Self::at(crate::host_paths::host_state_dir()?, create)
    }
    fn at(path: PathBuf, create: bool) -> io::Result<Self> {
        let sid = crate::broker::security::current_sid()?;
        let expected = descriptor(&format!("O:{sid}D:P(A;OICI;FA;;;{sid})"))?;
        if create {
            let parent = path.parent().ok_or_else(invalid)?;
            let _parent = weftext_core::qualify_profile(parent).map_err(io::Error::other)?;
            let attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: expected.0.0,
                bInheritHandle: false.into(),
            };
            match unsafe { CreateDirectoryW(&HSTRING::from(path.as_os_str()), Some(&attributes)) } {
                Ok(()) => {}
                Err(error) if error.code().0 == 0x800700b7u32 as i32 => {}
                Err(error) => return Err(io::Error::other(error)),
            }
        }
        let profile = weftext_core::qualify_existing_profile(&path).map_err(io::Error::other)?;
        let directory = OpenOptions::new()
            .access_mode(READ_CONTROL.0)
            .share_mode(FILE_SHARE_READ.0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(profile.path())?;
        if directory.metadata()?.file_attributes() & 0x400 != 0 {
            return Err(invalid());
        }
        let mut actual = PSECURITY_DESCRIPTOR::default();
        unsafe {
            GetSecurityInfo(
                HANDLE(directory.as_raw_handle()),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                None,
                None,
                None,
                None,
                Some(&mut actual),
            )
        }
        .ok()
        .map_err(io::Error::other)?;
        let actual = Descriptor(actual);
        // Existing unqualified directories are retained. This code never fixes
        // their ACL/owner or adopts an arbitrary preexisting host-state tree.
        if sddl(actual.0)? != sddl(expected.0)? {
            return Err(invalid());
        }
        let root = Self {
            profile,
            _directory: directory,
        };
        // A publisher may rename its staged file between read_dir and metadata.
        // Qualification must inspect members under the same physical lock as
        // publication, not interpret that live transition as corrupt evidence.
        drop(root.locked().map_err(|error| {
            LocalOriginDiagnostic::from_io(LocalOriginPhase::CatalogLock, &error).into_io()
        })?);
        Ok(root)
    }
    fn inspect_members(&self) -> io::Result<()> {
        let mut members = 0;
        for member in std::fs::read_dir(self.profile.path())? {
            members += 1;
            if members > 256 {
                return Err(invalid());
            }
            let member = member?;
            let name = member.file_name();
            let name = name.to_str().ok_or_else(invalid)?;
            let staged = name
                .strip_prefix(".host-routing.")
                .and_then(|name| name.strip_suffix(".pending"))
                .is_some_and(crate::broker::canonical_uuid);
            if name != LEAF && name != ".host-routing.lock" && !staged {
                return Err(invalid());
            }
            let metadata = member.metadata()?;
            if !metadata.is_file()
                || metadata.file_attributes() & (0x10 | 0x400) != 0
                || metadata.len() > LIMIT as u64
            {
                return Err(invalid());
            }
            if staged {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "An original host routing publication remains unresolved; its bytes were retained",
                ));
            }
        }
        Ok(())
    }
    pub fn separate(&self, profile: &weftext_core::QualifiedProfile) -> io::Result<()> {
        if self
            .profile
            .contains_profile(profile)
            .map_err(io::Error::other)?
            || profile
                .contains_profile(&self.profile)
                .map_err(io::Error::other)?
        {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Core and host-state must be separate physical directory trees",
            ))
        } else {
            Ok(())
        }
    }
    fn locked(&self) -> io::Result<Locked<'_>> {
        self.locked_with_limit(LOCK_WAIT)
    }
    fn locked_with_limit(&self, limit: Duration) -> io::Result<Locked<'_>> {
        let path = self.profile.path().join(".host-routing.lock");
        let file = match file_options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                file_options().read(true).write(true).open(&path)?
            }
            Err(error) => return Err(error),
        };
        ordinary(&file)?;
        if file.metadata()?.len() != 0 {
            return Err(invalid());
        }
        let started = Instant::now();
        loop {
            match file.try_lock() {
                Ok(()) => break,
                Err(TryLockError::Error(error)) => return Err(error),
                Err(TryLockError::WouldBlock) => {
                    let remaining = limit.saturating_sub(started.elapsed());
                    if remaining.is_zero() {
                        return Err(io::Error::new(
                            io::ErrorKind::WouldBlock,
                            "The private activation routing catalog is busy; no routing or OS effect was attempted",
                        ));
                    }
                    // This waits only for another original catalog operation.
                    // It never retries publication, an OS request or a failed
                    // qualification, and does not acquire another Core owner.
                    std::thread::sleep(remaining.min(Duration::from_millis(10)));
                }
            }
        }
        self.inspect_members()?;
        Ok(Locked {
            root: self,
            _lock: file,
        })
    }
    pub fn register(
        &self,
        profile: &weftext_core::QualifiedProfile,
        device: &weftext_core::DeviceIdentity,
        opaque: &str,
        execution_key: &str,
        app_id: &str,
    ) -> io::Result<()> {
        self.separate(profile)?;
        if !device.is_current
            || !crate::broker::canonical_uuid(&device.id)
            || !crate::broker::canonical_uuid(opaque)
            || !sha(execution_key)
            || !application_id(app_id)
        {
            return Err(invalid());
        }
        let mut locked = self.locked()?;
        let mut catalog = locked.read()?.unwrap_or_else(Catalog::empty);
        let profile = Profile {
            physical_identity: profile.physical_identity().into(),
            locator: profile.path().to_str().ok_or_else(invalid)?.into(),
            device_id: device.id.clone(),
        };
        let changed = catalog.remember_profile(&profile)?;
        let activation = Activation {
            opaque: opaque.into(),
            physical_identity: profile.physical_identity,
            device_id: device.id.clone(),
            execution_key: execution_key.into(),
            app_id: app_id.into(),
            build_id: crate::artifact_catalog::Catalog::get()?.build_id().into(),
        };
        match catalog.activations.iter().find(|old| old.opaque == opaque) {
            Some(old) if old == &activation => {
                if !changed {
                    return Ok(());
                }
            }
            Some(_) => return Err(invalid()),
            None => catalog.activations.push(activation),
        }
        catalog.revision = catalog.revision.checked_add(1).ok_or_else(invalid)?;
        locked.publish(&catalog)
    }
    pub fn lookup(&self, opaque: &str) -> io::Result<Route> {
        if !crate::broker::canonical_uuid(opaque) {
            return Err(invalid());
        }
        let locked = self.locked()?;
        let catalog = locked.read()?.ok_or_else(invalid)?;
        let activation = catalog
            .activations
            .iter()
            .find(|entry| entry.opaque == opaque)
            .ok_or_else(invalid)?;
        if activation.build_id != crate::artifact_catalog::Catalog::get()?.build_id() {
            return Err(invalid());
        }
        let profile = catalog
            .profiles
            .iter()
            .find(|entry| {
                entry.physical_identity == activation.physical_identity
                    && entry.device_id == activation.device_id
            })
            .ok_or_else(invalid)?;
        let actual =
            weftext_core::qualify_existing_profile(&profile.locator).map_err(io::Error::other)?;
        if actual.physical_identity() != profile.physical_identity {
            return Err(invalid());
        }
        self.separate(&actual)?;
        Ok(Route {
            profile: actual,
            device_id: profile.device_id.clone(),
            execution_key: activation.execution_key.clone(),
            app_id: activation.app_id.clone(),
        })
    }

    /// A single application-wide startup task selects one existing Core. This
    /// locator is not OS enablement and never substitutes for Core's intent.
    pub fn change_startup<T>(
        &self,
        profile: &weftext_core::QualifiedProfile,
        device: &weftext_core::DeviceIdentity,
        enabled: bool,
        request: impl FnOnce() -> io::Result<T>,
    ) -> io::Result<Option<T>> {
        self.separate(profile)?;
        if !device.is_current || !crate::broker::canonical_uuid(&device.id) {
            return Err(invalid());
        }
        let profile = Profile {
            physical_identity: profile.physical_identity().into(),
            locator: profile.path().to_str().ok_or_else(invalid)?.into(),
            device_id: device.id.clone(),
        };
        let mut locked = self.locked()?;
        let mut catalog = locked.read()?.unwrap_or_else(Catalog::empty);
        let startup = Startup {
            physical_identity: profile.physical_identity.clone(),
            device_id: profile.device_id.clone(),
        };
        if enabled {
            let changed = catalog.remember_profile(&profile)?;
            if changed || catalog.startup.as_ref() != Some(&startup) {
                catalog.startup = Some(startup);
                catalog.revision = catalog.revision.checked_add(1).ok_or_else(invalid)?;
                locked.publish(&catalog)?;
            }
        } else if catalog.startup.as_ref() != Some(&startup) {
            // Another physical profile owns the application-wide registration.
            // In particular, never run Disable after releasing this selection
            // check: another owner could select its profile in that interval.
            return Ok(None);
        }
        // The same OS lock covers the durable choice and the one OS request.
        // This is called outside the Core lock on an accepted WinRT worker.
        // Failure retains the original choice; it never implies no OS effect.
        request().map(Some)
    }

    pub fn startup_profile(&self) -> io::Result<Option<ExistingProfile>> {
        let locked = self.locked()?;
        let Some(catalog) = locked.read()? else {
            return Ok(None);
        };
        let Some(startup) = &catalog.startup else {
            return Ok(None);
        };
        let record = catalog
            .profiles
            .iter()
            .find(|profile| {
                profile.physical_identity == startup.physical_identity
                    && profile.device_id == startup.device_id
            })
            .ok_or_else(invalid)?;
        let profile =
            weftext_core::qualify_existing_profile(&record.locator).map_err(io::Error::other)?;
        self.separate(&profile)?;
        if profile.physical_identity() != record.physical_identity {
            return Err(invalid());
        }
        Ok(Some(ExistingProfile {
            profile,
            device_id: record.device_id.clone(),
        }))
    }

    /// Reserve a stable browser origin without creating a device identity.
    /// The listener stays unexposed until Core/router qualification succeeds.
    pub fn bind_local(
        &self,
        profile: &weftext_core::QualifiedProfile,
        requested: u16,
    ) -> Result<std::net::TcpListener, LocalOriginDiagnostic> {
        self.bind_local_at(
            profile,
            requested,
            &crate::host_paths::user_data_path(profile).map_err(|error| {
                LocalOriginDiagnostic::from_io(LocalOriginPhase::UserDataPath, &error)
            })?,
        )
    }
    fn bind_local_at(
        &self,
        profile: &weftext_core::QualifiedProfile,
        requested: u16,
        user_data: &Path,
    ) -> Result<std::net::TcpListener, LocalOriginDiagnostic> {
        use LocalOriginPhase as Phase;
        let diagnostic = LocalOriginDiagnostic::from_io;
        self.separate(profile)
            .map_err(|error| diagnostic(Phase::ProfileSeparation, &error))?;
        let locator = std::fs::canonicalize(profile.path())
            .map_err(|error| diagnostic(Phase::ProfileCanonicalize, &error))?
            .to_str()
            .ok_or_else(|| diagnostic(Phase::ProfileCanonicalize, &invalid()))?
            .to_owned();
        let mut locked = self
            .locked()
            .map_err(|error| diagnostic(Phase::CatalogLock, &error))?;
        let mut catalog = locked
            .read()
            .map_err(|error| diagnostic(Phase::CatalogRead, &error))?
            .unwrap_or_else(Catalog::empty);
        let identity = profile.physical_identity();
        let found = catalog
            .origins
            .iter()
            .position(|origin| origin.physical_identity == identity);
        // A prior browser/UDF is original author-recovery evidence. Losing its
        // origin metadata never authorizes a fresh origin in the same profile.
        let retained_user_data = match weftext_core::qualify_existing_profile(user_data) {
            Ok(qualified) => Some(qualified),
            Err(error) if error.code == "missing" => None,
            Err(error) => {
                return Err(diagnostic(
                    Phase::BrowserDataQualification,
                    &io::Error::other(error),
                ));
            }
        };
        if found.is_none() && retained_user_data.is_some() {
            return Err(diagnostic(
                Phase::OriginMetadataMissing,
                &io::Error::new(
                    io::ErrorKind::InvalidData,
                    "The original browser data remains but its retained local origin is missing",
                ),
            ));
        }
        if catalog.origins.iter().any(|origin| {
            origin.physical_identity != identity && same_locator(&origin.locator, &locator)
        }) {
            return Err(diagnostic(Phase::LocatorConflict, &invalid()));
        }
        let listener = if let Some(index) = found {
            let origin = &catalog.origins[index];
            if requested != 0 && requested != origin.port {
                return Err(diagnostic(
                    Phase::RetainedPortMismatch,
                    &io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "This physical profile already has a different retained local origin",
                    ),
                ));
            }
            // Occupied/unknown does not change origin or discover an owner.
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, origin.port))
                .map_err(|error| diagnostic(Phase::RetainedBind, &error))?
        } else {
            let reserved = catalog.origins.iter().map(|origin| origin.port).collect();
            select_new_origin(
                &reserved,
                requested,
                |port| std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)),
                |listener| listener.local_addr().map(|address| address.port()),
            )?
        };
        let port = listener
            .local_addr()
            .map_err(|error| diagnostic(Phase::ListenerAddress, &error))?
            .port();
        let changed = if let Some(index) = found {
            let old = &mut catalog.origins[index];
            if old.locator == locator {
                false
            } else {
                old.locator = locator;
                true
            }
        } else {
            catalog.origins.push(Origin {
                physical_identity: identity.into(),
                locator,
                port,
            });
            true
        };
        if changed {
            catalog.revision = catalog
                .revision
                .checked_add(1)
                .ok_or_else(|| diagnostic(Phase::CatalogRevision, &invalid()))?;
            locked
                .publish(&catalog)
                .map_err(|error| diagnostic(Phase::CatalogPublish, &error))?;
        }
        listener
            .set_nonblocking(true)
            .map_err(|error| diagnostic(Phase::ListenerNonblocking, &error))?;
        Ok(listener)
    }
}
pub(crate) struct Route {
    pub profile: weftext_core::QualifiedProfile,
    pub device_id: String,
    pub execution_key: String,
    pub app_id: String,
}
pub(crate) struct ExistingProfile {
    pub profile: weftext_core::QualifiedProfile,
    pub device_id: String,
}
impl Route {
    pub fn into_existing(self) -> ExistingProfile {
        ExistingProfile {
            profile: self.profile,
            device_id: self.device_id,
        }
    }
}
fn file_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options
        .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
    options
}
fn ordinary(file: &File) -> io::Result<()> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & (0x10 | 0x400) != 0 {
        Err(invalid())
    } else {
        Ok(())
    }
}
fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn application_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}
fn same_locator(left: &str, right: &str) -> bool {
    let left: Vec<u16> = left.encode_utf16().collect();
    let right: Vec<u16> = right.encode_utf16().collect();
    (unsafe { windows::Win32::Globalization::CompareStringOrdinal(&left, &right, true) })
        == windows::Win32::Globalization::CSTR_EQUAL
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Origin {
    physical_identity: String,
    locator: String,
    port: u16,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Startup {
    physical_identity: String,
    device_id: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Profile {
    physical_identity: String,
    locator: String,
    device_id: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Activation {
    opaque: String,
    physical_identity: String,
    device_id: String,
    execution_key: String,
    app_id: String,
    build_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    revision: u64,
    profiles: Vec<Profile>,
    activations: Vec<Activation>,
    origins: Vec<Origin>,
    startup: Option<Startup>,
}
impl Catalog {
    fn empty() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            profiles: Vec::new(),
            activations: Vec::new(),
            origins: Vec::new(),
            startup: None,
        }
    }
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != 1
            || self.revision == 0
            || self.profiles.len() > 128
            || self.activations.len() > 32768
            || self.origins.len() > MAX_ORIGINS
        {
            return Err(invalid());
        }
        let mut profiles = BTreeSet::new();
        let mut activations = BTreeSet::new();
        let mut origins = BTreeSet::new();
        let mut ports = BTreeSet::new();
        for origin in &self.origins {
            if origin.physical_identity.is_empty()
                || origin.physical_identity.len() > 256
                || !origins.insert(&origin.physical_identity)
                || origin.port == 0
                || !ports.insert(origin.port)
                || origin.locator.is_empty()
                || origin.locator.len() > 32768
                || origin.locator.contains('\0')
                || !Path::new(&origin.locator).is_absolute()
            {
                return Err(invalid());
            }
        }
        for profile in &self.profiles {
            if profile.physical_identity.is_empty()
                || profile.physical_identity.len() > 256
                || !profiles.insert(&profile.physical_identity)
                || !crate::broker::canonical_uuid(&profile.device_id)
                || profile.locator.is_empty()
                || profile.locator.len() > 32768
                || profile.locator.contains('\0')
                || !Path::new(&profile.locator).is_absolute()
            {
                return Err(invalid());
            }
        }
        for activation in &self.activations {
            if !crate::broker::canonical_uuid(&activation.opaque)
                || !activations.insert(&activation.opaque)
                || !sha(&activation.execution_key)
                || !sha(&activation.build_id)
                || !application_id(&activation.app_id)
                || !self.profiles.iter().any(|profile| {
                    profile.physical_identity == activation.physical_identity
                        && profile.device_id == activation.device_id
                })
            {
                return Err(invalid());
            }
        }
        if self.startup.as_ref().is_some_and(|startup| {
            !self.profiles.iter().any(|profile| {
                profile.physical_identity == startup.physical_identity
                    && profile.device_id == startup.device_id
            })
        }) {
            return Err(invalid());
        }
        Ok(())
    }
    fn remember_profile(&mut self, profile: &Profile) -> io::Result<bool> {
        match self
            .profiles
            .iter_mut()
            .find(|old| old.physical_identity == profile.physical_identity)
        {
            Some(old) if old.device_id != profile.device_id => Err(invalid()),
            Some(old) => {
                if old.locator == profile.locator {
                    Ok(false)
                } else {
                    old.locator = profile.locator.clone();
                    Ok(true)
                }
            }
            None => {
                self.profiles.push(profile.clone());
                Ok(true)
            }
        }
    }
}
struct Locked<'a> {
    root: &'a Root,
    _lock: File,
}
impl Locked<'_> {
    fn read(&self) -> io::Result<Option<Catalog>> {
        let mut file = match file_options()
            .read(true)
            .open(self.root.profile.path().join(LEAF))
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        ordinary(&file)?;
        let metadata = file.metadata()?;
        if metadata.len() > LIMIT as u64 {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > LIMIT
            || file.metadata()?.len() != metadata.len()
            || file.metadata()?.modified()? != metadata.modified()?
        {
            return Err(invalid());
        }
        let catalog: Catalog = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        catalog.validate()?;
        Ok(Some(catalog))
    }
    fn publish(&mut self, catalog: &Catalog) -> io::Result<()> {
        catalog.validate()?;
        let bytes = serde_json::to_vec(catalog).map_err(io::Error::other)?;
        if bytes.len() > LIMIT {
            return Err(invalid());
        }
        let temporary = self
            .root
            .profile
            .path()
            .join(format!(".host-routing.{}.pending", uuid::Uuid::new_v4()));
        let mut file = file_options()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        ordinary(&file)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        // Only same-volume exact-name publication. Failure keeps the candidate;
        // no delete, ACL repair, cross-volume copy or retry fallback is used.
        unsafe {
            MoveFileExW(
                &HSTRING::from(temporary.as_os_str()),
                &HSTRING::from(self.root.profile.path().join(LEAF).as_os_str()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(io::Error::other)?;
        let actual = self.read()?.ok_or_else(invalid)?;
        if serde_json::to_vec(&actual).map_err(io::Error::other)? != bytes {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bind(
        root: &Root,
        profile: &weftext_core::QualifiedProfile,
        port: u16,
    ) -> Result<std::net::TcpListener, LocalOriginDiagnostic> {
        root.bind_local_at(
            profile,
            port,
            &profile.path().with_extension("test-webview2"),
        )
    }

    fn bytes(root: &Root) -> Vec<u8> {
        std::fs::read(root.profile.path().join(LEAF)).unwrap()
    }

    #[derive(Debug)]
    struct Candidate {
        port: u16,
        retired: std::rc::Rc<std::cell::RefCell<Vec<u16>>>,
    }
    impl Drop for Candidate {
        fn drop(&mut self) {
            self.retired.borrow_mut().push(self.port);
        }
    }

    #[test]
    fn new_origin_crosses_long_retained_ranges_and_retires_collisions_before_return() {
        let reserved = (50000..54097).collect();
        let retired = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let calls = std::cell::Cell::new(0_u16);
        let selected = select_new_origin(
            &reserved,
            0,
            |requested| {
                assert_eq!(requested, 0);
                assert!(
                    retired.borrow().is_empty(),
                    "a prior candidate was released too soon"
                );
                let port = 50000 + calls.get();
                calls.set(calls.get() + 1);
                Ok(Candidate {
                    port,
                    retired: retired.clone(),
                })
            },
            |candidate| Ok(candidate.port),
        )
        .unwrap();
        assert_eq!(calls.get(), 4098);
        assert_eq!(selected.port, 54097);
        let mut actual = retired.borrow().clone();
        actual.sort_unstable();
        assert_eq!(actual, (50000..54097).collect::<Vec<_>>());
        drop(selected);
        assert_eq!(retired.borrow().len(), 4098);
    }

    #[test]
    fn new_origin_preserves_os_failures_and_retires_all_held_candidates() {
        for (address_fails, os_code) in [(false, 10055), (false, 10013), (true, 87)] {
            let retired = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let calls = std::cell::Cell::new(0_u16);
            let error = select_new_origin(
                &[50000].into_iter().collect(),
                0,
                |_| {
                    assert!(retired.borrow().is_empty());
                    let index = calls.get();
                    calls.set(index + 1);
                    if index == 1 && !address_fails {
                        return Err(io::Error::from_raw_os_error(os_code));
                    }
                    Ok(Candidate {
                        port: 50000 + index,
                        retired: retired.clone(),
                    })
                },
                |candidate| {
                    if address_fails && candidate.port == 50001 {
                        Err(io::Error::from_raw_os_error(os_code))
                    } else {
                        Ok(candidate.port)
                    }
                },
            )
            .unwrap_err();
            assert_eq!(calls.get(), 2);
            assert_eq!(retired.borrow().len(), if address_fails { 2 } else { 1 });
            assert_eq!(
                serde_json::to_value(error).unwrap(),
                serde_json::json!({
                    "phase": if address_fails { "candidateAddress" } else { "candidateBind" },
                    "ioKind": format!("{:?}", io::Error::from_raw_os_error(os_code).kind()),
                    "osCode": os_code,
                })
            );
        }
    }

    #[test]
    fn new_origin_exhaustion_is_bounded_even_for_a_repeating_binder() {
        let retired = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let calls = std::cell::Cell::new(0);
        let error = select_new_origin(
            &[50000, 50001].into_iter().collect(),
            0,
            |_| {
                assert!(retired.borrow().is_empty());
                calls.set(calls.get() + 1);
                Ok(Candidate {
                    port: 50000,
                    retired: retired.clone(),
                })
            },
            |candidate| Ok(candidate.port),
        )
        .unwrap_err();
        assert_eq!(calls.get(), 3);
        assert_eq!(retired.borrow().len(), 3);
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!({
                "phase": "candidateExhausted", "ioKind": "PermissionDenied", "osCode": null,
            })
        );
    }

    #[test]
    fn new_origin_capacity_and_reserved_requests_never_bind_a_candidate() {
        let full = (1..=MAX_ORIGINS as u16).collect();
        let capacity = select_new_origin(
            &full,
            0,
            |_| -> io::Result<u16> { panic!("capacity must be checked before bind") },
            |port| Ok(*port),
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(capacity).unwrap(),
            serde_json::json!({
                "phase": "originCapacity", "ioKind": "Other", "osCode": null,
            })
        );
        let reserved = select_new_origin(
            &[50000].into_iter().collect(),
            50000,
            |_| -> io::Result<u16> {
                panic!("a retained port must not be allocated to a new origin")
            },
            |port| Ok(*port),
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(reserved).unwrap(),
            serde_json::json!({
                "phase": "requestedPortReserved", "ioKind": "PermissionDenied", "osCode": null,
            })
        );
    }

    #[test]
    fn old_4096_record_catalog_admits_new_profile_without_changing_old_bindings() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("existing")).unwrap();
        let listener = bind(&root, &profile, 0).unwrap();
        let retained = listener.local_addr().unwrap().port();
        drop(listener);
        let mut locked = root.locked().unwrap();
        let mut catalog = locked.read().unwrap().unwrap();
        for port in (1..=u16::MAX).filter(|port| *port != retained).take(4095) {
            catalog.origins.push(Origin {
                physical_identity: format!("fixture-{port}"),
                locator: temp
                    .path()
                    .join(format!("unused-{port}"))
                    .to_string_lossy()
                    .into_owned(),
                port,
            });
        }
        catalog.revision += 1;
        locked.publish(&catalog).unwrap();
        let old = serde_json::to_value(&catalog.origins).unwrap();
        let ports: BTreeSet<_> = catalog.origins.iter().map(|origin| origin.port).collect();
        drop(locked);
        let new_profile = weftext_core::qualify_profile(temp.path().join("new")).unwrap();
        let listener = bind(&root, &new_profile, 0).unwrap();
        assert!(!ports.contains(&listener.local_addr().unwrap().port()));
        let current = root.locked().unwrap().read().unwrap().unwrap();
        assert_eq!(current.origins.len(), 4097);
        assert_eq!(serde_json::to_value(&current.origins[..4096]).unwrap(), old);
        drop(listener);
        let reopened = bind(&root, &profile, 0).unwrap();
        assert_eq!(reopened.local_addr().unwrap().port(), retained);
    }

    #[test]
    fn full_catalog_refuses_new_profiles_but_preserves_existing_origins() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("existing")).unwrap();
        let listener = bind(&root, &profile, 0).unwrap();
        let retained = listener.local_addr().unwrap().port();
        drop(listener);
        let mut locked = root.locked().unwrap();
        let mut catalog = locked.read().unwrap().unwrap();
        for port in (1..=u16::MAX)
            .filter(|port| *port != retained)
            .take(MAX_ORIGINS - 1)
        {
            catalog.origins.push(Origin {
                physical_identity: format!("fixture-{port}"),
                locator: temp
                    .path()
                    .join(format!("unused-{port}"))
                    .to_string_lossy()
                    .into_owned(),
                port,
            });
        }
        catalog.revision += 1;
        locked.publish(&catalog).unwrap();
        drop(locked);
        let original = bytes(&root);
        let new_profile = weftext_core::qualify_profile(temp.path().join("new")).unwrap();
        assert_eq!(
            serde_json::to_value(bind(&root, &new_profile, 0).unwrap_err()).unwrap(),
            serde_json::json!({ "phase": "originCapacity", "ioKind": "Other", "osCode": null })
        );
        assert_eq!(bytes(&root), original);
        let reopened = bind(&root, &profile, 0).unwrap();
        assert_eq!(reopened.local_addr().unwrap().port(), retained);
        assert_eq!(bytes(&root), original);
    }

    #[test]
    fn local_origin_survives_reopen_without_creating_device_records() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        let first = bind(&root, &profile, 0).unwrap();
        let port = first.local_addr().unwrap().port();
        let original = bytes(&root);
        assert!(!profile.path().join("device-state").exists());
        let stored: Catalog = serde_json::from_slice(&original).unwrap();
        assert!(stored.profiles.is_empty());
        assert!(stored.activations.is_empty());
        assert_eq!(stored.origins.len(), 1);
        assert_eq!(stored.origins[0].port, port);
        drop(first);
        drop(root);
        let reopened = Root::at(temp.path().join("host-state"), false).unwrap();
        let second = bind(&reopened, &profile, 0).unwrap();
        assert_eq!(second.local_addr().unwrap().port(), port);
        assert_eq!(bytes(&reopened), original);
    }

    #[test]
    fn occupied_retained_port_and_explicit_change_keep_original_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        let listener = bind(&root, &profile, 0).unwrap();
        let original = bytes(&root);
        let occupied = serde_json::to_value(bind(&root, &profile, 0).unwrap_err()).unwrap();
        assert_eq!(occupied["phase"], "retainedBind");
        let os_code = i32::try_from(occupied["osCode"].as_i64().unwrap()).unwrap();
        assert_eq!(
            occupied["ioKind"],
            format!("{:?}", io::Error::from_raw_os_error(os_code).kind())
        );
        assert_eq!(bytes(&root), original);
        let different = if listener.local_addr().unwrap().port() == 1 {
            2
        } else {
            1
        };
        assert_eq!(
            serde_json::to_value(bind(&root, &profile, different).unwrap_err()).unwrap(),
            serde_json::json!({
                "phase": "retainedPortMismatch", "ioKind": "InvalidInput", "osCode": null
            })
        );
        assert_eq!(bytes(&root), original);
    }

    #[test]
    fn other_profile_cannot_take_a_retained_origin_after_its_listener_stops() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let first = weftext_core::qualify_profile(temp.path().join("first")).unwrap();
        let listener = bind(&root, &first, 0).unwrap();
        let reserved = listener.local_addr().unwrap().port();
        drop(listener);
        let second = weftext_core::qualify_profile(temp.path().join("second")).unwrap();
        let original = bytes(&root);
        assert!(bind(&root, &second, reserved).is_err());
        assert_eq!(bytes(&root), original);
        let allocated = bind(&root, &second, 0).unwrap();
        assert_ne!(allocated.local_addr().unwrap().port(), reserved);
    }

    #[test]
    fn replaced_physical_profile_does_not_get_a_new_origin_at_the_old_locator() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let path = temp.path().join("core");
        {
            let original = weftext_core::qualify_profile(&path).unwrap();
            drop(bind(&root, &original, 0).unwrap());
        }
        let metadata = bytes(&root);
        std::fs::rename(&path, temp.path().join("retained-original")).unwrap();
        let replacement = weftext_core::qualify_profile(&path).unwrap();
        assert!(bind(&root, &replacement, 0).is_err());
        assert_eq!(bytes(&root), metadata);
    }

    #[test]
    fn corrupt_origin_evidence_is_retained_without_binding_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        drop(bind(&root, &profile, 0).unwrap());
        let bad = b"{\"schemaVersion\":1,\"revision\":1,\"origins\":[],\"unexpected\":true}";
        std::fs::write(root.profile.path().join(LEAF), bad).unwrap();
        assert_eq!(
            serde_json::to_value(bind(&root, &profile, 0).unwrap_err()).unwrap()["phase"],
            "catalogRead"
        );
        assert_eq!(bytes(&root), bad);
    }

    #[test]
    fn strict_host_state_rejects_unknown_members_and_both_overlap_directions() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("host-state");
        let root = Root::at(path.clone(), true).unwrap();
        let nested = weftext_core::qualify_profile(path.join("invalid-core")).unwrap();
        let ancestor = weftext_core::qualify_existing_profile(temp.path()).unwrap();
        assert!(root.separate(&nested).is_err());
        assert!(root.separate(&ancestor).is_err());
        drop(nested);
        let original = b"retained unknown evidence";
        std::fs::write(path.join("unknown.bin"), original).unwrap();
        assert!(Root::at(path.clone(), false).is_err());
        assert_eq!(std::fs::read(path.join("unknown.bin")).unwrap(), original);
    }

    #[test]
    fn missing_origin_with_existing_user_data_keeps_both_originals() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        let user_data = profile.path().with_extension("test-webview2");
        std::fs::create_dir(&user_data).unwrap();
        std::fs::write(user_data.join("retained-input"), b"original input").unwrap();
        assert_eq!(
            serde_json::to_value(bind(&root, &profile, 0).unwrap_err()).unwrap(),
            serde_json::json!({
                "phase": "originMetadataMissing", "ioKind": "InvalidData", "osCode": null
            })
        );
        assert!(!root.profile.path().join(LEAF).exists());
        assert_eq!(
            std::fs::read(user_data.join("retained-input")).unwrap(),
            b"original input"
        );
    }

    #[test]
    fn unresolved_publication_cannot_be_bypassed_by_a_later_bind() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        drop(bind(&root, &profile, 0).unwrap());
        let original = bytes(&root);
        let pending = root
            .profile
            .path()
            .join(format!(".host-routing.{}.pending", uuid::Uuid::new_v4()));
        let partial = b"retained incomplete publication";
        std::fs::write(&pending, partial).unwrap();
        assert_eq!(
            serde_json::to_value(bind(&root, &profile, 0).unwrap_err()).unwrap()["phase"],
            "catalogLock"
        );
        assert_eq!(bytes(&root), original);
        assert_eq!(std::fs::read(pending).unwrap(), partial);
    }

    #[test]
    fn simultaneous_first_origins_are_serialized_and_remain_stable() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("host-state");
        let root = Root::at(path.clone(), true).unwrap();
        let other = Root::at(path, false).unwrap();
        let first = weftext_core::qualify_profile(temp.path().join("first")).unwrap();
        let second = weftext_core::qualify_profile(temp.path().join("second")).unwrap();
        let start = std::sync::Barrier::new(2);
        let (first_listener, second_listener) = std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                start.wait();
                bind(&other, &second, 0).unwrap()
            });
            start.wait();
            let first_listener = bind(&root, &first, 0).unwrap();
            (first_listener, worker.join().unwrap())
        });
        let first_port = first_listener.local_addr().unwrap().port();
        let second_port = second_listener.local_addr().unwrap().port();
        assert_ne!(first_port, second_port);
        let original = bytes(&root);
        let catalog: Catalog = serde_json::from_slice(&original).unwrap();
        assert_eq!(catalog.revision, 2);
        assert_eq!(catalog.origins.len(), 2);
        assert!(catalog.profiles.is_empty());
        assert!(catalog.activations.is_empty());
        drop((first_listener, second_listener));
        assert_eq!(
            bind(&root, &first, 0).unwrap().local_addr().unwrap().port(),
            first_port
        );
        assert_eq!(
            bind(&other, &second, 0)
                .unwrap()
                .local_addr()
                .unwrap()
                .port(),
            second_port
        );
        assert_eq!(bytes(&root), original);
    }

    #[test]
    fn qualification_waits_for_the_original_publication_before_inspecting_members() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("host-state");
        let root = Root::at(path.clone(), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        drop(bind(&root, &profile, 0).unwrap());
        let mut locked = root.locked().unwrap();
        let mut catalog = locked.read().unwrap().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (completed_tx, completed_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                started_tx.send(()).unwrap();
                let reopened = Root::at(path, false).unwrap();
                completed_tx.send(bytes(&reopened)).unwrap();
            });
            started_rx.recv().unwrap();
            assert!(matches!(
                completed_rx.recv_timeout(Duration::from_millis(50)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ));
            // Use the real publisher while the other qualifier is waiting.
            // It must observe the completed revision, not the live staged path.
            catalog.revision += 1;
            locked.publish(&catalog).unwrap();
            let original = bytes(&root);
            drop(locked);
            assert_eq!(completed_rx.recv_timeout(LOCK_WAIT).unwrap(), original);
            worker.join().unwrap();
        });
    }

    #[test]
    fn busy_routing_lock_is_bounded_without_changing_original_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("host-state");
        let root = Root::at(path.clone(), true).unwrap();
        let other = Root::at(path, false).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        drop(bind(&root, &profile, 0).unwrap());
        let original = bytes(&root);
        let locked = root.locked().unwrap();
        let failure = other
            .locked_with_limit(Duration::from_millis(20))
            .err()
            .expect("the original operation must retain its lock");
        assert_eq!(failure.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(bytes(&root), original);
        drop(locked);
        assert!(other.locked_with_limit(Duration::ZERO).is_ok());
        assert_eq!(bytes(&root), original);
    }

    #[test]
    fn released_lock_does_not_adopt_an_unresolved_publication() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("host-state");
        let root = Root::at(path.clone(), true).unwrap();
        let profile = weftext_core::qualify_profile(temp.path().join("core")).unwrap();
        drop(bind(&root, &profile, 0).unwrap());
        let original = bytes(&root);
        let locked = root.locked().unwrap();
        let pending = path.join(format!(".host-routing.{}.pending", uuid::Uuid::new_v4()));
        let partial = b"original unconfirmed publication";
        std::fs::write(&pending, partial).unwrap();
        drop(locked);
        let failure = Root::at(path, false).err().unwrap();
        assert_eq!(failure.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            serde_json::to_value(LocalOriginDiagnostic::from_io(
                LocalOriginPhase::RootQualification,
                &failure,
            ))
            .unwrap(),
            serde_json::json!({
                "phase": "catalogLock", "ioKind": "InvalidData", "osCode": null
            })
        );
        assert_eq!(bytes(&root), original);
        assert_eq!(std::fs::read(pending).unwrap(), partial);
    }

    #[test]
    fn startup_selection_and_os_request_hold_one_physical_lock() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let other = Root::at(temp.path().join("host-state"), false).unwrap();
        let mut core = weftext_core::Core::new(temp.path().join("core")).unwrap();
        let device = core.device_identity().unwrap();
        let profile = weftext_core::qualify_existing_profile(temp.path().join("core")).unwrap();
        let value = root
            .change_startup(&profile, &device, true, || {
                // No contender may pass the original application's OS effect.
                // An immediate probe avoids waiting for this same-thread owner.
                assert_eq!(
                    other
                        .locked_with_limit(Duration::ZERO)
                        .err()
                        .unwrap()
                        .kind(),
                    io::ErrorKind::WouldBlock
                );
                let catalog: Catalog = serde_json::from_slice(&bytes(&root)).unwrap();
                assert_eq!(
                    catalog.startup.as_ref().unwrap().physical_identity,
                    profile.physical_identity()
                );
                Ok(23)
            })
            .unwrap();
        assert_eq!(value, Some(23));
        assert!(other.locked().is_ok());
        let original = bytes(&root);
        let error = root.change_startup(&profile, &device, true, || {
            Err::<(), _>(io::Error::other("unconfirmed one OS request"))
        });
        assert!(error.is_err());
        assert_eq!(bytes(&root), original);
    }

    #[test]
    fn disabling_an_unselected_profile_cannot_call_the_application_wide_os_task() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::at(temp.path().join("host-state"), true).unwrap();
        let mut first_core = weftext_core::Core::new(temp.path().join("first")).unwrap();
        let mut second_core = weftext_core::Core::new(temp.path().join("second")).unwrap();
        let first_device = first_core.device_identity().unwrap();
        let second_device = second_core.device_identity().unwrap();
        let first = weftext_core::qualify_existing_profile(temp.path().join("first")).unwrap();
        let second = weftext_core::qualify_existing_profile(temp.path().join("second")).unwrap();
        root.change_startup(&first, &first_device, true, || Ok(()))
            .unwrap();
        let original = bytes(&root);
        let result = root.change_startup(&second, &second_device, false, || {
            panic!("another profile's startup task must not be disabled")
        });
        assert_eq!(result.unwrap(), None::<()>);
        assert_eq!(bytes(&root), original);
        assert_eq!(
            root.startup_profile()
                .unwrap()
                .unwrap()
                .profile
                .physical_identity(),
            first.physical_identity()
        );
    }
}
