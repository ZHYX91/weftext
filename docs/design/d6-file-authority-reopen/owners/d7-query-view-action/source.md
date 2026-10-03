---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 3240b157-f174-4ab2-a3f2-d2b3be045388.

Candidate status: D7 file-authority coordination repair candidate; not independently accepted, activated or implemented. Historical stage labels and bounded evidence retain only their original scope. This package now consumes the authored PL-IR-01 stable-production-address/current-Observation repair together with current D3/D6 producers, but the new exact candidate still requires independent review and does not close P1 or imply overall activation.

# D7 Query, View, Action and Dynamic Block

This is a complete architecture candidate. Its norm consists jointly of this main contract, Value/CEL, Query Algebra, View, Execution/Action, Narrow Field Qualification, Prepared Action Binding, Definition Transfer, Preview and Effects Transport, scenarios, terminology and Impact. The four specialist contracts are normative bodies, not optional summaries. This document proves no product, host, release or complete parser implementation.

## 1. Problem, boundaries and inputs

Users need one mechanism for content retrieval, reusable parameterized queries, node collections, task/people/organization/calendar/library projections, complete statistics, charts and controlled changes. Core is the interpretation/execution authority. Local Desktop/CLI/Mobile Core and Server Core execute the same contract; WebUI calls only Server. Interfaces, renderers, plugins and conversion workers neither supplement domain computation nor write source directly.

Fixed S preserves complete upstream obligations of D1, D2 Profile2/wire2, D4 Context2/Binding2 and Recurrence1, and D5 without durable Records. This candidate consumes this package's D3 wire12/Result9 and D6 Policy/3/current Control producers. A complete candidate's existence is not independent cross-owner acceptance. D1/D2 and D4 type/domain semantics retain their respective original contracts. QuerySpec v5 is only an input awaiting review, not the current product wire.

Query selects/transforms data. View maps completely successful data to presentation channels. Action prepares explicit changes for original D3/D6 transactions. DynamicBlock stores one Query and one View call with explicit bindings. None adds a content-entity category.

## 2. Extension does not mean new syntax

Query domains are generic upstream objects/occurrences; operations are generic algebra. FacetId, FieldId and relation FieldId are Registry data, not grammar enums. New domains such as equipment, courses and contracts register combinations of existing D4 types/constraints, then use original scan/read/filter/traverse/aggregate. They cannot add `scan_people`, `scan_organizations` or domain-specific CEL functions. Built-in namespaces gain no special permission.

Registry supplies version-fixed static schemas within the same AuthorizedCut. Every field reference is a compile-time-known complete FieldId. CEL cannot choose arbitrary Fields by runtime strings or bypass dependency analysis. Parameters may control values and exact entity sets, not dynamically change FieldId, FacetId, operators or types. Physical extension installation, code execution, signatures, credentials, upgrades and external acquisition belong to D10. Query consumes only accepted definitions and author facts already admitted to current authority.

Known unavailable definitions, unknown definitions and unknown generic features yield `definition_unavailable`, `unknown_definition` and `unsupported_feature`, respectively, never continuation as missing fields/empty rows. These diagnostics follow definition-disclosure qualification; otherwise return `not_visible`. Removing a used definition makes dependent queries unavailable and resets old results, without text inference or ignored filters. Unused unknown opaque author entries retain D4 preservation without forcing unrelated queries to depend on them; full D2 source validity and actual read qualification still apply.

Only new generic operators, types or result semantics extend the stable feature-ID contract. For example, Algebra §11 first freezes shared temporal semantics for explicit period-boundary projection; later equipment/course Fields reuse it without grammar changes. Compatible additions do not raise the semantic major. Changes to old input meaning, defaults, types/errors, permissions, ordering or portable wire require a new major. Core infers features from the complete call closure; authors have no `requires` field. New business catalog entries are not Query features.

## 3. Identity-free results and six boundaries

Only complete D3 NodeRef, ResourceRef and AnnotationRef are durable content references; WorkspaceRef is a namespace. Task is an ordinary Node with exact tasks/task; Template is a Core meta-kind; both remain NodeRef. Document, heading, checklist, native row, Field occurrence, Query/View definition, result row and Calendar occurrence do not become a fourth entity category. There is no RecordRef, TaskRef, ViewRef, CollectionRef, DocumentRef or bare-UUID coercion.

| Category | Lifetime and role | Forbidden uses |
|---|---|---|
| EntityRef | D3-qualified stable identity, with separate current authorization/lifecycle checks | Ref existence grants no read/write; title/path cannot substitute |
| Locator | Complete revision-bound D3 position with its separate resolver | No generic Locator TypeSpec or CEL resolver. D4 author provenance may explicitly project original structure under Value §5.1 without location/write capability or durable identity |
| LogicalOccurrenceKey | Canonical Core-internal semantic invocation/operator/source-occurrence key | Never exposed to callers in CEL, View, wire, diagnostics or logs |
| ResultRowHandle | One row selection in a complete result epoch; opaque and temporary | Not permanent cached identity, not reused across results, not directly writable |
| Provenance | Core-internal row dependency/source trail | Neither D4 author provenance nor authorization |
| ActionEvidence | Managed proof of current principal, action, exact target, version, scope and deadline | Not generalized permission/capability and no target expansion |

D4 provenance in result cells is an ordinary author-supplied value that passed its original type gate and read authorization for the complete enclosing author fact. Its name must be `authoredProvenance`. Saved author Refs/Locators project their complete original values under Value §5.1, without querying or asserting current target state/location. Do not conflate them with prohibited leakage of internal row lineage/Locators. Actual resolution, relation endpoints and content reads separately require current target authorization. D3 Query Provenance remains internal and cannot appear in a namesake public field; public author values carry/restore no internal ActionEvidence.

## 4. Normative documents and versions

Every D7 JSON uses strict UTF-8 and a single object; duplicate keys, unknown members/enums and unpaired surrogates reject. Structural integers follow D6 Counter's non-Boolean lexical rules, with no fraction/exponent/-0 and the specified upper bound. Each optional-member default is explicit; otherwise members are required. JSON null is accepted only where the closed schema explicitly lists it. Named null branches exist in PreparedActionBinding/3 constructionInput/resolutionInput, MinimumMapping/3 registryBinding/resolutionAccess and the original effect Node-root sentinel. This does not extend any other author/wire optional/null rule or rewrite D2/D3 import decoders.

QuerySpec's exact top-level keys are `format:"weftext.query"`, `version:1`, `parameters`, `relations`, `scalars`, `result`. Persistence uses SavedQueryDefinition to wrap QuerySpec and optional CollectionCreationPolicy; pure QuerySpec has no creation policy. ViewSpec is a separate format. QuerySpec has no view, layout, pageSize, refresh or writes. Query author JSON is not result JSON; result is not source.

`parameters` is an array of ParameterSpec `{name,type,required,default?}`. Names uniquely match `[a-z][a-z0-9_]{0,63}`. required=true forbids default; required=false requires a TypedLiteral default, even an explicit Optional none. arguments is a same-name TypedLiteral object: unknown parameters reject, missing required parameters yield unbound_parameter, and missing optional parameters use definition defaults. Explicit none is valid only for Optional. Types match exactly without implicit nullable/int/ref conversions.

Each relation has `id`, `op` and its operator's members. IDs use the same name grammar, with separate relation/scalar namespaces. relations/scalars may be listed in any valid topological order and every reference resolves. The joint graph includes relation→scalar and scalar→relation; cycles reject. There is no implicit main pipeline. result is `{kind:"rows",relation:id}`, `{kind:"scalar",scalar:id}` or Execution §2's complete graph variant. Unreachable nodes reject to exclude hidden expressions/error branches. Definition parameters may be unused, but unused arguments cannot influence result identity.

Normalization retains semantic order inside operations/arrays, such as projected fields, sort keys and literal lists, but not local ID spelling or ordering of independent topological nodes. It uses §5's canonical graph, not arbitrary equivalent SQL/CEL normalization. Product caches may use D3-CJ3 plus a domain-separated SHA-256 digest, but must check the complete corresponding canonical description; hash collisions cannot select a wrong schema/permission context. This is a product algorithm, not review packaging.

## 5. Canonical DAG, errors and optimization

CanonicalGraph(Q) first removes IDs and directs references to graph nodes. Starting at result, traverse dependency edges in member-name UTF8 order, array-index order and expression-AST-child order. Assign canonicalOrdinal from zero on first visit, then recurse into dependencies; repeated references reuse the ordinal. Each ordinal's canonical record stores op, nonreference parameters, canonical expression AST with scalar references replaced by target ordinals while ordinary string literals remain unchanged, reference ordinals and declared field order. ParameterSpecs sort by name bytes. Each operator schema explicitly identifies reference keys; ordinary strings are not inferred as edges. Saved queries in the definition closure receive independent namespaces by caller path. The algorithm is independent of author IDs, topological array order, physical execution and index order. Different sharing structures may produce different canonicalGraphs; no common-subexpression equivalence is claimed.

The static Value/CEL checker infers expression totality as `total` or `may_error`. Ordinary filter requires bool and may_error is allowed; related predicates must be total bool. No error-as-none, null-as-false, bad-row skipping or partial success exists. Errors concern all semantically reachable evaluation over logical inputs, not only a physically fetched first page.

Base failure order is strict decode→current request/definition visibility→version/feature/Registry closure→type/graph/context→complete authorized execution→complete terminal type/order/size→publish. Outer current authorization precedes saved errors. Within execution, each failure candidate has the internal ordering key `(canonicalOrdinal, LogicalOccurrenceKey, expression AST path, fixed code rank)`. Report only the canonical minimum; independent branches must reach a determination, not race for first completion. Upstream failure prevents dependent nodes; other determinable branches still evaluate under budget. Budget/cancellation/environment unavailability are separate terminal results, never fabricated CEL errors. If complete logical determination cannot finish within budget, return the resource/cancellation class without promising the minimum CEL error.

Public diagnostic is exactly `{code,operator? ,expressionPath?}`. operator is only canonicalOrdinal; expressionPath is a static AST index array. No internal occurrence key, entity, value, path, hidden-candidate count or execution progress appears. Execution defines fixed code rank. Delivery requires observability of the diagnostic's inputs; otherwise `not_visible`.

Optimization preserves types, bag multiplicity, orderedness, internal occurrences, reachable errors, positive/negative dependencies, authorization and budgets. `may_error` cannot cross filter/take/security barriers; short-circuit cannot skip a branch whose error must be reported. A related total predicate may short-circuit existence over authorized inputs but retains complete range dependencies. Batch defines semantics. Parallel/cache/incremental execution requires an equivalence proof; otherwise run batch or reset.

## 6. Product choices and rejected alternatives

The choice is relation DAG plus pure CEL, not SQL/DQL/JS, executable Views, per-business-module query languages or a durable result-record database. DAG makes shared scalars and multilayer aggregation explicit, at the cost of author JSON unsuited to handwriting. D8 may supply a structured editor emitting exactly the same QuerySpec. Facet/Field extension adds no grammar.

v1 has no arbitrary join, union, recursive QueryRef, user functions, arbitrary code, external fetch, arbitrary JSON path, durable Record or View writes. Noncorrelated scalars, one-hop relation traversal and related(any/none) cover the specified combinations. Cases requiring full join/window/quantile are explicitly deferred in the scenario table; renderers cannot add them secretly.

This version explicitly accepts the memory/disk and initial-latency cost of validating complete results. Any insufficient budget fails wholly. Users may explicitly change semantic take/scope and execute a new Query, but the system cannot truncate the original. Field rights alone do not grant the new narrow editing path: explicit source-envelope metadata and specified-CommitDomain commit-sequence qualification are also required. Narrow qualification publishes these observation costs individually with an actual Registry/D2/D4 positive construction. It grants no body/other-Field read.

## 7. Acceptance boundary

Every original Mandatory requirement, nonrollback matrix entry, added extension-without-new-syntax constraint and retained D6 limitation must be covered. Activation additionally requires this run's authorized independent complete combination review, complex-dispute and final global review, no open P0/P1, terminology passing and acceptance by the sole coordinator. A local model proves only its listed subalgebra, not complete D2/D3/D4 decoders, real permissions, OS persistence, renderers or six implemented conformance callers.

## 8. Current sources, complete proofs and consumer boundaries

Current execution uses wireVersion2, protected Action records PreparedActionBinding/3 and full preview EffectManifest/2/EffectBytes/2. QuerySpec, SavedQueryDefinition, ViewSpec, DynamicBlock and ActionSpec retain their respective /1 closed author schemas. These versions govern runtime proofs, preparation records and delivery independently; similar names do not make them interchangeable or permit fields in old closed schemas. D3 native wire12 is the sole identity submit; D6 current commit/2 the sole source/action submit. D7 has no third decision or direct filesystem write.

The real source producer is D6 SourceObservation/1. Complete SourceVersion/2 records production domain/production observationEpoch/revision or externalSequence. SourceObservation records this observerDomain/observation epoch, FileObjectBinding, exact pins and complete current control/proof cut. SourceVersionRef/1 is only a narrow reference to a current protected observation, neither a portable production address nor authority to read all bytes. Equal Ref/revision, A:1 versus B:1, equal hash, mtime/path or a saved old Locator cannot prove the same current source. D7 consumes only D6's actually successful complete observations, never re-signing tokens or putting a conflict installation wrapper into ordinary sourceInputs. A genuinely observed external source not yet admitted as managed retains its actual external version; externalSequence cannot become managed revision.

Each complete Query proves every semantic result only within its explicitly selected scope. There is no exploratory/partial Query success, and semantic_pending, I misses, placeholders, unread sources and Frontier vectors are not complete negative evidence. Prior definition/permission/potential-observation scope precedes author reads; Algebra/Execution produce the full actual-source and positive/negative-range inventory individually. A missing strong-path dependency rejects that path only. Unrelated ordinary source/Resource reads, Draft, human save and already qualified local projections retain their actual owner contracts rather than a global D7 gate.

Current D3 guarded resolution closes through its complete typed prepare, native request, D3CanonicalEffectPlan/1 and public D3CanonicalEffects/1, actual D7 /3 record and full transport together. Original twelve receipt arrays express only native effects; canonical extension is mandatory, and saved commit is distinct from current effects_unavailable. D8 PreparedEditBinding/2 is an actual D6/full preview producer, not a new ActionSpec. D9 TemplateRecipe/2 is a persistent production address and TemplateConstructionInput/2 a complete current input. D7 consumes actual runtime qualification rather than upgrading recipe history directly into Observation. D10 current plan/execution consumers follow these actual /2/3 contracts; naming in D10 cannot replace a missing producer here.

PL-IR-01 now uses the authored D3/D6 stable-production-address/current-Observation repair. A persisted managed Locator can qualify a **new** read on another replica only through the canonical sealed binding, that replica's exact current Observation, exact coordinate/profile checks and the original authorization order; no old result/ActionEvidence/preparation is re-signed. Missing canonical binding/outbox, current Observation, exact production-version equality or coordinate/profile evidence yields the existing stale/incomplete/unavailable outcome for that use rather than an undefined integration gate. The repair remains pending independent review and does not close P1. Every real old preparation/result/effect/planned/saved/unknown record recovers using its original decoder and retention obligations.
