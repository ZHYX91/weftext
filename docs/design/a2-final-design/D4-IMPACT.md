---
source_language: zh-CN
translation_of: D4-IMPACT.zh-CN.md
translation_status: synced
---

[简体中文](D4-IMPACT.zh-CN.md)

Source document ID: ebd24e10-6020-41e0-8a08-f494e46c21ec.

# D4 Implementation Impact and Test Outline — D6-FA-r01

Candidate status: D6-FA-r01; P2 coordinated author candidate; not accepted, not activated, not implemented. All fixed-S revision37 preservation, revision36 admission, and revision35 integration obligations remain. This file adds D6-FA-r01 `SourceVersion/2`, SemanticState, and local-versus-complete consumer verification without treating historical runs as new-version success.

## 0. A2 current impact delta

Current unseen/fresh D4 consumers use `InputDescriptor/3` + `DependencyProof/3` + fifteen-arm `DependencyKey/3`, including `document_format` whenever managed Document semantics are parsed. A changed format binding/stamp invalidates the affected current D4 preparation even if source bytes are unchanged. Inner `Entry/1`, `RelationReadContext/2`, `RelationReadBinding/2`, `RecurrenceReadContext/1`, source/copy effects, and historical decoder bytes do not mechanically change version. Genuine saved/planned/unknown records recover under their recorded outer family before any new-consumer gate. Tests below remain design obligations unless an actual run is named; document checks are not runtime, OS, GUI, or replica execution.

## 1. implementation effect graph

~~~text
D6/D3 outer visibility/permission + authorized ObservationScope/2 upper bound
  -> outer namespace owner + RegistryBinding/schema/contribution availability
  -> authenticated Registry/catalog context
  -> qualified SourceObservation/1 + SourceVersionRef/1 (current source input)
       containing SourceVersion/2 (production version, not currentness alone)
  -> D2 exact source + raw carrier/span extraction
  -> D4 strict Entry decode
  -> TypeSpec / qualifiers / provenance
  -> declared/effective Facet closure
  -> authorized relation / Calendar / catalog validation
  -> D4 proposed semantic state
       -> D6 InputDescriptor/2 + actual DependencyProof/2 + Frontier/2 proof cut
       -> operation-applicable ordinary or complete semantic qualification
       -> strict or fully A§4.1-qualified human observed_only choice BEFORE planning
       -> planning CAS freezes protection; no fallback after planning starts
       -> file install + P seal
       -> portable publication
  -> future D7 complete Query/Action consumer

SourceVersion/2 retains its production commitDomain and version history. Current SourceObservation/1 is an outer qualification and does not redefine production version semantics; observerDomain must equal the operation CommitDomain, with matching entity/file/evidence/control/Registry/relation-incidence dependencies in the current observation cut. SourceVersionRef/1 selects the complete protected observation through sourceToken, not a bare revision, hash, I cache, or production version.

Source body remains authoritative. Rebuildable I is not a body mirror. Frontier/2 is a sealed causal/dependency prefix, not a complete Query result, payload materialization proof, or Registry generation proof. Ordinary and complete qualification remain semantic axes; strict and observed_only remain protection axes, and observed_only is limited to the approved trusted interactive ordinary source-save case.
~~~

I caches rebuildable projections/incidence/search candidates only. P stores non-reconstructible decisions/pins/control evidence. Neither carries a second current Field/Facet/relation truth.

## 2. component impacts

| component | future impact | forbidden shortcut |
|---|---|---|
| D4 Registry loader | preserve complete owner/digest/evolution/catalog validation | infer owner from install order/display name |
| Entry decoder | closed Entry/1, exact spans, raw retention | free JSON / whole-namespace blob |
| Type engine | exact integer/decimal/calendar/instant/ref/alias | host float/date defaults |
| Facet engine | declared/effective, requiredness, conflicts | implicit membership/last-wins |
| relation engine | Context/Binding/complete incidence/post-state | index as truth, inverse double-write |
| Calendar engine | comparator/recurrence/series scope | current page as complete scope |
| D6 adapter | InputDescriptor/2 + SourceVersion/2 + `SourceObservation/1` outer binding | bare revision, hash, or production version across observation cut |
| conflict/repair | exact source + ConflictRecord + current Registry | LWW/hash-only merge |
| downstream D7 | future complete cut/new Prepared | invent Action success from pending |

## 3. future implementation slices

### 3.1 source and Registry

- parse complete D2 exact source and build carrier/Entry spans;
- prove Namespace Owner/RegistryBinding before interpreting contribution-backed identifiers;
- preserve unknown/untrusted/incompatible schema as retained_unavailable raw bytes;
- bind current Registry generation to the source validation cut;
- rebuild Registry cache from portable Registry bytes rather than letting I assert current generation.

### 3.2 local ordinary save

The ordinary source-save adapter computes actual MutationFootprint and separates save semantics from `WriteProtection`. Ordinary save may remain `strict`; only a trusted `interactive_source_save` may explicitly choose `observed_only`, and only for one existing live Document with ordinary replica-local scope, complete source read/replace qualification, no applicable body/Field/Node-control deny, author source write set empty or limited to that Document, and no identity, parent, order, lifecycle, shared policy, Registry, Calendar-scope, or other-entity mutation. DraftBase must match the selected current SourceObservation.

The adapter proves at least current SourceVersion/2 and current SourceObservation/1, complete source, RegistryBinding, touched local Field/Facet/type/cardinality, write permission, one proposed source, D2 validity, and file-install qualification.

Untouched unavailable/invalid namespaces remain byte-equal. Touching one causes ordinary typed edit rejection. Invalid local fact, retained_unavailable namespace, D2 external_invalid, stale Base, or observation continuity failure remains failure, repair, or conflict/reprepare. semantic_pending is permitted only for a real cross-object/complete-range obligation that is not proved and never upgrades a missing strong Action proof.

### 3.3 strong operations

Facet mutation, relation mutation, series-scope unique, typed copy/fork/import, restore/purge, and D7 all_result/bulk/automation write require operation-applicable complete proof. Missing range never downgrades to ordinary and never completes through `observed_only` or an ordinary-save path.

Future acceptance coverage includes both ordinary `strict` and the approved A§4.1 human weak positive cases. Ordinary strict may succeed only with complete source, authorization, current observation, and installation qualification. A trusted `interactive_source_save` may validate `observed_only` only for one existing live Document, ordinary replica-local scope, complete source read/replace qualification, no applicable body/Field/Node-control deny, author write set empty or limited to that Document, no identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation, and DraftBase equal to the selected current `SourceObservation`.

Weak protection is invalid for non-trusted, non-interactive, new or multi-Document, non-ordinary, non-replica-local, incomplete source read/replace, applicable body/Field/Node-control deny, multi-entity write set, identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation, or DraftBase mismatch. Structured bulk, collection, promotion, automation, server checkpoint, Approval, Money, and any strong Action never use weak protection.

A human may explicitly select the weak profile before planning starts even when strict capability is unavailable in the ordinary directory; `writeProtection` is frozen from the start of planning, and strict capability failure, known conflict, authorization failure, durability failure, or strong obligation failure never falls back to weak. D4 local types, D2 validity, unavailable byte equality, and actual source read/write checks remain required. `semantic_pending` represents only real unproved cross-object or complete-range obligations and never converts invalid, unavailable, or incomplete strong proof into success.

`observed_only` durably retains observed before B and user input N. An unobserved external C may exist, and installing N may overwrite C bytes in the current file; a later C write may also replace current file state after N, while durable B/N retention is preserved. Prepare-only is not Saved; unknown install remains `recovery_unknown`; observed competition, stale Base, watcher gap, and continuity gap require conflict/reprepare.

Only an unseen new strong entry point requiring the unfinished D7 consumer is unavailable/owner_update_required; actual saved/planned/unknown records recover under their original contract before this gate. D4 defines no private preparation token.

## 4. test outline

### 4.1 source and grammar

1. Entry raw source, carrier framing, comments, CRLF/LF, unknown namespace round-trip.
2. malformed Entry JSON, duplicate member, missing envelope member, wrong constructor, over-budget.
3. body-only edit with unavailable namespace byte-equal succeeds.
4. changing one byte in that unavailable block rejects.
5. external strict UTF-8/D2 invalid has repair/raw path only and no managed D4 success.
6. equal source hash with advanced observationEpoch invalidates old selector/evidence.
7. concurrent replica source edits create ConflictRecord rather than LWW.

### 4.2 types

Cover integer/decimal canonical boundaries, semantic_code scope, calendar ISO dates, calendar comparators, RFC3339 arbitrary fraction/timezone/DST, date/instant ranges, quantity dimension without unauthorized conversion, Node/Resource/Annotation owner locality, alias limits, and absent optional versus explicit value.

### 4.3 occurrence and note

Cover duplicate values with separate occurrenceKeys, reorder/copy/delete/external-edit target stability, and preserve occurrenceKey as an inner owner Node + FieldId + expected current source revision selector. D6 outer SourceObservation/1 qualification is additional and does not replace the inner selector shape. Cover SourceVersion production epoch/revision/externalSequence changes, invalid SourceObservation token/epoch/continuity, absent note without placeholder, note versus typed qualifiers/provenance, stale SourceVersion, and same-value multi-replica merge without note reassignment.

### 4.4 Facet and availability

Cover declared/effective closure, missing dependency/cycle/conflict/required_field, source-declared versus effective-only tasks/task, Template+tasks/task rejection, legal non-task Template Facet, retained_without_membership, all namespace states, all Node typed states, and Cartesian negatives between D6 SemanticState and D4 typed state.

### 4.5 relations

Cover directed single fact/inverse, symmetric canonical owner, canonical-owner flip on copy/fork, node_ref_or_text literal, every RelationReadContext/2 node-state arm, and Binding/2 exact source/entity/incidence coverage.

SourceVersion/2 outer binding keeps the inner sourceRevision relationship unchanged. SourceVersion/2 production `commitDomain` may differ from the current operation observation domain; a complete current SourceObservation/1 is a valid positive case and is not rejected only because production and observation domains differ. The current observation requires `observerDomain` equal to the operation `CommitDomain`, entityRef equal to sourceVersion.entityRef, and current fileObjectBinding, evidencePins, control, Registry, and incidence dependencies in the same observation cut.

Negative cases cover observerDomain mismatch, entityRef mismatch, missing or stale fileObjectBinding/evidencePins/control/Registry/incidence dependencies, invalid SourceObservation token/epoch/continuity, and SourceVersion production epoch, revision, or externalSequence changes that invalidate the old inner selection. Equal production versions do not restore validity after watcher gap, external replacement, or discontinuous materialization; bare revision, hash, I cache, or key/value cannot recover the protected observation token.

`SourceVersionRef/1` uses `sourceToken` tagged `d6_source_observation/1` to select the complete protected Observation. `Frontier/2` is only the current sealed causal/dependency cut and is not a complete Query, payload, or Registry proof. Masked/unprovable RelationReadContext/2 states never produce successful binding/readSet, expected binding remains exact per member, and authorization/privacy gates remain unchanged.

Cover requiredness not satisfied by inverse, endpoint domain/lifecycle/cardinality, delete tombstoned/not_found old target handling, cleanup before symmetric purge, source migration/copy/fork effects, canonical owner preservation, and hidden endpoint non-disclosure. Preserve unchanged inner wire semantics, owner bijection, current authorization checks, and exact source/entity/incidence coverage for every binding path.

### 4.6 Calendar

Cover homogeneous recurrence arms, count/until exclusivity, 256 collection limits, period rules, explicit horizon/budget/cancellation, series-scope many/unique, complete source/cut rather than I/page for uniqueness, cache rebuild, and pending(calendar) never completing the unique Action.

### 4.7 domain fixtures

People: phone alias, optional label, people/work code, localized-label negative, customLabel, engagement Node/literal, relation direction.

Organizations: structural move versus organizations/parent, relation domains, identifier/classification non-identity.

Library: library/work versus project work/task, DOI/ISBN non-identity, Citation occurrence non-identity.

Tasks: source-declared tasks/task, tasks/status required, dependency endpoint Task proof, UI disable without semantic loss.

### 4.8 ordinary-versus-complete matrix

| operation | complete range missing | only allowed result |
|---|---|---|
| body/disjoint namespace save | irrelevant and ordinary qualification satisfied | success possible |
| local nonrelation Field edit | no cross-object duty and permission/observation valid | success possible |
| local valid Field + relation duty | missing | semantic_pending |
| local Entry invalid | any | reject |
| Assign/Remove Facet | missing | reject/unavailable |
| relation mutation | missing | reject/unavailable |
| Calendar unique | missing | reject/unavailable |
| D3 local Trash | inbound unproved | pending(inbound) |
| restore/purge | missing | reject |
| all_result/automation write | D7 incomplete | unavailable |

Additional weak-save boundary cases:

- valid `observed_only` is limited to trusted interactive source save; after B is read, an unobserved C file change may exist and N installation may overwrite C bytes, while durable B and user input N retention remains required;
- observed C, stale Base, watcher gap, and continuity gap require conflict/reprepare rather than success;
- prepare-only is not saved, and unknown install remains `recovery_unknown`;
- semantic_pending stores exact obligations and never treats invalid, unavailable, D2 external_invalid, or missing strong scope as empty.

### 4.9 r5/r6, I/P, external change

Test r5 pending commit, later r6 complete validation, exact r5 replay, I deletion/rebuild, P loss, external A->B->A, new-device replica registration, and equal hash under different SourceVersion/domain. None upgrades historical proof.

### 4.10 partial index and D7 boundary

A partial index may deliver authorized exploratory rows marked partial/pending. It never proves relation negative completeness, unique negative, Calendar unique, D5 complete collection membership, D7 all_result, automatic bulk target derivation, or Automation/Agent writes.

A full source scan may substitute for a complete index; if it cannot finish the strong consumer is unavailable.

### 4.11 diagnostics

Verify outer authorization/Registry availability before inner disclosure, UTF-8 half-open spans, sourceStart/rank/code ordering, no fabricated span for missing members, raw preservation for unknown contribution, and no hidden endpoint detail from masked state.

## 5. legacy and version tests

- D4 Entry/1 unchanged.
- RelationReadContext/2 and Binding/2 inner wire unchanged.
- RecurrenceReadContext/1 unchanged.
- D4RelationCopyEffects/1 and D4SourceMaterializationEffects/1 unchanged.
- outer new requests use D6 `InputDescriptor/2` and `SourceVersion/2`, and consume current source observation qualification through `sourceInputs[].observation` using `SourceObservation/1`. `SourceVersionRef/1` uses `sourceToken` tagged `d6_source_observation/1` to select the complete protected Observation. `ObservationScope/2`, `Frontier/2`, and `DependencyProof/2` are consumed according to their D6 owner definitions. `SourceVersion/2` retains production version semantics, including its own production `commitDomain`, epoch, and revision information; production domain may differ from the current operation observer domain. Current validity requires matching entityRef and complete fileObjectBinding, evidencePins, control, Registry, incidence, and observation-cut dependencies.
- saved D3 v9/v10/v11 use original decoders.
- historical D7 PreparedActionBinding/1,/2 recover under original rules.
- historical corpus counts are never renamed to claim new consumption passed.

## 6. terminology and non-regression

Mechanically exact-check all fixed-S 26 conceptIds, owners, owned wire/code/UI/locale names, and firstFreeze. D6/D3 imported names never enter the D4 owned set.

Catalog blob remains `ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb`. Any Field/Facet/alias/limit change requires a separate catalog replacement.

## 7. mandatory scenario preservation

Mandatory scenarios are pressure inputs, not feature approval. Future conformance retains People/Organizations/Calendar/ICS/Library, unknown provider, external editor, multi-replica conflict, terminology collision, and no-second-authority cases.

Chart/View and Office-template candidates belong to future D7/D8/D9 owners and do not expand D4 contracts here.

## 8. completion boundary

A future implementation slice is complete only when Core, affected adapters, fixtures, legacy replay, D6 source/pin binding, bilingual public docs, and negative gates are updated together with real execution evidence.

This file is a design candidate only; documentation CI and author self-review are not product-test success or independent acceptance.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings.

## 9. Complete preservation of effective fixed-S regression obligations

This section incorporates the concrete fixed-S implementation/test obligations that remain effective. Historical run counts, legacy wire names, and old implementation status are not upgraded into current success evidence. The semantic obligations remain and consume the current D3 wire12, D6 v2, and future D7 owner through the mapping in the final subsection. H2 versions remain historical saved-decoder/compatibility references and are not active current producers or proof of successful implementation. All fixed-S regression obligations remain, while D4 typed semantic results, legacy bytes, saved recovery behavior, and future owner gates remain unchanged.

### 9.1 source/grammar and parser

Required coverage includes:

- LF/CRLF/CR carrier/Entry;
- duplicate JSON keys, trailing tokens, unknown envelope members, illegal null, float/NaN/Infinity/-Infinity;
- Entry versions 0/2;
- dotted namespace + localField expansion;
- namespace 63/64-byte, FacetId 127/128-byte, empty dot segment, leading/trailing/consecutive hyphen;
- wrong namespace, multiple blocks in one namespace, D2 32/8192/64KiB boundaries;
- identical values with distinct keys accepted;
- same key in different Fields accepted;
- duplicate key in one expanded Field across any same-namespace blocks rejected;
- header attribute resembling a Field remains ordinary header;
- explicit Map emits one chosen carrier target and loss report;
- exact unknown-provider raw round-trip, external formatting damage, no old-projection fallback.

The strict parser preserves half-open UTF-8 spans for every nested key/value/array/scalar, including multibyte text, escaped keys, arbitrary whitespace, and repeated nested key names. Lone surrogate yields only invalid JSON. Tests compare the full Diagnostic sequence, pointer, and span rather than merely asserting an error exists.

### 9.2 types, numeric precision, and host boundaries

Cover canonical integer/decimal; accept -0.5, 0.5, -1, 1.25; decimal zero only 0; reject -0, 0.0, -0.0, 1.20, exponent.

calendar_date covers ISO/non-ISO unavailable, year/month/day precision, ASCII digits only, year 0000 rejection, and a registered comparator whose chronology deliberately differs from lexical order.

zoned instant covers real date/clock/offset, Arabic-Indic digits, +25:00, -00:00, leap-second rejection, sub-microsecond/arbitrary fraction precision, IANA zone + tzdbVersion, and no device-default guess.

date/instant ranges cover bounded/start-open/end-open, both-open rejection, equal/reverse/cross-offset cases, exact decoded ordering, and end-exclusive presentation.

Quantity/unit contribution/dimension coupling and height+kg / weight+cm negatives remain. No conversion occurs without an authenticated conversion contribution.

D3 refs/locators use the exact frozen decoder. Wrong kind, extra/missing members, non-v4, bare UUID/path/title/URL reject. Cross-owner ResourceRef in direct value or provenance rejects.

ValueTypeSpec/ObjectMemberSpec/UnionVariantSpec, alias cycle/missing/depth 8, schema/Entry byte caps, Boolean-as-integer, collection/cardinality/provenance-index bounds, bounded collection nesting, set canonical order/uniqueness, and generic array/map/any/opaque rejection all remain.

At least one 5000-digit legal recurrence/instant fraction path must round-trip exactly. Lower host Decimal context precision cannot change output and raising a process-global integer digit limit cannot be used to evade the author precision contract.

### 9.3 occurrence/note/ABA

Cover two equal phone values with distinct notes/provenance, exact-key patch/reorder/delete, missing/stale revision, delete/re-add ABA, duplicate key from external copy, and same-key/different-Field acceptance.

Identity-preserving move keeps bytes. Cross-owner copy/import/template may preserve key bytes, while a same-owner duplicate receives a fresh key. Sending key into EntityRef/Locator/AnnotationTarget/RecordRef decoder rejects.

Absent note is omitted, an empty placeholder rejects, and note is never behavior-bearing syntax.

### 9.4 Registry, Facet, availability, and diagnostics

Registry evolution covers exact generation+digest, same-generation substituted snapshot, user owner for wrong Workspace, predecessor/tombstone/migration ledger, three-generation no-resurrection, monotonic contribution identity, same-ID semantic drift across type/cardinality/constraint/relation/Facet/tzdb/unit, matching tombstone on deletion, fresh ID + exact migration on replacement, acyclic aliases, and complete 61-Field/7-Facet references.

Availability covers read/body edit/typed edit/query/export/reinstall. Disabling UI/runtime while portable schema remains never changes typed meaning. owner-unprovable/conflict/generation-changed masks inner parser before invocation and parser call count can be asserted zero.

Contribution preflight precedes wrapper structural/type checking, including union extra member, missing calendar comparator, qualifier observedAt + bad confidence, and external provenance observedAt + extra member; availability code is preserved.

Facet coverage includes declared reorder equality, requires closure/cycle/missing/conflict, compatible shared Field definition, Task+Project+Calendar, legal Template+nonTask, D2 rejection of Template+tasks/task, deleting the last required occurrence, Remove retaining Fields, and Cleanup never deleting Fields still used by another Facet.

### 9.5 relation state machine and all 27 Fields

Relation tests use a real stateful one-fact store that preserves raw Entry bytes, parsed TypedValue, qualifiers, note, provenance, occurrenceKey, and resolution and proves strictParse(raw)==entry.

Submitting the same symmetric fact from A->B or B->A yields byte-equal canonical state. Repeating same-key same-payload in one store keeps one source fact, same-key different-payload is fixed collision, distinct keys obey schema cardinality. Self-edge, Trash/restore suspension, and projection/index delete+rebuild are covered.

text<->NodeRef and NodeRef retarget bind complete before/after owner revisions, canonical-owner relocation, and stable occurrenceKey. Any CAS/auth/collision/cardinality/lifecycle/projection failure byte-exactly rolls back the applicable D4 proposed source, revisions and projections with empty write set; original D3 allocation/reservation/burn/custody history is not erased.

expectedSourceRevisions uses D3Integer: 7.0 is not 7 and true is not 1. sourceRevisionOwners covers source-bearing inventory only; a sourceless endpoint never receives a fake revision.

RelationReadContext/2 and Binding/2 cover source_node_state, sourceless_node_state, masked_node_state, unprovable_node_state. lifecycle ABA in stateToken, cross-Ref token use, negative-range allocation, and incidence concurrency invalidate old binding.

All 27 Fields are exercised:

~~~text
calendar/participant
library/creator
library/venue
library/version-of
organizations/allied-with
organizations/brand-of
organizations/business-guided-by
organizations/governs
organizations/jointly-led-by
organizations/member-of
organizations/owns
organizations/parent
organizations/related
organizations/subsidiary-of
organizations/supervised-by
organizations/territorially-administered-by
people/engagement
people/family-related
people/guardian
people/manager
people/mentor
people/parent
people/professional-relation
people/sibling
people/social-related
people/spouse
tasks/dependency
~~~

Every directed Field resolves its inverse contribution; symmetric Fields have no inverse code. Resource cross-owner relation rejects. Deleting graph/backlink I and rebuilding from author facts is byte-equivalent.

Typed copy/fork covers all 27 Fields, both fresh-NodeId sort orientations, canonical fresh-owner relocation, literals, provenance/locators, requiredness/cardinality/domain, empty incidence scopes, and unwritable existing endpoint negatives. Any relation fact that cannot be preserved legally fails the whole typed copy rather than being omitted.

### 9.6 domain fixtures

People fixtures include names, contacts, preset/custom accounts, preset birth/death/employment-start/graduation/marriage plus custom important dates, conflicting assertions, measurements, engagements, parent/spouse/sibling/family/social/professional, manager/mentor/guardian, missing provider, and same-name Nodes.

Spouse history accepts multiple independent assertions and covers non-overlap, overlap, absent validity, literal/NodeRef mixture. Updating one fact never changes unrelated raw source/provenance. Finite-cardinality counterexamples use conformance schema; maximum never becomes a product request field.

people/engagement NodeRef arm accepts any ordinary Node without requiring Organization Facet. Organization-specific roster/graph is enhancement only. Literal fallback has no inverse.

engagement rank is a closed object with required non-empty exact system and level. Missing rank is not inferred. Cover same level text under different systems, multiple organizations/roles, exact author text, missing/wrong/extra inferred members. D9 preserves unmappable external rank code with explicit mapping/loss.

Organizations fixtures cover semantic versus structural parent, governs/owns/allied-with/related, brand/subsidiary/supervised/member-of/business-guided-by/territorial/joint leadership, and prove that note never guesses relation kind.

Library fixtures cover venue Work->Journal/Organization, illegal Person target, missing provider, wrong Facet, target deletion, draft->published same Work, edition fresh Node+relation, DOI non-identity, creator/venue/Citation/Resource separation.

### 9.7 Calendar/recurrence complete validation

Public projection and edit authorize first, then validate owner/source revision, Registry, Field/key, and temporal read binding.

Cover:
- date template duration in calendar days;
- instant template duration in exact elapsed seconds;
- civil recurrence rather than +86400 seconds;
- DST gap/fold and explicit late-fold anchor;
- 30-digit fractions;
- failure when an output endpoint cannot be represented;
- shared exact-time base enumeration for count/until;
- civil iteration order differing from actual instant order;
- missing timezone lookahead coverage;
- work/output budget exhaustion with no rows;
- horizon intersection;
- replacement moving into window from both directions;
- replacement moving out;
- RDATE/EXDATE/cancel/replace precedence;
- equal final range with distinct selectors;
- re-projection after explicit edit/rebase;
- byte-equivalent derived state after cache deletion;
- authorization change;
- Calendar UI disable without Core semantic change.

series-scope unique|many covers concurrent collision, path/index rejection, and Registry policy-digest binding. Duration covers closed/open date/instant and authored-duration rejection. Calendar packs allow multiple sources and never write one isHoliday fact.

### 9.8 People state, events, Task, and Account

Person state uses separate nationality-state, legal-sex-state, and gender-identity-state Fields. Each assertion preserves TypedText, validity, provenance and conflicts/overlap never overwrite. D7 projection uses explicit FieldId/period and unknown period is never assumed current/permanent/unique.

People events cover five preset codes plus custom label, required eventTime, date/instant/year/month/day precision, unknown/cross-domain code, empty/missing label, wrong branch, extra member. Equal labels/dates, conflicting birth/death, and multiple preferred assertions may coexist; missing contribution is not empty.

Task classification uses one source-bound rule: ordinary + explicit source-declared tasks/task. Effective-only Task rejects with no relations, at relation endpoints, Create/Assign/Remove, and Task/Calendar recurrence. Cover direct/multilevel/diamond requires, Task+Project, Template nonTask, Template Task, source-revision mismatch. Remove dependents before Task; one request still does not multi-remove Facets.

people/account preset/custom union remains. Custom arm requires exact non-empty serviceKey and identifier, with usage/customLabel outside and Entry.note separate. Unknown custom service does not become a global registry entry or match by display label.

Three general People relationship Fields remain symmetric-general while guardian/manager/mentor are directed. Custom title never changes Field semantics and D9 never silently converts historical directed professional data into symmetric semantics.

### 9.9 Workspace, diagnostics, and source materialization

D4 context binds the real host Workspace. Source-owner mismatch returns namespace_owner_unprovable before Entry parsing/Facet/effect work with zero parse. A legal cross-Workspace ref value/provenance is not rewritten merely because source owner is bound.

Recursive provenance diagnostics cover external/node/resource/transform and nested refs/locators. Missing member keeps exact pointer plus nearest-object span; present tokens use narrowest UTF-8 span; unknown keys use RFC6901 escaping; independent faults are all returned.

D4SourceMaterializationEffects/1 is independently recomputed from true source assembly: Field tail across same-namespace carriers, qualified carrier on absent Field, migrate-out/in before removing empty carrier, same-owner in-place edit, canonical order for multiple imports, and exact preservation of comments/CRLF/unknown namespaces/unselected raw bytes. Expected before/after is never derived from the effect under test.

New-path Create validates the same frozen SourceRevisionPlan.after.revision, which is 1 only under a proved empty H history in that production domain. Reject fake revision0, a result differing from the frozen plan, missing/duplicate initial Entry, and an existing owner claiming fresh origin. Existing source changes use that production domain’s checked H+1, not arbitrary before+1; no-op/source-unchanged retains any existing complete managed source version without a new managed after or SourceRevisionPlan; deletion produces no managed after. The fixed-S fresh=1/old+1 checks remain only for actual historical records under their original allocator.

### 9.10 Mapping fixed-S version obligations into D6-FA-r01

Where the historical document says "D3 v11/Result9", the **new decision path** now consumes D3 wire12 + D6 Control/2. Saved v9/v10/v11 decisions still replay under original decoder/bytes/gates. All inner relation/Calendar/Entry/Facet wire versions remain unchanged merely because the outer binding changed.

Historical D6 source-revision/potentialChanges/current-observation tests map to `SourceVersion/2`, `CommitDomain/2`, `Frontier/2`, `InputDescriptor/2`, `Policy/3`, and D6 safe-install/P-seal/publication. No semantic test obligation is deleted; only its outer binding is replaced. `SourceVersion/2` keeps the production version and production domain semantics, while current operation qualification is provided by `SourceObservation/1`, including observerDomain, entityRef, fileObjectBinding, evidencePins, control, Registry, incidence, and current cut checks. `Frontier/2` is a sealed causal/dependency cut and is not a complete Query or Registry proof. Inner D4 wire, legacy saved decoder versions, bytes, and gates remain unchanged. Until D7 complete Prepared exists, dependent entry points remain `owner_update_required`.

Historical D7 revision03/PreparedActionBinding remains legacy replay evidence only. Until the new D7 complete-cut/Prepared owner exists, affected strong D4 entry points return owner_update_required/unavailable. Historical test counts are never renumbered to claim the new path passed.

D6 `ObservationScope/2` and privacy controls still require unauthorized paired hidden states to produce the same `not_visible` result with zero business read or decision; once authorized, all real constraints are fully checked. Current consumption composes with D6 control, pins, and the current observation cut rather than treating H2 as the active authority/control producer. Registry, Policy, Calendar-scope binding, and initial bootstrap remain combined with the current authority/control contract without changing D4 typed semantic results. I is not a privacy proof or an empty proof, and masked/unprovable states retain their existing boundaries.

### 9.11 acceptance boundary

All of these are future conformance obligations for the current candidate. Pure Python/model results, historical fixture counts, documentation CI, and author self-review do not establish product host, D6 physical transaction, new D7 Prepared, D8 UI, or D9/D10 implementation success.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings.

## 10. P2 current-producer verification and complete scenario mapping

The main text §§2–17 is the precise contract under test; its §14.1 and this file §9 together retain all mandatory categories and all original fifteen current-path cases. Verification must compare real complete source, independently reconstructed effects, exact diagnostic sequence/readSet/writeSet and original owner outcomes. A model mirroring its own request/effect is not independent evidence. The fixed 57 A2 rows, 125-row matrix and 302 propositions remain traceable evidence obligations, including all D5/D7/D8/D9/D10 consumer gates and the seven-plugin intake; unavailable external scenario artifacts remain declared gaps. Each applicable executable-model, contract-check, integrity-check or contract-review retains the required independent semantic review. Historical fixture counts and artifact hashes do not prove new consumer success.

Additional required cases and component changes:

1. Registry loader implements the common dotted-namespace/Field/Facet/code decoder; full immutable RegistrySnapshot/Binding/Evolution and ValidatedCatalogContext; all seven contribution identities, with same-snapshot verified-owner policyId, nonempty policyVersion and policySchemaDigest computed from the full closed CalendarSeriesScopePolicy/1 using D3-CJ/3 then SHA-256. Test substituted same-generation snapshots, policy subset digests, owner mismatch, semantic drift, tombstones and no resurrection.
2. Source adapters separate production CommitDomain/epoch from observerDomain/current epoch, accept a foreign production domain under a valid current Observation, and never substitute externalSequence for inner revision. Test H across production epochs, foreign-domain return, proven empty history versus missing history, MAX, true no-op, unchanged-source portable structure, deletion absent-after, and equal-byte external admission as a real managed version.
3. Every managed-after plan freezes its real before/lastIssued/after/afterPin plus one proposed RevisionTokenBinding/2 before C/Q materialization. Retry/restart of the same plan preserves that exact token; only the winning planning CAS and original P seal make it the resulting managed SourceVersion's canonical stable address and sealed-outbox association. Loser/aborted/unproved tokens never become canonical through an equal stamp or reused numeric H+1. A watcher gap/new observer generation invalidates the old SourceObservation/runtime selector, not the stable managed address itself; a later new read still requires exact production-version equality to a fresh current Observation. Proposed stamp/token remains plan-local before seal. Q two-pass saved-definition materialization and D4 C/provenance transformations share one final source, revision, token, pin and seal; no second identity/revision/token sample is permitted.
4. Implement and independently exercise all fourteen DependencyKey kinds and nine StructureRange variants under their real owners, while each operation uses only its applicable keys. D4 proves complete positive/negative/empty incidence, Calendar binding/series/period/scope-inbound, full Registry and finite temporal-rule coverage. Current disclosure precedes enumeration. Distinguish I-cache deletion from real correctness-evidence loss; test stable intact epochs, new epochs after gaps, independent absent/configuration/empty stamps, hidden paired worlds and incomplete/unavailable without empty success.
5. scope_dependencies requires the full continuous sealed extension chain, unchanged bound sources/control/auth/Registry/rules/ranges and retained unrelatedness proof; vector growth alone fails. Original request/base Frontier/proof/pins/targets/writeProtection/version basis remain frozen. Written components compare with planned after, unwritten dependencies with original before/cut. Test legitimate unrelated advance and every real bound change.
6. New portable publication consumes ContentCompletionProof/3 production before/after, real seal ChangeId and exact Notice component set; receiver builds its own Observation. Test absent deletion, source-unchanged empty sourceChanges, external admission, restored-without-success, historical /1,/2 replay, ConflictRecord/2 versus /1 and unchanged ConflictId, and publication failure after seal without installation/charge/H repetition. Notice baseFrontier may contain older sealed heads but no new ChangeId for this unsealed decision.
7. Verify all weak eligibility conditions and U4 timing: human choice before planning, freeze from planning start, no strict-failure downgrade. Final-check unseen C may be lost, while actual B/N survives; observed competition/gap/stale Base/revocation is outside the weak relaxation. Prepare only is retained, not Saved; unknown install keeps pins/recovery_unknown. Strict/`observed_only` and ordinary/complete remain independent; D3 structure/Trash, D5 structured cells/rows/columns/reorder, strong D4, bulk/collection/promotion, D7 Action/Automation, server checkpoint, Approval and Money stay strict. Local invalidity/cardinality/requiredness/deny/unavailable never becomes pending.
8. Verify common original-profile disclosure/domain/fence/trust/P-custody and actual-version request/fingerprint/protocolOwner lookup before saved/planned/unseen. Saved returns original authorized receipt/error/effects or resumes original outbox without old-current/TTL/new-consumer gates. Planned restores original candidate map/version plan/pins/reservations/Notice/budget/attempt/TTL-clock/install state and only continues that plan. Unknown retains original pins, Approval/Money/claim/outbox/stop and no-duplicate-effect responsibility; files/hash/I/empty DB never guess outcome or permit retry/refund/reset. A native D3 unseen request follows its actual typed request/owner stages; D6 planToken/PreparedIntent qualification is required only by the D6 prepared-submit path or a real applicable preparation contract, not added to D3 wire12.
9. Complete all positive catalog rules in main §10 and exact TypeSpec rules in §4: alias optional-label semantics; People names/account preset/custom distinctions, conflicting events/states and engagement rank; all Organization relations/scalars and Library work/venue/version boundaries; Calendar selector defaults, exact-time sorting/coverage, rebasing/exception/horizon/budget and policy proof; aggregate independent diagnostics after availability, one invalid_entry_json for one or more missing envelope members, nested missing-member pointer/span, and D4 rollback that preserves independent D3 reservation/burn history.

No listed fixture is asserted to have run. Product filesystem/install proof, adapter behavior, real desktop/service tests, performance, new D7 complete consumer, D10 upstream acceptance and fresh independent complete joint review remain separate evidence gates. A2 is already human-authorized after those gates, followed by a separate fresh ordinary Chat Pro global final review and freeze/start package; this document does not begin product implementation or close the eleven OPEN findings/U6/U7.

The six specific boundary regressions also require future conformance evidence: until=anchor and until exactly equal to a later base start both include that occurrence; monthly byWeekday=monday yields every matching Monday in the selected month, including multiple matches. A mixed fresh/changed operation with an unchanged foreign-production C owner keeps that owner's full version, bytes and inner revision, lists it in effects, and creates no source plan for it; using another owner's plan fails. Cleanup accepts only the two-key selector and binds the separate three-key pre-state Entry reference, rejecting an added rawEntrySource selector member. measurement_unit_dimension is the sole accepted kind; a renamed alias fails and height+kg remains invalid. Instrument preflight so an unavailable namespace's malformed raw Entry causes zero inner parses, and paired worlds differing only in hidden Calendar membership cause zero hidden business reads before the authorized scope gate. Protection choice is frozen before planning and cannot downgrade after failure. These are required cases, not asserted test results.

## 11. Retained source-preservation and verification discipline

Effective revision35–37 obligations remain concrete future checks. Relation-update and recurrence-edit outcomes retain detached pre-state without encoding or JSON round-trips; ordered replacement uses the same lossless copy. Unrelated unavailable raw strings stay opaque and no whole-state/operation byte cap is invented. Copy-effect verification first establishes exact ordered inventory/identity, then compares raw before/after with independently admitted expected source, then compares typed JSON structure. Boolean/integer and array order are exact; dictionary order matters for state preservation, not semantic object equality. Cached decoded Entries use the shared typed comparator without first encoding unchecked cache values.

Instrument actual parser/span/materializer and source-bearing encoder/state-round-trip decoder calls, testing small valid controls, early masked failures, byte boundaries, repeated aliases, independent canonical costs, non-bootstrap complete successor definitions, claim mismatch, detached results and exact raw/member order. Ordered transformations precede common post-state validation. Cover all 24 many-valued relation Fields and the 76 historical ordering/orientation/relocation cases, 196 public Create/Assign constraint combinations and 21 constructor-location/rejection cases as retained obligations, not a claim that those historical runs certify this version. Present wrong kind uses its token; missing kind uses nearest container; non-object points to itself. Independent siblings continue and unavailable inner constructors remain opaque.

Complete version-bridge coverage also retains row/checklist promotion, Resource creation with an existing occurrence, existing Annotation reply to a fresh same-owner reply, pure existing upsert owned by D6, mixed D3 batches, fresh Task SCC, complete Template, C carrier materialization and S fresh reply, every typed provenance/Locator root, same-source revision and actual authorization/custody. Actual D3 receipt arrays and current mode decoder remain D3-owned. Exact expected source cannot be derived from the effect under test. Cover safe failures/revocation/ABA/collision/crash/replay/watermark and public operation entry points, with source-owner mismatch before parsing and zero new writes.

Historical reproducibility obligations retain the operation-world source corpus, independent expected case IDs/dispositions/closure/testScope and detached case builders; missing generated outputs must rebuild byte-equivalently. normal, -O and -OO validator outputs must match. The isolated historical d4_relation_state.py and d4_relation_test_support.py are required where that retained corpus is used; merely naming them does not establish their presence in this repository or current execution. No new wrapper/proposition hash substitutes for complete source comparison or independent semantic review. Current source, protected user checkout and unrelated assets remain outside this authoring scope. D6 bootstrap, scope migration/configuration deletion/control inbound and D7 narrow-field/DefinitionTransfer/effects require their actual owner qualification and complete future tests; historical H2 profiles are not current authority by name alone.
