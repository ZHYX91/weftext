---
source_language: zh-CN
translation_of: TERMINOLOGY.zh-CN.md
translation_status: synced
---

[简体中文](TERMINOLOGY.zh-CN.md)

# D10 Terminology and Naming

revision: D10-r06-terminology-and-import-clarifications-2026-09-28; status: candidate. This lexicon defines only new D10 control/extension concepts and consumption boundaries for D1-D9; it does not reassign any upstream-owned concept. Natural language, historical material, test descriptions, and third-party product vocabulary do not automatically become controlled names.

## 1. Naming principles

- Author content, identity, locator, provenance, Field, Facet, Query, Action, Draft, conversion, publication, and related terms remain owned by their original stages.
- D10 control identities may not borrow D3 EntityRef/Locator naming; local ordinals, tokens, and IDs must not be described as content identity either.
- "plugin" is only a user-facing or historical generic term, not a canonical wire family. Controlled documents distinguish Extension Package, Contribution, Connector, Tool Adapter, Model Adapter, Pack, and Bundled Module.
- Bare "Provider" can collide with D9 Conversion Provider; D10 must say Model Provider, External Service, Connector Provider, or directly Contribution/Adapter.
- "Registry" in D10 prose means the D4 semantic Registry by default; D10 uses Capability Catalog for its own catalog and does not create a "plugin registry".
- "approval", "authorization", "delegation", and "confirmation" are distinct: D6 Policy is workspace authorization; Delegation Lease only narrows it; Standing Approval is bounded pre-approval; D8 confirmation is human confirmation of a current complete preview.
- "source" must be qualified as author source, external input, tool result, provider state, package asset, or source binding. Provider state is not author source.

## 2. New D10 concepts

| conceptId | 中文 / English | owner | Definition | Explicit exclusion |
| --- | --- | --- | --- | --- |
| weftext.term.extension-package | 扩展包 / Extension Package | D10 | Distribution/upgrade unit with named version, content digest, publisher, dependencies, and contribution catalog | Not a permission unit, namespace owner, or author object |
| weftext.term.contribution | 贡献项 / Contribution | D10 | One pure-data or executable capability inside a package that can be admitted, enabled, and authorized independently | Does not inherit whole-package authority |
| weftext.term.pack | 扩展包数据包 / Pack | D10 | Declarative/rule/schema/data Contribution set with no arbitrary host privileges and one parent domain/extension point | Not an executable Extension, Connector, author Profile, or D4 Registry |
| weftext.term.bundled-module | 内置模块 / Bundled Module | D1 product ownership + D10 lifecycle consumption | First-party user-visible product module that may provide domain extension points and UX | Not Core identity, package ownership, or author database |
| weftext.term.agent-session | Agent 会话 / Agent Session | D10 | Interactive Agent mode of a Run, bound to one managed task, context, and tool set | Not an author entity, permanent principal, or independent ledger |
| weftext.term.parent-extension-dependency | 父扩展依赖 / Parent Extension Dependency | D10 | Required dependency from a domain Pack/Contribution to parent domain, extension point, and compatible version range, resolved and bound at activation | UI hiding is not dependency failure and retained D4 definition history is not runtime activation |
| weftext.term.capability-catalog | 能力目录 / Capability Catalog | D10 | Contribution/runtime availability and host-privilege description for the active deployment | Not the D4 Registry, D1 capability response, or author schema |
| weftext.term.activation-binding | 激活绑定 / Activation Binding | D10 | Managed record binding current D4 RegistryBinding, Capability Catalog digest, and trust revision into one generation | Not a replacement for D4 semantic generation |
| weftext.term.publisher-identity | 发布者身份 / Publisher Identity | D10 | Package-signing subject proven by an accepted trust root | Not namespace ownership itself |
| weftext.term.namespace-claim | 命名空间所有权声明 / Namespace Claim | D10→D4 proof | Managed claim binding publisher identity to an allowed D4 publisher namespace | Install order, display name, and enablement are not claims |
| weftext.term.delegation-lease | 委托租约 / Delegation Lease | D10 | Task/run authorization boundary that further narrows the current D6 principal and includes finite maxRuns | Adds no D6 capability and cannot be redelegated |
| weftext.term.lease-run-use | 运行准入消费 / Lease Run Use | D10 | Durable deduplication fact consuming one maxRuns use for a leaseId lineage when a Run first enters protected execution | A queued claim is not consumption; failure, cancellation, or crash does not refund it |
| weftext.term.standing-approval | 持续批准 / Standing Approval | D10 | Finite pre-approval for a mechanically decidable future operation set with lifetime/count/budget limits | Not permanent consent, natural-language goal, or D6 Policy |
| weftext.term.approval-use | 批准使用记录 / Approval Use | D10 + proposed D6 amendment | Binds one prepared request, complete preview, actual footprint, and one approval reservation or consumption; count state may be unreserved, reserved, consumed, or released_terminal | Not an author plan, receipt, or third ledger |
| weftext.term.planned-decision-approval | 已计划决议交互批准 / Planned Decision Approval | D10 + proposed D7/D6 amendment | Finite one-shot interactive authorization for the exact original planned request after complete inspection through a new protected recovery preview | Does not re-prepare, change OperationId, or revive a conflicting plan |
| weftext.term.automation-definition | 自动化定义 / Automation Definition | D10 | Managed versioned schedule for one invocation | Not a general workflow DAG or author Document |
| weftext.term.automation-occurrence | 自动化发生项 / Automation Occurrence | D10 | One scheduling opportunity mechanically derived from a definition revision and source occurrence | Not author recurrence-occurrence identity |
| weftext.term.run | 运行 / Run | D10 | Managed execution record for an Agent or Automation plus step outcomes | Run completed does not mean author committed |
| weftext.term.context-bundle | 上下文包 / Context Bundle | D10 | Current-authorized, minimized, version-bound context for a specific model/tool recipient | Not an author snapshot or redelegable token |
| weftext.term.tool-value-profile | 工具值配置 / Tool Value Profile | D10 | Tool parameter/result algebra reusing a finite subset of D7 TypeSpec/V | Not arbitrary JSON Schema or the full D4 Field-value algebra |
| weftext.term.input-slot | 输入槽 / Input Slot | D10 | Per-invocation restricted file input handle bound to exact bytes/purpose/recipient | Not a path, ResourceRef, or cross-invocation file handle |
| weftext.term.tool-adapter | 工具适配器 / Tool Adapter | D10 | Adapter mapping an accepted external tool protocol to Tool Value and an explicit effect class | Not an author Action adapter |
| weftext.term.mcp-adapter | MCP 适配器 / MCP Adapter | D10 | A Tool Adapter that uses MCP as transport/discovery protocol | MCP annotations/prompts do not become Weftext permissions |
| weftext.term.model-adapter | 模型适配器 / Model Adapter | D10 | Adapter connecting managed ContextBundle to a model API/local model transport | Model output is not author authority |
| weftext.term.connector | 连接器 / Connector | D10 | Protocol/account/cursor/version and sync-control adapter for one named external system | Provider ID/cursor is not EntityRef |
| weftext.term.secret-reference | 凭据引用 / Secret Reference | D10 | Managed reference and generation for a credential stored in OS/Server secret storage | Not secret bytes, author source, or model input |
| weftext.term.external-effect-intent | 外部效果意图 / External Effect Intent | D10 | One frozen external mutation request including target/account/request/idempotency/approval/budget | Not a D6 transaction or rollback |
| weftext.term.external-outcome-unknown | 外部结果未知 / External Outcome Unknown | D10 | Durable state in which external mutation occurrence/completeness cannot be proven | Not failed or cancelled |
| weftext.term.cost-reservation | 费用预留 / Cost Reservation | D10 | Atomic finite upper-bound reservation against one or more cost accounts before a billable attempt | Not a billing fact or effect idempotency |
| weftext.term.audit-started | 审计开始记录 / Audit Started Record | D10 | Minimum durable security-audit fact written before a protected step truly executes | Does not mean the step succeeded |
| weftext.term.money | 费用值 / Money | D10 | Exact cost representation as currency + Counter microUnits | No implicit FX and no binary float |

## 3. Exact controlled names

The following type/record names are frozen controlled spellings by the D10 candidate. Whether a later implementation exposes them on a public wire is an interface-stage decision, but the same concept may not gain competing near-synonym types:

- `ActivationBinding/1`
- `DelegationLease/1`
- `LeaseRunUse/1`
- `StandingApprovalEnvelope/1`
- `ApprovalUse/1`
- `PlannedDecisionApproval/1`
- `ToolValueProfile/1`
- `InputSlot`
- `AutomationOccurrenceKey/1`
- `ExternalEffectIntent/1`
- `Money/1`

Existing upstream names are consumed exactly as owned:

- `RegistrySnapshot/1`
- `RegistryBinding/1`
- `PrincipalContext`
- `ObservationScope`
- `PreparedIntent`
- `ActionSpec`
- `PreparedActionBinding/2`
- `EffectManifest/1`
- `PreparedEditBinding/1`
- `SourceBinding`
- `OriginBinding`
- `Provenance`
- `SourceVersion`
- `OperationId`

D10 defines no alias replacing those names.

## 4. Package, module, pack, and plugin

**Extension Package** is the installation, upgrade, signature, and asset-integrity unit and may contain one or more Contributions. Permission, dependency, and runtime availability are computed per Contribution, so denying one network Connector cannot automatically disable pure-data Contributions in the same package.

**Bundled Module** means first-party product organization/UI such as existing Calendar, People, Organizations, and Library directions from D1. A module may provide domain extension points but does not become another namespace-owner class, author database, or Core identity.

**Pack** is reserved for declarative semantic, rule, schema, or data Contributions with no arbitrary code/process/network/secret/file host privilege. A "pack" that needs code, networking, credentials, or complex runtime behavior must be reclassified on controlled surfaces as an executable Contribution/Connector rather than using its label to bypass runtime gates.

Every domain Pack Contribution belongs to one primary parent domain/extension point and declares a required compatible version range. That dependency belongs to D10 Capability Catalog activation rather than UI hierarchy. Hiding only the parent UI does not change Pack activation when the parent semantic capability remains active and compatible. Only actual parent missing, disabled, incompatible, or unsupported-surface state makes the dependent Contribution inactive. The package itself may remain installed/verified and its configuration, source, and historical bindings remain recoverable.

Pack **semantic-definition retention** is separate from **runtime activation**. Schema/Field/Facet definitions and history already admitted to the D4 semantic ledger cannot be deleted because a Pack or parent UI/module is disabled/uninstalled. When the current RegistryBinding still proves definitions complete, Core may continue interpreting existing author facts, but that does not mean the Pack View/Action/rule/connector remains active. When definitions cannot be proven, D4 follows unavailable/raw-preservation semantics.

**plugin** remains only a user-comprehensible generic or historical term. Controlled schema, CLI/API, test fixtures, and candidate design must use the concrete category. It also cannot become a common wire kind for Pack, Module, Connector, and Provider.

## 5. Registry, Catalog, and capability

The **D4 Registry** owns semantic namespaces, Fields/Facets/Relations/Calendar/units and evolves under D4 cumulative rules. D4 Registry complete/unavailable state, generation, digest, tombstone, and migration history cannot be rewritten by D10 package enablement.

The **Capability Catalog** owns current deployment descriptions, parent extension-point dependency resolution, runtime availability, and host privilege for Contributions. It cannot define Field, Facet, or relation semantics. Catalog and Registry changes may be coordinated by one Activation Binding but remain different objects.

**Parent Extension Dependency** describes only parent domain, extension point, required version range, and current resolution result. It is neither D4 schema ownership nor UI visibility. The resolved parent binding enters the current Catalog digest. A parent version/binding change invalidates old dependent activation and requires a successor ActivationBinding.

A **D1 capability** describes whether a formal product surface can provide a product capability for the current release/subject and uses D1 fixed unavailable reasons. D10 dependency state is only one downstream fact used by D1 and creates no parallel reason set. Higher-priority D1 reasons still apply first. If parent dependency is the first actual blocker: missing→`missing_component`, disabled→`not_configured`, incompatible→`incompatible_version`, unsupported surface→`unsupported_surface`. UI-hidden-only creates no unavailable reason.
## 6. Provider terminology disambiguation

D9 **Conversion Provider** / **Route** remain D9-owned and are not renamed by D10.

Within D10:
- model services are **Model Provider**;
- external SaaS/systems are **External Service**;
- protocol implementations are **Connector** or **Tool Adapter**;
- natural-language "provider" is shortened only when context is unambiguous and cannot collide with D9.

"provider unavailable" is not a new D10 capability reason; formal capability results still use D1's existing `missing_component`, `not_configured`, `offline`, `incompatible_version`, and `temporarily_unavailable` reasons as applicable.

## 7. Identity, source, and provenance disambiguation

EntityRef, NodeRef, ResourceRef, AnnotationRef, Locator, and OperationId remain owned by their original upstream contracts.

RunId, AutomationId, ApprovalId, ExternalEffectId, AuditEventId, tool-call ID, package ID, and provider-account ID are control/external identities and cannot enter EntityRef.

Author source is the author payload recognized by D2/D3/D6. External input, ContextBundle, tool result, model output, package asset, transcript, connector cache, and provider response are not author source.

D3 Provenance is source evidence and does not grant authority. D10 package/provider origin likewise cannot become write permission. SourceBinding/OriginBinding retain D3 meanings; a Connector cannot call its own cursor/etag a "binding" to bypass them.

## 8. Approval, confirmation, authorization, and delegation

**authorization** is current workspace eligibility from D6 Policy, ObservationScope, authority/cut, and applicable upstream gates.

**delegation** is a Delegation Lease that further narrows current authorization; it is not an authorization source. maxRuns is the total number of Runs that one leaseId lineage may admit into protected execution and is not reset by leaseRevision, restart, or scheduler-cache rebuild.

**Lease Run Use** is the durable consumption fact written by the atomic Run-admission CAS immediately before a Run's first protected step. Recovery of the same Run reuses the original record, and failure or cancellation after admission never refunds the use.

**interactive confirmation** is explicit user confirmation of the original request after a current complete preview; D8/D7 keep this as the default path.

**Standing Approval** pre-allows only the mechanically bounded envelope defined by the candidate and still requires fresh prepare, preview, and current authorization.

**ApprovalUse** is the managed binding by which one prepared request actually consumes Standing Approval and cannot be client-asserted. Its count reservation becomes reserved only at D6 planned, becomes consumed on commit, and can become released_terminal only with authoritative terminal_failed.

**Planned Decision Approval** is finite one-shot interactive authorization for the exact original request when the D6 decision is already planned and the old preview transport expired or Standing Approval is no longer usable. It can be created only after complete inspection through a new read-only planned-preview recovery epoch. It changes no request, OperationId, PreparedActionBinding, or target and cannot revive a plan with a deterministic dependency conflict.

**approval-required / approval-expired** are D10 control results only before formal D6 submission. If approval loses a race after the unattended path has entered D6, the companion amendment uses D6 approval_unavailable/preflight rather than pretending the failure was a D10 preflight result.
## 9. Secret, context, egress, and external effect

Secret Reference points only to a credential in secret storage; secret bytes never enter ToolValue, ContextBundle, or ordinary logs.

Context Bundle is actual authorized context for one model/tool invocation. Holding it grants no new read and does not permit switching recipients.

egress means data leaving the current trusted Core/host boundary to a named model/tool/connector/external service; network capability is transport only and does not authorize arbitrary recipients.

external side effect means a request that changes external-system state. Reading external data and writing external data are different effect classes; a "read-only tool" exists only under the locally accepted Weftext contract and not because a remote server self-annotated it.

External Outcome Unknown means the external request's outcome cannot be proven; it is not timeout, failure, or cancellation. Cost released means billable execution is proven never to have started; an actually sent request with a reliable zero bill is settled(0).

## 10. Audit, transcript, log, and evidence

Audit contains protected facts required for recovery/security; Transcript is an optional human/model conversation record; ordinary log/telemetry is runtime observability. They have different retention, permission, and export rules.

Audit Started Record proves only that a protected step entered the execution boundary and does not prove success. Author success still requires the D3/D6 receipt; external success requires the complete success evidence defined by the named adapter.

Test reports, CI logs, model witnesses, screenshots, and protocol traces are evidence rather than authority. A candidate must state the limited claim each one supports.

## 11. Controlled prohibited conflations

- Do not call Capability Catalog the D4 Registry.
- Do not call package install successful namespace registration.
- Do not call Enable/Disable creation/deletion of author facts.
- Do not call Run completed committed.
- Do not call model/tool proposal Draft unless it actually enters a D8 Draft.
- Do not call Preview a receipt.
- Do not call ApprovalUse an author plan.
- Do not call ExternalEffectIntent a transaction.
- Do not call compensation rollback.
- Do not call outcome_unknown failed.
- Do not call cancel Run cancellation of a D6 planned decision.
- Do not call SecretRef credential bytes.
- Do not call UUID/Ref spelling inside ToolValue text an EntityRef.
- Do not call an MCP prompt/tool annotation policy.
- Do not call a finite-model pass product support.

## 12. Terminology-gate completion conditions

Before independent review, mechanically scan D10 controlled headings, type names, error codes, states, candidate interfaces, CLI/API examples, and bilingual pairs for consistency with the owned names above; scanning must not misclassify historical quotations, user text, counterexamples, or third-party natural language as controlled positive surfaces.

The independent reviewer still needs to inspect high-risk Registry/Catalog, Provider, source/binding, approval/authorization, Run/transaction, and audit/transcript uses for the separation defined here. This document remains a candidate and does not self-certify that the terminology gate has passed.

## 13. Per-concept structured mapping

This section is the mechanically traceable Lexicon artifact required by Intake §8.5.1. "No public IPC" and "no direct CLI" are explicit decisions rather than names deferred to implementation. Any future public wire/API/CLI name must revise this table first. Locale keys are candidate controlled mappings; this PR does not implement resource files. Names inherited from D1-D9 remain owned by their original owner and D10 only references them without creating aliases.

| stable concept/term ID | Chinese formal name | English formal name | Precise definition | owner/layer | Exclusion boundary | canonical wire/API/manifest/schema | code type/function/variable/namespace | CLI/UI label + locale key | Allowed short form | Forbidden/retired/historical aliases | Positive / negative example | First freeze, status, migration/deletion target |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| weftext.term.extension-package | 扩展包 | Extension Package | Named-version distribution/upgrade unit containing content digest, publisher, dependencies, and Contributions | D10 package lifecycle | Not a permission unit, namespace owner, or author object | No public IPC frozen; manifest category uses qualified `Extension Package` | type `ExtensionPackage`; variable `extension_package`; package ID never enters EntityRef | UI "Extension Package"; locale `term.extensionPackage` (candidate mapping; resource not implemented) | none | Bare `plugin` is forbidden as controlled synonym; historical/user prose may contain it | Positive: one package contains schema+connector; negative: install grants all permissions | First frozen `D10-r01`; mapping completed in `r04`; no published legacy wire; remove generic plugin controlled surfaces before release |
| weftext.term.contribution | 贡献项 | Contribution | Independently admitted, enabled, authorized, and dependency-resolved pure-data or executable capability inside a package | D10 capability lifecycle | Does not inherit whole-package authority and is not a D1 capability response | No public IPC frozen; manifest/schema uses qualified `Contribution` entry | type `Contribution`; variable `contribution`; canonical contribution ID | No standalone CLI by default; Settings label "Contribution"; locale `term.contribution` | none | Package/plugin/provider are forbidden synonyms | Positive: data pack remains usable while connector denied; negative: whole-package grant | `D10-r01`; no legacy compatibility alias |
| weftext.term.pack | 扩展包数据包 | Pack | Declarative/rule/schema/data Contribution set with no arbitrary code/network/secret/file privileges and a parent domain extension point | D10 extension taxonomy + parent domain owner | Not executable Extension/Connector and not an author Profile | No public IPC frozen; manifest must declare pack kind and parent dependency | type `PackContribution`; variable `pack`; namespace still follows D4 owner rules | UI "Pack" under parent domain; locale `term.pack` | pack | Flat "Weftext pack" is forbidden as formal no-parent name; `plugin` historical/user prose only | Positive: Calendar Holiday Schedule pack; negative: networked credential-bearing item still called pack | `D10-r04` explicitly freezes parent-domain lifecycle; existing candidate Pack term retained; no wire migration |
| weftext.term.bundled-module | 内置模块 | Bundled Module | First-party user-visible product-organization abstraction providing domain UX and extension points; disabling it does not change Core identity or author source | D10 package/product mapping; D1 owns only surface/capability availability | Not package identity, D4 namespace ownership, Core contract, or author database | This generation does not claim a pre-existing D1 module ID; the four concrete modules use §14.2 PackageId + module Contribution mappings | Candidate code/namespaces are in §14.2 and are not implemented; no second D4 namespace | Formal bilingual labels and candidate locale keys are in §14.2; locale resources are not implemented; no standalone CLI verb | module (qualified prose only) | Do not conflate module with Extension Package/Pack or D4 ownerId | Positive: Calendar UI hidden while portable author facts remain; negative: hiding UI deletes schema | `D10-r04` freezes the three-axis lifecycle; `D10-r05` adds package/module/schema mapping; no legacy runtime ID |
| weftext.term.capability-catalog | 能力目录 | Capability Catalog | Deployment description of contribution/runtime, dependency resolution, and host privileges under current ActivationBinding | D10 control domain | Not D4 Registry, D1 capability response, or author schema | No public IPC frozen; controlled name `Capability Catalog`; digest enters `ActivationBinding/1` | type `CapabilityCatalog`; variable `capability_catalog` | No ordinary direct UI label; admin diagnostic "Capability Catalog"; locale `term.capabilityCatalog` | Catalog (D10-qualified context only) | Bare Registry/plugin registry forbidden | Positive: records disabled parent dependency; negative: stores second FieldDefinition | `D10-r01`; `r04` adds parent dependency; no legacy wire |
| weftext.term.parent-extension-dependency | 父扩展依赖 | Parent Extension Dependency | Required dependency from a domain Contribution to parentDomainId, extensionPointId, and compatible version range, resolved to an exact parent binding during activation | D10 Capability Catalog; parent semantic owner remains domain module | Not UI visibility, D4 semantic history, or an authorization grant | No public IPC frozen; Catalog dependency record freezes semantics of `parentDomainId`,`extensionPointId`,`requiredVersionRange`,`resolvedParentVersion`,`resolutionState` | type `ParentExtensionDependency`; variable `parent_extension_dependency` | Usually no direct UI; diagnostic "Parent Extension Dependency"; locale `term.parentExtensionDependency` | parent dependency (prose) | UI hidden/install order cannot stand in for dependency state | Positive: Calendar pack pins compatible extension point; negative: holiday contribution runs while Calendar disabled | First frozen `D10-r04`; no legacy alias; any future public wire requires separate review |
| weftext.term.activation-binding | 激活绑定 | Activation Binding | Managed activation generation binding current D4 RegistryBinding, Capability Catalog digest, and trust revision | D10 control domain + D4 consumption boundary | Not D4 semantic generation or runtime health | Controlled internal type `ActivationBinding/1`; no public IPC frozen | type `ActivationBinding`; variable `activation_binding` | No ordinary CLI/UI label; diagnostic "Activation Binding"; locale `term.activationBinding` | none | Unversioned current/latest pointers forbidden | Positive: parent version change creates successor binding; negative: half Registry/half Catalog | `D10-r01`; no legacy wire; historical bindings retained |
| weftext.term.publisher-identity | 发布者身份 | Publisher Identity | Package-signing subject proven by an accepted trust root | D10 package trust | Not namespace ownership, user principal, or Node identity | Manifest/trust semantic name `PublisherIdentity`; public wire not frozen | type `PublisherIdentity`; variable `publisher_identity` | Admin UI "Publisher"; locale `term.publisherIdentity` | publisher (qualified context) | Signature-valid is not a namespace-owner alias | Positive: key-rotation continuity; negative: self-signed first install claims reserved namespace | `D10-r01`; no compatibility alias |
| weftext.term.namespace-claim | 命名空间所有权声明 | Namespace Claim | Managed claim binding PublisherIdentity to an allowed D4 publisher namespace | D10 trust proof consumed by D4 owner gate | Not installation, enablement, or display name | Controlled concept `NamespaceClaim`; D4 namespace-row shape unchanged | type `NamespaceClaim`; variable `namespace_claim` | Admin UI "Namespace Claim"; locale `term.namespaceClaim` | none | Namespace registration=install conflation forbidden | Positive: one active namespace has one owner; negative: two publishers claim it | `D10-r01`; no legacy alias |
| weftext.term.delegation-lease | 委托租约 | Delegation Lease | Finite task/Run authorization that further narrows a D6 principal, including lifetime, scope, maxRuns, and budget accounts | D10 delegation control | Not D6 Policy and cannot add authority or redelegate | Controlled internal type `DelegationLease/1`; no public IPC frozen | type `DelegationLease`; variable `delegation_lease` | UI "Delegation"; locale `term.delegationLease` | Lease (D10-qualified) | Permanent grant/session-wide allow are forbidden synonyms | Positive: maxRuns=1 same Run has later steps; negative: new Run starts after exhaustion | `D10-r01`; maxRuns converged in `r02/r04`; no legacy alias |
| weftext.term.lease-run-use | 运行准入消费 | Lease Run Use | Durable deduplication fact consuming one maxRuns use for a leaseId lineage when a Run first enters protected execution | D10 run admission control | Not occurrence claim, author ledger, or per-step counter | Controlled internal type `LeaseRunUse/1`; no public IPC frozen | type `LeaseRunUse`; variable `lease_run_use` | No direct UI; diagnostic "Run Admission"; locale `term.leaseRunUse` | none | Do not conflate with run counter/step counter | Positive: same-Run planned recovery reuses it; negative: re-consume because remaining=0 | First frozen `D10-r02`; `r04` clarifies same-Run continuation; no legacy alias |
| weftext.term.standing-approval | 持续批准 | Standing Approval | Finite pre-approval for a mechanically decidable future operation set with lifetime/count/budget bounds | D10 approval control + proposed D6 consumption | Not permanent consent, Policy, or natural-language goal | Controlled internal type `StandingApprovalEnvelope/1`; no public IPC frozen | type `StandingApprovalEnvelope`; variable `standing_approval` | UI "Standing Approval"; locale `term.standingApproval` | none | Always allow/remember forever are forbidden aliases | Positive: single_field_member envelope; negative: free whole-source write | `D10-r01`; narrowed with D6/D7 companion in `r02`; inactive |
| weftext.term.approval-use | 批准使用记录 | Approval Use | Binds a prepared request, complete preview, footprint, and one approval reservation/consumption | D10 control + proposed D6 amendment | Not an author plan, receipt, or third ledger | Controlled internal type `ApprovalUse/1`; no public IPC frozen | type `ApprovalUse`; variable `approval_use` | No direct UI; may display "Standing approval used"; locale `term.approvalUse` | none | Client approved=true field forbidden | Positive: reserved→consumed; negative: TTL auto-release | First frozen `D10-r01`; `r02` adds released_terminal; inactive |
| weftext.term.planned-decision-approval | 已计划决议交互批准 | Planned Decision Approval | Finite one-shot interactive authorization for the exact original request after rereading the saved planned preview | D10 control + proposed D6/D7 amendment | Does not reprepare, change OperationId, or revive conflicting plan | Controlled internal type `PlannedDecisionApproval/1`; D7 recovery transport defined by amendment | type `PlannedDecisionApproval`; variable `planned_decision_approval` | UI "Confirm Original Plan"; locale `term.plannedDecisionApproval` | none | Fresh prepare is not a recovery alias | Positive: inspect saved preview after old token expiry; negative: rerun Query/change target | First frozen `D10-r02`; inactive, no legacy alias |
| weftext.term.automation-definition | 自动化定义 | Automation Definition | Versioned schedule definition for one invocation | D10 scheduler control | Not a general DAG, script, or author Document | No public IPC frozen; controlled concept `Automation Definition` | type `AutomationDefinition`; variable `automation_definition` | UI "Automation"; locale `term.automationDefinition` | Automation (product context) | workflow/script forbidden as generation-one synonyms | Positive: one schedule→one invocation; negative: free DAG | `D10-r01`; no legacy wire |
| weftext.term.automation-occurrence | 自动化发生项 | Automation Occurrence | Scheduling opportunity mechanically derived from definition revision and source occurrence | D10 scheduler control | Not author recurrence identity or a new Node | Controlled key `AutomationOccurrenceKey/1`; no public IPC frozen | type `AutomationOccurrenceKey`; variable `automation_occurrence_key` | Usually no standalone label; diagnostic "Occurrence"; locale `term.automationOccurrence` | occurrence (Automation-qualified) | recurrence Node/Run are forbidden synonyms | Positive: terminal K rescan returns same Run; negative: enable creates second Run | `D10-r01`; terminal dedup added in `r02` |
| weftext.term.run | 运行 | Run | Managed execution record for Agent or Automation plus step outcomes | D10 runtime control | Not author transaction or receipt | No public IPC frozen; controlled concept `Run` | type `Run`; variable `run` | UI "Run"; locale `term.run` | Run | job=author transaction conflation forbidden | Positive: cancelled Run contains committed step; negative: Run completed=author committed | `D10-r01`; no legacy alias |
| weftext.term.agent-session | Agent 会话 | Agent Session | Interactive Agent mode of Run, bound to current task, ContextBundle, and tool allowlist | D10 Agent runtime | Not principal, Document, or durable authority | No public IPC frozen; controlled concept `Agent Session` | type `AgentSession`; variable `agent_session` | UI "Agent Session"; locale `term.agentSession` | Agent session | chat=authority and session=grant conflations forbidden | Positive: one interactive Run; negative: session inherits full Workspace | Mapping first frozen `D10-r04`; semantics inherited from r01 Candidate |
| weftext.term.context-bundle | 上下文包 | Context Bundle | Current-authorized, minimized, version-bound input for a specific recipient | D10 context/egress control | Not author snapshot, authorization token, or secret container | No public IPC frozen; controlled concept `ContextBundle` | type `ContextBundle`; variable `context_bundle` | Usually not directly shown; diagnostic "Context"; locale `term.contextBundle` | none | prompt=context authority conflation forbidden | Positive: Model A bundle cannot be sent to Model B; negative: ambient workspace | `D10-r01`; no legacy alias |
| weftext.term.tool-value-profile | 工具值配置 | Tool Value Profile | Tool parameter/result algebra reusing a bounded D7 TypeSpec/V subset | D10 Tool Adapter | Not arbitrary JSON Schema or full D4 value domain | Controlled profile `ToolValueProfile/1`; public transport adapter-specific | type `ToolValue`; variable `tool_value` | No direct UI; developer diagnostic "Tool Value"; locale `term.toolValueProfile` | ToolValue | any/open-map/float fallback forbidden | Positive: exact int64; negative: JS-double rounding | `D10-r01`; no compatibility fallback |
| weftext.term.input-slot | 输入槽 | Input Slot | Per-invocation file-input handle bound to exact bytes/media/purpose/recipient | D10 Tool/Model runtime | Not a path, ResourceRef, or cross-call file handle | Controlled concept `InputSlot`; no public IPC frozen | type `InputSlot`; variable `input_slot` | No direct UI; may display "Input File"; locale `term.inputSlot` | none | filepath/path handle aliases forbidden | Positive: exact attachment bytes; negative: pass `C:\Users\...` | `D10-r01`; no legacy alias |
| weftext.term.tool-adapter | 工具适配器 | Tool Adapter | Maps an admitted external tool protocol to ToolValue and effect class | D10 external capability adapter | Not a D7 author Action adapter | No public IPC frozen; manifest contribution kind `Tool Adapter` (semantic name) | type `ToolAdapter`; variable `tool_adapter` | Admin UI "Tool Adapter"; locale `term.toolAdapter` | tool adapter | tool=permission conflation forbidden | Positive: locally classify delete tool as mutation; negative: trust remote readOnly | `D10-r01`; no legacy alias |
| weftext.term.mcp-adapter | MCP 适配器 | MCP Adapter | Tool Adapter using MCP as discovery/transport protocol | D10 Tool Adapter | MCP metadata is not policy/identity/permission | External MCP is the transport; Weftext controlled category is `MCP Adapter` | type `McpAdapter`; variable `mcp_adapter` | UI "MCP Adapter"; locale `term.mcpAdapter` | MCP | MCP server=trusted principal conflation forbidden | Positive: schema drift becomes pending; negative: execute immediately on discovery | `D10-r01`; no legacy alias |
| weftext.term.model-adapter | 模型适配器 | Model Adapter | Adapter connecting managed ContextBundle to model transport | D10 Agent runtime | Model is not author authority or secret store | No public IPC frozen; manifest contribution kind `Model Adapter` (semantic name) | type `ModelAdapter`; variable `model_adapter` | UI "Model"; locale `term.modelAdapter` | model adapter | model provider=author conflation forbidden | Positive: recipient-specific egress; negative: switch provider using old grant | `D10-r01`; no legacy alias |
| weftext.term.connector | 连接器 | Connector | Protocol/account/cursor/version and sync-control adapter for a named external system | D10 connector control + D3/D9 mappings | Not author source, EntityRef, or generic sync authority | No public IPC frozen; manifest contribution kind `Connector` (semantic name) | type `Connector`; variable `connector` | UI "Connector"; locale `term.connector` | none | provider/cursor=identity conflation forbidden | Positive: cursor in control state; negative: cursor written to author source | `D10-r01`; no legacy alias |
| weftext.term.secret-reference | 凭据引用 | Secret Reference | Managed reference and generation for a credential in OS/Server secret storage | D10 secret control | Not secret bytes, author source, or ToolValue | Controlled concept `SecretRef`; public secret-value wire forbidden | type `SecretRef`; variable `secret_ref` | UI "Credential"; locale `term.secretReference` | SecretRef | Token/credential bytes are not aliases for reference | Positive: transport injection; negative: prompt/log exposure | `D10-r01`; no secret compatibility fallback |
| weftext.term.external-effect-intent | 外部效果意图 | External Effect Intent | One frozen external mutation request binding target/account/request/idempotency/approval/budget | D10 external-effect control | Not a D6 transaction, author receipt, or rollback | Controlled internal type `ExternalEffectIntent/1`; no public IPC frozen | type `ExternalEffectIntent`; variable `external_effect_intent` | UI "External Action"; locale `term.externalEffectIntent` | effect intent (prose) | transaction/rollback synonyms forbidden | Positive: reconcile same key; negative: retry unknown with new key | `D10-r01`; no legacy alias |
| weftext.term.external-outcome-unknown | 外部结果未知 | External Outcome Unknown | Durable outcome state where external mutation occurrence/completeness cannot be proven | D10 external-effect recovery | Not timeout, failed, or cancelled | Controlled state `outcome_unknown`; external adapter must preserve it | enum `ExternalOutcome::Unknown`; variable `outcome_unknown` | UI "Outcome Unknown"; locale `term.externalOutcomeUnknown` | unknown (external-effect context only) | timeout/failure/cancel aliases forbidden | Positive: response lost after send; negative: treat as failed and retry | `D10-r01`; no legacy alias |
| weftext.term.cost-reservation | 费用预留 | Cost Reservation | Finite cost upper bound atomically reserved across one or more accounts before a billable attempt | D10 cost control | Not billing fact, approval count, or effect idempotency | CONTROL-CONTRACT §10: `reserved→settled\|released\|uncertain` plus `uncertain→settled\|released`; only settled/released are terminal | type `CostReservation`; variable `cost_reservation` | UI "Cost Reservation"; locale `term.costReservation` (candidate, not implemented) | none | released=zero bill and uncertain=terminal are forbidden | Positive: sent zero bill `settled(0)`; negative: TTL/admin-without-evidence releases uncertain | `D10-r01`; `r05` makes uncertain explicitly recoverable non-terminal; no legacy alias |
| weftext.term.audit-started | 审计开始记录 | Audit Started Record | Minimum durable security-audit fact written before a protected step executes | D10 audit control | Not success, receipt, or ordinary telemetry | No public IPC frozen; controlled event semantic `audit_started` | type `AuditStartedRecord`; variable `audit_started` | Usually no direct UI; audit viewer label "Started"; locale `term.auditStarted` | none | log line=authoritative audit conflation forbidden | Positive: execute after local spool; negative: remote collector outage means local audit failure | `D10-r01`; no legacy alias |
| weftext.term.money | 费用值 | Money | Exact cost representation using currency + Counter microUnits | D10 cost value | No implicit FX or binary float | Controlled internal type `Money/1`; no public IPC frozen | type `Money`; variable `money` | UI formats by currency; no standalone concept locale label | none | float money / implicit conversion forbidden | Positive: USD microUnits; negative: implicit cross-currency sum | `D10-r01`; no legacy alias |

### 13.1 Inherited names and collision dispositions

| Name/concept | Original owner | D10 consumption rule | rejected collision / negative gate |
| --- | --- | --- | --- |
| `RegistrySnapshot/1`, `RegistryBinding/1`, Field/Facet | D4 | Reuse exactly; D10 only authenticates source and supplies the complete binding | Capability Catalog cannot be renamed Registry and cannot store a second FieldDefinition |
| `EntityRef`, `NodeRef`, `SourceBinding`, `OriginBinding`, `Provenance` | D3 | Reuse exactly; control IDs/provider IDs never enter these types | `adoption_binding` cannot become an OriginBinding alias; see B10-01 amendment |
| `PrincipalContext`, `ObservationScope`, D6 Policy/error | D6 | D10 delegation only narrows and never replaces them | Agent permission/approval cannot override a D6 deny |
| `ActionSpec`, `PreparedActionBinding/2`, `EffectManifest/1` | D7 | Workspace Actions still use original prepare/preview/commit | tool plan / AgentPlan cannot become a second Action wire |
| Draft / `PreparedEditBinding/1` | D8 | Interactive edits still use Draft/confirmation | model proposal is not Draft unless it actually enters D8 Draft |
| Conversion Provider / Route / PublicationReceipt | D9 | Remain qualified to conversion/publication owner | Bare D10 Provider cannot overwrite D9 Provider |
| author source / Provenance | D2/D3 | source is author payload; Provenance is non-authorizing source evidence | external input/provider state/tool result cannot be called author source |
| D1 capability / unavailable reason | D1 | Product availability keeps D1 closed reasons and fixed precedence | D10 dependency state cannot create a parallel product-availability reason |

## 14. R05 control, capability, and first-party module mappings

This section completes the Intake §8.5.1 mapping for new public/internal controlled concepts from CONTROL-CONTRACT. Candidate code namespaces and locale keys are design mappings rather than implementation evidence. Records marked as having no public IPC cannot be freely constructed by ordinary callers.

### 14.1 Control-record and recovery concepts

| stable concept ID | Chinese formal name | English formal name | precise definition | owner/layer | exclusion | canonical wire/API/manifest/schema | code symbol convention | CLI/UI label + locale | short form | forbidden/historical alias | positive / negative example | first freeze, status, migration |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| weftext.term.control-prepare-binding | 控制准备绑定 | Control Prepare Binding | Durable binding from stable key to complete canonical intent, allocated IDs, original commit request, preview, and dependency pins | D10 control | Not author decision, receipt, or authorization token | Internal ControlPrepareBinding/1; public prepare/result are CONTROL-CONTRACT §7–§8 | type ControlPrepareBinding; variable control_prepare_binding; namespace d10::control | No ordinary CLI; diagnostic "Control Preparation"; candidate locale term.controlPrepareBinding, not implemented | none | request cache and second ledger forbidden | Positive: recover same prepare after lost response; negative: same key allocates new OperationId | R05 candidate; no legacy alias |
| weftext.term.deployment-control-policy | 部署控制策略 | Deployment Control Policy | Authority listing H administrators and named cost reconcilers for one store incarnation | D10 host control | Not D6 Workspace Policy, IssuerControlPolicy, or Workspace grant | DeploymentControlPolicy/1; no author wire | type DeploymentControlPolicy; variable deployment_control_policy; namespace d10::control | Administrative diagnostic "Deployment Control Policy"; candidate locale term.deploymentControlPolicy, not implemented | none | first-login-admin and workspace-admin aliases forbidden | Positive: deployment operator configures explicitly; negative: first HTTP connection takes control | R05 candidate; no legacy alias |
| weftext.term.resource-use-grant | 资源使用授权 | Resource Use Grant | Finite use authorization plus cumulative usage for exactly one of cost/secret/egress/external-effect | D10 host/control | Not resource administration, author permission, or prepaid balance | ResourceUseGrant/1 + ResourcePermission/1; CONTROL-CONTRACT §6 | type ResourceUseGrant; variable resource_use_grant; namespace d10::control | UI "Resource Use Grant"; candidate locale term.resourceUseGrant, not implemented | grant (D10-qualified) | whole-package grant and quota-reset-on-renew forbidden | Positive: user consumes bounded share of account D; negative: grant holder edits D ceiling | R05 candidate; no legacy alias |
| weftext.term.deployment-control-decision | 部署控制决议 | Deployment Control Decision | Immutable applied outcome for one host-control stable key | D10 host control | Not D6 author receipt or second author ledger | Internal DeploymentControlDecision/1; consumed by host commit/result | type DeploymentControlDecision; variable deployment_control_decision; namespace d10::control | No ordinary UI; diagnostic "Deployment Control Decision"; candidate locale term.deploymentControlDecision, not implemented | none | author receipt forbidden | Positive: replay after lost response; negative: replay changes account again | R05 candidate; no legacy alias |
| weftext.term.cost-settlement-decision | 费用结算决议 | Cost Settlement Decision | Evidence-bound append-only decision settling one reservation as settled(actual) or released | D10 cost control | Not manual accounting adjustment, author ledger, or effect receipt | Internal CostSettlementDecision/1; CONTROL-CONTRACT §10 | type CostSettlementDecision; variable cost_settlement_decision; namespace d10::cost | UI "Cost Settlement"; candidate locale term.costSettlementDecision, not implemented | none | manual release and clear uncertain forbidden | Positive: 100→uncertain→bill20→settled20; negative: administrator enters zero | R05 candidate; no legacy alias |
| weftext.term.secret-stage-ticket | 凭据暂存票据 | Secret Stage Ticket | Account/audience/use-bound reference to an immutable secret version produced by trusted secret channel | D10 secret control | Not secret bytes, ordinary ToolValue, or credential alias | SecretStageTicket/1; produced only by successful d10_secret_stage | type SecretStageTicket; variable secret_stage_ticket; namespace d10::secret | No ordinary UI; diagnostic "Secret Staging"; candidate locale term.secretStageTicket, not implemented | none | plaintext-digest ticket forbidden | Positive: same-secret stable retry returns same ticket; negative: reuse across audience | R05 candidate; no legacy alias |
| weftext.term.execution-stop-latch | 执行停止闩锁 | Execution Stop Latch | Monotonic open→stopped safety fact bound to an exact automation/run incarnation | D10 safety control; consumed by D6 final commit | Not rollback, cancellation receipt, cost release, or permanent deletion | Internal ExecutionStopLatch/1; public d10_emergency_stop | type ExecutionStopLatch; variable execution_stop_latch; namespace d10::control | UI "Emergency Stop"; candidate locale term.executionStop, not implemented | stop in safety-control context | clear-stop and rollback aliases forbidden | Positive: stop works after ordinary budget exhaustion; negative: stop rolls back sent effect | R05 candidate; no clear/migration alias |

### 14.2 Complete mappings for four first-party modules

Common rule: PackageId below is D10 package identity, not D4 SemanticNamespaceId. module/schema are package-local ContributionIds. Code/locale values are candidate mappings and are not implemented. There is no standalone CLI verb; generic management uses complete PackageId+ContributionId.

| stable concept ID | Chinese / English | definition | owner/layer | exclusion | wire/manifest/schema | code convention | UI/locale | short | forbidden/historical alias | positive / negative example | first freeze/status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `weftext.term.calendar-module` | 日历 / Calendar | First-party Calendar product organization consuming existing temporal/Calendar semantics and exposing two child extension points | D10 package/product mapping; D4 semantic owner remains `calendar→first_party,weftext.calendar` | Not Node kind, Facet, or D4 namespace identity | PackageId `weftext.calendar`; module contribution `module`; schema contribution `schema`→`calendar/period-note`,`calendar/range-note`,`calendar/event`,`semanticMajor=1`; extension points `calendar-system`,`holiday-schedule` | `CalendarModuleContribution`; `calendar_module`; candidate namespace `d10::bundled::calendar` | label 日历/Calendar; candidate locale `module.calendar.title`, not implemented | Calendar | Time/Diary/Chrono forbidden as wire/package aliases | Positive: hidden module UI does not delete facts; negative: package update changes same-Facet digest | R05 candidate; no runtime legacy ID |
| `weftext.term.library-module` | 文献库 / Library | First-party bibliographic Work product organization | D10 package/product mapping; D4 semantic owner remains `library→first_party,weftext.library` | Not file library, Workspace, or Work identity | PackageId `weftext.library`; module=`module`; schema=`schema`→`library/work`,`semanticMajor=1`; no generation-one extension point | `LibraryModuleContribution`; `library_module`; candidate namespace `d10::bundled::library` | label 文献库/Library; candidate locale `module.library.title`, not implemented | Library | References/Bibliography/Literature/Works forbidden as wire aliases | Positive: My Works is a derived View; negative: Reference becomes entity owner | R05 candidate; no runtime legacy ID |
| `weftext.term.people-module` | 人物 / People | First-party People UX/product organization | D10 package/product mapping; D4 semantic owner remains `people→first_party,weftext.people` | Not Person Node identity or namespace owner | PackageId `weftext.people`; module=`module`; schema=`schema`→`people/person`,`semanticMajor=1`; no generation-one extension point | `PeopleModuleContribution`; `people_module`; candidate namespace `d10::bundled::people` | label 人物/People; candidate locale `module.people.title`, not implemented | People | PersonModuleId forbidden as D4-owner alias | Positive: names remain when provider UI absent; negative: module disable deletes Person facts | R05 candidate; no runtime legacy ID |
| `weftext.term.organizations-module` | 组织 / Organizations | First-party Organization UX/product organization exposing child schema-pack extension point | D10 package/product mapping; D4 semantic owner remains `organizations→first_party,weftext.organizations` | Not Organization Node identity or national ontology | PackageId `weftext.organizations`; module=`module`; schema=`schema`→`organizations/organization`,`semanticMajor=1`; extension point `schema-pack` | `OrganizationsModuleContribution`; `organizations_module`; candidate namespace `d10::bundled::organizations` | label 组织/Organizations; candidate locale `module.organizations.title`, not implemented | Organizations | Separate Business module/package owner and country-pack global enum are forbidden | Positive: inactive country pack retains accepted schema/raw; negative: disabling module deletes author facts | R05 candidate; no runtime legacy ID |

### 14.3 D10-defined, D1-discovered capability IDs

| stable concept ID | Chinese formal name | English formal name | definition | owner/layer | exclusion | canonical D1 capability ID | code convention | UI/locale | short | forbidden alias | positive / negative example | first freeze/status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| weftext.term.automation-manage-capability | 自动化管理能力 | Automation Management Capability | Self-service/authorized management of finite Automation control records | D10 semantics; D1 discovery/availability | Does not grant author write or deployment manage | automation.manage | AutomationManageCapability; automation_manage | candidate label "Automation Management"; locale not implemented | none | design-accepted=available forbidden | Positive: release+surface+policy pass; negative: forced Mobile visibility | R05 candidate, not product-released |
| weftext.term.workspace-extensions-manage-capability | 工作区扩展管理能力 | Workspace Extensions Management Capability | Manage Workspace activation/package-contribution selection | D10 semantics; D1 discovery/availability | Does not grant deployment trust or Registry content-write authority | workspace.extensions.manage | WorkspaceExtensionsManageCapability; workspace_extensions_manage | candidate label "Workspace Extensions"; locale not implemented | none | policy_admin=runtime-available forbidden | Positive: W+registry_admin activation; negative: hidden UI changes capability | R05 candidate, not product-released |
| `weftext.term.deployment-external-manage-capability` | 部署外部能力管理 | Deployment External Management Capability | Host trust/account/secret/pricing/grant management | D10 host; D1 discovery/availability | Grants no Workspace author read/write | `deployment.external.manage` | `DeploymentExternalManageCapability`; `deployment_external_manage` | candidate admin label; locale not implemented | none | issuer-admin alias forbidden | Positive: H manages secret; negative: Workspace admin changes pricing | R05 candidate, not product-released |
| weftext.term.automation-stop-capability | 自动化停止能力 | Automation Stop Capability | Set irreversible stop latch on authorized automation/run | D10 safety; D1 discovery/availability | Not rollback, cancellation, or settlement | automation.stop | AutomationStopCapability; automation_stop | label "Emergency Stop"; locale not implemented | stop | stop=rollback forbidden | Positive: stop works after ordinary budget exhaustion; negative: stop deletes evidence | R05 candidate, not product-released |
| weftext.term.automation-author-submit-capability | 自动化作者提交能力 | Automation Author Submit Capability | First generation allows only single_field_member Standing Approval into D6 author submit | D10 profile semantics + D6 submit; D1 discovery/availability | Not generic author-write, D8 Draft, or create/delete | automation.author_submit | AutomationAuthorSubmitCapability; automation_author_submit | candidate label "Automatic Single-field Submit"; locale not implemented | none | anonymous old capability profile forbidden | Positive: all D1 gates pass and D6 closed error is implemented; negative: design acceptance alone means available | R05 candidate, not product-released |

## 15. R06 upstream-owner terminology references

R06 does not re-register D8/D9 names under D10 ownership. UPSTREAM-AMENDMENTS §8 is the companion lexicon proposal by the original D8/D9 owners; this section only freezes reverse references used by D10.

The D8-owned concepts are exactly:

```text
weftext.term.edit_session
weftext.term.edit_draft
weftext.term.draft_projection
weftext.term.draft_edit_map
weftext.term.prepared_edit_binding
weftext.term.composition_transaction
weftext.term.caret_affinity
weftext.term.layout_epoch
weftext.term.direction_preference
```

A Draft reference in D10 must explicitly mean D8 `weftext.term.edit_draft`; Prepared Edit Binding means D8 `weftext.term.prepared_edit_binding`; Direction Preference means D8 `weftext.term.direction_preference`. Ownership of the 13 D8 kinds is determined by the owner table in UPSTREAM-AMENDMENTS §8.1 and D10 creates no alias.

The D9-owned new/split concepts are exactly:

```text
weftext.term.source-artifact
weftext.term.import-ir
weftext.term.source-location
weftext.term.import-observation
weftext.term.conversion-provider
weftext.term.conversion-route
weftext.term.import-mapping
weftext.term.mapping-proposal
weftext.term.import-job
weftext.term.coupling-group
weftext.term.import-batch
weftext.term.conversion-input
weftext.term.worker-invocation
weftext.term.template-recipe
weftext.term.template-construction-input
weftext.term.office-template
weftext.term.template-placeholder
weftext.term.style-directive
weftext.term.repeat-band
weftext.term.render-snapshot
weftext.term.d7-result-pin
weftext.term.export-plan
weftext.term.export-input-catalog
weftext.term.export-content-selection
weftext.term.export-projection
weftext.term.export-input-location
weftext.term.export-loss-location
weftext.term.export-loss-report
weftext.term.export-blocks
weftext.term.staged-output
weftext.term.publication-receipt
weftext.term.import-loss-report
weftext.term.import-loss-issue
weftext.term.template-loss-location
weftext.term.image-physical-size
weftext.term.image-size-selection
weftext.term.region-body
```

D7 TerminalSchema/V and `PreparedActionBinding/2`, D3 `SourceBinding`/`ForeignIdentityKey`/`OriginBinding`/`ResourceRegionLocator/l1`, the D2 Template meta-kind, and D3/D6 author receipts remain owned by their original specifications. In particular, `weftext.term.publication-receipt` denotes only D9 external-publication fact and can never alias an author receipt; `weftext.term.d7-result-pin` only pins existing D7 results and acquires no schema/value ownership.
