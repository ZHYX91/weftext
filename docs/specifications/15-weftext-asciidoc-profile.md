---
source_language: zh-CN
translation_of: 15-weftext-asciidoc-profile.zh-CN.md
translation_status: synced
---

[简体中文](15-weftext-asciidoc-profile.zh-CN.md)

# Weftext AsciiDoc Profile v1

This is the canonical managed-source profile. A managed document uses the required marker, the `weftext` envelope, AsciiDoc source, and the annotation sidecar. Markdown is accepted only through explicit import/export or as visible unmanaged content. Checklist/task nodes, Query, Template Library, Citation Data, lifecycle, accessibility/IME, restore, and cross-platform clients must preserve the contracts defined here and in the linked architecture specifications.

## Conservative-superset compatibility model

Weftext AsciiDoc v1 is a conservative source-language superset of the pinned AsciiDoc baseline, not a hand-picked syntax subset. The reference baseline is Asciidoctor 2.0.26 in secure mode with no third-party extensions; the profile conformance corpus, not a floating `latest` document, freezes the exact accepted behavior. A source construct that is valid in that baseline remains valid Weftext source, retains its native meaning, and survives open/edit/save byte-for-byte unless the author invokes an explicit Core action. Future upstream behavior enters only through a reviewed profile-version transition.

Source acceptance, semantic modeling, rendering, and permission to perform an effect are separate promises. In particular, recognizing an include, conditional, passthrough, URI, or processor declaration does not grant filesystem, network, environment, extension-loading, or executable access. A disabled active construct remains valid exact source with a capability diagnostic. This execution policy is not permission to reinterpret or discard the construct.

Every construct is recorded on two independent axes instead of being forced into one compatibility class:

| Axis | Values |
| --- | --- |
| Surface provenance | pinned AsciiDoc baseline; named Asciidoctor-compatible dialect; adopted ecosystem extension; Weftext extension |
| Weftext support state | full; constrained; preserve-only; prohibited effect |

The capability table additionally records whether Weftext owns any added semantics, the generic-processor degradation, compatibility-export behavior, and the first profile version that accepts the construct. A native role with Weftext presentation semantics, an ecosystem diagram spelling with a Weftext-safe renderer, and a preserved-but-disabled include therefore no longer receive misleading mutually exclusive labels.

Extensions must use an AsciiDoc extension point, must not change the meaning of a valid baseline construct, and must have an exact-source grammar, protected-context rules, diagnostics, generic degradation, and compatibility lowering. Desktop, CLI, Server, WebUI, importers, exporters, and agents consume one versioned Core model and may not add private syntax. A generic AsciiDoc processor is an interoperability target, not Weftext storage authority. Exact source always wins over a derived rendering.

The `cite` inline macro and `nocite` and `bibliography` block macros adopt a small established Asciidoctor-extension spelling while Weftext owns resolution, transactions, scope, and presentation semantics; Weftext does not load or claim compatibility with either `asciidoctor-bibtex` or `asciidoctor-bibliography`. Canonical reference-record authoring remains gated on a separately accepted typed Citation Data Profile construct. The complete authority and acceptance boundary are defined in [`16-citations-and-bibliography.md`](16-citations-and-bibliography.md).

The current capability inventory is:

| Construct | Surface provenance | Weftext support state | Added semantics and generic degradation |
| --- | --- | --- | --- |
| `weftext` YAML frontmatter envelope | ecosystem convention recognized by Asciidoctor skip-frontmatter behavior | full, Weftext-owned operational envelope | generic processors require skip-frontmatter configuration |
| document title/subtitle and H1–H5 | AsciiDoc baseline | full | native degradation |
| H6–H9 | Weftext extension | full | generic processors require compatibility lowering or warning |
| `[.run-in]` / `[.separate]` | native role syntax | full with Weftext presentation | generic processors keep an ordinary role-bearing section |
| `[quote] ____` | AsciiDoc baseline | full | native degradation |
| `>` nested quotation | named Asciidoctor-compatible dialect | full through depth 9 | compatibility export may lower to native quote blocks |
| native anchors and `xref:` | AsciiDoc baseline | full | native degradation |
| `node:` / `node::` | Weftext extension | full for links; block embed deferred | generic processors preserve an unknown macro; export lowers explicitly |
| block titles, tables, source/listing/literal blocks, images | AsciiDoc baseline | full within Core actions and safe renderers | native degradation |
| STEM and `latexmath` | AsciiDoc baseline | constrained safe rendering | unsupported expressions remain source; no TeX execution |
| `[mermaid] ....` | adopted diagram-extension spelling | constrained Weftext renderer | literal block source remains visible without the extension |
| `footnote:` | AsciiDoc baseline | full source semantics | native degradation |
| `endnote:` / `endnotes::[]` | Weftext extension | accepted, runtime deferred | unknown macros remain source; export lowers explicitly |
| `cite:`, `nocite::`, `bibliography::` | bibliography-extension spellings | constrained Weftext grammar | Weftext owns data/resolution/scope; generic export is explicit |
| native checklist | AsciiDoc baseline | exact-source checklist recognition | native static checklist degradation |
| `weftext-task` header profile | Weftext extension over native document attributes | canonical task-node profile | generic processors preserve literal document attributes and body |
| trailing `task:[...]` | superseded Weftext extension | explicit migration input only | never canonical output |
| `[.weftext-query,version=1,...] ....` | Weftext role-bearing literal-block extension | canonical Query block | generic processors retain a role-bearing literal block |
| `slot:name[]` / `slot::name[]` | Weftext inline/block macro extensions | constrained to valid Template Root/Part source; unavailable outside a configured Template Library | ordinary/generic AsciiDoc preserves unknown macros without evaluation |
| includes and conditionals | AsciiDoc baseline | source accepted, effects constrained | no path/network expansion without reviewed Core capability |
| passthrough and arbitrary processors | AsciiDoc baseline or processor extension | source accepted, effects prohibited by default | exact source plus capability diagnostic; never automatic execution |
| unknown third-party extensions | ecosystem extension | preserve-only | no dynamic loading; explicit adapter/export work is required |

## Managed document envelope

The only managed-node shape is `X/X.adoc`. The directory remains the current locator and the lowercase UUIDv4 remains identity. YAML frontmatter is a Weftext operational envelope before the AsciiDoc document header. It has exactly one top-level `weftext` mapping:

```adoc
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
  icon: "weftext:project"
  aliases:
    - 文缕
---
= Weftext: Local-first knowledge workspace
:lang: en
:description: Local-first knowledge workspace notes
:status: draft

== First section
Body text.
```

YAML frontmatter is part of the Weftext profile, not a claim that every AsciiDoc processor interprets YAML. Core owns its exact range and narrow patches. Processors used by Weftext must skip it rather than render it as document text. Top-level keys other than `weftext`, the old `_weftext` spelling, and general user YAML properties are not canonical.

`weftext` contains only the shallow Weftext operational fields `id`, `icon`, `aliases`, `child_sort`, `child_sort_direction`, `sibling_rank`, and root-only `adjacent_heading_body` plus `template_library_root`. The latter is a lowercase UUIDv4 selecting the one Template Library container; absence disables that role projection. Title, author, revision, language, description, keywords/tags, status, and custom note metadata use the native document header and its attributes. Only document-header attribute entries enter the stable Properties projection; later body redefinitions remain AsciiDoc processor state. Readers use bounded literal values and never expand properties through environment, path, URI, or executable effects.

The product has no competing managed document type. `.md` remains importable, exportable, and eligible for unmanaged classification, but is never selected as a canonical managed-node format.

## Document title, subtitle, and sections

The native document-title form is authoritative:

```adoc
= Main title: Subtitle
```

The first valid level-zero title is the document title. The native colon separator supplies the subtitle. Weftext does not duplicate either value in YAML. A managed document may omit the title, in which case navigation falls back to the node name; omission does not turn a later section heading into the document title. A second level-zero title is invalid profile structure rather than another body heading.

Document Title and Subtitle are header metadata. They do not become body-outline entries or consume a body heading level.

Body headings map as follows:

| Body level | Source marker | Provenance | Support |
| --- | --- | --- | --- |
| Heading 1 | `==` | AsciiDoc baseline | full |
| Heading 2 | `===` | AsciiDoc baseline | full |
| Heading 3 | `====` | AsciiDoc baseline | full |
| Heading 4 | `=====` | AsciiDoc baseline | full |
| Heading 5 | `======` | AsciiDoc baseline | full |
| Heading 6 | `=======` | Weftext extension | full |
| Heading 7 | `========` | Weftext extension | full |
| Heading 8 | `=========` | Weftext extension | full |
| Heading 9 | `==========` | Weftext extension | full |

H1–H5 use the native section range. H6–H9 are Weftext profile extensions. Core exposes the real level through nine; outline, heading paths, anchors, annotations, search, Write, Read, and export must not flatten H6–H9. A generic processor may not recognize the extended levels, so compatibility export must warn or transform them explicitly.

## Run-in heading and body presentation

Weftext retains the existing adjacent-heading/body feature without turning a heading and paragraph into one semantic block. The preferred per-heading form uses native role syntax with Weftext-defined presentation semantics:

```adoc
[.run-in]
== Definition
Weftext is a local-first knowledge workspace.
```

Write and Read may present this as one visual paragraph, with the heading styled as the leading run. Core still returns a distinct Heading block and Paragraph block with independent ranges, anchors, outline behavior, links, annotations, search text, and accessibility semantics. Source remains exact.

The root-node portable default accepts `run_in` or `separate`; absence is equivalent to `separate`. The run-in form is:

```yaml
weftext:
  adjacent_heading_body: run_in
```

Resolution order is:

1. `[.run-in]` forces the eligible heading and its first paragraph to render together.
2. `[.separate]` forces separate presentation.
3. Without either role, the root workspace setting applies.
4. Under the workspace `run_in` default, only a paragraph on the immediately following physical line participates; a blank line keeps it separate.

An explicit `[.run-in]` targets the first following ordinary paragraph even when conventional AsciiDoc spacing includes a blank line. Removing the role restores the default behavior. Only body H1–H9 are eligible. The document title, subtitle, lists, quotations, tables, code, mathematics, diagrams, images, delimited blocks, and other non-paragraph blocks cannot be silently merged. A generic AsciiDoc processor that does not apply Weftext styling degrades to an ordinary role-bearing section and paragraph.

## Quotations

Semantic or attributed quotations use the native quote block:

```adoc
[quote, Ada Lovelace, Notes]
____
Quoted text.
____
```

For quick nested quotations, the profile accepts the Asciidoctor-compatible marker form:

```adoc
> First level
> > Second level
> > > Third level
```

Depths 1–9 receive complete editing and presentation support. Greater depths remain exact, depth-aware source and must not be flattened. The precise continuation, blank-line, attribution, and nesting rules require conformance fixtures before parser support; quote markers inside literal, listing, source, or other protected blocks are never promoted.

## Anchors, ordinary cross-references, and managed-node links

Weftext preserves native AsciiDoc anchors and cross-references:

```adoc
[#section-id]
== Section

xref:other-file.adoc#section-id[Display text]
```

`[[id]]` remains an accepted native anchor form. It is therefore not reused as a Weftext node-link trigger.

Stable managed-node links use a Weftext inline macro whose target is a UUID:

```adoc
node:550e8400-e29b-41d4-a716-446655440000[]
node:550e8400-e29b-41d4-a716-446655440000[文缕]
node:550e8400-e29b-41d4-a716-446655440000#section-id[Relevant section]
```

An empty display uses the resolved document Title and then the current node name. The selected alias or custom text is stored explicitly inside brackets so later alias changes do not rewrite authored labels. A future block embed uses the corresponding block macro on its own line:

```adoc
node::550e8400-e29b-41d4-a716-446655440000[]
```

Core owns UUID resolution, fragments, outgoing occurrences, backlinks, graph edges, non-disclosure, and transactional rewriting. `xref:` remains the ordinary AsciiDoc file/section mechanism; `node:` is the portable managed-identity mechanism. Link insertion may use a command, slash action, picker, or `node:` completion and is not required to imitate a `[[` interaction.

## Aliases and metadata ownership

AsciiDoc reference text can supply one display label for a particular reference, but it is not a portable list of Weftext node aliases. Aliases therefore remain Weftext operational YAML:

```yaml
weftext:
  aliases:
    - 文缕
    - Weftext Notes
```

Aliases are search and link-picker candidates, not identity and not necessarily unique. Choosing an alias inserts it as explicit node-link display text.

Use native document-header fields only where their meaning matches: document title/subtitle, author/email, revision, language, description, keywords, copyright, doctype, table of contents, section numbering, and block titles/captions. Simple custom note properties also use literal document-header attribute entries. Keep only Weftext operational data in the shallow `weftext` envelope: ID, aliases, node icon, child sorting policy, and sparse manual rank. Complex authored domain data uses typed Profile constructs. One semantic value has one authority; clients diagnose conflicting import inputs rather than silently choosing or synchronizing them.

Top-level bibliographic mappings are not canonical under this envelope. Structured reference facts require the typed Citation Data construct described as an open target gate in [`16-citations-and-bibliography.md`](16-citations-and-bibliography.md).

AsciiDoc's `icons` attribute controls document/admonition rendering and is never interpreted as the Weftext node icon.

## Node icon

The v1 node icon is one scalar under `weftext`:

```yaml
weftext:
  icon: weftext:project
```

It may be one literal emoji or a stable Weftext-owned built-in token. Arbitrary URLs, paths, nested icon recipes, raw colors, and processor-selected icon sets are not v1. Unknown tokens are preserved and produce no explicit icon; a missing/unsupported declaration uses the derived workspace-item default and never writes metadata merely by viewing a node. Icon rendering is decorative and the accessible node name remains present. AsciiDoc's `icons` attribute controls document/admonition rendering and is never interpreted as this node icon.

## Block titles, captions, code, mathematics, and diagrams

Weftext uses native AsciiDoc block titles:

```adoc
.Architecture
image::architecture.svg[Architecture]

.Measurements
|===
|Name |Value
|===

.Example
[source,rust]
----
fn main() {}
----
```

The dot title remains authored text. Numbering, cross-reference labels, list-of-figures/tables/listings presentation, and export are derived according to the document profile.

### Structured table semantics

Weftext table editing uses native AsciiDoc table cells, header-cell styles, and row/column span forms pinned by the profile conformance fixtures. It does not introduce a JSON table model, HTML fragments, or opaque editor metadata. Existing valid authored spellings remain exact unless they are inside the requested edit range; Core uses one documented canonical native spelling when a UI action must add or replace table structure.

The first contiguous `N` rows form the column-header region, and the first contiguous `N` columns form the row-header region. Both may be present, so the upper-left intersection carries both semantic roles in the Core model even when the native spelling needs one canonical projection. A discontinuous or non-leading header region is outside v1. Read and export expose real header associations rather than relying on bold styling alone.

A merged cell is one rectangular native row/column span. A merge may not cross a table boundary, cut an existing span, or conceal unsupported nested structure. If several selected cells contain content, Core composes every cell body in row-major order through a visible exact preview. Splitting recreates the represented grid, leaves the complete merged body in the leading cell, and leaves the other cells empty; exact recovery of earlier cell distribution is Undo, not Split. These rules prevent content loss without inventing persisted merge history.

If the pinned baseline cannot represent an accepted header or span edit losslessly, Core reports that capability unavailable and preserves source. Generic AsciiDoc processors may render a simpler presentation, but Weftext-authored source remains valid native AsciiDoc and degrades to readable table content rather than requiring Weftext-only preprocessing.

Mathematics uses the native AsciiDoc STEM surface rather than Markdown dollar delimiters. Weftext's preferred authored notation is explicit LaTeX:

```adoc
The result is latexmath:[x^2 + y^2].

.Energy
[latexmath]
++++
E = mc^2
++++
```

Alternatively, a document may set `:stem: latexmath` in its native header and then use `stem:[...]` and `[stem]` blocks. An unqualified `stem` without that attribute retains the native AsciiMath meaning and is never silently reinterpreted as LaTeX. Weftext renders a safe reviewed subset without executing TeX or reading arbitrary files. Unsupported expressions remain exact source with diagnostics.

Mermaid is a Weftext-rendered block extension that preserves the literal diagram source:

```adoc
.Process
[mermaid]
....
flowchart LR
  A --> B
....
```

The renderer is pinned, isolated, resource-bounded, sanitized, offline-capable, and paired with source/error/accessibility fallbacks. Supported diagram families, including swimlane behavior, are capability-versioned.

## Footnotes and endnotes

Footnotes use the native inline macro forms:

```adoc
Text footnote:[A local footnote.]
Text footnote:source-note[Defined once.]
Later footnote:source-note[]
```

Weftext adds the parallel endnote macro:

```adoc
Text endnote:[A document endnote.]
Text endnote:source-note[Defined once.]
Later endnote:source-note[]
```

An explicit endnote placement block is:

```adoc
endnotes::[]
```

Without that block, Weftext derives the endnote list at the end of the node/document without rewriting source. In continuous Desktop and Web reading there is no page boundary: a footnote reference opens an accessible nearby popover, sidenote, or narrow-screen sheet, while an endnote navigates to the endnote list. In paged PDF, print, or word-processing export, footnotes map to page-bottom notes where the target format supports them and endnotes map to document/chapter-end notes. The first profile version keeps separate derived numbering sequences and document scope. Rich block note bodies, cross-node notes, section-reset numbering, and detailed export fallbacks remain deferred.

## Checklists, task nodes, canonical Query, and Template slots

Ordinary checklists use the native unordered-list markers. Weftext-authored completed items use `[x]`; the native `[*]` spelling is also accepted and preserved:

```adoc
* [ ] Open item
* [x] Completed item
* [*] Also completed
```

The checklist marker is the portable open/closed authority. Weftext makes it interactive through a revision-checked narrow Core edit; the Asciidoctor `%interactive` option is a renderer hint and is not required in canonical source. Nested checklist items retain list hierarchy and do not imply task dependencies. A checklist remains identity-free and cannot carry typed dates, priority, dependency, recurrence, task-level annotation, or other durable fields.

A durable task is an ordinary managed node marked by literal document-header attributes:

```adoc
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
---
= Submit the paper
:weftext-task: v1
:weftext-task-state: in-progress
:weftext-task-scheduled: 2026-09-01
:weftext-task-due: 2026-09-05
:weftext-task-priority: high
```

The node UUID is task identity and is never duplicated into an attribute. Only literal `weftext-task` and `weftext-task-*` entries in the document header participate in the closed profile; body redefinitions are processor state. The profile supplies fixed state, priority, date/instant, and task-node dependency fields. Attribute references, substitutions, inline markup, natural-language dates, and executable expressions are invalid values rather than inputs to evaluate. Recurrence is deferred because task-node series and occurrence history are not yet accepted.

A native checklist is promoted only by the explicit recoverable workspace action defined in [`../architecture/18-task-nodes-and-checklist-promotion.md`](../architecture/18-task-nodes-and-checklist-promotion.md). The transaction creates the task node, lifts any deterministically convertible continuation/descendant content, and replaces the original checklist position by a stable `node:` link. It removes the checkbox rather than retaining a second state source. The complete field, query, import, and migration contract is [`17-tasks-and-query.md`](17-tasks-and-query.md). The superseded trailing `task:[...]` macro is accepted only as reviewed migration input and is never recognized as canonical output.

There is one Query facility. Tasks are one explicit source domain of the versioned canonical role-bearing literal block, not a second `tasks` fence or language:

```adoc
.Due soon
[.weftext-query,version=1,view=task-list]
....
from tasks as task
scope subtree(this.node)
where task.closed = false
  and task.due is not null
  and task.due <= context.today + P14D
select task.id, task.title, task.state, task.due
order by task.due asc nulls last
limit 100
....
```

The complete `weftext.expr.v1`, canonical Query grammar, lexical `this`, explicit `context`, headings domain, and tagged checklist/task-node projection are frozen in [`18-canonical-query-and-expression.md`](18-canonical-query-and-expression.md) and [`17-tasks-and-query.md`](17-tasks-and-query.md). Product clients may consume the shared Core evaluator but cannot evaluate blocks independently. Query results are derived views. Toggling or editing a row invokes the exact checklist-source or task-node action selected by its typed row kind; no task database or materialized result becomes portable authority. JavaScript, shell commands, network access, raw filesystem reads, ambient time, and client-private evaluators are prohibited effects. Superseded Query grammar is private one-time migration input only and has no runtime parser alias.

A configured Template Root/Part subtree may contain `slot:title[]` inline or `slot::body[]` block macros declared by the Template Root's fixed profile-`weftext.node-template.v1`/version-`1` `weftext.template.json`; each declaration is scoped by the target Root/Part permanent Node UUID. Only that valid role/profile gives those macros slot semantics. Independently authored identical source in an ordinary managed document is inert and preserved. A validated prototype cannot be moved/restored into ordinary space and silently downgraded: explicit conversion must materialize/delete active slots and remove profile/sidecar or block. V1 has no generic raw slot, conditional, loop, or Query execution; generated ordinary nodes must contain no residual slot/profile/sidecar. The complete contract is [`19-node-template-library.md`](19-node-template-library.md).

## Includes, processors, and active content

No UI shell or third-party AsciiDoc processor receives unrestricted workspace, filesystem, network, environment-variable, URI, or command access. Includes, conditionals, attributes that resolve paths, extensions, diagram processors, and passthrough content are capabilities controlled by Core and a reviewed renderer boundary. Unsupported active behavior remains source text or a visible diagnostic; it never executes merely because a document is opened.

A future include feature must use validated workspace-relative locators, content-boundary and permission checks, cycle/depth/size limits, deterministic dependency revisions, and non-disclosure rules. Generic safe mode alone is not accepted as the complete sandbox.

## Exact source, editing, and derived state

Core preserves exact UTF-8 bytes, YAML formatting, line endings, protected blocks, and every range used for actions. Write and Read are projections; Source is not regenerated from a normalized AST. All patches remain revision-checked and narrow. Search, outline, links, backlinks, graph, captions, note numbering, rendered diagrams, and indexes are rebuildable derived state.

Core exposes one `DocumentFormatPlan` boundary for canonical AsciiDoc inline, heading, paragraph/list/quote/code, table, ordinary link, and image-resource reference commands. Inputs and returned selections are UTF-8 byte offsets. Protected ranges, invalid boundaries, missing semantic blocks, and malformed tables fail closed. The shared React shell maps browser string positions to this contract and never selects syntax from file extensions. Markdown behavior is confined to explicit importer/exporter boundaries, not a runtime peer profile.

`weftext.annotations.json` stores every portable review message as constrained exact `weftext.asciidoc.inline.v1` source. Imported bodies and anchors are converted only through an explicit preview with unique deterministic target mapping. Device drafts bind to the AsciiDoc profile and revision; an incompatible draft is recovery evidence and is never auto-applied.

Parser selection must prove exact source maps, malformed-input behavior, extension hooks controlled by Core, Windows/macOS/Linux packaging, performance, licensing, and the accepted Rust minimum version. A processor that only produces HTML is not sufficient storage authority.

## Deferred syntax

The following remain separate decisions and are not silently invented by clients:

- hosted/non-Owner task callers, canonical Query product views, editable derived views, and Saved Query storage beyond the contracts in [`17-tasks-and-query.md`](17-tasks-and-query.md) and [`18-canonical-query-and-expression.md`](18-canonical-query-and-expression.md);
- Template role inventory, fixed-sidecar parser, engine, Designer, instantiation, and role-transition transactions defined by [`19-node-template-library.md`](19-node-template-library.md);
- glossary/index terms and publication-level cross-document numbering;
- rich multi-block footnotes/endnotes;
- component/transclusion parameters beyond the UUID block embed;
- unrestricted AsciiDoc extensions, arbitrary Ruby processors, remote includes, and executable content;
- compatibility transforms beyond accepted explicit Markdown import/export mappings and diagnostics.

## Acceptance boundary

Engineering acceptance requires identical Core models across Desktop, CLI, Server, and WebUI; exact-source and malformed-source fixtures; index rebuild; shared canonical editor behavior; CJK/UTF-8/mixed-line-ending coverage; annotation/draft coordination; and proof that no processor bypasses Core transactions or Server authorization. Release acceptance additionally requires generic Asciidoctor degradation checks, Markdown import/export compatibility reports, signed packaged lifecycle and restore drills, physical accessibility/IME coverage, and the supported GUI platform matrix.
