//! The broker owns process-tree termination. This adapter owns only its exact
//! pipes and receipts; it cannot execute a local fallback or signal a numeric PID.
use crate::worker_failure::{Kind, WorkerFailure};
use std::{io, sync::Arc, time::Duration};
use tokio::net::unix::pipe::{Receiver, Sender};
use weftext_worker_broker::{BrokerClient, Completion, OwnedWorker, Receipt, WorkerKind};

const CLEANUP_DEADLINE: Duration = Duration::from_secs(5);

pub(super) struct Authority {
    client: BrokerClient,
    gate: Arc<tokio::sync::Semaphore>,
}
impl Authority {
    pub(super) fn connect(
        backend: &weftext_core::BackendContext,
        policy: &std::path::Path,
    ) -> io::Result<Self> {
        Ok(Self {
            client: BrokerClient::connect(backend.controlled_enrollment(), policy)
                .map_err(io::Error::other)?,
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
        })
    }
}

/// A single accepted launch owns this admission through its terminal receipt.
/// No queue can outlive abandoned HTTP callers. Unknown launch/cleanup closes
/// the authority until an explicit service restart; a permit alone is not proof.
struct Admission {
    gate: Arc<tokio::sync::Semaphore>,
    _permit: tokio::sync::OwnedSemaphorePermit,
    uncertain: bool,
}
impl Admission {
    fn claim(gate: Arc<tokio::sync::Semaphore>) -> io::Result<Self> {
        let permit = gate.clone().try_acquire_owned().map_err(|_| {
            io::Error::new(
                if gate.is_closed() {
                    io::ErrorKind::BrokenPipe
                } else {
                    io::ErrorKind::WouldBlock
                },
                "The original compiler worker authority is occupied or unavailable",
            )
        })?;
        Ok(Self {
            gate,
            _permit: permit,
            uncertain: false,
        })
    }
    fn begin_launch(&mut self) {
        self.uncertain = true;
    }
    fn settle(&mut self) {
        self.uncertain = false;
    }
}
impl Drop for Admission {
    fn drop(&mut self) {
        if self.uncertain {
            self.gate.close();
        }
    }
}

#[derive(Clone)]
pub(crate) struct Status {
    exit: crate::owned_process::Status,
    completion: Completion,
}
impl Status {
    fn from_receipt(receipt: &Receipt) -> io::Result<Self> {
        Ok(Self {
            exit: crate::owned_process::Status::linux(receipt.exit_code, receipt.signal)?,
            completion: receipt.completion.clone(),
        })
    }
    pub(crate) fn success(&self) -> bool {
        self.completion == Completion::Exited && self.exit.success()
    }
    pub(crate) fn exit(&self) -> crate::owned_process::ProcessExit {
        self.exit.exit()
    }
    pub(crate) fn completion(&self) -> Completion {
        self.completion.clone()
    }
    pub(crate) fn failure_kind(&self) -> Kind {
        match &self.completion {
            Completion::Timeout => Kind::Deadline,
            Completion::Cancelled => Kind::Cancelled,
            Completion::Disconnected | Completion::SetupFailed => Kind::SupervisionLost,
            Completion::Exited => Kind::NonzeroExit,
        }
    }
}

pub(crate) struct Child {
    owned: OwnedWorker,
    admission: Admission,
    confirmed: bool,
    pub(crate) stdin: Option<Sender>,
    pub(crate) stdout: Option<Receiver>,
    pub(crate) stderr: Option<Receiver>,
}
impl Child {
    pub(crate) fn try_wait(&mut self) -> io::Result<Option<Status>> {
        self.owned
            .try_wait()
            .map_err(io::Error::other)?
            .map(|receipt| {
                let status = Status::from_receipt(&receipt)?;
                self.confirmed = true;
                self.admission.settle();
                Ok(status)
            })
            .transpose()
    }
    pub(crate) async fn wait(&mut self) -> io::Result<Status> {
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    pub(crate) async fn kill(&mut self) -> io::Result<()> {
        if self.try_wait()?.is_some() {
            return Ok(());
        }
        // A nonblocking control send and finite receipt wait. Dropping this
        // future does not claim cleanup; Drop records unknown and disconnects.
        self.owned.request_cancel().map_err(io::Error::other)?;
        tokio::time::timeout(CLEANUP_DEADLINE, self.wait())
            .await
            .map_err(|_| {
                io::Error::new(io::ErrorKind::TimedOut, "Worker cleanup receipt deadline")
            })??;
        Ok(())
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        if !self.confirmed {
            crate::work::fault();
        }
    }
}

pub(super) async fn spawn(
    authority: Option<Arc<Authority>>,
    notes_edit: bool,
) -> Result<Child, WorkerFailure> {
    let authority = authority.ok_or_else(|| {
        let mut failure = WorkerFailure::new(Kind::Setup);
        failure.io_error = Some(io::Error::new(
            io::ErrorKind::Unsupported,
            "Compiler workers require the explicit controlled-service broker",
        ));
        failure
    })?;
    let mut admission = Admission::claim(authority.gate.clone()).map_err(|error| {
        let mut failure = WorkerFailure::new(Kind::Setup);
        failure.io_error = Some(error);
        failure
    })?;
    // Admission moves into the registered launch and then the Child itself.
    // Even a dropped join result retains unknown cleanup via Child::drop.
    let launch = crate::work::spawn_blocking(move || {
        admission.begin_launch();
        match authority.client.launch(
            if notes_edit {
                WorkerKind::Notes
            } else {
                WorkerKind::Render
            },
            Duration::from_secs(20),
        ) {
            Ok(owned) => Ok(Child {
                owned,
                admission,
                confirmed: false,
                stdin: None,
                stdout: None,
                stderr: None,
            }),
            Err(error) => {
                if error.kind != weftext_worker_broker::ErrorKind::Unknown {
                    admission.settle(); // Rejected before any launch request.
                } else {
                    crate::work::fault();
                }
                Err(error)
            }
        }
    })
    .await;
    let mut child = match launch {
        Ok(Ok(child)) => child,
        failed => {
            let mut failure = WorkerFailure::new(Kind::Setup);
            failure.cleanup_confirmed = matches!(&failed,
                Ok(Err(error)) if error.kind != weftext_worker_broker::ErrorKind::Unknown);
            failure.io_error = Some(match failed {
                Ok(Err(error)) => io::Error::other(error),
                Err(error) => io::Error::other(error),
                Ok(Ok(_)) => unreachable!(),
            });
            if !failure.cleanup_confirmed {
                crate::work::fault();
            }
            return Err(failure);
        }
    };
    let pipes = (|| -> io::Result<()> {
        let missing = || io::Error::other("Broker did not return its owned pipe");
        child.stdin = Some(Sender::from_owned_fd(
            child.owned.take_stdin().ok_or_else(missing)?.into(),
        )?);
        child.stdout = Some(Receiver::from_owned_fd(
            child.owned.take_stdout().ok_or_else(missing)?.into(),
        )?);
        child.stderr = Some(Receiver::from_owned_fd(
            child.owned.take_stderr().ok_or_else(missing)?.into(),
        )?);
        Ok(())
    })();
    if let Err(error) = pipes {
        let mut failure = WorkerFailure::new(Kind::Setup);
        failure.io_error = Some(error);
        failure.cleanup_confirmed = false;
        match child.kill().await {
            Ok(()) => match child.try_wait() {
                Ok(Some(status)) => {
                    failure.exit = Some(status.exit());
                    failure.completion = Some(status.completion());
                    failure.cleanup_confirmed = true;
                }
                Err(error) => failure.cleanup_error = Some(error),
                Ok(None) => {}
            },
            Err(error) => failure.cleanup_error = Some(error),
        }
        return Err(failure);
    }
    Ok(child)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn exact_broker_exit_reason_is_not_downgraded_to_nonzero_exit() {
        use axum::response::IntoResponse;
        let started = weftext_worker_broker::Started {
            version: 1,
            job_id: "a".repeat(32),
            pid: 123,
            image_sha256: "b".repeat(64),
        };
        for (completion, kind, code, signal) in [
            (Completion::Timeout, Kind::Deadline, None, Some(9)),
            (Completion::Cancelled, Kind::Cancelled, None, Some(9)),
            (
                Completion::Disconnected,
                Kind::SupervisionLost,
                None,
                Some(9),
            ),
            (
                Completion::SetupFailed,
                Kind::SupervisionLost,
                None,
                Some(9),
            ),
            (Completion::Exited, Kind::NonzeroExit, Some(7), None),
        ] {
            let receipt = Receipt {
                version: 1,
                job_id: started.job_id.clone(),
                completion: completion.clone(),
                exit_code: code,
                signal,
                reaped: 1,
                cgroup_empty: true,
                pidfd_exited: true,
            };
            receipt.validate(&started).unwrap();
            let status = Status::from_receipt(&receipt).unwrap();
            assert_eq!(status.completion(), completion);
            assert_eq!(status.failure_kind(), kind);
            assert!(!status.success());
            let mut failure = WorkerFailure::new(status.failure_kind());
            failure.exit = Some(status.exit());
            failure.completion = Some(status.completion());
            failure.stdout.bytes = b"PRIVATE WORKER SOURCE".to_vec();
            let response = crate::ApiError::worker_failure(failure).into_response();
            let (expected_status, expected_code) = match completion {
                Completion::Timeout => (axum::http::StatusCode::REQUEST_TIMEOUT, "compile_timeout"),
                Completion::Cancelled => (axum::http::StatusCode::CONFLICT, "compiler_cancelled"),
                Completion::Exited => (
                    axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                    "compiler_failed",
                ),
                Completion::Disconnected | Completion::SetupFailed => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "compiler_supervisor_unknown",
                ),
            };
            assert_eq!(response.status(), expected_status);
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["error"]["code"], expected_code);
            assert!(!std::str::from_utf8(&bytes).unwrap().contains("PRIVATE"));
        }
        let success = Status::from_receipt(&Receipt {
            version: 1,
            job_id: started.job_id,
            completion: Completion::Exited,
            exit_code: Some(0),
            signal: None,
            reaped: 1,
            cgroup_empty: true,
            pidfd_exited: true,
        })
        .unwrap();
        assert!(success.success());
    }

    use super::*;
    #[tokio::test]
    async fn detached_caller_retains_admission_until_original_terminal() {
        let gate = Arc::new(tokio::sync::Semaphore::new(1));
        let mut admission = Admission::claim(gate.clone()).unwrap();
        admission.begin_launch();
        let (release, released) = tokio::sync::oneshot::channel();
        let (done, completed) = tokio::sync::oneshot::channel();
        let caller = tokio::spawn(async move {
            let _ = released.await;
            admission.settle();
            drop(admission);
            done.send(()).unwrap();
        });
        drop(caller); // HTTP disconnect does not surrender accepted ownership.
        for _ in 0..16 {
            assert_eq!(
                Admission::claim(gate.clone()).err().unwrap().kind(),
                io::ErrorKind::WouldBlock
            );
        }
        release.send(()).unwrap();
        completed.await.unwrap();
        assert!(Admission::claim(gate).is_ok());
    }
    #[test]
    fn prelaunch_rejection_releases_but_unknown_termination_closes_admission() {
        let gate = Arc::new(tokio::sync::Semaphore::new(1));
        drop(Admission::claim(gate.clone()).unwrap());
        let mut rejected = Admission::claim(gate.clone()).unwrap();
        rejected.begin_launch();
        rejected.settle(); // Known validation rejection before launch send.
        drop(rejected);
        let mut unknown = Admission::claim(gate.clone()).unwrap();
        unknown.begin_launch();
        drop(unknown);
        assert!(gate.is_closed());
        assert_eq!(
            Admission::claim(gate).err().unwrap().kind(),
            io::ErrorKind::BrokenPipe
        );
    }
    #[test]
    fn a_terminal_tree_receipt_is_not_necessarily_product_success() {
        for completion in [
            Completion::Exited,
            Completion::Cancelled,
            Completion::Timeout,
            Completion::Disconnected,
            Completion::SetupFailed,
        ] {
            let receipt = Receipt {
                version: 1,
                job_id: "a".repeat(32),
                completion: completion.clone(),
                exit_code: Some(0),
                signal: None,
                reaped: 1,
                cgroup_empty: true,
                pidfd_exited: true,
            };
            assert_eq!(
                Status::from_receipt(&receipt).unwrap().success(),
                completion == Completion::Exited
            );
            let signal = Receipt {
                exit_code: None,
                signal: Some(9),
                ..receipt.clone()
            };
            assert!(!Status::from_receipt(&signal).unwrap().success());
            let invalid = Receipt {
                exit_code: Some(0),
                signal: Some(9),
                ..receipt
            };
            assert!(Status::from_receipt(&invalid).is_err());
        }
    }
    #[tokio::test]
    async fn absent_broker_never_falls_back_to_the_current_executable() {
        for notes in [false, true] {
            let error = match spawn(None, notes).await {
                Err(error) => error,
                Ok(_) => panic!("unqualified worker"),
            };
            assert_eq!(error.kind, Kind::Setup);
            assert!(error.cleanup_confirmed);
            assert_eq!(error.io_error.unwrap().kind(), io::ErrorKind::Unsupported);
        }
    }
    #[tokio::test]
    async fn anonymous_pipe_pump_preserves_bytes_and_has_no_blocking_pool_reads() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut sender, mut receiver) = tokio::net::unix::pipe::pipe().unwrap();
        let original: Vec<u8> = (0..(2 * 1024 * 1024)).map(|i| (i % 251) as u8).collect();
        let expected = original.clone();
        let writer = tokio::spawn(async move {
            sender.write_all(&original).await.unwrap();
        });
        let mut output = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), receiver.read_to_end(&mut output))
            .await
            .unwrap()
            .unwrap();
        writer.await.unwrap();
        assert_eq!(output, expected);
        let (_sender, mut pending) = tokio::net::unix::pipe::pipe().unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), pending.read_u8())
                .await
                .is_err()
        );
    }
}
