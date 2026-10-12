---
source_language: zh-CN
translation_of: REVIEW.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW.zh-CN.md)

# Validation and review status

Status: author candidate, without independent acceptance or product refactoring. Earlier bounded passes do not transfer to this solution. This file records actual evidence and its limits; unrun scenarios are not passes.

## 1. Evidence layers

Requirements reconciliation covered current REQUIREMENTS, the complete historical D4 and type catalog, producing the seven-facet capability table in SOURCE §5.1. Historical business inputs reveal omissions; old wrappers and runtime protocols do not transfer automatically. This is not a new word-for-word audit of every historical D1–D10 file.

| Actual work | Result and scope | What it does not establish |
| --- | --- | --- |
| Native structured data | The current complete [schema](examples/schema.json) passes 31 bounded [design checks](examples/design-checks.json). Official Typst 0.15.1 runs 17 [compile cases](examples/syntax-experiments.json): 16 succeed and a heading/Bib key collision fails as expected. Semantic negatives still compile | Python checks as an actual Rust installer, CEL parser, providers, permissions, forms or export; native compilation as proof of field validity |
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
| C-P1-01 | Protecting only the source directory can still move it into a replaced target parent; CORE must define actual ancestor/parent-object protection and same-handle actions | Author repair integrated; independent review pending |
| C-P1-02 | Same-user private staging does not isolate outside writers, and enumerated membership does not exclude a new child; CORE must make ordinary copy/purge viable while preserving unknown additions | Author repair integrated; independent review pending |
| C-P1-03 | Field-only qualification lacks a producer, and an unchanged hidden field's validity can control the public write outcome; SOURCE/CORE must define static dependencies, public qualification and authorized local validation | Author repair integrated; independent review pending |
| P1-REQ-01 | The closed schema cannot register promised relations and cross-field Event rules; SOURCE owns installable definitions, signatures, target domains and consumers | Author repair integrated; independent review pending |
| P1-REQ-02 | Saved Query retains only short names/CEL and cannot reconstruct original Field/Type bindings after clearing indexes; SOURCE/FEATURES/CORE own a durable resolution baseline and migration | Author repair integrated; independent review pending |
| FRESH-FEATURES-P1-01 | A transaction A10/B0→A0/B10 can be mixed into sum20 and pages can duplicate/omit rows; CORE/FEATURES own a common read snapshot and frozen result order | Author repair integrated; independent review pending |
| P1-EDITOR-01 | Stable execution traces include discarded measurement content and repeated layout candidates; EDITOR owns final consumption selection and necessary counterexample evidence | Final Html route and bounded prototype integrated; independent review pending |
| P2-REQ-01 | Native citation keys lack reconstructible stable paper NodeRef identity and can drift after key changes/collisions; SOURCE/FEATURES own durable citation inputs | Author repair integrated; independent review pending |
| P2-REQ-02 | Independently writable birthday and sourced birth events create two facts; SOURCE/FEATURES own one authored source and a simple form entry | Author repair integrated; independent review pending |
| FRESH-FEATURES-P2-01 | Applying EXDATE before replacement may remove an explicitly rescheduled occurrence; SOURCE/FEATURES own per-original-key decisions and consistent window handling | Author repair integrated; independent review pending |
| FRESH-FEATURES-P2-02 / P2-EDITOR-03 | English ordinary annotation reads disclose sourcePreimage and may reveal private URIs; FEATURES must use authorized stored visibleExcerpt | Author repair integrated; independent review pending |
| P2-EDITOR-02 | The probe runner visits only elem objects and misses depth-truncation sentinels; diagnostic completeness must check all containers | Runner repaired: eleven original traces have no truncation and the reviewer's actual deep trace is rejected by the new oracle; no recompilation claimed, independent review pending |

The editor reviewer independently reran the eleven original cases and nine new counterexamples, preserving bounded no-side-effect equality while reproducing discarded measurements, repeated table layouts and the deep-truncation false pass. Other reports' contract counterexamples are not represented as product executions. Repairs will form one coordinated fixed candidate for independent review of actual changes and affected dependencies, followed by a fresh Pro global review. That final review has not been dispatched and the repairs are not yet accepted.

The repair also incorporates user-confirmed requirement §5.4: source, rendered with syntax always visible, syntax on demand, syntax hidden with UI editing, and read-only reading. The earlier fixed-candidate reviews did not cover this clarification. The new review must check shared Draft/Undo, source-preserving switches, IME/focus and source preservation for unsupported constructs. These are neither font-size presets nor five persistent formats.

Review records must identify the exact commit, actual reading scope, findings and evidence. Author checks, compilation or old CLOSED states do not close findings. Checkers validate documentation pairs, links and protected inputs, not semantics. Freeze the refactoring baseline only after major design gaps are resolved, material choices have sufficient evidence and independent acceptance is obtained. See [INTEGRATION](INTEGRATION.md) for sequencing.
