---
source_language: zh-CN
translation_of: IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md
translation_status: synced
---

[简体中文](IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md)

# D10 Implementation Impact and Test Outline

revision: D10-r01-candidate-2026-09-25; status: candidate. This file describes future implementation obligations and evidence gates. It does not state that the repository currently implements Agents, automation, Connectors, MCP, standing approval, or a D10 runtime. It authorizes no product-code change; the current PR contains design material only.

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

## 2. Data and storage impact

### 2.1 Authority Store

Managed D10 control records are needed, without adding another author commit root. Logically they include at least:

- activation/trust/catalog records;
- Delegation Lease and Standing Approval;
- Automation Definition, occurrence claim, Run/step;
- ApprovalUse and approval-count reservation;
- budget/cost accounts and reservations;
- ExternalEffectIntent/attempt/reconciliation evidence;
- Audit Started/terminal links and protected local-spool metadata;
- SecretRef/account-generation metadata, never secret bytes.

These records are managed with Workspace/authority identity, fence, current principal, and version/CAS. SQL tables, indexes, and GC are implementation choices, but they may not alter candidate versioning, atomicity, replay, masking, or retention semantics.

The D6 author ledger remains the unique Workspace+OperationId author-decision namespace. D10 Run/Approval/ExternalEffect records may not generate a second "author committed" fact during recovery.

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

Scheduler persists definition revision, next finite schedule horizon, sourceOccurrenceKey, claim owner, Run link, and real skipped/started outcomes. First-generation serial semantics require:

- at most one active Run for one `AutomationOccurrenceKey/1`;
- enable/disable does not change definitionRevision;
- semantic definition changes create a new revision and activation point;
- restart resumes the original claim instead of recomputing a second occurrence;
- run_once chooses only the newest missed occurrence in a finite window;
- rule/source/authorization/ActivationBinding changes are revalidated before a new step;
- unprovable clock epoch/continuity pauses rather than extends deadlines.

Tests must do more than mock "scheduler returned one row": use real two-process/two-Server-frontend races, crash before/after claim, process pause, lost clock epoch, source/rule generation change, and enable/disable races.

## 7. Standing Approval coordinated implementation

UPSTREAM-AMENDMENTS is a prerequisite for S6. The feature may not be shipped secretly in Broker before that amendment is jointly accepted.

Core owns the StandingApprovalEnvelope validator, ApprovalUse builder, and atomic reservation/consumption logic. Broker may only request an attempt at mechanical approval and cannot submit an approval verdict.

Testing must traverse the real D7 `set_field_member`, FieldSelection/Narrow Field Qualification, PreparedActionBinding/preview, and D6 request/plan path. A simplified JSON fixture alone cannot establish closure.

Core race tests:

1. exactly-one Entry becomes two after prepare;
2. source A→B→A;
3. final approval use raced by two Runs;
4. approval revoke/expiry raced with D6 planning CAS;
5. current D6 write permission revoke/regrant after planning;
6. commit succeeds but receipt is lost;
7. committed raw no-op;
8. crash at approval reserved, D6 planned, and author commit points;
9. stale preview/cursor/EffectBytes delivery epoch;
10. mutant smuggles note/provenance/another member/Facet/body changes.

An automatic submission completes only when the real D6 commit transaction stores both author result and approval consumed.

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

D10 cost engine performs atomic multi-account reservation rather than read-then-decrement. It must enforce at least Run, DelegationLease, Automation, and deployment limits while remaining separately cumulative from original D6 work/attempt budgets.

Money uses one account currency and Counter microUnits only. Pricing rules are versioned and frozen:

- fixed charges;
- token/unit linear charges when finite input/output/work limits imply a finite maximum;
- dynamic/auction price without a finite maximum cannot offer hard-ceiling mode.

Reservation state is durable. uncertain is not released until billing truth is proven. Overcharge anomaly freezes the capability for administrative recovery and does not rewrite historical reservation to make it "legal".

Real provider tests include success billing, error billing, retry billing, delayed invoice, missing usage, usage disagreement, simulated over-ceiling, currency mismatch, and crash recovery.

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

## 11. Capability and error conformance

D1 capability reasons and fixed precedence are tested against real deployment combinations, especially:

- policy_denied + missing component;
- policy_denied + offline;
- missing component + not configured;
- offline + incompatible version;
- temporarily unavailable mutually exclusive with offline;
- Mobile unsupported_surface ahead of later install state.

D10 control errors must show the same not_visible response when a hidden object exists versus is missing until the caller has current visibility. Original D3/D6/D7/D8/D9 errors must pass through exactly from their owner rather than being wrapped in a generic AgentError.

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

## 13. Real implementation replacement and prohibited compatibility layer

A later implementation should atomically replace any old free tool callback, arbitrary JSON argument, UI-self-asserted approval, inherited process environment secrets, extension/name-driven automatic tool dispatch, Agent direct file writes, or cursor/provider state mixed into author source. If an old prototype was unpublished, do not preserve serde aliases, fallback parsers, or dual-read/dual-write.

Historical research fixtures may remain with explicit non-authoritative labels; public/API/CLI/schema/positive fixtures may not accept old and new controlled semantics simultaneously.

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

1. prompt injection attempting to expand read/egress/secret/tool/budget;
2. MCP descriptor/schema drift, fake readOnly, huge output;
3. two-entry same-value standing approval;
4. approval-count N=1 dual race;
5. cost N=1 dual race;
6. revocation raced with context delivery, author planning, external send;
7. crash at occurrence claim, ApprovalUse reservation, D6 planned, author commit;
8. external unknown with active/expired idempotency window;
9. credential rotation plus unknown mutation;
10. local audit failure versus collector offline;
11. package activation crash/failed upgrade/successor rollback;
12. D4 three-generation semantic-revival attack;
13. cancelled Run with prior author committed/external succeeded;
14. dirty D8 Draft plus valid background author commit;
15. Mobile upload/Agent/approval negative capability;
16. hidden object exists/missing non-disclosure;
17. provider billing uncertain/overcharge;
18. package disable with author raw unknown namespace retained.

Each case includes a positive path and a mutant/negative, not merely string-log comparisons.

## 16. Completion gate

The author implementation plan is ready for independent review only when:

- CANDIDATE, TERMINOLOGY, SCENARIO-DISPOSITIONS, UPSTREAM-AMENDMENTS, and this file agree;
- 48/48 upstream input coverage remains accurate;
- D6/D7 amendment is clearly an unactivated proposal;
- unsupported/deferred is never written as available;
- automatic author commit remains limited to single_field_member profile;
- External effect unknown, cost uncertain, audit failure, and cancel/planned recovery each have one normative result;
- D1 surface/reason, D3 identity, D4 Registry, D8 confirmation, and D9 worker/publication are not silently changed;
- every actually run evidence item is reported at its exact layer and pending items are not labeled pass.

These are candidate-completeness conditions, not an independent Gate verdict.
