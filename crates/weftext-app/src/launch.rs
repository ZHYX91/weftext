//! The two PE roles share earliest worker dispatch and one owner/service path.
//! The console owns business CLI output; the default desktop owns native UI.
use crate::host_environment;
#[cfg(windows)]
use crate::local_service::{InitialDocument, LocalServiceOptions};
use clap::Parser;
use std::{ffi::OsString, io, path::PathBuf};

#[derive(Debug, Clone, clap::Args)]
pub struct ServeArgs {
    #[arg(skip)]
    pub data_dir: Option<PathBuf>,
    /// Administrator-enrolled authority for the controlled Linux service.
    #[arg(long)]
    pub service_enrollment: Option<PathBuf>,
    /// Administrator-owned worker policy bound to that same Linux enrollment.
    #[arg(long)]
    pub worker_broker_policy: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    pub port: u16,
    #[arg(long,conflicts_with_all=["open","root","port"])]
    pub server_config: Option<PathBuf>,
    #[arg(long)]
    pub open: Option<PathBuf>,
    #[arg(long, requires = "open")]
    pub root: Option<PathBuf>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServeReady<'a> {
    pub schema_version: u32,
    pub kind: &'static str,
    pub url: &'a str,
    pub owner_identity: &'a crate::local_client::OwnerIdentity,
}
#[derive(Debug)]
pub struct ServeError {
    pub code: &'static str,
    pub diagnostic: &'static str,
    pub local_origin: Option<LocalOriginDiagnostic>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalOriginPhase {
    RootQualification,
    UserDataPath,
    ProfileSeparation,
    ProfileCanonicalize,
    CatalogLock,
    CatalogRead,
    BrowserDataQualification,
    OriginMetadataMissing,
    LocatorConflict,
    RetainedPortMismatch,
    RetainedBind,
    RequestedPortReserved,
    CandidateBind,
    CandidateAddress,
    CandidateExhausted,
    ListenerAddress,
    CatalogRevision,
    CatalogPublish,
    ListenerNonblocking,
    ListenerAdopt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalOriginDiagnostic {
    phase: LocalOriginPhase,
    #[serde(serialize_with = "serialize_io_kind")]
    io_kind: io::ErrorKind,
    os_code: Option<i32>,
}
fn serialize_io_kind<S: serde::Serializer>(
    kind: &io::ErrorKind,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    // Only the standard library's typed discriminant, never an error's
    // Display/Debug, source chain, paths or other runtime text.
    serializer.collect_str(&format_args!("{kind:?}"))
}
impl LocalOriginDiagnostic {
    #[cfg(any(windows, test))]
    pub(crate) fn from_io(phase: LocalOriginPhase, error: &io::Error) -> Self {
        // Root qualification also takes the catalog lock. Preserve that exact
        // boundary through its io::Result without inspecting arbitrary sources.
        if let Some(original) = error
            .get_ref()
            .and_then(|source| source.downcast_ref::<Self>())
        {
            return *original;
        }
        Self {
            phase,
            io_kind: error.kind(),
            os_code: error.raw_os_error(),
        }
    }
    #[cfg(windows)]
    pub(crate) fn into_io(self) -> io::Error {
        io::Error::new(self.io_kind, self)
    }
}
impl std::fmt::Display for LocalOriginDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The local origin operation failed")
    }
}
impl std::error::Error for LocalOriginDiagnostic {}
impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.diagnostic)
    }
}
impl std::error::Error for ServeError {}
pub(crate) fn write_ready(url: &str, owner: &crate::local_client::OwnerIdentity) -> io::Result<()> {
    use std::io::Write;
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(
        &mut output,
        &ServeReady {
            schema_version: 1,
            kind: "ready",
            url,
            owner_identity: owner,
        },
    )?;
    output.write_all(b"\n")?;
    output.flush()
}
pub fn default_data_dir() -> io::Result<PathBuf> {
    host_environment::default_data_dir()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Worker {
    Render,
    Notes,
}
fn worker(arguments: &[OsString]) -> io::Result<Option<Worker>> {
    let render = arguments.iter().any(|value| value == "--render-worker");
    let notes = arguments.iter().any(|value| value == "--notes-edit-worker");
    if !render && !notes {
        return Ok(None);
    }
    if arguments.len() != 1 || render && notes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "An internal worker accepts exactly its one role flag",
        ));
    }
    Ok(Some(if render {
        Worker::Render
    } else {
        Worker::Notes
    }))
}
/// Both synchronous executable mains call this before a runtime, path lookup,
/// artifact discovery, clap business parsing, COM, Core or any GUI side effect.
pub fn dispatch_worker() -> Option<Result<(), Box<dyn std::error::Error>>> {
    match worker(&std::env::args_os().skip(1).collect::<Vec<_>>()) {
        Ok(None) => None,
        Err(error) => Some(Err(error.into())),
        Ok(Some(Worker::Render)) => Some(crate::worker_main().or_else(|error| {
            crate::write_render_worker_failure(error, &mut std::io::stdout().lock())
        })),
        Ok(Some(Worker::Notes)) => Some(crate::notes_worker_main()),
    }
}

#[derive(Debug, Parser)]
#[command(name = "weftext", version, about = "Weftext Windows desktop")]
struct DesktopArgs {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long,conflicts_with_all=["login_startup","service_owner","toast_server"])]
    capture: bool,
    #[arg(long,conflicts_with_all=["capture","service_owner","toast_server","data_dir"])]
    login_startup: bool,
    #[arg(long,hide=true,conflicts_with_all=["capture","login_startup","toast_server"])]
    service_owner: bool,
    #[arg(long,hide=true,conflicts_with_all=["capture","login_startup","service_owner","data_dir"])]
    toast_server: bool,
    #[arg(long,hide=true,conflicts_with_all=["capture","login_startup","service_owner","data_dir","toast_server"])]
    toast_owner: Option<String>,
}

pub async fn desktop() -> Result<(), ServeError> {
    let mut raw = std::env::args_os().collect::<Vec<_>>();
    // COM LocalServer32 appends this protocol token. It is accepted only on the
    // exact toast-server route, never as a generic hidden launch argument.
    let embeddings = raw
        .iter()
        .skip(1)
        .filter(|value| {
            value
                .to_str()
                .is_some_and(|value| value.eq_ignore_ascii_case("-Embedding"))
        })
        .count();
    if embeddings != 0 {
        if embeddings != 1 || !raw.iter().any(|value| value == "--toast-server") {
            return Err(ServeError {
                code: "desktopArguments",
                diagnostic: "The COM embedding token requires the notification server role",
                local_origin: None,
            });
        }
        raw.retain(|value| {
            !value
                .to_str()
                .is_some_and(|value| value.eq_ignore_ascii_case("-Embedding"))
        });
    }
    let args = DesktopArgs::try_parse_from(raw).map_err(|_| ServeError {
        code: "desktopArguments",
        diagnostic: "Invalid desktop arguments; use weftext-cli for formal console commands",
        local_origin: None,
    })?;
    #[cfg(windows)]
    {
        if args.toast_server {
            return crate::native::toast_server().await;
        }
        if let Some(opaque) = args.toast_owner {
            return crate::native::run_toast_owner(&opaque).await;
        }
        if args.login_startup {
            return crate::native::run_login_startup().await;
        }
        let data = args
            .data_dir
            .map_or_else(default_data_dir, Ok)
            .map_err(|_| ServeError {
                code: "profileUnavailable",
                diagnostic: "The persistent profile location is unavailable",
                local_origin: None,
            })?;
        let activation = if args.service_owner {
            crate::native::Activation::Headless
        } else if args.capture {
            crate::native::Activation::Capture
        } else {
            crate::native::Activation::Library
        };
        crate::native::run(LocalServiceOptions::new(data), activation, false).await
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err(ServeError {
            code: "desktopUnsupported",
            diagnostic: "This native desktop role requires Windows 11 x64",
            local_origin: None,
        })
    }
}

/// A manual desktop start needs a visible canonical failure even though its
/// PE has no console. OS/background roles never display or activate an error UI.
pub fn report_desktop_error(error: &ServeError) {
    let background = std::env::args_os().skip(1).any(|argument| {
        argument.to_str().is_some_and(|argument| {
            [
                "--service-owner",
                "--login-startup",
                "--toast-server",
                "--toast-owner",
            ]
            .iter()
            .any(|flag| {
                argument == *flag
                    || argument
                        .strip_prefix(flag)
                        .is_some_and(|rest| rest.starts_with('='))
            }) || argument.eq_ignore_ascii_case("-Embedding")
        })
    });
    if !background {
        #[cfg(windows)]
        crate::native::report_start_failure(error);
        #[cfg(not(windows))]
        let _ = error;
    }
}

pub async fn serve(args: ServeArgs) -> Result<(), ServeError> {
    let data = args
        .data_dir
        .clone()
        .map_or_else(default_data_dir, Ok)
        .map_err(|_| ServeError {
            code: "profileUnavailable",
            diagnostic: "The persistent profile location is unavailable",
            local_origin: None,
        })?;
    #[cfg(windows)]
    {
        if args.service_enrollment.is_some() || args.worker_broker_policy.is_some() {
            return Err(ServeError {
                code: "invalidServiceEnrollment",
                diagnostic: "The Windows service uses its actual Windows authority, not a Linux enrollment",
                local_origin: None,
            });
        }
        if let Some(config) = args.server_config {
            return crate::hosted_owner::run(data, config).await;
        }
        crate::native::run(
            LocalServiceOptions {
                data_dir: data,
                port: args.port,
                initial_document: args.open.map(|path| InitialDocument {
                    path,
                    root: args.root,
                }),
            },
            crate::native::Activation::Headless,
            true,
        )
        .await
    }
    #[cfg(target_os = "linux")]
    {
        controlled_hosted(args, data).await
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (data, args);
        Err(ServeError {
            code: "serviceAuthorityRequired",
            diagnostic: "This host has no qualified service authority",
            local_origin: None,
        })
    }
}

#[cfg(target_os = "linux")]
async fn controlled_hosted(args: ServeArgs, data: PathBuf) -> Result<(), ServeError> {
    let enrollment = args.service_enrollment.ok_or(ServeError {
        code: "serviceAuthorityRequired",
        diagnostic: "Controlled Linux serve requires an administrator enrollment",
        local_origin: None,
    })?;
    let policy = args.worker_broker_policy.ok_or(ServeError {
        code: "workerAuthorityRequired",
        diagnostic: "Controlled Linux serve requires its explicit administrator worker policy",
        local_origin: None,
    })?;
    let profile = data.clone();
    let (backend, workers) = crate::work::spawn_blocking(move || {
        let backend = weftext_core::BackendContext::controlled_service_for_profile(&enrollment, &profile)
            .map_err(|_| ServeError { code: "serviceAuthorityRejected",
                diagnostic: "The actual UID, privileges, enrollment and filesystem roots did not qualify", local_origin: None })?;
        let workers = crate::compiler_worker::Transport::controlled(&backend, &policy)
            .map_err(|_| ServeError { code: "workerAuthorityRejected",
                diagnostic: "The worker policy did not qualify against the original Core enrollment", local_origin: None })?;
        Ok::<_, ServeError>((backend, workers))
    }).await.map_err(|_| ServeError { code: "workerAuthorityUnavailable",
        diagnostic: "The original Core and worker authority could not be inspected", local_origin: None })??;
    let config=args.server_config.ok_or(ServeError{code:"hostedConfigurationRequired",diagnostic:"Controlled Linux serve currently requires the explicit hosted configuration; local owner IPC is unavailable", local_origin: None })?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|_| ServeError {
        code: "shutdownSignalUnavailable",
        diagnostic: "The controlled service termination signal could not be monitored",
        local_origin: None,
    })?;
    let core = weftext_core::Core::with_backend(&data, backend).map_err(|_| ServeError {
        code: "coreUnavailable",
        diagnostic: "The exact enrolled persistent Core could not be opened",
        local_origin: None,
    })?;
    let service=crate::hosted::HostedService::bind(core,config,data.join("legacy-office"),workers).await.map_err(|error|{
        if error.is::<crate::hosted::PairingMismatch>(){ServeError{code:"syncPairingMismatch",diagnostic:"Persisted sync pairing differs from configuration; existing grants cannot silently move to another family", local_origin: None }}
        else{ServeError{code:"hostedConfiguration",diagnostic:"The controlled hosted configuration could not be bound", local_origin: None }}
    })?;
    let ready = serde_json::json!({"schemaVersion":1,"kind":"ready","url":service.origin(),"serviceAuthority":{"kind":"linuxControlledService","storageProfile":weftext_core::STORAGE_PROFILE}});
    // No Windows PID/FILETIME/broker identity is invented on this platform.
    let output = (|| -> io::Result<()> {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer(&mut stdout, &ready)?;
        stdout.write_all(b"\n")?;
        stdout.flush()
    })();
    let output_failed = output.is_err();
    service.serve_until(async move{if !output_failed{tokio::select!{_ = terminate.recv()=>{},_ = tokio::signal::ctrl_c()=>{}}}}).await.map_err(|_|ServeError{code:"shutdownUnknown",diagnostic:"The controlled service did not confirm accepted-work drain and final Core release", local_origin: None })?;
    if output_failed {
        Err(ServeError {
            code: "readyOutput",
            diagnostic: "The controlled service Ready output was not confirmed; accepted work was drained",
            local_origin: None,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn origin_qualification_keeps_the_original_catalog_lock_diagnostic() {
        let original = LocalOriginDiagnostic::from_io(
            LocalOriginPhase::CatalogLock,
            &io::Error::from_raw_os_error(5),
        );
        let wrapped = original.into_io();
        assert_eq!(
            LocalOriginDiagnostic::from_io(LocalOriginPhase::RootQualification, &wrapped),
            original
        );
        assert_eq!(serde_json::to_value(original).unwrap()["osCode"], 5);
    }
    #[test]
    fn internal_workers_are_exact_earliest_roles() {
        for flag in ["--render-worker", "--notes-edit-worker"] {
            assert!(worker(&[flag.into()]).unwrap().is_some());
            for extra in [
                "--capture",
                "--data-dir",
                "--notes-edit-worker",
                "--render-worker",
                "--toast-server",
            ] {
                assert!(worker(&[flag.into(), extra.into()]).is_err());
            }
        }
        assert_eq!(worker(&["--capture".into()]).unwrap(), None);
    }
    #[test]
    fn desktop_roles_are_independent_and_old_browser_mode_is_absent() {
        assert!(DesktopArgs::try_parse_from(["weftext"]).is_ok());
        for mode in ["--capture", "--service-owner"] {
            assert!(DesktopArgs::try_parse_from(["weftext", mode, "--data-dir", "TEST"]).is_ok());
        }
        assert!(DesktopArgs::try_parse_from(["weftext", "--login-startup"]).is_ok());
        assert!(
            DesktopArgs::try_parse_from(["weftext", "--login-startup", "--data-dir", "TEST"])
                .is_err()
        );
        assert!(DesktopArgs::try_parse_from(["weftext", "--no-browser"]).is_err());
        assert!(DesktopArgs::try_parse_from(["weftext", "--capture", "--login-startup"]).is_err());
        assert!(
            DesktopArgs::try_parse_from(["weftext", "--toast-server", "--data-dir", "TEST"])
                .is_err()
        );
    }
}
