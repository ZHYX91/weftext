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

Once execution enters D3/D6/D7/D8, that owner returns its original error unchanged. D10 does not wrap D9 errors. Unauthorized callers do not learn format/profile/provider/version, hidden object counts or detailed dependency state. For D9-owned current Workspace business errors, only the original §4a wire2 d9_error {wireVersion:2,kind:"d9_error",code} closed set applies. Invalid caller-selected inputIndex/payload kind is invalid_request only after authorization; positive authenticated protected-record contradiction may be integrity_conflict, missing proof is proof_unavailable and missing source is source_unavailable. An actual worker invalid_output is not a general catalog error. Historical D9/1 error envelopes remain original.

## 5. Analysis/inspect boundary

A complete analysis is immutable and inspectable only after its own full disclosure qualification. Inspect can return the complete fixed IR/mapping or Template construction, source/resource catalog, initial loss report, choices, groups/batches and proposed source/resource objects subject to a bounded output budget. It is not a public arbitrary streaming source API.

Changing any fixed semantic input creates a successor analysis; no token is mutated in place.

## 6. ImportJob recovery

`d9_import_prepare` and `d9_import_next` do not promise job-wide atomicity. Each batch carries one original DecisionKey/OperationId/request. Saved returns original receipt/error bytes subject to current disclosure. Planned/unknown resumes the original request and responsibility. Unseen alone may form a fresh plan.

Pause/cancel prevents only not-yet-planned future batches. It does not roll back committed batches or cancel a real planned author decision.

## 7. Export semantic interface

D9 freezes the semantic flow `prepare→inspect→publish/state/cancel`; this A2 candidate does **not** invent an additional public export wire envelope. The host route may differ by surface but fresh unseen export must consume the same current `ExportPlan/4`, `D9ExportConfirmation/2`, staged-byte and publication-intent semantics. Recorded Plan/Confirmation1-/3 families remain exact recovery inputs.

Every export operation first obeys §4: strict closed decode/version → D1 static capability → current audience/entry authorization → applicable Workspace/domain/P continuity. Only then dispatch by actual operation state. Existing saved/planned/unknown or inspect/confirmation/publish/state/receipt looks up the actual original protected ExportPlan by token tag/version and control responsibility, preserving exact pins/dependency/currentness, confirmation, bytes and original recovery; a fresh unseen prepare never requires an already stored Plan. It fixes the selected D9ExportInputDomain/2 and original potential ObservationScope before any private source/catalog read, then routes ONLY the consumed original owner inputs by the single eight-domain table in final FC SCHEMAS §6.6.1: document uses source_read and real SourceVersion/Observation/source pin, resource uses resource_read and real Resource bytes/pin, native_table uses qualified D2 table plus only selected narrow D4 Field/Resource, node_collection consumes a complete authorized Collection/Query producer with selected narrow Field, query_rows/query_json consume the complete D7 result (graph JSON retains nodes/edges/nodeDetails), annotation consumes current D8AnnotationReadResponse/1, and view selects real D7ResultPin/ViewSpec for original D7 View §7. There is no ninth field inputDomain. The seven closed catalog arms (document, resource, field, annotation_index, annotation_content, template, query_result) and actual template/route/style/asset/resource dependency permissions are checked only when consumed. Query-free exact Source/Resource/Field reads never gain a whole-Workspace Query or Annotation prerequisite; generationPolicy=none Source/Resource/query_json needs no unrelated renderer/profile.

For a View, only after public entry authorization may private routing prove viewInput exists and its selected payload is query_result with a real D7ResultPin. If the authorized caller-selected index=99 is missing, or the caller selected field/annotation_content rather than query_result, the D9/2 Workspace error is the original closed wire2 {wireVersion:2,kind:"d9_error",code:"invalid_request"}; D7 cannot be entered and no synthetic D7 error is returned. If a previously authenticated protected Plan actually contains a reachable, proved contradictory catalog index/kind/record/pin, integrity_conflict is the original wire2 result only after authorization and positive contradiction proof. Missing or unprovable pin/proof is proof_unavailable; unavailable source/Observation is source_unavailable; unavailable original domain continuity uses domain_unavailable/owner recovery as applicable, not a fabricated corruption diagnosis. invalid_output is reserved for actual worker output validation. Current authorization denial plus bad index/layout stays original non-disclosing not_visible/reset before D9 diagnostics; D1/closed-decode gates and original entered D3/D6/D7/D8 errors are never remapped. With a real selected D7 result, its original View §7 order is closed decode → result auth/epoch → complete → exact schema → layout/binding → whole-data structure/order → domain/numbers → budget → delivery; D9 renderer/profile/business errors may not preempt that owner.

After those owner gates, every final FC §6.6.1 selection↔projection/recordPin, View hash/complete_data/accessibility, target/destination/print receipt, canonical asset/pins, route/loss/budget check remains mandatory. Fresh §8.3 and original D9 workers/export §3a require full dataFile/report/manifest name preflight before ordering/token allocation; Core randomly allocates a unique non-public d9_export_plan/4 token BEFORE constructing loss report, manifest and Plan referencing it, generates/validates all dataFiles, freezes exact staged output/proofs and atomically saves the complete Plan before disclosing its token. No second public wire/store/CAS, no unreadable→empty, no reread/requery, and no repair of frozen bytes. Print resolves its actual protected confirmed Plan and compares Plan print target/destination, View binding, actual output and loss without a receipt.target; Resource author operation and server-download delivery remain separate.

## 8. Export to Resource

Saving one staged dataFile as a Resource is a separate D7/D3 author operation using the exact staged bytes, explicit owner/name and current write authorization. Export confirmation grants no write capability. External publication, print delivery and Resource creation expose three distinct outcomes; none can substitute for another.

## 9. Office-template compiler interface

The compiler accepts immutable Office template bytes plus a complete authorized projection and Plan policies. It scans visible tokens once, compiles them to exact binding records, validates style/repeat/layout/safety, and returns staged output plus exact loss evidence. It receives no Workspace path, arbitrary callback or permission handle.

For qualified native-table authoring, the compiler consumes the visible `native.table[...]::column[...]` token. It may derive internal `nt_...` / `nc_...` Plan keys, but the visible template token remains the authoring authority.

## 10. Worker interface

The only worker-facing control object is `WorkerInvocation/1`. The host assigns slots/handles. The worker cannot choose executables, paths, network, retries, loss choices, final publication destination or author operations. Host validates every declared output byte and rejects undeclared output.

## 11. Region interface

D9 may validate/sign only the inner `RegionBody` geometry profile after the original D3 disclosure/currentness gates permit the operation. The outer `ResourceRegionLocator/l1` remains D3-owned. D9 exposes no separate durable region identity.

## 12. Surfaces

Desktop/CLI local mode may coordinate local reviewed providers. Remote Desktop/CLI and WebUI call Server. Server never accepts client arbitrary filesystem paths. Mobile has no conversion execution/delegation/approval path. Presentation differences, CJK/RTL, and transport framing cannot change request bytes or domain outcomes.
