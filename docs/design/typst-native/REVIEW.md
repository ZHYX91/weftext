---
source_language: zh-CN
translation_of: REVIEW.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW.zh-CN.md)

# Review and evidence for the Typst revision

Status: **the following records bounded independent Sol acceptance of the prior Typst revision; current requirements and technical reassessment are not accepted**. Prior input: `af6737a057c7bb69763dfaad0a57da4652e085f3`; accepted semantics: `c6b9199de54178891ec2e9ff91616ff9fb0d0b59`; status-record commit: `97eee15fc9a9a4dfe73feae95e22cc6389e443e4`. The new round distinguishes requirements from replaceable mechanisms under [REQUIREMENTS](REQUIREMENTS.md). Prior closure does not establish that retaining those mechanisms remains the best solution. Tables retain evidence at their original fixed scope without transferring acceptance or product passes.

## 1. Original-21 dispositions

All IDs have prefix `A2-GLOBAL-6B13-`. The 21 bounded CLOSED findings remain historical facts for their fixed predecessor. The Typst revision retains substantive obligations, replaces language-specific mechanisms and rechecks affected paths.

| Original ID | Substantive concern | New owner | Candidate disposition and review oracle |
| --- | --- | --- | --- |
| P1-01 | Whole-copy identity and Query materialization | CORE §§3–5; FEATURES | Native source construction replaces H/Q framing; retain one map, two-pass validation, complete preview and one commit. |
| P1-02 | Presentation commit, sync and recovery | CORE; EDITOR | Run-in presentation uses native show source; retire the independent presentation-policy effect in the new candidate, retaining original-version recovery of genuine records. |
| P1-03 | Complete heading levels | EDITOR; FEATURES | Native title/heading replaces AsciiDoc book/part0; retain deep headings and all direct consumers without a six-level clamp. |
| P1-04 | Complete projection and unaddressable heading queries | CORE; EDITOR; FEATURES | Separate native evaluation occurrences from writable addresses; legal headings without locators still enter Query. |
| P1-05 | Explicit Trash search | FEATURES; CORE | Retain authorized read-only qualification, coverage and fresh reads; no promotion into strong writable collections. |
| P1-06 | Saved-search contribution binding | FEATURES; SOURCE | Native values carry exact contribution ID/version/path; reopening never substitutes the ambient latest version. |
| P2-01 | Definition-transfer effect consistency | CORE §4; FEATURES | Replace language slots while actual effects, requests and previews use one exact version domain. |
| P2-02 | Genuine history and commit faults | CORE §§4–5,8 | Retain same-P atomicity, frozen original-operation bytes and post-commit transport without invented migrations. |
| P2-03 | Exact nested Frontier fields | CORE §5 | Validate CommitDomain/Workspace inside actual heads; empty heads never erase outer qualification. |
| P2-04 | Complete terminology projection | INTEGRATION; replacements.json | Old terms retain their fixed generation; new named definitions/navigation do not pretend to rewrite the old lexicon. |
| P2-05 | Exact native numeric representation | SOURCE; CORE | Native int/float/decimal retain distinct domains; exact values and finite domains replace the Ruby Float oracle. |
| P2-06 | Complete legal tables | EDITOR §§2–4; FEATURES | Use full native table semantics; control eligibility does not decide legal-source reading, saving or output. |
| P2-07 | Both genuine old CP2 recovery arms | CORE §8 | Recover committed/restored with original closed fields/evidence, without inserting new document_format members. |
| P2-08 | Dual-trust bootstrap and continuation | CORE §§5,8; FEATURES | Retain both declarations/proofs, same-decision activation and fault responsibility; source metadata owns no trust. |
| P2-09 | Mixed declarations and signature domains | CORE §§5,8 | Validate each recorded declaration in its own signature domain and retain actual evidence slots. |
| P2-10 | Source obligations and exact patches | SOURCE; EDITOR; replacements.json | Explicitly remove complex contact facts while retaining comments, unknown members, unselected bytes, exact preimages and recovery. |
| P2-11 | Audio/video interval annotations | EDITOR §8; FEATURES | Retain exact timebase/ticks/resource-region issuance, storage and consumption rather than whole-resource references. |
| P2-12 | Actual root-source fields | CORE §§2,5–7; EDITOR | All consumers use one qualification/sourceObservation without nonexistent flattened aliases. |
| P2-13 | Annotation digests and hidden-target boundaries | EDITOR §8; INTEGRATION TI-08 | A complete Annotation may expose its stored digest without extra hidden-target reads or recomputation. |
| P2-14 | Complete symbolic initialization and sorting | CORE §§3–5; FEATURES | Retain actual plans and mandatory initialization; freeze slots, materialize with one map, then sort by actual canonical keys. |
| P2-15 | One total order for effects | CORE §4; FEATURES | New run-in source edits use source-effect ordering; no new independent presentation effects, and no historical reordering. |

## 2. Independent review status

After the four module authors produce specifications and challenge each other, non-authors separately inspect requirements/original-21, protocols/authorization, and language/interaction. Final verdicts bind a fixed commit; a changing checkout supports discussion, not a recorded pass. Repairs are rechecked against actual deltas and directly affected paths.

The first fixed candidate is `f404b914642048404d84846714552d1818a4196f`. The three independent scopes—requirements, protocols, and language/interaction—all required changes at that candidate. Duplicate findings are merged by repair object at the highest reported severity into these nine items. These identifiers belong only to this revision; historical findings are not renumbered.

| ID | Severity | Repair object and independent review requirement | Current status |
| --- | --- | --- | --- |
| TR-01 | P1 | New Query types reach parameters, defaults, arguments, definition references and actual CEL typed slots. | Independently CLOSED within this revision |
| TR-02 | P1 | Native equality encoding reaches distinct, grouping and result keys, preserving signed-zero and offset-independent instant equivalence. | Independently CLOSED within this revision |
| TR-03 | P2 | Native none expresses legitimate schema-null slots without widening ordinary-value or Optional closed sets. | Independently CLOSED within this revision |
| TR-04 | P2 | Schema evolution compares the binding derived from the predecessor snapshot, not objects with different shapes. | Independently CLOSED within this revision |
| TR-05 | P2 | Rich editing has actual empty-body, first-text, math insertion and list conversion entries bound to exact origins and one Undo. | Independently CLOSED within this revision |
| TR-06 | P2 | Authored links and compiled occurrences have complete distinct keys, producers and mappings, enabling complete offline-route coverage checks. | Independently CLOSED within this revision |
| TR-07 | P2 | Annotation entries retain mandatory complete saves and strict write protection without restricting ordinary Documents. | Independently CLOSED within this revision |
| TR-08 | P2 | The actual recurrence dependencyProof slot and ancestor consumers select the current protocol; genuine old records retain their decoder. | Independently CLOSED within this revision |
| TR-09 | P2 | Template parameters, source-to-subject bindings, resource/loss choices and definition-transfer effects have complete independently decodable types. | Independently CLOSED within this revision |

The last protocol deltas found and repaired two further items, retaining their original identifiers and finding candidates:

| ID | Severity | Repair object | Current status |
| --- | --- | --- | --- |
| PR618-01 | P2 | Complete request keys, unique results, post-materialization effect ordering and exact correspondence for repeated-source instances. | Independently CLOSED within this revision |
| PRD5F-01 | P2 | New/mapped/existing Node results consume actual variant-specific receipt fields; preview versus commit and identity lookup versus change evidence remain separate. | Independently CLOSED within this revision |

Four Sol module authors challenged each other, followed by three independent non-author reviewers and actual-delta rechecks. All 11 merged findings from this revision (2 P1, 9 P2) are independently closed. All three assigned scopes PASS, with no undisposed finding from this revision.

| Independent scope | Fixed candidate and actual verdict |
| --- | --- |
| Requirements, original 21 findings and selected research capabilities | Requirement delta PASS at `618ae46d1aa30e1af89b0864d5eee79cde958fbf`; repeated-instance requirement impact PASS at `d5f154453dc01da22b111291f0eb23a3625bbc0a`. Protocol-variant residuals were closed in the scope below. |
| Language, native values, rich editing and link occurrences | TR-01/02/03/05/06 and directly affected paths PASS at `618ae46d1aa30e1af89b0864d5eee79cde958fbf`; the final two deltas only changed template protocols and corresponding cases, not these paths. |
| Types, authorization, sync/recovery and template protocols | Initial protocol gaps closed at `618ae46d1aa30e1af89b0864d5eee79cde958fbf`; PR618-01 closed at `d5f154453dc01da22b111291f0eb23a3625bbc0a`; PRD5F-01 closed at `c6b9199de54178891ec2e9ff91616ff9fb0d0b59`, with final protocol scope PASS. |

This is bounded design acceptance for the Typst revision. Unchanged text retains earlier actual-reading evidence; the final two deltas checked only FEATURES §6, TF-24/25 and necessary original D3/Core consumers, without claiming repeated whole-repository reading. Unfinished original Pro global-review scope remains; no implementation or release acceptance is created.

## 3. Validation levels

| Level | Recording rule | What it establishes |
| --- | --- | --- |
| Documentation and fixed-input checks | Record commands/results after fixing the candidate | Bilingual text, links, source bytes and mechanical consistency |
| Official native examples | Record exact compiler, source and actual compile/evaluation results | The demonstrated author syntax and native values |
| Design counterexamples and cross-checks | Independent verdicts cite owners and cases | Normative connections and requirement dispositions within scope |
| Product acceptance | UNRUN | No complete editor, authorization, synchronization, fault or cross-platform evidence yet |

Actual mechanical results: `scripts/check_docs.py` passed 395 public Markdown files; `scripts/check_design_inputs.py` and Git diff checks passed; the fixed 49 snapshots and inputs were unchanged. Twenty per-file source-navigation groups and 104 design acceptance cases per language had no missing entries. Official Typst 0.15.1 compiled 18 documentation examples and successfully evaluated root metadata in 16 at `618ae46d1aa30e1af89b0864d5eee79cde958fbf`; the final two deltas did not change those examples. The initial 14 compilations/12 evaluations remain earlier evidence. Mechanical checks and native compilation did not replace independent semantic verdicts; all 104 product scenarios remain UNRUN.

The existing research probe's 32 Rust checks, six local body-edit compilations and small browser examples remain research evidence, not product acceptance for this design. Experimental HTML, rich-edit mapping and the production flowing renderer remain explicit implementation delivery conditions.

## 4. Completion boundary

This revision delivers coordinated native Typst design and independent Sol review. Fixed inputs remain unchanged; product implementation and Pro review do not restart automatically. All actual findings from this revision have been disposed of and independently checked, so this design revision is complete. This does not establish complete product delivery or all historical global-review scope.
