//! The desktop owner, local console serve and CLI service owner share this one
//! Core/HTTP/broker supervisor. Only an actual desktop activation starts STA UI.
mod apartment;
mod close;
mod events;
mod notice;
mod notification;
mod package;
#[cfg(test)]
mod proof_tests;
mod runtime;
mod scheduler;
mod startup;
mod terminal;
mod toast_server;
mod tray;
mod ui;
mod view;
mod window;
mod winrt;

use crate::{
    broker::{
        Command,
        windows::{HostRequest, OwnerPipe},
    },
    launch::ServeError,
    local_client::{OwnerRole, ProfileLocator},
    local_service::{LocalService, LocalServiceOptions},
};
use close::{Proof, SurfaceKind};
use std::{
    io,
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};
use weftext_messages::{UiLocale, UiMessage};

pub(crate) async fn toast_server() -> Result<(), ServeError> {
    toast_server::run().await
}

async fn activate_original_toast(opaque: &str, app_id: &str) -> io::Result<()> {
    let opaque_owned = opaque.to_owned();
    let route = crate::work::spawn_blocking(move || {
        crate::host_state::Root::qualify(false)?.lookup(&opaque_owned)
    })
    .await
    .map_err(io::Error::other)??;
    if route.app_id != app_id {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The original notification identity differs",
        ));
    }
    crate::broker::windows::activate_original_toast(route.profile, opaque, Duration::from_secs(45))
        .await
        .map_err(io::Error::other)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Activation {
    Library,
    Capture,
    LoginStartup,
    Headless,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExistingReason {
    Toast,
    Login,
}
struct ExistingOwner {
    target: crate::host_state::ExistingProfile,
    reason: ExistingReason,
}
fn error(code: &'static str, diagnostic: &'static str) -> ServeError {
    ServeError { code, diagnostic }
}
fn closing() -> io::Error {
    io::Error::other("The owner is already closing or the author surface cannot safely close")
}
pub(crate) fn report_start_failure(failure: &ServeError) {
    // Startup has not established a qualified Core/UI preference. Use the
    // catalog default rather than opening another profile merely to show an error.
    let message = if failure.code == "webviewRuntimeUnavailable" {
        UiMessage::native_runtime_unavailable()
    } else {
        UiMessage::native_host_start_failed()
    };
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            None,
            &windows::core::HSTRING::from(message.render(UiLocale::default())),
            windows::core::w!("Weftext"),
            windows::Win32::UI::WindowsAndMessaging::MB_OK
                | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}

struct Desktop {
    thread: ui::Thread,
    _paths: crate::host_paths::NativePaths,
    preferences: weftext_core::DesktopPreferencesSnapshot,
    locale: UiLocale,
    startup: startup::Observation,
    continuation: Arc<crate::broker::lifetime::ServiceContinuation>,
}
impl Desktop {
    async fn start(
        state: &crate::AppState,
        profile: &weftext_core::QualifiedProfile,
        continuation: &Arc<crate::broker::lifetime::ServiceContinuation>,
    ) -> Result<Self, ServeError> {
        qualify_gui()?;
        runtime::qualify()?;
        let paths = crate::host_paths::NativePaths::qualify(profile).map_err(|_| {
            error(
                "desktopPaths",
                "The durable Core and disposable WebView namespaces could not be qualified",
            )
        })?;
        let preferences = crate::core_call(state, |core| core.desktop_preferences())
            .await
            .map_err(|_| {
                error(
                    "desktopPreferences",
                    "The exact device's desktop preferences could not be read",
                )
            })?;
        // This is the same Core profile display preference used by the WebUI;
        // it does not create or infer a separate OS/device language setting.
        let locale = crate::core_call(state, |core| core.client_preferences())
            .await
            .map_err(|_| {
                error(
                    "clientPreferences",
                    "The personal interface preference could not be read",
                )
            })?
            .preferences
            .ui_locale;
        let startup = startup::observe(profile, &preferences.device).await;
        let thread = ui::Thread::start(paths.user_data.clone(), continuation.clone())
            .map_err(|_| error("nativeThread", "The native STA could not be started"))?;
        Ok(Self {
            thread,
            _paths: paths,
            preferences,
            locale,
            startup,
            continuation: continuation.clone(),
        })
    }
    async fn reload(
        &mut self,
        state: &crate::AppState,
        profile: &weftext_core::QualifiedProfile,
    ) -> Result<(), ServeError> {
        self.preferences = crate::core_call(state, |core| core.desktop_preferences())
            .await
            .map_err(|_| {
                error(
                    "desktopPreferences",
                    "The original desktop intent is unavailable",
                )
            })?;
        self.locale = crate::core_call(state, |core| core.client_preferences())
            .await
            .map_err(|_| {
                error(
                    "clientPreferences",
                    "The personal interface preference is unavailable",
                )
            })?
            .preferences
            .ui_locale;
        self.startup = startup::observe(profile, &self.preferences.device).await;
        Ok(())
    }
    fn refresh(&self) -> Result<(), ServeError> {
        self.refresh_with_information(None)
    }
    fn refresh_with_information(&self, information: Option<UiMessage>) -> Result<(), ServeError> {
        self.thread
            .sender
            .send(ui::Command::Preferences {
                locale: self.locale,
                preferences: self.preferences.effective_preferences(),
                startup: self.startup,
                cli_service: self.continuation.cli_active(),
                information,
            })
            .map_err(|_| error("nativeDispatcher", "The native dispatcher is unavailable"))
    }
    fn feedback(&self, failure: &ServeError) -> Result<(), ServeError> {
        let message = match failure.code {
            "webviewRuntimeUnavailable" => UiMessage::native_runtime_unavailable(),
            "startupUnavailable" => UiMessage::native_startup_unavailable(),
            "startupRequestUnknown" => UiMessage::desktop_startup_request_failed(),
            "preferenceConflict" => UiMessage::desktop_preference_update_failed(),
            _ => UiMessage::native_host_start_failed(),
        };
        self.thread
            .sender
            .send(ui::Command::Feedback(message))
            .map_err(|_| {
                error(
                    "nativeDispatcher",
                    "The native feedback could not be delivered",
                )
            })
    }
    fn open(&self, kind: SurfaceKind, url: &str, admission_epoch: u64) -> Result<(), ServeError> {
        if kind == SurfaceKind::Capture && crate::ASSETS.get_file("capture.html").is_none() {
            return Err(error(
                "captureUnavailable",
                "This build has no complete Capture entry",
            ));
        }
        let mut address = url::Url::parse(url)
            .map_err(|_| error("localAddress", "The private local UI address is invalid"))?;
        if kind == SurfaceKind::Capture {
            address.set_path("/capture.html");
        }
        self.thread
            .sender
            .send(ui::Command::Open {
                kind,
                address: address.into(),
                admission_epoch,
            })
            .map_err(|_| error("nativeDispatcher", "The native dispatcher is unavailable"))
    }
}

/// The GUI/package support contract is deliberately scoped to desktop effects.
/// Headless Core/HTTP/CLI/worker checks on Server 2022 do not call this function.
fn qualify_gui() -> Result<(), ServeError> {
    use windows::Win32::System::SystemInformation::*;
    let mut version = OSVERSIONINFOEXW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOEXW>() as u32,
        dwMajorVersion: 10,
        dwMinorVersion: 0,
        dwBuildNumber: 22000,
        wProductType: 1,
        ..Default::default()
    };
    let mut condition = 0;
    for field in [
        VER_MAJORVERSION,
        VER_MINORVERSION,
        VER_SERVICEPACKMAJOR,
        VER_SERVICEPACKMINOR,
        VER_BUILDNUMBER,
    ] {
        condition = unsafe { VerSetConditionMask(condition, field, 3) };
    }
    condition = unsafe { VerSetConditionMask(condition, VER_PRODUCT_TYPE, 1) };
    unsafe {
        VerifyVersionInfoW(
            &mut version,
            VER_MAJORVERSION
                | VER_MINORVERSION
                | VER_SERVICEPACKMAJOR
                | VER_SERVICEPACKMINOR
                | VER_BUILDNUMBER
                | VER_PRODUCT_TYPE,
            condition,
        )
    }
    .map_err(|_| {
        error(
            "desktopPlatform",
            "The Windows desktop requires Windows 11 x64 build 22000 or later",
        )
    })
}

pub(crate) async fn run(
    options: LocalServiceOptions,
    activation: Activation,
    console_ready: bool,
) -> Result<(), ServeError> {
    run_owner(options, activation, console_ready, None).await
}

pub(crate) async fn run_toast_owner(opaque: &str) -> Result<(), ServeError> {
    let opaque = opaque.to_owned();
    let route = crate::work::spawn_blocking(move || {
        crate::host_state::Root::qualify(false)?.lookup(&opaque)
    })
    .await
    .map_err(|_| {
        error(
            "toastProfileUnknown",
            "The original notification profile was not confirmed",
        )
    })?
    .map_err(|_| {
        error(
            "toastProfileUnavailable",
            "The original notification profile is unavailable",
        )
    })?;
    run_owner(
        LocalServiceOptions::new(route.profile.path().to_owned()),
        Activation::Headless,
        false,
        Some(ExistingOwner {
            target: route.into_existing(),
            reason: ExistingReason::Toast,
        }),
    )
    .await
}

pub(crate) async fn run_login_startup() -> Result<(), ServeError> {
    let original =
        crate::work::spawn_blocking(|| crate::host_state::Root::qualify(false)?.startup_profile())
            .await
            .map_err(|_| {
                error(
                    "startupProfileUnknown",
                    "The original startup profile was not confirmed",
                )
            })?
            .map_err(|_| {
                error(
                    "startupProfileUnavailable",
                    "The original startup profile is unavailable",
                )
            })?;
    let Some(target) = original else {
        return Ok(());
    };
    run_owner(
        LocalServiceOptions::new(target.profile.path().into()),
        Activation::LoginStartup,
        false,
        Some(ExistingOwner {
            target,
            reason: ExistingReason::Login,
        }),
    )
    .await
}

async fn run_owner(
    options: LocalServiceOptions,
    activation: Activation,
    console_ready: bool,
    existing: Option<ExistingOwner>,
) -> Result<(), ServeError> {
    crate::artifact_catalog::Catalog::get().map_err(|_| {
        error(
            "artifactQualification",
            "This executable is not a complete qualified Weftext build",
        )
    })?;
    let profile_path = options.data_dir.clone();
    let expected_profile = existing
        .as_ref()
        .map(|original| original.target.profile.physical_identity().to_owned());
    let profile =
        crate::work::spawn_blocking(move || -> io::Result<weftext_core::QualifiedProfile> {
            if let Some(expected) = expected_profile {
                let actual = weftext_core::qualify_existing_profile(&profile_path)
                    .map_err(io::Error::other)?;
                crate::host_state::Root::qualify(false)?.separate(&actual)?;
                if actual.physical_identity() != expected {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "The original physical profile changed",
                    ))
                } else {
                    Ok(actual)
                }
            } else {
                crate::host_paths::qualify_core(&profile_path)
            }
        })
        .await
        .map_err(|_| {
            error(
                "profileQualification",
                "The original profile qualification did not complete",
            )
        })?
        .map_err(|_| {
            error(
                "profileQualification",
                "The exact persistent profile could not be qualified",
            )
        })?;
    let data = profile.path().to_path_buf();
    let owner = match OwnerPipe::acquire(
        weftext_core::qualify_existing_profile(&data).map_err(|_| {
            error(
                "profileQualification",
                "The exact persistent profile changed",
            )
        })?,
        OwnerRole::Local,
    ) {
        Ok(owner) => owner,
        Err(_)
            if (!console_ready && activation != Activation::Headless)
                && existing
                    .as_ref()
                    .is_none_or(|original| original.reason == ExistingReason::Login) =>
        {
            // Occupied/unknown election never opens Core. Only this same
            // protected existing owner may receive an explicit activation.
            let command = match activation {
                Activation::Capture => Command::ShowCapture,
                Activation::Library => Command::ShowLibrary { document: None },
                Activation::LoginStartup => Command::LoginStartup,
                Activation::Headless => unreachable!(),
            };
            crate::broker::windows::activate_existing(&ProfileLocator{data_dir:data},command,Duration::from_secs(30)).await.map_err(|_|error("ownerActivationUnknown","The existing qualified owner activation was not confirmed; no second Core was opened"))?;
            return Ok(());
        }
        Err(_) => {
            return Err(error(
                "ownerUnavailable",
                "This physical profile already has an owner or owner qualification failed; no second Core was opened",
            ));
        }
    };
    let identity = owner.identity();
    let _owner_name = owner.retain_name().map_err(|_| {
        error(
            "ownerLease",
            "The original owner name could not be retained through Core release",
        )
    })?;
    let origin_profile = weftext_core::qualify_existing_profile(&data).map_err(|_| {
        error(
            "profileQualification",
            "The existing physical profile changed",
        )
    })?;
    let requested_port = options.port;
    let origin_guard = owner.retain_name().map_err(|_| {
        error(
            "ownerLease",
            "The original owner name could not be retained through local origin binding",
        )
    })?;
    let (listener, origin_guard, origin_profile) = crate::work::spawn_blocking(move || {
        let listener =
            crate::host_state::Root::qualify(false)?.bind_local(&origin_profile, requested_port)?;
        Ok::<_, io::Error>((listener, origin_guard, origin_profile))
    })
    .await
    .map_err(|_| {
        error(
            "localOriginUnknown",
            "The original local origin reservation did not complete",
        )
    })?
    .map_err(|_| {
        error(
            "localOriginUnavailable",
            "The retained local origin is unavailable; no changed-origin fallback was used",
        )
    })?;
    let listener = tokio::net::TcpListener::from_std(listener).map_err(|_| {
        error(
            "localOriginUnavailable",
            "The retained local listener could not be adopted",
        )
    })?;
    let options = LocalServiceOptions {
        data_dir: data,
        ..options
    };
    // Cancellation must retain both the owner election and the exact physical
    // directory through the worker's real Core-open tail.
    let opening_guard = (origin_guard, origin_profile);
    let service = if let Some(original) = &existing {
        LocalService::bind_existing_with_listener(
            options,
            listener,
            original.target.profile.physical_identity(),
            &original.target.device_id,
            opening_guard,
        )
        .await
    } else {
        LocalService::bind_with_listener(options, listener, opening_guard).await
    }
    .map_err(|_| {
        error(
            "localService",
            "The exact local Core service could not be bound",
        )
    })?;
    let state = service.authority();
    if existing
        .as_ref()
        .is_some_and(|original| original.reason == ExistingReason::Login)
    {
        let enabled = crate::core_call(&state, |core| {
            Ok(core
                .desktop_preferences()?
                .effective_preferences()
                .login_startup_requested)
        })
        .await
        .map_err(|_| {
            error(
                "startupPreferences",
                "The original startup intent could not be qualified",
            )
        })?;
        if !enabled {
            drop(state);
            drop(service);
            return Ok(());
        }
    }
    let url = service.browser_url().to_owned();
    let continuation = Arc::new(crate::broker::lifetime::ServiceContinuation::new(
        // Toast bootstrap is temporarily windowless, but is not a CLI service.
        // Its qualified user activation must still obey last-window residency.
        activation == Activation::Headless && existing.is_none(),
    ));
    let (stop_broker, broker_stopping) = watch::channel(false);
    let (stop_http, mut http_stopping) = watch::channel(false);
    let (requests, mut requested) = mpsc::channel(16);
    let mut broker = tokio::spawn(owner.serve(
        Some(url.clone()),
        requests,
        Arc::new(AtomicUsize::new(0)),
        continuation.clone(),
        broker_stopping,
    ));
    let mut serving = tokio::spawn(service.serve_retaining_until(async move {
        if !*http_stopping.borrow() {
            let _ = http_stopping.changed().await;
        }
    }));
    let mut broker_result = None;
    let mut http_result = None;
    let mut desktop = None;
    let mut failure = if console_ready {
        crate::launch::write_ready(&url, &identity).err().map(|_| {
            error(
                "readyOutput",
                "The service bound but its Ready output was not confirmed; shutdown was requested",
            )
        })
    } else {
        None
    };
    if failure.is_none()
        && activation != Activation::Headless
        && let Err(error) = activate(
            &state,
            &profile,
            &url,
            &mut desktop,
            activation,
            &continuation,
        )
        .await
    {
        failure = Some(error);
    }
    let mut shutdown_reply: Option<oneshot::Sender<io::Result<Vec<u8>>>> = None;
    let mut exit_requested = false;
    let mut native_faulted = false;
    let mut control_open = true;
    let mut notices_open = true;
    let mut console_signals = console_ready;
    let mut toast_starting = existing
        .as_ref()
        .is_some_and(|original| original.reason == ExistingReason::Toast);
    let toast_deadline = tokio::time::Instant::now() + Duration::from_secs(35);
    let mut proofs = tokio::task::JoinSet::<(ui::Sender, ui::Command)>::new();
    let mut activations = tokio::task::JoinSet::<()>::new();
    let mut scheduler: Option<scheduler::Scheduler> = None;
    let mut scheduler_faulted = false;
    if (failure.is_none() || desktop.is_some())
        && !(failure.is_some() && automatic_exit(&desktop, &mut exit_requested))
    {
        loop {
            // Only this original notification wait ends after another actual
            // UI/CLI owner takes over. Their normal author barriers stay live.
            if toast_starting && continuation.has_takeover() {
                toast_starting = false;
            }
            if desktop.is_some() && scheduler.is_none() {
                scheduler = Some(scheduler::Scheduler::start(state.clone(), &profile));
            }
            if let Some(scheduler) = &scheduler {
                scheduler.pause(exit_requested || failure.is_some());
            }
            let mut ui_fault = desktop
                .as_ref()
                .map(|desktop: &Desktop| desktop.thread.fault.clone());
            let mut scheduler_fault = scheduler.as_ref().map(|scheduler| scheduler.fault.clone());
            tokio::select! {
                            biased;
                            _=async {if let Some(fault)=scheduler_fault.as_mut(){let current=*fault.borrow();if !current{let _=fault.changed().await;}}else{std::future::pending::<()>().await;}},if !scheduler_faulted=>{
                                scheduler_faulted=true;failure=Some(error("notificationCoordinatorStopped","The notification coordinator stopped unexpectedly; original delivery records remain retained"));
                                if automatic_exit(&desktop,&mut exit_requested){break;}
                            }
                            _=tokio::time::sleep_until(toast_deadline),if toast_starting=>{
                                toast_starting=false;
                                if continuation.end_unclaimed_bootstrap(){failure=Some(error("toastActivationUnknown","The original notification owner did not receive a qualified activation"));break;}
                            }
                            result=&mut broker,if broker_result.is_none()=>{
                                broker_result=Some(result);failure=Some(error("ownerStoppedUnexpectedly","The protected owner listener stopped unexpectedly"));
                                if automatic_exit(&desktop,&mut exit_requested){break;}
                            }
                            result=&mut serving,if http_result.is_none()=>{
                                http_result=Some(result);failure=Some(error("httpStoppedUnexpectedly","The local listener stopped unexpectedly; live input still requires the original Core proof"));
                                if automatic_exit(&desktop,&mut exit_requested){break;}
                            }
                            fault=async{match ui_fault.as_mut(){Some(fault)=>{if *fault.borrow(){true}else{fault.changed().await.is_err()||*fault.borrow()}},None=>std::future::pending().await}},if !native_faulted=>{
                                if fault{native_faulted=true;failure=Some(error("nativeFailed","The native dispatcher failed; a live unconfirmed author surface remains blocked"));if automatic_exit(&desktop,&mut exit_requested){break;}}
                            }
                            result=proofs.join_next(),if !proofs.is_empty()=>{
                                match result {
                                    Some(Ok((sender,command)))=>if sender.send(command).is_err(){failure=Some(error("nativeFailed","The native proof result could not be delivered"));if automatic_exit(&desktop,&mut exit_requested){break;}},
                                    _=>{failure=Some(error("proofUnknown","The original Core close verification did not complete"));if exit_requested&& let Some(desktop)=&desktop{let _=desktop.thread.sender.send(ui::Command::CancelClose);}
            if automatic_exit(&desktop,&mut exit_requested){break;}}
                                }
                            }
                            result=activations.join_next(),if !activations.is_empty()=>{
                                if !matches!(result,Some(Ok(()))) { failure.get_or_insert(error("toastNavigationUnknown","The original notification navigation did not complete")); }
                            }
                            request=requested.recv(),if control_open=>{
                                let Some(request)=request else{
                                    control_open=false;failure=Some(error("ownerControlUnavailable","The protected owner control channel became unavailable"));
                                    if automatic_exit(&desktop,&mut exit_requested){break;}
                                    continue;
                                };
                                let HostRequest{command,reply}=request;
                                if exit_requested{let _=reply.send(Err(closing()));continue;}
                                match command {
                                    Command::Shutdown=>{
                                        if let Some(native)=&desktop{if native.thread.sender.send(ui::Command::BeginExit).is_err(){let _=reply.send(Err(closing()));failure=Some(error("nativeFailed","The native close request was not delivered"));if native.thread.returned_without_exposed(){break;}continue;}shutdown_reply=Some(reply);exit_requested=true;}
                                        else{let _=reply.send(Ok(br#"{"accepted":true}"#.to_vec()));break;}
                                    }
                                    Command::ShowLibrary{document}=>{
                                        let result=activate(&state,&profile,&url,&mut desktop,Activation::Library,&continuation).await;
                                        let result=match (result,document){(Ok(()),Some(document))=>open_document(&state,desktop.as_ref().unwrap(),document).await,(result,_)=>result};
                                        let _=reply.send(result.map(|()|br#"{"accepted":true}"#.to_vec()).map_err(io::Error::other));
                                    }
                                    Command::ShowCapture=>{let result=activate(&state,&profile,&url,&mut desktop,Activation::Capture,&continuation).await;let _=reply.send(result.map(|()|br#"{"accepted":true}"#.to_vec()).map_err(io::Error::other));}
                                    Command::LoginStartup=>{let result=activate_login(&state,&profile,&url,&mut desktop,&continuation).await;let _=reply.send(result.map(|()|br#"{"accepted":true}"#.to_vec()).map_err(io::Error::other));}
                                    Command::ToastActivation{opaque}=>{
                                        let result=activate_toast(&state,&profile,&url,&mut desktop,opaque,&continuation).await;
                                        let failed=result.is_err();
                                        match result {
                                            Ok(completed)=>{activations.spawn(async move {
                                                let accepted=tokio::time::timeout(Duration::from_secs(40),completed).await;
                                                let result=if matches!(accepted,Ok(Ok(true))){Ok(br#"{"accepted":true}"#.to_vec())}else{Err(io::Error::other("The original notification navigation was not confirmed"))};
                                                let _=reply.send(result);
                                            });}
                                            Err(error)=>{let _=reply.send(Err(io::Error::other(error)));}
                                        }
                                        if failed && toast_starting {toast_starting=false;if continuation.end_unclaimed_bootstrap(){failure=Some(error("toastActivationRejected","The original notification is no longer qualified"));break;}}
                                        if !failed {toast_starting=false;}
                                    }
                                    Command::Business{..}=>{let _=reply.send(Err(io::Error::other("Business calls must use the fixed owner bridge")));}
                                }
                            }
                            notice=async{match desktop.as_mut(){Some(desktop)=>desktop.thread.notices.recv().await,None=>std::future::pending().await}},if notices_open=>{
                                let Some(notice)=notice else{
                                    notices_open=false;failure=Some(error("nativeFailed","The native notice channel closed unexpectedly"));
                                    if automatic_exit(&desktop,&mut exit_requested){break;}
                                    continue;
                                };
                                match notice {
                                    ui::Notice::CaptureOpen(request)=>{
                                        let opened=match open_capture_document(&state,&profile,&url,&mut desktop,&continuation,&request).await {
                                            Ok(opened)=>Some(opened),
                                            Err(error)=>{if error.code=="sessionRetirementUnknown"{failure.get_or_insert(error);}None}
                                        };
                                        let session=opened.as_ref().map(|opened|opened.session_id.clone());
                                        if desktop.as_ref().unwrap().thread.sender.send(ui::Command::CaptureResult{request,opened:opened.map(Box::new)}).is_err(){
                                            if let Some(session)=session {let _=retire(&state,Vec::new(),vec![session]).await;}
                                            failure.get_or_insert(error("captureDeliveryUnknown","The original Capture delivery could not be dispatched; the author record remains retained"));
                                        }
                                    }
                                    ui::Notice::Draft{surface_id,surface_epoch,request_id,request}=>{
                                        let authority=state.clone();let sender=desktop.as_ref().unwrap().thread.sender.clone();
                                        proofs.spawn(async move{
                                            let (body,session_ack)=draft_call(&authority,request).await;drop(authority);
                                            (sender,ui::Command::DraftResult{surface_id,surface_epoch,request_id,body,session_ack})
                                        });
                                    }
                                    ui::Notice::Verify{kind,request,serial,final_pass,proofs:prepared}=>{
                                        let authority=state.clone();let sender=desktop.as_ref().unwrap().thread.sender.clone();
                                        proofs.spawn(async move{let valid=verify(&authority,prepared).await;drop(authority);(sender,ui::Command::VerifyResult{kind,request,serial,final_pass,valid})});
                                    }
                                    ui::Notice::Closed{proofs:closed,sessions}=>{if !retire(&state,closed,sessions).await{failure.get_or_insert(error("sessionRetirementUnknown","The confirmed author input remains retained; original window-owned session retirement was not confirmed"));}}
                                    ui::Notice::Empty{admission_epoch}=>{
                                        if failure.is_some(){if automatic_exit(&desktop,&mut exit_requested){break;}continue;}
                                        let current=crate::core_call(&state,|core|core.desktop_preferences()).await;
                                        if let Some(desktop)=desktop.as_mut(){match current{
                                            Ok(current)=>{desktop.preferences=current;if continuation.exit_after_last_window(desktop.preferences.effective_preferences().resident,admission_epoch){break;}}
                                            Err(_)=>{let _=desktop.feedback(&error("preferenceConflict","The current resident preference could not be confirmed"));}
                                        }}
                                    }
                                    ui::Notice::ExitPermitted=>{continuation.close();if let Some(reply)=shutdown_reply.take(){let _=reply.send(Ok(br#"{"accepted":true}"#.to_vec()));}break;}
                                    ui::Notice::CloseBlocked=>{exit_requested=false;if let Some(reply)=shutdown_reply.take(){let _=reply.send(Err(closing()));}}
                                    ui::Notice::ScheduleChanged=>{if let Some(scheduler)=&scheduler{scheduler.refresh();}}
                                    ui::Notice::MenuRequested=>{
                                        if let Some(desktop)=desktop.as_mut(){
                                            match desktop.reload(&state,&profile).await.and_then(|()|desktop.refresh()){
                                                Ok(())=>{let _=desktop.thread.sender.send(ui::Command::Popup);}
                                                Err(error)=>{let _=desktop.feedback(&error);}
                                            }
                                        }
                                    }
                                    ui::Notice::ExternalUrl(uri)=>{let _=crate::host_environment::open_browser(&uri).await;}
                                    ui::Notice::LocaleChanged=>{
                                        let current=crate::core_call(&state,|core|core.client_preferences()).await;
                                        if let (Ok(current),Some(desktop))=(current,desktop.as_mut()){desktop.locale=current.preferences.ui_locale;let _=desktop.refresh();}
                                    }
                                    ui::Notice::Action(action)=>{
                                        if exit_requested{continue;}
                                        match action {
                                            tray::Action::Library=>{if let Err(error)=activate(&state,&profile,&url,&mut desktop,Activation::Library,&continuation).await&& let Some(desktop)=&desktop{let _=desktop.feedback(&error);}}
                                            tray::Action::Capture=>{if let Err(error)=activate(&state,&profile,&url,&mut desktop,Activation::Capture,&continuation).await&& let Some(desktop)=&desktop{let _=desktop.feedback(&error);}}
                                            tray::Action::Reminders=>{if activate(&state,&profile,&url,&mut desktop,Activation::Library,&continuation).await.is_ok(){let _=desktop.as_ref().unwrap().thread.sender.send(ui::Command::Post{kind:SurfaceKind::Library,message:serde_json::json!({"type":"open-reminders"}),activate:true});}}
                                            tray::Action::Exit=>{if !exit_requested {if desktop.as_ref().unwrap().thread.sender.send(ui::Command::BeginExit).is_err(){failure=Some(error("nativeFailed","The native close request was not delivered"));if desktop.as_ref().unwrap().thread.returned_without_exposed(){break;}}else{exit_requested=true;}}}
                                            action=>{if let Some(desktop)=desktop.as_mut()&& let Err(error)=update_preference(&state,&profile,desktop,action).await{let _=desktop.feedback(&error);}}
                                        }
                                    }
                                }
                            }
                            signal=tokio::signal::ctrl_c(),if console_signals=>{
                                if signal.is_err(){console_signals=false;failure=Some(error("shutdownSignalUnavailable","The console shutdown signal could not be monitored"));if automatic_exit(&desktop,&mut exit_requested){break;}continue;}
                                if let Some(native)=&desktop{if !exit_requested{if native.thread.sender.send(ui::Command::BeginExit).is_err(){failure=Some(error("nativeFailed","The native close request was not delivered"));if native.thread.returned_without_exposed(){break;}}else{exit_requested=true;}}}
                                else{break;}
                            }
                            else=>{std::future::pending::<()>().await;}
                        }
        }
    }
    // Every exit, including failed startup/fatal UI, uses this same drain.
    continuation.close();
    if let Some(reply) = shutdown_reply.take() {
        let _ = reply.send(Err(closing()));
    }
    requested.close();
    while let Ok(request) = requested.try_recv() {
        let _ = request.reply.send(Err(closing()));
    }
    drop(requested);
    let _ = stop_broker.send(true);
    let _ = stop_http.send(true);
    if let Some(scheduler) = scheduler
        && scheduler.stop().await.is_err()
    {
        failure.get_or_insert(error(
            "notificationDrainUnknown",
            "The original notification tail did not confirm its result",
        ));
    }
    while let Some(result) = proofs.join_next().await {
        if result.is_err() {
            failure.get_or_insert(error(
                "proofUnknown",
                "An accepted Core close verification did not retire",
            ));
        }
    }
    while let Some(result) = activations.join_next().await {
        if result.is_err() {
            failure.get_or_insert(error(
                "toastNavigationUnknown",
                "The original notification navigation tail did not retire",
            ));
        }
    }
    let broker_result = match broker_result {
        Some(result) => result,
        None => broker.await,
    };
    let http_result = match http_result {
        Some(result) => result,
        None => serving.await,
    };
    let http_failure = match http_result {
        Ok(stopped) => {
            let same_core = Arc::ptr_eq(&stopped.core, &state.core);
            let drained = stopped.outcome.is_ok();
            drop(stopped.core);
            if !same_core {
                Some(error(
                    "shutdownUnknown",
                    "The HTTP tail returned a different Core authority",
                ))
            } else if !drained {
                Some(error(
                    "shutdownUnknown",
                    "The HTTP listener or its accepted work did not drain successfully",
                ))
            } else {
                None
            }
        }
        Err(_) => Some(error(
            "shutdownUnknown",
            "The HTTP supervisor task did not join successfully",
        )),
    };
    // One final owner releases the same Core after HTTP, broker, author proofs,
    // notifications and worker tails. Normal and fatal HTTP share this path.
    let drained = crate::work::registry().drain().await.is_ok();
    let release_failure = match Arc::try_unwrap(state.core) {
        Err(_) => Some(error(
            "shutdownUnknown",
            "Core authority is still retained after HTTP and accepted work drained",
        )),
        Ok(core) => match core.into_inner() {
            Err(_) => Some(error(
                "shutdownUnknown",
                "Core authority was poisoned during final release",
            )),
            Ok(core) => {
                drop(core);
                None
            }
        },
    };
    let released = release_failure.is_none();
    let core_quiet = drained && released;
    if !matches!(broker_result, Ok(Ok(()))) {
        failure.get_or_insert(error(
            "shutdownUnknown",
            "The protected owner listener or its accepted requests did not drain successfully",
        ));
    } else if let Some(http_failure) = http_failure {
        failure.get_or_insert(http_failure);
    } else if !drained {
        failure.get_or_insert(error(
            "shutdownUnknown",
            "Accepted worker quiescence was not confirmed",
        ));
    } else if let Some(release_failure) = release_failure {
        failure.get_or_insert(release_failure);
    }
    if let Some(desktop) = desktop {
        // Final Core release uses an independent retained terminal slot, never
        // a bounded ordinary command queue. Join still owns the actual STA.
        let result = if core_quiet {
            terminal::Outcome::Quiescent
        } else {
            terminal::Outcome::ReleaseFailed
        };
        if desktop.thread.sender.finish_core(result).is_err() {
            failure.get_or_insert(error(
                "nativeShutdownUnknown",
                "The native apartment did not accept the final Core release result",
            ));
        }
        if desktop.thread.join().await.is_err() {
            failure.get_or_insert(error(
                "nativeShutdownUnknown",
                "The native apartment did not confirm clean callback and window retirement",
            ));
        }
    }
    failure.map_or(Ok(()), Err)
}
/// A live STA, even with no window yet shown, may still consume Ready. Only
/// its serial BeginExit cut (or exact return with no exposed surface) permits
/// automatic teardown. Failed delivery remains blocked/unknown.
fn automatic_exit(desktop: &Option<Desktop>, exit_requested: &mut bool) -> bool {
    let Some(desktop) = desktop else {
        return true;
    };
    if desktop.thread.returned_without_exposed() {
        return true;
    }
    if !*exit_requested && desktop.thread.sender.send(ui::Command::BeginExit).is_ok() {
        *exit_requested = true;
    }
    false
}

async fn activate(
    state: &crate::AppState,
    profile: &weftext_core::QualifiedProfile,
    url: &str,
    desktop: &mut Option<Desktop>,
    activation: Activation,
    continuation: &Arc<crate::broker::lifetime::ServiceContinuation>,
) -> Result<(), ServeError> {
    let admission_epoch = continuation.admit_ui().ok_or_else(|| error(
            "ownerClosing",
            "The owner is closing or its UI admission epoch is unavailable; no new author surface was admitted",
        ))?;
    if desktop.is_none() {
        match Desktop::start(state, profile, continuation).await {
            Ok(created) => *desktop = Some(created),
            Err(error) => {
                continuation.failed_ui_before_creation(admission_epoch);
                return Err(error);
            }
        }
    } else {
        desktop.as_mut().unwrap().reload(state, profile).await?;
    }
    let desktop = desktop.as_ref().unwrap();
    desktop.refresh()?;
    match activation {
        Activation::Library => desktop.open(SurfaceKind::Library, url, admission_epoch),
        Activation::Capture => desktop.open(SurfaceKind::Capture, url, admission_epoch),
        Activation::LoginStartup => {
            if desktop
                .preferences
                .effective_preferences()
                .open_library_on_launch
            {
                desktop.open(SurfaceKind::Library, url, admission_epoch)
            } else {
                Ok(())
            }
        }
        Activation::Headless => Ok(()),
    }
}
async fn activate_toast(
    state: &crate::AppState,
    profile: &weftext_core::QualifiedProfile,
    url: &str,
    desktop: &mut Option<Desktop>,
    opaque: String,
    continuation: &Arc<crate::broker::lifetime::ServiceContinuation>,
) -> Result<oneshot::Receiver<bool>, ServeError> {
    let lookup = opaque.clone();
    let route = crate::work::spawn_blocking(move || {
        crate::host_state::Root::qualify(false)?.lookup(&lookup)
    })
    .await
    .map_err(|_| {
        error(
            "toastRouteUnknown",
            "The original notification route did not complete",
        )
    })?
    .map_err(|_| {
        error(
            "toastRouteUnavailable",
            "The original notification route is unavailable",
        )
    })?;
    if route.profile.physical_identity() != profile.physical_identity() {
        return Err(error(
            "toastProfileChanged",
            "The original notification belongs to another physical profile",
        ));
    }
    let device_id = route.device_id;
    let execution_key = route.execution_key;
    let app_id = route.app_id;
    let activation_key = opaque.clone();
    crate::core_call(state, move |core| {
        let device = core.existing_device_identity()?;
        if !device.is_current || device.id != device_id {
            return Err(weftext_core::CoreError::new(
                "reminderDeviceUnavailable",
                weftext_messages::UiMessage::error_api_mod_the_original_notification_device_is_not_current(),
            ));
        }
        let original = core.original_reminder_notification(&execution_key)?;
        if original.execution_key != execution_key
            || original.request.activation_key != opaque
            || original.request.app_id != app_id
        {
            return Err(weftext_core::CoreError::new(
                "reminderUnavailable",
                weftext_messages::UiMessage::error_api_mod_the_original_notification_address_differs(),
            ));
        }
        core.resolve_reminder_activation(&opaque)
    })
    .await
    .map_err(|_| {
        error(
            "toastQualification",
            "The original notification or current task permission is unavailable",
        )
    })?;
    // Only this explicit user activation exposes/focuses a product surface.
    activate(
        state,
        profile,
        url,
        desktop,
        Activation::Library,
        continuation,
    )
    .await?;
    let (completed, completion) = oneshot::channel();
    desktop
        .as_ref()
        .unwrap()
        .thread
        .sender
        .send(ui::Command::ConfirmedPost {
            message: serde_json::json!({"type":"reminder-activation","activationKey":activation_key}),
            completed,
        })
        .map_err(|_| {
            error(
                "toastNavigationUnknown",
                "The qualified reminder navigation was not confirmed",
            )
        })?;
    Ok(completion)
}
async fn activate_login(
    state: &crate::AppState,
    profile: &weftext_core::QualifiedProfile,
    url: &str,
    desktop: &mut Option<Desktop>,
    continuation: &Arc<crate::broker::lifetime::ServiceContinuation>,
) -> Result<(), ServeError> {
    let original =
        crate::work::spawn_blocking(|| crate::host_state::Root::qualify(false)?.startup_profile())
            .await
            .map_err(|_| {
                error(
                    "startupProfileUnknown",
                    "The original startup profile was not confirmed",
                )
            })?
            .map_err(|_| {
                error(
                    "startupProfileUnavailable",
                    "The original startup profile is unavailable",
                )
            })?;
    let Some(original) = original else {
        return Ok(());
    };
    if original.profile.physical_identity() != profile.physical_identity() {
        return Err(error(
            "startupProfileChanged",
            "The application startup task now selects another physical profile",
        ));
    }
    let device_id = original.device_id;
    let enabled = crate::core_call(state, move |core| {
        let device = core.existing_device_identity()?;
        if !device.is_current || device.id != device_id {
            return Err(weftext_core::CoreError::new(
                "deviceNotCurrent",
                weftext_messages::UiMessage::error_api_mod_the_original_startup_device_is_not_current(),
            ));
        }
        Ok(core
            .desktop_preferences()?
            .effective_preferences()
            .login_startup_requested)
    })
    .await
    .map_err(|_| {
        error(
            "startupQualification",
            "The original startup intent is unavailable",
        )
    })?;
    if enabled {
        activate(
            state,
            profile,
            url,
            desktop,
            Activation::LoginStartup,
            continuation,
        )
        .await
    } else {
        Ok(())
    }
}
async fn open_document(
    state: &crate::AppState,
    desktop: &Desktop,
    document: crate::local_service::InitialDocument,
) -> Result<(), ServeError> {
    let opened = crate::core_call(state, move |core| core.open(document.path, document.root))
        .await
        .map_err(|_| {
            error(
                "documentUnavailable",
                "The explicit document could not be qualified",
            )
        })?;
    let value = serde_json::to_value(opened).map_err(|_| {
        error(
            "documentDelivery",
            "The explicit document delivery could not be encoded",
        )
    })?;
    crate::remember(state, &value).map_err(|_| {
        error(
            "documentDelivery",
            "The explicit document could not be registered",
        )
    })?;
    desktop
        .thread
        .sender
        .send(ui::Command::Post {
            kind: SurfaceKind::Library,
            message: serde_json::json!({"type":"open-document","document":value}),
            activate: true,
        })
        .map_err(|_| {
            error(
                "documentDelivery",
                "The explicit document delivery was not confirmed",
            )
        })
}
async fn open_capture_document(
    state: &crate::AppState,
    profile: &weftext_core::QualifiedProfile,
    url: &str,
    desktop: &mut Option<Desktop>,
    continuation: &Arc<crate::broker::lifetime::ServiceContinuation>,
    request: &close::CaptureOpen,
) -> Result<weftext_core::OpenedDocument, ServeError> {
    let draft = request.draft_id.clone();
    let operation = request.operation_id.clone();
    // Core qualifies the original committed operation/member and creates one
    // fresh Session. No path, UI receipt or cached Session substitutes for it.
    let opened = crate::core_call(state, move |core| {
        core.open_capture_result(&draft, &operation)
    })
    .await
    .map_err(|_| {
        error(
            "captureResultUnavailable",
            "The original Capture result is unavailable",
        )
    })?;
    let prepared = match serde_json::to_value(&opened) {
        Ok(value) => crate::remember(state, &value).map_err(|_| {
            error(
                "captureResultUnavailable",
                "The original Capture Session could not be registered",
            )
        }),
        Err(_) => Err(error(
            "captureResultUnavailable",
            "The original Capture result could not be encoded",
        )),
    };
    let prepared = match prepared {
        Ok(()) => {
            activate(
                state,
                profile,
                url,
                desktop,
                Activation::Library,
                continuation,
            )
            .await
        }
        Err(error) => Err(error),
    };
    if let Err(failure) = prepared {
        if !retire(state, Vec::new(), vec![opened.session_id]).await {
            return Err(error(
                "sessionRetirementUnknown",
                "The undelivered original Capture Session retirement was not confirmed; its author record remains retained",
            ));
        }
        return Err(failure);
    }
    Ok(opened)
}
async fn verify(state: &crate::AppState, proofs: Vec<Proof>) -> bool {
    crate::core_call(state, move |core| {
        for proof in proofs {
            match proof {
                Proof::Document {
                    session_id,
                    base_id,
                    source_digest, input_id, evidence, record_digest,
                    ..
                } => {
                    let receipt = core.flush_document_draft(weftext_core::DocumentDraftFlush {
                        session_id,
                        base_id,
                        source_digest, input_id: Some(input_id),
                    })?;
                    if receipt.evidence != evidence || (evidence == weftext_core::DocumentDraftEvidence::SavedView && receipt.record_digest != record_digest) { return Err(weftext_core::CoreError::new("draftFlushMismatch", weftext_messages::UiMessage::error_core_draft_recovery_the_persisted_draft_does_not_contain_the_prepared_input())); }
                }
                Proof::Capture {
                    draft_id,
                    record_revision,
                    input_revision,
                    last_mutation_id,
                    record_digest,
                    pending_operation_id,
                    ..
                } => {
                    let receipt = core.capture_flush_receipt(
                        &draft_id,
                        record_revision,
                        input_revision,
                        &last_mutation_id,
                    )?;
                    if receipt.record_digest != record_digest
                        || receipt.pending_operation_id != pending_operation_id
                    {
                        return Err(weftext_core::CoreError::new(
                            "captureFlushMismatch",
                            weftext_messages::UiMessage::error_api_mod_the_exact_prepared_capture_record_differs(),
                        ));
                    }
                }
            }
        }
        Ok(())
    })
    .await
    .is_ok()
}
async fn draft_call(
    state: &crate::AppState,
    request: close::DraftRequest,
) -> (serde_json::Value, Option<String>) {
    let session = request.session_id().to_owned();
    let acknowledges = request.acknowledges_input();
    let result = crate::core_call(state, move |core| match request {
        close::DraftRequest::Persist { body } => {
            let revision = body.input_revision;
            let evidence = core.persist_draft_ordered(body)?;
            Ok(serde_json::json!({"ok":true,"inputRevision":revision,"evidence":evidence}))
        }
        close::DraftRequest::Resume { body } => {
            core.resume_recovered_draft(body)?;
            Ok(serde_json::json!({"ok":true}))
        }
        close::DraftRequest::Flush { body } => {
            Ok(serde_json::to_value(core.flush_document_draft(body)?)?)
        }
    })
    .await;
    match result {
        Ok(body) => (body, acknowledges.then_some(session)),
        Err(error) => (serde_json::json!({"error":error}), None),
    }
}
async fn retire(state: &crate::AppState, proofs: Vec<Proof>, sessions: Vec<String>) -> bool {
    let result = crate::core_call(state, move |core| {
        for proof in proofs {
            if let Proof::Document {
                session_id,
                base_id,
                source_digest, input_id, evidence, record_digest,
                ..
            } = proof
            {
                let receipt = core.flush_document_draft(weftext_core::DocumentDraftFlush {
                    session_id: session_id.clone(),
                    base_id,
                    source_digest, input_id: Some(input_id),
                })?;
                if receipt.evidence != evidence || (evidence == weftext_core::DocumentDraftEvidence::SavedView && receipt.record_digest != record_digest) { return Err(weftext_core::CoreError::new("draftFlushMismatch", weftext_messages::UiMessage::error_core_draft_recovery_the_persisted_draft_does_not_contain_the_prepared_input())); }
            }
        }
        // This set was built only by real successful native producer calls on
        // this surface. It includes earlier sessions explicitly superseded by
        // its own document rebinds, never unrelated cross-command CLI sessions.
        let mut retired = Vec::new();
        for session in sessions {
            match core.close_session(&session) {
                Ok(()) => {}
                Err(error) if error.code == "sessionMissing" => {}
                Err(_) => return Ok((retired, false)),
            }
            retired.push(session);
        }
        Ok((retired, true))
    })
    .await;
    let Ok((retired, mut complete)) = result else {
        return false;
    };
    // These are only disposable projections of the sessions actually retired
    // above. No CLI session, author Draft or other surface is pruned by age.
    for id in &retired {
        if crate::forget_session_projection(state, id).is_err() {
            complete = false;
        }
    }
    complete
}
async fn update_preference(
    state: &crate::AppState,
    profile: &weftext_core::QualifiedProfile,
    desktop: &mut Desktop,
    action: tray::Action,
) -> Result<(), ServeError> {
    desktop.reload(state, profile).await?;
    let mut preferences = desktop.preferences.effective_preferences();
    match action {
        tray::Action::Resident => preferences.resident = !preferences.resident,
        tray::Action::OpenOnLaunch => {
            preferences.open_library_on_launch = !preferences.open_library_on_launch
        }
        tray::Action::LoginStartup => {
            if !desktop.startup.can_request() {
                desktop.refresh()?;
                return Err(error(
                    "startupUnavailable",
                    "The OS startup capability is unavailable or controlled outside the app",
                ));
            }
            preferences.login_startup_requested =
                !(desktop.startup.enabled() && preferences.login_startup_requested)
        }
        _ => return Ok(()),
    }
    let previous = desktop.preferences.clone();
    let (current, information) = crate::core_call(state, move |core| {
        persist_preference_change(core, previous, preferences)
    })
    .await
    .map_err(|_| {
        error(
            "preferenceConflict",
            "The desktop preference changed or could not be durably stored",
        )
    })?;
    desktop.preferences = current;
    if matches!(action, tray::Action::LoginStartup) {
        desktop.startup = match startup::request(
            profile,
            &desktop.preferences.device,
            preferences.login_startup_requested,
        )
        .await
        {
            Ok(observation) => observation,
            Err(_) => {
                desktop.startup = startup::Observation {
                    state: startup::State::Unknown,
                    selected: desktop.startup.selected,
                };
                desktop.refresh()?;
                return Err(error(
                    "startupRequestUnknown",
                    "The one startup request was not confirmed; the persisted intent is retained",
                ));
            }
        };
    }
    desktop.refresh_with_information(information)
}

/// A notice accompanies only this explicit successful off-to-on change. Reading
/// settings, reopening a window or changing another intent cannot acknowledge it.
/// No durable "seen" flag is invented from merely queueing a native dialog.
fn persist_preference_change(
    core: &mut weftext_core::Core,
    previous: weftext_core::DesktopPreferencesSnapshot,
    preferences: weftext_core::DesktopPreferences,
) -> weftext_core::Result<(weftext_core::DesktopPreferencesSnapshot, Option<UiMessage>)> {
    let was_resident = previous.effective_preferences().resident;
    let current = core.update_desktop_preferences(previous.revision, preferences)?;
    let information = (!was_resident && current.effective_preferences().resident)
        .then(UiMessage::desktop_resident_enabled);
    Ok((current, information))
}

#[cfg(test)]
mod preference_tests {
    use super::*;
    use weftext_core::{Core, DesktopPreferences};

    #[test]
    fn first_resident_enable_after_other_preferences_is_explained_and_durable() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("core");
        let mut core = Core::new(&data).unwrap();
        let initial = core.desktop_preferences().unwrap();
        let other_settings = DesktopPreferences {
            open_library_on_launch: true,
            ..Default::default()
        };
        let (before, information) =
            persist_preference_change(&mut core, initial, other_settings).unwrap();
        assert!(information.is_none());
        let enabled = DesktopPreferences {
            resident: true,
            ..other_settings
        };
        let (current, information) = persist_preference_change(&mut core, before, enabled).unwrap();
        assert_eq!(information.unwrap().key(), "desktop.residentEnabled");
        assert_eq!(current.preferences, enabled);
        assert_eq!(current.revision, 2);
        drop(core);
        let mut reopened = Core::new(&data).unwrap();
        assert_eq!(reopened.desktop_preferences().unwrap(), current);
        // Reasserting existing residency after reopening is not another enable.
        let (_, information) = persist_preference_change(&mut reopened, current, enabled).unwrap();
        assert!(information.is_none());
    }

    #[test]
    fn disabling_and_other_intents_are_silent_but_explicit_reenable_is_explained() {
        let directory = tempfile::tempdir().unwrap();
        let mut core = Core::new(directory.path().join("core")).unwrap();
        let initial = core.desktop_preferences().unwrap();
        let enabled = DesktopPreferences {
            resident: true,
            ..Default::default()
        };
        let (current, _) = persist_preference_change(&mut core, initial, enabled).unwrap();
        let other_settings = DesktopPreferences {
            open_library_on_launch: true,
            login_startup_requested: true,
            ..enabled
        };
        let (current, information) =
            persist_preference_change(&mut core, current, other_settings).unwrap();
        assert!(information.is_none());
        let disabled = DesktopPreferences {
            resident: false,
            ..other_settings
        };
        let (current, information) =
            persist_preference_change(&mut core, current, disabled).unwrap();
        assert!(information.is_none());
        let (current, information) =
            persist_preference_change(&mut core, current, other_settings).unwrap();
        assert_eq!(information.unwrap().key(), "desktop.residentEnabled");
        assert_eq!(current.preferences, other_settings);
    }

    #[test]
    fn rejected_resident_enable_has_no_success_notice_and_retains_current_settings() {
        let directory = tempfile::tempdir().unwrap();
        let mut core = Core::new(directory.path().join("core")).unwrap();
        let stale = core.desktop_preferences().unwrap();
        let original = core
            .update_desktop_preferences(
                stale.revision,
                DesktopPreferences {
                    open_library_on_launch: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let failure = persist_preference_change(
            &mut core,
            stale,
            DesktopPreferences {
                resident: true,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(failure.code, "deviceStateConflict");
        assert_eq!(core.desktop_preferences().unwrap(), original);
    }
}
