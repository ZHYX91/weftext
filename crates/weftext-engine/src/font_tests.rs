//! Inspect actual shaped native paged glyphs, including nested frames.
use crate::{DEFAULT_LIBRARY_STYLE, Engine, StyleModule, world::LocalWorld};
use typst::{
    layout::{Abs, Frame, FrameItem},
    text::TextItem,
};
use typst_layout::PagedDocument;

#[test]
fn offline_paged_frames_use_real_cjk_body_code_and_math_fonts() {
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let style = StyleModule {
        path: "library-style.typ".into(),
        source: DEFAULT_LIBRARY_STYLE.into(),
        enabled: true,
        entry_function: "library-style".into(),
    };
    let source = "正文汉字简繁體\n\n`ABC代码漢字`\n\n$ \"数学中文\" = frac(1,2) + x^2 $";
    let path = dir.path().join("note.typ");
    let world = LocalWorld::with_style(
        &engine.library,
        &engine.fonts,
        source,
        &path,
        dir.path(),
        &style,
    )
    .unwrap();
    let compiled = typst::compile::<PagedDocument>(&world);
    let document = compiled.output.unwrap();
    fn collect<'a>(frame: &'a Frame, out: &mut Vec<&'a TextItem>) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => collect(&group.frame, out),
                FrameItem::Text(text) => out.push(text),
                _ => {}
            }
        }
    }
    let mut items = vec![];
    for page in document.pages() {
        collect(&page.frame, &mut items);
    }
    assert!(!items.is_empty());
    for item in &items {
        for glyph in &item.glyphs {
            let text = &item.text[glyph.range()];
            if text.chars().any(|c| !c.is_control() && !c.is_whitespace()) {
                assert_ne!(
                    glyph.id,
                    0,
                    "missing real glyph for {text:?} in {}",
                    item.font.info().family
                );
            }
        }
    }
    for marker in ["正文", "汉字", "简繁體"] {
        assert!(
            items
                .iter()
                .any(|i| i.text.contains(marker) && i.font.info().family == "Noto Serif CJK SC"),
            "body {marker}: {items:?}"
        );
    }
    assert!(items.iter().any(|i|i.text.contains("代码漢字")&&i.font.info().family=="Noto Sans Mono CJK SC"),"raw: {items:?}");
    assert!(
        items
            .iter()
            .any(|i| i.text.contains("数学中文") && i.font.info().family == "Noto Serif CJK SC"),
        "math fonts: {:?}",
        items
            .iter()
            .map(|i| (&i.text, &i.font.info().family))
            .collect::<Vec<_>>()
    );
    assert!(
        items
            .iter()
            .any(|i| i.font.info().family == "New Computer Modern Math"
                && i.text
                    .chars()
                    .any(|c| matches!(c, '1' | '2' | '=' | '+' | '𝑥'))),
        "math structure: {items:?}"
    );
    let code = items
        .iter()
        .find(|i| i.text.contains("ABC") && i.font.info().family == "Noto Sans Mono CJK SC")
        .unwrap();
    for glyph in &code.glyphs {
        let text = &code.text[glyph.range()];
        if text.chars().all(|c| matches!(c, 'A' | 'B' | 'C')) {
            assert!((glyph.x_advance.at(Abs::pt(1.)).to_pt() - 0.5).abs() < 1e-6);
        } else if text.chars().all(|c| "代码漢字".contains(c)) {
            assert!((glyph.x_advance.at(Abs::pt(1.)).to_pt() - 1.).abs() < 1e-6);
        }
    }
}
