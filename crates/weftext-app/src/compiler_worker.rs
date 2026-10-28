//! Compiler-only process selection. No caller can select an executable or a UID.
#[cfg(not(target_os = "linux"))]
use crate::worker_failure::Kind;
use crate::worker_failure::WorkerFailure;

#[derive(Clone, Default)]
pub(crate) struct Transport {
    #[cfg(target_os = "linux")]
    client: Option<std::sync::Arc<linux::Authority>>,
}

impl Transport {
    #[cfg(target_os = "linux")]
    pub(crate) fn controlled(
        backend: &weftext_core::BackendContext,
        policy: &std::path::Path,
    ) -> std::io::Result<Self> {
        Ok(Self {
            client: Some(std::sync::Arc::new(linux::Authority::connect(
                backend, policy,
            )?)),
        })
    }

    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn controlled_test(
        backend: &weftext_core::BackendContext,
        policy: &std::path::Path,
        timeout: std::time::Duration,
    ) -> std::io::Result<Self> {
        let mut authority = linux::Authority::connect(backend, policy)?;
        authority.test_timeout = timeout;
        Ok(Self {
            client: Some(std::sync::Arc::new(authority)),
        })
    }
    pub(crate) async fn spawn(&self, notes_edit: bool) -> Result<Child, WorkerFailure> {
        #[cfg(target_os = "linux")]
        {
            linux::spawn(self.client.clone(), notes_edit).await
        }
        #[cfg(not(target_os = "linux"))]
        {
            spawn_local(notes_edit).await
        }
    }
}

#[cfg(not(target_os = "linux"))]
type Child = crate::owned_process::Child;
#[cfg(target_os = "linux")]
use linux::Child;
#[cfg(target_os = "linux")]
mod linux;

#[cfg(not(target_os = "linux"))]
async fn spawn_local(notes_edit: bool) -> Result<Child, WorkerFailure> {
    use crate::owned_process::{Command, Stdio};
    #[cfg(test)]
    use std::path::PathBuf;
    let exe = std::env::current_exe().map_err(|error| {
        let mut failure = WorkerFailure::new(Kind::Setup);
        failure.io_error = Some(error);
        failure
    })?;
    #[cfg(test)]
    let exe = std::env::var_os("WEFTEXT_TEST_EXECUTABLE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            exe.parent()
                .and_then(std::path::Path::parent)
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(if cfg!(windows) {
                    "weftext.exe"
                } else {
                    "weftext"
                })
        });
    let mut command = Command::new(exe);
    command
        .arg(if notes_edit {
            "--notes-edit-worker"
        } else {
            "--render-worker"
        })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.spawn().await.map_err(|error| {
        let mut failure = WorkerFailure::new(Kind::Setup);
        failure.io_error = Some(error);
        failure
    })
}
