---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: ebd24e10-6020-41e0-8a08-f494e46c21ec.

# D4 Implementation Impact and Test Outline — D6-FA-r01

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. All fixed-S revision37 preservation, revision36 admission, and revision35 integration obligations remain. This file adds D6-FA-r01 SourceVersion/2, SemanticState, and local-versus-complete consumer verification without treating historical runs as new-version success.

## 1. implementation effect graph

~~~text
D2 exact source
  -> D4 carrier/Entry decode
  -> RegistryBinding
  -> TypeSpec / qualifiers / provenance
  -> declared/effective Facet closure
  -> relation / Calendar / catalog validation
  -> D4 proposed semantic state
       -> D6 InputDescriptor/2
       -> SourceVersion/2 (production version)
       -> SourceObservation/1 + SourceVersionRef/1 (current observation qualification)
       -> ObservationScope/2 + DependencyProof/2 + Frontier/2 (current proof cut)
       -> D6 ordinary or complete qualification
       -> strict or A§4.1-qualified observed_only protection choice
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
| D6 adapter | InputDescriptor/2 + SourceVersion/2 + SourceObservation/1 outer binding | bare revision, hash, or production version across observation cut |
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

Future acceptance coverage includes both ordinary strict and the approved A§4.1 human weak positive cases. Ordinary strict may succeed only with complete source, authorization, current observation, and installation qualification. A trusted `interactive_source_save` may validate `observed_only` only for one existing live Document, ordinary replica-local scope, complete source read/replace qualification, no applicable body/Field/Node-control deny, author write set empty or limited to that Document, no identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation, and DraftBase equal to the selected current `SourceObservation`.

Weak protection is invalid for non-trusted, non-interactive, new or multi-Document, non-ordinary, non-replica-local, incomplete source read/replace, applicable body/Field/Node-control deny, multi-entity write set, identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation, or DraftBase mismatch. Structured bulk, collection, promotion, automation, server checkpoint, Approval, Money, and any strong Action never use weak protection.

A human may explicitly select the weak profile during planning even when strict capability is unavailable in the ordinary directory; after `writeProtection` is frozen, strict capability failure, known conflict, authorization failure, durability failure, or strong obligation failure never falls back to weak. D4 local types, D2 validity, unavailable byte equality, and actual source read/write checks remain required. `semantic_pending` represents only real unproved cross-object or complete-range obligations and never converts invalid, unavailable, or incomplete strong proof into success.

`observed_only` durably retains observed before B and user input N. An unobserved external C may exist, and installing N may overwrite C bytes in the current file; a later C write may also replace current file state after N, while durable B/N retention is preserved. Prepare-only is not Saved; unknown install remains `recovery_unknown`; observed competition, stale Base, watcher gap, and continuity gap require conflict/reprepare.

Until a new D7 Prepared contract exists, an entry point that requires D7 complete preparation is unavailable/owner_update_required. D4 defines no private preparation token.

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

SourceVersion/2 outer binding keeps the inner sourceRevision relationship unchanged. SourceVersion/2 production `commitDomain` may differ from the current operation observation domain; a complete current SourceObservation/1 is a valid positive case and is not rejected only because production and observation domains differ. The current observation requires observerDomain equal to the operation CommitDomain, entityRef equal to sourceVersion.entityRef, and current fileObjectBinding, evidencePins, control, Registry, and incidence dependencies in the same observation cut.

Negative cases cover observerDomain mismatch, entityRef mismatch, missing or stale fileObjectBinding/evidencePins/control/Registry/incidence dependencies, invalid SourceObservation token/epoch/continuity, and SourceVersion production epoch, revision, or externalSequence changes that invalidate the old inner selection. Equal production versions do not restore validity after watcher gap, external replacement, or discontinuous materialization; bare revision, hash, I cache, or key/value cannot recover the protected observation token.

SourceVersionRef/1 uses `sourceToken` tagged `d6_source_observation/1` to select the complete protected Observation. Frontier/2 is only the current sealed causal/dependency cut and is not a complete Query, payload, or Registry proof. Masked/unprovable RelationReadContext/2 states never produce successful binding/readSet, expected binding remains exact per member, and authorization/privacy gates remain unchanged.

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

text<->NodeRef and NodeRef retarget bind complete before/after owner revisions, canonical-owner relocation, and stable occurrenceKey. Any CAS/auth/collision/cardinality/lifecycle/projection failure byte-exactly rolls back source, revisions, projections, and allocations with empty write set.

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

Fresh Create validates result revision=1. Fake revision0 pre-state, result2, missing/duplicate initial Entry, and existing owner claiming fresh origin reject. Normal existing mutation still increments once.

### 9.10 Mapping fixed-S version obligations into D6-FA-r01

Where the historical document says "D3 v11/Result9", the **new decision path** now consumes D3 wire12 + D6 Control/2. Saved v9/v10/v11 decisions still replay under original decoder/bytes/gates. All inner relation/Calendar/Entry/Facet wire versions remain unchanged merely because the outer binding changed.

Historical D6 source-revision/potentialChanges/current-observation tests map to `SourceVersion/2`, `CommitDomain/2`, `Frontier/2`, `InputDescriptor/2`, `Policy/3`, and D6 safe-install/P-seal/publication. No semantic test obligation is deleted; only its outer binding is replaced. `SourceVersion/2` keeps the production version and production domain semantics, while current operation qualification is provided by `SourceObservation/1`, including observerDomain, entityRef, fileObjectBinding, evidencePins, control, Registry, incidence, and current cut checks. `Frontier/2` is a sealed causal/dependency cut and is not a complete Query or Registry proof. Inner D4 wire, legacy saved decoder versions, bytes, and gates remain unchanged. Until D7 complete Prepared exists, dependent entry points remain `owner_update_required`.

Historical D7 revision03/PreparedActionBinding remains legacy replay evidence only. Until the new D7 complete-cut/Prepared owner exists, affected strong D4 entry points return owner_update_required/unavailable. Historical test counts are never renumbered to claim the new path passed.

D6 `ObservationScope/2` and privacy controls still require unauthorized paired hidden states to produce the same `not_visible` result with zero business read or decision; once authorized, all real constraints are fully checked. Current consumption composes with D6 control, pins, and the current observation cut rather than treating H2 as the active authority/control producer. Registry, Policy, Calendar-scope binding, and initial bootstrap remain combined with the current authority/control contract without changing D4 typed semantic results. I is not a privacy proof or an empty proof, and masked/unprovable states retain their existing boundaries.

### 9.11 acceptance boundary

All of these are future conformance obligations for the current candidate. Pure Python/model results, historical fixture counts, documentation CI, and author self-review do not establish product host, D6 physical transaction, new D7 Prepared, D8 UI, or D9/D10 implementation success.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings.
