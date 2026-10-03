---
source_language: zh-CN
translation_of: REVIEW-ENTRY.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW-ENTRY.zh-CN.md)

# Joint candidate review entrypoint

Status: candidate, not accepted, activated, or implemented. This entry describes the design candidate collected on 2026-10-03; every review must record the actual Git commit read rather than treating a moving branch as a fixed revision.

## Sources and current bodies

Fixed source S is `7e18168dad3e6d120fce0dd607dc10fa7894e252`. The 49 sources in the [original inventory](../inputs.json), and the inventory itself, retain their original bytes. The [replacement manifest](replacements.json) maps 45 sources to 87 candidate files; this is candidate reading routing, not acceptance or permission to replace historical sources. Unmapped sources must still be read from the original inventory. Current D10 bodies are the nine bilingual pairs listed by its [task entrypoint](../d10/TASK.md).

A complete joint review reads each relevant source, complete candidate bodies, both languages, and machine catalogs. Diffs, summaries, author reports, file hashes, and passing checks alone cannot establish acceptance. All 25 D7 files and the 13 joint D6/D10 producer files have completed author delivery. Their producers now exist rather than being future interfaces, but whole-package independent acceptance remains pending. D7 integration subsequently standardized only the Chinese words for caller and contract.

## Existing evidence and open issues

Named revisions for D3 IR01–09, six D4 findings, two D5 findings, three D8 repairs, and eleven historical D10 findings have limited independent passing evidence, applicable only to the actual reviewed revision and scope. Full independent review of the sixteen D9 files concluded limited pass; their joint integration dependencies remain open. Historical R08's three P1 and eight P2 findings are not automatically globally closed by local repairs. Public historical statements that predate this delivery must be interpreted at their original revision and reconciled individually during integration.

Independent review of PR #3 head `cdc267f29e6359f0f42a4e766d4015a2f90b160d` completed with **REVISE (P0=0, P1=1, P2=0)**. Its sole finding is `PR3-PLIR-P1-01`: the PL-IR-01 production-address/current-Observation split and downstream consumers were otherwise accepted within that review scope, but the canonical token lacked a closed authenticated portable carrier tying exact binding bytes to the portable-trust root. The current author revision addresses only that finding and remains pending a fresh independent review of its new exact head; this paragraph is not a closure verdict.

1. **PL-IR-01 / P1 remains open; the current author candidate specifically repairs `PR3-PLIR-P1-01`.** `RevisionTokenBinding/2` remains the stable production address. The winning P seal now emits one closed `RevisionTokenSealAssociation/1` inside a domain-separated Ed25519 `RevisionTokenSealArtifact/1`, pins its exact canonical bytes as Portable Workspace Metadata, and records `RevisionTokenSealOutboxItem/1`; the verifier key comes from the authenticated historical portable-trust declaration for the producing CommitDomain/profile at that seal cut. Receiver admission verifies that artifact and CP3/history independently and cross-checks DecisionKey, ChangeId, complete SourceVersion and SourceStamp fields before storing token→version mapping, then separately establishes a fresh local `SourceObservation/1`. Same-stamp forged tokens, loser/aborted plans, sender assertions, equal digest/version, retry or I rebuild cannot fabricate authentication; only two distinct artifacts that both genuinely validate for the same exact V are an integrity contradiction. D6 PL01–PL18 are design requirements, not executed product tests. This author candidate must return to an independent reviewer before P1 can be closed.
2. **Joint D3/D6/D7 review remains open.** Check original wire12, the sole preparation descriptor, single decision, installation recovery, PAB3, MinimumMapping3, native twelve arrays versus mandatory complete canonical effects, and actual D8 PB2 and D9 Construction2 consumption.
3. **Joint D6/D10 review remains open.** Check complete internal control images, range CAS, two approval-count steps, separation of dynamic external consent from immutable inputs, stop after installation, shared budgets and unknown-send responsibility, schedule-history continuity, and genuine old bootstrap-family capabilities. Actual portable Registry activation uses strict complete installation and the original ChangeId; only a protected catalog-selector-only change may use control_only. Other D10 consumers must be reconciled individually with these actual producers.
4. **A2 and final global acceptance remain open.** Self-contained D1–D10 integration, source coverage, terminology and bilingual consistency, and a fresh independent Pro global review remain required. Documentation structure checks cannot replace implementation, interoperability, performance, or historical model evidence.

## Revision and acceptance

Reports must identify the fixed commit, actual coverage, and gaps. Each finding needs a location, reproducible design scenario, consequence, and suggested remedy. Authors revise through a separate branch and PR; only one candidate writer is active at a time, and the final head returns to a reviewer who did not author it. Preserve decoding and recovery obligations for real historical decisions without presuming every historical prototype was deployed.

Final design freeze requires complete independent reading, zero open P0/P1 findings, an explicit disposition for every P2, and an explicit acceptance verdict. Design acceptance is not implementation acceptance and does not authorize automatic merge, release, or deployment.
