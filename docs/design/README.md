---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# New Typst architecture design

Status: **design-refinement-not-implemented**. The refinement keeps the main architecture, clarifies requirements/CEL/copy contracts and simplifies builtin configuration. [REVIEW](REVIEW.md) §§7–8 separately record earlier fixed acceptance and current checks.

The 2026-10-20 hierarchical-tag addition specifies REQUIREMENTS §5.1.1: CORE §2.3 owns forests/identity/upgrades, FEATURES §§5.6,7.4 own trusted queries/interaction, with SOURCE/SERVER/INTEGRATION aligned. It requires necessary shared implementation refactoring and is not accepted alpha.3 behavior. ROADMAP TAG01–10 contains its completion gates.

This directory replaces the prior `typst-native` technical solution as the current technical design. Prior files and their fixed-scope reviews remain historical evidence, not algorithms to combine with this directory. The 49 historical inputs, `inputs.json` and full review history are archived, not build or runtime dependencies of this repository. Retain user outcomes while choosing technology anew, without normal-runtime compatibility layers for undeployed protocol families.

| Document | Responsibility |
| --- | --- |
| [SOURCE](SOURCE.md) | Native author source, identity/type declarations, one type/value model, schemas, structured note types and safe writeback |
| [CORE](CORE.md) | File/structure authority, identity/revisions, save/recovery, external editing, synchronization, authorization and controlled evaluation |
| [EDITOR](EDITOR.md) | One draft, flowing reading, rich editing, origin maps, titles/subtitles/run-in headings and Word semantics |
| [FEATURES](FEATURES.md) | Links, Query/navigation/views, tasks, annotations, templates, conversion, downloads and recoverable external jobs |
| [INTEGRATION](INTEGRATION.md) | Shared contracts, requirement and prior-finding coverage, implementation order and refactoring entry criteria |
| [REVIEW](REVIEW.md) | Actual experiments, independent findings, unresolved issues and completion boundaries |

Source uses standard Typst; schemas and explicit role conventions supply application meaning. Common interfaces stay simple without deleting necessary nesting, multiple values, relations or history because of old AsciiDoc attribute limits. Compiling data alone does not establish forms, Query, authorization or export; those consumers require coordinated validation.

This round refines documents and bounded prototypes without changing real materials. Earlier fixed acceptance and current refinements are recorded separately. Refactoring follows vertical capability slices, each with risk-appropriate implementation acceptance. Design review, limited prototypes, product acceptance and release are distinct states.
