use super::{
    events::{Event, Queue},
    window::WindowState,
};
use std::{cell::Cell, rc::Rc, time::Instant};
use webview2_com::{
    Microsoft::Web::WebView2::Win32::*, NavigationCompletedEventHandler,
    NavigationStartingEventHandler, NewWindowRequestedEventHandler, ProcessFailedEventHandler,
    WebMessageReceivedEventHandler,
};
use windows::{
    Win32::{
        Foundation::{E_INVALIDARG, E_POINTER, RECT},
        System::Com::CoTaskMemFree,
        UI::WindowsAndMessaging::GetClientRect,
    },
    core::{Error, HSTRING, Interface, PWSTR, Result},
};

pub(super) struct Pending {
    count: Rc<Cell<usize>>,
    queue: Queue,
}
impl Pending {
    pub fn new(count: &Rc<Cell<usize>>, queue: &Queue) -> Result<Self> {
        count.set(
            count
                .get()
                .checked_add(1)
                .ok_or_else(|| Error::from(E_INVALIDARG))?,
        );
        Ok(Self {
            count: count.clone(),
            queue: queue.clone(),
        })
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        match self.count.get().checked_sub(1) {
            Some(count) => self.count.set(count),
            None => self.queue.fault(),
        }
        self.queue.wake();
    }
}

fn text(pointer: PWSTR, maximum: usize) -> Result<String> {
    struct Allocation(PWSTR);
    impl Drop for Allocation {
        fn drop(&mut self) {
            unsafe {
                CoTaskMemFree(Some(self.0.0.cast()));
            }
        }
    }
    let allocation = Allocation(pointer);
    if allocation.0.is_null() {
        return Err(Error::from(E_POINTER));
    }
    let mut length = 0;
    while length <= maximum && unsafe { *allocation.0.0.add(length) } != 0 {
        length += 1;
    }
    if length > maximum {
        return Err(Error::from(E_INVALIDARG));
    }
    String::from_utf16(unsafe { std::slice::from_raw_parts(allocation.0.0, length) })
        .map_err(|_| Error::from(E_INVALIDARG))
}

#[derive(Clone)]
pub(super) struct NavigationPolicy {
    origin: url::Origin,
    path: String,
    query: Option<String>,
}
impl NavigationPolicy {
    pub fn new(address: &str) -> Result<Self> {
        let address = url::Url::parse(address).map_err(|_| Error::from(E_INVALIDARG))?;
        if address.scheme() != "http"
            || address.host_str() != Some("127.0.0.1")
            || address.port().is_none()
            || !address.username().is_empty()
            || address.password().is_some()
        {
            return Err(Error::from(E_INVALIDARG));
        }
        Ok(Self {
            origin: address.origin(),
            path: address.path().into(),
            query: address.query().map(str::to_owned),
        })
    }
    pub fn permits(&self, address: &str) -> bool {
        url::Url::parse(address).is_ok_and(|address| {
            address.origin() == self.origin
                && address.path() == self.path
                && address.query() == self.query.as_deref()
                && address.username().is_empty()
                && address.password().is_none()
        })
    }
}

/// The one admitted document load is distinct from later navigations that the
/// close barrier deliberately cancels. Their unsuccessful completion must not
/// retire a healthy author surface or hide an actual initial-load failure.
struct InitialNavigation {
    policy: NavigationPolicy,
    started: Cell<bool>,
    pending: Cell<Option<u64>>,
    succeeded_at: Cell<Option<Instant>>,
}
impl InitialNavigation {
    fn new(policy: NavigationPolicy) -> Self {
        Self {
            policy,
            started: Cell::new(false),
            pending: Cell::new(None),
            succeeded_at: Cell::new(None),
        }
    }
    fn start(&self, address: &str, navigation_id: u64) -> bool {
        if !self.policy.permits(address) || self.started.replace(true) {
            return false;
        }
        self.pending.set(Some(navigation_id));
        true
    }
    fn failed_completion(&self, navigation_id: u64, success: bool, now: Instant) -> bool {
        if self.pending.get() != Some(navigation_id) {
            return false;
        }
        self.pending.set(None);
        if success {
            self.succeeded_at.set(Some(now));
        }
        !success
    }
}

pub(super) struct View {
    controller: ICoreWebView2Controller,
    webview: ICoreWebView2,
    tokens: Vec<(u8, i64)>,
    closed: bool,
    queue: Queue,
    pub policy: NavigationPolicy,
    initial: Rc<InitialNavigation>,
}

// An OpenedDocument contains a 2 MiB source (up to 12 MiB after JSON escaping)
// plus the selected recovery record, whose Core serialization is bounded at
// 16 MiB, and path/identity/message metadata. A 512 KiB envelope rejected valid
// ordinary documents; keep an explicit complete-message budget instead.
const MAX_HOST_MESSAGE_BYTES: usize = 32 * 1024 * 1024;
fn post_json(message: &serde_json::Value) -> Result<String> {
    let text = serde_json::to_string(message).map_err(|_| Error::from(E_INVALIDARG))?;
    if text.len() > MAX_HOST_MESSAGE_BYTES {
        return Err(Error::from(E_INVALIDARG));
    }
    Ok(text)
}

impl View {
    /// Runs only in the outer STA reducer. The guard exists before any fallible
    /// settings/registration/navigation call can leave a controller behind.
    pub fn attach(
        controller: ICoreWebView2Controller,
        state: &Rc<WindowState>,
        address: &str,
        queue: Queue,
    ) -> Result<Self> {
        let webview = match unsafe { controller.CoreWebView2() } {
            Ok(webview) => webview,
            Err(error) => {
                let _ = unsafe { controller.Close() };
                return Err(error);
            }
        };
        let policy = match NavigationPolicy::new(address) {
            Ok(policy) => policy,
            Err(error) => {
                let _ = unsafe { controller.Close() };
                return Err(error);
            }
        };
        let initial = Rc::new(InitialNavigation::new(policy.clone()));
        let mut view = Self {
            controller,
            webview,
            tokens: Vec::new(),
            closed: false,
            queue: queue.clone(),
            policy,
            initial: initial.clone(),
        };
        let settings = unsafe { view.webview.Settings()? };
        unsafe {
            settings.SetAreHostObjectsAllowed(false)?;
            settings.SetAreDevToolsEnabled(false)?;
            settings.SetAreDefaultContextMenusEnabled(false)?;
            settings
                .cast::<ICoreWebView2Settings3>()?
                .SetAreBrowserAcceleratorKeysEnabled(false)?;
            view.controller.SetIsVisible(false)?;
        }
        let starting = initial.clone();
        let callback = NavigationStartingEventHandler::create(Box::new(move |_, args| {
            let args = args.ok_or_else(|| Error::from(E_POINTER))?;
            let mut uri = PWSTR::null();
            let mut navigation_id = 0;
            unsafe {
                args.NavigationId(&mut navigation_id)?;
                args.Uri(&mut uri)?;
            }
            let allowed = text(uri, 32768).is_ok_and(|uri| starting.start(&uri, navigation_id));
            // Reload/back/context actions cannot destroy a live unsaved producer
            // outside the same close barrier. A host-controlled reopen gets a
            // new controller/surface epoch after its prior barrier completes.
            unsafe { args.SetCancel(!allowed) }
        }));
        let mut token = 0;
        unsafe {
            view.webview.add_NavigationStarting(&callback, &mut token)?;
        }
        view.tokens.push((0, token));
        let weak = Rc::downgrade(state);
        let events = queue.clone();
        let callback = NavigationCompletedEventHandler::create(Box::new(move |_, args| {
            let args = args.ok_or_else(|| Error::from(E_POINTER))?;
            let mut navigation_id = 0;
            let mut success = false.into();
            unsafe {
                args.NavigationId(&mut navigation_id)?;
                args.IsSuccess(&mut success)?;
            }
            if initial.failed_completion(navigation_id, success.as_bool(), Instant::now()) {
                // Navigation errors (including a failed local response) need
                // not crash a WebView process. Keep the existing exact-surface
                // failure/author-retention path instead of waiting forever for
                // a page that cannot announce surface-ready.
                events.push(Event::ViewFailed(weak.clone()));
            }
            Ok(())
        }));
        let mut token = 0;
        unsafe {
            view.webview
                .add_NavigationCompleted(&callback, &mut token)?;
        }
        view.tokens.push((4, token));
        let weak = Rc::downgrade(state);
        let events = queue.clone();
        let callback = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            let args = args.ok_or_else(|| Error::from(E_POINTER))?;
            let mut uri = PWSTR::null();
            let mut json = PWSTR::null();
            unsafe {
                args.Source(&mut uri)?;
            }
            let source = text(uri, 32768)?;
            unsafe {
                args.WebMessageAsJson(&mut json)?;
            }
            let message = text(json, 16 * 1024 * 1024)?;
            events.push(Event::Message(weak.clone(), source, message));
            Ok(())
        }));
        let mut token = 0;
        unsafe {
            view.webview.add_WebMessageReceived(&callback, &mut token)?;
        }
        view.tokens.push((1, token));
        let events = queue.clone();
        let callback = NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
            let args = args.ok_or_else(|| Error::from(E_POINTER))?;
            unsafe {
                args.SetHandled(true)?;
            }
            let mut initiated = false.into();
            unsafe {
                args.IsUserInitiated(&mut initiated)?;
            }
            if !initiated.as_bool() {
                return Ok(());
            }
            let mut uri = PWSTR::null();
            unsafe {
                args.Uri(&mut uri)?;
            }
            let uri = text(uri, 32768)?;
            if url::Url::parse(&uri).is_ok_and(|uri| {
                matches!(uri.scheme(), "https" | "http")
                    && uri.username().is_empty()
                    && uri.password().is_none()
            }) {
                events.push(Event::ExternalUrl(uri));
            }
            Ok(())
        }));
        let mut token = 0;
        unsafe {
            view.webview.add_NewWindowRequested(&callback, &mut token)?;
        }
        view.tokens.push((2, token));
        let weak = Rc::downgrade(state);
        let events = queue;
        let callback = ProcessFailedEventHandler::create(Box::new(move |_, _| {
            events.push(Event::ViewFailed(weak.clone()));
            Ok(())
        }));
        let mut token = 0;
        unsafe {
            view.webview.add_ProcessFailed(&callback, &mut token)?;
        }
        view.tokens.push((3, token));
        view.resize(state)?;
        if state.live_hwnd().is_none() || state.destroying.get() {
            return Err(Error::from(E_POINTER));
        }
        unsafe {
            view.webview.Navigate(&HSTRING::from(address))?;
        }
        Ok(view)
    }
    pub fn initial_load_succeeded_at(&self) -> Option<Instant> {
        self.initial.succeeded_at.get()
    }
    pub fn post(&self, message: &serde_json::Value) -> Result<()> {
        let text = post_json(message)?;
        unsafe { self.webview.PostWebMessageAsJson(&HSTRING::from(text)) }
    }
    pub fn show(&self) -> Result<()> {
        unsafe { self.controller.SetIsVisible(true) }
    }
    pub fn resize(&self, state: &Rc<WindowState>) -> Result<()> {
        if let Some(hwnd) = state.live_hwnd() {
            let mut bounds = RECT::default();
            unsafe {
                GetClientRect(hwnd, &mut bounds)?;
                self.controller.SetBounds(bounds)?;
            }
        }
        Ok(())
    }
    pub fn focus(&self) -> Result<()> {
        unsafe {
            self.controller
                .MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC)
        }
    }
    pub fn parent_moved(&self) -> Result<()> {
        unsafe { self.controller.NotifyParentWindowPositionChanged() }
    }
    pub fn close(mut self) -> Result<()> {
        self.close_inner()
    }
    fn close_inner(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let mut failure = None;
        for (kind, token) in self.tokens.drain(..) {
            let result = unsafe {
                match kind {
                    0 => self.webview.remove_NavigationStarting(token),
                    1 => self.webview.remove_WebMessageReceived(token),
                    2 => self.webview.remove_NewWindowRequested(token),
                    3 => self.webview.remove_ProcessFailed(token),
                    _ => self.webview.remove_NavigationCompleted(token),
                }
            };
            if let Err(error) = result {
                failure.get_or_insert(error);
            }
        }
        if let Err(error) = unsafe { self.controller.Close() } {
            failure.get_or_insert(error);
        }
        failure.map_or(Ok(()), Err)
    }
}
impl Drop for View {
    fn drop(&mut self) {
        if self.close_inner().is_err() {
            self.queue.fault();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initial_navigation() -> InitialNavigation {
        InitialNavigation::new(
            NavigationPolicy::new("http://127.0.0.1:47821/?native=library").unwrap(),
        )
    }

    #[test]
    fn admitted_initial_navigation_failure_is_reported_once() {
        let initial = initial_navigation();
        assert!(initial.start("http://127.0.0.1:47821/?native=library", 41));
        assert!(initial.failed_completion(41, false, Instant::now()));
        assert!(!initial.failed_completion(41, false, Instant::now()));
        assert!(initial.succeeded_at.get().is_none());
    }

    #[test]
    fn cancelled_other_navigation_cannot_erase_the_initial_failure() {
        let initial = initial_navigation();
        assert!(!initial.start("https://example.invalid/", 40));
        assert!(!initial.failed_completion(40, false, Instant::now()));
        assert!(initial.start("http://127.0.0.1:47821/?native=library", 41));
        assert!(!initial.start("http://127.0.0.1:47821/?native=library", 42));
        assert!(!initial.failed_completion(42, false, Instant::now()));
        assert!(initial.failed_completion(41, false, Instant::now()));
    }

    #[test]
    fn successful_initial_load_is_not_failed_by_cancelled_reload_or_late_completion() {
        let initial = initial_navigation();
        let loaded = Instant::now();
        assert!(initial.start("http://127.0.0.1:47821/?native=library", 41));
        assert!(!initial.failed_completion(41, true, loaded));
        assert_eq!(initial.succeeded_at.get(), Some(loaded));
        assert!(!initial.start("http://127.0.0.1:47821/?native=library", 42));
        assert!(!initial.failed_completion(42, false, Instant::now()));
        assert!(!initial.failed_completion(41, false, Instant::now()));
        assert!(!initial.failed_completion(41, true, loaded + std::time::Duration::from_secs(30)));
        assert_eq!(initial.succeeded_at.get(), Some(loaded));
    }

    #[test]
    fn another_surface_completion_does_not_consume_the_original_navigation() {
        let library = initial_navigation();
        let capture = InitialNavigation::new(
            NavigationPolicy::new("http://127.0.0.1:47821/capture?native=capture").unwrap(),
        );
        assert!(library.start("http://127.0.0.1:47821/?native=library", 41));
        assert!(capture.start("http://127.0.0.1:47821/capture?native=capture", 52));
        assert!(!library.failed_completion(52, true, Instant::now()));
        assert!(library.succeeded_at.get().is_none());
        assert!(capture.failed_completion(52, false, Instant::now()));
        assert!(library.failed_completion(41, false, Instant::now()));
    }

    #[test]
    fn complete_large_opened_document_includes_escaped_source_and_recovery() {
        let maximum_source = 2 * 1024 * 1024;
        let document = serde_json::json!({
            "type": "open-document",
            "deliveryId": "00000000-0000-4000-8000-000000000001",
            "document": {
                "sessionId": "00000000-0000-4000-8000-000000000002",
                "source": "\u{0000}".repeat(maximum_source),
                "recoveredDraft": "a".repeat(maximum_source),
                "recoveredDraftBaseSource": "\u{0000}".repeat(maximum_source),
                "recoveredDraftVariants": [],
                "path": "C:/notes/original.typ", "root": "C:/notes"
            }
        });
        let encoded = post_json(&document).unwrap();
        assert!(encoded.len() > 26 * 1024 * 1024);
        assert!(encoded.len() < MAX_HOST_MESSAGE_BYTES);
    }

    #[test]
    fn oversized_host_envelopes_still_fail_without_truncating_document_data() {
        let oversized = serde_json::json!({ "source": "a".repeat(MAX_HOST_MESSAGE_BYTES) });
        assert!(post_json(&oversized).is_err());
    }
}
