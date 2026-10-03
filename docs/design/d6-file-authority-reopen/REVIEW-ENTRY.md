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

1. **PL-IR-01 / P1 remains open.** After the same complete production version and content synchronize from replica A to B, D3/D6 must resolve how A's observation qualification in persistent Locators such as body_text is separated from, and legitimately reacquired under, B's current observation. Equal bare counters, equal digests, or re-signing old tokens do not prove qualification; unrelated ordinary operations must not be disabled because of this gap.
2. **Joint D3/D6/D7 review remains open.** Check original wire12, the sole preparation descriptor, single decision, installation recovery, PAB3, MinimumMapping3, native twelve arrays versus mandatory complete canonical effects, and actual D8 PB2 and D9 Construction2 consumption.
3. **Joint D6/D10 review remains open.** Check complete internal control images, range CAS, two approval-count steps, separation of dynamic external consent from immutable inputs, stop after installation, shared budgets and unknown-send responsibility, schedule-history continuity, and genuine old bootstrap-family capabilities. Actual portable Registry activation uses strict complete installation and the original ChangeId; only a protected catalog-selector-only change may use control_only. Other D10 consumers must be reconciled individually with these actual producers.
4. **A2 and final global acceptance remain open.** Self-contained D1–D10 integration, source coverage, terminology and bilingual consistency, and a fresh independent Pro global review remain required. Documentation structure checks cannot replace implementation, interoperability, performance, or historical model evidence.

## Revision and acceptance

Reports must identify the fixed commit, actual coverage, and gaps. Each finding needs a location, reproducible design scenario, consequence, and suggested remedy. Authors revise through a separate branch and PR; only one candidate writer is active at a time, and the final head returns to a reviewer who did not author it. Preserve decoding and recovery obligations for real historical decisions without presuming every historical prototype was deployed.

Final design freeze requires complete independent reading, zero open P0/P1 findings, an explicit disposition for every P2, and an explicit acceptance verdict. Design acceptance is not implementation acceptance and does not authorize automatic merge, release, or deployment.
