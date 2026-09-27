---
source_language: zh-CN
translation_of: CANDIDATE.zh-CN.md
translation_status: synced
---

[简体中文](CANDIDATE.zh-CN.md)

# D10 Agent, Automation, and External Capabilities Candidate

revision: D10-r03-supplement-2026-09-28; status: candidate, pending independent review and coordinated activation. This candidate takes the original 48 D1-D9 inputs at fixed upstream U=`f205831c848729f7ddbc3ba0cf32b689459c0c98` as authoritative and has additionally read the D4 reference catalog added by supplemental S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`; current author input coverage is 49/49. Historical 48/48 reading records still refer only to the original U inventory and are not retroactively rewritten as 49/49. This is an author candidate; it does not mean the Gate has passed, the product is implemented, D6/D7 have changed, or A2 may start.

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

Supplemental S contributes the 49th current input, the D4 reference catalog: 61 Fields, 7 Facets, 22 value-type aliases, 4 qualifier sets, and 1 Calendar series policy. It changes no existing D4/D7 semantics; it supplies the previously omitted normative machine graph. For `single_field_member`, the actual catalog contains 27 relation Fields, a cross-Field `union_variant_equal` between `calendar/range` and `calendar/recurrence` under `calendar/event`, plus required-field and Field-local constraints. Therefore the automatic path must still run the complete Registry-graph proof from D7 Narrow Field Qualification and can never infer safety merely from "one Field is modified". The positive `people/phone` case is a non-relation Field in `people/person`, whose Facet has no additional constraints and whose Field constraints are empty, while alias, qualifier, source-envelope, and all other D7 dependencies still require full validation.

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

The first generation permits only one delegation layer from a user or administrator to a named Automation or Run; Agents, tools, and connectors cannot redelegate authority to another principal. Child steps remain part of the same Run and share the original Lease and budget restrictions.

Effective eligibility is D1 capability ∩ current D6 Policy/ObservationScope ∩ DelegationLease ∩ contribution deployment policy ∩ egress grant ∩ secret-use grant ∩ external-effect approval ∩ current ActivationBinding ∩ budgets. Failure in one dimension cannot be compensated by another.

Lease readScope cannot invent D6 Field ref-set permissions that D6 does not provide. Core first obtains a legal read/write scope under D6, and D10 then narrows it using exact owner, Field, and context constraints. Expiry, revocation, or generation change prevents new protected steps; already committed author decisions and externally linearized sends are not retroactively rolled back.

`maxRuns` is the finite positive Counter limit on Runs that may enter protected execution over the full lineage of one `leaseId`. Changing `leaseRevision` never resets prior consumption. An authorized successor revision may raise the limit but cannot lower it below already consumed uses; revocation uses a separate revoked state. Creating a queued Run, establishing an occurrence claim, performing scheduler-only bookkeeping, or cancelling before the first protected step consumes no `maxRuns` use.

Immediately before the first protected step is permitted, a Core-managed Run-admission CAS validates the exact current `leaseId/leaseRevision`, proves trusted current time lies within `notBefore..notAfter`, validates current ActivationBinding and Run/occurrence binding, validates all admission budgets, and proves cumulative consumption for the `leaseId` lineage is below `maxRuns`. The winning CAS durably creates:

```text
LeaseRunUse/1 {
  leaseId,
  leaseRevision,
  runId,
  automationId,
  definitionRevision,
  occurrenceKey,
  admissionClockEpoch,
  admittedAt
}
```

The record and cumulative consumption are stored in the same managed control transaction. It is a Run-admission deduplication fact, not an author ledger. The successful CAS is the `maxRuns` consumption linearization point: later model or tool failure, user cancellation, a failed or cancelled Run, or process crash never refunds it. Restart of the same Run restores the original `LeaseRunUse/1` and does not consume again. Exhaustion returns D10 `delegation_exhausted` before entering D6/D7 or external transport.


`maxRuns` is compared and consumed only on the first-admission branch where this Run has no existing `LeaseRunUse/1`. When a complete existing `LeaseRunUse/1` is bound to the same `runId`, `leaseId`, and exact `leaseRevision`, the second and later protected steps of the same Run, including recovery of an original D6 planned request inside that Run, **do not recheck remaining>0 and do not consume again**. Thus with `maxRuns=1` and cumulative consumption already equal to 1, the same admitted Run is not misclassified as a new exhausted Run. Every later step still revalidates current D6/D10 authorization, the exact admitted Lease revision, trusted time, current ActivationBinding, applicable approval, and budgets. Revocation, expiry, revision/binding change, or other current-eligibility failure still blocks execution. Only a new Run with no existing LeaseRunUse and cumulative use at maxRuns returns `delegation_exhausted`. Missing or unprovable continuity of an existing LeaseRunUse returns `state_unavailable`; recovery never guesses by consuming again.

Trusted-time eligibility of the Lease is rechecked before every new protected step and before final author or external submission. Lease expiry does not depend on a cleanup task having run: once trusted current time is after `notAfter`, a new step returns `delegation_expired`. If the clock epoch or time continuity cannot be proven, return `state_unavailable` and pause rather than assuming no time elapsed. Once trusted time is re-established, adjudicate using actual current time. Already completed author commits, external effects, costs, and `LeaseRunUse/1` history are never rolled back.
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

One key maps to at most one Run identity and one durable claim, including after terminal state: restart, schedule rescan, disable→enable, and scheduler-cache rebuild cannot create a second Run. Enable/disable changes control revision but not definitionRevision and does not delete claim or terminal proof. A semantic change to invocation, schedule, Lease, approval, or budget produces a new definitionRevision and takes over only occurrences after an explicit activation point.

Terminal occurrence history may be compacted only into durable proof that the key already produced its original Run and terminal outcome. While a definition revision can still be rescanned or recovered, ordinary GC may not turn "already executed" back into "never seen". Recovery of the original claim always returns the original Run identity and outcome.

An occurrence claim may exist before Run admission, so a queued or blocked Run that has never entered protected execution consumes no Lease `maxRuns`. Once the same Run already has `LeaseRunUse/1`, later steps and recovery of its original planned request reuse that admission fact and do not re-run the global remaining-count gate merely because remaining is now zero. Only the §8 Run-admission CAS immediately before the first protected step creates `LeaseRunUse/1`. If `maxRuns` is exhausted, the occurrence keeps its existing claim/Run, enters blocked, and returns `delegation_exhausted`; creating another Run for the same occurrence cannot bypass the limit.

Before occurrence start and every new protected step, revalidate current capability, exact Lease revision, trusted time, D6 authorization, ActivationBinding, secret generation, and budget. Once a D3/D6 request has been produced, restart resumes only the original Run and original request and never resamples a target or creates a new OperationId. A paused Run may naturally outlive its Lease even when scheduler cleanup never ran; once trusted time advances beyond expiry, new context, model, tool, external steps and final author submission are blocked. Loss of clock continuity fails closed as `state_unavailable`; once trusted time is restored, determine expired or active from actual current time.
## 14. Standing Approval

Standing Approval does not mean an Agent may modify arbitrary future content. First-generation unattended author submission supports one narrow profile of the existing D7 `set_field_member` action: one existing Node, one Field, exactly one Entry in the current complete Field, and one existing scalar member.

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

Common mechanical prerequisites are: current D1/D6/D10 authorization; exact approval/automation/definition/delegation/activation bindings; a fresh current source revision; exactly one Entry in the complete Field; successful D7 Narrow Field Qualification; exact owner, Field, member, and type; new value inside the frozen constraint; a complete retrievable owner_fields preview; and ability to establish audit and approval-count/cost reservations.

There are exactly two legal actual-effect branches:

1. **One real member change:** the complete proposed source differs from before and actual MutationFootprint contains exactly the selected existing scalar-member change; occurrenceKey, other members, qualifiers, note, provenance, other Entries, body, Facets, Refs, relations, identity, placement, and control remain unchanged. The public owner_fields preview contains the complete Field before/after `field_change` under the original D7 contract.
2. **Byte-exact raw no-op:** the original `set_field_member` still resolves exactly to the same unique Entry and existing member, the current typed member value already equals the requested value, and the complete proposed source produced by the original adapter is byte-for-byte equal to the before source. MutationFootprint is empty, preview contains no `field_change`, and final D6 receipt has empty `sourceVersions`; standing approval never invents a member footprint, Field effect, or revision. Target existence, type, valueConstraint, current authorization, dependencies, and Narrow Field Qualification are still fully validated.

Anything outside these two branches cannot automatically choose first, widen to a whole Entry/source, degrade to append/remove, or select another same-named target; it can only transition to awaiting_confirmation, blocked, or failure. If the original D6 rules commit a raw no-op decision, it still consumes one `maxSuccessfulCommits` use; replay of the same saved decision does not consume again.
## 15. Fresh prepare, approval consumption, and planned recovery

Every automated author mutation still performs: fresh current Field selection → original D7 ActionSpec → original `d7_action_prepare` → complete D7 EffectManifest/bytes → Core mechanically matches StandingApprovalEnvelope → create ApprovalUse → original D6 request.

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

ApprovalUse is managed authorization evidence, not an author plan, and adds no member to ActionSpec, PreparedActionBinding/2, or `d6_commit_request`. A client cannot submit approved=true. Core creates it independently from the saved prepared record, preview, and footprint. Its approval-count state is only `unreserved → reserved → consumed | released_terminal`; historical state is never deleted or rolled back.

Existing upstream text is insufficient to freeze unattended confirmation, so UPSTREAM-AMENDMENTS proposes a coordinated D6/D7 amendment. Until it is jointly accepted, unattended author commit remains unavailable and Automation may only prepare a proposal for interactive confirmation.

### 15.1 Pre-D6 D10 denial versus in-D6 race failure

Before a formal `d6_commit_request` is sent, the D10 adapter may return D10 `approval_required`, `approval_expired`, `delegation_expired`, or `delegation_exhausted` from control state the caller is already allowed to observe. These are pre-submit control outcomes and do not prove a later race is impossible.

Once the unattended D10 path has associated ApprovalUse as an internal dependency of the original planToken and entered D6, an approval-eligibility race belongs to the D6 owner. The companion amendment adds exactly one `d6_error.code`: `approval_unavailable`, always with disposition `preflight`. It is reachable only after original D6 current authorization/ObservationScope and applicable business visibility have passed and the associated standing or supplemental approval can no longer be consumed before planning CAS or planned recovery submission. Original D6 permission failure remains `not_visible`; business dependency/semantic/budget failures keep their original D6 codes and cannot be masked by `approval_unavailable`.

For an unseen request, `approval_unavailable/preflight` writes no author ledger, no recorded rejection, and no planned decision. The caller may retry while the original preparation remains valid under newly valid approval, or return to an interactive path. For an already planned request, the same error leaves the ledger planned and preserves the original plan, pins, and reservations; it cannot be written as `semantic_rejected` or `terminal_failed`. The D10 adapter does not wrap this D6 error in a D10 approval code. If UI needs to distinguish exhausted, revoked, or expired approval, it performs a separately authorized D10 control read.

This is a necessary extension of the D6 closed error set. Existing D6 request/receipt/error object shapes remain unchanged, but a consumer selecting this coordinated capability must support the added enum value. Existing capability profiles never emit this code, and an unnegotiated old consumer may not be exposed to this branch.

### 15.2 Re-inspecting the original preview after planned

PreparedActionBinding/2, semantic preview, and recovery pins survive with a planned decision under the original ledger-recovery lifetime, while the original preview token may expire independently. Existing `d7_effects_resolve/open` is committed-only. The D7 transport companion therefore needs a read-only planned-preview recovery entrypoint rather than extending an old token or re-preparing:

```text
d7_planned_preview_open {
  wireVersion: 1,
  kind: "d7_planned_preview_open",
  protocolOwner: "D6",
  request: <original d6_commit_request>
}

d7_planned_preview_opened {
  wireVersion: 1,
  kind: "d7_planned_preview_opened",
  request,
  previewToken,
  previewCursorToken,
  previewManifest
}
```

Processing order is fixed: closed decode → current authenticated audience and protected minimal locator mapping → original D6 current authorization, original ObservationScope, and complete-preview disclosure eligibility → authority/custody/ledger continuity → byte-equal request at the same key with state exactly planned → complete original PreparedActionBinding/2, semantic preview, and all pins → create a new finite recovery delivery epoch.

The new `previewToken` remains in the action_preview transport family but binds only the immutable preview semantics and original pins saved by the planned decision. It never reruns Query, reparses a drifted definition, reselects a target, regenerates proposed source, or changes the request. It has an independent finite delivery lifetime and neither extends nor revives the old previewToken/cursor. Page and EffectBytes reads continue to use the original D7 transport; expiration of this recovery epoch returns the preview transport's `preview_expired`. A non-planned state, request mismatch, or unprovable pins/continuity follows existing `d7_effects_error` `effects_unavailable` or an earlier original authorization error. `d7_effects_resolve/open` remains committed-only.

Only after manifest, all pages, and required EffectBytes have been completely consumed under one recovery epoch may UI create new interactive authorization:

```text
PlannedDecisionApproval/1 {
  approvalId,
  approvalRevision,
  workspaceRef,
  grantingPrincipal,
  request,
  previewSemanticBinding,
  delegationBinding,
  activationBinding,
  grantedAt,
  expiresAt
}
```

It binds only the exact original planned request and saved immutable preview semantic record, not the expiring transport token, and changes no OperationId, plan, PreparedActionBinding, or target. Final submission still revalidates current D6 authorization, trusted time, and original business dependencies. A deterministic dependency conflict follows the original D6 authoritative-abort rules and cannot be revived by the new interactive approval.

### 15.3 Terminal state of an approval reservation

Only D6 planning CAS changes ApprovalUse count state atomically from unreserved to reserved. Successful author commit changes it to consumed in the same final transaction; replay of the same saved decision never consumes again.

When the original D6 ledger, under complete continuity and current authorization, proves that a planned decision **can never commit** and records authoritative `terminal_failed` under the original rules, the same abort transaction changes the ApprovalUse count reservation from reserved to `released_terminal`. released_terminal does not count against the reserved/consumed total for `maxSuccessfulCommits`, but immutable history remains. Terminal replay never releases twice.

Only authoritative terminal_failed performs this release. Run cancellation, preview/approval TTL, temporary revocation, Lease expiry, attempt/work pause, or temporarily unprovable authority/continuity is not an abort and keeps the reservation. Cost reservations are a separate state machine: a charge that occurred, may have occurred, or is unknown is never refunded merely because the author plan becomes terminal_failed; it follows §18 settled/released/uncertain semantics independently.

If a planned decision is later explicitly rescued by `PlannedDecisionApproval/1` and ultimately committed, the original standing-approval count reservation still moves reserved→consumed. The interactive rescue does not refund the already occupied standing slot for use by another Run.
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

Existing D6 work/attempt budgets remain. D10 adds managed multi-account reservation for model, tool, network, and external cost; the narrowest remaining balances of Run, Lease, Automation, and deployment accounts must be reserved atomically so two concurrent Runs cannot both observe the final balance.

```text
Money/1 {
  currency,
  microUnits
}
```

microUnits is a Counter and all multiplication/accumulation uses checked arithmetic. One account uses one configured currency and performs no implicit FX. Before a potentially billable attempt begins, reserve a finite upper bound under the currently accepted pricing rule, binding priceVersion, request limits, and included charge categories. A service whose finite maximum cannot be proven cannot offer hard cost-ceiling mode.

Cost reservation states are `reserved → settled | released | uncertain`, with fixed terminal meanings:

- `released` is allowed only when billable execution or a billable send is proven **never to have started**, such as cancellation before the send fence. It means the reservation has no corresponding billing attempt.
- `settled` means a billable attempt started and reliable final billing truth is known; the actual amount may be 0 microUnits. A request that was actually sent and later has a reliable zero bill is `settled(0)`, never released.
- `uncertain` means a billable attempt may have started but final cost cannot be proven. It continues to occupy the original upper bound until reliable billing truth or an authorized accounting-recovery result is obtained.

TTL, restart, author terminal_failed, and transcript deletion cannot release uncertain. Every retry establishes a separate cost reservation; effect idempotency does not imply free billing. Author approval-count reservations and cost reservations are separate: the former can be released with authoritative terminal_failed under §15.3, while cost becomes released only from proof that no billable attempt began.

If a service violates an accepted pricing contract and produces cost above reservation, record the actual anomaly, freeze the affected capability, and require administrative handling. The system may not silently raise the ceiling while continuing to claim the original guarantee.
## 19. Audit and retention

Before execution, the following protected steps require a durable local/Server authority-bound audit started record: sensitive workspace/context read, egress, secret use, D10-initiated author submit, external mutation, and delegation/approval/package-activation mutation. If it cannot be written, fail closed.

Loss of a remote log collector need not disable local operation when a protected local audit spool, integrity, and retention budget remain provable; later aggregation is allowed. Failure of the durable local audit store stops new protected steps.

The audit link for a Core author commit is stored atomically with the original author decision. If terminal evidence for an external effect cannot be durably recorded, preserve started/outcome_unknown and do not resend merely to obtain logging. Cancellation, revocation, and emergency stop have independent reserved control-write capacity and cannot be blocked by exhaustion of ordinary telemetry quota.

Transcript and audit are separate. Transcript may have finite deployment retention/deletion policy, but deletion cannot remove pins needed for planned author recovery, unknown external effect, uncertain cost, or required security audit. Secret and raw credential never enter transcript/audit export.

## 20. Run, cancellation, and recovery

Run state is the closed set queued|running|awaiting_confirmation|blocked|cancelling|reconciling|completed|failed|cancelled. Each step separately stores its domain outcome, and Run terminal state cannot erase already committed author facts or external effects that already occurred.

Occurrence claim, Run identity, `LeaseRunUse/1`, and terminal outcome are four distinct control facts. Once scheduler creates a claim/Run for an occurrence, restart, rescan, disable→enable, or cache rebuild restores the original Run; terminal K never receives a new Run identity. Implementation may compact history only while preserving proof that K was already handled rather than recreating "not executed".

A queued/prepared Run that has not passed the §8 Run-admission CAS may be cancelled before the first protected execution without consuming `maxRuns`. Once `LeaseRunUse/1` exists, later failure, cancellation, or crash never refunds that run use. Recovery of the same Run reads the original use.

A request that has entered D6 planned is not automatically aborted by Run cancellation; Run becomes cancelling/blocked and follows original D6 planned recovery. Loss of current delegation, trusted time, or Standing Approval may prevent a new author commit that no longer has current eligibility, but temporary loss is not persisted as a permanent business rejection. A committed decision is never rolled back.

After an external effect enters submitting, cancellation only prevents later steps; that effect still resolves to succeeded, failed_no_effect, or outcome_unknown. Late model/tool output cannot start new work after its step is closed.

Restart first restores durable occurrence claim, Run, LeaseRunUse, ApprovalUse/cost reservations, and original requests. Recovery of the same Run first proves continuity of the existing LeaseRunUse, then current authorization, exact Lease revision, trusted time, ActivationBinding, approval, and budgets; it never performs a second new-Run admission merely because maxRuns remaining is now zero. A paused Lease expires naturally without a cleanup task; once trusted time is after `notAfter`, new steps and final submissions are refused. If clock continuity cannot be proven, return `state_unavailable` and retain original control records without extending the deadline or assuming the Lease remains valid. If execution side-effect outcome cannot be proven, transition to blocked/reconciling rather than creating a new OperationId or effect ID.
## 21. Errors and unavailability semantics

D1 capability availability and its fixed reason precedence remain unchanged, especially `policy_denied` before component, configuration, network, version, and health details. D10 does not replace `missing_component`, `not_configured`, `offline`, `incompatible_version`, or `temporarily_unavailable` with one runtime_unavailable.

D10 control request stages are: closed decode/version → D1 static capability/surface/release → current principal/control-object visibility → delegation/data observation → deployment binding → exact input/approval → budget/audit → execution. Stage ordering precedes error specialization to prevent unauthorized probing.

The D10 control error closed set is `invalid_request|not_visible|control_conflict|binding_changed|approval_required|approval_expired|delegation_expired|delegation_exhausted|budget_exceeded|audit_unavailable|state_unavailable|invalid_output|cancelled|external_outcome_unknown`. Unknown tokens, wrong tag, wrong audience, and unauthorized control objects all return `not_visible`; expired, exhausted, or conflict detail is disclosed only when the caller may already read that caller-owned record.

Error ownership is determined by the boundary already entered:

| Failure location | Owner and normative result |
| --- | --- |
| Before formal D6 request, D10 finds no usable approval | D10 `approval_required` or `approval_expired` |
| Run admission finds Lease run count exhausted | D10 `delegation_exhausted`; D6/D7 is not entered |
| Lease has expired | D10 `delegation_expired`; if trusted time is unprovable, `state_unavailable` |
| After D6 entry, original D6 author permission/ObservationScope fails | original D6 `not_visible/preflight` |
| After D6 entry, business dependency/semantic/budget fails | original D6 code/disposition |
| After D6 entry, only associated standing/supplemental approval loses a race | companion amendment D6 `approval_unavailable/preflight`; unseen writes no ledger and planned remains planned |
| New planned-preview recovery transport fails | original D7 `d7_effects_error` with existing `not_visible|preview_expired|reset_required|effects_unavailable|budget_exceeded` codes as applicable |
| Committed effects read | existing `d7_effects_resolve/open` and original errors only |

A D10 probe therefore cannot eliminate an approval race after D6 entry and cannot hide a private submission result outside D6. The new `approval_unavailable` is a necessary coordinated extension of the D6 closed enum; request, receipt, and error object shapes remain unchanged, but the capability cannot be advertised available until the consumer supports the new enum.

A D6 `approval_unavailable/preflight` may be retried with the **same original request** after control conditions change: unseen retries only while the original preparation remains valid and new valid approval exists; planned resumes under §15.2 `PlannedDecisionApproval/1` or still-valid original approval. It never writes `semantic_rejected` and never creates a terminal decision merely because of TTL, cancellation, or temporary revocation.

Once a request enters an original D3/D6/D7/D8/D9 entrypoint, that owner returns its original error/disposition except for the explicitly proposed D6 enum extension above. D10 does not wrap it in a generic AgentError or reorder refusal based on internal deployment details.
## 22. Product surfaces

Local Desktop hosts the shared local Broker, scheduler, secret transport, and Core adapters; local CLI connects to or starts the same capability family rather than creating a second scheduler or authority. Remote Desktop/CLI call Server only.

Server hosts the managed Broker, scheduler, connector/model/tool executors, and secret transport while continuing to satisfy the D1 single actual commit-holder boundary; this generation does not define a multi-primary scheduler/database cluster. WebUI manages and initiates capabilities only through Server and directly runs no worker/model/connector and holds no secret.

Mobile returns D1 unsupported_surface for agent.session, automation, connector execution, conversion execution, and credential management. A "read-only monitor" cannot become a hidden approval/delegation path prohibited by D1. Mobile may consume ordinary committed workspace facts.

LTR/RTL, locale, screen reader, and Web/CLI transport differences affect presentation and interaction only and do not change request bytes, target, approval rules, cost, errors, or author/external outcomes.

## 23. D1-D9 composition and required amendments

D1 surfaces, capability reasons, and sole commit holder remain; D2 raw source/unknown-provider preservation remains; D3 identity/SourceBinding/OriginBinding/Provenance remains; D4 Registry exact shape/evolution remains; D5 gains no persistent Record; D8 Draft/explicit edit confirmation remains; D9 worker/ExportPlan/publication remains.

Only the D6/D7 standing-approval author-submit and planned-preview recovery branches need coordinated amendments, specified precisely in UPSTREAM-AMENDMENTS. They include the D6 `approval_unavailable/preflight` error extension, ApprovalUse reservation/terminal rules, and D7 planned-preview recovery transport. Existing upstream remains authoritative until joint acceptance and coordinated activation, so the product must keep unattended author commit unavailable.

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
