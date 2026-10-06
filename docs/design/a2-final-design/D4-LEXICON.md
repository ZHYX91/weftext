---
source_language: zh-CN
translation_of: D4-LEXICON.zh-CN.md
translation_status: synced
---

[简体中文](D4-LEXICON.zh-CN.md)

Source document ID: 362b6465-71ed-4176-9e74-6207fe3fab24.

# D4 Terminology and Naming Lexicon — D6-FA-r01

Candidate status: D6-FA-r01; P2 coordinated author candidate; not accepted, not activated, not implemented. All 26 fixed-S D4 concepts preserve stable conceptId, owner, existing public wire/code/UI/locale names, and firstFreeze. This batch deletes, renames, or transfers none of them.

## 0. A2 current imported-version terminology

Current outer imported names are `InputDescriptor/3`, `DependencyProof/3`, `DependencyKey/3`, `PreparedIntent/3`, `InstallationNotice/3`, `ContentCompletionProof/4`, `ChangeRecord/1`, and D3 `wire13`. D4 does not rename or alias preserved inner `Entry/1`, relation/recurrence types, Field Value Occurrence, Namespace Owner, Facet, Calendar Period, Temporal Range, or Bibliographic Work concepts. Historical outer names remain decoder-qualified recovery terms only.

## 1. Control rules

D4 owns only schema/value/relation/Calendar/Library terminology. D3 Node/Entity/Reference/Owner/Authority, D6 CommitDomain/SourceVersion/SemanticState/ConflictRecord/Execution Responsibility, and D7 Query/Action complete cut remain imported concepts and are not re-owned by D4.

A public name cannot belong to multiple concepts. A localized display label is not a FieldId or SemanticCodeId. Semantic non-alias checks use an expected role and never scan user content, historical evidence, or third-party formats.

## 2. Complete controlled concepts

| conceptId | canonical names | owner/layer | exact meaning and exclusion | owned wire/code names | UI/locale | firstFreeze |
|---|---|---|---|---|---|---|
| `weftext.term.semantic-namespace` | 语义命名空间 / Semantic Namespace | D4/schema | exact owner scope shared by FacetId, FieldId, SemanticCodeId; not package/provider display name or carrier identity | `SemanticNamespaceId`, `semanticNamespaceId` | `schema.namespace` / 语义命名空间; qualified namespace | D4 |
| `weftext.term.namespace-owner` | 命名空间所有者 / Namespace Owner | D4/schema registry | principal authorized to define one Semantic Namespace; not D3 Owner, installer, consumer, or authority | `NamespaceOwner`, `namespaceOwner` | `schema.namespace_owner` / 命名空间所有者 | D4 |
| `weftext.term.field` | 字段 / Field | D4/schema | FieldId-addressed schema slot with type/shape/cardinality/constraints; not header attribute, UI property, occurrence, or metadata | `FieldDefinition`, `fieldDefinition` | `schema.field` / 字段 | D4 |
| `weftext.term.field-id` | 字段标识 / Field ID | D4/schema | canonical `namespace/local-field-path` semantic identifier; not display label, locale key, source range, or UUID | `FieldId`, `fieldId` | `schema.field_id` / 字段标识 | D4 |
| `weftext.term.field-value-occurrence` | 字段值出现项 / Field Value Occurrence | D4/content semantics | one authored Field value fact in D4 Entry/1, still owned by Document; not Entity, Record, or durable occurrence subtype | `FieldValueOccurrence`, `fieldValueOccurrence` | `field.entry` / 字段项 | D4 |
| `weftext.term.occurrence-key` | 出现项键 / Occurrence Key | D4/content selector | owner+FieldId+expected-revision scoped patch selector; not EntityRef, Locator, RecordRef, OperationId, or durable identity | `OccurrenceKey`, `occurrenceKey` | `field.occurrence_key` / 出现项键 | D4 |
| `weftext.term.typed-value` | 类型化值 / Typed Value | D4/value | author value interpreted by a closed D4 constructor and Field schema; not untyped JSON, display string, or provider blob | `TypedValue`, `typedValue` | `field.typed_value` / 类型化值 | D4 |
| `weftext.term.value-type` | 值类型 / Value Type | D4/value schema | closed constructor defining legal Typed Value shape/comparison/limits; not Facet, Node kind, or runtime class | `ValueType`, `valueType` | `schema.value_type` / 值类型 | D4 |
| `weftext.term.field-shape` | 字段语义形态 / Field Semantic Shape | D4/schema | one of `fact|event_assertion|observation|relation`; not JSON shape, widget, or Facet | `FieldSemanticShape`, `fieldSemanticShape` | `schema.field_shape` / 字段语义形态 | D4 |
| `weftext.term.facet-schema` | Facet 模式 / Facet Schema | D4/schema | persistent closed semantic contract for FacetId; not D2 AsciiDoc Profile, package manifest, Template, or Preset | `FacetSchema`, `facetSchema` | `facet.schema` / Facet 模式 | D4 |
| `weftext.term.declared-facet-set` | 声明 Facet 集 / Declared Facet Set | D4/D2 projection | exact source-declared FacetId set; not effective closure or installed provider list | `DeclaredFacetSet`, `declaredFacetSet` | `facet.declared_set` / 声明 Facet | D4 |
| `weftext.term.effective-facet-closure` | 有效 Facet 闭包 / Effective Facet Closure | D4/derived | rebuildable requires closure of declared set; not author source and never written back | `EffectiveFacetClosure`, `effectiveFacetClosure` | `facet.effective_closure` / 有效 Facet | D4 |
| `weftext.term.inline-field-note` | 字段项内嵌备注 / Inline Field Note | D4/content | optional plain text stored with one Field occurrence; not Annotation, typed qualifier, or provenance | source/API `note`; code `InlineFieldNote` | `field.note` / 备注 | D4 |
| `weftext.term.semantic-code` | 语义码 / Semantic Code | D4/value | locale-independent stable code; cross-field/contribution form uses SemanticCodeId; not display/custom text or ordinal | `SemanticCodeId`, `semanticCodeId` | `field.semantic_code` / 语义码 | D4 |
| `weftext.term.validity-interval` | 有效期区间 / Validity Interval | D4/value qualifier | date/instant range where an author fact applies; not recordedAt, operation time, or lifecycle | `ValidityInterval`, `validity` | `field.validity` / 有效期 | D4 |
| `weftext.term.event-assertion` | 事件断言 / Event Assertion | D4/fact | potentially conflicting author assertion about a domain event with precision/provenance/confidence; not Calendar Event Node/Facet or operation event | `EventAssertion`, shape `event_assertion` | `field.event_assertion` / 事件断言 | D4 |
| `weftext.term.observation` | 观测项 / Observation | D4/fact | observation with observedAt and optional unit/provenance/confidence; not current state, audit event, or derived metric | `ObservationValue`, shape `observation` | `field.observation` / 观测 | D4 |
| `weftext.term.relation-field` | 关系字段 / Relation Field | D4/relation schema | relation-shaped Field with direction/inverse/cardinality/lifecycle/delete/projection; not RelationType entity, Node Link, or Reference slot | `RelationFieldDefinition`, member `relation` | `relation.field` / 关系字段 | D4 |
| `weftext.term.inverse-relation-projection` | 反向关系投影 / Inverse Relation Projection | D4/derived | rebuildable inverse query/View from one authored relation fact; not a second author fact | `InverseRelationProjection`, `inverseCode` | `relation.inverse` / 反向关系 | D4 |
| `weftext.term.symmetric-relation-owner` | 对称关系规范事实端 / Symmetric Relation Canonical Owner | D4/relation | unique symmetric-fact owner selected from two NodeRef canonical encodings; not replacement for D3 Owner or an auth bypass | `SymmetricRelationCanonicalOwner` | `relation.canonical_owner` / canonical owner | D4 |
| `weftext.term.retained-unavailable` | 保留但类型语义不可用 / Retained but Typed-Unavailable | D4/availability | raw source retained while owner/schema/contribution is unprovable, so typed operation unavailable; not empty/deleted/D2-invalid/stale cache | state `retained_unavailable` | `field.retained_unavailable` / 已保留，类型功能不可用 | D4 |
| `weftext.term.retained-without-membership` | 无 Facet 仍保留 / Retained without Facet Membership | D4/field state | known-schema facts retained after Facet removal without continuing Facet constraints; not orphan entity, invalid, or cleanup auth | state `retained_without_membership` | `field.retained_without_membership` / retained field | D4 |
| `weftext.term.calendar-period` | 历法周期 / Calendar Period | D4/temporal | calendar-defined day/week/month/quarter/year scope; not Node identity/title/path/arbitrary range/Event | `CalendarPeriodValue`, `calendar/period` | `calendar.period` / 历法周期 | D4 |
| `weftext.term.temporal-range` | 时间范围 / Temporal Range | D4/temporal | typed end-exclusive all-day date or zoned-instant interval; not Calendar Event or Range Node kind | `DateRangeValue`, `InstantRangeValue` | `calendar.range` / 时间范围 | D4 |
| `weftext.term.calendar-event-semantics` | 日历事件语义 / Calendar Event Semantics | D4/calendar | persistent status/recurrence/participant/reminder semantics in addition to range; not multi-day range, VEVENT identity, or UI card | FacetId `calendar/event` | `calendar.event` / 日历事件 | D4 |
| `weftext.term.bibliographic-work` | 文献作品 / Bibliographic Work | D4/library | ordinary Node+Facet semantics for a Library work that can be authored/published/identified/versioned/cited; not project work/task, Citation occurrence, or Reference | FacetId `library/work`; code `BibliographicWork` | `library.work` / 文献作品 | D4 |

## 3. D6-FA-r01 imported names

These remain owned elsewhere and are only consumed by D4:

| imported name | owner | D4 use | not |
|---|---|---|---|
| `CommitDomain/2` | D6 | operation, pin, and source observation scope | D4 namespace |
| `SourceVersion/2` | D6 | production version of a source-bearing owner and ABA binding | occurrence identity or current observation domain |
| `SourceObservation/1` | D6 | current observation qualification | D4 ObservationValue |
| `SourceVersionRef/1` | D6 | `sourceToken` selection of the complete protected `d6_source_observation/1` | bare revision, hash, I cache, or production version |
| `Frontier/2` | D6 | sealed causal prefix and dependency cut | complete Query set, payload materialization, or Registry generation |
| `SemanticState/1` | D6 | save completion state | D4 namespace state |
| `ConflictRecord` | D6 | portable conflict address | relation fact |
| `NodeRef/ResourceRef/AnnotationRef` | D3 | typed values/relations | D4 Field identity |
| D3 wire12 guarantee | D3/D6 | local versus managed qualification | D4 Action kind |

`SourceVersion/2` retains its own production `commitDomain`, `observationEpoch`, revision, or externalSequence semantics. It does not require equality with the current operation observation domain. Current D6 qualification is provided by `SourceObservation/1`: `observerDomain` equals the operation `CommitDomain`, with matching `entityRef` and current observation, file, control, Registry, and relation-incidence dependencies in the protected cut.

`complete_semantics/semantic_pending` is not the same concept as D4 `complete/partial/unavailable`, and `external_invalid` is not namespace `invalid`. Ordinary save `strict|observed_only` remains a D6 save-protection distinction; `observed_only` only consumes the D6-defined weak save qualification and is not introduced as a new D4 terminology state.

### 3.1 Current producer concepts remain imported

| Imported concept | Owner and precise D4 consumption |
|---|---|
| `DecisionKey/2` | D6: Workspace + complete CommitDomain + OperationId; protocolOwner selects the single original decision owner and is not another key field |
| `SourceRevisionPlan/1`, `SourceStamp/1` | D6: the original plan's protected proposed managed-after basis, using production-domain H and exact afterPin; not sealed source, current Observation, or early Locator capability |
| `RevisionTokenBinding/2`, `RevisionTokenSource/2`, `d6_source_revision/2` | D6: protected stable production-address binding to the closed managed-stamp/external-version arm; current observer qualification remains separate in `SourceObservation/1`/`SourceVersionRef/1`; D3 retains opaque Locator lexical ownership |
| `DependencyKey/2`, `DependencyProof/2`, `StructureRange` | D6 carrier; D3/D4/D7 real enumeration owners: fourteen key kinds and nine structure ranges, with exact key-specific completeness and continuity |
| `ContentCompletionProof/3` | D6: portable transport of sealed production SourceVersion before/after; never sender Observation/token or complete Query proof |
| `ConflictRecord/2` | D6: Frontier/2 current record; historical /1 remains original Frontier/1, while ConflictKey/1 and ConflictId retain identity |
| `WriteProtection`, `ReliableSaveState`, `InputRetentionState` | D6: installation protection, sealed save guarantee and durable input retention remain independent; prepared is not Saved |

Managed revision belongs to one production domain/entity's continuously sealed H history; production observationEpoch does not reset H. External SourceVersion has externalSequence and no managed revision or ChangeId. Current SourceObservation separately binds the operation's observerDomain/current generation, exact file object, pins and same-cut control/Registry/incidence; production domain/epoch may differ. An occurrence selector uses its actual managed inner sourceRevision plus this current outer qualification. External sequence, equal number/hash, I, or a newly signed Observation never substitutes.

Dependency stamp epoch is the continuity generation of its exact key, distinct from both source epochs. Complete empty is a proved range, not unknown, unavailable or no index hit. I rebuild neither creates such proof nor owns its durable continuity. D4 owns only relation-incidence, Calendar, Registry and temporal semantic enumeration; other keys retain their original owners.

Shared semantic IDs use the main text §2.1 decoder, including dotted namespaces, distinct ID domains, byte/segment/hyphen limits and exact ASCII comparison. Field occurrence is not Entity/Record; D4 ObservationValue is not D6 SourceObservation; event assertion is not Calendar Event; namespace owner is not D3 Owner or execution authority. No imported name enters the 26 owned concepts or the exact inventory below.

Saved, planned and unseen are original decision states, not renamed D4 typed states. Actual historical records keep original version/bytes/retention/recovery and current original-scope delivery authorization; only unseen applies the new business/consumer gate. Unknown never implies empty or permits repeat effects. Missing strong consumers gate only the dependent new strong path; ordinary source/Resource reads, Draft and qualified human whole-source saves retain their own qualification.

## 4. Terminology preservation and anti-spoof

- `people`, `organizations`, `calendar`, `library` are namespaces, not atomic data fields.
- FieldId, SemanticCodeId, package ID, display label, and locale key are separate.
- localized labels such as work/personal never substitute for semantic codes such as `people/work`.
- Relation, Reference, Node Link, and Citation remain separate.
- Owner, Namespace Owner, and Authority remain separate.
- occurrenceKey never becomes EntityRef/Locator/RecordRef.
- D6 ConflictRecord is not a D4 relation record.
- D5 Document Table row/cell is not a D4 Field/Field occurrence.

A provider trying to claim a first-party namespace fails Registry ownership regardless of install order, display label, or same-named Field.

## 5. Compatibility and Gate

All 26 fixed-S conceptId values, owners, owned names, and firstFreeze values must mechanically exact-preserve. New D6 imported names do not enter D4 ownership. Legacy identifiers and historical evidence remain under original migration/nonalias rules.

Later D7/D8/D9/D10 public names are registered by their real owner rather than reserved by D4.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings. This Lexicon closes none.

## 6. fixed-S controlled-surface exact snapshot

The JSON below is only the mechanical exact-preservation inventory for fixed-S controlled surfaces; semantic definitions remain the owner entries above. Any field drift fails the terminology Gate and is never auto-migrated by implementation.

~~~json
{
  "format": "weftext.controlled-surface-snapshot",
  "version": 1,
  "sourceCommit": "7e18168dad3e6d120fce0dd607dc10fa7894e252",
  "entries": [
    {
      "conceptId": "weftext.term.semantic-namespace",
      "ownerLayer": "D4/schema",
      "ownedWireApiCode": "`SemanticNamespaceId`, `semanticNamespaceId`",
      "uiLocaleMapping": "`schema.namespace` / 语义命名空间",
      "allowedShort": "namespace（schema上下文明示）",
      "retiredRejectedAliases": "provider namespace-as-owner guess",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.namespace-owner",
      "ownerLayer": "D4/schema registry",
      "ownedWireApiCode": "`NamespaceOwner`, `namespaceOwner`",
      "uiLocaleMapping": "`schema.namespace_owner` / 命名空间所有者",
      "allowedShort": "none",
      "retiredRejectedAliases": "provider owner, installer owner",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.field",
      "ownerLayer": "D4/schema",
      "ownedWireApiCode": "`FieldDefinition`, `fieldDefinition`",
      "uiLocaleMapping": "`schema.field` / 字段",
      "allowedShort": "field",
      "retiredRejectedAliases": "property-as-wire, attribute-as-schema-slot",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.field-id",
      "ownerLayer": "D4/schema",
      "ownedWireApiCode": "`FieldId`, `fieldId`",
      "uiLocaleMapping": "advanced `schema.field_id` / 字段标识",
      "allowedShort": "full field ID",
      "retiredRejectedAliases": "fieldName-as-ID, localized field key",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.field-value-occurrence",
      "ownerLayer": "D4/content semantics",
      "ownedWireApiCode": "`FieldValueOccurrence`, `fieldValueOccurrence`",
      "uiLocaleMapping": "`field.entry` / 字段项",
      "allowedShort": "occurrence（已明确Field上下文）",
      "retiredRejectedAliases": "field entity, property record",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.occurrence-key",
      "ownerLayer": "D4/content selector",
      "ownedWireApiCode": "`OccurrenceKey`, `occurrenceKey`",
      "uiLocaleMapping": "advanced `field.occurrence_key` / 出现项键",
      "allowedShort": "key（entry内部）",
      "retiredRejectedAliases": "EntryId, FieldRef, OccurrenceRef",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.typed-value",
      "ownerLayer": "D4/value",
      "ownedWireApiCode": "`TypedValue`, `typedValue`",
      "uiLocaleMapping": "`field.typed_value` / 类型化值",
      "allowedShort": "value（type已确定）",
      "retiredRejectedAliases": "anyValue, metadata value",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.value-type",
      "ownerLayer": "D4/value schema",
      "ownedWireApiCode": "`ValueType`, `valueType`",
      "uiLocaleMapping": "`schema.value_type` / 值类型",
      "allowedShort": "type（schema上下文明示）",
      "retiredRejectedAliases": "class, open type",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.field-shape",
      "ownerLayer": "D4/schema",
      "ownedWireApiCode": "`FieldSemanticShape`, `fieldSemanticShape`",
      "uiLocaleMapping": "`schema.field_shape` / 字段语义形态",
      "allowedShort": "shape",
      "retiredRejectedAliases": "factType, propertyKind",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.facet-schema",
      "ownerLayer": "D4/schema",
      "ownedWireApiCode": "`FacetSchema`, `facetSchema`",
      "uiLocaleMapping": "`facet.schema` / Facet 模式",
      "allowedShort": "schema（Facet已明确）",
      "retiredRejectedAliases": "Node Profile, model class",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.declared-facet-set",
      "ownerLayer": "D4/D2 projection",
      "ownedWireApiCode": "`DeclaredFacetSet`, `declaredFacetSet`",
      "uiLocaleMapping": "`facet.declared_set` / 声明 Facet",
      "allowedShort": "declared set",
      "retiredRejectedAliases": "active profiles",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.effective-facet-closure",
      "ownerLayer": "D4/derived",
      "ownedWireApiCode": "`EffectiveFacetClosure`, `effectiveFacetClosure`",
      "uiLocaleMapping": "`facet.effective_closure` / 有效 Facet",
      "allowedShort": "effective closure",
      "retiredRejectedAliases": "inherited profiles",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.inline-field-note",
      "ownerLayer": "D4/content",
      "ownedWireApiCode": "source/API `note`; code `InlineFieldNote`",
      "uiLocaleMapping": "`field.note` / 备注",
      "allowedShort": "note（field editor内）",
      "retiredRejectedAliases": "AnnotationNote, metadata note",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.semantic-code",
      "ownerLayer": "D4/value",
      "ownedWireApiCode": "`SemanticCodeId`, `semanticCodeId`",
      "uiLocaleMapping": "`field.semantic_code` / 语义码",
      "allowedShort": "code",
      "retiredRejectedAliases": "label-as-code, enumIndex",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.validity-interval",
      "ownerLayer": "D4/value qualifier",
      "ownedWireApiCode": "`ValidityInterval`, `validity`",
      "uiLocaleMapping": "`field.validity` / 有效期",
      "allowedShort": "validity",
      "retiredRejectedAliases": "history timestamp",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.event-assertion",
      "ownerLayer": "D4/fact",
      "ownedWireApiCode": "`EventAssertion`, shape `event_assertion`",
      "uiLocaleMapping": "`field.event_assertion` / 事件断言",
      "allowedShort": "assertion（event上下文）",
      "retiredRejectedAliases": "EventRecord, birthday scalar",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.observation",
      "ownerLayer": "D4/fact",
      "ownedWireApiCode": "`ObservationValue`, shape `observation`",
      "uiLocaleMapping": "`field.observation` / 观测",
      "allowedShort": "observation",
      "retiredRejectedAliases": "measurement record",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.relation-field",
      "ownerLayer": "D4/relation schema",
      "ownedWireApiCode": "`RelationFieldDefinition`, `relation` member",
      "uiLocaleMapping": "`relation.field` / 关系字段",
      "allowedShort": "relation field",
      "retiredRejectedAliases": "RelationType, edge field",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.inverse-relation-projection",
      "ownerLayer": "D4/derived",
      "ownedWireApiCode": "`InverseRelationProjection`, `inverseCode`",
      "uiLocaleMapping": "`relation.inverse` / 反向关系",
      "allowedShort": "inverse",
      "retiredRejectedAliases": "back relation fact",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.symmetric-relation-owner",
      "ownerLayer": "D4/relation",
      "ownedWireApiCode": "`SymmetricRelationCanonicalOwner`",
      "uiLocaleMapping": "advanced `relation.canonical_owner`",
      "allowedShort": "canonical owner",
      "retiredRejectedAliases": "left side, creator side",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.retained-unavailable",
      "ownerLayer": "D4/availability",
      "ownedWireApiCode": "state `retained_unavailable`",
      "uiLocaleMapping": "`field.retained_unavailable` / 已保留，类型功能不可用",
      "allowedShort": "unavailable（状态上下文明示）",
      "retiredRejectedAliases": "unknown-as-empty, provider-missing-null",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.retained-without-membership",
      "ownerLayer": "D4/field state",
      "ownedWireApiCode": "state `retained_without_membership`",
      "uiLocaleMapping": "`field.retained_without_membership`",
      "allowedShort": "retained field",
      "retiredRejectedAliases": "orphan field",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.calendar-period",
      "ownerLayer": "D4/temporal",
      "ownedWireApiCode": "`CalendarPeriodValue`, `calendar/period`",
      "uiLocaleMapping": "`calendar.period` / 历法周期",
      "allowedShort": "period",
      "retiredRejectedAliases": "time note identity",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.temporal-range",
      "ownerLayer": "D4/temporal",
      "ownedWireApiCode": "`DateRangeValue`, `InstantRangeValue`",
      "uiLocaleMapping": "`calendar.range` / 时间范围",
      "allowedShort": "range",
      "retiredRejectedAliases": "IntervalNode, event interval identity",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.calendar-event-semantics",
      "ownerLayer": "D4/calendar",
      "ownedWireApiCode": "FacetId `calendar/event`",
      "uiLocaleMapping": "`calendar.event` / 日历事件",
      "allowedShort": "event（Calendar明确）",
      "retiredRejectedAliases": "scheduled range auto-claim",
      "firstFreeze": "D4"
    },
    {
      "conceptId": "weftext.term.bibliographic-work",
      "ownerLayer": "D4/library",
      "ownedWireApiCode": "FacetId `library/work`; code `BibliographicWork`",
      "uiLocaleMapping": "`library.work` / 文献作品",
      "allowedShort": "Work（Library上下文明示）",
      "retiredRejectedAliases": "Reference entity, LiteratureItem",
      "firstFreeze": "D4"
    }
  ]
}
~~~

## 7. Retained naming and domain dispositions

The 26 concept inventory remains unchanged. These are the existing concepts' qualified domain/code usages, not new entities or aliases. Bare Profile does not name a Node schema: use Facet, fully qualified Weftext AsciiDoc Profile, or fully qualified CEL/Value Profile. Attribute names D2 source syntax; Field names schema; Field Value Occurrence names an authored fact; UI Property is only a label. Source, Provenance, SourceBinding and OriginBinding retain D3 distinctions. Facet requires is dependency, while required_field expresses requiredness. author_order means source order; it is not canonical inventory sorting. union_variant_equal means one common branch across all occurrences. Missing constructor kind still has its kind pointer and nearest-container span.

| Qualified term | Existing Field/member; code; UI/locale | Boundary |
|---|---|---|
| Profession / 职业 | people/profession, profession; PeopleProfessionValue; people.profession | occupation is explicit import vocabulary only; optional organization is context, not engagement |
| Engagement / 任职与隶属 | people/engagement; PeopleEngagementValue; people.engagement | appointment/affiliation are domain/import vocabulary, not alternative wire; no employer scalar |
| Position / 职务 | position; EngagementPosition; people.engagement.position | not profession/rank/organization; office/jobTitle are not second wire names |
| Rank / 职级 | rank; EngagementRankValue; people.engagement.rank | required system+level author text; grade needs explicit import mapping, no global inferred rank |
| Department / 部门 | department; EngagementDepartment; people.engagement.department | text within engagement, not organization identity, parent, or measurement unit |
| Engagement Organization Target / 任职组织目标 | target; EngagementOrganizationTarget; people.engagement.organization_target | ordinary Node target, not proof of Organization classification; organization is not an alias for target |
| Measurement Unit / 计量单位 | unitId; MeasurementUnitId; measurement.unit | registered dimension, not organizational unit; conversion requires authenticated contribution |

Life Event Assertion is the existing Event Assertion in people/life-event; Important Date is a use, and Birthday/Death Anniversary/Anniversary are selected-fact derivations. eventCode is preset semantics, customLabel exact author text; eventTime and validity.start are not interchangeable. Task is ordinary plus explicit source-declared tasks/task, never effective-only, a Task Field, status or UI flag. Account Identifier's preset/custom arms, Custom Account Service Key, Custom Account Identifier, Account Usage, Account Display Label and Account Note remain distinct; note alone belongs to Entry.note. Neutral relationship descriptors do not infer directed guardian/manager/mentor roles, inverse roles, family rank, or another Field. Asserted colleague and derived colleague are distinct sources of meaning.

Organization Primary Affiliation, Business Guidance, Territorial Administration, Explicit Joint Leadership, Supervision, Subsidiary, independently referenceable Brand Affiliation and Alliance Membership retain the exact Field/inverse directions in the catalog. Business guidance is professional guidance, not merely commercial activity; territorial administration is not location or structural parent. Joint leadership requires an explicit assertion. A brand label does not create organization identity; organization membership is not Person engagement. Publication Venue is the existing library/venue Work/container-or-Organization relationship; publisher is neither alias nor automatic mapping. Spouse Relationship is an independent historical assertion, with current spouse only a justified projection.

Template/Preset/export template/default, plugin/extension/module/pack/connector/provider, import/copy/adopt/promote/subscribe/sync, Calendar system/View/source, Resource/attachment/file, and Bibliographic Work/Reference/Citation retain their real owners and qualified meanings. A diary use creates no Node kind. Bare occurrence/key is permitted only inside a closed typed Field-editor/Entry parent and never overwrites D3 Occurrence or D7 LogicalOccurrenceKey. Negative controlled-name scans cover normative headings, schema/wire, public code/CLI and locale surfaces only; they do not rewrite user content, third-party formats, historical evidence or quoted counterexamples. Each rejection identifies its expected concept and unique replacement/deletion target.

D4SourceMaterializationEffects/1 uses existing code D4SourceMaterializationEffects and internal sourceMaterializationEffects; D4RelationCopyEffects/1 is parallel, with no new CLI/UI/locale identity. C is a typed carrier partition, not a Reference Slot/Entity/Locator. D3 PreparationBinding, DefinitionTransfer and Result/9.Q, D7 PreparedActionBinding/EffectManifest/EffectBytes, and D6 observation/metadata capabilities remain imported technical projections under their actual versions. Historical names are decoded only for actual original records; current producer consumption adds no second schema owner or ledger.
