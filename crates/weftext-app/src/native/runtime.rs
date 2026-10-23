//! Loader preflight uses the same default channel lookup as environment
//! creation. It neither installs a Runtime nor changes browser arguments.
use super::error;
use crate::launch::ServeError;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    CompareBrowserVersions, GetAvailableCoreWebView2BrowserVersionString,
};
use windows::{
    Win32::System::Com::CoTaskMemFree,
    core::{PCWSTR, PWSTR},
};

struct Version(PWSTR);
impl Drop for Version {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.0.0.cast())) };
    }
}

pub(super) fn qualify() -> Result<(), ServeError> {
    let minimum = &crate::host_contract::get()
        .map_err(|_| error("hostContract", "The Windows host contract is invalid"))?
        .webview2
        .minimum_version;
    let mut parts = minimum.split('.');
    if !(0..4).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part.parse::<u32>().is_ok()
        })
    }) || parts.next().is_some()
    {
        return Err(error(
            "hostContract",
            "The minimum WebView2 version is invalid",
        ));
    }
    let minimum: Vec<u16> = minimum.encode_utf16().chain(Some(0)).collect();
    let mut version = Version(PWSTR::null());
    unsafe { GetAvailableCoreWebView2BrowserVersionString(PCWSTR::null(), &mut version.0) }
        .map_err(|_| unavailable())?;
    if version.0.is_null() {
        return Err(unavailable());
    }
    let mut order = 0;
    unsafe { CompareBrowserVersions(PCWSTR(version.0.0), PCWSTR(minimum.as_ptr()), &mut order) }
        .map_err(|_| unavailable())?;
    if order < 0 {
        return Err(unavailable());
    }
    Ok(())
}

fn unavailable() -> ServeError {
    error(
        "webviewRuntimeUnavailable",
        "A supported WebView2 Runtime is unavailable",
    )
}
