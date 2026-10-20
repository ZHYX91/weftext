use std::num::NonZeroUsize;
use std::sync::Arc;

use az::SaturatingAs;
use comemo::Track;
use ecow::{EcoVec, eco_format};
use typst_library::diag::{At, SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{
    Content, Context, NativeElement, NativeRuleMap, Selector, ShowFn, Smart, StyleChain, Target,
};
use typst_library::introspection::{
    Counter, DocumentIntrospection, Location, Locator, QueryIntrospection, QueryLabelIntrospection,
};
use typst_library::layout::resolve::{Cell, CellGrid, Entry, Header};
use typst_library::layout::{BlockElem, HElem, OuterVAlignment, Sizing};
use typst_library::math::EquationElem;
use typst_library::math::ir::resolve_equation;
use typst_library::model::{
    Attribution, BibliographyElem, CitationForm, CiteElem, CiteGroup, CslIndentElem, CslLightElem,
    Destination, DirectLinkElem, DividerElem, EarlyLinkResolver, EmphElem, EnumElem, FigureCaption,
    FigureElem, FigureKind, FootnoteContainer, FootnoteElem, FootnoteEntry, FootnoteMarker,
    HeadingElem, LinkElem, LinkTarget, ListElem, OutlineElem, OutlineEntry, OutlineNode, ParElem,
    ParbreakElem, QuoteElem, RefElem, RefForm, Refable, StrongElem, TableCell, TableElem,
    TermsElem, TitleElem, Works,
};
use typst_library::routines::Arenas;
use typst_library::text::{
    HighlightElem, LinebreakElem, OverlineElem, RawElem, RawLine, SmallcapsElem, SpaceElem,
    StrikeElem, SubElem, SuperElem, TextElem, UnderlineElem,
};
use typst_library::visualize::{Color, ImageElem};
use typst_syntax::Span;

use crate::mathml::convert_math_to_nodes;
use crate::{
    FrameElem, HtmlAttr, HtmlAttrs, HtmlElem, HtmlTag, WeftextHtmlSemantics, attr, css, tag,
};

/// Registers show rules for the [HTML target](Target::Html).
pub fn register(rules: &mut NativeRuleMap) {
    use Target::{Html, Paged};

    // Model.
    rules.register(Html, PAR_RULE);
    rules.register(Html, STRONG_RULE);
    rules.register(Html, EMPH_RULE);
    rules.register(Html, LIST_RULE);
    rules.register(Html, ENUM_RULE);
    rules.register(Html, TERMS_RULE);
    rules.register(Html, LINK_RULE);
    rules.register(Html, DIRECT_LINK_RULE);
    rules.register(Html, DIVIDER_RULE);
    rules.register(Html, TITLE_RULE);
    rules.register(Html, HEADING_RULE);
    rules.register(Html, FIGURE_RULE);
    rules.register(Html, FIGURE_CAPTION_RULE);
    rules.register(Html, QUOTE_RULE);
    rules.register(Html, FOOTNOTE_RULE);
    rules.register(Html, FOOTNOTE_MARKER_RULE);
    rules.register(Html, FOOTNOTE_CONTAINER_RULE);
    rules.register(Html, FOOTNOTE_ENTRY_RULE);
    rules.register(Html, OUTLINE_RULE);
    rules.register(Html, OUTLINE_ENTRY_RULE);
    rules.register(Html, REF_RULE);
    rules.register(Html, CITE_GROUP_RULE);
    rules.register(Html, BIBLIOGRAPHY_RULE);
    rules.register(Html, CSL_LIGHT_RULE);
    rules.register(Html, CSL_INDENT_RULE);
    rules.register(Html, TABLE_RULE);
    rules.register(Html, TABLE_CELL_RULE);

    // Text.
    rules.register(Html, SUB_RULE);
    rules.register(Html, SUPER_RULE);
    rules.register(Html, UNDERLINE_RULE);
    rules.register(Html, OVERLINE_RULE);
    rules.register(Html, STRIKE_RULE);
    rules.register(Html, HIGHLIGHT_RULE);
    rules.register(Html, SMALLCAPS_RULE);
    rules.register(Html, RAW_RULE);
    rules.register(Html, RAW_LINE_RULE);

    // Visualize.
    rules.register(Html, IMAGE_RULE);

    // Math.
    rules.register(Html, EQUATION_RULE);

    // For the HTML target, `html.frame` is a primitive. In the laid-out target,
    // it should be a no-op so that nested frames don't break (things like `show
    // math.equation: html.frame` can result in nested ones).
    rules.register::<FrameElem>(Paged, |elem, _, _| Ok(elem.body.clone()));
}

const PAR_RULE: ShowFn<ParElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::p)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const STRONG_RULE: ShowFn<StrongElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::strong)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const EMPH_RULE: ShowFn<EmphElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::em)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const LIST_RULE: ShowFn<ListElem> = |elem, _, styles| {
    Ok(BlockElem::packed(
        HtmlElem::new(tag::ul)
            .with_body(Some(Content::sequence(elem.children.iter().map(|item| {
                // Text in wide lists shall always turn into paragraphs.
                let mut body = item.body.clone();
                if !elem.tight.get(styles) {
                    body += ParbreakElem::shared();
                }
                HtmlElem::new(tag::li)
                    .with_body(Some(body))
                    .pack()
                    .spanned(item.span())
            }))))
            .pack()
            .spanned(elem.span()),
    ))
};

const ENUM_RULE: ShowFn<EnumElem> = |elem, _, styles| {
    let mut ol = HtmlElem::new(tag::ol);

    if elem.reversed.get(styles) {
        ol = ol.with_attr(attr::reversed, "reversed");
    }

    if let Some(n) = elem.start.get(styles).custom() {
        ol = ol.with_attr(attr::start, eco_format!("{n}"));
    }

    let body = Content::sequence(elem.children.iter().map(|item| {
        let mut li = HtmlElem::new(tag::li);
        if let Smart::Custom(nr) = item.number.get(styles) {
            li = li.with_attr(attr::value, eco_format!("{nr}"));
        }
        // Text in wide enums shall always turn into paragraphs.
        let mut body = item.body.clone();
        if !elem.tight.get(styles) {
            body += ParbreakElem::shared();
        }
        li.with_body(Some(body)).pack().spanned(item.span())
    }));

    Ok(BlockElem::packed(
        ol.with_body(Some(body)).pack().spanned(elem.span()),
    ))
};

const TERMS_RULE: ShowFn<TermsElem> = |elem, _, styles| {
    Ok(BlockElem::packed(
        HtmlElem::new(tag::dl)
            .with_body(Some(Content::sequence(elem.children.iter().flat_map(
                |item| {
                    // Text in wide term lists shall always turn into paragraphs.
                    let mut description = item.description.clone();
                    if !elem.tight.get(styles) {
                        description += ParbreakElem::shared();
                    }

                    [
                        HtmlElem::new(tag::dt)
                            .with_body(Some(item.term.clone()))
                            .pack()
                            .spanned(item.term.span()),
                        HtmlElem::new(tag::dd)
                            .with_body(Some(description))
                            .pack()
                            .spanned(item.description.span()),
                    ]
                },
            ))))
            .pack()
            .spanned(elem.span()),
    ))
};

// Also check `PATCHED_LINK_RULE` in `docs/src/main.rs` when editing this.
const LINK_RULE: ShowFn<LinkElem> = |elem, engine, _| {
    let span = elem.span();
    let dest = elem.dest.resolve_early(engine, span)?;

    let href = match dest {
        Destination::Url(url) => Some(url.clone().into_inner()),
        Destination::Position(_) => {
            engine.sink.warn(warning!(
                span,
                "positional link was ignored during HTML export"
            ));
            None
        }
        Destination::Location(location) => Some(
            EarlyLinkResolver::new(elem.location().unwrap(), span)
                .resolve(engine, location)
                .and_then(|link| link.into_relative_uri())
                .at(span)?,
        ),
    };

    Ok(HtmlElem::new(tag::a)
        .with_optional_attr(attr::href, href)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const DIRECT_LINK_RULE: ShowFn<DirectLinkElem> = |elem, _, _| {
    Ok(LinkElem::new(
        LinkTarget::Dest(Destination::Location(elem.loc)),
        elem.body.clone(),
    )
    .pack())
};

const DIVIDER_RULE: ShowFn<DividerElem> = |elem, _, _| {
    Ok(BlockElem::packed(
        HtmlElem::new(tag::hr).pack().spanned(elem.span()),
    ))
};

const TITLE_RULE: ShowFn<TitleElem> = |elem, _, styles| {
    Ok(BlockElem::packed(
        HtmlElem::new(tag::h1)
            .with_body(Some(elem.resolve_body(styles).at(elem.span())?))
            .pack()
            .spanned(elem.span()),
    ))
};

const HEADING_RULE: ShowFn<HeadingElem> = |elem, engine, styles| {
    let span = elem.span();

    let mut realized = elem.body.clone();
    let numbered = elem.numbering.get_ref(styles).is_some();
    if let Some(numbering) = elem.numbering.get_ref(styles).as_ref() {
        let location = elem.location().unwrap();
        let numbering = Counter::of(HeadingElem::ELEM)
            .display_at(engine, location, styles, numbering, span)?
            .spanned(span);
        // Retain the exact evaluated prefix, including its separator, as a
        // distinct subtree. DOCX may replace only this subtree with dynamic
        // Word numbering, even when the title itself begins with digits.
        let prefix = HtmlElem::new(tag::span)
            .with_weftext(WeftextHtmlSemantics::new("heading-prefix"))
            .with_body(Some(numbering + TextElem::packed(" ")))
            .pack()
            .spanned(span);
        realized = prefix + realized;
    }

    // HTML's h1 is closer to a title element. There should only be one.
    // Meanwhile, a level 1 Typst heading is a section heading. For this
    // reason, levels are offset by one: A Typst level 1 heading becomes
    // a `<h2>`.
    let level = elem.resolve_level(styles).get();
    let semantic = WeftextHtmlSemantics {
        role: Some("heading"),
        level: Some(level),
        numbered: Some(numbered),
        counter_state: if numbered {
            Some(heading_counter_state(
                engine,
                styles,
                elem.location().unwrap(),
                span,
            )?)
        } else {
            None
        },
        label: elem.label().map(|label| label.resolve().as_str().into()),
        ..Default::default()
    }
    .with_numbering(elem.numbering.get_ref(styles).as_ref());
    Ok(BlockElem::packed(if level >= 6 {
        engine.sink.warn(warning!(
            span,
            "heading of level {} was transformed to \
             <div role=\"heading\" aria-level=\"{}\">, which is not \
             supported by all assistive technology",
            level, level + 1;
            hint: "HTML only supports <h1> to <h6>, not <h{}>", level + 1;
            hint: "you may want to restructure your document so that \
                   it doesn't contain deep headings";
        ));
        HtmlElem::new(tag::div)
            .with_weftext(semantic)
            .with_body(Some(realized))
            .with_attr(attr::role, "heading")
            .with_attr(attr::aria_level, eco_format!("{}", level + 1))
            .pack()
            .spanned(elem.span())
    } else {
        let t = [tag::h2, tag::h3, tag::h4, tag::h5, tag::h6][level - 1];
        HtmlElem::new(t)
            .with_weftext(semantic)
            .with_body(Some(realized))
            .pack()
            .spanned(elem.span())
    }))
};

/// Read one already evaluated native heading counter at its final location.
/// No display strings are parsed and no numbering function is evaluated here.
#[doc(hidden)]
pub fn heading_counter_state(
    engine: &mut Engine,
    styles: StyleChain,
    location: Location,
    span: Span,
) -> SourceResult<ecow::EcoString> {
    let state = Counter::of(HeadingElem::ELEM).at(
        engine,
        Context::new(Some(location), Some(styles)).track(),
        span,
        location.into(),
    )?;
    Ok(eco_format!(
        "[{}]",
        state
            .0
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ))
}

fn native_counter_state(
    counter: &Counter,
    engine: &mut Engine,
    styles: StyleChain,
    location: Location,
    span: Span,
) -> SourceResult<ecow::EcoString> {
    let state = counter.at(
        engine,
        Context::new(Some(location), Some(styles)).track(),
        span,
        location.into(),
    )?;
    Ok(eco_format!(
        "[{}]",
        state
            .0
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ))
}

fn figure_kind(kind: &FigureKind) -> ecow::EcoString {
    match kind {
        FigureKind::Elem(element) if *element == TableElem::ELEM => "table".into(),
        FigureKind::Elem(element) if *element == ImageElem::ELEM => "figure".into(),
        FigureKind::Elem(element) if *element == EquationElem::ELEM => "equation".into(),
        FigureKind::Elem(element) => eco_format!("native:{}", element.name()),
        FigureKind::Name(name) => eco_format!("custom:{}", name),
    }
}

const FIGURE_RULE: ShowFn<FigureElem> = |elem, engine, styles| {
    let span = elem.span();
    let mut realized = elem.body.clone();
    let location = elem.location().unwrap();
    let numbering = elem.numbering.get_ref(styles).as_ref();
    let caption = elem.caption.get_cloned(styles);
    let semantic = WeftextHtmlSemantics {
        role: Some("figure"),
        target_id: Some(eco_format!("{:032x}", location.hash())),
        label: elem.label().map(|label| label.resolve().as_str().into()),
        caption_kind: match elem.kind.get_ref(styles) {
            Smart::Custom(kind) => Some(figure_kind(kind)),
            Smart::Auto => None,
        },
        caption_position: caption
            .as_ref()
            .map(|caption| match caption.position.get(styles) {
                OuterVAlignment::Top => "top",
                OuterVAlignment::Bottom => "bottom",
            }),
        numbered: Some(numbering.is_some()),
        counter_state: if numbering.is_some() {
            Some(native_counter_state(
                &elem.counter(),
                engine,
                styles,
                location,
                span,
            )?)
        } else {
            None
        },
        ..Default::default()
    }
    .with_numbering(numbering);

    // Build the caption, if any.
    if let Some(caption) = caption {
        realized = match caption.position.get(styles) {
            OuterVAlignment::Top => caption.pack() + realized,
            OuterVAlignment::Bottom => realized + caption.pack(),
        };
    }

    // Ensure that the body is considered a paragraph.
    realized += ParbreakElem::shared().clone().spanned(span);

    Ok(BlockElem::packed(
        HtmlElem::new(tag::figure)
            .with_weftext(semantic)
            .with_body(Some(realized))
            .pack()
            .spanned(elem.span()),
    ))
};

const FIGURE_CAPTION_RULE: ShowFn<FigureCaption> = |elem, engine, styles| {
    let span = elem.span();
    let target_id = elem
        .figure_location
        .flatten()
        .map(|loc| eco_format!("{:032x}", loc.hash()));
    let numbering = elem.numbering.as_ref().and_then(Option::as_ref);
    let semantic = WeftextHtmlSemantics {
        role: Some("caption"),
        target_id: target_id.clone(),
        caption_kind: elem.kind.as_ref().map(figure_kind),
        caption_position: Some(match elem.position.get(styles) {
            OuterVAlignment::Top => "top",
            OuterVAlignment::Bottom => "bottom",
        }),
        numbered: Some(numbering.is_some()),
        counter_state: if let (Some(Some(counter)), Some(Some(location))) =
            (&elem.counter, &elem.figure_location)
        {
            Some(native_counter_state(
                counter, engine, styles, *location, span,
            )?)
        } else {
            None
        },
        ..Default::default()
    }
    .with_numbering(numbering);
    // Mirror the official realization while retaining the exact evaluated
    // prefix and author body as separate native producer subtrees.
    let mut realized = HtmlElem::new(tag::span)
        .with_weftext(WeftextHtmlSemantics {
            role: Some("caption-body"),
            target_id: target_id.clone(),
            ..Default::default()
        })
        .with_body(Some(elem.body.clone()))
        .pack()
        .spanned(elem.body.span());
    if let (
        Some(Some(mut supplement)),
        Some(Some(numbering)),
        Some(Some(counter)),
        Some(Some(location)),
    ) = (
        elem.supplement.clone(),
        &elem.numbering,
        &elem.counter,
        &elem.figure_location,
    ) {
        let numbers = counter.display_at(engine, *location, styles, numbering, span)?;
        let numbers = HtmlElem::new(tag::span)
            .with_weftext(WeftextHtmlSemantics {
                role: Some("caption-number"),
                ..semantic.clone()
            })
            .with_body(Some(numbers))
            .pack()
            .spanned(span);
        if !supplement.is_empty() {
            supplement += TextElem::packed('\u{a0}');
        }
        let separator = elem.separator.get_cloned(styles).unwrap_or_default();
        let prefix = HtmlElem::new(tag::span)
            .with_weftext(WeftextHtmlSemantics {
                role: Some("caption-prefix"),
                ..semantic.clone()
            })
            .with_body(Some(supplement + numbers + separator))
            .pack()
            .spanned(span);
        realized = prefix + realized;
    }
    Ok(BlockElem::packed(
        HtmlElem::new(tag::figcaption)
            .with_weftext(semantic)
            .with_body(Some(realized))
            .pack()
            .spanned(elem.span()),
    ))
};

const QUOTE_RULE: ShowFn<QuoteElem> = |elem, _, styles| {
    let span = elem.span();
    let block = elem.block.get(styles);

    let mut realized = elem.body.clone();

    if elem.quotes.get(styles).unwrap_or(!block) {
        realized = QuoteElem::quoted(realized, styles);
    }

    let attribution = elem.attribution.get_ref(styles);

    if block {
        let mut blockquote = HtmlElem::new(tag::blockquote).with_body(Some(realized));
        if let Some(Attribution::Content(attribution)) = attribution
            && let Some(link) = attribution.to_packed::<LinkElem>()
            && let LinkTarget::Dest(Destination::Url(url)) = &link.dest
        {
            blockquote = blockquote.with_attr(attr::cite, url.clone().into_inner());
        }

        realized = BlockElem::packed(blockquote.pack().spanned(span));

        if let Some(attribution) = attribution.as_ref() {
            realized += attribution.realize(span);
            realized += ParbreakElem::shared();
        }
    } else if let Some(Attribution::Label(label)) = attribution {
        realized += SpaceElem::shared().clone();
        realized += CiteElem::new(*label).pack().spanned(span);
    }

    Ok(realized)
};

const FOOTNOTE_RULE: ShowFn<FootnoteElem> = |elem, engine, styles| {
    let span = elem.span();

    // The footnote number that links to the footnote entry.
    let link = elem.realize(engine, styles)?;
    let sup = SuperElem::new(link)
        .pack()
        .styled(HtmlElem::role.set(Some("doc-noteref".into())))
        .spanned(span);
    let reference = HtmlElem::new(tag::span)
        .with_weftext(
            WeftextHtmlSemantics {
                role: Some("footnote-reference"),
                note_definition: Some(!elem.is_ref()),
                note_id: Some(eco_format!(
                    "{:032x}",
                    elem.declaration_location(engine).at(span)?.hash()
                )),
                ..Default::default()
            }
            .with_numbering(Some(elem.numbering.get_ref(styles))),
        )
        .with_body(Some(sup))
        .pack()
        .spanned(span);

    // Indicates the presence of a default footnote rule to emit an error when
    // no footnote container is available.
    let marker = FootnoteMarker::new().pack().spanned(span);

    Ok(HElem::hole().clone() + reference + marker)
};

const FOOTNOTE_MARKER_RULE: ShowFn<FootnoteMarker> = |_, _, _| Ok(Content::empty());

const FOOTNOTE_CONTAINER_RULE: ShowFn<FootnoteContainer> = |elem, engine, _| {
    let mut selector = FootnoteElem::ELEM.select();

    // In bundle export, we only want the footnotes in the current document.
    if let Some(doc_location) =
        engine.introspect(DocumentIntrospection(elem.location().unwrap(), elem.span()))
    {
        selector = Selector::Within {
            selector: Arc::new(FootnoteElem::ELEM.select()),
            ancestor: Arc::new(doc_location.into()),
        };
    }

    // Create entries for all footnotes in the document.
    let notes = engine.introspect(QueryIntrospection(selector, elem.span()));
    let items = notes.into_iter().filter_map(|note| {
        let note = note.into_packed::<FootnoteElem>().unwrap();
        if note.is_ref() {
            return None;
        }

        let loc = note.location().unwrap();
        let span = note.span();
        Some(
            HtmlElem::new(tag::li)
                .with_body(Some(FootnoteEntry::new(note).pack().spanned(span)))
                .with_parent(loc)
                .pack()
                .located(loc.variant(1))
                .spanned(span),
        )
    });

    // Don't create a container if we filtered out all notes.
    let mut items = items.peekable();
    if items.peek().is_none() {
        return Ok(Content::empty());
    }

    // There can be multiple footnotes in a container, so they semantically
    // represent an ordered list. However, the list is already numbered with the
    // footnote superscripts in the DOM, so we turn off CSS' list enumeration.
    let list = HtmlElem::new(tag::ol)
        .with_css(css::Properties::new().with("list-style-type", "none"))
        .with_body(Some(Content::sequence(items)))
        .pack();

    // The user may want to style the whole footnote element so we wrap it in an
    // additional selectable container. This is also how it's done in the ARIA
    // spec (although there, the section also contains an additional heading).
    Ok(BlockElem::packed(
        HtmlElem::new(tag::section)
            .with_weftext(WeftextHtmlSemantics::new("footnote-list"))
            .with_attr(attr::role, "doc-endnotes")
            .with_body(Some(list))
            .pack()
            .spanned(elem.span()),
    ))
};

const FOOTNOTE_ENTRY_RULE: ShowFn<FootnoteEntry> = |elem, engine, styles| {
    let (sup, body) = elem.realize(engine, styles)?;

    // The prefix is a link back to the first footnote reference, so
    // `doc-backlink` is the appropriate ARIA role.
    let prefix = sup
        .styled(HtmlElem::role.set(Some("doc-backlink".into())))
        .spanned(elem.span());

    // We do not use the ARIA role `doc-footnote` because it "is only for
    // representing individual notes that occur within the body of a work" (see
    // <https://www.w3.org/TR/dpub-aria-1.1/#doc-footnote>). Our footnotes more
    // appropriately modelled as ARIA endnotes. This is also in line with how
    // Pandoc handles footnotes.
    let body = HtmlElem::new(tag::div)
        .with_weftext(WeftextHtmlSemantics {
            role: Some("footnote-body"),
            note_id: elem
                .note
                .location()
                .map(|loc| eco_format!("{:032x}", loc.hash())),
            ..Default::default()
        })
        .with_body(Some(body.clone()))
        .pack()
        .spanned(body.span());
    Ok(prefix + body)
};

const OUTLINE_RULE: ShowFn<OutlineElem> = |elem, engine, styles| {
    fn convert_list(list: Vec<OutlineNode>) -> Content {
        // The Digital Publishing ARIA spec also proposed to add
        // `role="directory"` to the `<ol>` element, but this role is
        // deprecated, so we don't do that. The elements are already easily
        // selectable via `nav[role="doc-toc"] ol`.
        HtmlElem::new(tag::ol)
            .with_css(css::Properties::new().with("list-style-type", "none"))
            .with_body(Some(Content::sequence(list.into_iter().map(convert_node))))
            .pack()
    }

    fn convert_node(node: OutlineNode) -> Content {
        let body = if !node.children.is_empty() {
            // The `<div>` is not technically necessary, but otherwise it
            // auto-wraps in a `<p>`, which results in bad spacing. Perhaps, we
            // can remove this in the future. See also:
            // <https://github.com/typst/typst/issues/5907>
            HtmlElem::new(tag::div)
                .with_body(Some(node.entry.pack()))
                .pack()
                + convert_list(node.children)
        } else {
            node.entry.pack()
        };
        HtmlElem::new(tag::li).with_body(Some(body)).pack()
    }

    let title = elem.realize_title(styles);
    let tree = elem.realize_tree(engine, styles)?;
    let list = convert_list(tree);

    Ok(BlockElem::packed(
        HtmlElem::new(tag::nav)
            .with_attr(attr::role, "doc-toc")
            .with_body(Some(title.unwrap_or_default() + list))
            .pack()
            .spanned(elem.span()),
    ))
};

const OUTLINE_ENTRY_RULE: ShowFn<OutlineEntry> = |elem, engine, styles| {
    let span = elem.span();
    let context = Context::new(None, Some(styles));

    let mut realized = elem.body().at(span)?;

    if let Some(prefix) = elem.prefix(engine, context.track(), span)? {
        let wrapped = HtmlElem::new(tag::span)
            .with_attr(attr::class, "prefix")
            .with_body(Some(prefix))
            .pack()
            .spanned(span);

        let separator = match elem.element.to_packed::<FigureElem>() {
            Some(elem) => elem.resolve_separator(styles),
            None => SpaceElem::shared().clone(),
        };

        realized = Content::sequence([wrapped, separator, realized]);
    }

    let loc = elem.element_location().at(span)?;
    let dest = Destination::Location(loc);

    Ok(LinkElem::new(dest.into(), realized).pack())
};

const REF_RULE: ShowFn<RefElem> = |elem, engine, styles| {
    let span = elem.span();
    // This bounded branch mirrors the official heading reference producer,
    // placing a marker around the actual counter result before combining it
    // with the native supplement. All other references use the original rule.
    if elem.form.get(styles) == RefForm::Normal
        && !BibliographyElem::has(engine, elem.target, span)
        && let Ok(target) = engine.introspect(QueryLabelIntrospection(elem.target, span))
        && (target.is::<HeadingElem>() || target.is::<FigureElem>() || target.is::<EquationElem>())
        && let Some(refable) = target.with::<dyn Refable>()
        && let Some(numbering) = refable.numbering()
    {
        let location = target.location().unwrap();
        let numbers = refable.counter().display_at(
            engine,
            location,
            styles,
            &numbering.clone().trimmed(),
            span,
        )?;
        let supplement = match elem.supplement.get_ref(styles) {
            Smart::Auto => refable.supplement(),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => {
                supplement.resolve(engine, styles, [target.clone()])?
            }
        };
        let alt = eco_format!("{} {}", supplement.plain_text(), numbers.plain_text());
        let number = HtmlElem::new(tag::span)
            .with_weftext(
                WeftextHtmlSemantics::new("reference-number").with_numbering(Some(numbering)),
            )
            .with_body(Some(numbers))
            .pack()
            .spanned(span);
        let content = if supplement.is_empty() {
            number
        } else {
            supplement + TextElem::packed("\u{a0}") + number
        }
        .spanned(span);
        let body = DirectLinkElem::new(location, content, Some(alt))
            .pack()
            .spanned(span);
        Ok(HtmlElem::new(tag::span)
            .with_weftext(WeftextHtmlSemantics {
                role: Some("reference"),
                reference_kind: Some("number"),
                label: Some(elem.target.resolve().as_str().into()),
                target_id: Some(eco_format!("{:032x}", location.hash())),
                ..Default::default()
            })
            .with_body(Some(body))
            .pack()
            .spanned(elem.span()))
    } else {
        elem.realize(engine, styles)
    }
};

const CITE_GROUP_RULE: ShowFn<CiteGroup> = |elem, engine, styles| {
    // Realization confirms the actual official bibliography assignment branch.
    // Native RefElem's synthesized citation alone does not establish a cite.
    let body = elem.realize(engine)?;
    let mut semantics = WeftextHtmlSemantics::new("citation");
    for child in &elem.children {
        if let Some(cite) = child.to_packed::<CiteElem>() {
            let form = match cite.form.get(styles) {
                Some(CitationForm::Normal) => "normal",
                Some(CitationForm::Prose) => "prose",
                Some(CitationForm::Full) => "full",
                Some(CitationForm::Author) => "author",
                Some(CitationForm::Year) => "year",
                None => "none",
            };
            semantics
                .citation_keys
                .push(cite.key.resolve().as_str().into());
            semantics.citation_forms.push(form);
            semantics.citation_spans.push(cite.span());
        }
    }
    Ok(HtmlElem::new(tag::span)
        .with_weftext(semantics)
        .with_body(Some(body))
        .with_attr(attr::role, "doc-biblioref")
        .pack()
        .spanned(elem.span()))
};

// For the bibliography, we have a few elements that should be styled (e.g.
// indent), but inline styles are not apprioriate because they couldn't be
// properly overridden. For those, we currently emit classes so that a user can
// style them with CSS, but do not emit any styles ourselves.
const BIBLIOGRAPHY_RULE: ShowFn<BibliographyElem> = |elem, engine, styles| {
    let loc = elem.location().unwrap();
    let span = elem.span();
    let works = Works::generate(engine, elem.span())?;
    let bibliography = works.bibliography(loc, span)?;

    let items = bibliography.entries.iter().map(|entry| {
        let mut realized = entry.body.clone();

        if let Some(mut prefix) = entry.prefix.clone() {
            // If we have a link back to the first citation referencing this
            // entry, attach the appropriate role.
            if prefix.is::<DirectLinkElem>() {
                prefix = prefix.set(HtmlElem::role, Some("doc-backlink".into()));
            }

            let wrapped = HtmlElem::new(tag::span)
                .with_attr(attr::class, "prefix")
                .with_body(Some(prefix))
                .pack()
                .spanned(span);

            let separator = SpaceElem::shared().clone();
            realized = Content::sequence([wrapped, separator, realized]);
        }

        HtmlElem::new(tag::li)
            .with_body(Some(realized))
            .pack()
            .located(entry.backlink)
            .spanned(span)
    });

    let title = elem.realize_title(styles);
    let list = HtmlElem::new(tag::ul)
        .with_css(css::Properties::new().with("list-style-type", "none"))
        .with_body(Some(Content::sequence(items)))
        .pack()
        .spanned(span);

    Ok(BlockElem::packed(
        HtmlElem::new(tag::section)
            .with_attr(attr::role, "doc-bibliography")
            .with_optional_attr(
                attr::class,
                bibliography.hanging_indent.then_some("hanging-indent"),
            )
            .with_body(Some(title.unwrap_or_default() + list))
            .pack()
            .spanned(elem.span()),
    ))
};

const CSL_LIGHT_RULE: ShowFn<CslLightElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::span)
        .with_attr(attr::class, "light")
        .with_body(Some(elem.body.clone()))
        .pack())
};

const CSL_INDENT_RULE: ShowFn<CslIndentElem> = |elem, _, _| {
    Ok(BlockElem::packed(
        HtmlElem::new(tag::div)
            .with_attr(attr::class, "indent")
            .with_body(Some(elem.body.clone()))
            .pack()
            .spanned(elem.span()),
    ))
};

const TABLE_RULE: ShowFn<TableElem> = |elem, engine, styles| {
    let grid = elem.grid.as_ref().unwrap();
    for cell in grid.entries.iter().filter_map(|entry| entry.as_cell()) {
        if [
            cell.stroke.top.as_ref(),
            cell.stroke.right.as_ref(),
            cell.stroke.bottom.as_ref(),
            cell.stroke.left.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(|stroke| {
            let stroke = stroke.as_ref().clone().unwrap_or_default();
            stroke.dash.is_some()
                || !matches!(stroke.paint, typst_library::visualize::Paint::Solid(_))
        }) {
            engine.sink.warn(warning!(
                cell.body.span(),
                "dashed or non-solid table stroke has no flow style adapter; use paged preview"
            ));
        }
        if cell
            .fill
            .as_ref()
            .is_some_and(|paint| !matches!(paint, typst_library::visualize::Paint::Solid(_)))
        {
            engine.sink.warn(warning!(
                cell.body.span(),
                "non-solid table fill has no flow style adapter; use paged preview"
            ));
        }
    }
    Ok(show_cellgrid(grid, styles, elem.span()))
};

fn show_cellgrid(grid: &CellGrid, styles: StyleChain, span: Span) -> Content {
    let elem = |tag, body| {
        HtmlElem::new(tag)
            .with_body(Some(body))
            .pack()
            .spanned(span)
    };
    let mut rows: Vec<_> = grid
        .entries
        .chunks(grid.non_gutter_column_count())
        .collect();

    let tr = |tag, row: &[Entry], header_repeat: Option<bool>| {
        let row = row
            .iter()
            .flat_map(|entry| entry.as_cell())
            .map(|cell| show_cell(tag, cell, styles));
        let mut row = HtmlElem::new(tag::tr).with_body(Some(Content::sequence(row)));
        if let Some(repeat) = header_repeat {
            row = row.with_weftext(WeftextHtmlSemantics {
                role: Some("table-header-row"),
                repeat: Some(repeat),
                ..WeftextHtmlSemantics::default()
            });
        }
        row.pack().spanned(span)
    };

    // TODO(subfooters): similarly to headers, take consecutive footers from
    // the end for 'tfoot'.
    let footer = grid.footer.as_ref().map(|ft| {
        // Convert from gutter to non-gutter coordinates. Use ceil as it might
        // include the previous gutter row
        // (cf. typst-library/layout/grid/resolve.rs).
        let footer_start = if grid.has_gutter {
            ft.start.div_ceil(2)
        } else {
            ft.start
        };
        let rows = rows.drain(footer_start..);
        elem(
            tag::tfoot,
            Content::sequence(rows.map(|row| tr(tag::td, row, None))),
        )
    });

    // Header range converting from gutter (doubled) to non-gutter coordinates.
    let header_range = |hd: &Header| {
        if grid.has_gutter {
            // Use ceil as it might be `2 * row_amount - 1` if the header is at
            // the end (cf. typst-library/layout/grid/resolve.rs).
            hd.range.start / 2..hd.range.end.div_ceil(2)
        } else {
            hd.range.clone()
        }
    };

    // Store all consecutive headers at the start in 'thead'. All remaining
    // headers are just 'th' rows across the table body.
    let mut consecutive_header_end = 0;
    let first_mid_table_header = grid
        .headers
        .iter()
        .take_while(|hd| {
            let range = header_range(hd);
            let is_consecutive = range.start == consecutive_header_end;
            consecutive_header_end = range.end;
            is_consecutive
        })
        .count();

    let (y_offset, header) = if first_mid_table_header > 0 {
        let removed_header_rows =
            header_range(grid.headers.get(first_mid_table_header - 1).unwrap()).end;
        let rows = rows.drain(..removed_header_rows);

        (
            removed_header_rows,
            Some(elem(
                tag::thead,
                Content::sequence(rows.enumerate().map(|(y, row)| {
                    // A single thead can contain multiple native headers with
                    // different repeat settings. Keep each resolved request
                    // on its actual rows rather than deriving it from the DOM.
                    let repeat = grid.headers[..first_mid_table_header]
                        .iter()
                        .find(|header| header_range(header).contains(&y))
                        .map(|header| header.repeated);
                    tr(tag::th, row, repeat)
                })),
            )),
        )
    } else {
        (0, None)
    };

    // TODO: Consider improving accessibility properties of multi-level headers
    // inside tables in the future, e.g. indicating which columns they are
    // relative to and so on. See also:
    // https://www.w3.org/WAI/tutorials/tables/multi-level/
    let mut next_header = first_mid_table_header;
    let mut body = Content::sequence(rows.into_iter().enumerate().map(|(relative_y, row)| {
        let y = relative_y + y_offset;
        if let Some((current_header_range, repeat)) = grid
            .headers
            .get(next_header)
            .map(|header| (header_range(header), header.repeated))
            && current_header_range.contains(&y)
        {
            if y + 1 == current_header_range.end {
                next_header += 1;
            }

            tr(tag::th, row, Some(repeat))
        } else {
            tr(tag::td, row, None)
        }
    }));

    if header.is_some() || footer.is_some() {
        body = elem(tag::tbody, body);
    }

    let content = header
        .into_iter()
        .chain(core::iter::once(body))
        .chain(footer);
    BlockElem::packed(elem(tag::table, Content::sequence(content)))
}

fn show_cell(tag: HtmlTag, cell: &Cell, styles: StyleChain) -> Content {
    let resolved = cell;
    let cell = cell.body.clone();
    let Some(cell) = cell.to_packed::<TableCell>() else {
        return cell;
    };
    let mut attrs = HtmlAttrs::new();
    let span = |n: NonZeroUsize| (n != NonZeroUsize::MIN).then(|| n.to_string());
    if let Some(colspan) = span(cell.colspan.get(styles)) {
        attrs.push(attr::colspan, colspan);
    }
    if let Some(rowspan) = span(cell.rowspan.get(styles)) {
        attrs.push(attr::rowspan, rowspan);
    }
    let mut css = css::Properties::new();
    for (name, stroke) in [
        ("border-top", &resolved.stroke.top),
        ("border-right", &resolved.stroke.right),
        ("border-bottom", &resolved.stroke.bottom),
        ("border-left", &resolved.stroke.left),
    ] {
        if let Some(stroke) = stroke {
            let fixed = stroke.as_ref().clone().unwrap_or_default();
            if fixed.dash.is_none()
                && let typst_library::visualize::Paint::Solid(color) = fixed.paint
            {
                let parts = css::Properties::build(())
                    .with("width", fixed.thickness)
                    .with("color", color)
                    .finish();
                let width = parts.iter().find(|p| p.name == "width").unwrap();
                let color = parts.iter().find(|p| p.name == "color").unwrap();
                css.push(name, eco_format!("{} solid {}", width.value, color.value));
            }
        } else {
            css.push(name, "none");
        }
    }
    if let Some(typst_library::visualize::Paint::Solid(color)) = &resolved.fill {
        for property in css::Properties::build(())
            .with("background-color", color.clone())
            .finish()
            .iter()
        {
            css.push(property.name, property.value.clone());
        }
    }
    if let Smart::Custom(align) = cell.align.get(styles) {
        if let Some(x) = align.x() {
            css.push(
                "text-align",
                match x {
                    typst_library::layout::HAlignment::Start => "start",
                    typst_library::layout::HAlignment::Left => "left",
                    typst_library::layout::HAlignment::Center => "center",
                    typst_library::layout::HAlignment::Right => "right",
                    typst_library::layout::HAlignment::End => "end",
                },
            );
        }
        if let Some(y) = align.y() {
            css.push(
                "vertical-align",
                match y {
                    typst_library::layout::VAlignment::Top => "top",
                    typst_library::layout::VAlignment::Horizon => "middle",
                    typst_library::layout::VAlignment::Bottom => "bottom",
                },
            );
        }
    }
    HtmlElem::new(tag)
        .with_css(css)
        .with_body(Some(cell.clone().pack()))
        .with_attrs(attrs)
        .pack()
        .spanned(cell.span())
}

const TABLE_CELL_RULE: ShowFn<TableCell> = |elem, _, _| Ok(elem.body.clone());

const SUB_RULE: ShowFn<SubElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::sub)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const SUPER_RULE: ShowFn<SuperElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::sup)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const UNDERLINE_RULE: ShowFn<UnderlineElem> = |elem, _, _| {
    // Note: In modern HTML, `<u>` is not the underline element, but
    // rather an "Unarticulated Annotation" element (see HTML spec
    // 4.5.22). Using `text-decoration` instead is recommended by MDN.
    Ok(HtmlElem::new(tag::span)
        .with_css(css::Properties::new().with("text-decoration", "underline"))
        .with_body(Some(elem.body.clone()))
        .pack())
};

const OVERLINE_RULE: ShowFn<OverlineElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::span)
        .with_css(css::Properties::new().with("text-decoration", "overline"))
        .with_body(Some(elem.body.clone()))
        .pack())
};

const STRIKE_RULE: ShowFn<StrikeElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::s)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const HIGHLIGHT_RULE: ShowFn<HighlightElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::mark)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const SMALLCAPS_RULE: ShowFn<SmallcapsElem> = |elem, _, styles| {
    let variant = if elem.all.get(styles) {
        "all-small-caps"
    } else {
        "small-caps"
    };
    Ok(HtmlElem::new(tag::span)
        .with_css(css::Properties::new().with("font-variant-caps", variant))
        .with_body(Some(elem.body.clone()))
        .pack())
};

const RAW_RULE: ShowFn<RawElem> = |elem, _, styles| {
    let lines = elem.lines.as_deref().unwrap_or_default();

    let mut seq = EcoVec::with_capacity((2 * lines.len()).saturating_sub(1));
    for (i, line) in lines.iter().enumerate() {
        if i != 0 {
            seq.push(LinebreakElem::shared().clone());
        }

        seq.push(line.clone().pack());
    }

    let lang = elem.lang.get_ref(styles);
    let code = HtmlElem::new(tag::code)
        .with_optional_attr(const { HtmlAttr::constant("data-lang") }, lang.clone())
        .with_body(Some(Content::sequence(seq)))
        .pack()
        .spanned(elem.span());

    Ok(if elem.block.get(styles) {
        BlockElem::packed(
            HtmlElem::new(tag::pre)
                .with_body(Some(code))
                .pack()
                .spanned(elem.span()),
        )
    } else {
        code
    })
};

/// This is used by `RawElem::synthesize` through a routine.
///
/// It's a temporary workaround until `TextElem::fill` is supported in HTML
/// export.
#[doc(hidden)]
pub fn html_span_filled(content: Content, color: Color) -> Content {
    let span = content.span();
    HtmlElem::new(tag::span)
        .with_css(css::Properties::build(()).with("color", color).finish())
        .with_body(Some(content))
        .pack()
        .spanned(span)
}

const RAW_LINE_RULE: ShowFn<RawLine> = |elem, _, _| Ok(elem.body.clone());

// Also check `PATCHED_IMAGE_RULE` in `docs/src/main.rs` when editing this.
const IMAGE_RULE: ShowFn<ImageElem> = |elem, engine, styles| {
    let image = elem.decode(engine, styles)?;

    let mut attrs = HtmlAttrs::new();
    let src = typst_svg::WebImage::new(&image).to_base64_url();
    attrs.push(attr::src, src);

    if let Some(alt) = elem.alt.get_cloned(styles) {
        attrs.push(attr::alt, alt);
    }

    // The `width` and `height` properties on the HTML element are only used to
    // reserve space while the browser is fetching. They are integers. Still, in
    // case of fractional image sizes, rounding is better than nothing and will
    // not disrupt the aspect ratio of the final image.
    let cast = |v: f64| eco_format!("{}", v.round().saturating_as::<i64>());
    attrs.push(attr::width, cast(image.width()));
    attrs.push(attr::height, cast(image.height()));

    let mut css = css::Properties::build((engine, elem.span()));

    // TODO: Exclude in semantic profile.
    if let Some(value) = typst_svg::convert_image_scaling(image.scaling()) {
        css.push("image-rendering", value);
    }

    // TODO: Exclude in semantic profile?
    match elem.width.get(styles) {
        Smart::Auto => {}
        Smart::Custom(rel) => css.push("width", rel),
    }

    // TODO: Exclude in semantic profile?
    match elem.height.get(styles) {
        Sizing::Auto => {}
        Sizing::Rel(rel) => css.push("height", rel),
        Sizing::Fr(_) => {}
    }

    Ok(BlockElem::packed(
        HtmlElem::new(tag::img)
            .with_attrs(attrs)
            .with_css(css.finish())
            .pack()
            .spanned(elem.span()),
    ))
};

const EQUATION_RULE: ShowFn<EquationElem> = |elem, engine, styles| {
    let arenas = Arenas::default();
    let item = resolve_equation(
        elem,
        engine,
        Locator::synthesize(elem.location().unwrap()),
        &arenas,
        styles,
    )?;

    let block = elem.block.get(styles);
    let body = convert_math_to_nodes(item, engine, styles, block)?;
    let math = HtmlElem::new(tag::mathml::math)
        .with_weftext(WeftextHtmlSemantics::new("math"))
        .with_body(Some(Content::sequence(body)))
        .with_optional_attr(attr::mathml::display, block.then_some("block"))
        .pack()
        .spanned(elem.span());

    if !block {
        return Ok(math);
    }
    let numbering = elem.numbering.get_ref(styles).as_ref();
    let location = elem.location().unwrap();
    let mut body = math;
    if let Some(numbering) = numbering {
        let numbers =
            elem.counter()
                .display_at(engine, location, styles, numbering, elem.span())?;
        let number = HtmlElem::new(tag::span)
            .with_weftext(
                WeftextHtmlSemantics {
                    role: Some("equation-number"),
                    target_id: Some(eco_format!("{:032x}", location.hash())),
                    ..Default::default()
                }
                .with_numbering(Some(numbering)),
            )
            .with_body(Some(numbers))
            .pack()
            .spanned(elem.span());
        body += HtmlElem::new(tag::span)
            .with_weftext(
                WeftextHtmlSemantics {
                    role: Some("equation-prefix"),
                    target_id: Some(eco_format!("{:032x}", location.hash())),
                    ..Default::default()
                }
                .with_numbering(Some(numbering)),
            )
            .with_body(Some(number))
            .pack()
            .spanned(elem.span());
    }
    Ok(BlockElem::packed(
        HtmlElem::new(tag::div)
            .with_weftext(
                WeftextHtmlSemantics {
                    role: Some("equation"),
                    target_id: Some(eco_format!("{:032x}", location.hash())),
                    label: elem.label().map(|label| label.resolve().as_str().into()),
                    numbered: Some(numbering.is_some()),
                    counter_state: if numbering.is_some() {
                        Some(native_counter_state(
                            &elem.counter(),
                            engine,
                            styles,
                            location,
                            elem.span(),
                        )?)
                    } else {
                        None
                    },
                    ..Default::default()
                }
                .with_numbering(numbering),
            )
            .with_body(Some(body))
            .pack()
            .spanned(elem.span()),
    ))
};

/// Returns the body of a MathML `HtmlElem`, if the content is one.
#[doc(hidden)]
pub fn html_mathml_body<'a>(
    content: &'a Content,
    styles: StyleChain<'a>,
) -> Option<Option<&'a Content>> {
    let elem = content.to_packed::<HtmlElem>()?;
    tag::mathml::is_mathml(elem.tag).then(|| elem.body.get_ref(styles).as_ref())
}
