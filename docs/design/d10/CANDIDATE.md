---
source_language: zh-CN
translation_of: CANDIDATE.zh-CN.md
translation_status: synced
---

[简体中文](CANDIDATE.zh-CN.md)

# D10 Agent, Automation, and External Capabilities Candidate

revision: D10-r01-candidate-2026-09-25; status: candidate, pending independent review and coordinated activation. This candidate takes D1-D9 at fixed upstream input commit `f205831c848729f7ddbc3ba0cf32b689459c0c98` as authoritative and has completely read all 48/48 inputs listed by `docs/design/inputs.json`. This is an author candidate; it does not mean the Gate has passed, the product is implemented, D6/D7 have changed, or A2 may start.

## 1. Selection and problem boundary

D10 selects **Narrow Delegation Broker + Typed Contribution Catalog + Specialized Executors**: one narrow shared security control plane, while the Agent runtime, automation scheduler, connector/tool adapters, model adapters, and D9 conversion workers keep specialized closed protocols. The Broker coordinates only authenticated delegation, contribution availability, secret references, budgets, audit, Run lifecycle, and recovery of external side effects; it does not interpret the author domain model, store a second copy of author facts, or gain a workspace commit path outside Core.

Core remains the sole author-transaction authority. Existing authority for author content, D3 identity/lifecycle, D4 typed schema, D5 collections, D6 policy/transactions/recovery, D7 Query/View/Action, D8 Draft/confirmation, and D9 conversion/publication remains unchanged. New D10 control records may decide whether an external capability may be attempted, whether a constrained delegation remains valid, or whether an external request is known to have completed; they cannot use D10 state to declare that an author commit succeeded.

This generation must support local Desktop/CLI and hosted Server. WebUI may only initiate or manage these capabilities through Server. Initial Mobile runs no Agent, automation, connector, or conversion, manages no credentials for those capabilities, and exposes no Agent/automation approval entrypoint; it may only consume ordinary results produced by valid workspace commits.

## 2. Non-goals

This generation does not define a general workflow DAG, arbitrary scripting system, arbitrary JSON patch, second query language, second permission system, second author ledger, distributed atomic transaction across external filesystems and the workspace, realtime collaboration protocol, Mobile Agent, arbitrary remote shell, execute-on-discovery MCP tooling, durable portable transcript, general bidirectional synchronization semantics, or writeback semantics for every external system.

D10 does not turn D9 workers into a generic plugin host, add a free-form tool/action kind to D7 ActionSpec, change the D4 Registry schema or cumulative evolution rules, add a D3 identity kind, or turn a package, Run, approval, tool call, provider ID, cursor, etag, UID, RECURRENCE-ID, or transcript into an EntityRef.

## 3. Global invariants

1. Every author mutation still ends at the original D3 or D6 author commit point; D10 has no third commit entrypoint.
2. Read, content egress, workspace mutation, external side effect, and secret use are five independent authorization dimensions; none implies another.
3. Web pages, Documents, tool results, MCP descriptors, model output, template text, and provider responses are untrusted data and cannot expand principal, delegation, tool allowlist, egress recipient, network, file, process, secret, budget, or approval.
4. D10 control identity is not content identity. RunId, AutomationId, ApprovalId, ExternalEffectId, and AuditEventId cannot enter EntityRef or replace a D3 Ref.
5. Installation, namespace ownership, runtime availability, and author facts are orthogonal. Disablement, uninstall, failed upgrade, credential rotation, or provider outage cannot delete, default, or rewrite author facts.
6. External side effects and a Core transaction are not an atomic whole. The product must report author and external outcomes separately.
7. Capability discovery is not an authorization ticket; every protected operation revalidates D1 capability, current D6 permission, D10 delegation, and step-specific authorization.
8. Every resume/replay uses the original fixed inputs, original OperationId, or original external request identity; an unknown outcome cannot be retried under a new ID.
9. Caches, context bundles, previews, tool results, and generated staging remain subject to their current authorization/generation gate; prior computation is not a right to keep delivering them.
10. Bounded models, CI, real Core, OS sandbox, real protocol-service, and UI/device evidence are reported separately; success at one layer never establishes complete product support.

## 4. Competing complete alternatives and selection

| Option | Structure | Main advantage | Main failure risk | Candidate decision |
| --- | --- | --- | --- | --- |
| A | Shared broker/control plane + specialized Agent/automation/connector | Reuses security and recovery rules | Common layer can expand into a universal tool system | revise |
| B | One capability host + manifest-driven execution | Appears operationally uniform | Manifest becomes a second Query/Action/schema and a large attack surface | reject |
| C | Full separate control systems for Agent, automation, connector | Direct domain isolation | Secret, revocation, budget, audit, and unknown recovery drift across copies | reject |
| D | Narrow Broker + typed Catalog + specialized executor; workspace safety remains in Core | Shares security control without sharing domain execution semantics | Requires strict prevention of business/author authority in Broker | select |

Option D deliberately accepts a small amount of specialized executor state-machine duplication in exchange for avoiding an open-ended universal tool host. Only identity, delegation, install trust, budget, audit, Run, and external-effect recovery live in the common layer; domain inputs and commits remain decoded and validated by their existing owners.

## 5. Authority and durable state

D10 control state lives in a managed control domain bound to the D6 Authority Store and is mutated through closed Core/Server Core adapters. Broker, package code, Agents, connectors, and MCP servers receive no database, workspace directory, author-store, SQL, or direct file-write handle.

| State | Authority | Rule |
| --- | --- | --- |
| Author source, Ref, revision, D3/D6 decision | D3/D6 | Existing transaction, CAS, fence, and receipt remain |
| D4 Registry semantic ledger | Consumed by D4, source authenticated by D10 | Cumulative evolution; historical rollback/deletion forbidden |
| D10 Capability Catalog and ActivationBinding | D10 managed control domain | Describes contribution/runtime availability only; does not interpret author values |
| Delegation Lease, Standing Approval, Automation Definition | D10 managed control domain | Restrictive, finite lifetime, versioned |
| Run, step, occurrence claim, ApprovalUse, cost reservation | D10 runtime control record | Cannot declare an author commit |
| ExternalEffectIntent and recovery evidence | D10 runtime control record | Separate from Core receipt |
| Credential secret value | OS/Server secret store | D10 stores only SecretRef and generation |
| Package/runtime immutable assets | Managed asset area | Exact digest/version; staging is not activation |
| Transient context, model/tool output, transcript | Limited pins/cache | Cannot replace author/control ledger |

Every durable record that affects workspace-operation eligibility must be changed through a Core-managed closed adapter. Ordinary runtime telemetry may be transported independently, but recovery may not treat telemetry as approval, authorization, cost, or side-effect fact.

## 6. Registry, Catalog, and activation

The D4 Registry remains the only semantic namespace/schema authority. D10 authenticates package/publisher/namespace claims, installed assets, and contribution origins, then supplies the complete candidate `RegistrySnapshot/1` and `RegistryBinding/1` to existing D4 validation, evolution, and catalog loading. D10 adds no Registry member and changes no D4 Field/Facet/Relation/Calendar/Unit semantics.

D10 separately maintains a Capability Catalog that records each executable or pure-data contribution's exact package digest/version, contribution kind/version, runtime profile, platform/architecture, dependencies, host privileges, network/egress class, secret requirements, cost profile, and D1 capability ID. The Catalog cannot keep a second FieldDefinition or prove namespace ownership from display name or install order.

```text
ActivationBinding/1 {
  workspaceRef,
  activationGeneration,
  registryBinding,
  capabilityCatalogDigest,
  trustRevision
}
```

Activation order is fixed: stage immutable assets → verify publisher/namespace/dependencies → construct candidate Registry and Catalog → complete D4 evolution/load validation → prove runtime assets are available → switch ActivationBinding in one managed transaction. Before failure, the old binding continues; after failure there may not be a half-Catalog or half-Registry.

Temporary offline/health failure does not create a semantic generation. A package, definition, runtime contract, or trust change produces a successor binding. An unactivated failed upgrade may discard staging; rollback after activation must be a successor activation and cannot move the D4 semantic-ledger pointer backward or delete intervening history.

Disable/uninstall may make executable contributions unavailable and may place definitions whose proof requires that provider into the existing D4 unavailable branch, but author raw source, cumulative tombstones/migrations, and required accepted semantic evidence remain. Unavailable cannot be interpreted as an empty set or deletion of facts.

## 7. Publisher, namespace, and package trust

This generation uses SHA-256 complete-content binding and application-level Ed25519 signatures for package authenticity. This is not platform release signing, notarization, or app-store identity and does not change the D1 release boundary.

PublisherIdentity, NamespaceClaim, and PackageManifest are distinct layers: the publisher root/key proves who signed, NamespaceClaim proves whether that publisher owns a publisher namespace, and PackageManifest proves that a package's asset catalog and contributions came from an accepted publisher. A self-signed package gains no namespace ownership, and first install does not automatically create a claim.

D4's frozen core, first-party, workspace_user, and reserved namespace rules remain unchanged. A publisher may claim only an unreserved namespace, and one active namespace has exactly one owner. Install order, enablement, localized label, package display name, download location, or a merely valid signature is not owner proof.

Key rotation requires proof of continuity from the prior publisher and acceptance by current trust policy; recovery from key loss is an explicit administrative decision. Trust revocation prevents new related execution and trusted-context construction, but does not delete historical signatures, saved decisions, or D4 semantic history.

## 8. Principal, delegation, and authorization intersection

D10 does not define a permission algebra parallel to D6 Policy. A Delegation Lease can only further restrict the current authenticated principal/D6 Policy.

```text
DelegationLease/1 {
  leaseId,
  leaseRevision,
  workspaceRef,
  principal,
  automationOrRun,
  activationBinding,
  capabilityAllowlist,
  readScope,
  egressRecipients,
  externalEffectClasses,
  secretUsages,
  notBefore,
  notAfter,
  maxRuns,
  budgetAccounts
}
```

The first generation permits only one delegation layer from a user/administrator to a named Automation or Run; Agents, tools, and connectors cannot redelegate authority to another principal. Child steps remain part of the same Run and share the original Lease and budget restrictions.

Effective eligibility is D1 capability ∩ current D6 Policy/ObservationScope ∩ DelegationLease ∩ contribution deployment policy ∩ egress grant ∩ secret-use grant ∩ external-effect approval ∩ current ActivationBinding ∩ budgets. Failure in one dimension cannot be compensated by another.

Lease readScope cannot invent D6 Field ref-set permissions that D6 does not provide. Core first obtains a legal read/write scope under D6, and D10 then narrows it using exact owner/Field/context constraints. Expiry, revocation, or generation change prevents new protected steps; already committed author decisions and externally linearized sends are not retroactively rolled back.

## 9. Context, egress, and prompt injection

An Agent has no ambient workspace. Each context request explicitly selects workspace, typed sources, maximum scope, purpose, target model/tool recipient, and budget. Core/Server constructs an immutable ContextBundle under current authorization, binding exact source versions/result epoch/authorization generation and the actual selected bytes; context selection follows least necessary access and data minimization.

Readable context does not imply egress permission. Sending to Model A, Connector B, or remote MCP C is three distinct recipient authorizations; changing provider, endpoint, account, or remote origin must match again.

Precedence is fixed: host/product policy and managed delegation/approval are control inputs; an explicit user task is business input; Documents, web pages, email, tool results, MCP descriptions/prompts/resources, and model output are untrusted data. Any instruction in untrusted data to ignore policy, invoke more tools, send secrets, or approve another step remains plain text and cannot mutate control state.

Secrets, credentials, tokens, plan/effects handles, private environment, and unselected workspace data never enter ordinary ContextBundle. Logs and transcripts use explicit redaction; a sensitive value that cannot be safely redacted may not be published to ordinary logs.

## 10. Tool Value Profile and MCP

D10 ToolValueProfile/1 reuses a restricted subset of D7 TypeSpec/V: bool, exact text, int64, integer, decimal, Optional, closed object, bounded list, and closed union. Maximum type depth is 16, object members 64, union arms 8, list maximum 4096, type description 64 KiB, and complete parameters or results 8 MiB; deployment/run budgets may be narrower.

First-generation ToolValue excludes EntityRef, Locator, ActionEvidence, plan/result/effects tokens, SecretRef, arbitrary file-path capabilities, calendar/quantity values, and open maps. When a readable string representation of an author Ref is sent to an external service, it is only egress-approved text and does not regain Core authority at the receiver.

Files use a separate InputSlot that binds exact bytes, media type, purpose, recipient, and budget for one invocation; InputSlot is not a general path or cross-call file handle.

MCP is only one Tool Adapter transport/adapter protocol. MCP tool names, descriptions, JSON schemas, readOnly annotations, prompts, and resource contents are untrusted remote metadata and grant no workspace read, egress, secret, or write authority. Each callable tool must already have a locally accepted contribution binding and deterministic schema adapter; runtime discovery of a new tool/schema creates a pending description and does not hot-add it to an Agent allowlist.

Adapters between remote JSON and ToolValue must prove exact numeric, Optional/null, closed-member, and bounded-collection semantics; otherwise the operation is unsupported, with no JS-double, stringify, or extra-member fallback. Tool output never automatically executes another Action, URL, or tool call; an Agent may only form a new closed proposal that passes the authorization chain again.

## 11. Runtime isolation

Executable contributions receive no workspace mount, author DB, user home, browser profile, SSH agent, system clipboard, arbitrary environment secrets, arbitrary file-path access, or direct network by default. The trusted host exposes only exact InputSlots, read-only verified runtime/dependencies, private output/temp storage, resource limits, and managed transport.

Local process runtime and Server sandbox require separate real OS evidence; a timeout or container name does not prove isolation. If file, network, process-tree, resource, and cleanup boundaries cannot be demonstrated for a claimed platform, that contribution is unavailable on the platform.

D9 conversion workers keep their narrower no-network-by-default contract. D10 connector/model/remote-tool networking may only pass through managed transport with explicit contribution, account, recipient, and purpose authorization; "D10 has network" never grants network to a D9 worker.

## 12. Agent

AgentSession is an interactive mode of Run, not a content entity. A typical sequence is: obtain ContextBundle → model invocation → parse plain output/typed tool proposal → re-evaluate capability/authorization for each proposal → tool/model step or D7/D8 proposal → user or constrained standing approval decides whether to submit → consume the original receipt/result.

A model cannot submit an arbitrary source patch. Workspace changes can only become an existing D7 ActionSpec or an explicit D8 edit proposal; a change without a closed adapter cannot escape through natural language, tool JSON, or an MCP command.

Interactive author mutation requires explicit confirmation after a complete current preview by default. Model/tool timeout, cancellation, or parse failure has no author effect. Output arriving after cancellation may be retained as constrained diagnostics but cannot launch the next step after its step is closed.

Run completed only means all required steps reached their own terminal states; it does not mean a particular author or external effect succeeded. UI/CLI must expose actual per-step outcomes.

## 13. Automation scheduling

First-generation Automation schedules one accepted invocation and does not provide a general DAG, loop, or free script. A definition contains the capability invocation, schedule, DelegationLease, approval binding, budgets, queue limit, and missed policy.

A schedule may be a one-time D4 ZonedInstant or occurrences derived from an explicit Calendar recurrence/range source with finite horizon and limit. Core computes the occurrences from actual source and frozen D4 rule context; an executor may not self-assert trusted temporal context, and date-only input that does not determine an instant is not silently assigned midnight.

Concurrency policy is serial. Missed policy is only skip or run_once; run_once executes only the newest missed occurrence in the current recovery window and records the rest as skipped. The recovery window has a finite policy limit and never replays an unbounded history.

```text
AutomationOccurrenceKey/1 =
  (automationId, definitionRevision, sourceOccurrenceKey)
```

One key has at most one active Run claim. Enable/disable changes control revision but not definitionRevision, so repeated enabling cannot duplicate execution. A semantic change to invocation, schedule, Lease, approval, or budget produces a new definitionRevision and takes over only occurrences after an explicit activation point.

Before each occurrence begins, current capability, Lease, D6 authorization, ActivationBinding, secret generation, and budget are revalidated. Once a D3/D6 request has been produced, restart resumes only the original Run/original request and never resamples targets or creates a new OperationId.

## 14. Standing Approval

Standing Approval does not mean an Agent may modify arbitrary future content. First-generation unattended author submission supports one narrow profile of the existing D7 set_field_member action: one existing Node, one Field, exactly one Entry in the current complete Field, and one existing scalar member.

```text
StandingApprovalEnvelope/1 {
  approvalId,
  approvalRevision,
  workspaceRef,
  automationId,
  definitionRevision,
  grantingPrincipal,
  delegationBinding,
  activationBinding,
  notBefore,
  notAfter,
  rule: {
    kind: "single_field_member",
    ownerNodeRef,
    fieldId,
    selection: "require_exactly_one_entry",
    memberPath,
    memberType,
    valueConstraint
  },
  maxSuccessfulCommits,
  budgetAccountBindings
}
```

memberPath is a static object-member path with no list index, wildcard, or dynamic FieldId. memberType is restricted to bool, exact text, int64, integer, decimal, and semantic_code. valueConstraint is either a finite set of at most 64 complete TypedLiteral values, an exact same-type numeric closed interval, or exact text with a UTF-8 byte limit and no CR/LF. All D4 nonEmpty, code-scope, schema/cardinality, and constraint rules still apply separately.

Mechanical approval requires every condition: current D1/D6/D10 authorization; matching approval/automation/definition/delegation/activation binding; fresh current source revision; exactly one Entry in the complete Field; successful D7 Narrow Field Qualification; exact owner/field/member/type; new value within the frozen constraint; actual mutation footprint changes only that member while occurrenceKey, other members, qualifiers, note, provenance, other Entries, body, Facets, Refs, relations, identity, placement, and control remain unchanged; complete owner_fields preview exists and is retrievable; audit and approval-count/cost reservations succeed.

Failure cannot choose the first Entry, widen to a whole Entry/source, degrade to append/remove, or select another same-named target. It can only transition to awaiting_confirmation, blocked, or failure.

## 15. Fresh prepare, ApprovalUse, and D6/D7/D8

Every automated author mutation still performs: fresh current Field selection → original D7 ActionSpec → original d7_action_prepare → complete D7 EffectManifest/bytes → Core mechanically matches StandingApprovalEnvelope → create ApprovalUse → original D6 request.

```text
ApprovalUse/1 {
  approvalId,
  approvalRevision,
  runId,
  stepId,
  request,
  planToken,
  preparedBindingRef,
  previewBinding,
  footprintProof,
  delegationBinding,
  activationBinding,
  approvalCountReservation,
  budgetReservations
}
```

ApprovalUse is managed authorization evidence, not an author plan, and adds no member to ActionSpec, PreparedActionBinding/2, or d6_commit_request. A client cannot submit approved=true. Core creates it independently from the saved prepared record, preview, and footprint.

Existing upstream text is insufficient to freeze unattended confirmation: current D7 describes user confirmation after preview, while D6 has no StandingApproval/ApprovalUse atomic-consumption and counting contract. This candidate therefore carries a coordinated D6/D7 amendment proposal; until it is jointly accepted, unattended author commit remains unavailable and Automation may only prepare a proposal for interactive confirmation.

The coordinated amendment requires D6 planning CAS to validate and reserve approval use/count/budgets, and final author commit to revalidate current authorization and the original ApprovalUse while publishing author effects, the original receipt, consumed approval count, and required audit link in one transaction. A committed raw no-op still consumes one successful commit. Same-request replay does not consume again.

If a planned decision loses sufficient approval, it remains planned/blocked and cannot silently switch to a renewed envelope. A user may explicitly authorize the exact original plan once and attach new authorization evidence for recovery, without modifying request, OperationId, source plan, or PreparedActionBinding. A deterministic business dependency conflict cannot be revived by new approval.

D8 interactive editing is not loosened. Automation does not forge EditSession, draftSerial, IME, or user clicks. Agent output requiring D8 document/annotation edit still enters D8 Draft/preview/explicit confirmation. A background commit to the same owner only drives a dirty Draft through existing stale/conflict handling and never overwrites the Draft.

## 16. Connector and secret

A Connector is a protocol adapter for one named external system. It owns provider-specific cursor/etag/version/account control state, but these stay in D10/D6 control state and are not author Refs or Fields. External stable IDs may participate in lookup/upsert only through frozen D3 SourceBinding/OriginBinding protocols; connector installation never creates identity authority.

A flow that changes SourceBinding, OriginBinding, watermark, or persistent sync control state needs a named closed adapter that produces the original D3/D6 request and associates progress atomically with the corresponding author commit. The first-generation single_field_member standing approval excludes those control effects and cannot be advertised as general bidirectional sync.

SecretRef binds contribution, external account, usage, audience, and secretGeneration. Credential values are injected only through a trusted transport authentication channel; they never enter workspace source, ContextBundle, model prompt, ordinary ToolValue, transcript, ordinary log, or export.

Credential rotation creates a new generation. New unsent invocations use a currently allowed generation; recovery of an already started external request stays bound to the generation it actually used. A new credential may be used, with explicit permission, for read-only reconciliation of the same account, but cannot silently retransmit an outcome_unknown mutation.

## 17. External effect

An external mutation and a Core transaction never combine into one atomic success.

```text
ExternalEffectIntent/1 {
  effectId,
  contributionBinding,
  accountBinding,
  targetBinding,
  requestPayload,
  secretGeneration,
  idempotencyBinding,
  delegationBinding,
  approvalBinding,
  egressBinding,
  budgetReservations
}
```

States are prepared → submitting → succeeded | failed_no_effect | outcome_unknown; only a request proven not to have started sending may move from prepared to cancelled. outcome_unknown remains unknown until reliable reconciliation; manual_required is a recovery mode, not a fake failed terminal state.

Automatic retry is allowed only when an accepted adapter explicitly supplies a still-valid idempotency window/key or reliable failed_no_effect evidence exists. Retry keeps the same EffectIntent, semantic request, target/account, and original idempotency key and revalidates current authorization/egress/cost. Expired idempotency window, changed target/request, or credential change that breaks the original contract stops automatic sending.

Trusted transport uses a send fence: first durable intent, audit started, and cost reservation; then begin sending inside a host gate serialized with revocation. If revocation wins first, no send occurs; once send has begun, cancel/revoke cannot prove no external effect occurred. A crash between durable started and a provable send outcome recovers conservatively as outcome_unknown.

Compensation is a new ExternalEffectIntent with its own authorization, approval, and cost and is not rollback. A workflow requiring both Core write and external mutation shows two outcomes and creates no composite author receipt.

## 18. Budget, cost, and concurrent reservation

Existing D6 work/attempt budgets remain. D10 adds managed multi-account reservation for model/tool/network/external cost; the narrowest remaining balances of Run, Lease, Automation, and deployment accounts must be reserved atomically so two concurrent Runs cannot both observe and spend the final balance.

```text
Money/1 {
  currency,
  microUnits
}
```

microUnits is a Counter and all multiplication/accumulation is checked. One account uses one configured currency and performs no implicit FX. Before a potentially billable attempt, a finite conservative maximum is reserved under the currently accepted pricing rule, binding priceVersion, request limits, and included charge categories. A service whose finite maximum cannot be proven cannot offer hard cost-ceiling mode.

Cost reservation states are reserved → settled | released | uncertain. Release requires proof that no billable send occurred; settlement requires reliable final billing evidence; timeout, crash, or billing uncertainty becomes uncertain and continues to occupy the maximum. TTL, restart, and transcript deletion cannot release uncertain reservations. Every retry reserves independently; effect idempotency does not imply free billing.

If a service violates an accepted pricing contract and produces cost above reservation, record the actual anomaly, freeze the affected capability, and require administrative handling. The system may not silently raise the ceiling while continuing to claim the original guarantee.

## 19. Audit and retention

Before execution, the following protected steps require a durable local/Server authority-bound audit started record: sensitive workspace/context read, egress, secret use, D10-initiated author submit, external mutation, and delegation/approval/package-activation mutation. If it cannot be written, fail closed.

Loss of a remote log collector need not disable local operation when a protected local audit spool, integrity, and retention budget remain provable; later aggregation is allowed. Failure of the durable local audit store stops new protected steps.

The audit link for a Core author commit is stored atomically with the original author decision. If terminal evidence for an external effect cannot be durably recorded, preserve started/outcome_unknown and do not resend merely to obtain logging. Cancellation, revocation, and emergency stop have independent reserved control-write capacity and cannot be blocked by exhaustion of ordinary telemetry quota.

Transcript and audit are separate. Transcript may have finite deployment retention/deletion policy, but deletion cannot remove pins needed for planned author recovery, unknown external effect, uncertain cost, or required security audit. Secret and raw credential never enter transcript/audit export.

## 20. Run, cancellation, and recovery

Run state is the closed set queued|running|awaiting_confirmation|blocked|cancelling|reconciling|completed|failed|cancelled. Each step separately stores its domain outcome, and Run terminal state cannot erase already committed author/external facts.

A queued/prepared unsent step can become cancelled. A request that has entered D6 planned is not automatically aborted by Run cancellation; Run becomes cancelling/blocked and follows the original D6 planned-recovery contract. Revoking current delegation may prevent a new author commit that no longer has current authority, but revocation is not a permanent business rejection. A committed decision is never rolled back.

After an external effect enters submitting, cancellation only prevents later steps; that effect still resolves to succeeded, failed_no_effect, or outcome_unknown. Late model/tool output cannot start new work after its step is closed.

Restart first restores durable occurrence claim, Run, ApprovalUse/cost reservations, and original requests. If clock continuity cannot be proven, the current attempt ends or pauses and its deadline is not extended. If execution side-effect outcome cannot be proven, transition to blocked/reconciling rather than creating a new OperationId or effect ID.

## 21. Errors and unavailability semantics

D1 capability availability and its fixed reason precedence remain unchanged, especially policy_denied before component, configuration, network, version, and health details. D10 does not replace missing_component, not_configured, offline, incompatible_version, or temporarily_unavailable with one runtime_unavailable.

D10 control request stages are: closed decode/version → D1 static capability/surface/release → current principal/control-object visibility → delegation/data observation → deployment binding → exact input/approval → budget/audit → execution. Stage ordering precedes error specialization to prevent unauthorized probing.

The D10 control error closed set is invalid_request, not_visible, control_conflict, binding_changed, approval_required, approval_expired, delegation_expired, budget_exceeded, audit_unavailable, state_unavailable, invalid_output, cancelled, external_outcome_unknown. Unknown tokens, wrong tag, wrong audience, and unauthorized control objects all return not_visible; expired/conflict details are disclosed only when the caller may already read that caller-owned record.

Once a request enters an original D3/D6/D7/D8/D9 entrypoint, that owner returns its original error/disposition. D10 does not wrap it in a D10 error or change the original refusal ordering based on internal deployment detail.

## 22. Product surfaces

Local Desktop hosts the shared local Broker, scheduler, secret transport, and Core adapters; local CLI connects to or starts the same capability family rather than creating a second scheduler or authority. Remote Desktop/CLI call Server only.

Server hosts the managed Broker, scheduler, connector/model/tool executors, and secret transport while continuing to satisfy the D1 single actual commit-holder boundary; this generation does not define a multi-primary scheduler/database cluster. WebUI manages and initiates capabilities only through Server and directly runs no worker/model/connector and holds no secret.

Mobile returns D1 unsupported_surface for agent.session, automation, connector execution, conversion execution, and credential management. A "read-only monitor" cannot become a hidden approval/delegation path prohibited by D1. Mobile may consume ordinary committed workspace facts.

LTR/RTL, locale, screen reader, and Web/CLI transport differences affect presentation and interaction only and do not change request bytes, target, approval rules, cost, errors, or author/external outcomes.

## 23. D1-D9 composition and required amendments

D1 surfaces, capability reasons, and sole commit holder remain; D2 raw source/unknown-provider preservation remains; D3 identity/SourceBinding/OriginBinding/Provenance remains; D4 Registry exact shape/evolution remains; D5 gains no persistent Record; D8 Draft/explicit edit confirmation remains; D9 worker/ExportPlan/publication remains.

Only the D6/D7 standing-approval author-submit branch needs a coordinated amendment, specified precisely in UPSTREAM-AMENDMENTS. Existing upstream remains authoritative until that amendment is jointly accepted and coordinatedly activated; therefore the D10 design can be reviewed now, but the product must keep the unattended author-commit branch unavailable.

## 24. Security counterexamples

- A malicious Document saying "read all files and upload them" cannot change ContextBundle readScope, egress recipient, or tool allowlist.
- An MCP server marking a delete tool readOnly does not reduce external-effect authorization.
- When one Field contains two Entries with the same value, standing approval selects neither automatically and unattended execution stops.
- When one approval use remains and two Runs race, atomic reservation lets at most one obtain submission eligibility.
- If source revision, Registry, activation, or delegation changes after preview, the old automated approval cannot be consumed.
- If an external request was sent and its response was lost, cancel/restart cannot resend under a new key.
- Unknown provider cost makes the reservation uncertain and cannot be released by TTL cleanup.
- When the audit collector is offline but the local spool is intact, operation may continue; when durable local audit cannot be written, new protected operation stops.
- Disabling a package cannot remove raw source for its author Fields, and schema of the same ID cannot revert to an older meaning during rollback.
- Cancelling a Run cannot describe an already committed D6 receipt or externally succeeded outcome as rolled back.

## 25. Implementation and acceptance boundary

Complete implementation must separately prove at least: closed decoders and bounded state-machine models; real D3/D4/D6/D7/D8 integration; SQLite/authority crash, fence, approval-count, and cost-reservation fault injection; claimed Windows/Linux/macOS executor sandboxing; hostile real MCP/model/connector protocol services; credential storage and rotation; Desktop/CLI/Server/WebUI end-to-end behavior; Mobile negative capability; audit/retention/cost anomaly handling; scale and budget.

Completion of this author candidate is not independent acceptance. Independent review must evaluate a simpler complete alternative, adequacy of the D6/D7 amendment, zero remaining P0/P1 conditions, terminology and mandatory-scenario closure, and which evidence remains implementation pending.
