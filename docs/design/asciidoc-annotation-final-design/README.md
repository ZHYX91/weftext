---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# AsciiDoc / Annotation final design candidate

Status: **candidate-design-not-implemented**.

This directory turns the independently reviewed complete AsciiDoc design, Weftext extensions, independent portable JSON Annotation design, and their FC-3.4-through-d coordination against fixed-e8aa D2-D10 owners into one public, independently reviewable candidate.

- parent commit: `e8aa0b341630a57c786c0891d4bbd1620247441d`
- parent PR branch: `fix/pl-ir-01-portable-locator-requalification`
- immutable accepted input S: `7e18168dad3e6d120fce0dd607dc10fa7894e252`
- S49 snapshots and `docs/design/inputs.json`: **unchanged**
- design only; not implementation, release, A2, or global acceptance.

## Reading order

1. [REVIEW-ENTRY.md](REVIEW-ENTRY.md)
2. [SPEC.md](SPEC.md) — behavior, algorithms, and owner replacements
3. [SCHEMAS.md](SCHEMAS.md) — current closed shapes, ordering, and historical dispatch
4. [ACCEPTANCE.md](ACCEPTANCE.md) — 731 explicit unexecuted design obligations
5. [terminology-registry.json](terminology-registry.json)
6. [replacements.json](replacements.json)

`replacements.json` binds fixed-e8aa actual-owner blobs and normative concerns to replacement sections in SPEC. Unlisted fixed-parent semantics remain; listed conflicting contracts are replaced rather than patched by private-chat appendices.

This is one joint PR because managed document format, PAB4, EffectManifest3, D10 recovery, and BootstrapPlan4 share one closed successor family; splitting would require a temporary same-version union expansion.

## Acceptance accounting

- core design oracles: 438
- actual-owner coordination fixtures: 293
- total: 731
- status: **all unexecuted**.

The author runs only document/JSON/router consistency checks. Acceptance requires a non-author review bound to the exact stopped PR head SHA.
