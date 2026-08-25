---
source_language: zh-CN
translation_of: 12-document-actions.zh-CN.md
translation_status: synced
---
[简体中文](12-document-actions.zh-CN.md)

# Document revisions and edit actions

This specification defines the single-document action boundary; multi-file actions use the workspace transaction boundary in [`13-workspace-transactions.md`](13-workspace-transactions.md).

## Exact source authority

A document revision is the lowercase SHA-256 digest of the exact UTF-8 bytes in the same-named `X/X.adoc` canonical node document. Line endings, byte order, YAML spelling, comments, scalar styles, blank lines, and format whitespace all contribute. Weftext does not normalize before hashing or saving. The required `.weftext-format` marker must select `weftext.asciidoc.v1`; a missing/unknown marker, missing canonical document, or competing same-named candidate fails closed.

The Core read result contains node identity, node directory, document path, exact source, and revision inside the trusted process. A read fails closed for invalid node structure, non-UTF-8 source, linked/reparse document paths, invalid frontmatter, missing identity, ambiguous metadata, or a `.weftext-rules` ancestor that classifies the directory or canonical document outside the managed boundary. Transport and CLI responses expose UUID plus relative locators where needed, never these absolute host paths.

## Deterministic edit plan

The first edit action is an ordered set of non-overlapping UTF-8 byte-range replacements against one base revision. Ranges must land on character boundaries. Core canonicalizes edits by start, end, and replacement text, applies them to the exact source, and computes the next revision without writing.

The preview reports node ID, target, base and next revisions, canonical edits, old/new byte lengths, whether content changes, and the exact proposed source. Source outside an edit range remains byte-for-byte identical. This permits ordinary body edits and future narrow YAML CST patches without whole-frontmatter serialization.

The resulting source must still contain the same sole top-level `weftext` mapping and `weftext.id`. Removing, changing, duplicating, or making identity metadata ambiguous rejects the plan. An ordinary text action cannot repair or regenerate identity.

Native checklist toggle is a narrow single-document action against one parser-confirmed occurrence and exact marker range; it never creates identity or fields. Task-node state, priority, and date edits are narrow literal document-header attribute actions against a valid `weftext-task` v1 profile and preserve unrelated header/body bytes. Core reparses and validates the complete resulting profile before returning a plan. Dependency replacement additionally requires the authorized workspace graph and therefore uses the workspace transaction boundary. Checklist-to-task-node promotion creates a node and replaces source together, so it is never split into a single-document edit plus an unrelated create call; [`13-workspace-transactions.md`](13-workspace-transactions.md) governs it.

An unmanaged file, including Markdown or an `.adoc` file, is not accepted by the node-document action boundary even if its bytes contain a valid-looking envelope. Core does not parse, normalize, draft, icon-patch, or commit it as a node. Explicit raw-file reading and import are different capabilities and cannot reuse node transactions implicitly.

## Standalone external AsciiDoc boundary

Desktop may grant a separate standalone action boundary to one explicitly selected external `.adoc` or `.asciidoc` file. The grant is the exact normalized file path plus file-object identity where the platform exposes it and the digest of the opened bytes. It does not scan the parent or siblings, inspect or adopt a workspace marker, generate a UUID/envelope/sidecar, or allow node and workspace actions. An `.adoc` file classified as workspace unmanaged content remains under its workspace content boundary and is not silently reopened through this escape hatch.

Standalone plans use the shared safe AsciiDoc parser and semantic formatting operations, but validate generic AsciiDoc rather than the Weftext managed-node envelope. Weftext-only constructs remain byte-preserved and unavailable. Save stages beside the granted target, preserves applicable permissions, flushes, atomically replaces, reopens, and verifies the exact result. A changed digest or file identity, target disappearance, link/reparse transition, or permission change rejects the commit and offers Compare or Save As; it never replans and overwrites automatically. Drafts and recent history are device-local records keyed by a privacy-preserving file identity and never create sibling authority files.

“Add to workspace” exits this boundary through an explicit import preview that creates a new canonical node with a fresh UUID. It does not adopt the external directory, retain the external file as node authority, or overwrite that source. The standalone ribbon capability matrix is specified in [`06-application-ui.md`](06-application-ui.md).

## Profile-aware formatting plans

`DocumentFormatPlan` is the single-document typed boundary for toolbar, selection-bubble, slash-menu, context-menu, keyboard, command-palette, CLI, and Server formatting actions. The request binds the node UUID, base revision, exact UTF-8 selection, action kind, and typed action input. Core reparses the selected profile, resolves the semantic blocks/cells affected, returns selection capabilities and diagnostics, and constructs narrow replacements. A caller cannot translate a visual DOM selection directly into AsciiDoc punctuation or regenerate the document from HTML.

Core may extract a `FormatDescriptor` from one parser-confirmed selection and apply it to another compatible selection. The descriptor contains only portable semantic formatting: supported inline marks and compatible paragraph, list, quote, or heading presentation. It never contains text content, link targets or node UUIDs, explicit anchors/IDs, citation or footnote/endnote bodies, annotations, task state/dates/dependencies, Query expressions, Template slots, resource paths, or arbitrary source fragments. Capture is read-only. Apply returns a normal revision-bound `DocumentFormatPlan`; an incompatible, mixed-kind, protected, malformed, or stale target fails closed rather than partially applying the descriptor.

Undo/redo over a controlled device draft is client-local edit history until Save. The saved result still crosses this plan and commit boundary and is never exempt from identity, exact-source, profile, or stale-revision checks. Repeat Last Format stores only the last `FormatDescriptor` in device-local editor state and revalidates it against every new target.

## Structured table and heading plans

A table request carries a Core-resolved table range plus logical row, column, or rectangular cell selection; frontend DOM coordinates are not transaction evidence. Header-region plans accept only the first contiguous `N` rows as column headers and the first contiguous `N` columns as row headers. Either count may be zero, both regions may coexist, and Core reports their intersecting cells once. Repeating the current counts is a no-op. Native profile spelling is selected by the Core profile writer and no hidden table model or sidecar becomes storage authority.

A merge plan accepts one continuous rectangular selection inside one valid table. It rejects selections that cut an existing span, table boundary, protected range, or unsupported nested cell structure. Zero or one non-empty selected cell can merge directly. Multiple non-empty cells require an explicit confirmation input bound to Core's exact row-major composition preview; the plan preserves every cell body in that order and never silently overwrites one with another. A split plan restores the represented row/column grid, retains the merged content in the leading cell, and creates empty remaining cells. Split is a structural operation, not historical reconstruction; only draft Undo restores the exact pre-merge distribution. Core returns resulting dimensions, span/header regions, content disposition, affected byte ranges, and exact proposed source.

A heading-level plan identifies the parser-confirmed heading and requested H1-H9 level. `heading_only` changes that heading and reparses the resulting hierarchy. `preserve_subtree` shifts every descendant heading by the same delta while preserving relative structure. Core rejects an H0/body-title conflation, a result outside H1-H9, a protected/malformed range, or a hierarchy prohibited by the profile. Convert-to-paragraph is a separate action and must report descendant reparenting before it is accepted. Heading text, explicit anchors, links, and annotation targets remain unchanged unless a separate explicit action says otherwise; the preview reports the old/new outline paths and all affected headings.

All table and heading plans capture the base revision and resolved source ranges before an asynchronous preview. A later cursor, selection, pane, or document change cannot retarget the request. The complete next source must reparse as `weftext.asciidoc.v1`; otherwise no executable plan is returned.

## Single-document commit

Commit accepts a previously constructed plan. Core rereads the node and rejects a changed identity or stale base revision before staging. It writes the exact next source to a request-owned temporary file in the same node directory, copies portable file permissions, flushes and synchronizes the staged file, reopens and verifies its digest, rechecks the target revision, atomically replaces the document through the reviewed cross-platform temporary-file implementation, synchronizes the committed file, then reopens and verifies identity and revision.

A no-op plan returns the current verified revision without rewriting the file. Failed preview or stale-revision commit does not change the target. Temporary names use the reserved `.__weftext-transaction-` prefix and are ignored by workspace scanning.

Core plan/commit handles one document; workspace structural journals and startup rollback use the separate transaction specification. Automatic merge, cross-process locking against uncooperative external editors, and recovery UI require their own explicit contracts before exposure.

## Caller and agent boundary

Desktop, CLI, Server, and approved Agent actions must call the same read, plan, and commit functions. A caller cannot label generated text as committed until Core returns the verified committed revision. A stale result is a conflict, not permission to replan and overwrite automatically.

The read-only agent surface exposes exact reads and revisions only after startup workspace scoping and UUID lookup through a valid rebuilt inventory. It exposes no mutation method. Agent mutation remains disabled until an action request binds one preview to an explicit approval and the same Core commit path.
