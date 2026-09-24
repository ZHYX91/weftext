---
_weftext:
  id: "362b6465-71ed-4176-9e74-6207fe3fab24"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。


# D4 Terminology and Naming Lexicon



本文件是 D4 已冻结新概念的术语增补。它导入但不修改当前D3 Lexicon：本轮共42个concept，其中39个继承、3个由D3新增；下列原D3概念的含义保持。D3 已拥有的 Node、Document、Occurrence、Resource、Annotation、Typed Reference、Reference、Node Link、Citation、Relation、Owner、Authority、Source、SourceBinding、Provenance、OriginBinding、Facet、FacetId 等条目继续原样生效；这里不得重定义或删除它们。

Revision35 clarification: author_order refers to authored occurrence sequence, not canonical ordering of an unordered binding or projection. union_variant_equal means one common variant across all occurrences of both present Fields; it is not variant-set equality. A constructor fault pointer names kind even when the member is missing; the source span then identifies the nearest existing container. These clarify existing concepts without introducing a new identity, public alias or upstream term.

## 1. naming invariants

1. 裸 `Profile` 禁止用于 D4 Node schema。D2 grammar必须称 `Weftext AsciiDoc Profile`；Node持续schema mechanism称 `Facet`；CEL/Value Profile必须完整限定。
2. `Attribute`只用于D2 AsciiDoc/header或attribute carrier的source语法；D4 schema slot称`Field`，持久作者事实称`Field Value Occurrence`。UI可显示“属性”，但locale key必须映射到明确concept。
3. `Relation`不得与Typed Reference、Reference、Node Link、Citation、structural parent、Annotation reply或Graph edge projection混称。
4. `Source`只表示输入来源；`Provenance`固定中文“来源证据”；`SourceBinding`/`OriginBinding`沿用D3，不得缩写成source/origin字段。
5. wire/API/code/CLI/locale name只有表中owned names；display label可本地化但不改变ID。
6. FacetSchema的`requires`仅表示Facet依赖；Field必填性由`required_field`约束表达，二者不得混称。

## 2. D4 concept registry

| concept ID | 中文正式名 | English canonical term | owner/layer | exact definition | exclusions | owned wire/API/code names | UI/locale mapping | allowed short | retired / rejected aliases | first freeze target |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `weftext.term.semantic-namespace` | 语义命名空间 | Semantic Namespace | D4/schema | FacetId、FieldId、SemanticCodeId共享的exact owner scope。 | 不是package、provider display name、carrier block identity。 | `SemanticNamespaceId`, `semanticNamespaceId` | `schema.namespace` / 语义命名空间 | namespace（schema上下文明示） | provider namespace-as-owner guess | D4 |
| `weftext.term.namespace-owner` | 命名空间所有者 | Namespace Owner | D4/schema registry | 可证明有权定义一个Semantic Namespace中Facet/Field/Code的主体。 | 不是D3 object Owner、当前安装者、consumer、authority。 | `NamespaceOwner`, `namespaceOwner` | `schema.namespace_owner` / 命名空间所有者 | none | provider owner, installer owner | D4 |
| `weftext.term.field` | 字段 | Field | D4/schema | 由唯一FieldId标识、声明value type/shape/cardinality/constraints的schema slot。 | 不是header attribute、UI property、stored occurrence、metadata。 | `FieldDefinition`, `fieldDefinition` | `schema.field` / 字段 | field | property-as-wire, attribute-as-schema-slot | D4 |
| `weftext.term.field-id` | 字段标识 | Field ID | D4/schema | `namespace/local-field-path` canonical semantic identifier。 | 不是display label、locale key、source range、UUID。 | `FieldId`, `fieldId` | advanced `schema.field_id` / 字段标识 | full field ID | fieldName-as-ID, localized field key | D4 |
| `weftext.term.field-value-occurrence` | 字段值出现项 | Field Value Occurrence | D4/content semantics | D4 Entry/1中一个Field的一个value事实，仍属于owning Node/Document内容。 | 不是D3 Entity、Record、Document Occurrence durable subtype或Annotation target。 | `FieldValueOccurrence`, `fieldValueOccurrence` | `field.entry` / 字段项 | occurrence（已明确Field上下文） | field entity, property record | D4 |
| `weftext.term.occurrence-key` | 出现项键 | Occurrence Key | D4/content selector | owner Node + FieldId内、expected-revision-bound的value-internal patch selector。 | 不是EntityRef、Locator、RecordRef、OperationId、durable identity。 | `OccurrenceKey`, `occurrenceKey` | advanced `field.occurrence_key` / 出现项键 | key（entry内部） | EntryId, FieldRef, OccurrenceRef | D4 |
| `weftext.term.typed-value` | 类型化值 | Typed Value | D4/value | 由closed D4 value constructor及Field schema完整解释的作者值。 | 不是untyped JSON、display string、provider blob。 | `TypedValue`, `typedValue` | `field.typed_value` / 类型化值 | value（type已确定） | anyValue, metadata value | D4 |
| `weftext.term.value-type` | 值类型 | Value Type | D4/value schema | 定义Typed Value合法shape、comparison和limits的closed constructor。 | 不是Facet、Node kind、programming-language runtime class。 | `ValueType`, `valueType` | `schema.value_type` / 值类型 | type（schema上下文明示） | class, open type | D4 |
| `weftext.term.field-shape` | 字段语义形态 | Field Semantic Shape | D4/schema | `fact|event_assertion|observation|relation`之一，决定allowed qualifiers与projection。 | 不是JSON shape、UI widget、Facet。 | `FieldSemanticShape`, `fieldSemanticShape` | `schema.field_shape` / 字段语义形态 | shape | factType, propertyKind | D4 |
| `weftext.term.facet-schema` | Facet 模式 | Facet Schema | D4/schema | 一个FacetId的closed持续semantic contract：dependencies/conflicts/fields/constraints。 | 不是D2 Weftext AsciiDoc Profile、package manifest、Template、Preset。 | `FacetSchema`, `facetSchema` | `facet.schema` / Facet 模式 | schema（Facet已明确） | Node Profile, model class | D4 |
| `weftext.term.declared-facet-set` | 声明 Facet 集 | Declared Facet Set | D4/D2 projection | `wf-facets`唯一作者源投影出的unordered exact FacetId set。 | 不是effective closure、installed provider list。 | `DeclaredFacetSet`, `declaredFacetSet` | `facet.declared_set` / 声明 Facet | declared set | active profiles | D4 |
| `weftext.term.effective-facet-closure` | 有效 Facet 闭包 | Effective Facet Closure | D4/derived | declared set经requires确定推导的可重建集合。 | 不是作者source、不得回写membership。 | `EffectiveFacetClosure`, `effectiveFacetClosure` | `facet.effective_closure` / 有效 Facet | effective closure | inherited profiles | D4 |
| `weftext.term.inline-field-note` | 字段项内嵌备注 | Inline Field Note | D4/content | 与同一Field Value Occurrence同存、plain text、optional的作者备注。 | 不是Annotation、typed qualifier、provenance。 | source/API `note`; code `InlineFieldNote` | `field.note` / 备注 | note（field editor内） | AnnotationNote, metadata note | D4 |
| `weftext.term.semantic-code` | 语义码 | Semantic Code | D4/value | locale-independent stable code；跨Field/贡献时使用SemanticCodeId。 | 不是localized label、custom text、enum ordinal。 | `SemanticCodeId`, `semanticCodeId` | `field.semantic_code` / 语义码 | code | label-as-code, enumIndex | D4 |
| `weftext.term.validity-interval` | 有效期区间 | Validity Interval | D4/value qualifier | 作者事实生效的date/instant range。 | 不是recordedAt、operation time、lifecycle state。 | `ValidityInterval`, `validity` | `field.validity` / 有效期 | validity | history timestamp | D4 |
| `weftext.term.event-assertion` | 事件断言 | Event Assertion | D4/fact | 对一次domain event的可冲突、可带precision/provenance/confidence的作者断言。 | 不是Calendar Event Node/Facet、operation event。 | `EventAssertion`, shape `event_assertion` | `field.event_assertion` / 事件断言 | assertion（event上下文） | EventRecord, birthday scalar | D4 |
| `weftext.term.observation` | 观测项 | Observation | D4/fact | 带observedAt、可能带unit/provenance/confidence的测量或观察事实。 | 不是current state、audit event、derived metric。 | `ObservationValue`, shape `observation` | `field.observation` / 观测 | observation | measurement record | D4 |
| `weftext.term.relation-field` | 关系字段 | Relation Field | D4/relation schema | value shape为relation、明确方向/inverse/cardinality/lifecycle/delete/projection的Field。 | 不是RelationType entity、Node Link、Reference slot。 | `RelationFieldDefinition`, `relation` member | `relation.field` / 关系字段 | relation field | RelationType, edge field | D4 |
| `weftext.term.inverse-relation-projection` | 反向关系投影 | Inverse Relation Projection | D4/derived | 从唯一authored relation fact可重建的反向查询/View结果。 | 不是第二作者事实、不得双写。 | `InverseRelationProjection`, `inverseCode` | `relation.inverse` / 反向关系 | inverse | back relation fact | D4 |
| `weftext.term.symmetric-relation-owner` | 对称关系规范事实端 | Symmetric Relation Canonical Owner | D4/relation | 由两个NodeRef canonical encoding确定、保存唯一symmetric fact的端。 | 不是D3 general Owner concept替代、不是权限绕过。 | `SymmetricRelationCanonicalOwner` | advanced `relation.canonical_owner` | canonical owner | left side, creator side | D4 |
| `weftext.term.retained-unavailable` | 保留但类型语义不可用 | Retained but Typed-Unavailable | D4/availability | raw author source完整保留但owner/schema/provider不可证明，typed operations不可用。 | 不是empty、deleted、invalid D2、stale cache。 | state `retained_unavailable` | `field.retained_unavailable` / 已保留，类型功能不可用 | unavailable（状态上下文明示） | unknown-as-empty, provider-missing-null | D4 |
| `weftext.term.retained-without-membership` | 无 Facet 仍保留 | Retained without Facet Membership | D4/field state | Remove Facet后仍由known Field schema解释、但不受该Facet持续约束的facts。 | 不是orphan entity、invalid、cleanup authorization。 | state `retained_without_membership` | `field.retained_without_membership` | retained field | orphan field | D4 |
| `weftext.term.calendar-period` | 历法周期 | Calendar Period | D4/temporal | 由calendar system定义的day/week/month/quarter/year scope。 | 不是Node identity、title/path、arbitrary range或Event。 | `CalendarPeriodValue`, `calendar/period` | `calendar.period` / 历法周期 | period | time note identity | D4 |
| `weftext.term.temporal-range` | 时间范围 | Temporal Range | D4/temporal | all-day date range或zoned instant range的typed end-exclusive interval。 | 不是Calendar Event、独立Range Node kind。 | `DateRangeValue`, `InstantRangeValue` | `calendar.range` / 时间范围 | range | IntervalNode, event interval identity | D4 |
| `weftext.term.calendar-event-semantics` | 日历事件语义 | Calendar Event Semantics | D4/calendar | range之外的status/recurrence/participant/reminder-intent持续schema。 | 不是跨日范围本身、VEVENT identity、Calendar UI card。 | FacetId `calendar/event` | `calendar.event` / 日历事件 | event（Calendar明确） | scheduled range auto-claim | D4 |
| `weftext.term.bibliographic-work` | 文献作品 | Bibliographic Work | D4/library | Library领域中可被创作、出版、标识、版本化和引用的普通Node+Facet语义。 | 不是project work/task、Citation occurrence、Reference/NodeRef。 | FacetId `library/work`; code `BibliographicWork` | `library.work` / 文献作品 | Work（Library上下文明示） | Reference entity, LiteratureItem | D4 |

## 3. controlled collision dispositions

| collision | disposition |
| --- | --- |
| Profile triple | `Facet` for Node schema; keep-qualified D2/CEL terms; bare Profile retired on D4 controlled surfaces. |
| type/class/schema/facet/specialization/role | `Value Type`, `Facet Schema`, D2 Core kind/Facet, and relation role are distinct; class/instance is prose only. |
| plugin/extension/module/pack/connector/provider | defer-with-owner D10/A2; D4 only uses `namespace owner`, `schema provider`, `registry contribution` where technically exact. |
| Template/Preset/export template/default | keep frozen Template term; D4 Facet defaults only in explicit Create plan; D9/D10 own the other terms. |
| Node/note/document/item/record/entity/occurrence | import D3 meanings; D4 adds qualified Field Value Occurrence only, never Entity/Record. |
| attribute/property/field/fact/metadata | source Attribute, schema Field, stored Field Value Occurrence/author fact, UI Property label, D2/system metadata remain distinct. |
| relation/link/reference/citation | import D3/D4 exclusions; relation Field is the only authored D4 Relation source. |
| Resource/attachment/file | import D3; D4 ResourceRef fields remain owner-local. |
| Time/Calendar/period/range/event/diary/journal | Calendar is product candidate; Calendar Period/Temporal Range/Event Semantics distinct; diary is usage anti-expansion; scholarly journal is Work kind/venue. |
| calendar system/view/source | Calendar system contribution, derived View, and D3 Source/SourceBinding remain distinct. |
| Library/Work/Reference/Citation/Publication | Bibliographic Work qualified; publication-state/version relation distinct; Citation remains D2 occurrence. |
| import/copy/adopt/promote/subscribe/sync | import exact D3 operation meanings; D4 mapping never renames them. |
| owner/authority/source/provenance/origin | import exact D3 concepts; Namespace Owner is qualified and cannot replace object Owner. |

## 4. negative gate scope

受控扫描面仅限本D4 decision/Impact中的 normative headings、JSON schema/wire identifiers、public code symbol convention、CLI verb/flag候选和locale keys。不得粗暴扫描review response、historical evidence、migration note、quoted counterexample、用户Document/Annotation内容或第三方format。

必须拒绝的controlled identifiers包括：`NodeProfile`, `profileId`（指Facet时）、`FieldRef`, `OccurrenceRef`, `EntryId`（指occurrenceKey时）、`RelationType`（作为开放global entity时）、`propertyBlob`, `peopleObject`, `unknownAsEmpty`, `lastWinsField`。每个hit必须给expected concept与唯一replacement/deletion target。

限定简称回归：normative heading/code/locale中裸 `occurrence` 或裸 `key` 不得直接映射D4概念；`Field Value Occurrence`/`fieldValueOccurrence`与`Occurrence Key`/`occurrenceKey`是正例。只有已由closed typed parent明确为Field editor或Entry object的member label才可显示短`occurrence`/`key`；离开该parent必须恢复完整term，且不得覆盖D3元术语Occurrence或D7 LogicalOccurrenceKey。

## 5. Employment terminology and exact domain dispositions

以下映射冻结术语和现有Field member的含义，不新增Node kind、Field或HR实体。wire/code/UI候选各自限定所属层；custom rank现以制度限定的作者对象提供；独立职位/部门实体与机器可解释的rank presets仍待显式schema演进评审。D9导入可识别外部同义词，但必须显式映射，不能新增alias读写路径。

| 中文及输入别名 | canonical term | 精确定义与排除边界 | owned wire / code / UI name | disposition / rejected aliases |
| --- | --- | --- | --- | --- |
| 职业、专业身份；profession/occupation | Profession / 职业 | Person一般职业，可重复且有validity；optional organization仅是背景，不建立具体任职或inverse。 | `people/profession`, member `profession`; `PeopleProfessionValue`; `people.profession` / 职业 | keep `profession`; `occupation`仅外部import alias，禁止作为第二Field/API名；不与employer/title合并。 |
| 任职、隶属、参与；engagement/appointment/affiliation | Engagement / 任职与隶属 | Person在特定组织的持续relation fact；职位/部门/职级是正交成员，时间与状态只在qualifiers。 | `people/engagement`; `PeopleEngagementValue`; `people.engagement` / 任职与隶属 | keep `engagement`; appointment与affiliation是领域表述/import候选，不是可互换wire/Field别名；禁止单一employer scalar。 |
| 职务、职位；position/office title | Position / 职务 | 一项Engagement中的职务名称，不表示一般职业、组织实体或等级制度。 | engagement member `position`; `EngagementPosition`; `people.engagement.position` / 职务 | keep `position`; `office`、`jobTitle`不得成为第二wire名；office实体候选defer-with-owner D5/D10。 |
| 职级、等级；rank/grade | Rank / 职级 | 特定制度中的作者级别，不能从position或国家默认值推导；当前对象要求独立system与level原文。 | engagement member `rank`; `EngagementRankValue`; `people.engagement.rank` / 职级 | keep `rank`; `grade`仅显式import mapping；拒绝全球硬编码enum与文本自动转code；custom system/level现已支持；机器码域、制度预设及迁移贡献由D10后续显式评审，不能反向推迟当前作者值。 |
| 部门、组织内单元；department/unit | Department / 部门 | Engagement的内部部门text，不能充当任职组织Node identity、结构父节点或计量单位。 | engagement member `department`; `EngagementDepartment`; `people.engagement.department` / 部门 | keep `department`; `unit`禁止作为裸wire/UI名；独立部门identity defer-with-owner D5/D10。 |
| 任职单位、任职组织 | Engagement Organization Target / 任职组织目标 | engagement的target；NodeRef可指向任意普通Node，不要求Organization Facet；literal text不产生可靠inverse。 | engagement member `target`; `EngagementOrganizationTarget`; `people.engagement.organization_target` / 任职组织 | “单位”必须限定为任职组织；不得把`organization`当成engagement target的wire alias，也不与profession的optional organization context混同。 |
| 计量单位 | Measurement Unit / 计量单位 | quantity的已注册unit identity与dimension，和组织单元无关；本轮没有换算规则authority。 | quantity member `unitId`; `MeasurementUnitId`; `measurement.unit` / 计量单位 | keep-qualified；禁止裸unit跨领域映射，conversion unavailable until authenticated D10 contract。 |

接受intake的任意普通Node任职target：`people/engagement`保留Person侧唯一作者事实，NodeRef目标采用显式ordinary_node条件。Organization Facet仅增强组织专属投影，不是基本引用资格；通用inverse从实际NodeRef派生。任职组织目标是作者角色名称，不证明目标已分类为Organization；不自动创建组织、Assign Facet或双写membership。

关系target arm逐项处分：parent/spouse/sibling/guardian/family-related/social-related/professional-relation/manager/mentor均允许NodeRef和text。literal为逐字保留的作者事实，不产生target resolution、inverse、graph或自动建Node；family-related/social-related/professional-relation的NodeRef为symmetric general；parent/guardian/manager/mentor为明确directed，后两者为ranked。`library/venue`为Work指向书目Work/container或适当Organization的有向关系；期刊是Work，出版机构是Organization，venue不等于publisher或creator。wire `venue`、领域名 Publication Venue、UI“发表载体/场所”对应本合同；`publisher`不得作为其读写alias或自动导入映射。

配偶关系 / Spouse Relationship：一条作者关系事实对应一个occurrenceKey；同一Person可保留多条有独立validity/provenance的关系声明。历史配偶不被后续声明替代；“当前配偶”是有依据的投影，不等于最后一项或唯一可存项。D4未引入全局同时排他的婚姻规则。

## 6. People事件与重要日期用语

Life Event Assertion / 生命事件断言是现有Event Assertion在`people/life-event`中的领域使用，不增加另一种identity或carrier。wire alias `people/life-event-value`提供preset/custom两分支；`eventCode`只表示birth、death、employment-start、graduation、marriage的稳定预设语义，`customLabel`是逐字作者标签，不能当作code或解析别名。UI可将这些预设显示为“出生、死亡、入职、毕业、结婚”，自定义显示作者label；不得由locale文字改变wire分支。

Important Date / 重要日期是用户用途，不是新Field或Node kind；Birthday / 生日、Death Anniversary / 逝世纪念日及其他Anniversary / 纪念日是对已选事实和版本化规则的派生称谓。出生/死亡事件时间属于`qualifiers.eventTime`，关系期间起点属于`qualifiers.validity.start`；后二者不能因界面都显示“日期”而互为wire alias。Employment Start / 入职事件不等于所有Engagement / 任职与隶属的开始；Marriage / 结婚事件也不自动等于Spouse Relationship / 配偶关系期间起点。当前候选不新增birthday scalar、importantDate属性或Anniversary Node；该说明不扩展D3 identity集合。

## 7. Task分类与Facet闭包

Task / 任务沿用D2：ordinary Node在当前来源中显式声明exact tasks/task。Declared Facet Set / 声明Facet集合对应源NodeFacetSet，Effective Facet Closure / 有效Facet闭包包含requires派生；后者不得重新定义前者或生成另一个Task谓词。wire `coreKind`使用ordinary/template，`declaredFacetIds`保持源声明，`effectiveFacetIds`仅为经验证的派生读取。扩展Facet依赖tasks/task不构成源Task声明；同样，Task Field、Task UI或非空status不是Task身份。D4没有新增Task Node kind或isTask作者属性。

## 8. Account服务与作者标识

以下为现有People Field中的限定用语，不增加Core实体、Locator或全局服务Registry。Account Identifier / 账号标识对应people/account.value.members.identifier的preset/custom TypedUnion。Preset Account Service / 预设账号服务是preset external_identifier.scheme，使用受限SemanticCode式scheme ID及本地化显示；Custom Account Service Key / 自定义账号服务键是custom.value.members.serviceKey的non-empty exact作者文本，不能充当可信scheme ID或身份匹配依据。Custom Account Identifier / 自定义账号值是同一custom对象的identifier成员，与serviceKey分离。Account Usage / 账号用途是外层usage语义码，Account Display Label / 账号显示标签是外层customLabel，Account Note / 账号备注仍唯一属于Entry.note；这些名称和字段不可互作wire alias。custom文本“wechat”不自动获得预设微信语义，切换locale不改变作者服务键、账号值、显示标签或备注。

## 9. Revision30 人物关系的类别与断言

People relationships／人物关系是编辑区名称；family/social/professional是Field所属类别，不是布局指令。Relationship descriptor／关系描述由`relationship` union承载：preset为该Field的exact contribution code，custom为作者原文标签。Extended relative／一般亲属关联、friend／朋友、acquaintance／相识者、classmate／同学、asserted colleague／作者断言的同事关联均是general关联；derived colleague／由任职重叠派生的同事没有另一份作者权威。Guardian／监护人、manager／上级、mentor／导师分别是明确的Field角色，ward／被监护人、direct-report／下属、mentee／学生或受指导者分别是声明的inverse code显示；不能从自由称谓或类别猜测角色。Custom relationship label／自定义关系称谓不是note、target label、SemanticCodeId或全局关系类型名称，不触发翻译回写、自动注册、方向切换或Family Tree/rank推导。

## 10. Organizations 关系角色与方向

以下名称限定于 Organizations schema pack 的关系断言，不新增 Core Entity kind 或全局关系类型注册表。表中的 inverse 是派生投影语义码，不是另一条作者事实。中文显示名不参与 wire 标识匹配。

| 限定关系名 | 作者 FieldId | 主体 → 目标 | 派生 inverse code |
|---|---|---|---|
| Primary Organizational Affiliation / 组织主要隶属 | `organizations/parent` | 隶属组织 → 主要上级组织 | 沿用 parent 的既定 inverse |
| Organizational Business Guidance / 组织业务指导 | `organizations/business-guided-by` | 接受业务指导的组织 → 业务指导机关 | `organizations/business-guides` |
| Organizational Territorial Administration / 组织属地管理 | `organizations/territorially-administered-by` | 受属地管理的机构 → 属地政府组织 | `organizations/territorially-administers` |
| Explicit Organizational Joint Leadership / 明确的组织共同领导 | `organizations/jointly-led-by` | 共同领导安排下的机构 → 一个参与领导的组织 | `organizations/jointly-leads` |
| Organizational Supervision / 组织监管 | `organizations/supervised-by` | 受监管组织 → 监管组织 | `organizations/supervises` |
| Corporate Subsidiary Relationship / 企业母子公司关系 | `organizations/subsidiary-of` | 子公司组织 → 母企业组织 | `organizations/has-subsidiary` |
| Independently Referable Brand Affiliation / 可独立引用的品牌归属 | `organizations/brand-of` | 明确建立身份的品牌组织 Node → 关联企业组织 | `organizations/has-brand` |
| Organizational Alliance Membership / 组织联盟成员关系 | `organizations/member-of` | 成员组织 → 联盟或协会组织 | `organizations/has-member-organization` |

Business Guidance 中的 business 指业务或专业指导，不限定商业经营；Territorial Administration 是机构与属地政府的关系，不是地点属性或结构父级。Joint Leadership 必须有明确作者断言，不能由业务指导与属地管理两条关系自动推导。主要隶属的单目标约束不限制同一机构的其他关系类型。`governs` 的一般治理、`owns` 的所有权、`allied-with` 的对等合作与 `related` 的中性关联均保留既定含义，不作为这些新增限定关系的解析别名。

Brand 文本标签不自动建立组织身份；只有用户明确将品牌作为可独立引用的组织 Node 时才使用 brand-of。Member Organization 是组织成员，不是 Person、员工名册或 Engagement 的替代名称。关系类型独立于 validity、status 与 note；停用编辑 UI/runtime 不改变已提供的 schema 语义，实际贡献不可用仍遵循候选的 availability 规则。这些限定名称不定义跨国家机关体系、全局职级或隐含法律归属。

## D6组合修订的技术投影归属

保持上述所有concept ID、正式双语名、locale及排除边界。D3的existingPayloadEdits、potentialChanges、artifactBinding、companion_effect、fresh_annotation_reply和Result/9 C是Operation/identity-preserving effect与Payload Subject的内部closed计划技术投影；不新增内容identity或用户实体术语。C内FieldId/OccurrenceKey/typed根与carrier沿D4 Field Value Occurrence和D2 lexical carrier概念归属；不能把C或Entry称为新的Reference Slot/Entity/Locator。

D4SourceMaterializationEffects/1及d4_source_materialization_effects是D4 Field Value Occurrence现有概念的原子源效果技术投影，与D4RelationCopyEffects/1并列；准确schema由D4完整决议§15拥有，D3 v11 §4.1.3a拥有前验绑定。代码类型用D4SourceMaterializationEffects，内部member sourceMaterializationEffects；无新CLI/UI/locale alias，用户仍看到原字段值与操作效果。新技术名称首次适用本次D6 coordinated replacement，既有v9/Result7历史证据不删除；实施发布前当前closed schemas统一迁至wire11/Result9；原v9/v10仅按各自原decoder重放或恢复已保存决议，不接受新旧版decision。负向门禁应拒绝把这些技术计划句柄导出为RecordRef/EntryRef或第二作者源。



关系读取版本2采用source_node_state、sourceless_node_state、masked_node_state、unprovable_node_state闭集。stateToken/entityStates是D4 RelationReadBinding对D6受保护DependencyProof的技术映射，不是作者内容、身份或SourceVersion；无新增manifest/CLI/UI术语，代码类型RelationReadContext/RelationReadBinding维持D4所有权，版本2成员按正文exact shape。源、实体状态、incidence分别完整绑定；所有source-bearing、sourceless、prepared-origin边界都要覆盖，不能给tombstone填空分类。新Facet request/outcome及关系模型版本2必须同步；Entry、Recurrence及source/copy effect原版本不变。旧模型运行只证明历史形状。

实施验收：directed旧target purge后，真实旧source selector+state+完整incidence允许显式删除/retarget与D3 restore合成；新target必须live/domain，owner实际requiredness及同源一次revision保持。symmetric purge须原子移除全incident；无源owner携带Entry拒绝。源不变lifecycle ABA、state token跨Ref/跨版本/缺失/伪造、masked状态、prepared-origin冒用、负范围分配、incidence并发、每个失败点零写回滚，均独立覆盖。分类投影、四种Facet动作、general update、C/fresh copy桥和完整effects只能使用新版读取，tombstoned端点不得进入source effects。

当前D6消费引用ObservationScope、IssuerControlPolicy、WorkspaceBootstrapPlan、WorkspaceBootstrapProfile、CalendarPeriodScopeBinding，概念及所有权以同包D6完整机器术语表为准；scope不是新Field/Node identity，bootstrap不是第二decision，四种profile不得成为裸通用Profile产品别名。D3 observationScope已有局部conformance计数成员仍保持其原小写局部含义，不作为新公共类型导出；D6计划内该成员由明确schema分域。

## D7联合revision03消费补充（未接受）

当前协议基线为D3 wire11/Result9；D4 C及全部领域类型语义不变。新增受控名称和exact位置：PreparationBinding=D3.identity_operation_request.preparationBinding；DefinitionTransfer=D3.intent.plan.definitionTransfers[]；D3-Symbolic-Result/9.Q（定义结果分段）=D3-Symbolic-Result/9的Q；PreparedActionBinding=D7受管不可变准备输入；SourceEnvelopeStateCapability/CommitSequenceStateCapability=D6 Policy/2.capabilities；EffectManifest/EffectBytes=D7只读效果运输。它们均非内容实体或第二ledger。完整定义以同包D3主稿§21、D6 Control Interfaces §15–16及D7专项正文为准。

必须新增实际版本解码/legacy replay、fingerprint差异、planned恢复、保存定义typed引用转移/Locator、完整效果及窄Field正向outcome测试；历史wire10与Result8测试保留历史归属，不以数字替换声称新版本通过。其它原实现影响和必需检查不因本补充被删除。
