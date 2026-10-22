use super::window::WindowState;
use std::{
    cell::RefCell,
    collections::VecDeque,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2Environment,
};
use windows::{
    Win32::System::Threading::{CreateEventW, ResetEvent, SetEvent},
    core::Result,
};

#[derive(Clone)]
pub(crate) struct Wake {
    fatal: Arc<AtomicBool>,
    fault: tokio::sync::watch::Sender<bool>,
    event: Arc<OwnedHandle>,
}
impl Wake {
    pub(crate) fn new() -> Result<(Self, tokio::sync::watch::Receiver<bool>)> {
        let (fault, received) = tokio::sync::watch::channel(false);
        let event = unsafe { CreateEventW(None, true, false, windows::core::PCWSTR::null()) }?;
        let event = Arc::new(unsafe { OwnedHandle::from_raw_handle(event.0) });
        Ok((
            Self {
                fatal: Arc::new(AtomicBool::new(false)),
                fault,
                event,
            },
            received,
        ))
    }
    pub(super) fn wait_handle(&self) -> windows::Win32::Foundation::HANDLE {
        windows::Win32::Foundation::HANDLE(self.event.as_raw_handle())
    }
    pub(super) fn consumed(&self) -> Result<()> {
        unsafe { ResetEvent(self.wait_handle()) }?;
        Ok(())
    }
    pub(crate) fn fail(&self) {
        self.fatal.store(true, Ordering::Release);
        let _ = self.fault.send(true);
        let _ = unsafe { SetEvent(self.wait_handle()) };
    }
    pub(super) fn failed(&self) -> bool {
        self.fatal.load(Ordering::Acquire)
    }
    pub(crate) fn notify(&self) {
        // MsgWaitForMultipleObjectsEx waits on this exact owned event. No
        // cross-thread HWND lookup/post is needed, including before dispatcher
        // creation or after return; a reused numeric HWND cannot be targeted.
        if unsafe { SetEvent(self.wait_handle()) }.is_err() {
            self.fail();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};

    #[test]
    fn wake_retains_kernel_signal_without_any_window_identity() {
        let (wake, _fault) = Wake::new().unwrap();
        wake.notify();
        assert!(!wake.failed());
        assert_eq!(
            unsafe { WaitForSingleObject(wake.wait_handle(), 0) },
            WAIT_OBJECT_0
        );
    }

    #[test]
    fn dispatcher_fault_does_not_erase_kernel_signal_or_terminal_result() {
        let (wake, _fault) = Wake::new().unwrap();
        let terminal = super::super::terminal::Terminal::default();
        terminal
            .publish(super::super::terminal::Outcome::Quiescent)
            .unwrap();
        wake.fail();
        wake.notify();
        assert!(wake.failed());
        assert_eq!(
            unsafe { WaitForSingleObject(wake.wait_handle(), 0) },
            WAIT_OBJECT_0
        );
        assert_eq!(
            terminal.outcome(),
            Some(super::super::terminal::Outcome::Quiescent)
        );
        assert!(terminal.acknowledge(super::super::terminal::Outcome::Quiescent));
        terminal.stopped();
        terminal.confirm_joined().unwrap();
    }

    #[test]
    fn merged_kernel_signals_drain_every_producer_and_the_last_terminal() {
        let (wake, _fault) = Wake::new().unwrap();
        let terminal = Arc::new(super::super::terminal::Terminal::default());
        let (send, received) = std::sync::mpsc::channel();
        let mut producers = Vec::new();
        for producer in 0..4 {
            let send = send.clone();
            let wake = wake.clone();
            producers.push(std::thread::spawn(move || {
                for sequence in 0..128 {
                    send.send((producer, sequence)).unwrap();
                    wake.notify();
                    std::thread::yield_now();
                }
            }));
        }
        drop(send);
        let final_wake = wake.clone();
        let final_terminal = terminal.clone();
        let joining = std::thread::spawn(move || {
            for producer in producers {
                producer.join().unwrap();
            }
            final_terminal
                .publish(super::super::terminal::Outcome::Quiescent)
                .unwrap();
            final_wake.notify();
        });
        let mut observed = std::collections::BTreeSet::new();
        loop {
            // This is the production order: reset first, then inspect both
            // retained sources, then wait. A producer publishes before notify.
            wake.consumed().unwrap();
            while let Ok(message) = received.try_recv() {
                assert!(observed.insert(message));
            }
            if observed.len() == 512 && terminal.outcome().is_some() {
                break;
            }
            assert_eq!(
                unsafe { WaitForSingleObject(wake.wait_handle(), 5000) },
                WAIT_OBJECT_0
            );
        }
        joining.join().unwrap();
        assert!(!wake.failed());
        assert!(terminal.acknowledge(super::super::terminal::Outcome::Quiescent));
        terminal.stopped();
        terminal.confirm_joined().unwrap();
    }
}

pub(super) enum Event {
    Environment(Result<ICoreWebView2Environment>),
    Controller(Weak<WindowState>, Result<ICoreWebView2Controller>),
    Close(Weak<WindowState>),
    Resize(Weak<WindowState>),
    Focus(Weak<WindowState>),
    ParentMoved(Weak<WindowState>),
    Destroyed(Weak<WindowState>),
    Message(Weak<WindowState>, String, String),
    ViewFailed(Weak<WindowState>),
    ExternalUrl(String),
    Tray(u32),
    Command(usize),
    TaskbarCreated,
    ScheduleChanged,
    Quit,
}

#[derive(Clone)]
pub(super) struct Queue {
    events: Rc<RefCell<Queued>>,
    wake: Wake,
}
#[derive(Default)]
struct Queued {
    events: VecDeque<Event>,
    bytes: usize,
}
fn event_bytes(event: &Event) -> usize {
    match event {
        Event::Message(_, source, json) => source.len().saturating_add(json.len()),
        Event::ExternalUrl(url) => url.len(),
        _ => 0,
    }
}
impl Queue {
    pub(super) fn new(wake: Wake) -> Self {
        Self {
            events: Rc::new(RefCell::new(Queued::default())),
            wake,
        }
    }
    pub(super) fn push(&self, event: Event) {
        let pushed = if let Ok(mut events) = self.events.try_borrow_mut() {
            // Retain COM completion ownership even while input floods the queue.
            // Only the small, bounded number of lifecycle completions may use
            // the reserve; web messages and coalescible OS events fail closed.
            let critical = matches!(
                &event,
                Event::Environment(_) | Event::Controller(_, _) | Event::Destroyed(_)
            );
            let bytes = event_bytes(&event);
            if (events.events.len() >= 4096
                || events.bytes.saturating_add(bytes) > 32 * 1024 * 1024)
                && !critical
            {
                false
            } else {
                events.bytes = events.bytes.saturating_add(bytes);
                events.events.push_back(event);
                true
            }
        } else {
            false
        };
        if pushed {
            self.wake.notify();
        } else {
            self.wake.fail();
        }
    }
    pub(super) fn pop(&self) -> Option<Event> {
        let mut queue = self.events.try_borrow_mut().ok()?;
        let event = queue.events.pop_front()?;
        queue.bytes = queue.bytes.saturating_sub(event_bytes(&event));
        Some(event)
    }
    pub(super) fn fault(&self) {
        self.wake.fail();
    }
    pub(super) fn wake(&self) {
        self.wake.notify();
    }
}
