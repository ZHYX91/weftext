//! Typed product messages shared by native hosts, Core error descriptions and WebUI.
//! Author text and original third-party diagnostics are not catalog templates.
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiLocale {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

impl UiLocale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::EnUs => "en-US",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidUiLocale;
impl fmt::Display for InvalidUiLocale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Unsupported interface language tag")
    }
}
impl std::error::Error for InvalidUiLocale {}
impl FromStr for UiLocale {
    type Err = InvalidUiLocale;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "zh-CN" => Ok(Self::ZhCn),
            "en-US" => Ok(Self::EnUs),
            _ => Err(InvalidUiLocale),
        }
    }
}

/// Product-message numbers share JavaScript's exact integer domain; author numbers are separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiInteger(i64);
impl UiInteger {
    pub const MAX: i64 = 9_007_199_254_740_991;
    pub const fn new(value: i64) -> Option<Self> {
        if value >= -Self::MAX && value <= Self::MAX {
            Some(Self(value))
        } else {
            None
        }
    }
}
impl fmt::Display for UiInteger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
impl Serialize for UiInteger {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.0)
    }
}
impl<'de> Deserialize<'de> for UiInteger {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IntegerVisitor;
        impl serde::de::Visitor<'_> for IntegerVisitor {
            type Value = UiInteger;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JavaScript-safe integer")
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<UiInteger, E> {
                UiInteger::new(value).ok_or_else(|| {
                    E::custom("Product-message integer is outside the exact wire domain")
                })
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<UiInteger, E> {
                i64::try_from(value)
                    .ok()
                    .and_then(UiInteger::new)
                    .ok_or_else(|| {
                        E::custom("Product-message integer is outside the exact wire domain")
                    })
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<UiInteger, E> {
                if value.is_finite()
                    && value.fract() == 0.0
                    && value >= -(UiInteger::MAX as f64)
                    && value <= UiInteger::MAX as f64
                {
                    Ok(UiInteger(value as i64))
                } else {
                    Err(E::custom(
                        "Product-message number is not an exact safe integer",
                    ))
                }
            }
        }
        deserializer.deserialize_any(IntegerVisitor)
    }
}

fn render_template(template: &str, params: &[(&str, String)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut remainder = template;
    while let Some(start) = remainder.find('{') {
        output.push_str(&remainder[..start]);
        remainder = &remainder[start..];
        let Some(end) = remainder.find('}') else {
            output.push_str(remainder);
            return output;
        };
        let key = &remainder[1..end];
        if let Some((_, value)) = params.iter().find(|(name, _)| *name == key) {
            output.push_str(value);
        } else {
            output.push_str(&remainder[..=end]);
        }
        remainder = &remainder[end + 1..];
    }
    output.push_str(remainder);
    output
}

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_and_web_message_share_exact_identity_and_params() {
        let message = UiMessage::CommonOperation {
            operation_id: "original<{count}>😀".into(),
        };
        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            serde_json::json!({"key":"common.operation","params":{"operationId":"original<{count}>😀"}})
        );
        assert_eq!(
            message.render(UiLocale::EnUs),
            "Original operation: original<{count}>😀"
        );
        assert_eq!(
            message.render(UiLocale::ZhCn),
            "原操作：original<{count}>😀"
        );
    }

    #[test]
    fn message_wire_rejects_unknown_keys_params_and_locales() {
        for value in [
            serde_json::json!({"key":"not.aMessage","params":{}}),
            serde_json::json!({"key":"common.save","params":{"source":"protected"}}),
            serde_json::json!({"key":"common.operation","params":{}}),
        ] {
            assert!(serde_json::from_value::<UiMessage>(value).is_err());
        }
        assert!(serde_json::from_str::<UiLocale>("\"en\"").is_err());
        assert_eq!(UiLocale::default(), UiLocale::ZhCn);
        assert_eq!("en-US".parse::<UiLocale>().unwrap(), UiLocale::EnUs);
        assert!("en".parse::<UiLocale>().is_err());
    }

    #[test]
    fn unicode_and_integer_wire_domain_matches_web_messages() {
        for invalid in [
            r#"{"key":"common.operation","params":{"operationId":"\ud800"}}"#,
            r#"{"key":"reminder.missed","params":{"count":9007199254740993}}"#,
            r#"{"key":"reminder.missed","params":{"count":1.5}}"#,
        ] {
            assert!(serde_json::from_str::<UiMessage>(invalid).is_err());
        }
        for value in ["9007199254740991", "-9007199254740991", "400.0", "4e2"] {
            let message: UiMessage = serde_json::from_str(&format!(
                r#"{{"key":"reminder.missed","params":{{"count":{value}}}}}"#
            ))
            .unwrap();
            assert_eq!(
                serde_json::from_str::<UiMessage>(&serde_json::to_string(&message).unwrap())
                    .unwrap(),
                message
            );
        }
        assert!(UiInteger::new(i64::MAX).is_none());
    }
}
