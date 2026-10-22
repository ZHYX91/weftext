use weftext_data::{DataError, DataErrorKind, UiMessage};

pub fn assert_data_error(error: &DataError, kind: DataErrorKind, explanation: UiMessage) {
    assert_eq!(error.kind(), kind, "{error}");
    assert_eq!(error.description().explanation, explanation);
}
