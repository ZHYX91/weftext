---
source_language: zh-CN
translation_of: DOCUMENTATION.zh-CN.md
translation_status: synced
---

[简体中文](DOCUMENTATION.zh-CN.md)

# Public documentation policy

## Language layout

Human-facing public Markdown uses paired files. The Chinese source is named `name.zh-CN.md`; the synchronized English translation keeps the stable `name.md` path. Both files contain matching heading levels and a language-switch link. One change updates both languages.

The GNU AGPL text in root `LICENSE` remains verbatim and authoritative; `LICENSE.zh-CN.md` is only a non-authoritative reading guide. Test fixtures, generated evidence, lockfiles, machine-readable examples, and upstream license texts are not translated.

## Public content

Public documents describe current contracts, current implemented boundaries, and remaining release gates. They do not retain development-stage numbering, completed-task narratives, old decision chronology, internal schedules, handoffs, research comparisons, acceptance ledgers, or competitor references. Git records document history; the private control workspace holds planning and research.

Compatibility and migration text states only the active boundary needed to read or convert existing data. Retired syntax is not presented as another supported product language.

Public documentation uses neutral, verifiable language. It contains no promotional slogans, comparison-style superiority claims, unsupported maturity claims, or descriptions of target behavior as a product achievement. Security, integrity, and performance statements must be testable constraints, current evidence, or clearly labeled unmet gates rather than self-assessment.

## Reader and contract layers

`docs/guides` explains user concepts, common tasks, and capability status in plain language. It must distinguish current foundations, accepted designs still being implemented, and pre-release limitations. A guide never presents a planned capability as delivered.

`docs/specifications` and `docs/architecture` hold implementation contracts and architecture authority. They may use strict constraint language but still follow the [terminology guide](TERMINOLOGY.md), avoiding machine-translated phrasing, undefined abbreviations, and unnecessarily long paragraphs. If a guide conflicts with a newer specification, the specification wins and the guide is synchronized in the same change.

## Synchronization gate

Run the documentation check after editing public documents:

```text
python scripts/check_docs.py
```

The check validates pairs, frontmatter, language links, heading-level parity, relative links, localized cross-links, forbidden Chinese translations, forbidden historical labels, and excluded product-comparison names. The source gate runs the same command. Automated checks do not replace paragraph-level semantic editing; a contract change still requires human comparison of both languages.
