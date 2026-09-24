---
source_language: zh-CN
translation_of: TERMINOLOGY.zh-CN.md
translation_status: synced
---

[简体中文](TERMINOLOGY.zh-CN.md)

# D10 Terminology and Naming

revision: D10-r01-candidate-2026-09-25; status: candidate. This lexicon defines only new D10 control/extension concepts and consumption boundaries for D1-D9; it does not reassign any upstream-owned concept. Natural language, historical material, test descriptions, and third-party product vocabulary do not automatically become controlled names.

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
| weftext.term.capability-catalog | 能力目录 / Capability Catalog | D10 | Contribution/runtime availability and host-privilege description for the active deployment | Not the D4 Registry, D1 capability response, or author schema |
| weftext.term.activation-binding | 激活绑定 / Activation Binding | D10 | Managed record binding current D4 RegistryBinding, Capability Catalog digest, and trust revision into one generation | Not a replacement for D4 semantic generation |
| weftext.term.publisher-identity | 发布者身份 / Publisher Identity | D10 | Package-signing subject proven by an accepted trust root | Not namespace ownership itself |
| weftext.term.namespace-claim | 命名空间所有权声明 / Namespace Claim | D10→D4 proof | Managed claim binding publisher identity to an allowed D4 publisher namespace | Install order, display name, and enablement are not claims |
| weftext.term.delegation-lease | 委托租约 / Delegation Lease | D10 | Task/run authorization boundary that further narrows the current D6 principal | Adds no D6 capability and cannot be redelegated |
| weftext.term.standing-approval | 持续批准 / Standing Approval | D10 | Finite pre-approval for a mechanically decidable future operation set with lifetime/count/budget limits | Not permanent consent, natural-language goal, or D6 Policy |
| weftext.term.approval-use | 批准使用记录 / Approval Use | D10 + proposed D6 amendment | Binds one prepared request, complete preview, actual footprint, and one reservation/consumption of an approval | Not an author plan, receipt, or third ledger |
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
- `StandingApprovalEnvelope/1`
- `ApprovalUse/1`
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

**Extension Package** is the installation/upgrade/signature unit and may contain one or more Contributions. Permission and runtime availability are computed per Contribution, so denying one network Connector cannot automatically disable pure-data contributions in the same package.

**Bundled Module** means first-party product organization/UI such as existing Calendar/People/Library directions from D1; a module does not create another namespace owner class or permission domain.

**Pack** is reserved for declarative semantic/data extensions with no arbitrary code/process/network/secret capability. A "pack" that needs code or networking must be reclassified on controlled surfaces as an executable Contribution/Connector rather than using its label to bypass runtime gates.

**plugin** remains only a user-comprehensible generic or historical term. Controlled schema, CLI/API, test fixtures, and candidate design must use the concrete category instead.

## 5. Registry, Catalog, and capability

The **D4 Registry** owns semantic namespaces, Fields/Facets/Relations/Calendar/units and evolves under D4 cumulative rules.

The **Capability Catalog** owns current deployment descriptions for contributions/runtimes and cannot define Field, Facet, or relation semantics. Catalog and Registry changes may be coordinated by one Activation Binding but remain different objects.

A **D1 capability** describes whether a product capability is available on a formal surface for the current release/subject and uses D1's fixed unavailable reasons. D10 Contribution availability is only one downstream fact used by D1 and cannot change D1 reason precedence.

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

**delegation** is a Delegation Lease that further narrows current authorization; it is not an authorization source.

**interactive confirmation** is explicit user confirmation of the original request after a current complete preview; D8/D7 keep this as the default path.

**Standing Approval** pre-allows only the mechanically bounded envelope defined by the candidate and still requires fresh prepare/preview/current authorization.

**ApprovalUse** is the managed binding by which one prepared request actually consumes a Standing Approval; it cannot be client-asserted.

**approval-required** is a D10 control-layer state meaning a Standing Approval cannot be mechanically consumed and interactive confirmation is required; it does not replace original D6/D7 semantic errors.

## 9. Secret, context, egress, and external effect

Secret Reference points only to a credential in secret storage; secret bytes never enter ToolValue, ContextBundle, or ordinary logs.

Context Bundle is actual authorized context for one model/tool invocation. Holding it grants no new read and does not permit switching recipients.

egress means data leaving the current trusted Core/host boundary to a named model/tool/connector/external service; network capability is transport only and does not authorize arbitrary recipients.

external side effect means a request that changes external-system state. Reading external data and writing external data are different effect classes; a "read-only tool" exists only under the locally accepted Weftext contract and not because a remote server self-annotated it.

External Outcome Unknown means the external request's outcome cannot be proven; it is not timeout, failure, or cancellation.

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
