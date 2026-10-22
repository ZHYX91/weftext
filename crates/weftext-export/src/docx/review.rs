use super::*;
pub fn word_comment_date(value: &str) -> Option<String> {
    if let Some(seconds) = value
        .strip_prefix("unix:")
        .and_then(|s| s.parse::<i64>().ok())
    {
        chrono::DateTime::from_timestamp(seconds, 0)
            .map(|date| date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    } else {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|date| date.to_rfc3339())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxReview {
    pub id: String,
    pub parent: Option<String>,
    pub author: String,
    pub date: Option<String>,
    pub body: String,
    pub resolved: bool,
    pub range: Option<(usize, usize)>,
    pub whole_document: bool,
    /// Some only for explicitly selected, validated pending tracked changes.
    pub replacement: Option<String>,
    #[serde(default)]
    pub downgrade_reason: Option<weftext_messages::ErrorDescription>,
    #[serde(default)]
    pub empty_body_point: bool,
}
struct Located {
    review: DocxReview,
    id: usize,
    start: (usize, usize),
    end: (usize, usize),
    math_target: Option<usize>,
}
fn math_targets(
    e: &Element,
    next: &mut usize,
    path: &mut Vec<usize>,
    out: &mut Vec<(usize, usize, usize, Vec<usize>)>,
) {
    if e.name.starts_with("m:") {
        if let Some((from, raw)) = &e.source {
            out.push((*next, *from, *from + raw.len(), path.clone()));
        }
        *next += 1;
    }
    for (i, child) in e.children.iter().filter_map(Xml::element).enumerate() {
        path.push(i);
        math_targets(child, next, path, out);
        path.pop();
    }
}
fn rewrite_math(e: &mut Element, next: &mut usize, located: &[Located]) {
    let mut out = vec![];
    for child in std::mem::take(&mut e.children) {
        let Xml::Element(mut node) = child else {
            out.push(child);
            continue;
        };
        let target = if node.name.starts_with("m:") {
            let value = *next;
            *next += 1;
            Some(value)
        } else {
            None
        };
        let relevant: Vec<_> = located
            .iter()
            .filter(|r| r.math_target.is_some() && r.math_target == target)
            .collect();
        rewrite_math(&mut node, next, located);
        let argument = matches!(
            node.name.as_str(),
            "m:num" | "m:den" | "m:e" | "m:sub" | "m:sup" | "m:deg" | "m:lim"
        );
        if argument {
            for item in relevant.iter().rev() {
                node.children
                    .insert(0, Xml::Element(mark("w:commentRangeStart", item.id)));
            }
            for item in &relevant {
                node.push(mark("w:commentRangeEnd", item.id));
                node.push(comment_ref(item.id));
            }
            out.push(Xml::Element(node));
            continue;
        }
        for item in &relevant {
            out.push(Xml::Element(mark("w:commentRangeStart", item.id)));
        }
        out.push(Xml::Element(node));
        for item in &relevant {
            out.push(Xml::Element(mark("w:commentRangeEnd", item.id)));
            out.push(Xml::Element(comment_ref(item.id)));
        }
    }
    e.children = out;
}
fn loss(
    out: &mut Vec<ExportLoss>,
    id: &str,
    message: impl Into<weftext_messages::ErrorDescription>,
) {
    out.push(ExportLoss {
        code: "word-review-unmapped".into(),
        description: message.into(),
        location: Some(id.into()),

        product_diagnostics: Vec::new(),
    });
}
type TextRunSource = (usize, usize, String, String);
fn source_boundary(run: &TextRunSource, source: usize) -> Option<usize> {
    let offset = source.checked_sub(run.1)?;
    if run.2 == run.3 {
        run.2.is_char_boundary(offset).then_some(offset)
    } else if offset == 0 {
        Some(0)
    } else if offset == run.2.len() {
        Some(run.3.len())
    } else {
        // Native escapes/shorthands are atomic, certified literal source.
        // An offset inside their syntax is not an editable display boundary.
        None
    }
}
fn collect(e: &Element, index: &mut usize, runs: &mut Vec<TextRunSource>) {
    if e.name == "m:oMath" {
        *index += 1;
        return;
    }
    if e.name == "w:r" {
        if let Some((from, text)) = &e.source {
            runs.push((
                *index,
                *from,
                text.clone(),
                e.child("w:t").map(Element::text).unwrap_or_default(),
            ));
        }
        *index += 1;
    }
    for c in e.children.iter().filter_map(Xml::element) {
        collect(c, index, runs);
    }
}
fn comment_ref(id: usize) -> Element {
    let mut r = Element::new("w:r");
    r.push(Element::new("w:commentReference").attr("w:id", id));
    r
}
fn mark(name: &str, id: usize) -> Element {
    Element::new(name).attr("w:id", id)
}
fn revision(run: Element, deletion: bool, r: &Located, next: &mut usize) -> Element {
    fn deleted(e: &mut Element) {
        if e.name == "w:t" {
            e.name = "w:delText".into();
        }
        for c in e.children.iter_mut().filter_map(Xml::element_mut) {
            deleted(c);
        }
    }
    let mut e = Element::new(if deletion { "w:del" } else { "w:ins" })
        .attr("w:id", *next)
        .attr("w:author", &r.review.author);
    *next += 1;
    if let Some(date) = &r.review.date {
        e.set_attr("w:date", date);
    }
    let mut run = run;
    if deletion {
        deleted(&mut run);
    }
    e.push(run);
    e
}
fn rewrite(e: &mut Element, index: &mut usize, located: &[Located], next: &mut usize) {
    let mut out = vec![];
    for child in std::mem::take(&mut e.children) {
        let Xml::Element(mut n) = child else {
            out.push(child);
            continue;
        };
        if n.name == "m:oMath" {
            *index += 1;
            out.push(Xml::Element(n));
            continue;
        }
        if n.name != "w:r" {
            rewrite(&mut n, index, located, next);
            out.push(Xml::Element(n));
            continue;
        }
        let run_index = *index;
        *index += 1;
        if n.source.is_none() {
            out.push(Xml::Element(n));
            continue;
        }
        let text = n.child("w:t").map(Element::text).unwrap_or_default();
        let relevant: Vec<_> = located
            .iter()
            .filter(|r| r.start.0 <= run_index && r.end.0 >= run_index)
            .collect();
        if relevant.is_empty() {
            out.push(Xml::Element(n));
            continue;
        }
        let mut cuts = BTreeSet::from([0, text.len()]);
        for r in &relevant {
            if r.start.0 == run_index {
                cuts.insert(r.start.1);
            }
            if r.end.0 == run_index {
                cuts.insert(r.end.1);
            }
        }
        let cuts: Vec<_> = cuts.into_iter().collect();
        let style = n
            .child("w:rPr")
            .cloned()
            .unwrap_or_else(|| Element::new("w:rPr"));
        for (i, &at) in cuts.iter().enumerate() {
            // End existing ranges before opening the next; point ranges open/end together.
            for r in &relevant {
                if r.end == (run_index, at) && r.start != r.end {
                    out.push(Xml::Element(mark("w:commentRangeEnd", r.id)));
                    out.push(Xml::Element(comment_ref(r.id)));
                }
            }
            for r in &relevant {
                if r.start == (run_index, at) {
                    out.push(Xml::Element(mark("w:commentRangeStart", r.id)));
                    if let Some(insert) = &r.review.replacement
                        && !insert.is_empty()
                    {
                        out.push(Xml::Element(revision(
                            text_run(insert, &style),
                            false,
                            r,
                            next,
                        )));
                    }
                    if r.start == r.end {
                        out.push(Xml::Element(mark("w:commentRangeEnd", r.id)));
                        out.push(Xml::Element(comment_ref(r.id)));
                    }
                }
            }
            if let Some(&end) = cuts.get(i + 1)
                && end > at
            {
                let mut run = text_run(&text[at..end], &style);
                if let Some(r) = relevant.iter().find(|r| {
                    r.review.replacement.is_some()
                        && r.start < (run_index, end)
                        && r.end > (run_index, at)
                }) {
                    run = revision(run, true, r, next);
                }
                out.push(Xml::Element(run));
            }
        }
    }
    e.children = out;
}
pub(super) fn apply(
    package: &mut Package,
    doc: &mut Element,
    reviews: &[DocxReview],
    losses: &mut Vec<ExportLoss>,
) -> Result<(), DocxError> {
    if reviews.is_empty() {
        return Ok(());
    }
    if reviews.len() > 2000 {
        return Err(DocxError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_docx_review_comment_budget_exceeded(),
        )));
    }
    let mut settings = package
        .xml("word/settings.xml")?
        .unwrap_or_else(|| Element::new("w:settings").attr("xmlns:w", package::W));
    if settings.child("w:compat").is_none() {
        settings.push(Element::new("w:compat"));
    }
    let compat = settings.child_mut("w:compat").unwrap();
    let prior = compat
        .children
        .iter()
        .filter_map(Xml::element)
        .find(|e| e.get("w:name") == Some("compatibilityMode"))
        .and_then(|e| e.get("w:val"))
        .and_then(|s| s.parse::<u32>().ok());
    if prior.is_none_or(|v| v < 15) {
        if prior.is_some() {
            losses.push(ExportLoss {
                code: "word-review-compatibility".into(),
                description:
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_docx_review_review_compatibility_upgraded()),
                location: Some("word/settings.xml".into()),

product_diagnostics: Vec::new(),
});
        }
        compat.children.retain(|e| {
            e.element()
                .is_none_or(|e| e.get("w:name") != Some("compatibilityMode"))
        });
        compat.push(
            Element::new("w:compatSetting")
                .attr("w:name", "compatibilityMode")
                .attr("w:uri", "http://schemas.microsoft.com/office/word")
                .attr("w:val", "15"),
        );
    }
    package.set_xml("word/settings.xml", &settings);
    let comments_path =
        package.related_part(&format!("{}comments", package::REL), "word/comments.xml")?;
    let extended_path = package.related_part(
        "http://schemas.microsoft.com/office/2011/relationships/commentsExtended",
        "word/commentsExtended.xml",
    )?;
    let mut comments = package
        .xml(&comments_path)?
        .unwrap_or_else(|| Element::new("w:comments").attr("xmlns:w", package::W));
    let mut extended = package.xml(&extended_path)?.unwrap_or_else(|| {
        Element::new("w15:commentsEx").attr(
            "xmlns:w15",
            "http://schemas.microsoft.com/office/word/2012/wordml",
        )
    });
    comments.set_attr(
        "xmlns:w14",
        "http://schemas.microsoft.com/office/word/2010/wordml",
    );
    comments.set_attr(
        "xmlns:mc",
        "http://schemas.openxmlformats.org/markup-compatibility/2006",
    );
    let mut ignorable = comments
        .get("mc:Ignorable")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    ignorable.insert("w14".into());
    comments.set_attr(
        "mc:Ignorable",
        &ignorable.into_iter().collect::<Vec<_>>().join(" "),
    );
    let maximum = existing_max(&comments, "w:comment", "w:id")
        .max(existing_max(doc, "w:ins", "w:id"))
        .max(existing_max(doc, "w:del", "w:id"));
    if maximum > 2_000_000_000 {
        return Err(DocxError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_docx_review_review_id_out_of_range(),
        )));
    }
    let mut next = maximum + 1;
    let mut runs = vec![];
    let mut run_count = 0;
    collect(doc, &mut run_count, &mut runs);
    // A separately verified native empty-body insertion point has no visible
    // text run. Create an empty Word paragraph, never a fake source character.
    let points: BTreeSet<_> = reviews
        .iter()
        .filter(|r| r.empty_body_point)
        .filter_map(|r| r.range.filter(|(a, b)| a == b).map(|(a, _)| a))
        .collect();
    if run_count == 0 && points.len() == 1 {
        let point = *points.first().unwrap();
        let mut paragraph = Element::new("w:p");
        let mut run = Element::new("w:r");
        run.source = Some((point, String::new()));
        paragraph.push(run);
        if let Some(body) = doc.child_mut("w:body") {
            let at = body
                .children
                .iter()
                .position(|c| c.element().is_some_and(|e| e.name == "w:sectPr"))
                .unwrap_or(body.children.len());
            body.children.insert(at, Xml::Element(paragraph));
            collect(doc, &mut 0, &mut runs);
        }
    }
    let mut located = vec![];
    let mut mathematics = vec![];
    math_targets(doc, &mut 0, &mut vec![], &mut mathematics);
    for review in reviews.iter().filter(|r| r.parent.is_none()) {
        if let Some(reason) = &review.downgrade_reason {
            loss(losses, &review.id, reason.clone());
        }
        let exact_math: Vec<_> = review
            .range
            .map(|(a, b)| {
                mathematics
                    .iter()
                    .filter(|(_, from, to, _)| a == *from && b == *to)
                    .collect()
            })
            .unwrap_or_default();
        let unique_math = exact_math.iter().all(|a| {
            exact_math
                .iter()
                .all(|b| a.3.starts_with(&b.3) || b.3.starts_with(&a.3))
        });
        if !exact_math.is_empty() && unique_math && !review.whole_document {
            let target = exact_math.iter().min_by_key(|item| item.3.len()).unwrap();
            let mut value = review.clone();
            if value.replacement.take().is_some() {
                loss(
                    losses,
                    &value.id,
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_docx_review_math_revision_discussion_only()),
                );
            }
            located.push(Located {
                review: value,
                id: next,
                start: (usize::MAX, 0),
                end: (usize::MAX, 0),
                math_target: Some(target.0),
            });
            next += 1;
            continue;
        }
        let position = if review.whole_document {
            runs.first()
                .zip(runs.last())
                .map(|(a, b)| ((a.0, 0), (b.0, b.3.len())))
        } else {
            review.range.and_then(|(from, to)| {
                if from > to {
                    return None;
                }
                let starts: Vec<_> = runs
                    .iter()
                    .filter(|run| {
                        run.1 <= from
                            && from < run.1 + run.2.len()
                            && source_boundary(run, from).is_some()
                    })
                    .collect();
                let ends: Vec<_> = runs
                    .iter()
                    .filter(|run| {
                        run.1 < to
                            && to <= run.1 + run.2.len()
                            && source_boundary(run, to).is_some()
                    })
                    .collect();
                if from == to {
                    let empty: Vec<_> = runs
                        .iter()
                        .filter(|run| run.1 == from && run.2.is_empty() && review.empty_body_point)
                        .collect();
                    if empty.len() == 1 {
                        return Some(((empty[0].0, 0), (empty[0].0, 0)));
                    }
                    if starts.len() == 1 {
                        return Some((
                            (starts[0].0, source_boundary(starts[0], from)?),
                            (starts[0].0, source_boundary(starts[0], from)?),
                        ));
                    }
                    if starts.is_empty() && ends.len() == 1 {
                        return Some((
                            (ends[0].0, source_boundary(ends[0], to)?),
                            (ends[0].0, source_boundary(ends[0], to)?),
                        ));
                    }
                    return None;
                }
                if starts.len() != 1 || ends.len() != 1 {
                    return None;
                }
                let a = starts[0];
                let b = ends[0];
                let span: Vec<_> = runs.iter().filter(|r| a.0 <= r.0 && r.0 <= b.0).collect();
                (a.0 <= b.0
                    && span
                        .windows(2)
                        .all(|w| w[0].0 + 1 == w[1].0 && w[0].1 + w[0].2.len() <= w[1].1))
                .then_some((
                    (a.0, source_boundary(a, from)?),
                    (b.0, source_boundary(b, to)?),
                ))
            })
        };
        let Some((start, end)) = position else {
            loss(
                losses,
                &review.id,
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_docx_review_comment_target_not_unique(
                    ),
                ),
            );
            continue;
        };
        let mut review = review.clone();
        if review.replacement.is_some()
            && located.iter().any(|r: &Located| {
                r.review.replacement.is_some()
                    && (if start == end && r.start == r.end {
                        start == r.start
                    } else if start == end {
                        r.start < start && start < r.end
                    } else if r.start == r.end {
                        start < r.start && r.start < end
                    } else {
                        r.start < end && r.end > start
                    })
            })
        {
            loss(
                losses,
                &review.id,
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_docx_review_overlapping_revision_comment_only()),
            );
            review.replacement = None;
        }
        located.push(Located {
            review,
            id: next,
            start,
            end,
            math_target: None,
        });
        next += 1;
    }
    let mut ids: BTreeMap<String, usize> = located
        .iter()
        .map(|r| (r.review.id.clone(), r.id))
        .collect();
    // Parent-first closure also handles nested replies without dropping them.
    let mut pending: Vec<_> = reviews
        .iter()
        .filter(|r| r.parent.is_some())
        .cloned()
        .collect();
    while !pending.is_empty() {
        let before = pending.len();
        pending.retain(|r| {
            if let Some(parent) = r
                .parent
                .as_ref()
                .and_then(|p| located.iter().find(|a| &a.review.id == p))
            {
                let mut reply = r.clone();
                reply.replacement = None;
                located.push(Located {
                    review: reply,
                    id: next,
                    start: parent.start,
                    end: parent.end,
                    math_target: parent.math_target,
                });
                ids.insert(r.id.clone(), next);
                next += 1;
                false
            } else {
                true
            }
        });
        if pending.len() == before {
            for r in &pending {
                loss(
                    losses,
                    &r.id,
                    weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_docx_review_reply_parent_not_exported(),
                    ),
                );
            }
            break;
        }
    }
    // Word recognizes replies only when equal-position anchor endings keep
    // each parent's replies adjacent. Independent overlapping comments must
    // not split a thread's markers, even though commentEx names its parent.
    // Keep the same parent-first order in both body markers and comment parts.
    let mut order = BTreeMap::new();
    for root in located.iter().filter(|item| item.review.parent.is_none()) {
        let mut stack = vec![root.review.id.as_str()];
        while let Some(id) = stack.pop() {
            if order.contains_key(id) {
                continue;
            }
            order.insert(id.to_owned(), order.len());
            stack.extend(
                located
                    .iter()
                    .rev()
                    .filter(|item| item.review.parent.as_deref() == Some(id))
                    .map(|item| item.review.id.as_str()),
            );
        }
    }
    located.sort_by_key(|item| order.get(&item.review.id).copied().unwrap_or(usize::MAX));
    let mut existing_paragraphs = vec![];
    comments.descendants("w:p", &mut existing_paragraphs);
    let mut used_paragraph_ids: BTreeSet<String> = existing_paragraphs
        .iter()
        .filter_map(|p| p.get("w14:paraId").map(|s| s.to_uppercase()))
        .collect();
    let mut para_next = 0x50000000u32;
    let mut para_ids = BTreeMap::new();
    for id in ids.values() {
        loop {
            let pid = format!("{para_next:08X}");
            para_next += 1;
            if used_paragraph_ids.insert(pid.clone()) {
                para_ids.insert(*id, pid);
                break;
            }
        }
    }
    for item in &located {
        let review = &item.review;
        let Some(&id) = ids.get(&review.id) else {
            continue;
        };
        let para_id = para_ids[&id].clone();
        let mut c = Element::new("w:comment")
            .attr("w:id", id)
            .attr("w:author", &review.author)
            .attr("w:initials", "WF");
        if let Some(date) = &review.date {
            c.set_attr("w:date", date);
        }
        let mut p = Element::new("w:p").attr("w14:paraId", &para_id);
        p.push(text_run(&review.body, &Element::new("w:rPr")));
        c.push(p);
        comments.push(c);
        let mut ext = Element::new("w15:commentEx")
            .attr("w15:paraId", para_id)
            .attr("w15:done", if review.resolved { "1" } else { "0" });
        if let Some(parent) = review.parent.as_ref().and_then(|p| ids.get(p)) {
            ext.set_attr("w15:paraIdParent", &para_ids[parent]);
        }
        extended.push(ext);
    }
    rewrite(doc, &mut 0, &located, &mut next);
    rewrite_math(doc, &mut 0, &located);
    package.set_xml(&comments_path, &comments);
    package.ensure_rel("comments", &format!("/{comments_path}"));
    package.content_type(
        &format!("/{comments_path}"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
    );
    package.set_xml(&extended_path, &extended);
    if !package
        .rels
        .children
        .iter()
        .filter_map(Xml::element)
        .any(|e| {
            e.get("Type")
                == Some("http://schemas.microsoft.com/office/2011/relationships/commentsExtended")
        })
    {
        package.rels.push(
            Element::new("Relationship")
                .attr("Id", format!("weftextCommentsExtended{next}"))
                .attr(
                    "Type",
                    "http://schemas.microsoft.com/office/2011/relationships/commentsExtended",
                )
                .attr("Target", format!("/{extended_path}")),
        );
    }
    package.content_type(
        &format!("/{extended_path}"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml",
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_comments_keep_parent_and_reply_endings_adjacent() {
        let mut package = Package::load(None).unwrap();
        let mut doc = Element::new("w:document").attr("xmlns:w", package::W);
        let mut body = Element::new("w:body");
        let mut paragraph = Element::new("w:p");
        let mut run = text_run("收入增长", &Element::new("w:rPr"));
        run.source = Some((100, "收入增长".into()));
        paragraph.push(run);
        body.push(paragraph);
        doc.push(body);
        let parent = DocxReview {
            id: "parent".into(),
            parent: None,
            author: "作者".into(),
            date: None,
            body: "父批注".into(),
            resolved: true,
            range: Some((100, 106)),
            whole_document: false,
            replacement: None,
            downgrade_reason: None,
            empty_body_point: false,
        };
        let mut independent = parent.clone();
        independent.id = "independent".into();
        independent.body = "独立同范围".into();
        let mut reply = parent.clone();
        reply.id = "reply".into();
        reply.parent = Some(parent.id.clone());
        reply.body = "回复".into();
        let mut losses = vec![];
        apply(
            &mut package,
            &mut doc,
            &[independent, reply, parent],
            &mut losses,
        )
        .unwrap();
        assert!(losses.is_empty());
        let comments = package.xml("word/comments.xml").unwrap().unwrap();
        let mut entries = vec![];
        comments.descendants("w:comment", &mut entries);
        assert_eq!(
            entries.iter().map(|entry| entry.text()).collect::<Vec<_>>(),
            ["独立同范围", "父批注", "回复"]
        );
        let mut endings = vec![];
        doc.descendants("w:commentRangeEnd", &mut endings);
        assert_eq!(
            endings
                .iter()
                .map(|entry| entry.get("w:id").unwrap())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.get("w:id").unwrap())
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn preserves_custom_comment_parts_and_declines_nonunique_intermediate_provenance() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/custom-comments.xml", &xml::parse(format!(r#"<w:comments xmlns:w="{}" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:comment w:id="9"><w:p w14:paraId="50000000"/></w:comment></w:comments>"#,package::W).as_bytes()).unwrap());
        p.ensure_rel("comments", "custom-comments.xml");
        let mut doc = p.xml("word/document.xml").unwrap().unwrap();
        let mut paragraph = Element::new("w:p");
        for (at, text) in [(0, "abc"), (2, "cd"), (4, "ef")] {
            let mut run = text_run(text, &Element::new("w:rPr"));
            run.source = Some((at, text.into()));
            paragraph.push(run);
        }
        doc.child_mut("w:body").unwrap().push(paragraph);
        let comment = DocxReview {
            id: "ambiguous".into(),
            parent: None,
            author: "test".into(),
            date: None,
            body: "check".into(),
            resolved: false,
            range: Some((0, 6)),
            whole_document: false,
            replacement: None,
            downgrade_reason: None,
            empty_body_point: false,
        };
        let mut exact = comment.clone();
        exact.id = "exact".into();
        exact.range = Some((0, 1));
        let mut losses = vec![];
        apply(&mut p, &mut doc, &[comment, exact], &mut losses).unwrap();
        assert_eq!(losses.len(), 1);
        assert_eq!(losses[0].location.as_deref(), Some("ambiguous"));
        let comments = p.xml("word/custom-comments.xml").unwrap().unwrap();
        let mut all = vec![];
        comments.descendants("w:comment", &mut all);
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].get("w:id"), Some("10"));
        assert_eq!(
            all[1].child("w:p").unwrap().get("w14:paraId"),
            Some("50000001")
        );
        assert!(!p.parts.contains_key("word/comments.xml"));
    }
    #[test]
    fn exact_unicode_ranges_replies_and_true_revisions() {
        let mut p = Package::load(None).unwrap();
        let mut doc = p.xml("word/document.xml").unwrap().unwrap();
        let mut paragraph = Element::new("w:p");
        let mut r = text_run("甲😀乙丙", &Element::new("w:rPr"));
        r.source = Some((100, "甲😀乙丙".into()));
        paragraph.push(r);
        doc.child_mut("w:body").unwrap().push(paragraph);
        let comment = DocxReview {
            id: "one".into(),
            parent: None,
            author: "作者".into(),
            date: word_comment_date("unix:0"),
            body: "意见".into(),
            resolved: false,
            range: Some((103, 110)),
            whole_document: false,
            replacement: Some("替换".into()),
            downgrade_reason: None,
            empty_body_point: false,
        };
        let mut reply = comment.clone();
        reply.id = "two".into();
        reply.parent = Some("one".into());
        reply.body = "回复".into();
        reply.replacement = None;
        let mut losses = vec![];
        apply(&mut p, &mut doc, &[comment, reply], &mut losses).unwrap();
        assert!(losses.is_empty());
        let mut deleted = vec![];
        doc.descendants("w:delText", &mut deleted);
        assert_eq!(deleted.iter().map(|r| r.text()).collect::<String>(), "😀乙");
        let mut inserts = vec![];
        doc.descendants("w:ins", &mut inserts);
        assert_eq!(inserts[0].text(), "替换");
        let mut starts = vec![];
        doc.descendants("w:commentRangeStart", &mut starts);
        assert_eq!(starts.len(), 2);
        let c = p.xml("word/comments.xml").unwrap().unwrap();
        let mut all = vec![];
        c.descendants("w:comment", &mut all);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].get("w:date"), Some("1970-01-01T00:00:00Z"));
        let ex = p.xml("word/commentsExtended.xml").unwrap().unwrap();
        assert!(p.types.children.iter().filter_map(Xml::element).any(|e| e.get("PartName") == Some("/word/commentsExtended.xml") && e.get("ContentType") == Some("application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml")));
        let mut references = vec![];
        doc.descendants("w:commentReference", &mut references);
        assert_eq!(
            references
                .iter()
                .map(|e| e.get("w:id").unwrap())
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
        assert!(
            ex.children
                .iter()
                .filter_map(Xml::element)
                .any(|e| e.get("w15:paraIdParent").is_some())
        );
        assert!(
            !String::from_utf8(xml::serialize(&doc))
                .unwrap()
                .contains("source")
        );
    }
}
