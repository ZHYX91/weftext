---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 6e66067a-08b5-4c21-9791-45aaa6a50323.

# D4 Attribute Types, Schema, and Relations

Candidate status: D6-FA-r01 / P2 D4 coordinated author candidate; not accepted, activated, or implemented. This main text and its paired Lexicon/Impact consume the current P1 producers while preserving effective fixed-S semantics and actual historical recovery. The immutable catalog is unchanged. See §15 for joint acceptance, already-authorized gated A2, and final review.

Fixed provenance:
- S=7e18168dad3e6d120fce0dd607dc10fa7894e252
- source blob=2c03f2050dc523ccd8ffd3696f89f91a3b9c3c07
- unchanged catalog blob=ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb

## 1. Scope and sole owner

D4 solely owns Semantic Namespace, Field/FieldId, Entry/1, TypeSpec/Typed Value, Field Semantic Shape, qualifiers/provenance, FacetSchema/effective closure, relation direction/inverse/canonical owner/cardinality/delete policy, Calendar typed values/comparators/recurrence/series-scope, RegistrySnapshot/RegistryBinding/schema evolution, and D4 semantic validation/diagnostics/effect extensions.

D4 does not own Document bytes, Node/Resource/Annotation identity, parent/order, file installation, SQLite, Query complete cut, Editor Draft, ImportJob, Automation, Extension runtime, or credentials. Current bytes, portable metadata, P/I, SourceVersion/2, SourceVersionRef/1, CommitDomain/2, Frontier/2, SemanticState/1, and ConflictRecord consume the current D6 owner types. Identity/lifecycle consumes D3 wire12. Historical saved decoder versions remain unchanged and are not mechanically rewritten to current consumer types.

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

The shared ID decoder is exact and common to every Field, Facet, alias, inverse code, unit, calendar, and external scheme. `SemanticNamespaceId` is exactly D2 `namespace-token`: one or more dot-separated nonempty lowercase ASCII segments; each segment begins with `a`–`z`, then uses `a`–`z`, `0`–`9`, or internal, nonconsecutive hyphens. Leading, trailing, or doubled hyphens and empty segments are forbidden. The complete namespace is 1..63 ASCII bytes; a dotted namespace is legal and must not be reduced to a single kebab segment.

`FacetId=namespace/facet-name`: facet-name is one segment with the same lowercase/hyphen grammar, 1..63 ASCII bytes, and the complete ID is 1..127 ASCII bytes. `FieldId=namespace/local-field-path` and `SemanticCodeId=namespace/code-path`: the path has 1..8 dot-separated segments, each 1..63 ASCII bytes with that same segment grammar; the complete ID is at most 255 ASCII bytes. Comparison is exact ASCII bytes, with no trim, case folding, Unicode normalization, percent decoding, or locale conversion. The three domains remain distinct even when their lexical bytes are equal; display labels never supply missing IDs.

### 2.2 RegistrySnapshot / RegistryBinding

Registry remains the D4 semantic truth for portable shared configuration. D6 continues to own only its physical carriage, versioning, permission, currentness proof, and managed commit; that does not create a second D4 schema owner. The D4/D10 handshake remains one immutable, generation-bound, read-only `RegistrySnapshot/1`. Its exact members are `kind,registryGeneration,snapshotDigest,predecessor,rows,calendarComparators,calendarPeriodRuleContributions,calendarSeriesScopePolicyContributions,tzdbContributions,unitContributions,semanticCodeContributions,externalSchemeContributions,fieldSemanticBindings,facetSemanticBindings,semanticTombstones,semanticMigrations`. No member is omitted, aliased, extended with unknown members, or replaced by open JSON.

`snapshotDigest` is the complete canonical snapshot-content digest delivered to D4 after D10 authentication and covers every member except `snapshotDigest` itself. `registryGeneration` remains the plan/retry binding token and never substitutes for the digest. Every decode, catalog load, validation, or operation plan carries exact `RegistryBinding/1={expectedRegistryGeneration,expectedSnapshotDigest}` and byte-matches both values against the same snapshot's `registryGeneration` and `snapshotDigest`. Replacing one self-consistent snapshot by another under the same generation is still `incompatible_schema`. One decode/operation consumes one bound snapshot only; it never refreshes mid-operation, mixes generations, or promotes a cache/materialized copy into new Registry truth.

The semantic-owner class remains the closed union `core | first_party | publisher | workspace_user`. Reserved owner tuples remain exactly `core→(core,weftext.core)`, `d4→(core,weftext.d4)`, `tasks→(core,weftext.tasks)`, `people→(first_party,weftext.people)`, `organizations→(first_party,weftext.organizations)`, `calendar→(first_party,weftext.calendar)`, `library→(first_party,weftext.library)`, and `user→(workspace_user,<current WorkspaceId>)`. `tasks` remains Core-exclusive; `people`, `organizations`, `calendar`, and `library` remain first-party reserved; `user.ownerId` byte-equals the current D3 WorkspaceId authenticated by D10; `wf` is permanently unallocatable and any `wf` row in a snapshot rejects. One namespace never has two active owners. Install order, current enablement, display name, and localized label are not owner proof. When ownership is unprovable or conflicting, D2 raw source may still be read under its own authorization, but D4 typed state is never interpreted as known or empty and a typed operation that touches or depends on that namespace cannot proceed as success.

Each namespace row has the exact members `namespaceId,ownerClass,ownerId,aliasDefinitionsState,aliasSchemaDigestSet,facetDefinitionsState,facetSchemaDigestSet,fieldDefinitionsState,fieldSchemaDigestSet,registryGeneration,verificationState`. `aliasDefinitionsState`, `facetDefinitionsState`, and `fieldDefinitionsState` are independent and each is only `complete|unavailable`. `complete` with an empty digest set proves that the complete loaded set truly contains no definitions. `unavailable` requires the corresponding digest set to be empty and means that the definition set cannot be proved. The two states are never interchangeable. D4 accepts only `verificationState=verified` rows with exactly one owner for the namespace.

The seven contribution classes keep the fixed-S closed objects:
- calendar comparator: `{kind:"calendar_comparator_contribution",calendarId,calendarVersion,precision,comparatorId,orderedLexemes}`;
- calendar period rule: `{kind:"calendar_period_rule_contribution",calendarId,calendarVersion,periodKind,periodRuleId,keyProfile}`;
- calendar series/scope policy: `{kind:"calendar_series_scope_policy_contribution",policyId,policyVersion,policySchemaDigest}`;
- tzdb: `{kind:"tzdb_contribution",tzdbVersion,zoneIds}`;
- unit: `{kind:"unit_contribution",unitId,dimensionId}`;
- semantic code: `{kind:"semantic_code_contribution",codeId}`;
- external scheme: `{kind:"external_scheme_contribution",schemeId}`.

Every collection is first proved to be a real JSON array. Empty object/string, `null`, Boolean, or number never masquerades as an empty array, and malformed containers/elements produce closed incompatibility rather than a host exception. Arrays are strictly ascending by `D3-CJ/3` canonical bytes for their frozen identity tuples and identities are unique. Unknown members, illegal `null`, duplicates, disorder, out-of-bounds values, invalid IDs/profiles/zones/lexeme sets, or contributions outside the verified owner make the whole snapshot `incompatible_schema`. The seven contribution identities remain calendar comparator `(calendarId,calendarVersion,precision)`, period rule `(calendarId,calendarVersion,periodKind,periodRuleId)`, series/scope policy `(policyId,policyVersion)`, tzdb `tzdbVersion`, unit `unitId`, semantic code `codeId`, and external scheme `schemeId`. `calendarPeriodRuleContributions` still enforces the closed `periodKind↔keyProfile` mapping `day→iso-date-v1|week→iso-week-v1|month→iso-month-v1|quarter→iso-quarter-v1|year→iso-year-v1`; a valid profile cannot be reassigned to another kind. `dimensionId` is a verified-owner `SemanticCodeId`. Quantity-dimension proof comes from the current `unitContributions[].dimensionId` plus the corresponding namespace owner proof; the unit name itself implies no dimension and `dimensionId` need not also appear in `semanticCodeContributions`. Measurement `byCode` still matches the current unit-contribution ledger.

Namespace-owner lookup and `RegistryBinding/1` validation happen before reading, parsing, or interpreting inner Entry JSON. Zero verified rows yields `namespace_owner_unprovable`; multiple usable owner rows yields `namespace_owner_conflict`; generation mismatch against the plan yields `registry_generation_changed`; incompatible snapshot digest or loaded-definition digest yields `incompatible_schema`. These failure paths do not call the inner parser. A reserved-tuple mismatch likewise yields `namespace_owner_unprovable` before Entry parsing. Signatures, publisher identity, package install/trust/revocation, and how rows become authenticated remain D10-owned. D4 never reads package installation order or credentials to synthesize proof, and D10 authentication mechanics never alter already validated D4 schema semantics.

Every loaded alias, `FieldDefinition`, and `FacetSchema` fully expands aliases, passes closed validation, computes its semantic digest with `D3-CJ/3`, and matches that digest in the corresponding verified namespace row's advertised set. Any alias/Field/Facet definition set marked `complete` also proves reverse completeness: the actual loaded digest set equals the advertised digest set item-for-item. Missing or extra loaded definitions fail context construction. Only a complete set can determine known/unknown definitions. A complete empty set may produce a real unknown-definition result, while `unavailable` returns `provider_or_schema_unavailable` before inner Entry parsing. Calendar comparator, period rule, series/scope policy, tzdb zone, unit, registry-scope semantic code, and external scheme also match their contribution table under the same `RegistryBinding/1`; namespace-owner proof alone is insufficient.

A successful `ValidatedCatalogContext` immutably binds one authenticated current WorkspaceId. That Workspace may come from the explicit host Workspace supplied by D10 or the unique verified `user` owner row in the same authenticated Registry; when both are supplied they byte-equal or context construction fails. The source containing owner on public Entry, Facet, Relation, and Calendar/recurrence acceptance paths belongs to that context Workspace. A mismatch yields `namespace_owner_unprovable` before inner Entry parsing/semantic planning, preserves the complete original state, and has no write set or successful read set. Registry structural validation proves only that `user` carries a legal D3 WorkspaceId and `workspace_user` class; it never treats a conformance-fixture Workspace constant as product identity. This source-owner constraint does not rewrite or ban legal cross-Workspace NodeRefs appearing as ordinary value/provenance; their resolvability, disclosure, and target constraints remain governed by their own contracts.

`fieldSemanticBindings` and `facetSemanticBindings` are canonically ordered by `(fieldId,semanticMajor)` and `(facetId,semanticMajor)` respectively. A Field item is exactly `{fieldId,semanticMajor,semanticDigest}` and a Facet item is exactly `{facetId,semanticMajor,semanticDigest}`. The Field digest covers fully expanded `valueType` plus `shape,qualifierSetId,cardinality,occurrenceOrder,duplicatePolicy,constraints,relation`; the Facet digest covers `requires,conflicts,fields,relations,constraints`. Package version, map iteration order, and display metadata never substitute for the semantic digest of the same identity.

`predecessor` has only two closed arms: the one-time D10-trust-root-authenticated `{kind:"registry_bootstrap",bootstrapId:"weftext-d4-semantic-ledger-v1"}`, or the mandatory later-generation `{kind:"registry_predecessor",registryGeneration,snapshotDigest}`. Every non-bootstrap validation plan supplies closed `RegistryEvolutionProof/1={previousSnapshot,currentSnapshot}`. `currentSnapshot.predecessor` exact-binds the previous generation and complete digest. Both `previousSnapshot` and current snapshot independently satisfy their D10 authentication prerequisites and D4 closed validation; current ledger is never merely compared with itself. `proof.currentSnapshot` is the complete current snapshot used by the operation and `previousSnapshot` matches the `predecessor`.

The sole successful Registry/catalog load result is one immutable `ValidatedCatalogContext`: complete `RegistryBinding/1`, closed-snapshot validation, and required `RegistryEvolutionProof/1` happen first, then catalog loading under that same binding. A non-bootstrap snapshot without the evolution proof exposes no definitions usable for Entry or operation acceptance. Public Entry, Facet, and Calendar/Relation planners consume only this context plus explicit revision-bound inputs. A materialized inspection copy, low-level schema decoder, or individual fixture predicate never replaces the context or independently issues complete acceptance. This conformance factory may check structure, digest, and evolution; real D10 signature, publisher-identity, and trust-root authentication remain explicit upstream prerequisites. A Python/object wrapper is never described as implementing provider trust.

Before one snapshot reaches any catalog/context/transition entry point, its own history is internally consistent: active and tombstone identities are disjoint; every tombstone has exactly one migration whose from identity/digest matches; every migration target digest resolves to a current active binding or a tombstone retired later; the history graph is acyclic and eventually reaches an active identity. Bootstrap is the first ledger with no retirement or migration history and historical import never masquerades as a fresh bootstrap. Upstream authentication does not waive these structural or semantic checks.

Generation-to-generation comparison covers both Fields and Facets. A retained major-1 definition under the same ID keeps a byte-equal digest. Raising the major under the same ID, changing its digest, or silently deleting it is `incompatible_schema`. Every disappearing active ID appears in current as exact tombstone `{kind:"semantic_tombstone",semanticKind,semanticId,semanticMajor:1,semanticDigest,retiredInGeneration,reasonCode:"replaced_by_migration"}`, and a newly created tombstone's `retiredInGeneration` byte-equals the current `registryGeneration` that first records retirement; later generations preserve it byte-for-byte. Every retired ID also has exactly one exact migration `{kind:"semantic_migration",migrationId,semanticKind,fromId,fromDigest,toId,toDigest}` to a fresh active ID. `fromDigest`/`toDigest` match previous/current ledgers and `fromId != toId`. Orphan tombstone, orphan migration, deletion without migration, replacement without tombstone, digest mismatch, and wrong retirement generation all reject. Migration-record identity is `(semanticKind,fromId,toId)`. `migrationId` is immutable UUID payload only; one migration batch may reuse it across distinct identity records and `migrationId` is never used to lossy-merge history.

`semanticTombstones` and `semanticMigrations` are cumulative, item-wise byte-immutable ledgers across every later generation. Current contains every previous record unchanged; a tombstoned ID never becomes active again and cannot be resurrected by dropping history for one generation and returning under a new digest. All seven contribution tables likewise form monotonic identity ledgers: an existing calendar comparator, period rule, series/scope policy, tzdb version, unitId, codeId, or schemeId is neither deleted nor mutated under the same identity; new semantics uses a fresh identity. A fresh Field/Facet unrelated to any replaced semantic may be added without fabricating a migration. Bootstrap, wrong predecessor, same-ID mutation, silent deletion, incomplete/complete replacement, tombstone mismatch, wrong retirement generation, standalone addition, multi-generation resurrection attack, and same-identity contribution mutation remain future corpus/acceptance obligations; this candidate prose does not claim that any such scenario has been executed or passed.

Contribution preflight and structural diagnostics share constructor recognition at the declared slot. Field/QualifierSet or D3 reference position first determines the permitted constructor/closed arm. A missing, unknown, or mismatched constructor reports only the type diagnostic at that location, does not interpret inner members, and does not parse inner contribution IDs; independently interpretable siblings continue. A D3-unmarked source span gains no fabricated `kind` requirement. Once the constructor is recognized, every contribution-backed identifier recursively completes availability preflight and namespace-owner proof against the same bound snapshot before its containing object's structure/type is interpreted. This covers values in object/union/collection, range bounds, qualifier `validity|eventTime|observedAt|status`, external-provenance `scheme|observedAt`, registry-scope semantic code, calendar ID/version/comparator, tzdb version/zone, unit ID, external scheme, alias ID, and every directed-relation inverse code. A syntactically valid identifier whose owner or concrete contribution is unproved never degrades to an ordinary unknown string and is never normalized by a wrapper into `invalid_value|invalid_qualifier|invalid_provenance`; raw source is retained as `retained_unavailable`, or the operation that actually touches it is rejected. One decode always uses the same fixed snapshot.

Portable Registry bytes remain shared configuration facts. Copying them to a replica copies neither Registry administration nor execution custody and grants no right to advance `registryGeneration`. Registry mutation still requires current Policy, Registry-admin qualification, and applicable D6 control/execution responsibility. The current P1 D6 `registry` `DependencyKey/2`/proof only binds the snapshot/binding, complete directory, and continuity used by this operation into the protected current cut. D4 does not duplicate that D6 carrier/schema here and does not let the proof replace the D4 Registry semantics above.

For every `calendarSeriesScopePolicyContributions` row, `(policyId,policyVersion)` is the contribution identity and `policyVersion` is nonempty. `policyId` passes the verified Namespace Owner preflight against this same bound snapshot. `policySchemaDigest` is SHA-256 of D3-CJ/3 canonical bytes of the **complete closed `CalendarSeriesScopePolicy/1` value defined in §16.6**; a label, subset, Registry digest, or another snapshot's policy never substitutes. Row identity and digest are checked together with that full value before a SeriesScope consumer can use it.

### 2.3 Reference Catalog v1

The fixed catalog continues to define exactly 4 `QualifierSetSpec` objects, 22 aliases, 61 `FieldDefinition` objects, 7 `FacetSchema` objects, and 1 `CalendarSeriesScopePolicy`. This batch restores the existing D4 Registry/catalog contract only; it creates no catalog replacement and changes no fixed-S catalog bytes.

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

The catalog loads only under the same `RegistryBinding/1`, immutable `ValidatedCatalogContext`, Workspace binding, and applicable evolution proof completed by §2.2. Any failure in alias closure, any Field/Facet exact member set, QualifierSet, requires/conflicts graph, semantic-binding digest, contribution availability, global limit, or required canonical sorting/uniqueness makes the entire catalog/context unavailable. No “partial success” definitions are exposed for Entry/Facet/Relation/Calendar acceptance and a loaded subset never represents a complete catalog. Every namespace marked `complete` still proves reverse equality between the actually loaded digest set and the Registry-advertised digest set; `unavailable` remains strictly distinct from a genuinely complete empty set.

All later D4 parsers/planners consume only this complete context. A single schema/decoder, cache, materialized inspection copy, partial catalog, or current UI/module enablement never substitutes for it. Existing fixed-catalog TypeSpec/Facet/Relation/Calendar/People/Organizations/Library/Task semantics, recursive-depth limits, and byte/collection budgets remain governed by later sections and the catalog itself. This coordination changes none of those domain rules, source selectors, or revision wire.

## 3. Entry/1 and author source

A D4 Field Value Occurrence always comes from the owning Node's D2 exact-source Document. Attribute Carrier Block/Lexical Attribute Entry are current-revision lexical occurrences, not Entities, Records, sidecars, or database rows.

D4 `Entry/1` retains its original closed semantic members: version, FieldId, occurrenceKey, typed value, optional qualifiers, optional note, and optional provenance. Unknown/duplicate members, wrong constructors, invalid UTF-8/JSON, budget overflow, and Field/schema mismatch continue to fail in the original diagnostic order. This group versions neither `Entry/1`, `occurrenceKey`, nor any selector wire.

`occurrenceKey` remains only a value-internal selector scoped to owner Node + FieldId + expected current source revision. It is not EntityRef, Locator, OperationId, RecordRef, or cross-revision identity. For a new managed path using this P1 contract, that inner expected source revision byte-equals the complete managed production `SourceVersion/2.revision` of the actual source-bearing owner. The production version's own production `CommitDomain` and production `observationEpoch` remain in the outer `SourceVersion/2` and are never folded into the inner integer. A legitimately current managed before may have a production domain different from the current operation observer domain; that cross-production-domain observation is not itself stale or a domain mismatch. What is forbidden is donating the foreign before revision to the after production domain's H, using it as another owner/production version, or placing external `externalSequence` into a managed inner revision.

Current runtime selector qualification still comes from the complete `SourceObservation/1` under the current operation `CommitDomain`; `SourceVersionRef/1` selects that protected Observation. A persistent authored D3 Locator is different: its d6_source_revision/2 token first resolves through the authenticated canonical `RevisionTokenBinding/2` to one exact production version, then a new D3 read independently proves a current Observation whose complete sourceVersion equals that address before coordinates/profile are used. This new read never replaces the D4 inner integer or revives an old occurrence selector, relation binding or prepared action. Relation `incidenceScopes[].revisionToken` remains the relation-incidence range token, not this source-version address.

An external `SourceVersion/2` has no managed revision. Authorized valid external bytes remain usable for raw/source read, D2 repair, Draft, and ordinary offline/human whole-source work under their own contracts; absence of a managed inner integer never permanently disables those paths. Only a structured/typed-selector path that truly requires a D4 managed inner revision first completes an explicit authorized managed admission/save. For example, after a qualified human whole-source save successfully seals, its real managed production `SourceVersion/2` supplies the revision used by a later new structured request. Reading external source never auto-writes, silently admits, or adds Approval merely to manufacture a revision. If admission/save seal or outcome cannot be proved, the original plan/pins/unknown boundary remains and the structured request never guesses a revision from equal bytes/hash, `externalSequence`, or current file shape; raw/read/repair/Draft/ordinary capability remains independently qualified.

Equal values may have multiple occurrences. External edit/reorder/delete+reinsert, production-version change, or current-observation discontinuity never preserves an old selection by key/value. A watcher gap invalidates the old Observation/sourceToken and every D4 runtime selector that depended on it. If protected history still proves the exact same managed production version, an independent later D3 Locator read may obtain a new current Observation and succeed as a read; that never re-signs or restores the old D4 selector. Equal hash/text or rebuilt I cannot prove either production-version equality or old selector continuity.

Inline Field Note remains optional plain author text; absence writes no empty placeholder. Typed qualifier/provenance is never hidden in note text.

Raw Entry, carrier framing, comments, line endings, unknown namespace bytes, and unselected occurrences retain the D2/D4 lossless contract. A structured mutation binds complete before source, original selector/span, the real managed-before revision, outer complete `SourceVersion/2`/`SourceObservation/1`, and `RegistryBinding`. Equal hash, equal bare revision, or equal token payload proves neither continuity nor absence of ABA. Historical saved/planned/legacy bytes continue under the decoder/revision contract that produced them and are never retroactively upgraded by this P1 bridge.

## 4. TypeSpec, Typed Value, and Field Shape

The fixed-S constructor set remains: text, Boolean, integer, decimal, semantic_code, calendar_date, zoned_instant, date_range, instant_range, quantity, node_ref, resource_ref, annotation_ref, external_identifier, closed object, closed union, bounded ordered collection, and alias_ref.

D4 integer is a canonical signed decimal string and differs from D3Integer. decimal rejects exponent, NaN, Infinity, negative zero, and redundant fractional zero. ResourceRef/AnnotationRef owner equals the containing owner NodeRef; D4 never fills ambient owner.

calendar_date uses only the verified calendar/version/precision comparator. Built-in `calendar/iso8601` v1 still validates real Gregorian dates and years 0001..9999. zoned_instant keeps exact RFC3339 plus IANA timeZone/tzdbVersion rather than device defaults. date_range/instant_range remain end-exclusive, at least one endpoint bounded, and strictly start < endExclusive when both are bounded.

quantity preserves exact decimal + namespaced unitId and performs no conversion without a verified conversion contribution.

Field Semantic Shape remains closed to fact, event_assertion, observation, relation. Shape owns legal qualifiers/projection; event/state/observation/relation semantics are not flattened into an untyped row list.

### 4.1 Exact scalar and temporal semantics

Text is preserved as authored. A schema chooses exact comparison or `nfc-for-compare`; NFC comparison never rewrites source. `nonEmpty`, when declared, is exactly true. Optional absent members remain absent, not null, empty text, zero, or a default. Boolean is not an integer. D4 integer accepts exactly `0|-?[1-9][0-9]*`; D3 control counters retain their separate bounded `D3Integer` contract. D4 decimal accepts exactly `(?:0|-?[1-9][0-9]*|-?(?:0|[1-9][0-9]*)\.[0-9]*[1-9])`: `0`, `0.5`, and `-0.5` are legal; `-0`, `0.0`, `-0.0`, `1.20`, and exponent forms are not. Neither number type acquires a host-language integer-digit, floating-point, or decimal-context precision limit. An explicit precision fact is represented by its declared schema, never by redundant zeros.

`zoned_instant` validates ASCII RFC3339 digits, a real Gregorian local date in years 0001..9999, `T`, hours 00..23, minutes/seconds 00..59, optional arbitrary-length fractional seconds, and `Z` or signed hours 00..23/minutes 00..59. Unknown offset `-00:00` and leap second 60 are rejected. Exact proleptic arithmetic may decode a legal local instant into UTC year 0 or 10000; host date limits, float rounding, and microsecond truncation cannot reject or alter it. The textual offset is retained and need not equal the separate IANA `timeZone`'s offset. That zone and `tzdbVersion` must be verified; creating an instant in a local gap/fold requires an explicit resolved instant rather than device defaults.

Date comparison uses the verified `(calendar,calendarVersion,precision)` comparator, with equal basis at both bounded range endpoints; lexical order is not a universal calendar comparator. Instant comparison uses exact decoded UTC values, including cross-offset equality and arbitrary fractions. A range has at least one bound and, when both exist, strict `start < endExclusive`; open is distinct from unavailable. A quantity's exact decimal and verified unit contribution must agree with its dimension. No implicit conversion is permitted without the relevant verified contribution.

### 4.2 Closed recursion, constraints, and validation limits

The §16 TypeSpec, ObjectMemberSpec, UnionVariantSpec, qualifier, provenance, and Field/Facet contracts remain the only closed shapes. Bounded sequences/sets occur only as object members, including recursive object members; they are not a root Field/alias type or a direct collection item. Repeatable Fields use separate Entries. There is no generic array, map, any, binary, executable expression, or provider JSON escape hatch. Alias expansion validates every referenced definition in the same immutable Registry, rejects missing/cyclic definitions, and counts the root as depth 1 with maximum depth 8 across alias/object/union/item edges.

After permission and availability preflight, the Entry byte cap is checked before allocating its parser, nested spans, or candidate structures, including Facet pre-state, initial Entries, and Cleanup input. Schema validation memoizes expanded-size bounds and uses saturating checked arithmetic before allocation: UTF-8 bytes, escaping, punctuation, repeated aliases, and outer Field/Facet wrappers all count. The existing count/byte limits remain independent; no new dependency-depth or host numeric/time cap is added.

`mutually_exclusive_members` names must be direct members of every object arm of the root object/root union, with at least one such object arm. Runtime checks inspect only the validated active object. `measurement_unit_dimension` maps the complete legal namespaced code domain: exactly the contribution set, or every code in the declared namespace. It cannot map a Field-local code as a namespaced ID. Every mapped dimension agrees with the unit contribution; height is length and weight is mass. Admitting another legal namespace code requires a complete updated mapping. `union_variant_equal` requires every occurrence of both Fields to use one identical branch; two equally mixed branch sets do not pass.

Every D3 Ref/Locator uses the shared exact D3 decoder. Resource/Annotation values remain owner-local; a node provenance Locator has the same owner as its node Ref. External provenance requires a verified scheme and nonempty identifier; transform inputs reference earlier provenance array positions through `D3Integer`, with the original operation UUID. Qualifiers carry time/status/confidence/selection under their declared shapes, while role and other value members stay in the value. Inline note remains separate plain text and cannot override either. Independent valid-to-interpret branches are all checked under §11; missing contributions never trigger speculative nested parsing.

## 5. FacetSchema

D2 source `facetMemberships` remains the sole author source for declared membership. D4 interprets it as an unordered exact FacetId set; source order gives no precedence/override/last-wins. Effective closure is mechanically derived from the current Registry requires graph, rebuildable, and never written back.

Missing dependency, cycle, conflict, or incompatible Field definition makes affected typed state unavailable/invalid and blocks touching operations. FacetSchema closes fields, relations, requires, conflicts, constraints with no implicit membership, priority, or shadow.

Current Task semantics remain ordinary Node + source-declared exact `tasks/task` Facet. Template remains the D2/Core meta-kind and Template + exact tasks/task is invalid. D4 does not restore specialization dual authority.

Assign/Remove/Cleanup Facet is a strong typed Action requiring complete source/Registry, closure, relevant requiredness, complete relation incidence, Policy/auth, and exact post-state. ordinary pending save does not downgrade that Action.

The requires graph has no self-edge or cycle; conflicts are symmetric and irreflexive and are checked over the entire effective closure. The existing limits are 1024 registered Facets and 32 declared Facets. Iterative memoized traversal supports a long legal DAG without inventing a separate dependency-depth cap or using host recursion overflow as semantics. Effective `tasks/task` without explicit source-declared `tasks/task` is a conflict even when the Node has no relations; an extension may require Task only when Task is also explicitly declared. Node classification comes from the fully source-bound D2 ordinary/Template classification, never a caller's `isTask` flag.

Create/Assign consume ordered initial Entries. Assign's result membership is exactly the original sequence plus the requested Facet and preserves existing source. Remove removes one declared Facet; dependents must be removed first, with no implied atomic multi-Facet removal. Remove retains Field bytes as `retained_without_membership`. Cleanup deletes only explicitly selected Entries of Fields unused by the remaining effective Facets, using exact `{fieldId,occurrenceKey}` selectors after availability preflight. Each selector binds the matching pre-state Entry reference `{fieldId,occurrenceKey,rawEntrySource}`; rawEntrySource is not a selector member. Unselected Entries/order/raw bytes remain exact. Requiredness and every affected relation endpoint/domain are revalidated in the actual post-state. The managed result revision comes from the same D6 source plan in §3/§16.3, never a local fresh=1/old+1 allocator.

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

Each new D4 operation consumes the real D6 `DependencyProof/2` only for its applicable proof duties; raw read/repair does not acquire an unrelated complete-range requirement, and no operation must enumerate all fourteen key kinds; it never accepts free owner JSON, a coarse `Frontier/2`, a Derived Index hit, or a caller claim of completeness. The closed `DependencyKey/2` union has exactly fourteen kinds: `source`, `lifecycle`, `placement_range`, `ref_inbound`, `relation_incidence`, `calendar_scope`, `registry`, `temporal_rules`, `authorization`, `foreign_binding`, `query_scan`, `replica_registry`, `conflict_record`, and `execution_resource`. The `StructureRange` union nested under `placement_range` has exactly nine variants: `live_children`, `trash_children`, `trash_roots`, `ancestor_chain`, `subtree`, `owner_resources`, `owner_annotations`, `reply_closure`, and `restore_membership`. D4 does not copy another owner's enumeration algorithm. D3 continues to own lifecycle/placement/ref-inbound and structure-range enumeration, D6 continues to own the source/authorization/replica_registry/conflict_record/execution_resource carriers and their control proof, D7 continues to own complete `query_scan` enumeration, and concrete foreign-binding version comparison remains with D3/D9/D10 or the real source owner. D4 itself must completely implement and prove the semantic enumeration for `relation_incidence`, `calendar_scope`, `registry`, and `temporal_rules`; D6 carries only their closed key, stamp, evidence pins, currentness, and commit/recovery consumption boundary.

The minimum complete D4 semantics of those four keys are fixed. `relation_incidence(fieldId,endpointNodeRef)` reuses the exact `RelationReadContext/2.incidenceScopes` `fieldId`, `endpointNodeRef`, current incidence `revisionToken`, and every `factSelectors={ownerNodeRef,fieldId,occurrenceKey}`, and proves canonical owner, each source/entity state, the real old/new endpoints, and the complete positive, negative, and empty incidence range. A literal arm performs no target Node lookup/incidence, target cardinality, inverse, graph, or target lifecycle check, and one symmetric authored fact is counted only once at one endpoint. `calendar_scope` follows the D6 closed `CalendarRange` arms `binding|series|period|scope_inbound` and proves the real binding/configuration revision, same `RegistryBinding`/policy, complete period membership and negative range, and applicable control inbound. Configuration-absent, binding-absent, and a complete-empty period range are independently versioned states. `registry` proves the complete same-Workspace `RegistrySnapshot/1`, `RegistryBinding/1`, required `RegistryEvolutionProof/1`, the immutable `ValidatedCatalogContext`, and every alias/Field/Facet/namespace/contribution directory actually used by this operation; the D6 `registry` carrier never erases or summarizes those D4 contracts. `temporal_rules` binds every calendar comparator, period rule, timezone/tzdb rule set, `RecurrenceReadBinding/1`, and finite horizon coverage actually read. It never materializes infinite time and device-current timezone never fills a gap.

All fourteen keys share one completeness discipline. Applicable `ObservationScope/2`, current `Policy/3`, and owner disclosure are established before hidden sibling/member/incidence/calendar/Registry content is read. Empty and non-empty ranges use the same complete-enumeration standard. A `stamp={epoch,revision}` epoch is the proof-continuity generation of that exact `DependencyKey/2`; it is not production `SourceVersion/2.observationEpoch`, current `SourceObservation/1.observationEpoch`, a CommitDomain generation, or a Workspace-global epoch. A new epoch may begin at revision 0 only after a real complete enumeration; zero means the first complete proof revision in that range generation, never empty, unknown, not-scanned, or source revision zero. Watcher/journal gaps, owner rule/decoder-version changes, an unrecoverable event gap, actual loss of the correctness-evidence directory, or unproved range continuity invalidate the old stamp and require a new proof-continuity generation. Equal final bytes/hash/member count never restores it. Deleting or rebuilding I alone merely rebuilds cache when the real P/portable-metadata correctness boundary and gap-free continuity remain complete; it neither re-signs proof nor changes epoch.

P/portable metadata continues to retain the real closed key, stamp, complete-enumeration boundary, last-reference pins, and continuous-consumption evidence; I caches candidates/enumerations only. A `source` proof uses the complete current `SourceObservation/1`, real production `SourceVersion/2`, `FileObjectBinding`, exact source/value pin, and current source validity. Within one committable D4 proof cut, source Observation/FileObjectBinding/pins, control, Registry/rules, authorization, and every actually read range stamp are mutually consistent. `Frontier/2` proves only its own verified sealed causal prefix and never proves range completeness. unavailable/unknown, a missing shard, placeholder, I/O failure, partial/building I, no-hit, equal hash/member count, provider “synced”, or a causal-Frontier prefix never masquerades as complete or empty.

Each operation still proves its local facts and only the complete positive/negative ranges it actually requires. `semantic_pending` never skips locally decidable Entry/Facet/type/cardinality/requiredness, a real deny, a touched `retained_unavailable` namespace, or a D2-invalid gate. Ordinary/local semantic scope remains orthogonal to the `strict|observed_only` installation axis; this contract expands no weak-protection eligibility. Missing an unrelated strong-range proof never permanently disables an otherwise qualified ordinary/local path that does not depend on it, while Facet/Relation/Calendar strong Actions, unique/many, typed copy/fork/import, restore/purge, and D7 complete Action paths that do depend on complete proof still obtain the real proof and remain `strict` structured/strong paths.

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
| relation add/update/delete | `RelationReadContext/2` + `RelationReadBinding/2` | complete incidence/endpoint/domain/cardinality | complete only |
| local recurrence value edit | exact TypeSpec/calendar/tzdb/source post-state | local if no series uniqueness | ordinary or pending |
| series-scope unique/many | recurrence/period + policy | complete `(series,periodKey,scope)` positive/negative range | complete only |
| raw copy/export | exact source + status | no typed-success claim | byte-preserving + loss/status |
| typed copy/fork/import | D3 wire12 map + D4 typed source | complete canonical-owner/requiredness | managed_atomic only |
| restore/purge | D3 managed_atomic + current D4 facts | complete relation/inbound; purge also needs real replica/cut proof | missing proof rejects |
| D7 all_result/bulk/automation write | local typed is prerequisite only | future D7 complete cut | unavailable before D7 afterimage |

A D6 obligation `relation|unique|calendar|inbound|cross_object_type` still means “unproved”, never “allowed to violate”. A known relevant scope/stamp change follows the original owner's stale/conflict/reprepare path. Unproved continuity/completeness is unavailable/unknown, not a passed constraint. D3 local Trash may retain a real pending obligation where its contract permits; restore/purge never reuse that pending state as strong proof.

Later r6 validation proves only r6/current and never rewrites an r5 receipt, old readSet, or historical strong qualification. Rebuilding I, equal hash, device migration, replica registration, external A->B->A, or an observation-generation change never restores old proof.

## 8. Relations

Relation kind/direction/inverse/cardinality/resolution/delete policy/graphProjection belongs to FieldDefinition, not an open RelationType registry. Directed relations author one source fact and derive inverse. Symmetric relations author exactly one fact at canonical owner selected by NodeRef canonical encodings; canonical owner never bypasses authorization.

The text arm of `node_ref_or_text` has no target lookup/inverse. text->NodeRef is explicit and atomic.

Fixed closed shapes remain:
- `RelationReadContext/2 = {kind:"d4_relation_read_context",wireVersion:2,registryBinding,nodeStates,entries,incidenceScopes}`;
- nodeStates union source_node_state / sourceless_node_state / masked_node_state / unprovable_node_state;
- `RelationReadBinding/2 = {kind:"d4_relation_read_binding",wireVersion:2,nodeRevisions,entityStates,incidenceRevisions}`.

D6 provides trusted snapshot/stateToken/source revision/incidence revision/auth. Client never self-reports complete or empty incidence.

New D6-FA requests retain the existing D4/2 inner wire and the shapes of `RelationReadContext/2`, `RelationReadBinding/2`, `sourceRevision`, `expectedSourceRevisions`, and `incidenceScopes[].revisionToken`. Outer `InputDescriptor/2` binds complete `SourceVersion/2` for every source-bearing owner and Context owners biject to `sourceInputs`. When that owner is managed, the inner `sourceRevision` byte-equals the complete managed production `SourceVersion/2.revision`. `SourceVersion/2` retains its own production `commitDomain`, production `observationEpoch`, managed `revision/changeId`, or external `externalSequence`. A legitimate managed before may have a production domain different from the current operation observer domain and is never rejected merely for being from a foreign production domain. What is forbidden is donating that foreign-before integer to after-domain `H(D,E)`, using it as another owner/version, or placing `externalSequence` into a managed inner revision.

Current qualification comes from complete `SourceObservation/1` under the operation `CommitDomain`: `observerDomain` equals the operation domain, `entityRef` matches `sourceVersion.entityRef`, and current `observationEpoch`, `FileObjectBinding`, evidence pins, control revisions, Registry binding, and relation-incidence dependencies belong to one current cut. The `SourceVersionRef/1` `d6_source_observation/1` sourceToken only selects that protected Observation. Where a proposed/current selector needs revision-token currentness, it consumes the real `RevisionTokenBinding/2`/closed `RevisionTokenSource/2` and does not change the inner integer. `incidenceScopes[].revisionToken` continues to version only the complete relation-incidence range and never substitutes for a source revision token/binding.

An external source has no managed inner integer. Raw/read/repair/Draft/ordinary paths remain independently available. A new relation structured mutation that really requires managed inner revision first obtains a real managed production revision through explicit authorized managed admission/save. Reading never auto-admits/writes and adds no Approval. Unknown admission/seal never guesses revision or produces relation success. The D4/2 inner wire, legacy saved decisions, ownership bijections, permission/authorization gates, and historical revision bytes remain under their original contracts.

The relation gate remains D3/D6 disclosure/auth -> D4 closed context/binding -> coverage -> immutable cut -> Registry/raw Entry/selector/revision -> complete incidence -> proposed state -> domain/lifecycle/cardinality/Facet requiredness -> D6 CAS -> author commit. “Complete incidence” here is proved jointly by the real `DependencyKey/2.kind=relation_incidence` and the D4-owned enumeration algorithm; it is never inferred from `Frontier/2`, I, the mere presence of `RelationReadBinding/2`, or a caller completeness flag.

Every `(fieldId,endpointNodeRef)` actually required by this operation has one complete `relation_incidence` proof that byte-matches the corresponding `RelationReadContext/2.incidenceScopes` `revisionToken` and all `factSelectors`. Positive evidence covers the real source facts, source owners, canonical owners, and source/entity lifecycle state. Negative evidence proves that no active incidence was omitted at the same cut. Relation add/retarget enumerates the real scopes needed for the subject, new target, old target where deletion/counting still depends on it, old/new canonical owners, and every endpoint whose endpoint-total cardinality, requiredness, or purge cleanup can change. Empty `factSelectors` means empty only after complete negative enumeration under the same directory/range and stamp; an I miss, hidden/unreadable endpoint, or partial source never creates an empty scope.

The literal arm of `node_ref_or_text` keeps its original boundary: it performs no target Node lookup, target incidence, target cardinality, inverse, graph, or target lifecycle. It remains governed only by containing-subject, source-cardinality, Entry/type/qualifier/requiredness, and author-source rules. Only the NodeRef arm enters endpoint proof. A symmetric NodeRef relation still has one canonical authored fact and that fact is counted only once for a given endpoint's incidence/cardinality. Endpoint-total cardinality at both ends, canonical-owner relocation, old/new source-owner CAS, and `explicit_remove_before_either_endpoint_purge` all keep the fixed-S contract. Trash changes only proved lifecycle/resolution and never removes a still-authored relation fact from incidence; restore never fabricates a fact from owner equality. Purge first explicitly removes every still-active symmetric incident fact under complete scope as required by the original contract.

A required masked/unprovable endpoint still cannot produce a successful readSet. A new/retargeted NodeRef target is live and domain-valid. Deleting an old fact may inspect a determinate tombstoned/not_found endpoint without inventing a Document and without re-admitting that old target as a new relation, but the true before fact, selector/source revision, old incidence, and operation-applicable cardinality/requiredness are still proved. A known incidence-stamp/source/entity-state change makes the original plan stale/conflict/reprepare. If complete incidence cannot be proved, the result is proof unavailable/unknown rather than a passed constraint.

`D4RelationCopyEffects/1` keeps its original shape and owner and never enters the D3 ref-slot union. D3 wire12 typed copy/fork/import consumes it only with `managed_atomic` plus complete D4 proof. Result relations re-prove every mapped/preserved endpoint, canonical owner, complete positive/negative incidence, requiredness, and the same final source. Raw byte-preserving copy never masquerades as typed fork. Every failure keeps complete rollback: the applicable D4 raw source/revisions/projections/allocations, empty write set, and no observable intermediate state; independent D3 allocation/reservation/burn/custody history remains governed by D3.

### 8.1 Authored order, counting, and typed copy

A same-owner/Field update replaces the selected occurrence in place. Relocation removes it from the old stream and appends at the destination Field tail across all same-namespace carriers, or the destination owner stream when that Field is absent, using the already bound carrier. Every unaffected occurrence, comment, raw byte and relative order is preserved. Canonical sorting applies to unordered bindings/derived inventories, not the entire authored stream; remove empty carriers only after all migrations finish.

Literal text, including empty text where the Field schema permits it, remains source-local and never enters Node-target cardinality. Directed target cardinality counts every active authored NodeRef-arm fact for the exact Field and target, including retained suspended, tombstoned, not-visible or unprovable target states; counting grants no hidden-state disclosure or successful unproved binding. Symmetric maximum comes from Field cardinality and counts unique incident NodeRef facts plus endpoint-local literal facts at each semantic endpoint. There is no stored second subject: the operation subject must be the original source owner or, for a symmetric NodeRef fact, the original other endpoint. Domain validation uses actual canonical owner and authored target, including legal asymmetric extension predicates. Sourceless states have no Entries or invented revision0.

Typed copy/fork/import maps internal typed Refs through the one D3 map and preserves an external Ref only under its original D3 rule. Same-Workspace closure-external live/trashed targets may be preserved; foreign, not-found or tombstoned targets do not acquire new admission. Cross-Workspace fork/transfer cannot retain an unmapped foreign target. Each included authored fact maps exactly once; inverses are never copied as source. Directed facts retain mapped source owner and literal facts retain mapped containing owner. Symmetric NodeRef facts reselect canonical owner only among the fresh mapped closure. If preservation requires writing an existing endpoint, the whole typed operation fails; it cannot expand closure, redraw IDs, drop the fact or silently turn it into text.

OccurrenceKey bytes may survive under the new owner scope and never enter identityMap. Owner relocation revalidates every owner-local Resource/Annotation/provenance root and never reparents, drops or repairs it implicitly. Existing mapped-owner authored order is preserved; incoming facts append in full source selector order `(sourceOwnerNodeRef,fieldId,occurrenceKey)`. Every mapped owner, even one emptied by migration, passes complete final requiredness/cardinality/domain checks; incoming inverse facts do not satisfy an authored required Field. Fresh ID order can make asymmetric endpoint predicates fail and such a failure rejects the whole operation. A lossy transformation requires a separate explicit preview. D4 rollback preserves its exact pre-state while any independent D3 allocation/reservation/burn/custody history follows the original D3 failure contract.

## 9. Calendar

CalendarPeriod, Temporal Range, and Calendar Event Semantics remain separate. A period is calendar-defined scope, a range is a date/instant range, and an event adds Facet semantics such as status/recurrence/participant/reminder outside the range. A View hit, cross-day presentation, or title pattern never auto-Assigns Event.

`calendar/recurrence-value` keeps homogeneous date/instant arms, count/until exclusivity, and the 256-item RDATE/EXDATE/exception limits. A derived occurrence is rebuildable and has no Node identity. In addition to the real current source, projection consumes complete D4-owned `registry` and `temporal_rules` proof: the complete same-Workspace `RegistrySnapshot/1`, `RegistryBinding/1`, required `RegistryEvolutionProof/1`, immutable `ValidatedCatalogContext`, and calendar/tzdb/rule contributions actually used; and the actual comparator, period rule, timezone/tzdb rule sets, `RecurrenceReadBinding/1`, explicit horizon, and finite coverage. Missing coverage, unproved rule provenance, or a missing segment never falls back to OS timezone, cache, or partial rows labeled complete.

The sole catalog `calendar/series-scope` v1 still uses key=`(series,periodKey,scope)`, scopeKinds=`node|workspace`, and multiplicity=`many|unique`. D4 consumes D6's closed `CalendarRange` arms for `DependencyKey/2.kind=calendar_scope`: `binding` proves one period Node's real `CalendarPeriodScopeBinding`, independent binding revision, actual source period/series, and current configuration; `series` proves complete periodKey membership for one series+scope, its `SeriesScopeConfiguration`, the complete negative range, and applicable control inbound; `period` proves every member of one complete `{series,periodKey,scope}` range, with unique/many coming only from the currently validated configuration; `scope_inbound` proves every period binding/configuration/control inbound referring to that scope. Configuration-absent, binding-absent, and a complete-empty period range each carry real version/stamp evidence and are never inferred from the current page, a path, provider cache, an I miss, or a target that merely “looks empty”.

Every Calendar positive/negative/empty range first passes same-cut `ObservationScope/2`, Policy, and disclosure before hidden period members, scope inbound, or conflict counts are read. Complete `calendar_scope` proof and the operation's source `SourceObservation/1`/`FileObjectBinding`/pins, Registry/rules, authorization, and temporal-rule stamps belong to the same committable cut. A `Frontier/2` causal prefix, provider “synced”, equal count/hash, or building/partial I never proves Calendar-scope completeness. The range-stamp epoch is the proof generation of that exact Calendar dependency key. A real gap, decoder change, or continuity loss invalidates the old stamp, while rebuilding I alone never re-signs it when the real P/M correctness evidence remains complete.

A local recurrence-value edit that does not invoke series uniqueness may still follow its real local contract as ordinary or genuinely pending. A series-scope `unique|many` strong mutation, however, completely proves current configuration, key, positive/negative membership, and temporal/Registry inputs. Only then may `unique` decide conflict; `many` likewise never skips key/config proof. copy/fork scope rebinding follows the D3 map and D6 current configuration to create a real target proof and never skips unique/many merely because the new target has no current index row.

An ordinary save may still record contract-permitted `semantic_pending(calendar)` when all local Calendar facts are valid but the complete Calendar range is not yet proved. That is not successful strong “create unique period/event” Action and does not satisfy D7 complete Query/Action. An invalid local Calendar value, D2 invalidity, deny, a touched unavailable Registry/rule, known conflict, or a proved complete-proof failure never washes into success through pending. Strong Calendar Actions remain strict paths and this contract expands no `observed_only` eligibility.

## 10. Positive catalog consumption

### 10.1 Tasks and People

`tasks/task` requires `tasks/status`; dependency is a directed many-valued relation whose subject and target both pass the source-bound explicit Task classification. The inverse is a projection, and UI module disable never removes semantics or requiredness. Task recurrence and Calendar recurrence do not implicitly create identities, execution, or complete-range proof.

`people/name` is repeatable: required nonempty text with `nfc-for-compare`, required role from `alias|former|legal|ordinary|transliteration`, optional language/script, and validity qualifiers. Its `atMostOnePreferred` applies to the name Field. A title may be initialized once from a chosen name but never remains automatically synchronized. Same-name Nodes remain distinct.

`people/phone`, `people/email`, `people/address`, and `people/website` use `people/labeled-text-value`: required exact nonempty text, optional label limited to `people/other|people/personal|people/work`, optional exact nonempty customLabel. Missing label means no author-supplied label, not other, unknown, empty, or unavailable. Locale labels never replace SemanticCodeId; note is separate.

`people/account` is repeatable. Its required identifier is the closed union of a preset external identifier restricted to `people/facebook|people/qq|people/wechat|people/x`, or custom `{serviceKey,identifier}` with both exact nonempty text. Optional usage uses the same three contact codes; optional customLabel is exact text and may be empty under this catalog. No member is inferred from a custom service's spelling; adding an external scheme globally does not expand the preset set. Equal values with distinct occurrenceKeys remain independent; service and identifier are not concatenated into a fabricated identity.

`people/life-event` retains independent, possibly conflicting assertions. The preset value requires eventCode from birth/death/employment-start/graduation/marriage, allows a nonempty customLabel and exact description; the custom value requires nonempty customLabel, optional exact description, and no preset code. `eventTime` is required, confidence/selection optional. There is no single preferred assertion constraint over this whole Field: preferred birth and death, or conflicting assertions, can coexist. Consumers must explicitly choose an assertion; latest/max/preferred guessing is forbidden. Anniversaries derive only from an explicitly selected assertion or verified chosen spouse/engagement validity.start and never duplicate source or create global event IDs. Year/month precision never fabricates a day; unrepresentable yearless input requires explicit loss. Editing a selected assertion differs from adding another and preserves all others. Later Calendar/D7/D10 execution needs its own explicit contract.

`people/nationality-state`, `people/legal-sex-state`, and `people/gender-identity-state` are separate repeatable nonempty exact-text assertions with validity/provenance. No global enum, inference between Fields, last-write-wins, or overwriting conflicting history is introduced. Absent validity means unknown, not permanent/current. Measurements retain required height/weight code plus quantity and required observedAt; dimensions and every historical observation remain explicit. `people/profession` has required exact text and optional organization Node/text context; that context does not create an engagement and the catalog does not add nonEmpty where it is absent.

`people/engagement` has Person subject and Node/text target. The Node arm accepts any ordinary Node without an Organizations Facet precondition. Position/department are optional exact text. Optional rank is the closed `{system,level}` object with both exact nonempty text; equal level labels in different systems never imply a shared numeric rank. Validity/status are qualifiers. Only the Person owns the authored fact; Organization roster is derived and is not double-written.

All nine other People relation Fields retain Node/text arms. `parent` is directed family with child inverse; `guardian` is directed general with ward inverse; `manager` and `mentor` are directed ranked with direct-report and mentee inverses. `spouse` and `sibling` are symmetric family, many-valued, with no monogamy or history-overwrite constraint. `family-related`, `social-related`, and `professional-relation` are symmetric general with required neutral relationship descriptor: preset respectively extended-relative; acquaintance/classmate/friend; colleague; or custom exact nonempty text. A descriptor never changes direction/Field. Unresolved people may remain literal text without creating Nodes. Derived colleagues may be projected from proved overlapping engagements without materializing O(N²) authored pairs; explicit colleague assertions remain separate. Avatar is at most one owner-local Resource.

### 10.2 Organizations and Library

Organizations retain seven repeatable scalar Fields: name with NFC comparison; namespaced status with validity; external identifier; classification with an Organizations namespace scheme and exact value; exact-text site; and address/contact using their own labeled-text alias. That alias has required exact text, optional Organizations label and optional exact customLabel; People nonEmpty restrictions must not be copied into it.

All twelve Organization relation Fields are Node-only, Organization→Organization. `parent` has maximum one and derived child; `governs` derives governed-by, `owns` derives owned-by; `allied-with` and `related` are symmetric. The seven other directed, many-valued Fields remain distinct: business-guided-by/business-guides, territorially-administered-by/territorially-administers, jointly-led-by/jointly-leads, supervised-by/supervises, subsidiary-of/has-subsidiary, brand-of/has-brand, member-of/has-member-organization. Multiple roles between the same pair are legal; business guidance, governance, mission/context text, and joint leadership do not infer one another. Person membership belongs to People engagement. Structural parent/order is independent: move never changes organizations/parent and a relation edit never moves a Node. Country/status/classification catalogs are not universal hardcoded enums. Rename preserves identity; merge/split requires explicit identity operations, allocating fresh identities where needed; external identifiers never become identity.

`library/work` is an ordinary bibliographic Node with optional at-most-one work-kind from article/book/dataset/report/standard; repeatable publication-state from accepted/draft/published; repeatable external identifiers; repeatable creator relation Work→Person|Organization with optional Library role; at-most-one venue Work→Work|Organization with hosts-work inverse; at-most-one version-of Work→Work with has-version inverse; and repeatable owner-local Resources. A journal may itself be a Work and is not automatically a publisher Organization. Draft→published may preserve one Work; an independently referenceable edition/version gets a fresh Work and explicit relation. My Works is a derived view, never a new Facet. DOI/ISBN/provider IDs are external data; a Citation is an occurrence in a Document and is not Work identity, a generic Reference entity, or a Resource identity.

### 10.3 Calendar period, range, event, and recurrence

The §16.6 closed shapes are authoritative. A period binds the exact verified calendar/version, timeZone/tzdbVersion, periodKind/rule, seriesKey, and periodKey. Required exact-text seriesKey may be empty. ISO-v1 period keys are `YYYY-MM-DD`, `YYYY-Www`, `YYYY-MM`, `YYYY-Qq`, `YYYY`; semantic validation includes real dates, actual ISO week 53, quarter/month bounds, and nonzero year. Contribution-backed TypedText tokens are preflighted before wrapper structure; the constructor gate must already identify the type. Independent calendar/tz failures aggregate at their first unproved token; periodKey faults use its own span.

`calendar/period-note` requires period; `calendar/range-note` requires range; `calendar/event` requires range-note and the same range, adding optional event-status, recurrence, participant and reminder intent. It preserves `union_variant_equal` between range and recurrence. A range alone never becomes Event semantics. Date recurrence uses homogeneous Gregorian `calendar/iso8601` v1 day values; instant recurrence uses one verified timezone/tzdb basis. With recurrence, the template range is bounded at both ends and start equals anchor on the same basis. Date duration is calendar days; instant duration is exact elapsed seconds, not a repeated civil end time. A replacement supplies a complete range; unrepresentable endpoints fail rather than clamp.

The anchor is the first base occurrence even when it does not match selectors; later candidates are at or after anchor and deduplicated. Daily phase uses day difference modulo interval; weekly phase uses the anchor's week bucket, default Monday weekStart and default anchor weekday. Monthly phase uses month difference, defaulting to anchor day only when both byMonthDay and byWeekday are absent. Yearly phase uses year difference; anchor month defaults only when all three month/day/weekday selectors are absent, and anchor day defaults when byMonthDay/byWeekday are absent. If a yearly day/weekday selector exists but byMonth is absent, all months participate. Selector dimensions intersect, values within a dimension union; monthly weekday means all matching weekdays. weekStart applies only to weekly frequency. Invalid civil dates are skipped, never clamped.

Instant recurrence preserves anchor wall-time and arbitrary fractional precision under the frozen tzdb. Gaps skip and folds choose the earlier UTC instant, while the explicitly authored anchor retains its exact chosen instant, including late-fold choice. There is no OS timezone or +86400-second civil shortcut. Count is 1..2147483647 and mutually exclusive with until, whose base-start upper bound is inclusive. Count applies to exact-time sorted, deduplicated base occurrences before exceptions. Civil traversal may not be UTC order: correct bounds use the ±86400-second offset envelope, never stop at the first civil candidate beyond until. Missing coverage or work budget fails without falsely reporting empty/partial success.

RDATE may be before anchor or beyond count/until and does not consume count. Base∪RDATE deduplicates by temporal equality, including equivalent offsets. Precedence is cancel > replace > EXDATE > template. An unhit EXDATE is retained; each exception's originalStart membership must be proved. Replacement title is nonempty; empty note explicitly clears; eventStatus is closed. Absent overrides inherit without rewriting author source. Occurrence-level participant/reminder overrides are not representable and require D9 explicit mapping/loss. set_exception changes only the selected exception; removing an already orphaned exception is legal. edit_series accounts for every old selector through explicit keep/remap/drop; dropping/remapping does not require old membership, but the complete after must validate, with no collision or display-label rebinding. All phases share one total work budget.

Public projection binds complete author dependencies: Entry wrappers' Field/key equal the raw Entry and duplicate Field/key pairs reject; unrelated namespaces remain exact and are not forcibly parsed. Projection requires Event+range+recurrence, not an unrelated global acceptance pass over every Event feature. Cache identity includes all source, Registry/rule/read bindings and horizon. Replacements moving into the horizon from either direction are considered, then final interval-overlap filtering applies and results order by originalStart. No partial rows escape failure. Derived instants use canonical UTC Z with arbitrary fraction and trailing fractional zeros removed; source bytes remain exact. Finite contiguous UTC rule segments and the ±86400 candidate envelope must prove complete coverage before declaring a gap. Date-only evaluation still consumes its empty-timezone rule context and work budget.

Series-scope unique/many consumes the verified policy, current independent binding/configuration revisions, and complete `{series,periodKey,scope}` range. A repeat of the same typed key returns only the correctly bound original result; title/path/current hits never determine scope or uniqueness. Many still requires configuration/proof. DerivedDuration uses the verified calendar day/month/year or table-index comparator, or exact elapsed seconds; open duration is available only after provider checks, and duration is never an authorable replacement fact. Calendar packs are Workspace/View context with pack identity/version/source/applicability; competing packs do not write a universal isHoliday fact. ICS UIDs remain foreign identifiers; VFREEBUSY/VTIMEZONE do not create Nodes, while VTODO/VJOURNAL require explicit mapping and preview.

## 11. diagnostics, permission, and source materialization

D4 diagnostics remain a closed envelope under §16.5 with stable `(sourceStart,rank,stableCode)` ordering. Permission and owner/schema/contribution availability precede structural interpretation. Within the authorized, interpretable scope, the unified validation path aggregates every independently provable fault in current source bytes; it does not stop at the first structural/type error. Independent sibling, note, qualifier, provenance, value, and later semantic faults are collected, while relation/Calendar/dependency semantics wait for their necessary structure, type, and contribution gates.

Unparseable raw input produces only its actual parse failure, without invented nested faults. A parsed envelope missing **one or more** required members produces one `invalid_entry_json` over the whole envelope. A missing member inside an already established nested object instead uses its semantic pointer and nearest valid object span. No branch fabricates a token position. Applicable D4 failure rolls back its proposed changes, with empty `writeSetOwners` and `readSet=null` under the closed outcome; it does not erase D3 allocation, reservation, burn, or custody history. Intermediate failed state is unobservable. Hidden Field/relation/Calendar data is never read first to choose a diagnostic. Unknown/unavailable is not empty, and does not prohibit unrelated legal raw reads, disjoint edits, or qualified ordinary pending saves.

Preflight remains outer D6/D3 visibility/permission -> Registry owner/binding/contribution -> D2 carrier/span -> strict JSON -> Field/Facet -> typed value/qualifier/provenance -> operation-applicable post-state. Under this P1 dependency contract, `ObservationScope/2` is the conservative authorized observation upper bound established before any D4 hidden-range/business evaluation; it is neither completeness proof nor write permission. Current `Policy/3`/principal disclosure first proves which Fields, relation incidence, Calendar membership, Registry/contributions, and other owner dependencies this operation may read. Registry unavailable never parses inner JSON first to leak schema, and a missing member never fabricates a span for a nonexistent token.

Complete proof and disclosure are independent. Even if durable D6/P holds a complete `DependencyProof/2`, a principal without current disclosure authority never reads hidden facts first and then decides that “this run was safe”. Two worlds that differ only in hidden incidence/member/unique conflict produce the same outer `not_visible`/non-disclosing result before business evaluation, with zero hidden business read/decision. Conversely, authorization permits the real read but does not itself prove completeness; completeness still comes from the owning domain's positive/negative enumeration, stamp, and current cut.

A Field-level or narrow-field path proves the static upper bound of possible read/write dependencies before hidden reads and then uses actual read tracking to prove that the real footprint stayed within that upper bound and remained independent of unauthorized scope. If either static upper bound or actual tracing cannot prove independence, the real complete operation scope is used. Core never reads hidden siblings/Fields/incidence first and narrows afterward. After materialization it revalidates actual MutationFootprint, source/Registry/range stamps, and current authorization. Every unselected author byte/comment/CRLF/unknown namespace remains byte-exact.

External strict UTF-8/D2 invalidity remains the D6 `external_invalid` repair/raw-read path and is never wrapped as D4 `semantic_pending` success. Local D4 typed invalidity, requiredness/cardinality failure, deny, or a touched `retained_unavailable` namespace likewise never disappears merely because a strong proof is missing. unknown/proof unavailable means the required complete range cannot be established; it never means the constraint passed.

The source `SourceObservation/1`, real production `SourceVersion/2`, `FileObjectBinding`, pins, Registry/rules, authorization, and range stamps consumed by one D4/D6 commit belong to one compatible current cut. Two replicas editing different Fields may still create a file-level conflict, and D6 `ConflictRecord`/source versions retain the real branches. A D4 semantic merge proposal never bypasses file CAS, real dependency revalidation, or rollback. A historical r5 receipt/readSet is never rewritten because r6 later obtains new disclosure/proof.

Final commit eligibility remains D2 eligible AND operation-applicable D4 gate AND D6 gate AND applicable D7 gate.

## 12. I, P, current state, and historical recovery

I caches derived Registry parse, typed projection, incidence, Calendar projection, and search candidates only. It owns no current author bytes, correctness-critical range continuity, durable decision, or execution responsibility. If real P/portable-metadata range evidence remains complete, rebuilding I repopulates cache without changing proof epoch or re-signing tokens. If real evidence or continuity is lost, old proof remains invalid; a new authorized complete enumeration establishes a new proof generation. Partial exploration is explicitly marked and cannot establish relation/unique/Calendar negative scope, D5 complete membership, D7 all_result/bulk, or Automation qualification.

Submit and recovery share one ordering: closed static decode; current minimum disclosure and authorization applicable to the **original profile**; CommitDomain/fence/portable trust/backend plus P-custody continuity; then same-key lookup using the record's actual version, complete original canonical request/fingerprint, and protocolOwner. Hidden business facts are not read to choose scope. Different owner/request at the same key is the original conflict, not a second decision. Current business proof and new-consumer gates are never moved before that lookup.

| Original state | Required continuation | Forbidden substitution |
|---|---|---|
| saved committed/rejected/terminal outcome | current delivery authorization for the original actual effect/mode or result-disclosure scope, then exact original receipt/error/effects bytes or original publication/outbox recovery | rechecking old before Observation, old Frontier, preview TTL or current new business/consumer proof; reinstalling after; reallocating H/revision/ChangeId; charging or consuming again |
| planned, unsealed | restore the same request, InputDescriptor/owner binding, candidate map, SourceRevisionPlan with original H basis/afterPin, before/after pins, reservation/write set, Notice, WriteProtection, owner version, attempts/budget, original TTL/clock and install state; current auth, dependency continuity and installation provenance decide same-plan continue/paused/conflict/recovery_unknown | new prepare, reselected Query/target/current page, resampled identity/H/revision, modified after, or invented ChangeId |
| unknown or outcome-unproved | preserve original pins, decision/install/external evidence, Approval/Money/claim/outbox/stop and no-duplicate-effect responsibility under the original owner | guessing Saved from current files/equal hash/I/empty control DB; blind retry with a new OperationId; resend, refund, quota/approval reset |
| truly unseen | apply the current source Observation, real production version plan, actual DependencyProof, semantic/profile and applicable consumer gates | importing historical success or partial evidence as new strong qualification |

Original last-reference/permanent pin promises remain effective. A genuinely expired unbound preparation or effect follows its original version's expiry rule and is never revived; expiry of preview is not authority to collect planned/saved/unknown responsibility. Revocation may hide historical delivery, not rewrite the saved decision; regained authorization permits original recovery only while continuity is proved. r5 pending and later r6 complete remain separate: r6 proof does not rewrite r5, old readSet, or old strong qualification. Equal bytes, ABA, new-device registration, I rebuild, or P loss cannot recreate missing P history. Undo/restore is a new explicit operation, never receipt replay rolling current source backward.

## 13. Downstream and version ownership

D5 owns Document Table and Node Collection domains; a D4 Field Value Occurrence never becomes Record identity. D7's actual coordinated owner set includes Query Algebra, Value/CEL, View, Narrow Field Qualification, Definition Transfer, Preview/Effects, Execution/Action, Prepared Action Binding, Scenario Dispositions, Lexicon/Registry, and Impact. It must consume the real source Observation, fourteen dependency keys/nine structure ranges, version basis, and complete result cut where applicable; a Prepared-only rewrite cannot replace those owners. D4 invents no PreparedActionBinding/3 or new D7 wire.

D8 owns Draft/base, selection/IME and editing presentation; labels are not FieldIds. D9 owns construction/import/export, Office/template and ICS explicit mappings/loss; it cannot infer code from localized labels, turn foreign identifiers into D3 identity, or silently omit an unrepresentable relation. D10 owns authenticated extension/provider intake, runtime/credentials and execution responsibility, including approval/recipient-target-payload/sourceOccurrenceKey/Money/unknown continuity. Portable Registry copying conveys none of those execution rights. Table/query/chart presentation introduces no implicit conversion and preserves absent optional, null where a real consumer permits it, empty, unavailable, and complete bounded range as separate cases.

The fixed candidate already had D3 wire12 and D4/D5 consumers for the earlier file-authority boundary. This P2 D4 candidate consumes P1's actual production-domain revision plan, protected revision-token binding, expanded DependencyProof, ContentCompletionProof/3, ConflictRecord/2 and historical recovery split. Existence of earlier native shapes is not acceptance of current producers. The D3 candidate is likewise an author candidate, and later D5/D7–D10 coordination remains required. D6 owns the native-descriptor/companion coordination and InstallationNotice wording: the D3 candidate exists without implied acceptance, and baseFrontier may contain historical sealed ChangeIds while the unsealed decision has no new ChangeId. D4 consumes those producer rules and invents no wire workaround.

Entry/1, TypeSpec, catalog v1, RelationReadContext/2, RelationReadBinding/2, RecurrenceReadContext/1, D4RelationCopyEffects/1 and D4SourceMaterializationEffects/1 retain their closed inner shapes. D3 v9–v11, D6 wire1, old tokens and actual PreparedActionBinding/1,/2 saved/planned/unknown records retain original decoders, bytes, retention, custody, authorization and recovery. A historical prototype with no deployment/record evidence is not automatically an active compatibility surface; an actual record's obligations are not cancelled for that reason.

After original-key lookup, only an unseen strong path that genuinely needs an unfinished consumer is owner-gated with no new success. Authorized ordinary `.adoc`/Resource reads, repair, Draft, qualified human whole-source saves, and independent local/offline operations remain available under their own qualification. Conversely, ordinary pending/partial/local success never proves complete Query, all_result, post-query, strong Action, Automation, or another owner's complete negative range.

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

### 14.1 Complete retained test and scenario obligations

The fifteen cases above remain required. The companion Impact §9 retains every effective fixed-S regression category, including exact diagnostic sequences and all 27 relation Fields, rather than treating representative examples as coverage. Current additions must exercise positive, negative, unavailable/unknown, recovery and original-version cases for each applicable contract:

- IDs and Registry: all shared byte/segment boundaries; every reserved owner; authenticated host Workspace; generation+digest substitution; all contribution variants and policy identity/digest/owner preflight; alias DAG, expanded limits, 61 Fields/7 Facets; predecessor, tombstone/migration and three-generation no-resurrection; immutable ValidatedCatalogContext and complete reference/owner/digest corpus.
- Entry/types: all strict JSON, framing, span, cardinality/key, qualifier, note, recursive provenance and closed TypeSpec cases; exact arbitrary-precision numbers and instants, UTC boundary years, all range open/equality/reversal cases, quantity dimensions, collection/member/depth/byte limits, constructor-before-contribution and availability-before-inner-parse, aggregate independent faults and no hidden disclosure.
- Facets and domains: declared/effective DAG/Task/Template, Create/Assign/Remove/Cleanup, actual-source ordering/materialization; every positive People name/contact/account/life-event/state/measurement/profession/engagement/rank/relation/avatar case; all Organization scalar/directed/symmetric/semantic-parent cases; all Library work/creator/venue/version/resource/Citation cases; absent labels, legal empty values and conflicting history remain explicit.
- Relations: the full stateful one-fact store and all 27 Fields, each direction/inverse/domain/cardinality/lifecycle; literal and Node transitions; canonical-owner flips under both fresh-ID sort orders; provenance owner-locality; source-preserving migration and complete incidence; all Context/Binding arms, masked/unprovable empty failure, stale source/entity/incidence token, requiredness even for emptied mapped owners, unwritable existing endpoint and whole-copy failure; no rollback of independent D3 burn history.
- Calendar: every §10.3 rule, recurrence phase/default/filter combinations, leap/invalid date, exact fraction and UTC ordering, explicit fold anchor/gap/lookahead coverage, count/until/RDATE/EXDATE/exception precedence and rebasing, replaced ranges entering/leaving both sides of horizon, shared work/output/cancellation budget with no partial rows, duration and pack behavior, many/unique concurrent scope/configuration changes, same-snapshot policy digest, cache deletion and byte-equivalent derivation, ICS foreign mapping/loss.
- Current producers: production D≠observer D, production epoch≠observer epoch; H across epoch and cross-domain return; proved empty H vs history loss; raw no-op/unchanged-source structure/deletion vs equal-byte external admission; SourceRevisionPlan frozen before/lastIssued/after/afterPin and MAX; stable tagged RevisionTokenBinding across retry/restart, invalidation across ABA/gaps; original D3 candidate map and Q two-pass materialization sharing one final revision; all fourteen keys/nine ranges, independent absent/empty stamps, current auth, I-only rebuild vs actual range-evidence loss, valid and invalid scope_dependencies extension chains.
- Save/recovery: ordinary strict and approved weak eligibility one condition at a time; all three U4 timing statements; final-check unseen C versus observed competition; B/N retention and later current C; invalid/deny/unavailable never pending; prepare/installed/sealed/unknown separation; before/after/third/unavailable installation classification; saved/planned/unseen lookup, original request/owner conflict, historical r5 delivery under revocation/reauthorization, original TTL/last-reference pins, publication failure after seal without reinstallation/charge/H advance, unknown Approval/Money/claim/outbox/stop continuity, real legacy decoders and no invented active prototypes.

The complete fixed `D4–D10–A2 Mandatory Scenario Inputs` remains a pressure intake. Its People/Organizations/Calendar/ICS/Library and source-preservation cases map to §§4–11 and Impact §9. Its Table/domain/computed values, Query/View/Search/chart and partial-vs-complete cases remain named D5/D7 gates; editing and presentation remain D8; Office/template/import/export and lossy round-trip remain D9; provider/plugin install/disable/unavailable, semantic-owner authentication, authorization, approval/recipient-target-payload, sourceOccurrenceKey, external unknown and Money remain D10 gates. The seven-plugin intake does not register new D4 Fields or activate packages. Shared identity, locale/name collision, no-second-authority and source/control/index separation apply across all of them.

The intake's 57 A2 acceptance rows, 125-row matrix and 302 propositions remain evidence obligations, not completed tests. Each applicable obligation needs its real executable-model, contract-check, integrity-check or contract-review evidence, with `independentSemanticReviewRequired=true` where required. Exact hashes prove artifact integrity only. Referenced external scenario/evidence artifacts not supplied in the fixed input remain explicit evidence gaps; their content is not guessed from a count or summary. No author self-check, historical count, documentation CI or fixture name closes them.

## 15. Candidate acceptance boundary

This is the P2 D4 coordinated author candidate across main text, terminology and impact, not independent acceptance or activation. Fixed-S snapshots, inputs and catalog remain immutable. All required owner afterimages and D10 upstream acceptance, fresh independent full joint review, and repair recheck remain gates. A2 self-contained reconstruction is already human-authorized and starts only after those gates; no duplicate approval request is required. A2 is followed by a separate fresh ordinary Chat Pro global final review and freeze/start package. None of this grants product implementation, dependency/CI changes, merge, release or deployment.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, P1=3, P2=8, eleven OPEN findings; U6/U7 and independent verification of coordinated producer changes remain tracked by their owners. This author does not close, reclassify, or independently accept any finding. Documentation checks prove only the named checks actually executed.

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

`occurrenceKey` selects duplicate values for patch/reorder/note only within current owner + FieldId + expected source revision, with its original wire and UUID semantics unchanged. On a new managed path under this P1 contract, expected source revision is the inner integer from the owner's real complete managed production `SourceVersion/2.revision`; production `CommitDomain`/`observationEpoch` stay in the outer complete version, while current qualification is separately proved by operation-domain `SourceObservation/1`/`SourceVersionRef/1`, `FileObjectBinding`, pins, and same-cut dependencies. A legitimately current managed before may come from a different production domain and remains its own complete before version, but its revision never donates to another after production domain's `H(D,E)`. Mutation still binds owner NodeRef, FieldId, occurrenceKey, and that complete current source evidence; an applicable `RevisionTokenBinding/2` is protected currentness/version evidence only and never replaces occurrenceKey or the integer selector.

An external source has no managed revision usable in this selector. A new structured mutation needing the selector waits for explicit managed admission/save to seal successfully. Raw/read/repair/Draft/ordinary paths remain independently available, and an unknown admission never guesses revision from `externalSequence`, hash, or text. `occurrenceKey` never enters D3 EntityRef/Locator/AnnotationTarget, never independently resolves/authorizes/queries across owners, and has no tombstone/restore. Reappearance after delete is a new source fact. Node move/rename may preserve Entry bytes; fresh-owner copy/import may preserve key bytes, while copying within the same owner+Field requires a fresh key. D3 identityMap never contains occurrenceKey and historical selector/saved bytes are not upgraded.

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

A successful existing-source Facet change still produces exactly one real managed after, but its owner revision is no longer independently calculated as “before revision + 1”. On a new P1 path it byte-equals the winning plan's frozen `SourceRevisionPlan/1.after.revision = H(afterProductionDomain,E)+1`. `H(D,E)` remains continuous across production `observationEpoch` changes in the same production domain and never resets; a foreign-before revision/epoch and external `externalSequence` never donate a value. H=0 is proved only when birth/registration, P continuity, and verified continuous sealed history in the after production domain completely prove that no managed version of that entity has ever sealed there; only then is the frozen after revision 1. Fresh D3 identity alone is not that proof. Checked increment at `MAX` fails and never wraps.

The winning planning CAS freezes the actual current before `SourceObservation/1` or proved-absent branch, same-domain `lastIssued` or proved-empty history, `SourceStamp/1`, exact `afterPin`, and every candidate map/pin/reservation and version-basis member applicable to the original plan. Before seal this decision has no `ChangeId` and no current managed `SourceVersion/2`. If another managed version for the same production-domain entity seals before this plan, the plan becomes stale/conflict/reprepare; Core never resamples H or edits the after revision in place. Only seal of a real source change combines the frozen stamp with the same decision's seal-allocated `ChangeId` into managed `SourceVersion/2`.

Remove still changes declared membership only. Cleanup still removes only request-listed, proved-unused occurrences. A true no-op with no source change advances neither H/revision nor fabricates a managed after. Any failure still returns byte-exact pre-state, empty write set, null successful read sets, and `intermediateStateObservable=false`. This group changes no Facet request/outcome wire, rollback, or read-set semantics, and historical saved/planned revision bytes continue under their original contract.

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

masked/unprovable still never produces a successful binding/readSet and expected binding remains item-for-item exact. D6-FA-r01 outer `InputDescriptor/2` binds complete `SourceVersion/2` and current `SourceObservation/1` for every source-bearing owner. For a managed owner, the corresponding inner integers in `RelationReadContext/2.nodeStates[].sourceRevision`, `RelationReadBinding/2.nodeRevisions[].sourceRevision`, and operation `sourceRevisions/expectedSourceRevisions` all represent that owner's same complete managed production `SourceVersion/2.revision`. The legitimate before production `CommitDomain` may differ from the current operation observer `CommitDomain`; such a foreign-production managed before remains a valid current before, but its revision/epoch never allocates another after production domain's H and never impersonates another owner/version.

Current `SourceObservation/1.observerDomain` equals the operation `CommitDomain` and binds `entityRef`, complete production sourceVersion, current `observationEpoch`, `FileObjectBinding`, evidence pins, control, Registry, and relation-incidence dependencies to one current proof cut. `SourceVersionRef/1` only projects that Observation. Protected source-revision currentness, where required, consumes only `RevisionTokenBinding/2` and its closed `RevisionTokenSource/2`; it replaces no D4 inner integer. In particular, `RelationReadContext/2.incidenceScopes[].revisionToken` and `RelationReadBinding/2.incidenceRevisions[].revisionToken` remain complete incidence-range version evidence, not `RevisionTokenBinding/2`, not a production source revision, and not interchangeable with either.

An external `SourceVersion/2` has no managed revision and therefore cannot directly construct those managed inner integers. Valid external source remains available for raw/read/repair/Draft/ordinary paths. A new relation mutation that truly requires managed inner revision first completes explicit authorized managed admission/save and then uses the real managed production revision after successful seal. Unknown admission/P seal remains unknown and never guesses sourceRevision from `externalSequence`, equal hash/text, or current file state. Relation/Binding closed shapes, permission gates, literal-arm semantics, symmetric-owner rules, and legacy saved bytes are not versioned by this bridge.

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

The common relation gate remains outer disclosure/auth -> closed context/request/binding -> source/sourceless coverage -> immutable cut/binding equality -> Registry/raw Entry/selector/revision -> complete old/new incidence -> unique proposed state -> subject/target domain+lifecycle+cardinality+Facet requiredness -> D6 Policy/version CAS -> author commit. The existing wire and meaning of `RelationReadContext/2`, `RelationReadBinding/2`, `incidenceScopes[].revisionToken`, and `incidenceRevisions[].revisionToken` do not change. That `revisionToken` versions the complete relation-incidence range for D4/D6; it is not a source `RevisionTokenBinding/2` and is not managed sourceRevision.

Complete incidence is proved independently for every actually required endpoint through `DependencyKey/2={kind:"relation_incidence",workspaceRef,fieldId,endpointNodeRef}`. The corresponding D4 enumeration completely reads the real source facts for that Field/endpoint at the same cut and reconstructs every fact as one unique `{ownerNodeRef,fieldId,occurrenceKey}` selector. It also proves the fact's source owner, canonical owner, source revision, entity lifecycle/stateToken, and applicable Registry/Facet classification. The positive side exactly covers all active authored incidence; the negative side proves that no source owner/fact was omitted. An empty scope likewise has a complete directory and current stamp and is never inferred from no index row, current page, a hidden endpoint, not-downloaded state, or a coarse Frontier.

For the before/proposed diff of a relation update, a newly added fact, owner relocation, retarget, or copy-created NodeRef fact proves its new endpoint live and domain-valid and obtains complete scopes for the subject, new target, new canonical owner, and every endpoint whose source/target endpoint-total cardinality can change. Deleting an old fact proves the true before, selector/source revision, determinate state of the old target, and complete old incidence. Deletion does not re-admit a tombstoned/not_found old target as a new relation, but the current subject domain, final requiredness, and affected cardinality still evaluate completely. An unchanged directed fact with tombstoned/not_found target is retained only under `retain_fact_explicit_cleanup` and never supports a new domain assertion. A required masked/unprovable endpoint fails. A literal arm establishes no target incidence or target lifecycle and never looks up a Node merely because text matches.

A symmetric NodeRef fact still has one canonical authored source. For each semantic endpoint D4 aggregates every active NodeRef fact of that Field plus locally authored literal facts for endpoint-total cardinality. One canonical NodeRef fact counts only once at a given endpoint; storage owner does not replace endpoint identity. Canonical-owner relocation remains atomic remove-old/add-new and preserves occurrenceKey plus all other Entry members/raw order. `endpointPurgePolicy=explicit_remove_before_either_endpoint_purge` is unchanged: before either endpoint is purged, every active symmetric incident fact is explicitly removed in the same strong transaction. Trash/suspended/hidden never means the fact disappeared. Independent Trash/restore only changes real lifecycle/resolution and never adds or removes incidence by owner equality.

Facet Assign/Remove/Cleanup and relation occurrence update continue to share the complete proposed-relation-state semantics. A membership change reads every applicable relation-Field scope for the affected owner. A relation change reads old/new owners and NodeRef targets for the corresponding Field but does not recursively expand into every Field of every untouched endpoint. Only ranges required by the real constraint algebra enter proof. Each scope stamp shares the cut with source Observation, Registry, and authorization. A known stamp/source/entity change makes the original plan stale/conflict/reprepare; unknown or missing complete proof returns unavailable/owner gate rather than acceptance.

Only after every required endpoint and range passes may Core form the unique proposed state and write set. Any structure, version, source, binding, range, domain/lifecycle/cardinality/requiredness, permission, CAS, or injected precommit failure preserves the byte-exact pre-state, raw author source, revisions, projections, and allocations, with an empty write set, null successful readSet, and `intermediateStateObservable=false`. This D4 pre-state rollback never erases independently durable D3 allocation/reservation/burn/custody history. Strong relation mutation and typed copy/fork/import remain `strict` and never downgrade through local pending or partial I.

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

`sourceBytes` remains canonical no-padding base64url exact UTF-8 bytes and `before absent` remains limited to a fresh Node in the same D3 receipt. For every source-bearing owner, a non-absent `before.sourceRevision` corresponds to the actual complete managed production before `SourceVersion/2.revision` selected by the original current SourceObservation. The after basis has two explicit branches:

- When this operation actually produces a new managed after for that owner, `after.sourceRevision` byte-equals the same winning plan's frozen `SourceRevisionPlan/1.after.revision`. This is neither an independent D4 “before+1” allocator nor a hard-coded fresh 1. The plan freezes after=1 only when complete sealed empty history in the after production domain proves H=0. A foreign-before revision/epoch and external `externalSequence` never donate a value; before seal this decision has neither a `ChangeId` nor a newly sealed managed after.
- When an existing C-carrier result owner retains byte-equal source, including a raw-no-op existing pair in a mixed operation that changes or creates another owner, its before and after retain the same complete already sealed managed production SourceVersion and exact source bytes. Its after inner revision equals its before inner revision under that original Observation, even when the production domain differs from the operation domain. This owner has no new SourceRevisionPlan, proposed source stamp, H increment or source version; another owner's plan or the current domain's H cannot supply its revision. A source-unchanged portable structure/lifecycle effect uses the same retained source basis. External input never acquires a fictitious managed inner revision through this unchanged branch.

`owners` still covers every C-carrier result owner and every actually modified existing source container. `entries` remains uniquely sorted by source subject key, FieldId, OccurrenceKey, and `beforeRawEntrySource` is null only for an explicit fresh initial Entry. All D4 source/materialization and relation-copy effects consume the same original D3 private candidate map, allocation/burn result and one final source: changed/fresh managed after uses that owner's same `SourceRevisionPlan/1`/`afterPin`/revision binding, while an unchanged existing owner uses the retained complete production version/current-observation basis and exact pins above. Coverage still includes every unchanged C result owner. They never redraw IDs, expand closure, manufacture a second source, or resample H in the effect layer.

`D4SourceMaterializationEffects/1` and `D4RelationCopyEffects/1` are independently recomputed from the real complete before/after source and agree item-for-item with the same D3 decision's receipt/effects, complete typed DefinitionTransfer slots, Result/9 C/Q partition, and Q two-pass final spans. Q, C, and D4 transformation bind one final source/revision basis; any recomputation mismatch rejects the original operation rather than splitting two locally correct results into separate commits. The effects remain saved with the same original decision as the D3 request/candidate-map/receipt and D6 decision and never become a second author source.

A true raw no-op or source-unchanged structure/lifecycle effect never fabricates a new managed source revision/H increment in a D4 effect. Whole-source deletion has D6 portable after absent and no after `SourceRevisionPlan/1`; D4 never invents a “deleted source revision” through this /1 effect. The existing `D4SourceMaterializationEffects/1` and `D4RelationCopyEffects/1` wire versions remain unchanged and historical effect/saved bytes retain their original decoders.

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

A no-op accept still changes neither source/revision nor cache. A real recurrence source change still produces exactly one managed after for the series owner, but on a new P1 path its after revision is the winning plan's frozen `SourceRevisionPlan/1.after.revision = H(afterProductionDomain,E)+1`, never an additional D4 “+1” applied to the current integer. H does not reset across production `observationEpoch` in the same production domain; a foreign-before revision/epoch and external `externalSequence` never donate a value. The frozen after is 1 only when complete after-domain sealed-history proof establishes H=0, and overflow at `MAX` fails.

`expectedOwnerRevision`, recurrence selector, and projection/readSet inner revision members keep their existing shapes and bind the real complete managed production before/after revisions on a new managed request; outer production domain/epoch and current `SourceObservation/1` qualification remain separate. If another managed version for the same production-domain entity seals before this plan, the original recurrence plan is stale/conflict/reprepare and never resamples H/revision in place. Any failure still returns complete pre-state, empty write owners, null readSet, and empty invalidations, and failure injection remains harness-only. Historical saved recurrence bytes are never retroactively rewritten by the new allocator.

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

The policy continues to resolve only through `RegistryBinding + policyId + policyVersion + policySchemaDigest`. `series` is reconstructed completely from the author `calendar/period` value that already passed the common D4 Entry validator, and periodKey is canonically decoded by the verified period-rule contribution in the same snapshot. Path, folder, View row, title, locale, current page, and caller reports never enter the key.

Every strong unique/many operation consumes the real `DependencyKey/2.kind=calendar_scope`. When checking one `{series,periodKey,scope}`, the corresponding `CalendarRange.period` proof completely enumerates every current member and the negative range for that key and binds `SeriesScopeConfiguration`, configuration revision, scope binding, `RegistryBinding`/policy, and control inbound. If the operation depends on the whole series, a `CalendarRange.series` proof additionally establishes all periodKey membership and the negative range for that series+scope. Creating/changing a period-scope binding uses `binding`; scope deletion/migration uses `scope_inbound` where the real operation requires it. Every empty range carries its own complete stamp and is never inferred from an I miss or a target that merely appears empty.

The same operation also consumes `registry` and `temporal_rules` proof: the complete restored `RegistrySnapshot/1`/`RegistryEvolutionProof/1`/`ValidatedCatalogContext`, the real policy contribution, period rule, calendar comparator, tzdb/timezone rule sets, `RecurrenceReadBinding/1`, and finite horizon coverage used by this operation belong to one current cut. A missing rule segment, unknown decoder, unavailable configuration/binding, or continuity gap makes the strong result unavailable/owner-gated; it never falls back to device timezone or treats an unproved range as empty.

`unique` returns `calendar_series_scope_conflict` with zero write only when the complete range proves an existing/concurrent second distinct NodeRef at the same key, and succeeds only when the complete range proves no second member. `many` allows distinct NodeRefs but still proves the same typed key, policy/config binding, and range currentness; it never means “no proof required”. Retry byte-reuses the same key and current revision/stamp. Any key replacement, configuration revision, or relevant dependency change follows the original owner's stale/conflict/reprepare path. Unknown proof never means conflict absent.

An ordinary local Calendar edit that does not depend on complete series scope may continue under its real local/pending contract. Strong unique/many, Calendar Action, and typed copy/fork target-scope validation remain `strict` and never acquire strong success through `semantic_pending`, partial I, or a missing D7 consumer. A historical saved r5 result is never rewritten by a new current r6 proof/stamp.

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

All D4 inner wire versions above remain unchanged. A new request uses D6 InputDescriptor/2 to bind every source-bearing owner to complete SourceVersion/2 and consumes the current SourceObservation/1 for observation qualification. SourceVersion.commitDomain remains the production domain and does not have to equal the operation observation domain. CommitDomain, Frontier/2, Registry/Policy/control revisions, current fileObjectBinding, evidencePins, and real source pins form the same current proof cut. SourceVersionRef/1 uses sourceToken tagged as `d6_source_observation/1` to select the protected Observation; bare sourceRevision, I cache, equal hash, production version, and old stateToken never carry proof across invalidated replica or observation continuity.

ordinary save keeps save semantics separate from `WriteProtection` choice. A normal ordinary save may remain `strict`; `observed_only` is an explicit weaker profile that may be selected only by a trusted `interactive_source_save` for one existing live Document, with ordinary replica-local scope, complete source read and replacement qualification, no applicable body/Field/Node-control deny, an author-source write set limited to that Document or empty, and no identity, parent, order, lifecycle, shared-policy, Registry, Calendar-scope, or other-entity mutation. DraftBase must equal the complete selected current `SourceObservation` before planning starts. Non-interactive flows, complete or strong Action, structured bulk, collection mutation or promotion, automation, server checkpoint, Approval, or Money-related execution never use `observed_only`.

Before any ordinary save succeeds, every touched local `Entry`, `Facet`, `Type`, cardinality, and requiredness rule must pass. An invalid local fact, a touched `retained_unavailable` namespace, `D2 external_invalid`, physical invalid source state, or missing real read/write qualification remains a failure or repair path and is never converted into success through `semantic_pending` or weak protection. `semantic_pending` only records unproved `relation|unique|calendar|inbound|cross_object_type` obligations; it never upgrades a strong Action without complete proof.

`observed_only` durably retains the observed before state B and the user's intended input N only. It does not guarantee that an unobserved external competing write C does not exist or that its bytes are preserved from being overwritten when N is installed from the observed B state. If a later unobserved C write replaces the current file after N is installed, that affects the current file state only; the durable observed B state and user's intended input N retention are not discarded. Local typed facts and unselected bytes prove only the bound B→N transformation; they do not prove all unobserved intermediate source states and must not be used to intentionally omit required observation. Observed competition, stale Base, or continuity gaps follow the existing conflict/reprepare path; unknown installation remains `recovery_unknown`.

A trusted human may explicitly choose the weak profile for an interactive ordinary save before planning starts even when the ordinary directory does not provide strict capability; `writeProtection` is frozen from the start of planning. Known Base conflict, authorization failure, durability failure, strict-plan failure, or any strong obligation failure never falls back to `observed_only` and never broadens the weak scope.

Preparation only retains the durable proposal, read-before evidence, and pins required for the later operation; it is not a saved, installed, or sealed result. Installed, sealed, and unknown D6 states remain distinct. `durable_observed_only` is not strict reliable save and is not qualification for complete-set Query or Action. Existing D4 source-layer, current dependency, Registry, relation, and source observation obligations remain unchanged.

Historical saved/planned/unknown recovery uses the complete ordering in §12 and version boundaries in §13. Only truly unseen requests enter new current-business/consumer gates. Later current validation never rewrites an old receipt, pins, or recovery duty. D3 structure/Trash, D5 structured cell/row/column/reorder, bulk/collection/promotion, D4 strong Relation/Facet/Calendar, D7 Action/Automation, server checkpoint, Approval and Money all remain strict. Missing an unrelated strong consumer never blocks a separately qualified ordinary read/Draft/human whole-source save.

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

For current new-decision candidates, the historical fixed-S section 15 D3 v11 candidate-map wording is now carried by the delivered-but-not-independently-accepted/activated/implemented P2 D3 wire12 author candidate together with the applicable P1 D6 `InputDescriptor/2`, `SourceRevisionPlan/1` and revision-token producers (`PreparedIntent/2` only on a path whose actual preparation contract requires it). This synchronizes the actual candidate contract now authored rather than treating historical H2 shorthand as current law, and delivery of the D3 author candidate bypasses neither D4/D5/D7 coordination nor fresh joint acceptance. All original preconditions remain:

- fresh Node/Resource/Annotation candidates come only from the same original D3 stage12 private candidate map/intent/plan, never from caller data, payload UUID, hash, or a second validator;
- D4 initial Entries, Template results, copy/fork/import typed refs, and every known D4 typed root are validated against the same real proposed complete final source before the winning planning CAS; every fresh/mapped identity, owner materialization, and allocation/burn decision uses that one candidate map;
- every source-bearing owner that will produce a managed after uses the same original plan's frozen `SourceRevisionPlan/1`: actual current before `SourceObservation/1` or proved absence, same-production-domain `lastIssued` or proved-empty history, `SourceStamp/1`, exact `afterPin`, candidate map, pins/reservations, and version basis freeze together. `after.revision` comes only from `H(afterProductionDomain,E)+1`; production `observationEpoch` never resets H, foreign-before revision/epoch and external `externalSequence` never donate a value, and missing/gapped history is never empty;
- after revision is 1 only when birth/registration, P continuity, and verified continuous sealed history in the after production domain completely prove H=0. Fresh identity itself never hard-codes revision=1, existing/foreign before never implies before+1, and checked H at `MAX` fails;
- after the planning CAS wins, candidate map, allocation/burn, `SourceRevisionPlan/1`, afterPin, revision-token binding, source payload, lifecycle, and current Registry/Policy remain one frozen original-decision proof. Before seal there is no `ChangeId` for this decision, current managed `SourceVersion/2`, or public fresh Ref. A same-domain concurrent seal makes the original plan stale/conflict/reprepare and never resamples H/revision/identity/token in place;
- protected revision evidence uses only the real `RevisionTokenBinding/2`, whose `source` is the closed `RevisionTokenSource/2`. The token instance, D4 inner integer, production `observationEpoch`, and observer `observationEpoch` are distinct roles and never encode or compare as one revision;
- symmetric relation source assembly still recomputes canonical owner from result endpoints, and D4 conformance never redraws a committed ID merely to satisfy a constraint, expands closure, drops provenance, or changes Facet;
- complete typed DefinitionTransfer, the Result/9 C/Q partition, every typed slot, and Q bijection use the same candidate map, the same frozen `SourceRevisionPlan/1.after`/revision binding, and the same final source. Q two-pass materialization obtains stable spans in that one source; C, Q, and D4-authored transformations are mutually compatible or the entire operation fails, with no iterative position guessing and no second commit;
- `D4SourceMaterializationEffects/1` and `D4RelationCopyEffects/1` are independently recomputed from that same real before/final-after source and agree item-for-item with D3 receipt/effects, DefinitionTransfer/C/Q, owner mapping, and frozen revision basis.

The D4 semantic pre-state for fresh create remains that same bound complete proposed result source, but prospective `sourceRevision` on a new P1 path is no longer hard-coded to 1. It byte-equals that owner's frozen `SourceRevisionPlan/1.after.revision`; only a fresh managed source with completely proved after-domain H=0 receives 1. Success still materializes/commits once, never appends twice, and never increments revision again in the D4 effect layer. An ordinary existing owner never claims fresh origin to bypass CAS. True no-op/source-unchanged branches never advance H, and source deletion with after absent has no after `SourceRevisionPlan/1`. When P/seal outcome is unknown, Core never guesses committed/revision from source bytes, C/Q, effect projection, or equal hash and only recovers the original frozen plan. Existing D3/D4/D6 saved/planned/legacy bytes retain their original decoder, integer, and recovery contracts and are never retroactively upgraded by this group.

### 17.4 D6 ObservationScope and disclosure

A complete dependency read set does not imply that the principal may observe constraint results. Before any D4 value/range/business evaluation, D6 supplies a conservative `ObservationScope/2` compatible with the current `CommitDomain`, complete source `SourceObservation/1`, Registry, Policy, and planned scope, and establishes protected read dependencies through the real `DependencyProof/2`/Policy. Those dependencies include negative ranges that may truly be empty. `ObservationScope/2` is only an authorized observation upper bound; it is neither completeness proof, write permission, nor a declaration that every dependency exists.

Without disclosure authority, two worlds that differ only in a hidden Field, relation incidence, Calendar membership, Registry contribution, sibling/member, or negative range produce the same outer `not_visible`/non-disclosing result before D4 business evaluation, with zero hidden business read/decision. Core never scans a hidden range first and then chooses a narrower profile, and the presence of an old proof in P never bypasses current disclosure. Only after disclosure succeeds do real unique/cardinality/incoming/cross-Field/Registry/Calendar checks run. Authorization alone still does not prove completeness.

D4 adds no permission token and never trusts a client statement that a range is complete. A local/narrow Field path is usable only when both conditions hold: the static dependency upper bound proves no hidden/external scope can affect it, and actual read tracking proves that this execution stayed within that upper bound. If either condition fails, the real complete operation scope is used. This is the same rule as Diagnostic authority masking.

New D4 strong proof in this candidate consumes only the fourteen closed D6 `DependencyProof/2` key kinds: `source|lifecycle|placement_range|ref_inbound|relation_incidence|calendar_scope|registry|temporal_rules|authorization|foreign_binding|query_scan|replica_registry|conflict_record|execution_resource`. `placement_range` accepts only the nine closed `StructureRange` variants `live_children|trash_children|trash_roots|ancestor_chain|subtree|owner_resources|owner_annotations|reply_closure|restore_membership`. D4 never derives another key from free JSON, a generic owner callback, or `Frontier/2`. D4 owns only the concrete semantic enumeration of `relation_incidence`, `calendar_scope`, `registry`, and `temporal_rules`; every other key algorithm remains with its real owner.

A proof-stamp `epoch` denotes only the continuity generation of that exact DependencyKey. It is distinct from production `SourceVersion/2.observationEpoch` and current `SourceObservation/1.observationEpoch`. Within one gap-free proof generation, any real change that can affect source, membership, order, Registry/rule, authorization, visibility, or range contents first invalidates the old proof; only after complete current revalidation may revision checked-increment and a new current stamp publish. A watcher/journal gap, owner decoder/rule-version change, actual loss of the correctness directory, or unproved event continuity requires a new epoch. Equal hash/count/text never revives the old stamp. If P/portable-metadata correctness boundary and continuous event chain remain complete, deleting/rebuilding I rebuilds candidate cache only and changes no stamp. If those real facts are lost, only a new complete current-authorized enumeration establishes a new epoch.

Empty and non-empty proof use the same standard: establish state/content/metadata disclosure, traverse every applicable directory/shard/owner range/source, and perform final revalidation under a consistent barrier or gap-free stream from a known complete baseline. A placeholder, missing shard, unknown decoder, I/O failure, hidden unauthorized member, partial/building I, no-hit, provider “synced”, equal hash/count, or a causal Frontier prefix never yields empty success. P/portable M retains key/stamp/complete boundary/evidence pins/continuity; I is not the proof owner.

Within one committable cut, the `SourceObservation/1`, production `SourceVersion/2`, `FileObjectBinding`, source/evidence pins, control, Registry/rules, authorization, and every D4 range stamp actually used by D4 are mutually compatible. A known dependency/stamp change follows the original owner's stale/conflict/reprepare path. Unknown continuity/completeness becomes `proof_unavailable`, owner-specific unavailable, or recovery state and is never interpreted as a passed constraint. Missing an unrelated strong proof does not permanently block an otherwise qualified ordinary/local operation that does not depend on it, but that operation gains no complete Query/Action qualification from this exception.

### 17.5 Calendar scope control

The author value of `calendar/period` still contains only period + seriesKey and gains no scope Field. The scope in a complete `CalendarSeriesScopePolicy/1` key comes only from a D6 portable/control scope binding maintained at the same current cut, never from path, folder, View row, title, current page, or ambient locale. D4 continues to own the typed semantics of series/period/scope keys. D6 continues to own persistence, version stamps, creation, explicit migration/deletion, control inbound, and CAS for scope binding/configuration. This section creates no second D6 schema.

D4 consumes the real `DependencyKey/2.kind=calendar_scope` for that control surface. `CalendarRange.binding(nodeRef)` proves the period Node's real scope binding, independent binding revision, author period/series interpretation, and current configuration. `series(seriesScope)` proves complete series+scope membership, `SeriesScopeConfiguration`, negative range, and control inbound. `period(seriesScope,periodKey)` proves the complete `{series,periodKey,scope}` member set. `scope_inbound(scope)` proves every binding/configuration/control inbound referring to that scope. An operation needing several facts lists several real keys; it never substitutes a free “calendar closure” or coarse Frontier.

Policy/configuration agrees with the same-cut `registry` proof: `RegistryBinding/1`, policyId, policyVersion, policySchemaDigest, and the complete restored Registry context all match. periodKey/series remain reconstructed from the real validated author Entry. Calendar temporal interpretation is simultaneously constrained by `temporal_rules` proof, so the actual comparator, period rule, tzdb/timezone revisions, `RecurrenceReadBinding/1`, and finite horizon coverage are never supplied by a device default.

A new period that needs workspace scope has explicit current configuration; configuration-absent is not a default unique/many choice. copy/fork re-establishes legal scope proof using the D3 identity map and target Workspace current binding. When a node scope maps to a fresh Node, the target configuration and complete negative/positive range are still proved. A new target, empty current index, non-colliding path, or current-page miss never skips unique/many. Strong `unique|many` results continue to depend on complete proof and remain strict. An ordinary local Calendar save that does not depend on those strong ranges continues under its own local/pending contract and is not permanently disabled merely because a D7 complete consumer is missing.

### 17.6 D7 consumer boundary

D4 C carrier, Entry/Facet/Relation/Registry/Recurrence/DerivedDuration remain D4-owned; D7 owns its Query/Value/View/preparation/Action/effects consumption. The complete owner set and operation-applicable new-decision gate are in §13. Actual historical /1,/2 preparation and saved/planned/unknown records recover under §12 and their original contracts; current new-owner availability never retrospectively cancels that responsibility. An unseen strong path needing an unfinished D7 consumer returns the existing owner_update_required/proof_unavailable without a new managed success. Qualified ordinary file/Resource reading, Draft, human whole-source saves and independent local operations continue. No historical binding, I cache, current page, semantic_pending, or SourceStamp before seal fabricates complete new-version proof.
