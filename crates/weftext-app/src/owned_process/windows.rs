//! Published CreateProcessW/JOB_LIST/HANDLE_LIST only; no suspended process,
//! undocumented resume function, global process-name kill, or post-spawn Job race.
//! https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute
use super::{Command, Status, Stdio};
use std::{
    ffi::{OsStr, OsString},
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    time::Duration,
};
use tokio::net::windows::named_pipe::NamedPipeServer;
use windows::{
    Win32::{
        Foundation::{
            DUPLICATE_SAME_ACCESS, DuplicateHandle, GENERIC_READ, GENERIC_WRITE, GetLastError,
            HANDLE,
        },
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{JobObjects::*, Threading::*},
    },
    core::{PCWSTR, PWSTR},
};

pub(crate) type Pipe = NamedPipeServer;
fn error(error: windows::core::Error) -> io::Error {
    io::Error::other(error.to_string())
}
fn raw(handle: &OwnedHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle())
}
fn own(handle: HANDLE) -> OwnedHandle {
    unsafe { OwnedHandle::from_raw_handle(handle.0) }
}
fn wide(text: &OsStr) -> io::Result<Vec<u16>> {
    let mut value = text.encode_wide().collect::<Vec<_>>();
    if value.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "A process argument contains NUL",
        ));
    }
    value.push(0);
    Ok(value)
}

fn quote(arg: &OsStr, out: &mut Vec<u16>) -> io::Result<()> {
    let units = wide(arg)?;
    out.push(b'"' as u16);
    let mut slashes = 0;
    for &unit in &units[..units.len() - 1] {
        if unit == b'\\' as u16 {
            slashes += 1;
            continue;
        }
        if unit == b'"' as u16 {
            out.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2 + 1));
        } else {
            out.extend(std::iter::repeat_n(b'\\' as u16, slashes));
        }
        slashes = 0;
        out.push(unit);
    }
    out.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
    out.push(b'"' as u16);
    Ok(())
}

struct Attributes {
    _storage: Vec<usize>,
    list: LPPROC_THREAD_ATTRIBUTE_LIST,
    jobs: Box<[HANDLE]>,
    handles: Box<[HANDLE]>,
}
impl Attributes {
    fn new(job: HANDLE, handles: &[HANDLE]) -> io::Result<Self> {
        let mut bytes = 0;
        let _ = unsafe { InitializeProcThreadAttributeList(None, 2, None, &mut bytes) };
        if bytes == 0 || bytes > 64 * 1024 {
            return Err(io::Error::other("Invalid process attribute budget"));
        }
        let mut storage = vec![0usize; bytes.div_ceil(std::mem::size_of::<usize>())];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        unsafe { InitializeProcThreadAttributeList(Some(list), 2, None, &mut bytes) }
            .map_err(error)?;
        let attrs = Self {
            _storage: storage,
            list,
            jobs: vec![job].into_boxed_slice(),
            handles: handles.to_vec().into_boxed_slice(),
        };
        unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                Some(attrs.jobs.as_ptr().cast()),
                std::mem::size_of_val(attrs.jobs.as_ref()),
                None,
                None,
            )
        }
        .map_err(error)?;
        unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                Some(attrs.handles.as_ptr().cast()),
                std::mem::size_of_val(attrs.handles.as_ref()),
                None,
                None,
            )
        }
        .map_err(error)?;
        Ok(attrs)
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.list) };
    }
}

async fn stdio(stdio: Stdio, input: bool, sid: &str) -> io::Result<(OwnedHandle, Option<Pipe>)> {
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: true.into(),
    };
    match stdio {
        Stdio::Null => {
            let name = wide(OsStr::new("NUL"))?;
            let file = unsafe {
                CreateFileW(
                    PCWSTR(name.as_ptr()),
                    if input {
                        GENERIC_READ.0
                    } else {
                        GENERIC_WRITE.0
                    },
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    Some(&attrs),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
            }
            .map_err(error)?;
            Ok((own(file), None))
        }
        Stdio::File(file) => {
            let mut inherited = HANDLE::default();
            unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    HANDLE(file.as_raw_handle()),
                    GetCurrentProcess(),
                    &mut inherited,
                    0,
                    true,
                    DUPLICATE_SAME_ACCESS,
                )
            }
            .map_err(error)?;
            Ok((own(inherited), None))
        }
        Stdio::Pipe => {
            let name = format!(r"\\.\pipe\Weftext.Worker.v1.{}", uuid::Uuid::new_v4());
            let server = crate::broker::security::create_server(&name, sid, true)?;
            let name = wide(OsStr::new(&name))?;
            let file = unsafe {
                CreateFileW(
                    PCWSTR(name.as_ptr()),
                    if input {
                        GENERIC_READ.0
                    } else {
                        GENERIC_WRITE.0
                    },
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    Some(&attrs),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
            }
            .map_err(error)?;
            let file = own(file);
            server.connect().await?;
            Ok((file, Some(server)))
        }
    }
}

pub(super) async fn spawn(command: &mut Command) -> io::Result<Child> {
    if !command.program.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Worker executable must be absolute",
        ));
    }
    let application = wide(command.program.as_os_str())?;
    let mut arguments = Vec::new();
    quote(command.program.as_os_str(), &mut arguments)?;
    for arg in &command.args {
        arguments.push(b' ' as u16);
        quote(arg, &mut arguments)?;
    }
    arguments.push(0);
    if arguments.len() > 32767 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Worker command line exceeds Windows budget",
        ));
    }
    let cwd = command
        .cwd
        .as_ref()
        .map(|p| wide(p.as_os_str()))
        .transpose()?;
    let environment = if command.clear_environment {
        let mut pairs = command
            .environment
            .iter()
            .map(|(key, value)| {
                let key = key
                    .to_str()
                    .filter(|s| !s.is_empty() && s.is_ascii() && !s.contains(['=', '\0']))
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Worker environment keys must be ASCII names",
                        )
                    })?;
                Ok::<_, io::Error>((key.to_ascii_uppercase(), key.to_owned(), value))
            })
            .collect::<io::Result<Vec<_>>>()?;
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        if pairs.windows(2).any(|p| p[0].0 == p[1].0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Duplicate worker environment key",
            ));
        }
        let mut block = Vec::new();
        for (_, key, value) in pairs {
            let pair = OsString::from(format!("{key}="));
            let mut pair = pair;
            pair.push(value);
            block.extend(wide(&pair)?);
        }
        if block.is_empty() {
            block.push(0);
        }
        block.push(0);
        Some(block)
    } else {
        if !command.environment.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Explicit worker environment requires a clear environment",
            ));
        }
        None
    };
    let sid = crate::broker::security::current_sid()?;
    let (input, stdin) = stdio(
        std::mem::replace(&mut command.stdin, Stdio::Null),
        true,
        &sid,
    )
    .await?;
    let (output, stdout) = stdio(
        std::mem::replace(&mut command.stdout, Stdio::Null),
        false,
        &sid,
    )
    .await?;
    let (diagnostics, stderr) = stdio(
        std::mem::replace(&mut command.stderr, Stdio::Null),
        false,
        &sid,
    )
    .await?;
    let job = own(unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(error)?);
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
    limits.ProcessMemoryLimit = 1024 * 1024 * 1024;
    unsafe {
        SetInformationJobObject(
            raw(&job),
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
        )
    }
    .map_err(error)?;
    let handles = [raw(&input), raw(&output), raw(&diagnostics)];
    let attributes = Attributes::new(raw(&job), &handles)?;
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
    startup.StartupInfo.wShowWindow = 0;
    startup.StartupInfo.hStdInput = handles[0];
    startup.StartupInfo.hStdOutput = handles[1];
    startup.StartupInfo.hStdError = handles[2];
    startup.lpAttributeList = attributes.list;
    let mut process = PROCESS_INFORMATION::default();
    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(arguments.as_mut_ptr())),
            None,
            None,
            true,
            CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
            environment.as_ref().map(|v| v.as_ptr().cast()),
            cwd.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr())),
            &startup.StartupInfo,
            &mut process,
        )
    }
    .map_err(error)?;
    drop(own(process.hThread));
    Ok(Child {
        process: Some(own(process.hProcess)),
        job: Some(job),
        exit_code: None,
        complete: false,
        stdin,
        stdout,
        stderr,
    })
}

pub(crate) struct Child {
    process: Option<OwnedHandle>,
    job: Option<OwnedHandle>,
    exit_code: Option<u32>,
    complete: bool,
    pub stdin: Option<Pipe>,
    pub stdout: Option<Pipe>,
    pub stderr: Option<Pipe>,
}
impl Child {
    pub fn try_wait(&mut self) -> io::Result<Option<Status>> {
        if let Some(process) = &self.process {
            match unsafe { WaitForSingleObject(raw(process), 0) }.0 {
                258 => return Ok(None),
                0 => {
                    let mut code = 0;
                    unsafe { GetExitCodeProcess(raw(process), &mut code) }.map_err(error)?;
                    self.exit_code = Some(code);
                    self.process.take();
                }
                _ => {
                    return Err(io::Error::from_raw_os_error(
                        unsafe { GetLastError() }.0 as i32,
                    ));
                }
            }
        }
        if let Some(job) = &self.job {
            let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
            unsafe {
                QueryInformationJobObject(
                    Some(raw(job)),
                    JobObjectBasicAccountingInformation,
                    (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                    std::mem::size_of_val(&accounting) as u32,
                    None,
                )
            }
            .map_err(error)?;
            if accounting.ActiveProcesses != 0 {
                return Ok(None);
            }
        }
        let code = self
            .exit_code
            .ok_or_else(|| io::Error::other("Owned process exit was not observed"))?;
        self.complete = true;
        self.job.take();
        Ok(Some(Status::windows(code)))
    }
    pub async fn wait(&mut self) -> io::Result<Status> {
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    pub async fn kill(&mut self) -> io::Result<()> {
        if self.complete {
            return Ok(());
        }
        if let Some(job) = &self.job {
            unsafe { TerminateJobObject(raw(job), 1) }.map_err(error)?;
        }
        self.stdin.take();
        self.stdout.take();
        self.stderr.take();
        self.wait().await.map(|_| ())
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        if self.complete {
            return;
        }
        // This fallback is actual retained cleanup, not an exit receipt. The
        // caller's result stays unknown; shutdown joins this work as well.
        let child = Child {
            process: self.process.take(),
            job: self.job.take(),
            exit_code: self.exit_code,
            complete: false,
            stdin: self.stdin.take(),
            stdout: self.stdout.take(),
            stderr: self.stderr.take(),
        };
        self.complete = true;
        if tokio::runtime::Handle::try_current().is_ok() {
            crate::work::spawn(async move {
                let mut child = child;
                if child.kill().await.is_err() {
                    crate::work::fault();
                    child.complete = true;
                }
            });
        } else {
            crate::work::fault();
            let mut child = child;
            child.complete = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_preserve_empty_quotes_backslashes_and_unicode_without_shell_syntax() {
        let mut out = Vec::new();
        quote(OsStr::new(""), &mut out).unwrap();
        assert_eq!(String::from_utf16(&out).unwrap(), "\"\"");
        out.clear();
        quote(OsStr::new("a\\\"b\\"), &mut out).unwrap();
        assert_eq!(String::from_utf16(&out).unwrap(), "\"a\\\\\\\"b\\\\\"");
        out.clear();
        quote(OsStr::new("中文 $()&;😀"), &mut out).unwrap();
        assert_eq!(String::from_utf16(&out).unwrap(), "\"中文 $()&;😀\"");
        assert!(quote(OsStr::new("a\0b"), &mut Vec::new()).is_err());
    }
}
