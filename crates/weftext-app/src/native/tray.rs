use super::window::TRAY_MESSAGE;
use weftext_messages::{UiLocale, UiMessage};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, POINT, WPARAM},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::{Error, HSTRING, Result},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Library,
    Capture,
    Reminders,
    Resident,
    OpenOnLaunch,
    LoginStartup,
    Exit,
}
impl Action {
    fn id(self) -> usize {
        match self {
            Self::Library => 0x3001,
            Self::Capture => 0x3002,
            Self::Reminders => 0x3003,
            Self::Resident => 0x3004,
            Self::OpenOnLaunch => 0x3005,
            Self::LoginStartup => 0x3006,
            Self::Exit => 0x3007,
        }
    }
    pub fn decode(id: usize) -> Option<Self> {
        Some(match id {
            0x3001 => Self::Library,
            0x3002 => Self::Capture,
            0x3003 => Self::Reminders,
            0x3004 => Self::Resident,
            0x3005 => Self::OpenOnLaunch,
            0x3006 => Self::LoginStartup,
            0x3007 => Self::Exit,
            _ => return None,
        })
    }
}

struct Icon(HICON);
impl Drop for Icon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

pub(super) struct Tray {
    data: NOTIFYICONDATAW,
    _icon: Icon,
    installed: bool,
}
impl Tray {
    pub fn new(hwnd: HWND) -> Result<Self> {
        let bytes = include_bytes!("../../../../packaging/windows/assets/weftext.ico");
        if bytes.len() < 22 || bytes[..4] != [0, 0, 1, 0] {
            return Err(Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Product icon is not a complete ICO",
            ));
        }
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        if count == 0 || count > 64 || 6 + 16 * count > bytes.len() {
            return Err(Error::from(windows::Win32::Foundation::E_INVALIDARG));
        }
        let entry = &bytes[6..22];
        let length = u32::from_le_bytes(
            entry[8..12]
                .try_into()
                .map_err(|_| Error::from(windows::Win32::Foundation::E_INVALIDARG))?,
        ) as usize;
        let offset = u32::from_le_bytes(
            entry[12..16]
                .try_into()
                .map_err(|_| Error::from(windows::Win32::Foundation::E_INVALIDARG))?,
        ) as usize;
        let pixels = bytes
            .get(
                offset
                    ..offset
                        .checked_add(length)
                        .ok_or_else(|| Error::from(windows::Win32::Foundation::E_INVALIDARG))?,
            )
            .ok_or_else(|| Error::from(windows::Win32::Foundation::E_INVALIDARG))?;
        let icon = Icon(unsafe {
            CreateIconFromResourceEx(pixels, true, 0x00030000, 32, 32, LR_DEFAULTCOLOR)?
        });
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: icon.0,
            ..Default::default()
        };
        for (target, source) in data.szTip.iter_mut().zip("Weftext".encode_utf16()) {
            *target = source;
        }
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let mut tray = Self {
            data,
            _icon: icon,
            installed: false,
        };
        tray.restore()?;
        Ok(tray)
    }
    pub fn restore(&mut self) -> Result<()> {
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) }.as_bool() {
            return Err(Error::from_thread());
        }
        self.installed = true;
        if !unsafe { Shell_NotifyIconW(NIM_SETVERSION, &self.data) }.as_bool() {
            return Err(Error::from_thread());
        }
        Ok(())
    }
    pub fn popup(
        &self,
        locale: UiLocale,
        prefs: weftext_core::DesktopPreferences,
        startup: super::startup::Observation,
        capture_ready: bool,
        cli_service: bool,
    ) -> Result<Option<Action>> {
        let menu = Menu(unsafe { CreatePopupMenu()? });
        if cli_service {
            unsafe {
                AppendMenuW(
                    menu.0,
                    MF_STRING | MF_GRAYED,
                    0,
                    &HSTRING::from(UiMessage::desktop_cli_service_active().render(locale)),
                )?;
            }
        }
        let items = [
            (
                Action::Library,
                UiMessage::desktop_show_library(),
                false,
                true,
            ),
            (
                Action::Capture,
                UiMessage::desktop_capture(),
                false,
                capture_ready,
            ),
            (Action::Reminders, UiMessage::reminder_label(), false, true),
            (
                Action::Resident,
                UiMessage::desktop_resident(),
                prefs.resident,
                true,
            ),
            (
                Action::OpenOnLaunch,
                UiMessage::desktop_open_interface(),
                prefs.open_library_on_launch,
                true,
            ),
            (
                Action::LoginStartup,
                match startup.state {
                    super::startup::State::Unavailable => UiMessage::native_startup_unavailable(),
                    super::startup::State::Unknown => UiMessage::desktop_startup_request_failed(),
                    super::startup::State::DisabledByUser
                    | super::startup::State::DisabledByPolicy => {
                        UiMessage::desktop_startup_disabled()
                    }
                    super::startup::State::EnabledByPolicy => UiMessage::desktop_startup_managed(),
                    _ => UiMessage::desktop_login_startup(),
                },
                prefs.login_startup_requested && startup.enabled(),
                startup.can_request(),
            ),
            (Action::Exit, UiMessage::desktop_exit(), false, true),
        ];
        for (action, label, checked, enabled) in items {
            if !enabled && action == Action::Capture {
                continue;
            } // No Capture command until the real controller is integrated.
            let flags = MF_STRING
                | if checked { MF_CHECKED } else { MF_UNCHECKED }
                | if enabled { MF_ENABLED } else { MF_GRAYED };
            unsafe {
                AppendMenuW(
                    menu.0,
                    flags,
                    action.id(),
                    &HSTRING::from(label.render(locale)),
                )?;
            }
        }
        let mut point = POINT::default();
        unsafe {
            GetCursorPos(&mut point)?;
            let _ = SetForegroundWindow(self.data.hWnd);
        }
        let selected = unsafe {
            TrackPopupMenuEx(
                menu.0,
                (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY).0,
                point.x,
                point.y,
                self.data.hWnd,
                None,
            )
        }
        .0 as usize;
        unsafe {
            PostMessageW(Some(self.data.hWnd), WM_NULL, WPARAM(0), LPARAM(0))?;
        }
        Ok(Action::decode(selected))
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        if self.installed {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
            }
        }
    }
}
