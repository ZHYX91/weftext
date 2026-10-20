use weftext_engine::*;

#[test]
fn managed_heading_numbering_stays_in_shared_editor_and_rejects_modified_rules() {
    let source = "= 标题 <first>\n\n正文";
    let out = edit(
        source,
        &EditRequest {
            source_hash: source_hash(source),
            intent: EditIntent::HeadingNumbering {
                action: HeadingNumberingIntent {
                    scheme: HeadingNumberingScheme::Chinese,
                    start: 3,
                    max_level: 3,
                    exclude_labels: vec!["first".into()],
                },
            },
        },
    )
    .unwrap();
    assert!(out.source.ends_with(source));
    assert!(
        out.map.unavailable_reason.is_none(),
        "{:?}",
        out.map.unavailable_reason
    );
    assert!(out.map.literal_regions.iter().any(|r| r.text == "正文"));
    assert_eq!(out.patches.len(), 1);
    assert_eq!((out.patches[0].from, out.patches[0].to), (0, 0));
    let altered = out
        .source
        .replace("set heading(numbering: none)", "it => it");
    assert_eq!(
        inspect_editor(&altered).unavailable_reason.as_deref(),
        Some("unverified-show-rule")
    );
}

#[test]
fn note_body_writer_is_one_source_revision_bound_draft_edit() {
    let source = "前文#footnote[原注] <note-a>后文";
    let before = inspect_editor(source);
    let definition = before
        .notes
        .entries
        .iter()
        .find(|e| e.kind == "footnote-definition")
        .unwrap();
    let out = edit(
        source,
        &EditRequest {
            source_hash: before.source_hash,
            intent: EditIntent::NativeNote {
                action: NotesIntent::SetNoteBody {
                    construct: definition.construct.clone(),
                    body: NotesBody {
                        mode: NotesBodyMode::Literal,
                        value: "中文 # 注释".into(),
                    },
                },
            },
        },
    )
    .unwrap();
    assert!(out.source.starts_with("前文#footnote["));
    assert!(out.source.ends_with("] <note-a>后文"));
    assert_eq!(out.patches.len(), 1);
    assert_eq!(out.patches[0].expected, "原注");
    assert!(out.source.contains("\\#"));
    assert_eq!(out.map.notes.entries.len(), 1);
}

#[test]
fn ordinary_body_insertion_stays_before_one_verified_trailing_endnotes_list() {
    let source =
        "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n正文\n\n#endnotes()\n";
    let map = inspect_editor(source);
    let at = map.body_insertion.unwrap().from;
    assert_eq!(at, source.find("#endnotes()").unwrap());
    let out = edit(
        source,
        &EditRequest {
            source_hash: map.source_hash,
            intent: EditIntent::InsertBlock {
                at,
                block: BlockInsert::Paragraph {
                    text: "新增".into(),
                },
            },
        },
    )
    .unwrap();
    assert!(out.source.ends_with("\n\n新增\n\n#endnotes()\n"));
    let unverified = source.replace("#endnotes()", "#endnotes(unknown: true)");
    assert_eq!(
        inspect_editor(&unverified).body_insertion.unwrap().from,
        unverified.len()
    );
}
