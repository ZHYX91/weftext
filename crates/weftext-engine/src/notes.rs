//! Exact author-source slots for native notes and captions. These are candidates
//! until joined to the same successful native producer; they grant no identity.
use crate::{SourceRange, source_hash, standard_binding_available};
use serde::{Deserialize, Serialize};
use typst_syntax::{
    LinkedNode, Source, SyntaxKind,
    ast::{self, AstNode},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesSourceMap {
    pub source_hash: String,
    pub syntax_valid: bool,
    pub entries: Vec<NoteSourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSourceEntry {
    pub kind: String,
    pub construct: SourceRange,
    pub body: Option<SourceRange>,
    pub label: Option<String>,
    pub target: Option<String>,
    pub writable: bool,
    pub unavailable_kind: Option<NoteSourceUnavailableKind>,
    pub unavailable_reason: Option<weftext_messages::ErrorDescription>,
}

/// Native proof facts used by edit admission, independent of presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoteSourceUnavailableKind {
    FrozenModuleRequired,
    ReferenceDynamic,
    ReferenceArgumentsComplex,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NotesIntent {
    InsertFootnote {
        at: SourceRange,
        body: NotesBody,
    },
    InsertEndnote {
        at: SourceRange,
        body: NotesBody,
    },
    RepeatFootnote {
        at: SourceRange,
        definition: SourceRange,
    },
    RepeatEndnote {
        at: SourceRange,
        definition: SourceRange,
    },
    SetNoteBody {
        construct: SourceRange,
        body: NotesBody,
    },
    RemoveNote {
        construct: SourceRange,
        all_references: bool,
    },
    SetCaption {
        construct: SourceRange,
        body: NotesBody,
        position: Option<String>,
        numbering: Option<NotesNumbering>,
    },
    WrapCaption {
        object: SourceRange,
        body: NotesBody,
        category: String,
        position: Option<String>,
        numbering: Option<NotesNumbering>,
    },
    SetEquationNumbering {
        construct: SourceRange,
        numbering: NotesNumbering,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesBody {
    pub mode: NotesBodyMode,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotesBodyMode {
    Literal,
    Source,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NotesNumbering {
    None,
    Pattern { pattern: String },
}

impl NotesIntent {
    pub fn requires_frozen_module(&self, source: &str) -> bool {
        match self {
            Self::InsertEndnote { .. } | Self::RepeatEndnote { .. } => true,
            Self::RepeatFootnote { .. } => endnotes_import(source),
            Self::SetNoteBody { construct, .. } | Self::RemoveNote { construct, .. } => {
                inspect_notes_source(source).entries.iter().any(|entry| {
                    (entry.kind.starts_with("endnote-")
                        || (matches!(self, Self::RemoveNote { .. })
                            && entry.kind == "footnote-definition"
                            && endnotes_import(source)))
                        && entry.construct.from == construct.from
                        && entry.construct.to == construct.to
                        && entry.construct.raw == construct.raw
                })
            }
            _ => false,
        }
    }
}

impl crate::Engine {
    /// Source editing against Core's immutable authorized resource cut. A name,
    /// caller-supplied hash, or ambient file cannot certify the helper module.
    pub fn edit_notes_frozen(
        &self,
        input: &crate::FrozenRenderRequest,
        request: &crate::EditRequest,
    ) -> Result<crate::EditOutput, crate::EngineError> {
        self.edit_notes_frozen_with_dependencies(input, request)
            .map(|result| result.edit)
    }

    pub fn edit_notes_frozen_with_dependencies(
        &self,
        input: &crate::FrozenRenderRequest,
        request: &crate::EditRequest,
    ) -> Result<FrozenNotesEdit, crate::EngineError> {
        if request.source_hash != source_hash(&input.source) {
            return Err(crate::EngineError::new(
                weftext_messages::UiMessage::error_engine_notes_source_stale(),
            ));
        }
        let (notes, numbering) = match &request.intent {
            crate::EditIntent::NativeNote { action } => (Some(action), None),
            crate::EditIntent::HeadingNumbering { action } => (None, Some(action)),
            _ => {
                return Err(crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_frozen_intent_unsupported(),
                ));
            }
        };
        if !notes.is_some_and(|action| action.requires_frozen_module(&input.source))
            && !numbering.is_some_and(|action| action.requires_frozen_module(&input.source))
        {
            return crate::edit(&input.source, request).map(|edit| FrozenNotesEdit {
                edit,
                dependencies: vec![],
            });
        }
        const MODULE: &[u8] = include_bytes!("../../weftext-export/assets/weftext-endnotes.typ");
        let relative = if input.path.is_absolute() {
            input.path.strip_prefix(&input.root).map_err(|_| {
                crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_frozen_main_unauthorized(),
                )
            })?
        } else {
            input.path.as_path()
        };
        let module_key = relative
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""))
            .join("weftext-endnotes.typ");
        // Validate the complete original cut first, including duplicate FileIds
        // spelled with different path separators and an explicit main copy.
        let world = crate::world::LocalWorld::frozen(&self.library, &self.fonts, input)?;
        if world.admitted_bytes(&module_key)? != Some(MODULE) {
            return Err(crate::EngineError::new(
                weftext_messages::UiMessage::error_engine_notes_frozen_module_mismatch(),
            ));
        }
        let pinned = PinnedEndnotes {
            source: &input.source,
        };
        let mut main_key = None;
        for key in input.files.keys() {
            if world.is_main_admitted_path(std::path::Path::new(key))? {
                main_key = Some(key.clone());
            }
        }
        let current_derived = world.derived_dependency_paths();
        let mut dependencies = vec![];
        if !matches!(notes, Some(NotesIntent::InsertEndnote { .. })) {
            let current = match self.render_frozen_compilation(input)? {
                crate::render::RenderCompilation::Compiled(output) => output,
                crate::render::RenderCompilation::Rejected {
                    diagnostics,
                    product_diagnostics,
                    ..
                } => {
                    let mut description = weftext_messages::ErrorDescription::new(
                        crate::UiMessage::error_engine_notes_frozen_output_unavailable(),
                    );
                    description.diagnostics = diagnostics;
                    return Err(crate::EngineError::new(description)
                        .with_product_diagnostics(product_diagnostics));
                }
            };
            dependencies.extend(classify_note_dependencies(
                current.dependencies.clone(),
                &current_derived,
            ));
            let selected = match notes {
                Some(
                    NotesIntent::RepeatEndnote { definition, .. }
                    | NotesIntent::RepeatFootnote { definition, .. },
                ) => Some(definition),
                Some(
                    NotesIntent::SetNoteBody { construct, .. }
                    | NotesIntent::RemoveNote { construct, .. },
                ) => Some(construct),
                _ => None,
            };
            if selected.is_some_and(|selected| {
                !current.source_info.notes.entries.iter().any(|entry| {
                    entry.construct.from == selected.from
                        && entry.construct.to == selected.to
                        && entry.construct.raw == selected.raw
                        && entry.writable
                })
            }) {
                return Err(crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_endnote_origin_uncertified(),
                ));
            }
        }
        let updated = if let Some(action) = notes {
            edit_notes_qualified(&input.source, action, true)?
        } else if let Some(action) = numbering {
            crate::numbering::edit_heading_numbering_qualified(
                &input.source,
                action,
                Some(&pinned),
            )?
        } else {
            unreachable!()
        };
        let mut proposed = input.clone();
        proposed.source = updated.clone();
        if let Some(main_key) = main_key {
            proposed.files.insert(main_key, updated.as_bytes().to_vec());
        }
        let proposed_world =
            crate::world::LocalWorld::frozen(&self.library, &self.fonts, &proposed)?;
        let proposed_derived = proposed_world.derived_dependency_paths();
        let compiled = match self.render_frozen_compilation(&proposed)? {
            crate::render::RenderCompilation::Compiled(output) => output,
            crate::render::RenderCompilation::Rejected {
                diagnostics,
                product_diagnostics,
                ..
            } => {
                let mut description = weftext_messages::ErrorDescription::new(
                    crate::UiMessage::error_engine_notes_frozen_compilation_failed(),
                );
                description.diagnostics = diagnostics;
                return Err(crate::EngineError::new(description)
                    .with_product_diagnostics(product_diagnostics));
            }
        };
        dependencies.extend(classify_note_dependencies(
            compiled.dependencies.clone(),
            &proposed_derived,
        ));
        fn actual_list(node: &crate::RenderNode) -> bool {
            node.attrs
                .get("data-weftext-role")
                .is_some_and(|role| role == "endnotes")
                || node.children.iter().any(actual_list)
        }
        if notes.is_some()
            && (!compiled.tree.as_ref().is_some_and(actual_list)
                || compiled.source_info.notes.entries.iter().any(|entry| {
                    entry.kind.starts_with("endnote-")
                        && entry.construct.raw.starts_with("#box[")
                        && !entry.writable
                }))
        {
            return Err(crate::EngineError::new(
                weftext_messages::UiMessage::error_engine_notes_endnote_call_origin_unavailable(),
            ));
        }
        let mut from = input
            .source
            .bytes()
            .zip(updated.bytes())
            .take_while(|(left, right)| left == right)
            .count();
        while !input.source.is_char_boundary(from) || !updated.is_char_boundary(from) {
            from -= 1;
        }
        let mut suffix = input.source.as_bytes()[from..]
            .iter()
            .rev()
            .zip(updated.as_bytes()[from..].iter().rev())
            .take_while(|(left, right)| left == right)
            .count();
        while !input.source.is_char_boundary(input.source.len() - suffix)
            || !updated.is_char_boundary(updated.len() - suffix)
        {
            suffix -= 1;
        }
        let to = input.source.len() - suffix;
        let patch = crate::SourcePatch {
            from,
            to,
            utf16_from: crate::types::utf16(&input.source, from),
            utf16_to: crate::types::utf16(&input.source, to),
            expected: input.source[from..to].into(),
            insert: updated[from..updated.len() - suffix].into(),
        };
        let mut output = crate::editor::finish(&input.source, vec![patch])?;
        output.map.notes = compiled.source_info.notes;
        if numbering.is_some() {
            // This witness is scoped to the exact proposal from the same
            // authenticated immutable resource cut, never a browser flag.
            let proposed_pinned = PinnedEndnotes { source: &updated };
            output.map.heading_numbering = crate::numbering::inspect_heading_numbering_qualified(
                &updated,
                Some(&proposed_pinned),
            );
        }
        dependencies.sort_by(|left, right| {
            (&left.path, &left.kind, &left.status).cmp(&(&right.path, &right.kind, &right.status))
        });
        dependencies.dedup_by(|left, right| {
            left.path == right.path && left.kind == right.kind && left.status == right.status
        });
        Ok(FrozenNotesEdit {
            edit: output,
            dependencies,
        })
    }
}

/// Constructed only after this module verifies the exact bundled bytes in an
/// immutable World. It cannot be deserialized or reused for another source.
pub(crate) struct PinnedEndnotes<'a> {
    source: &'a str,
}
impl PinnedEndnotes<'_> {
    pub(crate) fn certifies(&self, source: &str) -> bool {
        self.source == source
    }
}

/// A rendering certificate comes from the same compiler World's actual reads,
/// not the admission map, a matching filename elsewhere, or a new file read.
pub(crate) fn pinned_endnotes_from_loaded_world<'a>(
    source: &'a str,
    world: &crate::world::LocalWorld<'_>,
) -> Option<PinnedEndnotes<'a>> {
    if world.main_source.text() != source || !endnotes_import(source) {
        return None;
    }
    let module_path = world
        .main_id
        .vpath()
        .parent()?
        .join("weftext-endnotes.typ")
        .ok()?;
    let module_id = typst_syntax::FileId::new(typst_syntax::RootedPath::new(
        world.main_id.root().clone(),
        module_path,
    ));
    const MODULE: &str = include_str!("../../weftext-export/assets/weftext-endnotes.typ");
    if !world
        .loaded_sources()
        .iter()
        .any(|loaded| loaded.id() == module_id && loaded.text() == MODULE)
        || !world.dependencies().iter().any(|dependency| {
            dependency.path == world.display_path(module_id)
                && dependency.kind == "source"
                && dependency.status == "loaded"
        })
    {
        return None;
    }
    Some(PinnedEndnotes { source })
}

#[cfg(test)]
mod pinned_world_tests {
    use super::*;
    use typst::World;

    #[test]
    fn admitted_but_unread_module_cannot_mint_render_proof() {
        let dir = tempfile::TempDir::new().unwrap();
        let source = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n= Heading\n#endnotes()";
        let input = crate::FrozenRenderRequest {
            source: source.into(),
            path: dir.path().join("nested/note.typ"),
            root: dir.path().into(),
            style: None,
            files: std::collections::BTreeMap::from([(
                "nested/weftext-endnotes.typ".into(),
                include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec(),
            )]),
        };
        let engine = crate::Engine::new_with_system_fonts(false).unwrap();
        let world =
            crate::world::LocalWorld::frozen(&engine.library, &engine.fonts, &input).unwrap();
        assert!(pinned_endnotes_from_loaded_world(source, &world).is_none());
        let module_id = typst_syntax::FileId::new(typst_syntax::RootedPath::new(
            typst_syntax::VirtualRoot::Project,
            typst_syntax::VirtualPath::new("/nested/weftext-endnotes.typ").unwrap(),
        ));
        world.source(module_id).unwrap();
        let proof = pinned_endnotes_from_loaded_world(source, &world).unwrap();
        assert!(proof.certifies(source));
        let inspection =
            crate::numbering::inspect_heading_numbering_qualified(source, Some(&proof));
        assert!(inspection.editable);
        assert!(!inspection.frozen_endnotes_required);
        assert!(crate::inspect_heading_numbering(source).frozen_endnotes_required);
        assert!(!proof.certifies(&(source.to_owned() + "\nchanged")));
        assert!(
            pinned_endnotes_from_loaded_world(&(source.to_owned() + "\nchanged"), &world).is_none()
        );
    }
}

#[derive(Debug, Clone)]
pub struct FrozenNotesEdit {
    pub edit: crate::EditOutput,
    /// Union of actual current-qualification and proposed-compilation reads.
    /// Synthetic entry points are explicit derived-source/derived-file kinds.
    /// No unused file from the admitted snapshot is added to this set.
    pub dependencies: Vec<crate::Dependency>,
}

fn classify_note_dependencies(
    mut dependencies: Vec<crate::Dependency>,
    derived: &std::collections::HashSet<String>,
) -> Vec<crate::Dependency> {
    for dependency in &mut dependencies {
        if derived.contains(&dependency.path) {
            dependency.kind = match dependency.kind.as_str() {
                "source" => "derived-source".into(),
                "file" => "derived-file".into(),
                _ => dependency.kind.clone(),
            };
        }
    }
    dependencies
}

fn body_source(body: &NotesBody) -> Result<String, crate::EngineError> {
    match body.mode {
        NotesBodyMode::Literal => crate::encode_literal(&body.value),
        NotesBodyMode::Source if body.value.len() <= 1_048_576 && !body.value.contains('\0') => {
            let probe = Source::detached(format!("#block[{}]", body.value));
            let nodes: Vec<_> = probe.root().children().collect();
            let confined = nodes.len() == 2 && nodes[0].kind() == SyntaxKind::Hash
                && nodes[1].cast::<ast::FuncCall>().is_some_and(|call| {
                    let args: Vec<_> = call.args().items().collect();
                    matches!(args.as_slice(), [ast::Arg::Pos(expr)] if expr.to_untyped().cast::<ast::ContentBlock>().is_some())
                });
            if confined && probe.root().errors_and_warnings().0.is_empty() {
                Ok(body.value.clone())
            } else {
                Err(crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_note_body_slot_required(),
                ))
            }
        }
        NotesBodyMode::Source => Err(crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_note_body_invalid(),
        )),
    }
}

pub fn edit_notes(source: &str, intent: &NotesIntent) -> Result<String, crate::EngineError> {
    edit_notes_qualified(source, intent, false)
}

fn edit_notes_qualified(
    source: &str,
    intent: &NotesIntent,
    frozen_module: bool,
) -> Result<String, crate::EngineError> {
    use NotesIntent::*;
    let mut map = inspect_notes_source(source);
    if frozen_module {
        for entry in &mut map.entries {
            if entry.unavailable_kind == Some(NoteSourceUnavailableKind::FrozenModuleRequired) {
                entry.writable = true;
                entry.construct.writable = true;
                if let Some(body) = &mut entry.body {
                    body.writable = true;
                }
            }
        }
    }
    let fail = || {
        crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_note_origin_unavailable(),
        )
    };
    let entry = |range: &SourceRange| {
        map.entries
            .iter()
            .find(|entry| {
                entry.writable
                    && entry.construct.from == range.from
                    && entry.construct.to == range.to
                    && entry.construct.raw == range.raw
            })
            .ok_or_else(fail)
    };
    let mut edits = vec![];
    match intent {
        InsertFootnote { at, body } => {
            certify_note_insertion(source, at)?;
            if !notes_binding_available(source, "footnote") {
                return Err(fail());
            }
            edits.push((at.from..at.to, format!("#footnote[{}]", body_source(body)?)));
        }
        InsertEndnote { at, body } => {
            if !frozen_module {
                return Err(crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_endnotes_frozen_input_required(
                    ),
                ));
            }
            certify_note_insertion(source, at)?;
            let module_import = endnotes_import(source);
            if !module_import
                && !["endnote", "endnote-ref", "endnotes", "box"]
                    .iter()
                    .all(|name| standard_binding_available(source, name))
            {
                return Err(fail());
            }
            let id = format!(
                "note-{}",
                &source_hash(&format!("{source}:{}:{}", at.from, body.value))[..16]
            );
            edits.push((
                at.from..at.to,
                format!(
                    "#box[#endnote(id:{})[{}]]",
                    crate::encode_string(&id)?,
                    body_source(body)?
                ),
            ));
            if !module_import {
                let after = crate::inspect(source)
                    .metadata
                    .map_or(0, |metadata| metadata.to);
                edits.push((
                    after..after,
                    "\n#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n".into(),
                ));
            }
            if !map.entries.iter().any(|entry| entry.kind == "endnotes") {
                edits.push((source.len()..source.len(), "\n\n#endnotes()".into()));
            }
        }
        RepeatFootnote { at, definition } => {
            certify_note_insertion(source, at)?;
            let definition = entry(definition)?;
            if definition.kind != "footnote-definition"
                || !footnote_binding_available(source, frozen_module)
            {
                return Err(fail());
            }
            let label = definition.label.clone().unwrap_or_else(|| {
                format!(
                    "weftext-footnote-{}",
                    &source_hash(&format!("{}:{}", source, definition.construct.from))[..16]
                )
            });
            if map
                .entries
                .iter()
                .filter(|entry| entry.label.as_ref() == Some(&label))
                .count()
                > usize::from(definition.label.is_some())
            {
                return Err(fail());
            }
            if definition.label.is_none() {
                edits.push((
                    definition.construct.to..definition.construct.to,
                    format!(" <{label}>"),
                ));
            }
            edits.push((at.from..at.to, format!("#footnote(<{label}>)")));
        }
        RepeatEndnote { at, definition } => {
            if !frozen_module || !endnotes_import(source) {
                return Err(fail());
            }
            certify_note_insertion(source, at)?;
            let definition = entry(definition)?;
            if definition.kind != "endnote-definition" {
                return Err(fail());
            }
            let id = definition.target.as_ref().ok_or_else(fail)?;
            edits.push((
                at.from..at.to,
                format!("#box[#endnote-ref({})]", crate::encode_string(id)?),
            ));
        }
        SetNoteBody { construct, body } => {
            let definition = entry(construct)?;
            if !matches!(
                definition.kind.as_str(),
                "footnote-definition" | "endnote-definition" | "figure"
            ) {
                return Err(fail());
            }
            let slot = definition.body.as_ref().ok_or_else(fail)?;
            edits.push((slot.from..slot.to, body_source(body)?));
        }
        RemoveNote {
            construct,
            all_references,
        } => {
            let selected = entry(construct)?;
            if !selected.kind.starts_with("footnote-") && !selected.kind.starts_with("endnote-") {
                return Err(fail());
            }
            if selected.kind == "footnote-definition" || selected.kind == "endnote-definition" {
                // Unknown imported/generated references cannot be discovered
                // by a source-only writer. Retain the original declaration.
                let is_endnote = selected.kind == "endnote-definition";
                if is_endnote {
                    if !frozen_module || !endnotes_import(source) {
                        return Err(fail());
                    }
                } else if !footnote_binding_available(source, frozen_module) {
                    return Err(fail());
                }
                let target = if is_endnote {
                    selected.target.as_ref()
                } else {
                    selected.label.as_ref()
                };
                if !is_endnote && !footnote_relations_closed_qualified(source, &map, frozen_module)
                {
                    return Err(fail());
                }
                let references: Vec<_> = target
                    .map(|label| {
                        map.entries
                            .iter()
                            .filter(|candidate| {
                                candidate.target.as_ref() == Some(label)
                                    && if is_endnote {
                                        candidate.kind == "endnote-reference"
                                    } else {
                                        matches!(
                                            candidate.kind.as_str(),
                                            "reference" | "footnote-reference"
                                        )
                                    }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                if !references.is_empty() && !notes_binding_available(source, "ref") {
                    return Err(fail());
                }
                if references.iter().any(|reference| {
                    reference.unavailable_kind == Some(NoteSourceUnavailableKind::ReferenceDynamic)
                        || (reference.kind != "reference" && !reference.writable)
                }) {
                    return Err(fail());
                }
                if *all_references {
                    for reference in references {
                        edits.push((
                            reference.construct.from..reference.construct.to,
                            String::new(),
                        ));
                    }
                } else if let Some(reference) = references.iter().find(|reference| {
                    reference.unavailable_kind
                        != Some(NoteSourceUnavailableKind::ReferenceArgumentsComplex)
                }) {
                    // Transfer the original exact declaration, including its
                    // body and label, to an existing actual native reference.
                    edits.push((
                        reference.construct.from..reference.construct.to,
                        selected.construct.raw.clone(),
                    ));
                } else if !references.is_empty() {
                    return Err(fail());
                }
            }
            edits.push((
                selected.construct.from..selected.construct.to,
                String::new(),
            ));
        }
        SetCaption {
            construct,
            body,
            position,
            numbering,
        } => {
            let figure = entry(construct)?;
            if figure.kind != "figure" {
                return Err(fail());
            }
            edits.extend(caption_edits(
                source,
                figure,
                body,
                position.as_deref(),
                numbering.as_ref(),
            )?);
        }
        WrapCaption {
            object,
            body,
            category,
            position,
            numbering,
        } => {
            if !notes_binding_available(source, "figure")
                || !matches!(category.as_str(), "figure" | "table" | "equation")
            {
                return Err(fail());
            }
            let map = crate::inspect_editor(source);
            if map.unavailable_reason.is_some()
                || source.get(object.from..object.to) != Some(object.raw.as_str())
            {
                return Err(fail());
            }
            let parsed = Source::detached(source);
            let call = native_call_at(
                parsed.root(),
                &parsed,
                object.from + usize::from(object.raw.starts_with('#')),
            );
            let expected = match category.as_str() {
                "table" => "table",
                "figure" => "image",
                _ => "math",
            };
            let is_math = map.structures.iter().any(|structure| {
                structure.kind == "displayMath"
                    && structure.range.from == object.from
                    && structure.range.to == object.to
            }) || map.notes.entries.iter().any(|entry| {
                entry.kind == "equation"
                    && entry.writable
                    && entry.construct.from == object.from
                    && entry.construct.to == object.to
            });
            let is_call = call.is_some_and(|call| {
                parsed
                    .find(call.span())
                    .is_some_and(|node| node.range().end == object.to)
                    && call
                        .callee()
                        .to_untyped()
                        .cast::<ast::Ident>()
                        .is_some_and(|name| name.get() == expected)
            });
            if !notes_binding_available(source, expected)
                || if category == "equation" {
                    !is_math
                } else {
                    !is_call
                }
            {
                return Err(fail());
            }
            if category == "equation"
                && numbering
                    .as_ref()
                    .is_some_and(|numbering| !matches!(numbering, NotesNumbering::None))
            {
                return Err(crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_equation_caption_numbering_choice(),
                ));
            }
            let caption = match position.as_deref() {
                Some(position @ ("top" | "bottom")) => format!(
                    "figure.caption(position:{position})[{}]",
                    body_source(body)?
                ),
                None => format!("[{}]", body_source(body)?),
                _ => return Err(fail()),
            };
            let numbering = match numbering {
                Some(numbering) => numbering_source(numbering)?,
                None if category == "equation" => "none".into(),
                None => "\"1\"".into(),
            };
            let kind = match category.as_str() {
                "table" => "table",
                "equation" => "math.equation",
                _ => "image",
            };
            let mut object_range = object.from..object.to;
            if is_call
                && !object.raw.starts_with('#')
                && object.from > 0
                && source.as_bytes()[object.from - 1] == b'#'
            {
                // The official FuncCall range excludes its markup Hash token.
                // Consume that exact preceding token with the wrapped object.
                object_range.start -= 1;
            }
            let original = if category == "equation" {
                if let Some(node) = equation_source_node(
                    &LinkedNode::new(parsed.root()),
                    object.from + usize::from(object.raw.starts_with('#')),
                ) {
                    let mut next = node.next_sibling();
                    while next.as_ref().is_some_and(|node| node.kind().is_trivia()) {
                        next = next.unwrap().next_sibling();
                    }
                    if let Some(label) =
                        next.filter(|node| node.get().cast::<ast::Label>().is_some())
                    {
                        object_range.end = label.range().end;
                    }
                }
                format!("[{}]", &source[object_range.clone()])
            } else {
                object
                    .raw
                    .strip_prefix('#')
                    .unwrap_or(&object.raw)
                    .to_string()
            };
            edits.push((
                object_range,
                format!("#figure({original},caption:{caption},kind:{kind},numbering:{numbering})"),
            ));
        }
        SetEquationNumbering {
            construct,
            numbering,
        } => {
            let map = crate::inspect_editor(source);
            if map.unavailable_reason.is_some()
                || !notes_binding_available(source, "math")
                || source.get(construct.from..construct.to) != Some(construct.raw.as_str())
            {
                return Err(fail());
            }
            if matches!(numbering, NotesNumbering::None) {
                let parsed = Source::detached(source);
                if let Some(equation) = equation_source_node(
                    &LinkedNode::new(parsed.root()),
                    construct.from + usize::from(construct.raw.starts_with('#')),
                ) {
                    let mut following = equation.next_sibling();
                    while following
                        .as_ref()
                        .is_some_and(|node| node.kind().is_trivia())
                    {
                        following = following.unwrap().next_sibling();
                    }
                    if let Some(label) = following
                        .as_ref()
                        .and_then(|node| node.get().cast::<ast::Label>())
                        && (map.notes.entries.iter().any(|entry| {
                            entry.kind == "reference"
                                && entry.target.as_deref() == Some(label.get())
                        }) || !footnote_relations_closed(source, &map.notes))
                    {
                        return Err(crate::EngineError::new(weftext_messages::UiMessage::error_engine_notes_equation_references_unresolved()));
                    }
                }
            }
            let value = numbering_source(numbering)?;
            if let Some(structure) = map.structures.iter().find(|structure| {
                structure.kind == "displayMath"
                    && structure.range.from == construct.from
                    && structure.range.to == construct.to
            }) {
                let slot = structure.body.as_ref().ok_or_else(fail)?;
                edits.push((
                    construct.from..construct.to,
                    format!(
                        "#math.equation(block:true,numbering:{value},${}$)",
                        slot.raw.trim()
                    ),
                ));
            } else if let Some(equation) = map.notes.entries.iter().find(|entry| {
                entry.kind == "equation"
                    && entry.writable
                    && entry.construct.raw.starts_with('$')
                    && entry.construct.from == construct.from
                    && entry.construct.to == construct.to
            }) {
                let slot = equation.body.as_ref().ok_or_else(fail)?;
                edits.push((
                    construct.from..construct.to,
                    format!(
                        "#math.equation(block:true,numbering:{value},${}$)",
                        slot.raw.trim()
                    ),
                ));
            } else {
                let parsed = Source::detached(source);
                let call = native_call_at(
                    parsed.root(),
                    &parsed,
                    construct.from + usize::from(construct.raw.starts_with('#')),
                )
                .filter(|call| {
                    parsed
                        .find(call.span())
                        .is_some_and(|node| node.range().end == construct.to)
                        && call
                            .callee()
                            .to_untyped()
                            .cast::<ast::FieldAccess>()
                            .is_some_and(|field| {
                                field.field().get() == "equation"
                                    && field
                                        .target()
                                        .to_untyped()
                                        .cast::<ast::Ident>()
                                        .is_some_and(|name| name.get() == "math")
                            })
                })
                .ok_or_else(fail)?;
                edits.push(named_argument_edit(
                    call,
                    "numbering",
                    value,
                    &parsed,
                    source,
                )?);
            }
        }
    }
    apply_note_edits(source, edits)
}

fn certify_note_insertion(source: &str, at: &SourceRange) -> Result<(), crate::EngineError> {
    let map = crate::inspect_editor(source);
    let exact = at.from == at.to
        && at.raw.is_empty()
        && source.is_char_boundary(at.from)
        && at.utf16_from == crate::types::utf16(source, at.from)
        && at.utf16_to == at.utf16_from;
    let available = map.unavailable_reason.is_none()
        && (map
            .body_insertion
            .as_ref()
            .is_some_and(|boundary| boundary.from == at.from)
            || map.literal_regions.iter().any(|region| {
                region.range.writable && at.from >= region.range.from && at.from <= region.range.to
            }));
    if exact && available {
        Ok(())
    } else {
        Err(crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_insertion_boundary_uncertified(),
        ))
    }
}

fn apply_note_edits(
    source: &str,
    mut edits: Vec<(std::ops::Range<usize>, String)>,
) -> Result<String, crate::EngineError> {
    edits.sort_by_key(|(range, _)| (range.start, range.end));
    if edits.windows(2).any(|pair| pair[0].0.end > pair[1].0.start) {
        return Err(crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_note_patches_overlap(),
        ));
    }
    let mut output = source.to_string();
    for (range, replacement) in edits.into_iter().rev() {
        if output.get(range.clone()).is_none() {
            return Err(crate::EngineError::new(
                weftext_messages::UiMessage::error_engine_notes_note_range_stale(),
            ));
        }
        output.replace_range(range, &replacement);
    }
    if !Source::detached(output.clone())
        .root()
        .errors_and_warnings()
        .0
        .is_empty()
    {
        return Err(crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_note_syntax_check_failed(),
        ));
    }
    Ok(output)
}

fn footnote_relations_closed(source: &str, map: &NotesSourceMap) -> bool {
    footnote_relations_closed_qualified(source, map, false)
}

fn footnote_binding_available(source: &str, frozen_module: bool) -> bool {
    standard_binding_available(source, "footnote")
        || (frozen_module && endnotes_import(source) && notes_binding_available(source, "footnote"))
}

fn footnote_relations_closed_qualified(
    source: &str,
    map: &NotesSourceMap,
    frozen_module: bool,
) -> bool {
    if map.entries.iter().any(|entry| {
        (entry.kind == "reference" && entry.target.is_none())
            || (entry.kind == "footnote-definition" && entry.body.is_none())
    }) {
        return false;
    }
    let parsed = Source::detached(source);
    let header = crate::numbering::verified_numbering_header_range(source);
    // The caller has authenticated the module bytes in this exact frozen
    // World. The source certificate admits one direct, unaliased import only.
    let module_import = (frozen_module && endnotes_import(source))
        .then(|| {
            parsed
                .root()
                .children()
                .find(|node| node.cast::<ast::ModuleImport>().is_some())
                .and_then(|node| parsed.find(node.span()))
                .map(|node| node.range())
        })
        .flatten();
    fn safe(
        node: &typst_syntax::SyntaxNode,
        parsed: &Source,
        source: &str,
        header: Option<&std::ops::Range<usize>>,
        module_import: Option<&std::ops::Range<usize>>,
    ) -> bool {
        if parsed.find(node.span()).is_some_and(|node| {
            header.is_some_and(|header| {
                node.range().start >= header.start && node.range().end <= header.end
            })
        }) {
            return true;
        }
        if node.kind() == SyntaxKind::ModuleImport
            && parsed
                .find(node.span())
                .is_some_and(|node| module_import.is_some_and(|import| node.range() == *import))
        {
            return true;
        }
        if matches!(
            node.kind(),
            SyntaxKind::ModuleImport
                | SyntaxKind::ModuleInclude
                | SyntaxKind::LetBinding
                | SyntaxKind::Closure
                | SyntaxKind::Contextual
                | SyntaxKind::Conditional
                | SyntaxKind::ForLoop
                | SyntaxKind::WhileLoop
                | SyntaxKind::ShowRule
        ) {
            return false;
        }
        if let Some(call) = node.cast::<ast::FuncCall>() {
            let name = call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .map(|name| name.get().to_string())
                .or_else(|| {
                    call.callee()
                        .to_untyped()
                        .cast::<ast::FieldAccess>()
                        .and_then(|field| field.target().to_untyped().cast::<ast::Ident>())
                        .map(|name| name.get().to_string())
                });
            let allowed = name.as_deref().is_some_and(|name| {
                matches!(
                    name,
                    "footnote"
                        | "ref"
                        | "figure"
                        | "image"
                        | "table"
                        | "grid"
                        | "text"
                        | "strong"
                        | "emph"
                        | "strike"
                        | "link"
                        | "raw"
                        | "box"
                        | "block"
                        | "align"
                        | "pad"
                        | "place"
                        | "h"
                        | "v"
                        | "math"
                        | "heading"
                        | "title"
                        | "metadata"
                        | "cite"
                        | "bibliography"
                        | "linebreak"
                        | "parbreak"
                        | "pagebreak"
                ) && if module_import.is_some() {
                    notes_binding_available(source, name)
                } else {
                    standard_binding_available(source, name)
                } || (module_import.is_some()
                    && matches!(name, "endnote" | "endnote-ref" | "endnotes")
                    && call.callee().to_untyped().cast::<ast::Ident>().is_some())
            });
            // In math syntax, standard mathematical functions have MathIdent
            // callees. User bindings and unknown imports remain excluded from
            // this profile; reference-producing/dynamic evaluators stay closed.
            let math = call
                .callee()
                .to_untyped()
                .cast::<ast::MathIdent>()
                .is_some_and(|name| !matches!(name.get().as_str(), "footnote" | "ref" | "eval"));
            if !allowed && !math {
                return false;
            }
        }
        node.children()
            .all(|node| safe(node, parsed, source, header, module_import))
    }
    safe(
        parsed.root(),
        &parsed,
        source,
        header.as_ref(),
        module_import.as_ref(),
    )
}

fn caption_edits(
    source: &str,
    figure: &NoteSourceEntry,
    body: &NotesBody,
    position: Option<&str>,
    numbering: Option<&NotesNumbering>,
) -> Result<Vec<(std::ops::Range<usize>, String)>, crate::EngineError> {
    let parsed = Source::detached(source);
    let call = native_call_at(
        parsed.root(),
        &parsed,
        figure.construct.from + usize::from(figure.construct.raw.starts_with('#')),
    )
    .ok_or_else(|| {
        crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_caption_origin_stale(),
        )
    })?;
    let mut edits = vec![];
    let caption = named_argument(call, "caption")?;
    let body = body_source(body)?;
    if let Some(position) = position
        && !matches!(position, "top" | "bottom")
    {
        return Err(crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_caption_position_invalid(),
        ));
    }
    if let Some(caption) = caption {
        if let Some(slot) = figure.body.as_ref() {
            edits.push((slot.from..slot.to, body));
            if let Some(position) = position {
                if caption.to_untyped().cast::<ast::ContentBlock>().is_some() {
                    let range = parsed.find(caption.span()).unwrap().range();
                    edits.push((
                        range.start..range.start,
                        format!("figure.caption(position:{position})"),
                    ));
                } else if let Some(call) = caption
                    .to_untyped()
                    .cast::<ast::FuncCall>()
                    .filter(|call| is_caption_call(*call))
                {
                    edits.push(named_argument_edit(
                        call,
                        "position",
                        position.to_string(),
                        &parsed,
                        source,
                    )?);
                } else {
                    return Err(crate::EngineError::new(
                        weftext_messages::UiMessage::error_engine_notes_caption_slot_unavailable(),
                    ));
                }
            }
        } else {
            return Err(crate::EngineError::new(
                weftext_messages::UiMessage::error_engine_notes_caption_computed(),
            ));
        }
    } else {
        let content = match position {
            Some(position) => format!("figure.caption(position:{position})[{body}]"),
            None => format!("[{body}]"),
        };
        edits.push(named_argument_edit(
            call, "caption", content, &parsed, source,
        )?);
    }
    if let Some(numbering) = numbering {
        let value = match numbering {
            NotesNumbering::None => "none".into(),
            NotesNumbering::Pattern { pattern } if pattern.len() <= 128 && !pattern.is_empty() => {
                crate::encode_string(pattern)?
            }
            NotesNumbering::Pattern { .. } => {
                return Err(crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_caption_numbering_invalid(),
                ));
            }
        };
        edits.push(named_argument_edit(
            call,
            "numbering",
            value,
            &parsed,
            source,
        )?);
    }
    // Coalesce additions at one official argument boundary, preserving both.
    edits.sort_by_key(|(range, _)| (range.start, range.end));
    let mut merged: Vec<(std::ops::Range<usize>, String)> = vec![];
    for (range, replacement) in edits {
        if range.is_empty()
            && let Some((previous, value)) = merged.last_mut()
            && *previous == range
        {
            if value.starts_with('(')
                && value.ends_with(')')
                && replacement.starts_with('(')
                && replacement.ends_with(')')
            {
                value.pop();
                value.push(',');
                value.push_str(&replacement[1..]);
            } else {
                value.push_str(&replacement);
            }
        } else {
            merged.push((range, replacement));
        }
    }
    Ok(merged)
}

fn numbering_source(numbering: &NotesNumbering) -> Result<String, crate::EngineError> {
    match numbering {
        NotesNumbering::None => Ok("none".into()),
        NotesNumbering::Pattern { pattern } if !pattern.is_empty() && pattern.len() <= 128 => {
            crate::encode_string(pattern)
        }
        _ => Err(crate::EngineError::new(
            weftext_messages::UiMessage::error_engine_notes_numbering_pattern_invalid(),
        )),
    }
}

pub(crate) fn notes_binding_available(source: &str, name: &str) -> bool {
    let parsed = Source::detached(source);
    let mut imports = vec![];
    fn collect(
        node: &typst_syntax::SyntaxNode,
        parsed: &Source,
        name: &str,
        ranges: &mut Vec<std::ops::Range<usize>>,
    ) -> bool {
        if let Some(import) = node.cast::<ast::ModuleImport>() {
            if import.source().to_untyped().cast::<ast::Str>().is_none() {
                return false;
            }
            let safe = match import.imports() {
                Some(ast::Imports::Wildcard) => false,
                Some(ast::Imports::Items(items)) => {
                    items.iter().all(|item| item.bound_name().get() != name)
                }
                None => import
                    .new_name()
                    .map(|name| name.get().to_string())
                    .or_else(|| import.bare_name().ok().map(|name| name.to_string()))
                    .is_some_and(|bound| bound != name),
            };
            if !safe {
                return false;
            }
            if let Some(node) = parsed.find(import.span()) {
                ranges.push(node.range());
            }
            return true;
        }
        node.children()
            .all(|child| collect(child, parsed, name, ranges))
    }
    if !collect(parsed.root(), &parsed, name, &mut imports) {
        return false;
    }
    let mut sanitized = source.to_string();
    for range in imports.into_iter().rev() {
        let start = range.start
            - usize::from(range.start > 0 && source.as_bytes()[range.start - 1] == b'#');
        let spaces: String = source[start..range.end]
            .chars()
            .map(|character| {
                if character == '\n' || character == '\r' {
                    character
                } else {
                    ' '
                }
            })
            .collect();
        sanitized.replace_range(start..range.end, &spaces);
    }
    standard_binding_available(&sanitized, name)
}

pub(crate) fn endnotes_import(source: &str) -> bool {
    let parsed = Source::detached(source);
    let mut matched = vec![];
    for node in parsed.root().children() {
        if let Some(import) = node.cast::<ast::ModuleImport>()
            && import
                .source()
                .to_untyped()
                .cast::<ast::Str>()
                .is_some_and(|path| path.get() == "weftext-endnotes.typ")
            && let Some(ast::Imports::Items(items)) = import.imports()
        {
            let names: Vec<_> = items
                .iter()
                .map(|item| {
                    (
                        item.original_name().get().to_string(),
                        item.bound_name().get().to_string(),
                    )
                })
                .collect();
            if names.len() == 3
                && ["endnote", "endnote-ref", "endnotes"].iter().all(|name| {
                    names
                        .iter()
                        .filter(|(old, new)| old == name && new == name)
                        .count()
                        == 1
                })
            {
                let range = parsed.find(import.span()).unwrap().range();
                matched.push(
                    range.start
                        - usize::from(range.start > 0 && source.as_bytes()[range.start - 1] == b'#')
                        ..range.end,
                );
            }
        }
    }
    if matched.len() != 1 {
        return false;
    }
    let range = matched.pop().unwrap();
    let mut sanitized = source.to_string();
    sanitized.replace_range(range, " ");
    ["endnote", "endnote-ref", "endnotes", "box"]
        .iter()
        .all(|name| standard_binding_available(&sanitized, name))
}

pub(crate) fn trailing_endnotes_insertion(source: &str) -> Option<usize> {
    if !endnotes_import(source) {
        return None;
    }
    let parsed = Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() || has_unverified_notes_show(&parsed) {
        return None;
    }
    if inspect_notes_source(source)
        .entries
        .iter()
        .filter(|entry| entry.kind == "endnotes")
        .count()
        != 1
    {
        return None;
    }
    parsed.root().children().find_map(|node| {
        let call = node.cast::<ast::FuncCall>()?;
        if !call
            .callee()
            .to_untyped()
            .cast::<ast::Ident>()
            .is_some_and(|name| name.get() == "endnotes")
            || call.args().items().next().is_some()
        {
            return None;
        }
        let range = parsed.find(call.span())?.range();
        if !source[range.end..].trim().is_empty() {
            return None;
        }
        Some(
            range.start
                - usize::from(range.start > 0 && source.as_bytes()[range.start - 1] == b'#'),
        )
    })
}

fn native_call_at<'a>(
    node: &'a typst_syntax::SyntaxNode,
    parsed: &Source,
    start: usize,
) -> Option<ast::FuncCall<'a>> {
    if let Some(call) = node.cast::<ast::FuncCall>()
        && parsed
            .find(call.span())
            .is_some_and(|node| node.range().start == start)
    {
        return Some(call);
    }
    node.children()
        .find_map(|child| native_call_at(child, parsed, start))
}

fn is_caption_call(call: ast::FuncCall<'_>) -> bool {
    call.callee()
        .to_untyped()
        .cast::<ast::FieldAccess>()
        .is_some_and(|field| {
            field.field().get() == "caption"
                && field
                    .target()
                    .to_untyped()
                    .cast::<ast::Ident>()
                    .is_some_and(|name| name.get() == "figure")
        })
}

fn equation_source_node<'a>(node: &LinkedNode<'a>, start: usize) -> Option<LinkedNode<'a>> {
    let native = node.get().cast::<ast::Equation>().is_some()
        || node
            .get()
            .cast::<ast::FuncCall>()
            .is_some_and(is_equation_call);
    if native && node.range().start == start {
        return Some(node.clone());
    }
    node.children()
        .find_map(|node| equation_source_node(&node, start))
}

fn is_equation_call(call: ast::FuncCall<'_>) -> bool {
    call.callee()
        .to_untyped()
        .cast::<ast::FieldAccess>()
        .is_some_and(|field| {
            field.field().get() == "equation"
                && field
                    .target()
                    .to_untyped()
                    .cast::<ast::Ident>()
                    .is_some_and(|name| name.get() == "math")
        })
}

fn direct_equation_source(node: &LinkedNode<'_>, source: &str) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if matches!(
            ancestor.kind(),
            SyntaxKind::LetBinding
                | SyntaxKind::Closure
                | SyntaxKind::Contextual
                | SyntaxKind::Conditional
                | SyntaxKind::WhileLoop
                | SyntaxKind::ForLoop
                | SyntaxKind::ShowRule
                | SyntaxKind::Binary
                | SyntaxKind::Unary
                | SyntaxKind::SetRule
                | SyntaxKind::CodeBlock
        ) {
            return false;
        }
        if let Some(call) = ancestor.get().cast::<ast::FuncCall>() {
            let name = call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .map(|name| name.get().to_string())
                .or_else(|| {
                    call.callee()
                        .to_untyped()
                        .cast::<ast::FieldAccess>()
                        .and_then(|field| field.target().to_untyped().cast::<ast::Ident>())
                        .map(|name| name.get().to_string())
                });
            if !name.as_deref().is_some_and(|name| {
                matches!(
                    name,
                    "box" | "block" | "strong" | "emph" | "align" | "pad" | "figure"
                ) && notes_binding_available(source, name)
            }) {
                return false;
            }
        }
        parent = ancestor.parent();
    }
    true
}

fn named_argument<'a>(
    call: ast::FuncCall<'a>,
    key: &str,
) -> Result<Option<ast::Expr<'a>>, crate::EngineError> {
    let mut found = None;
    for argument in call.args().items() {
        if let ast::Arg::Spread(_) = argument {
            return Err(crate::EngineError::new(
                weftext_messages::UiMessage::error_engine_notes_argument_dynamic(),
            ));
        }
        if let ast::Arg::Named(named) = argument
            && named.name().get() == key
        {
            if found.is_some() {
                return Err(crate::EngineError::new(
                    weftext_messages::UiMessage::error_engine_notes_argument_duplicate(),
                ));
            }
            found = Some(named.expr());
        }
    }
    Ok(found)
}

fn named_argument_edit(
    call: ast::FuncCall<'_>,
    key: &str,
    value: String,
    parsed: &Source,
    source: &str,
) -> Result<(std::ops::Range<usize>, String), crate::EngineError> {
    if let Some(expression) = named_argument(call, key)? {
        return Ok((parsed.find(expression.span()).unwrap().range(), value));
    }
    let args = parsed
        .find(call.args().span())
        .ok_or_else(|| {
            crate::EngineError::new(
                weftext_messages::UiMessage::error_engine_notes_argument_boundary_unavailable(),
            )
        })?
        .range();
    if source[args.clone()].starts_with('(') {
        Ok((args.start + 1..args.start + 1, format!("{key}:{value},")))
    } else {
        Ok((args.start..args.start, format!("({key}:{value})")))
    }
}

pub fn inspect_notes_source(source: &str) -> NotesSourceMap {
    fn body_range(expr: ast::Expr<'_>, parsed: &Source, source: &str) -> Option<SourceRange> {
        if let Some(call) = expr
            .to_untyped()
            .cast::<ast::FuncCall>()
            .filter(|call| is_caption_call(*call))
        {
            let args: Vec<_> = call
                .args()
                .items()
                .filter_map(|arg| {
                    if let ast::Arg::Pos(expr) = arg {
                        Some(expr)
                    } else {
                        None
                    }
                })
                .collect();
            return (args.len() == 1)
                .then(|| body_range(args[0], parsed, source))
                .flatten();
        }
        let body = expr.to_untyped().cast::<ast::ContentBlock>()?;
        let range = parsed.find(body.span())?.range();
        Some(crate::editor::source_range(
            source,
            range.start + 1..range.end - 1,
        ))
    }
    fn visit(
        node: &LinkedNode<'_>,
        parsed: &Source,
        source: &str,
        show: bool,
        bindings: &[bool; 4],
        out: &mut Vec<NoteSourceEntry>,
    ) {
        let equation = node
            .get()
            .cast::<ast::Equation>()
            .filter(|equation| equation.block());
        let equation_call = node
            .get()
            .cast::<ast::FuncCall>()
            .filter(|call| is_equation_call(*call));
        if equation.is_some() || equation_call.is_some() {
            let mut range = node.range();
            if equation_call.is_some()
                && range.start > 0
                && source.as_bytes()[range.start - 1] == b'#'
            {
                range.start -= 1;
            }
            let body = if let Some(equation) = equation {
                parsed
                    .find(equation.body().to_untyped().span())
                    .map(|node| crate::editor::source_range(source, node.range()))
            } else {
                equation_call
                    .and_then(|call| {
                        call.args().items().find_map(|argument| match argument {
                            ast::Arg::Pos(expression) => Some(expression),
                            ast::Arg::Named(named) if named.name().get() == "body" => {
                                Some(named.expr())
                            }
                            _ => None,
                        })
                    })
                    .and_then(|expression| parsed.find(expression.span()))
                    .map(|node| crate::editor::source_range(source, node.range()))
            };
            let writable = !show
                && bindings[3]
                && direct_equation_source(node, source)
                && equation_call.is_none_or(|call| {
                    !call
                        .args()
                        .items()
                        .any(|arg| matches!(arg, ast::Arg::Spread(_)))
                });
            let mut construct = crate::editor::source_range(source, range);
            construct.writable = writable;
            let body = body.map(|mut body| {
                body.writable = writable;
                body
            });
            out.push(NoteSourceEntry {
                kind: "equation".into(),
                construct,
                body,
                label: None,
                target: None,
                writable,
                unavailable_kind: (!writable).then_some(NoteSourceUnavailableKind::Other),
                unavailable_reason: (!writable).then(|| {
                    weftext_messages::UiMessage::error_engine_reason_equation_origin_unverified()
                        .into()
                }),
            });
        }
        if let Some(reference) = node.get().cast::<ast::Ref>() {
            let mut parent = node.parent();
            let mut direct = true;
            while let Some(ancestor) = parent {
                if matches!(
                    ancestor.kind(),
                    SyntaxKind::LetBinding
                        | SyntaxKind::Closure
                        | SyntaxKind::Contextual
                        | SyntaxKind::Conditional
                        | SyntaxKind::WhileLoop
                        | SyntaxKind::ForLoop
                        | SyntaxKind::ShowRule
                        | SyntaxKind::FuncCall
                        | SyntaxKind::Binary
                        | SyntaxKind::Unary
                        | SyntaxKind::SetRule
                        | SyntaxKind::CodeBlock
                ) {
                    direct = false;
                    break;
                }
                parent = ancestor.parent();
            }
            out.push(NoteSourceEntry {
                kind: "reference".into(),
                construct: crate::editor::source_range(source, node.range()),
                body: None,
                label: None,
                target: Some(reference.target().into()),
                writable: false,
                unavailable_kind: Some(if direct && reference.supplement().is_some() {
                    NoteSourceUnavailableKind::ReferenceArgumentsComplex
                } else if direct {
                    NoteSourceUnavailableKind::Other
                } else {
                    NoteSourceUnavailableKind::ReferenceDynamic
                }),
                unavailable_reason: Some(
                    if direct && reference.supplement().is_some() {
                        weftext_messages::UiMessage::error_engine_reason_reference_arguments_complex()
                    } else if direct {
                        weftext_messages::UiMessage::error_engine_reason_reference_kind_proof_required()
                    } else {
                        weftext_messages::UiMessage::error_engine_reason_reference_origin_dynamic()
                    }
                    .into(),
                ),
            });
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|name| name.get() == "ref")
        {
            let args: Vec<_> = call.args().items().collect();
            let target = args.iter().find_map(|arg| {
                if let ast::Arg::Pos(expr) = arg {
                    expr.to_untyped()
                        .cast::<ast::Label>()
                        .map(|label| label.get().to_string())
                } else {
                    None
                }
            });
            let mut range = node.range();
            if range.start > 0 && source.as_bytes()[range.start - 1] == b'#' {
                range.start -= 1;
            }
            let mut parent = node.parent();
            let mut direct = true;
            while let Some(ancestor) = parent {
                if matches!(
                    ancestor.kind(),
                    SyntaxKind::LetBinding
                        | SyntaxKind::Closure
                        | SyntaxKind::Contextual
                        | SyntaxKind::Conditional
                        | SyntaxKind::ForLoop
                        | SyntaxKind::WhileLoop
                        | SyntaxKind::ShowRule
                        | SyntaxKind::FuncCall
                        | SyntaxKind::Binary
                        | SyntaxKind::Unary
                        | SyntaxKind::SetRule
                        | SyntaxKind::CodeBlock
                ) {
                    direct = false;
                    break;
                }
                parent = ancestor.parent();
            }
            let reason = if !direct {
                weftext_messages::UiMessage::error_engine_reason_reference_origin_dynamic()
            } else if target.is_none() {
                weftext_messages::UiMessage::error_engine_reason_reference_target_computed()
            } else if args.len() != 1 {
                weftext_messages::UiMessage::error_engine_reason_reference_arguments_complex()
            } else {
                weftext_messages::UiMessage::error_engine_reason_reference_kind_proof_required()
            };
            out.push(NoteSourceEntry {
                kind: "reference".into(),
                construct: crate::editor::source_range(source, range),
                body: None,
                label: None,
                target,
                writable: false,
                unavailable_kind: Some(if !direct {
                    NoteSourceUnavailableKind::ReferenceDynamic
                } else if args.len() != 1 {
                    NoteSourceUnavailableKind::ReferenceArgumentsComplex
                } else {
                    NoteSourceUnavailableKind::Other
                }),
                unavailable_reason: Some(reason.into()),
            });
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && let Some(name) = call.callee().to_untyped().cast::<ast::Ident>()
            && matches!(
                name.get().as_str(),
                "footnote" | "figure" | "endnote" | "endnote-ref" | "endnotes"
            )
        {
            let name = name.get();
            let args: Vec<_> = call.args().items().collect();
            let positions: Vec<_> = args
                .iter()
                .filter_map(|arg| {
                    if let ast::Arg::Pos(expr) = arg {
                        Some(*expr)
                    } else {
                        None
                    }
                })
                .collect();
            let module_note = matches!(name.as_str(), "endnote" | "endnote-ref" | "endnotes");
            let body = if name == "footnote" || name == "endnote" {
                positions
                    .first()
                    .and_then(|expr| body_range(*expr, parsed, source))
            } else {
                args.iter().find_map(|arg| {
                    if let ast::Arg::Named(named) = arg {
                        (named.name().get() == "caption")
                            .then(|| body_range(named.expr(), parsed, source))
                            .flatten()
                    } else {
                        None
                    }
                })
            };
            let target = if name == "footnote" {
                positions
                    .first()
                    .and_then(|expr| expr.to_untyped().cast::<ast::Label>())
                    .map(|label| label.get().to_string())
            } else if name == "endnote" {
                args.iter().find_map(|arg| {
                    if let ast::Arg::Named(named) = arg {
                        (named.name().get() == "id")
                            .then(|| {
                                named
                                    .expr()
                                    .to_untyped()
                                    .cast::<ast::Str>()
                                    .map(|id| id.get().to_string())
                            })
                            .flatten()
                    } else {
                        None
                    }
                })
            } else if name == "endnote-ref" {
                positions
                    .first()
                    .and_then(|expr| expr.to_untyped().cast::<ast::Str>())
                    .map(|id| id.get().to_string())
            } else {
                None
            };
            let label_node = node
                .next_sibling()
                .filter(|next| next.kind().is_trivia())
                .and_then(|mut next| {
                    while next.kind().is_trivia() {
                        next = next.next_sibling()?;
                    }
                    Some(next)
                })
                .or_else(|| node.next_sibling());
            let label = label_node
                .as_ref()
                .and_then(|n| n.get().cast::<ast::Label>())
                .map(|label| label.get().to_string());
            let mut range = node.range();
            if range.start > 0 && source.as_bytes()[range.start - 1] == b'#' {
                range.start -= 1;
            }
            if label.is_some() {
                range.end = label_node.unwrap().range().end;
            }
            if module_note {
                let mut ancestor = node.parent();
                while let Some(parent) = ancestor {
                    if let Some(wrapper) = parent.get().cast::<ast::FuncCall>() {
                        if wrapper
                            .callee()
                            .to_untyped()
                            .cast::<ast::Ident>()
                            .is_some_and(|name| name.get() == "box")
                        {
                            let args: Vec<_> = wrapper.args().items().collect();
                            if let [ast::Arg::Pos(expr)] = args.as_slice()
                                && let Some(content) = expr.to_untyped().cast::<ast::ContentBlock>()
                                && let Some(content_range) =
                                    parsed.find(content.span()).map(|node| node.range())
                                && source[content_range.start + 1..content_range.end - 1].trim()
                                    == format!("#{}", &source[node.range()])
                            {
                                range = parent.range();
                                if range.start > 0 && source.as_bytes()[range.start - 1] == b'#' {
                                    range.start -= 1;
                                }
                            }
                        }
                        break;
                    }
                    ancestor = parent.parent();
                }
            }
            let qualified = !show
                && bindings[if module_note {
                    2
                } else if name == "footnote" {
                    0
                } else {
                    1
                }]
                && !args.iter().any(|arg| matches!(arg, ast::Arg::Spread(_)))
                && (name == "figure" || name == "endnotes" || positions.len() == 1)
                && {
                    let mut parent = node.parent();
                    let mut direct = true;
                    while let Some(ancestor) = parent {
                        if matches!(
                            ancestor.kind(),
                            SyntaxKind::LetBinding
                                | SyntaxKind::Closure
                                | SyntaxKind::Contextual
                                | SyntaxKind::Conditional
                                | SyntaxKind::WhileLoop
                                | SyntaxKind::ForLoop
                                | SyntaxKind::ShowRule
                                | SyntaxKind::Binary
                                | SyntaxKind::Unary
                                | SyntaxKind::SetRule
                                | SyntaxKind::CodeBlock
                        ) {
                            direct = false;
                            break;
                        }
                        if ancestor.kind() == SyntaxKind::FuncCall
                            && !(module_note
                                && source.as_bytes().get(range.start) == Some(&b'#')
                                && ancestor.range().start == range.start + 1
                                && ancestor.range().end == range.end)
                        {
                            direct = false;
                            break;
                        }
                        parent = ancestor.parent();
                    }
                    direct
                };
            let writable = !module_note
                && qualified
                && (name == "figure" || body.is_some() || target.is_some());
            let requires_frozen = module_note
                && qualified
                && name != "endnotes"
                && target.as_ref().is_some_and(|id| !id.is_empty())
                && (name != "endnote" || body.is_some());
            let mut construct = crate::editor::source_range(source, range);
            construct.writable = writable;
            let body = body.map(|mut body| {
                body.writable = writable;
                body
            });
            out.push(NoteSourceEntry {
                kind: match name.as_str() {
                    "figure" => "figure",
                    "endnote" => "endnote-definition",
                    "endnote-ref" => "endnote-reference",
                    "endnotes" => "endnotes",
                    _ if target.is_some() => "footnote-reference",
                    _ => "footnote-definition",
                }
                .into(),
                construct,
                body,
                label,
                target,
                writable,
                unavailable_kind: (!writable).then_some(if requires_frozen && !show {
                    NoteSourceUnavailableKind::FrozenModuleRequired
                } else {
                    NoteSourceUnavailableKind::Other
                }),
                unavailable_reason: (!writable).then(|| {
                    if show {
                        weftext_messages::UiMessage::error_engine_reason_show_rule_unverified()
                    } else if requires_frozen {
                        weftext_messages::UiMessage::error_engine_reason_endnotes_frozen_module_required()
                    } else if !qualified {
                        weftext_messages::UiMessage::error_engine_reason_note_binding_unverified()
                    } else {
                        weftext_messages::UiMessage::error_engine_reason_note_body_computed()
                    }
                    .into()
                }),
            });
        }
        for child in node.children() {
            visit(&child, parsed, source, show, bindings, out);
        }
    }
    let parsed = Source::detached(source);
    let valid = parsed.root().errors_and_warnings().0.is_empty();
    let mut map = NotesSourceMap {
        source_hash: source_hash(source),
        syntax_valid: valid,
        entries: vec![],
    };
    if valid {
        let bindings = [
            notes_binding_available(source, "footnote"),
            notes_binding_available(source, "figure"),
            endnotes_import(source),
            notes_binding_available(source, "math"),
        ];
        visit(
            &LinkedNode::new(parsed.root()),
            &parsed,
            source,
            has_unverified_notes_show(&parsed),
            &bindings,
            &mut map.entries,
        );
    }
    map
}

pub(crate) fn has_unverified_notes_show(source: &Source) -> bool {
    fn visit(
        node: &typst_syntax::SyntaxNode,
        source: &Source,
        verified: Option<&std::ops::Range<usize>>,
    ) -> bool {
        if node.kind() == SyntaxKind::ShowRule {
            return !source.find(node.span()).is_some_and(|node| {
                verified.is_some_and(|range| {
                    node.range().start >= range.start && node.range().end <= range.end
                })
            });
        }
        node.children().any(|node| visit(node, source, verified))
    }
    let verified = crate::numbering::verified_numbering_header_range(source.text());
    visit(source.root(), source, verified.as_ref())
}
