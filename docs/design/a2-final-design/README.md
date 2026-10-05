---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)
# A2 Final Design Integration Candidate

Status: candidate design, not independently accepted, not implemented, not released.

## 1. Fixed object and authorship

This A2 integration is based on parent commit 97f4734f82a760cb6716c8122b84494da2b61164. Fixed historical input S is 7e18168dad3e6d120fce0dd607dc10fa7894e252. The protected input inventory is docs/design/inputs.json at blob 787d03c31a55496f81ed03fd54a6fdfff50a2ad4.

This directory is the only A2 candidate authority for modules integrated here. Earlier snapshots, D6 file-authority owner afterimages, and the AsciiDoc/Annotation final-design candidate remain source provenance and historical decoder evidence; they do not form a second current A2 definition.

The current author batch integrates D1 and D2 only. It is authored work, not independent acceptance.

## 2. Batch progress

| Module | A2 status in this batch |
| --- | --- |
| D1 | integrated current candidate |
| D2 | integrated current candidate |
| D3 | TODO; source/navigation known, not integrated |
| D4 | TODO; source/navigation known, not integrated |
| D5 | TODO; source/navigation known, not integrated |
| D6 | TODO; only D1/D2 intersecting current sections read |
| D7 | TODO; only D2 intersecting current overlay/consumer sections read |
| D8 | TODO; only D1/D2 intersecting current sections read |
| D9 | TODO; only D2 intersecting current overlay/consumer sections read |
| D10 | TODO; TASK/navigation read, final module not integrated |
| Mandatory A2 source | TODO except D1/D2 intersections already named |

No TODO module is treated as accepted, complete, or semantically read merely because its path, route, blob, or title is known.

## 3. Current files

- D1.md is the self-contained current D1 candidate for this batch.
- D2.md is the self-contained current D2 candidate for this batch.
- SOURCE-MAP.md gives the human-readable source/disposition map.
- SOURCE-MAP.json records all 49 fixed-S inputs, exact S blobs, read status, current sources, and source-qualified obligation groups.

## 4. Precedence inside this candidate

For D1 and D2, the A2 files in this directory are the current candidate text. A named historical decoder or historical scenario remains normative only for historical recovery or source-qualified evidence when D1.md, D2.md, or SOURCE-MAP explicitly says so.

For D3 through D10 there is no A2 current definition yet. Their existing owners remain inputs for later batches; this directory must not be read as silently replacing them.

## 5. Acceptance boundary

The D1/D2 integration preserves source-qualified historical obligations and current limited-review evidence, but this author does not independently close A2 findings. All design/runtime acceptance cases remain unexecuted unless an exact external run is explicitly recorded. Final A2 acceptance still requires a fresh non-author global review of one fixed completed A2 commit.
