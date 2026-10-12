---
source_language: zh-CN
translation_of: INTEGRATION.zh-CN.md
translation_status: synced
---

[简体中文](INTEGRATION.zh-CN.md)

# Integration and review of the Typst candidate

Status: this coordinated design revision and independent Sol review are complete; product unimplemented and full global review incomplete. Fixed input: `af6737a057c7bb69763dfaad0a57da4652e085f3`. This document defines normative dispatch and cross-module acceptance without changing fixed sources, prior reports or actual historical records.

## 1. One candidate with explicit ownership

This directory coordinates the language, primary source and direct consumers for new notes. This document and the four owners form one candidate, not four interchangeable proposals:

| Owner | Sole responsibility | Main consumers |
| --- | --- | --- |
| [SOURCE](SOURCE.md) | Root declaration, native values, schema, ordinary business data and source qualification | Properties, Query, creation and copying |
| [CORE](CORE.md) | Format profile, compilation environment, input evidence, durable qualification and transaction dispatch | All surfaces, compiler, synchronization receiver and recovery |
| [EDITOR](EDITOR.md) | Reading projection, edit maps, Draft, annotations and accessibility | Desktop, WebUI, Mobile and export |
| [FEATURES](FEATURES.md) | Links, Query/View, tasks, templates, conversion, intranet output and extensions | Core domain actions and shared interaction entry points |

SOURCE defines what the source says; CORE verifies who can currently read or commit it; EDITOR defines which visible positions can safely be written back; FEATURES defines how these data are organized and consumed. No layer substitutes for another layer's evidence.

Each owner lists named replacements; [replacements.json](replacements.json) provides machine navigation. New rules cover only explicitly listed concerns. Retained generic algorithms reference named clauses of the fixed A2 owners, rather than selecting conflicting format rules from old files. Roadmaps, review status and historical versions in original inputs remain source records, not new execution instructions.

## 2. Boundaries of reuse and removal

Retain [D1](../a2-final-design/D1.md) §§2–12: five formal surfaces, one Core, local and Server modes, one durable committer, revocation, distinct guarantee levels, progressive indexes and release evidence. The Typst compiler is a controlled capability, not a sixth writing authority. The old implementation's `X/X.adoc`, AsciiDoc marker and predecessor authoring protocols describe the old implementation or candidate, not the new target.

New authoring rules replace AsciiDoc-specific syntax, attribute blocks, native oracles and write adapters. SOURCE explicitly simplifies ordinary People/Organization contacts; default scope no longer includes per-value fact identity, elaborate qualifiers or automatic inverse relations. Ordinary relationships can use authorized note links. Generic version history, integrity and permissions survive that business-model simplification.

Native functionality and Weftext functionality have separate acceptance. Standard `metadata` syntax does not establish identity; `link` does not establish target access; Typst `query` does not execute Workspace-wide Query; compilation success is not commit success; an unavailable rich-edit control does not make legal source invalid. Operations lacking source, authorization or capability evidence are explicitly unavailable while authorized source repair remains available.

JSON may remain in independent annotations, configuration and internal transport; these do not become a second authoring format inside new notes. Historical import previews preservation and loss and retains originals. Undeployed candidate versions do not invent runtime migration requirements, and genuine historical records are never reinterpreted as the new protocol.

## 3. Cross-module acceptance

These are design requirements. Product behavior is UNRUN; documentation or example checks cannot mark them as product passes.

| ID | Positive path | Required rejection or explicit state |
| --- | --- | --- |
| TI-01 | Create ordinary notes, contacts and tasks: schema form → native source → one identity allocation/commit → fresh read | Template-forged birth, a second creation after unknown save, or title used as NodeId |
| TI-02 | Edit only the selected one of two identical phone values, preserving other bytes and one Undo | Old index used after reorder, display label treated as field identity, or per-value UUIDs restored |
| TI-03 | Legal computed body remains readable, static properties independently readable, writable regions qualified by current origins | A macro prevents ordinary source saving, or equal display text is used to infer a generated write location |
| TI-04 | Field and body edits share one Draft with complete interaction validation before commit | Half-installation, narrow Field rights used to read/submit whole source, or stale evidence revived by identical bytes |
| TI-05 | Includes, packages, fonts and dates enter through a controlled environment and invalidate on change/revocation | Engine bypasses Server to access disk, or hidden dependencies leak through errors, caches, export or Query |
| TI-06 | Link conversion preserves target, fragment, visible text and unrelated source | Shortest-path guessing, ID conversion drops an anchor, or local absolute address grants remote file access |
| TI-07 | Crossing potential matches, alias ambiguity and more than two layers stay selectable; explicit conversion reauthorizes | Automatic writes, selecting the first ambiguous target, suggestions counted as backlinks, or hidden candidate counts exposed |
| TI-08 | Native body range → independent annotation → fresh same-version cross-device read; temporal/resource-region semantics retained | Old map/Observation updated and revived, navigation or digest recomputation reads hidden source without authorization, or invalid annotations discarded |
| TI-09 | Query/View reopen, saved search, explicit Trash search, collection creation and task dragging use actual consumers | Old result rows stay writable, 200 displayed rows prove complete membership, or aggregate/generated rows become narrow Fields |
| TI-10 | Template confirmation → fixed data/resources → generation/conversion → authorized intranet download | Compilation called saving, a PDF table called XLSX, or retry returns a different unconfirmed artifact |
| TI-11 | Rename, move, copy and initialization materialize and recover through one plan/map | Reallocated identities, noncanonical array ordering after materialization, or mixed evidence rewritten as the latest format |
| TI-12 | Flowing reading, print, math, deep/run-in headings share source; mode changes retain the draft | PDF-only reading, styles become another body copy, or screen coordinates replace logical selections |

Finer module cases remain jointly applicable. These twelve rows do not replace complete source obligations or all product tests.

TI-08 retains the original annotation disclosure domains: authorization to read the complete Annotation permits its already stored `expectedText` byte length and digest. It does not authorize rereading the target, recomputing that digest or attaching hidden context.

## 4. Review and evidence

First form a coordinated candidate and run necessary documentation/example checks, then fix one commit for non-authors. Challenges exchanged among authors remove interface conflicts but are not independent acceptance. Non-authors inspect current source rules, direct consumers, simplified requirements and the actual new-format disposition of old counterexamples. Findings return to authors, followed by delta and affected-path rechecks.

Fixed snapshots and `inputs.json` remain unchanged. The original 21 CLOSED findings still belong only to their prior candidate. Assess each obligation as retained or explicitly superseded: neither copy old passes into new passes nor invent new historical issues merely by reusing numbers. [REVIEW](REVIEW.md) records candidate commits, checks, review scope and residuals. Full Pro review and product implementation have separate boundaries.
