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
    pub downgrade_reason: Option<String>,
    #[serde(default)]
    pub empty_body_point: bool,
}
struct Located {
    review: DocxReview,
    id: usize,
    start: (usize, usize),
    end: (usize, usize),
}
fn loss(out: &mut Vec<ExportLoss>, id: &str, message: &str) {
    out.push(ExportLoss {
        code: "word-review-unmapped".into(),
        message: message.into(),
        location: Some(id.into()),
    });
}
fn collect(e: &Element, index: &mut usize, runs: &mut Vec<(usize, usize, String)>) {
    if e.name == "w:r" {
        if let Some((from, text)) = &e.source {
            runs.push((*index, *from, text.clone()));
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
        if n.name != "w:r" {
            rewrite(&mut n, index, located, next);
            out.push(Xml::Element(n));
            continue;
        }
        let run_index = *index;
        *index += 1;
        let Some((_, text)) = n.source.clone() else {
            out.push(Xml::Element(n));
            continue;
        };
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
        return Err(DocxError::new("批注输出超过 2000 条预算"));
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
                message:
                    "为保留可处理的批注讨论，审阅副本升级为 Word 2013 兼容模式；旧模板版式可能变化"
                        .into(),
                location: Some("word/settings.xml".into()),
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
        return Err(DocxError::new("Word 审阅标识超出支持范围"));
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
    for review in reviews.iter().filter(|r| r.parent.is_none()) {
        if let Some(reason) = &review.downgrade_reason {
            loss(losses, &review.id, reason);
        }
        let position = if review.whole_document {
            runs.first()
                .zip(runs.last())
                .map(|(a, b)| ((a.0, 0), (b.0, b.2.len())))
        } else {
            review.range.and_then(|(from, to)| {
                if from > to {
                    return None;
                }
                let starts: Vec<_> = runs
                    .iter()
                    .filter(|(_, a, t)| {
                        *a <= from && from < *a + t.len() && t.is_char_boundary(from - *a)
                    })
                    .collect();
                let ends: Vec<_> = runs
                    .iter()
                    .filter(|(_, a, t)| {
                        *a < to && to <= *a + t.len() && t.is_char_boundary(to - *a)
                    })
                    .collect();
                if from == to {
                    let empty: Vec<_> = runs
                        .iter()
                        .filter(|(_, at, text)| {
                            *at == from && text.is_empty() && review.empty_body_point
                        })
                        .collect();
                    if empty.len() == 1 {
                        return Some(((empty[0].0, 0), (empty[0].0, 0)));
                    }
                    if starts.len() == 1 {
                        return Some((
                            (starts[0].0, from - starts[0].1),
                            (starts[0].0, from - starts[0].1),
                        ));
                    }
                    if starts.is_empty() && ends.len() == 1 {
                        return Some(((ends[0].0, to - ends[0].1), (ends[0].0, to - ends[0].1)));
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
                .then_some(((a.0, from - a.1), (b.0, to - b.1)))
            })
        };
        let Some((start, end)) = position else {
            loss(
                losses,
                &review.id,
                "批注目标不能唯一映射到本次输出正文；原批注保留，本工件未附着该批注",
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
                "重叠修订不能安全同时输出；此项仅作为普通批注",
            );
            review.replacement = None;
        }
        located.push(Located {
            review,
            id: next,
            start,
            end,
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
                    "回复的父批注未能导出；讨论仍保留在原批注文件",
                );
            }
            break;
        }
    }
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
    for review in reviews {
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
