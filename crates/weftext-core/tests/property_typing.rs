use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;
use weftext_core::{
    PROPERTY_VALUE_PROFILE_ID, PropertyScalarStyle, PropertyScalarValue, PropertyTypingError,
    classify_property_scalar,
};

#[test]
fn quoted_and_block_scalars_are_always_text() {
    for style in [
        PropertyScalarStyle::SingleQuoted,
        PropertyScalarStyle::DoubleQuoted,
        PropertyScalarStyle::Literal,
        PropertyScalarStyle::Folded,
    ] {
        assert_eq!(
            classify_property_scalar("2026-08-24", style),
            Ok(PropertyScalarValue::Text("2026-08-24".to_owned()))
        );
        assert_eq!(
            classify_property_scalar("true", style),
            Ok(PropertyScalarValue::Text("true".to_owned()))
        );
    }
}

#[test]
fn plain_scalar_types_are_closed_exact_and_bounded() {
    assert_eq!(
        classify_property_scalar("null", PropertyScalarStyle::Plain),
        Ok(PropertyScalarValue::Null)
    );
    assert_eq!(
        classify_property_scalar("false", PropertyScalarStyle::Plain),
        Ok(PropertyScalarValue::Boolean(false))
    );
    assert_eq!(
        classify_property_scalar("-42", PropertyScalarStyle::Plain),
        Ok(PropertyScalarValue::Integer("-42".to_owned()))
    );
    assert_eq!(
        classify_property_scalar("123.4500", PropertyScalarStyle::Plain),
        Ok(PropertyScalarValue::Decimal("123.4500".to_owned()))
    );
    assert_eq!(
        classify_property_scalar("2026-08-24", PropertyScalarStyle::Plain),
        Ok(PropertyScalarValue::Date("2026-08-24".to_owned()))
    );
    assert_eq!(
        classify_property_scalar("2026-08-24T15:30:00+08:00", PropertyScalarStyle::Plain),
        Ok(PropertyScalarValue::Instant(
            "2026-08-24T15:30:00+08:00".to_owned()
        ))
    );
    assert_eq!(
        classify_property_scalar("研究", PropertyScalarStyle::Plain),
        Ok(PropertyScalarValue::Text("研究".to_owned()))
    );
}

#[test]
fn ambiguous_yaml_and_invalid_typed_shapes_fail_closed() {
    for value in ["yes", "NO", "Y", "On", "~", "TRUE", "Null", ".inf", ".NaN"] {
        assert_eq!(
            classify_property_scalar(value, PropertyScalarStyle::Plain),
            Err(PropertyTypingError::AmbiguousKeyword),
            "{value}"
        );
    }
    for value in [
        "01",
        "+1",
        ".5",
        "1e3",
        "9223372036854775808",
        "1.1234567890123456789",
        "123456789012345678901234567890123456789.0",
    ] {
        assert_eq!(
            classify_property_scalar(value, PropertyScalarStyle::Plain),
            Err(PropertyTypingError::InvalidNumber),
            "{value}"
        );
    }
    for value in [
        "2026-02-30",
        "2026-08-24T12:00:00",
        "2026-08-24 12:00:00Z",
        "2026-08-24T25:00:00Z",
    ] {
        assert_eq!(
            classify_property_scalar(value, PropertyScalarStyle::Plain),
            Err(PropertyTypingError::InvalidTemporal),
            "{value}"
        );
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureManifest {
    profile: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCase {
    id: String,
    decoded: String,
    style: PropertyScalarStyle,
    value: Option<PropertyScalarValue>,
    error: Option<PropertyTypingError>,
}

#[test]
fn machine_readable_property_typing_corpus_is_frozen() {
    let root = fixture_root();
    let source = fs::read_to_string(root.join("manifest.json")).expect("fixture manifest");
    let manifest: FixtureManifest = serde_json::from_str(&source).expect("parse fixture manifest");
    assert_eq!(manifest.profile, PROPERTY_VALUE_PROFILE_ID);
    assert!(manifest.cases.len() >= 16);
    for case in manifest.cases {
        let result = classify_property_scalar(&case.decoded, case.style);
        match (case.value, case.error) {
            (Some(expected), None) => assert_eq!(result, Ok(expected), "{}", case.id),
            (None, Some(expected)) => assert_eq!(result, Err(expected), "{}", case.id),
            _ => panic!("fixture {} must declare exactly one outcome", case.id),
        }
    }
}

#[test]
fn property_value_schema_is_closed_and_versioned() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/property-value-v1.schema.json"),
    )
    .expect("property value schema");
    let schema: Value = serde_json::from_str(&source).expect("parse property value schema");
    assert_eq!(
        schema["$id"],
        "https://weftext.org/schemas/property-value-v1.schema.json"
    );
    assert_eq!(schema["oneOf"].as_array().map(Vec::len), Some(7));
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/property-values-v1")
}
