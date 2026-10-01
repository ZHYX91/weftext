---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：6e66067a-08b5-4c21-9791-45aaa6a50323。

# D4 属性类型、Schema 与关系

候选状态：D6-FA-r01；部分联合候选；未接受、未激活、未实现。固定 S 的 D4 revision37-preservation-01 只作为来源和兼容历史。本文件是 D4 的完整候选 owner 后像；只有与 D1/D3/D5/D6 及后续 D7/D8/D9/D10 消费者共同接受后才可激活。本文件不授权产品实现、A2、发布或 catalog 变更。

固定来源：
- S=7e18168dad3e6d120fce0dd607dc10fa7894e252
- source blob=2c03f2050dc523ccd8ffd3696f89f91a3b9c3c07
- unchanged catalog blob=ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb

## 1. 范围与唯一 owner

D4 唯一拥有 Semantic Namespace、Field/FieldId、Entry/1、
  TypeSpec/Typed Value、Field Semantic Shape、qualifier/provenance、
  FacetSchema/effective closure、relation 方向/inverse/canonical owner/cardinality/delete policy、
  Calendar typed value/comparator/recurrence/series-scope、
  RegistrySnapshot/RegistryBinding/schema evolution，以及 D4 semantic validation/diagnostic/effect extensions。

D4 不拥有 Document bytes、Node/Resource/Annotation identity、parent/order、文件安装、SQLite、
  Query complete cut、Editor Draft、ImportJob、Automation、Extension runtime 或 credentials。
  当前 bytes、portable metadata、P/I、SourceVersion/2、SourceVersionRef/1、CommitDomain/2、Frontier/2、SemanticState/1 和 ConflictRecord 直接消费当前 D6 owner types；identity/lifecycle 继续消费 D3 wire12。历史 H2 D6/D3 saved decoder 按原版本和 retention 规则重放，不机械替换历史字段版本。D4 不拥有 Document bytes、SQLite P/I、identity/lifecycle 或 current body mirror；I 仍可重建，P 仍保存不可由 source 文件重建的 durable decision/control state。

P/I cache、Registry cache、relation index、Calendar/Graph projection 和 UI property model 都不得成为第二份作者真相。

## 2. Semantic Namespace 与 Registry

### 2.1 reserved owner tuples

reserved tuples 原样保留：

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

`wf` 永远不可分配；`user` 绑定 current WorkspaceId。复制库或注册新 replica 不产生新的 namespace owner，也不把 first-party namespace 转成 user namespace。

### 2.2 RegistrySnapshot / RegistryBinding

RegistrySnapshot 是 portable shared configuration 的 D4 semantic truth；D6 只负责其物理承载、版本与权限。一个受信 snapshot 必须完整验证 namespace ownership、Field/Facet/QualifierSet/alias closed shape、
  contribution tables、alias closure、requires/conflicts graph、
  active definitions、tombstone/migration ledger、semantic digests 与 catalog limits。

RegistryBinding继续绑定 exact generation + snapshot digest；不能只比较 generation 名。一次 decode/operation只使用一个固定 snapshot，不得中途 refresh。

Registry evolution保持 monotonic ledger：同 ID major-1 definition digest必须 byte-equal；
  breaking change使用 fresh ID + exact tombstone + migration。
  复制 portable Registry bytes只复制 shared configuration事实，不赋予 replica 推进 Registry generation 的管理权；Registry mutation仍要求 current Policy、Registry admin资格和适用 D6 control/execution responsibility。

### 2.3 Reference Catalog v1

固定 catalog继续逐字定义 4 QualifierSetSpec、22 aliases、61 FieldDefinition、7 FacetSchema 和 1 CalendarSeriesScopePolicy。本批不创建 catalog replacement。

limits原样保持：

| limit | value |
|---|---:|
| maximumTypeDepth | 8 |
| maximumObjectMembers | 64 |
| maximumUnionVariants | 8 |
| maximumCollectionItems | 256 |
| maximumEntryUtf8Bytes | 65528 |
| maximumSchemaUtf8Bytes | 65536 |
| maximumProvenanceAtoms | 16 |

catalog任一结构/语义失败使整个 catalog unavailable，不做“部分成功”。

## 3. Entry/1 与作者 source

D4 Field Value Occurrence 始终来自 owning Node 的 D2 exact-source Document。
  Attribute Carrier Block/Lexical Attribute Entry 是 current-revision lexical occurrences，
  不是 Entity、Record、sidecar 或 DB row。

D4 Entry/1 保持 closed semantic members：version、FieldId、
  occurrenceKey、typed value、optional qualifiers、optional note、
  optional provenance。unknown/duplicate member、wrong constructor、
  非法 UTF-8/JSON、budget 超限或 Field/schema mismatch 都按原 diagnostic顺序拒绝。

occurrenceKey 只是 owner Node + FieldId + expected current source revision 内的 value-internal selector；
  不是 EntityRef、Locator、OperationId、RecordRef 或 cross-revision identity。相同 value 可以有多个 occurrence；外部 edit/reorder/delete+reinsert 或 SourceVersion 生产 epoch、revision、externalSequence 变化后不能仅凭 key/value 续认。当前 D6 外层 SourceObservation/1 资格验证作为附加保护，不能改变 occurrenceKey 原有基于 owner Node + FieldId + expected current source revision 的 selector 形状。即使 production version 相同，当前 SourceObservation token/epoch 在 watcher gap、external replacement 或 discontinuous materialization 后失效，也不能恢复旧 occurrence selection。

Inline Field Note 是 optional plain author text；无内容不写空占位。typed qualifier/provenance不能塞进 note后再自然语言解析。

raw Entry、carrier framing、comments、line endings、
  unknown namespace bytes和未选中 occurrences必须按 D2/D4 lossless规则保持。
  structured mutation绑定完整 before source、selector/span、
  current SourceVersion/2 与 RegistryBinding；相同 hash 不证明 continuity 或 ABA。

## 4. TypeSpec、Typed Value 与 Field Shape

fixed-S constructor集合原样保持：text、Boolean、integer、decimal、
  semantic_code、calendar_date、zoned_instant、date_range、instant_range、
  quantity、node_ref、resource_ref、annotation_ref、external_identifier、
  closed object、closed union、bounded ordered collection与 alias_ref。

D4 integer是 canonical signed decimal string，与 D3Integer不同。decimal禁止 exponent、NaN、Infinity、
  negative zero与多余 fractional zero。
  ResourceRef/AnnotationRef owner必须等于 containing owner NodeRef；D4不做 ambient owner补全。

calendar_date只用 verified calendar/version/precision comparator。内建 `calendar/iso8601` v1继续验证真实 Gregorian date 与 year 0001..9999。zoned_instant保留 exact RFC3339 + IANA timeZone/tzdbVersion；不使用设备默认 locale/timezone。date_range/instant_range仍 end-exclusive、至少一端有界，两端有界时严格 start < endExclusive。

quantity保持 exact decimal + namespaced unitId；没有 verified conversion contribution时不做单位换算。

Field Semantic Shape closed为 fact、event_assertion、observation、relation。shape决定合法 qualifier/projection；事件、状态、观测、关系不能因为 UI 都像一行而压成无类型 list item。

## 5. FacetSchema

D2 source `facetMemberships` 是 declared membership唯一作者源。D4将其解释为 unordered exact FacetId set；source order无 precedence/override/last-wins。effective closure只由当前 Registry requires graph机械派生，可删除重建，不回写作者 source。

missing dependency、cycle、conflict 或 incompatible Field definition使相关 typed state unavailable/invalid并阻断触及操作。
  每个 FacetSchema closed声明 fields、relations、requires、conflicts、constraints；不存在 implicit membership、priority 或 shadow。

当前 Task规则保持：ordinary Node + source-declared exact `tasks/task` Facet 才是 Task；Template仍是 D2/Core meta-kind，Template + exact tasks/task冲突。D4不恢复旧 specialization 双权威。

显式 Assign/Remove/Cleanup Facet是强 typed Action：需要完整 source/Registry、
  declared/effective closure、相关 Field requiredness、
  相关 relation incidence、current Policy/auth 和 exact proposed post-state。
  ordinary save允许 pending不等于 Facet Action可降级。

## 6. typed availability 与 D6 SemanticState

### 6.1 Namespace 与 Node typed state

namespace typed state closed为：

- `available`：owner/schema/Entry syntax/type/constraints可证明；
- `retained_unavailable`：owner/schema/contribution unavailable或不兼容，raw source保留，不能投影为空；
- `invalid`：schema已知但 Entry/type/cardinality/key/constraint/relation非法；
- `not_present`：exact source确实无该namespace。

Node typed snapshot closed为 `complete|partial|unavailable`。partial必须携带逐namespace状态；omitted namespace不能当empty。D2 exact source不因D4 typed state改变。

### 6.2 四个正交维度

| 维度 | owner | 状态例 | 不可替代 |
|---|---|---|---|
| current bytes | D6 | managed / external / external_invalid | D4 typed validity |
| local typed projection | D4 | namespace四态；Node complete/partial/unavailable | Workspace完整约束 |
| save semantic completion | D6 | complete_semantics / semantic_pending | D7 complete result |
| Query/Action cut | D7 | future versioned complete/partial evidence | D4 local validity |

unknown/pending/unavailable不能转换成 empty value、empty relation、zero members 或“全库无匹配”。

## 7. operation-applicable proof matrix

每个操作分别证明：current source/SourceVersion/2；RegistryBinding/touched definitions；actual MutationFootprint；local Entry/Facet closure；真实需要的正负跨对象范围；unique proposed post-state；current Policy/permission；D6 CommitDomain/Frontier/install；以及仅在强consumer需要时的 D7 complete cut/preparation。本地可证明项不能因 pending省略。

| operation | local proof | complete proof | result |
|---|---|---|---|
| 精确 source 读取/修复 | D2 bytes + 披露；typed 可为 partial/unavailable | 无 | raw + 明确 status |
| 仅正文编辑 | D2 parse；D4 carrier 前后 byte-equal；Facet closure 不依赖；write-set 不相交 | 无关 namespace 不全扫 | complete 或真实 pending；不可改 unavailable/invalid namespace |
| edit另一available namespace | touched Entry/Facet完整local validation；其它 unavailable/invalid raw byte-equal | 未触及跨对象可不读 | 同上 |
| edit affected unavailable/invalid namespace | 仅repair/remove/cleanup完整计划 | 依实际计划 | ordinary typed edit拒绝 |
| D3 replica_local create_node | 新 source 完整通过 D2+D4 局部 typed/Facet | relation/unique/calendar/cross-object 可成为待证义务 | semantic_pending 可保存；不等于强 D4 Action |
| D3 replica_local move/reorder | 仅实际predicate/permission需要的D4 facts | 不证明全局relation/collection | D3 local success可与pending并存 |
| D3 replica_local Trash | 局部 source/Facet/lifecycle policy | 缺 inbound/relation 全集形成待证义务 | semantic_pending 局部 lifecycle |
| non-relation Entry edit |完整owner source、Field、Entry、cardinality、qualifier/provenance、requiredness | Field无跨对象义务则无需全库 | ordinary可成功；其它义务仍pending |
| Assign/Remove/Cleanup Facet |完整local closure + relation incidence | 必须完整 | 缺范围reject/unavailable |
| 关系新增/更新/删除 | RelationReadContext/2 + Binding/2 | 完整 incidence/endpoint/domain/cardinality | 仅 complete |
| 局部 recurrence value 编辑 | 精确 TypeSpec/calendar/tzdb/source 后状态 | 无 series unique 时可局部 | ordinary 或 pending |
| series-scope unique/many | recurrence/period + policy | 完整 (series,periodKey,scope) 正负范围 | 仅 complete |
| 原始字节复制/导出 | 精确 source + status | 不声明 typed success | 字节保留 + loss/status |
| typed 复制/分叉/导入 | D3 wire12 映射 + D4 typed source | 完整 canonical-owner/requiredness | 仅 managed_atomic |
| restore/purge | D3 managed_atomic + current D4 facts |完整relation/inbound；purge另Frontier | 缺proof拒绝 |
| D7 all_result/bulk/automation 写入 | 局部 typed 只是必要条件 | 未来 D7 complete cut | D7 后像完成前 unavailable |

D6 obligation `relation|unique|calendar|inbound|cross_object_type` 表示未证明，不表示允许失败；它绑定产生 source 的 SourceVersion/2、CommitDomain/2、Frontier/cut、RegistryBinding、Policy和pins。

r6后来补验成功只证明r6；不能改写r5 receipt。删/重建I、hash相同、设备迁移、replica注册、外部A→B→A或observationEpoch变化都不会恢复old proof。

## 8. Relations

relation kind/direction/inverse/cardinality/resolution/delete policy/graphProjection由 FieldDefinition拥有，不建立开放 RelationType registry。directed relation只写source fact，inverse派生；symmetric relation只在两个 NodeRef canonical encoding决定的 canonical owner写一份事实。canonical owner不绕权限。

`node_ref_or_text` 的 text arm不做 target lookup/inverse；text→NodeRef是显式原子替换。

fixed-S closed shape保持：
- `RelationReadContext/2 = {kind:"d4_relation_read_context",wireVersion:2,registryBinding,nodeStates,entries,incidenceScopes}`；
- nodeStates closed union：source_node_state /
  sourceless_node_state / masked_node_state / unprovable_node_state；
- `RelationReadBinding/2 = {kind:"d4_relation_read_binding",wireVersion:2,nodeRevisions,entityStates,incidenceRevisions}`。

D6提供可信snapshot/stateToken/source revision/incidence revision/auth；client不能自报complete或empty incidence。

新 D6-FA 请求不改 D4/2 inner wire，而由 InputDescriptor/2 为每个 source-bearing owner 绑定 SourceVersion/2。Context owner 与 sourceInputs 一一对应；inner sourceRevision 必须对应实际 source-bearing owner 的生产 revision，并与该 SourceVersion 表示的版本一致。SourceVersion/2 保留自身的生产 commitDomain、observationEpoch、revision 或 externalSequence、changeId 语义；SourceVersion.commitDomain 可以不同于当前 operation 的观察域。当前资格由 SourceObservation/1 决定：observerDomain 等于 operation CommitDomain，entityRef 等于 sourceVersion.entityRef，当前 observationEpoch、fileObjectBinding、evidencePins、control revisions、Registry 绑定以及关系 incidence 依赖共同属于当前 observation cut。SourceVersionRef/1 的 sourceToken 使用 `d6_source_observation/1` 标记并选择完整受保护 SourceObservation，而不是裸 revision、digest 或生产 SourceVersion。即使 production version 相同，watcher gap、外部替换或不连续重物化也会使旧 token 失效。D4/2 内层 wire、legacy saved decisions、owner 双射、权限与授权 gate 保持不变。

relation gate保持：D3/D6 disclosure/auth → D4 closed context/binding
  → coverage → immutable cut → Registry/raw Entry/selector/revision
  → complete incidence → proposed state →
  domain/lifecycle/cardinality/Facet requiredness → D6 CAS → author commit。

masked/unprovable必要端点不能产生成功readSet。新/retargeted NodeRef target必须live且domain-valid；删除旧事实可以读取确定 tombstoned/not_found endpoint而不伪造Document。symmetric endpoint purge前必须显式清除全部incident facts。

D4RelationCopyEffects/1保持原shape与owner，不进入D3 ref-slot union。
  D3 wire12 typed copy/fork/import只有 managed_atomic + complete D4 proof才能消费；
  raw byte-preserving copy不能冒充typed fork。

## 9. Calendar

CalendarPeriod、Temporal Range、Calendar Event Semantics继续分域：period是calendar-defined周期；range是date/instant range；event是在range外增加status/recurrence/participant/reminder的Facet语义。View命中/跨日/title模式不自动Assign Event。

`calendar/recurrence-value` 保持date/instant同质分支；count与until互斥；RDATE/EXDATE/exception集合上限256。derived occurrence可重建且无Node identity；projection绑定真实Registry/calendar/tzdb/rule contributions、current source和显式horizon/budget，coverage不足不返回partial rows冒充complete。

Catalog唯一 `calendar/series-scope` v1保持 key=(series,periodKey,scope)，scopeKinds=node|workspace，multiplicity=many|unique。
  unique mutation需要完整正负range + current Registry/Policy/SourceVersion/Frontier；
  building I、当前page、provider cache或path collision都不能证明unique。

ordinary save可保存本地合法Calendar facts为 semantic_pending(calendar)，但不能声称strong“创建唯一period/event”Action成功。

## 10. catalog正向消费

fixed catalog `people/labeled-text-value` 保持：
- text required、exact、non-empty；
- label optional semantic_code，contribution-set codes为 `people/other|people/personal|people/work`；
- customLabel optional exact non-empty。

`people/phone`、people/email、people/address、people/website消费同一alias。label absent表示作者没提供label，不是empty、unknown、people/other或typed-unavailable；localized display label绝不回写为SemanticCodeId。phone alias必须先验证alias/Registry，再验证closed object，不能把同名label字符串猜成semantic code。

People各names/life-event/measurement/state/profession/engagement/relation仍是独立Fields；
  people只是namespace，
  不存在single people blob。engagement Node arm接受 ordinary Node，
  Organizations Facet只增强UI/query，不是NodeRef admission前提。

Organizations structural parent与organizations/parent分域；move不改组织关系，关系edit不move。identifier/classification不是identity；symmetric/directed关系保持各自canonical owner/inverse。

`library/work` 是 Bibliographic Work Facet，不是project work/task；
  DOI/ISBN/provider ID不是NodeRef，Citation occurrence不等于Work identity。

tasks/task仍required tasks/status；tasks/dependency两端Task domain必须完整证明。

## 11. diagnostics、权限与 source materialization

D4 diagnostic仍是closed envelope，无free-form details map。稳定排序sourceStart、rank、stable code。

preflight顺序：outer D6/D3 visibility/permission → Registry owner/binding/contribution → D2 carrier/span
  → strict JSON → Field/Facet → typed
  value/qualifier/provenance → operation-applicable post-state。
  Registry unavailable时不先解析inner JSON泄露schema；missing member不为不存在token制造span。

external strict UTF-8/D2 invalid属于D6 external_invalid repair，不包装成D4 semantic_pending success。

Field-level权限必须先静态证明可能write footprint，不得先读取hidden Field再判断。materialization后重验 actual footprint；所有未选中author bytes/comments/CRLF/unknown namespace保持。两个replica改不同Field仍可产生file-level conflict，必须由D6 ConflictRecord保留bytes/SourceVersion；D4 semantic merge proposal不能绕file CAS。

最终 commit eligibility仍为：D2 eligible AND operation-applicable
  D4 gate AND D6 gate AND applicable D7 gate。

## 12. I、P、partial index 与 r5/r6

I只缓存Registry parse、typed projection、incidence、Calendar projection或search candidates；删除重建不创建新D4 validation decision。

P丢失不能从current source猜old strong Action/readSet/pending validation。r5如果pending，r6后来complete只产生r6/current资格；重放r5仍返回原bytes/status。

partial index可以在明确权限下显示 exploratory local projection，并标partial/pending；但不能满足 relation negative incidence、
  unique negative、Calendar unique、D5 collection membership、D7 all_result/bulk或Automation write。

## 13. 下游与兼容

D5消费真实Field Value Occurrence/Document Table/Node Collection域，不能把D4 occurrence变Record identity。

D7未来冻结complete cut、Prepared新版、Search；D4不私造 PreparedActionBinding/3，也不从历史 /1,/2 生成新版success。D7 afterimage前，需要其complete proof的D4 Action unavailable。

D8 UI label不是FieldId；D9 import/export不按label猜Field/semantic code；
  D10 runtime/credentials/execution responsibility不因 portable Registry复制而转移。

D4 Entry/1、TypeSpec、catalog v1、RelationReadContext/2、RelationReadBinding/2、RecurrenceReadContext/1、D4RelationCopyEffects/1、
  D4SourceMaterializationEffects/1保持原semantic shape。新 D6-FA consumer只在outer InputDescriptor/2加入SourceVersion/2/CommitDomain/Frontier binding，不重编码旧saved decisions。

## 14. 强制场景与验收

fixed mandatory scenarios只作为pressure obligations，不是功能批准。必须保留People labeled entries、Organizations relation/structural parent、
  Calendar period/range/event、ICS foreign identity、Library Work、
  unknown provider、multi-replica conflict、术语一词一义和no-second-authority边界。

未来测试至少覆盖：
1. 离线body edit + unavailable namespace byte-equal成功；
2. body/edit触及unavailable/invalid namespace拒绝；
3. local Entry type/cardinality/requiredness失败即使ordinary也拒绝；
4. local valid edit但relation/unique/calendar完整range缺失只能pending；
5. relation缺incidence negative range拒绝；
6. Calendar unique缺complete series range拒绝；
7. partial I只允许partial探索，不all_result/automation write；
8. external D2-invalid保留repair、不D4 success；
9. A/B离线冲突不LWW；
10. r5 replay与r6 current分开；
11. rebuild I不恢复proof，lost P不重建decision；
12. people/phone optional semantic-code label正例；
13. symmetric relation canonical owner只有一份事实；
14. D3 wire12 copy/fork缺complete D4 post-state整次拒绝；
15. legacy saved decoder bytes不变。

这些是未来验收义务，不是已通过产品测试或性能秒数声明。

## 15. 候选接受边界

这是作者修订候选，不是独立接受。catalog仍是固定S原blob，不登记replacement。后继immutable candidate仍需fresh独立联合审查与项目协议协调接受。

旧D10 B13保持REVISE、术语/双语FAIL、P1=3、P2=8、共11 OPEN；本文件不关闭或重分类任何finding。

## 16. 规范性 exact-contract 恢复

本节是本后像的规范组成部分，用来恢复 fixed-S 中不能被摘要化的 exact wire、预算、诊断和正向接受路径。若前文概述与本节 exact shape 有任何张力，以本节 exact shape 为准；D6-FA-r01 只改变 outer storage/version/qualification binding，不修改这些 D4 内部 wire major。

### 16.1 Entry/1 outer binding、closed envelope 与 carrier

D2 carrier bytes保持原样，D4只解释 rawEntrySource。规范示例：

~~~adoc
[weftext-attributes]
....
namespace people
entry {"v":1,"field":"name","occurrenceKey":"5e6d8b56-8f22-4b97-a6e1-1aacbf2f9a78","value":{"kind":"object","members":{"role":{"kind":"semantic_code","code":"ordinary"},"text":{"kind":"text","text":"张三"}}}}
....
~~~

payload必须是一个 strict-JSON object；duplicate member、
  trailing token、NaN/Infinity、JSON浮点number、
  unknown envelope key、missing required key、
  illegal null全部拒绝。object member source order没有语义优先级。

Entry/1 exact envelope：

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

FieldId展开固定为 namespaceToken + "/" + field。Entry不能重复声明namespace。多个同namespace carrier block按作者source order组成一个semantic stream，block没有identity或precedence。duplicate key的唯一性域是完整三元组：

~~~text
(owner NodeRef, expanded FieldId, occurrenceKey)
~~~

同owner同Field第二个相同key固定reject，即使value相同；不同Field可以复用相同key bytes。检查跨该Field在所有同namespace blocks的entries进行，不按block重置且没有last-wins。

occurrenceKey只在当前owner+Field+expected source revision内精确选择duplicate values的patch/reorder/note目标。mutation必须同时绑定owner NodeRef、FieldId、occurrenceKey与current SourceVersion/2；它不进入D3 EntityRef/Locator/AnnotationTarget，不可独立resolve/授权/跨owner查询。delete无tombstone/restore；同key重新出现是新source fact。Node move/rename可保留Entry bytes；fresh-owner copy/import可保留key bytes，但同owner+Field内复制occurrence必须fresh key。D3 identityMap从不包含occurrenceKey。

普通D2 header attribute不因名称相似而成为D4 Field。显式Map Attribute Action必须预览exact source range、target FieldId、conversion、fresh occurrence keys、loss、schema依赖及删除/保留原source的选择；commit只留下一个chosen canonical target。flat export/cache永不是Weftext Document authority。

### 16.2 TypedValue/ValueTypeSpec、qualifier 与 provenance

每个作者TypedValue先检查string kind，再按下表closed decode；缺kind、非string、unknown kind、wrong constructor都不是成功value。所有object拒绝unknown/missing/illegal null，nested value递归使用同一规则。

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

set items按每个item的D3-CJ/3 canonical UTF-8 bytes unsigned byte-lexicographic升序且无重复；sequence保留作者顺序。任意深度的浮点JSON number与NaN/Infinity均拒绝。

ValueTypeSpec/1是closed tagged meta-wire：

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

递归depth从FieldDefinition顶层根记1；每次alias expansion、object member、union variant或collection item各+1，
  maximum=8。alias expanded root、完整expanded FieldDefinition与FacetSchema canonical UTF-8最大65536 bytes；
  raw Entry JSON最大65528 bytes；object members 64、union variants 8、collection items 256、provenance atoms 16。计数在大分配/转换前完成，不能借wrapper缩小expanded root。

所有schema meta-wire整数先用D3Integer 0..2^63-1且Boolean拒绝，再应用较窄范围。CardinalitySpec/1 exact：

~~~text
{minimum:0|1,maximum:<positive D3Integer|"many">}
~~~

FieldDefinition.cardinality.minimum与directed RelationDefinition.targetCardinality.minimum在v1规范值均为0；business requiredness只由FacetConstraintSpec负责。

FieldConstraintSpec/1闭集：

~~~text
{kind:"at_most_one_preferred"}
{kind:"mutually_exclusive_members",
 members:[2..64 sorted unique ObjectMember names]}
{kind:"measurement_unit_dimension",
 codeMember,quantityMember,byCode}
~~~

FacetConstraintSpec/1闭集：

~~~text
{kind:"required_field",fieldId,when:"effective"}
{kind:"union_variant_equal",
 leftFieldId,rightFieldId,when:"both_present"}
~~~

union_variant_equal要求两侧所有已验证occurrences共同使用唯一同一variant；相同“variant集合”仍不够。Create/Assign必须满足effective requiredness和branch equality；删除最后一个effective required occurrence拒绝；Remove Facet结束其要求但保留Field为retained_without_membership。

Field shape与qualifier set固定映射：

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

QualifierSetSpec IDs固定为 d4/fact-qualifiers-v1、d4/event-assertion-qualifiers-v1、d4/observation-qualifiers-v1、d4/relation-qualifiers-v1；
  qualifier object closed，missing required、unknown、null、wrong type拒绝。
  relation status使用registry-backed完整SemanticCodeId，不另带Field-local codeScope。

provenance atoms闭集：

~~~text
{kind:"node",nodeRef,locator?}
{kind:"resource",resourceRef,regionLocator?}
{kind:"external",scheme,value,observedAt?}
{kind:"transform",inputIndex,operationId}
~~~

node/resource locator必须与其ref byte-equal owner绑定；
  transform inputIndex是zero-based D3Integer且必须指向同array更早atom；
  operationId是canonical lowercase RFC4122 UUID。
  credentials、provider account、sync token、etag、cursor、SourceBinding、
  ForeignIdentityKey、OriginBinding不进入portable Entry provenance。

### 16.3 FieldDefinition/1、FacetSchema/1 与 FacetOperationRequest/2

FieldDefinition/1 exact示例：

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

非relation Field恰为上述11 keys；relation Field额外required relation且shape=relation。
  duplicatePolicy v1唯一允许key_unique_values_may_repeat。
  FieldDefinition独立于Facet membership可解码；Remove Facet后known Field facts为retained_without_membership。

FacetSchema/1 exact：

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

FacetId+semanticMajor=1绑定完整semantic digest；同ID后续revision不得改变requires/conflicts/fields/relations/constraints。breaking change使用fresh FacetId+explicit migration。数组按D3-CJ/3 bytes排序唯一，fields与relations不重叠；requires graph无环，conflicts无self且对称。catalog最多1024 Facets，D2 declared set上限仍32。

Task分类必须来自同source revision的D2 coreKind=ordinary + source-declared exact tasks/task；
  effective-only Task不成立。
  Template + explicit tasks/task仍非法。
  source_node_state中的coreKind/declared/effective必须同source/Registry证明，不能caller补isTask。

FacetOperationRequest/2 exact：

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

operationKind闭集：
create_with_facets | assign_facet | remove_facet | cleanup_facet_fields

成员适用：

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

不适用member必须exact null/[]，不能省略或带内容。pre-state exact：

~~~text
{nodeRef,coreKind,declaredFacetIds,entries,
 ownerRevision,registryBinding}
~~~

outcome exact：

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

成功existing source变更只增加一次owner revision；trusted D3 fresh create验证完整result revision=1而不是再加到2。Remove只改declared membership；Cleanup只删request列出且已证明unused的occurrences。失败返回byte-exact pre-state、空write set、null成功read sets且无intermediate state。

### 16.4 relation exact contracts

RelationDefinition/1只有两种exact member set：

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

EndpointPredicate闭集：

~~~text
{kind:"ordinary_node"}
{kind:"facet_any",facetIds:[1..8 sorted unique FacetIds]}
~~~

targetDomain仅node_ref|node_ref_or_text并必须与targetMember展开type逐字一致。
  text arm是literal，不resolution/inverse/graph/target cardinality/lifecycle；
  node_ref-only relation拒绝text。directed inverseCode必须命中同Registry semantic-code contribution；
  symmetric selfEdgePolicy仅accept|reject，endpointPurgePolicy固定explicit_remove_before_either_endpoint_purge。
  resolutionPolicy固定live_required_on_create_retain_suspended，deletePolicy固定retain_fact_explicit_cleanup。

symmetric NodeRef fact只写一份：两端nodeId的D3-CJ/3 canonical UTF-8 bytes unsigned compare较小端为canonical owner；self-edge按policy。canonical owner变化是原子 remove-old/add-new，同occurrenceKey保留，其余Entry members/raw order保持；失败必须byte-exact rollback。

RelationReadContext/2 exact：

~~~text
{kind:"d4_relation_read_context",
 wireVersion:2,
 registryBinding,
 nodeStates,
 entries,
 incidenceScopes}
~~~

nodeStates closed union：

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

entries每项是真实 source fact：

~~~text
{ownerNodeRef,fieldId,entry,rawEntrySource}
~~~

incidenceScopes每项：

~~~text
{fieldId,endpointNodeRef,revisionToken,factSelectors}
factSelector := {ownerNodeRef,fieldId,occurrenceKey}
~~~

RelationReadBinding/2 exact：

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

masked/unprovable不能产生成功binding/readSet。expected binding必须逐项exact equality。D6-FA-r01额外要求outer InputDescriptor/2对每个source-bearing owner绑定完整SourceVersion/2与当前 SourceObservation/1 资格；inner sourceRevision与实际source-bearing owner及该managed SourceVersion代表同一生产revision。SourceVersion.commitDomain保持生产域定义，可以不同于当前operation观察域；当前 SourceObservation.observerDomain 必须等于operation CommitDomain，且entityRef、observationEpoch、fileObjectBinding、evidencePins、control、Registry与relation-incidence依赖属于同一当前proof cut。

relation operation model继续是：

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

injectFailureAt仅conformance harness，不能成为产品自授权字段。sourceRevisionOwners只映射source-bearing inventory；sourceRevisions与expectedSourceRevisions键集完全相同并用D3Integer exact比较。

关系公共gate的顺序固定为：outer disclosure/auth → closed context/request/binding → source/sourceless coverage → immutable cut/binding equality → Registry/raw Entry/selector/revision →完整旧/新incidence →唯一proposed state → subject/target domain+lifecycle+cardinality+Facet requiredness → D6 Policy/version CAS → author commit。masked/unprovable必要端点失败；new/retarget target必须live；删除旧事实不重新接受旧target，但必须证明真实before与完整incidence；保留directed tombstoned/not_found事实只能按retain_fact_explicit_cleanup保留，不能作新域断言。symmetric endpoint purge前所有active incident facts显式清除。

D4RelationCopyEffects/1 exact：

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

referenceChanges只列实际变化的typed ref/locator根，pointer是以$为根的RFC6901语义路径，按UTF-8 bytes排序，根不重叠。owner迁移由owner字段表达；occurrenceKey不是D3 ref。effect不替代D3 identityMap/lifecycle/placement/authorization/receipt。

D4SourceMaterializationEffects/1 exact：

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

sourceBytes是canonical no-padding base64url exact UTF-8 bytes；
  before absent只用于同D3 receipt fresh Node。
  owners覆盖C-carrier全部result owners和实际改写existing source containers。
  entries按source subject key、FieldId、OccurrenceKey排序唯一；beforeRawEntrySource仅fresh initial Entry可null。effect与D3 request/candidate-map/receipt和D6 decision同一原decision保存；它不是第二作者源。

### 16.5 Diagnostic/1、sourceRange 与完整错误聚合

D4 diagnostic envelope exact：

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

同source point rank/code family：

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

排序key固定(sourceStart, rank, stableCode)。namespace owner不可证明时rank10在inner JSON解释前获胜；owner/schema可用后必须用同一个strict parser+catalog resolver+recursive typed validator收集所有不跨authority mask且由当前bytes可判定的独立fault，不能第一个structural/type错误就short-circuit。一个availability-failed child只遮蔽该child，独立sibling/note/qualifier/provenance错误继续聚合。

strict parser保留每个key/value/array/scalar最窄half-open UTF-8 byte span；dynamic key path用RFC6901 escaping（~→~0，/→~1）。每个诊断sourceStart取最窄fault token起点。missing nested member没有token时保存语义pointer并使用最近实际存在container object的完整half-open span；不能制造zero-width span。

Entry-local span必须通过 exact projection绑定Document revision：

~~~text
D2SourceProjection/1 =
{kind:"d2_source_projection",
 documentRevisionToken,
 entryStartUtf8}
~~~

documentRevisionToken不能从“当前文档”隐式补。projection把Entry-local UTF-8 half-open span变为该绑定source revision的absolute range。

strict JSON object若缺required envelope member，只产生一项invalid_entry_json定位整个envelope span；多个缺项也只有这一项envelope错误。存在且可独立解释的version/Field/value/qualifier/provenance/note分支仍按availability与version门继续收集；raw完全不能strict parse时只有parse failure，不伪造nested fault。

所有authority/availability结果在object/union/collection/range/qualifier/provenance wrapper内保持原code/state，只有已证明available的child structural/type failure才归一为invalid_value/invalid_qualifier/invalid_provenance。diagnostic masking从不产生TypedValue成功。

### 16.6 Calendar recurrence、selector、projection 与 series-scope exact contracts

calendar/period-value中的seriesKey是required exact TypedText；
  空字符串合法，missing/null/non-string拒绝。CalendarPeriod/1 exact members：

~~~text
calendarId
calendarVersion
timeZone
tzdbVersion
periodKind
periodRuleId
periodKey
~~~

periodKind闭集day|week|month|quarter|year。series identity exact tuple：

~~~text
(calendarId,calendarVersion,timeZone,tzdbVersion,
 periodKind,periodRuleId,seriesKey)
~~~

period identity再加periodKey。title/path/locale从不参与。

calendar/recurrence-value是closed date|instant union。每个rule object required：
profileVersion=TypedInteger "1", anchor, frequency, interval。
optional：
weekStart, byMonth, byMonthDay, byWeekday, count, until, rDates, exDates, exceptions。
unknown member拒绝；没有从系统时钟/title/range/horizon/default timezone补值。

profile1 frequency闭集daily|weekly|monthly|yearly，interval 1..65535。weekly weekStart缺省monday；monthly/yearly的selector默认规则按fixed-S定义。byMonth=1..12，byMonthDay=-31..-1或1..31，0非法，byWeekday为七个closed codes。不存在的月日跳过不clamp。instant arm用固定tzdb civil rule；DST gap跳过、fold取较早UTC instant，anchor保留原exact instant；不使用OS当前timezone或86400秒替代civil recurrence。

count为1..2147483647，与until互斥；count对去重base候选在EXDATE/exception前计数。until是inclusive base-start上界且同basis、不早于anchor。省略两者表示语义无限但执行必须有bounded horizon/work/output budget。

集合语义：

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

RDATE可在anchor前或count/until外且不消耗count。exception originalStart必须属于B∪R。temporal equality按exact semantic instant/date而非词法offset。

exception exact object：

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

replacement range与arm basis一致且ordered；title non-empty exact text；note exact text可空；eventStatus域cancelled|confirmed|tentative。replacement start可移日但originalStart不改；不同originalStart可映到同display start而仍是两个 occurrence。

recurrence selector exact：

~~~text
{ownerNodeRef,
 fieldId:"calendar/recurrence",
 occurrenceKey,
 originalStart}
~~~

它是revision-bound语义地址，不是D3 durable locator。

RecurrenceEditRequest/1 exact：

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

editKind闭集set_exception|remove_exception|edit_series。前两种使用originalStart且rebaseDecisions为空；edit_series时originalStart=null，并对每个旧exception提供exact rebase decision：

~~~text
{originalStart,disposition,nextOriginalStart}
disposition := keep|remap|drop
~~~

decision集合恰覆盖旧selectors；keep要求语义同一且仍属于新集合，remap明确新selector，drop要求nextOriginalStart=null且新source无该exception；多个decision不能映到同一新selector。rule/tzdb/basis变化没有完整rebase时零写失败。

recurrence edit pre-state exact：

~~~text
{nodeRef,coreKind,declaredFacetIds,entries,
 ownerRevision,registryBinding,lifecycle,
 recurrenceProjection}
~~~

lifecycle必须live。outcome exact：

~~~text
{kind:"d4_recurrence_edit_outcome",
 operationId,status,postState,writeSetOwners,
 readSet,projectionInvalidations,
 rollbackByteEqual,
 intermediateStateObservable:false}
~~~

成功readSet：

~~~text
{nodeRevisions:[{nodeRef,sourceRevision}],
 recurrence:<RecurrenceReadBinding/1>}
~~~

no-op accept不改source/revision/cache。实际source变化只增series owner revision一次；任一错误返回完整pre-state、空write owners、null readSet、空invalidations。inject-failure仅测试harness。

Recurrence projection request exact：

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

horizon是同basis bounded range且start<endExclusive；outputLimit是1..2^63-1 D3Integer且Boolean拒绝。先证明所有exceptions∈B∪R，再枚举bounded base prefix并加入RDATE/窗口外exception selector，最后以final range与horizon相交过滤。结果按originalStart exact time排序，replacement start不重分配selector。

projection outcome exact：

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

accept时complete=true、writeSetOwners=[]。




























  任一来源、绑定、语义域、成员、预算或覆盖证明失败时，complete=false，rows/projectionIdentity/readSet均为null。
  writeSetOwners=[]；outputLimit耗尽固定operation_precondition_failed，禁止截前N后标complete。

RecurrenceReadContext/1 exact：

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

workBudget是1..2^63-1 D3Integer；timezoneRuleSets按timeZone/tzdbVersion唯一。segments是UTC升序、相接、不重叠的非空有限数组；offsetSeconds是-86400..86400 canonical integer string。coverage不足是provider_or_schema_unavailable，不调用OS timezone补齐；fold选择所有已验证解中的较早instant。

RecurrenceReadBinding/1 exact：

~~~text
{registryBinding,timezoneRevisions}

timezoneRevision:
  {timeZone,tzdbVersion,revisionToken}
~~~

timezoneRevisions按D3-CJ/3 bytes排序唯一且完整覆盖实际读取规则。expectedRecurrenceReadBinding必须逐项相等。成功Facet outcome保存实际recurrenceReadSet；失败relation/recurrence read sets均null。

CalendarSeriesScopePolicy/1 exact：

~~~text
{kind:"calendar_series_scope_policy",
 policyId,policyVersion,
 keyMembers,scopeKinds,nodeScopeMember,
 multiplicities,conflictCode,
 retryRevisionRequired}
~~~

contribution exact：

~~~text
{kind:"calendar_series_scope_policy_contribution",
 policyId,policyVersion,policySchemaDigest}
~~~

key exact：

~~~text
{series,periodKey,scope}

scope :=
  {kind:"workspace",workspaceId}
  OR
  {kind:"node",scopeNodeRef}

multiplicity := unique | many
~~~

policy以RegistryBinding + policyId + policyVersion + policySchemaDigest解析。series必须从已验证author Entry完整还原，periodKey必须由同snapshot规则canonical验证。unique在同key已有/并发第二个不同NodeRef时返回calendar_series_scope_conflict且零写；many允许。retry使用同key + current revision；path/index从不去重。

DerivedDuration/1不是authorable Field。bounded date_range产生：

~~~text
{kind:"calendar_units",
 calendarId,calendarVersion,precision,units}
~~~

bounded instant_range产生：

~~~text
{kind:"exact_seconds",seconds}
~~~

open bound在其余provider gate成功后产生：

~~~text
{kind:"unavailable",reason:"open_range"}
~~~

任何写回duration的请求固定constraint_conflict。

### 16.7 D6-FA-r01 outer binding、pending 与 legacy

上述 D4 inner wire 版本保持不变。新请求必须由 D6 InputDescriptor/2 对每个 source-bearing owner绑定完整 SourceVersion/2，并通过 SourceObservation/1 消费当前观察资格；SourceVersion.commitDomain 保持生产域定义，可以不同于当前operation CommitDomain。CommitDomain、Frontier/2、Registry/Policy/control revisions、当前 fileObjectBinding、evidencePins 和实际 source pins 形成同一当前proof cut。SourceVersionRef/1 的 sourceToken 选择完整受保护 Observation，而不是裸sourceRevision、I cache、相同hash或旧stateToken；这些值不能跨replica/observationEpoch连续性失效后复用。

ordinary save 的保存语义与 `WriteProtection` 选择保持分离。普通保存仍可以保持 `strict`；`observed_only` 只能由受信 `interactive_source_save` 显式选择，并且仅适用于一个既有 live Document、ordinary replica-local 范围、完整 source read 与 replace 资格、没有适用的 body/Field/Node-control deny、author-source write set 仅限该 Document 或为空，且不存在 identity、parent、order、lifecycle、shared-policy、Registry、Calendar-scope 或其他 entity mutation。选择弱 profile 前，DraftBase 与当前 source 状态必须由对应当前 `SourceObservation` 表示。非交互流程、complete 或 strong Action、structured bulk、collection mutation 或 promotion、automation、server checkpoint、Approval 或 Money 相关执行均不得使用 `observed_only`。

任何 ordinary save 成功前，所有 touched local `Entry`、`Facet`、`Type` 和 requiredness 规则都必须通过。invalid local fact、touched `retained_unavailable` namespace、`D2 external_invalid`、physical invalid source state 或真实 read/write 资格不足仍保持失败或独立 repair 路径，不得通过 `semantic_pending` 或弱保护转换为成功。`semantic_pending` 只登记尚未证明完整范围的 `relation|unique|calendar|inbound|cross_object_type` 义务，不把缺少 complete proof 的 strong Action 变成成功。

`observed_only` 只耐久保留已观察前像 B 与用户输入 N。它不保证未观察到的外部竞争写入 C 不存在，也不保证从已观察 B 状态安装用户输入 N 时，未观察的 C 字节能够免于被 N 覆盖。如果 N 安装后又发生未观察的 C 写入并替换当前文件，这只影响当前文件状态；已耐久保留的观察前像 B 与用户输入 N 不因此丢弃。局部 typed facts 与未选 bytes 只能证明已绑定的 B→N 转换，不能声称验证所有未观察中间 source，也不能主动省略必要观察。已观察竞争、stale Base 或 continuity gap 继续走现有 conflict/reprepare 路径；未知安装保持 `recovery_unknown`。

受信 interactive ordinary save 可以在 planning 阶段明确选择弱 profile，即使普通目录没有 strict capability，但 `writeProtection` 规划后必须冻结。任何已知 Base 冲突、授权失败、耐久失败、strict plan 失败或强义务失败都不得 fallback 到 `observed_only`，也不得扩大弱保护范围。

Preparation 只保留后续操作所需的 durable proposal、read-before 证据和 pins；它尚未成为 saved、installed 或 sealed 结果。installed、sealed 和 unknown D6 状态必须区分。`durable_observed_only` 不是 strict reliable save，也不是 complete-set Query 或 Action 的资格证明。既有 D4 source 层、当前依赖、Registry、关系和 source observation 义务继续保持不变。

补验只产生current SourceVersion的新资格；历史r5 receipt永不改写成r6 complete。I重建、P丢失、设备迁移和A→B→A均不恢复旧proof。

历史 D3 v9/v10/v11、D6 wire1、D7 PreparedActionBinding/1,/2 继续按原 decoder/bytes/gates/retention重放。新 D7 Prepared 未完成前，需要完整D7 preparation的D4 strong Action返回owner_update_required/unavailable且零新版managed-success；D4不私造新版Prepared。

## 17. D6-FA-r01 当前跨 owner 桥接条款

### 17.1 People × Organizations intake 与 rank

people/engagement 的 NodeRef arm继续允许任意 ordinary Node，不要求目标安装 Organization Facet。subjectPredicate仍是Person条件，targetPredicate仍是ordinary_node。Organization roster/org-chart等增强视图可以要求Organization条件，但不能反向否定Person侧真实任职事实，也不能自动Assign/创建/搬移target。

以下People关系Field继续支持exact NodeRef/text target union：people/parent、
  people/spouse、people/sibling、people/guardian、
  people/family-related、people/social-related、
  people/professional-relation、people/manager、people/mentor。
  text arm只保留作者literal，无resolution/inverse/graph/cardinality/lifecycle；NodeRef arm按各Field固定方向/域处理。
  family/social/professional为symmetric general；
  parent/guardian/manager/mentor为directed。text↔NodeRef变换是显式atomic occurrence update。

people/profession与engagement分开。engagement的position/department是text，rank是制度限定作者值。EngagementRankValue/1 exact为一个TypedObject，required members只有：

~~~text
system := non-empty exact TypedText
level  := non-empty exact TypedText
~~~

不得额外加入code、国家默认、推断ordinal，也不从position/department/organization补值。system是作者给出的制度上下文文字，不是全球制度ID；同名level不证明跨制度可比较。未来preset/机器码必须通过新的closed contribution和显式migration，不回写现有custom author value。

### 17.2 candidate 与已激活 Registry ledger

D6-FA-r01 是未激活联合候选。fixed-S Registry/Field/Facet ledger的历史记录继续作为来源；任何已激活Workspace若未来采用本候选，仍按D4 monotonic evolution执行fresh identity/tombstone/migration和current Policy授权。不能把“候选重新绑定”当成已激活ledger可原地改写许可。

portable Registry复制到replica只复制shared fact，不复制Registry admin/execution custody。

### 17.3 D3 wire12 fresh source composition

旧fixed-S §15中“D3 v11 fresh candidate map”在新decision路径替换为H2 D3 wire12 + D6 PreparedIntent/2/InputDescriptor/2。语义义务保持：

- fresh Node/Resource/Annotation candidate必须来自已验证D3 intent/plan，而不是caller自造；
- D4 initial Entries、Template结果、copy/fork typed refs在planning reservation前对真实拟议完整source执行；
- candidate identity map、allocation/burn、source payload、
  lifecycle与current Registry/Policy属于同一原decision proof；
- symmetric relation source assembly仍按result endpoints重新计算canonical owner；
- 不得为满足D4约束重抽已承诺ID、扩大closure、删provenance或改Facet；
- D4SourceMaterializationEffects/1与D4RelationCopyEffects/1必须从真实before/after source独立重算并与D3 receipt/effects一致。

fresh create的D4语义pre-state仍是同一已绑定完整拟议结果source，prospective sourceRevision=1；成功不二次append、不把revision加到2。普通existing owner不能借fresh-origin绕CAS。

### 17.4 D6 ObservationScope 与 disclosure

完整依赖读集不等于principal有权观察constraint结果。D6在D4值/range求值前提供与同一CommitDomain/SourceVersion/Registry/Policy cut绑定的保守 ObservationScope/受保护read dependencies，包括可能为空的negative range。

无权时，两个仅在隐藏状态不同的世界必须在D4业务求值前得到同一外层not_visible/不可披露结果，且零隐藏业务读取/decision；获得披露权限后才可执行真实unique/cardinality/incoming/cross-Field检查。

D4不新增permission token，也不信任client声称“范围完整”。局部Field路径只有在静态依赖上界与实际读取追踪都证明独立时才成立；否则使用实际完整operation scope。该规则与Diagnostic authority masking一致。

### 17.5 Calendar scope control

calendar/period作者value仍只有period + seriesKey，不增加scope Field。
  完整CalendarSeriesScopePolicy key中的scope来自D6同cut的portable/control scope binding，
  而不是path、folder、View row或ambient locale。

scope binding的建立、显式迁移/删除、control inbound与版本CAS由D6 owner承担；D4只验证同Registry policy下的typed key与unique|many语义。新period在需要workspace scope时必须有显式当前配置；copy/fork按D3 map重绑合法scope，不能因为target“看起来为空”省略unique/many验证。

### 17.6 D7 consumer boundary

D4 C carrier、Entry/Facet/Relation/Registry/Recurrence/DerivedDuration仍由D4拥有。D7只消费保存定义、Query/Action和effects transport，不能取得D4 author typed-value权威。

历史 PreparedActionBinding/1,/2 与旧D3 saved decisions只用于legacy replay。未来D7 owner必须定义与CommitDomain/Frontier/SourceVersion/2相容的complete cut和Prepared contract。在其实际后像接受前：
- D4 ordinary local source-save按D6 complete/pending规则可用；
- 需要D7 complete proof的relation/Facet/Calendar strong Action固定owner_update_required/unavailable；
- 不得从历史binding、I cache或当前page伪造新版success。
