//! Windows identity checks never accept a SID/PID/path supplied by the peer.
use std::{
    ffi::c_void,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::PathBuf,
};
use windows::{
    Win32::{
        Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, FILETIME, HANDLE, HLOCAL, LocalFree},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            },
            GetTokenInformation, PSECURITY_DESCRIPTOR, RevertToSelf, SECURITY_ATTRIBUTES,
            TOKEN_QUERY, TOKEN_USER, TokenUser,
        },
        System::{
            Pipes::{GetNamedPipeServerProcessId, ImpersonateNamedPipeClient},
            Threading::{
                GetCurrentProcess, GetCurrentThread, GetExitCodeProcess, GetProcessTimes,
                OpenProcess, OpenProcessToken, OpenThreadToken, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW,
                WaitForSingleObject,
            },
        },
    },
    core::{HSTRING, PWSTR},
};

fn error(error: windows::core::Error) -> io::Error {
    io::Error::other(error.to_string())
}
fn own(handle: HANDLE) -> OwnedHandle {
    unsafe { OwnedHandle::from_raw_handle(handle.0) }
}
fn raw(handle: &OwnedHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle())
}

fn token_sid(token: HANDLE) -> io::Result<String> {
    let mut length = 0;
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut length) };
    if length < std::mem::size_of::<TOKEN_USER>() as u32 || length > 64 * 1024 {
        return Err(io::Error::other("Unexpected TokenUser size"));
    }
    let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            length,
            &mut length,
        )
    }
    .map_err(error)?;
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let mut text = PWSTR::null();
    unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) }.map_err(error)?;
    let result = unsafe { text.to_string() }.map_err(io::Error::other);
    unsafe {
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
    }
    result
}

pub(crate) fn current_sid() -> io::Result<String> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.map_err(error)?;
    token_sid(raw(&own(token)))
}

/// The protected DACL exists at creation, including the first observable pipe.
pub(crate) fn create_server(
    name: &str,
    sid: &str,
    first: bool,
) -> io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let sddl = HSTRING::from(format!("D:P(A;;GA;;;{sid})"));
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(&sddl, 1, &mut descriptor, None)
    }
    .map_err(error)?;
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let result = unsafe {
        tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .max_instances(16)
            .create_with_security_attributes_raw(
                name,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast::<c_void>(),
            )
    };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
    }
    result
}

pub(crate) struct Peer {
    _process: OwnedHandle,
    _image: crate::artifact_catalog::VerifiedImage,
    pub pid: u32,
    pub created: u64,
}
impl Peer {
    pub(crate) async fn wait(&self, limit: std::time::Duration) -> io::Result<u32> {
        let process = self._process.try_clone()?;
        let milliseconds = u32::try_from(limit.as_millis())
            .unwrap_or(u32::MAX - 1)
            .min(u32::MAX - 1);
        crate::work::spawn_blocking(move || {
            match unsafe { WaitForSingleObject(raw(&process), milliseconds) }.0 {
                0 => {
                    let mut code = 0;
                    unsafe { GetExitCodeProcess(raw(&process), &mut code) }.map_err(error)?;
                    Ok(code)
                }
                258 => Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "The original owner process has not exited",
                )),
                _ => Err(io::Error::other(
                    "Original owner exit could not be observed",
                )),
            }
        })
        .await
        .map_err(io::Error::other)?
    }
}

fn process_identity(process: HANDLE) -> io::Result<(String, u64, PathBuf)> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.map_err(error)?;
    let sid = token_sid(raw(&own(token)))?;
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe { GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) }
        .map_err(error)?;
    let creation = (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
    let mut path = vec![0u16; 32768];
    let mut length = path.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(error)?;
    let path = String::from_utf16(&path[..length as usize]).map_err(io::Error::other)?;
    Ok((sid, creation, PathBuf::from(path)))
}

pub(crate) fn self_created() -> io::Result<u64> {
    process_identity(unsafe { GetCurrentProcess() }).map(|(_, created, _)| created)
}

pub(crate) fn verify_server(
    pipe: &impl AsRawHandle,
    expected: &crate::artifact_catalog::Catalog,
    sid: &str,
) -> io::Result<Peer> {
    let mut pid = 0;
    unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid) }
        .map_err(error)?;
    let process = own(unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            false,
            pid,
        )
    }
    .map_err(error)?);
    let (actual_sid, created, image) = process_identity(raw(&process))?;
    let image = expected.verify_image(&image)?;
    // WAIT_TIMEOUT is the only live result. Keep the process handle through the
    // connection so PID reuse cannot stand in for the verified owner instance.
    if actual_sid != sid || unsafe { WaitForSingleObject(raw(&process), 0) }.0 != 258 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The pipe owner is not the expected live product process",
        ));
    }
    Ok(Peer {
        _process: process,
        _image: image,
        pid,
        created,
    })
}

/// Identification is a synchronous section on a dedicated, short-lived OS
/// thread. No impersonated thread is ever returned to Tokio's reusable pool.
pub(crate) enum AuthenticationError {
    Rejected,
    Fatal(io::Error),
}

pub(crate) async fn authenticate_client(
    pipe: &impl AsRawHandle,
    expected_sid: String,
) -> Result<(), AuthenticationError> {
    let duplicate = duplicate_handle(pipe).map_err(AuthenticationError::Fatal)?;
    let (send, received) = tokio::sync::oneshot::channel();
    let thread = std::thread::Builder::new()
        .name("weftext-pipe-identification".into())
        .spawn(move || {
            let result = unsafe { ImpersonateNamedPipeClient(raw(&duplicate)) }
                .map_err(|_| AuthenticationError::Rejected)
                .and_then(|()| {
                    let mut token = HANDLE::default();
                    let result = unsafe {
                        OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token)
                    }
                    .map_err(error)
                    .and_then(|()| token_sid(raw(&own(token))))
                    .and_then(|sid| {
                        if sid == expected_sid {
                            Ok(())
                        } else {
                            Err(io::Error::new(
                                io::ErrorKind::PermissionDenied,
                                "The client user identity differs",
                            ))
                        }
                    });
                    if unsafe { RevertToSelf() }.is_err() {
                        // Exiting this dedicated OS thread discards its token. Do not
                        // execute Core, logs, files or another request on this thread.
                        return Err(AuthenticationError::Fatal(io::Error::other(
                            "Pipe identification could not revert; owner must quiesce",
                        )));
                    }
                    result.map_err(|_| AuthenticationError::Rejected)
                });
            let _ = send.send(result);
        })
        .map_err(AuthenticationError::Fatal)?;
    // Register the actual join before any await. Cancellation of this caller
    // leaves a tracked join waiting for the dedicated thread's real exit.
    let joined = crate::work::spawn_blocking(move || thread.join());
    let result = received.await;
    if !matches!(joined.await, Ok(Ok(()))) {
        return Err(AuthenticationError::Fatal(io::Error::other(
            "Pipe identification thread exit is unconfirmed",
        )));
    }
    result.map_err(|_| {
        AuthenticationError::Fatal(io::Error::other(
            "Pipe identification ended without a result",
        ))
    })?
}

fn duplicate_handle(pipe: &impl AsRawHandle) -> io::Result<OwnedHandle> {
    let mut duplicate = HANDLE::default();
    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(pipe.as_raw_handle()),
            GetCurrentProcess(),
            &mut duplicate,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    }
    .map_err(error)?;
    Ok(own(duplicate))
}
