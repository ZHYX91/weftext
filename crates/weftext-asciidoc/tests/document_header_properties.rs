use std::fmt::Write as _;
use std::ops::Range;

use weftext_asciidoc::{
    DocumentHeaderAttributeForm, DocumentHeaderAttributeKind, DocumentHeaderIssueCode,
    DocumentHeaderPatchError, SourceEditPlan, analyze_document_header,
    patch_document_header_attribute, plan_document_header_attribute_patch,
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
fn projects_only_bounded_literal_properties_from_the_document_header() {
    let source = format!(
        concat!(
            "---\r\nweftext:\r\n  id: \"{}\"\r\n---\r\n",
            "= 文缕：属性 authority\r\n",
            "Zheng Ming <zheng@example.com>\r\n",
            "v1.0, 2026-08-24\r\n",
            ":lang: zh-CN\r\n",
            ":status: 审阅中 😀\r\n",
            ":literal: {{env}}/https://example.invalid\r\n",
            ":toc: left\r\n",
            "\r\n",
            "正文\r\n",
            ":status: body-only\r\n"
        ),
        UUID
    );
    let header = analyze_document_header(&source);

    assert_eq!(exact(&source, &header.range).chars().next(), Some('='));
    assert!(exact(&source, &header.range).ends_with(":toc: left\r\n"));
    assert_eq!(
        header
            .attributes
            .iter()
            .filter(|attribute| attribute.projected)
            .map(|attribute| { (attribute.name.as_str(), attribute.literal_value.as_deref(),) })
            .collect::<Vec<_>>(),
        [
            ("lang", Some("zh-CN")),
            ("status", Some("审阅中 😀")),
            ("literal", Some("{env}/https://example.invalid"))
        ]
    );
    let toc = header
        .attributes
        .iter()
        .find(|attribute| attribute.name == "toc")
        .expect("processor attribute evidence");
    assert_eq!(toc.kind, DocumentHeaderAttributeKind::ProcessorControl);
    assert_eq!(toc.literal_value.as_deref(), Some("left"));
    assert!(!toc.projected);
    assert_eq!(exact(&source, &toc.name_range), "toc");
    assert_eq!(exact(&source, &toc.value_range), "left");
    assert!(header.issues.iter().any(|issue| {
        issue.code == DocumentHeaderIssueCode::ProcessorControl
            && issue.name.as_deref() == Some("toc")
            && exact(&source, &issue.range) == ":toc: left\r\n"
    }));
    assert!(
        !header
            .attributes
            .iter()
            .any(|attribute| attribute.literal_value.as_deref() == Some("body-only"))
    );

    for attribute in &header.attributes {
        let _ = exact(&source, &attribute.range);
        let _ = exact(&source, &attribute.name_range);
        let _ = exact(&source, &attribute.value_range);
    }
}

#[test]
fn duplicate_unset_and_continued_entries_keep_exact_patch_evidence() {
    let source = concat!(
        ":project: 文缕\n",
        "// prefix comment\n",
        "\n",
        "= 标题\n",
        "Zheng Ming <zheng@example.com>\n",
        "v2.0, 2026-08-24\n",
        ":status: first\n",
        ":status: second\n",
        ":wrapped: first \\\n",
        "第二行 \\\n",
        "third line\n",
        ":!hidden:\n",
        ":after: 安全\n",
        "\n",
        "Body\n"
    );
    let header = analyze_document_header(source);

    assert_eq!(
        header
            .attributes
            .iter()
            .filter(|attribute| attribute.projected)
            .map(|attribute| attribute.name.as_str())
            .collect::<Vec<_>>(),
        ["project", "status", "after"]
    );
    let wrapped = header
        .attributes
        .iter()
        .find(|attribute| attribute.name == "wrapped")
        .expect("continued entry");
    assert_eq!(wrapped.continuation_ranges.len(), 2);
    assert_eq!(
        exact(source, &wrapped.range),
        ":wrapped: first \\\n第二行 \\\nthird line\n"
    );
    assert_eq!(
        exact(source, &wrapped.value_range),
        "first \\\n第二行 \\\nthird line"
    );
    assert_eq!(
        exact(source, &wrapped.continuation_ranges[0]),
        "第二行 \\\n"
    );
    assert_eq!(
        exact(source, &wrapped.continuation_ranges[1]),
        "third line\n"
    );
    assert!(header.issues.iter().any(|issue| {
        issue.code == DocumentHeaderIssueCode::DuplicateName
            && issue.name.as_deref() == Some("status")
    }));
    assert!(header.issues.iter().any(|issue| {
        issue.code == DocumentHeaderIssueCode::ContinuedValue
            && issue.name.as_deref() == Some("wrapped")
    }));
    assert!(header.issues.iter().any(|issue| {
        issue.code == DocumentHeaderIssueCode::UnsupportedUnset
            && issue.name.as_deref() == Some("hidden")
    }));
    let hidden = header
        .attributes
        .iter()
        .find(|attribute| attribute.name == "hidden")
        .expect("unset evidence");
    assert_eq!(hidden.form, DocumentHeaderAttributeForm::Unset);
    assert_eq!(exact(source, &hidden.name_range), "hidden");

    assert_eq!(
        patch_document_header_attribute(source, "status", Some("third")),
        Err(DocumentHeaderPatchError::DuplicateName)
    );
    assert_eq!(
        patch_document_header_attribute(source, "wrapped", Some("flat")),
        Err(DocumentHeaderPatchError::UnsupportedHeader)
    );
    assert_eq!(
        patch_document_header_attribute(source, "hidden", Some("visible")),
        Err(DocumentHeaderPatchError::UnsupportedHeader)
    );
}

#[test]
fn patches_are_narrow_for_crlf_removal_insertion_and_missing_final_newline() {
    let source = format!(
        "---\r\nweftext:\r\n  id: \"{UUID}\"\r\n---\r\n= Title\r\n:status: old\r\n\r\nBody\r\n:status: body\r\n"
    );
    let replaced =
        patch_document_header_attribute(&source, "status", Some("新值")).expect("replace property");
    assert_eq!(
        replaced,
        source.replacen(":status: old", ":status: 新值", 1)
    );

    let inserted = patch_document_header_attribute(&replaced, "project", Some("Weftext"))
        .expect("insert property");
    assert!(inserted.contains(":status: 新值\r\n:project: Weftext\r\n\r\nBody"));
    let removed =
        patch_document_header_attribute(&inserted, "status", None).expect("remove property");
    assert!(!removed.contains(":status: 新值"));
    assert!(removed.contains(":status: body"));

    assert_eq!(
        patch_document_header_attribute("= Title", "status", Some("draft")),
        Ok("= Title\n:status: draft\n".to_owned())
    );
    assert_eq!(
        patch_document_header_attribute("Body", "status", Some("draft")),
        Ok(":status: draft\n\nBody".to_owned())
    );
    assert_eq!(
        patch_document_header_attribute("= T\n\n", "path", Some("C:\\")),
        Ok("= T\n:path: C:\\\n\n".to_owned())
    );
    assert_eq!(
        patch_document_header_attribute("= T\n:status:   \n\n", "status", Some("draft")),
        Ok("= T\n:status:   draft\n\n".to_owned())
    );
    assert_eq!(
        patch_document_header_attribute("= T\n:status: old\nBody\n", "status", None),
        Err(DocumentHeaderPatchError::UnsupportedHeader),
        "removal must not let an unseparated body line become native header metadata"
    );
}

#[test]
fn unclosed_envelope_and_attribute_count_limit_fail_closed() {
    let unclosed = format!("---\nweftext:\n  id: \"{UUID}\"\n:status: hidden\n");
    let header = analyze_document_header(&unclosed);
    assert!(header.attributes.is_empty());
    assert!(header.issues.iter().any(|issue| {
        issue.code == DocumentHeaderIssueCode::UnclosedEnvelope
            && issue.range == (0..unclosed.len() as u64)
    }));
    assert_eq!(
        patch_document_header_attribute(&unclosed, "status", Some("draft")),
        Err(DocumentHeaderPatchError::UnclosedEnvelope)
    );

    let mut oversized = String::from("= T\n");
    for index in 0..=256 {
        let _ = writeln!(oversized, ":property-{index}: {index}");
    }
    oversized.push_str("\nBody\n");
    let header = analyze_document_header(&oversized);
    assert_eq!(header.attributes.len(), 256);
    assert!(
        header
            .issues
            .iter()
            .any(|issue| issue.code == DocumentHeaderIssueCode::AttributeLimitExceeded)
    );
    assert_eq!(
        patch_document_header_attribute(&oversized, "new-property", Some("value")),
        Err(DocumentHeaderPatchError::UnsupportedHeader)
    );
}

#[test]
fn author_revision_and_body_boundaries_follow_header_order() {
    let source = concat!(
        "= Title\n",
        "// author position is now closed\n",
        "This is body text\n",
        ":status: body-only\n"
    );
    let header = analyze_document_header(source);
    assert!(header.attributes.is_empty());
    assert!(exact(source, &header.range).ends_with("// author position is now closed\n"));

    let patched = patch_document_header_attribute(source, "status", Some("draft"))
        .expect("insert before body");
    assert_eq!(
        patched,
        concat!(
            "= Title\n",
            "// author position is now closed\n",
            ":status: draft\n",
            "\n",
            "This is body text\n",
            ":status: body-only\n"
        )
    );
}

#[test]
fn exact_patch_planner_returns_one_verified_base_relative_edit_or_no_op() {
    let source = format!(
        concat!(
            "---\r\nweftext:\r\n  id: \"{}\"\r\n---\r\n",
            "= 标题 😀\r\n",
            "// تعليق محفوظ\r\n",
            ":unknown-note: 保留\r\n",
            ":status:   old\r\n",
            ":literal: {{{{env}}}}\r\n",
            "\r\n",
            "正文 אבג\n",
        ),
        UUID
    );
    let edit = plan_document_header_attribute_patch(&source, "status", Some("新值 😀"))
        .expect("replace plan")
        .expect("one edit");
    assert_eq!(&source[edit.range.clone()], "old");
    assert_eq!(edit.replacement, "新值 😀");
    let proposed = SourceEditPlan::new(&source, vec![edit.clone()])
        .expect("valid edit")
        .apply(&source)
        .expect("apply edit");
    assert_eq!(&source[..edit.range.start], &proposed[..edit.range.start]);
    assert_eq!(
        &source[edit.range.end..],
        &proposed[edit.range.start + edit.replacement.len()..]
    );
    assert!(proposed.contains(":status:   新值 😀\r\n"));
    assert!(proposed.ends_with("正文 אבג\n"));

    assert_eq!(
        plan_document_header_attribute_patch(&source, "status", Some("old")),
        Ok(None),
        "decoded literal equality is an exact no-op"
    );
    assert_eq!(
        plan_document_header_attribute_patch(&source, "absent", None),
        Ok(None)
    );

    let removal = plan_document_header_attribute_patch(&source, "status", None)
        .expect("removal plan")
        .expect("one removal");
    assert_eq!(&source[removal.range.clone()], ":status:   old\r\n");
    assert!(removal.replacement.is_empty());

    let missing_final_newline = "= 标题 😀";
    let insertion =
        plan_document_header_attribute_patch(missing_final_newline, "status", Some("draft"))
            .expect("insertion plan")
            .expect("one insertion");
    assert_eq!(
        insertion.range,
        missing_final_newline.len()..missing_final_newline.len()
    );
    assert_eq!(insertion.replacement, "\n:status: draft\n");
    assert_eq!(
        patch_document_header_attribute(missing_final_newline, "status", Some("draft")),
        Ok("= 标题 😀\n:status: draft\n".to_owned())
    );
}
