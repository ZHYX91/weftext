---
source_language: zh-CN
translation_of: MVP.zh-CN.md
translation_status: synced
---

[简体中文](MVP.zh-CN.md)

# Local writing and knowledge tool

The current version is **0.6.0-alpha.1, a Windows candidate**, retaining bounded local v0.2–v0.5 features and adding a separate [first Server / WebUI slice](SERVER.md). Full milestone acceptance still has the limitations below; this is neither the complete product nor a published release. Previous candidates remain separate. See the [roadmap](ROADMAP.md). Native `.typ` files remain the authoring source; editors, search and views do not store a competing body.

This candidate connects the **local note timeline** and **five-view body-embedded Query**, reusing Core save/recovery, shared source/Draft/Undo and the existing Query executor. The [roadmap](ROADMAP.md) records bounded local scope and outstanding platform conditions; implementation does not by itself establish all acceptance conditions.

## Launch and files

Run the packaged `weftext.exe`. Its window reports a local session address and opens the default browser; closing the process stops the service. Default local mode listens only on `127.0.0.1`, without a cloud account. The bounded multiuser entry requires explicit `--server-config`; see the [Server guide](SERVER.md) for configuration, startup and remaining scope.

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

Alpha.4 connects four local checks and Word review exchange. Open an admitted note and use Links and annotations → Proofreading to configure dictionaries, allowlists and scope. Checks consume the current successful visible-body compilation; code, equations, links and metadata are excluded, and generated text without exact provenance remains read-only. Character checks cover replacement characters, body BOMs and unusual controls. Pair checks cover common English/Chinese brackets and Chinese quotes, including crossed nesting, rather than full language grammar. Typos and sensitive terms use exact user-dictionary matches; empty dictionaries are explicitly unconfigured. Rules support JSON import/export. Browser settings are temporary preferences for the current local address; a changed startup address requires reimporting the dictionary, which is not claimed to be library-persisted or synchronized. Results are capped at 2000 and dictionary matches at 20000 per paragraph; exhausted budgets are partial.

Selected findings can become annotations or persistent ignore records, at most 200 per batch. Save the body first. Unlocated findings cannot create range annotations, and words spanning formatting can be reported without inventing executable replacements. The same rule/version and verified occurrence reuse existing annotations, preserving replies and review state; accepting a suggestion is a separate action. Ignored findings remain resolved annotations, while allowlists exclude exact terms. All records use `weftext.annotations.json`. After edits, stale ranges remain stale; explicitly select a current whole-grapheme range or exact ordinary-text/empty-body insertion point to reconfirm. Automatic relocation without proven edit maps is unavailable.

Selecting DOCX import previews body source, comment/revision counts and losses before creating a new copy and retaining the uploaded original. The first profile handles ordinary paragraphs, Heading-style headings, bold/italic, links, list contents, simple tables and readable images. Numbering/hierarchy, merged cells, headers/footers, pagination and complex layout are not guaranteed equivalent and are reported as losses. Comments preserve body, author/date, replies and resolved state. Uncertain targets attach to the original resource, never guessed same-text positions. Basic text revisions use the **pre-change body**: inserted text stays a pending insertion, and deleted text remains in the body with a pending deletion. Replacements may remain adjacent deletion/insertion suggestions without automatic acceptance. Nested, structural, formatting and move revisions preserve the original and report unsupported semantics.

DOCX output offers body-only, comments, or comments with pending text revisions, with an option to include resolved discussions. Review output requires a saved body and freezes both source and annotations; generation never reruns proofreading or changes application suggestion states. Exact targets become Word comments/replies; currently executable pending suggestions can become tracked text changes. Overlapping or unlocated items report downgrades/losses. Review copies use Word 2013 or later compatibility mode to support resolved state; upgrading an older template reports a loss. Arbitrary Word review structures are not claimed lossless. Native-host acceptance distinguishes Word comment editing/replies/resolution and revision acceptance/rejection, LibreOffice save/reopen, and WPS revision acceptance/rejection. WPS 12.1.0.29161 Replies.Add creates an independent comment in both native WPS and Weftext documents without recording a new reply relationship; its COM view groups overlapping scopes as replies. XML and actual Word reopening confirm that existing reply relationships remain intact. This localizes the failure to the current WPS automation reply-creation path, not a demonstrated Weftext serialization defect. Manual WPS reply interaction remains unverified.

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

The local writer profile remains `weftext.local-mvp.v1`, supporting Windows/NTFS. It does not promise network shares, cloud drives, other file systems, cross-device admission or sync. Hosted multiuser commits are serialized through this same backend; separate writers must not share the author tree. Symlinks, junctions, hard links and unprovable paths do not enter writes.

Author source is limited to 2 MiB; raw scans to 5,000 entries, with explicit incomplete outcomes at budgets. Compiler subprocesses have a 20-second timeout, 1 GiB Windows memory limit and 24 MiB output limit; frozen Query compile inputs are limited to 64 MiB. Ordinary compilation rejects dependencies outside its root; frozen compilation never reads host files outside its captured set. Network packages are not downloaded automatically and there is no package-installation UI.

Local Office output remains available through the authenticated local session. The first hosted slice separately verifies source reading/saving, history and source-download chunks. Hosted Office, annotations, Query and all remaining consumers continue in v0.6; other clients and Agent automation follow the roadmap. Windows results do not cover other platforms. Power-loss, operating-system IME and complete assistive-technology acceptance need separate evidence.

Candidate acceptance runs frontend/Rust tests, baseline/feature/output HTTP scenarios, formatting/Clippy, fonts and documentation/bounded-model checks. Actual browser and Office checks are recorded separately; XML inspection does not replace opening, numbering updates or editability in a target application. See the delivery record for exact candidate results rather than reusing earlier pass counts as this version's evidence.

Requires Rust, Node.js/npm, Python 3.11+ and Windows C++ build tools. This work used Rust 1.98, Node.js 24 and official Typst 0.15.1.

```text
python scripts/mvp.py build
python scripts/mvp.py check
python scripts/mvp.py package
```

The default builds a local debug candidate; `--release` selects optimization. Checks cover documents/bounded models, fonts, frontend behavior, Rust formatting/static analysis/tests and real-program HTTP scenarios. They do not replace GUI, IME or power-loss testing.

Packaging creates a new `dist/` directory, retains previous candidates and does not push or publish. It includes the working-tree source snapshot, licenses, font manifests and build dependency notices. The inventory includes test/build dependencies, not a claim every item is in the binary. Published releases follow [WORKFLOW](WORKFLOW.md).


## Same-owner local replica controls

The toolbar Sync control works without an open note. Enable sync on the original library and copy the complete pairing identity. Refresh saved versions publishes immutable original committed operations into the local sync store; it does not save the editor draft. Select objects and an exchange directory for export. The destination must be absent or already paired to the same identity, outside author trees and private state; an existing unpaired folder is refused.

A separate program uses its own `--data-dir` and a different library path. Paste the pairing identity, receive the exchange directory, select objects and preview an independent clone at an absent destination. Review all paths, files and byte changes before confirmation. Existing replicas use the apply preview. Receiving retains history without changing working files. Object selection includes original transaction/dependency closure; the frozen preview defines the actual write set. Partial replicas distinguish installed objects and exact revisions from received history.

Replicas can save while disconnected from the exchange directory. Original commits remain in the local outbox and are exported/received later. The UI lists every concurrent version and common base alongside the installed revision; revision identifiers are not timestamps. Current branches and common bases expose exact UTF-8 file inspection; binary files or files above 2 MiB show only digest and length. The current explicit merge selects one complete branch, including its source, annotations, resources and placement. It does not merge individual fields automatically. Both originals remain in sync history; unrelated objects can be selected and applied independently of a conflict.

The request, operation ID and preview are retained before confirmation. After a lost response, inspect that original operation; reloading the same origin retains its record. If the origin changes or browser storage is lost, use the previously retained original operation ID to recover the exact proposal without reconstructing it. A proposal can be explicitly cancelled only before its original installation marker exists. Once installation may have started, cancellation is refused and recovery inspection is required. Cancellation never rolls back files or overwrites external changes. Applying retains open editor drafts and Undo; changed disk source requires an explicit comparison before continuing.

These local-owner APIs retain the local token and Host/Origin boundary and are absent from hosted actor routes. Controlled local directory exchange is not an accepted remote hosted sync service. Multiple-tree moves, some structural/removal combinations, external-edit admission, a complete received-but-uninstalled history timeline, hosted rich editing and all-consumer authorization remain v0.6 work. This entry point does not establish complete v0.6 delivery.

`scripts/test_sync.py` is included in the existing `scripts/mvp.py check/package`; `scripts/test_sync_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` uses an explicitly supplied `WEFTEXT_PLAYWRIGHT_MODULE` for headless two-program browser acceptance. Tests create and retain only owned libraries/private state and never drive the host desktop. Actual passing scope belongs to the candidate record.
