use super::events::{Event, Queue};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
    sync::atomic::{AtomicU32, Ordering},
};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
    core::{Error, HSTRING, Result, w},
};

const CLASS: windows::core::PCWSTR = w!("Weftext.Native.Surface.v1");
pub(super) const TRAY_MESSAGE: u32 = WM_APP + 2;
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
thread_local! {static WINDOWS:RefCell<HashMap<isize,Weak<WindowState>>>=RefCell::new(HashMap::new());}

pub(super) struct WindowState {
    pub id: u64,
    hwnd: Cell<Option<HWND>>,
    pub destroying: Cell<bool>,
    queue: Queue,
}
impl WindowState {
    pub fn live_hwnd(self: &Rc<Self>) -> Option<HWND> {
        let hwnd = self.hwnd.get()?;
        let same = WINDOWS.with(|windows| {
            windows
                .try_borrow()
                .ok()
                .and_then(|windows| windows.get(&(hwnd.0 as isize)).and_then(Weak::upgrade))
                .is_some_and(|state| Rc::ptr_eq(&state, self))
        });
        (same && unsafe { IsWindow(Some(hwnd)).as_bool() }).then_some(hwnd)
    }
    fn send(self: &Rc<Self>, make: impl FnOnce(Weak<Self>) -> Event) {
        self.queue.push(make(Rc::downgrade(self)));
    }
}

pub(super) struct Window {
    pub state: Rc<WindowState>,
}
impl Window {
    pub fn register() -> Result<()> {
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: HINSTANCE(unsafe { GetModuleHandleW(None)? }.0),
            lpszClassName: CLASS,
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(Error::from_thread());
        }
        let taskbar = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        if taskbar == 0 {
            return Err(Error::from_thread());
        }
        TASKBAR_CREATED.store(taskbar, Ordering::Release);
        Ok(())
    }
    /// id=0 is the hidden top-level dispatcher. It receives TaskbarCreated;
    /// HWND_MESSAGE deliberately is not used.
    pub fn create(id: u64, title: &str, queue: Queue) -> Result<Self> {
        let owned = Self {
            state: Rc::new(WindowState {
                id,
                hwnd: Cell::new(None),
                destroying: Cell::new(false),
                queue,
            }),
        };
        let title = HSTRING::from(title);
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                CLASS,
                &title,
                if id == 0 {
                    WS_POPUP
                } else {
                    WS_OVERLAPPEDWINDOW
                },
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                if id == 2 { 520 } else { 1200 },
                if id == 2 { 640 } else { 850 },
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some(std::ptr::addr_of!(owned.state).cast()),
            )?;
        }
        Ok(owned)
    }
    pub fn show(&self) {
        if let Some(hwnd) = self.state.live_hwnd() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
    pub fn set_title(&self, title: &str) -> Result<()> {
        if let Some(hwnd) = self.state.live_hwnd() {
            unsafe {
                SetWindowTextW(hwnd, &HSTRING::from(title))?;
            }
        }
        Ok(())
    }
    pub fn destroy(&self) -> Result<()> {
        self.state.destroying.set(true);
        if let Some(hwnd) = self.state.live_hwnd() {
            unsafe {
                DestroyWindow(hwnd)?;
            }
        }
        Ok(())
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        if self.destroy().is_err() {
            self.state.queue.fault();
        }
    }
}

fn lookup(hwnd: HWND) -> Option<Rc<WindowState>> {
    WINDOWS.with(|windows| {
        windows
            .try_borrow()
            .ok()?
            .get(&(hwnd.0 as isize))?
            .upgrade()
    })
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match catch_unwind(AssertUnwindSafe(|| unsafe {
        dispatch(hwnd, message, wparam, lparam)
    })) {
        Ok(result) => result,
        Err(_) => {
            if let Some(state) = lookup(hwnd) {
                state.queue.fault();
            }
            LRESULT(0)
        }
    }
}

unsafe fn dispatch(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        let state = unsafe { &*(create.lpCreateParams as *const Rc<WindowState>) };
        state.hwnd.set(Some(hwnd));
        let registered = WINDOWS.with(|windows| {
            windows
                .try_borrow_mut()
                .map(|mut windows| {
                    windows.insert(hwnd.0 as isize, Rc::downgrade(state));
                })
                .is_ok()
        });
        if !registered {
            state.hwnd.set(None);
            state.queue.fault();
            return LRESULT(0);
        }
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    if let Some(state) = lookup(hwnd) {
        if state.id == 0 && message == TASKBAR_CREATED.load(Ordering::Acquire) {
            state.queue.push(Event::TaskbarCreated);
            return LRESULT(0);
        }
        match message {
            WM_TIMECHANGE | WM_SETTINGCHANGE if state.id == 0 => {
                // Re-observe exact saved definitions; never interpret an OS
                // timezone change as permission to rewrite author intent.
                state.queue.push(Event::ScheduleChanged);
            }
            WM_POWERBROADCAST
                if state.id == 0
                    && matches!(
                        wparam.0 as u32,
                        PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND
                    ) =>
            {
                state.queue.push(Event::ScheduleChanged);
            }
            TRAY_MESSAGE => {
                state.queue.push(Event::Tray((lparam.0 as u32) & 0xffff));
                return LRESULT(0);
            }
            WM_COMMAND => {
                state.queue.push(Event::Command(wparam.0 & 0xffff));
                return LRESULT(0);
            }
            WM_CLOSE => {
                if state.id == 0 {
                    state.queue.push(Event::Quit);
                } else {
                    state.send(Event::Close);
                }
                return LRESULT(0);
            }
            WM_SIZE if state.id != 0 => state.send(Event::Resize),
            WM_DPICHANGED if state.id != 0 => {
                let suggested = unsafe { *(lparam.0 as *const RECT) };
                if unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        suggested.left,
                        suggested.top,
                        suggested.right - suggested.left,
                        suggested.bottom - suggested.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                }
                .is_err()
                {
                    state.queue.fault();
                }
                state.send(Event::Resize);
            }
            WM_MOVE if state.id != 0 => state.send(Event::ParentMoved),
            WM_SETFOCUS if state.id != 0 => state.send(Event::Focus),
            WM_DESTROY => return LRESULT(0),
            WM_NCDESTROY => {
                state.hwnd.set(None);
                state.destroying.set(true);
                WINDOWS.with(|windows| {
                    if let Ok(mut windows) = windows.try_borrow_mut() {
                        windows.remove(&(hwnd.0 as isize));
                    }
                });
                if state.id != 0 {
                    state.send(Event::Destroyed);
                }
            }
            _ => {}
        }
    } else if message == WM_CLOSE {
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
