---
source_language: zh-CN
translation_of: REVIEW.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW.zh-CN.md)

# Validation and review status

Status: author candidate, without global acceptance or product refactoring. Earlier bounded passes do not transfer to this solution. This file records actual evidence and its limits; unrun scenarios are not passes.

## 1. Evidence layers

Requirements reconciliation covered current REQUIREMENTS, the complete historical D4 and type catalog, producing the seven-facet capability table in SOURCE §5.1. Historical business inputs reveal omissions; old wrappers and runtime protocols do not transfer automatically. This is not a new word-for-word audit of every historical D1–D10 file.

| Actual work | Result and scope | What it does not establish |
| --- | --- | --- |
| Native structured data | The current complete [schema](examples/schema.json) passes 31 bounded [design checks](examples/design-checks.json). Official Typst 0.15.1 runs 23 [compile cases](examples/syntax-experiments.json): 22 succeed and a heading/Bib key collision fails as expected. Semantic negatives still compile | Python checks as an actual Rust installer, CEL parser, providers, permissions, forms or export; native compilation as proof of field validity |
| Headings and run-in | Seven earlier bounded cases cover shared standalone/run-in numbering, outline, references and insert/delete/move/level changes; subtitle stays outside heading counters | Complete editor, deep/long documents or all custom show rules |
| Word | Ten OOXML structural checks pass on an isolated sample, including whole Heading N paragraphs, effective body-character resets and fields | Real Word/WPS/LibreOffice field updates, TOCs or save/reopen; reference images are not Office renders |
| Origin counterexamples | Seven official CLI samples show query.body differing from actual post-show output, removed display retaining heading introspection, and repeated includes not derivable from root AST | A complete flow tree from AST or CLI query alone |
| Local file protection | A bounded Windows NTFS probe verifies share=0 excluding tested write opens, replacement and deletion; existing writers/mappings prevent acquisition | Every platform/driver/filter, power loss, external ABA or automatic recovery |
| Interrupted recovery | Eight two-file process-interruption cases exercise before/after states, retained third states and original-operation zero-repeat writes | Actual power loss, torn journals or same-object equal-byte continuity after protection release |
| Directory counterexamples | Directory share=0 permits child additions/writes; child handles may prevent parent rename; no-replace preserves occupied targets | Capture as a whole-directory CAS or complete negative scopes from scans of open namespaces |
| Ordinary directory moves | Same-object no-replace move preserves directory/child identity and carries late-added children and external edits; an open writer makes it busy until closed; two-step Node rename succeeds, while a competing old name prevents rollback and retains both drafts | Instant multi-object atomicity for external tools, equivalent cross-volume primitives, power loss, ancestor races or complete fault recovery |
| Directory guard, creation and cleanup repair | A [self-contained probe](probes/core-repairs/README.md) on local NTFS exercises actual LIST-access ancestor/target-parent protection, same-handle move, NtCreateFile returning the original directory creation handle, name collisions, planned-file protection and preservation of unknown children in nonempty directories | Every ancestry chain/alias/filter, metadata durability, power loss, multi-step recovery, Server isolation or a qualified production backend |
| Query common-cut model | The same bundle's bounded memory model reproduces individual reads totaling20; a common cut returns10, frozen pages remain fixed and modeled revocation stops later delivery | Implemented filesystem journal, complete inventory, current publication, authorization, durable retention or performance |
| Library styles | Five official compilations measure body/list sizes, table borders, local restoration and actual relative assets; changing a module leaves note bytes/AST unchanged; a set inside transformational show demonstrates its override limitation | Implemented library-settings UI, Core dependency/revocation/late-result invalidation, export or rich editing |
| Official evaluator observer | [Reviewable prototype](probes/semantic-bridge/README.md): all 11 observer-on/off/official-release PDFs are byte-identical; on/off World request sequences and final Frames match; tracked Sink replays final stable-round observations through the cache | Complete value-flow origins, all layout/table/math cases, a renderer, inverse rich editing, AT, performance or all of E1 |
| Html final consumption and native styles | [16 reproducible cases](probes/html-flow/README.md): adapter-off Html matches official bytes; adapter-on serial/parallel Html and final trees match; three Paged PDFs match; bounded body semantics are preserved, with World ordering equal in 15/16 cases | Browser renderer, every style/table topology, production origins/writers, local-frame reading/copy trees, AT or all of E1 |

These experiments changed neither product implementation nor real materials. Complete raw runs remain in the private workspace. The public observer includes a fixed patch, lock, eleven inputs, runner and compact actual results, excluding downloads, build directories and private chats.

Naive direct observer logs lost inner events on cache hits. The actual repair adds an independent observation slot to the official tracked Sink, merged in original order and replayed by memoization, without disabling cache or rerunning show. The default stable round recovers four realization entries from one physical call; repeated include recovers seven and the run-in/list/table/math sample forty-two. Independent counterexamples subsequently showed that a stable trace still includes discarded measurement content and repeated layout, so it cannot directly supply the final reading tree. Those counts establish observation coverage, not visible occurrences. The current route is EDITOR §8's official final Html consumer tree and Paged output, with separate style/role validation. Computed text in `text(left + right)` has only a call span, which cannot recover both input origins or grant a writer; retain it as generated/not directly editable unless another validated origin proof exists. The original eleven equality results remain valid but do not close E1.

## 2. Design risks and product acceptance

Design review determines whether mechanisms are concrete, feasible and sufficient. Product acceptance validates actual implementations and supported platforms. Design does not require implementing the entire product first; future tests cannot conceal a missing viable algorithm.

| Area | Design question to settle now | Later real implementation acceptance |
| --- | --- | --- |
| E1 official semantic bridge | Final Html consumer tree excludes measured/discarded work and retains real repetition; bounded native style/role and origin adaptation with explicit Html/Paged boundaries; defaults flow and ambiguity grants no writer | Complete renderer, all inverse editing maps, AT/RTL, resources/budgets, security and upgrade regression |
| C1 files and structure | Explicit protection/object-move boundaries; ordinary move is not whole-tree copying and does not imply strong negative scopes | Supported filesystems, permission/rename failures, power loss, recovery, external races and reader barriers |
| C2 authorization and services | Static field isolation, actual filesystem writer fencing, current delivery and cache invalidation | Hidden-value side channels, dual holders, revoke/chunk/commit races and process isolation |
| S1 types and Query | One Type/Value system; concrete selected shapes/consumers; consistent missing values, same-item collections and precision | Complete seeds/providers, CEL profile and forms/Query/import/export vertical acceptance |
| F1 annotations and copy | Accurate origins and one allocation/result map; source and accepted annotation commit together; separate permission domains | Crash/race/media precision, complex source selections, reattachment and exports |
| E5 editable output | Native heading/body-format behavior, explicit target limits, exact artifact confirmation/download | Real Office/WPS/LibreOffice, XLSX precision, loss reports and intranet resume |

Ordinary local use, direct external editing, complete legal Typst and hosted multi-user needs remain. Unqualified production backends cannot be advertised as supported; marking every normal action unavailable does not satisfy requirements.

## 3. Independent review and entry criteria

All four independent Sol reports bind `b895b350d4b4ceda5f2317f2d4b1d8efb42bd11e` and conclude **REVISE**. Their scopes are requirements/types, Core/files/authorization, editor/semantic consumption, and features/Query. They neither re-read every historical protocol nor claim product acceptance. After deduplication there are seven P1 and five P2 findings, with no reported P0. Two reviewers found the annotation disclosure translation defect; it is counted once. Complete private reports have been collected and read. The following self-contained findings and repair owners do not publish private conversations.

| Independent finding | Concrete harm and repair owner | Current disposition |
| --- | --- | --- |
| C-P1-01 | Protecting only the source directory can still move it into a replaced target parent; CORE must define actual ancestor/parent-object protection and same-handle actions | Independently design CLOSED at c53af36 within the finding scope |
| C-P1-02 | Same-user private staging does not isolate outside writers, and enumerated membership does not exclude a new child; CORE must make ordinary copy/purge viable while preserving unknown additions | Independently design CLOSED at c53af36 within the finding scope |
| C-P1-03 | Field-only qualification lacks a producer, and an unchanged hidden field's validity can control the public write outcome; SOURCE/CORE must define static dependencies, public qualification and authorized local validation | Independently design CLOSED at c53af36 within the finding scope |
| P1-REQ-01 | The closed schema cannot register promised relations and cross-field Event rules; SOURCE owns installable definitions, signatures, target domains and consumers | Independently design CLOSED at c53af36 within the finding scope |
| P1-REQ-02 | Saved Query retains only short names/CEL and cannot reconstruct original Field/Type bindings after clearing indexes; SOURCE/FEATURES/CORE own a durable resolution baseline and migration | Independently design CLOSED at c53af36 within the finding scope |
| FRESH-FEATURES-P1-01 | A transaction A10/B0→A0/B10 can be mixed into sum20 and pages can duplicate/omit rows; CORE/FEATURES own a common read snapshot and frozen result order | Independently design CLOSED at c53af36 within the finding scope |
| P1-EDITOR-01 | Stable execution traces include discarded measurement content and repeated layout candidates; EDITOR owns final consumption selection and necessary counterexample evidence | Final Html route and bounded evidence independently design CLOSED at c53af36 |
| P2-REQ-01 | Native citation keys lack reconstructible stable paper NodeRef identity and can drift after key changes/collisions; SOURCE/FEATURES own durable citation inputs | Independently design CLOSED at c53af36 within the finding scope |
| P2-REQ-02 | Independently writable birthday and sourced birth events create two facts; SOURCE/FEATURES own one authored source and a simple form entry | Independently design CLOSED at c53af36 within the finding scope |
| FRESH-FEATURES-P2-01 | Applying EXDATE before replacement may remove an explicitly rescheduled occurrence; SOURCE/FEATURES own per-original-key decisions and consistent window handling | Independently design CLOSED at c53af36 within the finding scope |
| FRESH-FEATURES-P2-02 / P2-EDITOR-03 | English ordinary annotation reads disclose sourcePreimage and may reveal private URIs; FEATURES must use authorized stored visibleExcerpt | Independently design CLOSED at c53af36 within the finding scope |
| P2-EDITOR-02 | The probe runner visits only elem objects and misses depth-truncation sentinels; diagnostic completeness must check all containers | Independently design CLOSED at c53af36: original eleven traces have no truncation and the actual deep trace is rejected |

All four follow-up independent reports bind `c53af36afc674b16dfcde1acbc95c839c37d1348` against the earlier candidate. They close the twelve unique original findings within their actual scopes, without transferring acceptance to unreviewed contracts. Core and editor report no new findings; requirements and features report four unique new P2 findings and no new P0/P1. Complete reports were collected and read. Core independently reran the public namespace/common-cut scripts; editor reran the sixteen public cases plus seven additional inputs and the actual deep-trace oracle. These are bounded feasibility results, not production acceptance.

| New finding | Coordinated repair | Current disposition |
| --- | --- | --- |
| P2-C53-REQ-01 | FEATURES now names the installed temporal/range validator; no invented alias. | Independently design CLOSED at d9e63fd within this delta |
| P2-C53-REQ-02 | Both paper examples include required CalendarDate version. | Independently design CLOSED at d9e63fd within this delta |
| P2-C53-REQ-03 | Saved Query select guards optional phone labels; QOPT01 covers mixed labelled/unlabelled data. | Independently design CLOSED at d9e63fd within this delta |
| C53-FEATURES-P2-01 | SOURCE, FEATURES and schema distinguish unknown absent During endpoints from explicit unbounded native none; DUR01–09 specify forms/providers/Query and preserve separate Range behavior. | Independently design CLOSED at d9e63fd within this delta |

Requirements and editor independently confirmed the five user-selected views at c53: source, rendered with all syntax, syntax on demand, syntax hidden with UI editing, and read-only reading. Shared Draft/Undo, source-preserving switches, IME/focus and explicit source access for unsupported constructs remain requirements. This accepts their design preservation, not five implemented views.

Two non-author reviewers independently read the complete ten-file delta at `d9e63fd12b0cc33786cccd568ac4729d50da506e` against c53, including the schema, During/Range/provider/Cell consumers and numbered design oracles. Both close all four P2 findings with no new findings in that bounded scope. Complete reports were collected and read; provider/CEL/UI behavior was not executed. Unchanged Core/editor closures remain tied to c53 and their unaffected contracts. A fresh Pro global review is still required before freezing the refactoring baseline; there is no global acceptance or product implementation claim.

Review records must identify the exact commit, actual reading scope, findings and evidence. Author checks, compilation or old CLOSED states do not close findings. Checkers validate documentation pairs, links and protected inputs, not semantics. Freeze the refactoring baseline only after major design gaps are resolved, material choices have sufficient evidence and independent acceptance is obtained. See [INTEGRATION](INTEGRATION.md) for sequencing.

## 4. Review boundary for Chinese defaults

The user-confirmed 2026-10-13 supplement adds REQUIREMENTS §5.5, EDITOR §8.6 and CORE/FEATURES consumption: no per-note Chinese activation; distinct body/raw/math fonts and language; offline font delivery, native overrides and missing-glyph/export boundaries. CJK01–08 are new, unexecuted product acceptance requirements. This change installs no fonts, implements no renderer and verifies no Office behavior. Existing bounded style experiments do not cover these cases.

A non-author editor reviewer fully checked the ten-file bilingual delta at `c4a37874114b333442a9fd531e4aff824097061b` against cdb, plus affected five-view, module override/opt-out, Html/Paged/DOCX, Environment and resource-delivery contracts. The result is bounded PASS with no new P0/P1/P2. The complete report was collected and read. This accepts requirement preservation and interface consistency within that supplement, not CJK01–08 product behavior.

The completed global final review (REVISE, two OPEN P2 findings; see §5) is fixed to `cdb05a3f7c5fe74fe147f34d6f91f7ee0eda0a0d` and excludes this supplement; its verdict cannot automatically accept later changes. This bounded result must still be reconciled with the global verdict for complete design acceptance. Completed unaffected reading need not be restarted.

## 5. cdb global review and coordinated repair candidate

Independent Pro global review targeted `cdb05a3f7c5fe74fe147f34d6f91f7ee0eda0a0d` and concluded **REVISE**: no P0/P1 in its complete current architecture/requirements/configuration and declared related historical-business scope, with two new OPEN P2 findings below. The coordinator received the complete report. These self-contained harms/repair entries preserve its judgment without private conversation or author acceptance. The c4a bounded Chinese-default PASS still covers only §4.

| Original Pro finding | Harm and this repair candidate | Review state |
| --- | --- | --- |
| TYPST-GLOBAL-CDB-P2-01 | Actual During/Range accepted only DateOrInstant, losing March-2020 appointment starts without a known day. SOURCE §3.1.1 and real TemporalEndpoint reuse CalendarDate, including selected Task.start/due date precision and provider/Query/UI/export consumers. Same-grain comparison/years-months duration works, mixed precision is proved or unavailable, presence/none distinctions survive and recurrence stays exact. | Author repair candidate, pending independent; not CLOSED. |
| TYPST-GLOBAL-CDB-P2-02 | Generic role/customLabel lacked category discrimination/domains, allowing same-text friend intent to drift. SOURCE §4.1 and actual Field-specific Shapes require closed preset/custom relationship branches, preserving direction/single authority and consumer round trips without guessed classifications. | Author repair candidate, pending independent; not CLOSED. |

This coordinated candidate starts at clean `c048626acb89654a4d6c986aa8bd798a3a3a3c53`, leaving snapshots/inputs and Chinese-default additions intact. CORE clarifies unchanged authorization with data-only changes; FEATURES distinguishes venue/container and the absence of a defined independent publisher field without expanding the publication model.

Actually run: 31 existing closure/path/design-ABI checks pass; 69 new [bounded actual-Field model checks](examples/precision-relation-checks.json) pass; official 0.15.1 runs 23 [compilation cases](examples/syntax-experiments.json), with 22 successes and the expected heading/Bib-key ambiguity failure. Two new semantic negatives remain legal Typst. Model structural transport is not Office; design ABI is not the trusted Rust installer. Complete providers/CEL, UI/authorization/transactions, fonts/CJK/Office product behavior remain UNRUN, and unchanged Html/Core platform probes were not rerun. Independent review of the new fixed candidate determines both P2 dispositions and global acceptance.
