use serde::{Deserialize, Serialize};

use crate::TaskDateTime;

pub const PROPERTY_VALUE_PROFILE_ID: &str = "weftext.property-value.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PropertyScalarValue {
    Null,
    Text(String),
    Integer(String),
    Decimal(String),
    Boolean(bool),
    Date(String),
    Instant(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyTypingError {
    EmptyPlainScalar,
    AmbiguousKeyword,
    InvalidNumber,
    InvalidTemporal,
}

/// Classifies one already-decoded YAML scalar through the portable property v1 rules.
///
/// Quoted and block scalars are always text. Plain scalars recognize only the closed null,
/// boolean, signed-i64, bounded decimal, Gregorian date, and explicit-offset RFC 3339 spellings.
/// YAML-version-dependent keywords and number/date-shaped invalid values fail closed so two
/// clients cannot silently infer different types.
///
/// # Errors
///
/// Returns a typed ambiguity error for an empty plain scalar, a YAML-version-dependent keyword,
/// an invalid/overflowing number shape, or an invalid temporal shape.
pub fn classify_property_scalar(
    decoded: &str,
    style: PropertyScalarStyle,
) -> Result<PropertyScalarValue, PropertyTypingError> {
    if style != PropertyScalarStyle::Plain {
        return Ok(PropertyScalarValue::Text(decoded.to_owned()));
    }
    if decoded.is_empty() {
        return Err(PropertyTypingError::EmptyPlainScalar);
    }
    match decoded {
        "null" => return Ok(PropertyScalarValue::Null),
        "true" => return Ok(PropertyScalarValue::Boolean(true)),
        "false" => return Ok(PropertyScalarValue::Boolean(false)),
        _ => {}
    }
    if ambiguous_keyword(decoded) {
        return Err(PropertyTypingError::AmbiguousKeyword);
    }
    if looks_like_date(decoded) || looks_like_instant(decoded) {
        return match crate::task::parse_task_date_time(decoded) {
            Some(TaskDateTime::Date(value)) => Ok(PropertyScalarValue::Date(value)),
            Some(TaskDateTime::Instant(value)) => Ok(PropertyScalarValue::Instant(value)),
            None => Err(PropertyTypingError::InvalidTemporal),
        };
    }
    if integer_shape(decoded) {
        if !canonical_integer(decoded) {
            return Err(PropertyTypingError::InvalidNumber);
        }
        decoded
            .parse::<i64>()
            .map_err(|_| PropertyTypingError::InvalidNumber)?;
        return Ok(PropertyScalarValue::Integer(decoded.to_owned()));
    }
    if decimal_shape(decoded) {
        return validate_decimal(decoded)
            .then(|| PropertyScalarValue::Decimal(decoded.to_owned()))
            .ok_or(PropertyTypingError::InvalidNumber);
    }
    if looks_like_number(decoded) {
        return Err(PropertyTypingError::InvalidNumber);
    }
    Ok(PropertyScalarValue::Text(decoded.to_owned()))
}

fn ambiguous_keyword(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "null"
            | "true"
            | "false"
            | "yes"
            | "no"
            | "y"
            | "n"
            | "on"
            | "off"
            | "~"
            | ".inf"
            | "+.inf"
            | "-.inf"
            | ".nan"
    )
}

fn looks_like_date(value: &str) -> bool {
    value.is_ascii()
        && value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn looks_like_instant(value: &str) -> bool {
    value.is_ascii()
        && value.len() > 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && matches!(value.as_bytes().get(10), Some(b'T' | b' '))
        && value[..4].bytes().all(|byte| byte.is_ascii_digit())
        && value[5..7].bytes().all(|byte| byte.is_ascii_digit())
        && value[8..10].bytes().all(|byte| byte.is_ascii_digit())
}

fn integer_shape(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn canonical_integer(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    digits == "0" || (!digits.starts_with('0') && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

fn decimal_shape(value: &str) -> bool {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let mut parts = unsigned.split('.');
    let Some(whole) = parts.next() else {
        return false;
    };
    let Some(fraction) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && !whole.is_empty()
        && !fraction.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_decimal(value: &str) -> bool {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let Some((whole, fraction)) = unsigned.split_once('.') else {
        return false;
    };
    let canonical_whole = whole == "0" || !whole.starts_with('0');
    let significant_digits = whole.trim_start_matches('0').len() + fraction.len();
    canonical_whole && fraction.len() <= 18 && (1..=38).contains(&significant_digits)
}

fn looks_like_number(value: &str) -> bool {
    let value = value.strip_prefix(['-', '+']).unwrap_or(value);
    (value
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_digit())
        || (value.starts_with('.') && value.as_bytes().get(1).is_some_and(u8::is_ascii_digit)))
        && value.bytes().all(|byte| {
            byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-' | b'_')
        })
}
