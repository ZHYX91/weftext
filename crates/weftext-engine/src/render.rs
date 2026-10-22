use crate::{inspect, inspect::Utf16Offsets, types::*, world::LocalWorld};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
use typst::{
    Feature, Library, LibraryExt, World,
    diag::{Severity, SourceDiagnostic},
    foundations::{NativeElement, StyleChain, Value},
    introspection::{Introspector, MetadataElem, Tag},
    model::{Document, HeadingElem},
    text::{FontFamily, FontList, Lang, TextElem},
    utils::LazyHash,
};
use typst_html::{HtmlDocument, HtmlNode};
use typst_kit::fonts::{self, FontStore};
use typst_syntax::{DiagSpanKind, FileId, Span};

pub struct Engine {
    pub(crate) library: LazyHash<Library>,
    pub(crate) fonts: FontStore,
    pub(crate) families: Vec<String>,
}

/// The edit boundary receives compiler failure evidence from its true producer,
/// before successful-output product warnings can enter the public diagnostic list.
pub(crate) enum RenderCompilation {
    Compiled(RenderOutput),
    Rejected {
        output: RenderOutput,
        diagnostics: Vec<RawDiagnostic>,
        product_diagnostics: Vec<ErrorDescription>,
    },
}
impl RenderCompilation {
    pub(crate) fn into_output(self) -> RenderOutput {
        match self {
            Self::Compiled(output) | Self::Rejected { output, .. } => output,
        }
    }
}

impl Engine {
    pub fn new() -> Result<Self, EngineError> {
        Self::new_with_system_fonts(true)
    }

    pub fn new_with_system_fonts(system_fonts: bool) -> Result<Self, EngineError> {
        let mut fonts = FontStore::new();
        fonts.extend(fonts::embedded());
        fonts.extend(crate::fonts::cjk_fonts().flat_map(|bytes| {
            typst::text::Font::iter(typst::foundations::Bytes::new(bytes)).map(|font| {
                let info = font.info().clone();
                (font, info)
            })
        }));
        if system_fonts {
            fonts.extend(fonts::system());
        }
        let families: Vec<_> = fonts
            .book()
            .families()
            .map(|(name, _)| name.to_owned())
            .collect();
        let mut library = Library::builder()
            .with_features([Feature::Html].into_iter().collect())
            .build();
        let preferred = [
            "Noto Serif CJK SC",
            "Source Han Serif SC",
            "Microsoft YaHei",
            "SimSun",
            "New Computer Modern",
        ];
        let fallback: Vec<_> = preferred
            .into_iter()
            .filter(|name| fonts.book().contains_family(&name.to_lowercase()))
            .map(FontFamily::new)
            .collect();
        library.styles.set(TextElem::font, FontList(fallback));
        library.styles.set(TextElem::lang, Lang::CHINESE);
        crate::style_adapter::install(&mut library);
        Ok(Self {
            library: LazyHash::new(library),
            fonts,
            families,
        })
    }

    pub fn render(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
    ) -> Result<RenderOutput, EngineError> {
        let world = LocalWorld::new(&self.library, &self.fonts, source, path, root)?;
        self.render_world(source, world, crate::inspect_editor(source))
            .map(RenderCompilation::into_output)
    }

    pub fn render_with_style(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
        style: &crate::StyleModule,
    ) -> Result<RenderOutput, EngineError> {
        let world = LocalWorld::with_style(&self.library, &self.fonts, source, path, root, style)?;
        self.render_world(
            source,
            world,
            crate::inspect_editor_with_style(source, style),
        )
        .map(RenderCompilation::into_output)
    }

    pub(crate) fn render_world(
        &self,
        source: &str,
        world: LocalWorld<'_>,
        editor_map: crate::EditorMap,
    ) -> Result<RenderCompilation, EngineError> {
        let scope = format!("{:x}", Sha256::digest(source.as_bytes()));
        let mut source_info = inspect(source);
        source_info.body_insertion = editor_map.body_insertion.clone();
        if let Some(reason) = &editor_map.unavailable_reason {
            source_info.body_insertion = None;
            source_info.writer_status = "unavailable".into();
            source_info.writer_reason = Some(reason.clone());
        }
        let result = typst::compile::<HtmlDocument>(&world);
        // Existing Sources are immutable in this request after compilation.
        // Retain their shared text snapshots while projecting, indexed by the
        // exact FileId rather than a display path or equal author text.
        let sources = world.loaded_sources();
        let offsets: OriginOffsets<'_> = sources
            .iter()
            .map(|source| {
                (
                    source.id(),
                    (source.text(), Utf16Offsets::new(source.text())),
                )
            })
            .collect();
        let mut diagnostics: Vec<_> = result
            .warnings
            .iter()
            .map(|d| diagnostic_with_offsets(d, &world, Some(&offsets)))
            .collect();
        let mut raw_compiler_diagnostics = result
            .warnings
            .iter()
            .map(raw_compiler_diagnostic)
            .collect::<Vec<_>>();
        let mut compilation_failure = None;
        let mut tree = None;
        let mut html = String::new();
        let mut labels = BTreeMap::<String, CompiledLabel>::new();
        let mut products = None;
        let ok = match result.output {
            Ok(document) => {
                for content in document.introspector().query_labelled() {
                    if let Some(label) = content.label() {
                        let label = label.resolve().as_str().to_string();
                        let entry = labels.entry(label.clone()).or_insert(CompiledLabel {
                            label,
                            count: 0,
                            origins: vec![],
                        });
                        entry.count += 1;
                        if let Some(origin) =
                            origin_with_offsets(content.span(), &world, Some(&offsets))
                        {
                            entry.origins.push(origin);
                        }
                    }
                }
                if let Some(title) = &document.info().title {
                    source_info.title = Some(title.to_string());
                }
                let mut counter = 0;
                let mut native_headings = vec![];
                let endnote_sources = endnote_producers(&document, &world, &mut diagnostics);
                let mut note_producers = NoteProducerState {
                    sources: &endnote_sources,
                    offsets: &offsets,
                    definitions: HashSet::new(),
                    reference: None,
                    product_warnings: &mut diagnostics,
                };
                let mut projected = project(
                    document.root_node(),
                    &world,
                    &mut counter,
                    &mut native_headings,
                    0,
                    &scope,
                    &mut note_producers,
                )
                .map_err(|error| error.with_product_diagnostics(world.product_diagnostics()))?;
                qualify_run_in_roles(&mut projected);
                qualify_managed_numbering(&mut projected, source, &scope, &world);
                qualify_subtitle_role(
                    &mut projected,
                    labels
                        .get("weftext-subtitle")
                        .map_or(0, |label| label.count),
                );
                if !endnote_sources.is_empty() {
                    qualify_endnote_origins(
                        &mut projected,
                        &mut source_info.notes,
                        &document,
                        &world,
                        &offsets,
                    );
                }
                let mut occurrences = HashMap::new();
                count_origins(&projected, &mut occurrences);
                qualify(&mut projected, &editor_map, &occurrences);
                qualify_note_writers(&projected, &mut source_info.notes, &world);
                // Query the successful target's real native introspector. This
                // includes headings in same-target frame islands and excludes
                // unevaluated author AST branches.
                native_headings = document
                    .introspector()
                    .query(&HeadingElem::ELEM.select())
                    .iter()
                    .filter_map(|content| {
                        let heading = content.to_packed::<HeadingElem>()?;
                        let origin = origin_with_offsets(content.span(), &world, Some(&offsets));
                        Some(HeadingInfo {
                            text: heading.body.plain_text().to_string(),
                            path: origin.as_ref().map(|o| o.path.clone()),
                            is_main: origin.as_ref().is_some_and(|o| o.is_main),
                            level: heading
                                .resolve_level(StyleChain::new(&world.library.styles))
                                .get(),
                            from: origin.as_ref().map_or(0, |o| o.from),
                            to: origin.as_ref().map_or(0, |o| o.to),
                            utf16_from: origin.as_ref().map_or(0, |o| o.utf16_from),
                            utf16_to: origin.as_ref().map_or(0, |o| o.utf16_to),
                            generated: origin
                                .as_ref()
                                .is_none_or(|o| !o.is_main || o.syntax_kind != "Heading"),
                            visible: false,
                        })
                    })
                    .collect();
                // Native final heading tags retain actual resolved levels, even
                // for generated headings. Their bodies are semantic metadata,
                // never inserted into the displayed consumer tree.
                if !native_headings.is_empty() {
                    for heading in &mut native_headings {
                        heading.visible = contains_range(
                            &projected,
                            heading.path.as_deref(),
                            heading.from,
                            heading.to,
                        );
                    }
                    source_info.headings = native_headings.clone();
                } else {
                    for heading in &mut source_info.headings {
                        heading.visible =
                            contains_range(&projected, None, heading.from, heading.to);
                    }
                }
                html = safe_html(&projected);
                products = Some(crate::products::native_products(
                    document.root_node(),
                    &projected,
                    document.info().title.as_ref().map(ToString::to_string),
                    native_headings,
                ));
                tree = Some(projected);
                true
            }
            Err(errors) => {
                raw_compiler_diagnostics.extend(errors.iter().map(raw_compiler_diagnostic));
                compilation_failure = Some(raw_compiler_diagnostics);
                diagnostics.extend(
                    errors
                        .iter()
                        .map(|d| diagnostic_with_offsets(d, &world, Some(&offsets))),
                );
                false
            }
        };
        let product_diagnostics = world.product_diagnostics();
        diagnostics.extend(product_diagnostics.iter().map(|description| Diagnostic {
            severity: if ok { "warning" } else { "error" }.into(),
            explanation: Some(description.explanation.clone()),
            diagnostics: description.diagnostics.clone(),
            origin: None,
        }));
        let output = RenderOutput {
            ok,
            html,
            tree,
            diagnostics,
            source_info,
            dependencies: world.dependencies(),
            font_families: self.families.clone(),
            compiler_version: "0.15.1".into(),
            labels: labels.into_values().collect(),
            products,
        };
        Ok(match compilation_failure {
            Some(diagnostics) => RenderCompilation::Rejected {
                output,
                diagnostics,
                product_diagnostics,
            },
            None => RenderCompilation::Compiled(output),
        })
    }

    pub fn compile(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
    ) -> Result<RenderOutput, EngineError> {
        self.render(source, path, root)
    }
}

pub(crate) fn diagnostic(d: &SourceDiagnostic, world: &LocalWorld<'_>) -> Diagnostic {
    diagnostic_with_offsets(d, world, None)
}
fn raw_compiler_diagnostic(diagnostic: &SourceDiagnostic) -> RawDiagnostic {
    RawDiagnostic {
        origin: DiagnosticOrigin::Typst,
        text: diagnostic.message.to_string(),
    }
}

type OriginOffsets<'a> = HashMap<FileId, (&'a str, Utf16Offsets<'a>)>;

fn origin_utf16(
    source: &typst_syntax::Source,
    at: usize,
    offsets: Option<&OriginOffsets<'_>>,
) -> usize {
    offsets
        .and_then(|offsets| offsets.get(&source.id()))
        // Source::clone shares its immutable Arc representation. If a future
        // caller supplies a replacement Source, preserve the original path
        // through prefix conversion, without a full-text equality scan here.
        .filter(|(snapshot, _)| std::ptr::eq(*snapshot, source.text()))
        .map_or_else(|| utf16(source.text(), at), |(_, offsets)| offsets.at(at))
}

fn diagnostic_with_offsets(
    d: &SourceDiagnostic,
    world: &LocalWorld<'_>,
    offsets: Option<&OriginOffsets<'_>>,
) -> Diagnostic {
    let origin = d.span.id().and_then(|id| {
        let source = world.source(id).ok()?;
        let range = match d.span.get() {
            DiagSpanKind::Number { num, sub_range, .. } => source.range(num, sub_range)?,
            DiagSpanKind::Range { range, .. } => range,
            DiagSpanKind::Detached => return None,
        };
        Some(Origin {
            path: world.display_path(id),
            from: range.start,
            to: range.end,
            utf16_from: origin_utf16(&source, range.start, offsets),
            utf16_to: origin_utf16(&source, range.end, offsets),
            raw: source.text().get(range)?.into(),
            syntax_kind: "Diagnostic".into(),
            is_main: id == world.main_id,
        })
    });
    Diagnostic {
        severity: match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
        .into(),
        explanation: None,
        diagnostics: vec![RawDiagnostic {
            origin: DiagnosticOrigin::Typst,
            text: d.message.to_string(),
        }],
        origin,
    }
}

pub(crate) fn origin(span: Span, world: &LocalWorld<'_>) -> Option<Origin> {
    origin_with_offsets(span, world, None)
}

fn origin_with_offsets(
    span: Span,
    world: &LocalWorld<'_>,
    offsets: Option<&OriginOffsets<'_>>,
) -> Option<Origin> {
    let id = span.id()?;
    let source = world.source(id).ok()?;
    let node = source.find(span)?;
    let range = node.range();
    Some(Origin {
        path: world.display_path(id),
        from: range.start,
        to: range.end,
        utf16_from: origin_utf16(&source, range.start, offsets),
        utf16_to: origin_utf16(&source, range.end, offsets),
        raw: source.text().get(range)?.into(),
        syntax_kind: format!("{:?}", node.kind()),
        is_main: id == world.main_id,
    })
}

/// Certify only the product's pinned, actually loaded native module. A custom
/// show may clone/mutate HTML elements while retaining their source spans, so
/// that broader valid Typst profile stays readable but gains no exchange role.
fn endnote_producers(
    document: &HtmlDocument,
    world: &LocalWorld<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> HashSet<FileId> {
    const MODULE: &str = include_str!("../../weftext-export/assets/weftext-endnotes.typ");
    let sources = world.loaded_sources();
    let ids: HashSet<_> = sources
        .iter()
        .filter(|source| source.text() == MODULE)
        .map(|source| source.id())
        .collect();
    if ids.is_empty() {
        if crate::notes::endnotes_import(world.main_source.text()) {
            diagnostics.push(Diagnostic {
                severity: "warning".into(),
                explanation: Some(UiMessage::error_engine_render_endnotes_module_uncertified()),
                diagnostics: vec![],
                origin: None,
            });
        }
        return ids;
    }
    let ambiguous = sources.iter().any(|source| {
        crate::notes::has_unverified_notes_show(source)
            && !world.trusted_style_sources.contains(&source.id())
    }) || document
        .introspector()
        .query(&MetadataElem::ELEM.select())
        .iter()
        .any(|content| {
            content.label().is_some_and(|label| {
                matches!(
                    label.resolve().as_str(),
                    "weftext-endnote-definition"
                        | "weftext-endnote-reference"
                        | "weftext-endnote-body"
                        | "weftext-endnotes-list"
                )
            }) && content.span().id().is_none_or(|id| !ids.contains(&id))
        });
    if ambiguous {
        diagnostics.push(Diagnostic {
            severity: "warning".into(),
            explanation: Some(UiMessage::error_engine_render_endnotes_role_ambiguous()),
            diagnostics: vec![],
            origin: None,
        });
        HashSet::new()
    } else {
        ids
    }
}

fn qualify_managed_numbering(
    tree: &mut RenderNode,
    source: &str,
    source_hash: &str,
    world: &LocalWorld<'_>,
) {
    let pinned = crate::notes::pinned_endnotes_from_loaded_world(source, world);
    let inspection = crate::numbering::inspect_heading_numbering_qualified(source, pinned.as_ref());
    let Some(settings) = inspection
        .settings
        .filter(|_| inspection.editable && inspection.source_hash == source_hash)
    else {
        return;
    };
    let pattern = match settings.scheme {
        crate::HeadingNumberingScheme::Decimal => "1.1",
        crate::HeadingNumberingScheme::Chinese => "一.一",
        crate::HeadingNumberingScheme::None => return,
    };
    let Some(header) = crate::numbering::verified_numbering_header_range(source) else {
        return;
    };
    fn visit(
        node: &mut RenderNode,
        settings: &crate::HeadingNumberingIntent,
        pattern: &str,
        hash: &str,
        header: &std::ops::Range<usize>,
    ) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "heading")
            && node
                .attrs
                .get("data-weftext-numbering-kind")
                .is_some_and(|kind| kind == "function")
            && node.origin.as_ref().is_some_and(|origin| origin.is_main)
            && node
                .attrs
                .get("data-weftext-numbering-function-from")
                .and_then(|from| from.parse::<usize>().ok())
                .is_some_and(|from| from >= header.start)
            && node
                .attrs
                .get("data-weftext-numbering-function-to")
                .and_then(|to| to.parse::<usize>().ok())
                .is_some_and(|to| to <= header.end)
        {
            let visible = node
                .attrs
                .get("data-weftext-level")
                .and_then(|level| level.parse::<u32>().ok())
                .is_some_and(|level| level <= settings.max_level)
                && !node
                    .attrs
                    .get("data-weftext-label")
                    .is_some_and(|label| settings.exclude_labels.contains(label));
            node.attrs.insert(
                "data-weftext-managed-numbering-visible".into(),
                visible.to_string(),
            );
            node.attrs.insert(
                "data-weftext-managed-numbering-source-hash".into(),
                hash.into(),
            );
            if visible {
                node.attrs.insert(
                    "data-weftext-managed-numbering-pattern".into(),
                    pattern.into(),
                );
            }
        }
        for child in &mut node.children {
            visit(child, settings, pattern, hash, header);
        }
    }
    visit(tree, &settings, pattern, source_hash, &header);
}

fn qualify_note_writers(
    tree: &RenderNode,
    notes: &mut crate::NotesSourceMap,
    world: &LocalWorld<'_>,
) {
    let unknown_show = world.loaded_sources().iter().any(|source| {
        crate::notes::has_unverified_notes_show(source)
            && !world.trusted_style_sources.contains(&source.id())
    });
    fn collect(node: &RenderNode, counts: &mut HashMap<(String, usize), Vec<usize>>) {
        if let Some(role) = node
            .attrs
            .get("data-weftext-role")
            .filter(|role| matches!(role.as_str(), "footnote-reference" | "figure" | "equation"))
            && let Some(origin) = node.origin.as_ref().filter(|origin| origin.is_main)
        {
            counts
                .entry((role.clone(), origin.from))
                .or_default()
                .push(origin.to);
        }
        for child in &node.children {
            collect(child, counts);
        }
    }
    let mut counts = HashMap::new();
    collect(tree, &mut counts);
    for entry in &mut notes.entries {
        if !entry.writable {
            continue;
        }
        let role = match entry.kind.as_str() {
            "footnote-definition" | "footnote-reference" => Some("footnote-reference"),
            "figure" => Some("figure"),
            "equation" => Some("equation"),
            _ => None,
        };
        let unique = role.is_none_or(|role| {
            counts
                .get(&(
                    role.into(),
                    entry.construct.from + usize::from(entry.construct.raw.starts_with('#')),
                ))
                .is_some_and(|ends| {
                    ends.iter()
                        .filter(|end| **end <= entry.construct.to)
                        .count()
                        == 1
                })
        });
        if unknown_show || !unique {
            entry.writable = false;
            entry.construct.writable = false;
            if let Some(body) = &mut entry.body {
                body.writable = false;
            }
            entry.unavailable_kind = Some(crate::notes::NoteSourceUnavailableKind::Other);
            entry.unavailable_reason = Some(
                if unknown_show {
                    UiMessage::error_engine_reason_show_rule_unverified()
                } else {
                    UiMessage::error_engine_reason_native_origin_nonunique_or_undisplayed()
                }
                .into(),
            );
        }
    }
}

fn qualify_endnote_origins(
    tree: &mut RenderNode,
    notes: &mut crate::NotesSourceMap,
    document: &HtmlDocument,
    world: &LocalWorld<'_>,
    offsets: &OriginOffsets<'_>,
) {
    // The native module's actual definition metadata retains the original
    // Content value and its author span, including for unboxed imported notes.
    // The id is the module's real relationship key, never rendered text.
    for content in document.introspector().query(&MetadataElem::ELEM.select()) {
        let Some(metadata) = content.to_packed::<MetadataElem>() else {
            continue;
        };
        let Value::Dict(value) = &metadata.value else {
            continue;
        };
        let Ok(Value::Str(kind)) = value.get("kind") else {
            continue;
        };
        if kind.as_str() != "definition"
            || content
                .label()
                .is_none_or(|label| label.resolve().as_str() != "weftext-endnote-definition")
        {
            continue;
        }
        let (Ok(Value::Str(id)), Ok(Value::Content(body))) = (value.get("id"), value.get("body"))
        else {
            continue;
        };
        let Some(actual) =
            origin_with_offsets(body.span(), world, Some(offsets)).filter(|origin| origin.is_main)
        else {
            continue;
        };
        let matching: Vec<_> = notes
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.kind == "endnote-definition"
                    && entry.target.as_deref() == Some(id.as_str())
                    && entry.unavailable_kind
                        == Some(crate::NoteSourceUnavailableKind::FrozenModuleRequired)
                    && entry.body.as_ref().is_some_and(|slot| {
                        actual.from >= slot.from.saturating_sub(1) && actual.to <= slot.to + 1
                    })
            })
            .map(|(index, _)| index)
            .collect();
        if let [index] = matching.as_slice() {
            let entry = &mut notes.entries[*index];
            entry.writable = true;
            entry.construct.writable = true;
            entry.unavailable_reason = None;
            entry.unavailable_kind = None;
            if let Some(body) = &mut entry.body {
                body.writable = true;
            }
        }
    }
    fn assign_reference(
        node: &mut RenderNode,
        entry: &crate::NoteSourceEntry,
        origin: &Origin,
    ) -> usize {
        let mut count = 0;
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "endnote-reference")
            && node.attrs.get("data-weftext-note-id") == entry.target.as_ref()
        {
            let mut actual = origin.clone();
            actual.from = entry.construct.from;
            actual.to = entry.construct.to;
            actual.utf16_from = entry.construct.utf16_from;
            actual.utf16_to = entry.construct.utf16_to;
            actual.raw = entry.construct.raw.clone();
            actual.syntax_kind = "CertifiedEndnoteBox".into();
            node.origin = Some(actual);
            count += 1;
        }
        for child in &mut node.children {
            count += assign_reference(child, entry, origin);
        }
        count
    }
    fn visit(node: &mut RenderNode, notes: &mut crate::NotesSourceMap) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "source-box")
            && let Some(origin) = node.origin.clone().filter(|origin| origin.is_main)
            && let Some(entry) = notes.entries.iter_mut().find(|entry| {
                entry.kind.starts_with("endnote-")
                    && (entry.unavailable_kind
                        == Some(crate::NoteSourceUnavailableKind::FrozenModuleRequired)
                        || entry.writable)
                    && entry.construct.raw.starts_with("#box[")
                    && entry.construct.from + 1 == origin.from
                    && entry.construct.to == origin.to
            })
            && assign_reference(node, entry, &origin) == 1
        {
            entry.writable = true;
            entry.construct.writable = true;
            entry.unavailable_reason = None;
            entry.unavailable_kind = None;
            if let Some(body) = &mut entry.body {
                body.writable = true;
            }
        }
        for child in &mut node.children {
            visit(child, notes);
        }
    }
    visit(tree, notes);
    // A declaration's author Content span survives as its actual body text.
    // It is safe to expose that slot only once; module source locations are
    // never repurposed as main-source addresses.
    fn bodies(node: &mut RenderNode, notes: &crate::NotesSourceMap, main_source_path: &str) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "endnote-body")
            && let Some(id) = node.attrs.get("data-weftext-note-id")
        {
            let entries: Vec<_> = notes
                .entries
                .iter()
                .filter(|entry| {
                    entry.kind == "endnote-definition"
                        && entry.target.as_ref() == Some(id)
                        && entry.writable
                })
                .collect();
            if let [entry] = entries.as_slice()
                && let Some(body) = &entry.body
            {
                node.origin = Some(Origin {
                    path: main_source_path.into(),
                    from: body.from,
                    to: body.to,
                    utf16_from: body.utf16_from,
                    utf16_to: body.utf16_to,
                    raw: body.raw.clone(),
                    syntax_kind: "CertifiedEndnoteBody".into(),
                    is_main: true,
                });
            }
        }
        for child in &mut node.children {
            bodies(child, notes, main_source_path);
        }
    }
    bodies(tree, notes, &world.display_path(world.main_id));
}

struct NoteProducerState<'a> {
    sources: &'a HashSet<FileId>,
    offsets: &'a OriginOffsets<'a>,
    definitions: HashSet<String>,
    reference: Option<(String, bool)>,
    product_warnings: &'a mut Vec<Diagnostic>,
}

fn project(
    node: &HtmlNode,
    world: &LocalWorld<'_>,
    counter: &mut usize,
    headings: &mut Vec<HeadingInfo>,
    depth: usize,
    scope: &str,
    note_producers: &mut NoteProducerState<'_>,
) -> Result<RenderNode, EngineError> {
    *counter += 1;
    if *counter > 100_000 || depth > 256 {
        return Err(EngineError::DisplayTreeBudget {
            product_diagnostics: vec![],
        });
    }
    let mut out = RenderNode {
        key: format!("{scope}-n{counter}"),
        kind: String::new(),
        tag: None,
        text: None,
        attrs: BTreeMap::new(),
        children: vec![],
        origin: origin_with_offsets(node.span(), world, Some(note_producers.offsets)),
        writable: false,
    };
    match node {
        HtmlNode::Text(text, _) => {
            out.kind = "text".into();
            out.text = Some(text.to_string());
        }
        HtmlNode::Element(element) => {
            let tag = element.tag.resolve().as_str().to_owned();
            for warning in &element.weftext.warnings {
                let explanation = match warning.kind {
                    typst_html::WeftextFlowWarningKind::NonSolidTextPaint => {
                        UiMessage::error_engine_render_non_solid_text_paint()
                    }
                    typst_html::WeftextFlowWarningKind::TableStrokeUnsupported => {
                        UiMessage::error_engine_render_table_stroke_unsupported()
                    }
                    typst_html::WeftextFlowWarningKind::NonSolidTableFill => {
                        UiMessage::error_engine_render_non_solid_table_fill()
                    }
                    typst_html::WeftextFlowWarningKind::DynamicListMarker => {
                        UiMessage::error_engine_render_dynamic_list_marker()
                    }
                };
                note_producers.product_warnings.push(Diagnostic {
                    severity: "warning".into(),
                    explanation: Some(explanation),
                    diagnostics: vec![],
                    origin: origin_with_offsets(warning.span, world, Some(note_producers.offsets)),
                });
            }
            out.kind = "element".into();
            if matches!(tag.as_str(), "head" | "meta" | "title" | "link" | "base") {
                // The compiler's document head is metadata, not a displayed
                // consumer subtree. Native Typst title body is a separate
                // ordinary content element and remains in the body.
                out.kind = "tag".into();
                return Ok(out);
            }
            if matches!(
                tag.as_str(),
                "script"
                    | "style"
                    | "iframe"
                    | "object"
                    | "embed"
                    | "form"
                    | "input"
                    | "button"
                    | "textarea"
                    | "select"
            ) {
                // Preserve the source and show no executable/resource-fetching
                // subtree. Ordinary Typst output uses the semantic allowlist.
                out.tag = Some("span".into());
                out.attrs
                    .insert("class".into(), "weftext-unsupported".into());
                note_producers.product_warnings.push(Diagnostic {
                    severity: "warning".into(),
                    explanation: Some(UiMessage::error_engine_render_html_element_unsupported(
                        weftext_messages::params::ErrorEngineRenderHtmlElementUnsupported {
                            tag: tag.clone(),
                        },
                    )),
                    diagnostics: vec![],
                    origin: out.origin.clone(),
                });
                return Ok(out);
            }
            out.tag = Some(if allowed_tag(&tag) {
                tag.clone()
            } else {
                "span".into()
            });
            for (key, value) in &element.attrs.0 {
                let name = key.resolve().as_str().to_owned();
                if let Some(value) = safe_attr(&name, value.as_str(), &tag) {
                    out.attrs.insert(name, value);
                }
            }
            // Semantic attributes come only from the native producer's typed
            // side information, never from author-controlled HTML attributes.
            if let Some(role) = element.weftext.role {
                if role == "strong" {
                    out.attrs
                        .insert("data-weftext-producer".into(), role.into());
                } else {
                    out.attrs.insert("data-weftext-role".into(), role.into());
                }
            }
            if let Some(level) = element.weftext.level {
                out.attrs
                    .insert("data-weftext-level".into(), level.to_string());
            }
            if let Some(repeat) = element.weftext.repeat {
                out.attrs
                    .insert("data-weftext-repeat".into(), repeat.to_string());
            }
            if let Some(numbered) = element.weftext.numbered {
                out.attrs
                    .insert("data-weftext-numbered".into(), numbered.to_string());
            }
            if let Some(kind) = element.weftext.numbering_kind {
                out.attrs
                    .insert("data-weftext-numbering-kind".into(), kind.into());
            }
            if let Some(pattern) = &element.weftext.numbering_pattern {
                out.attrs
                    .insert("data-weftext-numbering-pattern".into(), pattern.to_string());
            }
            if let Some(function) = element
                .weftext
                .numbering_function_span
                .and_then(|span| origin_with_offsets(span, world, Some(note_producers.offsets)))
                .filter(|origin| origin.is_main)
            {
                out.attrs.insert(
                    "data-weftext-numbering-function-from".into(),
                    function.from.to_string(),
                );
                out.attrs.insert(
                    "data-weftext-numbering-function-to".into(),
                    function.to.to_string(),
                );
            }
            if let Some(state) = &element.weftext.counter_state {
                out.attrs
                    .insert("data-weftext-counter-state".into(), state.to_string());
            }
            if let Some(reference_kind) = element.weftext.reference_kind {
                out.attrs
                    .insert("data-weftext-reference-kind".into(), reference_kind.into());
            }
            if let Some(label) = &element.weftext.label {
                out.attrs
                    .insert("data-weftext-label".into(), label.to_string());
            }
            for (name, value) in [
                ("data-weftext-note-id", element.weftext.note_id.as_ref()),
                ("data-weftext-target-id", element.weftext.target_id.as_ref()),
                (
                    "data-weftext-caption-kind",
                    element.weftext.caption_kind.as_ref(),
                ),
            ] {
                if let Some(value) = value {
                    out.attrs.insert(name.into(), value.to_string());
                }
            }
            if let Some(position) = element.weftext.caption_position {
                out.attrs
                    .insert("data-weftext-caption-position".into(), position.into());
            }
            if let Some(definition) = element.weftext.note_definition {
                out.attrs.insert(
                    "data-weftext-note-definition".into(),
                    definition.to_string(),
                );
            }
            if element.weftext.role == Some("citation") {
                out.attrs.insert(
                    "data-weftext-citation-keys".into(),
                    serde_json::to_string(&element.weftext.citation_keys).unwrap(),
                );
                out.attrs.insert(
                    "data-weftext-citation-forms".into(),
                    serde_json::to_string(&element.weftext.citation_forms).unwrap(),
                );
                let origins: Vec<_> = element
                    .weftext
                    .citation_spans
                    .iter()
                    .map(|span| origin_with_offsets(*span, world, Some(note_producers.offsets)))
                    .collect();
                out.attrs.insert(
                    "data-weftext-citation-origins".into(),
                    serde_json::to_string(&origins).unwrap(),
                );
            }
            // The bundled module has no private Rust fields. Its exact native
            // call-site span and frozen bytes authenticate its explicit
            // producer protocol. Arbitrary author HTML and metadata cannot.
            if element
                .span
                .id()
                .is_some_and(|id| note_producers.sources.contains(&id))
            {
                let attr = |name: &str| {
                    element
                        .attrs
                        .0
                        .iter()
                        .find(|(key, _)| key.resolve().as_str() == name)
                        .map(|(_, value)| value.as_str())
                };
                let role = match attr("data-weftext-endnote-kind") {
                    Some("reference") => Some("endnote-reference"),
                    Some("body") => Some("endnote-body"),
                    Some("list") => Some("endnotes"),
                    _ => None,
                };
                if let Some(role) = role {
                    if role == "endnotes" {
                        out.attrs.insert("data-weftext-role".into(), role.into());
                    } else if let Some(id) = attr("data-weftext-endnote-id")
                        .filter(|id| !id.is_empty() && id.len() <= 512)
                    {
                        out.attrs.insert("data-weftext-role".into(), role.into());
                        out.attrs.insert("data-weftext-note-id".into(), id.into());
                        if role == "endnote-reference"
                            && let Some((actual_id, definition)) = note_producers.reference.take()
                            && actual_id == id
                        {
                            out.attrs.insert(
                                "data-weftext-note-definition".into(),
                                definition.to_string(),
                            );
                        }
                    }
                }
            }
            let css = safe_css(&element.css);
            if !css.is_empty() {
                out.attrs.insert("style".into(), css);
            }
            if element.pre_span {
                out.attrs.insert("class".into(), "weftext-pre".into());
            }
            if tag == "img" && !out.attrs.contains_key("src") {
                out.tag = Some("span".into());
                out.text = Some(format!(
                    "[图片：{}]",
                    out.attrs
                        .get("alt")
                        .map(String::as_str)
                        .unwrap_or("资源预览暂不可用")
                ));
            }
            for child in &element.children {
                out.children.push(project(
                    child,
                    world,
                    counter,
                    headings,
                    depth + 1,
                    scope,
                    note_producers,
                )?);
            }
        }
        HtmlNode::Tag(Tag::Start(content, _)) => {
            out.kind = "tag".into();
            // Actual metadata events from the exact module's fixed
            // definition -> _marker protocol preserve definition identity,
            // including forward references in unboxed standalone source.
            if content
                .span()
                .id()
                .is_some_and(|id| note_producers.sources.contains(&id))
                && let Some(metadata) = content.to_packed::<MetadataElem>()
                && let Value::Dict(value) = &metadata.value
                && let Ok(Value::Str(id)) = value.get("id")
                && let Some(label) = content.label()
            {
                match label.resolve().as_str() {
                    "weftext-endnote-definition" => {
                        note_producers.definitions.insert(id.as_str().into());
                    }
                    "weftext-endnote-reference" => {
                        note_producers.reference = Some((
                            id.as_str().into(),
                            note_producers.definitions.remove(id.as_str()),
                        ));
                    }
                    _ => {}
                }
            }
            // Keep the native metadata tag at its actual final-tree position.
            // The UI can attach derived Query presentation here, but this tag
            // never becomes body text, an exported result, or a source writer.
            if content.to_packed::<MetadataElem>().is_some()
                && let Some(label) = content.label()
                && label.resolve().as_str().starts_with("weftext-query-")
            {
                out.attrs.insert(
                    "data-weftext-query-label".into(),
                    label.resolve().as_str().into(),
                );
                if let Some(location) = content.location() {
                    out.attrs.insert(
                        "data-weftext-query-occurrence".into(),
                        format!("{:032x}", location.hash()),
                    );
                }
                out.origin =
                    origin_with_offsets(content.span(), world, Some(note_producers.offsets));
            }
            if let Some(heading) = content.to_packed::<HeadingElem>() {
                out.attrs.insert(
                    "data-weftext-native-heading-start".into(),
                    format!("{:032x}", content.location().unwrap().hash()),
                );
                out.attrs.insert(
                    "data-weftext-level".into(),
                    heading
                        .resolve_level(StyleChain::default())
                        .get()
                        .to_string(),
                );
                out.attrs.insert(
                    "data-weftext-numbered".into(),
                    heading
                        .numbering
                        .get_ref(StyleChain::default())
                        .is_some()
                        .to_string(),
                );
                let numbering = typst_html::WeftextHtmlSemantics::default()
                    .with_numbering(heading.numbering.get_ref(StyleChain::default()).as_ref());
                if let Some(kind) = numbering.numbering_kind {
                    out.attrs
                        .insert("data-weftext-numbering-kind".into(), kind.into());
                }
                if let Some(pattern) = numbering.numbering_pattern {
                    out.attrs
                        .insert("data-weftext-numbering-pattern".into(), pattern.to_string());
                }
                if let Some(label) = content.label() {
                    out.attrs
                        .insert("data-weftext-label".into(), label.resolve().as_str().into());
                }
                let origin =
                    origin_with_offsets(content.span(), world, Some(note_producers.offsets));
                let (from, to, uf, ut, generated) = origin
                    .as_ref()
                    .map(|o| {
                        (
                            o.from,
                            o.to,
                            o.utf16_from,
                            o.utf16_to,
                            !o.is_main || o.syntax_kind != "Heading",
                        )
                    })
                    .unwrap_or((0, 0, 0, 0, true));
                headings.push(HeadingInfo {
                    path: origin.as_ref().map(|o| o.path.clone()),
                    is_main: origin.as_ref().is_some_and(|o| o.is_main),
                    text: heading.body.plain_text().to_string(),
                    level: heading
                        .resolve_level(StyleChain::new(&world.library.styles))
                        .get(),
                    from,
                    to,
                    utf16_from: uf,
                    utf16_to: ut,
                    generated,
                    visible: true,
                });
            }
        }
        HtmlNode::Tag(Tag::End(location, _, _)) => {
            out.kind = "tag".into();
            out.attrs.insert(
                "data-weftext-native-end".into(),
                format!("{:032x}", location.hash()),
            );
        }
        HtmlNode::Frame(_) => {
            out.kind = "frame".into();
            out.text = Some("[原生分页排版区域；使用源码查看内容]".into());
        }
    }
    Ok(out)
}

/// Associate only the certified controlled recipe's actual native Strong
/// producer with its enclosing final native heading tags. Arbitrary returned
/// plain text, custom HTML h tags, and deleted headings acquire no role.
fn qualify_run_in_roles(tree: &mut RenderNode) {
    #[derive(Clone)]
    struct Heading {
        location: String,
        label: Option<String>,
        level: String,
        numbered: String,
        numbering_kind: Option<String>,
        numbering_pattern: Option<String>,
    }
    fn clear_prefix_roles(node: &mut RenderNode) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "heading-prefix")
        {
            node.attrs.remove("data-weftext-role");
            node.attrs.remove("data-weftext-label");
        }
        for child in &mut node.children {
            clear_prefix_roles(child);
        }
    }
    fn visit(node: &mut RenderNode, headings: &mut Vec<Heading>) {
        if let Some(location) = node.attrs.get("data-weftext-native-heading-start") {
            headings.push(Heading {
                location: location.clone(),
                label: node.attrs.get("data-weftext-label").cloned(),
                level: node.attrs["data-weftext-level"].clone(),
                numbered: node.attrs["data-weftext-numbered"].clone(),
                numbering_kind: node.attrs.get("data-weftext-numbering-kind").cloned(),
                numbering_pattern: node.attrs.get("data-weftext-numbering-pattern").cloned(),
            });
        }
        if let Some(location) = node.attrs.get("data-weftext-native-end")
            && let Some(position) = headings
                .iter()
                .rposition(|heading| &heading.location == location)
        {
            headings.truncate(position);
        }
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "run-in-candidate")
        {
            if let Some(heading) = headings
                .last()
                .filter(|heading| heading.label.as_ref() == node.attrs.get("data-weftext-label"))
            {
                node.attrs
                    .insert("data-weftext-role".into(), "heading".into());
                node.attrs
                    .insert("data-weftext-run-in".into(), "true".into());
                node.attrs
                    .insert("data-weftext-level".into(), heading.level.clone());
                node.attrs
                    .insert("data-weftext-numbered".into(), heading.numbered.clone());
                if let Some(kind) = &heading.numbering_kind {
                    node.attrs
                        .insert("data-weftext-numbering-kind".into(), kind.clone());
                }
                if let Some(pattern) = &heading.numbering_pattern {
                    node.attrs
                        .insert("data-weftext-numbering-pattern".into(), pattern.clone());
                }
            } else {
                node.attrs.remove("data-weftext-role");
                node.attrs.remove("data-weftext-label");
                clear_prefix_roles(node);
            }
        }
        for child in &mut node.children {
            visit(child, headings);
        }
    }
    visit(tree, &mut vec![]);
}

/// The subtitle convention is meaningful only with a unique displayed native
/// title and one finally consumed labelled native block. Source labels or a
/// title kept solely in the introspector do not establish that association.
fn qualify_subtitle_role(tree: &mut RenderNode, label_occurrences: usize) {
    fn count(node: &RenderNode, titles: &mut usize, subtitles: &mut usize) {
        match node.attrs.get("data-weftext-role").map(String::as_str) {
            Some("title") => *titles += 1,
            Some("subtitle-candidate") => *subtitles += 1,
            _ => {}
        }
        for child in &node.children {
            count(child, titles, subtitles);
        }
    }
    fn resolve(node: &mut RenderNode, associated: bool) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "subtitle-candidate")
        {
            if associated {
                node.attrs
                    .insert("data-weftext-role".into(), "subtitle".into());
            } else {
                node.attrs.remove("data-weftext-role");
                node.attrs.remove("data-weftext-label");
            }
        }
        for child in &mut node.children {
            resolve(child, associated);
        }
    }
    let (mut titles, mut subtitles) = (0, 0);
    count(tree, &mut titles, &mut subtitles);
    resolve(
        tree,
        titles == 1 && subtitles == 1 && label_occurrences == 1,
    );
}

fn count_origins(node: &RenderNode, counts: &mut HashMap<(String, usize, usize), usize>) {
    if node.kind == "text"
        && let Some(o) = &node.origin
    {
        *counts.entry((o.path.clone(), o.from, o.to)).or_default() += 1;
    }
    for child in &node.children {
        count_origins(child, counts);
    }
}

fn literal_display(origin: &Origin) -> Option<String> {
    match origin.syntax_kind.as_str() {
        "Text" => Some(origin.raw.clone()),
        "Escape" | "Shorthand" | "Space" => {
            let parsed = typst_syntax::Source::detached(&origin.raw);
            let mut children = parsed.root().children();
            let node = children.next()?;
            if children.next().is_some() || node.len() != origin.raw.len() {
                return None;
            }
            match (origin.syntax_kind.as_str(), node.kind()) {
                ("Escape", typst_syntax::SyntaxKind::Escape) => node
                    .cast::<typst_syntax::ast::Escape>()
                    .map(|escape| escape.get().to_string()),
                ("Shorthand", typst_syntax::SyntaxKind::Shorthand) => node
                    .cast::<typst_syntax::ast::Shorthand>()
                    .map(|shorthand| shorthand.get().to_string()),
                ("Space", typst_syntax::SyntaxKind::Space) => Some(" ".into()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Certify a frozen native literal's displayed text, without granting source
/// editing permission. Consumers must separately prove unique occurrences and
/// complete atomic boundaries when attaching discussion to its source range.
pub fn native_literal_display(node: &RenderNode) -> bool {
    node.kind == "text"
        && node.origin.as_ref().is_some_and(|origin| {
            origin.is_main
                && origin.to.checked_sub(origin.from) == Some(origin.raw.len())
                && literal_display(origin).as_deref() == node.text.as_deref()
        })
}

fn qualify(
    node: &mut RenderNode,
    info: &crate::EditorMap,
    counts: &HashMap<(String, usize, usize), usize>,
) {
    if info.unavailable_reason.is_none()
        && node.kind == "text"
        && let Some(o) = &node.origin
    {
        let expected = literal_display(o);
        node.writable = o.is_main
            && expected.as_deref() == node.text.as_deref()
            && info
                .literal_regions
                .iter()
                .any(|r| o.from >= r.range.from && o.to <= r.range.to)
            && counts.get(&(o.path.clone(), o.from, o.to)) == Some(&1);
    }
    for child in &mut node.children {
        qualify(child, info, counts);
    }
}

fn contains_range(node: &RenderNode, path: Option<&str>, from: usize, to: usize) -> bool {
    (node.kind == "text"
        && node.origin.as_ref().is_some_and(|o| {
            path.map_or(o.is_main, |p| o.path == p) && o.from >= from && o.to <= to
        }))
        || node
            .children
            .iter()
            .any(|n| contains_range(n, path, from, to))
}

fn allowed_tag(tag: &str) -> bool {
    matches!(
        tag,
        "html"
            | "head"
            | "body"
            | "main"
            | "article"
            | "section"
            | "header"
            | "footer"
            | "nav"
            | "aside"
            | "div"
            | "span"
            | "p"
            | "br"
            | "hr"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "strong"
            | "b"
            | "em"
            | "i"
            | "s"
            | "del"
            | "u"
            | "sub"
            | "sup"
            | "small"
            | "mark"
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "blockquote"
            | "pre"
            | "code"
            | "a"
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "td"
            | "th"
            | "caption"
            | "colgroup"
            | "col"
            | "figure"
            | "figcaption"
            | "img"
            | "title"
            | "math"
            | "semantics"
            | "annotation"
            | "mrow"
            | "mi"
            | "mo"
            | "mn"
            | "mtext"
            | "mspace"
            | "ms"
            | "mfrac"
            | "msqrt"
            | "mroot"
            | "mstyle"
            | "merror"
            | "mpadded"
            | "mphantom"
            | "mfenced"
            | "menclose"
            | "msub"
            | "msup"
            | "msubsup"
            | "munder"
            | "mover"
            | "munderover"
            | "mmultiscripts"
            | "mprescripts"
            | "none"
            | "mtable"
            | "mlabeledtr"
            | "mtr"
            | "mtd"
    )
}

fn safe_css(properties: &typst_html::css::Properties) -> String {
    properties
        .iter()
        .filter(|p| {
            matches!(
                p.name,
                "font-family"
                    | "font-size"
                    | "font-weight"
                    | "font-style"
                    | "color"
                    | "background-color"
                    | "border-top"
                    | "border-right"
                    | "border-bottom"
                    | "border-left"
                    | "border-collapse"
                    | "text-align"
                    | "vertical-align"
                    | "white-space"
                    | "display"
                    | "line-height"
                    | "margin-inline-end"
                    | "list-style-type"
            )
        })
        .filter(|p| {
            p.value.len() < 4096
                && !p
                    .value
                    .chars()
                    .any(|c| c.is_control() || ";{}<>".contains(c))
                && !p.value.to_lowercase().contains("url(")
                && !p.value.to_lowercase().contains("expression(")
        })
        .map(|p| format!("{}: {}", p.name, p.value))
        .collect::<Vec<_>>()
        .join("; ")
}

fn safe_attr(name: &str, value: &str, tag: &str) -> Option<String> {
    let limit = if name == "src" && tag == "img" {
        8 * 1024 * 1024
    } else {
        4096
    };
    if value.len() > limit || value.chars().any(|c| c == '\0') {
        return None;
    }
    match name {
        "href" if tag == "a" => safe_url(value).then(|| value.into()),
        // Native image resource transport is deliberately not an ambient URL
        // loader; only inline raster bytes can be displayed in this MVP.
        "src" if tag == "img" => (value.starts_with("data:image/png;base64,")
            || value.starts_with("data:image/jpeg;base64,")
            || value.starts_with("data:image/gif;base64,")
            || value.starts_with("data:image/webp;base64,"))
        .then(|| value.into()),
        "linethickness" if tag == "mfrac" => (matches!(value, "thin" | "medium" | "thick")
            || safe_math_length(value, false))
        .then(|| value.into()),
        "width" if tag == "mspace" => safe_math_length(value, true).then(|| value.into()),
        "id" | "title" | "alt" | "lang" | "dir" | "role" | "aria-level" | "aria-label"
        | "colspan" | "rowspan" | "scope" | "start" | "value" | "display" | "mathvariant"
        | "stretchy" | "fence" | "separator" | "accent" | "accentunder" | "lspace" | "rspace"
        | "encoding" | "xmlns" => Some(value.into()),
        _ => None,
    }
}

fn safe_math_length(value: &str, negative: bool) -> bool {
    if value.is_empty()
        || value.len() > 64
        || !value.is_ascii()
        || value.chars().any(char::is_whitespace)
    {
        return false;
    }
    let scalar = ["em", "ex", "px", "pt", "pc", "cm", "mm", "in", "%"]
        .into_iter()
        .find_map(|unit| value.strip_suffix(unit))
        .unwrap_or(value);
    scalar.parse::<f64>().is_ok_and(|number| {
        number.is_finite() && number.abs() <= 100_000.0 && (negative || number >= 0.0)
    })
}

pub fn safe_url(value: &str) -> bool {
    let value = value.trim();
    if value.chars().any(|c| c.is_control()) {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    let local_file = lower.strip_prefix("file:///").is_some_and(|path| {
        !path.is_empty()
            && !path.starts_with(['/', '\\'])
            && !path.starts_with("%2f")
            && !path.starts_with("%5c")
    });
    lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("weftext-node://")
        || lower.starts_with("weftext-resource://")
        || lower.starts_with("weftext-workspace://")
        || local_file
        || value.starts_with('#')
        || (!value.contains(':') && !value.starts_with("//") && !value.starts_with('\\'))
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn safe_html(node: &RenderNode) -> String {
    if node.kind == "tag" {
        return String::new();
    }
    if node.kind == "text" {
        return escape(node.text.as_deref().unwrap_or(""));
    }
    if node.kind == "frame" {
        return format!(
            "<span class=\"weftext-frame\">{}</span>",
            escape(node.text.as_deref().unwrap_or(""))
        );
    }
    let tag = node.tag.as_deref().unwrap_or("span");
    let children = escape(node.text.as_deref().unwrap_or(""))
        + &node.children.iter().map(safe_html).collect::<String>();
    if matches!(tag, "html" | "head" | "body" | "title") {
        return if matches!(tag, "head" | "title") {
            String::new()
        } else {
            children
        };
    }
    let attrs = node
        .attrs
        .iter()
        .map(|(k, v)| format!(" {k}=\"{}\"", escape(v)))
        .collect::<String>();
    if matches!(tag, "br" | "hr" | "img" | "col") {
        format!("<{tag}{attrs}>")
    } else {
        format!("<{tag}{attrs}>{children}</{tag}>")
    }
}

#[cfg(test)]
mod origin_offset_tests {
    use super::*;
    use typst_syntax::{LinkedNode, RootedPath, Source, VirtualPath, VirtualRoot};

    #[test]
    fn actual_native_note_origins_keep_typed_writer_refusals_and_original_source_ranges() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = Engine::new_with_system_fonts(false).unwrap();
        let mut input = crate::FrozenRenderRequest {
            source: "Author#footnote[原文😀]".into(),
            path: dir.path().join("main.typ"),
            root: dir.path().into(),
            files: BTreeMap::new(),
            style: None,
        };
        let output = engine.render_frozen(&input).unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        let actual_tree = output.tree.unwrap();
        let world = LocalWorld::frozen(&engine.library, &engine.fonts, &input).unwrap();
        world.source(world.main()).unwrap();
        for copies in [1, 0, 2] {
            let mut notes = crate::notes::inspect_notes_source(&input.source);
            let original_hash = notes.source_hash.clone();
            let original = notes
                .entries
                .iter()
                .find(|entry| entry.kind == "footnote-definition")
                .unwrap()
                .clone();
            assert!(original.writable);
            let tree = if copies == 1 {
                actual_tree.clone()
            } else {
                let mut wrapper = actual_tree.clone();
                wrapper.attrs.clear();
                wrapper.origin = None;
                wrapper.children = vec![actual_tree.clone(); copies];
                wrapper
            };
            qualify_note_writers(&tree, &mut notes, &world);
            let entry = notes
                .entries
                .iter()
                .find(|entry| entry.kind == "footnote-definition")
                .unwrap();
            assert_eq!(entry.writable, copies == 1);
            assert_eq!(entry.construct.raw, original.construct.raw);
            assert_eq!(entry.construct.from, original.construct.from);
            assert_eq!(entry.construct.to, original.construct.to);
            assert_eq!(
                entry.body.as_ref().map(|body| &body.raw),
                original.body.as_ref().map(|body| &body.raw)
            );
            if copies == 1 {
                assert!(entry.unavailable_kind.is_none());
                assert!(entry.unavailable_reason.is_none());
            } else {
                assert_eq!(
                    entry.unavailable_kind,
                    Some(crate::notes::NoteSourceUnavailableKind::Other)
                );
                let reason = entry.unavailable_reason.as_ref().unwrap();
                assert_eq!(
                    reason.explanation,
                    UiMessage::error_engine_reason_native_origin_nonunique_or_undisplayed()
                );
                assert!(reason.diagnostics.is_empty());
                assert!(reason.causes.as_slice().is_empty());
                let wire = serde_json::to_value(entry).unwrap();
                let _ = reason.explanation.render(weftext_messages::UiLocale::ZhCn);
                let _ = reason.explanation.render(weftext_messages::UiLocale::EnUs);
                assert_eq!(serde_json::to_value(entry).unwrap(), wire);
                assert_eq!(
                    serde_json::to_value(
                        serde_json::from_value::<crate::notes::NoteSourceEntry>(wire.clone())
                            .unwrap()
                    )
                    .unwrap(),
                    wire
                );
            }
            assert_eq!(notes.source_hash, original_hash);
        }

        input.source = "#show text: it => it\nAuthor#footnote[原文😀]".into();
        let world = LocalWorld::frozen(&engine.library, &engine.fonts, &input).unwrap();
        world.source(world.main()).unwrap();
        let mut notes = crate::notes::inspect_notes_source(&input.source);
        qualify_note_writers(&actual_tree, &mut notes, &world);
        let entry = notes
            .entries
            .iter()
            .find(|entry| entry.kind == "footnote-definition")
            .unwrap();
        assert!(!entry.writable);
        assert_eq!(
            entry.unavailable_kind,
            Some(crate::notes::NoteSourceUnavailableKind::Other)
        );
        assert_eq!(
            entry.unavailable_reason.as_ref().unwrap().explanation,
            UiMessage::error_engine_reason_show_rule_unverified()
        );
    }

    #[test]
    fn compiler_rejection_preserves_all_original_diagnostics_before_product_warning_projection() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = Engine::new_with_system_fonts(false).unwrap();
        let mut input = crate::FrozenRenderRequest {
            source: "#missing_author_function(原名)".into(),
            path: dir.path().join("main.typ"),
            root: dir.path().into(),
            files: BTreeMap::new(),
            style: None,
        };
        let RenderCompilation::Rejected {
            output,
            diagnostics,
            product_diagnostics,
        } = engine.render_frozen_compilation(&input).unwrap()
        else {
            panic!("invalid native source must reject compilation")
        };
        assert!(!output.ok);
        assert!(product_diagnostics.is_empty());
        assert!(!diagnostics.is_empty());
        assert!(
            output
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.explanation.is_none())
        );
        assert_eq!(
            diagnostics,
            output
                .diagnostics
                .iter()
                .flat_map(|diagnostic| diagnostic.diagnostics.iter().cloned())
                .collect::<Vec<_>>()
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.origin == DiagnosticOrigin::Typst)
        );

        input.source = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n#endnote[Author]\n#endnotes()".into();
        input.files.insert(
            "weftext-endnotes.typ".into(),
            b"#let endnote(body) = body\n#let endnote-ref(id) = []\n#let endnotes() = []".to_vec(),
        );
        let RenderCompilation::Compiled(output) = engine.render_frozen_compilation(&input).unwrap()
        else {
            panic!("the custom module remains valid native source")
        };
        assert!(output.ok);
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.explanation.as_ref().is_some_and(|message| {
                message.key() == "error.engine.render.endnotesModuleUncertified"
            })
        }));
    }

    #[test]
    fn frozen_package_rejection_keeps_product_and_original_compiler_evidence_separate() {
        let root = tempfile::TempDir::new().unwrap();
        let engine = Engine::new_with_system_fonts(false).unwrap();
        let input = crate::FrozenRenderRequest {
            source: "#import \"@preview/author-package:0.1.0\": *".into(),
            path: root.path().join("main.typ"),
            root: root.path().into(),
            files: BTreeMap::new(),
            style: None,
        };
        let RenderCompilation::Rejected {
            output,
            diagnostics,
            product_diagnostics,
        } = engine.render_frozen_compilation(&input).unwrap()
        else {
            panic!("frozen input cannot download packages")
        };
        assert!(!output.ok);
        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.origin == DiagnosticOrigin::Typst)
        );
        assert_eq!(product_diagnostics.len(), 1);
        assert_eq!(
            product_diagnostics[0].explanation,
            UiMessage::error_engine_world_frozen_packages_unavailable()
        );
        assert!(product_diagnostics[0].diagnostics.is_empty());
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.explanation.as_ref()
                    == Some(&UiMessage::error_engine_world_frozen_packages_unavailable()))
        );
        let error = EngineError::new(UiMessage::error_engine_notes_frozen_compilation_failed())
            .with_product_diagnostics(product_diagnostics.clone());
        let (_, retained) = error.into_parts();
        assert_eq!(retained, product_diagnostics);
    }

    #[test]
    fn actual_projection_budget_preserves_its_kind_and_parallel_world_admission_evidence() {
        let root = tempfile::TempDir::new().unwrap();
        let engine = Engine::new_with_system_fonts(false).unwrap();
        let input = crate::FrozenRenderRequest {
            source: "author source 原件😀".into(),
            path: root.path().join("main.typ"),
            root: root.path().into(),
            files: (0..=512)
                .map(|index| (format!("part{index}.typ"), b"part".to_vec()))
                .collect(),
            style: None,
        };
        let world = LocalWorld::frozen(&engine.library, &engine.fonts, &input).unwrap();
        for index in 0..=512 {
            let path = VirtualPath::new(format!("/part{index}.typ")).unwrap();
            let file = FileId::new(RootedPath::new(VirtualRoot::Project, path));
            assert_eq!(world.file(file).is_ok(), index < 512);
        }
        let root_node = typst_html::HtmlElement::new(typst_html::tag::div)
            .with_children(
                (0..100_001)
                    .map(|_| HtmlNode::text("x", Span::detached()))
                    .collect(),
            )
            .into();
        let mut warnings = vec![];
        let mut producers = NoteProducerState {
            sources: &HashSet::new(),
            offsets: &HashMap::new(),
            definitions: HashSet::new(),
            reference: None,
            product_warnings: &mut warnings,
        };
        let error = project(
            &root_node,
            &world,
            &mut 0,
            &mut vec![],
            0,
            "scope",
            &mut producers,
        )
        .unwrap_err()
        .with_product_diagnostics(world.product_diagnostics());
        let EngineError::DisplayTreeBudget {
            product_diagnostics,
        } = &error
        else {
            panic!("the projection budget is a machine fact independent of explanations")
        };
        assert_eq!(product_diagnostics.len(), 1);
        assert_eq!(
            product_diagnostics[0].explanation,
            UiMessage::error_engine_world_dependency_count_budget()
        );
        let (description, diagnostics) = error.into_parts();
        assert_eq!(
            description.explanation,
            UiMessage::engine_render_display_tree_budget()
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(world.main_source.text(), input.source);
    }

    #[test]
    fn snapshots_cover_derived_sources_and_late_or_replaced_sources_fall_back() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = Engine::new_with_system_fonts(false).unwrap();
        let input = crate::FrozenRenderRequest {
            source: "主😀𠮷e\u{301}\r\n".into(),
            path: dir.path().join("notes/主😀.typ"),
            root: dir.path().into(),
            files: BTreeMap::from([("notes/late.typ".into(), "后来😀𠮷\r\n".as_bytes().to_vec())]),
            style: Some(crate::StyleModule {
                path: "styles/样式😀.typ".into(),
                source: "#let library-style(body) = [样式𠮷 #body]".into(),
                enabled: true,
                entry_function: "library-style".into(),
            }),
        };
        let world = LocalWorld::frozen(&engine.library, &engine.fonts, &input).unwrap();
        let sources = world.loaded_sources();
        let offsets: OriginOffsets<'_> = sources
            .iter()
            .map(|source| {
                (
                    source.id(),
                    (source.text(), Utf16Offsets::new(source.text())),
                )
            })
            .collect();
        let derived = world.main();
        assert_ne!(derived, world.main_id);
        assert!(offsets.contains_key(&derived));
        fn walk(node: &LinkedNode<'_>, world: &LocalWorld<'_>, offsets: &OriginOffsets<'_>) {
            assert_eq!(
                serde_json::to_value(origin_with_offsets(node.span(), world, Some(offsets)))
                    .unwrap(),
                serde_json::to_value(origin(node.span(), world)).unwrap()
            );
            for child in node.children() {
                walk(&child, world, offsets);
            }
        }
        for source in &sources {
            assert!(std::ptr::eq(
                source.text(),
                world.source(source.id()).unwrap().text()
            ));
            walk(&LinkedNode::new(source.root()), &world, &offsets);
        }
        let late_id = FileId::new(RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("/notes/late.typ").unwrap(),
        ));
        assert!(!offsets.contains_key(&late_id));
        let late = world.source(late_id).unwrap();
        walk(&LinkedNode::new(late.root()), &world, &offsets);
        let first = Source::new(late_id, "éxy".into());
        let replaced = Source::new(late_id, "😀".into());
        let guarded = HashMap::from([(late_id, (first.text(), Utf16Offsets::new(first.text())))]);
        assert_eq!(first.text().len(), replaced.text().len());
        for at in 0..=replaced.text().len() + 1 {
            assert_eq!(
                origin_utf16(&replaced, at, Some(&guarded)),
                utf16(replaced.text(), at)
            );
        }
        assert_eq!(origin_utf16(&replaced, usize::MAX, Some(&guarded)), 0);
    }
}
