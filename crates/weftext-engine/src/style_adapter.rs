//! Public native rule replacements. Primitive text/cell hooks remain in the
//! small pinned HTML adapter because the official API cannot observe them.
use typst::{
    Library, World,
    diag::warning,
    foundations::{Content, Depth, NativeElement, Target},
    layout::BlockElem,
    model::{EmphElem, ListElem, ListMarker, ParElem, ParbreakElem, StrongElem, TitleElem},
    text::{ItalicToggle, SpaceElem, TextElem, WeightDelta},
};
use typst_html::{HtmlElem, WeftextHtmlSemantics, css, tag};

pub(crate) fn install(library: &mut Library) {
    library
        .rules
        .replace::<TitleElem>(Target::Html, |elem, _, styles| {
            use typst::diag::At;
            let body = elem.resolve_body(styles).at(elem.span())?;
            let semantic = WeftextHtmlSemantics {
                role: Some("title"),
                label: elem.label().map(|label| label.resolve().as_str().into()),
                body: Some(body.plain_text()),
                ..Default::default()
            };
            Ok(BlockElem::packed(
                HtmlElem::new(tag::h1)
                    .with_body(Some(body))
                    .with_weftext(semantic)
                    .pack()
                    .spanned(elem.span()),
            ))
        });
    library
        .rules
        .replace::<StrongElem>(Target::Html, |elem, engine, styles| {
            let run_in = elem
                .span()
                .id()
                .and_then(|id| engine.world.source(id).ok())
                .and_then(|source| {
                    typst_html::controlled_run_in_producer(&source, elem.span(), "strong")
                });
            let semantic = WeftextHtmlSemantics {
                role: Some(if run_in.is_some() {
                    "run-in-candidate"
                } else {
                    "strong"
                }),
                label: run_in.clone(),
                counter_state: if run_in.is_some() {
                    Some(typst_html::heading_counter_state(
                        engine,
                        styles,
                        elem.location().unwrap(),
                        elem.span(),
                    )?)
                } else {
                    None
                },
                ..Default::default()
            };
            let mut body = elem.body.clone();
            if let Some(label) = &semantic.label
                && let Some(id) = elem.span().id()
                && let Ok(source) = engine.world.source(id)
            {
                let mut children = vec![];
                body.sequence_recursive_for_each(&mut |child| children.push(child.clone()));
                if let Some(separator) = children.iter().position(|child| {
                    child.is::<TextElem>()
                        && typst_html::controlled_run_in_producer(
                            &source,
                            child.span(),
                            "prefix-separator",
                        )
                        .as_ref()
                            == Some(label)
                }) {
                    let mut end = separator + 1;
                    while children.get(end).is_some_and(|child| {
                        child.is::<SpaceElem>()
                            && typst_html::controlled_run_in_producer(
                                &source,
                                child.span(),
                                "prefix-whitespace",
                            )
                            .as_ref()
                                == Some(label)
                    }) {
                        end += 1;
                    }
                    let prefix = Content::sequence(children[..end].iter().enumerate().map(
                        |(index, child)| {
                            // Literal spacing retains the already normalized space
                            // when the prefix is placed at an inline wrapper's end.
                            if index > separator && child.is::<SpaceElem>() {
                                TextElem::packed(" ").spanned(child.span())
                            } else {
                                child.clone()
                            }
                        },
                    ));
                    let prefix = HtmlElem::new(tag::span)
                        .with_weftext(WeftextHtmlSemantics::new("heading-prefix"))
                        .with_body(Some(prefix))
                        .pack()
                        .spanned(elem.span());
                    body = prefix + Content::sequence(children[end..].iter().cloned());
                }
            }
            Ok(HtmlElem::new(tag::strong)
                .with_weftext(semantic)
                .with_body(Some(
                    body.styled(TextElem::delta.set(WeightDelta(elem.delta.get(styles)))),
                ))
                .pack()
                .spanned(elem.span()))
        });
    library
        .rules
        .replace::<EmphElem>(Target::Html, |elem, _, _| {
            Ok(HtmlElem::new(tag::em)
                .with_body(Some(
                    elem.body
                        .clone()
                        .styled(TextElem::emph.set(ItalicToggle(true))),
                ))
                .pack())
        });
    library
        .rules
        .replace::<ParElem>(Target::Html, |elem, _, styles| {
            let leading = styles.resolve(ParElem::leading);
            let leading = css::Properties::build(()).with("gap", leading).finish();
            let css = css::Properties::new()
                .with("line-height", format!("calc(1em + {})", leading[0].value));
            Ok(HtmlElem::new(tag::p)
                .with_body(Some(elem.body.clone()))
                .with_css(css)
                .pack())
        });
    library
        .rules
        .replace::<ListElem>(Target::Html, |elem, engine, styles| {
            let marker = match elem.marker.get_ref(styles) {
                ListMarker::Content(markers) => markers
                    .get(styles.get(ListElem::depth).0 % markers.len().max(1))
                    .cloned(),
                ListMarker::Func(_) => {
                    engine.sink.warn(warning!(
                        elem.span(),
                        "dynamic list marker has no flow style adapter; use paged preview"
                    ));
                    None
                }
            };
            let css = if marker.is_some() {
                css::Properties::new().with("list-style-type", "none")
            } else {
                css::Properties::new()
            };
            let body = Content::sequence(elem.children.iter().map(|item| {
                let mut body = item.body.clone().styled(ListElem::depth.set(Depth(1)));
                if !elem.tight.get(styles) {
                    body += ParbreakElem::shared();
                }
                if let Some(marker) = &marker {
                    body = HtmlElem::new(tag::span)
                        .with_css(css::Properties::new().with("margin-inline-end", "0.5em"))
                        .with_body(Some(marker.clone()))
                        .pack()
                        + body;
                }
                HtmlElem::new(tag::li)
                    .with_body(Some(body))
                    .pack()
                    .spanned(item.span())
            }));
            Ok(BlockElem::packed(
                HtmlElem::new(tag::ul)
                    .with_css(css)
                    .with_body(Some(body))
                    .pack()
                    .spanned(elem.span()),
            ))
        });
}
