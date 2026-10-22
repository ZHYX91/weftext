//! Host paths and browser handoff. These choices do not certify a file backend.
#[cfg(any(test, not(windows)))]
use std::ffi::OsString;
use std::{io, path::PathBuf};
use weftext_core::Core;
use weftext_messages::{DiagnosticOrigin, ErrorDescription, UiMessage};

pub(crate) fn default_data_dir() -> io::Result<PathBuf> {
    #[cfg(windows)]
    {
        crate::host_paths::default_data_dir()
    }
    #[cfg(not(windows))]
    data_dir_from(std::env::consts::OS, |key| std::env::var_os(key))
}

#[cfg(any(test, not(windows)))]
fn data_dir_from(
    platform: &str,
    environment: impl Fn(&str) -> Option<OsString>,
) -> io::Result<PathBuf> {
    let anchor = |name: &str| -> io::Result<PathBuf> {
        let value = environment(name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                ErrorDescription::new(UiMessage::error_api_host_environment_data_anchor_missing(
                    weftext_messages::params::ErrorApiHostEnvironmentDataAnchorMissing {
                        name: name.into(),
                    },
                )),
            )
        })?;
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                ErrorDescription::new(
                    UiMessage::error_api_host_environment_data_anchor_not_absolute(
                        weftext_messages::params::ErrorApiHostEnvironmentDataAnchorNotAbsolute {
                            name: name.into(),
                        },
                    ),
                ),
            ));
        }
        Ok(path)
    };
    match platform {
        "macos" => Ok(anchor("HOME")?.join("Library/Application Support/Weftext")),
        "linux" => Ok(if environment("XDG_DATA_HOME").is_some() {
            anchor("XDG_DATA_HOME")?.join("weftext")
        } else {
            anchor("HOME")?.join(".local/share/weftext")
        }),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            ErrorDescription::new(
                UiMessage::error_api_host_environment_data_directory_unsupported(),
            ),
        )),
    }
}

pub(crate) fn open_core(data_dir: PathBuf) -> io::Result<(Core, PathBuf)> {
    if data_dir.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            ErrorDescription::new(
                UiMessage::error_api_host_environment_persistent_data_directory_required(),
            ),
        ));
    }
    let data_dir = std::path::absolute(data_dir)?;
    #[cfg(windows)]
    let _profile = crate::host_paths::qualify_core(&data_dir)?;
    // Retain CoreError as the typed source so a host can distinguish an owned
    // instance from an unsupported backend without parsing diagnostic prose.
    let core = Core::new(&data_dir).map_err(io::Error::other)?;
    let data_dir = std::fs::canonicalize(&data_dir).map_err(|error| {
        io::Error::other(ErrorDescription::new(UiMessage::error_api_host_environment_data_directory_resolution_failed(weftext_messages::params::ErrorApiHostEnvironmentDataDirectoryResolutionFailed {
            path: data_dir.display().to_string(),
        })).diagnostic(DiagnosticOrigin::Os, error.to_string()))
    })?;
    Ok((core, data_dir))
}

#[cfg(windows)]
pub(crate) fn open_existing_core(
    data_dir: PathBuf,
    physical_identity: &str,
    device_id: &str,
) -> io::Result<(Core, PathBuf)> {
    let data = std::path::absolute(data_dir)?;
    let profile = weftext_core::qualify_existing_profile(&data).map_err(io::Error::other)?;
    crate::host_state::Root::qualify(false)?.separate(&profile)?;
    if profile.physical_identity() != physical_identity {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The original physical profile was replaced",
        ));
    }
    let core =
        Core::open_existing(&data, physical_identity, device_id).map_err(io::Error::other)?;
    Ok((core, std::fs::canonicalize(&data)?))
}

pub(crate) async fn open_browser(address: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        let address = address.to_owned();
        bounded_browser_handoff(
            move || windows_browser(&address),
            std::time::Duration::from_secs(10),
        )
        .await
    }
    #[cfg(not(windows))]
    {
        let mut command = tokio::process::Command::new(browser_program(std::env::consts::OS)?);
        command.arg(address);
        run_browser_command(command).await
    }
}

#[cfg(any(windows, test))]
async fn bounded_browser_handoff(
    operation: impl FnOnce() -> io::Result<()> + Send + 'static,
    limit: std::time::Duration,
) -> io::Result<()> {
    let (send, result) = tokio::sync::oneshot::channel();
    let thread = std::thread::Builder::new()
        .name("weftext-browser-handoff".into())
        .spawn(move || {
            let _ = send.send(operation());
        })?;
    // An OS extension can stall. Do not put this call in Tokio's blocking pool:
    // dropping the host runtime must not wait for an unresponsive shell call.
    // The single handoff thread owns its COM apartment until the call returns.
    drop(thread);
    match tokio::time::timeout(limit, result).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(io::Error::other(ErrorDescription::new(
            UiMessage::error_api_host_environment_browser_confirmation_missing(),
        ))),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            ErrorDescription::new(
                UiMessage::error_api_host_environment_browser_handoff_still_unconfirmed(),
            ),
        )),
    }
}

#[cfg(windows)]
fn windows_browser(address: &str) -> io::Result<()> {
    use windows_sys::Win32::{
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        UI::{
            Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };
    // Shell extensions can use COM. Balance this helper thread's apartment and
    // complete the handoff before it returns; this is not a GUI event loop.
    let initialized = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    if initialized < 0 {
        return Err(io::Error::other(ErrorDescription::new(
            UiMessage::error_api_host_environment_browser_initialization_failed(
                weftext_messages::params::ErrorApiHostEnvironmentBrowserInitializationFailed {
                    hresult: format!("{initialized:#x}"),
                },
            ),
        )));
    }
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _apartment = Apartment;
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    let address: Vec<u16> = address.encode_utf16().chain(Some(0)).collect();
    let mut execute = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: verb.as_ptr(),
        lpFile: address.as_ptr(),
        nShow: SW_SHOWNORMAL,
        ..Default::default()
    };
    if unsafe { ShellExecuteExW(&mut execute) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // Acceptance by the shell does not prove a browser tab has loaded.
    Ok(())
}

#[cfg(any(not(windows), test))]
fn browser_program(platform: &str) -> io::Result<&'static str> {
    match platform {
        "macos" => Ok("/usr/bin/open"),
        "linux" => Ok("xdg-open"),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            ErrorDescription::new(
                UiMessage::error_api_host_environment_browser_launcher_unsupported(),
            ),
        )),
    }
}

#[cfg(any(not(windows), test))]
async fn run_browser_command(mut command: tokio::process::Command) -> io::Result<()> {
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let status = tokio::time::timeout(std::time::Duration::from_secs(10), command.status())
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                ErrorDescription::new(
                    UiMessage::error_api_host_environment_browser_handoff_timed_out(),
                ),
            )
        })??;
    if !status.success() {
        return Err(io::Error::other(
            ErrorDescription::new(
                UiMessage::error_api_host_environment_browser_launcher_rejected(),
            )
            .diagnostic(DiagnosticOrigin::Os, status.to_string()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn description(error: &io::Error) -> &ErrorDescription {
        error
            .get_ref()
            .and_then(|source| source.downcast_ref::<ErrorDescription>())
            .expect("product-owned host error retains its typed description")
    }

    #[test]
    fn non_windows_persistent_defaults_use_the_host_location() {
        let base = std::env::current_dir().unwrap().join("TEST-user");
        for (platform, variable, suffix) in [
            ("macos", "HOME", "Library/Application Support/Weftext"),
            ("linux", "HOME", ".local/share/weftext"),
            ("linux", "XDG_DATA_HOME", "weftext"),
        ] {
            assert_eq!(
                data_dir_from(platform, |key| {
                    (key == variable).then(|| base.clone().into_os_string())
                })
                .unwrap(),
                base.join(suffix)
            );
        }
    }

    #[test]
    fn missing_or_invalid_persistent_roots_never_select_temporary_state() {
        assert_eq!(
            open_core(PathBuf::new()).err().unwrap().kind(),
            io::ErrorKind::InvalidInput
        );
        for platform in ["macos", "linux"] {
            assert_eq!(
                data_dir_from(platform, |_| None).unwrap_err().kind(),
                io::ErrorKind::NotFound
            );
            for value in ["", "relative-state"] {
                let error = data_dir_from(platform, |_| Some(value.into())).unwrap_err();
                assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
                assert_eq!(
                    description(&error).explanation.key(),
                    "error.api.hostEnvironment.dataAnchorNotAbsolute"
                );
            }
        }
        let home = std::env::current_dir().unwrap();
        assert!(
            data_dir_from("linux", |key| Some(if key == "XDG_DATA_HOME" {
                "relative-xdg".into()
            } else {
                home.clone().into_os_string()
            }))
            .is_err()
        );
        assert_eq!(
            data_dir_from("unknown", |_| None).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_uses_the_known_folder_and_shared_durable_contract() {
        let expected = crate::host_paths::local_app_data().unwrap().join(
            &crate::host_contract::get()
                .unwrap()
                .paths
                .persistent_relative_root,
        );
        assert_eq!(default_data_dir().unwrap(), expected);
        assert_eq!(expected.file_name().unwrap(), "core");
        assert_eq!(
            expected.parent().unwrap().file_name().unwrap(),
            "Weftext-MVP"
        );
    }

    #[test]
    fn macos_and_linux_choose_their_actual_launchers() {
        assert_eq!(browser_program("macos").unwrap(), "/usr/bin/open");
        assert_eq!(browser_program("linux").unwrap(), "xdg-open");
        assert_eq!(
            browser_program("unknown").unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[tokio::test]
    async fn missing_and_failed_launchers_return_errors_without_opening_a_browser() {
        let absent =
            std::env::temp_dir().join(format!("weftext-absent-launcher-{}", uuid::Uuid::new_v4()));
        assert_eq!(
            run_browser_command(tokio::process::Command::new(absent))
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
        let mut rejected = tokio::process::Command::new(std::env::current_exe().unwrap());
        rejected.arg("--weftext-invalid-test-harness-option");
        let rejected = run_browser_command(rejected).await.unwrap_err();
        assert_eq!(
            description(&rejected).explanation.key(),
            "error.api.hostEnvironment.browserLauncherRejected"
        );
        assert_eq!(description(&rejected).diagnostics.len(), 1);
        assert_eq!(
            description(&rejected).diagnostics[0].origin,
            DiagnosticOrigin::Os
        );
    }

    #[test]
    fn stalled_os_handoff_times_out_and_does_not_hold_the_host_runtime() {
        let (release, held) = std::sync::mpsc::channel();
        let (finished, host_stopped) = std::sync::mpsc::channel();
        let host = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();
            let error = runtime
                .block_on(bounded_browser_handoff(
                    move || {
                        let _ = held.recv();
                        Ok(())
                    },
                    std::time::Duration::from_millis(50),
                ))
                .unwrap_err();
            // The callback is still held here. A blocking-pool implementation
            // would prevent this drop and leave the host unable to exit.
            drop(runtime);
            finished.send(error).unwrap();
        });
        let outcome = host_stopped.recv_timeout(std::time::Duration::from_secs(3));
        // Release the owned callback even when the assertion would fail.
        let _ = release.send(());
        host.join().unwrap();
        let error = outcome.expect("stalled browser prevented host runtime shutdown");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert_eq!(
            description(&error).explanation.key(),
            "error.api.hostEnvironment.browserHandoffStillUnconfirmed"
        );
        assert!(description(&error).diagnostics.is_empty());
    }
}
