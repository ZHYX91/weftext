---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：362b6465-71ed-4176-9e74-6207fe3fab24。

# D4 Terminology and Naming Lexicon — D6-FA-r01

候选状态：D6-FA-r01；P2 协调作者候选；未接受、未激活、未实现。固定 S 的 D4 lexicon 26 个 concept 全部保留 stable conceptId、owner、既有 public wire/code/UI/locale names 和 firstFreeze；本批不删除、改名或转移任何既有受控名称。

## 1. 控制规则

D4 只拥有 schema/value/relation/Calendar/Library 术语。D3 的 Node/Entity/Reference/Owner/Authority，
  D6 的 CommitDomain/SourceVersion/SemanticState/ConflictRecord/Execution Responsibility，
  D7 的 Query/Action complete cut 都是 imported concepts，不在 D4 重新占有。

同一 public name不得跨 concept重用；localized display label不是 FieldId/SemanticCodeId。semantic non-alias 只在 expected role已知时判定，不扫描用户内容、历史 evidence 或第三方 format。

## 2. 完整受控 concepts

| conceptId | 正式中英名 | owner/layer | 精确定义与排除 | owned wire/code names | UI/locale | firstFreeze |
|---|---|---|---|---|---|---|
| `weftext.term.semantic-namespace` | 语义命名空间 / Semantic Namespace | D4/schema | FacetId、FieldId、SemanticCodeId共享的 exact owner scope；不是 package/provider display name 或 carrier identity。 | `SemanticNamespaceId`, `semanticNamespaceId` | `schema.namespace` / 语义命名空间；qualified “namespace” | D4 |
| `weftext.term.namespace-owner` | 命名空间所有者 / Namespace Owner | D4/schema registry | 有权定义一个 Semantic Namespace 的主体；不是 D3 Owner、当前安装者、consumer 或 authority。 | `NamespaceOwner`, `namespaceOwner` | `schema.namespace_owner` / 命名空间所有者 | D4 |
| `weftext.term.field` | 字段 / Field | D4/schema | FieldId 标识的 schema 槽位，声明 type/shape/cardinality/constraints；不是 header 属性、UI 属性、出现项或元数据。 | `FieldDefinition`, `fieldDefinition` | `schema.field` / 字段 | D4 |
| `weftext.term.field-id` | 字段标识 / Field ID | D4/schema | `namespace/local-field-path` canonical semantic ID；不是 display label、locale key、source range 或 UUID。 | `FieldId`, `fieldId` | `schema.field_id` / 字段标识 | D4 |
| `weftext.term.field-value-occurrence` | 字段值出现项 / Field Value Occurrence | D4/content semantics | D4 Entry/1 中一个 Field 的作者 value 事实；仍属于 owning Document；不是 Entity、Record 或 durable occurrence subtype。 | `FieldValueOccurrence`, `fieldValueOccurrence` | `field.entry` / 字段项 | D4 |
| `weftext.term.occurrence-key` | 出现项键 / Occurrence Key | D4/content selector | owner+FieldId+expected revision 内的 patch selector；不是 EntityRef、Locator、RecordRef、OperationId 或持久身份。 | `OccurrenceKey`, `occurrenceKey` | `field.occurrence_key` / 出现项键 | D4 |
| `weftext.term.typed-value` | 类型化值 / Typed Value | D4/value | closed D4 constructor + Field schema 解释的作者值；不是 untyped JSON/display string/provider blob。 | `TypedValue`, `typedValue` | `field.typed_value` / 类型化值 | D4 |
| `weftext.term.value-type` | 值类型 / Value Type | D4/value schema | 定义合法 Typed Value 的 shape/comparison/limits 的 closed constructor；不是 Facet、Node kind 或运行时 class。 | `ValueType`, `valueType` | `schema.value_type` / 值类型 | D4 |
| `weftext.term.field-shape` | 字段语义形态 / Field Semantic Shape | D4/schema | `fact|event_assertion|observation|relation` 之一；不是 JSON shape/UI widget/Facet。 | `FieldSemanticShape`, `fieldSemanticShape` | `schema.field_shape` / 字段语义形态 | D4 |
| `weftext.term.facet-schema` | Facet 模式 / Facet Schema | D4/schema | 一个 FacetId 的持续 closed 语义合同；不是 D2 AsciiDoc Profile、package manifest、Template 或 Preset。 | `FacetSchema`, `facetSchema` | `facet.schema` / Facet 模式 | D4 |
| `weftext.term.declared-facet-set` | 声明 Facet 集 / Declared Facet Set | D4/D2 projection | source 明确声明的 exact FacetId 集合；不是 effective closure 或已安装 provider 列表。 | `DeclaredFacetSet`, `declaredFacetSet` | `facet.declared_set` / 声明 Facet | D4 |
| `weftext.term.effective-facet-closure` | 有效 Facet 闭包 / Effective Facet Closure | D4/derived | declared set 经 requires 推导的可重建集合；不是作者 source，不回写 membership。 | `EffectiveFacetClosure`, `effectiveFacetClosure` | `facet.effective_closure` / 有效 Facet | D4 |
| `weftext.term.inline-field-note` | 字段项内嵌备注 / Inline Field Note | D4/content | 与 Field Value Occurrence 同存的可选 plain text；不是 Annotation、typed qualifier 或 provenance。 | source/API `note`; code `InlineFieldNote` | `field.note` / 备注 | D4 |
| `weftext.term.semantic-code` | 语义码 / Semantic Code | D4/value | 与 locale 无关的稳定 code；跨 Field/contribution 时使用 SemanticCodeId；不是本地化 label、custom text 或 enum ordinal。 | `SemanticCodeId`, `semanticCodeId` | `field.semantic_code` / 语义码 | D4 |
| `weftext.term.validity-interval` | 有效期区间 / Validity Interval | D4/value qualifier | 作者事实生效的 date/instant range；不是 recordedAt、operation time 或 lifecycle。 | `ValidityInterval`, `validity` | `field.validity` / 有效期 | D4 |
| `weftext.term.event-assertion` | 事件断言 / Event Assertion | D4/fact | 对 domain event 的可冲突作者断言，可带 precision/provenance/confidence；不是 Calendar Event Node/Facet 或 operation event。 | `EventAssertion`, shape `event_assertion` | `field.event_assertion` / 事件断言 | D4 |
| `weftext.term.observation` | 观测项 / Observation | D4/fact | 带 observedAt、可带 unit/provenance/confidence 的观察事实；不是 current state/audit event/derived metric。 | `ObservationValue`, shape `observation` | `field.observation` / 观测 | D4 |
| `weftext.term.relation-field` | 关系字段 / Relation Field | D4/relation schema | relation 形态的 Field，拥有方向/inverse/cardinality/lifecycle/delete/projection；不是 RelationType entity、Node Link 或 Reference slot。 | `RelationFieldDefinition`, member `relation` | `relation.field` / 关系字段 | D4 |
| `weftext.term.inverse-relation-projection` | 反向关系投影 / Inverse Relation Projection | D4/derived | 从唯一 authored relation fact 可重建的 inverse query/View；不是第二作者事实。 | `InverseRelationProjection`, `inverseCode` | `relation.inverse` / 反向关系 | D4 |
| `weftext.term.symmetric-relation-owner` | 对称关系规范事实端 / Symmetric Relation Canonical Owner | D4/relation | 由两个 NodeRef canonical encoding 确定的唯一 symmetric fact owner；不是 D3 general Owner 替代或权限绕过。 | `SymmetricRelationCanonicalOwner` | `relation.canonical_owner` / canonical owner | D4 |
| `weftext.term.retained-unavailable` | 保留但类型语义不可用 / Retained but Typed-Unavailable | D4/availability | raw author source 保留但 owner/schema/contribution不可证明，typed操作不可用；不是 empty/deleted/D2-invalid/stale cache。 | state `retained_unavailable` | `field.retained_unavailable` / 已保留，类型功能不可用 | D4 |
| `weftext.term.retained-without-membership` | 无 Facet 仍保留 / Retained without Facet Membership | D4/field state | Remove Facet 后仍由已知 Field schema 解释，但不受该 Facet 持续约束；不是 orphan entity、invalid 状态或 cleanup 授权。 | state `retained_without_membership` | `field.retained_without_membership` / retained field | D4 |
| `weftext.term.calendar-period` | 历法周期 / Calendar Period | D4/temporal | calendar system 定义的 day/week/month/quarter/year 范围；不是 Node identity、title/path、任意 range 或 Event。 | `CalendarPeriodValue`, `calendar/period` | `calendar.period` / 历法周期 | D4 |
| `weftext.term.temporal-range` | 时间范围 / Temporal Range | D4/temporal | 全天日期范围或带时区瞬时范围的端点排除型 typed 区间；不是 Calendar Event 或 Range Node 类型。 | `DateRangeValue`, `InstantRangeValue` | `calendar.range` / 时间范围 | D4 |
| `weftext.term.calendar-event-semantics` | 日历事件语义 / Calendar Event Semantics | D4/calendar | range 之外的 status/recurrence/participant/reminder-intent 持续 schema；不是跨日range本身、VEVENT identity 或 UI card。 | FacetId `calendar/event` | `calendar.event` / 日历事件 | D4 |
| `weftext.term.bibliographic-work` | 文献作品 / Bibliographic Work | D4/library | Library 中可创作/出版/标识/版本化/引用的 ordinary Node+Facet语义；不是 project work/task、Citation occurrence 或 Reference。 | FacetId `library/work`; code `BibliographicWork` | `library.work` / 文献作品 | D4 |

## 3. D6-FA-r01 imported names

下列名称保持原 owner，D4 只消费，不重新拥有：

| imported name | owner | D4用途 | 不是 |
|---|---|---|---|
| `CommitDomain/2` | D6 | operation、pin、source observation范围 | D4 namespace |
| `SourceVersion/2` | D6 | source-bearing owner 的生产版本与 ABA 绑定 | occurrence identity 或当前观察域 |
| `SourceObservation/1` | D6 | 当前 observation qualification | D4 ObservationValue |
| `SourceVersionRef/1` | D6 | 通过 `sourceToken` 选择完整当前 `d6_source_observation/1` | 裸 revision、hash、I cache 或 production version |
| `Frontier/2` | D6 | 已 seal 因果前缀与依赖 cut | Query 全集、payload 物化或 Registry generation |
| `SemanticState/1` | D6 | save completion状态 | D4 namespace state |
| `ConflictRecord` | D6 | portable conflict定位 | relation fact |
| `NodeRef/ResourceRef/AnnotationRef` | D3 | typed values/relations | D4 Field identity |
| D3 wire12 guarantee | D3/D6 | local vs managed qualification | D4 Action kind |

`SourceVersion/2` 保留自身生产 `commitDomain`、`observationEpoch`、revision 或 externalSequence 定义；它不要求等于当前 operation observation domain。当前 D6 外层资格由 `SourceObservation/1` 提供：`observerDomain` 必须等于 operation `CommitDomain`，并与 `entityRef`、当前 observation、file/control/Registry/relation-incidence 依赖共同形成保护 cut。

`complete_semantics/semantic_pending` 不与 D4 `complete/partial/unavailable` 共用一个概念；`external_invalid` 也不是 `invalid` namespace。普通保存中的 `strict|observed_only` 与 D4 类型状态保持分域；`observed_only` 仅消费 D6 已定义的弱保存资格，不成为新的 D4 状态。

### 3.1 当前 producer 概念保持原所有权

| 引入概念 | owner 及 D4 的精确消费方式 |
|---|---|
| `DecisionKey/2` | D6：Workspace、完整 CommitDomain、OperationId 共同构成 key；protocolOwner 标识唯一原 decision owner，不是额外 key 成员 |
| `SourceRevisionPlan/1`, `SourceStamp/1` | D6：原 plan 中受保护的拟议托管 after 基准，使用生产域 H 和精确 afterPin；不是已 seal 的源、当前 Observation 或提前可用的 Locator |
| `RevisionTokenBinding/2`, `RevisionTokenSource/2`, `d6_source_revision/2` | D6：到闭合 managed stamp/external version 分支的受保护稳定生产地址绑定；当前 observer 资格仍由 `SourceObservation/1`/`SourceVersionRef/1` 独立证明；D3 保留 Locator 不透明词法所有权 |
| `DependencyKey/2`, `DependencyProof/2`, `StructureRange` | D6 承载，D3/D4/D7 各自提供真实枚举：十四种依赖键、九种结构范围，各键有精确完整性和连续性合同 |
| `ContentCompletionProof/3` | D6：可移植传输已 seal 的生产 SourceVersion 前后像；不是发送方 Observation/token，也不是完整 Query 证明 |
| `ConflictRecord/2` | D6：当前记录使用 Frontier/2；历史 /1 保持原 Frontier/1，ConflictKey/1 与 ConflictId 身份不变 |
| `WriteProtection`, `ReliableSaveState`, `InputRetentionState` | D6：安装保护、seal 后保存保证和输入耐久保留相互独立；prepared 不等于 Saved |

托管 revision 属于某一生产域和实体连续 seal 的 H 历史，生产 observationEpoch 变化不重置 H。external SourceVersion 有 externalSequence，没有托管 revision 或 ChangeId。当前 SourceObservation 独立绑定操作的 observerDomain、当前观察代、精确文件对象、pins 和同 cut 的控制、Registry、关系关联；生产域和生产 epoch 可以不同。出现项选择器使用实际托管 inner sourceRevision，并叠加当前外层资格；external sequence、相同数字或 hash、I、新签 Observation 均不能替代。

依赖 stamp 的 epoch 是精确 key 的证明连续性代，与两种 source epoch 都不同。完整空范围需要证明，不是 unknown、unavailable 或索引无命中。重建 I 不产生此证明，I 也不拥有其耐久连续性。D4 只拥有关系关联、Calendar、Registry 和时间规则的语义枚举，其它 key 保持原 owner。

共享语义 ID 使用主文 §2.1 解码器，包括点分命名空间、独立 ID 域、字节/分段/连字符限制和精确 ASCII 比较。Field 出现项不是 Entity/Record；D4 ObservationValue 不是 D6 SourceObservation；事件断言不是 Calendar Event；Namespace Owner 不是 D3 Owner 或执行权威。引入名称不进入26项自有概念或下方精确清单。

saved、planned、unseen 是原 decision 状态，不能改名为 D4 类型状态。实际历史记录保留原版本、字节、保留及恢复合同，交付时检查当前原范围授权；只有 unseen 应用新业务或 consumer 门。unknown 不表示空，也不允许重复 effect。缺少强 consumer 仅限制依赖它的新强路径，普通 source/Resource 读取、Draft、合格人工 whole-source 保存保持各自资格。

## 4. 术语守恒与 anti-spoof

- `people`、`organizations`、`calendar`、`library` 是 namespace，不是一个 atomic data field。
- FieldId/SemanticCodeId/canonical package ID/display label/locale key 分域。
- localized “工作/个人”等标签不能替代 `people/work` 等 semantic code。
- relation、Reference、Node Link、Citation继续分域。
- Owner、Namespace Owner、Authority继续分域。
- occurrenceKey 不提升为 EntityRef/Locator/RecordRef。
- D6 ConflictRecord不是 D4 relation record。
- D5 Document Table row/cell不是 D4 Field/Field occurrence。

provider/插件尝试声明 first-party namespace 必须在 Registry owner gate失败；安装顺序、display label或同名 Field不授 namespace ownership。

## 5. compatibility 与 Gate

fixed-S 26 conceptId、每项 owner/owned names/firstFreeze必须机械 exact-preserve。新 D6 imported names不得加入 D4 owned set。legacy identifiers与历史 evidence只按原 migration/nonalias规则读取。

后续 D7/D8/D9/D10新增公共名必须由其真实 owner登记，不由 D4预先占位。

旧 D10 B13仍 REVISE、术语/双语 FAIL、11 OPEN；本 Lexicon不关闭任何 finding。

## 6. fixed-S 受控表面 exact snapshot

以下 JSON 只作为 fixed-S 受控表面的机械 exact-preservation inventory；语义定义仍以上文 owner 条目为准。任何字段差异都使术语 Gate 失败，不能由实现自动迁移。

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

## 7. 保留的名称与领域处分

26项概念清单不变。以下只是既有概念的限定领域或代码用法，不新增实体或别名。裸 Profile 不能表示节点模式，分别使用 Facet、完整限定的 Weftext AsciiDoc Profile 或 CEL/Value Profile。Attribute 指 D2 源语法，Field 指模式，Field Value Occurrence 指作者事实，UI Property 只是标签。Source、Provenance、SourceBinding、OriginBinding 保持 D3 区分。Facet requires 表达依赖，required_field 表达必填。author_order 是源顺序，不是清单规范排序；union_variant_equal 要求全部出现项使用同一分支；缺少构造器 kind 时仍使用 kind pointer 和最近容器 span。

| 限定术语 | 既有 Field/成员、代码、界面或区域键 | 边界 |
|---|---|---|
| Profession / 职业 | people/profession, profession; PeopleProfessionValue; people.profession | occupation 仅为显式导入词汇，可选 organization 是背景，不是任职 |
| Engagement / 任职与隶属 | people/engagement; PeopleEngagementValue; people.engagement | appointment/affiliation 只用于领域描述或导入，不是替代 wire，不设单一 employer 标量 |
| Position / 职务 | position; EngagementPosition; people.engagement.position | 不等于职业、职级或组织；office/jobTitle 不能成为第二 wire 名 |
| Rank / 职级 | rank; EngagementRankValue; people.engagement.rank | system+level 作者文本均必需；grade 需显式导入映射，不推断全球职级 |
| Department / 部门 | department; EngagementDepartment; people.engagement.department | 任职内部文本，不是组织身份、父节点或计量单位 |
| Engagement Organization Target / 任职组织目标 | target; EngagementOrganizationTarget; people.engagement.organization_target | 普通 Node 目标不证明已分类为 Organization；organization 不是 target 别名 |
| Measurement Unit / 计量单位 | unitId; MeasurementUnitId; measurement.unit | 注册维度而非组织单元；换算需要经过认证的贡献项 |

Life Event Assertion 是 people/life-event 中既有 Event Assertion 的领域使用；Important Date 是用途，Birthday、Death Anniversary、Anniversary 是基于选定事实的派生称谓。eventCode 是 preset 语义，customLabel 是作者精确文本，eventTime 与 validity.start 不互换。Task 必须是 ordinary 且源中显式声明 tasks/task，不能仅凭有效闭包、Task Field、status 或界面标志认定。Account Identifier 的 preset/custom、自定义服务键、自定义账号值、账号用途、显示标签和备注保持独立；只有备注属于 Entry.note。中性关系描述不推断 guardian/manager/mentor 等方向角色、inverse、家族等级或另一个 Field；作者断言同事与派生同事的语义来源不同。

组织主要隶属、业务指导、属地管理、明确共同领导、监管、母子公司、可独立引用品牌归属和联盟成员，保持目录中的精确 Field/inverse 方向。业务指导不限商业经营，属地管理不是地点或结构 parent，共同领导要求明确断言。品牌标签不创建组织身份，组织成员不是 Person engagement。Publication Venue 是既有 library/venue 的 Work/container 或 Organization 关系，publisher 既不是别名，也不自动映射。配偶关系是独立历史断言，“当前配偶”只能是有根据的投影。

Template/Preset/导出模板/默认值，插件/扩展/模块/pack/connector/provider，导入/复制/adopt/promote/订阅/同步，历法系统/View/source，Resource/附件/文件，文献 Work/Reference/Citation，都保持真实 owner 和限定含义。日记用途不创建节点种类。裸 occurrence/key 只在闭合类型明确的 Field 编辑器或 Entry 父级内部使用，不能覆盖 D3 Occurrence 或 D7 LogicalOccurrenceKey。受控名称负向扫描只检查规范标题、schema/wire、公共 code/CLI、locale 表面，不改用户内容、第三方格式、历史证据或引用反例；每次拒绝明确预期概念及唯一替换或删除目标。

D4SourceMaterializationEffects/1 沿用代码类型 D4SourceMaterializationEffects 和内部成员 sourceMaterializationEffects，与 D4RelationCopyEffects/1 并列，不新增 CLI/UI/locale 身份。C 是类型 carrier 分区，不是 Reference Slot/Entity/Locator。D3 PreparationBinding、DefinitionTransfer、Result/9.Q，D7 PreparedActionBinding/EffectManifest/EffectBytes，D6 观察和元数据能力，均按真实版本作为外部技术投影引入。历史名称只为实际原记录解码；当前 producer 消费不新增第二 schema owner 或 ledger。
