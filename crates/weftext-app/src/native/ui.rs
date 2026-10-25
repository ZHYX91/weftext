//! One STA reducer owns HWNDs and every WebView COM pointer. Its independent
//! kernel wake also lets the Core supervisor quiesce after a dispatcher fault.
use super::{
    apartment::Sta,
    close::{CaptureDelivery, CaptureOpen, Gate, Message, Proof, Request, SurfaceKind},
    events::{Event, Queue, Wake},
    terminal::{Outcome, Terminal},
    tray::{Action, Tray},
    view::{Pending, View},
    window::{Window, WindowState},
};
use std::{
    cell::Cell,
    collections::BTreeMap,
    io,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::mpsc::{Receiver, SyncSender, TryRecvError},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use webview2_com::{
    CoreWebView2EnvironmentOptions, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, Microsoft::Web::WebView2::Win32::*,
};
use weftext_messages::{UiLocale, UiMessage};
use windows::{
    Win32::{
        Foundation::{E_FAIL, E_POINTER},
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
            WindowsAndMessaging::*,
        },
    },
    core::{Error, HSTRING, PCWSTR, Result},
};

pub(super) enum Command {
    Open {
        kind: SurfaceKind,
        address: String,
        admission_epoch: u64,
    },
    Post {
        kind: SurfaceKind,
        message: serde_json::Value,
        activate: bool,
    },
    ConfirmedPost {
        message: serde_json::Value,
        completed: tokio::sync::oneshot::Sender<bool>,
    },
    Preferences {
        locale: UiLocale,
        preferences: weftext_core::DesktopPreferences,
        startup: super::startup::Observation,
        cli_service: bool,
        // Present only after this user action's Core write and strict readback.
        information: Option<UiMessage>,
    },
    Popup,
    Feedback(UiMessage),
    VerifyResult {
        kind: SurfaceKind,
        request: Request,
        serial: u64,
        final_pass: bool,
        valid: bool,
    },
    DraftResult {
        surface_id: String,
        surface_epoch: String,
        request_id: String,
        body: serde_json::Value,
        session_ack: Option<String>,
    },
    CaptureResult {
        request: CaptureOpen,
        opened: Option<Box<weftext_core::OpenedDocument>>,
    },
    BeginExit,
    CancelClose,
}
pub(super) enum Notice {
    CaptureOpen(CaptureOpen),
    Draft {
        surface_id: String,
        surface_epoch: String,
        request_id: String,
        request: super::close::DraftRequest,
    },
    Verify {
        kind: SurfaceKind,
        request: Request,
        serial: u64,
        final_pass: bool,
        proofs: Vec<Proof>,
    },
    Closed {
        proofs: Vec<Proof>,
        sessions: Vec<String>,
    },
    Empty {
        admission_epoch: u64,
    },
    ExitPermitted,
    CloseBlocked,
    Action(Action),
    MenuRequested,
    ExternalUrl(String),
    LocaleChanged,
    ScheduleChanged,
}
#[derive(Clone)]
pub(super) struct Sender {
    send: SyncSender<Command>,
    wake: Wake,
    terminal: Arc<Terminal>,
}
impl Sender {
    pub fn send(&self, command: Command) -> io::Result<()> {
        if !self.terminal.accepts_commands() {
            return Err(io::Error::other(
                "The native STA is retiring or has returned",
            ));
        }
        self.send.try_send(command).map_err(|_| {
            self.wake.fail();
            io::Error::other("The native dispatcher is unavailable or over budget")
        })?;
        self.wake.notify();
        Ok(())
    }
    /// Only the supervisor's final Core lease handoff may publish this result.
    /// Publication is retained even when a normal queue or OS signalling fails.
    pub fn finish_core(&self, outcome: Outcome) -> io::Result<()> {
        let result = self.terminal.publish(outcome);
        self.wake.notify();
        result
    }
}
pub(super) struct Thread {
    pub sender: Sender,
    pub notices: mpsc::Receiver<Notice>,
    pub fault: tokio::sync::watch::Receiver<bool>,
    exposed: Arc<AtomicUsize>,
    join: std::thread::JoinHandle<io::Result<()>>,
    terminal: Arc<Terminal>,
}
impl Thread {
    pub fn start(
        user_data: PathBuf,
        owner_lifetime: Arc<crate::broker::lifetime::ServiceContinuation>,
    ) -> io::Result<Self> {
        let (wake, fault) = Wake::new().map_err(io::Error::other)?;
        let (send, commands) = std::sync::mpsc::sync_channel(64);
        let (notices, received) = mpsc::channel(64);
        let thread_wake = wake.clone();
        let exposed = Arc::new(AtomicUsize::new(0));
        let live = exposed.clone();
        let terminal = Arc::new(Terminal::default());
        let thread_terminal = terminal.clone();
        let join = std::thread::Builder::new()
            .name("weftext-webview-sta".into())
            .spawn(move || {
                struct Returned(Arc<Terminal>);
                impl Drop for Returned {
                    fn drop(&mut self) {
                        self.0.stopped();
                    }
                }
                let _returned = Returned(thread_terminal.clone());
                let result = run(
                    user_data,
                    commands,
                    notices,
                    thread_wake.clone(),
                    live,
                    thread_terminal,
                    owner_lifetime,
                );
                if result.is_err() {
                    thread_wake.fail();
                }
                result
            })?;
        Ok(Self {
            sender: Sender {
                send,
                wake,
                terminal: terminal.clone(),
            },
            notices: received,
            fault,
            exposed,
            join,
            terminal,
        })
    }
    pub async fn join(self) -> io::Result<()> {
        // This exact thread handle remains owned until its actual return.
        tokio::task::spawn_blocking(move || {
            let result = self
                .join
                .join()
                .map_err(|_| io::Error::other("The native STA panicked"))?;
            self.terminal.confirm_joined()?;
            result
        })
        .await
        .map_err(io::Error::other)?
    }
    /// Only the exact owned thread's actual return removes the possibility of
    /// a later Ready/show. A live thread plus a sampled zero count is insufficient.
    pub fn returned_without_exposed(&self) -> bool {
        self.join.is_finished() && self.exposed.load(Ordering::Acquire) == 0
    }
}

struct Surface {
    view: Option<View>,
    window: Window,
    gate: Gate,
    address: String,
    requested: bool,
    after_ready: Vec<PendingPost>,
    sessions: std::collections::BTreeSet<String>,
    draft_calls: std::collections::BTreeSet<String>,
    capture_requests: std::collections::BTreeSet<String>,
    confirmed_posts: BTreeMap<String, ConfirmedAction>,
    exposed: bool,
}
struct ConfirmedAction {
    completed: tokio::sync::oneshot::Sender<bool>,
    posted: bool,
}
impl ConfirmedAction {
    fn admit(completed: tokio::sync::oneshot::Sender<bool>) -> Option<Self> {
        let action = Self {
            completed,
            posted: false,
        };
        action.can_post().then_some(action)
    }
    fn can_post(&self) -> bool {
        !self.posted && !self.completed.is_closed()
    }
}
fn complete_confirmed_action(
    gate: &Gate,
    actions: &mut BTreeMap<String, ConfirmedAction>,
    surface_id: &str,
    surface_epoch: &str,
    action_id: &str,
    accepted: bool,
) -> bool {
    if !gate.owns_surface(SurfaceKind::Library, surface_id, surface_epoch)
        || !actions.get(action_id).is_some_and(|action| action.posted)
    {
        return false;
    }
    if let Some(action) = actions.remove(action_id) {
        let _ = action.completed.send(accepted);
        return true;
    }
    false
}
struct PendingPost {
    message: serde_json::Value,
    activate: bool,
    delivery_id: Option<String>,
}
struct Reducer {
    dispatcher: Window,
    surfaces: BTreeMap<u64, Surface>,
    environment: Option<ICoreWebView2Environment>,
    queue: Queue,
    wake: Wake,
    pending: Rc<Cell<usize>>,
    notices: mpsc::Sender<Notice>,
    tray: Tray,
    locale: UiLocale,
    preferences: weftext_core::DesktopPreferences,
    startup: super::startup::Observation,
    cli_service: bool,
    global_exit: bool,
    core_retirement: Option<Instant>,
    fatal: bool,
    exposed: Arc<AtomicUsize>,
    owner_lifetime: Arc<crate::broker::lifetime::ServiceContinuation>,
    admission_epoch: u64,
    visibility: crate::broker::lifetime::VisibilityFence,
    capture_delivery: Option<CaptureDelivery>,
}
fn id(kind: SurfaceKind) -> u64 {
    match kind {
        SurfaceKind::Library => 1,
        SurfaceKind::Capture => 2,
    }
}
fn failed() -> Error {
    Error::from(E_FAIL)
}
impl Reducer {
    fn notice(&self, notice: Notice) -> Result<()> {
        self.notices.try_send(notice).map_err(|_| {
            self.wake.fail();
            failed()
        })
    }
    fn fail(&mut self) {
        if !self.fatal {
            self.fatal = true;
            let _ = self.cancel_all();
            let _ = self.notice(Notice::CloseBlocked);
        }
        self.wake.fail();
    }
    fn environment(&self, user_data: PathBuf) -> Result<()> {
        let folder = HSTRING::from(user_data.as_os_str());
        // Published defaults retain tracking prevention and disable SSO/browser
        // extensions; no additional browser arguments or security relaxations.
        let options: ICoreWebView2EnvironmentOptions =
            CoreWebView2EnvironmentOptions::default().into();
        let pending = Pending::new(&self.pending, &self.queue)?;
        let queue = self.queue.clone();
        let callback = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |status, environment| {
                let _pending = pending;
                queue.push(Event::Environment(
                    status.and_then(|()| environment.ok_or_else(|| Error::from(E_POINTER))),
                ));
                Ok(())
            },
        ));
        unsafe {
            CreateCoreWebView2EnvironmentWithOptions(PCWSTR::null(), &folder, &options, &callback)
        }
    }
    fn begin_controller(&mut self, key: u64) -> Result<()> {
        let Some(environment) = self.environment.as_ref() else {
            return Ok(());
        };
        let Some(surface) = self.surfaces.get_mut(&key) else {
            return Ok(());
        };
        if surface.requested || self.core_retirement.is_some() {
            return Ok(());
        }
        let Some(hwnd) = surface.window.state.live_hwnd() else {
            return Err(failed());
        };
        surface.requested = true;
        let weak = Rc::downgrade(&surface.window.state);
        let queue = self.queue.clone();
        let pending = Pending::new(&self.pending, &queue)?;
        let callback = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |status, controller| {
                let _pending = pending;
                queue.push(Event::Controller(
                    weak,
                    status.and_then(|()| controller.ok_or_else(|| Error::from(E_POINTER))),
                ));
                Ok(())
            },
        ));
        unsafe { environment.CreateCoreWebView2Controller(hwnd, &callback) }
    }
    fn live_key(&self, weak: &Weak<WindowState>) -> Option<u64> {
        let state = weak.upgrade()?;
        self.surfaces
            .get(&state.id)
            .filter(|surface| {
                Rc::ptr_eq(&surface.window.state, &state) && state.live_hwnd().is_some()
            })
            .map(|_| state.id)
    }
    fn begin_close(&mut self, key: u64) -> Result<()> {
        let Some(surface) = self.surfaces.get_mut(&key) else {
            return Ok(());
        };
        if self
            .capture_delivery
            .as_ref()
            .is_some_and(|delivery| delivery.blocks(&surface.gate.id, &surface.gate.epoch))
        {
            self.blocked()?;
            return Ok(());
        }
        let Some(request) = surface.gate.begin() else {
            if !surface.gate.ready {
                self.blocked()?;
            }
            return Ok(());
        };
        surface
            .view
            .as_ref()
            .ok_or_else(failed)?
            .post(&message("prepare-close", &request))
    }
    fn cancel_all(&mut self) -> Result<()> {
        // A published final author cut is irreversible: a later COM/fault
        // callback must not thaw the browser while the supervisor drains Core.
        if !self.visibility.accepts_exposure() {
            return Ok(());
        }
        self.global_exit = false;
        for surface in self.surfaces.values_mut() {
            let request = surface.gate.current_request();
            surface.gate.cancel();
            if let Some(request) = request
                && let Some(view) = &surface.view
            {
                view.post(&message("close-cancelled", &request))?;
            }
        }
        Ok(())
    }
    fn blocked(&mut self) -> Result<()> {
        self.cancel_all()?;
        self.notice(Notice::CloseBlocked)?;
        let text = HSTRING::from(UiMessage::desktop_unsaved_close().render(self.locale));
        // This belongs to an explicit own-window close action. No editor input
        // or focus is synthesized and the original surface remains intact.
        unsafe {
            MessageBoxW(
                self.dispatcher.state.live_hwnd(),
                &text,
                windows::core::w!("Weftext"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        Ok(())
    }
    fn capture_result(&self, request: &CaptureOpen, accepted: bool) -> Result<()> {
        if let Some(surface) = self.surfaces.get(&id(SurfaceKind::Capture))
            && surface.gate.id == request.surface_id
            && surface.gate.epoch == request.surface_epoch
        {
            surface
                .view
                .as_ref()
                .ok_or_else(failed)?
                .post(&serde_json::json!({
                    "type":"capture-open-result", "surfaceId":request.surface_id,
                    "surfaceEpoch":request.surface_epoch, "requestId":request.request_id,
                    "accepted":accepted
                }))?;
        }
        Ok(())
    }
    fn retire_undelivered(&self, opened: weftext_core::OpenedDocument) -> Result<()> {
        self.notice(Notice::Closed {
            proofs: Vec::new(),
            sessions: vec![opened.session_id],
        })
    }
    fn reject_capture_delivery(&mut self) -> Result<()> {
        let Some(delivery) = self.capture_delivery.take() else {
            return Ok(());
        };
        for surface in self.surfaces.values_mut() {
            surface.after_ready.retain(|pending| {
                pending.delivery_id.as_deref() != Some(delivery.request.request_id.as_str())
            });
            if delivery.targets(&surface.gate.id, &surface.gate.epoch)
                && let Some(session) = delivery.undelivered_session()
            {
                surface.sessions.remove(session);
            }
        }
        // A queued value has never reached any browser consumer. A posted
        // value can already have a live author even if its reply was lost;
        // retain that Session with its Library until the original close gate.
        if let Some(session) = delivery.undelivered_session() {
            self.notice(Notice::Closed {
                proofs: Vec::new(),
                sessions: vec![session.to_owned()],
            })?;
        }
        self.capture_result(&delivery.request, false)
    }
    fn capture_opened(
        &mut self,
        request: CaptureOpen,
        opened: Option<weftext_core::OpenedDocument>,
    ) -> Result<()> {
        if !self
            .capture_delivery
            .as_ref()
            .is_some_and(|delivery| delivery.owns(&request))
        {
            if let Some(opened) = opened {
                self.retire_undelivered(opened)?;
            }
            return Ok(());
        }
        let source_live = self
            .surfaces
            .get(&id(SurfaceKind::Capture))
            .is_some_and(|surface| {
                surface.gate.id == request.surface_id
                    && surface.gate.epoch == request.surface_epoch
                    && surface.gate.ready
                    && surface.window.state.live_hwnd().is_some()
                    && !surface.window.state.destroying.get()
            });
        if !source_live || self.fatal || self.global_exit {
            if let Some(opened) = opened {
                self.retire_undelivered(opened)?;
            }
            self.capture_delivery = None;
            self.capture_result(&request, false)?;
            return Ok(());
        }
        let Some(opened) = opened else {
            self.capture_delivery = None;
            self.capture_result(&request, false)?;
            return Ok(());
        };
        let Some(surface) = self.surfaces.get_mut(&id(SurfaceKind::Library)) else {
            self.capture_delivery = None;
            self.retire_undelivered(opened)?;
            self.capture_result(&request, false)?;
            return Ok(());
        };
        if surface.sessions.len() >= 32768
            || surface.after_ready.len() >= 64
            || surface.window.state.live_hwnd().is_none()
            || surface.window.state.destroying.get()
        {
            self.capture_delivery = None;
            self.retire_undelivered(opened)?;
            self.capture_result(&request, false)?;
            return Ok(());
        }
        let message = surface.gate.bind_message(serde_json::json!({"type":"open-document", "deliveryId":request.request_id, "document":opened})).ok_or_else(failed)?;
        let delivery = self.capture_delivery.as_mut().ok_or_else(failed)?;
        if !delivery.bind(&surface.gate.id, &surface.gate.epoch, &opened.session_id) {
            return Err(failed());
        }
        // Before either posting or queuing, the target owns this exact Session.
        // Rejected and unknown deliveries stay in its retirement set, never in
        // a different/recreated Library or the original Capture author record.
        surface.sessions.insert(opened.session_id);
        if surface.gate.ready {
            if surface
                .view
                .as_ref()
                .ok_or_else(failed)?
                .post(&message)
                .is_err()
            {
                self.capture_delivery = None;
                self.capture_result(&request, false)?;
                return Ok(());
            }
            if !delivery.mark_posted(&request.request_id) {
                return Err(failed());
            }
            surface.window.show();
        } else {
            surface.after_ready.push(PendingPost {
                message,
                activate: true,
                delivery_id: Some(request.request_id),
            });
        }
        Ok(())
    }
    fn command(&mut self, command: Command) -> Result<()> {
        if self.owner_lifetime.is_closing() || !self.visibility.accepts_exposure() {
            // An already accepted idle/global exit cannot expose a queued new
            // window while Core drains. Final COM release has its own slot.
            return Ok(());
        }
        match command {
            Command::Open {
                kind,
                address,
                admission_epoch,
            } => {
                if self.core_retirement.is_some() || self.global_exit || self.fatal {
                    return Err(failed());
                }
                if admission_epoch == 0 || admission_epoch < self.admission_epoch {
                    return Err(failed());
                }
                self.admission_epoch = admission_epoch;
                let key = id(kind);
                if let Some(surface) = self.surfaces.get_mut(&key) {
                    let request = surface.gate.current_request();
                    if surface.gate.reactivate()
                        && let Some(request) = request
                        && let Some(view) = &surface.view
                    {
                        view.post(&message("close-cancelled", &request))?;
                    }
                    // Repeated launch must not expose an unready controller
                    // before its real producer barriers have registered.
                    if surface.exposed {
                        surface.window.show();
                    }
                    return Ok(());
                }
                let title = match kind {
                    SurfaceKind::Library => UiMessage::desktop_library(),
                    SurfaceKind::Capture => UiMessage::desktop_capture(),
                }
                .render(self.locale);
                let window = Window::create(key, &title, self.queue.clone())?;
                self.surfaces.insert(
                    key,
                    Surface {
                        window,
                        view: None,
                        gate: Gate::new(kind),
                        address,
                        requested: false,
                        after_ready: Vec::new(),
                        sessions: Default::default(),
                        draft_calls: Default::default(),
                        capture_requests: Default::default(),
                        confirmed_posts: Default::default(),
                        exposed: false,
                    },
                );
                self.begin_controller(key)?;
            }
            Command::Post {
                kind,
                message,
                activate,
            } => {
                let Some(surface) = self.surfaces.get_mut(&id(kind)) else {
                    return Err(failed());
                };
                let message = surface.gate.bind_message(message).ok_or_else(failed)?;
                if surface.gate.ready {
                    surface.view.as_ref().ok_or_else(failed)?.post(&message)?;
                    if activate {
                        surface.window.show();
                    }
                } else if surface.after_ready.len() < 64 {
                    surface.after_ready.push(PendingPost {
                        message,
                        activate,
                        delivery_id: None,
                    });
                } else {
                    return Err(failed());
                }
            }
            Command::ConfirmedPost {
                mut message,
                completed,
            } => {
                // This command may have waited in the main native queue past
                // its original caller's deadline, before any surface was bound.
                let Some(action) = ConfirmedAction::admit(completed) else {
                    return Ok(());
                };
                let Some(surface) = self.surfaces.get_mut(&id(SurfaceKind::Library)) else {
                    let _ = action.completed.send(false);
                    return Ok(());
                };
                surface
                    .confirmed_posts
                    .retain(|_, action| !action.completed.is_closed());
                if self.global_exit
                    || self.fatal
                    || surface.gate.closing()
                    || surface.confirmed_posts.len() >= 16
                    || surface.after_ready.len() >= 64
                {
                    let _ = action.completed.send(false);
                    return Ok(());
                }
                let action_id = uuid::Uuid::new_v4().to_string();
                message
                    .as_object_mut()
                    .ok_or_else(failed)?
                    .insert("actionId".into(), action_id.clone().into());
                let message = surface.gate.bind_message(message).ok_or_else(failed)?;
                surface.confirmed_posts.insert(action_id.clone(), action);
                if surface.gate.ready {
                    surface.view.as_ref().ok_or_else(failed)?.post(&message)?;
                    surface
                        .confirmed_posts
                        .get_mut(&action_id)
                        .ok_or_else(failed)?
                        .posted = true;
                    surface.window.show();
                } else {
                    surface.after_ready.push(PendingPost {
                        message,
                        activate: true,
                        delivery_id: None,
                    });
                }
            }
            Command::Preferences {
                locale,
                preferences,
                startup,
                cli_service,
                information,
            } => {
                self.locale = locale;
                self.preferences = preferences;
                self.startup = startup;
                self.cli_service = cli_service;
                for surface in self.surfaces.values() {
                    let title = match surface.gate.kind {
                        SurfaceKind::Library => UiMessage::desktop_library(),
                        SurfaceKind::Capture => UiMessage::desktop_capture(),
                    }
                    .render(locale);
                    surface.window.set_title(&title)?;
                }
                if let Some(message) = information {
                    // This is a consequence of the user's explicit tray action;
                    // no startup, background read or settings refresh supplies it.
                    unsafe {
                        MessageBoxW(
                            self.dispatcher.state.live_hwnd(),
                            &HSTRING::from(message.render(self.locale)),
                            windows::core::w!("Weftext"),
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                }
            }
            Command::Popup => {
                if let Some(action) = self.tray.popup(
                    self.locale,
                    self.preferences,
                    self.startup,
                    crate::ASSETS.get_file("capture.html").is_some(),
                    self.cli_service,
                )? {
                    self.notice(Notice::Action(action))?;
                }
            }
            Command::Feedback(message) => unsafe {
                MessageBoxW(
                    self.dispatcher.state.live_hwnd(),
                    &HSTRING::from(message.render(self.locale)),
                    windows::core::w!("Weftext"),
                    MB_OK | MB_ICONWARNING,
                );
            },
            Command::VerifyResult {
                kind,
                request,
                serial,
                final_pass,
                valid,
            } => {
                let Some(surface) = self.surfaces.get_mut(&id(kind)) else {
                    return Ok(());
                };
                if !surface.gate.owns_attempt(&request) {
                    return Ok(());
                }
                if !valid
                    || !surface.draft_calls.is_empty()
                    || self.capture_delivery.as_ref().is_some_and(|delivery| {
                        delivery.blocks(&surface.gate.id, &surface.gate.epoch)
                    })
                {
                    self.blocked()?;
                    return Ok(());
                }
                if final_pass {
                    if !surface.gate.final_verified(&request, serial) {
                        return Ok(());
                    }
                    if self.global_exit {
                        if self
                            .surfaces
                            .values()
                            .all(|surface| surface.gate.permitted())
                        {
                            // Every actual author surface is frozen and has
                            // its final original Core proof. Seal before notice.
                            self.visibility.confirm_exit(true, true);
                            self.notice(Notice::ExitPermitted)?;
                        }
                    } else {
                        self.destroy_surface(id(kind))?;
                    }
                } else if surface.gate.verified(&request, serial) {
                    surface
                        .view
                        .as_ref()
                        .ok_or_else(failed)?
                        .post(&message("commit-close", &request))?;
                }
            }
            Command::DraftResult {
                surface_id,
                surface_epoch,
                request_id,
                body,
                session_ack,
            } => {
                let Some(surface) = self.surfaces.values_mut().find(|surface| {
                    surface.gate.id == surface_id && surface.gate.epoch == surface_epoch
                }) else {
                    if let Some(session) = session_ack {
                        self.notice(Notice::Closed {
                            proofs: Vec::new(),
                            sessions: vec![session],
                        })?;
                    }
                    return Ok(());
                };
                if !surface.draft_calls.remove(&request_id) {
                    return Ok(());
                }
                if let Some(session) = session_ack {
                    if surface.sessions.len() >= 32768 && !surface.sessions.contains(&session) {
                        return Err(failed());
                    }
                    surface.sessions.insert(session);
                }
                surface.view.as_ref().ok_or_else(failed)?.post(&serde_json::json!({"type":"core-draft-result","surfaceId":surface_id,"surfaceEpoch":surface_epoch,"requestId":request_id,"body":body}))?;
            }
            Command::CaptureResult { request, opened } => {
                self.capture_opened(request, opened.map(|opened| *opened))?
            }
            Command::BeginExit => {
                if self.global_exit {
                    return Ok(());
                }
                self.global_exit = true;
                if self
                    .visibility
                    .confirm_exit(self.surfaces.values().any(|surface| surface.exposed), false)
                {
                    // This serial STA cut also covers queued late Ready and
                    // controller completions for windows never shown to a user.
                    self.notice(Notice::ExitPermitted)?;
                } else {
                    for key in self.surfaces.keys().copied().collect::<Vec<_>>() {
                        self.begin_close(key)?;
                        if !self.global_exit {
                            break;
                        }
                    }
                }
            }
            Command::CancelClose => {
                self.cancel_all()?;
                self.notice(Notice::CloseBlocked)?;
            }
        }
        Ok(())
    }
    fn terminal(&mut self, outcome: Outcome) {
        if outcome != Outcome::Quiescent {
            self.fatal = true;
            self.wake.fail();
        }
        if self.core_retirement.is_none() {
            self.core_retirement = Some(Instant::now());
            // Continue releasing every owned surface after an individual close
            // failure. No ordinary notice delivery is needed after Core exit.
            for key in self.surfaces.keys().copied().collect::<Vec<_>>() {
                if self.destroy_surface(key).is_err() {
                    self.fatal = true;
                    self.wake.fail();
                }
            }
            self.environment = None;
        }
    }
    fn destroy_surface(&mut self, key: u64) -> Result<()> {
        let Some(mut surface) = self.surfaces.remove(&key) else {
            return Ok(());
        };
        let proofs = surface.gate.proofs();
        if surface.exposed {
            self.exposed.fetch_sub(1, Ordering::AcqRel);
        }
        if let Some(view) = surface.view.take() {
            view.close()?;
        }
        surface.window.destroy()?;
        if self.core_retirement.is_none() {
            self.notice(Notice::Closed {
                proofs,
                sessions: surface.sessions.iter().cloned().collect(),
            })?;
            if self.surfaces.is_empty() {
                self.notice(Notice::Empty {
                    admission_epoch: self.admission_epoch,
                })?;
            }
        }
        Ok(())
    }
    fn event(&mut self, event: Event) -> Result<()> {
        if (self.core_retirement.is_some()
            || self.owner_lifetime.is_closing()
            || !self.visibility.accepts_exposure())
            && !matches!(&event, Event::Environment(_) | Event::Controller(_, _))
        {
            // Late browser/OS input cannot reopen, focus, or submit work after
            // Core retirement. COM completion events still release ownership.
            return Ok(());
        }
        match event {
            Event::ScheduleChanged => self.notice(Notice::ScheduleChanged)?,
            Event::Environment(environment) => {
                let environment = environment?;
                if self.core_retirement.is_none()
                    && !self.owner_lifetime.is_closing()
                    && self.visibility.accepts_exposure()
                    && !self.fatal
                {
                    self.environment = Some(environment);
                    for key in self.surfaces.keys().copied().collect::<Vec<_>>() {
                        self.begin_controller(key)?;
                    }
                }
            }
            Event::Controller(weak, controller) => {
                let controller = controller?;
                let Some(key) = self.live_key(&weak).filter(|_| {
                    self.core_retirement.is_none()
                        && !self.owner_lifetime.is_closing()
                        && self.visibility.accepts_exposure()
                        && !self.fatal
                }) else {
                    unsafe {
                        controller.Close()?;
                    }
                    return Ok(());
                };
                let surface = self.surfaces.get_mut(&key).ok_or_else(failed)?;
                surface.view = Some(View::attach(
                    controller,
                    &surface.window.state,
                    &surface.address,
                    self.queue.clone(),
                )?);
            }
            Event::Close(weak) => {
                if let Some(key) = self.live_key(&weak) {
                    self.begin_close(key)?;
                }
            }
            Event::Resize(weak) => {
                if let Some(key) = self.live_key(&weak) {
                    let surface = &self.surfaces[&key];
                    if let Some(view) = &surface.view {
                        view.resize(&surface.window.state)?;
                    }
                }
            }
            Event::Focus(weak) => {
                if let Some(key) = self.live_key(&weak)
                    && let Some(view) = &self.surfaces[&key].view
                {
                    view.focus()?;
                }
            }
            Event::ParentMoved(weak) => {
                if let Some(key) = self.live_key(&weak)
                    && let Some(view) = &self.surfaces[&key].view
                {
                    view.parent_moved()?;
                }
            }
            Event::Message(weak, source, json) => {
                let Some(key) = self.live_key(&weak) else {
                    return Ok(());
                };
                let surface = self.surfaces.get_mut(&key).ok_or_else(failed)?;
                if !surface
                    .view
                    .as_ref()
                    .is_some_and(|view| view.policy.permits(&source))
                {
                    return Err(failed());
                }
                let incoming: Message = match serde_json::from_str(&json) {
                    Ok(message) => message,
                    Err(_) => {
                        // A bad author/protocol payload is not permission to close
                        // a still-live editor or to pretend its input is durable.
                        surface.gate.ready = false;
                        self.cancel_all()?;
                        self.notice(Notice::CloseBlocked)?;
                        return Ok(());
                    }
                };
                match incoming {
                    Message::RequestClose {
                        surface_id,
                        surface_epoch,
                    } => {
                        if surface.gate.owns_surface(
                            SurfaceKind::Capture,
                            &surface_id,
                            &surface_epoch,
                        ) {
                            self.begin_close(key)?;
                        }
                    }
                    Message::CaptureOpen {
                        surface_id,
                        surface_epoch,
                        request_id,
                        draft_id,
                        operation_id,
                    } => {
                        let request = CaptureOpen {
                            surface_id,
                            surface_epoch,
                            request_id,
                            draft_id,
                            operation_id,
                        };
                        if !surface.gate.owns_surface(
                            SurfaceKind::Capture,
                            &request.surface_id,
                            &request.surface_epoch,
                        ) {
                            return Ok(());
                        }
                        if !request.valid() {
                            return Err(failed());
                        }
                        if self
                            .capture_delivery
                            .as_ref()
                            .is_some_and(|delivery| delivery.owns(&request))
                        {
                            return Ok(());
                        }
                        if !surface.gate.ready
                            || surface.gate.closing()
                            || self.global_exit
                            || self.fatal
                            || self.capture_delivery.is_some()
                            || surface.capture_requests.len() >= 32768
                            || !surface.capture_requests.insert(request.request_id.clone())
                        {
                            self.capture_result(&request, false)?;
                            return Ok(());
                        }
                        self.capture_delivery = CaptureDelivery::new(request.clone());
                        self.notice(Notice::CaptureOpen(request))?;
                    }
                    Message::DocumentDelivered {
                        surface_id,
                        surface_epoch,
                        delivery_id,
                        accepted,
                    } => {
                        if !surface.gate.owns_surface(
                            SurfaceKind::Library,
                            &surface_id,
                            &surface_epoch,
                        ) || !self.capture_delivery.as_ref().is_some_and(|delivery| {
                            delivery.acknowledges(&surface_id, &surface_epoch, &delivery_id)
                                && delivery
                                    .session_id()
                                    .is_some_and(|session| surface.sessions.contains(session))
                        }) {
                            return Ok(());
                        }
                        let delivery = self.capture_delivery.take().ok_or_else(failed)?;
                        self.capture_result(&delivery.request, accepted)?;
                    }
                    Message::ActionDelivered {
                        surface_id,
                        surface_epoch,
                        action_id,
                        accepted,
                    } => {
                        complete_confirmed_action(
                            &surface.gate,
                            &mut surface.confirmed_posts,
                            &surface_id,
                            &surface_epoch,
                            &action_id,
                            accepted,
                        );
                    }
                    Message::DraftCall {
                        surface_id,
                        surface_epoch,
                        request_id,
                        request,
                    } => {
                        if surface.gate.kind != SurfaceKind::Library
                            || surface.gate.id != surface_id
                            || surface.gate.epoch != surface_epoch
                            || !crate::broker::canonical_uuid(&request_id)
                            || !crate::broker::canonical_uuid(request.session_id())
                            || surface.draft_calls.len() >= 8
                            || !surface.draft_calls.insert(request_id.clone())
                        {
                            return Err(failed());
                        }
                        let cancelled = surface.gate.current_request();
                        let invalidated = surface.gate.draft_call();
                        if invalidated {
                            if let Some(request) = cancelled
                                && let Some(view) = &surface.view
                            {
                                view.post(&message("close-cancelled", &request))?;
                            }
                            self.cancel_all()?;
                            self.notice(Notice::CloseBlocked)?;
                        }
                        self.notice(Notice::Draft {
                            surface_id,
                            surface_epoch,
                            request_id,
                            request,
                        })?;
                    }
                    Message::SurfaceUnready {
                        surface_id,
                        surface_epoch,
                    } => {
                        if surface.gate.id != surface_id || surface.gate.epoch != surface_epoch {
                            return Err(failed());
                        }
                        surface.gate.ready = false;
                        self.cancel_all()?;
                        self.notice(Notice::CloseBlocked)?;
                    }
                    Message::SurfaceFailed { kind } => {
                        if kind != surface.gate.kind {
                            return Err(failed());
                        }
                        let rejects_delivery =
                            self.capture_delivery.as_ref().is_some_and(|delivery| {
                                delivery.blocks(&surface.gate.id, &surface.gate.epoch)
                            });
                        surface.gate.ready = false;
                        self.cancel_all()?;
                        self.notice(Notice::CloseBlocked)?;
                        if rejects_delivery {
                            self.reject_capture_delivery()?;
                        }
                        let surface = self.surfaces.get(&key).ok_or_else(failed)?;
                        let can_destroy = !surface.exposed
                            && surface.sessions.is_empty()
                            && surface.draft_calls.is_empty();
                        self.command(Command::Feedback(UiMessage::native_host_start_failed()))?;
                        // A never-exposed bootstrap with no producer or result
                        // ownership can be retried with a fresh incarnation.
                        // Every exposed/owned surface remains intact and blocked.
                        if can_destroy {
                            self.destroy_surface(key)?;
                        }
                    }
                    Message::SurfaceReady { kind } => {
                        if kind != surface.gate.kind || surface.gate.ready {
                            return Err(failed());
                        }
                        surface.gate.ready = true;
                        let view = surface.view.as_ref().ok_or_else(failed)?;
                        view.post(&serde_json::json!({"type":"surface-bound","surfaceId":surface.gate.id,"surfaceEpoch":surface.gate.epoch}))?;
                        view.show()?;
                        // These COM calls can fail while the HWND is still
                        // hidden. Publish exposure only after they succeed;
                        // their callbacks merely enqueue work for this reducer.
                        if !surface.exposed {
                            surface.exposed = true;
                            self.exposed.fetch_add(1, Ordering::AcqRel);
                        }
                        surface.window.show();
                        let mut rejected_delivery = None;
                        for pending in surface.after_ready.drain(..) {
                            if let Some(delivery_id) = &pending.delivery_id {
                                let Some(delivery) = self
                                    .capture_delivery
                                    .as_mut()
                                    .filter(|delivery| delivery.request.request_id == *delivery_id)
                                else {
                                    continue;
                                };
                                if view.post(&pending.message).is_err() {
                                    rejected_delivery = Some(delivery.request.clone());
                                    continue;
                                }
                                if !delivery.mark_posted(delivery_id) {
                                    return Err(failed());
                                }
                            } else {
                                if let Some(action_id) = pending
                                    .message
                                    .get("actionId")
                                    .and_then(serde_json::Value::as_str)
                                {
                                    let Some(action) = surface.confirmed_posts.get_mut(action_id)
                                    else {
                                        continue;
                                    };
                                    if !action.can_post() {
                                        continue;
                                    }
                                    view.post(&pending.message)?;
                                    action.posted = true;
                                } else {
                                    view.post(&pending.message)?;
                                }
                            }
                            if pending.activate {
                                surface.window.show();
                            }
                        }
                        if let Some(request) = rejected_delivery {
                            self.capture_delivery = None;
                            self.capture_result(&request, false)?;
                        }
                    }
                    Message::SurfaceEdited {
                        surface_id,
                        surface_epoch,
                        surface_edit_revision,
                    } => {
                        let cancelled = surface.gate.current_request();
                        if !surface
                            .gate
                            .edited(&surface_id, &surface_epoch, surface_edit_revision)
                        {
                            return Err(failed());
                        }
                        if let Some(request) = cancelled
                            && !surface.gate.closing()
                        {
                            if let Some(view) = &surface.view {
                                view.post(&message("close-cancelled", &request))?;
                            }
                            self.cancel_all()?;
                            self.notice(Notice::CloseBlocked)?;
                        }
                    }
                    Message::ClosePrepared {
                        surface_id,
                        surface_epoch,
                        close_attempt_id,
                        surface_edit_revision,
                        proofs,
                    } => {
                        if !surface.draft_calls.is_empty()
                            || self.capture_delivery.as_ref().is_some_and(|delivery| {
                                delivery.blocks(&surface.gate.id, &surface.gate.epoch)
                            })
                        {
                            self.blocked()?;
                            return Ok(());
                        }
                        if proofs.iter().any(|proof|matches!(proof,Proof::Document{session_id,..} if !surface.sessions.contains(session_id))){self.blocked()?;return Ok(())}
                        let request = Request {
                            surface_id,
                            surface_epoch,
                            close_attempt_id,
                        };
                        if let Some(proofs) =
                            surface
                                .gate
                                .prepared(&request, surface_edit_revision, proofs)
                        {
                            self.notice(Notice::Verify {
                                kind: self.surfaces[&key].gate.kind,
                                request,
                                serial: surface_edit_revision,
                                final_pass: false,
                                proofs,
                            })?;
                        }
                    }
                    Message::CloseFrozen {
                        surface_id,
                        surface_epoch,
                        close_attempt_id,
                        surface_edit_revision,
                    } => {
                        if !surface.draft_calls.is_empty()
                            || self.capture_delivery.as_ref().is_some_and(|delivery| {
                                delivery.blocks(&surface.gate.id, &surface.gate.epoch)
                            })
                        {
                            self.blocked()?;
                            return Ok(());
                        }
                        let request = Request {
                            surface_id,
                            surface_epoch,
                            close_attempt_id,
                        };
                        if let Some(proofs) = surface.gate.frozen(&request, surface_edit_revision) {
                            self.notice(Notice::Verify {
                                kind: self.surfaces[&key].gate.kind,
                                request,
                                serial: surface_edit_revision,
                                final_pass: true,
                                proofs,
                            })?;
                        }
                    }
                    Message::CloseBlocked {
                        surface_id,
                        surface_epoch,
                        close_attempt_id,
                        reason,
                    } => {
                        if reason.is_empty() || reason.len() > 64 * 1024 {
                            return Err(failed());
                        }
                        if surface.gate.owns_attempt(&Request {
                            surface_id,
                            surface_epoch,
                            close_attempt_id,
                        }) {
                            self.blocked()?;
                        }
                    }
                    Message::LocaleChanged {
                        surface_id,
                        surface_epoch,
                    } => {
                        if surface.gate.id == surface_id && surface.gate.epoch == surface_epoch {
                            self.notice(Notice::LocaleChanged)?;
                        }
                    }
                }
            }
            Event::Destroyed(weak) => {
                if weak
                    .upgrade()
                    .is_some_and(|state| self.surfaces.contains_key(&state.id))
                {
                    return Err(failed());
                }
            }
            Event::ViewFailed(weak) => {
                if self.live_key(&weak).is_some() {
                    return Err(failed());
                }
            }
            Event::ExternalUrl(uri) => self.notice(Notice::ExternalUrl(uri))?,
            Event::Tray(code) => {
                // shellapi.h defines NIN_KEYSELECT as NIN_SELECT | NINF_KEY(1).
                if code == windows::Win32::UI::Shell::NIN_SELECT
                    || code == (windows::Win32::UI::Shell::NIN_SELECT | 1)
                {
                    self.notice(Notice::Action(Action::Library))?;
                } else if code == WM_CONTEXTMENU || code == WM_RBUTTONUP {
                    self.notice(Notice::MenuRequested)?;
                }
            }
            Event::Command(command) => {
                if let Some(action) = Action::decode(command) {
                    self.notice(Notice::Action(action))?;
                }
            }
            Event::TaskbarCreated => self.tray.restore()?,
            Event::Quit => self.notice(Notice::Action(Action::Exit))?,
        }
        Ok(())
    }
}
fn message(kind: &str, request: &Request) -> serde_json::Value {
    serde_json::json!({"type":kind,"surfaceId":request.surface_id,"surfaceEpoch":request.surface_epoch,"closeAttemptId":request.close_attempt_id})
}

fn run(
    user_data: PathBuf,
    commands: Receiver<Command>,
    notices: mpsc::Sender<Notice>,
    wake: Wake,
    exposed: Arc<AtomicUsize>,
    terminal: Arc<Terminal>,
    owner_lifetime: Arc<crate::broker::lifetime::ServiceContinuation>,
) -> io::Result<()> {
    let pending = Rc::new(Cell::new(0));
    let apartment = Sta::enter(pending.clone()).map_err(io::Error::other)?;
    let old_dpi =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if old_dpi.0.is_null() {
        return Err(io::Error::other("The native UI DPI context is unavailable"));
    }
    Window::register().map_err(io::Error::other)?;
    let queue = Queue::new(wake.clone());
    let dispatcher = Window::create(0, "Weftext", queue.clone()).map_err(io::Error::other)?;
    let tray = Tray::new(
        dispatcher
            .state
            .live_hwnd()
            .ok_or_else(|| io::Error::other("Native dispatcher HWND is not live"))?,
    )
    .map_err(io::Error::other)?;
    let mut reducer = Reducer {
        dispatcher,
        surfaces: BTreeMap::new(),
        environment: None,
        queue,
        wake: wake.clone(),
        pending: pending.clone(),
        notices,
        tray,
        locale: UiLocale::default(),
        preferences: Default::default(),
        startup: Default::default(),
        cli_service: false,
        global_exit: false,
        core_retirement: None,
        fatal: false,
        exposed,
        owner_lifetime,
        admission_epoch: 0,
        visibility: Default::default(),
        capture_delivery: None,
    };
    if reducer.environment(user_data).is_err() {
        reducer.fail();
    }
    loop {
        if wake.consumed().is_err() {
            reducer.fail();
        }
        if let Some(outcome) = terminal.outcome() {
            reducer.terminal(outcome);
            terminal.acknowledge(outcome);
        }
        loop {
            if !terminal.accepts_commands() {
                break;
            }
            match commands.try_recv() {
                Ok(command) => {
                    if reducer.command(command).is_err() {
                        reducer.fail()
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    reducer.fail();
                    break;
                }
            }
        }
        while let Some(event) = reducer.queue.pop() {
            if let Some(outcome) = terminal.outcome() {
                reducer.terminal(outcome);
                terminal.acknowledge(outcome);
            }
            if reducer.event(event).is_err() {
                reducer.fail();
            }
        }
        if wake.failed() {
            reducer.fatal = true;
        }
        if let Some(stopped) = reducer.core_retirement {
            if pending.get() == 0 {
                // A concurrent contradictory publication must be applied and
                // acknowledged before the exact STA can report retirement.
                if let Some(outcome) = terminal.outcome() {
                    reducer.terminal(outcome);
                    if !terminal.acknowledge(outcome) {
                        continue;
                    }
                }
                break;
            }
            if stopped.elapsed() > Duration::from_secs(10) {
                reducer.fail();
                break;
            }
        }
        let mut message = MSG::default();
        if unsafe { PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE) }.as_bool() {
            // GetMessage has three states. WM_QUIT is a fatal loss of the UI
            // pump until Core quiescence, never a last-window shortcut.
            match unsafe { GetMessageW(&mut message, None, 0, 0) }.0 {
                -1 => reducer.fail(),
                0 => reducer.fail(),
                _ => unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                },
            }
            continue;
        }
        let waited = unsafe {
            MsgWaitForMultipleObjectsEx(
                Some(&[wake.wait_handle()]),
                100,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            )
        };
        if waited.0 == u32::MAX {
            reducer.fail();
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    let failed = reducer.fatal;
    drop(reducer);
    unsafe {
        SetThreadDpiAwarenessContext(old_dpi);
    }
    if pending.get() == 0 {
        apartment.finish().map_err(io::Error::other)?;
    } else {
        drop(apartment);
        return Err(io::Error::other(
            "The final Core result was consumed, but native COM callback retirement is unconfirmed",
        ));
    }
    if failed || wake.failed() {
        Err(io::Error::other(
            "Native dispatch or final release failed; clean shutdown was not confirmed",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_reply_requires_a_posted_action_on_the_original_library_incarnation() {
        let gate = Gate::new(SurfaceKind::Library);
        let other = Gate::new(SurfaceKind::Library);
        let id = uuid::Uuid::new_v4().to_string();
        let (completed, mut result) = tokio::sync::oneshot::channel();
        let mut actions = BTreeMap::from([(
            id.clone(),
            ConfirmedAction {
                completed,
                posted: false,
            },
        )]);
        assert!(!complete_confirmed_action(
            &gate,
            &mut actions,
            &gate.id,
            &gate.epoch,
            &id,
            true
        ));
        assert!(matches!(
            result.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        actions.get_mut(&id).unwrap().posted = true;
        assert!(!complete_confirmed_action(
            &gate,
            &mut actions,
            &other.id,
            &other.epoch,
            &id,
            true
        ));
        assert!(!complete_confirmed_action(
            &gate,
            &mut actions,
            &gate.id,
            &other.epoch,
            &id,
            true
        ));
        assert!(!complete_confirmed_action(
            &gate,
            &mut actions,
            &gate.id,
            &gate.epoch,
            "another-action",
            true
        ));
        assert!(complete_confirmed_action(
            &gate,
            &mut actions,
            &gate.id,
            &gate.epoch,
            &id,
            false
        ));
        assert!(!result.try_recv().unwrap());
        assert!(!complete_confirmed_action(
            &gate,
            &mut actions,
            &gate.id,
            &gate.epoch,
            &id,
            true
        ));
    }

    #[test]
    fn original_navigation_timeout_prevents_a_still_queued_action_from_being_posted() {
        let (completed, result) = tokio::sync::oneshot::channel();
        let action = ConfirmedAction {
            completed,
            posted: false,
        };
        assert!(action.can_post());
        drop(result);
        assert!(!action.can_post());
        let (completed, _result) = tokio::sync::oneshot::channel();
        assert!(
            !ConfirmedAction {
                completed,
                posted: true
            }
            .can_post()
        );
    }

    #[test]
    fn original_navigation_timeout_in_the_main_command_queue_rejects_admission() {
        let (wake, _fault) = Wake::new().unwrap();
        let (send, queued) = std::sync::mpsc::sync_channel(2);
        let sender = Sender {
            send,
            wake,
            terminal: Arc::new(Terminal::default()),
        };
        let (completed, expired) = tokio::sync::oneshot::channel();
        sender
            .send(Command::ConfirmedPost {
                message: serde_json::json!({"type":"reminder-activation"}),
                completed,
            })
            .unwrap();
        drop(expired);
        let Command::ConfirmedPost { completed, .. } = queued.recv().unwrap() else {
            panic!("expected original confirmed command");
        };
        assert!(ConfirmedAction::admit(completed).is_none());

        let (completed, mut live) = tokio::sync::oneshot::channel();
        sender
            .send(Command::ConfirmedPost {
                message: serde_json::json!({"type":"reminder-activation"}),
                completed,
            })
            .unwrap();
        let Command::ConfirmedPost { completed, .. } = queued.recv().unwrap() else {
            panic!("expected live confirmed command");
        };
        let action = ConfirmedAction::admit(completed).unwrap();
        assert!(action.can_post());
        action.completed.send(false).unwrap();
        assert!(!live.try_recv().unwrap());
    }

    #[test]
    fn real_sender_terminal_bypasses_full_normal_queue_and_failed_wake() {
        for outcome in [Outcome::Quiescent, Outcome::ReleaseFailed] {
            let (wake, _fault) = Wake::new().unwrap();
            let (send, _queued) = std::sync::mpsc::sync_channel(64);
            let terminal = Arc::new(Terminal::default());
            let sender = Sender {
                send,
                wake: wake.clone(),
                terminal: terminal.clone(),
            };
            for _ in 0..64 {
                sender.send(Command::CancelClose).unwrap();
            }
            assert!(sender.send(Command::CancelClose).is_err());
            assert!(wake.failed());
            sender.finish_core(outcome).unwrap();
            assert_eq!(terminal.outcome(), Some(outcome));
            assert!(sender.send(Command::CancelClose).is_err());
            assert!(terminal.acknowledge(outcome));
            terminal.stopped();
            assert_eq!(
                terminal.confirm_joined().is_ok(),
                outcome == Outcome::Quiescent
            );
        }
    }

    #[test]
    fn real_sender_rejects_already_returned_ui_without_inventing_a_result() {
        let (wake, _fault) = Wake::new().unwrap();
        let (send, queued) = std::sync::mpsc::sync_channel(64);
        let terminal = Arc::new(Terminal::default());
        let sender = Sender {
            send,
            wake,
            terminal: terminal.clone(),
        };
        terminal.stopped();
        assert!(sender.send(Command::CancelClose).is_err());
        assert!(sender.finish_core(Outcome::Quiescent).is_err());
        assert!(queued.try_recv().is_err());
        assert_eq!(terminal.outcome(), None);
    }
}
