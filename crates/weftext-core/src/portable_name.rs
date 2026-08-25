use caseless::Caseless;
use unicode_normalization::UnicodeNormalization;

pub const MAX_PORTABLE_NODE_NAME_BYTES: usize = 120;

/// Derives a portable node-name candidate from a parser-visible document title.
///
/// The result is only a suggestion. Callers must still run the normal Core
/// planning path, which validates the reviewed name and checks workspace
/// occupancy without truncating or silently adding a suffix.
#[must_use]
pub fn suggest_portable_node_name(title: &str) -> Option<String> {
    let normalized = title.nfc().collect::<String>();
    let trimmed = normalized
        .trim_matches(char::is_whitespace)
        .trim_end_matches('.');

    if is_reserved_storage_name(trimmed, false) || is_windows_device_name(trimmed) {
        return None;
    }

    let suggestion = portable_equivalent(trimmed);
    validate_portable_node_name(&suggestion, false).ok()?;
    Some(suggestion)
}

pub(crate) fn validate_portable_node_name(
    name: &str,
    allow_reserved_trash: bool,
) -> Result<(), &'static str> {
    validate_portable_name_component(name, allow_reserved_trash)?;
    if is_reserved_storage_name(name, allow_reserved_trash) {
        return Err("node name is reserved for Weftext storage");
    }
    Ok(())
}

pub(crate) fn validate_portable_name_component(
    name: &str,
    allow_reserved_trash: bool,
) -> Result<(), &'static str> {
    if name.is_empty() || matches!(name, "." | "..") {
        return Err("node name is empty or reserved");
    }
    if name.len() > MAX_PORTABLE_NODE_NAME_BYTES {
        return Err("node name exceeds 120 UTF-8 bytes");
    }
    if name.trim_matches(char::is_whitespace) != name || name.ends_with('.') {
        return Err("node name cannot start/end with whitespace or end with a dot");
    }
    if !allow_reserved_trash && portable_casefold(name) == ".weftext-trash" {
        return Err(".weftext-trash is reserved for Workspace Trash");
    }
    if name.chars().any(is_forbidden_name_character) {
        return Err("node name contains a non-portable character");
    }
    if is_windows_device_name(name) {
        return Err("node name is reserved on Windows");
    }
    Ok(())
}

/// Returns the platform-independent equality key used for occupancy checks.
///
/// This stays crate-visible until a reviewed transaction consumes it; exposing
/// a collision key alone must not imply that a caller has mutation authority.
#[allow(dead_code, reason = "reserved for reviewed workspace occupancy checks")]
pub(crate) fn portable_name_collision_key(name: &str) -> String {
    let normalized = name.nfc().collect::<String>();
    let folded = normalized.chars().default_case_fold().collect::<String>();
    let recomposed = folded.nfc().collect::<String>();
    portable_equivalent(&recomposed)
}

fn portable_equivalent(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut pending_separator = false;

    for character in value.chars() {
        if character == '-' || character.is_whitespace() || is_forbidden_name_character(character) {
            pending_separator = !result.is_empty();
            continue;
        }
        if pending_separator {
            result.push('-');
            pending_separator = false;
        }
        result.push(character);
    }

    result
        .trim_matches(|character: char| {
            character == '-' || character == '.' || character.is_whitespace()
        })
        .to_owned()
}

fn is_forbidden_name_character(character: char) -> bool {
    character.is_control() || "\\/:*?\"<>|".contains(character) || is_bidi_control(character)
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{206f}'
    )
}

fn is_windows_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    let stem = portable_casefold(stem);
    if matches!(stem.as_str(), "con" | "prn" | "aux" | "nul") {
        return true;
    }
    let bytes = stem.as_bytes();
    bytes.len() == 4
        && (bytes[..3] == *b"com" || bytes[..3] == *b"lpt")
        && matches!(bytes[3], b'1'..=b'9')
}

fn is_reserved_storage_name(name: &str, allow_reserved_trash: bool) -> bool {
    let folded = portable_casefold(name);
    if allow_reserved_trash && folded == ".weftext-trash" {
        return false;
    }
    folded == ".git"
        || folded.starts_with(".weftext-")
        || folded == "_weftext"
        || folded.starts_with("_weftext.")
        || folded == "weftext.annotations.json"
        || folded.starts_with(".__weftext-transaction-")
        || folded.starts_with(".__weftext-resource-")
}

fn portable_casefold(value: &str) -> String {
    value
        .nfc()
        .collect::<String>()
        .chars()
        .default_case_fold()
        .collect::<String>()
        .nfc()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{portable_name_collision_key, suggest_portable_node_name};

    #[test]
    fn collision_key_uses_full_unicode_default_case_folding() {
        assert_eq!(
            portable_name_collision_key("Straße"),
            portable_name_collision_key("STRASSE")
        );
        for sigma in ["Σ", "σ", "ς"] {
            assert_eq!(portable_name_collision_key(sigma), "σ");
        }
    }

    #[test]
    fn collision_key_composes_canonical_equivalents() {
        assert_eq!(
            portable_name_collision_key("Café"),
            portable_name_collision_key("Cafe\u{301}")
        );
    }

    #[test]
    fn collision_key_applies_portable_separator_equivalence() {
        assert_eq!(
            portable_name_collision_key("  A\u{a0}/--B...  "),
            portable_name_collision_key("a-b")
        );
    }

    #[test]
    fn collision_key_preserves_uncased_unicode_and_emoji() {
        assert_eq!(portable_name_collision_key("中文🧑‍💻"), "中文🧑‍💻");
    }

    #[test]
    fn suggestion_preserves_internal_dots_and_replaces_bidi_controls() {
        assert_eq!(
            suggest_portable_node_name("  发布.计划\u{202e}\u{206f}/第二版.  "),
            Some("发布.计划-第二版".to_owned())
        );
    }

    #[test]
    fn reserved_names_use_nfc_and_unicode_default_casefold() {
        assert_eq!(suggest_portable_node_name(".weftext-traſh"), None);
        assert_eq!(suggest_portable_node_name("weftext.annotationſ.json"), None);
        assert_eq!(suggest_portable_node_name(".__weftext-reſource-item"), None);
    }
}
