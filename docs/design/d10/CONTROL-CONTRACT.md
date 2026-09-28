---
source_language: zh-CN
translation_of: CONTROL-CONTRACT.zh-CN.md
translation_status: synced
---

[简体中文](CONTROL-CONTRACT.zh-CN.md)

# D10 Control and Management Contract

revision: D10-r08-joint-review-fixes-2026-09-28; status: complete R08 author-revised candidate after the complete independent joint review of fixed R07 C=`cf46461848d5dfe4dcd0f482ede934243cd098a4` against S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` returned REVISE (P0=0, P1=1, P2=10). All eleven findings remain open. The candidate is complete for author handoff and awaits a fresh complete independent joint review of the fixed R08 commit; it is not independently accepted, activated, merged, released, or product implementation evidence.

## 1. Authority, scope, and error boundary

Core remains the sole author-transaction authority. Workspace author commits, saved decisions, receipts, and planned recovery remain owned by D6; this contract never stores a second author-success ledger. Deployment trust, packages, secrets, external accounts, pricing, and resource-use grants belong to the D10 host control domain and grant no Workspace content authority.

Public capabilities still pass the real D1 contractMajor, surface, release, policy, principal, component/configuration, reachability/version-combination, and health gates before entering this contract. Design acceptance, coordinated design activation, or the existence of a control record is not runtime available evidence.


The D10 management/control error object is:

    D10ControlError/1 {
      kind:"d10_control_error", wireVersion:1,
      code:
        "invalid_request" | "not_visible" |
        "authority_unavailable" | "integrity_conflict" |
        "control_conflict" | "stale_revision" |
        "state_unavailable" | "budget_exceeded"
    }

The separate D10 Run/step error object is:

    D10RunStepError/1 {
      kind:"d10_run_step_error", wireVersion:1,
      code:
        "invalid_request" | "not_visible" |
        "control_conflict" | "binding_changed" |
        "approval_required" | "approval_expired" |
        "delegation_expired" | "delegation_exhausted" |
        "budget_exceeded" | "audit_unavailable" |
        "state_unavailable" | "invalid_output" |
        "cancelled" | "external_outcome_unknown"
    }

`D10ControlError/1` is used only by `d10_control_prepare`, `d10_host_control_commit`, `d10_control_result`, `d10_control_read`, `d10_secret_stage`, `d10_emergency_stop`, and `d10_emergency_stop_result`. `D10RunStepError/1` is used only while a D10-owned Run/step has not entered another protocol owner: Run admission, ContextBundle/model/tool/connector execution, pre-D6 approval/delegation checks, and external-effect execution/recovery.

Once a request enters D3, D6, D7, D8, or D9, that owner returns its original closed error/envelope unchanged. D10 never wraps D6 `not_visible`, `approval_unavailable`, `execution_stopped`, or `transaction_aborted`; never wraps D7 action/effects errors; and never renames D3/D8/D9 errors. A D10 diagnostic UI may perform a separately authorized `d10_control_read` after the owner error, but that read cannot alter the formal result.

Management/control ordering is fixed: closed decode and D1 capability gates → current principal/scope/audience/object-observation authority → authority/fence/custody → stable-key conflict or exact ControlRef lookup → protected-record integrity/continuity → current-revision/dependency/budget checks only when applicable. Absent object, wrong scope/audience, and current lack of object-observation authority are all `not_visible`. Unprovable authority/fence/custody is `authority_unavailable`; a proven protected-record/evidence contradiction is `integrity_conflict`; same stable key with different complete input is `control_conflict`; an expected configuration revision mismatch on a currently visible object is `stale_revision`; temporarily unprovable trusted time, external evidence, or required continuity is `state_unavailable`.

Run/step ordering remains Candidate §21: closed decode/version → D1 static capability/surface/release → current principal/control visibility → delegation/data observation → deployment binding → exact input/approval → budget/audit → execution. Stage ordering precedes specialization. In particular `binding_changed` belongs only to the Run/step domain; management CAS uses `stale_revision`, and pre-D6 `approval_required|approval_expired` never replace the D6-owned in-race `approval_unavailable/preflight`.

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


`Binding<K>/1.revision` is the sole configuration/lifecycle CAS revision for the addressed control record. When an existing owned record already has a named revision, the names are identical rather than parallel authorities: `leaseRevision == Binding<lease>.revision`, `approvalRevision == Binding<approval|planned_approval|external_approval>.revision`, `grantRevision == Binding<grant>.revision`, `CostReservation.revision == Binding<reservation>.revision`, and `DeploymentControlPolicy.revision == Binding<deployment_policy>.revision`. `ActivationBinding.activationGeneration == Binding<activation>.revision`. Automation intentionally has a distinct `definitionRevision`: enable/disable/archive advances the automation control revision without changing the immutable semantic definition revision.

Independent cumulative-use CAS uses `usageRevision` only for `lease`, `approval`, `grant`, and `cost_account`. A normal Run admission increments Lease usage without changing `leaseRevision`, so the already-admitted Run's exact Lease binding does not become stale merely because another Run consumed capacity. Approval reserve/consume/released_terminal similarly advances approval usage without changing `approvalRevision`. Grant already owns `usageRevision` in §6; cost-account held/spent usage receives the same independent revision in §7 below. Configuration/lifecycle changes never reset cumulative usage, and usage changes never revive a revoked/retired/closed configuration.

    Money/1 = {
      currency:CurrencyCode,
      microUnits:Counter
    }

CurrencyCode is exact three-character ASCII A-Z. D10 performs no implicit foreign-exchange conversion.

Other primitive scalars reuse existing closed JSON semantics. `Token` is the non-empty opaque token from D6 §1; `Text` is a Unicode-scalar string; `Bytes` is a byte sequence bounded by the applicable entrypoint budget; `Boolean` accepts JSON true/false only; `Sha256` is `sha256:` plus 64 lowercase hex digits; `HostPrincipal` is a `Token` produced by trusted host authentication mapping. The current actor is never caller-supplied, but an H-authorized management operation may name another `HostPrincipal` in a policy, reconciler, or ResourceUseGrant **target-principal field**; that configures the authorized target and does not impersonate that principal as the caller. `Ed25519PublicKey` and `Ed25519Signature` appear only inside accepted package/trust adapters whose admitted profile fixes their encoding; an ordinary control caller cannot self-assert verification.

    HostOrWorkspacePrincipal/1 =
        {kind:"workspace",
         workspaceRef:D3.WorkspaceRef,
         principal:Token}
      | {kind:"deployment",
         storeIncarnation:Uuid,
         principal:HostPrincipal}

`HostOrWorkspacePrincipal/1` is a D10 closed projection of the trusted authenticated actor at the successful safety-transition cut. The workspace arm pairs the current D6-authenticated Workspace principal with its exact WorkspaceRef; the deployment arm pairs the H-authenticated HostPrincipal with the actual D10 storeIncarnation. Neither arm is caller-supplied, an author EntityRef, or a capability token.

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


A `view` Contribution descriptor asset has a closed root dispatch. It is either the original D7 `ViewSpec` owned by D7, or the following D10 carrier for the D7-owned pure-data SearchContribution:

    D10D7SearchDescriptor/1 = {
      kind:"d10_d7_search_descriptor",
      wireVersion:1,
      search:<exact S D7 Query Algebra §6 SearchContribution>
    }

This does not make `SearchContribution` a ViewSpec and does not create a `search` ContributionKind. One D10 Contribution carries exactly one D7 SearchContribution. The D7 object remains exact `{contributionId,version,fieldId,textPath,role}` with its original D7 owner, D4-style semantic contribution ID grammar, positive Counter version, 0..8 static text path, and `name|alias|content` role.

R08 also freezes one named, versioned first-party Core author adapter. It is not a generic Tool callback and does not make ToolValue a D7 value alias:

    CoreFieldMemberAdapterDescriptor/1 = {
      kind:"d10_core_field_member_adapter",
      wireVersion:1,
      actionKind:"set_field_member"
    }

The only generation-one adapter identity is packageId `weftext.automation`, package-local contributionId `set-field-member`, Contribution kind `action`, and contractVersion `{major:1,minor:0,patch:0}`. packageVersion and descriptorDigest still use ordinary accepted first-party PackageManifest/ContributionBinding rules. This package is not a Bundled Module and creates no D4 namespace or author fact.

    SingleFieldMemberPath/1 =
      [D4.ObjectMemberSpec.name]   // exact length 1..7

    FieldMemberTask/1 = {
      ownerNodeRef:D3.NodeRef,
      fieldId:D4.FieldId,
      selection:"require_exactly_one_entry",
      memberPath:SingleFieldMemberPath/1,
      value:D7.TypedLiteral
    }

`FieldMemberTask.value` is the original D7 Action literal, not the approval scalar. It is either one allowed scalar `SingleFieldMemberScalarType/1`, or exactly one D7 Optional wrapper whose item is that scalar and whose value state is `some`. `optional.none`, Ref/Locator/control tokens, objects, lists, sets, unions, nested Optional, and ToolValue text promoted into Ref/FieldId are rejected. An optional D4 member therefore keeps the original D7 Optional bridge while `SingleFieldMemberRule.memberType` compares the present underlying scalar.

    D10AuthorPreparationLink/1 = {
      run:ControlRef<run>/1,
      stepId:Counter,
      automation:Binding<automation>/1,
      definitionRevision:Counter,
      taskDigest:Sha256,
      preparedBindingToken:Token,
      request:D6.d6_commit_request
    }

This is a protected Core recovery link, not a public request and not a second author decision. `preparedBindingToken` is the exact token of the original D7 PreparedActionBinding/2 and `request` is its original D6 request. Core saves this link, the original PreparedActionBinding/2, and required pins atomically before returning the prepared author step or allowing submission.

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


For `D10D7SearchDescriptor/1`, activation additionally proves all of the following before it may enter the current Capability Catalog:

1. `descriptorAssetId` resolves to exactly one `PackageAsset/1`; immutable bytes have the declared `byteLength`, SHA-256 equals that asset's `digest`, the strict root decoder is `d10_d7_search_descriptor`, and the active `ContributionBinding.descriptorDigest` is that same digest.
2. D10 PackageId/packageVersion, D10 package-local `Contribution.contributionId`, D10 `contractVersion`, D7 `SearchContribution.contributionId`, and D7 positive Counter `version` are five distinct identity/version domains. Equal spellings or numbers create no mapping. Changing any D7 search member/version changes the Catalog digest even when D10 contractVersion is unchanged.
3. The namespace portion of the D7 SearchContribution ID is authorized by the current D4 owner tuple plus D10 trust/NamespaceClaim proof. Independently, `fieldId` resolves under the exact current `RegistryBinding/1`, its namespace owner is verified, the Field is current-available, and the complete alias-expanded `textPath` terminates in D7 text or Optional<text>. No rule requires the SearchContribution ID namespace to equal the Field namespace.
4. In one current Catalog, D7 SearchContribution IDs are unique. All active `view` Contributions whose descriptor root is `d10_d7_search_descriptor` form the complete SearchContribution set, sorted by D7 contributionId canonical bytes. Omission, duplicate ID, wrong owner, wrong digest, malformed path, or unavailable required Field makes the successor activation fail closed; the previous ActivationBinding remains current.
5. `ActivationBinding.activationGeneration`, `capabilityCatalogDigest`, and the exact D4 `registryBinding` jointly bind that complete set. Explicit D7 search selection binds the selected `(contributionId,version)` rows; D7 `all` binds the complete set at that generation. A successor addition/removal/version/descriptor/owner/Registry change invalidates old search dependencies/results rather than silently changing their fields.
6. The carrier is pure data. It grants no `field_read`, network, secret, script, index-private payload, author write, or alias-source authority. Query execution remains the D7 `scan → explicit read → CEL match/rank → sort/project` pipeline under current D6 authorization. Missing/unavailable selected contributions follow original D7 unavailability; they are never silently skipped and never become an empty successful search.

The positive first-party construction is a valid `weftext.people` package `view` carrier whose accepted descriptor contains D7 SearchContribution `people/search-name`, version `1`, `fieldId:"people/name"`, `textPath:["text"]`, `role:"name"`. Activation succeeds only with the reserved D4 tuple `people→(first_party,weftext.people)`, matching descriptor asset digest, current Registry proof, and complete Catalog construction. A same-named third-party package, wrong digest, or runtime-only discovery remains inactive/pending.

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
   lease/approval→revoke|archive;
   grant→revoke|archive, where archive is the existing `retired` state;
   run→cancel|archive;
   trust→revoke;
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

    AutomationInvocation/1 =
        {kind:"tool",
         contribution:ContributionBinding/1,
         parameters:ToolValue/1}
      | {kind:"core_field_member",
         contribution:ContributionBinding/1,
         task:FieldMemberTask/1}

    AutomationSpec/1 = {
      label:Text,
      invocation:AutomationInvocation/1,
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

`tool` parameters must validate against the active ToolValueProfile/1 input type of that Contribution. `core_field_member` accepts only the exact accepted first-party adapter identity above and its closed `FieldMemberTask/1`; it does not use ToolValueProfile and grants no extra read/write authority. A recurrence selector only locates D4 recurrence source rebound to the same current revision and never becomes a durable EntityRef.

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


For one `core_field_member` Run step, Core performs exactly this mapping and no callback dispatch:

1. Bind the current Automation definitionRevision, exact invocation, ActivationBinding, Lease, current D6 principal, and all finite budgets.
2. Before reading values, run the original D7 Narrow Field Qualification graph for the configured owner/Field/member and prove every required current observation/write scope. Full-source validation may occur internally, but a narrow caller never receives hidden source bytes.
3. Read the complete current Field for `task.ownerNodeRef/task.fieldId`. Automatic execution requires exactly one Entry and the configured member must already be present. From that exact preimage obtain `expectedRevision`, its real `occurrenceKey`, and the original complete `rawEntrySource`; no stale selector is persisted in Automation configuration.
4. Construct the original D7 intent exactly as `{format:"weftext.action",version:1,intent:{kind:"set_field_member",selector:{owner:task.ownerNodeRef,fieldId:task.fieldId,expectedRevision:<fresh owner source revision>,occurrenceKey:<the unique current Entry key>,rawEntrySource:<the exact original Entry JSON text>},memberPath:task.memberPath,value:task.value}}`.
5. Call the original D7 prepare path, retain its PreparedActionBinding/2, fetch and validate the complete preview/effects/MutationFootprint, then compare the **actual** member-change or byte-exact raw-no-op against the separate current Standing Approval. Approval never supplies the target, Entry, member path, or requested value.
6. Core constructs ApprovalUse from the original prepared semantics and enters D6 with the original `d6_commit_request`. Planning/final/replay remain owned by D6.

For optional S `people/phone.label`, task.value is the D7 Optional TypedLiteral `{type:{kind:"optional",item:{kind:"semantic_code",scope:<the complete people contribution-set scope>}},value:{state:"some",value:"people/work"}}`. The D4 scope remains all three codes `people/other|people/personal|people/work`; an approval enum may intentionally allow only a subset. One current phone Entry with present `personal→work` is a true member-change. Present `work→work` is eligible only when the complete proposed source is byte-equal and therefore exercises the existing raw-no-op branch. Zero or multiple current phone Entries make the automatic profile inapplicable; an interactive user may still select the second same-value phone by its real D7 selector and use the ordinary D7 confirmation path.

All original D2/D4/D6/D7 limits remain active, including D4 raw Entry 65,528 bytes and the existing complete source/header/carrier/entry/check budgets. Internal complete-source validation never expands public disclosure. If `D10AuthorPreparationLink/1` exists after restart, Core resumes only that exact original PreparedActionBinding/request. If it is absent and Core can prove the atomic save never succeeded and no request was delivered, a new prepare may be created. If existence/continuity is unknown, return `state_unavailable`; planned or submitted-unknown work recovers the original request and never creates a new OperationId.

    StandingApprovalSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      rule:SingleFieldMemberRule/1,
      maxSuccessfulCommits:Counter,
      costGrants:[Binding<grant>/1]
    }

`CONTROL-CONTRACT` is the unique owner of the generation-one automatic-author rule:

    SingleFieldMemberRule/1 = {
      kind:"single_field_member",
      ownerNodeRef:D3.NodeRef,
      fieldId:D4.FieldId,
      selection:"require_exactly_one_entry",
      memberPath:[D4.ObjectMemberSpec.name],
      memberType:SingleFieldMemberScalarType/1,
      valueConstraint:SingleFieldMemberValueConstraint/1
    }

    SingleFieldMemberScalarType/1 =
        {kind:"bool"}
      | {kind:"text"}
      | {kind:"int64"}
      | {kind:"integer"}
      | {kind:"decimal"}
      | {kind:"semantic_code",
         scope:<exact ResolvedCodeScope structure frozen by S D7 Value §5.2>}

    SingleFieldMemberValueConstraint/1 =
        {kind:"enum", values:[D7.TypedLiteral]}
      | {kind:"numeric_range", minimum:D7.TypedLiteral, maximum:D7.TypedLiteral}
      | {kind:"text_utf8_no_crlf", maximumUtf8Bytes:Counter}

`memberPath` has 1..7 names. Each name uses the original D4 `ObjectMemberSpec/1.name` decoder. Starting at the complete alias-expanded Field valueType root, every nonterminal element must select a direct object member; list/set/sequence indexes, union-arm selectors, wildcard, JSON Pointer syntax, runtime strings, and dynamic FieldIds are forbidden. The terminal member must already be present in the current Entry. D4 type depth remains maximum 8 and alias expansion consumes that original depth, so 1..7 is only an absolute outer bound and never bypasses the D4 depth/schema/source checks.

`memberType` is the underlying scalar D7 bridge type. For a required D4 member the original D7 `set_field_member` Action literal has exactly that TypeSpec. For an optional D4 member the Action literal must have exact D7 type `{kind:"optional",item:<memberType>}` and value `some`; Core extracts only that present scalar for the rule/constraint comparison. `optional.none` is never automatically approved. This preserves the original D7 optional-member bridge rather than confusing the scalar domain with its Optional wrapper.

`enum.values` has 0..64 complete scalar D7 TypedLiterals. Empty is legal and matches no automatic operation. Every literal type must be byte-equal to `memberType`; values are strictly sorted and unique by complete D3-CJ/3 canonical UTF-8 bytes. Membership uses the original D7 Value §2 same-type equality: exact text by scalar value, numeric values exactly, and semantic_code by complete resolved scope plus code. No numeric widening or text/code coercion is permitted.

`numeric_range` is only for int64/integer/decimal. minimum and maximum must have exactly the same complete type as `memberType` and satisfy the original D7 same-type order `minimum <= maximum`. `text_utf8_no_crlf` is only for exact `{kind:"text"}`; `maximumUtf8Bytes` is 0..65528, directly bounded by the S D4 maximum raw Entry UTF-8 size. The candidate Unicode-scalar text is encoded as UTF-8, must be at most that limit, and must contain neither U+000D nor U+000A. The complete proposed D4 Entry/source is still separately encoded and checked against D4/D2 framing, escaping, nonEmpty, schema, cardinality, and source budgets.

All constraints only narrow the current D4/D7 domain. A valid enum hit never substitutes for current Registry contribution availability, Narrow Field Qualification, current D6 authorization, `source_envelope_state`, or the separately required `commit_sequence_state` at author commit.

The two actual-effect branches remain closed. Member-change requires the original D7 adapter to produce exactly one existing scalar-member MutationFootprint and the complete owner_fields field_change while every other source/member/qualifier/note/provenance/Entry/body/Facet/Ref/relation/control fact stays unchanged. Raw-no-op additionally requires the same current owner/Field/unique Entry/member, D7 same-type equality after the required/present-optional projection above, and byte-for-byte equality of the original adapter's complete proposed source to the before source; MutationFootprint, field_change, and D6 sourceVersions remain empty. Typed equality alone cannot authorize a byte rewrite.

A malformed Standing Approval configuration returns management `D10ControlError.invalid_request`. A currently valid D7 Action whose scalar value is outside the frozen constraint is simply not covered by Standing Approval and returns the pre-D6 Run/step `approval_required` path; expiry returns `approval_expired`. After formal D6 entry, D6 owns the error exactly as specified in the coordinated amendment.

Normative positive semantic-code fixture: current S `people/phone` expands `people/labeled-text-value.label` as an optional contribution-set semantic_code. With one current phone Entry whose `label` is present, the D7 Action value is Optional<semantic_code>.some; `SingleFieldMemberRule.memberType` is the underlying semantic_code scope. An enum containing `people/personal` and `people/work` may authorize present `personal→work`, while `work→work` exercises raw-no-op. An allowed D4 code omitted from the approval enum, for example `people/other`, does not become automatically approved.

R08 separates immutable external-request semantics, a concrete send attempt, and the mutable external-effect lifecycle:

    FrozenEffectBytes/1 = {
      bytes:Bytes,
      byteLength:Counter,
      digest:Sha256
    }

    ExternalTarget/1 = {
      operation:LocalOperationId,
      target:ToolValue/1
    }

    ExternalIdempotencyBinding/1 =
        {kind:"none"}
      | {kind:"bounded_key",
         key:Text,
         notBefore:D4.zoned_instant,
         notAfter:D4.zoned_instant,
         proof:FrozenEffectBytes/1}

    ExternalEffectIntent/1 = {
      effect:ControlRef<external_effect>/1,
      workspaceRef:D3.WorkspaceRef,
      contributionBinding:ContributionBinding/1,
      accountBinding:Binding<external_account>/1,
      targetBinding:ExternalTarget/1,
      requestPayload:FrozenEffectBytes/1,
      idempotencyBinding:ExternalIdempotencyBinding/1
    }

    ExternalRequestBinding/1 = {
      effect:ControlRef<external_effect>/1,
      requestDigest:Sha256
    }

    ExternalExecutionBinding/1 = {
      sendAttemptId:Uuid,
      intent:ExternalRequestBinding/1,
      delegationBinding:Binding<lease>/1,
      approvalBinding:Binding<external_approval>/1,
      externalEffectGrant:Binding<grant>/1,
      egressBinding:Binding<grant>/1,
      secretGeneration:Option<{
        secret:Binding<secret>/1,
        secretVersionId:Token,
        grant:Binding<grant>/1
      }>,
      budgetReservations:[{
        billableAttemptId:Uuid,
        reservation:ControlRef<reservation>/1
      }]
    }

    ExternalEffectCurrentView/1 = {
      kind:"external_effect_state",
      state:"prepared" | "submitting" | "succeeded" |
            "failed_no_effect" | "outcome_unknown" | "cancelled",
      recoveryMode:"automatic" | "manual_required",
      contribution:ContributionBinding/1,
      account:Binding<external_account>/1,
      operation:LocalOperationId,
      requestDigest:Sha256,
      targetDigest:Sha256
    }

`FrozenEffectBytes.byteLength` equals the exact bytes and digest equals their SHA-256. Every instance must fit the narrowest applicable finite Run/Automation/Lease input and egress limits. `bounded_key.key` is non-empty UTF-8 text of at most 1024 bytes, `notBefore < notAfter`, and proof is the accepted adapter's complete immutable evidence for the same contribution/account/operation/target/key/window. This proof is internal bytes and is **not** an EvidenceTicket arm.

`requestDigest` is SHA-256 of the complete canonical frozen ExternalEffectIntent under domain D10-External-Intent/1; `targetDigest` is the complete ExternalTarget digest under D10-External-Target/1. Core retains the full bytes. `budgetReservations` is 0..32, sorted/unique by reservation ref, and each item resolves to a CostReservation whose `attemptId==billableAttemptId`; later settlement may advance that reservation revision without mutating the immutable send binding. `sendAttemptId` is distinct from every billableAttemptId.

The public current view is only `ExternalEffectCurrentView/1`. It never returns request payload bytes, target ToolValue, idempotency key/proof, secret generation, approval record, or reservation identities. Even digest/account/contribution disclosure requires current authority for the original frozen effect scope.

    ConsentSpec/1 =
      {kind:"planned",
       originalRequest:D6.d6_commit_request,
       previewSemanticDigest:Sha256,
       lease:Binding<lease>/1,
       activationBinding:ActivationBinding/1,
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}
    | {kind:"external",
       intent:ControlRef<external_effect>/1,
       requestDigest:Sha256,
       resourceGrants:[Binding<grant>/1],
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}

planned authorizes only the original planned request and never reprepares, changes OperationId, or changes target. external authorizes only the immutable ExternalEffectIntent whose exact ControlRef and requestDigest match. A legal `prepared→submitting→...` lifecycle revision never changes that frozen request digest and therefore never invalidates consent by itself; changing contribution/account/target/payload/idempotency creates a new effect intent and needs new consent.

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

Historical prepare/apply and current exact-record reads are deliberately separate.

    D10ControlResult/1 =
        {kind:"d10_control_result_prepared", wireVersion:1,
         scope:Scope/1, requestId:Uuid, prepared:ControlPreparedHistory/1}
      | {kind:"d10_control_result_applied", wireVersion:1,
         scope:Scope/1, requestId:Uuid,
         prepared:ControlPreparedHistory/1, applied:ControlAppliedHistory/1}

    ControlOperationKind/1 =
        "automation_configure" | "consent" | "state" |
        "workspace_limits" | "activation" |
        "deployment_put" | "cost_reconcile"

    ControlAffectedChange/1 = {
      ref:ControlRef<K>/1,
      change:"create" | "update" | "enable" | "disable" |
             "revoke" | "cancel" | "archive" | "retire" |
             "disconnect" | "close" | "settle",
      beforeRevision:Option<Counter>,
      proposedAfterRevision:Option<Counter>
    }

    ControlResourceUse/1 = {
      grant:Binding<grant>/1,
      maximum:Option<Money/1>
    }

    ControlPreviewSummary/1 = {
      affected:[ControlAffectedChange/1],
      resourceUses:[ControlResourceUse/1]
    }

    ControlPreparedHistory/1 = {
      intentDigest:Sha256,
      operation:ControlOperationKind/1,
      allocatedControlRefs:[ControlRef<K>/1],
      previewSummary:ControlPreviewSummary/1,
      commitOwner:"D6" | "D10"
    }

The protected `ControlPrepareBinding/1.canonicalIntentBytes` continues to store complete canonical control intent B, including any nested planned author request A; stable-key conflict still compares the complete bytes. `intentDigest` is only SHA-256 of those saved bytes. Historical public result never returns A, complete B, generated control-submit request M, prepareToken, or planned-preview bytes/token. `operation` is exactly one of the seven ControlBody kinds; `ControlAffectedChange/1` and `ControlResourceUse/1` are the existing ControlPreview item shapes with unchanged fields, enums, sorting, uniqueness, Option, and Money semantics.

Initial `d10_control_prepare` may return full `D10ControlPrepared/1`, including M, only after current disclosure authority covers complete B and any nested A scope. Earlier possession of A is not current read authority. Same-key prepare after an authoritative applied success returns the same `d10_control_result_applied` historical arm, never a fresh-looking prepared object. If applied-success existence or linkage is unprovable, return `state_unavailable`; never downgrade to prepared.

    ControlAppliedHistory/1 =
        {kind:"workspace",
         ownerReceipt:D6.d6_commit_receipt,
         changes:[ControlRevisionDelta/1],
         usageChanges:[ControlUsageRevisionDelta/1]}
      | {kind:"deployment",
         changes:[ControlRevisionDelta/1],
         usageChanges:[ControlUsageRevisionDelta/1],
         evidence:[EvidenceTicket/1],
         auditRef:Token}

    ControlRevisionDelta/1 = {
      ref:ControlRef<K>/1,
      beforeRevision:Option<Counter>,
      afterRevision:Counter
    }

    ControlUsageRevisionDelta/1 = {
      ref:ControlRef<lease|approval|grant|cost_account>/1,
      beforeUsageRevision:Counter,
      afterUsageRevision:Counter
    }

Workspace applied history is projected from the same authoritative D6 saved decision: `ownerReceipt` is the original immutable receipt bytes and `changes/usageChanges` are only its decision-linked D10 control effects. Deployment applied history is projected from the original `DeploymentControlDecision/1` plus the record/account deltas atomically linked to that decision by §8. Neither arm is a second success ledger.

Result resolution order is: current result-disclosure authorization → authority/custody/continuity → stable key. Missing/hidden is `not_visible`. If a prepare binding is proven and no applied success exists, return prepared. If an authoritative applied success exists, return applied. A D6 recorded rejection/terminal failure remains owned by D6 and is obtained by replaying the original D6 request; D10 result may return prepared history only after it proves that no applied success exists. If the presence/absence or linkage of an applied success is temporarily unprovable, return `state_unavailable` rather than downgrading to prepared. Proven decision/effect linkage contradiction is `integrity_conflict`.

If r5 applied, the response was lost, and another valid request later changes the same record to r6, retry of the original request returns the saved r5 applied history after current disclosure authorization. It never substitutes r6.

Current state uses a different entrypoint:

    D10ControlReadRequest/1 = {
      kind:"d10_control_read", wireVersion:1,
      scope:Scope/1,
      ref:ControlRef<K>/1
    }

    D10ControlCurrent/1 = {
      kind:"d10_control_current", wireVersion:1,
      scope:Scope/1,
      binding:Binding<K>/1,
      usageRevision:Option<Counter>,
      view:ControlCurrentView<K>/1
    }

`scope` must be the record's exact real scope. There is no name lookup, wildcard, list, or display-label lookup. Current disclosure authorization occurs before existence/state. Missing, wrong scope/audience, and hidden all return `not_visible`. A visible record whose protected continuity is temporarily unprovable returns `state_unavailable`; an unknown closed state/member in a supposedly compatible record is an integrity/version failure, never `state:"unknown"`.

The generation-one current projections are closed as follows:

| K | exact scope | exact `view` | config/domain/usage revision semantics |
| --- | --- | --- | --- |
| `automation` | `workspace` | `{kind:"automation_state",state:"enabled"|"disabled"|"archived",definitionRevision:Counter,definition:AutomationSpec/1,lease:Binding<lease>/1,approval:Option<Binding<approval>/1>}` | `binding.revision` is the control/lifecycle CAS; `definitionRevision` changes only semantic definition. usageRevision none. |
| `lease` | `workspace` | `{kind:"lease_state",state:"active"|"revoked"|"archived",principal:Token,target:ControlRef<automation|run>/1,spec:LeaseSpec/1,runsConsumed:Counter}` | `binding.revision==leaseRevision`; usageRevision some and advances only when a new `LeaseRunUse/1` consumes the lineage. Normal usage never stales an already-admitted Run's leaseRevision. |
| `approval` | `workspace` | `{kind:"approval_state",state:"active"|"revoked"|"archived",grantingPrincipal:Token,automation:Binding<automation>/1,definitionRevision:Counter,lease:Binding<lease>/1,activationBinding:ActivationBinding/1,spec:StandingApprovalSpec/1,reserved:Counter,consumed:Counter,releasedTerminal:Counter}` | `binding.revision==approvalRevision`; usageRevision some for ApprovalUse reserve/consume/released_terminal only. Revocation/archive advances approvalRevision without resetting usage. |
| `planned_approval` | `workspace` | `{kind:"planned_approval_state",grantingPrincipal:Token,originalRequestDigest:Sha256,previewSemanticDigest:Sha256,lease:Binding<lease>/1,activationBinding:ActivationBinding/1,notBefore:D4.zoned_instant,notAfter:D4.zoned_instant}` | binding revision is its approvalRevision; usageRevision none. No author request/preview bytes are exposed here. |
| `external_approval` | `workspace` | `{kind:"external_approval_state",grantingPrincipal:Token,intent:ControlRef<external_effect>/1,requestDigest:Sha256,resourceGrants:[Binding<grant>/1],notBefore:D4.zoned_instant,notAfter:D4.zoned_instant}` | binding revision is its approvalRevision; usageRevision none. |
| `run` | `workspace` | `{kind:"run_state",lifecycle:"active"|"archived",executionState:"queued"|"running"|"awaiting_confirmation"|"blocked"|"cancelling"|"reconciling"|"completed"|"failed"|"cancelled",automation:Binding<automation>/1,definitionRevision:Counter,lease:Binding<lease>/1,admission:{kind:"not_admitted"}|{kind:"admitted",leaseId:Uuid,leaseRevision:Counter,admittedAt:D4.zoned_instant},stop:Binding<stop>/1}` | binding revision advances on durable Run/lifecycle transition. usageRevision none; maxRuns consumption belongs to Lease usage. |
| `workspace_budget` | `workspace` | `{kind:"workspace_budget_state",limits:BudgetCaps/1}` | binding revision is limits CAS. usageRevision none; actual cost usage remains in grants/accounts/reservations rather than a second budget ledger. |
| `activation` | `workspace` | `{kind:"activation_state",current:Boolean,activation:ActivationBinding/1,packages:[ContributionBinding/1],trust:Binding<trust>/1}` | `binding.revision==activation.activationGeneration`; successor activation makes earlier record `current:false` without deleting it. usageRevision none. |
| `deployment_policy` | `deployment` | `{kind:"deployment_policy_state",policy:DeploymentControlPolicy/1}` | `binding.revision==policy.revision`; usageRevision none. |
| `trust` | `deployment` | `{kind:"trust_state",state:"active"|"revoked",publisherId:Text,publicKey:Ed25519PublicKey,previous:Option<Binding<trust>/1>,proof:EvidenceTicket/1,claims:[NamespaceClaim/1]}` | binding revision covers key/claim/rotation/revocation. usageRevision none. |
| `package` | `deployment` | `{kind:"package_state",state:"installed"|"retired",manifest:PackageManifest/1,signature:Ed25519Signature}` | binding revision covers install/update/retire; packageVersion is independent. usageRevision none. |
| `external_account` | `deployment` | `{kind:"external_account_state",state:"connected"|"disconnected",provider:ContributionBinding/1,externalAccountId:Text,endpointId:LocalOperationId,proof:EvidenceTicket/1}` | binding revision covers configuration/disconnect. usageRevision none. |
| `secret` | `deployment` | `{kind:"secret_state",state:"active"|"revoked",account:Binding<external_account>/1,audience:ContributionBinding/1,usageKind:"authenticate",secretVersionId:Token}` | binding revision covers publish/rotation/rebind/revoke. No plaintext/staged bytes are returned. usageRevision none; secret-use counts remain in the ResourceUseGrant. |
| `grant` | `deployment` | `{kind:"grant_state",grant:ResourceUseGrant/1}` | `binding.ref.id==grant.grantId`, `binding.revision==grant.grantRevision`, and usageRevision is some and equals `grant.usageRevision`. `archive` maps to the existing `retired` state. |
| `cost_account` | `deployment` | `{kind:"cost_account_state",state:"active"|"frozen"|"closed",currency:CurrencyCode,ceiling:Counter,pricing:Option<Binding<pricing>/1>,spentMicroUnits:Counter,heldMicroUnits:Counter}` | binding revision covers config/close/freeze; usageRevision some for held/spent reservation deltas. Freeze never clears liabilities. |
| `pricing` | `deployment` | `{kind:"pricing_state",state:"active"|"retired",account:Binding<external_account>/1,currency:CurrencyCode,fixedMicroUnits:Counter,meters:[PricingMeter/1],evidence:EvidenceTicket/1}` | binding revision covers pricing update/retire. usageRevision none. |
| `reservation` | `deployment` | `{kind:"reservation_state",reservation:CostReservation/1}` | `binding.ref.id==reservation.reservationId` and `binding.revision==reservation.revision`. usageRevision none; `uncertain` is the existing recoverable state, not unknown. |
| `external_effect` | `workspace` | `ExternalEffectCurrentView/1` | binding revision advances only on durable effect/recovery lifecycle transition and is distinct from the immutable requestDigest. usageRevision none; costs/attempt quotas remain their reservation/grant owners. |
    ExecutionStopLatchView/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal/1>
    }

| `stop` | exact target Workspace scope when read through W, or exact deployment storeIncarnation when read through H | `{kind:"stop_state",latch:ExecutionStopLatchView/1}` | fresh open record has revision 1; only `open→stopped` advances once; idempotent repeated stop is a no-op. Internal safety sequence is not public; usageRevision none. |

For `lease|approval|grant|cost_account`, `D10ControlCurrent.usageRevision` must be `some` and equal the record's current independent usage revision; every other kind requires `none`. Trusted time crossing `notBefore/notAfter` changes current eligibility but does not silently mutate configuration revision or create a persisted `expired` state. Revocation/retirement/archive/close remains observable to an authorized reader and never aliases absence.

`activation_state.current` is derived in the same authorized cut from the exact active-selector record; the selector, not the historical ActivationBinding record, is the CAS object for switching. Successor activation compares the exact current predecessor selector and atomically publishes the new ActivationBinding plus selector. An old generation becoming non-current never changes that historical generation or its Binding revision.

Generation one exposes no independent early-revoke state action for `planned_approval` or `external_approval`. Their finite time bounds, exact Lease/Activation/request binding, current ResourceUseGrant state, current D6 authorization, and irreversible stop gates are rechecked at the relevant planned/send boundary. This support limitation does not create a generic approval lifecycle state and never releases planned work except through the original authoritative-abort rule.

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
        ref:ControlRef<lease|approval|grant|cost_account>/1,
        usageRevision:Counter
      }],
      authorityProof:Token,
      authorizationGenerations:[Token],
      stopRefs:[ControlRef<stop>/1],
      sourceOrRegistryBindings:[Sha256]
    }

`ControlDependencies/1` is the complete internal dependency set Core derives from actual authorized reads and is never caller input. Arrays are sorted/unique by complete canonical bytes. A usage binding exists only for lease maxRuns lineage use, Standing Approval count use, ResourceUseGrant cumulative use, or actual cost-account held/spent use; it never substitutes for that record's configuration Binding. sourceOrRegistryBindings stores binding digests owned by existing source/Registry contracts and creates no new author or Registry token.

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
      ownerId:NamespaceOwnerId/1,
      proof:EvidenceTicket/1
    }

`NamespaceOwnerId/1` is only a D10 validation name for the exact scalar `ownerId` already carried by the D4 Registry row; it is not a new namespace identity. For `first_party`, the value must be the exact reserved D4 tuple value for that namespace (`weftext.people`, `weftext.organizations`, `weftext.calendar`, or `weftext.library`). For `publisher`, it must be byte-equal to the accepted trust record's `publisherId` and that exact `(namespaceId,ownerClass,ownerId)` tuple must be proven by the `namespace_claim` EvidenceTicket. A D6 random Token/ticket can never occupy `ownerId`; proof is the separate `proof` member.

`NamespaceClaim/1` binds a verified PublisherIdentity/first-party root to the already-owned D4 semantic-namespace tuple. It creates no second namespace registry and cannot override `core`, `wf`, another reserved owner, or a different verified publisher. Install order, package/display name, enablement, and string coincidence are never owner proof.

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


Each `CostReservation/1` belongs to exactly one billable `attemptId`, one actual `cost_account`, one grant, one pricing binding, and one currency. Run/Lease/Automation/Workspace/deployment limits checked during the same admission are layered ceilings/projections, not additional actual accounts for this reservation, and the same cost is recorded once. If one operation truly creates separately attributable charges against multiple actual accounts, it creates separately attributable attempts/reservations/evidence for those accounts; any required group admission is atomic over those reservations without turning one reservation into a multi-account object.

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

Public stop is a specialized safety transaction outside ordinary control prepare:

    D10EmergencyStopRequest/1 = {
      kind:"d10_emergency_stop",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      target:
        ControlRef<automation>/1
      | ControlRef<run>/1
    }

Generation one requires `requestId == target.id`. requestId is a deterministic dedup coordinate for the exact target, not a caller-selected capability. The request still carries the complete target Ref; no text, display name, bare UUID, or ToolValue is promoted into it.

    StopOwner/1 = {
      storeIncarnation:Uuid,
      workspaceRef:D3.WorkspaceRef,
      target:ControlRef<automation|run>/1,
      latch:ControlRef<stop>/1,
      requestId:Uuid
    }

    StopCapacity/1 = {
      issued:Counter,
      reserved:Counter
    }

The protected stop stable key is `("D10-Emergency-Stop/1",target.storeIncarnation,target.kind,requestId)`. `requestId==target.id`, target/latch storeIncarnations equal StopOwner.storeIncarnation, and one exact target has exactly one latch. workspaceRef is the target's real Workspace from protected control state. D10 storeIncarnation is an internal persistent control-domain incarnation bound to the actually opened D6 authority-store backend; it is not a frozen D6 public API name, WorkspaceId, filesystem path, or authority token.

W uses exact Workspace scope and H exact deployment storeIncarnation. Both may address the same latch only after their independent current authority gates. Scope is access qualification, not a second stop fact.

Before first enable/admission, object creation atomically reserves one durable latch, one StopOwner association, one audit/result slot, and one unit of safety-sequence capacity. If reservation fails, the executable object cannot become manageable/runnable. Ordinary control prepare, configuration Counter space, budgets, approval/lease counts, cost, and executor quotas cannot consume this capacity. Stop requires no target configuration revision and no preceding ordinary management write.

    ExecutionStopLatch/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal/1>,
      stoppedAtControlSequence:Option<Counter>
    }

Open requires both options none; stopped requires both some. The transition is exactly Binding revision 1/open → revision 2/stopped. There is no clear operation. Safety sequence reservation obeys `reserved <= MAX-issued`, MAX=2^63-1. First stop atomically executes `issued:=issued+1`, `reserved:=reserved-1`, writes the sequence/actor/latch revision, immutable stop result/audit link, and required invalidations. Thus capacity exhaustion may reject creation of a new executable target, never the first stop of an existing reserved target.

    D10EmergencyStopReceipt/1 = {
      kind:"d10_emergency_stop_receipt",
      wireVersion:1,
      requestId:Uuid,
      target:ControlRef<automation|run>/1,
      latch:Binding<stop>/1,
      state:"stopped"
    }

The receipt requires latch revision 2 and is a deterministic projection of the single latch transition; it is neither a D6 author receipt nor a second success ledger.

    D10EmergencyStopResultRequest/1 = {
      kind:"d10_emergency_stop_result",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      target:ControlRef<automation|run>/1
    }

    D10EmergencyStopResult/1 =
        D10EmergencyStopReceipt/1
      | {kind:"d10_emergency_stop_not_applied",
         wireVersion:1,
         requestId:Uuid,
         target:ControlRef<automation|run>/1,
         latch:Binding<stop>/1,
         state:"open"}

`not_applied` requires proven latch revision 1/open at that read linearization point and is not a promise about a later stop.

Stop/result order is: closed decode and D1 automation.stop gate → current authenticated principal plus exact W/H scope and target/stop disclosure authority → authority/fence/custody → protected stable-key/target equality → latch/result continuity → transition or read → final current disclosure gate. Hidden, missing, wrong-scope, and wrong-store are `not_visible`; unprovable authority/fence/custody is `authority_unavailable`; proven corruption is `integrity_conflict`. After visibility, the same stable key bound to a different target is `control_conflict`. Unprovable latch/result continuity is `state_unavailable`, never not_applied or a fresh stop.

If target configuration r5 is stopped and the response is lost, then a separately legal configuration changes the target to r6, retry with the same exact target/requestId returns the original receipt. Current r6 neither invalidates nor rewrites it. Loss of result-disclosure authority instead returns `not_visible` while the latch remains stopped.

Linearization rules:

1. New Run admission and the safety transaction share the same Authority Store serialization domain. Admission checks latches and atomically writes occurrence claim/LeaseRunUse. Stop first means no admission; admission first preserves its consumed count and all later protected steps still check stop.
2. D6 final author commit rechecks every referenced stop latch inside the actual final author transaction while holding the same write-serialization boundary. Commit first preserves the committed result; stop first prevents the new commit.
3. External transport and stop share one send fence. Before the first irreversible handoff, Core has frozen the immutable ExternalEffectIntent and exact ExternalExecutionBinding, rechecks stop plus approval/grant/secret/cost bindings, durably stores send-attempt/started evidence and holds, and retains the fence through the first real send handoff. Stop first means no send; handoff first preserves the original send attempt and may later be outcome_unknown.
4. After crash, restore latch, send-attempt evidence, reservations, and original decisions before restoring an executor. Unknown evidence can never become cancelled or justify a new effectId, idempotency key, or OperationId.
5. Stop does not block currently authorized authoritative abort, cost settlement, evidence/audit retention, or reference-safe cleanup. Those operations do not regain executor authority.
6. Temporary disable, Lease expiry, temporary authorization loss, or ordinary cancel is not irreversible abort proof.

The stop safety transaction is a specialized closed write in the same managed Authority Store, not D10ControlPrepare and not a D6 author transaction. The coordinated D6 amendment only defines how D6 final/planned author work consumes the latch and returns `execution_stopped/preflight` or the original authoritative-abort result. Stop never fabricates ordinary control history, never rolls back committed author facts or an already-sent external effect, and never releases cost merely because execution was stopped.

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
