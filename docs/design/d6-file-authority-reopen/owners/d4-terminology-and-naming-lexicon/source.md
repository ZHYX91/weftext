---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 362b6465-71ed-4176-9e74-6207fe3fab24.

# D4 Terminology and Naming Lexicon — D6-FA-r01

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. All 26 fixed-S D4 concepts preserve stable conceptId, owner, existing public wire/code/UI/locale names, and firstFreeze. This batch deletes, renames, or transfers none of them.

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

These remain owned elsewhere:

| imported name | owner | D4 use | not |
|---|---|---|---|
| CommitDomain/2 | D6 | operation/pin/source scope | D4 namespace |
| SourceVersion/2 | D6 | current author-source/ABA binding | occurrence identity |
| Frontier/1 | D6 | complete-range/cut dependency | Registry generation |
| SemanticState/1 | D6 | save completion state | D4 namespace state |
| ConflictRecord | D6 | portable conflict address | relation fact |
| NodeRef/ResourceRef/AnnotationRef | D3 | typed values/relations | D4 Field identity |
| D3 wire12 guarantee | D3/D6 | local versus managed qualification | D4 Action kind |

`complete_semantics/semantic_pending` is not the same concept as D4 `complete/partial/unavailable`, and `external_invalid` is not namespace `invalid`.

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
