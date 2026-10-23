//! Opt-in test diagnostics only. These records are never capabilities or facts.
use crate::{CoreError, backend::FileStamp, hash};
use std::io::Write;

pub(crate) fn enabled() -> bool {
    enabled_value(std::env::var_os("WEFTEXT_TEST_RESOURCE_DIAGNOSTICS").as_deref())
}

fn enabled_value(value: Option<&std::ffi::OsStr>) -> bool {
    value == Some(std::ffi::OsStr::new("1"))
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
    emit(phase, token, &error.code, error.os_error());
}

pub(crate) fn branch(phase: &str, token: &str, code: &str) {
    emit(phase, token, code, None);
}

pub(crate) fn stamp_token(stamp: &FileStamp) -> String {
    format!("{:?}:{}:{}", stamp.identity, stamp.modified, stamp.size)
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
        let original = std::io::Error::from_raw_os_error(32);
        let error = CoreError::from(original);
        assert_eq!(error.os_error(), Some(32));
    }
}
