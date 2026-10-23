//! All external workers use one creation/containment/exit authority.
//! Windows assigns JOB_LIST during CreateProcess, before user code can run.
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io,
    path::{Path, PathBuf},
};
use tokio::io::AsyncReadExt;

pub(crate) enum Stdio {
    Null,
    Pipe,
    File(File),
}
impl Stdio {
    pub fn null() -> Self {
        Self::Null
    }
    pub fn piped() -> Self {
        Self::Pipe
    }
}
impl From<File> for Stdio {
    fn from(file: File) -> Self {
        Self::File(file)
    }
}

pub(crate) struct Command {
    program: PathBuf,
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    clear_environment: bool,
    environment: Vec<(OsString, OsString)>,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessExit {
    #[cfg(windows)]
    Windows { code: u32 },
    #[cfg(target_os = "linux")]
    Linux {
        code: Option<i32>,
        signal: Option<i32>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Status(ProcessExit);
impl Status {
    pub fn success(&self) -> bool {
        match self.0 {
            #[cfg(windows)]
            ProcessExit::Windows { code } => code == 0,
            #[cfg(target_os = "linux")]
            ProcessExit::Linux { code, signal } => code == Some(0) && signal.is_none(),
        }
    }
    pub fn exit(&self) -> ProcessExit {
        self.0
    }
    #[cfg(windows)]
    fn windows(code: u32) -> Self {
        Self(ProcessExit::Windows { code })
    }
    #[cfg(target_os = "linux")]
    pub fn linux(code: Option<i32>, signal: Option<i32>) -> io::Result<Self> {
        if code.is_some() == signal.is_some()
            || code.is_some_and(|code| code < 0)
            || signal.is_some_and(|signal| signal <= 0)
        {
            return Err(io::Error::other(
                "The owned worker has no single confirmed process exit",
            ));
        }
        Ok(Self(ProcessExit::Linux { code, signal }))
    }
}
pub(crate) struct Output {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Command {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: PathBuf::from(program.as_ref()),
            args: Vec::new(),
            cwd: None,
            clear_environment: false,
            environment: Vec::new(),
            stdin: Stdio::Null,
            stdout: Stdio::Null,
            stderr: Stdio::Null,
        }
    }
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }
    pub fn current_dir(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.cwd = Some(path.as_ref().to_owned());
        self
    }
    pub fn env_clear(&mut self) -> &mut Self {
        self.clear_environment = true;
        self
    }
    pub fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.environment
            .push((key.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }
    pub fn stdin(&mut self, stdio: Stdio) -> &mut Self {
        self.stdin = stdio;
        self
    }
    pub fn stdout(&mut self, stdio: Stdio) -> &mut Self {
        self.stdout = stdio;
        self
    }
    pub fn stderr(&mut self, stdio: Stdio) -> &mut Self {
        self.stderr = stdio;
        self
    }

    pub async fn spawn(&mut self) -> io::Result<Child> {
        #[cfg(windows)]
        {
            windows::spawn(self).await
        }
        #[cfg(not(windows))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Owned worker containment requires the Windows backend",
            ))
        }
    }

    /// Small capability probes have the same containment and bounded streams as
    /// actual workers. A caller dropping this future still triggers owned drain.
    pub async fn output(&mut self) -> io::Result<Output> {
        self.stdout(Stdio::Pipe).stderr(Stdio::Pipe);
        let mut child = self.spawn().await?;
        let result = async {
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| io::Error::other("Missing owned stdout"))?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| io::Error::other("Missing owned stderr"))?;
            let read = |stream: Pipe| async move {
                let mut bytes = Vec::new();
                stream.take(256 * 1024 + 1).read_to_end(&mut bytes).await?;
                if bytes.len() > 256 * 1024 {
                    return Err(io::Error::other("Worker probe output budget exceeded"));
                }
                Ok::<_, io::Error>(bytes)
            };
            let (stdout, stderr, status) =
                tokio::try_join!(read(stdout), read(stderr), child.wait())?;
            Ok(Output {
                status,
                stdout,
                stderr,
            })
        }
        .await;
        if result.is_err() {
            child.kill().await?;
        }
        result
    }
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{Child, Pipe};

#[cfg(not(windows))]
pub(crate) type Pipe = tokio::io::DuplexStream;
#[cfg(not(windows))]
pub(crate) struct Child {
    pub stdin: Option<Pipe>,
    pub stdout: Option<Pipe>,
    pub stderr: Option<Pipe>,
}
#[cfg(not(windows))]
impl Child {
    pub async fn wait(&mut self) -> io::Result<Status> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No owned Windows child exists",
        ))
    }
    pub async fn kill(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No owned Windows child exists",
        ))
    }
    pub fn try_wait(&mut self) -> io::Result<Option<Status>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No owned Windows child exists",
        ))
    }
}
