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
    pub cache_known_folder: String,
    pub cache_relative_root: String,
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
            if contract.schema_version != 1
                || contract.platform.architecture != "x64"
                || contract.paths.persistent_known_folder != "LocalAppData"
                || contract.paths.cache_known_folder != "LocalAppData"
                || contract.roles.desktop.pe_subsystem != 2
                || contract.roles.cli.pe_subsystem != 3
                || !leaf(&contract.roles.desktop.filename)
                || !leaf(&contract.roles.cli.filename)
                || !crate::broker::canonical_uuid(&contract.activation.toast_clsid)
            {
                return Err("The shared Windows host contract is invalid".into());
            }
            Ok(contract)
        })
        .as_ref()
        .map_err(|error| io::Error::other(error.clone()))
}
