use tempfile::TempDir;
use weftext_engine::*;

fn source_body(value: &str) -> NotesBody {
    NotesBody {
        mode: NotesBodyMode::Source,
        value: value.into(),
    }
}
fn literal(value: &str) -> NotesBody {
    NotesBody {
        mode: NotesBodyMode::Literal,
        value: value.into(),
    }
}
fn end(source: &str) -> SourceRange {
    inspect_editor(source).body_insertion.unwrap()
}
fn definition(source: &str) -> SourceRange {
    inspect_notes_source(source)
        .entries
        .into_iter()
        .find(|entry| entry.kind == "footnote-definition")
        .unwrap()
        .construct
}
fn render(source: &str) -> RenderOutput {
    let dir = TempDir::new().unwrap();
    let output = Engine::new_with_system_fonts(false)
        .unwrap()
        .render(source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(output.ok, "{source}\n{:?}", output.diagnostics);
    output
}

#[test]
fn note_definition_removal_moves_exact_body_and_label_to_existing_reference() {
    let source = "中文#footnote[甲 *强调*\n\n第二段] <note> 后引@note 再引#footnote(<note>)";
    let updated = edit_notes(
        source,
        &NotesIntent::RemoveNote {
            construct: definition(source),
            all_references: false,
        },
    )
    .unwrap();
    assert_eq!(
        updated,
        "中文 后引#footnote[甲 *强调*\n\n第二段] <note> 再引#footnote(<note>)"
    );
    render(&updated);
    let removed = edit_notes(
        source,
        &NotesIntent::RemoveNote {
            construct: definition(source),
            all_references: true,
        },
    )
    .unwrap();
    assert_eq!(removed, "中文 后引 再引");
}

#[test]
fn insert_repeat_and_source_edit_keep_one_draft_and_parse_native_content() {
    let source = "正文";
    let inserted = edit(
        source,
        &EditRequest {
            source_hash: source_hash(source),
            intent: EditIntent::NativeNote {
                action: NotesIntent::InsertFootnote {
                    at: end(source),
                    body: literal("中文 #[]"),
                },
            },
        },
    )
    .unwrap();
    assert!(inserted.source.contains("#footnote[中文\\u{20}\\#\\[\\]]"));
    let repeated = edit_notes(
        &inserted.source,
        &NotesIntent::RepeatFootnote {
            at: end(&inserted.source),
            definition: definition(&inserted.source),
        },
    )
    .unwrap();
    assert_eq!(
        inspect_notes_source(&repeated)
            .entries
            .iter()
            .filter(|entry| entry.kind == "footnote-definition")
            .count(),
        1
    );
    let changed = edit_notes(
        &repeated,
        &NotesIntent::SetNoteBody {
            construct: definition(&repeated),
            body: source_body("*强调*\n\n- 列表\n- 第二项\n\n$ x^2 $"),
        },
    )
    .unwrap();
    assert!(changed.contains("*强调*\n\n- 列表"));
    render(&changed);
    assert!(
        edit_notes(
            &changed,
            &NotesIntent::SetNoteBody {
                construct: definition(&changed),
                body: source_body("]#metadata((id:\"injected\"))[ ")
            }
        )
        .is_err()
    );
    assert!(
        edit(
            &changed,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::NativeNote {
                    action: NotesIntent::SetNoteBody {
                        construct: definition(&changed),
                        body: literal("迟到")
                    }
                }
            }
        )
        .is_err()
    );
}

#[test]
fn caption_changes_only_exact_body_position_and_numbering_slots() {
    let source = "#figure(table(columns:1)[甲],caption:figure.caption(separator:[ --- ],position:bottom)[旧题],gap:3pt,numbering:\"1\") <tab>";
    let figure = inspect_notes_source(source)
        .entries
        .into_iter()
        .find(|entry| entry.kind == "figure")
        .unwrap();
    assert_eq!(figure.body.as_ref().unwrap().raw, "旧题");
    let updated = edit_notes(
        source,
        &NotesIntent::SetCaption {
            construct: figure.construct,
            body: source_body("新 *题*"),
            position: Some("top".into()),
            numbering: Some(NotesNumbering::Pattern {
                pattern: "一".into(),
            }),
        },
    )
    .unwrap();
    assert!(updated.contains("separator:[ --- ],position:top)[新 *题*],gap:3pt,numbering:\"一\""));
    render(&updated);
    let source = "#figure[对象]";
    let figure = inspect_notes_source(source)
        .entries
        .into_iter()
        .find(|entry| entry.kind == "figure")
        .unwrap();
    let updated = edit_notes(
        source,
        &NotesIntent::SetCaption {
            construct: figure.construct,
            body: literal("新题"),
            position: Some("top".into()),
            numbering: Some(NotesNumbering::None),
        },
    )
    .unwrap();
    render(&updated);
}

#[test]
fn unknown_binding_dynamic_source_and_missing_definition_keep_input() {
    for source in [
        "#let footnote(x)=x\n#footnote[正文]",
        "#show footnote: none\n#footnote[正文]",
        "#let f()=[#footnote[正文]]\n#f()",
    ] {
        let selected = definition(source);
        assert!(!selected.writable);
        assert!(
            edit_notes(
                source,
                &NotesIntent::SetNoteBody {
                    construct: selected,
                    body: literal("替换")
                }
            )
            .is_err()
        );
    }
    let source = "#footnote[正文] <n> #if false [@n]";
    assert!(
        edit_notes(
            source,
            &NotesIntent::RemoveNote {
                construct: definition(source),
                all_references: false
            }
        )
        .is_err()
    );
    let source = "#import \"other.typ\": make\n#footnote[正文] <n> #make()";
    assert!(
        inspect_notes_source(source)
            .entries
            .iter()
            .find(|entry| entry.kind == "footnote-definition")
            .unwrap()
            .writable
    );
    assert!(
        edit_notes(
            source,
            &NotesIntent::RemoveNote {
                construct: definition(source),
                all_references: false
            }
        )
        .is_err()
    );
}

#[test]
fn object_caption_and_local_equation_numbering_are_native_and_preserve_target() {
    let source = "#table(columns:1)[甲]";
    let table = inspect_editor(source)
        .structures
        .into_iter()
        .find(|entry| entry.kind == "table")
        .unwrap();
    let wrapped = edit_notes(
        source,
        &NotesIntent::WrapCaption {
            object: table.range,
            body: literal("表说明"),
            category: "table".into(),
            position: None,
            numbering: None,
        },
    )
    .unwrap();
    assert!(wrapped.starts_with("#figure(table("), "{wrapped}");
    assert!(!wrapped.starts_with("##"));
    render(&wrapped);
    let source = "正文\n\n$ x = 1 $ <eq>\n\n@eq";
    let math = inspect_editor(source)
        .structures
        .into_iter()
        .find(|entry| entry.kind == "displayMath")
        .unwrap();
    let numbered = edit_notes(
        source,
        &NotesIntent::SetEquationNumbering {
            construct: math.range,
            numbering: NotesNumbering::Pattern {
                pattern: "(1)".into(),
            },
        },
    )
    .unwrap();
    let output = render(&numbered);
    let explicit = output
        .source_info
        .notes
        .entries
        .iter()
        .find(|entry| entry.kind == "equation" && entry.construct.raw.starts_with("#math.equation"))
        .unwrap();
    assert!(explicit.writable, "{:?}", output.source_info.notes);
    assert!(
        edit_notes(
            &numbered,
            &NotesIntent::SetEquationNumbering {
                construct: explicit.construct.clone(),
                numbering: NotesNumbering::None,
            },
        )
        .is_err()
    );
    let without_reference = numbered.strip_suffix("\n\n@eq").unwrap();
    let unnumbered = edit_notes(
        without_reference,
        &NotesIntent::SetEquationNumbering {
            construct: explicit.construct.clone(),
            numbering: NotesNumbering::None,
        },
    )
    .unwrap();
    assert!(unnumbered.contains("numbering:none"));
    render(&unnumbered);
    let captioned = edit_notes(
        &numbered,
        &NotesIntent::WrapCaption {
            object: explicit.construct.clone(),
            body: literal("描述"),
            category: "equation".into(),
            position: None,
            numbering: None,
        },
    )
    .unwrap();
    assert!(captioned.contains(" <eq>]"), "{captioned}");
    render(&captioned);
    let source = "#set math.equation(numbering:\"(1)\")\n$ x = 1 $ <eq>\n@eq";
    let math = inspect_editor(source)
        .structures
        .into_iter()
        .find(|entry| entry.kind == "displayMath")
        .unwrap();
    let wrapped = edit_notes(
        source,
        &NotesIntent::WrapCaption {
            object: math.range,
            body: literal("描述"),
            category: "equation".into(),
            position: None,
            numbering: None,
        },
    )
    .unwrap();
    assert!(wrapped.contains("[$ x = 1 $ <eq>]"), "{wrapped}");
    assert!(wrapped.contains("numbering:none"));
    render(&wrapped);
}

#[test]
fn frozen_endnotes_use_exact_authorized_module_actual_callsite_and_one_draft() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let source =
        "#metadata((id:\"8f3fe32e-e7ce-47f7-a731-dda1eb15480f\",types:())) <weftext>\n正文";
    let mut input = FrozenRenderRequest {
        source: source.into(),
        path: dir.path().join("note.typ"),
        root: dir.path().into(),
        files: std::collections::BTreeMap::from([
            (
                "weftext-endnotes.typ".into(),
                include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec(),
            ),
            ("note.typ".into(), source.as_bytes().to_vec()),
        ]),
        style: Some(StyleModule {
            path: "default-style.typ".into(),
            source: DEFAULT_LIBRARY_STYLE.into(),
            enabled: true,
            entry_function: "library-style".into(),
        }),
    };
    assert!(style_allows_rich_editing(input.style.as_ref().unwrap()));
    let action = NotesIntent::InsertEndnote {
        at: end(source),
        body: source_body("中文 *强调*\n\n第二段"),
    };
    assert!(edit_notes(source, &action).is_err());
    let inserted = engine
        .edit_notes_frozen(
            &input,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::NativeNote {
                    action: action.clone(),
                },
            },
        )
        .unwrap();
    assert_eq!(inspect(&inserted.source).identity_status, "valid");
    assert!(inserted.source.contains("#box[#endnote(id:"));
    let definition = inserted
        .map
        .notes
        .entries
        .iter()
        .find(|entry| entry.kind == "endnote-definition")
        .unwrap();
    assert!(definition.writable, "{:?}", inserted.map.notes);
    input.source = inserted.source.clone();
    input
        .files
        .insert("note.typ".into(), input.source.as_bytes().to_vec());
    let rendered = engine.render_frozen(&input).unwrap();
    assert!(rendered.ok, "{:?}", rendered.diagnostics);
    fn markers<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "endnote-reference")
        {
            out.push(node)
        }
        for child in &node.children {
            markers(child, out)
        }
    }
    let mut actual = vec![];
    markers(rendered.tree.as_ref().unwrap(), &mut actual);
    assert_eq!(actual.len(), 1);
    assert!(
        actual[0]
            .origin
            .as_ref()
            .is_some_and(|origin| origin.is_main && origin.raw.starts_with("#box[#endnote"))
    );
    let repeated = engine
        .edit_notes_frozen(
            &input,
            &EditRequest {
                source_hash: source_hash(&input.source),
                intent: EditIntent::NativeNote {
                    action: NotesIntent::RepeatEndnote {
                        at: end(&input.source),
                        definition: definition.construct.clone(),
                    },
                },
            },
        )
        .unwrap();
    input.source = repeated.source.clone();
    input
        .files
        .insert("note.typ".into(), input.source.as_bytes().to_vec());
    let definition = repeated
        .map
        .notes
        .entries
        .iter()
        .find(|entry| entry.kind == "endnote-definition")
        .unwrap();
    let removed = engine
        .edit_notes_frozen(
            &input,
            &EditRequest {
                source_hash: source_hash(&input.source),
                intent: EditIntent::NativeNote {
                    action: NotesIntent::RemoveNote {
                        construct: definition.construct.clone(),
                        all_references: false,
                    },
                },
            },
        )
        .unwrap();
    assert_eq!(
        removed
            .map
            .notes
            .entries
            .iter()
            .filter(|entry| entry.kind == "endnote-definition")
            .count(),
        1
    );
    assert_eq!(
        removed
            .map
            .notes
            .entries
            .iter()
            .filter(|entry| entry.kind == "endnote-reference")
            .count(),
        0
    );
    let mut denied = input.clone();
    denied
        .files
        .get_mut("weftext-endnotes.typ")
        .unwrap()
        .extend(b"\n// changed");
    assert!(
        engine
            .edit_notes_frozen(
                &denied,
                &EditRequest {
                    source_hash: source_hash(&input.source),
                    intent: EditIntent::NativeNote {
                        action: NotesIntent::InsertEndnote {
                            at: end(&input.source),
                            body: literal("保稿")
                        }
                    }
                }
            )
            .is_err()
    );
}

#[test]
fn imported_unboxed_endnote_body_has_real_native_content_provenance() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let source = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n正文#endnote(id:\"a\")[原 *正文*] 重复#endnote-ref(\"a\")\n#endnotes()";
    let input = FrozenRenderRequest {
        source: source.into(),
        path: dir.path().join("note.typ"),
        root: dir.path().into(),
        files: std::collections::BTreeMap::from([(
            "weftext-endnotes.typ".into(),
            include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec(),
        )]),
        style: None,
    };
    let rendered = engine.render_frozen(&input).unwrap();
    assert!(rendered.ok, "{:?}", rendered.diagnostics);
    let definition = rendered
        .source_info
        .notes
        .entries
        .iter()
        .find(|entry| entry.kind == "endnote-definition")
        .unwrap();
    assert!(definition.writable, "{:?}", rendered.source_info.notes);
    assert!(
        !rendered
            .source_info
            .notes
            .entries
            .iter()
            .find(|entry| entry.kind == "endnote-reference")
            .unwrap()
            .writable
    );
    let edited = engine
        .edit_notes_frozen(
            &input,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::NativeNote {
                    action: NotesIntent::SetNoteBody {
                        construct: definition.construct.clone(),
                        body: source_body("修改 *正文*"),
                    },
                },
            },
        )
        .unwrap();
    assert!(edited.source.contains("[修改 *正文*]"));
}

#[test]
fn native_reference_family_partial_ranges_and_nested_math_targets_are_kept_distinct() {
    let source = "正文#footnote[正文] <n> 再引#ref(<n>)";
    let migrated = edit_notes(
        source,
        &NotesIntent::RemoveNote {
            construct: definition(source),
            all_references: false,
        },
    )
    .unwrap();
    assert_eq!(migrated, "正文 再引#footnote[正文] <n>");
    render(&migrated);
    let removed = edit_notes(
        source,
        &NotesIntent::RemoveNote {
            construct: definition(source),
            all_references: true,
        },
    )
    .unwrap();
    assert_eq!(removed, "正文 再引");
    for source in [
        "正文#footnote[正文] <n> 再引#ref(label(\"n\"))",
        "正文#footnote[正文] <n> #eval(\"@n\")",
        "正文#footnote[正文] <n> #repeat(2)[@n]",
        "正文#footnote[正文] <n> #([@n] * 2)",
    ] {
        assert!(
            edit_notes(
                source,
                &NotesIntent::RemoveNote {
                    construct: definition(source),
                    all_references: false
                }
            )
            .is_err(),
            "{source}"
        );
    }
    let source = "#table(columns:1)[甲]";
    let partial = SourceRange {
        from: 0,
        to: 6,
        utf16_from: 0,
        utf16_to: 6,
        raw: "#table".into(),
        writable: true,
    };
    assert!(
        edit_notes(
            source,
            &NotesIntent::WrapCaption {
                object: partial,
                body: literal("不得截断"),
                category: "table".into(),
                position: None,
                numbering: None
            }
        )
        .is_err()
    );
    let source = "#set math.equation(numbering:\"(1)\")\n#block[$ x = 1 $ <eq>]\n@eq";
    let math = inspect_editor(source)
        .structures
        .into_iter()
        .find(|entry| entry.kind == "displayMath")
        .unwrap();
    let wrapped = edit_notes(
        source,
        &NotesIntent::WrapCaption {
            object: math.range,
            body: literal("描述"),
            category: "equation".into(),
            position: None,
            numbering: None,
        },
    )
    .unwrap();
    assert!(wrapped.contains("[$ x = 1 $ <eq>]"), "{wrapped}");
    render(&wrapped);

    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let source = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n#set heading(numbering:\"1\")\n= 标题 <x>\n见@x。#endnote(id:\"x\")[尾注]\n#endnotes()";
    let input = FrozenRenderRequest {
        source: source.into(),
        path: dir.path().join("note.typ"),
        root: dir.path().into(),
        files: std::collections::BTreeMap::from([(
            "weftext-endnotes.typ".into(),
            include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec(),
        )]),
        style: None,
    };
    let current = engine.render_frozen(&input).unwrap();
    assert!(current.ok, "{:?}", current.diagnostics);
    let definition = current
        .source_info
        .notes
        .entries
        .iter()
        .find(|entry| entry.kind == "endnote-definition")
        .unwrap();
    for all_references in [false, true] {
        let updated = engine
            .edit_notes_frozen(
                &input,
                &EditRequest {
                    source_hash: source_hash(source),
                    intent: EditIntent::NativeNote {
                        action: NotesIntent::RemoveNote {
                            construct: definition.construct.clone(),
                            all_references,
                        },
                    },
                },
            )
            .unwrap();
        assert!(updated.source.contains("见@x。"), "{}", updated.source);
        assert!(updated.source.contains("= 标题 <x>"));
    }
}

#[test]
fn pure_note_entry_points_do_not_substitute_lexical_binding_for_frozen_module_proof() {
    let source = "#import \"weftext-endnotes.typ\": endnote,endnote-ref,endnotes\n正文#footnote[脚注] <f> 再引#footnote(<f>) 尾#endnote(id:\"tail\")[尾注]\n#endnotes()";
    assert!(standard_binding_available(source, "footnote"));
    let footnote = definition(source);
    let endnote = inspect_notes_source(source)
        .entries
        .into_iter()
        .find(|entry| entry.kind == "endnote-definition")
        .unwrap()
        .construct;
    let at = end(source);
    for action in [
        NotesIntent::RepeatFootnote {
            at: at.clone(),
            definition: footnote.clone(),
        },
        NotesIntent::RemoveNote {
            construct: footnote.clone(),
            all_references: false,
        },
        NotesIntent::RemoveNote {
            construct: footnote,
            all_references: true,
        },
        NotesIntent::InsertEndnote {
            at: at.clone(),
            body: literal("新增尾注"),
        },
        NotesIntent::RepeatEndnote {
            at,
            definition: endnote.clone(),
        },
        NotesIntent::SetNoteBody {
            construct: endnote.clone(),
            body: literal("更新尾注"),
        },
        NotesIntent::RemoveNote {
            construct: endnote.clone(),
            all_references: false,
        },
        NotesIntent::RemoveNote {
            construct: endnote,
            all_references: true,
        },
    ] {
        assert!(action.requires_frozen_module(source), "{action:?}");
        let direct = edit_notes(source, &action).unwrap_err();
        assert_eq!(
            direct.description().explanation.key(),
            "error.engine.notes.endnotesFrozenInputRequired",
            "{action:?}"
        );
        let generic = edit(
            source,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::NativeNote {
                    action: action.clone(),
                },
            },
        )
        .unwrap_err();
        assert_eq!(
            generic.description().explanation.key(),
            "error.engine.notes.endnotesFrozenInputRequired",
            "{action:?}"
        );
    }
}

#[test]
fn frozen_mixed_notes_repeat_and_migrate_footnote_definition_with_exact_module_only() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let source = "#import \"weftext-endnotes.typ\": endnote,endnote-ref,endnotes\n正文#footnote[脚 *正文*] <f> 再引#footnote(<f>) 尾#box[#endnote(id:\"tail\")[尾 *正文*]] 重引#box[#endnote-ref(\"tail\")]\n#endnotes()";
    let mut input = FrozenRenderRequest {
        source: source.into(),
        path: dir.path().join("note.typ"),
        root: dir.path().into(),
        files: std::collections::BTreeMap::from([(
            "weftext-endnotes.typ".into(),
            include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec(),
        )]),
        style: Some(StyleModule {
            path: "default-style.typ".into(),
            source: DEFAULT_LIBRARY_STYLE.into(),
            enabled: true,
            entry_function: "library-style".into(),
        }),
    };
    let repeated_action = NotesIntent::RepeatFootnote {
        at: end(source),
        definition: definition(source),
    };
    assert!(repeated_action.requires_frozen_module(source));
    assert!(edit_notes(source, &repeated_action).is_err());
    assert!(
        edit(
            source,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::NativeNote {
                    action: repeated_action.clone()
                },
            }
        )
        .is_err()
    );
    let repeated = engine
        .edit_notes_frozen(
            &input,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::NativeNote {
                    action: repeated_action.clone(),
                },
            },
        )
        .unwrap();
    assert_eq!(repeated.source.matches("#footnote(<f>)").count(), 2);
    input.source = repeated.source;
    let remove = NotesIntent::RemoveNote {
        construct: definition(&input.source),
        all_references: false,
    };
    assert!(remove.requires_frozen_module(&input.source));
    assert!(edit_notes(&input.source, &remove).is_err());
    assert!(
        edit(
            &input.source,
            &EditRequest {
                source_hash: source_hash(&input.source),
                intent: EditIntent::NativeNote {
                    action: remove.clone()
                },
            }
        )
        .is_err()
    );
    let migrated = engine
        .edit_notes_frozen_with_dependencies(
            &input,
            &EditRequest {
                source_hash: source_hash(&input.source),
                intent: EditIntent::NativeNote { action: remove },
            },
        )
        .unwrap();
    assert!(
        migrated
            .edit
            .source
            .contains("正文 再引#footnote[脚 *正文*] <f>")
    );
    assert_eq!(
        migrated
            .edit
            .source
            .matches("#footnote[脚 *正文*] <f>")
            .count(),
        1
    );
    assert!(
        migrated
            .edit
            .source
            .contains("#box[#endnote(id:\"tail\")[尾 *正文*]]")
    );
    assert!(
        migrated
            .edit
            .source
            .contains("#box[#endnote-ref(\"tail\")]")
    );
    assert!(migrated.dependencies.iter().any(|dependency| {
        dependency.path == "weftext-endnotes.typ" && dependency.status == "loaded"
    }));
    input.source = migrated.edit.source;
    let actual = engine.render_frozen(&input).unwrap();
    assert!(actual.ok, "{:?}", actual.diagnostics);
    fn count_role(node: &RenderNode, role: &str) -> usize {
        usize::from(
            node.attrs
                .get("data-weftext-role")
                .is_some_and(|actual| actual == role),
        ) + node
            .children
            .iter()
            .map(|child| count_role(child, role))
            .sum::<usize>()
    }
    let tree = actual.tree.as_ref().unwrap();
    assert_eq!(count_role(tree, "footnote-reference"), 2);
    assert_eq!(count_role(tree, "footnote-body"), 1);
    assert_eq!(count_role(tree, "endnote-reference"), 2);
    assert_eq!(count_role(tree, "endnote-body"), 1);
    let mut denied = input.clone();
    denied.files.clear();
    assert!(
        engine
            .edit_notes_frozen(
                &denied,
                &EditRequest {
                    source_hash: source_hash(&denied.source),
                    intent: EditIntent::NativeNote {
                        action: NotesIntent::RepeatFootnote {
                            at: end(&denied.source),
                            definition: definition(&denied.source),
                        }
                    },
                }
            )
            .is_err()
    );
    let mut denied = input.clone();
    denied
        .files
        .get_mut("weftext-endnotes.typ")
        .unwrap()
        .extend(b"\n// changed");
    assert!(
        engine
            .edit_notes_frozen(
                &denied,
                &EditRequest {
                    source_hash: source_hash(&denied.source),
                    intent: EditIntent::NativeNote {
                        action: NotesIntent::RepeatFootnote {
                            at: end(&denied.source),
                            definition: definition(&denied.source),
                        }
                    },
                }
            )
            .is_err()
    );
    for prefix in ["#import \"other.typ\": make\n", "#show footnote: none\n"] {
        let changed = format!("{prefix}{}", input.source);
        let action = NotesIntent::RemoveNote {
            construct: definition(&changed),
            all_references: false,
        };
        assert!(edit_notes(&changed, &action).is_err());
    }
}

#[test]
fn frozen_notes_use_validated_file_identity_for_native_path_separators() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let source = "正文#footnote[脚注] <f>";
    let module = include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec();
    let input = FrozenRenderRequest {
        source: source.into(),
        path: dir.path().join("Shared").join("note.typ"),
        root: dir.path().into(),
        files: std::collections::BTreeMap::from([
            ("Shared/note.typ".into(), source.as_bytes().to_vec()),
            ("Shared/weftext-endnotes.typ".into(), module.clone()),
        ]),
        style: None,
    };
    let request = EditRequest {
        source_hash: source_hash(source),
        intent: EditIntent::NativeNote {
            action: NotesIntent::InsertEndnote {
                at: end(source),
                body: literal("尾注"),
            },
        },
    };
    let canonical = engine.edit_notes_frozen(&input, &request).unwrap();
    let mut native = input.clone();
    native.files = native
        .files
        .into_iter()
        .map(|(path, bytes)| (path.replace('/', "\\"), bytes))
        .collect();
    let actual = engine
        .edit_notes_frozen_with_dependencies(&native, &request)
        .unwrap();
    assert_eq!(actual.edit.source, canonical.source);
    assert!(actual.dependencies.iter().any(|dependency| {
        dependency.path == "Shared/weftext-endnotes.typ" && dependency.status == "loaded"
    }));
    for duplicate in ["Shared/note.typ", "Shared/weftext-endnotes.typ"] {
        let mut denied = native.clone();
        denied
            .files
            .insert(duplicate.into(), input.files[duplicate].clone());
        assert!(
            engine.edit_notes_frozen(&denied, &request).is_err(),
            "{duplicate}"
        );
    }
    let mut denied = native.clone();
    denied
        .files
        .insert("Shared\\note.typ".into(), b"different original".to_vec());
    assert!(engine.edit_notes_frozen(&denied, &request).is_err());
    let mut denied = native.clone();
    denied
        .files
        .get_mut("Shared\\weftext-endnotes.typ")
        .unwrap()
        .extend(b"\n// changed");
    assert!(engine.edit_notes_frozen(&denied, &request).is_err());
    let mut denied = native;
    denied
        .files
        .insert("Shared/../escaped.typ".into(), b"outside".to_vec());
    assert!(engine.edit_notes_frozen(&denied, &request).is_err());
}

#[test]
fn frozen_notes_dependency_union_contains_only_actual_reads_and_exact_derived_entries() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let source = "#import \"weftext-endnotes.typ\": endnote,endnote-ref,endnotes\n正文#box[#endnote(id:\"a\")[#read(\"prior.txt\")]]\n#endnotes()";
    let input = FrozenRenderRequest {
        source: source.into(),
        path: dir.path().join("note.typ"),
        root: dir.path().into(),
        files: std::collections::BTreeMap::from([
            (
                "weftext-endnotes.typ".into(),
                include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec(),
            ),
            ("prior.txt".into(), "先前输入".as_bytes().to_vec()),
            ("next.txt".into(), "新内容".as_bytes().to_vec()),
            ("unused.txt".into(), b"not a dependency".to_vec()),
            (
                ".weftext-runtime-entry-authored.typ".into(),
                b"not synthetic".to_vec(),
            ),
        ]),
        style: Some(StyleModule {
            path: "default-style.typ".into(),
            source: DEFAULT_LIBRARY_STYLE.into(),
            enabled: true,
            entry_function: "library-style".into(),
        }),
    };
    let current = engine.render_frozen(&input).unwrap();
    assert!(current.ok, "{:?}", current.diagnostics);
    let definition = current
        .source_info
        .notes
        .entries
        .iter()
        .find(|entry| entry.kind == "endnote-definition")
        .unwrap();
    assert!(definition.writable);
    let result = engine
        .edit_notes_frozen_with_dependencies(
            &input,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::NativeNote {
                    action: NotesIntent::SetNoteBody {
                        construct: definition.construct.clone(),
                        body: source_body("#read(\"next.txt\")"),
                    },
                },
            },
        )
        .unwrap();
    for actual in ["prior.txt", "next.txt", "weftext-endnotes.typ"] {
        assert!(
            result
                .dependencies
                .iter()
                .any(|dependency| dependency.path == actual && dependency.status == "loaded"),
            "{actual}: {:?}",
            result.dependencies
        );
    }
    assert!(
        !result
            .dependencies
            .iter()
            .any(|dependency| dependency.path == "unused.txt"
                || dependency.path == ".weftext-runtime-entry-authored.typ")
    );
    assert!(result.dependencies.iter().any(|dependency|dependency.path=="default-style.typ"&&dependency.kind=="source"));
    let derived: std::collections::BTreeSet<_> = result
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == "derived-source")
        .map(|dependency| dependency.path.as_str())
        .collect();
    assert_eq!(derived.len(), 2, "{:?}", result.dependencies);
}
