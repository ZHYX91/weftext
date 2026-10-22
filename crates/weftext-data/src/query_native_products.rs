//! Native products from the same final HTML compile. No source scanning or IO.
use crate::{Cell, NativeMember, NativeValue, NoteProducts};

pub fn native_query_products(
    identity: String,
    products: weftext_engine::RenderProducts,
) -> NoteProducts {
    let title = match (products.title_status.as_str(), products.title) {
        ("value", Some(title)) => Cell::value(NativeValue::Text(title)),
        ("absent", None) => Cell::Absent,
        ("ambiguous", _) => Cell::Invalid {
            diagnostics: vec![weftext_messages::UiMessage::error_data_query_native_products_final_document_title_producers_disagree().into()],
            value: None,
        },
        _ => Cell::Unavailable {kind: crate::DataErrorKind::Unavailable, diagnostics: Vec::new(),
            reason: weftext_messages::UiMessage::error_data_query_native_products_final_document_title_producer_unqualified().into(),
        },
    };
    let body = match (products.body_status.as_str(), products.body) {
        ("complete", Some(body)) => Cell::value(NativeValue::Text(body)),
        _ => Cell::Unavailable {kind: crate::DataErrorKind::Unavailable, diagnostics: Vec::new(),
            reason: weftext_messages::UiMessage::error_data_query_native_products_final_html_body_island_unsupported().into(),
        },
    };
    let headings = NativeValue::Array(
        products
            .headings
            .into_iter()
            .map(|heading| {
                let display = if heading.visible {
                    "visible"
                } else if heading.is_main {
                    "mainSource"
                } else if heading.path.is_some() {
                    "includedSource"
                } else {
                    "unassociated"
                };
                NativeValue::Record(vec![
                    NativeMember {
                        key: "level".into(),
                        value: NativeValue::Int(heading.level.to_string()),
                    },
                    NativeMember {
                        key: "text".into(),
                        value: NativeValue::Text(heading.text),
                    },
                    NativeMember {
                        key: "generated".into(),
                        value: NativeValue::Bool(heading.generated),
                    },
                    NativeMember {
                        key: "displayAssociation".into(),
                        value: NativeValue::Text(display.into()),
                    },
                ])
            })
            .collect(),
    );
    NoteProducts {
        compile_identity: identity,
        target: products.target,
        title,
        body,
        headings: Cell::value(headings),
    }
}
