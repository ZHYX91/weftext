---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# New Typst architecture design

This directory maintains the current technical owners and evolving contracts. [REVIEW](REVIEW.md) §7 records independent design acceptance of the fixed earlier baseline on 2026-10-13; §8 records that period's bounded requirement/CEL/copy-contract and builtin-configuration refinements. Their unimplemented/unrun states are historical snapshots. [MVP](../MVP.md) and [ROADMAP](../ROADMAP.md) define current implementation, actual support and unverified scope. Earlier design acceptance does not replace current product acceptance, and an earlier stage's status does not describe later implementation.

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

The 2026-10-13 refinement round changed documents and bounded prototypes without changing real materials; its evidence and earlier baseline acceptance remain separate. Subsequent product implementation/refactoring follows vertical capability slices, each verifying its actual impact and consumers. MVP/ROADMAP and the corresponding candidate records retain current results. Design review, limited prototypes, product acceptance and release are distinct states; this directory's overview does not establish whole-version or whole-architecture completion.
