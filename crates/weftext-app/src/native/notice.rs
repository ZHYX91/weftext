//! One modeless, STA-owned notice. Its dialog procedure never invokes the
//! author reducer: the ordinary outer pump remains the sole event consumer.
use super::events::Wake;
use std::{cell::Cell, rc::Rc};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::*,
    },
    core::{HSTRING, Result},
};

const TEXT: i32 = 100;

struct State {
    hwnd: Cell<Option<HWND>>,
    initialized: Cell<bool>,
    wake: Wake,
}

pub(super) struct Notice {
    state: Rc<State>,
}

impl Notice {
    pub fn new(wake: Wake) -> Self {
        Self {
            state: Rc::new(State {
                hwnd: Cell::new(None),
                initialized: Cell::new(false),
                wake,
            }),
        }
    }

    fn live_hwnd(&self) -> Option<HWND> {
        let hwnd = self.state.hwnd.get()?;
        // WM_NCDESTROY clears the retained identity before releasing the
        // window's Rc. A recycled numeric handle must never be targeted.
        (unsafe { IsWindow(Some(hwnd)).as_bool() }
            && unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) }
                == Rc::as_ptr(&self.state) as isize)
            .then_some(hwnd)
    }

    fn update(&mut self, text: &str, dismiss: &str) -> Result<HWND> {
        let hwnd = if let Some(hwnd) = self.live_hwnd() {
            hwnd
        } else {
            let state = Rc::new(State {
                hwnd: Cell::new(None),
                initialized: Cell::new(false),
                wake: self.state.wake.clone(),
            });
            let template = template();
            // The dialog owns this strong reference until WM_NCDESTROY,
            // including if creating a control fails after WM_INITDIALOG.
            let raw = Rc::into_raw(state.clone());
            let created = unsafe {
                CreateDialogIndirectParamW(
                    None,
                    template.as_ptr().cast(),
                    None,
                    Some(dialog_proc),
                    LPARAM(raw as isize),
                )
            };
            if !state.initialized.get() {
                unsafe { drop(Rc::from_raw(raw)) };
            }
            let hwnd = created?;
            self.state = state;
            hwnd
        };
        unsafe { SetDlgItemTextW(hwnd, TEXT, &HSTRING::from(text))? };
        unsafe { SetDlgItemTextW(hwnd, 1, &HSTRING::from(dismiss))? };
        Ok(hwnd)
    }

    pub fn show(&mut self, text: &str, dismiss: &str) -> Result<()> {
        let hwnd = self.update(text, dismiss)?;
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(hwnd);
        }
        Ok(())
    }

    pub fn translate(&self, message: &MSG) -> bool {
        self.live_hwnd()
            .is_some_and(|hwnd| unsafe { IsDialogMessageW(hwnd, message).as_bool() })
    }

    pub fn close(&self) -> Result<()> {
        if let Some(hwnd) = self.live_hwnd()
            && let Err(error) = unsafe { DestroyWindow(hwnd) }
        {
            self.state.wake.fail();
            return Err(error);
        }
        Ok(())
    }
}

impl Drop for Notice {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match message {
        WM_INITDIALOG => {
            let state = unsafe { &*(lparam.0 as *const State) };
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0) };
            state.hwnd.set(Some(hwnd));
            state.initialized.set(true);
            1
        }
        WM_COMMAND if matches!((wparam.0 & 0xffff) as i32, 1 | 2) => {
            unsafe { dismiss(hwnd) };
            1
        }
        WM_CLOSE => {
            unsafe { dismiss(hwnd) };
            1
        }
        WM_NCDESTROY => {
            let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const State;
            if !raw.is_null() {
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
                let state = unsafe { Rc::from_raw(raw) };
                state.hwnd.set(None);
            }
            0
        }
        _ => 0,
    }
}

unsafe fn dismiss(hwnd: HWND) {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const State;
    // DestroyWindow synchronously releases the dialog's Rc. Retain only the
    // independent fault signal across that call, never the borrowed state.
    let wake = (!raw.is_null()).then(|| unsafe { &*raw }.wake.clone());
    if unsafe { DestroyWindow(hwnd) }.is_err()
        && let Some(wake) = wake
    {
        wake.fail();
    }
}

/// A DWORD-aligned standard dialog template, with native static text and a
/// default dismissal button. Dialog units and the dialog font scale together.
fn template() -> Vec<u32> {
    fn dword(out: &mut Vec<u16>, value: u32) {
        out.extend([value as u16, (value >> 16) as u16]);
    }
    fn string(out: &mut Vec<u16>, value: &str) {
        out.extend(value.encode_utf16().chain(Some(0)));
    }
    fn align(out: &mut Vec<u16>) {
        if !out.len().is_multiple_of(2) {
            out.push(0);
        }
    }
    fn control(out: &mut Vec<u16>, style: u32, rect: [u16; 4], id: u16, class: u16, title: &str) {
        align(out);
        dword(out, style);
        dword(out, 0);
        out.extend(rect);
        out.extend([id, 0xffff, class]);
        string(out, title);
        out.push(0); // No creation data.
    }
    let mut out = Vec::new();
    dword(
        &mut out,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut out, WS_EX_APPWINDOW.0);
    out.extend([2, 20, 20, 280, 100, 0, 0]);
    string(&mut out, "Weftext");
    out.push(9);
    string(&mut out, "Segoe UI");
    control(
        &mut out,
        WS_CHILD.0 | WS_VISIBLE.0, // Static controls default to SS_LEFT.
        [12, 12, 256, 55],
        TEXT as u16,
        0x82,
        "",
    );
    control(
        &mut out,
        WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
        [218, 75, 50, 14],
        1,
        0x80,
        "",
    );
    align(&mut out);
    out.as_chunks::<2>()
        .0
        .iter()
        .map(|words| u32::from(words[0]) | (u32::from(words[1]) << 16))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{
        events::{Event, Queue, Wake},
        window::Window,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;

    #[test]
    fn notice_keeps_author_messages_live_and_has_bounded_identity_and_teardown() {
        std::thread::spawn(|| {
            Window::register().unwrap();
            let (wake, _) = Wake::new().unwrap();
            let queue = Queue::new(wake.clone());
            let author = Window::create(1, "author", queue.clone()).unwrap();
            let author_hwnd = author.state.live_hwnd().unwrap();
            let mut notice = Notice::new(wake.clone());
            // Keep native test windows hidden; product show uses this exact
            // creation/update path before exposing the dialog.
            let original = notice.update("first", "Close").unwrap();
            assert_eq!(notice.update("updated", "关闭").unwrap(), original);
            let mut text = [0; 64];
            unsafe { GetDlgItemTextW(original, TEXT, &mut text) };
            assert_eq!(String::from_utf16_lossy(&text[..7]), "updated");
            while queue.pop().is_some() {}
            unsafe {
                PostMessageW(Some(author_hwnd), WM_SIZE, WPARAM(0), LPARAM(0)).unwrap();
                PostMessageW(Some(author_hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
            }
            let mut seen = 0;
            while seen < 2 {
                let mut message = MSG::default();
                assert!(unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0);
                if message.hwnd == author_hwnd && matches!(message.message, WM_SIZE | WM_CLOSE) {
                    seen += 1;
                }
                if !notice.translate(&message) {
                    unsafe {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
            }
            assert!(matches!(queue.pop(), Some(Event::Resize(_))));
            assert!(matches!(queue.pop(), Some(Event::Close(_))));
            assert_eq!(notice.live_hwnd(), Some(original));
            assert!(unsafe { IsWindowEnabled(author_hwnd).as_bool() });
            let unrelated = MSG {
                hwnd: author_hwnd,
                message: WM_APP + 20,
                ..Default::default()
            };
            assert!(!notice.translate(&unrelated));
            // Native dialog keyboard translation handles Escape and retires
            // the original identity without closing the author window.
            let escape = MSG {
                hwnd: original,
                message: WM_KEYDOWN,
                wParam: WPARAM(0x1b),
                ..Default::default()
            };
            assert!(notice.translate(&escape));
            assert!(notice.live_hwnd().is_none());
            assert!(author.state.live_hwnd().is_some());
            let stale = Notice {
                state: notice.state.clone(),
            };
            notice.update("reopened", "Close").unwrap();
            stale.close().unwrap();
            let reopened = notice.live_hwnd().unwrap();
            unsafe { SendMessageW(reopened, WM_COMMAND, Some(WPARAM(1)), None) };
            assert!(notice.live_hwnd().is_none());
            notice.update("explicit teardown", "Close").unwrap();
            notice.close().unwrap();
            notice.close().unwrap();
            assert!(notice.live_hwnd().is_none());
            notice.update("exit", "Close").unwrap();
            let state = notice.state.clone();
            drop(notice);
            assert!(state.hwnd.get().is_none());
            // Tray/CLI owners may have no author surface at all.
            author.destroy().unwrap();
            let mut notice = Notice::new(wake);
            let first = notice.update("no author", "Close").unwrap();
            assert_eq!(notice.update("no author updated", "Close").unwrap(), first);
            notice.close().unwrap();
            assert!(notice.live_hwnd().is_none());
        })
        .join()
        .unwrap();
    }
}
