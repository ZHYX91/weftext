//! Actual package identity, shared by the desktop OS adapters. A build contract
//! or caller-supplied AUMID alone never qualifies a system registration.
use windows::{
    ApplicationModel::{AppInfo, Package},
    Win32::{
        Foundation::{
            APPMODEL_ERROR_NO_PACKAGE, E_ACCESSDENIED, E_UNEXPECTED, ERROR_INSUFFICIENT_BUFFER,
        },
        Storage::Packaging::Appx::GetCurrentPackageFullName,
    },
    core::{Error, Result},
};

pub(super) struct Identity {
    pub app_id: String,
}
pub(super) fn current() -> Result<Option<Identity>> {
    let mut length = 0;
    let status = unsafe { GetCurrentPackageFullName(&mut length, None) };
    if status == APPMODEL_ERROR_NO_PACKAGE {
        return Ok(None);
    }
    if status != ERROR_INSUFFICIENT_BUFFER {
        return Err(Error::from(E_UNEXPECTED));
    }
    let contract = crate::host_contract::get().map_err(|_| Error::from(E_UNEXPECTED))?;
    let package = Package::Current()?.Id()?;
    let app = AppInfo::Current()?.Id()?.to_string();
    if package.Name()? != contract.identity.name
        || package.Publisher()? != contract.identity.publisher
        || app != contract.identity.application_id
    {
        return Err(Error::from(E_ACCESSDENIED));
    }
    let family = package.FamilyName()?.to_string();
    let app_id = format!("{family}!{app}");
    if family.is_empty()
        || app_id.len() > 128
        || !app_id.is_ascii()
        || app_id
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte == 0)
    {
        return Err(Error::from(E_UNEXPECTED));
    }
    Ok(Some(Identity { app_id }))
}
