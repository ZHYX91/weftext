//! Typed product messages shared by native hosts, Core error descriptions and WebUI.
//! Author text and original third-party diagnostics are not catalog templates.
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
mod message_template;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidUiInteger;
impl fmt::Display for InvalidUiInteger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ui_integer_out_of_range")
    }
}
impl std::error::Error for InvalidUiInteger {}
impl TryFrom<usize> for UiInteger {
    type Error = InvalidUiInteger;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        i64::try_from(value)
            .ok()
            .and_then(Self::new)
            .ok_or(InvalidUiInteger)
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
    let mut offset = 0;
    for (span, key) in message_template::placeholders(template) {
        output.push_str(&template[offset..span.start]);
        if let Some((_, value)) = params.iter().find(|(name, _)| *name == key) {
            output.push_str(value);
        } else {
            output.push_str(&template[span.clone()]);
        }
        offset = span.end;
    }
    output.push_str(&template[offset..]);
    output
}

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

/// Provenance of an unmodified diagnostic produced outside the product message catalog.
/// Product validation and policy explanations always use `UiMessage` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticOrigin {
    Os,
    Json,
    Typst,
    Network,
    ExternalTool,
    Dependency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawDiagnostic {
    pub origin: DiagnosticOrigin,
    pub text: String,
}

/// A product explanation and separately attributed, original diagnostic evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorDescription {
    pub explanation: UiMessage,
    pub diagnostics: Vec<RawDiagnostic>,
    pub causes: ErrorCauses,
}

/// A bounded flat chain. Causes never contain recovery source or another chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorCause {
    pub code: String,
    pub explanation: UiMessage,
    pub diagnostics: Vec<RawDiagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ErrorCauses(Vec<ErrorCause>);
#[derive(Debug, Clone)]
pub struct CauseBoundary {
    pub outer: ErrorDescription,
    pub code: String,
    pub original: ErrorDescription,
}
impl ErrorCauses {
    pub const MAX: usize = 32;
    pub fn as_slice(&self) -> &[ErrorCause] {
        &self.0
    }
    pub fn one(code: impl Into<String>, explanation: UiMessage) -> Self {
        Self(vec![ErrorCause {
            code: code.into(),
            explanation,
            diagnostics: Vec::new(),
        }])
    }
}
impl<'de> Deserialize<'de> for ErrorCauses {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CausesVisitor;
        impl<'de> serde::de::Visitor<'de> for CausesVisitor {
            type Value = ErrorCauses;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "at most {} flat error causes", ErrorCauses::MAX)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut causes =
                    Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(ErrorCauses::MAX));
                while causes.len() < ErrorCauses::MAX {
                    match sequence.next_element::<ErrorCause>()? {
                        Some(cause) => causes.push(cause),
                        None => return Ok(ErrorCauses(causes)),
                    }
                }
                if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::invalid_length(
                        ErrorCauses::MAX + 1,
                        &self,
                    ));
                }
                Ok(ErrorCauses(causes))
            }
        }
        deserializer.deserialize_seq(CausesVisitor)
    }
}

impl ErrorDescription {
    pub fn new(explanation: UiMessage) -> Self {
        Self {
            explanation,
            diagnostics: Vec::new(),
            causes: ErrorCauses::default(),
        }
    }

    pub fn diagnostic(mut self, origin: DiagnosticOrigin, text: impl Into<String>) -> Self {
        self.diagnostics.push(RawDiagnostic {
            origin,
            text: text.into(),
        });
        self
    }

    /// Used only after an owning disclosure boundary has approved the product parameters.
    pub fn without_raw_diagnostics(mut self) -> Self {
        self.diagnostics.clear();
        for cause in &mut self.causes.0 {
            cause.diagnostics.clear();
        }
        self
    }

    pub fn try_caused_by(
        mut self,
        code: impl Into<String>,
        cause: ErrorDescription,
    ) -> Result<Self, CauseBoundary> {
        let code = code.into();
        if self.causes.0.len() + 1 + cause.causes.0.len() > ErrorCauses::MAX {
            return Err(CauseBoundary {
                outer: self,
                code,
                original: cause,
            });
        }
        self.causes.0.push(ErrorCause {
            code,
            explanation: cause.explanation,
            diagnostics: cause.diagnostics,
        });
        self.causes.0.extend(cause.causes.0);
        Ok(self)
    }

    pub fn render(&self, locale: UiLocale) -> String {
        use fmt::Write;
        let mut output = self.explanation.render(locale);
        for diagnostic in &self.diagnostics {
            let _ = write!(output, "\n{:?}: {}", diagnostic.origin, diagnostic.text);
        }
        for cause in self.causes.as_slice() {
            let _ = write!(
                output,
                "\n{}: {}",
                cause.code,
                cause.explanation.render(locale)
            );
            for diagnostic in &cause.diagnostics {
                let _ = write!(output, "\n{:?}: {}", diagnostic.origin, diagnostic.text);
            }
        }
        output
    }
}

impl From<UiMessage> for ErrorDescription {
    fn from(explanation: UiMessage) -> Self {
        Self::new(explanation)
    }
}

impl fmt::Display for ErrorDescription {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render(UiLocale::default()))
    }
}
impl std::error::Error for ErrorDescription {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_templates_match_the_web_placeholder_grammar_in_code_examples() {
        let example =
            "{array,binding} {\"limit\":10} \\text{…} \\end{{p1}} {p1} {not_declared} {p2}";
        assert_eq!(
            message_template::placeholders(example)
                .map(|(_, name)| name)
                .collect::<Vec<_>>(),
            vec!["p1", "p1", "p2"]
        );
        assert_eq!(
            render_template(example, &[("p1", "matrix".into()), ("p2", "{p1}".into())]),
            "{array,binding} {\"limit\":10} \\text{…} \\end{matrix} matrix {not_declared} {p1}"
        );
    }

    #[test]
    fn error_wire_is_closed_and_diagnostics_keep_their_original_bytes() {
        let original = ErrorDescription::new(UiMessage::CommonSave {})
            .diagnostic(DiagnosticOrigin::Typst, "raw 中文 <source>\n{count}\0😀");
        let value = serde_json::to_value(&original).unwrap();
        assert_eq!(value["causes"], serde_json::json!([]));
        assert_eq!(
            serde_json::from_value::<ErrorDescription>(value.clone()).unwrap(),
            original
        );
        for missing in ["explanation", "diagnostics", "causes"] {
            let mut malformed = value.clone();
            malformed.as_object_mut().unwrap().remove(missing);
            assert!(serde_json::from_value::<ErrorDescription>(malformed).is_err());
        }
        for malformed in [
            serde_json::json!({"explanation":{"key":"common.save","params":{}},"diagnostics":[],"causes":[],"message":"legacy"}),
            serde_json::json!({"explanation":{"key":"common.save","params":{}},"diagnostics":[{"origin":"product","text":"own text"}],"causes":[]}),
            serde_json::json!({"explanation":{"key":"common.save","params":{}},"diagnostics":[{"origin":"os","text":"original","extra":true}],"causes":[]}),
        ] {
            assert!(serde_json::from_value::<ErrorDescription>(malformed).is_err());
        }
        for locale in [UiLocale::EnUs, UiLocale::ZhCn] {
            assert!(
                original
                    .render(locale)
                    .contains("raw 中文 <source>\n{count}\0😀")
            );
            assert_eq!(serde_json::to_value(&original).unwrap(), value);
        }
    }

    #[test]
    fn full_cause_chain_rejects_extension_without_losing_either_description() {
        let mut full = ErrorDescription::new(UiMessage::CommonSave {});
        for index in 0..ErrorCauses::MAX {
            full = full
                .try_caused_by(
                    format!("cause{index}"),
                    ErrorDescription::new(UiMessage::CommonOperation {
                        operation_id: index.to_string(),
                    })
                    .diagnostic(DiagnosticOrigin::Dependency, format!("original {index}")),
                )
                .unwrap();
        }
        let value = serde_json::to_value(&full).unwrap();
        assert_eq!(
            serde_json::from_value::<ErrorDescription>(value.clone()).unwrap(),
            full
        );
        let outer = ErrorDescription::new(UiMessage::CommonOperation {
            operation_id: "outer".into(),
        });
        let boundary = outer
            .clone()
            .try_caused_by("originalCode", full.clone())
            .unwrap_err();
        assert_eq!(boundary.outer, outer);
        assert_eq!(boundary.code, "originalCode");
        assert_eq!(boundary.original, full);
        let mut too_long = value;
        let duplicate = too_long["causes"][0].clone();
        too_long["causes"].as_array_mut().unwrap().push(duplicate);
        assert!(serde_json::from_value::<ErrorDescription>(too_long).is_err());
        let stripped = full.clone().without_raw_diagnostics();
        assert_eq!(stripped.causes.as_slice().len(), ErrorCauses::MAX);
        for (original, stripped) in full
            .causes
            .as_slice()
            .iter()
            .zip(stripped.causes.as_slice())
        {
            assert_eq!(original.code, stripped.code);
            assert_eq!(original.explanation, stripped.explanation);
            assert!(stripped.diagnostics.is_empty());
        }
    }

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
