---
source_language: zh-CN
translation_of: REVIEW-ENTRY.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW-ENTRY.zh-CN.md)
# A2 D1-D6 author-candidate review entry — P2-01 R1/R2 handoff

Status: author candidate only; not independently accepted, implemented, merged, released, deployed, or globally frozen.

## 1. Fixed object and binding

Base branch: docs/asciidoc-annotation-final-design.
Fixed base SHA: 97f4734f82a760cb6716c8122b84494da2b61164.
Fixed historical input S: 7e18168dad3e6d120fce0dd607dc10fa7894e252.
Protected inputs blob: 787d03c31a55496f81ed03fd54a6fdfff50a2ad4.
Candidate branch: docs/a2-final-design-integration.
This narrow R1/R2 repair starts from the exact stopped D6 handoff head `01cc40b819df78fbe724f1c64c27284ad60fc6c8`.

The next reviewer must bind the exact final PR5 stop head. Do not follow the moving branch and do not extend any older fixed-SHA conclusion beyond its recorded scope.

## 2. Prior independent state

- fixed-446 independently CLOSED the bounded D1/D2 findings; those closures are retained.
- fixed-a62 independently CLOSED D3:P1-01 and D3:P1-02. The older fixed-446 OPEN state is historical only.
- The completed non-author report bound to fixed-a62 `a62aa7d4eaf56113cd1e9e32336ae8814f83589b` (review request `7b6e1f16-fa41-499c-b078-647f1942e94e`) returned P0=0/P1=0/P2=2 for D4/D5: normative semantics LIMITED PASS, audit package REVISE.
- The later independent report bound to fixed4282 `4282d416e6a1ea4a344a2647d9a2feb82e3ba15a` returned P0=0/P1=0/P2=2 for this audit package. It independently **CLOSED A2-D4D5:P2-02 only in the bounded six-real-D10-source qualification scope**. P2-01 remained OPEN with the two residuals now named **R1** (all 72/130 fixed97 source→actual-owner mappings) and **R2** (Mandatory 303–306 ownership split).
- This author repair addresses only P2-01 R1/R2 and marks them **author-resolved-pending-independent**. It does not reopen P2-02 and cannot self-close P2-01.

D6 author work stopped at `01cc40b819df78fbe724f1c64c27284ad60fc6c8` with five D6 findings author-resolved-pending-independent. Its separate fixed-SHA non-author review does not make D6 accepted and is not decided here.

## 3. P2-01 review target

Review the machine maps, not counts alone:
- D4 fixed-S text: 906/906 nonblank lines, exact source path/blob/line/text, one source-qualified obligation group per line.
- D5 fixed-S text: 170/170 nonblank lines under the same rule.
- Mandatory lines 1–924: 716/716 nonblank lines mapped to actual owners/targets; metadata/review rows are not mislabeled as business semantics.
- D4 catalog: 2,854/2,854 RFC 6901 JSON Pointers in seven non-overlapping subtree groups; the document root is the empty pointer `""`, while `"/"` denotes an empty-key member; 95 named records plus 7 global limits remain under the sole fixed-catalog normative value authority.
- fixed97 selected rows: 72 D4 and 130 D5 rows, each preserving actual English and Chinese source row plus current A2 target/disposition/owner/basis/oracle.

R1 reviewer requirement: inspect all 72 D4 and 130 D5 fixed97 rows, not examples or counts. D2 media/provenance must point to D2; Annotation to D3/D8; presentation policy to the D8 current holder; import/export to D9; format/ChangeRecord/SourceTransform/trust to D6; mixed execution responsibility to D6-CONTROL §21 + D10; direct D4/D5 rows remain at D4/D5 with the actual upstream proof producer named.

R2 reviewer requirement: Mandatory 303–304 must split recurrence/derived-occurrence/identity from source-binding/import/current-proof owners; 305 is the grouping heading, not a recurrence rule; 306 must keep external Calendar authority while provider token/etag/cursor/credentials/fetch state remains D10 control-plane state. The exact source path/blob/line/text and total 716/716 union must remain unchanged.

## 4. P2-02 bounded CLOSED state and regression target

P2-02 is already independently CLOSED at fixed4282 **only** for the bounded direct-source qualification below; this R1/R2 task must preserve it without claiming whole-D10/D6/global acceptance.

The maps directly qualify six actual D10 files by their real blobs:
- CONTROL-CONTRACT.md / .zh-CN.md
- CANDIDATE.md / .zh-CN.md
- UPSTREAM-AMENDMENTS.md / .zh-CN.md

Each is marked `direct_partial_not_full_D10` and maps its relevant Package/Contribution, namespace/anti-spoof, provider availability, schedule/temporal, connector/credential, external-effect and execution-custody clauses to the existing D4/D5 current clauses.

Raw D10 predecessor/original-owner references to Proof2/Key2/wire12/PAB3 remain verbatim and source-qualified. They are not rewritten. Fresh/current authority remains the real A2 successor family in D3-SCHEMAS §2/§6.1 plus D4/D5 §0; historical records keep their recorded decoder/bytes/pins/recovery.

## 5. Scope guard

This repair changes audit/source maps and review/progress entrypoints only. It does not change D1-D6 normative business semantics, product implementation, dependencies, checker, CI, source snapshots, or protected inputs. It creates no second catalog/Registry/current authority.

D7-D10 full A2 owner integration remains pending. Mandatory source 925–1141 remains pending. Fresh Pro/global review remains pending. Runtime, OS, GUI, real-replica, migration, activation, deployment, provider, crash-recovery and performance evidence are UNRUN.

### 5.1 Mandatory intake and current workflow qualification

Mandatory source §§1–14 remain scenario/pressure input, not self-activating authority. Historical `$council`, OpenCode-panel, controller/task and chat-routing instructions are retained as source-qualified workflow provenance; the current repository workflow is the single author branch, explicit source-to-current-clause dispositions, then a new non-author review bound to one exact stop SHA. A source conflict with an already-current owner still requires an explicit owner/reopen disposition; it is never silently rewritten here.

### 5.2 Freeze and final-acceptance qualification

The product obligations listed by Mandatory §10 remain real obligations at their actual owners; this audit repair does not convert them to historical exemptions. A bounded mapping repair may be independently accepted only after its exact-source/current-target audit closes with no open P0/P1 and no remaining finding in that bounded scope. Global A2 freeze still additionally requires D7–D10 full owner integration, Mandatory 925–1141, the complete applicable product/test obligations, and a fresh Pro/global non-author review. Documentation checks are evidence of repository consistency, not semantic acceptance.

### 5.3 Supersession and handoff qualification

Mandatory §§11–11.3 remain source evidence for supersession and product intent, while their old D3-era controller/session scheduling is historical workflow provenance rather than a current command. Current handoff uses the GitHub branch/PR state and the exact fixed review object recorded above. Fixed-a62/fixed4282 D4/D5 conclusions are not expanded to D6; D6 remains separately bound to stopped author head `01cc40b819df78fbe724f1c64c27284ad60fc6c8` and its five-finding exact-SHA review. No old council/session instruction resumes a prior interrupted batch.

Global A2 design is not accepted.
