---
source_language: zh-CN
translation_of: SCENARIO-DISPOSITIONS.zh-CN.md
translation_status: synced
---

[简体中文](SCENARIO-DISPOSITIONS.zh-CN.md)

# D10 Scenario Dispositions

revision: D10-r01-candidate-2026-09-25; status: candidate. This file maps D10 routing in the fixed upstream inputs, mandatory TASK failures, and new competitive boundaries from this candidate to executable dispositions. A disposition describes how the D10 candidate handles a scenario and is not a claim that a test has passed.

The execution-mode column has only four values: automatic means the closed rules in this document permit progress without per-operation human confirmation; interactive means preparation is allowed but per-operation confirmation is required; unsupported means explicitly unavailable in this generation; deferred means a named future owner must freeze a contract before availability.

## 1. Upstream mandatory routing and domain boundaries

| ID | Source location | Scenario/risk | disposition | Execution mode | D10 candidate landing | Validation obligation |
| --- | --- | --- | --- | --- | --- | --- |
| D10-U01 | D1 §2–§4, D1-I01/I06 | Agent, automation, or connector writes the workspace directly or concurrently with Core | reject | unsupported | Core remains sole author commit point; executor has no author store/workspace mount | Dependency graph, file/DB-handle negatives, dual-writer/fence tests |
| D10-U02 | D1 §4.1/4.4 | Desktop and CLI start separate schedulers and duplicate one occurrence | revise | automatic | Shared local control domain and unique occurrence claim; CLI connects to the same capability family | Dual-process race, crash takeover, one active Run per same key |
| D10-U03 | D1 §4.2/4.3 | WebUI directly runs model/connector or browser holds credentials | reject | unsupported | WebUI only through Server Broker; secrets stay in Server secret store | Browser-bundle/API scan and credential-leak negatives |
| D10-U04 | D1 §4.5, D8 Direction §7 | Mobile Agent, automation approval, conversion/connector management | reject | unsupported | Preserve D1 unsupported_surface; consume committed facts only | Five-surface capability fixture and no hidden Mobile approval entrypoint |
| D10-U05 | Mandatory Intake §2.4/§2.5 and D4 Calendar | Calendar recurrence/background reminder guesses device time or unbound rules | revise | automatic | Automation schedule uses verified D4 temporal rules, finite horizon/limit, exact source dependencies | tzdb/rule-generation change, DST/out-of-domain, restart counterexamples |
| D10-U06 | Mandatory Intake §2.6 Organizations | Organization provider ID/service name becomes Node identity | reject | unsupported | Connector ID remains control state; author relations remain D3/D4 identity | Same-name, rename, provider-ID reuse cannot merge Nodes |
| D10-U07 | Mandatory Intake §2.7–§2.8 Calendar packs | Pack install mutates author Fields or provider loss deletes facts | reject | unsupported | Activation changes Registry/Catalog availability only; raw source and semantic ledger survive | Author bytes unchanged after disable/uninstall/failed update |
| D10-U08 | Mandatory Intake §9 A2-08 | Connector writes cursor/credential into author source during sync | reject | unsupported | cursor/etag/credential are control state; writeback requires named closed adapter | No secret/cursor in author-source scan; atomic commit/control association |
| D10-U09 | Mandatory Intake §9 A2-09/A2-15 | Each surface guesses capability availability independently | reject | unsupported | D1 capability is sole product availability; D10 supplies downstream facts only | Same reason precedence across Desktop/CLI/Server/WebUI |
| D10-U10 | Mandatory Intake §9 A2-21 | schema, connector, runtime state merged into one provider state | reject | unsupported | Separate D4 Registry, D10 Catalog, runtime health | schema-available/runtime-unavailable and inverse combinations |
| D10-U11 | Mandatory Intake §9 A2-28 | Old derived results survive holiday/workday provider change | revise | automatic | rule/contribution generation is a dependency; invalidate/reset/recompute | Old result/cache/automation schedule cannot mix generations |
| D10-U12 | Mandatory Intake §9 A2-32 | ICS UID/RECURRENCE-ID automatically becomes D3 identity/upsert key | reject | unsupported | lookup/upsert only through frozen SourceBinding/OriginBinding adapter | Same UID in two ordinary imports remains fresh; unopened sync is unavailable |
| D10-U13 | Mandatory Intake §9 A2-33 | subscribe, sync, copy, and adopt collapse into one "sync" action | reject | unsupported | Each authority/effect uses its owner closed protocol | Requests/receipts/approvals for distinct intents are not interchangeable |
| D10-U14 | Mandatory Intake §9 A2-36 | package receives whole-package workspace permission | reject | unsupported | Permissions are per Contribution and five authorization dimensions remain separate | Same-package pure data available while network connector denied |
| D10-U15 | Mandatory Intake §9 A2-37 | Settings/Marketplace UI state becomes capability authority | reject | unsupported | UI is projection of Activation/Policy/Catalog only | Button visibility cannot change Core eligibility |
| D10-U16 | Mandatory Intake §9 A2-43 | localized module/manifest name changes canonical ID | reject | unsupported | canonical namespace/contribution ID is locale-independent | Chinese/English/RTL change leaves request bytes identical |
| D10-U17 | Mandatory Intake §9 A2-46 | Mobile upload automatically delegates Server conversion/Agent | reject | unsupported | Mobile saves ordinary attachment only; no conversion/Agent delegation/approval | Upload triggers no worker/model and returns unsupported_surface |
| D10-U18 | Mandatory Intake §9 A2-49 | unadmitted contribution dynamically injects SearchContribution/schema | reject | unsupported | D10 authenticates and generation-binds Contribution; D7 grammar unchanged | Runtime discovery creates pending contribution only, not current-query input |
| D10-U19 | Mandatory Intake §9 A2-50 | disabling a rule pack deletes/defaults author values | reject | unsupported | related derived capability becomes unavailable/reset, author source persists | disable/reenable leaves raw-source digest unchanged |
| D10-U20 | Mandatory Intake §9 A2-53/54 | D10 terms overwrite D1-D9 owned names or wire aliases drift | revise | automatic | Terminology file and controlled-name gate | Scan controlled positive surfaces while excluding historical prose |
| D10-U21 | Mandatory Intake §9 A2-56, D9 Templates | Node/Office Template acts as durable Agent script/arbitrary callback | reject | unsupported | Template remains D9 one-shot construction/rendering; ordinary text is not executed | Tool instructions in template text remain untrusted text |
| D10-U22 | D7 Algebra §6 SearchContribution | SearchContribution contains script/network or missing provider is silently skipped | reject | unsupported | Keep D7 pure-data closed descriptor; D10 only authenticates origin/generation | Missing contribution follows original D7 unavailability without grammar change |
| D10-U23 | D7 Execution §5–§6 | Query row/evidence becomes durable Agent write authorization | reject | unsupported | evidence/selector remains TTL/revision/dependency/current-auth bound | A→B→A, auth change, result reset invalidate old evidence |
| D10-U24 | D8 Main §2–§3/Interfaces §4–§5 | model output masquerades as human Draft and auto-clicks confirmation | reject | unsupported | D8 edit remains interactive; Automation creates no EditSession | dirty Draft, composition, stale preview, unknown receipt chain |
| D10-U25 | D9 Workers §1–§2 | D9 conversion worker becomes generic networked tool host | reject | unsupported | D9 worker remains specialized with no workspace/network by default; D10 executor separate | Worker sandbox cannot inherit D10 network capability |
| D10-U26 | D9 Export §4 | external publication receipt becomes Resource/author receipt | reject | unsupported | PublicationReceipt and D3/D6 receipt remain separate | publication success + Resource-create failure coexist |

## 2. TASK mandatory failure boundaries

| ID | Source location | Scenario/risk | disposition | Execution mode | D10 candidate landing | Validation obligation |
| --- | --- | --- | --- | --- | --- | --- |
| D10-F01 | TASK Constraints/Acceptance | malicious Document asks to upload other workspace data/secrets | accept | automatic | untrusted-data precedence, minimized ContextBundle, recipient-specific egress | Two hidden worlds expose same tool catalog; no secret/extra read |
| D10-F02 | TASK | prompt injection requests enabling a new MCP tool or larger budget | accept | unsupported | remote descriptor/output has no control authority; allowlist/budget are managed | Injection text causes zero control-record delta |
| D10-F03 | TASK | queued Run continues after grant/Delegation Lease expiry | accept | automatic | current lease check at every protected step; queued→blocked | Expiry linearization and restart does not extend time |
| D10-F04 | TASK | cached context/read/result keeps delivering after revocation | accept | automatic | current D6 generation/owner gate; preserve upstream cache rules | revocation races with chunk/page/tool-call |
| D10-F05 | TASK | duplicate claim of one schedule occurrence | accept | automatic | `AutomationOccurrenceKey/1` plus durable unique claim | two-process/restart/enable-disable race |
| D10-F06 | TASK | crash after an external request may have been sent | accept | automatic | durable ExternalEffectIntent + send fence; recover outcome_unknown | faults on both sides of durable/send boundary |
| D10-F07 | TASK | cancel races D6 planning | accept | automatic | planned is not aborted by Run cancel; original D6 recovery applies | cancel-before-plan, plan-before-cancel, commit-before-cancel |
| D10-F08 | TASK | cancel races external send | accept | automatic | only proven-unsent may cancel; submitting has three outcomes | fault injection around send fence |
| D10-F09 | TASK | credential rotation causes old request to auto-resend with new credential | reject | unsupported | old attempt binds actual secretGeneration; new credential only for authorized reconciliation | rotation + unknown request; mutation resend prohibited |
| D10-F10 | TASK | failed package upgrade half-activates Registry/Catalog | accept | automatic | staged validation + one ActivationBinding switch | crash at every activation step; old binding stays complete |
| D10-F11 | TASK | rollback after activation moves Registry pointer backward | reject | unsupported | rollback is successor activation; semantic ledger remains cumulative | three-generation mutation/revival attack |
| D10-F12 | TASK | remote audit collector offline stops all local work | revise | automatic | local durable spool is safety gate; remote collector can lag | collector offline, spool full, disk failure branches |
| D10-F13 | TASK | protected step executes after local durable audit failure | reject | unsupported | fail closed; safety stop has reserved capacity | audit failure before read/egress/secret/send/author submit |
| D10-F14 | TASK | same request gets different target/error on Desktop/CLI/Server/WebUI | accept | automatic | shared Core semantics; host authenticates/transports only | identical fixture across four surfaces; Mobile negative |
| D10-F15 | TASK | capability probe leaks missing package/account to denied principal | accept | automatic | D1 reason precedence and policy_denied masks deployment detail | overlapping-reason matrix |
| D10-F16 | TASK | external effect and Core write shown as one "atomic success" | reject | unsupported | independent outcomes/receipts, no composite author receipt | Core-success/external-unknown and inverse combination |
| D10-F17 | TASK | unknown external outcome retries with a new idempotency key | reject | unsupported | original EffectIntent/key; stop without reliable reconciliation | timeout, eventual consistency, idempotency-window expiry |
| D10-F18 | TASK | idempotent effect treated as free to retry | reject | unsupported | every attempt gets its own CostReservation | retry cost, delayed billing, unknown charge |
| D10-F19 | TASK | two Runs consume the final Standing Approval use | accept | automatic | ApprovalUse count reservation plus D6 planning CAS | N=1 race, at most one reservation/commit |
| D10-F20 | TASK | two Runs consume the final cost budget | accept | automatic | atomic multi-account CostReservation | Run/Lease/Automation/deployment account race |
| D10-F21 | TASK | billing unknown releases budget at TTL | reject | unsupported | CostReservation=uncertain holds the bound | crash/timeout/restart/retention |
| D10-F22 | TASK | provider exceeds accepted cost ceiling and system silently continues | revise | automatic | record anomaly, freeze capability, require admin handling | simulated overcharge without automatic ceiling raise |

## 3. Standing Approval and confirmation boundaries

| ID | Source location | Scenario/risk | disposition | Execution mode | D10 candidate landing | Validation obligation |
| --- | --- | --- | --- | --- | --- | --- |
| D10-A01 | D7 Execution §5–§6 + D10 Candidate §14–§15 | update a bool member when one Node/Field has exactly one Entry and value is allowed | revise | automatic | fresh `set_field_member` prepare + complete owner_fields preview + ApprovalUse | Real D7 Narrow Field/preview/D6 commit integration |
| D10-A02 | same | one Field has two Entries with identical values | accept | interactive | `require_exactly_one_entry` fails, never first | two-same-value occurrenceKey counterexample |
| D10-A03 | same | Field changes from one to two Entries before approval consumption | accept | interactive | fresh selection/source revision + dependency conflict | prepare/approve/commit race |
| D10-A04 | same | proposal also changes note/provenance/Facet/relation | reject | unsupported | footprint must be one member only | footprint mutant must reject |
| D10-A05 | same | append/remove/replace-whole-Entry attempts to use standing envelope | reject | interactive | unattended profile excludes it; ordinary interactive D7 remains | action-kind negative matrix |
| D10-A06 | D8 Interfaces §4–§5 | automated whole-document edit | accept | interactive | stays D8 Draft/preview/explicit confirmation | Agent creates no fake EditSession |
| D10-A07 | D3/D7 create/lifecycle | Agent creates/Trashes/restores/copies a Node automatically | accept | interactive | original D3/D7 preview + per-operation confirmation | fresh identity, closed modes, no standing-envelope consumption |
| D10-A08 | proposed D6/D7 amendment | approval expired after request committed, then user replays lost receipt | accept | automatic | saved decision returns original bytes under current auth; no second approval charge | lost receipt after approval expiry |
| D10-A09 | proposed D6/D7 amendment | request planned, then approval revoked | revise | interactive | remain planned/blocked; exact original plan can receive new one-shot authorization | no burn and no envelope substitution |
| D10-A10 | proposed D6/D7 amendment | raw no-op author decision | accept | automatic | committed no-op still consumes one successful-commit count | replay does not recount |
| D10-A11 | D8 Main §3 | valid background update and current dirty Draft share owner | accept | automatic | author commit stands; D8 Draft enters stale/conflict and is not overwritten | current Draft bytes/selection retained |
| D10-A12 | D8 Main §3 | Agent proposal auto-submits while composition is active | reject | interactive | D8 composition/preview gate remains | composition trace + delayed proposal |

## 4. MCP, tool, secret, and egress

| ID | Source location | Scenario/risk | disposition | Execution mode | D10 candidate landing | Validation obligation |
| --- | --- | --- | --- | --- | --- | --- |
| D10-T01 | D10 Candidate §10 | MCP server labels a delete tool readOnly | reject | interactive | remote annotation grants no effect class; local Contribution decides | hostile MCP descriptor fixture |
| D10-T02 | D10 Candidate §10 | MCP runtime discovers a new tool/schema | accept | deferred | enters pending admission only; current allowlist unchanged | discovery diff cannot change callable catalog |
| D10-T03 | D10 Candidate §10 | remote JSON rounds 2^63+1 through double | reject | unsupported | ToolValue exact integer; unsupported if adapter cannot prove precision | integer/decimal/null/unknown-member corpus |
| D10-T04 | D10 Candidate §10 | EntityRef/token passed as a generic tool capability | reject | unsupported | first ToolValue excludes Ref/Locator/control token | decoder negative |
| D10-T05 | D10 Candidate §10 | file tool receives arbitrary host path | reject | unsupported | InputSlot exact bytes only, no path capability | ../, symlink, home/workspace path negatives |
| D10-T06 | D6 §5 + D10 Candidate §16 | credential written into Document, prompt, transcript | reject | unsupported | SecretRef + trusted transport injection | secret canary across source/context/log/export |
| D10-T07 | D10 Candidate §9 | readable workspace context sent to a newly selected Model Provider | reject | interactive | recipient-specific egress; read does not imply egress | provider switch requires a new match |
| D10-T08 | D10 Candidate §9 | tool output instructs access to a hidden Field | reject | unsupported | tool output is untrusted data and cannot expand readScope | hidden-world noninterference |
| D10-T09 | D10 Candidate §11 | executable package inherits SSH agent/browser cookie/env secret | reject | unsupported | runtime lacks those host capabilities by default | real OS sandbox tests |
| D10-T10 | D9 Worker §2 + D10 Runtime | D9 conversion route borrows D10 transport networking | reject | unsupported | D9 no-network default remains; transport capability is not inherited | dependency/handle/network scan |

## 5. Connector, external effect, and recovery

| ID | Source location | Scenario/risk | disposition | Execution mode | D10 candidate landing | Validation obligation |
| --- | --- | --- | --- | --- | --- | --- |
| D10-E01 | D3 bindings + D10 Candidate §16 | connector cursor/etag becomes author Field/identity | reject | unsupported | control-state separation; only closed SourceBinding/OriginBinding adapter changes binding | author-source/control-store diff |
| D10-E02 | D10 Candidate §17 | crash after durable intent but before send | revise | automatic | conservatively outcome_unknown unless transport can prove unsent | fault injection at send fence |
| D10-E03 | same | send completed but success response lost | accept | automatic | reconcile same EffectIntent/key; do not resend anew | provider idempotency/reconciliation fixture |
| D10-E04 | same | provider supplies no idempotency/conditional proof | accept | interactive | unattended mutation capability unavailable; human handling required | capability matrix |
| D10-E05 | same | eventually consistent readback temporarily finds no target | accept | deferred | remain unknown; cannot infer failed_no_effect | delayed-visibility service |
| D10-E06 | same | compensating deletion of external object | revise | interactive | compensation is new ExternalEffectIntent/approval/cost | original success + compensation failure |
| D10-E07 | D9 Publication + D10 | external publication succeeds and Core Resource creation fails | accept | interactive | two independent results; no automatic deletion of user file | two-stage failure |
| D10-E08 | D10 Candidate §16 | read-only reconciliation after credential rotation | accept | automatic | new secret may read same account under current permission; no mutation resend | account-continuity proof |
| D10-E09 | same | old mutation automatically resent after credential rotation | reject | unsupported | old EffectIntent does not change generation for resend | rotation race |
| D10-E10 | D10 Candidate §17 | revoke races send gate | accept | automatic | revoke-first prevents send; send-first cannot claim no effect | linearization test |

## 6. Package, Registry, and trust

| ID | Source location | Scenario/risk | disposition | Execution mode | D10 candidate landing | Validation obligation |
| --- | --- | --- | --- | --- | --- | --- |
| D10-P01 | D4 §3.2/Registry handshake | two publishers claim the same namespace | accept | unsupported | NamespaceClaim unique owner; D4 owner conflict fails closed | claim conflict before inner parse |
| D10-P02 | D4 Registry evolution | same semantic ID rolls back to prior digest | reject | unsupported | semantic ledger cumulative; rollback is successor activation | three-generation mutation/revival |
| D10-P03 | D10 Candidate §7 | self-signed package gains namespace claim on first install | reject | unsupported | PublisherIdentity ≠ NamespaceClaim | trust-root/claim negative |
| D10-P04 | D10 Candidate §7 | valid publisher-key rotation | revise | automatic | old-key continuity + current trust policy + successor activation | old/new key chain |
| D10-P05 | D10 Candidate §6 | runtime binary changes while Registry semantics do not | accept | automatic | successor ActivationBinding/Catalog; RegistryBinding may remain | exact Catalog-digest generation |
| D10-P06 | D10 Candidate §6 | health outage creates semantic generation each time | reject | unsupported | runtime health separate from semantic activation | flapping health causes no Registry churn |
| D10-P07 | D10 Candidate §6 | uninstall cleans a saved unknown Field from author source | reject | unsupported | raw source/history remain; typed state unavailable | uninstall/reinstall roundtrip |
| D10-P08 | D10 Candidate §6 | crash during activation switch | accept | automatic | either old or complete new ActivationBinding, never half-state | crash at every staging/commit point |

## 7. Deferred boundaries with named owners

| ID | Source location | Scenario | disposition | Execution mode | Owner/reason | Requirement before reopening |
| --- | --- | --- | --- | --- | --- | --- |
| D10-D01 | Mandatory Intake Calendar holiday/anniversary | unfrozen holiday/anniversary provider algorithm | defer-with-owner | deferred | D4 temporal semantics + D10 provider admission | closed rule contribution, version/coverage, D7 consumer |
| D10-D02 | Mandatory Intake ICS sync | general bidirectional ICS subscription/upsert | defer-with-owner | deferred | D3 binding + D9 mapping + D10 connector | SourceBinding/OriginBinding, conflict/watermark, idempotency |
| D10-D03 | D10 Candidate §13 | general multi-step workflow DAG | defer-with-owner | deferred | future D10 | typed DAG, recovery, budget, approval composition |
| D10-D04 | D10 Candidate §14 | unattended create/delete/Facet/native-table/bulk | defer-with-owner | interactive | future D10 + owning D3/D7 adapters | mechanical approval envelope and D6 atomic use |
| D10-D05 | D10 Candidate §10 | auto-generated tools from arbitrary JSON Schema/OpenAPI | defer-with-owner | deferred | future D10 ToolValue profile | exact numeric/null/additionalProperties/recursion contract |
| D10-D06 | D10 Candidate §11 | browser-local MCP/process execution | defer-with-owner | unsupported | D1 surface boundary | new D1 surface/review |
| D10-D07 | D10 Candidate §22 | Mobile Agent/connector/approval | defer-with-owner | unsupported | joint D1 + D8 + D10 | reopen D1 plus real Mobile evidence |
| D10-D08 | TASK/A2 | A2 system-level acceptance | defer-with-owner | deferred | A2 | start only after D10 independent acceptance/activation |

## 8. Current evidence status

These rows are normative dispositions rather than execution results. The D10 author stage does not claim implementation of product Core, OS sandbox, real MCP/model/connector services, credential storage, cost system, or UI.

Mechanical document checks for this candidate prove only repository format, bilingual synchronization, and input-inventory consistency. If later commits actually run bounded D10 state/race models, they must include source, fixtures, and real outputs and state explicitly that they do not prove production implementation. The Implementation Impact document lists the complete later evidence gates.
