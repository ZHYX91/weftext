---
source_language: zh-CN
translation_of: D9-INTERFACES.zh-CN.md
translation_status: synced
---

[简体中文](D9-INTERFACES.zh-CN.md)

# A2 D9 Interfaces

Status: **author-resolved-pending-independent-review**. These interfaces do not create an author submit protocol outside D7/D3/D6.

## 1. Public artifact interfaces

D9/1 pure artifact requests remain closed:
- `d9_probe` → `d9_probe_result`
- `d9_convert` → `d9_conversion_started`
- `d9_conversion_state` → `d9_conversion_state_result`
- `d9_conversion_cancel`

They consume only immutable `SourceArtifact`, installed route/profile/options and budget. They do not read Workspace author facts. Probe and conversion results are evidence, never author receipts.

## 2. Workspace analysis/import interfaces

Current Workspace orchestration remains D9/2:
- `d9_import_analyze` binds Workspace, CommitDomain, complete Frontier, IR, mapping and budget.
- `d9_import_choose` binds the immutable analysis and the complete loss-choice set.
- `d9_import_prepare` returns `d9_import_prepared`; each prepared arm contains the **actual current D7 prepared outcome**, not a D9 preview substitute.
- `d9_import_next` materializes only the unique next finite batch after authoritative predecessor recovery.
- `d9_import_state` / `d9_import_state_result` operate on the D6-owned ImportJob and original receipts.
- `d9_template_analyze` uses the same analysis/loss/job path with `TemplateConstruct/2`.

WorkspaceRef and CommitDomain must match exactly. Static potential scope/authorization precedes hidden record reads. There is no downgrade after failure, no read-then-hide, no post-read scope expansion and no empty substitution for not-visible/unavailable data.

## 3. Current author handoff

D9 never submits author changes itself. A current prepared author operation is passed through D7 current `PreparedActionBinding/4`, current `D3IdentityOperationRequest/13` or the existing `d6_commit_request/2`, `DependencyProof/3`, current PIntent and `EffectManifest/3` / `EffectBytes/3`.

D3 first performs the original saved/planned/unseen request-and-input lookup, then the final concrete payload/effects/currentness revalidation, and finally seals only the original request. The D6 planning CAS/P remains single. A D9 wrapper cannot add planToken, a second DecisionKey, a second receipt or a second CAS.

MinimumMapping is validated before fetching large binding/PIntent records. The built-in D9 Node-template adapter is the only non-null D9 construction producer; arbitrary public construction kinds reject.

## 4. Error priority

D9 public errors use only the closed current D9 family after D1 surface/capability handling. Ordering is:
closed decode/version → D1 static surface/release capability → current audience/entry authorization → current Workspace/domain/P continuity where applicable → exact pins/dependencies/currentness → format/business validation → budget.

Once execution enters D3/D6/D7/D8, that owner returns its original error unchanged. D10 does not wrap D9 errors. Unauthorized callers do not learn format/profile/provider/version, hidden object counts or detailed dependency state.

## 5. Analysis/inspect boundary

A complete analysis is immutable and inspectable only after its own full disclosure qualification. Inspect can return the complete fixed IR/mapping or Template construction, source/resource catalog, initial loss report, choices, groups/batches and proposed source/resource objects subject to a bounded output budget. It is not a public arbitrary streaming source API.

Changing any fixed semantic input creates a successor analysis; no token is mutated in place.

## 6. ImportJob recovery

`d9_import_prepare` and `d9_import_next` do not promise job-wide atomicity. Each batch carries one original DecisionKey/OperationId/request. Saved returns original receipt/error bytes subject to current disclosure. Planned/unknown resumes the original request and responsibility. Unseen alone may form a fresh plan.

Pause/cancel prevents only not-yet-planned future batches. It does not roll back committed batches or cancel a real planned author decision.

## 7. Export semantic interface

D9 freezes the semantic flow `prepare→inspect→publish/state/cancel`; this A2 candidate does **not** invent an additional public export wire envelope. The host route may differ by surface but must consume the same current `ExportPlan/3`, `D9ExportConfirmation/1`, staged-byte and publication-intent semantics.

Prepare finishes complete input/catalog/projection/loss/staged bytes before returning. Inspect is read-only and rechecks current disclosure. Confirmation binds exact Plan/loss choices. Publish is create-only and binds the original destination intent. State/cancel operate on that publication responsibility and never become author commit.

## 8. Export to Resource

Saving one staged dataFile as a Resource is a separate D7/D3 author operation using the exact staged bytes, explicit owner/name and current write authorization. Export confirmation grants no write capability. External publication and Resource creation expose two distinct outcomes and receipts.

## 9. Office-template compiler interface

The compiler accepts immutable Office template bytes plus a complete authorized projection and Plan policies. It scans visible tokens once, compiles them to exact binding records, validates style/repeat/layout/safety, and returns staged output plus exact loss evidence. It receives no Workspace path, arbitrary callback or permission handle.

For qualified native-table authoring, the compiler consumes the visible `native.table[...]::column[...]` token. It may derive internal `nt_...` / `nc_...` Plan keys, but the visible template token remains the authoring authority.

## 10. Worker interface

The only worker-facing control object is `WorkerInvocation/1`. The host assigns slots/handles. The worker cannot choose executables, paths, network, retries, loss choices, final publication destination or author operations. Host validates every declared output byte and rejects undeclared output.

## 11. Region interface

D9 may validate/sign only the inner `RegionBody` geometry profile after the original D3 disclosure/currentness gates permit the operation. The outer `ResourceRegionLocator/l1` remains D3-owned. D9 exposes no separate durable region identity.

## 12. Surfaces

Desktop/CLI local mode may coordinate local reviewed providers. Remote Desktop/CLI and WebUI call Server. Server never accepts client arbitrary filesystem paths. Mobile has no conversion execution/delegation/approval path. Presentation differences, CJK/RTL, and transport framing cannot change request bytes or domain outcomes.
