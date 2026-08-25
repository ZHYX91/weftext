use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{ImportError, ImportErrorCode};

const MAX_PORTABLE_PATH_BYTES: usize = 512;
const MAX_COMPONENT_BYTES: usize = 120;
const MAX_COMPONENTS: usize = 32;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PortablePath(String);

impl PortablePath {
    /// Parses a relative, cross-platform-safe portable locator.
    ///
    /// # Errors
    ///
    /// Returns an error for absolute, escaping, reserved-device, malformed, or
    /// over-limit paths.
    pub fn parse(value: impl Into<String>) -> Result<Self, ImportError> {
        let value = value.into();
        validate_portable_path(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }

    /// Appends one component and validates the resulting portable locator.
    ///
    /// # Errors
    ///
    /// Returns an error when the appended value makes the path unsafe.
    pub fn join(&self, component: &str) -> Result<Self, ImportError> {
        Self::parse(format!("{}/{component}", self.0))
    }
}

impl fmt::Display for PortablePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for PortablePath {
    type Error = ImportError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<PortablePath> for String {
    fn from(value: PortablePath) -> Self {
        value.0
    }
}

impl Serialize for PortablePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PortablePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

fn validate_portable_path(value: &str) -> Result<(), ImportError> {
    if value.is_empty() || value.len() > MAX_PORTABLE_PATH_BYTES {
        return invalid_path("portable paths must contain 1 through 512 UTF-8 bytes");
    }
    if value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains('\0')
        || has_windows_drive_prefix(value)
    {
        return invalid_path("absolute, drive-prefixed, backslash, and NUL paths are forbidden");
    }

    let mut count = 0_usize;
    for component in value.split('/') {
        count += 1;
        if component.is_empty() || matches!(component, "." | "..") {
            return invalid_path("empty, dot, and parent path components are forbidden");
        }
        if component.len() > MAX_COMPONENT_BYTES
            || component.ends_with(' ')
            || component.ends_with('.')
            || component.chars().any(char::is_control)
            || component.contains([':', '*', '?', '"', '<', '>', '|'])
        {
            return invalid_path("a portable path component is invalid or too long");
        }
        if is_windows_device_name(component) {
            return invalid_path("Windows device names are forbidden in portable paths");
        }
    }
    if count > MAX_COMPONENTS {
        return invalid_path("portable paths may contain at most 32 components");
    }
    Ok(())
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_windows_device_name(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit()
            && upper.as_bytes()[3] != b'0')
}

fn invalid_path<T>(message: &str) -> Result<T, ImportError> {
    Err(ImportError::new(ImportErrorCode::InvalidPath, message))
}

#[cfg(test)]
mod tests {
    use super::PortablePath;

    #[test]
    fn rejects_absolute_parent_drive_and_windows_device_paths() {
        for malicious in [
            "../escape",
            "safe/../../escape",
            "/absolute",
            "C:/windows",
            "safe\\escape",
            "safe//escape",
            "CON/file.txt",
            "safe/NUL.txt",
            "safe/trailing. ",
        ] {
            assert!(
                PortablePath::parse(malicious).is_err(),
                "accepted malicious path: {malicious}"
            );
        }
    }

    #[test]
    fn accepts_portable_cjk_resource_paths() {
        let path = PortablePath::parse("章节/插图/示意图-一.png").expect("portable CJK path");
        assert_eq!(path.file_name(), "示意图-一.png");
    }
}
