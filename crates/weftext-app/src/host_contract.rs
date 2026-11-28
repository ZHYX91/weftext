//! Shared with the one Windows package builder. Manifest and runtime cannot
//! independently choose IDs, persistent roots, role names or activation flags.
use serde::Deserialize;
use std::{io, sync::OnceLock};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Contract {
    pub schema_version: u32,
    pub identity: Identity,
    pub platform: Platform,
    pub roles: Roles,
    pub paths: Paths,
    pub activation: Activation,
    pub webview2: WebView,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Identity {
    pub name: String,
    pub publisher: String,
    pub application_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Platform {
    pub architecture: String,
    pub min_version: String,
    pub max_version_tested: String,
}

fn version(value: &str) -> Option<[u16; 4]> {
    value
        .split('.')
        .map(|part| {
            if part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
            {
                return None;
            }
            part.parse::<u16>().ok()
        })
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()
}

impl Platform {
    /// VerifyVersionInfoW compares major/minor/build, not servicing revisions.
    /// Keep the runtime gate and package MinVersion on the same build boundary.
    pub(crate) fn minimum_os_version(&self) -> Option<[u16; 3]> {
        let [major, minor, build, revision] = version(&self.min_version)?;
        (self.architecture == "x64" && [major, minor, build] >= [10, 0, 19045] && revision == 0)
            .then_some([major, minor, build])
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Roles {
    pub desktop: Role,
    pub cli: Role,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Role {
    pub cargo_bin: String,
    pub filename: String,
    pub pe_subsystem: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Paths {
    pub persistent_known_folder: String,
    pub persistent_relative_root: String,
    pub core_relative_path: String,
    pub host_state_relative_path: String,
    pub browser_data_known_folder: String,
    pub browser_data_relative_root: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Activation {
    pub portable_aumid: String,
    pub startup_task_id: String,
    pub startup_argument: String,
    pub toast_clsid: String,
    pub toast_argument: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WebView {
    pub minimum_version: String,
}

pub(crate) fn get() -> io::Result<&'static Contract> {
    static CONTRACT: OnceLock<Result<Contract, String>> = OnceLock::new();
    CONTRACT
        .get_or_init(|| {
            let contract: Contract = serde_json::from_str(include_str!(
                "../../../packaging/windows/host-contract.json"
            ))
            .map_err(|error| error.to_string())?;
            let leaf = |value: &str| {
                !value.is_empty()
                    && !value.contains(['/', '\\', ':'])
                    && value != "."
                    && value != ".."
            };
            let min_version = version(&contract.platform.min_version);
            let max_version = version(&contract.platform.max_version_tested);
            if contract.schema_version != 1
                || contract.platform.minimum_os_version().is_none()
                || max_version.is_none()
                || min_version > max_version
                || contract.paths.persistent_known_folder != "LocalAppData"
                || contract.paths.browser_data_known_folder != "LocalAppData"
                || contract.roles.desktop.pe_subsystem != 2
                || contract.roles.cli.pe_subsystem != 3
                || !leaf(&contract.roles.desktop.filename)
                || !leaf(&contract.roles.cli.filename)
                || !leaf(&contract.roles.desktop.cargo_bin)
                || !leaf(&contract.roles.cli.cargo_bin)
                || contract.roles.desktop.filename
                    != format!("{}.exe", contract.roles.desktop.cargo_bin)
                || contract.roles.cli.filename != format!("{}.exe", contract.roles.cli.cargo_bin)
                || contract.roles.desktop.cargo_bin == contract.roles.cli.cargo_bin
                || contract.activation.portable_aumid.is_empty()
                || contract.activation.portable_aumid.len() > 128
                || !contract
                    .activation
                    .portable_aumid
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
                || contract.activation.startup_argument != "--login-startup"
                || contract.activation.toast_argument != "--toast-server"
                || !crate::broker::canonical_uuid(&contract.activation.toast_clsid)
            {
                return Err("The shared Windows host contract is invalid".into());
            }
            Ok(contract)
        })
        .as_ref()
        .map_err(|error| io::Error::other(error.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_host_minimum_is_22h2_and_follows_the_shared_contract() {
        let contract = get().unwrap();
        assert_eq!(contract.platform.minimum_os_version(), Some([10, 0, 19045]));
        let mut platform = Platform {
            architecture: "x64".into(),
            min_version: "10.0.22000.0".into(),
            max_version_tested: "10.0.26100.0".into(),
        };
        assert_eq!(platform.minimum_os_version(), Some([10, 0, 22000]));
        platform.min_version = "10.0.26100.0".into();
        assert_eq!(platform.minimum_os_version(), Some([10, 0, 26100]));
    }

    #[test]
    fn windows_host_rejects_older_or_unrepresentable_minimum_versions() {
        for minimum in [
            "6.3.9600.0",
            "10.0.19041.0",
            "10.0.19044.0",
            "10.0.19045.1",
            "10.0.019045.0",
            "10.0.19045",
            "10.0.19045.0.0",
            "10.0.65536.0",
            "10.0.+19045.0",
        ] {
            let platform = Platform {
                architecture: "x64".into(),
                min_version: minimum.into(),
                max_version_tested: "10.0.26100.0".into(),
            };
            assert_eq!(platform.minimum_os_version(), None, "{minimum}");
        }
        let platform = Platform {
            architecture: "arm64".into(),
            min_version: "10.0.19045.0".into(),
            max_version_tested: "10.0.26100.0".into(),
        };
        assert_eq!(platform.minimum_os_version(), None);
    }
}
