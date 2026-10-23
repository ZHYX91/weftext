//! Narrow noninteractive Windows binding; no settings, registry or credentials
//! are changed. The seal is an installation/profile witness, not an auth token.
use super::*;
use std::ffi::c_void;
use weftext_messages::UiMessage;

#[repr(C)]
struct Blob {
    length: u32,
    data: *mut u8,
}
#[repr(C)]
struct SidAndAttributes {
    sid: *mut c_void,
    attributes: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
    fn GetTokenInformation(
        token: *mut c_void,
        class: u32,
        buffer: *mut c_void,
        length: u32,
        required: *mut u32,
    ) -> i32;
    fn ConvertSidToStringSidW(sid: *const c_void, text: *mut *mut u16) -> i32;
}
#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        input: *const Blob,
        description: *const u16,
        entropy: *const Blob,
        reserved: *const c_void,
        prompt: *const c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
    fn CryptUnprotectData(
        input: *const Blob,
        description: *mut *mut u16,
        entropy: *const Blob,
        reserved: *const c_void,
        prompt: *const c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
}

struct Token(*mut c_void);
impl Drop for Token {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}

pub(super) fn current_user_sid() -> Result<String> {
    let mut token = std::ptr::null_mut();
    // TOKEN_QUERY only. This neither impersonates nor changes token privileges.
    if unsafe { OpenProcessToken(GetCurrentProcess(), 8, &mut token) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let token = Token(token);
    let mut length = 0;
    unsafe {
        GetTokenInformation(token.0, 1, std::ptr::null_mut(), 0, &mut length);
    }
    if length < std::mem::size_of::<SidAndAttributes>() as u32 || length > 64 * 1024 {
        return Err(evidence(
            UiMessage::error_core_device_state_windows_unexpected_token_user_byte_length(),
        ));
    }
    let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
    if unsafe { GetTokenInformation(token.0, 1, buffer.as_mut_ptr().cast(), length, &mut length) }
        == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    // TokenUser begins with a correctly aligned SID_AND_ATTRIBUTES structure.
    let sid = unsafe { &*buffer.as_ptr().cast::<SidAndAttributes>() }.sid;
    let mut text = std::ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let _allocation = LocalAllocation(text.cast());
    let mut end = 0;
    while end < 256 && unsafe { *text.add(end) } != 0 {
        end += 1;
    }
    if end == 256 {
        return Err(evidence(
            UiMessage::error_core_device_state_windows_unexpected_windows_s_i_d_length(),
        ));
    }
    String::from_utf16(unsafe { std::slice::from_raw_parts(text, end) }).map_err(|_| {
        evidence(UiMessage::error_core_device_state_windows_windows_s_i_d_text_not_valid_u_t_f16())
    })
}

pub(super) fn seal(bytes: &[u8]) -> Result<Vec<u8>> {
    let input = Blob {
        length: u32::try_from(bytes.len()).map_err(|_| {
            evidence(UiMessage::error_core_device_state_windows_binding_byte_budget_exceeded())
        })?,
        data: bytes.as_ptr().cast_mut(),
    };
    let mut output = Blob {
        length: 0,
        data: std::ptr::null_mut(),
    };
    // LOCAL_MACHINE prevents roaming-user protection from treating another
    // installation as current. UI_FORBIDDEN + null prompt never asks for input.
    if unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            4 | 1,
            &mut output,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    copy_output(output)
}

pub(super) fn unseal(bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    let input = Blob {
        length: u32::try_from(bytes.len()).map_err(|_| {
            evidence(UiMessage::error_core_device_state_windows_binding_byte_budget_exceeded())
        })?,
        data: bytes.as_ptr().cast_mut(),
    };
    let mut output = Blob {
        length: 0,
        data: std::ptr::null_mut(),
    };
    if unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            1,
            &mut output,
        )
    } == 0
    {
        // Foreign, damaged or otherwise unavailable protection is never adopted.
        // The original record and its ID remain intact; do not create another.
        return Ok(None);
    }
    copy_output(output).map(Some)
}

fn copy_output(output: Blob) -> Result<Vec<u8>> {
    let _allocation = LocalAllocation(output.data.cast());
    if output.data.is_null() || output.length == 0 || output.length > 64 * 1024 {
        return Err(evidence(
            UiMessage::error_core_device_state_windows_unexpected_protected_binding_length(),
        ));
    }
    Ok(unsafe { std::slice::from_raw_parts(output.data, output.length as usize) }.to_vec())
}
