//! Opt-in test diagnostics only. These records are never capabilities or facts.
use crate::{CoreError, backend::FileStamp, hash};
use std::io::Write;

pub(crate) fn enabled() -> bool {
    enabled_value(std::env::var_os("WEFTEXT_TEST_RESOURCE_DIAGNOSTICS").as_deref())
}

fn enabled_value(value: Option<&std::ffi::OsStr>) -> bool {
    value == Some(std::ffi::OsStr::new("1"))
}

fn os_error(message: &str) -> Option<i32> {
    let tail = message.rsplit_once("(os error ")?.1;
    tail.strip_suffix(')')?.parse().ok()
}

fn record(phase: &str, token: &str, code: &str, os: Option<i32>) -> serde_json::Value {
    let safe = |text: &str| {
        text.chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
            .take(96)
            .collect::<String>()
    };
    serde_json::json!({"phase":safe(phase),"id":&hash(token.as_bytes())[..16],"code":safe(code),"osError":os,"pid":std::process::id()})
}

fn emit(phase: &str, token: &str, code: &str, os: Option<i32>) {
    if !enabled() {
        return;
    }
    let line = record(phase, token, code, os);
    let _ = writeln!(
        std::io::stderr().lock(),
        "weftext-resource-diagnostic {line}"
    );
}

pub(crate) fn error(phase: &str, token: &str, error: &CoreError) {
    emit(phase, token, &error.code, os_error(&error.message));
}

pub(crate) fn io_error(phase: &str, token: &str, error: &std::io::Error) {
    emit(phase, token, "io", error.raw_os_error());
}

pub(crate) fn branch(phase: &str, token: &str, code: &str) {
    emit(phase, token, code, None);
}

pub(crate) fn stamp_token(stamp: &FileStamp) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        stamp.identity.volume,
        stamp.identity.index,
        stamp.identity.created,
        stamp.modified,
        stamp.size
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_diagnostic_gate_is_off_unless_exactly_one_and_payloads_are_redacted() {
        for value in [
            None,
            Some(std::ffi::OsStr::new("0")),
            Some(std::ffi::OsStr::new("true")),
        ] {
            assert!(!enabled_value(value));
        }
        assert!(enabled_value(Some(std::ffi::OsStr::new("1"))));
        let secret = r"C:\private\note.typ bearer private-body";
        let output = record("collection.qualify", secret, "busy", Some(32)).to_string();
        assert!(!output.contains("private"));
        assert!(!output.contains("bearer"));
        assert!(!output.contains("note.typ"));
        assert_eq!(
            record("collection.qualify", secret, "busy", Some(32))["id"]
                .as_str()
                .unwrap()
                .len(),
            16
        );
        assert_eq!(
            os_error("Windows file protection failed: text (os error 32)"),
            Some(32)
        );
        assert_eq!(os_error("private message without an OS error"), None);
    }
}
