//! Core, host-state and persistent browser data have separate owners.
//! The WebView UDF may hold the only unsent author input; it is not disposable cache.
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

fn browser_data_dir() -> io::Result<PathBuf> {
    Ok(local_app_data()?.join(relative(
        &crate::host_contract::get()?
            .paths
            .browser_data_relative_root,
    )?))
}

fn user_data_path_at(browser_data: &Path, physical_identity: &str) -> io::Result<PathBuf> {
    let key = physical_identity
        .rsplit(':')
        .next()
        .filter(|key| {
            key.len() == 64
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or_else(|| io::Error::other("Profile identity is not canonical"))?;
    Ok(browser_data.join(key).join("webview2"))
}

pub(crate) fn user_data_path(profile: &weftext_core::QualifiedProfile) -> io::Result<PathBuf> {
    user_data_path_at(&browser_data_dir()?, profile.physical_identity())
}

/// Qualify the reserved sibling namespaces before creating the requested Core.
/// In particular an invalid data-dir inside host-state cannot first create an
/// inherited-ACL directory that poisons the catalog's later private creation.
pub(crate) fn qualify_core(data: &Path) -> io::Result<weftext_core::QualifiedProfile> {
    let host = crate::host_state::Root::qualify(true)?;
    let browser_data =
        weftext_core::qualify_profile(browser_data_dir()?).map_err(io::Error::other)?;
    let absolute = std::path::absolute(data)?;
    let mut ancestor = absolute.as_path();
    loop {
        match weftext_core::qualify_existing_profile(ancestor) {
            Ok(actual) => {
                if host
                    .profile
                    .contains_profile(&actual)
                    .map_err(io::Error::other)?
                    || browser_data
                        .contains_profile(&actual)
                        .map_err(io::Error::other)?
                    || ancestor == absolute
                        && (actual
                            .contains_profile(&host.profile)
                            .map_err(io::Error::other)?
                            || actual
                                .contains_profile(&browser_data)
                                .map_err(io::Error::other)?)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "The requested Core data-dir overlaps reserved host-state or persistent browser data",
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
    pub user_data: PathBuf,
    _user_data_guard: weftext_core::QualifiedProfile,
}

impl NativePaths {
    pub(crate) fn qualify(profile: &weftext_core::QualifiedProfile) -> io::Result<Self> {
        let browser_data = browser_data_dir()?;
        let browser_data =
            weftext_core::qualify_profile(&browser_data).map_err(io::Error::other)?;
        if profile
            .contains_profile(&browser_data)
            .map_err(io::Error::other)?
            || browser_data
                .contains_profile(profile)
                .map_err(io::Error::other)?
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The Core profile and persistent browser data must be separate physical namespaces",
            ));
        }
        let user_data = user_data_path(profile)?;
        let user_data_guard =
            weftext_core::qualify_profile(&user_data).map_err(io::Error::other)?;
        Ok(Self {
            user_data,
            _user_data_guard: user_data_guard,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_data_preserves_the_original_profile_udf_path() {
        let paths = &crate::host_contract::get().unwrap().paths;
        assert_eq!(paths.browser_data_known_folder, "LocalAppData");
        let local_app_data = Path::new(r"C:\Users\Example\AppData\Local");
        let browser_data =
            local_app_data.join(relative(&paths.browser_data_relative_root).unwrap());
        let key = "a".repeat(64);
        assert_eq!(
            user_data_path_at(&browser_data, &format!("windows:original:{key}")).unwrap(),
            local_app_data
                .join("Weftext")
                .join("Cache")
                .join(key)
                .join("webview2")
        );
    }

    #[test]
    fn browser_data_rejects_noncanonical_profile_keys() {
        let root = Path::new(r"C:\Users\Example\AppData\Local\Weftext\Cache");
        for key in [
            "".into(),
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
        ] {
            assert!(user_data_path_at(root, &format!("windows:original:{key}")).is_err());
        }
    }

    #[test]
    fn host_relative_paths_reject_namespace_escapes() {
        for path in [
            "",
            ".",
            "..",
            "../Weftext",
            "Weftext/../Cache",
            "/Weftext",
            r"C:\Weftext",
            r"\\server\share",
        ] {
            assert!(relative(path).is_err(), "{path}");
        }
        assert_eq!(
            relative("Weftext/Cache").unwrap(),
            Path::new("Weftext/Cache")
        );
    }
}
