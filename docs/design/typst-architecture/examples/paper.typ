#metadata((
  id: "8407d3ac-b941-4447-9b4f-a78d85b35269", types: ("paper",),
  publication: (
    kind: "article", title: "原生数据的结构化笔记",
    alternative_titles: ((text: "Structured notes with native data", language: "en"),),
    contributors: (
      (agent: (ref: "weftext-node://2fd54310-65f7-4d3c-8668-2f1e344a56c0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654"), credited_as: "Zhang San",
       family: "Zhang", given: "San", roles: ("author",)),
      (agent: (text: "Li Si"), family: "Li", given: "Si", roles: ("author",)),
      (agent: (text: "Example Consortium"), roles: ("author",)),
    ),
    issued: (calendar: "gregorian", precision: "month", year: 2026, month: 10),
    publication_history: (
      (state: "accepted", during: (start: datetime(year: 2026, month: 8, day: 1))),
      (state: "published", during: (start: datetime(year: 2026, month: 10, day: 12))),
    ),
    container: "Example Research Journal", volume: "12", issue: "3", pages: "101–114",
    identifiers: ((scheme: "doi", value: "10.0000/example.placeholder"),),
    url: "https://example.org/paper",
    abstract: "Fictional research fixture; no publication claim.",
    keywords: ("notes", "structured data"), language: "en", citation_key: "fixture2026",
    sources: ((reference: (url: "https://example.org/paper"), note: "演示导入来源"),),
  ),
)) <weftext>
#context {
  let paper = query(<weftext>).first().value
  title(paper.publication.title)
}
= 阅读笔记
正式文献标题仅在 publication 中维护。
