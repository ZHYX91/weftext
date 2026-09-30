---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 6e66067a-08b5-4c21-9791-45aaa6a50323.

# D4 Attribute Types, Schema, and Relations

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. Fixed-S D4 revision37-preservation-01 is source and compatibility history only. This is the complete D4 candidate owner afterimage; activation requires coordinated acceptance with D1/D3/D5/D6 and later D7/D8/D9/D10 consumers. It authorizes no product implementation, A2, release, or catalog change.

Fixed provenance:
- S=7e18168dad3e6d120fce0dd607dc10fa7894e252
- source blob=2c03f2050dc523ccd8ffd3696f89f91a3b9c3c07
- unchanged catalog blob=ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb

## 1. Scope and sole owner

D4 solely owns Semantic Namespace, Field/FieldId, Entry/1, TypeSpec/Typed Value, Field Semantic Shape, qualifiers/provenance, FacetSchema/effective closure, relation direction/inverse/canonical owner/cardinality/delete policy, Calendar typed values/comparators/recurrence/series-scope, RegistrySnapshot/RegistryBinding/schema evolution, and D4 semantic validation/diagnostics/effect extensions.

D4 does not own Document bytes, Node/Resource/Annotation identity, parent/order, file installation, SQLite, Query complete cut, Editor Draft, ImportJob, Automation, Extension runtime, or credentials. Current bytes, portable metadata, P/I, SourceVersion/2, CommitDomain/2, Frontier/1, SemanticState/1, and ConflictRecord consume the H2 D6 owner. Identity/lifecycle consumes H2 D3 wire12.

P/I caches, Registry cache, relation index, Calendar/Graph projection, and UI property models never become a second author truth.

## 2. Semantic Namespace and Registry

### 2.1 reserved owner tuples

Reserved tuples remain exact:

| namespace | owner kind | owner identity |
|---|---|---|
| core | core | weftext.core |
| d4 | core | weftext.d4 |
| tasks | core | weftext.tasks |
| people | first_party | weftext.people |
| organizations | first_party | weftext.organizations |
| calendar | first_party | weftext.calendar |
| library | first_party | weftext.library |
| user | workspace_user | current WorkspaceId |
| wf | unallocatable | none |

`wf` is never allocatable and `user` binds the current WorkspaceId. Copying a workspace or registering a replica creates no new namespace owner and never turns a first-party namespace into a user namespace.

### 2.2 RegistrySnapshot / RegistryBinding

RegistrySnapshot is D4 semantic truth for portable shared configuration; D6 owns physical storage, versioning, and permission. A trusted snapshot fully validates namespace ownership, closed Field/Facet/QualifierSet/alias shape, contribution tables, alias closure, requires/conflicts graph, active definitions, tombstone/migration ledger, semantic digests, and catalog limits.

RegistryBinding continues to bind exact generation plus snapshot digest; generation name alone is insufficient. One decode/operation uses one fixed snapshot and never refreshes mid-operation.

Registry evolution remains monotonic: the digest of a retained major-1 definition under the same ID is byte-equal; breaking changes use fresh IDs plus exact tombstones and migrations. Copying portable Registry bytes copies shared configuration facts but no right to advance Registry generation. Registry mutation still requires current Policy, Registry admin, and applicable D6 control/execution responsibility.

### 2.3 Reference Catalog v1

The fixed catalog continues to define 4 QualifierSetSpec, 22 aliases, 61 FieldDefinition, 7 FacetSchema, and 1 CalendarSeriesScopePolicy byte-for-byte. This batch creates no catalog replacement.

Limits remain:

| limit | value |
|---|---:|
| maximumTypeDepth | 8 |
| maximumObjectMembers | 64 |
| maximumUnionVariants | 8 |
| maximumCollectionItems | 256 |
| maximumEntryUtf8Bytes | 65528 |
| maximumSchemaUtf8Bytes | 65536 |
| maximumProvenanceAtoms | 16 |

Any structural/semantic catalog failure makes the whole catalog unavailable; there is no partial-success loader.

## 3. Entry/1 and author source

A D4 Field Value Occurrence always comes from the owning Node's D2 exact-source Document. Attribute Carrier Block/Lexical Attribute Entry are current-revision lexical occurrences, not Entities, Records, sidecars, or database rows.

D4 Entry/1 retains closed members: version, FieldId, occurrenceKey, typed value, optional qualifiers, optional note, optional provenance. Unknown/duplicate members, wrong constructors, invalid UTF-8/JSON, budget overflow, or Field/schema mismatch fail in the original diagnostic order.

occurrenceKey is only a value-internal selector scoped to owner Node + FieldId + expected current source revision. It is not EntityRef, Locator, OperationId, RecordRef, or cross-revision identity. Equal values may have different occurrences; external edit/reorder/delete+reinsert or SourceVersion observationEpoch change cannot preserve selection by key/value alone.

Inline Field Note is optional plain author text; absent note writes no empty placeholder. Typed qualifier/provenance is not hidden in note text.

Raw Entry, carrier framing, comments, line endings, unknown namespace bytes, and unselected occurrences follow D2/D4 lossless rules. A structured mutation binds complete before source, selector/span, current SourceVersion/2, and RegistryBinding. Equal hash alone proves neither continuity nor absence of ABA.

## 4. TypeSpec, Typed Value, and Field Shape

The fixed-S constructor set remains: text, Boolean, integer, decimal, semantic_code, calendar_date, zoned_instant, date_range, instant_range, quantity, node_ref, resource_ref, annotation_ref, external_identifier, closed object, closed union, bounded ordered collection, and alias_ref.

D4 integer is a canonical signed decimal string and differs from D3Integer. decimal rejects exponent, NaN, Infinity, negative zero, and redundant fractional zero. ResourceRef/AnnotationRef owner equals the containing owner NodeRef; D4 never fills ambient owner.

calendar_date uses only the verified calendar/version/precision comparator. Built-in `calendar/iso8601` v1 still validates real Gregorian dates and years 0001..9999. zoned_instant keeps exact RFC3339 plus IANA timeZone/tzdbVersion rather than device defaults. date_range/instant_range remain end-exclusive, at least one endpoint bounded, and strictly start < endExclusive when both are bounded.

quantity preserves exact decimal + namespaced unitId and performs no conversion without a verified conversion contribution.

Field Semantic Shape remains closed to fact, event_assertion, observation, relation. Shape owns legal qualifiers/projection; event/state/observation/relation semantics are not flattened into an untyped row list.

## 5. FacetSchema

D2 source `facetMemberships` remains the sole author source for declared membership. D4 interprets it as an unordered exact FacetId set; source order gives no precedence/override/last-wins. Effective closure is mechanically derived from the current Registry requires graph, rebuildable, and never written back.

Missing dependency, cycle, conflict, or incompatible Field definition makes affected typed state unavailable/invalid and blocks touching operations. FacetSchema closes fields, relations, requires, conflicts, constraints with no implicit membership, priority, or shadow.

Current Task semantics remain ordinary Node + source-declared exact `tasks/task` Facet. Template remains the D2/Core meta-kind and Template + exact tasks/task is invalid. D4 does not restore specialization dual authority.

Assign/Remove/Cleanup Facet is a strong typed Action requiring complete source/Registry, closure, relevant requiredness, complete relation incidence, Policy/auth, and exact post-state. ordinary pending save does not downgrade that Action.

## 6. typed availability and D6 SemanticState

### 6.1 Namespace and Node typed state

Namespace typed state is closed:

- `available`: owner/schema/Entry syntax/type/constraints proved;
- `retained_unavailable`: owner/schema/contribution unavailable or incompatible, raw source retained and never projected empty;
- `invalid`: known schema but invalid Entry/type/cardinality/key/constraint/relation;
- `not_present`: exact source truly has no namespace.

Node typed snapshot is `complete|partial|unavailable`. partial carries per-namespace state and omission never means empty. D2 exact source is unchanged by D4 state.

### 6.2 Four orthogonal dimensions

| Dimension | owner | example states | cannot replace |
|---|---|---|---|
| current bytes | D6 | managed / external / external_invalid | D4 typed validity |
| local typed projection | D4 | namespace four-state; Node complete/partial/unavailable | workspace-wide constraint proof |
| save semantic completion | D6 | complete_semantics / semantic_pending | D7 complete result |
| Query/Action cut | D7 | future versioned complete/partial evidence | D4 local validity |

Unknown/pending/unavailable never becomes empty value, empty relation, zero members, or whole-workspace no-match.

## 7. operation-applicable proof matrix

Every operation separately proves current source/SourceVersion/2; RegistryBinding/touched definitions; actual MutationFootprint; local Entry/Facet closure; required positive/negative cross-object scope; unique proposed post-state; Policy/permission; D6 CommitDomain/Frontier/install; and D7 complete cut only for a strong consumer that requires it. Pending never skips locally provable facts.

| operation | local proof | complete proof | result |
|---|---|---|---|
| exact source read/repair | D2 bytes + disclosure; typed may partial/unavailable | none | raw + explicit status |
| body-only edit | D2 parse; D4 carrier old/new byte-equal; independent Facet closure; disjoint write-set | unrelated namespace not globally scanned | complete or real pending; unavailable/invalid namespace unchanged |
| edit another available namespace | complete touched Entry/Facet; other unavailable/invalid raw byte-equal | untouched cross-object scope not read | same |
| edit affected unavailable/invalid namespace | complete dedicated repair/remove/cleanup | as required | ordinary typed edit rejects |
| D3 replica_local create_node | complete new-source D2+D4 local typed/Facet | relation/unique/calendar/cross-object may become obligation | semantic_pending save allowed; not strong D4 Action success |
| D3 replica_local move/reorder | D4 facts only where actual predicate/permission needs them | does not prove global relation/collection | D3 local success may coexist with pending |
| D3 replica_local Trash | local source/Facet/lifecycle policy | missing inbound/relation complete scope becomes obligation | semantic_pending local lifecycle |
| non-relation Entry edit | complete owner source, Field, Entry, cardinality, qualifier/provenance, requiredness | no global scope if Field has no cross-object obligation | ordinary save may succeed; other obligations remain pending |
| Assign/Remove/Cleanup Facet | complete local closure + relation incidence | complete required | missing range rejects/unavailable |
| relation add/update/delete | RelationReadContext/2 + Binding/2 | complete incidence/endpoint/domain/cardinality | complete only |
| local recurrence value edit | exact TypeSpec/calendar/tzdb/source post-state | local if no series uniqueness | ordinary or pending |
| series-scope unique/many | recurrence/period + policy | complete (series,periodKey,scope) positive/negative range | complete only |
| raw copy/export | exact source + status | no typed-success claim | byte-preserving + loss/status |
| typed copy/fork/import | D3 wire12 map + D4 typed source | complete canonical-owner/requiredness | managed_atomic only |
| restore/purge | D3 managed_atomic + current D4 facts | complete relation/inbound; purge also Frontier | missing proof rejects |
| D7 all_result/bulk/automation write | local typed is prerequisite only | future D7 complete cut | unavailable before D7 afterimage |

D6 obligation `relation|unique|calendar|inbound|cross_object_type` means unproved, not allowed-to-fail. It binds SourceVersion/2, CommitDomain/2, Frontier/cut, RegistryBinding, Policy, and pins.

Later r6 validation proves only r6 and never rewrites r5. Rebuilding I, equal hash, device migration, replica registration, external A->B->A, or observationEpoch change never restores old proof.

## 8. Relations

Relation kind/direction/inverse/cardinality/resolution/delete policy/graphProjection belongs to FieldDefinition, not an open RelationType registry. Directed relations author one source fact and derive inverse. Symmetric relations author exactly one fact at canonical owner selected by NodeRef canonical encodings; canonical owner never bypasses authorization.

The text arm of `node_ref_or_text` has no target lookup/inverse. text->NodeRef is explicit and atomic.

Fixed closed shapes remain:
- `RelationReadContext/2 = {kind:"d4_relation_read_context",wireVersion:2,registryBinding,nodeStates,entries,incidenceScopes}`;
- nodeStates union source_node_state / sourceless_node_state / masked_node_state / unprovable_node_state;
- `RelationReadBinding/2 = {kind:"d4_relation_read_binding",wireVersion:2,nodeRevisions,entityStates,incidenceRevisions}`.

D6 provides trusted snapshot/stateToken/source revision/incidence revision/auth. Client never self-reports complete or empty incidence.

New D6-FA requests keep the D4/2 inner wire. InputDescriptor/2 binds SourceVersion/2 for every source-bearing owner. Context owner bijects to sourceInputs; inner sourceRevision is the same managed source revision; SourceVersion.commitDomain matches the operation; observationEpoch/ChangeId remains current; entity/incidence versions and RegistryBinding belong to the same cut. Same inner revision with different SourceVersion observationEpoch/domain remains stale/unavailable.

Gate order remains D3/D6 disclosure/auth -> D4 closed context/binding -> coverage -> immutable cut -> Registry/raw Entry/selector/revision -> complete incidence -> proposed state -> domain/lifecycle/cardinality/requiredness -> D6 CAS -> author commit.

Masked/unprovable required endpoints cannot yield a successful readSet. New/retargeted NodeRef targets are live/domain-valid; deleting an old fact may inspect determinate tombstoned/not_found endpoints without inventing Documents. Symmetric endpoint purge requires explicit removal of all incident facts.

D4RelationCopyEffects/1 remains an independent D4 effect, never a D3 ref-slot arm. D3 wire12 typed copy/fork/import consumes it only with managed_atomic + complete D4 proof; raw preservation does not become typed fork success.

## 9. Calendar

CalendarPeriod, Temporal Range, and Calendar Event Semantics remain separate. period is calendar-defined scope, range is date/instant range, and event adds persistent status/recurrence/participant/reminder semantics. View/card/title patterns never auto-Assign Event.

`calendar/recurrence-value` keeps homogeneous date/instant variants, count/until exclusivity, and 256-item RDATE/EXDATE/exception limits. Derived occurrence is rebuildable and has no Node identity. Projection binds Registry/calendar/tzdb/rule contributions, current source, and explicit horizon/budget; incomplete coverage never returns partial rows labeled complete.

`calendar/series-scope` v1 keeps key=(series,periodKey,scope), scopeKinds=node|workspace, multiplicity=many|unique. A unique mutation requires complete positive/negative scope under current Registry/Policy/SourceVersion/Frontier. Building I/current page/provider cache/path collision never proves uniqueness.

ordinary save may persist local Calendar facts as semantic_pending(calendar) but does not complete the strong unique-period/event Action.

## 10. Positive catalog consumption

`people/labeled-text-value` remains text required exact non-empty, label optional semantic_code with contribution-set `people/other|people/personal|people/work`, and customLabel optional exact non-empty.

`people/phone`, people/email, people/address, people/website use that alias. Missing label means author supplied no label, not empty, unknown, people/other, or typed-unavailable. Localized display labels are never SemanticCodeId. Alias/Registry validates before object members.

People names/life-event/measurement/state/profession/engagement/relations remain independent Fields; people is a namespace, not a single blob. engagement Node arm accepts an ordinary Node and Organizations Facet is UI/query enhancement rather than NodeRef admission requirement.

Organizations structural parent is distinct from organizations/parent. Move does not edit organization relation and relation edit does not move. identifier/classification is data, not identity; relation directions/canonical owner remain.

`library/work` is Bibliographic Work, not project work/task. DOI/ISBN/provider IDs are not NodeRef and Citation occurrence is not Work identity.

tasks/task still requires tasks/status and tasks/dependency requires complete Task-domain proof at both endpoints.

## 11. diagnostics, permission, and source materialization

D4 diagnostic remains closed with stable sourceStart/rank/code ordering and no machine-significant free details map.

Order: D6/D3 visibility/permission -> Registry owner/binding/contribution -> D2 carrier/span -> strict JSON -> Field/Facet -> typed value/qualifier/provenance -> operation-applicable post-state. Registry unavailable never leaks inner schema by parsing first. external strict UTF-8/D2 invalid is D6 external_invalid repair, not D4 semantic_pending.

Field-level authorization proves possible write footprint before hidden reads; final materialization revalidates actual footprint. Unselected bytes/comments/CRLF/unknown namespaces remain exact.

Concurrent replica edits to different Fields can still be file conflicts; D6 ConflictRecord retains both bytes/SourceVersion. A D4 semantic merge proposal still passes file CAS.

Final commit eligibility remains D2 eligible AND operation-applicable D4 gate AND D6 gate AND applicable D7 gate.

## 12. I, P, partial index, and r5/r6

I caches only derived Registry parse, typed projection, incidence, Calendar projection, or search candidates. Rebuild creates no D4 decision.

Loss of P cannot reconstruct a strong Action/readSet/pending validation. If r5 was pending and r6 later validates, only r6/current gains that proof; r5 replay remains original.

A partial index may support explicitly marked exploratory partial/pending projection, never relation/unique/Calendar negative scope, D5 collection membership, D7 all_result/bulk, or Automation write.

## 13. Downstream and compatibility

D5 consumes real Field Value Occurrence/Document Table/Node Collection domains and never turns a D4 occurrence into Record identity.

D7 later owns complete cut/new Prepared/Search. D4 never invents PreparedActionBinding/3 or converts legacy /1,/2 into new success. Strong D4 Action needing D7 remains unavailable before that afterimage.

D8 display labels are not FieldIds; D9 does not infer Field/semantic code from localized labels; D10 runtime/credentials/execution responsibility is not transferred by portable Registry copy.

D4 Entry/1, TypeSpec, catalog v1, RelationReadContext/2, RelationReadBinding/2, RecurrenceReadContext/1, D4RelationCopyEffects/1, D4SourceMaterializationEffects/1 keep their original semantic shape. New D6-FA consumption adds outer InputDescriptor/2 SourceVersion/2/CommitDomain/Frontier binding, never rewriting old saved decisions.

## 14. Mandatory scenarios and acceptance

The fixed mandatory scenarios remain pressure obligations, not feature approval. Preserve People labeled entries, Organizations relation-vs-parent, Calendar period/range/event, ICS foreign identity, Library Work, unknown provider, multi-replica conflict, terminology, and no-second-authority boundaries.

Future tests include:
1. offline body edit succeeds with byte-equal unavailable namespace;
2. touching unavailable/invalid namespace rejects;
3. local Entry type/cardinality/requiredness failure rejects under ordinary profile;
4. local valid edit with missing relation/unique/calendar complete scope is pending only;
5. relation missing incidence negative scope rejects;
6. Calendar unique missing complete series scope rejects;
7. partial I supports partial exploration but not all_result/automation write;
8. external D2-invalid stays repairable without D4 success;
9. offline conflict is explicit, not LWW;
10. r5 replay is separate from r6 current;
11. I rebuild does not restore proof and P loss does not rebuild decision;
12. people/phone optional semantic-code label positive case;
13. one authored fact for symmetric canonical owner;
14. D3 wire12 copy/fork lacking complete D4 post-state rejects;
15. legacy saved bytes stay unchanged.

These are future acceptance obligations, not claims of implemented tests or performance.

## 15. Candidate acceptance boundary

This is an author candidate, not independent acceptance. The catalog remains the fixed-S blob and is not a replacement. The immutable candidate still requires fresh independent joint review and coordinated acceptance.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, P1=3, P2=8, eleven OPEN findings. This file closes or reclassifies none.

## 16. Normative exact-contract restoration

This section is normative and restores the fixed-S exact wire, budget, diagnostic, and positive acceptance contracts that must not be summarized away. If an earlier overview conflicts with an exact shape here, this exact shape wins. D6-FA-r01 changes outer storage/version/qualification binding only and does not change these D4 inner wire majors.

### 16.1 Entry/1 outer binding, closed envelope, and carrier

D2 carrier bytes remain unchanged and D4 interprets only rawEntrySource. Normative example:

~~~adoc
[weftext-attributes]
....
namespace people
entry {"v":1,"field":"name","occurrenceKey":"5e6d8b56-8f22-4b97-a6e1-1aacbf2f9a78","value":{"kind":"object","members":{"role":{"kind":"semantic_code","code":"ordinary"},"text":{"kind":"text","text":"张三"}}}}
....
~~~

The payload is one strict-JSON object. Duplicate members, trailing tokens, NaN/Infinity, JSON floating numbers, unknown envelope keys, missing required keys, and illegal null are rejected. Object-member source order has no semantic precedence.

Entry/1 exact envelope:

~~~text
required:
  v               := JSON integer 1
  field           := block-local field path
  occurrenceKey   := canonical lowercase RFC 4122 UUID text,
                     RFC-4122 variant, version 1..5
  value           := one closed TypedValue
optional:
  qualifiers      := closed object selected by Field semantic shape
  note            := non-empty Unicode plain text; omit when absent
  provenance      := 1..16 D3-compatible provenance atoms

no other members; null is never an absent-member encoding.
~~~

FieldId expands as namespaceToken + "/" + field. Entry never repeats namespace. Multiple blocks in one namespace form one semantic stream in author-source order; a block has no identity or precedence. Duplicate-key scope is exactly:

~~~text
(owner NodeRef, expanded FieldId, occurrenceKey)
~~~

A second same key under the same owner/Field rejects even when value bytes match. Different FieldIds may reuse key bytes. Validation spans all entries for that Field across all same-namespace blocks, never resetting at block boundaries and never using last-wins.

occurrenceKey only selects duplicate values for patch/reorder/note within current owner+Field+expected source revision. Mutation binds owner NodeRef, FieldId, occurrenceKey, and current SourceVersion/2. The key never enters D3 EntityRef/Locator/AnnotationTarget, cannot independently resolve/authorize/query across owners, and has no tombstone/restore. Reappearance after delete is a new source fact. Node move/rename may preserve Entry bytes; fresh-owner copy/import may preserve key bytes, while copying within the same owner+Field requires a fresh key. D3 identityMap never contains occurrenceKey.

A normal D2 header attribute never becomes a D4 Field merely by name. Explicit Map Attribute Action previews exact source range, target FieldId, conversion, fresh occurrence keys, loss, schema dependency, and original-source disposition, and commits exactly one chosen canonical target. Flat export/cache is never Weftext Document authority.

### 16.2 TypedValue/ValueTypeSpec, qualifiers, and provenance

Every authored TypedValue checks string kind before constructor decoding. Missing/non-string/unknown kind and wrong constructor are not successful values. Every object rejects unknown/missing/illegal null and nested values recurse through the same table.

~~~text
text:
  {kind:"text",text:<Unicode string>}
boolean:
  {kind:"boolean",value:<JSON boolean>}
integer | decimal:
  {kind,value:<canonical string>}
semantic_code:
  {kind:"semantic_code",code:<short code or SemanticCodeId per schema>}
calendar_date:
  {kind:"calendar_date",calendarId,calendarVersion,precision,lexeme}
zoned_instant:
  {kind:"zoned_instant",instant,timeZone,tzdbVersion}
date_range | instant_range:
  {kind,start,endExclusive}
  each bound := corresponding TypedValue | exact {kind:"unbounded"}
quantity:
  {kind:"quantity",decimal:<canonical decimal>,unitId:<SemanticCodeId>}
node_ref | resource_ref | annotation_ref:
  {kind,nodeRef|resourceRef|annotationRef:<D3 typed ref>}
external_identifier:
  {kind:"external_identifier",scheme:<SemanticCodeId>,value:<non-empty text>}
object:
  {kind:"object",members:<closed object>}
union:
  {kind:"union",variant:<declared token>,value:<variant TypedValue>}
bounded_set | bounded_sequence:
  {kind,items:[<TypedValue>...]}
  legal only where parent ObjectMemberSpec allows collection
~~~

Set items are sorted by unsigned byte-lexicographic D3-CJ/3 canonical UTF-8 bytes with no duplicates; sequence preserves author order. Floating JSON numbers and NaN/Infinity are rejected at every recursive depth.

ValueTypeSpec/1 is a closed tagged meta-wire:

~~~text
scalar:
  {kind} where kind ∈
  boolean|decimal|calendar_date|zoned_instant|date_range|instant_range|
  quantity|node_ref|resource_ref|annotation_ref|external_identifier

text:
  {kind:"text",normalization:"exact"|"nfc-for-compare"}
  or same plus nonEmpty:true

bounded integer/decimal:
  {kind,minimum:<canonical string>,maximum:<canonical string>}
integer may also add:
  excludedValues:[1..64 sorted unique canonical integer strings]

semantic_code:
  {kind:"semantic_code",codeScope:<CodeScopeSpec/1>}
CodeScopeSpec/1:
  {kind:"field_local",codes:[1..256 sorted unique short codes]}
  {kind:"namespace",namespaceId:<SemanticNamespaceId>}
  {kind:"contribution_set",codes:[1..256 sorted unique SemanticCodeIds]}

external_identifier:
  {kind:"external_identifier"}
  or
  {kind:"external_identifier",
   schemeScope:{kind:"contribution_set",
                schemes:[1..256 sorted unique SemanticCodeIds]}}

alias:
  {kind:"alias_ref",aliasId:<FieldId-shaped registry-local ID>}

object:
  {kind:"object",members:[ObjectMemberSpec/1...]}
ObjectMemberSpec/1:
  {name,required,valueType}
  name := lowerCamel ASCII; 1..64 sorted unique members

union:
  {kind:"union",variants:[UnionVariantSpec/1...]}
UnionVariantSpec/1:
  {variant,valueType}
  variant := lowercase kebab; 2..8 sorted unique variants

collection:
  {kind:"bounded_set"|"bounded_sequence",
   minimum:<0..256>,maximum:<1..256>,
   itemType:<ValueTypeSpec/1>}
  minimum <= maximum
  only inside ObjectMemberSpec.valueType
  never a Field root, alias root, or direct collection item
~~~

Recursive depth starts at 1 at the FieldDefinition root; every alias expansion, object member, union variant, or collection item adds 1, with maximum 8. Alias expanded root, fully expanded FieldDefinition, and FacetSchema canonical UTF-8 each cap at 65536 bytes; raw Entry JSON caps at 65528 bytes; object members 64, union variants 8, collection items 256, provenance atoms 16. Counting happens before large allocation/transformation and wrappers never shrink the expanded-root limit.

All schema meta-wire integers first pass D3Integer 0..2^63-1 with Boolean rejection and then narrower constraints. CardinalitySpec/1 is:

~~~text
{minimum:0|1,maximum:<positive D3Integer|"many">}
~~~

FieldDefinition.cardinality.minimum and directed RelationDefinition.targetCardinality.minimum are normatively 0 in v1. Business requiredness comes only from FacetConstraintSpec.

FieldConstraintSpec/1 is closed:

~~~text
{kind:"at_most_one_preferred"}
{kind:"mutually_exclusive_members",
 members:[2..64 sorted unique ObjectMember names]}
{kind:"measurement_unit_dimension",
 codeMember,quantityMember,byCode}
~~~

FacetConstraintSpec/1 is closed:

~~~text
{kind:"required_field",fieldId,when:"effective"}
{kind:"union_variant_equal",
 leftFieldId,rightFieldId,when:"both_present"}
~~~

union_variant_equal requires every validated occurrence on both sides to use one common variant; equal mixed variant sets are not enough. Create/Assign satisfies effective requiredness and branch equality. Deleting the last required occurrence while the Facet remains effective rejects; removing the Facet ends the requirement while retaining Fields as retained_without_membership.

Field shape to qualifier contract:

~~~text
fact:
  validity?: date_range|instant_range
  selection?: ordinary|preferred|deprecated

event_assertion:
  eventTime: calendar_date|zoned_instant   (required)
  confidence?: decimal in [0,1]
  selection?: ordinary|preferred|deprecated

observation:
  observedAt: zoned_instant                (required)
  confidence?: decimal in [0,1]
  selection?: ordinary|preferred|deprecated

relation:
  validity?: date_range|instant_range
  status?: semantic_code
~~~

QualifierSetSpec IDs are d4/fact-qualifiers-v1, d4/event-assertion-qualifiers-v1, d4/observation-qualifiers-v1, d4/relation-qualifiers-v1. Qualifier objects are closed; missing required, unknown, null, or wrong type rejects. Relation status uses a Registry-backed full SemanticCodeId and has no separate Field-local codeScope.

Provenance atom set is closed:

~~~text
{kind:"node",nodeRef,locator?}
{kind:"resource",resourceRef,regionLocator?}
{kind:"external",scheme,value,observedAt?}
{kind:"transform",inputIndex,operationId}
~~~

Node/resource locators must owner-bind byte-equal to their refs. transform inputIndex is a zero-based D3Integer referencing an earlier atom in the same array; operationId is canonical lowercase RFC4122 UUID. Credentials, provider account, sync token, etag, cursor, SourceBinding, ForeignIdentityKey, and OriginBinding never enter portable Entry provenance.

### 16.3 FieldDefinition/1, FacetSchema/1, and FacetOperationRequest/2

FieldDefinition/1 exact example:

~~~json
{
  "wireVersion":1,
  "kind":"field_definition",
  "fieldId":"people/name",
  "semanticMajor":1,
  "valueType":{"kind":"alias_ref","aliasId":"people/name-value"},
  "shape":"fact",
  "qualifierSetId":"d4/fact-qualifiers-v1",
  "cardinality":{"minimum":0,"maximum":"many"},
  "occurrenceOrder":"author_order",
  "duplicatePolicy":"key_unique_values_may_repeat",
  "constraints":[]
}
~~~

A non-relation Field has exactly those eleven keys. A relation Field additionally requires relation and shape=relation. v1 duplicatePolicy accepts only key_unique_values_may_repeat. FieldDefinition decodes independently of Facet membership; after Facet removal a known Field fact is retained_without_membership.

FacetSchema/1 exact:

~~~json
{
  "wireVersion":1,
  "kind":"facet_schema",
  "facetId":"people/person",
  "semanticMajor":1,
  "requires":[],
  "conflicts":[],
  "fields":["people/name"],
  "relations":["people/parent","people/sibling","people/spouse"],
  "constraints":[]
}
~~~

FacetId + semanticMajor=1 binds the complete semantic digest; a later Registry revision under the same ID cannot change requires/conflicts/fields/relations/constraints. Breaking change uses a fresh FacetId and explicit migration. Arrays are D3-CJ/3-byte sorted and unique, fields/relations disjoint, requires acyclic, conflicts symmetric and non-self. The catalog supports at most 1024 Facets while D2 declared set remains capped at 32.

Task classification comes from D2 coreKind=ordinary plus source-declared exact tasks/task in the same source revision. Effective-only Task is insufficient and Template + explicit tasks/task remains invalid. source_node_state coreKind/declared/effective is proven from the same source/Registry rather than caller isTask.

FacetOperationRequest/2 exact:

~~~text
{kind:"d4_facet_operation_request",
 wireVersion:2,
 operationId,
 operationKind,
 expectedOwnerRevision,
 expectedRegistryBinding,
 expectedRelationReadBinding,
 expectedRecurrenceReadBinding,
 declaredFacetIds,
 targetFacetId,
 initialEntries,
 cleanupSelectors}
~~~

operationKind is closed:
create_with_facets | assign_facet | remove_facet | cleanup_facet_fields

Applicability:

~~~text
create_with_facets:
  declaredFacetIds=0..32 unique D2 FacetIds
  targetFacetId=null
  initialEntries=explicit Entry references
  cleanupSelectors=[]

assign_facet:
  declaredFacetIds=[]
  targetFacetId=one FacetId
  initialEntries=explicit Entry references
  cleanupSelectors=[]

remove_facet:
  declaredFacetIds=[]
  targetFacetId=one FacetId
  initialEntries=[]
  cleanupSelectors=[]

cleanup_facet_fields:
  declaredFacetIds=[]
  targetFacetId=null
  initialEntries=[]
  cleanupSelectors=unique {fieldId,occurrenceKey}
~~~

Inapplicable members are exact null/[] rather than omitted or populated. pre-state:

~~~text
{nodeRef,coreKind,declaredFacetIds,entries,
 ownerRevision,registryBinding}
~~~

outcome:

~~~text
{kind:"d4_facet_operation_outcome",
 wireVersion:2,
 operationId,
 status,
 postState,
 writeSetOwners,
 readSet,
 recurrenceReadSet,
 rollbackByteEqual,
 intermediateStateObservable:false}
~~~

A successful existing-source change increments owner revision once. Trusted D3 fresh create validates the complete result at revision=1 and never increments it to 2. Remove changes declared membership only. Cleanup removes only explicitly selected and proved-unused occurrences. Failure returns byte-exact pre-state, empty write set, null successful read sets, and no observable intermediate state.

### 16.4 relation exact contracts

RelationDefinition/1 has exactly two member sets:

~~~text
directed:
{kind:"relation_definition",
 targetMember,targetDomain,
 direction:"directed",
 inverseCode,
 sourceCardinality,targetCardinality,
 subjectPredicate,targetPredicate,
 resolutionPolicy,deletePolicy,graphProjection}

symmetric:
{kind:"relation_definition",
 targetMember,targetDomain,
 direction:"symmetric",
 selfEdgePolicy,
 subjectPredicate,targetPredicate,
 resolutionPolicy,deletePolicy,graphProjection,
 endpointPurgePolicy}
# symmetric MUST omit sourceCardinality,targetCardinality
~~~

EndpointPredicate is closed:

~~~text
{kind:"ordinary_node"}
{kind:"facet_any",facetIds:[1..8 sorted unique FacetIds]}
~~~

targetDomain is only node_ref|node_ref_or_text and exactly matches the expanded targetMember type. The text arm has no resolution/inverse/graph/target-cardinality/lifecycle semantics; a node_ref-only relation rejects text. Directed inverseCode resolves in the same Registry semantic-code contribution. Symmetric selfEdgePolicy is accept|reject and endpointPurgePolicy is explicit_remove_before_either_endpoint_purge. resolutionPolicy is live_required_on_create_retain_suspended and deletePolicy is retain_fact_explicit_cleanup.

A symmetric NodeRef fact is authored once: the endpoint whose nodeId has smaller unsigned byte-lexicographic D3-CJ/3 canonical UTF-8 bytes is canonical owner; a self-edge follows policy. A canonical-owner change is one atomic remove-old/add-new preserving occurrenceKey and all other Entry members/raw ordering. Failure is byte-exact rollback.

RelationReadContext/2 exact:

~~~text
{kind:"d4_relation_read_context",
 wireVersion:2,
 registryBinding,
 nodeStates,
 entries,
 incidenceScopes}
~~~

nodeStates closed union:

~~~text
source_node_state:
  {kind,nodeRef,lifecycle,stateToken,sourceRevision,
   coreKind,declaredFacetIds,effectiveFacetIds}
  lifecycle := live|trashed

sourceless_node_state:
  {kind,nodeRef,lifecycle,stateToken}
  lifecycle := tombstoned|not_found

masked_node_state:
  {kind,nodeRef}

unprovable_node_state:
  {kind,nodeRef}
~~~

entries item:

~~~text
{ownerNodeRef,fieldId,entry,rawEntrySource}
~~~

incidenceScopes item:

~~~text
{fieldId,endpointNodeRef,revisionToken,factSelectors}
factSelector := {ownerNodeRef,fieldId,occurrenceKey}
~~~

RelationReadBinding/2 exact:

~~~text
{kind:"d4_relation_read_binding",
 wireVersion:2,
 nodeRevisions,
 entityStates,
 incidenceRevisions}

nodeRevisions item:
  {nodeRef,sourceRevision}

entityStates item:
  {nodeRef,lifecycle,stateToken}

incidenceRevisions item:
  {fieldId,endpointNodeRef,revisionToken}
~~~

masked/unprovable never produces a successful binding/readSet. Expected binding equals the actual trusted binding exactly. D6-FA-r01 additionally uses outer InputDescriptor/2 to bind SourceVersion/2 for every source-bearing owner; inner sourceRevision is the same managed source revision and CommitDomain/observationEpoch/ChangeId/current control versions share one cut.

Relation operation model remains:

~~~text
operation:
{wireVersion:2,fieldId,subjectNodeRef,
 fromOwnerNodeRef,toOwnerNodeRef,
 occurrenceKey,nextTarget,
 authorizedOwners,
 expectedSourceRevisions,
 expectedRegistryBinding,
 expectedRelationReadBinding,
 injectFailureAt}

pre-state:
{wireVersion:2,
 entries,sourceRevisions,sourceRevisionOwners,nodeStates,
 projections,allocations}

outcome:
{wireVersion:2,status,postState,writeSetOwners,
 readSet,rollbackByteEqual,
 intermediateStateObservable:false}
~~~

injectFailureAt is conformance-only, never product self-authorization. sourceRevisionOwners maps source-bearing inventory only; sourceRevisions and expectedSourceRevisions have identical key sets and exact D3Integer values.

The common relation gate order is outer disclosure/auth -> closed context/request/binding -> source/sourceless coverage -> immutable cut/binding equality -> Registry/raw Entry/selector/revision -> complete old/new incidence -> unique proposed state -> subject/target domain+lifecycle+cardinality+Facet requiredness -> D6 Policy/version CAS -> author commit. Required masked/unprovable endpoints fail. New/retargeted targets are live. Deleting an old fact proves the true before fact and complete incidence without re-accepting the old target. A retained directed tombstoned/not_found target follows retain_fact_explicit_cleanup only and cannot support a new domain assertion. Symmetric endpoint purge requires explicit removal of all active incident facts.

D4RelationCopyEffects/1 exact:

~~~text
{kind:"d4_relation_copy_effects",
 wireVersion:1,
 operationId,
 sourceRegistryBinding,
 targetRegistryBinding,
 sourceOwners,
 resultOwners,
 facts}

sourceOwners/resultOwners item:
  {nodeRef,sourceRevision}

fact:
{sourceOwnerNodeRef,resultOwnerNodeRef,
 fieldId,occurrenceKey,
 beforeRawEntrySource,afterRawEntrySource,
 referenceChanges}

referenceChanges item:
  {pointer,before,after}
~~~

referenceChanges includes each actually changed typed-ref/locator root exactly once, using an RFC6901 semantic path rooted at $, UTF-8-byte sorted, without overlapping roots. Owner migration is represented by owner fields; occurrenceKey is not a D3 ref. The effect does not replace D3 identityMap/lifecycle/placement/authorization/receipt.

D4SourceMaterializationEffects/1 exact:

~~~text
{kind:"d4_source_materialization_effects",
 wireVersion:1,
 operationId,
 registryBindings,
 owners,
 entries}

registryBindings item:
  {workspaceRef,registryBinding}

owners item:
  {nodeRef,before,after}

before:
  {kind:"absent"}
  OR
  {kind:"source",sourceRevision,sourceBytes}

after:
  {kind:"source",sourceRevision,sourceBytes}

entries item:
{sourceSubject,resultOwnerNodeRef,
 fieldId,occurrenceKey,
 beforeRawEntrySource,afterRawEntrySource,
 referenceChanges}
~~~

sourceBytes is canonical no-padding base64url exact UTF-8 bytes. before absent is limited to fresh Nodes in the same D3 receipt. owners covers every C-carrier result owner and every actually modified existing source container. entries is uniquely sorted by source subject key, FieldId, OccurrenceKey; beforeRawEntrySource is null only for explicit fresh initial Entry. The effect is saved with the same original decision as D3 request/candidate-map/receipt and D6 control evidence; it is not a second author source.

### 16.5 Diagnostic/1, sourceRange, and complete error aggregation

D4 diagnostic envelope exact:

~~~text
weftext.d4.diagnostic/1
closed fields:
  code
  namespace
  fieldId?
  occurrenceKey?
  sourceRange?
# v1 has no machine-significant details map
~~~

Rank/code family at the same source point:

~~~text
10  namespace_owner_unprovable | namespace_owner_conflict
20  provider_or_schema_unavailable | incompatible_schema
30  invalid_entry_json | unsupported_entry_version | unknown_entry_member
40  invalid_field_id | unknown_field
50  invalid_occurrence_key | duplicate_occurrence_key
60  value_type_mismatch | unknown_value_kind | invalid_value
70  invalid_note | invalid_qualifier | invalid_provenance
80  constraint_conflict | field_cardinality_conflict |
    calendar_series_scope_conflict | required_field_missing |
    preferred_selection_conflict
90  facet_dependency_missing | facet_dependency_cycle |
    facet_conflict | field_definition_conflict
100 relation_target_invalid | relation_cardinality_conflict |
    relation_target_unavailable
110 operation_precondition_failed | registry_generation_changed
~~~

Ordering is (sourceStart, rank, stableCode). Unprovable namespace owner wins at rank 10 before inner JSON interpretation. After owner/schema availability succeeds, the same strict parser, catalog resolver, and recursive typed validator collects every independent fault determinable from current bytes without crossing an authority mask; it never short-circuits after the first structural/type error. An availability-failed child masks only that child; independent sibling/note/qualifier/provenance faults continue.

The strict parser retains the narrowest half-open UTF-8 byte span for each key/value/array/scalar. Dynamic key paths use RFC6901 escaping (~->~0, /->~1). Diagnostic sourceStart is the narrowest fault-token start. A missing nested member has no token, so its semantic pointer uses the full half-open span of the nearest existing containing object; zero-width spans are never fabricated.

Entry-local span projects into the bound Document revision through:

~~~text
D2SourceProjection/1 =
{kind:"d2_source_projection",
 documentRevisionToken,
 entryStartUtf8}
~~~

documentRevisionToken is never implicitly taken from the current document.

A strict-JSON object missing required envelope members emits one invalid_entry_json over the full envelope span even when several are absent. Independently interpretable version/Field/value/qualifier/provenance/note branches continue under their availability/version gates. A raw input that cannot strict-parse has only the parse failure and no fabricated nested faults.

Authority/availability outcomes retain their original code/state through object/union/collection/range/qualifier/provenance wrappers. Only a child proved available may normalize a structural/type failure to invalid_value/invalid_qualifier/invalid_provenance. Diagnostic masking never yields TypedValue success.

### 16.6 Calendar recurrence, selector, projection, and series-scope exact contracts

calendar/period-value seriesKey is required exact TypedText; empty string is legal and missing/null/non-string rejects. CalendarPeriod/1 exact members are:

~~~text
calendarId
calendarVersion
timeZone
tzdbVersion
periodKind
periodRuleId
periodKey
~~~

periodKind is day|week|month|quarter|year. Series identity is exactly:

~~~text
(calendarId,calendarVersion,timeZone,tzdbVersion,
 periodKind,periodRuleId,seriesKey)
~~~

Period identity adds periodKey. title/path/locale never participates.

calendar/recurrence-value is a closed date|instant union. Each rule object requires profileVersion=TypedInteger "1", anchor, frequency, interval. Optional members are weekStart, byMonth, byMonthDay, byWeekday, count, until, rDates, exDates, exceptions. Unknown members reject; no system-clock/title/range/horizon/default-timezone default exists.

profile1 frequency is daily|weekly|monthly|yearly and interval is 1..65535. weekly defaults weekStart to monday. byMonth is 1..12; byMonthDay is -31..-1 or 1..31, never 0; byWeekday uses seven closed codes. Invalid month-days are skipped rather than clamped. The instant arm uses the bound tzdb civil rule: DST gap skips a candidate, fold selects the earlier UTC instant, anchor remains its original exact instant, and OS current timezone/86400-second substitution is forbidden.

count is 1..2147483647 and mutually exclusive with until. count applies to deduplicated base candidates before EXDATE/exception. until is inclusive over base start, shares the anchor basis, and is not earlier than anchor. Omitting both means semantically unbounded while execution remains bounded by horizon/work/output budgets.

Set semantics:

~~~text
B := base recurrence set
R := explicit RDATE
E := EXDATE

for each exact temporal originalStart in dedup(B ∪ R):
  cancel exception  -> omit
  replace exception -> use replacement (wins over EXDATE)
  else if in E      -> omit
  else              -> inherit series template
~~~

RDATE may be before anchor or outside count/until and never consumes count. exception originalStart is proved in B∪R. Temporal equality is semantic instant/date equality rather than offset spelling.

Exception exact object:

~~~text
{originalStart,action}

action cancel:
  TypedUnion variant cancel
  value := field-local semantic_code "cancelled"

action replace:
  TypedUnion variant replace
  value := TypedObject
    required range
    optional title,eventStatus,note
~~~

Replacement range shares the arm basis and is ordered; title is non-empty exact text, note is exact text and may be empty, eventStatus is cancelled|confirmed|tentative. Replacement may move start while originalStart remains unchanged. Different originalStarts may display at the same replacement start and remain distinct occurrences.

Recurrence selector exact:

~~~text
{ownerNodeRef,
 fieldId:"calendar/recurrence",
 occurrenceKey,
 originalStart}
~~~

It is revision-bound and not a D3 durable locator.

RecurrenceEditRequest/1 exact:

~~~text
{kind:"d4_recurrence_edit_request",
 operationId,
 editKind,
 ownerNodeRef,
 expectedOwnerRevision,
 expectedRegistryBinding,
 expectedRecurrenceReadBinding,
 beforeRecurrenceSource,
 beforeRangeSource,
 originalStart,
 afterRecurrenceSource,
 afterRangeSource,
 rebaseDecisions}
~~~

editKind is set_exception|remove_exception|edit_series. The first two use originalStart and require empty rebaseDecisions. edit_series uses originalStart=null and supplies one exact decision per old exception:

~~~text
{originalStart,disposition,nextOriginalStart}
disposition := keep|remap|drop
~~~

Decisions exactly cover old selectors. keep preserves semantic selector and requires membership in the new set; remap supplies an explicit new selector; drop requires nextOriginalStart=null and no matching exception in the new source. No two decisions map to one new selector. Rule/tzdb/basis changes without complete explicit rebase fail with zero write.

Recurrence edit pre-state:

~~~text
{nodeRef,coreKind,declaredFacetIds,entries,
 ownerRevision,registryBinding,lifecycle,
 recurrenceProjection}
~~~

lifecycle is live. Outcome:

~~~text
{kind:"d4_recurrence_edit_outcome",
 operationId,status,postState,writeSetOwners,
 readSet,projectionInvalidations,
 rollbackByteEqual,
 intermediateStateObservable:false}
~~~

Successful readSet:

~~~text
{nodeRevisions:[{nodeRef,sourceRevision}],
 recurrence:<RecurrenceReadBinding/1>}
~~~

A no-op accept changes neither source/revision nor cache. A real source change increments the series owner once. Any failure returns complete pre-state, empty writes, null readSet, empty invalidations. Failure injection is harness-only.

Recurrence projection request exact:

~~~text
{kind:"d4_recurrence_projection_request",
 ownerNodeRef,
 expectedOwnerRevision,
 expectedRegistryBinding,
 expectedRecurrenceReadBinding,
 recurrenceOccurrenceKey,
 rangeOccurrenceKey,
 horizon,
 outputLimit}
~~~

horizon is a bounded same-basis range with start<endExclusive. outputLimit is D3Integer 1..2^63-1 and rejects Boolean. Membership of every exception in B∪R is proved before bounded base enumeration. RDATE and proven outside-window exception selectors are included; filtering is by final range intersection with horizon. Result order is exact originalStart, not replacement start.

Projection outcome exact:

~~~text
{kind:"d4_recurrence_projection_outcome",
 status,complete,rows,
 projectionIdentity,
 readSet,
 writeSetOwners}

row:
  {originalStart,range,overrides}

projectionIdentity:
  {ownerNodeRef,sourceRevision,sourceKeys,
   registryBinding,recurrenceReadBinding,horizon}

sourceKeys:
  [{fieldId,occurrenceKey}, ...] sorted by FieldId

readSet:
  {nodeRevisions:[{nodeRef,sourceRevision}],
   recurrence:<actual RecurrenceReadBinding/1>}
~~~

accept has complete=true and writeSetOwners=[]. Any source/binding/domain/member/budget/coverage failure has complete=false with rows/projectionIdentity/readSet all null and writeSetOwners=[]. Exhausted outputLimit is operation_precondition_failed; truncating first N and claiming complete is forbidden.

RecurrenceReadContext/1 exact:

~~~text
{kind:"d4_recurrence_read_context",
 registryBinding,
 workBudget,
 timezoneRuleSets}

timezoneRuleSet:
  {timeZone,tzdbVersion,revisionToken,segments}

segment:
  {start,endExclusive,offsetSeconds}
~~~

workBudget is D3Integer 1..2^63-1. timezoneRuleSets is unique by timeZone/tzdbVersion. segments is a non-empty finite UTC-sorted contiguous non-overlapping array and offsetSeconds is canonical integer string in -86400..86400. Insufficient coverage is provider_or_schema_unavailable; OS timezone never fills gaps. fold selects the earlier UTC instant among all proved solutions.

RecurrenceReadBinding/1 exact:

~~~text
{registryBinding,timezoneRevisions}

timezoneRevision:
  {timeZone,tzdbVersion,revisionToken}
~~~

timezoneRevisions is D3-CJ/3-byte sorted, unique, and complete for rules actually read. expectedRecurrenceReadBinding equals the actual binding item-for-item. Successful Facet outcome carries the actual recurrenceReadSet; failed relation/recurrence read sets are null.

CalendarSeriesScopePolicy/1 exact:

~~~text
{kind:"calendar_series_scope_policy",
 policyId,policyVersion,
 keyMembers,scopeKinds,nodeScopeMember,
 multiplicities,conflictCode,
 retryRevisionRequired}
~~~

Contribution exact:

~~~text
{kind:"calendar_series_scope_policy_contribution",
 policyId,policyVersion,policySchemaDigest}
~~~

Key exact:

~~~text
{series,periodKey,scope}

scope :=
  {kind:"workspace",workspaceId}
  OR
  {kind:"node",scopeNodeRef}

multiplicity := unique | many
~~~

The policy resolves through RegistryBinding + policyId + policyVersion + policySchemaDigest. series is reconstructed from the validated author Entry and periodKey is canonical under the same snapshot. unique returns calendar_series_scope_conflict with zero write when a second different NodeRef exists/concurrently appears at the same key; many allows it. Retry uses the same key with current revision. Path/index never deduplicates.

DerivedDuration/1 is not authorable. A bounded date_range yields:

~~~text
{kind:"calendar_units",
 calendarId,calendarVersion,precision,units}
~~~

A bounded instant_range yields:

~~~text
{kind:"exact_seconds",seconds}
~~~

An open bound, after the remaining provider gate succeeds, yields:

~~~text
{kind:"unavailable",reason:"open_range"}
~~~

Writing duration into an Entry/Field is constraint_conflict.

### 16.7 D6-FA-r01 outer binding, pending, and legacy

All D4 inner wire versions above remain unchanged. A new request uses D6 InputDescriptor/2 to bind every source-bearing owner to complete SourceVersion/2; CommitDomain, Frontier, Registry/Policy/control revisions, and real source pins form the same proof cut. Bare sourceRevision, I cache, equal hash, and old stateToken never carry proof across replica/observationEpoch.

ordinary save may record D6 semantic_pending for relation|unique|calendar|inbound|cross_object_type only after every touched local Entry/Facet/Type/requiredness rule passed. An invalid local fact, a touched retained_unavailable namespace, D2 external_invalid, or a strong Action missing complete proof never succeeds by pending.

Later validation creates qualification only for current SourceVersion; it never rewrites historical r5 receipt as r6 complete. I rebuild, P loss, device migration, and A->B->A do not restore old proof.

Historical D3 v9/v10/v11, D6 wire1, and D7 PreparedActionBinding/1,/2 continue under original decoders/bytes/gates/retention. Until the new D7 Prepared exists, a D4 strong Action that requires complete D7 preparation returns owner_update_required/unavailable with zero new managed-success; D4 invents no new Prepared schema.

## 17. Current D6-FA-r01 cross-owner bridge clauses

### 17.1 People × Organizations intake and rank

The people/engagement NodeRef arm still accepts any ordinary Node and does not require Organization Facet on the target. subjectPredicate remains Person-domain and targetPredicate remains ordinary_node. Organization roster/org-chart enhancements may require Organization semantics but never invalidate the real Person-side engagement or auto-Assign/create/move its target.

These People relation Fields continue to support exact NodeRef/text target union: people/parent, people/spouse, people/sibling, people/guardian, people/family-related, people/social-related, people/professional-relation, people/manager, people/mentor. The text arm is author literal only and has no resolution/inverse/graph/cardinality/lifecycle. The NodeRef arm follows each Field's fixed direction/domain. family/social/professional are symmetric general; parent/guardian/manager/mentor are directed. text<->NodeRef change is an explicit atomic occurrence update.

people/profession remains distinct from engagement. Engagement position/department are text and rank is institution-qualified author data. EngagementRankValue/1 is exactly a TypedObject whose only required members are:

~~~text
system := non-empty exact TypedText
level  := non-empty exact TypedText
~~~

No code, country default, inferred ordinal, or position/department/organization fallback is added. system is author-supplied institutional context text rather than a global system ID, and equal level text does not prove cross-system comparability. Future presets/machine codes require a new closed contribution and explicit migration and never rewrite the existing custom author value.

### 17.2 candidate versus activated Registry ledger

D6-FA-r01 is an unactivated coordinated candidate. Fixed-S Registry/Field/Facet ledger history remains provenance. If an activated Workspace later adopts this candidate, D4 monotonic evolution still requires fresh identity/tombstone/migration and current Policy authorization. Candidate rebinding is never permission to mutate an activated ledger in place.

Copying portable Registry data to another replica copies shared facts only, never Registry administration or execution custody.

### 17.3 D3 wire12 fresh-source composition

The historical fixed-S section 15 reference to a D3 v11 fresh candidate map is replaced for new decisions by H2 D3 wire12 plus D6 PreparedIntent/2/InputDescriptor/2. The semantic obligations remain:

- fresh Node/Resource/Annotation candidates come from a validated D3 intent/plan rather than caller invention;
- D4 initial Entries, Template results, and copy/fork typed refs are validated against the real complete proposed source before planning reservation;
- candidate identity map, allocation/burn, source payload, lifecycle, current Registry/Policy share one original-decision proof;
- symmetric relation source assembly recomputes canonical owner from result endpoints;
- D4 conformance never redraws an already committed ID merely to satisfy a constraint, expands closure, drops provenance, or changes Facet;
- D4SourceMaterializationEffects/1 and D4RelationCopyEffects/1 are independently recomputed from true before/after source and cross-checked with D3 receipt/effects.

Fresh-create D4 semantic pre-state is the same bound complete proposed result source at prospective sourceRevision=1. Success never appends twice or increments it to 2. An existing owner never claims fresh-origin to bypass CAS.

### 17.4 D6 ObservationScope and disclosure

A complete dependency read set does not imply the principal may observe constraint results. Before D4 value/range evaluation, D6 supplies conservative ObservationScope/protected read dependencies bound to the same CommitDomain/SourceVersion/Registry/Policy cut, including potentially empty negative ranges.

Without disclosure authority, two worlds differing only in hidden state produce the same outer not_visible/non-disclosing result before D4 business evaluation, with zero hidden business read/decision. Once disclosure is authorized, real unique/cardinality/incoming/cross-Field checks run completely.

D4 adds no permission token and never trusts a client claim that a range is complete. A local Field path is valid only when both static dependency upper bound and actual read tracing prove independence; otherwise the real complete operation scope applies. This is the same principle as Diagnostic authority masking.

### 17.5 Calendar scope control

calendar/period author value remains period + seriesKey with no scope Field. The scope in a full CalendarSeriesScopePolicy key comes from a D6 same-cut portable/control scope binding, never path, folder, View row, or ambient locale.

D6 owns creation, explicit migration/deletion, control inbound, and version CAS for the scope binding. D4 validates only the typed key and unique|many semantics under the same Registry policy. A new workspace-scoped period needs explicit current configuration. copy/fork rebinds legal scope through D3 map and never skips unique/many validation merely because the target appears empty.

### 17.6 D7 consumer boundary

D4 C carrier, Entry/Facet/Relation/Registry/Recurrence/DerivedDuration remain D4-owned. D7 consumes saved definitions, Query/Action, and effects transport and never becomes authority for D4 author typed values.

Historical PreparedActionBinding/1,/2 and old D3 saved decisions serve legacy replay only. A future D7 owner must define a complete cut and Prepared contract compatible with CommitDomain/Frontier/SourceVersion/2. Until that afterimage is accepted:
- D4 ordinary local source-save remains available under D6 complete/pending rules;
- relation/Facet/Calendar strong Actions that require D7 complete proof return owner_update_required/unavailable;
- no historical binding, I cache, or current page may fabricate new-version success.
