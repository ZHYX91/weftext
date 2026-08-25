use weftext_core::{
    AdjacentHeadingBody, analyze_citation_source, analyze_document, analyze_query_source,
    analyze_task_source,
};

#[test]
fn document_header_is_one_exact_protected_authority_for_all_body_domain_scanners() {
    let source = concat!(
        "---\r\n",
        "weftext:\r\n",
        "  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n",
        "---\r\n",
        "= 标题 cite:[title2024] 😀\r\n",
        ":note: cite:[header2024]\r\n",
        ":task-example: * [ ] hidden task:[id=11111111-1111-4111-8111-111111111111]\r\n",
        ":query-example: [.weftext-query,version=1,view=task-list] ....\r\n",
        "\r\n",
        "正文 cite:[visible2024]\n",
        "* [ ] visible task:[id=22222222-2222-4222-8222-222222222222]\n",
        "\n",
        "[.weftext-query,version=1,view=task-list]\n",
        "....\n",
        "from tasks as task\n",
        "scope workspace\n",
        "where task.closed = false\n",
        "select task.title, task.owner_node.path\n",
        "order by task.owner_node.path asc\n",
        "limit 100\n",
        "....\n",
    );

    let profile = weftext_asciidoc::analyze(source);
    let header = &profile.document_header.range;
    let header_start = usize::try_from(header.start).unwrap();
    let header_end = usize::try_from(header.end).unwrap();
    assert!(source.is_char_boundary(header_start));
    assert!(source.is_char_boundary(header_end));
    assert_eq!(
        &source[header_start..header_end],
        concat!(
            "= 标题 cite:[title2024] 😀\r\n",
            ":note: cite:[header2024]\r\n",
            ":task-example: * [ ] hidden task:[id=11111111-1111-4111-8111-111111111111]\r\n",
            ":query-example: [.weftext-query,version=1,view=task-list] ....\r\n",
        )
    );
    assert!(
        profile
            .protected_ranges
            .iter()
            .any(|range| range.start <= header.start && header.end <= range.end)
    );

    let core = analyze_document(source, AdjacentHeadingBody::Separate);
    assert!(
        core.occurrences
            .protected_ranges
            .iter()
            .any(|range| range.start <= header.start && header.end <= range.end)
    );
    assert!(
        core.occurrences
            .eligible_text_ranges
            .iter()
            .all(|range| { range.end <= header.start || header.end <= range.start })
    );

    let citations = analyze_citation_source(source);
    assert_eq!(citations.clusters.len(), 1);
    assert_eq!(citations.clusters[0].items[0].key.key, "visible2024");

    let tasks = analyze_task_source(source);
    assert_eq!(tasks.tasks.len(), 1);
    assert_eq!(
        tasks.tasks[0]
            .metadata
            .as_ref()
            .expect("visible structured task")
            .id
            .to_string(),
        "22222222-2222-4222-8222-222222222222"
    );

    let queries = analyze_query_source(source);
    assert_eq!(queries.blocks.len(), 1);
    assert!(queries.blocks[0].valid);
}
