//! One request boundary: framework status/provenance plus strict original JSON.
//! Successful responses continue using axum::Json directly.
use crate::{ApiError, DiagnosticOrigin, StatusCode, UiMessage};
use axum::{
    body::Bytes,
    extract::{FromRequest, Request, rejection::JsonRejection},
    http::{HeaderMap, header},
};
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub(crate) struct RequestJson<T>(pub(crate) T);

impl<T, S> FromRequest<S> for RequestJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Keep the framework's media-type precedence before reading any body.
        if !json_content_type(request.headers()) {
            let error = axum::Json::<T>::from_request(request, state)
                .await
                .err()
                .expect("the framework rejects a missing JSON media type before reading the body");
            return Err(rejection(error));
        }
        // Bytes honors the router's original DefaultBodyLimit while streaming.
        let bytes = Bytes::from_request(request, state)
            .await
            .map_err(|error| rejection(JsonRejection::from(error)))?;
        // The original bytes, rather than an already-decoded Value, establish
        // uniqueness even for generic or nested JSON. Nothing reaches a handler
        // before this check succeeds. Reuse the sole Data parser and its typed
        // policy diagnostics; do not turn owned duplicate failures into raw text.
        if let Err(error) = weftext_data::parse_unique_json(&bytes) {
            // Preserve actual framework syntax diagnostics without invoking a
            // request type's deserializer on an ambiguous object.
            let _ = axum::Json::<serde_json::Value>::from_bytes(&bytes).map_err(rejection)?;
            return Err(ApiError::from_data(
                StatusCode::BAD_REQUEST,
                "invalid_json",
                error,
            ));
        }
        let axum::Json(value) = axum::Json::<T>::from_bytes(&bytes).map_err(rejection)?;
        Ok(Self(value))
    }
}

fn json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<mime::Mime>().ok())
        .is_some_and(|value| {
            value.type_() == mime::APPLICATION
                && (value.subtype() == mime::JSON || value.suffix() == Some(mime::JSON))
        })
}

fn rejection(error: JsonRejection) -> ApiError {
    let status = error.status();
    let (code, explanation, origin) = match &error {
        JsonRejection::JsonDataError(_) => (
            "invalid_request",
            UiMessage::error_api_json_data_invalid(),
            DiagnosticOrigin::Json,
        ),
        JsonRejection::JsonSyntaxError(_) => (
            "invalid_json",
            UiMessage::error_api_json_syntax_invalid(),
            DiagnosticOrigin::Json,
        ),
        JsonRejection::MissingJsonContentType(_) => (
            "json_content_type",
            UiMessage::error_api_json_content_type_required(),
            DiagnosticOrigin::Dependency,
        ),
        JsonRejection::BytesRejection(_) if status == StatusCode::PAYLOAD_TOO_LARGE => (
            "request_budget",
            UiMessage::error_api_request_body_budget(),
            DiagnosticOrigin::Dependency,
        ),
        JsonRejection::BytesRejection(_) => (
            "request_body",
            UiMessage::error_api_request_body_unavailable(),
            DiagnosticOrigin::Dependency,
        ),
        _ => (
            "request_body",
            UiMessage::error_api_request_body_unavailable(),
            DiagnosticOrigin::Dependency,
        ),
    };
    ApiError::new(status, code, explanation).diagnostic(origin, error.body_text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, response::IntoResponse};
    use serde::Deserialize;
    use serde_json::{Value, json};
    use weftext_messages::UiLocale;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Preference {
        ui_locale: UiLocale,
    }
    fn request(body: Vec<u8>, content_type: Option<&str>) -> Request {
        let mut request = Request::builder();
        if let Some(content_type) = content_type {
            request = request.header(header::CONTENT_TYPE, content_type);
        }
        request.body(Body::from(body)).unwrap()
    }

    #[tokio::test]
    async fn framework_rejections_keep_original_status_and_diagnostic_inside_typed_wire() {
        for (body, content_type, expected) in [
            (
                br#"{"uiLocale":"en"}"#.to_vec(),
                Some("application/json"),
                UiMessage::error_api_json_data_invalid(),
            ),
            (
                br#"{}"#.to_vec(),
                Some("application/json"),
                UiMessage::error_api_json_data_invalid(),
            ),
            (
                br#"{"uiLocale":"#.to_vec(),
                Some("application/json"),
                UiMessage::error_api_json_syntax_invalid(),
            ),
            (
                vec![b' '; 2 * 1024 * 1024 + 1],
                None,
                UiMessage::error_api_json_content_type_required(),
            ),
            (
                vec![b' '; 2 * 1024 * 1024 + 1],
                Some("application/json"),
                UiMessage::error_api_request_body_budget(),
            ),
        ] {
            let original =
                axum::Json::<Preference>::from_request(request(body.clone(), content_type), &())
                    .await
                    .unwrap_err();
            // The duplicate-guard parses raw JSON before the request type. For
            // a syntax failure this preserves the framework's neutral parser
            // diagnostic, not a type-specific field path inferred after the
            // original bytes already failed admission.
            let syntax_guard =
                (expected == UiMessage::error_api_json_syntax_invalid()).then(|| body.clone());
            let error = RequestJson::<Preference>::from_request(request(body, content_type), &())
                .await
                .unwrap_err();
            assert_eq!(error.status, original.status());
            assert_eq!(error.explanation, expected);
            assert_eq!(error.diagnostics.len(), 1);
            let framework_diagnostic = if let Some(bytes) = syntax_guard {
                axum::Json::<serde::de::IgnoredAny>::from_request(request(bytes, content_type), &())
                    .await
                    .unwrap_err()
                    .body_text()
            } else {
                original.body_text()
            };
            assert_eq!(error.diagnostics[0].text, framework_diagnostic);
            let response = error.into_response();
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
            let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024)
                .await
                .unwrap();
            let wire: Value = serde_json::from_slice(&bytes).unwrap();
            assert!(wire["error"].get("message").is_none());
            assert_eq!(
                wire["error"]["explanation"],
                serde_json::to_value(expected).unwrap()
            );
            assert!(wire["error"]["causes"].is_array());
        }
    }

    #[tokio::test]
    async fn generic_nested_duplicates_are_checked_against_original_bytes_before_handler_delivery()
    {
        for bytes in [
            br#"{"outer":{"member":1,"member":2}}"#.as_slice(),
            br#"{"member":1,"member":2}"#.as_slice(),
        ] {
            // The ordinary generic extractor accepts these and loses one member.
            assert!(axum::Json::<Value>::from_bytes(bytes).is_ok());
            let error = RequestJson::<Value>::from_request(
                request(bytes.to_vec(), Some("application/json")),
                &(),
            )
            .await
            .unwrap_err();
            assert_eq!(error.status, StatusCode::BAD_REQUEST);
            assert_eq!(error.code, "invalid_json");
            assert!(error.diagnostics.is_empty());
            let wire = serde_json::to_value(error).unwrap();
            assert_eq!(
                wire["details"]["details"]["groups"][0]["classification"],
                "invalid"
            );
            assert_eq!(
                wire["explanation"]["key"],
                "error.data.schema.jsonKeyDuplicate"
            );
            assert_eq!(wire["explanation"]["params"], json!({"key":"member"}));
        }
    }

    #[tokio::test]
    async fn ambiguous_json_never_invokes_the_request_type_deserializer() {
        #[derive(Debug)]
        struct Rejected;
        impl<'de> serde::Deserialize<'de> for Rejected {
            fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
                Err(serde::de::Error::custom(
                    "request deserializer must not see ambiguous input",
                ))
            }
        }
        let error = RequestJson::<Rejected>::from_request(
            request(
                br#"{"member":1,"member":2}"#.to_vec(),
                Some("application/json"),
            ),
            &(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            error.explanation,
            UiMessage::error_data_schema_json_key_duplicate(
                weftext_messages::params::ErrorDataSchemaJsonKeyDuplicate {
                    key: "member".into()
                }
            )
        );
    }

    #[tokio::test]
    async fn valid_preferences_mime_suffix_author_values_and_correct_number_rounding_survive() {
        for content_type in [
            "application/json",
            "application/json; charset=utf-8",
            "application/problem+json",
        ] {
            let bytes = br#"{"uiLocale":"en-US"}"#.to_vec();
            let RequestJson(value) =
                RequestJson::<Preference>::from_request(request(bytes, Some(content_type)), &())
                    .await
                    .unwrap();
            assert_eq!(value.ui_locale, UiLocale::EnUs);
        }
        let bytes =
            br#"{"number":9007199254740991.0,"raw":"author\r\n","units":[55296,0]}"#.to_vec();
        let RequestJson(value) =
            RequestJson::<Value>::from_request(request(bytes, Some("application/json")), &())
                .await
                .unwrap();
        assert_eq!(value["number"].as_f64(), Some(9_007_199_254_740_991_f64));
        assert_eq!(value["raw"], "author\r\n");
        assert_eq!(value["units"], json!([55296, 0]));
    }
}
