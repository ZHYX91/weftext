---
source_language: zh-CN
translation_of: MVP.zh-CN.md
translation_status: synced
---

[简体中文](MVP.zh-CN.md)

# Local writing and knowledge tool

The current version is **0.5.0-alpha.3, a Windows local candidate**, adding Office output and interchange to the bounded v0.2–v0.4 features. Full milestone acceptance still has the limitations below; this is neither the complete product nor a published release. Previous candidates remain separate. See the [roadmap](ROADMAP.md). Native `.typ` files remain the authoring source; editors, search and views do not store a competing body.

This candidate connects the **local note timeline** and **five-view body-embedded Query**, reusing Core save/recovery, shared source/Draft/Undo and the existing Query executor. The [roadmap](ROADMAP.md) records bounded local scope and outstanding platform conditions; implementation does not by itself establish all acceptance conditions.

## Launch and files

Run the packaged `weftext.exe`. Its window reports a local session address and opens the default browser; closing the process stops the service. It listens only on `127.0.0.1`, without a cloud account. This local browser interface does not establish multiuser Server/WebUI availability.

```text
weftext.exe --open "D:\Notes\example.typ"
weftext.exe --open "D:\Project\chapter.typ" --root "D:\Project"
weftext.exe --no-browser --data-dir "D:\Weftext-Local-State"
```

The UI accepts full local paths. New libraries use an absent directory; ordinary directories can be browsed without automatic admission. New notes use `A/A.typ` with native metadata for ID, types and fields. Ordinary files inside or outside a library can be opened, edited and saved without adding metadata. Titleless notes are valid; navigation may display a filename fallback.

Explicit directory admission supplies stable navigation identity without creating a note body. Directory shortcuts and tags resolve actual identities. Equal names do not merge identities, and missing or invalid targets never silently become another same-name object.

## Features

| Scope | Candidate implementation |
| --- | --- |
| Writing | V01 complete source, V02 styled source, V03 local syntax, V04 text/structure controls and V05 reading share a draft and Undo. Titles/subtitles, run-in headings, lists, tables, equations, code and inline formatting; native library styles and note overrides; continuous preview and actual paged PDF. |
| Structured information | Central schema, native fields, repeated records, exact values and unknown-member preservation; people, organizations, literature and tasks; quick capture; actual CEL Query, saved definitions, list/table/board/calendar/summary views; period notes. |
| Knowledge organization | Actual links, property references and overlapping potential links; address conversion; source/media annotations; template preview/create; resource import, copy, rename, move, Trash/restore and related-material backup. |

Only sources proved by official Typst AST and current compilation can be rewritten. Complex cross-construct selections, nested multiline list adjustments, advanced table topology and generated content retain source with a source-editing entry. Frames that flow cannot represent explicitly require paged output. Styles use standard modules and set/show; changing library styles does not rewrite note bodies. Offline Chinese, code and math fonts are served from the same origin; exact files/licenses are in `resources/fonts`. Failures are reported without relying on a public font CDN.

The sidebar filters filenames/titles; body/property search use Query. Query consumes Core-captured admitted-note snapshots, with unmanaged raw-directory items separately identified. Partial, stale, invalid and complete-empty outcomes differ. Advanced operators, current/pinned definitions and native compiled products require actual frozen inputs rather than unbounded latest-file reads. Period navigation resolves existing objects before explicit creation. Finite calendar/zone tables report unavailable outside coverage; bounded recurrence expansion never treats incomplete output as proof of no occurrences.

Potential-link hints can be disabled. Multiple targets, nested and crossing spans remain available. At most two underline lanes are displayed; all other candidates stay accessible in the full list. Hints do not edit source; conversion verifies exact targets and current source evidence. Independent JSON stores annotation discussion and targets. Staleness preserves discussion, and similar text does not automatically reanchor. Accepting a suggestion commits source and annotation state through one Core operation.

Images, PDF, WAV and supported MP4 use actual dimensions/timebases. Complex PDF structures, rotation/cropping and unsupported video timelines explicitly reject region writes. OCR, arbitrary media formats, a complete IANA database and all named lunar-calendar packs are not delivered promises of this candidate.

Templates and multi-file operations show actual scope before committing a frozen plan. Copy allocates actual new identities, handling registered resources and schema-specified references. Unprovable content is rejected or reported as a loss. Unknown outcomes inspect the original operation without blindly allocating another ID. Occupied restore destinations are never overwritten.

## Office output and interchange

Body-embedded Query appears at its actual position in continuous view. V01/V02 expose source, V03 expands it explicitly, V04 offers a builder or explicit source proposal, and V05 is read-only. Edits share Draft/Undo; view changes do not autosave. Choose DOCX or PDF in Output and Import to include the exact current Draft's displayed frozen Query results at their body positions; DOCX templates must bind a body slot. Original rows, order and partial/stale/empty-scope states are retained without rerunning queries or saving. Inspect original cuts, coverage and diagnostics before confirmation. Missing current results report omissions instead of borrowing another Draft's results. Native paged preview still excludes Query; use the separate PDF output entry. Complex, generated or ambiguous occurrences, and PDF insertion with unproved custom bindings, remain explicitly bounded.

With a note open, choose exact Typst source, DOCX, PDF, XLSX, a native package or bibliography format in Output and Import. Generate and inspect actual inputs, artifact information and losses, then explicitly confirm and download. Download checks the same bytes' length and SHA-256. Export does not save the current draft on the user's behalf. Local artifacts last one hour; expiration requires explicit regeneration. Closing the program requires regeneration without promising cross-process output-job recovery. Limits are 32 outputs and 32 Query results, 256 MiB total cache, and 64 MiB per file/frozen dependency set.

DOCX converts one frozen Typst HTML compile to editable paragraphs, runs, headings, lists, tables, images and supported equations, reporting unmappable elements. Uploaded DOCX templates bind existing content-control tags to title, body, properties or a selected Query. Template styles with retained emphasis are the default; representable source styles are optional. Font dependencies, complex numbering, references and equations remain subject to concrete losses and actual target-application verification. Arbitrary Typst programs are not guaranteed equivalent to Word.

A run-in heading defaults to a whole Heading paragraph; a normal TOC may include its following body. When body text precedes a numbered run-in heading, Word native numbering remains at the paragraph start. Body text and character formatting are retained with `run-in-leading-body-loss`; equivalent number placement is not claimed.

XLSX exports current-note fields or the exact result selected through a Query result button, without silently rerunning after file changes. Named-range/table templates have explicit capacity and do not automatically expand or rewrite formula references. Ordinary strings never execute as formulas. Numbers beyond Excel precision and complex temporal/array/record values use exact text, with a companion sheet retaining full typed Cells, schema and row/column order. Null, absent, invalid, partial and stale results remain distinguishable.

Native packages export the **saved entire library** and admitted dependencies; save open library drafts first. They exclude local history, journals and unmanaged raw files and do not replace recovery backups. Import creates fresh identities in a new destination folder, rewrites proved references and retains the original upload. Incompatible schemas are rejected, library settings are not replaced, and context changes in complex resource modules are reported. Existing notes are not overwritten.

BibTeX, CSL JSON and RIS import first shows complete mappings, unknown data and conflicts. Conflict-free input creates independent paper notes and retains true originals. Equal DOI, title or suggested citation key never authorizes overwriting or merging; the current UI does not automatically refresh or merge existing Works. Export uses typed publication data from a paper or explicitly bound Works. Citation declarations generate a native `.bib` from the same frozen input; an authored file at that path prevents replacement. Unknown members and non-round-trippable data remain in originals with explanations rather than claiming every format is lossless.

## Saving and recovery

The Timeline button for admitted notes lists saves actually retained by Core, with recorded save time, unknown time or missing exact source. Read old source without editing, comparing with current saved content or an explicitly selected Draft snapshot. Restoration first confirms Draft protection and creates a new save, keeping later history. An original dirty buffer or input made during the request remains in the editor and recovery records. Historical preview currently means exact source/text diff, without reconstructing old fonts, modules or configuration for rich rendering.

Private state defaults to `%LOCALAPPDATA%\Weftext-MVP`, including drafts, operation before/after images and actual identity records. It is not disposable cache. One process owns a data directory; do not use different data directories concurrently on the same tree. Upgrades retain the original data directory; matching IDs in files cannot recreate admission evidence.

Saving checks actual file objects and original bytes, rejecting old drafts that would overwrite external changes. Save As creates only absent targets and verifies the reopened result, preserving input typed meanwhile in the same draft and Undo. Sessions/drafts follow proved objects across rename/move/Trash. Competing retained drafts require comparison instead of silent selection.

Recovery records show every participant's before, planned and current content; binary versions can be downloaded for comparison. A coherent before-state can cancel the original plan, and a proved complete after-state can adopt it. Proven partial content updates can continue the original operation's remaining files. Third-state bytes or mismatched identities are not overwritten. Intermediate directory-rename placement remains available for inspection without automatic replay. Prior drafts and confirmed merge proposals remain in private recovery history.

A complete single-library backup includes ordinary files, ignored bytes, related control evidence and consistently captured related journal/drafts. It excludes private material from other libraries or outside files. Retained recovery evidence does not grant a new device the original local NTFS identity and does not establish synchronization.

## Boundaries and checks

The local writer profile remains `weftext.local-mvp.v1`, supporting Windows/NTFS. It does not promise network shares, cloud drives, other file systems, cross-device admission, sync or multiuser operation. Symlinks, junctions, hard links and unprovable paths do not enter writes.

Author source is limited to 2 MiB; raw scans to 5,000 entries, with explicit incomplete outcomes at budgets. Compiler subprocesses have a 20-second timeout, 1 GiB Windows memory limit and 24 MiB output limit; frozen Query compile inputs are limited to 64 MiB. Ordinary compilation rejects dependencies outside its root; frozen compilation never reads host files outside its captured set. Network packages are not downloaded automatically and there is no package-installation UI.

Current output is available only through the authenticated local session; each download checks the session and origin. Multiuser permissions, server revocation and resumed-download authorization are verified in v0.6; other clients and Agent automation follow the roadmap. Windows results do not cover other platforms. Power-loss, operating-system IME and complete assistive-technology acceptance need separate evidence.

Candidate acceptance runs frontend/Rust tests, baseline/feature/output HTTP scenarios, formatting/Clippy, fonts and documentation/bounded-model checks. Actual browser and Office checks are recorded separately; XML inspection does not replace opening, numbering updates or editability in a target application. See the delivery record for exact candidate results rather than reusing earlier pass counts as this version's evidence.

Requires Rust, Node.js/npm, Python 3.11+ and Windows C++ build tools. This work used Rust 1.98, Node.js 24 and official Typst 0.15.1.

```text
python scripts/mvp.py build
python scripts/mvp.py check
python scripts/mvp.py package
```

The default builds a local debug candidate; `--release` selects optimization. Checks cover documents/bounded models, fonts, frontend behavior, Rust formatting/static analysis/tests and real-program HTTP scenarios. They do not replace GUI, IME or power-loss testing.

Packaging creates a new `dist/` directory, retains previous candidates and does not push or publish. It includes the working-tree source snapshot, licenses, font manifests and build dependency notices. The inventory includes test/build dependencies, not a claim every item is in the binary. Published releases follow [WORKFLOW](WORKFLOW.md).
