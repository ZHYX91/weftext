use std::ops::Range;

use weftext_asciidoc::{
    DiagnosticCode, InlineKind, LinkKind, NodeLinkLabelCodecError, analyze, decode_node_link_label,
    encode_node_link_label,
};

const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";

fn exact<'a>(source: &'a str, range: &Range<u64>) -> &'a str {
    let start = usize::try_from(range.start).expect("range start");
    let end = usize::try_from(range.end).expect("range end");
    assert!(source.is_char_boundary(start));
    assert!(source.is_char_boundary(end));
    &source[start..end]
}

#[test]
fn codec_has_one_canonical_escape_for_every_reserved_character() {
    let label = "\\[]:,\" 中文 مرحبا 😀";
    let encoded = encode_node_link_label(label).unwrap();
    assert_eq!(encoded, r#"\\\[\]\:\,\" 中文 مرحبا 😀"#);
    assert_eq!(decode_node_link_label(&encoded).unwrap(), label);

    for reserved in ['[', ']', ':', ',', '"'] {
        assert!(matches!(
            decode_node_link_label(&reserved.to_string()),
            Err(NodeLinkLabelCodecError::UnescapedReservedCharacter {
                character,
                ..
            }) if character == reserved
        ));
    }
    assert!(matches!(
        decode_node_link_label("\\"),
        Err(NodeLinkLabelCodecError::TrailingEscape { .. })
    ));
    assert!(matches!(
        decode_node_link_label(r"unknown\q"),
        Err(NodeLinkLabelCodecError::UnknownEscape { character: 'q', .. })
    ));
    assert!(matches!(
        decode_node_link_label("trailing\\"),
        Err(NodeLinkLabelCodecError::TrailingEscape { .. })
    ));
}

#[test]
fn codec_roundtrips_a_deterministic_unicode_sample_space() {
    let atoms = ["", "a", "中", "م", "😀", "\\", "[", "]", ":", ",", "\""];
    for first in atoms {
        for second in atoms {
            for third in ["", "Z", "界", "ש", "🧪"] {
                let label = format!("{first}{second}{third}");
                let encoded = encode_node_link_label(&label).unwrap();
                assert_eq!(decode_node_link_label(&encoded).unwrap(), label);
                assert_eq!(
                    encode_node_link_label(&decode_node_link_label(&encoded).unwrap()).unwrap(),
                    encoded
                );
            }
        }
    }
}

#[test]
fn controls_and_bidi_formatting_are_rejected_but_rtl_letters_remain_valid() {
    for character in [
        '\0', '\n', '\r', '\u{001f}', '\u{007f}', '\u{0085}', '\u{009f}', '\u{061c}', '\u{200e}',
        '\u{200f}', '\u{202a}', '\u{202e}', '\u{2066}', '\u{2069}', '\u{206f}',
    ] {
        let label = format!("before{character}after");
        assert!(matches!(
            encode_node_link_label(&label),
            Err(NodeLinkLabelCodecError::ProhibitedCharacter { .. })
        ));
        assert!(matches!(
            decode_node_link_label(&label),
            Err(NodeLinkLabelCodecError::ProhibitedCharacter { .. })
        ));
    }

    let ordinary_rtl = "مرحبا שלום";
    assert_eq!(
        decode_node_link_label(&encode_node_link_label(ordinary_rtl).unwrap()).unwrap(),
        ordinary_rtl
    );
}

#[test]
fn scanner_decodes_display_and_preserves_exact_label_ranges() {
    let display = "\\[]:,\" 中文 مرحبا 😀";
    let encoded = encode_node_link_label(display).unwrap();
    let source = format!("node:{UUID}[{encoded}]\r\nnode::{UUID}#part[]");
    let analysis = analyze(&source);
    assert_eq!(analysis.links.len(), 2);

    let link = &analysis.links[0];
    assert_eq!(link.kind, LinkKind::Node);
    assert_eq!(link.display.as_deref(), Some(display));
    assert_eq!(exact(&source, &link.label_range), encoded);
    assert_eq!(link.target, UUID);

    let embed = &analysis.links[1];
    assert_eq!(embed.kind, LinkKind::NodeEmbed);
    assert_eq!(embed.fragment.as_deref(), Some("part"));
    assert_eq!(embed.display, None);
    assert_eq!(exact(&source, &embed.label_range), "");
    assert_eq!(embed.label_range.start, embed.label_range.end);

    let inline = analysis
        .inlines
        .iter()
        .find(|inline| inline.kind == InlineKind::Node && inline.range == link.range)
        .expect("node inline semantic");
    assert_eq!(inline.text.as_deref(), Some(display));
    assert_eq!(inline.label_range.as_ref(), Some(&link.label_range));
}

#[test]
fn macro_like_and_url_text_inside_a_label_is_never_scanned_recursively() {
    let display = format!(
        "node:{UUID}[inner] node::{UUID}[] xref:local[ref] image::pic.png[] https://example.test/a"
    );
    let encoded = encode_node_link_label(&display).unwrap();
    let source = format!("node:{UUID}[{encoded}]");
    let analysis = analyze(&source);
    assert_eq!(analysis.links.len(), 1);
    assert_eq!(analysis.links[0].kind, LinkKind::Node);
    assert_eq!(analysis.links[0].display.as_deref(), Some(display.as_str()));

    let label = &analysis.links[0].label_range;
    assert!(!analysis.inlines.iter().any(|inline| {
        label.start <= inline.range.start
            && inline.range.end <= label.end
            && matches!(
                inline.kind,
                InlineKind::Node
                    | InlineKind::NodeEmbed
                    | InlineKind::Xref
                    | InlineKind::Image
                    | InlineKind::NativeLink
            )
    }));
}

#[test]
fn malformed_labels_fail_closed_and_hide_nested_macro_spelling() {
    let sources = [
        format!(r"node:{UUID}[unknown\q xref:inner]"),
        format!("node:{UUID}[raw:colon]"),
        format!("node:{UUID}[raw,comma]"),
        format!("node:{UUID}[raw\"quote]"),
        format!("node:{UUID}[raw[open]"),
        format!("node:{UUID}[bidi\u{202e}text]"),
        format!("node:{UUID}[escaped-close\\]"),
    ];
    for source in sources {
        let analysis = analyze(&source);
        assert!(analysis.links.is_empty(), "{source:?}: {analysis:#?}");
        assert!(analysis.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.code,
                DiagnosticCode::InvalidNodeLink | DiagnosticCode::UnsupportedProfileSyntax
            )
        }));
        assert!(
            analysis
                .inlines
                .iter()
                .all(|inline| inline.kind != InlineKind::Xref)
        );
    }
}

#[test]
fn invalid_node_target_keeps_the_existing_occurrence_and_diagnostic_rule() {
    let source = "node:not-a-uuid[valid label]";
    let analysis = analyze(source);
    assert_eq!(analysis.links.len(), 1);
    assert_eq!(analysis.links[0].target, "not-a-uuid");
    assert_eq!(analysis.links[0].display.as_deref(), Some("valid label"));
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidNodeLink)
    );
}

#[test]
fn non_node_macros_keep_their_existing_label_semantics() {
    let source = concat!(
        "xref:local[Display, \"quoted\": value]\n",
        "image::picture.png[Alt text]\n",
        "link:https://example.test[Site]\n",
    );
    let analysis = analyze(source);
    let xref = analysis
        .links
        .iter()
        .find(|link| link.kind == LinkKind::Xref)
        .expect("xref occurrence");
    assert_eq!(xref.display.as_deref(), Some("Display, \"quoted\": value"));
    assert!(
        analysis
            .inlines
            .iter()
            .any(|inline| inline.kind == InlineKind::Image)
    );
    assert!(
        analysis
            .inlines
            .iter()
            .any(|inline| inline.kind == InlineKind::NativeLink)
    );
}

#[test]
fn link_ranges_are_exact_across_line_endings_and_missing_final_newline() {
    for line_ending in ["\n", "\r\n"] {
        let source = format!("node:{UUID}[LF]{line_ending}node:{UUID}[EOF]");
        let analysis = analyze(&source);
        assert_eq!(analysis.links.len(), 2);
        assert_eq!(exact(&source, &analysis.links[0].label_range), "LF");
        assert_eq!(exact(&source, &analysis.links[1].label_range), "EOF");
        assert_eq!(analysis.links[1].range.end, source.len() as u64);
    }
}
