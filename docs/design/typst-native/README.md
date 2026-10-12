---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Typst primary-format coordinated candidate

Status: **design revision and independent review in progress; unimplemented and not globally accepted**. Fixed input: `af6737a057c7bb69763dfaad0a57da4652e085f3`; author branch: `design/typst-native-20261012`. [REVIEW](REVIEW.md) records current review and validation.

New Weftext notes use Typst as their sole primary format, reusing the official Rust parser/compiler, native syntax and standard library. Ordinary writing, flowing reading, rich editing, structured properties, Query, links, independent annotations, templates and intranet use remain requirements. Prior implementation or review investment does not constrain the choice, nor does the choice reduce the product to source plus PDF.

## 1. Reading order

| Document | Responsibility |
| --- | --- |
| [SOURCE](SOURCE.md) | Native metadata, identity/type declarations, simplified fields and schema |
| [CORE](CORE.md) | `.typ` storage, versions/environment, authorization, synchronization, transactions and recovery |
| [EDITOR](EDITOR.md) | Flowing reading, rich/source editing, ordinary tables, maps, annotations and accessibility |
| [FEATURES](FEATURES.md) | Link forms, potential suggestions, Query/tasks/views, templates and output |
| [INTEGRATION](INTEGRATION.md) | Ownership, retention/replacement rules and cross-module acceptance |
| [REVIEW](REVIEW.md) | Original-21 impact, fixed-candidate reviews, actual validation and residuals |

[replacements.json](replacements.json) provides machine navigation; each owner's named dispositions provide the semantic basis. The predecessor [A2 revision](../a2-final-design/REVISION.md) and fixed inputs remain; only explicitly referenced generic obligations enter this candidate. Do not combine the two generations' formats, fields or parsing oracles, or transfer prior acceptance.

## 2. Data inside notes

Ordinary notes, contacts and tasks share one source. Schema defines field types and validation; authors write native values. Repeated contacts use arrays of objects, not a JSON fact with identity and type wrappers for each value.

```typst
#metadata((
  id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
  types: ("person",),
  name: "Zhang San",
  company: "Cloud Mountain Company",
  phones: (
    (number: "+86 13800138000", label: "Mobile", note: "Contact on weekdays"),
    (number: "010-12345678", label: "Work"),
  ),
)) <weftext>

#title[Zhang San]

= Contact notes
Contact on weekdays. Write other details in the body.
```

This is standard Typst `metadata`, dictionary, array, label and body syntax. The meanings of `id`, `types` and business keys are application semantics, not another grammar. SOURCE defines complete root qualification, placement, duplication/shadowing, repair and schema rules. A declared UUID still proves neither birth nor authorization nor a current version.

People/Organizations retain ordinary contact fields, multiple phones/emails and optional notes; per-value fact UUIDs, elaborate qualifiers, confidence and automatic inverse-relation machinery leave default scope. Task details live in the body, properties use native values, and table/board/calendar views share one task. New notes require no YAML front matter, embedded JSON attribute blocks or private backlink notation. Independent annotations, configuration and internal protocols may still use JSON.

## 3. Native-first boundaries

| Requirement | Native mechanism | Application responsibility |
| --- | --- | --- |
| Title, levels 1–9 and deeper legal headings, run-in headings | `title`, `heading`, standard markup and show rules | Outline, shared-paragraph interaction and accurate accessible levels |
| Formatting, lists, tables and math | Standard elements and math | Controls, narrow writes and source access for complex content |
| Business data | Native values, dictionaries/arrays and accurately applicable standard constructors | Schema, missing/error states, origins and Field permissions |
| Links | `link`, label and ref | ID/path/Workspace addresses, same-target conversion, incoming/outgoing links and optional suggestions |
| Query/multiple views | Native data carriers | Workspace-wide evaluation, ACL, completeness, pagination and real Actions |
| Templates/output | Functions/import, set/show and native compilation | Authorized inputs, conversion, downloads and loss confirmation |
| Independent annotations | Separate sidefiles and native inline profile | Versions, ranges, target reads and write qualification |

Use the official engine, without another Typst parser or interpreter. Separate valid complex syntax from rich-edit qualification. Macros/generated content without a safe reverse mapping expose their origins; equal displayed text never invents a write location. Typst's document-local `query` does not replace Workspace-wide Query.

EDITOR defines a concrete flowing-reading backend and delivery matrix rather than waiting for upstream HTML. Official HTML remains experimental, and ordinary notes must not become PDF-only. Named providers retain scoped LaTeX/Mermaid compatibility with loss handling; actual DOCX/XLSX output uses conversion capabilities, never a paginated table relabeled as a workbook.

## 4. Evidence and completion boundaries

These files are target design, not a migration of the early product prototype to Typst. REVIEW owns actual status: documentation checks establish mechanical navigation/bilingual conditions; official example compilation establishes the shown syntax; research probes establish bounded behavior. None establishes product editing, authorization, crash recovery, synchronization or accessibility acceptance.

The 49 fixed-S snapshots and `inputs.json` retain their exact bytes. Reassess the original 21 substantive obligations individually; their prior verdicts retain their original fixed objects. Authors challenge each other, followed by non-author review of a fixed candidate. Full Pro review, implementation and release do not start automatically.

## 5. Official references

- [Official Rust implementation](https://github.com/typst/typst) and [World interface](https://docs.rs/typst/0.15.1/typst/trait.World.html)
- [Metadata](https://typst.app/docs/reference/introspection/metadata/), [Dictionary](https://typst.app/docs/reference/foundations/dictionary/), [Array](https://typst.app/docs/reference/foundations/array/)
- [Title](https://typst.app/docs/reference/model/title/), [Heading](https://typst.app/docs/reference/model/heading/), [Link](https://typst.app/docs/reference/model/link/)
- [Current HTML boundaries](https://typst.app/docs/reference/html/) and [document Query](https://typst.app/docs/reference/introspection/query/)
