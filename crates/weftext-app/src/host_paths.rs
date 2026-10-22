//! Windows durable and disposable paths have separate owners. WebView/cache
//! never lives inside the retained Core recovery namespace.
use std::{
    io,
    path::{Component, Path, PathBuf},
};
use windows::Win32::{
    System::Com::CoTaskMemFree,
    UI::Shell::{FOLDERID_LocalAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
};

pub(crate) fn local_app_data() -> io::Result<PathBuf> {
    let text = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None) }
        .map_err(|error| io::Error::other(error.to_string()))?;
    let path = unsafe { text.to_string() }
        .map(PathBuf::from)
        .map_err(|error| io::Error::other(error.to_string()));
    unsafe {
        CoTaskMemFree(Some(text.0.cast()));
    }
    path
}

fn relative(value: &str) -> io::Result<&Path> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The shared Windows path must be a nonempty relative normal path",
        ));
    }
    Ok(path)
}

pub(crate) fn default_data_dir() -> io::Result<PathBuf> {
    let paths = &crate::host_contract::get()?.paths;
    Ok(local_app_data()?
        .join(relative(&paths.persistent_relative_root)?)
        .join(relative(&paths.core_relative_path)?))
}

pub(crate) fn host_state_dir() -> io::Result<PathBuf> {
    let paths = &crate::host_contract::get()?.paths;
    Ok(local_app_data()?
        .join(relative(&paths.persistent_relative_root)?)
        .join(relative(&paths.host_state_relative_path)?))
}

fn cache_dir() -> io::Result<PathBuf> {
    Ok(local_app_data()?.join(relative(
        &crate::host_contract::get()?.paths.cache_relative_root,
    )?))
}

pub(crate) fn user_data_path(profile: &weftext_core::QualifiedProfile) -> io::Result<PathBuf> {
    let key = profile
        .physical_identity()
        .rsplit(':')
        .next()
        .filter(|key| {
            key.len() == 64
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or_else(|| io::Error::other("Profile identity is not canonical"))?;
    Ok(cache_dir()?.join(key).join("webview2"))
}

/// Qualify the reserved sibling namespaces before creating the requested Core.
/// In particular an invalid data-dir inside host-state cannot first create an
/// inherited-ACL directory that poisons the catalog's later private creation.
pub(crate) fn qualify_core(data: &Path) -> io::Result<weftext_core::QualifiedProfile> {
    let host = crate::host_state::Root::qualify(true)?;
    let cache = weftext_core::qualify_profile(cache_dir()?).map_err(io::Error::other)?;
    let absolute = std::path::absolute(data)?;
    let mut ancestor = absolute.as_path();
    loop {
        match weftext_core::qualify_existing_profile(ancestor) {
            Ok(actual) => {
                if host
                    .profile
                    .contains_profile(&actual)
                    .map_err(io::Error::other)?
                    || cache.contains_profile(&actual).map_err(io::Error::other)?
                    || ancestor == absolute
                        && (actual
                            .contains_profile(&host.profile)
                            .map_err(io::Error::other)?
                            || actual.contains_profile(&cache).map_err(io::Error::other)?)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "The requested Core data-dir overlaps a reserved host-state or disposable cache tree",
                    ));
                }
                break;
            }
            Err(error) if error.code == "missing" => {
                ancestor = ancestor.parent().ok_or_else(|| {
                    io::Error::other("The requested Core has no qualified ancestor")
                })?
            }
            Err(error) => return Err(io::Error::other(error)),
        }
    }
    let profile = weftext_core::qualify_profile(&absolute).map_err(io::Error::other)?;
    host.separate(&profile)?;
    Ok(profile)
}

pub(crate) struct NativePaths {
    pub data: PathBuf,
    pub physical_identity: String,
    pub user_data: PathBuf,
    _cache_guard: weftext_core::QualifiedProfile,
}

impl NativePaths {
    pub(crate) fn qualify(profile: &weftext_core::QualifiedProfile) -> io::Result<Self> {
        let cache = cache_dir()?;
        let cache = weftext_core::qualify_profile(&cache).map_err(io::Error::other)?;
        if profile.contains_profile(&cache).map_err(io::Error::other)?
            || cache.contains_profile(profile).map_err(io::Error::other)?
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The Core profile and disposable cache must be separate physical namespaces",
            ));
        }
        let user_data = user_data_path(profile)?;
        let cache_guard = weftext_core::qualify_profile(&user_data).map_err(io::Error::other)?;
        Ok(Self {
            data: profile.path().into(),
            physical_identity: profile.physical_identity().into(),
            user_data,
            _cache_guard: cache_guard,
        })
    }
}
