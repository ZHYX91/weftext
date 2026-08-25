---
source_language: zh-CN
translation_of: 16-citations-and-bibliography.zh-CN.md
translation_status: synced
---

[简体中文](16-citations-and-bibliography.zh-CN.md)

# Weftext citations and bibliography

This specification defines citation occurrence, resolution, compilation, and presentation boundaries. Reference-record creation and editing require the typed Citation Data construct described here.

## Current target boundary

The canonical YAML frontmatter contains only the `weftext` operational mapping. Structured bibliographic facts are authored document data but are too complex for generic string attributes. They therefore require a versioned typed Weftext AsciiDoc construct. This specification retains the accepted `cite:`, `nocite::`, `bibliography::`, UUID resolution, compilation, and provider-neutral CSL boundaries, but does not invent the replacement record syntax without parser, editing, generic-degradation, import/export, and fixture evidence.

Until that construct is accepted, no canonical reference-record authoring surface exists. Citation occurrences without available typed records remain diagnosed rather than guessed or dropped. Markdown `@key` forms, Pandoc forms, native AsciiDoc bibliography anchors, and third-party `citep`/`citet`/`citenp` macros remain explicit importer inputs rather than parser aliases.

The `cite:[key]` and `bibliography::[]` spellings are the established intersection of the `asciidoctor-bibtex` and `asciidoctor-bibliography` extensions. Immediate `+[key]` cluster continuation is adopted from `asciidoctor-bibliography`. This provenance is a surface-language choice, not a runtime dependency or compatibility promise: Weftext loads neither Ruby gem, does not use a document-selected `.bib` file as authority, and does not accept either extension's remaining aliases, attributes, database settings, or rendering behavior. Weftext owns the exact grammar below, ordinary-node data authority, UUID resolution, transactions, compilation scope, and provider-neutral CSL boundary.

Core owns reference metadata validation, citation parsing, source ranges, key resolution, rename planning, diagnostics, citation clusters, bibliography collection, and the provider-neutral CSL presentation request. Desktop, CLI, Server, WebUI, exporters, and agents consume that model and may not invent private citation syntax or render from unresolved source text.

## Citation source grammar

The canonical inline macro is `cite`. Parenthetical is the default cluster form:

```adoc
The result has been replicated cite:[wang2024].
```

The `narrative` target selects narrative form for exactly one item:

```adoc
cite:narrative[wang2024] reports the same result.
```

The first positional attribute of each citation item is its key. The optional named attributes are `label`, `locator`, `prefix`, and `suffix`:

```adoc
See cite:[wang2024,label=page,locator="59-61"].

See cite:[wang2024,prefix="compare ",suffix=", especially figure 2"].
```

An immediately chained `+[...]` item belongs to the same parenthetical citation cluster and owns its own attributes. There is no whitespace between items:

```adoc
Several studies agree cite:[wang2024,label=page,locator=59]+[smith2025,label=chapter,locator=2].
```

The cluster form belongs to the complete macro occurrence. Narrative clusters cannot use `+[...]`; multiple narrative references are written as separate `cite:narrative[...]` occurrences joined by authored prose. A selected presenter that cannot produce the requested single-item narrative form fails with a precise diagnostic rather than silently changing form.

Citation item contents use a restricted literal attribute grammar. Items and named attributes are comma-separated outside double quotes; names are lowercase ASCII and may occur at most once. An unquoted value is a non-empty token without whitespace, comma, either quote, square bracket, backslash, or brace. A value that needs whitespace or punctuation uses a JSON-compatible double-quoted string with JSON escapes. Single-quoted strings, AsciiDoc attribute references, substitutions, inline markup, nested macros, and executable content are not evaluated or accepted as alternate value forms. Prefix and suffix are plain Unicode strings after JSON decoding, not nested AsciiDoc. Canonical writers quote a value whenever the unquoted-token rule would reject it.

Arbitrary display overrides, raw rendered citations, style names, locale names, citation numbers, and provider-specific flags are not citation-item attributes. A locator without an explicit `label` means `page`; a `label` without `locator` is invalid. Locator labels use the closed CSL-aligned v1 vocabulary defined by the schema and parser.

Unknown targets, attributes, locator labels, malformed chains, empty clusters, invalid keys, invalid literal values, narrative chains, and duplicate named attributes remain exact source and are diagnosed. Protected YAML, comments, literal/listing/source/passthrough blocks, inline passthroughs, and other profile-protected ranges never produce citation occurrences. Citation parsing occurs only where the pinned AsciiDoc baseline enables macro substitution; escaped or protected examples are not occurrences.

## Bibliography and uncited inclusion

The block macro marks an authored bibliography placement:

```adoc
bibliography::[]
```

The default inclusion is every uniquely resolved reference cited in the selected compilation scope, deduplicated by node UUID rather than citation key. In ordinary single-node Read/Write, the compilation component and default scope are that node. A multi-node export plan must supply an ordered component and an explicit permission-filtered reference scope before rendering; path proximity, open tabs, backlinks, or the position of the bibliography macro never infer membership. `include=all` additionally selects every valid reference-capable node in that explicit scope:

```adoc
bibliography::[include=all]
```

Specific uncited references are declared with a separate non-rendering block macro. A `nocite` contributes only to the component that contains it:

```adoc
nocite::[wang2024,smith2025]
```

Citation style, bibliography style, locale, sort, heading text and level, and compilation scope belong to the resolved export profile. They are not document attributes and are not persisted on reference nodes. A bibliography macro is a view placement, not a database declaration or materialized list. Citation Data may be imported from or exported to BibTeX, CSL-JSON, RIS, or another reviewed format, but no external database path becomes canonical document syntax.

Citation Data v1 permits at most one bibliography placement in one generated document component. A component with no placement still renders inline citations but materializes no bibliography in canonical source. Section bibliographies, filtered bibliographies, multiple bibliography views, note-style citation placement, and publication-wide cross-document numbering require a later explicit profile version rather than ambiguous v1 behavior.

## Presentation boundary

Core lowers resolved clusters and reference records into a provider-neutral presentation request. The accepted presenter uses a pinned CSL capability and returns structured rich text plus stable source associations; clients do not scrape HTML. Rendering is deterministic for the same ordered clusters, reference facts, style, locale, and export plan.

The presenter does not read the workspace, resolve keys, fetch URLs, load arbitrary processors, execute JavaScript or Ruby, or choose files. Missing or duplicate keys, invalid reference metadata, unavailable styles/locales, unsupported locators, and malformed presenter output are fail-closed diagnostics.

Citation Presentation v1 fixes the available styles to `apa`, `vancouver`, and `chicago-notes` and locales to `en-US`, `zh-CN`, and `ar`. Its capability response identifies the presenter and version and declares an offline data-only isolation mode with packaged-allowlist asset loading. Reference UUIDs, never authored keys, are provider item identity and returned run/entry ownership. Bibliography inclusion is supplied explicitly by the resolved compilation; hidden provider requests for uncited entries are ordered after rendered clusters so they cannot change repeated-citation behavior.

The pinned provider cannot safely express Citation Data v1 literal or `circa` dates and treats its `note` input as processor control rather than an ordinary CSL fact, so those fields fail with an unsupported-reference-data diagnostic in Presentation v1. Text containing `<` also fails closed because this provider interprets markup-like input; no caller or client receives provider markup. Provider links are accepted only as typed `http` or `https` runs, and provider markup/transparent elements, missing or extra bibliography entries, duplicate entry boundaries, invalid UUIDs, and panics all fail without partial output. These are explicit presentation limits, not alternate storage syntax and not permission to discard the authored facts.

Native AsciiDoc `<<key>>` references and `[[[key]]]` bibliography anchors are an optional generic export lowering only. That lowering is lossy and never becomes storage authority or a round-trip promise.

## Editing and transactions

Citation insertion, citation-cluster editing, `nocite`, and bibliography placement are typed Core plans. UI forms and pickers submit typed intent and never patch source strings independently. Reference creation, field editing, and key rename cannot ship until the typed Citation Data construct defines their source authority.

A future key rename must resolve one declaration and every affected occurrence by UUID, verify a complete source set, preview exact edits, commit atomically, and roll back exactly on failure. Merely opening, indexing, rendering, importing, or viewing a reference never normalizes or rewrites its source.

## Acceptance boundary

The feature is available only when all of the following are true:

1. The typed Citation Data construct has a machine-readable schema and cross-platform positive, negative, malformed, CJK, RTL, and exact-source fixtures.
2. Core parses macros with exact UTF-8 ranges and excludes every protected context.
3. Reference resolution, duplicate/missing diagnostics, key rename, and derived indexes are headlessly tested.
4. The CSL presenter is dependency-reviewed, deterministic, offline-capable, sandboxed from workspace and network access, and exercised against multiple citation styles and locales.
5. Bibliography scope, ordering, `nocite`, empty output, and presenter failures are fail-closed tests.
6. Desktop and WebUI citation picker, reference form, Source/Write/Read projections, stale-draft behavior, and accessibility/IME paths consume the same Core model without a browser-owned parser.
7. CLI and Server expose the same revisions, diagnostics, authorization, and transaction behavior without direct filesystem or client-side parser authority.
