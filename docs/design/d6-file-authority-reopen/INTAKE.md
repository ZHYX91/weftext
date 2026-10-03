---
source_language: zh-CN
translation_of: INTAKE.zh-CN.md
translation_status: synced
---

[简体中文](INTAKE.zh-CN.md)

# D6-FA-r01 Product Intake

Status: candidate intake only. It records confirmed requirements and candidate research inputs; it activates no feature and is not an implementation audit.

## 1. D8 Source / Live / Read input

Confirmed editor-surface requirements:

- Source edits exact source.
- Live is a Core-mapped editable representation of the current Draft.
- Read is read-only and may bind an explicitly labeled Draft preview or committed revision.
- Live source-marker visibility is a closed preference: `always_show | contextual | always_hide`; the default is `contextual`.
- Marker visibility is presentation state. Switching it never changes source, commits author state, or terminates/confirms IME composition.
- Source↔Live↔Read switching preserves one Draft, Selection/anchor/focus/affinity, Undo/Redo input log, and IME continuity. When mapping cannot be proved, the UI uses Source or an explicit readonly/atomic occurrence rather than guessing DOM offsets.
- An external file change invalidates stale maps/previews/cuts while retaining the current Draft as a local proposal. Rebinding uses the new SourceVersion and a three-way proposal; identical rendered text does not silently rebase.
- A complete D8 afterimage will be authored later. This D6 batch freezes only the file-version, reliable-save/conflict, and Server-collaboration inputs D8 will consume.

## 2. Plugin candidate intake

The following rows record candidate semantic directions from the supplied fixed README references only. **This batch did not read the seven plugin source trees, release artifacts, or research attachments, and did not perform implementation audits.** A README is not evidence that a Weftext capability is implemented.

| Candidate | Intake topic | Expected owner | Fixed README |
|---|---|---|---|
| Property Order | repeated property-value ordering and candidate ordering | D4/D5/D8 | https://github.com/ZHYX91/%6Fbsidian-property-order/blob/137668dfc239bd7e0757032bf84093fab09ca6aa/README.md |
| Folder Nodes | document/child navigation, resource views, manual order | D3/D5/D8 | https://github.com/ZHYX91/%6Fbsidian-folder-nodes/blob/27f6e0b6f24641eab0d23d2f1f255b133482213e/README.md |
| Chrono Notes | period/date navigation and calendar/task projections | D4/D7/D8 | https://github.com/ZHYX91/%6Fbsidian-chrono-notes/blob/0116aaa6397aa2b32bc57889b951c25114a3bbad/README.md |
| Link Integrity | reference diagnostics/reliability | D3/D7/D8 | https://github.com/ZHYX91/%6Fbsidian-link-integrity/blob/902b881585c346e599656c3d5629017165c5da9c/README.md |
| Number Suite | numbering/captions/footnotes/cross-references/outline/export | D2/D8/D9 | https://github.com/ZHYX91/%6Fbsidian-number-suite/blob/5ecdcf88040b5b5caaf7b2759410cf2d115fcfd8/README.md |
| Structural Tables | complex tables and row-to-Node-set promotion | D2/D5/D8/D9 | https://github.com/ZHYX91/%6Fbsidian-structural-tables/blob/2cd5a1c05a22bce801fa59a56295ee97e68b66e7/README.md |
| Assistant Workflow | snapshot/capability matching/proofing/progress/recovery workflow | D8/D9/D10 | https://github.com/ZHYX91/%6Fbsidian-%64ocwen-assistant/blob/65481ec5771fc44ad87a5433b10804a555e50590/README.md |

## 3. Items not approved by this intake

This intake does not approve new Weftext source syntax, durable fragment identity for headings/table rows/property occurrences, per-keystroke author commits, direct plugin writes around Core, external runtime/provider authority, support-matrix expansion inferred from a README, or import of private implementation history/market rankings.

These are not permanent prohibitions. Future product need must be handled by the actual owner with versioned semantics, complete permission/transaction/failure contracts, and independent review.

## 4. Direct D6-FA-r01 inputs

1. External file changes invalidate stale SourceVersion/map/preview state while preserving Draft/conflict branches.
2. Manual order cannot exist only in the discardable index; current parent/order must have portable logical ownership.
3. Snapshot/progress/recovery UI must not present a building index, prepared plan, preview, worker success, HTTP success, or sync-provider status as an author commit.
4. Large-workspace opening should prioritize the active document and ordinary reliable save while metadata/search/OCR progress independently; complete Actions still require complete ranges.
5. Server multi-user mode allows concurrent sessions. Back-end commit serialization does not mean a single front-end editor.

All other plugin semantics remain pending for their real owner afterimages.
