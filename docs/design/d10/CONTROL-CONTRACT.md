---
source_language: zh-CN
translation_of: CONTROL-CONTRACT.zh-CN.md
translation_status: synced
---

[简体中文](CONTROL-CONTRACT.zh-CN.md)

# D10 Control and Management Contract

revision: D10-r06-terminology-and-import-clarifications-2026-09-28; status: candidate author revision pending a complete independent joint final review. This file freezes the D10 management, authorization, idempotency, recovery, cost, stop, and package/contribution semantics required by R05. It does not modify fixed upstream S; every D6/D7 change exists only as an inactive proposal in UPSTREAM-AMENDMENTS.

## 1. Authority, scope, and error boundary

Core remains the sole author-transaction authority. Workspace author commits, saved decisions, receipts, and planned recovery remain owned by D6; this contract never stores a second author-success ledger. Deployment trust, packages, secrets, external accounts, pricing, and resource-use grants belong to the D10 host control domain and grant no Workspace content authority.

Public capabilities still pass the real D1 contractMajor, surface, release, policy, principal, component/configuration, reachability/version-combination, and health gates before entering this contract. Design acceptance, coordinated design activation, or the existence of a control record is not runtime available evidence.

The D10 control entrypoint error object is:

    D10ControlError/1 {
      kind:"d10_control_error", wireVersion:1,
      code:
        "invalid_request" | "not_visible" |
        "authority_unavailable" | "integrity_conflict" |
        "control_conflict" | "stale_revision" |
        "state_unavailable" | "budget_exceeded"
    }

These errors apply only to D10 prepare, host control, result, secret staging, and stop. Once a Workspace author commit enters D6, it continues to return the D6 closed error set; D10 does not wrap or rename D6 not_visible, approval_unavailable, execution_stopped, or transaction_aborted.

Absent object, wrong scope/audience, and current lack of object-observation authority are all not_visible. Unprovable authority/fence/custody is authority_unavailable; protected-record or evidence integrity conflict is integrity_conflict; same stable key with different complete input is control_conflict; an expected revision mismatch on a currently visible object is stale_revision; temporarily unprovable trusted time, external evidence, or required continuity is state_unavailable.

## 2. Shared closed types

The following are normative D10 generation-one contracts. Every object rejects unknown members, duplicate members, and illegal null. Counter reuses D6 exact 0..2^63-1 non-Boolean integer semantics; Uuid uses canonical lowercase RFC 4122 text. Every array is bounded, ordered, and unique as specified by its containing type.

    Version/1 = {
      major:Counter, minor:Counter, patch:Counter
    }

    VersionRange/1 = {
      minimum:Version/1,
      maximumExclusive:Version/1
    }

VersionRange requires minimum < maximumExclusive. There is no latest, wildcard, or ambient package version.

    Scope/1 =
      {kind:"workspace", workspaceRef:D3.WorkspaceRef}
    | {kind:"deployment", storeIncarnation:Uuid}

ControlRecordKind/1 is closed to:
The controlled record-kind set is `automation, lease, approval, planned_approval, external_approval, run, workspace_budget, activation, deployment_policy, trust, package, external_account, secret, grant, cost_account, pricing, reservation, external_effect, stop`.

    ControlRef<K>/1 = {
      storeIncarnation:Uuid,
      kind:K,
      id:Uuid
    }

    Binding<K>/1 = {
      ref:ControlRef<K>/1,
      revision:Counter
    }

    Target<K>/1 =
      {kind:"new"}
    | {kind:"existing", binding:Binding<K>/1}

Control IDs and incarnations are never reused. Retire, archive, revoke, close, or removal from a display surface never permits a later object to use the same ID. Re-creating the same display name receives a fresh ID.

    Money/1 = {
      currency:CurrencyCode,
      microUnits:Counter
    }

CurrencyCode is exact three-character ASCII A-Z. D10 performs no implicit foreign-exchange conversion.

Other primitive scalars reuse existing closed JSON semantics. `Token` is the non-empty opaque token from D6 §1; `Text` is a Unicode-scalar string; `Bytes` is a byte sequence bounded by the applicable entrypoint budget; `Boolean` accepts JSON true/false only; `Sha256` is `sha256:` plus 64 lowercase hex digits; `HostPrincipal` is a `Token` produced by trusted host authentication mapping. The current actor is never caller-supplied, but an H-authorized management operation may name another `HostPrincipal` in a policy, reconciler, or ResourceUseGrant **target-principal field**; that configures the authorized target and does not impersonate that principal as the caller. `Ed25519PublicKey` and `Ed25519Signature` appear only inside accepted package/trust adapters whose admitted profile fixes their encoding; an ordinary control caller cannot self-assert verification.

Controlled ASCII-token grammar:

```text
LowerCamelAscii = [a-z][A-Za-z0-9]{0,62}
LowerKebabAscii = [a-z][a-z0-9]*(?:-[a-z0-9]+)*
CanonicalInteger = "-"? ("0" | [1-9][0-9]*)
CanonicalDecimal = CanonicalInteger ("." [0-9]*[1-9])?
```

`LowerKebabAscii` is 1..63 bytes. `CanonicalInteger` and `CanonicalDecimal` must also satisfy the explicit bounds of the applicable ToolType. decimal forbids trailing zero, empty fractional part, exponent notation, and negative zero.

    ScheduleHorizon/1 = {
      start:D4.zoned_instant,
      endExclusive:D4.zoned_instant
    }

ScheduleHorizon is not a new D4 type. Both members use the exact temporal semantics of D4 zoned_instant and the same exact UTC comparator proves start < endExclusive. This document does not use the nonexistent name “D4 bounded-instant-range”.

## 3. Tool Value Profile

ToolValueProfile/1 is owned by the D10 Tool Adapter. It is not an alias for D7 TypeSpec and is not arbitrary JSON Schema. It provides only a bounded value algebra for external-tool arguments and results.

    ToolValueProfile/1 = {
      kind:"d10_tool_value_profile",
      wireVersion:1,
      input:ToolType/1,
      output:ToolType/1
    }

    ToolType/1 =
      {kind:"bool"}
    | {kind:"text", maximumUtf8Bytes:Counter}
    | {kind:"int64"}
    | {kind:"integer", minimum:CanonicalInteger, maximum:CanonicalInteger}
    | {kind:"decimal", minimum:CanonicalDecimal, maximum:CanonicalDecimal}
    | {kind:"optional", item:ToolType/1}
    | {kind:"object", members:[ToolMember/1]}
    | {kind:"list", minimum:Counter, maximum:Counter, item:ToolType/1}
    | {kind:"union", arms:[ToolArm/1]}

    ToolMember/1 = {
      name:LowerCamelAscii,
      required:Boolean,
      type:ToolType/1
    }

    ToolArm/1 = {
      tag:LowerKebabAscii,
      type:ToolType/1
    }

`maximumUtf8Bytes` is 1..8388608; an object has at most 64 `members`; a union has 2..8 `arms`; list `maximum` is 1..4096 with `minimum <= maximum`; complete type depth is at most 16 and canonical type bytes at most 65536. Member names and arm tags are sorted by UTF-8 bytes and unique. integer/decimal use canonical base-10 strings; binary float, NaN, Infinity, and negative zero are forbidden.

ToolValue/1 exact wire is:

```text
ToolValue/1 =
  {kind:"bool", value:Boolean}
| {kind:"text", value:Text}
| {kind:"int64", value:CanonicalInteger}
| {kind:"integer", value:CanonicalInteger}
| {kind:"decimal", value:CanonicalDecimal}
| {kind:"optional", value:{kind:"none"} | {kind:"some", value:ToolValue/1}}
| {kind:"object", members:[{name:LowerCamelAscii, value:ToolValue/1}]}
| {kind:"list", items:[ToolValue/1]}
| {kind:"union", tag:LowerKebabAscii, value:ToolValue/1}
```

ToolValue/1 matches the ToolType/1 bound at the call site recursively. object members are name-sorted and unique and contain only declared members; list length satisfies the bounds; union tag matches one declared arm. int64 also lies within signed 64-bit range.

Missing required members, extras, wrong arms, bounds violations, or budget overflow are invalid_request. ToolValue contains no EntityRef, Locator, SecretRef, file-path capability, ActionEvidence, plan/result token, open map, or executable value.

## 4. Package, Contribution, and dependencies

PackageId/1 and D4 SemanticNamespaceId are distinct types even when their strings match. PackageId uses 1..127 ASCII bytes of dot-separated lower-kebab segments. LocalContributionId, ExtensionPointId, and LocalOperationId each use one 1..63 ASCII-byte lower-kebab segment. All comparisons are exact ASCII.

    PackageManifest/1 = {
      kind:"d10_package_manifest",
      wireVersion:1,
      packageId:PackageId,
      packageVersion:Version/1,
      publisherKeyId:Sha256,
      assets:[PackageAsset/1],
      contributions:[Contribution/1]
    }

    PackageAsset/1 = {
      assetId:LowerKebabAscii,
      digest:Sha256,
      byteLength:Counter
    }

    Contribution/1 = {
      contributionId:LocalContributionId,
      kind:ContributionKind,
      contractVersion:Version/1,
      descriptorAssetId:LowerKebabAscii,
      dependencies:[ContributionDependency/1]
    }

ContributionKind is closed to:
The Contribution-kind closed set is `module, schema, view, action, template, preset, pack, tool, model, connector, importer, exporter, conversion, renderer, localization`.

This set covers the Mandatory Intake-selected module, Profile/schema, View, Action, Template, Preset, Pack, Connector/Adapter, D9 import/export/conversion, and D10 tool/model/connector contribution families. `runtime` is not an independent `ContributionKind`; it denotes D10 runtime infrastructure used by those executable Contributions. The kind list does not claim that every category is product-implemented.

    ContributionDependency/1 = {
      role:"primary_parent" | "additional",
      parent:{
        packageId:PackageId,
        contributionId:LocalContributionId,
        extensionPointId:ExtensionPointId
      },
      requiredContractRange:VersionRange/1
    }

A dependency belongs to one concrete dependent Contribution; there is no ambient package-level dependency. A pack Contribution has exactly one primary_parent. Other Contributions have at most one primary_parent and 0..32 additional dependencies. The parent tuple resolves to one exact activated Contribution and extension point. The resolved parent contract version and dependency result enter the Capability Catalog digest and never follow latest.

An unavailable connector Contribution makes only that Contribution inactive. A template, schema, pack, or view in the same package is evaluated from its own dependencies and capabilities. Definition history, Contribution activation, and UI visibility remain three separate axes.

Each kind’s descriptor is validated by its existing semantic owner: schema references existing D4 Registry namespace/Facet bindings; view/action consume D7 closed descriptors; template/preset/importer/exporter/conversion consume D9 contracts; D10 owns tool/model/connector descriptors and their runtime infrastructure; module only organizes product contributions and owns no author facts.

## 5. Unique package mapping for four first-party modules

This section is a new D10 candidate mapping. It does not claim that D1 already defined package/module IDs, code paths, or locale resources. D4-frozen semantic namespaces, owner tuples, FacetIds, and semanticMajor remain unchanged.

| Product | PackageId | module contribution | schema contribution | D4 semantic owner / Facet | extension points |
| --- | --- | --- | --- | --- | --- |
| Calendar / 日历 | `weftext.calendar` | `module` | `schema` | `calendar` → `first_party,weftext.calendar`; `calendar/period-note`, `calendar/range-note`, `calendar/event`, `semanticMajor=1` | `calendar-system`, `holiday-schedule` |
| Library / 文献库 | `weftext.library` | `module` | `schema` | `library` → `first_party,weftext.library`; `library/work`, `semanticMajor=1` | `none` |
| People / 人物 | `weftext.people` | `module` | `schema` | `people` → `first_party,weftext.people`; `people/person`, `semanticMajor=1` | `none` |
| Organizations / 组织 | `weftext.organizations` | `module` | `schema` | `organizations` → `first_party,weftext.organizations`; `organizations/organization`, `semanticMajor=1` | `schema-pack` |

Each module descriptor contains only the formal Chinese/English product names, the same-package schema-contribution reference, and the extension points above. A schema descriptor lists only exact SemanticNamespaceId, D4 namespace ownerId, and FacetId/semanticMajor; it does not copy FieldDefinition or FacetSchema bytes.

PackageId, Contribution contractVersion, and D4 semanticMajor are three independent version domains. A package update cannot change the D4 semantic digest under the same FacetId/semanticMajor.

Candidate code conventions are CalendarModuleContribution, LibraryModuleContribution, PeopleModuleContribution, and OrganizationsModuleContribution; candidate variables are calendar_module, library_module, people_module, and organizations_module. Candidate code namespace d10::bundled::<domain> and candidate locale keys module.calendar.title, module.library.title, module.people.title, and module.organizations.title are design mappings with no current implementation evidence. There is no independent CLI verb; generic management consumes the complete package/contribution reference.

A D4 ownerId is never automatically package-ownership proof. The table creates an explicit mapping only. Third-party signature, install order, display name, or an identical string cannot acquire first-party package/namespace ownership.

## 6. Principal, management domains, and ResourceUseGrant

Requests never carry principal, owner, or an authorized Boolean. Current identity only comes from trusted host/D10 authentication mapping.

Workspace self-service qualification S is an explicit allow of proposed D6 Policy/2 capability d10_control_self for the current principal at workspace scope. It only allows management of that principal’s own finite D10 control records and grants no author read/write, policy_admin, registry_admin, deployment-account management, or secret plaintext. Actual author operations still require their original D6/D7 permissions.

Workspace-management qualification W is policy_admin at workspace scope in the current Workspace. Registry activation additionally requires registry_admin. W may stop, revoke, archive, and manage Workspace budgets, but cannot impersonate another principal to create or enlarge that principal’s Lease/Approval.

Deployment-management qualification H is owned by DeploymentControlPolicy/1:

    DeploymentControlPolicy/1 = {
      kind:"d10_deployment_control_policy",
      wireVersion:1,
      revision:Counter,
      administrators:[HostPrincipal],
      reconcilers:[{
        principal:HostPrincipal,
        costAccountIds:[ControlRef<cost_account>/1]
      }]
    }

Initial H comes only from explicit trusted-local-host initialization or Server deployment-operator configuration. First HTTP request, Workspace ownership, allocate_workspace, or administer_issuer does not imply H. A reconciler may only perform evidence-driven settlement on listed cost accounts.

    ResourceUseGrant/1 = {
      kind:"d10_resource_use_grant",
      wireVersion:1,
      grantId:Uuid,
      grantRevision:Counter,
      usageRevision:Counter,
      state:"active" | "revoked" | "retired",
      grantee:{
        hostPrincipal:HostPrincipal,
        workspacePrincipal:Token,
        workspaceRef:D3.WorkspaceRef
      },
      contributions:[ContributionBinding/1],
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      permission:ResourcePermission/1,
      usage:ResourceUsage/1
    }

    ContributionBinding/1 = {
      packageId:PackageId,
      packageVersion:Version/1,
      contributionId:LocalContributionId,
      contractVersion:Version/1,
      descriptorDigest:Sha256
    }

    ResourceUseGrantSpec/1 = {
      grantee:{
        hostPrincipal:HostPrincipal,
        workspacePrincipal:Token,
        workspaceRef:D3.WorkspaceRef
      },
      contributions:[ContributionBinding/1],
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      permission:ResourcePermission/1
    }

`ResourceUseGrantSpec/1` is the create/update input. `grantId`, both revisions, state, and usage are produced only by Core from the existing record and transaction result.

ResourcePermission/1 has exactly four variants:

    {kind:"cost",
     account:Binding<cost_account>/1,
     limits:{currency:CurrencyCode,
             totalMicroUnits:Counter,
             perAttemptMicroUnits:Counter,
             maxAttempts:Counter}}

    {kind:"secret",
     secret:Binding<secret>/1,
     account:Binding<external_account>/1,
     audience:ContributionBinding/1,
     usageKind:"authenticate",
     limits:{maxUses:Counter}}

    {kind:"egress",
     account:Binding<external_account>/1,
     operations:[LocalOperationId],
     limits:{maxCalls:Counter,
             totalBytes:Counter,
             perCallBytes:Counter}}

    {kind:"external_effect",
     account:Binding<external_account>/1,
     calls:[{operation:LocalOperationId,target:ToolValue/1}],
     limits:{maxAttempts:Counter}}

One permission kind is present per grant. secret does not imply egress, egress does not imply mutation, and cost does not imply account administration. cost requires perAttemptMicroUnits <= totalMicroUnits; egress requires perCallBytes <= totalBytes. Executable limits are finite positive values; cost amounts may be zero.

ResourceUsage/1 matches the permission kind:
cost={spentMicroUnits,heldMicroUnits,attemptsStarted};
secret={usesStarted};
egress={callsStarted,bytesSent,bytesHeld};
external_effect={attemptsStarted}.
These counters plus outstanding attempt reservations are admission dependencies.

Renewing, narrowing, or changing grantRevision under the same grantId never clears usage, held, spent, or attemptsStarted; usageRevision advances independently. A reduced limit cannot be below already consumed plus held usage. Changing grantee, resource kind, cost account, external account, or currency requires a new grantId. A new grant never erases old attempt, old grant, or actual-account liabilities, and every grant still competes for the same actual account ceiling.

## 7. Stable prepare, closed body, and public entrypoints

The ordinary control entrypoint is:

    D10ControlPrepare/1 = {
      kind:"d10_control_prepare",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      body:ControlBody/1
    }

ControlBody/1 has exactly seven variants:

1. automation_configure:
   automation:Target<automation>;
   lease:Target<lease>;
   approval:Option<Target<approval>>;
   definition:AutomationSpec/1;
   delegation:LeaseSpec/1;
   standing:Option<StandingApprovalSpec/1>.
   approval and standing are both none or both some. This fixed bundle is the only combined operation for creating/rebinding Automation, Lease, and Standing Approval and is not a generic batch/DAG.

2. consent:
   target:Target<planned_approval|external_approval>;
   consent:ConsentSpec/1.

3. state:
   target:Binding<K>;
   action:StateAction.
   Closed applicability:
   automation→enable|disable|archive;
   lease/approval/grant→revoke|archive;
   run→cancel|archive;
   package/pricing→retire;
   external_account→disconnect;
   cost_account→close;
   secret→revoke.
   There is no generic delete or revoked→active transition.

4. workspace_limits:
   target:Binding<workspace_budget>;
   limits:BudgetCaps/1.
   W only.

5. activation:
   target:Target<activation>;
   packages:[ContributionBinding/1];
   registrySnapshot:D4.RegistrySnapshot/1;
   registryEvolution:Option<D4.RegistryEvolutionProof/1>;
   trust:Binding<trust>.
   W plus registry_admin only. A non-bootstrap Registry requires evolution proof.

6. deployment_put:
   target:Target<K>;
   value:DeploymentValue/1.
   K is only `deployment_policy|trust|package|external_account|secret|grant|cost_account|pricing`, must match `value.kind`, and is available only to H.

7. cost_reconcile:
   reservation:Binding<reservation>;
   evidence:EvidenceTicket/1.
   H or the account’s reconciler only; no targetState, actual, or manual-amount field exists.

Option<T> is only {kind:"none"} or {kind:"some",value:T}; null is forbidden.

    BudgetCaps/1 = {
      maxSteps:Counter,
      maxInputBytes:Counter,
      maxOutputBytes:Counter,
      maxElapsedMillis:Counter,
      costs:[{
        grant:Binding<grant>/1,
        maximum:Money/1
      }]
    }

All step/input/output/elapsed maxima are finite positive values. costs is sorted and unique by grant ref and currency matches the grant/account.

    AutomationSpec/1 = {
      label:Text,
      invocation:{
        contribution:ContributionBinding/1,
        parameters:ToolValue/1
      },
      schedule:AutomationSchedule/1,
      missedPolicy:"skip" | "run_once",
      queueLimit:Counter,
      budgets:BudgetCaps/1
    }

    AutomationSchedule/1 =
      {kind:"once", at:D4.zoned_instant}
    | {kind:"recurrence",
       ownerNodeRef:D3.NodeRef,
       recurrenceOccurrenceKey:D4.occurrenceKey,
       rangeOccurrenceKey:D4.occurrenceKey,
       horizon:ScheduleHorizon/1,
       outputLimit:Counter}

parameters must validate against the active ToolValueProfile/1 input type of the Contribution. A recurrence selector only locates D4 recurrence source rebound to the same current revision and never becomes a durable EntityRef.

    LeaseReadGrant/1 = {
      scope:<exact D6 Policy/2 grant.scope from S D6 Control §4>,
      capabilities:[LeaseReadCapability/1]
    }

    LeaseReadCapability/1 =
      {kind:"field_read", fieldIds:[D4.FieldId]}
    | {kind:
        "workspace_state" | "entity_state" | "locator_state" |
        "source_read" | "resource_read" | "annotation_read" |
        "source_envelope_state"}

`LeaseReadGrant/1` is a D10 attenuation projection rather than a new D6 grant wire. It consumes the exact scope/capability decoder and scope-applicability matrix from S D6 Control §4: for example `workspace_state` is workspace-scope only, `entity_state|locator_state|source_envelope_state` retain the original workspace/ref_set rules, and Field read uses the original D6 `field_read` shape. It forbids write/admin/repair/audit/export, `commit_sequence_state`, and `d10_control_self`, so it can only narrow read/state-observation authority the actor already possesses.

    LeaseSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      maxRuns:Counter,
      activationBinding:ActivationBinding/1,
      capabilityAllowlist:[D1.CapabilityId],
      readGrants:[LeaseReadGrant/1],
      resourceGrants:[Binding<grant>/1],
      budgets:BudgetCaps/1
    }

`readGrants` accepts only `LeaseReadGrant/1` above; actual authorization is still computed from current D6 Policy with its original deny precedence, and a Lease cannot add authority. `notBefore < notAfter` and `maxRuns` is finite positive.

    StandingApprovalSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      rule:SingleFieldMemberRule/1,
      maxSuccessfulCommits:Counter,
      costGrants:[Binding<grant>/1]
    }

`SingleFieldMemberRule/1` is the controlled D10 internal projection of Candidate §14 `StandingApprovalEnvelope/1.rule`, not an independent D6/D7 wire. It exactly carries that section: action `set_field_member`; one member of the unique Entry under one owner/Field; value-type closed set `text|bool|int64|integer|decimal`; constraint closed set `any|enum|numeric_range`; and the raw-no-op branch. Any change must first amend Candidate §14 and then synchronize this projection. It never expands to create, delete, Facet, native-table, or bulk operations.

    ConsentSpec/1 =
      {kind:"planned",
       originalRequest:D6.d6_commit_request,
       previewSemanticDigest:Sha256,
       lease:Binding<lease>/1,
       activationBinding:ActivationBinding/1,
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}
    | {kind:"external",
       intent:Binding<external_effect>/1,
       requestDigest:Sha256,
       resourceGrants:[Binding<grant>/1],
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}

planned authorizes only the original planned request and never reprepares, changes OperationId, or changes target. external authorizes only the original ExternalEffectIntent.

A successful prepare returns:

    D10ControlPrepared/1 = {
      kind:"d10_control_prepared",
      wireVersion:1,
      requestId:Uuid,
      prepareToken:Token,
      preview:ControlPreview/1,
      commit:
        {kind:"workspace", request:D6.d6_commit_request}
      | {kind:"deployment",
         request:{kind:"d10_host_control_commit",
                  wireVersion:1,
                  scope:Scope/1,
                  requestId:Uuid,
                  prepareToken:Token}}
    }

    ControlPreview/1 = {
      kind:"d10_control_preview",
      wireVersion:1,
      canonicalIntentBytes:Bytes,
      affected:[{
        ref:ControlRef<K>/1,
        change:"create" | "update" | "enable" | "disable" |
               "revoke" | "cancel" | "archive" | "retire" |
               "disconnect" | "close" | "settle",
        beforeRevision:Option<Counter>,
        proposedAfterRevision:Option<Counter>
      }],
      resourceUses:[{
        grant:Binding<grant>/1,
        maximum:Option<Money/1>
      }]
    }

affected is sorted/unique by ref canonical bytes; resourceUses is sorted/unique by grant ref. canonicalIntentBytes is the complete successfully closed-decoded D10-Control-Intent/1 byte sequence, so it carries the proposed control semantics already supplied by and visible to the caller. preview adds no secret bytes, hidden author value, or another user's billing.

ControlPreview/1 contains only control metadata currently visible to the principal. Budget overflow rejects rather than truncates.

Result query:

    D10ControlResultRequest/1 = {
      kind:"d10_control_result",
      wireVersion:1,
      scope:Scope/1,
      requestId:Uuid
    }

It returns the currently disclosable prepare/apply history and never re-executes the operation or claims that the historical after revision is still current.

## 8. Idempotency, CAS, and replay order

The stable key is (scope incarnation, initiating principal, requestId). The client persists requestId before the first call and reuses it for transport retry.

Core internally stores:

    StableControlKey/1 = {
      scope:Scope/1,
      initiatingPrincipal:Token,
      requestId:Uuid
    }

    PreparedCommitRequest/1 =
      {kind:"workspace", request:D6.d6_commit_request}
    | {kind:"deployment", request:{
        kind:"d10_host_control_commit",
        wireVersion:1,
        scope:Scope/1,
        requestId:Uuid,
        prepareToken:Token
      }}

    ControlDependencies/1 = {
      configBindings:[Binding<K>/1],
      usageBindings:[{
        ref:ControlRef<K>/1,
        usageRevision:Counter
      }],
      authorityProof:Token,
      authorizationGenerations:[Token],
      stopRefs:[ControlRef<stop>/1],
      sourceOrRegistryBindings:[Sha256]
    }

`ControlDependencies/1` is the complete internal dependency set Core derives from actual authorized reads and is never caller input. Arrays are sorted/unique by complete canonical bytes. sourceOrRegistryBindings stores binding digests owned by existing source/Registry contracts and creates no new author or Registry token.

    ControlPrepareBinding/1 = {
      key:StableControlKey/1,
      canonicalIntentBytes:Bytes,
      allocatedControlRefs:[ControlRef<K>/1],
      originalCommitRequest:PreparedCommitRequest/1,
      immutablePreview:ControlPreview/1,
      dependencyPins:ControlDependencies/1
    }

canonicalIntentBytes is the complete D3-CJ/3 canonical byte sequence after successful closed decode with domain tag D10-Control-Intent/1. A digest may index it but conflicts compare complete bytes. The binding stabilizes preparation and lookup and is not a second author decision.

ControlDependencies/1 internally and exactly contains the config bindings, usage bindings, authority/fence/custody proof, authorization generations, stop refs, and required source/Registry bindings actually read by Core. The client cannot assert completeness.

The shared order is fixed:

1. closed decode plus D1 capability/release/surface/version/health gates;
2. use minimal protected mappings to validate current principal, scope, audience, observation, and operation authority; unauthorized, wrong-audience, and absent all become not_visible;
3. prove authority/fence/custody/continuity;
4. look up the stable key; visible same-key different canonical bytes → control_conflict;
5. for same-key same-input with an authoritative saved decision, revalidate current disclosure authority for the original result/effect scope and then replay the saved result; current target revision does not invalidate historical success;
6. only an undecided operation validates expected config/usage revisions, current authority, dependencies, time, and budget and constructs or resumes the original preparation;
7. Workspace submission enters the original D6 transaction/ledger; Deployment submission enters the closed host-control transaction in the same store incarnation;
8. atomically save effect, decision/receipt linkage, record/account deltas, dependency invalidation, evidence pins, and required audit linkage.

If another valid operation changes an object r5→r6 after the original r5 operation committed but its response was lost, retry of the original request returns the saved r5 outcome after current disclosure authorization; it neither recreates the object nor incorrectly fails stale. Reading current state is a different authorized read. If the caller later loses result-disclosure authority, result query returns not_visible while the saved decision remains immutable.

A failed prepare/commit creates no applied decision. An existing prepare binding may resume the same exact intent after transient state_unavailable. Changing expected revision, body, or target requires a fresh requestId. Dedup bindings, terminal proof, and planned/unknown/uncertain pins cannot be TTL-deleted so that an old requestId executes again.

Configuration revision and usageRevision are distinct. Checked increment at Counter maximum returns budget_exceeded and never wraps or resets. Retired IDs/incarnations are never reused, preventing ABA.

## 9. Deployment values and evidence

    DeploymentValue/1 =
      {kind:"deployment_policy", value:DeploymentControlPolicy/1}
    | {kind:"trust", publisherId:Text, publicKey:Ed25519PublicKey,
       previous:Option<Binding<trust>/1>,
       proof:EvidenceTicket/1,
       claims:[NamespaceClaim/1]}
    | {kind:"package", manifest:PackageManifest/1,
       signature:Ed25519Signature}
    | {kind:"external_account",
       provider:ContributionBinding/1,
       externalAccountId:Text,
       endpointId:LocalOperationId,
       proof:EvidenceTicket/1}
    | {kind:"secret",
       account:Binding<external_account>/1,
       audience:ContributionBinding/1,
       usageKind:"authenticate",
       staged:SecretStageTicket/1}
    | {kind:"grant", spec:ResourceUseGrantSpec/1}
    | {kind:"cost_account",
       currency:CurrencyCode,
       ceiling:Counter,
       pricing:Option<Binding<pricing>/1>}
    | {kind:"pricing",
       account:Binding<external_account>/1,
       currency:CurrencyCode,
       fixedMicroUnits:Counter,
       meters:[PricingMeter/1],
       evidence:EvidenceTicket/1}

    NamespaceClaim/1 = {
      namespaceId:D4.SemanticNamespaceId,
      ownerClass:"first_party" | "publisher",
      ownerId:Token
    }

`NamespaceClaim/1` only binds a verified PublisherIdentity to an existing D4 semantic-namespace ownership claim. It creates no second namespace registry and cannot override `core`, `wf`, or another reserved owner.

    PricingMeter/1 = {
      meterId:LocalOperationId,
      numerator:Counter,
      denominator:Counter,
      maxUnits:Counter
    }

`denominator` and `maxUnits` are positive and meterId is ASCII-sorted and unique within one pricing record. Maximum cost uses checked integer/rational arithmetic and never binary float.

EvidenceTicket/1 is:

    {ticketId:Token,
     evidenceClass:
       "publisher_rotation" | "namespace_claim" |
       "account_control" | "pricing_contract" |
       "final_bill" | "never_started",
     evidenceDigest:Sha256}

A trusted adapter creates the ticket and internally binds complete original evidence, principal, scope, provider/account/attempt. An ordinary caller cannot assert verified or reuse one evidenceClass as another.

Secret plaintext uses only the trusted secret channel:

    D10SecretStageRequest/1 = {
      kind:"d10_secret_stage",
      wireVersion:1,
      requestId:Uuid,
      account:Binding<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretBytes:Bytes
    }

Only H may stage. secretBytes never enter the ordinary control canonical intent, preview, log, transcript, or Workspace. Success returns:

    SecretStageTicket/1 = {
      ticketId:Token,
      account:ControlRef<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretVersionId:Token
    }

Staging has its own same-principal/store/requestId stable key. The trusted secret store replays the original ticket only when it can prove the exact same secret input. The ordinary database transaction publishes only the immutable secret version referenced by the ticket.

DeploymentControlDecision/1 is the host-domain success record:

    DeploymentControlDecision/1 = {
      key:StableControlKey/1,
      canonicalIntentBytes:Bytes,
      changes:[{
        ref:ControlRef<K>/1,
        beforeRevision:Option<Counter>,
        afterRevision:Counter
      }],
      evidence:[EvidenceTicket/1],
      auditRef:Token
    }

Only an atomically successful commit saves a decision. Preflight/authorization/CAS/evidence/overflow failure saves no applied decision. changes exactly covers configuration changes and is empty for a true no-op. Usage/account/reservation deltas commit in the same transaction as the decision.

## 10. Cost reservation and recoverable settlement

    CostReservation/1 = {
      reservationId:Uuid,
      attemptId:Uuid,
      account:Binding<cost_account>/1,
      grant:Binding<grant>/1,
      pricing:Binding<pricing>/1,
      currency:CurrencyCode,
      upperBound:Money/1,
      revision:Counter,
      state:"reserved" | "uncertain" | "settled" | "released",
      actual:Option<Money/1>
    }

`actual` is some only when state=settled and is none in every other state.

`attemptId` and `reservationId` are never reused.

State machine:

    reserved -> settled(actual) | released | uncertain
    uncertain -> settled(actual) | released

settled and released are terminal. uncertain is recoverable non-terminal and continues to occupy the complete upperBound.

released is legal only when reliable never_started evidence proves that billable execution/send for this attempt never began. An actually sent attempt whose reliable final bill is zero is settled(0), not released. Reliable final_bill must be attributable to the same reservation/attempt/account/currency/pricing and produces settled(actual). For reserve100→uncertain→final bill20, the only result is settled(20), returning 80.

    CostSettlementDecision/1 = {
      reservationId:Uuid,
      attemptId:Uuid,
      priorRevision:Counter,
      kind:"settled" | "released",
      actual:Option<Money/1>,
      evidence:EvidenceTicket/1
    }

`settled` requires `actual=some`. `released` requires `actual=none` and `evidenceClass=never_started`. `final_bill` can produce only settled. Wrong attempt/account/currency, non-final evidence, an aggregate bill that cannot be uniquely split, or insufficient continuity leaves uncertain with the complete bound and returns `state_unavailable`. Malformed accepted-adapter output follows its existing invalid-output contract and never guesses a result.

Reconcile uses CAS on the expected reservation revision. Success atomically appends CostSettlementDecision, updates reservation state/revision, grant/account held/spent/available projections, and evidence/audit. Exact evidence/prior-revision/derived-decision replay does not return capacity twice; concurrent different decisions have at most one CAS winner. actual above upperBound follows the existing overcharge anomaly/freeze path and normal reconciliation never raises the ceiling.

Administrator-entered zero, effect idempotency, business rollback, author abort, TTL, or Run terminal state is not released/settled evidence. ApprovalUse count, LeaseRunUse, and cost reservation remain three separate domains.

## 11. Emergency stop and linearization

Public stop:

    D10EmergencyStopRequest/1 = {
      kind:"d10_emergency_stop",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      target:
        ControlRef<automation>/1
      | ControlRef<run>/1
    }

An ordinary owner may stop the exact owned automation/run; W is Workspace-scoped and H Deployment-scoped. Stop requires no target configuration revision, so ordinary configuration-Counter exhaustion cannot block it. It requires the exact ID/incarnation and current stop authority.

Before first enable/admission every executable object reserves one durable ExecutionStopLatch/1:

    ExecutionStopLatch/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal>,
      stoppedAtControlSequence:Option<Counter>
    }

open→stopped is one-way and there is no clear operation. Repeated stop is idempotent. Stop consumes no maxRuns, ApprovalUse, cost, ordinary management quota, or executor budget.

Linearization rules:

1. New Run admission and stop share the same store serialization domain. The admission transaction checks applicable latches and atomically writes occurrence claim/LeaseRunUse. If stop commits first there is no admission. If admission commits first, its consumed count remains and later steps still check stop.
2. D6 final author commit rechecks every stop latch referenced by the protected RunBinding inside the actual final transaction while holding its write serialization. A prepare-time or out-of-transaction check is insufficient. Commit first preserves the committed result; stop first prevents the new commit.
3. External transport and stop share one send fence. Inside the fence the transport rechecks latches, durably stores ExternalEffectIntent/started evidence and cost hold, and retains the fence through the first irreversible real send handoff. Queue admission is not send linearization. The fence is released before waiting for a remote response. Stop first means no send; send handoff first preserves sent fact and may later produce outcome_unknown.
4. After crash, restore latch, started/send evidence, reservation, and decision before restoring the executor. Insufficient evidence cannot turn possibly-sent into cancelled.
5. Stop does not block currently authorized authoritative abort, cost settlement, evidence retention, audit retention, or reference-safe cleanup. Those operations do not regain executor authority.
6. Temporary disable, Lease expiry, temporary authorization loss, or ordinary cancel is not irreversible abort proof.

The coordinated D6 amendment adds execution_stopped/preflight. An unseen D10-bound author request blocked by irreversible stop after current authorization/ObservationScope succeeds receives it and writes no author decision. A planned request may reach the original transaction_aborted/terminal authoritative-abort path only after current authorization, continuity, complete RunBinding, and irreversible stop are all proven. The same transaction follows the existing ApprovalUse-count release rule; cost is not automatically released.

## 12. D6 Policy/2 and bootstrap profile/3 proposal boundary

Fixed-S Policy/1 decoder, existing Policy/2 capabilities, and all saved policy/decisions remain unchanged. R05 proposed Policy/2 adds closed no-argument capability d10_control_self at workspace scope only. It is implied by no Field/source/policy_admin capability and implies nothing else.

Under the amendment, fixed-S profile/2 “all non-Field capabilities” is frozen to the set present in S:

The frozen set is `workspace_state, entity_state, locator_state, source_read, source_write, body_write, node_control, node_create, resource_read, resource_write, annotation_read, annotation_write, lifecycle, registry_admin, binding_admin, policy_admin, export, repair, audit, source_envelope_state, commit_sequence_state`.

profile/2 never automatically gains later d10_control_self.

Add d6_bootstrap_profile wireVersion=3 with the same member shape as S profile/2. profile/3 still creates Policy/2; the initial creator grant equals the frozen set above plus d10_control_self, plus the original S rule that derives all target-Registry Field read/write grants. deny remains empty.

Only an explicit issuer-profile update by current administer_issuer affects subsequently issued families. Existing families keep their stored profile/1 or profile/2 copies; replacement, WorkspaceBootstrapPlan, saved decisions, replay/continue/failover never recompute or add grants. An existing Workspace gains d10_control_self only through an explicit current policy_admin modification under the original Policy-management path. A Field-authorized principal cannot self-grant it.

## 13. Public capability and first-public D6 error compatibility

Candidate capability IDs defined by D10 and discovered/published by D1 are:

- automation.manage
- workspace.extensions.manage
- deployment.external.manage
- automation.stop
- automation.author_submit

These IDs must enter the selected D1 contractMajor’s official capability catalog and still pass every real release/surface/policy/version/component/configuration/reachability/health gate. Document acceptance or coordinated design activation does not make them runtime available.

The first jointly specified public unattended author-submit contract directly includes D6 approval_unavailable/preflight and execution_stopped/preflight. There is no anonymous old/new D10 error profile. Unknown capability ID remains D1 unsupported_feature; a disallowed component-version combination remains D1 incompatible_version. Fixed D6 Policy/1/2, bootstrap profile/1/2, and historical saved-decision decoders are not removed by deleting unpublished D10-profile wording.

## 14. Atomicity, audit, and retention

A Workspace control change that affects an author decision enters the original D6 authority-store transaction. Deployment host control changes only Deployment records in the closed host transaction for the same store incarnation. No contract assumes atomicity across independent databases, HTTP services, or ATTACH/WAL.

Every successful protected control operation atomically saves record/config change, usage/account delta, decision/receipt linkage, dependency invalidation, evidence pins, and required audit link. Ordinary protected execution fails closed if its required audit-start fact cannot be durably stored. Emergency stop uses a separate safety slot reserved at enable/admission and is not blocked by ordinary audit/budget quota exhaustion.

Retire/archive does not delete a record still referenced by a saved decision, planned recovery, unknown external effect, uncertain cost, evidence, tombstone/migration, or active binding. Capacity exhaustion may reject a new ordinary operation; it cannot delete dedup history so an old requestId runs again.

## 15. R05 acceptance counterexamples

At minimum verify:

1. A narrow-Field user with d10_control_self, actual Field authority, and an administrator-issued cost grant can create a finite owned automation; missing self capability or grant rejects clearly.
2. An r5 stable-key write committed but response was lost; another request changed the object to r6; retry returns saved r5 result after current disclosure authorization, without duplicate creation or false stale rejection.
3. Same key with changed target/body/expected revision returns control_conflict; delete/recreate under the same display name never captures the old request.
4. Grant renewal/revision does not clear spent/held/attempts; a replacement grant does not erase old reservation/account liability.
5. reserve 100, actually send, crash→`uncertain`; a same-attempt final bill of 20 produces `settled(20)` and returns only 80; a final bill of 0 produces `settled(0)`; only never-started proof produces `released`.
6. Two reconcilers race one reservation: at most one CAS winner; response loss after commit replays without double return.
7. Every ordering of stop versus Run admission, D6 final commit, and external send fence; settlement/authoritative abort/evidence cleanup still works after stop.
8. A profile/2 family does not gain d10_control_self after software upgrade; explicit profile/3 affects only later families; existing Workspaces require explicit policy_admin grant.
9. Unavailable connector Contribution does not disable same-package schema/template/data-pack Contribution.
10. Calendar/Library/People/Organizations PackageId, module Contribution, schema Contribution, and D4 namespace/Facet map one-to-one without type conflation; a same-named third-party package gains no first-party D4 owner.
11. Design accepted but release not shipped, Mobile unsupported, policy denied, component combination disallowed, or provider unhealthy still yields the real D1 unavailable result.
12. After a caller loses result-disclosure authority, same-key result query is not_visible while the saved decision remains preserved.
