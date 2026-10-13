---
source_language: zh-CN
translation_of: REVIEW.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW.zh-CN.md)

# Validation and review status

Status: **accepted-design-not-implemented**. Independent global final review accepted technical baseline `f9512f05f86311ea685d5e8d213784ac0d416324` on 2026-10-13, with zero open design blockers; see §7. Sections 1–6 retain actual evidence and the candidate status at each stage. Unrun product scenarios do not become passes. Product refactoring has not started.

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
| TYPST-GLOBAL-CDB-P2-01 | Actual During/Range accepted only DateOrInstant, losing March-2020 appointment starts without a known day. SOURCE §3.1.1 and real TemporalEndpoint reuse CalendarDate, including selected Task.start/due date precision and provider/Query/UI/export consumers. Same-grain comparison/years-months duration works, mixed precision is proved or unavailable, presence/none distinctions survive and recurrence stays exact. | Bounded CLOSED at fixed 7dcff6c; original Pro independently confirms CLOSED at fixed f456 (see §6). |
| TYPST-GLOBAL-CDB-P2-02 | Generic role/customLabel lacked category discrimination/domains, allowing same-text friend intent to drift. SOURCE §4.1 and actual Field-specific Shapes require closed preset/custom relationship branches, preserving direction/single authority and consumer round trips without guessed classifications. | Bounded CLOSED at fixed 7dcff6c; original Pro independently confirms CLOSED at fixed f456 (see §6). |

This coordinated candidate starts at clean `c048626acb89654a4d6c986aa8bd798a3a3a3c53`, leaving snapshots/inputs and Chinese-default additions intact. CORE clarifies unchanged authorization with data-only changes; FEATURES distinguishes venue/container and the absence of a defined independent publisher field without expanding the publication model.

Actually run: 31 existing closure/path/design-ABI checks pass; 69 new [bounded actual-Field model checks](examples/precision-relation-checks.json) pass; official 0.15.1 runs 23 [compilation cases](examples/syntax-experiments.json), with 22 successes and the expected heading/Bib-key ambiguity failure. Two new semantic negatives remain legal Typst. Model structural transport is not Office; design ABI is not the trusted Rust installer. Complete providers/CEL, UI/authorization/transactions, fonts/CJK/Office product behavior remain UNRUN, and unchanged Html/Core platform probes were not rerun. Global acceptance still requires the original Pro reviewer to assess the final fixed repair and Chinese-default supplement.

A non-author requirements reviewer fully checked the 23-file delta at `7dcff6cdd1c0dcb49576fa5d7977e0a100fe7f9d` against c048 and affected actual Fields, interval/category, Query, Task and exact-recurrence contracts, using the complete original Pro report and necessary historical business sources. The bounded result is PASS: both original P2 findings and the Task.start/due precision impact are design CLOSED, with no new P0/P1/P2. The complete report was collected and read. Independent runs in a private fixed copy passed 31 configuration checks, 69 actual-Field model checks and 23 additional boundary/category counterexamples. These do not establish real providers/CEL/Office or global acceptance. Official Typst compilation results are cited from the actual versioned evidence and were not rerun by that reviewer.

That review also recommended clarifying the old colleague sentence in FEATURES. This bilingual clarification distinguishes on-demand views computed from appointment overlaps from independent explicit professional_relations assertions. Such assertions need no engagement when their actual Field rules are satisfied; the two origins are neither automatically dual-written nor merged. This matches existing SOURCE §4.1 and the native example without another authority. The final delta and §4 Chinese-default supplement return to the original Pro reviewer; this bounded closure does not replace the final verdict.

## 6. f456 global review and common-basis repair candidate

The original independent Pro reviewer fully assessed fixed `f4560ddd4f93421db1f6c070889150cbc22e9bdb` and concluded **REVISE**: both original TYPST-GLOBAL-CDB-P2 findings independently CLOSED, Chinese-default supplement accepted at design level, no P0/P1, and only new **TYPST-GLOBAL-F456-P2-01 OPEN**. The complete report and evidence were collected and read. This is neither global acceptance nor CJK/Office product acceptance.

SOURCE §3.1.1, FEATURES §8.3 and the bounded model validated each Range separately. Separately valid start-only or end-only civil/instant Ranges then returned overlap=true without a finite cross-argument comparison; swapped arguments did likewise. The original independent 70 cases had 66 passes and four failures. Same-side coarse cross-calendar/version open ranges could also be washed into true, with the same root cause.

The candidate based on that clean commit repairs both SOURCE/FEATURES languages and the actual model: qualify complete authorized inputs, finite endpoints, provider coverage and a common temporal basis before infinity/three-valued conjunction. Domain eligibility does not require proven endpoint order: same-calendar/version month/day same-side open ranges remain true. Native dates versus CalendarDate(day) and supported cross-calendar full days remain usable. Civil/instant mixtures are invalid; coarse cross-calendar/version or missing providers unavailable. No inferred timezone/date or change to During absence/none and exact recurrence. Schema/ABI Shapes and signatures remain unchanged; the trusted builtin call-level preflight is made explicit in its algorithm.

Actual bounded checks: 31 configuration/path/design-ABI checks pass; [96 model cases](examples/precision-relation-checks.json) pass (69 original plus 27, including SOURCE RDOM01–07); the author reruns all original independent 70 cases against the new candidate with unchanged expectations, all passing; eight additional original family expectations also pass. These latter runs are author checks, not passing evidence for fixed f456 or independent acceptance. Unchanged 23 native compilation cases and Html/Core platform probes were not rerun. Complete Rust providers/CEL, authorization/transactions/UI and CJK/fonts/Office remain UNRUN.

This section records **repaired-pending-independent** at repair handoff, retaining both original CLOSED findings and Chinese design acceptance. Section 7 records the subsequent non-author review and independent global final verdict; author reruns do not replace those judgments.

## 7. f951 independent global final review: design accepted

On 2026-10-13 independent global final review accepted technical baseline `f9512f05f86311ea685d5e8d213784ac0d416324`: **ACCEPT / accepted-design-not-implemented**, with **zero** open design blockers. The same non-author review chain combined the completed cdb global review, f456 repair review and final f951 repair review to judge the whole design, rather than only the nine-file delta. The complete verdict and actual evidence archive were collected, read and verified.

| Object | Final design disposition |
| --- | --- |
| TYPST-GLOBAL-CDB-P2-01: year/month precision in periods and related fields | Original independent CLOSED retained |
| TYPST-GLOBAL-CDB-P2-02: preset/custom categories for general person relationships | Original independent CLOSED retained |
| Chinese defaults: REQUIREMENTS §5.5, EDITOR §8.6/CJK01–08 | Design acceptance retained; product scenarios unrun |
| TYPST-GLOBAL-F456-P2-01: open ranges bypassing cross-input temporal-domain checks | Independently design CLOSED at fixed f951 |
| New P0/P1/P2 in this review | None found |

The final closure rests on a call-level common temporal basis: qualify all authorized inputs, finite endpoints and required providers before interpreting infinity or three-valued conjunction. Eligibility does not require total endpoint ordering; same-calendar/version year/month/full-day open ranges remain usable, as do supported cross-calendar full days. Civil/instant mixtures and coarse cross-calendar/version inputs cannot be washed into success by another comparison's true/false. During, duration, exact recurrence and normal outer Query short-circuit boundaries remain intact, without new author parameters or another fact authority.

Independent Sol review gave fixed f951 a bounded PASS, closed the finding and found no new issue; its complete report was read. The original independent Pro then delivered the global acceptance above. Neither reviewer authored the repair under review; author reruns retain their original attribution.

| Actual verification | Result and boundary |
| --- | --- |
| Pro: configuration/path/design ABI, finite model, original independent script with unchanged expectations, new root-cause family checks | 31/31, 96/96, 70/70 and 160/160 pass; the last set includes argument swaps and boundary combinations, not 160 product features |
| Independent Sol: configuration, model, original counterexamples and additional boundaries at the same fixed baseline | Separate sets of 31, 96, 70, 8 and 15 pass; not counted as Pro's own execution |
| Coordinator evidence verification | All 162 source quotations match fixed Git text; Pro's original 160-case script passes 160/160 against an independent exact-byte model/schema copy. Quotation identity establishes location and material identity only |
| Fixed f951 documentation CI | [Design documents 38339403259](https://github.com/ZHYX91/weftext/actions/runs/38339403259) succeeds: 419 documents, inputs, D3 and three specified checker test suites; S49/inputs protection retained |
| Fixed f951 source CI | [38339403265](https://github.com/ZHYX91/weftext/actions/runs/38339403265): six non-UI jobs succeed; UI fails at the existing npm audit report with three high dependency findings, with later UI steps unrun. No all-green source-CI claim |

Global acceptance covers the coordinated SOURCE/CORE/EDITOR/FEATURES and INTEGRATION: native source and centralized types, rich business data, five views, title/subtitle/per-heading run-in and Word semantics, files/recovery/sync/authorization, Query consistency, flow through the finally consumed Html tree and Paged printing, links, independent JSON annotations, templates/conversion/intranet delivery, Agent/cost boundaries and Chinese defaults. The original 21 harms were assessed against the current design through actual independent review, without mechanical transfer of historical CLOSED states or a claim to full reading of every historical protocol.

Acceptance means a design sufficient to guide refactoring. Production Rust providers/CEL, Core transactions and platform failure guarantees, Server, production Html origins and inverse editing, IME/AT, CJK font delivery, Word/WPS/LibreOffice, XLSX round trips, intranet revocation and performance still require implementation and testing. Unchanged 23 native compilation cases and three probes were not rerun in this round; their original evidence scopes remain. Follow INTEGRATION's vertical slices to finalize actual interfaces, persistent formats, backend capabilities and environments. These product responsibilities are not additional open findings in this design review.

This section and concurrent entry updates only record the final disposition and current status; they do not change accepted algorithms, schemas, models or acceptance oracles. Fixed f951 is the technical review baseline. Later status-record commits do not claim a new full technical review. The PR remains Draft, unmerged and unreleased, with product implementation not started.
