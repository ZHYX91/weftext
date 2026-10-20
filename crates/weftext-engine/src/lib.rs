//! Native Typst compilation and conservative author-source projections.

mod citations;
mod editor;
mod embedded_query;
pub use citations::*;
pub use embedded_query::*;
mod editor_types;
#[cfg(test)]
mod font_tests;
mod fonts;
mod frozen;
mod inspect;
mod notes;
mod numbering;
mod paged;
mod products;
mod render;
mod style;
mod style_adapter;
mod types;
mod world;
pub use fonts::{bundled_font_asset, bundled_font_manifest};
pub use frozen::FrozenRenderRequest;
pub use paged::PagedOutput;
pub use products::RenderProducts;
pub use style::*;

pub use editor::{
    edit, edit_with_style, encode_string, inspect_editor, inspect_editor_with_style, source_hash,
    standard_binding_available,
};
pub use editor_types::*;
pub use inspect::{certify_metadata_declaration, inspect, inspect_metadata};
pub use notes::*;
pub use numbering::*;
pub use render::{Engine, native_literal_display};
pub use types::*;

/// Encode plain text as native Typst markup without creating code or structure.
/// This is deliberately conservative; it does not normalize the user's text.
pub fn encode_literal(text: &str) -> Result<String, EngineError> {
    if text.len() > 1_048_576 || text.contains('\0') {
        return Err(EngineError("文字过长或含不支持的 NUL 字符".into()));
    }
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, ' ' | '\t') {
            out.push_str(&format!("\\u{{{:x}}}", ch as u32));
            continue;
        }
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            out.push_str(&format!("\\u{{{:x}}}", ch as u32));
            continue;
        }
        if "\\#[]{}$*_<>@`=-+/:~\"'.".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    Ok(out)
}
