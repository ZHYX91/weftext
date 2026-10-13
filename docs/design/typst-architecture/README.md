---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# New Typst architecture design

Status: **accepted-design-not-implemented**. Independent global final review accepted technical baseline `f9512f05f86311ea685d5e8d213784ac0d416324` on 2026-10-13; [REVIEW](REVIEW.md) §7 records the disposition and evidence limits. The coordinated rewrite began from `d167ae8c1d0346faf62e8e0887cf789de0bf0b89`. [REQUIREMENTS](../typst-native/REQUIREMENTS.md) owns requirements, including structured capabilities for all types, navigation, library styles, five views and Chinese defaults.

This directory replaces the prior `typst-native` technical solution as the current accepted technical design. Prior files and their fixed-scope reviews remain historical evidence, not algorithms to combine with this directory. The 49 inputs and `inputs.json` remain unchanged. Retain user outcomes while choosing technology anew, without normal-runtime compatibility layers for undeployed protocol families.

| Document | Responsibility |
| --- | --- |
| [SOURCE](SOURCE.md) | Native author source, identity/type declarations, one type/value model, schemas, structured note types and safe writeback |
| [CORE](CORE.md) | File/structure authority, identity/revisions, save/recovery, external editing, synchronization, authorization and controlled evaluation |
| [EDITOR](EDITOR.md) | One draft, flowing reading, rich editing, origin maps, titles/subtitles/run-in headings and Word semantics |
| [FEATURES](FEATURES.md) | Links, Query/navigation/views, tasks, annotations, templates, conversion, downloads and recoverable external jobs |
| [INTEGRATION](INTEGRATION.md) | Shared contracts, requirement and prior-finding coverage, implementation order and refactoring entry criteria |
| [REVIEW](REVIEW.md) | Actual experiments, independent findings, unresolved issues and completion boundaries |

Source uses standard Typst; schemas and explicit role conventions supply application meaning. Common interfaces stay simple without deleting necessary nesting, multiple values, relations or history because of old AsciiDoc attribute limits. Compiling data alone does not establish forms, Query, authorization or export; those consumers require coordinated validation.

This round does not change product code or real materials. Independent design acceptance is complete; subsequent refactoring follows vertical capability slices and runs each slice's risk-appropriate implementation acceptance. Design review, limited prototypes, product acceptance and release are distinct states.
