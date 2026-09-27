---
source_language: zh-CN
translation_of: IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md
translation_status: synced
---

[简体中文](IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md)

# D10 Implementation Impact and Test Outline

revision: D10-r05-unified-control-contract-2026-09-28; status: candidate. This file describes future implementation obligations and evidence gates. It does not state that the repository currently implements Agents, automation, Connectors, MCP, standing approval, or a D10 runtime. It authorizes no product-code change; the current PR contains design material only.

## 1. Implementation slices and state owners

The target implementation keeps a narrow control plane with specialized executors and does not create one generic tool host containing all domain semantics.

```text
Desktop / CLI local                     Server / WebUI remote
        |                                      |
        v                                      v
  D10 Broker / Scheduler                D10 Broker / Scheduler
        |                                      |
        +---- Core-managed D10 control adapters+
        |              |
        |              +--> D6 Policy / ledger / author commit
        |              +--> D4 Registry validation/evolution
        |              +--> D7 Action prepare/effects
        |              +--> D8 edit proposal/confirmation
        |              +--> D9 conversion/export
        |
        +--> Agent runtime
        +--> Model Adapter
        +--> Tool Adapter / MCP Adapter
        +--> Connector
        +--> managed transport / secret injection
```

Broker does not read/write authority DB tables, interpret D2/D4 source, or accept free callbacks that write the workspace. Core-managed control adapters are the sole entrypoints for durable delegation, approval, ActivationBinding, occurrence claims, ApprovalUse, and author-submit eligibility.

Recommended implementation order:

1. S1: D10 strict value/control decoders, managed token tags, ActivationBinding/Capability Catalog, publisher/namespace trust.
2. S2: DelegationLease, ContextBundle, egress, SecretRef, audit spool, and atomic budget/cost reservation.
3. S3: Run/Automation Definition, occurrence claim, serial scheduler, cancellation/restart.
4. S4: Model/Tool/MCP adapters and runtime isolation; start with read/compute only and no external mutation.
5. S5: ExternalEffectIntent, send fence, idempotency/reconciliation, credential rotation.
6. S6: coordinated implementation of D6/D7 standing-approval branch in UPSTREAM-AMENDMENTS; unattended author commit stays unavailable before this.
7. S7: named read-only Connector profiles; writeback opens per profile only where an existing closed SourceBinding/OriginBinding adapter exists.
8. S8: Desktop/CLI/Server/WebUI surfaces, diagnostics, audit/export/retention; Mobile performs negative-capability conformance only.
9. S9: remove old prototypes/aliases/free JSON/tool callbacks and update public specifications; support claims wait for real implementation/platform evidence.

### 1.1 R05 control-contract implementation slice

[CONTROL-CONTRACT](CONTROL-CONTRACT.md) is normative R05 input rather than an implementation sketch. Implementation may not choose a different authorization, replay, or accounting semantic.

- Workspace self-control is authorized by proposed D6 Policy/2 `d10_control_self`; Workspace administration remains original `policy_admin`, with `registry_admin` additionally required for Registry activation. Deployment trust/account/secret/pricing/grant is owned by D10 DeploymentControlPolicy and cannot be inherited from Workspace admin or issuer admin.
- Workspace author-affecting mutation continues through the original D6 authority store, `PreparedIntent`, decision/receipt, and author commit point. `DeploymentControlDecision` stores only host-control outcomes; the host adapter cannot write author source.
- `ControlPrepareBinding/1` uses stable `(scope incarnation, principal, requestId)` plus complete canonical intent bytes. Recovery order is current visibility/authority → same-key comparison → saved-decision replay → only an undecided operation checks current expected revision/eligibility.
- Configuration revision and usageRevision are separate; control ID/incarnation is never reused. Retire/archive cannot delete records still pinned by planned, unknown, uncertain, evidence, or dedup state.
- Implement all four `ResourceUseGrant/1` variants separately: cost, secret, egress, external-effect. Grant renewal/revision never clears spent/held/attempt/use, and replacing a grant never clears an old reservation or actual-account liability.
- The old non-Field set of fixed-S profile/2 is frozen byte-for-byte in tests; only profile/3 adds `d10_control_self`. Upgrade, replay, replacement, continue, and failover cannot alter an existing family's profile.
- PackageManifest dependencies resolve inside the concrete dependent Contribution. Contribution availability is independent, so an unavailable connector cannot transitively disable a same-package schema/template/pack.

These are contract implementation obligations and do not claim that this author stage ran real Core/OS/provider implementation tests.

## 2. Data and storage impact

### 2.1 Authority Store

Managed D10 control records are needed without adding another author commit root. Logically they include at least:

- activation, trust, and catalog records;
- Delegation Lease, LeaseRunUse, and Standing Approval;
- Automation Definition, occurrence claim, Run/step, and terminal deduplication proof;
- ApprovalUse, approval-count reservation, and PlannedDecisionApproval;
- budget/cost accounts and reservations;
- ExternalEffectIntent, attempt, and reconciliation evidence;
- Audit Started, terminal links, and protected local-spool metadata;
- SecretRef/account-generation metadata, never secret bytes.

These records are managed with Workspace/authority identity, fence, current principal, and version/CAS. SQL tables, indexes, and GC are implementation choices, but they may not alter candidate versioning, atomicity, replay, masking, or retention semantics.

The D6 author ledger remains the unique Workspace+OperationId author-decision namespace. D10 Run, Approval, LeaseRunUse, and ExternalEffect records may not generate a second "author committed" fact during recovery.

ApprovalUse count history durably records the one-way states unreserved, reserved, consumed, and released_terminal. Only the same abort transaction that records authoritative D6 terminal_failed may produce released_terminal. Run cancellation, TTL, temporary revocation, and Lease expiry cannot release the reservation through GC.

Occurrence claim and terminal deduplication proof may be compacted only while retaining proof that one AutomationOccurrenceKey already maps to the original Run/terminal outcome. While a definition revision can still be rescanned or recovered, deletion of detailed rows cannot make the occurrence executable again.

PlannedDecisionApproval is one-shot interactive authorization for the exact original planned request and binds the saved preview semantic record. A transport token is only a short-lived delivery handle and never becomes approval authority.
### 2.2 Secret Store

Desktop/CLI uses an appropriate OS secret store; Server uses a protected deployment secret store/KMS equivalent. Implementation must prove:

- secret bytes are absent from author DB, ordinary logs, crash-dump fixtures, transcript, and export;
- SecretRef cannot replay under another account/contribution/audience;
- rotation generation, disable/revoke, restart, backup/restore behavior is verifiable;
- multiple Server frontends do not expose raw secrets to browsers.

If a claimed platform cannot provide an adequate secret store, contributions depending on secrets are unavailable; plaintext config or inherited environment variables are not fallback storage.

### 2.3 Immutable assets

Package, manifest, definition, runtime, model, font, and other dependencies are stored as immutable assets under exact digest/version. Activation performs complete validation and active assets cannot be overwritten in place under the same identity. GC may remove only assets no longer referenced by current/historical bindings, planned decisions, unknown effects, or audit/recovery pins.

## 3. Activation, Registry, and package implementation obligations

The Package verifier implements SHA-256 catalog binding, Ed25519 package signature, PublisherIdentity, NamespaceClaim, dependency closure, and platform constraints. Key storage, revocation-distribution mechanism, and admin UX may vary, but behavior is fixed:

- self-signing creates no namespace ownership;
- reserved namespaces cannot be claimed by third parties;
- key rotation preserves publisher continuity;
- revocation invalidates new trusted context/execution;
- historical D4 semantic ledger, saved decisions, and author source remain;
- ActivationBinding changes as old-or-complete-new, never half-state;
- semantic rollback is a successor activation and never moves Registry history backward.

Candidate D4 Registry must still pass original `RegistrySnapshot/1`, `RegistryBinding/1`, evolution proof, and catalog loader. D10 package metadata may not be inserted into the D4 closed snapshot for convenience.

### 3.1 Pack parent dependency and lifecycle tests

Implementation separates package installation, Contribution activation, D4 definition retention, and UI visibility. A domain Pack Contribution stores primary `parentDomainId`, `extensionPointId`, `requiredVersionRange`, and the exact parent binding resolved at activation in the Catalog. The parent binding is covered by the Catalog digest. When the parent version changes, the old dependent binding becomes invalid and only a successor ActivationBinding can reactivate it.

At minimum test these combinations while preserving fixed D1 reason precedence:

| Condition | Contribution | Expected capability projection | schema/author facts |
| --- | --- | --- | --- |
| parent missing | inactive | `missing_component` when no higher-priority reason applies | history/raw source retained; typed interpretation follows D4 complete/unavailable |
| parent present but explicitly disabled | inactive | `not_configured` | same; definitions are not deleted |
| parent incompatible | inactive | `incompatible_version` | old history retained; old rules do not keep running |
| unsupported surface | inactive on that surface | `unsupported_surface` | portable facts follow that surface's Core/D4 capability |
| UI hidden only | activation unchanged | no new unavailable reason | RegistryBinding/author facts unchanged |
| parent ready/compatible | may activate | continue through all other D1/D6/D10 gates | current RegistryBinding interprets facts |

Tests cover both Pack families. A Calendar rule/data Pack produces no derived rule/View/cache or Connector behavior while the parent is inactive. An Organizations schema Pack whose accepted definitions remain completely proven by the current D4 binding may still interpret existing author facts, but retained schema cannot secretly expose domain-specific View/Action/Assign/Connector behavior. When definitions state is unavailable, raw source remains and typed operations fail closed.

GC/uninstall tests prove package config/source, accepted semantic ledger, tombstones/migrations, and immutable assets required for recovery are not deleted because UI/module is disabled. Tests also prove a parent version/binding change invalidates the old dependent Catalog binding rather than following ambient latest at runtime.

The supplemental S D4 reference catalog is now an implementation-test input: its 61 Fields, 7 Facets, 27 relation Fields, and real cross-Field constraints such as Calendar `union_variant_equal` participate in D7 Narrow Field Qualification. Tests include at least the positive `people/phone` construction, a relation-Field negative, a cross-field `calendar/range` or `calendar/recurrence` negative, and an unknown-constraint-constructor negative. No hard-coded "safe FieldId allowlist" may bypass the Registry-graph proof.

## 4. Agent, Tool, and MCP implementation obligations

Agent runtime holds only Run-local state, ContextBundle references, and the allowed tool catalog and does not cache ambient workspace authority. Context builder derives exact source/result pins from current Core-authorized input and rematches recipient before egress.

ToolValue decoders/encoders must prove exact semantics per type. Prohibited shortcuts include:

- JS double for arbitrary integer/decimal;
- one JSON null meaning Optional.none, missing member, and invalid;
- additionalProperties=true free maps;
- unbounded recursive schema;
- arbitrary path, URL fetch, shell command, secret/token as generic values;
- remote MCP annotation automatically choosing effect class.

MCP tests require hostile servers: descriptor drift, tool-name collision, schema changes, extra fields, huge recursive schema, fake readOnly, prompt injection, resource contents carrying tool instructions, truncated/duplicate responses, and over-budget results. A discovered change may only pending/reject/reset and may not hot-swap schema during a Run.

## 5. Runtime and OS sandbox

Every executable contribution support claim is bound to real sandbox evidence per platform. Minimum checks:

- no workspace/authority-store mount;
- no user home/browser profile/SSH agent/system clipboard;
- no arbitrary inherited environment secret;
- controlled child process tree with confirmed cleanup after cancel/crash;
- network disabled by default and managed transport only;
- read-only InputSlots with private output/temp;
- CPU, memory, disk, process-count, wall-time budgets;
- reject symlink/hardlink/device/reparse/path aliases;
- bounded redacted stdout/stderr;
- runtime crash does not alter author state.

Windows, macOS, and Linux are separately accepted; a container or sandbox name is not proof. Server architecture/platform support claims require equivalent evidence per line.

## 6. Automation and scheduler implementation obligations

Scheduler persists definition revision, finite schedule horizon, sourceOccurrenceKey, claim owner, Run identity, LeaseRunUse link, and actual skipped/started/terminal outcome. First-generation serial semantics require:

- one `AutomationOccurrenceKey/1` maps to at most one Run identity and one durable claim;
- a terminal occurrence restores its original Run/outcome after restart, rescan, disable→enable, or scheduler-cache rebuild and never creates a second Run;
- enable/disable does not change definitionRevision or erase claim/terminal proof;
- semantic definition changes create a new revision and explicit activation point;
- run_once chooses only the newest missed occurrence in a finite window;
- an occurrence claim may exist before Run admission, so queued/blocked Runs that have not entered protected execution consume no Lease `maxRuns`;
- immediately before the first protected step, a Core-managed Run-admission CAS validates current leaseId/leaseRevision, trusted time, ActivationBinding, budgets, and cumulative consumption over the leaseId lineage and atomically writes `LeaseRunUse/1`;
- after admission, failure, cancellation, or crash never refunds a run use; recovery of the same Run does not consume again;
- later protected steps of the same Run and recovery of its original planned request reuse a complete existing `LeaseRunUse/1` and do not compare remaining count again; even with `maxRuns=1` and cumulative use already 1, they reuse the original admission;
- reuse is not exemption from current gates: every step still checks current authorization, exact leaseRevision, trusted time, ActivationBinding, applicable approval, and budgets; revocation, expiry, revision change, or binding change still blocks;
- exhausted `maxRuns` returns D10 `delegation_exhausted`, retains the existing claim/blocked Run, and cannot be bypassed by creating another Run for the same occurrence;
- source/rule/authorization/ActivationBinding changes are revalidated before each new step;
- after trusted time passes Lease `notAfter`, new steps and final submission are refused whether or not a cleanup task ran;
- unprovable clock epoch/time continuity returns `state_unavailable` and pauses rather than treating unknown time as unexpired.

Deduplication history may be compacted into coverage proof/terminal summary only if it preserves verifiable proof that the key was already handled. GC policy cannot become a duplicate-execution protocol.

Tests must do more than mock a scheduler row. At minimum exercise:

1. dual processes or Server frontends claiming the same K;
2. restart, schedule rescan, disable→enable, and cache rebuild after K is terminal;
3. existing claim cancelled before first protected step, proving no `maxRuns` use;
4. model call billed and failed after admission, proving the run use is not refunded and the next K under `maxRuns=1` returns `delegation_exhausted`;
5. crash immediately after admission CAS, then recovery of the same Run without another consumption;
6. Lease expiry while Run is paused and cleanup never runs, with trusted time advancing and new context/model/tool/external steps plus final author submission refused;
7. lost clock continuity failing closed as `state_unavailable`, then actual expired/active adjudication after trusted time is restored;
8. source/rule-generation and definition-revision changes without rerunning old K under the successor revision;
9. with `maxRuns=1`, the second protected step of the same Run reuses the original LeaseRunUse when remaining=0, while a new Run is rejected with `delegation_exhausted`;
10. recovery of the same Run's original planned request at remaining=0 continues after proving LeaseRunUse continuity; missing or unprovable continuity returns `state_unavailable` and never re-consumes.
## 7. Standing Approval coordinated implementation

UPSTREAM-AMENDMENTS is a prerequisite for this slice. The feature may not be shipped secretly in Broker before the amendment is jointly accepted.

Core owns the StandingApprovalEnvelope validator, ApprovalUse builder, read-only planned-preview recovery validation, PlannedDecisionApproval builder, and atomic approval-count reserve/consume/released_terminal logic. Broker may only request mechanical approval or opening of the original planned preview and cannot submit an approval verdict, MutationFootprint, or author result.

Testing must traverse the real D7 `set_field_member`, FieldSelection/Narrow Field Qualification, PreparedActionBinding/preview, and D6 request/plan path. A simplified JSON fixture proves only its own decoder and not the composition semantics.

Error-owner tests cover the D10→D6 boundary:

- approval unavailable before D6 entry: D10 `approval_required|approval_expired`;
- R1 passes initial approval, R2 consumes the final use, and R1 loses only approval inside D6: new D6 `approval_unavailable/preflight`, with no ledger decision for unseen;
- approval revoked after planned: the same D6 error with ledger remaining planned;
- D6 permission loss remains original `not_visible`;
- dependency/semantic/budget conflicts remain original owner codes and cannot be masked by approval error.

Planned-preview recovery uses the real PreparedActionBinding/2, preview semantic record, and pins retained by a planned decision to open a new finite epoch. Tests prove that an expired old preview token can still deliver original semantics under current audience/ObservationScope/permission/continuity and that current Query/definition/target drift cannot change recovery contents. Expiry of the recovery token changes no planned record and revives no old token.

Core race/recovery tests include at least:

1. exactly-one Entry becomes two after prepare;
2. source A→B→A;
3. final approval use raced by two Runs;
4. approval revoke/expiry raced with D6 planning CAS;
5. current D6 write permission revoke/regrant after planning;
6. commit succeeds but receipt is lost;
7. changed-member and raw-no-op branches: no-op has empty MutationFootprint, field_change, and sourceVersions while still validating target/type/value/permission/dependencies;
8. crash at approval reserved, D6 planned, and author commit points;
9. stale preview/cursor/EffectBytes delivery epoch;
10. new client with no old copy completely reads original preview through planned recovery and creates PlannedDecisionApproval;
11. PlannedDecisionApproval cannot revive a deterministic dependency conflict;
12. mutant smuggles note, provenance, another member, Facet, or body changes;
13. authoritative terminal_failed atomically changes approval `reserved→released_terminal`; replay does not release twice;
14. cancellation, TTL, temporary revocation, and Lease expiry never create released_terminal.

An automatic submission completes only when the real D6 commit transaction stores both author result and approval consumed. Approval release on authoritative terminal_failed must likewise be proven in the same original abort transaction.
## 8. External effect and Connector implementation obligations

Each writable External Service requires a named adapter profile declaring:

- exact request payload/value types;
- target/account binding;
- current read/write authorization classes;
- secret use;
- accepted success proof;
- failed_no_effect proof;
- idempotency-key generation, server scope, and retention/window;
- safe reconciliation query;
- cost model;
- cancellation/timeout semantics.

When any item is missing, a read-only connector may remain while unattended mutation is unavailable. HTTP method names or 2xx status cannot be generalized into success/idempotency contracts for every service.

The send fence needs fault injection before durable intent, after intent before send, during write syscall/HTTP send, after remote acceptance before response, and after response before terminal audit. Unprovable outcomes are always outcome_unknown.

Connector sync changing SourceBinding/OriginBinding/watermark needs a separate owner-stage closed adapter. Ordinary ExternalEffectIntent or single_field_member approval cannot directly write those control fields.

## 9. Budget and cost implementation obligations

Cost implementation follows CONTROL-CONTRACT §6 and §9–§10 exactly. CostReservation state is `reserved→settled(actual)|released|uncertain` plus `uncertain→settled(actual)|released`. Only settled/released are terminal. uncertain retains the complete upper bound and may later recover from evidence.

Implementation stores reservationId, attemptId, actual cost account/grant, currency, pricing binding, upper bound, state/revision, and evidence attribution. The caller has no targetState or hand-entered actual amount. Final-bill and never-started conclusions come only from a trusted EvidenceTicket adapter.

Reconciliation CASes the expected reservation revision in the same authority transaction and atomically stores `CostSettlementDecision/1`, reservation state/revision, grant/account held/spent/available projections, evidence, and audit linkage. Exact decision replay never returns capacity twice; two conflicting reconcilers have at most one winner. Wrong attempt/account/currency, non-final evidence, or an aggregate bill that cannot be uniquely attributed leaves uncertain with the full bound.

A reliable zero bill after send is always `settled(0)`; only never-started proof is `released`. TTL, restart, author terminal_failed, Run terminal state, administrator-entered zero without evidence, and effect idempotency never release cost. actual above upper bound follows the existing overcharge-anomaly/freeze path rather than raising the ceiling through normal settlement.

ResourceUseGrant configuration and cumulative usage have separate revisions. Renewal/revision retains spent/held/attempts; a lowered limit cannot be below existing consumed+held use. Changing grantee/account/resource kind/currency creates a new grantId, while an old reservation continues to settle against the old grant and actual account.

Real billing evidence must cover at least: reserve100→send→crash→uncertain→final bill20→settled(20), returning only80; sent+final0→settled(0); never sent→released; concurrent reconcilers; response loss after commit and replay; wrong attempt/account/currency; non-final or unsplittable aggregate evidence; administrator zero without evidence; and overcharge. Provider tests not actually run remain pending.

## 10. Audit, retention, and export

The local/Server protected audit spool durably records started before protected operations; remote collector may be asynchronous. Implementation needs:

- append/sequence/integrity or equivalent tamper resistance;
- restricted linkage to Workspace/principal/Run/step/effect/author request;
- secret redaction via structural allowlists, not log-postprocessing guesses;
- reserve for cancellation/revocation/emergency stop;
- distinct states for storage full, permission failure, corruption, collector offline;
- retention pins for planned/unknown/uncertain evidence;
- export under current `audit` permission without hidden source/secret leakage.

The candidate does not freeze a numeric retention duration; deployments expose a finite policy and may not delete earlier than the minimum survival required by recovery, billing, and security evidence.

## 11. Capability, error ownership, and conformance

D1 capability reasons and fixed precedence are tested against real deployment combinations, especially policy_denied overlapping missing/offline components, offline versus incompatible version, temporarily unavailable mutually exclusive with offline, and Mobile unsupported_surface ahead of later install state.

Error ownership is mechanically verified rather than collapsed into a generic AgentError:

| Location | Owner | Required result |
| --- | --- | --- |
| D10 pre-submit with no usable Standing Approval | D10 | `approval_required` or `approval_expired` |
| Run admission has exhausted `maxRuns` | D10 | `delegation_exhausted`; D6 is not entered |
| Lease timeout | D10 | `delegation_expired`; `state_unavailable` when trusted time is unprovable |
| After D6 entry, author permission/ObservationScope fails | D6 | original `not_visible/preflight` |
| After D6 entry, only approval dependency loses a race | D6 coordinated extension | `approval_unavailable/preflight`; unseen has no ledger and planned stays planned |
| D6 dependency/semantic/budget failure | D6 | original code/disposition |
| Planned-preview recovery read fails | D7 transport | original `d7_effects_error` code and no author decision |
| Committed effects read | D7 transport | original `d7_effects_resolve/open` |

D10 control errors also verify the same `not_visible` response when a hidden object exists versus is missing until the caller has visibility. Only after the caller may read that caller-owned control record can expired, exhausted, or conflict detail be shown.

R05 defines no anonymous old D10 capability profile. The first jointly specified public unattended author-submit D6 closed enum directly contains `approval_unavailable` plus proposed `execution_stopped`; runtime capability is decided by the official D1 catalog and real availability gates. The D10 adapter may not rewrap a D6 code as approval_required, and existing Policy/bootstrap/saved-decision compatibility contracts remain.

Original D3/D7/D8/D9 errors likewise pass through exactly from their owner. Diagnostic UI may explain status through a separately authorized control read but cannot leak hidden data by changing the formal error wire.
### 11.1 R05 capability, stop, and public-contract gates

Candidate first-public D10 capability IDs are `automation.manage`, `workspace.extensions.manage`, `deployment.external.manage`, `automation.stop`, and `automation.author_submit`. Design acceptance or coordinated design activation freezes specification only. Implementation may report available only after the ID exists in the selected D1 contractMajor's official catalog and the real release/surface/policy/principal/component/configuration/version/reachability/health gates pass.

The first jointly specified public unattended author-submit D6 closed error set directly contains `approval_unavailable/preflight` and proposed `execution_stopped/preflight`. Tests do not construct an anonymous old capability profile; Policy/1/2, bootstrap profile/1/2, D1 bootstrap/contractMajor, and historical saved-decision replay all remain.

Emergency stop requires real race evidence at three boundaries: the same store transaction as Run admission, recheck inside the actual D6 final write-lock transaction, and an external send fence held through the first irreversible send handoff. A model that only checks stop then sleeps/commits/sends is insufficient. After stop, currently authorized authoritative abort, cost settlement, evidence/audit retention, and reference-safe cleanup must still work.

## 12. Cross-surface implementation matrix

| Capability | Desktop local | CLI local | Server | WebUI | Mobile |
| --- | --- | --- | --- | --- | --- |
| D10 Broker/control | local | same local capability family | hosted | Server client only | unavailable |
| Agent runtime | when capability available | same local capability family | when capability available | initiate/manage Server only | unavailable |
| Automation scheduler | one local scheduler | manages same scheduler | one authority-domain scheduler | Server client only | unavailable |
| Connector/model/tool executor | local sandbox/transport | same local capability family | Server sandbox/transport | does not run directly | unavailable |
| Secret management | OS secret store | same local store | Server secret store | managed only through Server | unavailable |
| Standing-approval author submit | after amendment activation | same Core | after amendment activation | only through Server | unavailable |
| Ordinary committed facts | original Core | original Core | original Core | Server Core | original Mobile Core/Server |

"When capability available" still requires D1 release/platform evidence; design existence does not create a Supported claim.

## 13. Controlled naming mapping and implementation replacement

The per-concept structured mapping in TERMINOLOGY §13 is the current D10 naming authority rather than a future test plan. Implementation, CLI/API/schema/manifest, UI labels, and locale resources consume only mappings already recorded there. "No public IPC", "no direct CLI", and "no historical alias" are controlled decisions and cannot be replaced by implementation-defined names.

A later implementation atomically replaces any old free tool callback, arbitrary JSON argument, UI-self-asserted approval, inherited process environment secret, extension/name-driven automatic tool dispatch, Agent direct file write, cursor/provider state mixed into author source, or prototype that violates the Terminology collision table. If an old prototype was unpublished, no serde aliases, fallback parsers, or dual-read/dual-write remain.

Every D10 concept must trace from stable concept ID to Chinese/English formal names, owner, wire/API/manifest/schema status, code type/function/variable/namespace, CLI/UI/locale, short/alias rules, examples/counterexamples, and first-freeze/migration target. Inherited D1-D9 names only reference the original owner; implementation cannot register new D10 aliases for `Registry`, `OriginBinding`, D6 permission, D7 Action, D8 Draft, or D9 Provider.

B10-01 implementation negative gate: Adopt code paths use only the `adopt_*` convention and associated binding values use the existing D3 `OriginBinding` / `origin_binding`. Controlled positive source, API/schema, fixtures, and terminology registry contain no `adoption_binding`, with no scanner exemption.

TERMINOLOGY §14 and CONTROL-CONTRACT additionally freeze R05 control records, five capability IDs, and four first-party module/package/schema mappings. Candidate code symbols/namespaces and locale keys are unimplemented mappings only; CI string presence is not implementation evidence. PackageId, D4 SemanticNamespaceId, D4 namespace ownerId, FacetId, and module ContributionId remain owner-typed and cannot be merged merely because strings match.
## 14. Test and evidence layers

| Layer | Can prove | Cannot prove |
| --- | --- | --- |
| D10 static/document checks | bilingual structure, controlled names, scenario/reference completeness | runtime correctness, security isolation, real protocols |
| Bounded models | specified state-machine, reservation, claim, race counterexamples | complete Core, OS, provider, arbitrary scale |
| real Core conformance | actual D3–D8 decoder/permission/plan/commit/replay composition | OS sandbox, external-provider behavior |
| durable fault model | SQLite/fence/crash/restart/atomic reservation | real cloud/network-service guarantees |
| OS sandbox evidence | file/network/process isolation for named build/platform | other platforms/versions |
| protocol-service evidence | real idempotency/schema/billing for named MCP/model/connector | other providers/future versions |
| UI/device evidence | Desktop/WebUI/CLI interaction, accessibility, status display | Core internal correctness itself |
| release evidence | current package/platform/profile may be claimed Supported | uncovered combinations |

The author stage of this candidate ran no new D10 bounded state-machine model, so there is no D10 model pass count to report. Existing CI is reported after commit only as actually observed and cannot turn green documentation checks into product conformance.

## 15. Minimum hostile/race corpus to implement

1. prompt injection attempting to expand read, egress, secret, tool, or budget;
2. MCP descriptor/schema drift, fake readOnly, huge output;
3. standing approval over two same-value Entries, with no automatic first;
4. approval-count N=1 dual race with at most one reserved;
5. R1 approval valid before D6, R2 consumes the use, and R1 losing only approval inside D6 receives `approval_unavailable/preflight`;
6. approval revoked after planned leaves the decision planned rather than semantic rejection/terminal;
7. old preview token expires and a new client completely reads original semantics through planned-preview recovery;
8. current Query/definition drifts during recovery but delivery remains the saved original preview;
9. PlannedDecisionApproval cannot revive deterministic dependency conflict;
10. authoritative terminal_failed atomically changes `reserved→released_terminal` and replay does not release twice;
11. raw no-op automatic path has empty MutationFootprint/field_change/sourceVersions while original Action/target/permission/dependencies still validate and commit consumes one approval;
12. cost N=1 dual race;
13. cost cancellation before send is `released`, actual sent zero bill is `settled(0)`, unknown cost is `uncertain`;
14. revocation raced with context delivery, author planning, and external send;
15. crash at occurrence claim, LeaseRunUse admission, ApprovalUse reserved, D6 planned, and author commit;
16. `maxRuns=1`: R1 is admitted, a billed model call fails, run use is not refunded, and R2/new K receives `delegation_exhausted`;
17. after K is terminal, restart/rescan/disable-enable/cache rebuild never starts another Run;
18. a paused Run's Lease expires while cleanup never runs, and trusted-time advancement blocks new steps/final submission;
19. lost clock continuity returns `state_unavailable`, then actual expired/active adjudication after trusted time is restored;
20. external unknown with active/expired idempotency window;
21. credential rotation plus unknown mutation;
22. local audit failure versus collector offline;
23. package activation crash, failed upgrade, successor rollback;
24. D4 three-generation semantic-revival attack;
25. cancelled Run with prior author committed/external succeeded;
26. dirty D8 Draft plus valid background author commit;
27. Mobile upload/Agent/approval negative capability;
28. hidden object exists/missing non-disclosure;
29. provider billing uncertain/overcharge;
30. package disable with author raw unknown namespace retained;
31. D4 reference-catalog positive narrow proof for `people/phone` plus relation/cross-Field/unknown-constructor negatives;
32. D3 lexicon positive mapping: `adopt_*` uses `OriginBinding` / `origin_binding`; negative controlled source/API/schema/fixtures/terminology registry contain no `adoption_binding`, with no scanner exemption.
33. a self-service P with `d10_control_self`, actual narrow Field authority, and a deployment cost grant creates a finite Automation; missing any qualification rejects and Field authority never becomes deployment-account administration;
34. stable-key r5 succeeds/response is lost, r6 later updates, and original retry replays saved r5 result; same key with different body/expected target returns `control_conflict`;
35. grant renewal/revision/new grant races an old `uncertain` reservation and proves spent/held/attempt/account liability never resets;
36. emergency stop races Run admission, D6 final commit, and external send, while settlement/authoritative abort/evidence cleanup remains possible after stop;
37. profile/2 family does not gain `d10_control_self` on upgrade; profile/3 affects only new families after explicit issuer update;
38. per-Contribution dependency: an unavailable connector does not disable same-package schema/template/pack;
39. Calendar/Library/People/Organizations D10 PackageId→module→schema mappings remain type-distinct from D4 namespace owner/Facet and reject a same-name third-party spoof;
40. design accepted while release/surface/policy/version/health gate fails still produces the real D1 unavailable result.

Each case includes positive and mutant/negative paths and is not satisfied by string-log comparison. Any case not actually executed remains pending in the evidence table.
## 16. Completion gate

The author implementation plan is ready for independent review only when:

- CANDIDATE, CONTROL-CONTRACT, TERMINOLOGY, SCENARIO-DISPOSITIONS, UPSTREAM-AMENDMENTS, and this file agree;
- TERMINOLOGY §13–§14 completes all 13 Intake §8.5.1 mapping fields for every current D10 controlled concept, inherited names reference their original owner, and no TODO/"implementation decides" placeholder remains;
- current author input coverage remains 49/49; historical U 48/48 records keep their historical context;
- D6/D7 amendment is clearly an unactivated proposal;
- unsupported/deferred is never written as available;
- automatic author commit remains limited to single_field_member profile;
- External effect unknown, cost uncertain, audit failure, and cancel/planned recovery each have one normative result;
- D1 surface/reason, D3 identity, D4 Registry, D8 confirmation, and D9 worker/publication are not silently changed;
- every actually run evidence item is reported at its exact layer and pending items are not labeled pass.

These are candidate-completeness conditions, not an independent Gate verdict.
