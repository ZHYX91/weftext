---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：ebd24e10-6020-41e0-8a08-f494e46c21ec。

# D4 Implementation Impact and Test Outline — D6-FA-r01

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 的 revision37 preservation / revision36 admission / revision35 integration 义务全部保留；本文件增加 D6-FA-r01 的 SourceVersion/2、SemanticState 和 local-vs-complete 消费验证，不把历史运行当作新版本通过。

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
  -> D7 future complete Query/Action consumer

`SourceVersion/2` 保留其生产 `commitDomain` 和版本历史。当前 `SourceObservation/1` 不替代生产版本语义；其 `observerDomain` 必须匹配 operation `CommitDomain`，并且其 entity、file、evidence、control、Registry、relation-incidence 依赖必须属于当前 observation cut。`SourceVersionRef/1` 通过 `sourceToken` 选择完整受保护 Observation，而不是裸 revision、hash、I cache 或 production version。

Source body 仍是权威来源。可重建 I 不是正文镜像。`Frontier/2` 是已 seal 的因果/依赖前缀，不是完整 Query 结果、payload 物化证明或 Registry generation 证明。ordinary 与 complete qualification 仍是语义轴；`strict` 与 `observed_only` 仍是保护轴，且 `observed_only` 仅限已批准的受信 interactive ordinary source-save 场景。
~~~

I 只缓存可重建 projection/incidence/search candidate；P只保存不可重建 decision/pins/control evidence。两者均不保存第二份 Field/Facet/relation current truth。

## 2. component impacts

| component | future impact | forbidden shortcut |
|---|---|---|
| D4 Registry loader | 保留 owner/digest/evolution/catalog完整验证 | 从安装顺序或display name认owner |
| Entry 解码器 | closed Entry/1、精确 span、raw 保留 | 禁止 free JSON / whole-namespace blob |
| 类型引擎 | 精确 integer/decimal/calendar/instant/ref/alias | 禁止 host float/date 默认值 |
| Facet engine | declared/effective、requiredness、conflicts | implicit membership/last-wins |
| relation engine | Context/Binding/complete incidence/post-state | index=truth、inverse双写 |
| Calendar engine | comparator/recurrence/series scope | current page=complete range |
| D6 adapter | InputDescriptor/2 + SourceVersion/2 + SourceObservation/1外层绑定 | bare revision、hash或production version跨观察cut复用 |
| conflict/repair | exact source + ConflictRecord + current Registry | LWW/hash-only merge |
| downstream D7 | complete cut/new Prepared future version | 从pending私造Action成功 |

## 3. future implementation slices

### 3.1 source and Registry

- 解析 full D2 exact source，建立 carrier/Entry spans。
- 先验证 Namespace Owner/RegistryBinding，再解释 contribution-backed identifier。
- unknown/untrusted/incompatible schema为 retained_unavailable，raw bytes保留。
- current Registry generation与 source validation cut明确绑定。
- Registry cache失效后必须从 portable Registry bytes重建，不能由 I 自报当前 generation。

### 3.2 local ordinary save

ordinary source-save adapter计算 actual MutationFootprint，并区分保存语义与 `WriteProtection` 选择。普通保存可以保持 `strict`；只有受信 `interactive_source_save` 才可显式选择 `observed_only`，并且必须同时满足：一个 existing live Document、ordinary replica_local 范围、complete source read/replace资格、无 applicable body/Field/Node-control deny、author source write set为空或仅该Document，且不涉及 identity、parent、order、lifecycle、shared policy、Registry、Calendar-scope 或其他 entity mutation。DraftBase必须对应selected current SourceObservation。

adapter在ordinary save前至少证明：

1. current SourceVersion/2 与 current SourceObservation/1；
2. 完整 source；
3. current RegistryBinding；
4. touched local Field/Facet/type/cardinality；
5. write permission；
6. unique proposed source；
7. D2 valid；
8. file install qualification。

未触及 unavailable/invalid namespace必须 byte-equal；触及它则普通 typed edit拒绝。invalid local fact、retained_unavailable namespace、D2 external_invalid、stale Base 或观察连续性失败保持失败、repair或conflict/reprepare路径。只有真实跨对象/全集 obligation未证明时才能产生 semantic_pending；它不能作为strong Action完整证明。

### 3.3 strong operations

Facet mutation、relation mutation、series-scope unique、typed copy/fork/import、restore/purge、D7 all_result/bulk/automation 写都必须取得 operation-applicable complete proof；missing range不能降级 ordinary，也不能通过 `observed_only` 或普通保存路径完成。

未来验收必须同时覆盖 ordinary strict 与符合 A§4.1 条件的人工 weak positive case：ordinary strict 在完整 source、权限、当前观察和安装资格满足时可成功；trusted `interactive_source_save` 对一个 existing live Document、ordinary replica_local 范围、complete source read/replace、无适用 body/Field/Node-control deny、author write set为空或仅该Document、无 identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation 且 DraftBase 等于 selected current `SourceObservation` 时，才可验证 `observed_only` 成功。

以下情况均不得选择或降级为 weak save：不是受信场景、不是交互式 source save、新建 Document、多 Document、不是 ordinary 范围、不是 replica_local 范围、source read/replace 不完整、body/Field/Node-control 存在 applicable deny、write set 涉及多个 entity、identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation、DraftBase 不等于当前选定 Observation。structured bulk、collection、promotion、automation、server checkpoint、Approval、Money 和任何 strong Action 均禁止使用弱保护。

人工可以在 planning 前明确选择 weak profile，即使普通目录缺少 strict capability；但 `writeProtection` profile 冻结后，strict capability 缺失、已知冲突、授权失败、耐久失败或 strong obligation 失败都不得 fallback 到 weak。D4 local types、D2 valid、unavailable byte-equal、真实 source read/write 检查继续保持；`semantic_pending` 只表示真实未证明的跨对象/全集义务，不洗掉 invalid、unavailable 或强 Action 缺失。

`observed_only` 只耐久保留已观察 before B 与用户输入 N。未来外部 C 可能在未被观察时存在，N 安装可能覆盖当前文件中的 C 字节；后续 C 也可能替换当前文件 N，但不丢弃已耐久的 B/N。prepare only 不是 Saved；unknown install 保持 `recovery_unknown`；observed competition、stale Base、watcher gap 和 continuity gap 进入 conflict/reprepare。

D7 新版 Prepared 尚未定义时，需要 D7 complete preparation 的入口固定 unavailable/owner_update_required；D4 不得增加私有准备 token。

## 4. test outline

### 4.1 source and grammar

1. Entry raw source、carrier framing、comments、CRLF/LF、unknown namespace round-trip。
2. malformed Entry JSON、duplicate member、
  missing envelope member、wrong constructor、over budget。
3. unavailable namespace body-only edit：affected raw block byte-equal成功。
4. 同操作误改 unavailable block一个byte：拒绝。
5. external strict UTF-8/D2 invalid：只有 repair/raw path，无 managed D4 success。
6. source hash相同但 observationEpoch改变：旧selector/evidence失效。
7.两个replica并发改同一source：显式ConflictRecord，不LWW。

### 4.2 types

- integer/decimal canonical boundary，特别是 -0、exponent、trailing zero；
- semantic_code Field-local/complete ID；
- calendar ISO日期、year 0000、不同 calendar comparator；
- RFC3339 arbitrary fraction、offset/timezone、DST gap/fold；
- date/instant range bounded/unbounded/end-exclusive；
- quantity unit dimension且无未授权conversion；
- NodeRef/ResourceRef/AnnotationRef owner locality；
- alias depth/cycle/member/union/list limits；
- optional absent 与 explicit值区分。

### 4.3 occurrence and note

- duplicate value仍有独立 occurrenceKey；
- occurrenceKey保持 owner Node + FieldId + expected current source revision 的内层selector，不因 D6 outer SourceObservation/1 消费而改变wire形状；
- reorder/copy/delete/external edit后不按(key,value)猜目标；
- SourceVersion生产epoch/revision/externalSequence变化，或当前SourceObservation token、epoch、连续性失效后，不能仅凭key/value续认；
- inline note absent不写空placeholder；
- note不能承载 typed qualifier/provenance；
- expected SourceVersion stale拒绝；
- same value在两个replica不同occurrence merge不串note。

### 4.4 Facet and availability

- declared/effective closure；
- missing dependency、cycle、conflict、required_field；
- source-declared tasks/task与effective-only tasks/task；
- Template + tasks/task拒绝，Template +合法非task Facet接受；
- retained_without_membership保持raw事实；
- namespace available/retained_unavailable/invalid/not_present；
- Node complete/partial/unavailable；
- D6 complete_semantics/semantic_pending 与 D4 typed state笛卡尔反例，禁止混为一态。

### 4.5 relations

- directed single authored fact + inverse projection；
- symmetric canonical owner；
- canonical owner排序翻转copy/fork；
- node_ref_or_text literal无inverse；
- RelationReadContext/2 source/sourceless/masked/unprovable；
- Binding/2 source/entity/incidence exact coverage；
- SourceVersion/2 outer绑定与 inner sourceRevision一致；
- SourceVersion/2 的 production `commitDomain` 可以不同于当前 operation observation 域；完整当前 SourceObservation/1 合法时不得仅因 production domain 不同拒绝；
- same inner sourceRevision 在 SourceVersion production epoch/revision/externalSequence变化后拒绝旧选择；即使 production version 相同，watcher gap、external replacement 或 discontinuous materialization 后也不能凭裸revision/hash/key/value恢复旧token；
- SourceVersionRef/1 的 `sourceToken` 使用 `d6_source_observation/1` 选择完整当前受保护 Observation；不是裸revision、hash、I cache或production version；
- current SourceObservation/1 要求 observerDomain 等于 operation CommitDomain，entityRef 等于 sourceVersion.entityRef，并完整覆盖当前 observationEpoch、fileObjectBinding、evidencePins、control、Registry 和 incidence 依赖；
- observerDomain错误、entityRef不一致、fileObjectBinding/evidencePins/control/Registry/incidence缺项或非current、token/epoch/continuity失效均拒绝；
- Frontier/2 只提供当前 sealed causal/dependency cut，不单独证明全集Query、payload或Registry完整范围；
- masked/unprovable不能产生成功binding/readSet；expected binding必须逐项exact equality；
- required relation field不能由incoming inverse满足；
- new target domain/lifecycle/cardinality；
- delete tombstoned/not_found旧target；
- symmetric purge前incident cleanup；
- relation source migration/copy/fork effects；
- hidden endpoint non-disclosure；
- 当前授权检查、内层wire、owner双射和完整source/entity/incidence覆盖保持不变。

### 4.6 Calendar

- recurrence date/instant homogeneous variant；
- count/until互斥；
- RDATE/EXDATE/exception 256 limits；
- period rules/day/week/month/quarter/year；
- explicit horizon、budget、cancellation；
- series-scope many/unique；
- unique negative range来自完整 source/cut，不来自 I/page；
- derived occurrence cache删除重建；
- pending(calendar)普通save不能变unique Action成功。

### 4.7 domain fixtures

People:
- people/phone alias text required；
- optional label absent；
- valid people/work semantic code；
- localized “工作”不得写成semantic code；
- customLabel独立；
- engagement ordinary NodeRef + literal；
- family/social/professional direction。

Organizations:
- structural move不改 organizations/parent；
- symmetric/directed事实分域；
- identifier/classification不是identity。

Library:
- library/work与project work/task分域；
- DOI/ISBN不是NodeRef；
- Citation occurrence不是Work。

Tasks:
- source-declared tasks/task；
- tasks/status required；
- dependency两端Task；
- UI module disable不删semantic。

### 4.8 ordinary-vs-complete matrix

必须逐格验证：

| operation | complete range缺失 | 唯一允许结果 |
|---|---|---|
| body/不相交namespace save | 无关且ordinary资格满足 | 可成功 |
| local nonrelation Field edit | 无跨对象义务且权限/观察通过 | 可成功 |
| local valid Field edit + relation obligation | 缺 | semantic_pending |
| local Entry invalid | 任意 | reject |
| Assign/Remove Facet | 缺 | reject/unavailable |
| relation mutation | 缺 | reject/unavailable |
| Calendar unique | 缺 | reject/unavailable |
| D3 local Trash | inbound未证明 | pending(inbound) |
| restore/purge | 缺 | reject |
| all_result/automation write | D7未完成 | unavailable |

额外覆盖弱保存边界：

- 合法 `observed_only` 只允许 trusted interactive source save；B读取后外部未观察C改变文件，随后N安装可能覆盖C字节，但已耐久保留B和用户输入N；
- observed external C、stale Base、watcher gap、continuity gap进入 conflict/reprepare，而不是成功安装；
- prepare only不等于saved，unknown install必须保持 `recovery_unknown`；
- semantic_pending必须保存 obligations exact set，不把 invalid、unavailable、D2 external_invalid 或 strong scope缺失洗成成功。

### 4.9 r5/r6, I/P and external change

1. r5 pending committed。
2. r6同source后续complete验证。
3. retry r5仍返回原pending bytes。
4. delete I再build：不得把r5改complete。
5. lose P：不得从source重建strong decision/readSet。
6. external A→B→A：observationEpoch变化，旧proof失效。
7. move library to new device：new replicaEpoch，不复制execution custody。
8. hash相同但SourceVersion/domain不同：不复用proof。

### 4.10 partial index and D7 boundary

partial index可以提供已授权 local exploratory rows并显式 partial/pending；必须拒绝：

- relation negative completeness；
- unique negative；
- Calendar unique；
- D5 collection complete membership；
- D7 all_result；
- automatic bulk target derivation；
- Automation/Agent write。

完整scan source可替代complete index；未完成则强consumer unavailable。

### 4.11 diagnostics

验证 outer permission/Registry availability早于inner payload disclosure；UTF-8 half-open spans、
  sourceStart/rank/code排序、missing member无虚构span、unknown contribution保留raw、masked endpoint无详细状态泄露。

## 5. legacy and version tests

- D4 Entry/1 unchanged；
- RelationReadContext/2、Binding/2 unchanged inner wire；
- RecurrenceReadContext/1 unchanged；
- D4RelationCopyEffects/1、D4SourceMaterializationEffects/1 unchanged；
- outer new request通过 D6 `InputDescriptor/2` 绑定 `SourceVersion/2`，并消费当前 source observation 资格：`sourceInputs[].observation` 使用 `SourceObservation/1`，`SourceVersionRef/1` 的 `sourceToken` 使用 `d6_source_observation/1` 选择完整受保护 Observation；同时按实际 owner 定义消费 `ObservationScope/2`、`Frontier/2` 与 `DependencyProof/2`。`SourceVersion/2` 保留生产版本定义，包括自身 production `commitDomain`、epoch、revision 等语义；生产域可以不同于当前 operation observer 域。当前合法观察要求 entityRef 与 sourceVersion.entityRef 一致，并由当前 fileObjectBinding、evidencePins、control、Registry、incidence 与完整 observation cut 共同证明。
- D3 v9/v10/v11 saved decisions用原decoder；
- D7 PreparedActionBinding/1,/2历史恢复；
- 不把旧corpus数字换成新版本号宣称新消费通过。

## 6. terminology and non-regression

机械检查 fixed-S 26 conceptId、owner、owned wire/code/UI/locale names、
  firstFreeze逐项 exact。D6/D3 imported names不得进入D4 owned set。

Catalog blob必须保持 `ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb`；任何Field/Facet/alias/limit变更都应触发独立 catalog replacement，而不是在实现中暗改。

## 7. mandatory scenario preservation

mandatory scenario 文件是压力输入，不是产品批准。未来 conformance至少重放 People/Organizations/Calendar/ICS/Library、unknown provider、external editor、multi-replica conflict、terminology collision、no-second-authority。

Chart/View、Office template 等由 D7/D8/D9 owner未来裁决，不能反向扩大D4 field/type合同。

## 8. completion boundary

一个未来实现切片只有在 Core、全部受影响 adapter、fixtures、legacy replay、D6 source/pin binding、双语公开文档和负向门同时更新并有真实运行证据时，才可声称实现。

本文件当前只是设计候选；文档CI/作者自查不等于产品测试通过或独立接受。

旧D10 B13仍REVISE、术语/双语FAIL、11 OPEN。

## 9. fixed-S 有效回归义务完整保留

本节把 fixed-S Impact 中仍有效的具体实现/测试义务纳入当前候选。历史测试数字、旧 wire 名和旧实现状态不升级为当前成功证据；语义义务继续有效，并按本节最后的版本映射消费当前 D3 wire12、D6 v2 及未来 D7 owner。H2 相关版本仅作为历史 saved decoder/兼容映射参考，不作为当前 producer 或成功实现证据。所有 fixed-S 回归义务继续保留，并由当前 owner 合同映射消费；D4 typed semantic result、legacy bytes、saved recovery 和未来 owner gate 不因版本映射而删除。

### 9.1 source/grammar 与 parser

必须覆盖：

- LF/CRLF/CR carrier/Entry；
- duplicate JSON keys、trailing tokens、unknown envelope members、
  illegal null、float/NaN/Infinity/-Infinity；
- Entry version 0/2；
- dotted namespace + localField expansion；
- namespace 63/64-byte、FacetId 127/128-byte、empty dot segment、leading/trailing/consecutive hyphen；
- wrong namespace、多个同namespace blocks、D2 32/8192/64KiB边界；
- identical values with distinct keys合法；
- same key in different Fields合法；
- same expanded Field跨任意same-namespace blocks duplicate key拒绝；
- header attribute名称像Field仍是ordinary header；
- explicit Map只生成一个chosen carrier target并保留loss；
- unknown provider raw exact round-trip、
  external formatting damage、无old typed projection fallback。

strict parser必须保留每个nested key/value/array/scalar的UTF-8 half-open span，支持multibyte、escaped key、arbitrary whitespace和同名nested key。lone surrogate只报invalid JSON。测试必须比较完整Diagnostic sequence、pointer和span，而不是只看“有错误”。

### 9.2 types、数值与宿主边界

必须覆盖 canonical integer/decimal；合法 -0.5、0.5、-1、1.25；decimal zero唯一0；-0、0.0、-0.0、1.20、exponent拒绝。

calendar_date覆盖ISO/non-ISO unavailable、year/month/day precision、
  ASCII digits only、year 0000拒绝及故意与lexical order不同的registered comparator。

zoned instant覆盖真实日期/clock/offset、Arabic-Indic digits、+25:00、-00:00、leap second拒绝、
  sub-microsecond/arbitrary fraction精度、IANA zone与tzdbVersion、无device-default guess。

date/instant ranges覆盖both bounded、
  start-open、end-open、both-open reject、
  equal/reverse/cross-offset、decoded exact ordering和end-exclusive显示。

quantity、unit contribution、dimension coupling、height+kg/weight+cm反例继续。没有认证conversion contribution时不得转换。

D3 refs/locators必须走exact frozen decoder；
  wrong kind/extra/missing/non-v4/bare UUID/path/title/URL拒绝。
  direct value和provenance中的cross-owner ResourceRef拒绝。

ValueTypeSpec/ObjectMemberSpec/UnionVariantSpec、alias cycle/missing/depth8、
  schema/Entry byte limits、Boolean masquerading as integer、
  collection/cardinality/provenance inputIndex bounds、bounded_set/sequence nesting、
  set canonical order/uniqueness、generic array/map/any/opaque rejection全部保留。

至少覆盖5000位合法fraction的recurrence/instant range exact round-trip；降低宿主Decimal context precision不得改变结果，也不得靠提高宿主全局integer-digit limit绕过作者精度合同。

### 9.3 occurrence/note/ABA

必须覆盖两个相同phone值带不同notes/provenance；exact key patch/reorder/delete；missing/stale expected revision；
  delete/re-add ABA；external copy duplicate key；same key different Field合法。

identity-preserving move保留bytes；
  cross-owner copy/import/template可保留key
  bytes但same-owner duplicate必须fresh key。
  key送EntityRef/Locator/AnnotationTarget/RecordRef decoder必须拒绝。

absent note必须省略，empty placeholder拒绝，note从不解析成behavior。

### 9.4 Registry、Facet、availability 与 diagnostics

Registry evolution测试继续覆盖：

- exact generation+digest；
- same-generation substituted snapshot；
- user owner wrong Workspace；
- predecessor/tombstone/migration ledger；
- retired ID跨三代不可复活；
- contribution identity monotonic；
- same-ID type/cardinality/constraint/relation/Facet/tzdb/unit drift拒绝；
- deletion必须matching tombstone；
- replacement必须fresh ID + exact old→new migration；
- catalog alias无环且61 Field/7 Facet引用完整。

availability测试继续覆盖 read/body edit/typed edit/query/export/reinstall。UI/runtime disable但portable schema仍在时，typed含义不变。owner-unprovable/conflict/generation-changed必须在inner parser前遮蔽，parser call count可验证为0。

contribution preflight先于wrapper structural/type，覆盖union extra member、calendar comparator缺失、
  qualifier observedAt+bad confidence、external provenance observedAt+extra member；availability code不被归一。

Facet继续覆盖 source reorder semantic equality、
  requires closure/cycle/missing/conflict、shared Field compatible definition、
  Task+Project+Calendar、Template+nonTask合法、Template+tasks/task保持D2拒绝、required last occurrence、Remove保留Field、Cleanup不删仍被其它Facet使用的Field。

### 9.5 relation state machine 与27个Field

关系测试必须使用真实stateful one-fact store，保存raw Entry bytes、parsed typed value、
  qualifiers、note、provenance、occurrenceKey、resolution，并证明strictParse(raw)==entry。

同一symmetric fact从A→B或B→A提交得到byte-equal canonical state；shared store重复same-key same-payload保持一份事实；same-key different-payload固定collision；
  distinct keys服从schema cardinality。self-edge、
  trash/restore suspension、projection/index delete+rebuild全部覆盖。

text↔NodeRef、NodeRef→text、NodeRef→NodeRef必须绑定完整before/after owner revisions、
  canonical-owner relocation和stable occurrenceKey。
  任一CAS/auth/collision/cardinality/lifecycle/projection failure逐字回滚raw source、
  revision、projection和allocation且write set为空。

expectedSourceRevisions必须是D3Integer，7.0不得等于7，true不得等于1。sourceRevisionOwners只覆盖source-bearing inventory；sourceless endpoint不得伪造revision。

RelationReadContext/2、Binding/2覆盖 source_node_state、sourceless_node_state、masked_node_state、
  unprovable_node_state；stateToken lifecycle ABA、跨Ref token、负范围分配和incidence concurrency都必须使旧binding失败。

全部27个Field逐项测试 direction/inverse/domain，完整集合：

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

directed项逐个命中其inverse contribution；symmetric项无inverse code。Resource cross-owner relation拒绝。graph/backlink I删除后从作者事实重建必须byte-equivalent。

typed copy/fork覆盖27 Fields、fresh NodeId两种排序、canonical fresh-owner迁移、literal、provenance/locator、requiredness/cardinality/domain、空incidence scope和existing endpoint不可写反例。任何无法合法保存的关系使whole typed copy失败，不静默omit。

### 9.6 domain fixtures

People fixtures至少包含 names、contacts、preset/custom accounts、
  birth/death/employment-start/graduation/marriage preset事件与custom重要日期、冲突assertions、
  measurements、engagements、parent/spouse/sibling/family/social/professional、
  manager/mentor/guardian、provider missing、same-name Nodes。

spouse历史允许多条独立声明，覆盖不重叠、重叠、无validity、literal/NodeRef混合；更新一个事实不得改其它raw source/provenance。有限cardinality反例使用独立conformance schema，maximum永远不进入产品request。

people/engagement NodeRef arm允许任意ordinary Node，不要求Organization Facet；
  Organization-specific roster/graph只是增强。literal fallback不inverse。

engagement rank是closed object，required system+level均nonEmpty exact text；无rank不推断。不同制度同名level、同人多组织/同组织兼任、原文保留、缺member/wrong type/extra推断member都覆盖。外部rank code无法解释时D9显式mapping/loss。

Organizations fixtures覆盖 semantic parent vs structural、governs/owns/allied-with/related、
  brand-of/subsidiary/supervised/member-of/business-guided-by/territorial/joint leadership，
  并证明note不能猜relation kind。

Library fixtures覆盖 venue Work→Journal/Organization、
  非法Person target、provider missing、wrong Facet、target deletion、
  draft→published same Work、edition fresh Node+relation、
  DOI不是identity、creator/venue/Citation/Resource分域。

### 9.7 Calendar/recurrence 完整验证

公共projection和edit都先授权，再验证owner/source revision、Registry、Field/key和temporal read binding。

必须覆盖：
- date模板保持calendar day duration；
- instant模板保持exact elapsed seconds；
- civil recurrence不是每日+86400秒；
- DST gap/fold、explicit late-fold anchor；
- 30位小数；
- 输出端点越出可表示日期域的failure；
- count/until共用exact-time base enumeration；
- civil traversal与actual instant排序不同；
- timezone lookahead coverage缺失；
- work/output budget不足返回无rows failure；
- horizon相交；
-窗口外双方向replacement移入；
- replacement移出；
- RDATE/EXDATE/cancel/replace precedence；
-相同final range但不同selector；
- edit/rebase后新revision重投影；
- cache删除后byte-equivalent派生；
- permission change；
- Calendar UI关闭不改Core语义。

series-scope unique|many覆盖并发collision、path/index rejection、Registry policy digest绑定。
  duration覆盖closed/open date/instant和authored-duration reject。
  Calendar pack多source且不写isHoliday。

### 9.8 People状态、事件、Task与Account

Person状态使用独立nationality-state、legal-sex-state、gender-identity-state Fields；每个assertion独立保留TypedText、validity、provenance，重叠/冲突不覆盖。D7只能按明确FieldId/period投影，未知期间不假定current/permanent/unique。

People事件覆盖五个preset code+custom label、required eventTime、
  date/instant/year/month/day precision、unknown/cross-domain code、
  empty/missing label、wrong branch/extra member。相同label/date、
  conflicting birth/death和多个preferred都可并存；缺contribution不是empty。

Task分类共享同一source-bound检查：ordinary + explicit source-declared tasks/task；
  effective-only Task在无关系、relation endpoint、Create/Assign/Remove、Task/Calendar recurrence都拒绝。
  direct/multilevel/diamond requires、Task+Project、Template nonTask、Template Task、
  source revision mismatch全部覆盖。Remove先移dependent再移Task；不支持一条request多Facet remove。

people/account preset/custom union保留；
  custom arm required serviceKey+identifier exact nonempty，
  usage/customLabel在outer value，Entry.note分开。未知custom service不自动全局注册或按display label匹配。

一般People关系三类symmetric general与guardian/manager/mentor directed分域；custom称谓不触发Field转换。旧directed professional事实不能被D9静默改成symmetric。

### 9.9 Workspace、diagnostic、source materialization

D4 context必须绑定真实host Workspace。source owner不匹配时在Entry parse/Facet/effect前返回namespace_owner_unprovable并零解析。合法cross-Workspace ref value/provenance不因此被改写。

recursive provenance diagnostic覆盖external/node/resource/transform及nested refs/locators；
  missing member保留exact pointer+nearest object span，
  existing token用最窄UTF-8 span，unknown key按RFC6901 escape，多个independent faults全部返回。

D4SourceMaterializationEffects/1必须由真实source assembly独立复核：跨同namespace carriers定位Field尾、Field缺席使用绑定carrier、先迁出/迁入再删除空carrier、same-owner原位更新、多迁入canonical排序、comments/CRLF/unknown namespace/unselected raw保持。before/after不能从待验effect反推。

fresh Create验证result revision=1；revision0假前像、结果2、缺/重复initial Entry、existing owner冒用fresh origin全部拒绝。普通existing改写仍只+1。

### 9.10 fixed-S 版本义务到 D6-FA-r01 的映射

旧文档中“D3 v11/Result9”对应的**新 decision path**现在由D3 wire12 + D6 Control/2承担；v9/v10/v11 saved decisions仍按原decoder/bytes/gates replay。所有原关系/Calendar/Entry/Facet内wire保持其自身版本，不能因outer升级而改数字。

旧 D6 source revision/potentialChanges/current observation 测试在新路径映射为 `SourceVersion/2`、`CommitDomain/2`、`Frontier/2`、`InputDescriptor/2`、`Policy/3` 以及 D6 safe-install/P-seal/publication。测试义务不删除，只替换其外层绑定。`SourceVersion/2` 保留生产版本及 production domain 语义；当前 operation 使用 `SourceObservation/1` 完成 observerDomain、entityRef、fileObjectBinding、evidencePins、control、Registry、incidence 与 current cut 验证。`Frontier/2` 是 sealed causal/dependency cut，不是全集 Query 或完整 Registry proof。内层 D4 wire、legacy saved decoder、bytes 与 gates 按原版本保持。D7 complete Prepared 尚未完成时，相关入口仍保持 `owner_update_required`。

旧D7 revision03/PreparedActionBinding历史只保留legacy replay。新D7 complete cut/Prepared尚未完成时，相关strong D4入口必须owner_update_required/unavailable；不得把历史测试数字替换后宣称新版已过。

D6 `ObservationScope/2` 与权限遮蔽仍要求无权隐藏状态得到同一 `not_visible` 结果，并保持零业务读取、零 decision；获得权限后才执行真实 constraints。当前消费结合 D6 control、pins 与 current observation cut，不再依赖 H2 作为 active authority/control producer。Registry、policy、Calendar scope binding 和首次 bootstrap 仍必须与当前 authority/control 合同组合。I 不作为隐私证明或 empty 证明；D4 typed semantic result、masked/unprovable 语义和权限边界保持不变。

### 9.11 接受边界

这些项目全部是当前候选的未来conformance义务。纯Python/model结果、historical fixture count、文档CI、作者自查都不证明产品host、D6物理事务、D7新Prepared、D8 UI或D9/D10已实现。

旧D10 B13仍REVISE、术语/双语FAIL、11 OPEN。
