---
_weftext:
  id: "1bde8ca5-50fd-41ec-a146-b31d951e46c2"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。

# D3 身份、引用、所有权和生命周期

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。



日期：2026-08-31。

## 1. 范围、冻结输入与非目标

本决议从零回答 D3：Workspace、Node、Document、Resource、Annotation 及 D2 已冻结对象的身份域；NodeRef 与 owner-local ResourceRef/AnnotationRef；locator、path、display label、source span 与 authoritative identity 的区别；创建、复制、克隆、移动、重排、重命名、删除、Trash、恢复、永久清理、导入/导出再进入时的身份规则；孤儿、悬空引用、tombstone、不可见、不存在、失效；引用解析和失败关闭；以及 D4–D10 的最小身份门禁。

冻结输入：

- D1：本机或 Server Core 是工作区语义和提交的唯一权威；身份不能依赖 UI、浏览器 URL、本地绝对路径、Server 数据库行、worker、Agent、Provider 或缓存。
- D2：Node 是 D2 content graph 中唯一 Document-bearing 内容实体；每个 Node 恰一 exact-source Document。
- D2 v2：Document metadata/body/occurrence 无独立 durable identity；Task 是包含 exact built-in `tasks/task` Facet 的 ordinary Node；Template 是 Core meta-kind；二者都只复用 Node identity。
- D2：Resource/Annotation 是 Node owner-local 非节点对象；Saved Query/View 是 Document occurrence；NodeCollectionResult 无 durable identity。
- D2 v2：`AttributeCarrierBlock`与`LexicalAttributeEntry`只是 exact-source Document 的 current-revision lexical occurrences；没有 EntityRef、独立 lifecycle、跨 revision continuity、locator token 或 Annotation target。
- D2：Record/RecordCollection 只从 D2 content algebra 排除；D5 仍可从零决定独立 record domain。

D2 的不可变规范引用为 [D2 Document Content and Domain Objects](../d2-document-content-and-domain-objects/source.md)，已冻结D2原验收保留的上游 authority target SHA-256 为 [D3-DIGEST:upstream_contract:atomic-d2-v2]`873D2487AC27C7C579D64C14CAB106215FBD462286416860225F2F404BC85137`，稳定 D2 ID 为 `993f6232-c8ae-476b-8ab3-9e071b55cd29`。D3 对 Annotation target 使用 D2 v2 §5.10 的五成员 closed union；对 source occurrence/locator 使用 D2 v2 已冻结的 A+ header、namespace-scoped lexical carrier 与 body outer grammar，并只补充 authoritative ref、owner、revision 与解析语义。当前D6所需组合与符号源演进采用D3 wireVersion11；D2保持其已经冻结的wireVersion2。wireVersion9仅按§4.1.3a提供历史保存决议重放，禁止新v9 decision；其他旧版本按其历史证据保留而不作为当前入口。

明确非目标：

- 不冻结 D4 的属性类型、schema、关系边、基数或删除策略，只规定任何未来 ref 字段必须服从 D3 identity/resolution。
- 不决定 D5 是否存在 Record、RecordCollection，也不决定它们的 owner、identity 或 persistence；若 D5 选择存在，必须另建不与 NodeRef coercion 的 ref 域。
- 不冻结 D6 的物理目录、sidecar、数据库、事务日志、锁、lease、ACL、同步、历史或 tombstone 存储实现。
- 不冻结 D7 Query/View/Action payload、D8 编辑器 reanchor UX、D9 import/export worker/模板 payload、D10 Agent/自动化/audit 细节。
- 不修改 repos/weftext、公开规范或 brand/。

除明确写为下游选择外，本决议中的必须、不得、仅均是 D3 规范要求。

本完整候选继承既有D2影响边界，并纳入§4.1.3a十数组/Result9符号源组合及D6观察/初始化的共同消费合同。当前请求wireVersion11；operation、receipt、existing-source组合的实际修订以这些当前规范条款为准，不宣称全部仍等同历史D3 v9。EntityRef、身份分域和原阶段先后继续遵守本稿完整规则。

## 2. 问题定义与设计目标

Weftext 必须同时允许：

1. Node 在同一 Workspace 内移动、重排、重命名而不失去引用；
2. 完整备份在灾难恢复时延续原 Workspace identity；
3. 普通复制、Workspace fork、跨 Workspace 转移和外部导入不会制造两个持有同一 identity 的独立可写权威；
4. Document occurrence 可精确定位，但正文编辑不会制造大量 entity churn；
5. Resource/Annotation 保持严格 owner-local，不能靠路径或裸 UUID 跨 owner 漂移；
6. 删除可恢复，永久清理不可恢复，同时旧 ref 可机械区分 known-purged 与 never-known；
7. 权限、离线、修订变化和损坏不会触发标题/路径/fuzzy search 回退；
8. Query、Action、history、backup、Agent 和 audit 的临时 ID 不会被误当成领域 identity。

设计目标是：一个明确 Workspace namespace，三个冻结的 durable content ref 域，三个 revision-bound locator 域，一套封闭 resolver outcome，一套显式 continue/fork/copy identity 规则，以及对所有短期证据 identity 的非转换门禁。

## 3. 术语分域

| 概念 | 定义 | 是否 authoritative identity | 稳定期 |
| --- | --- | --- | --- |
| WorkspaceId | 一个逻辑 Workspace authority namespace | 是 | Workspace authority 生命周期 |
| AuthorityInstanceId | 一个被激活为可写的物理 replica/authority activation 的控制身份 | 否；不进入 content ref | 该物理 activation 生命周期 |
| NodeRef | WorkspaceId + NodeId 的闭合复合引用 | 是 | create 到 tombstone；move/rename/reorder/revision 不变 |
| ResourceRef | owner NodeRef + ResourceId 的闭合 owner-local 引用 | 是 | create 到 tombstone；bytes revision 不变 |
| AnnotationRef | owner NodeRef + AnnotationId 的闭合 owner-local 引用 | 是 | create 到 tombstone；target stale 不变 |
| Document address | owning NodeRef | 否；Document 无第二 identity | 与 NodeRef 相同 |
| AuthorAnchorAddress | NodeRef + document-local author anchor | 否；symbolic locator | 逐 revision 重新解析 |
| RevisionLocator | owner ref + exact revision + kind + exact source/region coordinate | 否 | 只在绑定 revision |
| Path | D6 物理或 portable storage locator | 否 | 可随布局、move、rename 改变 |
| Display label | title、caption、name 或 UI label | 否 | 作者或 UI 可修改，可重名 |
| SourceSpan | exact source 中的坐标区间 | 否 | 只在同一 exact revision |
| Digest/Revision | bytes/source/state 的内容证据 | 否 | 对应内容不变时 |
| Operation/Audit ID | 一次 plan、commit、receipt、history、snapshot 或 event 的身份 | 否 | 对应 operation/evidence 生命周期 |
| LogicalOccurrenceKey | Query 内部一次逻辑 occurrence 的 scoped key | 否 | 只在 D7 定义的 execution namespace |
| ResultRowHandle | 对一个授权结果快照中行 occurrence 的 opaque handle | 否 | 只在 result/auth/revision envelope |
| Provenance | 解释结果来源的证据 | 否 | 继承其中每个 ref/locator 的限制 |
| ActionEvidence | Core 签发的窄、revision/auth-bound 动作证据 | 否 | 一次受限动作窗口 |
| SourceBinding | 一个连接来源实例、授权租户/日历与映射策略的 control-plane scope | 否；不是 Workspace/Node/Record identity | 订阅、导入或 provenance binding 生命周期 |
| ForeignIdentityKey | `SourceBinding + foreign kind + foreign persistent key` 的 source-scoped lookup key | 否；不得进入 content ref decoder | 对应 source binding 可解释期间 |
| DerivedOccurrence | 可由 series rule 与 occurrence key 重建的逻辑 occurrence | 否；默认没有 durable identity | recurring-series rule/cut 仍可重建期间 |
| OriginBinding | Weftext 对象与外部来源之间的 provenance/upsert 关联 | 否；不宣称 identity 相等 | 显式解绑、来源遗忘或下游 retention 终止前 |

任何 path、label、anchor、span、digest、revision、handle、provenance、evidence、event ID 或裸 UUID 都不得进入 durable content ref decoder。

### 3.1 RFC 5545 / ICS 身份与 provenance 压力切片

本切片只冻结 D3 的身份、来源、authority 与生命周期边界；不冻结 calendar/event schema、Record 域、同步存储、Provider token wire、View/Action、导入模板或 UI。RFC 5545 的 `VEVENT` 是逻辑事件组件；其 `UID` 是外部持久标识，但只在准确 SourceBinding 内构成 foreign key。`UID` 永不等于、替代、占用或导出 Weftext NodeId/NodeRef；跨 SourceBinding 的相同 UID 不产生自动相等或合并。`SEQUENCE`、`DTSTAMP`、provider ETag 或同类 token 可以作为版本/冲突证据，但都不是 NodeRef，也不进入 durable content ref decoder。

概念上的 foreign lookup key 为 `ForeignIdentityKey(SourceBinding, componentKind, UID, RECURRENCE-ID?)`。这是 D3 约束的分域和比较规则，不是本决议冻结的 JSON wire。SourceBinding 必须至少区分具体来源实例及其 calendar/collection scope；provider account token、cursor、etag、sync watermark 与授权材料属于控制面，不得作为内容 ref、Node identity 或用户可移植 provenance 的等价替代。SourceBinding 变更、解绑或重建不得静默把另一来源的同 UID 接到既有 NodeRef。

recurring-series anchor component由 `DTSTART + RRULE/RDATE/EXDATE` 等规则定义 occurrence set；同一 source-scoped UID 下带 `RECURRENCE-ID` 的组件是特定 occurrence override。规则可无限或长期延伸，因此默认 occurrence 是 `DerivedOccurrence`：它由 source binding、series-anchor foreign key、occurrence key和准确规则/版本 cut 重建，无 NodeRef、ResourceRef、AnnotationRef 或 locator continuity。override 默认也是 series-owned structured value；D5 若未来定义独立 Record 域，可以从零决定以 Record 表达series-anchor/override，但 D3 不预选、要求或否决该选择。无独立对象也是 derived occurrence 的合法且默认表示。

创建fresh Node的materialization intent是closed union：`initial_import | adopt | promote | managed_copy`。四者不得由UI文案、来源类型或“看起来像复制”互相猜测；请求必须显式声明一个intent，Core在任何allocation前按本节分类。`initial_import`只表示经过有界、可预览、显式mapping的foreign event/series初次导入；`adopt`只表示用户显式选择foreign event/occurrence；`promote`只表示已受Weftext管理的Document occurrence；`managed_copy`只表示现有managed live Node的普通D3 Copy。通用artifact Import仍遵守§11的fresh identity规则，但不得被当作绕过本ICS分类器的别名。

每个intent各有自己的输入形状；`ForeignIdentityKey`与managed Document occurrence locator的二分classifier只在`adopt`/`promote`这对verb之间判别，不得先于intent把合法`initial_import`误拒绝。完整shape分区如下；对每个intent，表外三个`ForeignIdentityKey present/absent × managed locator present/absent`组合一律`materialization_intent_mismatch + preflight_rejection`，零allocation、零binding write、零content/placement/lifecycle mutation且无receipt：

| declared intent | ForeignIdentityKey | managed occurrence locator | 额外authoritative preimage | shape effect |
| --- | --- | --- | --- | --- |
| `initial_import` | present | absent | 有界preview中唯一foreign logical event/series | 进入OriginBinding状态门；不经过adopt/promote二选一classifier |
| `adopt` | present | absent | 用户显式选择的foreign event/occurrence | 进入OriginBinding状态门 |
| `promote` | absent | present | locator精确解析为受管Document occurrence | fresh Node；`activeBindingWrites = 0` |
| `managed_copy` | absent | absent | 唯一managed source NodeRef | 只在source为live时fresh Node；`activeBindingWrites = 0` |

`OriginBindingState`在本分类器中的closed observation为`never_bound | active_live | active_non_live | retired | conflict | not_applicable`；`active_non_live`遮蔽trashed/tombstoned/not_found/not_visible/unprovable的具体存在性，但diagnostic可在授权repair channel细分。状态、allocation、binding write与authority/provenance effect完整如下；所有reject结果均零allocation、零active binding write、零content/placement/lifecycle mutation且无receipt：

| intent | binding/source state | 唯一结果 | allocations | activeBindingWrites | authority / provenance effect |
| --- | --- | --- | ---: | ---: | --- |
| `initial_import` | `never_bound` | `fresh_node_and_sole_binding`；exact retry复用保存decision | 1 | 1 | Weftext成为导入结果authority；保留foreign provenance |
| `initial_import` | `active_live` | `origin_binding_already_active`；转入deterministic re-import/upsert，不fresh | 0 | 0 | authority不变；沿用既有binding |
| `initial_import` | `active_non_live` | `origin_binding_non_live` | 0 | 0 | 不auto-restore、不泄漏具体存在性 |
| `initial_import` | `retired` | `origin_binding_retired_requires_explicit_adopt` | 0 | 0 | ordinary/initial import不得复活retired binding |
| `initial_import` | `conflict` | `origin_binding_conflict` | 0 | 0 | 停止写入 |
| `adopt` | `never_bound` | `fresh_node_and_sole_binding`；exact retry复用保存decision | 1 | 1 | 新Node为独立Weftext authority；保留foreign provenance |
| `adopt` | `active_live` | `origin_binding_already_active` | 0 | 0 | 不制造第二Node或第二binding |
| `adopt` | `active_non_live` | `origin_binding_non_live` | 0 | 0 | 不auto-restore；用户走独立lifecycle流程 |
| `adopt` | `retired` | `fresh_node_and_sole_binding_preserve_retirement_audit` | 1 | 1 | 只有本显式Adopt可重建sole active binding |
| `adopt` | `conflict` | `origin_binding_conflict` | 0 | 0 | 停止写入 |
| `promote` | `not_applicable`且locator resolved | `fresh_node_from_managed_occurrence` | 1 | 0 | Weftext authority连续；不产生foreign provenance/binding |
| `managed_copy` | `not_applicable`且source `live` | `fresh_managed_copy` | 1 | 0 | Weftext authority连续；copy lineage不是OriginBinding |
| `managed_copy` | `not_applicable`且source `non_live` | `managed_copy_source_non_live` | 0 | 0 | 不借copy绕过lifecycle |
| `managed_copy` | `not_applicable`且source `unresolvable` | `managed_copy_source_unresolvable` | 0 | 0 | 遮蔽not_found/not_visible/unprovable |

`initial_import|adopt`若观察`not_applicable`，或`promote|managed_copy`若观察任何非`not_applicable` binding state，固定返回`materialization_binding_state_mismatch + preflight_rejection`。adoption/import永不保留或转换UID为NodeId，也不把occurrence key变成NodeRef。任何创建OriginBinding的成功结果必须在创建Node的同一authoritative decision中写`OriginBinding(ForeignIdentityKey, fresh NodeRef)`；不得另造只按title/path/provider token去重的key。

外部authority模式只描述来源权威，不替代上述identity intent：

| 模式 | authority | materialization 与 identity |
| --- | --- | --- |
| subscribe/sync | 外部calendar/source为authoritative内容来源；Weftext保存受控镜像与来源绑定 | UID/RECURRENCE-ID只作source-scoped upsert key；derived occurrences默认无Node；解除绑定不把UID提升为Weftext identity |
| bounded `initial_import` | Weftext成为导入结果的内容authority | 只在`never_bound`按有界preview为logical event/series建fresh Node与sole active OriginBinding；recurring series若materialize默认至多一个series Node；不展开无界occurrences |
| `adopt` selected foreign event/occurrence | Weftext对新Node成为独立authority，同时保留Provenance | `never_bound`或`retired`按上表fresh-create；与foreign component不identity-equal；双向编辑须显式解决authority/conflict |

deterministic re-import/sync 必须以完全相同的 ForeignIdentityKey 做OriginBinding lookup：master使用SourceBinding + component kind + UID，override另含准确RECURRENCE-ID；SEQUENCE/timestamp/etag等只参与版本排序、冲突或stale证据。unique live只可upsert/no-op/version-conflict候选；active non-live返回`origin_binding_non_live`；scope/cardinality不能唯一证明返回`origin_binding_conflict`；ordinary miss返回`origin_binding_missing`，不得自动改成`initial_import`或`adopt`。已materialize对象的第二份独立副本必须从其managed live Node声明`managed_copy`，且`activeBindingWrites = 0`。

active OriginBinding 对同一ForeignIdentityKey的cardinality恰为0或1。显式retire/detach是binding控制面操作，不删除、恢复或改写Node identity；它必须保留足以防止ordinary re-import把retired误判为初次adopt的审计状态，且只允许后续新的**显式 adopt 请求**建立新的sole active binding。retirement与explicit adopt并发时必须按同一ForeignIdentityKey线性化：retire先胜出时后续explicit adopt可fresh-create并建立sole active binding且保留旧retirement audit；adopt先观察到unique active binding时零allocation/零write并返回already-active，不得在retire提交后把旧尝试自动升级为新adopt。ordinary re-import观察retired只返回requires-explicit-adopt，不能创建Node。D3不冻结retirement wire或retention期限。不存在“不同copy lineage”隐藏key、多条active binding或以title/path/provider token去重。若用户需要第二份内容，先解析unique live primary binding，再走ordinary Copy；primary Node为trashed/tombstoned/not_visible时不得借copy或import绕过相应状态。

同源 title/path/display label 改变不得创建重复 Node；跨源相同 UID 不自动合并。取消、来源删除、用户解除绑定、双向编辑及离线冲突必须分别记录“外部对象状态”“origin/source binding状态”“Weftext Node lifecycle”“authority/conflict状态”，不得用删除Node、改UID或改path来冒充另一维变化。

VEVENT 批量 materialization 只能是有界、可预览且显式声明`initial_import`的策略；按每个展开 occurrence 建 Node 永远不能成为默认策略，也不能接受无上界规则。`VCALENDAR` 中的 `VTODO`、`VJOURNAL`、`VFREEBUSY`、`VTIMEZONE` 不共享 Event identity。尤其`VFREEBUSY`与`VTIMEZONE`在任何authority mode下都不得materialize为Node，即使用户选择显式mapping；下游只能把它们路由为非Node structured value、控制数据或由未来D5从零决定的独立Record-domain候选，D3不替D5选择。`VTODO → Task`、[D3-TERM-EXCLUDE:third_party_format]`VJOURNAL → journal/time note` 等 mapping 由 D4/D9 未来以显式 preview 决定，并继续服从 D2 v2：Task是available Facet set包含exact `tasks/task`的ordinary Node；checklist item与其他Document occurrence没有独立durable identity，二者不得合称为一种“Task/checklist identity”。

D3只要求实现能观察并区分下列概念状态，不冻结其存储枚举：

    source_binding: unbound -> bound_external_authority
                                  -> source_cancelled | source_deleted | conflict | detached
    derived occurrence: reconstructable -> stale_rule_cut | no_longer_in_set
    adopted Node: absent -> fresh live NodeRef -> trashed -> tombstoned

三条状态轴不得互相代写：`source_deleted`不机械等于Node purge，Node trash不等于provider cancellation，`detached`不改变既有NodeRef，derived occurrence离开新occurrence set也不能把一个已adopted Node变回无identity。若下游支持双向sync，authority winner/merge decision是额外可观察的conflict状态；未决时停止写入而不是猜测。

替代表示的 D3 裁决为：derived occurrence 默认“无独立 durable 对象”；override 可为 series-owned structured value；D5 可另选独立 Record/RecordCollection 域；foreign object/occurrence只有显式adoption才可成为fresh Node，managed Document occurrence只有显式promotion才可成为fresh Node。structured value/Record/Node 之间不得通过共享 UID 或裸 UUID 相互 coercion。typed calendar wire、RecordRef、sync 数据结构、provider persistence、Query/View/Action 与 import contract 分别路由 D4、D5、D6、D7、D9、A2，不在 D3 冻结。

### 3.2 Terminology and Naming Gate

D3 的公共concept、wire/API、code symbol/convention、CLI/UI label与locale key必须共同服从 [D3 Terminology and Naming Lexicon](../d3-terminology-and-naming-lexicon/source.md)。每个concept恰一stable concept ID与一组闭合、分surface的正式名称映射；一词一义、全局retired identifier、concept-relative semantic non-alias、D1/D2 frozen entry与受控负向gate都以该Lexicon的结构化entry/全局退役表为产品权威。本次冻结的independent product-conformance evidence已证明Lexicon、Candidate、wire示例与受控名称映射一致，并对缺失依赖、陈旧产品世代或受控名称旁路fail closed；具体证据载体、runtime、OS sandbox、Unicode数据、scanner、mutant、manifest、生成器、validator与replay实现只属于Design Review Execution Protocol及本轮Research/Agent Session evidence，不是D3 public contract、wire、ontology或未来实现约束。自然语言、用户内容、第三方format、历史evidence、migration/deletion note与closed typed counterexample不因该负向gate被粗暴禁止。

负向Gate的可执行scanner、Unicode normalization、source grammar、mutation corpus和运行模式矩阵均为review-infrastructure-only evidence。D3只要求其覆盖受控surface、total-classify每个命中、对规避与依赖缺失fail closed，并与产品Lexicon保持双向一致；任何具体算法、文件、代次或runtime都不随本决议冻结。
冻结门禁如下：

- D1/D2导入entries标记`frozen`，D3发现冲突只能提交最小反例与显式reopen request；不得静默改名、添加兼容alias或用旧code type覆盖上游ontology。
- 每个concept恰一stable concept ID与一对canonical中英文term；wire/API/code/CLI/UI/locale的不同拼法必须一对一映射该ID。同一裸term不得多义，用户口语class/instance与历史实现名不自动成为domain concept。
- Node不是note/item/Document；Document没有第二identity；Entity只表示`Node|Resource|Annotation` closed meta-union而不是第四种wire kind；Occurrence是无durable identity的元术语，Document Occurrence与Derived Occurrence必须用完整限定名且都不获得默认Node identity。
- Typed Reference、Reference、Node Link、Citation与未来D4 Relation互不作同义词；D3只冻结target identity/resolution，Library citation semantics路由D4/D7。
- Resource是owner-local domain object；attachment只可作映射到Resource concept的UI role，file/blob/path/digest/External URI都不能作Resource identity或code kind alias。
- Owner、Authority、Source、SourceBinding、Provenance与OriginBinding分别表示归属、决策权、输入来源、foreign-key scope、解释证据与foreign→Weftext映射；任何两者不得共享裸[D3-TERM-EXCLUDE:quoted_counterexample]`origin`/`source`/`owner`字段规避分域。该marker只排除被引用的退役裸[D3-TERM-EXCLUDE:quoted_counterexample]`origin`，`source`与`owner`仍必须分别映射Source与Owner concept。中文controlled label中裸“来源”只属于Source，Provenance必须显示“来源证据”，不得以context重新复用裸term。
- Copy/Fork/Continue/Move/Import/Adopt/Promote的fresh/preserve/provenance effect由Lexicon逐项固定；[D3-TERM-EXCLUDE:migration_deletion_note]`clone`退役。Subscribe/Sync/Connect只冻结negative identity effect，具体workflow路由D6/D9/A2。
- 退役词负向gate只检查受控schema/wire identifiers、public symbols、CLI/locale keys和规范性标题；不得扫描或重写自然语言、历史evidence、migration note、quoted counterexample或用户Document/Annotation内容。

R0删除或替换只采用Lexicon §6 `retired-controlled-identifiers`中的case-folded exact项及其唯一replacement/deletion target；`clone*`、`master`、裸`origin`等concept-relative semantic non-alias只按调用点声明的expected concept/typed role裁决，不得成为flat retired-token规则。不存在第二套退役名称分配产品权威。Weftext未发布，不建立兼容parser、双读双写或deprecated alias surface；任何下游新增/改名必须先更新Lexicon对应entry或§6 exact global-retired table，并通过one-term-one-meaning、排除边界、中英文、wire/code/surface、用户理解和migration/deletion审查。

## 4. 身份代数

### 4.1 ID 词法与分配

WorkspaceId、AuthorityInstanceId、NodeId、ResourceId、AnnotationId 使用 canonical lowercase RFC 4122 UUIDv4 文本。选择 UUIDv4 是为了不泄露时间或暗示排序；产品顺序只来自 siblingOrdinal 或下游显式 order。本文所有D3Integer共享`MAX=2^63-1=9223372036854775807`；后文第一次出现`MAX`时即指此常量，不依赖宿主整数宽度。

规范要求：

- 仅authoritative Core可mint或reserve durable ID；客户端、UI、worker、Agent和import provider只能提出创建意图。Node/Resource/Annotation content ID只可在绑定准确OperationId的commit plan中reserve；create/fork所需WorkspaceId只可经§4.1.2 Core-issued WorkspaceAllocationProposal预签发，不能由调用方选择。
- 预览可显示 reserved proposed IDs，但在 commit 前它们不是 entity。reservation 只属于该 OperationId：同一 OperationId 的幂等重试必须复用同一组 reservation，不同 OperationId 不得取得这些 ID。
- Core持久化`terminal_failed`决议时，全部未提交reservation必须在同一决议中永久burn；可恢复的planned crash不是terminal abort，不得提前burn。burned ID是不可复用的allocation history，不是entity或tombstone；authorized resolver对它仍返回not_found，但allocator永远不得把它分给未来operation。
- 同一 WorkspaceId 内 NodeId 永不复用；同一 owner NodeRef 内 ResourceId、AnnotationId 永不复用，包括 permanent purge 后。
- 相等比较必须比较完整 typed ref；不得只比较 UUID leaf。
- UUID variant/version、canonical lowercase、必填字段和闭合 kind 均由 decoder 验证。
- 内容相同、标题相同、路径相同、来源相同、摘要相同或导入 provenance 相同不产生 identity 相等。

#### 4.1.1 OperationId ledger

除§4.1.2预签发但尚非entity的WorkspaceId proposal外，每个改变identity/lifecycle的请求在任何content reservation之前，先以canonical request绑定一个OperationId。OperationId的命名空间固定为Workspace-local，ledger key恰为`(boundWorkspaceRef.workspaceId, operationId)`；相同UUID可在不同Workspace独立使用，不发生跨Workspace conflict，也不需要全局OperationId authority。本文`CAS`固定指compare-and-swap：一个不可分割decision比较全部列出的expected authoritative state并仅在全部相等时写入全部列出的poststate；CAS loser零写且必须按指定restart point重判，不能做部分effect。OperationId ledger是控制/幂等事实，不是entity identity，状态机闭合为：

    unseen
      -> rejected(requestFingerprint, errorBytes)
      -> planned(requestFingerprint, reservations)
          -> committed(receiptBytes)
          -> terminal_failed(errorBytes)

- §13 唯一 authoritative stage 表、本节 `unseen | rejected | planned | committed | terminal_failed` ledger 状态与 allocation A1–A11 表是唯一 gate/decision 语义；request 矩阵和测试只能引用它们，不能改写或补充平行优先级。stage 1 仅做不访问 authoritative state 的 lexical/closed-shape/static cross-field 检查；真实 closure、payload、ref slot 或 current lifecycle 只能在授权和 authority gate 后检查。
- stage 3 对 ordinary operation 授权，create/fork 只对 proposal issuer、当前 principal 与 fork source 授权；失败固定 `identity_not_visible`，不得读取 target Workspace ledger、closure、revision 或 existence。stage 4 只检查该次 stage 3 已授权的现有 authority/source 的 availability、continuity 与 integrity；尚未 activated 的 target Workspace 不得被当作必须存在的 authority。
- create/fork 在 stage 4 后、任何 target ledger content read 前依次执行 proposal gate `P1` 与 `P2`。P1验证精确token body、current-principal audience、issuer及纯字节request binding。P2只读取issuer allocation family，接受：(a) current active proposal；(b)同OperationId的claimed/activated/terminal-failed winner；(c)同OperationId且同requestFingerprint的`decision_bound_rejected` proposal。replacement烧毁的predecessor、unclaimed-retired proposal及不同fingerprint均不接受。任一失败都是`workspace_identity_conflict + preflight_rejection`：target ledger availability/content zero-read、target ledger zero-write、family zero-effect。
- stage5 content/fingerprint read前必须执行唯一`TargetLedgerGate`，但入口只有两条：`create_workspace|fork_workspace`固定`stage4 → P1 → P2 → TL1 → TL2 → stage5`；全部ordinary mode（明确包含`continue_workspace`）固定`stage4 → TL1 → TL2 → stage5`，不经过P1/P2。proposal签发时issuer原子建立`TargetLedgerCustodyRecord(targetWorkspaceId, custodianAuthorityInstanceId, custodyGenerationToken, allocationFamilyKey)`；现有Workspace则已经有同形custody record。target未activated、decision_bound_rejected或terminal_failed-before-activation时custodian是连续issuer authority；planning/claim期间仍由issuer托管并与family/ledger CAS同域；commit激活后custody原子移交targetAuthorityInstanceId；continue/failover后移交经验证的新active authority。stage4只验证本次已授权的current authority/source/cut，不读取bound OperationId ledger、saved bytes或allocation/burn continuity；这些bound-ledger custody事实只由TL验证。TL1不可达或continuity/custody proof不可证明固定`identity_authority_unavailable`；只有TL1通过且custodian可达后，TL2发现duplicate custody/ledger record、key/state collision或saved bytes不唯一才为`workspace_integrity_conflict`。TL gate只检查custody/availability/continuity/integrity metadata，不读取ledger key内容；create/fork的P1/P2失败时它绝不运行，ordinary没有P1/P2前置。
- TL1/TL2通过后才执行stage5 target ledger/fingerprint gate。ordinary request携带continuity-proven的旧generation token时，stage4/TL只作**provisional admission**以允许读取ledger并重放saved decision；它不授权新的decision。stage5若key existing则按fingerprint replay/conflict；若key unseen，必须在任何stage7 read或decision write前比较request token与current active generation：byte-equal才继续，continuity-proven但older固定返回`identity_authority_unavailable + preflight_rejection`，ledger只发生一次read、零write、零reservation；unrelated/unproven token已在stage4失败。stage5 unseen且current-token的请求在stage7–15失败时必须在一个decision CAS中同时比较`expected ledger=unseen`与（create/fork）`expected family=current active P + same formal request role`，然后原子写`rejected`、把proposal转为`decision_bound_rejected(P,target,OperationId,requestFingerprint)`并burn target IDs。该状态只为exact saved-rejection replay开放P2，不可replacement/claim/activate。CAS loser零写、零burn、零retire：create/fork从P2重新判定，若另一fingerprint rejection/replacement已胜出固定在P2得到workspace_identity_conflict，同fingerprint winner则经P2→TL→stage5 byte-equal replay；ordinary operation直接经TL→stage5，same fingerprint replay、different fingerprint operation_id_conflict。stage1–4、P1/P2、TL1/TL2、stage5 old-generation unseen或existing-ledger conflict不新建decision。
- `unseen` 只有完整通过 §13 stage 1–15（create/fork还已通过P1/P2/TL1/TL2；ordinary还已在stage5证明expectedAuthority token等于current generation）后，才在一个不可分割 planning decision CAS 中同时：再次验证同一 authoritative cut、比较expected target ledger=unseen、再次比较ordinary expected generation=current、对create/fork比较同一current proposal family/formal role、写入 `planned(requestFingerprint, canonicalPlan, reservations)`、reserve content IDs，并在create/fork时claim proposal-family winner。CAS loser服从上一条同一restart规则，绝不写第二plan/reservation；若CAS时generation变化，零写并从ordinary TL1重新判定，旧token在新的stage5 unseen分支固定失败。不存在“某个validation stage已经post-planned”的第二解释。
- `planned` 保存已经通过 stage 1–15 的 canonical request、完整 preflight cut 与 canonical plan。same-fingerprint recovery 在当前授权、authority/continuity、proposal-family 与TL/ledger gate通过后复用这些事实和原 reservations，不重新读取一套可漂移 closure来重判request。若TL1或D6无法证明planning cut仍可作为同一原子commit基础，则保持`planned`并返回`identity_authority_unavailable`；不得产生第二计划或burn。
- `planned` 下只有 fingerprint 完全相同的请求可恢复执行并复用原 reservation；不得重新分配。不同 fingerprint 使用同一 OperationId 固定返回 `operation_id_conflict`，且不暴露原请求内容。
- crash 后不能证明 commit 的 `planned` 请求可以用同 fingerprint 恢复；只有 Core 作出并持久化 terminal decision 时才转 `committed` 或 `terminal_failed`。
- `rejected`、`committed`与`terminal_failed`中的ledger decision和保存bytes不可变；但“不可变决议”不等于“无条件交付”。只有当前调用通过stage 3–4、适用的P1/P2、TL1/TL2并在stage 5提交相同fingerprint时，rejected replay才返回保存的byte-equivalent recorded rejection，committed replay才返回保存的byte-equivalent receipt，terminal_failed replay才返回保存的byte-equivalent terminal error。权限撤销后的重放返回`identity_not_visible`且不改变决议；custody/authority/continuity暂不可证明返回`identity_authority_unavailable`且不改变决议。
- authority generation变化后，新active authority只有在continue/failover gate已验证同一Workspace ledger、saved decision bytes、allocation/burn history连续时，才可在当前授权通过后交付旧rejected/committed/terminal_failed结果。无法证明连续性固定`identity_authority_unavailable`；不得重执行、重新分配或产生第二decision。`planned`也只有在相同fingerprint、reservation与ledger连续性均可证明时恢复，否则保持planned并返回authority unavailable。
- 进入 `terminal_failed` 时该 OperationId 的全部未提交 reservation 同一原子决议永久 burn；burned ID 永不能进入 live。新意图必须使用新 OperationId。
- 所有 D3 内容依赖 validation 都在 planning boundary 前完成。进入 `planned` 后只有三个合法结果：恢复同一计划并 `committed`；保留 `planned` 等待可证明恢复；或由适用下游 gate 作出“该计划永不提交”的单一 authoritative abort，原子写入 `terminal_failed(errorBytes)` 并 burn 全部 reservations。D3 v11 对后一情况只允许 `disposition=terminal_failure, family=identity_commit_aborted`；它不透露或替 D6 定义物理失败原因，不得重新运行 stage 1–15 改变 primary outcome。其余 stage family 只能用于 `preflight_rejection` 或 `recorded_rejection`。

#### 4.1.2 Workspace allocation proposal

`create_workspace`和`fork_workspace`在正式operation request前必须由Core签发closed `workspace_allocation_proposal`；调用方不得构造target WorkspaceId或target AuthorityInstanceId。wireVersion固定11。每个proposal属于签发Core内唯一、持久的allocation-intent family。调用方可以生成`allocationIntentId`作为签发API幂等control key，但它只在签发Core与当前受权principal的scope内有意义，不是content identity、全局OperationId或Workspace选择权；target WorkspaceId、target AuthorityInstanceId与winner只由Core决定。不同allocation-intent family可以显式复用同一UUID OperationId，因为最终ledger key仍是Workspace-local且每个proposal target Workspace不同；D3不引入跨family/global OperationId registry。

签发输入也closed：initial `workspace_allocation_issue_request`字段恰为`wireVersion, kind, allocationIntentId, operationId, mode, sourceWorkspaceRef?, sourceSnapshotCutToken?`；replacement `workspace_allocation_replace_request`字段恰为`wireVersion, kind, allocationIntentId, expectedSequence, replaceProposalId`。initial的fork/create source规则与proposal相同；replacement必须准确指向当前active proposal/sequence。相同canonical request只有在成功或已保存的A7–A11确定性失败时才由family record保证byte-equal replay；A1–A5是每次按当前state重新执行的preflight，不创建或保存negative attempt。initial response完全丢失时重发同一issue request会在未被其他合法状态转换改变family时取得同一P0；若当前state已被另一个授权操作改变，则服从A1–A11当前顺序，不宣称历史失败重放。只有已经知道current proposalId的显式replace request才能生成successor。

allocation control outcome 是与正式 OperationId ledger 分离的 closed control wire：

- `workspace_allocation_issue_outcome` 的成功variant字段恰为`wireVersion,kind,status,proposal`且`status=issued`；失败variant字段恰为`wireVersion,kind,status,family`且`status=rejected`。
- `workspace_allocation_replace_outcome` 的成功variant字段恰为`wireVersion,kind,status,proposal`且`status=replaced`；失败variant字段恰为`wireVersion,kind,status,family`且`status=rejected`。
- allocation family 闭合为`invalid_allocation_request | allocation_not_visible | allocation_authority_unavailable | allocation_integrity_conflict | allocation_intent_conflict | allocation_family_retired | allocation_already_claimed | allocation_sequence_conflict | allocation_sequence_exhausted`。outcome不得附带principal、旧request、旧proposal、path、repair detail或私有transport status。

allocation control 是下列唯一总序：A1 closed decode/static source-mode matrix→`invalid_allocation_request`；A2 current principal 对 issuer/allocation control API 的授权→`allocation_not_visible`，不得读取任何family；A3 issuer authority unavailable→`allocation_authority_unavailable`；A4 issuer authority record不唯一/损坏→`allocation_integrity_conflict`；A5 对current-principal scoped key执行一次lookup，若请求是replace且该scoped family不存在，则→`allocation_not_visible`；此结果与wrong-principal不可区分、family zero-effect、不得生成proposal或保存到不存在的family；issue在scoped family不存在时不命中A5并继续；A6 exact canonical request已有保存outcome→byte-equal replay；A7 issue命中同family不同initial bytes→`allocation_intent_conflict`；A8 replace family已retired→`allocation_family_retired`；A9 已claimed→`allocation_already_claimed`；A10 active family但expectedSequence或replaceProposalId不等current active→`allocation_sequence_conflict`；A11 active sequence已为`9223372036854775807`且请求replacement→`allocation_sequence_exhausted`；其余才线性化issued/replaced成功。每个成功或A7–A11确定性失败都在返回前按canonical request digest保存outcome bytes；A1–A5是current-state preflight recomputation，没有可安全绑定的新family decision且不修改family。A5 exact retry只有在state未变时才由同一纯计算得到byte-equal `allocation_not_visible`；失败replace后同allocationIntentId的合法issue仍可在scoped family absent前提下创建首份family。故固定反例 `replace_absent → issue(sequence=0) → 重发原replace(expectedSequence=1)` 的三个outcome必须依次为 `allocation_not_visible, issued, allocation_sequence_conflict`，第三步在A10关闭，绝不得为历史`allocation_not_visible`。A11保持原proposal/family active且不做加法、wrap、burn或replacement。并发claim/replace只看同一线性化点，已有family的exact retry命中A6。

proposal字段恰为：

~~~json
{
  "wireVersion": 11,
  "kind": "workspace_allocation_proposal",
  "allocationIntentId": "a8407ae8-8a85-45d7-8c60-e571bf72dd5c",
  "sequence": 0,
  "proposalId": "0bb09158-8e2f-4861-826c-29f50a0fdb9d",
  "principalAudienceToken": "opaque-principal-audience-token",
  "targetWorkspaceRef": {"kind":"workspace_ref","workspaceId":"b6b8917e-f226-4e2a-8e2d-5a956ef16ea1"},
  "operationId": "bc550154-f436-4f5f-adcb-7458df568d34",
  "mode": "create_workspace",
  "issuerAuthorityInstanceId": "6c9aa848-7432-49b5-b6b3-f5a7a2fd7a0f",
  "targetAuthorityInstanceId": "d2dce9e1-bc98-4a17-a05e-72b0f296a4e9",
  "issuanceToken": "opaque-core-authenticity-token"
}
~~~

proposal字段恰为示例中的字段；fork variant另有且仅有`sourceWorkspaceRef,sourceSnapshotCutToken`。`allocationIntentId, proposalId, operationId, issuerAuthorityInstanceId, targetAuthorityInstanceId`是canonical lowercase UUID；`sequence`属于§4.1.4 common integer domain，首份固定0，replacement逐一加1且不可跳号/回退，max时服从A11。`principalAudienceToken`与`issuanceToken`都是non-empty opaque token；前者由issuer在P1映射到当前authenticated principal，后者真实性由issuer验证，D3不冻结MAC/签名格式。`ProposalTokenBody`精确定义为proposal object移除`issuanceToken`后的closed object，仍含`principalAudienceToken`；待认证bytes固定为ASCII `D3-Workspace-Allocation-Proposal/7`、一个NUL、再接`D3-CJ/3(ProposalTokenBody)` UTF-8 bytes。这里`/7`是proposal-authentication framing profile版本，和closed object内的D3 wireVersion `11`是独立version domain；二者不得相等推断、联动升级或互相替代。issuanceToken必须认证恰好这些bytes，不得少绑字段、接受另一serialization或宣称绑定尚未存在的formal request。formal request自身由requestFingerprint绑定完整proposal digest与intent；同proposal的竞争formal requests由target ledger/family planning CAS线性化。`mode`闭合为`create_workspace | fork_workspace`；fork额外字段绑定准确source Workspace与snapshot cut。每个proposal的target WorkspaceId和target AuthorityInstanceId都由issuer新mint/reserve且此前从未allocated/proposed/burned/activated；两者在planning boundary原子claim前都不是activated authority/entity。

proposal状态机闭合为：

    allocation_intent_family(issuerAuthorityInstanceId, principal, allocationIntentId)
      -> active_issued(sequence n, proposal Pn, target Wn/target authority Tn; both reserved;
                       issuer-custodied target ledger namespace L/Wn)
          -> active_issued(sequence n+1, proposal Pn+1, target Wn+1/Tn+1)
             [atomic replacement: Pn/Wn/Tn burned before Pn+1 becomes active]
          -> active_issued(sequence MAX, same proposal/family)
             [replacement rejected as allocation_sequence_exhausted; no state change]
          -> claimed_winner(Pn, Wn, Tn, same OperationId ledger planned; issuer custody retained)
               -> activated(ledger committed; atomic custody handoff to Tn;
                            Tn is Wn initial active authority)
               -> terminal_failed_winner(ledger terminal_failed; Wn/Tn burned)
          -> decision_bound_rejected(Pn, Wn, Tn, OperationId, requestFingerprint; Wn/Tn burned)
          -> retired_unclaimed(all target Workspace/Authority IDs burned)

- issue只由issuer Core选取从未分配、从未propose、从未burn/destroy的WorkspaceId和AuthorityInstanceId。proposal与allocation-intent family都不是Workspace entity、content ref、write capability或跨Core全局OperationId authority；family key恰为`(issuerAuthorityInstanceId, authorized principal, allocationIntentId)`，并永久绑定首份issue request的operationId/mode/source/cut。相同key不同issue bytes固定`allocation_intent_conflict`，不能重绑定family；不同family复用同一OperationId UUID合法且不要求全局/cross-family registry。
- proposal签发与target WorkspaceId reservation同一原子边界还建立唯一TargetLedgerCustodyRecord；它是幂等/连续性控制事实，不是target Workspace已存在、不是content identity，也不给调用方写能力。replacement必须同一原子决议burn旧target IDs并retire旧custody namespace后建立successor custody；任何stale predecessor都不能访问或修改successor ledger。
- replacement issuance必须在family记录上串行化：只有`active_issued(Pn)`且sequence<MAX可被替换；同一原子决议先把Pn/Wn/Tn永久burn，再发布n+1。claimed返回`allocation_already_claimed`；decision_bound_rejected/retired返回`allocation_family_retired`。任何stale predecessor request都不得修改、retire、burn或阻塞current successor；只有current active proposal自己的recorded failure可转decision_bound_rejected。并发claim/replacement线性化点唯一。
- 正式v11 request的`boundWorkspaceRef`必须byte-equal proposal.targetWorkspaceRef，`operationId/mode`及fork source/cut必须完全相等；create/fork的`workspaceProposal`顶层字段必需，其他mode禁止，intent不得另带或覆盖issuer/target AuthorityInstanceId。pure cross-field mismatch在§13 stage 1或2拒绝。P1按上文精确token body验证authenticity、current-principal audience、issuer与纯字节request binding；P2验证current family relation；二者失败均不得读取target ledger。committed receipt若不能回显proposal中的两个authority字段属于authority integrity failure，不得发布receipt。
- Core只在完整通过stage1–15后的planning boundary decision CAS中，把active proposal原子转claimed_winner并写planned ledger。fork closure在stage14完成；stage5 unseen的current proposal在stage7–15失败时用同一CAS模型写recorded rejection与decision_bound_rejected并burn target IDs，exact retry经P2(c)+TL1/TL2+stage5重放。P1/P2失败保持target ledger/family/IDs不变，decision CAS loser服从唯一restart rule，tampered-first与late predecessor均不能伤害genuine/current successor。
- claimed后的lost-response exact retry必须重交byte-equal proposal和canonical request；因此只恢复同一planned或返回原committed/terminal_failed bytes。不得签发/接受replacement；family winner不能改变。
- terminal_failed、replacement与unclaimed retirement都永久burn对应target WorkspaceId及target AuthorityInstanceId；activated后销毁的WorkspaceId与AuthorityInstanceId同样永不再次propose。proposed/claimed未commit/burned/已销毁WorkspaceRef在authorized Workspace resolver统一`workspace_unavailable`，但allocator保留no-reuse事实；普通resolver不区分这些原因。
- proposal issuance response丢失时先重发byte-equal issue/replace request并取得byte-equal原proposal；不需要replacement。若调用方已持有current proposal且显式请求replacement，旧proposal在新proposal发布前原子burn；旧response后来提交时在P2失败且对successor无作用。不同allocationIntentId是新的显式bootstrap意图，不是原意图retry；它可以复用同一OperationId UUID，因为新的target Workspace-local ledger key不同，且A1–A11不维护也不需要global/cross-family OperationId registry。
- 在第13节stage 3–4与P1/P2中，create_workspace/fork_workspace的authorization、family availability与authenticity上下文是proposal.issuerAuthorityInstanceId所指的bootstrap authority；fork还验证source authority/cut。不得要求尚未activated的target Workspace或proposal.targetAuthorityInstanceId已有active authority/entity record。P2后TL1/TL2只验证TargetLedgerCustodyRecord，未激活时custodian仍是issuer；commit时custody与target authority activation原子移交。committed create/fork receipt.targetAuthorityInstanceId必须byte-equal proposal字段；continue/failover不使用proposal但必须验证并移交同一ledger custody。

#### 4.1.3 Closed canonical request 与 requestFingerprint

`identity_operation_request` wireVersion固定为11。顶层字段恰为`wireVersion, kind, operationId, boundWorkspaceRef, mode, expectedAuthority, workspaceProposal?, preparationBinding?, intent`；unknown/missing/null/duplicate拒绝。`kind`固定`identity_operation_request`。fingerprint不包含top-level operationId；create/fork把完整canonical proposal计算为`workspaceProposalDigest = ASCII "sha256:" + lowercase_hex_64(SHA-256(D3-CJ/3(workspaceProposal)))`。摘要字符串总长恰为71个ASCII byte，冒号后恰为64位`0–9a–f`；uppercase、缺位、多位、base64、raw digest或其他前缀固定stage 1 `invalid_request_matrix -> invalid_identity_ref`。proposal中的allocationIntentId、sequence、proposalId、principalAudienceToken、target Workspace/Authority、operationId、mode、issuer authority、source/cut与issuanceToken全部受到摘要绑定。进入fingerprint的字段恰为`wireVersion, kind, boundWorkspaceRef, mode, expectedAuthority, workspaceProposalDigest（适用时）, preparationBinding（适用时）, intent`。issuanceToken只认证精确ProposalTokenBody，不宣称绑定formal intent；formal request的完整plan、result slots与payload由本fingerprint绑定。改变proposal或intent任一规范byte都会改变fingerprint。

`expectedAuthority` closed为`{"mode":"existing"|"create"|"continue","generationToken":<non-empty opaque token>?}`，逐operation mode只有下表一种合法shape；不匹配是纯request matrix错误，stage 1固定`invalid_identity_ref`：

| operation mode | required expectedAuthority.mode | generationToken | unseen | planned recovery | saved decision replay |
| --- | --- | --- | --- | --- | --- |
| create_workspace/fork_workspace | create | forbidden | proposal issuer/audience/token/family必须按stage3–4、P1/P2有效 | byte-equal原字段；用bootstrap family continuity恢复 | byte-equal原字段与完整proposal digest且P1/P2通过后才可取回 |
| continue_workspace | continue | required | token必须绑定被接续authority generation与cut | byte-equal旧token；只在continuity已证明时恢复 | byte-equal旧token；continuity已证明时交付 |
| 其余ordinary modes | existing | required | token必须等于当前bound Workspace generation | byte-equal旧token；generation改变时只在ledger continuity已证明后恢复 | byte-equal旧token；generation改变时只在continuity已证明后交付 |

token表达调用方绑定的authority generation，不是content identity。planned/rejected/terminal replay不得把旧token改写为新generation token；改写会改变fingerprint并在stage 5得到`operation_id_conflict`。

proposal replay翻转矩阵唯一为：nested `operationId`或`mode`与top-level不等是stage 1 `invalid_identity_ref`；`targetWorkspaceRef`与bound role不等是stage 2 `workspace_ref_misbound`；issuer/source authorization或availability变化分别可在stage 3/4遮蔽；token-body任一字段或issuanceToken/audience改变而不再真实性成立，在P1返回不读target ledger的`workspace_identity_conflict + preflight_rejection`；真实性成立但不再是current/同winner proposal在P2同样preflight拒绝且不改变successor。只有另一份独立真实、current且request-role-correct的proposal通过P1/P2后，才可在stage 5读取其target ledger；若该key已有不同fingerprint才为`operation_id_conflict`。因此不存在“所有字段flip都stage 5”的伪矩阵，也不存在saved-decision replay绕过proposal equality/authenticity。

`intent`是下列closed tagged union，variant kind必须等于mode，表中未列字段禁止：

| mode | intent required fields |
| --- | --- |
| create_workspace | `kind, plan`；target/OperationId/authority由required workspaceProposal唯一给出 |
| create_node | `kind, destinationParentRef, destinationOrdinal, plan` |
| create_resource/create_annotation | `kind, destinationOwnerRef, plan` |
| copy_node_subtree | `kind, sourceWorkspaceRef, sourceRef, destinationParentRef, destinationOrdinal, plan` |
| copy_resource/copy_annotation | `kind, sourceWorkspaceRef, sourceRef, destinationOwnerRef, plan` |
| fork_workspace | `kind, plan`；source/cut/target/authority由required workspaceProposal唯一给出 |
| continue_workspace | `kind, sourceWorkspaceRef, continuationCutToken, plan`；caller-supplied `authorityInstanceId`禁止 |
| move_node | `kind, subjectRef, destinationParentRef, destinationOrdinal, plan` |
| reorder_node | `kind, subjectRef, destinationOrdinal, plan` |
| trash/purge | `kind, subjectRef, plan` |
| restore | `kind, subjectRef, location, plan`；location按subject kind服从下述closed restore矩阵 |
| import_new | `kind, artifactClass, sourceWorkspaceRef?（按下文矩阵）, destinationParentRef, destinationOrdinal, plan` |

`destinationOrdinal`是非负整数。D3 v11 没有泛化的 rename/display-label mutation mode；title、path、UI label 与 occurrence presentation 只能由各自下游的typed action承载。`plan`固定为：

~~~json
{
  "definitionTransfers": [],
  "existingPayloadEdits": [],
  "freshSubjects": [],
  "selectedObjects": [],
  "referenceDispositionPlan": [],
  "structuralPlan": [],
  "resultPlacementPlan": [],
  "resultLifecyclePlan": [],
  "resultTrashPlacementPlan": [],
  "payloadBindings": []
}
~~~

- `freshSubjects`是create graph与ordinary import在reservation前的closed subject declaration/binding array。元素恰为以下variant之一：
  - `create_fresh_subject`：`{"kind":"create_fresh_subject","role":"primary"|"member","subject":<new_result_subject>}`；只允许`create_workspace|create_node|create_resource|create_annotation`。每个create request恰一`role=primary`，其subject kind、owner与intent mode精确匹配：Workspace/Node primary为new Node，Resource/Annotation primary分别为同kind且ownerSubject物化为intent destination owner；其余均为member。
  - `artifact_fresh_subject`：`{"kind":"artifact_fresh_subject","artifactObjectKey":<ArtifactObjectKey>,"subject":<new_result_subject>}`；只允许`import_new+ordinary_format`。`ArtifactObjectKey`恰为`{"kind":"artifact_object_key","artifactSha256":<64 lowercase hex>,"partOrdinal":<D3Integer>,"objectOrdinal":<D3Integer>}`。同一artifact part内objectOrdinal从0连续无洞；key按`artifactSha256,partOrdinal,objectOrdinal`形成总序且全plan唯一。
  `create_fresh_subject`按`roleRank(primary=0,member=1),subjectCanonicalKey`排序；`artifact_fresh_subject`按`ArtifactObjectKey,subjectCanonicalKey`排序；两variant不得混用。create graph中每个declared subject必须恰有一条result payload binding和一条resultAllocations；每个Node另恰有initial placement，每个Resource/Annotation的ownerSubject必须能由同graph或existing subject物化。不得出现undeclared new subject，也不得声明无payload、无placement/owner或无allocation的subject。ordinary import的fresh分区artifact-local key→subject bijection进入canonical request/fingerprint；Core先按ArtifactObjectKey排序，再在每个`(entityKind,ownerSubjectCanonicalKey?)`域分配从0连续的subject ordinal，绝不得用transport、provider、UI、payload digest、文件名或宿主枚举顺序。
- `selectedObjects`元素closed为`{"ref":<NodeRef|ResourceRef|AnnotationRef>,"disposition":"include"|"omit","reason"?:"unreconstructable_target"|"explicitly_omitted"|"reply_closure"}`。include禁止reason；omit必需reason且只允许`copy_node_subtree`与`import_new+partial_identity_bearing_transfer_bundle`中的Resource/Annotation，Node永远不得omit。copy root、全部live descendant Node都必须include；fork任何member禁止omit。restore request只允许include，already-live/tombstoned排除是stage 9 authoritative结果，不编码为request omit。按ref排序，ref唯一；完整omission closure见下文。
- `referenceDispositionPlan`是以下closed union，所有variant都有且仅有所列字段：
  - `preimage_reference_disposition`：`{"kind":"preimage_reference_disposition","fromSource":<ReferenceSlotAddress>,"from":<SlotCompatibleTarget>,"action":"map_target"|"preserve_exact"|"preserve_suspended"|"rewrite_to_existing"|"delete"|"suspend_with_result","resultSlot"?:<ReferenceResultSlotKey>,"resultContext"?:<ReferenceResultContextKey>,"existingTarget"?:<SlotCompatibleTarget>}`。delete必需resultContext且禁止resultSlot/existingTarget；其他action必需resultSlot且禁止resultContext；只有rewrite_to_existing另必需existingTarget。map_target结果由mapped/new target机械导出。`suspend_with_result`只在source payload/revision确实被本operation改写、准确target同时live→trashed且postimage slot仍byte-equal时合法。
  - `result_only_existing_reference`：`{"kind":"result_only_existing_reference","resultSlot":<ReferenceResultSlotKey>,"existingTarget":<SlotCompatibleTarget>,"expectedLifecycle":"live"|"trashed"}`，用于create/ordinary import等没有typed preimage、但结果slot指向既存target的E segment。
  - `result_only_fresh_reference`：`{"kind":"result_only_fresh_reference","resultSlot":<ReferenceResultSlotKey>,"targetTemplate":<FreshSlotTargetTemplate>}`，用于结果slot指向本operation fresh target的M/N segment。
  - `lifecycle_only_reference_transition`：`{"kind":"lifecycle_only_reference_transition","source":<ReferenceSlotAddress>,"target":<SlotCompatibleTarget>,"expectedPrestate":"non_live_source"|"resolved"|"suspended","expectedPoststate":"resolved"|"suspended"}`。它只允许`trash|restore`且只在source authoritative payload bytes与revision token同时byte-equal不变时使用；closed组合恰为：trash准确target `resolved→suspended`；restore source且target live `non_live_source→resolved`；restore source且target仍trashed `non_live_source→suspended`；restore准确target `suspended→resolved`。`non_live_source`表示同一slot存在于trashed source container但不是live graph reference；它不是`absent`。其他组合全部stage1 `invalid_request_matrix`。该entry没有`resultSlot`、`resultContext`或`existingTarget`，不要求result payload binding，不产生M/N/E/S，不进入`rewrittenReferences`；整个entry进入canonical request与requestFingerprint，并与receipt中完全相同`source,target,prestate,poststate`的一条`referenceLifecycleTransitions`一一对应。若source payload或revision改变，必须使用适用的preimage/result action并仍在receipt列transition，不能以lifecycle-only双表示同一slot。
  `ReferenceResultSlotKey`字段恰为`{"kind":"symbolic_result_slot","container":<existing_entity_subject|mapped_result_subject|new_result_subject>,"payloadKind":"exact_source_document"|"annotation_value","slotOrdinal":<D3Integer>,"slotKind":"node_link"|"citation"|"resource_occurrence"|"annotation_target"|"annotation_reply"}`，其中container禁止`artifact_part_subject`，且container entity kind必须与payload严格相容：Node↔`exact_source_document`，Annotation↔`annotation_value`；Resource container、`resource_bytes`与`import_artifact`都不得成为Result/9 slot。exact-source Document只允许`node_link|citation|resource_occurrence`，ordinal按D2 decoded authored slot顺序从0起；Annotation Value/3只允许`annotation_target@0|annotation_reply@1`，明确不由canonical member/span出现顺序决定。ordinal从来不是byte offset或segment index。M/N的targetSubject在可判定处必须与slot相容：node_link/citation→Node，resource_occurrence→Resource，annotation_reply→Annotation，annotation_target→Node或Resource subject。S必须精确绑定existing Annotation container、`annotation_value`、`annotation_reply@1`、同一subjectRef、non-empty expectedAnnotationRevisionToken；非null replyTo须为same-owner AnnotationRef或§4.1.3a的fresh_annotation_reply且物化为same-owner，null只表示移除。`ReferenceResultContextKey`字段恰为`{"kind":"symbolic_result_context","container":<existing_entity_subject|mapped_result_subject|new_result_subject>,"payloadKind":"exact_source_document"|"annotation_value"}`。slotKind/payloadKind/target union必须兼容。每个resultSlot全plan唯一；每个delete resultContext与fromSource一一对应。数组按kind、resultSlot/resultContext、fromSource、from、action排序，preimage fromSource唯一。
- `FreshSlotTargetTemplate`是slot-aware closed union：`fresh_entity_target{subject}`只允许node_link/citation→fresh Node、resource_occurrence→fresh Resource、annotation_reply→fresh Annotation；annotation_target只允许以下五个template并保留D2 target variant与全部非身份定位事实：`fresh_annotation_document_target{ownerSubject}`、`fresh_annotation_element_target{ownerSubject,elementKind,sourceSpan}`、`fresh_annotation_range_target{ownerSubject,sourceSpan}`、`fresh_annotation_resource_target{resourceSubject}`、`fresh_annotation_resource_region_target{resourceSubject,regionToken}`。ownerSubject/resourceSubject必须是mapped/new result subject且kind正确；elementKind/sourceSpan服从§6，regionToken为non-empty opaque string。Core在result payload/bytes已绑定且fresh ref与revision已分配后，只能由template机械签发完整D2 target locator/ref；不得从path、label、ambient owner、closest span或另一payload推断。template的outer kind、所有字段与subject进入fingerprint；仅写裸subject固定stage1 `invalid_request_matrix`。
- `preserve_exact`只在准确target在preimage/postimage均live且target bytes保持相等时合法；slot lifecycle按source状态另行确定，不能把非live source称为resolved graph reference；`preserve_suspended`只在preimage target已经trashed（source live时slot为suspended；source trashed时为non_live_source），且committed postimage保持byte-equal target、target仍trashed时合法；`suspend_with_result`只在本operation自身令准确target由live转trashed、source payload/revision同时改变且postimage slot保持byte-equal target时合法。source preimage与postimage都live时，前三者分别表达带result evidence的`resolved→resolved`、`suspended→suspended`、`resolved→suspended`；若带typed preimage的source原为trashed且本次postimage live，prestate为non_live_source，poststate按准确postimage target的live/trashed确定，action仍须满足自己的target与bytes条件。postimage source仍非live则不列reference lifecycle transition，reference result证据仍按payload规则保留；source bytes/revision不变但lifecycle上下文改变的四种组合只能用上一条lifecycle-only variant。delete移除slot并用resultContext证明postimage context；rewrite_to_existing绑定existingTarget；map_target保留preimage slot outer variant/coordinate facts并把其target identity按selected mapping机械换成fresh target；result_only_fresh使用targetTemplate。每个M/N/E symbolic segment必须恰对应一个非delete、非lifecycle-only reference plan resultSlot，每个这类resultSlot也必须恰对应一个segment；唯一例外是下文identity-preserving Annotation reply的S segment，它恰对应structuralPlan而不得进入reference plan。Plan内同一target出现两次仍是两个不同slotOrdinal，不能交换receipt对应关系。每个受本operation影响且postimage source live的slot都必须在receipt以pre/post state列入`referenceLifecycleTransitions`，包括result-only的`absent→resolved|suspended`、上述preimage action、source restore与target trash/restore；不得用数组缺席表达`target restore→resolved`。prestate只按**evidence origin**决定：`result_only_existing_reference|result_only_fresh_reference`无typed preimage，固定`absent`；每个非delete `preimage_reference_disposition`固定先读取fromSource container的authoritative preimage lifecycle：source trashed取`non_live_source`，source live才取准确target的`resolved|suspended`，即使mapped/new result container取得fresh identity也不得改成`absent`；`lifecycle_only_reference_transition`固定使用entry显式expectedPrestate。另有唯一的structural S reply origin：postimage reply非null且source live时必须列一次lifecycle transition，preimage reply为null取absent；否则source preimage trashed取non_live_source，source live才按准确preimage reply slot读取resolved/suspended；poststate由实际同owner live reply parent验证为resolved。postimage reply为null则无该slot transition。S transition不是reference result，不进入rewrittenReferences。result identity freshness不是prestate来源。
- `structuralPlan`元素closed为`{"kind":"node_placement","subjectRef":<NodeRef>,"destinationParentRef":<NodeRef>,"destinationOrdinal":<D3Integer>}`或`{"kind":"annotation_reply","subjectRef":<AnnotationRef>,"expectedAnnotationRevisionToken":<non-empty opaque token>,"resultSlot":<ReferenceResultSlotKey>,"replyTo":<AnnotationRef|null|fresh_annotation_reply>}`；这里仅replyTo允许null并表示移除reply parent。annotation_reply.resultSlot必须精确为container=`existing_entity_subject(subjectRef)`、payloadKind=`annotation_value`、logical slotOrdinal=1、slotKind=`annotation_reply`。该logical ordinal来自Value/3 schema rank而不是canonical byte-span出现顺序。它只表达primary operation之外、同commit的identity-preserving compound side effect。move_node/reorder_node的primary placement只由intent表达，structuralPlan必须为空；同一事实不得重复。identity-preserving Annotation reply edit只在structuralPlan/structuralChanges表达，并明确从referenceDispositionPlan/rewrittenReferences的“全部slot改写”中排除；result payload用唯一显式slot-keyed S segment绑定该structuralPlan entry，不形成第二语义表示。fresh copied/imported Annotation的初始reply仍由reference plan与postimage payload表达。Annotation token必须等于preimage current token，否则stale_locator；按下文StructuralPlanKey排序，subjectRef/resultSlot唯一。
- `resultPlacementPlan`元素closed为`{"subject":<mapped_result_subject|new_result_subject(entityKind=node)>,"parent":<existing_entity_subject(NodeRef)|mapped_result_subject(NodeRef)|new_result_subject(entityKind=node)|null>,"ordinal":<D3Integer>}`。`parent=null`只允许Workspace root且ordinal必须0；该0只是wire sentinel，root没有semantic siblingOrdinal；其他live Node必须有parent。该数组严格按`subjectCanonicalKey`排序且subject必须唯一；duplicate subject固定stage 1 `invalid_request_matrix -> invalid_identity_ref`。除fork中的trashed Node外，每个fresh Node恰一entry；fork中它恰覆盖mapped live Node且绝不含mapped trashed Node。它绑定symbolic subject、最终live parent subject与relative ordinal；不允许从`allocated`排序、payload顺序或UI枚举顺序推导。create/copy/import root destination的相等规则是`entry.parent.kind == existing_entity_subject && entry.parent.ref == intent.destinationParentRef && entry.ordinal == intent.destinationOrdinal`，不是异构JSON value byte-equal；fork/create Workspace root必须parent=null/ordinal=0。所有其他live fresh Node的parent必须能由同plan subject或existing target唯一物化。authoritative coverage遗漏在stage 14拒绝；parent cycle与live orphan分别在stage 10以`structural_cycle`、`orphan_creation`拒绝；inbound-ref、omission、reply、owner closure在stage 15拒绝。禁止自动reparent。
- `resultLifecyclePlan`元素closed为`{"subject":<mapped_result_subject>,"state":"live"|"trashed"}`，只在`fork_workspace`非空；它必须恰覆盖每个selected source Node/Resource/Annotation，state byte-for-byte保留source cut的authoritative lifecycle，按subjectCanonicalKey排序且subject唯一。其他mode必须静态为空。它把“保留两态”纳入fingerprint，不允许从payload、placement或数组位置推导。
- `resultTrashPlacementPlan`元素closed为`{"subject":<mapped_result_subject(NodeRef)>,"trashParent":<mapped_result_subject(NodeRef)|null>,"trashOrdinal":<D3Integer>,"restoreLocation":<ForkRestoreLocation>}`，只在`fork_workspace`非空，并恰覆盖`resultLifecyclePlan.state=trashed`的Node subjects；按subjectCanonicalKey排序且subject唯一。`trashParent`是source规范trash graph中的最近仍trashed直接parent映射；若无仍trashed parent则为null，表示唯一canonical Workspace Trash forest root而非Node或live-tree root。每个相同trashParent下的trashOrdinal必须从0连续、无洞、唯一，并保留source trash graph次序；subject不得出现在resultPlacementPlan。`ForkRestoreLocation`恰为`{"kind":"mapped_original_location","parent":<mapped_result_subject(NodeRef)>,"ordinal":<D3Integer>}`或`{"kind":"unavailable_original_location"}`：source记录的original parent仍为live/trashed且在exact fork map中时必须使用前者，否则（包括original parent已purged/tombstoned/not_found）必须使用后者。后者恢复时永远不能解释为root或自动reparent，未来restore必须显式提供live destination。trashed Resource/Annotation不进入本数组，其owner由identityMap、两态由resultLifecyclePlan证明。
- `PayloadSubjectKey`是closed union：`{"kind":"existing_entity_subject","ref":<typed ref>}`、`{"kind":"mapped_result_subject","sourceRef":<typed ref>}`、`{"kind":"new_result_subject","entityKind":"node","ordinal":<D3Integer>}`、`{"kind":"new_result_subject","entityKind":"resource"|"annotation","ordinal":<D3Integer>,"ownerSubject":<existing_entity_subject(NodeRef)|mapped_result_subject(NodeRef)|new_result_subject(entityKind=node)>}`、`{"kind":"artifact_part_subject","artifactSha256":<64 lowercase hex>,"partOrdinal":<D3Integer>}`。node禁止ownerSubject；resource/annotation必需ownerSubject。mapped_result由identityMap从sourceRef唯一导出；new_result ordinal在每个`(mode,entityKind,ownerSubjectCanonicalKey?)`域从0开始连续、无洞。create_resource/create_annotation仅role=primary的对应kind subject，其ownerSubject必须为等于intent.destinationOwnerRef的existing Node subject；member Resource/Annotation可分别绑定该create模式允许的existing或new Node subject，不强制全graph共享primary owner。create模式identityMap为空，故禁止mapped owner。ordinary_format import_new的每个fresh Resource/Annotation owner明确只允许本次freshSubjects中完整声明的new_result_subject(Node)，禁止existing或mapped owner；新增existing result payload pair不扩大该owner集合。receipt resultAllocations中的ResourceRef/AnnotationRef.owner必须等于ownerSubject物化后的NodeRef；owner swap即使payload digest相同也改变fingerprint并拒绝错误receipt。
- `RefKey`是唯一typed-ref tuple：Workspace=`(0,workspaceId)`；Node=`(1,workspaceId,nodeId)`；Resource=`(2,owner.workspaceId,owner.nodeId,resourceId)`；Annotation=`(3,owner.workspaceId,owner.nodeId,annotationId)`。UUID按canonical ASCII bytes序。
- `subjectCanonicalKey`是唯一递归tuple：existing=`(0,RefKey)`；mapped=`(1,RefKey(sourceRef))`；new Node=`(2,0,ordinal)`；new Resource=`(2,1,ordinal,ownerSubjectCanonicalKey)`；new Annotation=`(2,2,ordinal,ownerSubjectCanonicalKey)`；artifact=`(3,artifactSha256 ASCII bytes,partOrdinal)`。ownerSubject图必须无环且终止于Node subject；tuple先比较长度固定的rank/field sequence，再逐项比较；不存在string与integer同位、按完整JSON、payload、parent或输入位置排序的第二规则。

下列`D3-Canonical-Comparator/1` registry闭合所有request/receipt集合排序；数字rank是规范常量，不是实现enum值：

- scalar ranks：entityKind `node=0,resource=1,annotation=2`；payloadKind `exact_source_document=0,resource_bytes=1,annotation_value=2,import_artifact=3`；role `preimage=0,result=1,source_artifact=2`；lifecycle `live=0,trashed=1`；include disposition `include=0,omit=1`；omit reason `unreconstructable_target=0,explicitly_omitted=1,reply_closure=2,already_live=3,tombstoned=4`。
- `SpanKey=(startLine,startColumn,endLine,endColumn)`；`ArtifactObjectKey=(artifactSha256,partOrdinal,objectOrdinal)`；`LocatorKey`的variant rank为element=0, range=1, resource_region=2, anchor=3，再比较owner/resource RefKey、revision token UTF-8 bytes、elementKind UTF-8 bytes、SpanKey、region/anchor token UTF-8 bytes中该variant存在的固定字段。
- `SlotAddressKey`：document=0后接LocatorKey与slotKindRank(`node_link=0,citation=1,resource_occurrence=2`)；annotation=1后接Annotation RefKey、revision token、slotRank(`target=0,reply_to=1`)。`SlotTargetKey`的variant rank固定NodeRef=0, ResourceRef=1, AnnotationRef=2, annotation-document=3, annotation-element=4, annotation-range=5, annotation-resource=6, annotation-resource-region=7；再比较该variant全部RefKey/LocatorKey/target fields。AuthorAnchorAddress不属于任何D2 authored slot target，故无target rank。
- `ResultSlotKey=(subjectCanonicalKey(container),payloadKindRank,slotOrdinal,slotKindRank)`，全局slotKindRank为`node_link=0,citation=1,resource_occurrence=2,annotation_target=3,annotation_reply=4`；`ResultContextKey=(subjectCanonicalKey(container),payloadKindRank)`；`EvidenceBranchKey`固定slot=0、context=1，二者从不以缺失字段比较。
- `FreshTargetTemplateKey`的variant rank固定fresh_entity=0、annotation_document=1、annotation_element=2、annotation_range=3、annotation_resource=4、annotation_resource_region=5，再比较subjectCanonicalKey、elementKind、SpanKey、regionToken中该variant全部字段。
- `ReferencePlanKey`：preimage=0、result_only_existing=1、result_only_fresh=2、lifecycle_only=3。preimage再比较`EvidenceBranchKey, ResultSlotKey|ResultContextKey, SlotAddressKey(fromSource), SlotTargetKey(from), actionRank`，action rank为map=0,preserve_exact=1,preserve_suspended=2,rewrite_existing=3,delete=4,suspend_with_result=5，并在适用时追加existing SlotTargetKey；existing-result比较`ResultSlotKey,SlotTargetKey,expectedLifecycleRank`；fresh-result比较`ResultSlotKey,FreshTargetTemplateKey`；lifecycle-only比较`SlotAddressKey(source),SlotTargetKey(target),prestateRank(non_live_source=0,resolved=1,suspended=2),poststateRank(resolved=0,suspended=1)`。
- `StructuralPlanKey`：node_placement=0后接RefKey(subject),RefKey(parent),ordinal；annotation_reply=1后接RefKey(subject),ResultSlotKey,revision token, replyTargetRank(`null=0,concrete=1,fresh=2`)，concrete比较reply RefKey、fresh比较subjectCanonicalKey。
- `FreshSubjectKey`：create=0后接roleRank(`primary=0,member=1`)与subjectCanonicalKey；artifact=1后接ArtifactObjectKey与subjectCanonicalKey。`SelectedObjectKey=(RefKey,dispositionRank,reasonRank?)`；placement/lifecycle/trash-plan/payload keys分别为`subjectCanonicalKey`、`subjectCanonicalKey`、`subjectCanonicalKey`、`(roleRank,subjectCanonicalKey,payloadKindRank,sha256 ASCII)`。
- receipt `ReferenceResultKey`：preimage=0后严格依次接resultBranchRank(`rewritten=0,deleted=1`)、ResultSlotKey|ResultContextKey、SlotAddressKey(fromSource)、SlotTargetKey(from)，最后才接rewritten适用的SlotAddressKey(toSource),SlotTargetKey(to)或deleted适用的ResultContextValueKey；created=1后接ResultSlotKey,SlotAddressKey(toSource),SlotTargetKey(to)。不得把postimage to/context嵌入branch并排到fromSource/from之前。placement tuple也closed：`LivePlacementKey=(nullRank(parentRef),parentRef?,ordinal)`，nullRank为null=0/value=1；`RestoreLocationKey`为original=0后接`RefKey(parentRef),ordinal`，unavailable=1且无追加字段；`TrashPlacementKey=(nullRank(trashParentRef),trashParentRef?,trashOrdinal,RestoreLocationKey)`。`StructuralChangeKey` rank固定node_placement=0,node_trash_placement=1,node_restore_placement=2,node_trash_reparent=3,annotation_reply=4，再比较subject RefKey；node_placement接`LivePlacementKey(from),LivePlacementKey(to)`，node_trash_placement接`LivePlacementKey(from),TrashPlacementKey(to)`，node_restore_placement接`TrashPlacementKey(from),LivePlacementKey(to)`，node_trash_reparent接`TrashPlacementKey(from),TrashPlacementKey(to)`，annotation_reply接from/to revision token与`nullRank+RefKey` reply tuple。identityMap=`(RefKey(from),RefKey(to))`；resultAllocation/lifecycle/livePlacement/trashPlacement=`subjectCanonicalKey`；referenceLifecycle=`(SlotAddressKey(source),SlotTargetKey(target),prestateRank(absent=0,non_live_source=1,resolved=2,suspended=3),poststateRank(resolved=0,suspended=1))`；omitted=`(RefKey,reasonRank)`；plain ref arrays=`RefKey`。

registry中未列为数组的object没有集合排序，只服从D3-CJ member-name顺序。所有tuple分支先比较明确rank，再比较该分支固定字段；不得用absent/null sentinel、完整JSON bytes、宿主object comparator或map/provider order补足。每个request十数组和receipt十二数组只使用上述唯一key；相同key而entry bytes不同固定duplicate/conflict并在stage1拒绝，而不是用entry bytes作tie-break。跨Rust/TypeScript/.NET的permutation corpus必须由本registry生成。
- `payloadBindings`元素closed为`{"role":"preimage"|"result"|"source_artifact","subject":<PayloadSubjectKey>,"payloadKind":"exact_source_document"|"resource_bytes"|"annotation_value"|"import_artifact","sha256":<64 lowercase hex>}`。role/subject matrix精确为：existing subject允许preimage或result；mapped/new subject只允许result；artifact subject只允许source_artifact。payloadKind必须exact匹配node→exact_source_document、resource→resource_bytes、annotation→annotation_value、artifact→import_artifact；其他组合stage 1拒绝。preimage/source_artifact hash输入分别是D2 exact-source UTF-8 bytes、Resource exact bytes、下述D3-Annotation-Value/3 projection bytes或完整artifact/part exact bytes。result既不含D3 ref/structural slot、也不使用C/Q时hash exact result bytes；含slot或C/Q时hash下述`D3-Symbolic-Result/9` canonical bytes，且所有成功decoded reference slot必须由M/N/E/S之一表示并与其唯一reference/structural plan claim一一对应。Core在stage14–15按§4.1.3a候选map机械物化并完整验证；reservation后只能发布同一已验证payload并校验其完整性；`allocated`数组位置没有映射语义。

`D3-Annotation-Value/3`是D3 operation mutation-binding projection，不是第二个Annotation作者权威或新领域对象。它从D2 Annotation v2机械投影且字段恰为`kind,purpose,target,body,suggestion,replyTo`：`kind=d3_annotation_value`；purpose/body/suggestion逐字节等于D2字段；target必须且只能是§5.3 `D3-Annotation-Target-Projection/1`五成员closed union，绝不得直接放D2 outer token wire或省略document.owner；replyTo是null或D2 reply ref。它明确排除`wireVersion,annotationRef,ownerNodeRef,targetStatus`以及D6 revision token：identity/owner由PayloadSubjectKey与result allocation绑定，targetStatus严格按§5.3 exactness/lifecycle映射派生，revision由commit产生。materialized postimage从Value/3构造D2 Annotation outer wire时必须机械注入顶层`wireVersion=2`与`kind=annotation`，再按§5.3把Projection/1唯一变为D2 outer target，并与receipt分配的annotationRef/ownerNodeRef、唯一targetStatus组合成D2 Annotation v2 wire；这些outer字段不得从Value/3、ambient owner或客户端输入读取。不能双向唯一组合即stage14 `identity_map_incomplete`。

Value/3先按D3-CJ/3得到完整canonical base bytes，再定义两个**logical mutation slot**，logical ordinal不等于byte出现次序：

- `annotation_target`固定`slotOrdinal=0`，span是`target` member value的全部bytes；它始终是reference slot。
- `annotation_reply`固定`slotOrdinal=1`，span是`replyTo` member value的全部bytes。值非null时它同时是resolver-visible AnnotationRef slot；值为null时它不是reference slot，但仍是identity-preserving reply mutation可占用的structural span，exact bytes恰为ASCII `null`。

完整Value/3实际输入只接受concrete reply；Result/9的S骨架可携带§4.1.3a fresh_annotation_reply占位结构，必须在Value/3实际解码前物化为concrete Ref。两个span都从member value首byte开始到value末byte end-exclusive，不含member name、colon或comma。segment永远按真实span起点递增输出；因此D3-CJ/3 canonical object中`replyTo`先于`target`时，ordinal 1 segment合法地先于ordinal 0 segment。logical ordinal只能从每个non-B segment显式携带的完整`ReferenceResultSlotKey`读取，绝不得从segment位置推断。body/suggestion内不解析引用。fresh target用FreshSlotTargetTemplate物化完整Projection/1 value；existing target用SlotCompatibleTarget中的同一Projection/1；identity-preserving P→Q、P→null、null→Q都用唯一structural S。preimage/result annotation payload只hash完整Value/3 base或symbolic bytes，永不hashD2 outer token wire或包含尚未分配AnnotationRef的完整D2 wire。

`D3-Symbolic-Result/9`是closed byte grammar；partition事实全部内嵌于stream，validity不得读取fixture metadata：

1. magic恰为ASCII `D3-Symbolic-Result/9`后接一个NUL byte，再接ASCII `C`、positive segment count最短十进制与LF，再接ASCII `L`、positive baseLength最短十进制与LF；count=0或baseLength=0固定拒绝。
2. 每个segment共同前缀恰为`<tag><start>:<end>:<bodyLength>:`；start/end/bodyLength都是D3Integer最短十进制，必须`0<=start<end<=baseLength`。首段start=0、相邻段前end=后start、末段end=baseLength；重叠、洞、回退、越界或count不符仅从bytes即可拒绝。
3. `B` body是恰好bodyLength个raw byte，且`bodyLength=end-start`；保留所有非slot raw byte。空B禁止、相邻B必须合并。
4. `M` body恰为D3-CJ/3 `{"resultSlot":<ReferenceResultSlotKey>,"targetSubject":<mapped_result_subject>}`；`N`同形但targetSubject为new_result_subject。M/N分别只对应map_target或result_only_fresh；PayloadSubjectKey递归closed validation必须检查ownerSubject kind与每层ordinal的exact integer type/bounds。
5. `E` body先是bodyLength个D3-CJ/3 ReferenceResultSlotKey byte，再接`<exactLength>:<exactSlotBytes>`；exactLength最短十进制且必须等于`end-start`。它只对应result_only_existing、preserve_exact、preserve_suspended、rewrite_to_existing或suspend_with_result；不得含fresh reservation。
6. `S` body恰为D3-CJ/3 `{"resultSlot":<ReferenceResultSlotKey>,"structuralEntry":<annotation_reply structuralPlan entry>}`。structuralEntry.resultSlot必须byte-equal outer resultSlot；resultSlot.container必须是`existing_entity_subject(structuralEntry.subjectRef)`且两者为同一AnnotationRef；`end-start`必须等于D3-CJ/3(entry.replyTo) byte length。receipt只用annotation_reply_change证明，不得另有reference result。
7. 全部segment按base bytes中的真实span start升序输出并由stream自身证明从0无洞覆盖到baseLength；slot bytes落入B、non-B缺/错resultSlot固定stage14 `identity_map_incomplete`。相邻reference/structural span直接产生相邻non-B segment，首尾不补空B。没有mutation/reference span也没有typed carrier C或D7 Q的payload恰一B；Q使用§21；C使用§4.1.3a完整carrier span。
8. 每个非lifecycle-only reference-plan resultSlot恰一M/N/E，每个M/N/E恰一plan entry；每个identity-preserving reply structuralPlan resultSlot恰一S且不得进入reference plan。lifecycle-only transition没有resultSlot、payload或segment。
9. count、baseLength、start、end与length属于§4.1.4 D3Integer；bool/string/MAX+1在分配、截断或宿主转换前固定stage14失败。magic、projection、segment framing或subject schema改变必须升级operation wireVersion。

历史v9 exact向量及当时运行保留在D3 Wire Conformance Corpus v21.json（外部控制记录未随本输入发布）。该旧证据不包含v10九数组、Result/8 C/S或新组合语义；新版本接受依据是本次完整replacement review及具名有界模型，产品全量conformance仍须真实执行。

该grammar、referenceDispositionPlan、resultPlacementPlan、resultLifecyclePlan、resultTrashPlacementPlan与receipt subject mapping共同决定唯一postimage，不形成pre-reservation ID循环。
- payload排序固定为`(roleRank, subjectCanonicalKey, payloadKind, sha256)`，roleRank为preimage < result < source_artifact。唯一性按`(role,subject,payloadKind)`，同一key不得出现两个digest；相同digest绑定不同subject合法且必须分别出现。交换两个subject的digest、切换preimage/result role或改变任一payload bytes都会改变canonical request与fingerprint。

create graph中无preimage的N+E最小closed plan fragment如下；N0是create_node primary，N1是member且有自己的payload、placement与未来resultAllocation；slot0绑定fresh N1，slot1绑定既存B，二者不能因target相同或数组置换失去slot identity：

~~~json
{
  "freshSubjects": [
    {"kind":"create_fresh_subject","role":"primary","subject":{"kind":"new_result_subject","entityKind":"node","ordinal":0}},
    {"kind":"create_fresh_subject","role":"member","subject":{"kind":"new_result_subject","entityKind":"node","ordinal":1}}
  ],
  "referenceDispositionPlan": [
    {
      "kind": "result_only_fresh_reference",
      "resultSlot": {"kind":"symbolic_result_slot","container":{"kind":"new_result_subject","entityKind":"node","ordinal":0},"payloadKind":"exact_source_document","slotOrdinal":0,"slotKind":"node_link"},
      "targetTemplate": {"kind":"fresh_entity_target","subject":{"kind":"new_result_subject","entityKind":"node","ordinal":1}}
    },
    {
      "kind": "result_only_existing_reference",
      "resultSlot": {"kind":"symbolic_result_slot","container":{"kind":"new_result_subject","entityKind":"node","ordinal":0},"payloadKind":"exact_source_document","slotOrdinal":1,"slotKind":"node_link"},
      "existingTarget": {"kind":"node_ref","workspaceId":"9dfadab9-5f7a-4ba9-a6d4-681b29645283","nodeId":"8e4c0c8e-eed4-4dcc-a154-05fed18d4e18"},
      "expectedLifecycle": "live"
    }
  ],
  "resultPlacementPlan": [
    {"subject":{"kind":"new_result_subject","entityKind":"node","ordinal":0},"parent":{"kind":"existing_entity_subject","ref":{"kind":"node_ref","workspaceId":"9dfadab9-5f7a-4ba9-a6d4-681b29645283","nodeId":"c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"}},"ordinal":0},
    {"subject":{"kind":"new_result_subject","entityKind":"node","ordinal":1},"parent":{"kind":"new_result_subject","entityKind":"node","ordinal":0},"ordinal":0}
  ],
  "payloadBindings": [
    {"role":"result","subject":{"kind":"new_result_subject","entityKind":"node","ordinal":0},"payloadKind":"exact_source_document","sha256":"1111111111111111111111111111111111111111111111111111111111111111"},
    {"role":"result","subject":{"kind":"new_result_subject","entityKind":"node","ordinal":1},"payloadKind":"exact_source_document","sha256":"2222222222222222222222222222222222222222222222222222222222222222"}
  ]
}
~~~

这只是plan fragment；完整intent还含其余全部适用空数组（包括existingPayloadEdits）。若N1少fresh declaration、result payload、placement或committed receipt的resultAllocation，stage1/14唯一拒绝，绝不存在“只reserve N0后再猜N1”的实现分支。create_annotation同理可以把fresh Node/Resource member声明在同一graph，并用完整FreshSlotTargetTemplate构造mandatory target；single primary并不限制graph只有一个subject。

request per-mode exact矩阵如下。`static`只指可由request bytes本身判断的shape/empty/cardinality与cross-field equality，失败在stage 1；`authoritative complete`必须读取当前cut，固定在stage 14/15，失败为`identity_map_incomplete`或`inbound_reference_conflict`，绝不得提前到授权前：

| mode/variant | workspaceProposal | freshSubjects | selectedObjects | referenceDispositionPlan | structuralPlan | resultPlacementPlan | resultLifecyclePlan | resultTrashPlacementPlan | payloadBindings | existingPayloadEdits |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| create_workspace | 必需且create variant | 1 primary new Node + 0..N closed members；全subject coverage | static空 | result payload全部slot恰一result-only entry | static空 | 每个Node恰一；primary root→null/0 | 空 | 空 | 每个fresh subject恰一kind-matched result | static空 |
| create_node | 禁止 | 1 primary new Node + 0..N closed members；全subject coverage | static空 | fresh result slot恰一result-only；existing slot按前像分区闭合 | 仅显式existing Annotation reply | 每个Node恰一；primary→intent destination | 空 | 空 | 每个fresh subject恰一kind-matched result  另每个existing声明恰一preimage/result pair | 闭合existing edits；§4.1.3a |
| create_resource | 禁止 | 1 primary new Resource(owner=intent owner) + 0..N members；全subject coverage | static空 | fresh result slot恰一result-only；existing slot按前像分区闭合 | 仅显式existing Annotation reply | graph中每个Node恰一 | 空 | 空 | 每个fresh subject恰一kind-matched result  另每个existing声明恰一preimage/result pair | 闭合existing edits；§4.1.3a |
| create_annotation | 禁止 | 1 primary new Annotation(owner=intent owner) + 0..N members；全subject coverage | static空 | fresh mandatory target/reply为result-only；existing slot按前像分区闭合 | 仅显式existing Annotation reply | graph中每个Node恰一 | 空 | 空 | 每个fresh subject恰一kind-matched result；Annotation使用Value/3  另每个existing声明恰一preimage/result pair | 闭合existing edits；§4.1.3a |
| copy_node_subtree | 禁止 | static空 | root与全部live descendant Node authoritative complete且全include；owner-local对象可按omission algebra omit | include payload全部slot authoritative complete | 空 | 每个include Node的mapped subject恰一；root→intent destination，descendant→mapped parent | 空 | 空 | 每个selected source payload恰一preimage；每个include payload恰一mapped result；omit无result | static空 |
| copy_resource/copy_annotation | 禁止 | static空 | static恰一include且ref等于intent.sourceRef | copied subject authoritative complete；另可覆盖零到多个显式target-Workspace existing Node/Annotation container slot | 只允许显式existing Annotation container的identity-preserving reply side effect | 空 | 空 | 空 | source subject恰一preimage、mapped result恰一result；每个被修改existing container另恰一preimage/result pair；copy_annotation result含mandatory target | static空 |
| fork_workspace | 必需且fork variant | static空 | source cut全部live/trashed entity authoritative complete且全include | source payload全部slot authoritative complete | 空 | 只覆盖全部live Node；root→null/0，其他→mapped live parent | 每个source live/trashed entity恰一并保留两态 | 每个trashed Node恰一Trash placement与restoreLocation | 每个payload-bearing source恰一preimage和mapped result | static空 |
| continue_workspace | 禁止 | static空 | static空 | static空 | static空 | 空 | 空 | 空 | 空；continuation cut由intent token与§8.5 ledger验证，不复制payload | static空 |
| move_node/reorder_node | 禁止 | static空 | static恰一include且ref等于intent.subjectRef | static空 | static空；primary结构只由intent表达 | 空 | 空 | 空 | 空 | static空 |
| trash | 禁止 | static空 | subject及实际trash closure authoritative complete | affected slot authoritative complete；identity-preserving reply由S排除 | 只允许保持identity的compound Node placement或Annotation reply | 空 | 空 | 空 | 实际改写payload container的preimage/result逐主体成对绑定 | static空 |
| restore | 禁止 | static空 | request成员全部include；actual restorable closure authoritative complete | affected slot authoritative complete；identity-preserving reply由S排除 | 只允许intent location之外compound side effect | 空 | 空 | 空 | 实际改写payload container的preimage/result逐主体成对绑定 | static空 |
| purge | 禁止 | static空 | subject及实际purge closure authoritative complete | 全部live inbound slot authoritative complete；reply由S排除 | 只允许保持no-orphan的compound side effect | 空 | 空 | 空 | 每个被改写payload container的preimage/result成对；purged对象无result | static空 |
| import_new + ordinary_format | 禁止 | fresh artifact分区每member恰一ArtifactObjectKey→new subject bijection | static空 | fresh artifact分区无typed preimage，slot为result-only；existing分区使用准确preimage | 仅显式existing Annotation reply | 每个new Node subject恰一closed placement | 空 | 空 | artifact每part恰一source_artifact；每个fresh subject恰一new_result  另每个existing声明恰一preimage/result pair | 闭合existing edits；§4.1.3a |
| import_new + partial_identity_bearing_transfer_bundle | 禁止 | static空 | bundle Node全include；Resource/Annotation可按omission algebra omit | include payload全部slot authoritative complete | 空 | 每个include Node mapped subject恰一closed placement | 空 | 空 | artifact part完整；每个selected source preimage及include mapped result完整 | static空 |

restore intent 的 `location` closed matrix 恰为：

| subjectRef.kind / owner prestate | location required shape | structuralPlan / receipt placement |
| --- | --- | --- |
| Node | `{"kind":"original_location"}`或`{"kind":"explicit_location","destinationParentRef":<NodeRef>,"destinationOrdinal":<D3Integer>}`恰一 | subject自己的placement只由location表达，request structuralPlan禁止重复；committed receipt以一条`node_restore_placement`证明Trash→live |
| Resource/Annotation，owner live | `{"kind":"owner_local_no_placement"}`恰一 | 不得含owner Node placement；对象回到同一immutable owner，无parent/ordinal |
| Resource/Annotation，owner trashed且同operation恢复 | `{"kind":"owner_restore_location","ownerRef":<exact owner NodeRef>,"ownerLocation":{"kind":"original_location"}|{"kind":"explicit_location","destinationParentRef":<NodeRef>,"destinationOrdinal":<D3Integer>}}`恰一 | selectedObjects必须include owner及所需closure；owner placement只由ownerLocation表达，request structuralPlan禁止重复；receipt必须有owner的`node_restore_placement` |
| Resource/Annotation，owner tombstoned/not_found/foreign | 无合法shape | stage 9 `entity_not_restorable`或更早owner/Workspace family；不得解释或移动ambient owner |

`owner_restore_location.ownerRef`必须byte-equal subjectRef.owner；`ownerLocation`的nested original/explicit规则与Node相同。任何Node使用owner-local variant、owner-local对象使用Node variant、ownerRef不等encoded owner、或location与structuralPlan重复同一owner placement，固定stage 1 `invalid_request_matrix -> invalid_identity_ref`。`owner_local_no_placement`与`owner_restore_location`对owner-local subject都可static decode；其与authoritative owner live/trashed prestate不相容时只在stage 9写唯一recorded rejection。不得忽略location或把它作用于别的Node。

`artifactClass`字符串闭集恰为`ordinary_format | partial_identity_bearing_transfer_bundle`；full-workspace artifact不能import_new，只能fork。intent是primary operation唯一权威；freshSubjects声明create/import fresh identity graph，resultPlacementPlan只是fresh live Node结果的closed structure commitment，resultLifecyclePlan/resultTrashPlacementPlan只服务exact fork。与intent root destination的关系只用上文typed parent/ref/ordinal equality，不比较异构JSON bytes。plan不得重复primary authority/source/cut；不等固定`invalid_request_matrix`。`referenceDispositionPlan`在continue/move/reorder中必须空；create_workspace只允许result-only；create_node/create_resource/create_annotation与ordinary import的fresh container只允许result-only，existingPayloadEdits声明的existing container允许preimage map_target/preserve_exact/preserve_suspended/rewrite_to_existing/delete与真正新增slot的result-only；copy/fork/partial import允许preimage与result-only variant，preimage action上限是map_target/preserve_exact/preserve_suspended/rewrite_to_existing/delete；trash允许`suspend_with_result`与`resolved→suspended` lifecycle-only；restore允许`non_live_source→resolved|suspended`与`suspended→resolved` lifecycle-only；purge禁止凭空suspend/resume；identity-preserving Annotation reply edit只能用structuralPlan并由S绑定result bytes；known不适用action固定invalid_request_matrix。

`import_new + ordinary_format`明确允许单root或forest。artifact decoder先输出被source_artifact digest绑定的closed manifest；每个artifact occurrence都有唯一ArtifactObjectKey；只有fresh分区产生new subject，existing/companion按§4.1.3a。Core按ArtifactObjectKey排序并验证fresh分区的freshSubjects bijection及§4.1.3a existing分区和companion派生，再产生top-level new Node roots `r0..r(n-1)`，`n>=1`；root顺序来自其ArtifactObjectKey，不来自subject ordinal、payload digest、transport/provider/UI枚举。每个root的resultPlacementPlan.parent必须是同一个`existing_entity_subject(intent.destinationParentRef)`，ordinal是数学整数`intent.destinationOrdinal + i`，形成无重复、连续的单一插入block。每次加法先在无限精度数学整数上求值；结果大于MAX固定stage 10 `invalid_ordinal`，不得wrap、clamp或扩大wire域。非root new Node必须指向同plan中的唯一new Node parent。任何key重复/洞、key→subject非双射、subject ordinal与key排序不一致、按artifact transport顺序分配或第二destination parent都固定stage 1 `invalid_request_matrix`；当前parent容量/范围不足固定stage 10 `invalid_ordinal`。D9必须对每个受支持format定义确定性manifest occurrence enumeration，但不得改变本D3 key/bijection wire。

omission algebra唯一如下：

- `copy_node_subtree`与partial identity-bearing import中的Node集合必须对所选root的完整live descendant closure全include；Node omit一律static `invalid_request_matrix`。fork live/trashed全体均include。
- included Node的owner-local Resource/Annotation可omit；omitted Resource的全部included source occurrences必须有delete/rewrite action，不能留下ambient owner rebind；omitted Annotation的mandatory target随对象消失，不伪造slot result。
- omitted Annotation的每个included live reply child必须对其fresh-copy `reply_to` slot有显式delete或rewrite_to_existing；否则child也必须以`reply_closure`递归omit。不得自动reparent，也不得把identity-preserving structuralPlan用于fresh child。
- `explicitly_omitted`只来自request的owner-local Resource/Annotation omit；`reply_closure`只来自上述递归且必须能由request闭合重算；`unreconstructable_target`只允许调用方明确拒绝复制该owner-local对象。receipt omitted集合必须与request和authoritative closure精确相等。
- restore request没有omit；receipt只能因authoritative preflight列`already_live | tombstoned`。因此不存在合法restore receipt的`explicitly_omitted`。任何included child缺parent、未声明reparent或遗漏owner closure固定stage 15 `inbound_reference_conflict`，不是自动修复。

Workspace role绑定固定为：boundWorkspaceRef总是commit target；ordinary同Workspace operation的全部refs属于bound Workspace；copy sourceRef/fromSource/from与selected source可属于显式sourceWorkspaceRef，destination parent/owner、existingTarget、result subject parent与ambient expected owner必须属于bound target；fork foreign preimage只可属于proposal.sourceWorkspaceRef；continue source必须等于bound；partial import foreign preimage只可属于intent.sourceWorkspaceRef；ordinary import没有foreign typed ref。所有Workspace role违规只在stage 2映射`workspace_ref_misbound`，不得又称stage 1 invalid_request_matrix；stage 2只检查decoded request bytes的角色，不route/fetch或访问existence。`partial_identity_bearing_transfer_bundle`在stage 3、4、9、14及任何其他stage都禁止resolve、mount、fetch或查询sourceWorkspaceRef authority；stage 9 typed-key lookup只作用于target authoritative refs，artifact-local foreign refs从不进入source Workspace lookup。source cut只来自被`source_artifact` digest绑定的closed artifact manifest，foreign typed refs只作artifact-local rewrite/provenance。若实现需要在线source authority，该请求固定`domain_kind_mismatch`，必须改走online copy而不是用artifact事实冒充authority。

所有十数组允许调用方以任意JSON数组顺序提交；顺序不具语义。Core在任何cross-field validation、fingerprint或ledger access前，先验证元素closed shape/duplicate key，再按本节唯一排序规则canonicalize。structurally valid permutation一律接受并得到相同canonical request/fingerprint；“因非canonical输入顺序而拒绝”禁止。只依赖request bytes的duplicate、static matrix与cross-field equality固定`invalid_request_matrix -> invalid_identity_ref`于stage 1；Workspace role固定stage 2；真实closure/coverage少一或多一只能在stage 14/15返回`identity_map_incomplete`/`inbound_reference_conflict`。

canonical request bytes使用`D3-CJ/3`：UTF-8、无BOM/空白；任何object的member都按**未转义member name UTF-8 bytes升序**排列，因而schema prose与示例行序不参与canonical bytes。string输入必须是Unicode scalar sequence；`"`编码为`\"`、backslash编码为`\\`；U+0008/U+0009/U+000A/U+000C/U+000D分别用`\b/\t/\n/\f/\r`；其余U+0000–U+001F只用lowercase `\u00xx`；其他scalar直接输出其UTF-8 bytes，不转义`/`、U+2028/U+2029、非ASCII或non-BMP，也不做Unicode normalization。lone surrogate/非法UTF-8拒绝；`\u001A`等非lowercase或可用short escape却写成unicode escape的输入可被普通JSON层解析，但重编码后不等canonical evidence。整数属于§4.1.4并用最短十进制；禁止浮点；数组先按字段规则canonicalize；可选字段不适用时省略。`requestFingerprint`固定为ASCII `sha256:`加`SHA-256(D3-CJ/3(CanonicalFingerprintBody))`的64位lowercase hex。CanonicalFingerprintBody member恰为`wireVersion, kind, boundWorkspaceRef, mode, expectedAuthority, workspaceProposalDigest（适用时）, preparationBinding（适用时）, intent`；top-level operationId不进入摘要。

因此语义相同但selected/reference/structural/resultPlacement/resultLifecycle/resultTrashPlacement/payload数组输入顺序不同，在进入ledger前产生同一canonical bytes与fingerprint；三个result plan逐subject全排列也必须得到同一bytes。任一bound Workspace、mode、authority expectation/proposal、subject/destination、closure/disposition、structure、lifecycle、Trash placement、label或逐主体payload binding变化都产生不同canonical bytes。不同fingerprint只在同一Workspace-local ledger key内得到`operation_id_conflict`。

`D3-CJ/3`同时是所有D3 wireVersion 11 closed envelope与全部nested object的唯一authoritative encoder：Workspace allocation issue/replace request、proposal、allocation outcomes、operation request、identity_change_receipt、identity_operation_error、三个resolver outcome与cross_workspace_transfer_outcome都必须以该编码发布。request输入的数组permutation先canonicalize；authoritative receipt/error/outcome/proposal输出从一开始就是canonical bytes。ledger/control family保存并重放这些canonical bytes，所以byte-equivalent既适用于同一decision replay，也使相同D3事实跨独立实现获得相同首份bytes。所有JSON string必须成功decode为Unicode scalar sequence；lone surrogate或非法UTF-8固定`invalid_closed_shape`。非canonical JSON可由普通展示层解析，但不得被验签、hash或作为canonical evidence接受；验证者必须重编码后比较或拒绝其evidence claim。

`D3-CJ/3` field-order registry只有一条且历史基线覆盖所有v9 variant；当前wire11的新增variant按本稿独立验证：object member按上述UTF-8 key order；没有variant可override。最小string vectors固定为：decoded U+000B→bytes `"\u000b"`，U+001A→`"\u001a"`，quote→`"\""`，backslash→`"\\"`，U+1F642→直接UTF-8 `"🙂"`；decoded `é`与`e`+U+0301保留为不同bytes/hash。示例`EntityResolveOutcome(tombstoned)`的canonical key顺序为`kind, requestedRef, status, wireVersion`，nested NodeRef为`kind, nodeId, workspaceId`。issue/replace/proposal/allocation outcome/request/receipt/error/三个resolver/transfer及每个nested tagged union必须各有至少一份exact UTF-8 bytes与SHA-256 golden。

`continue_workspace`的新AuthorityInstanceId只由Core决定。外部request及intent不得携带、建议或预留该ID；出现`authorityInstanceId`固定stage 1 `invalid_request_matrix -> invalid_identity_ref`，并且零ledger read、零allocation read、零reservation。stage 1–4、TL1/TL2与stage 5 existing-key conflict都不采样ID。stage 5确认Workspace-local OperationId unseen、fingerprint与current/continuously proven authority generation有效后，Core在stage 12选择一个尚未被本operation持久化的canonical UUIDv4 **ephemeral candidate**并读allocation history；该值不是reservation、不是caller input，也不可对外作为成功identity。若candidate已被另一OperationId的`ContinueAuthorityReservation`占用，或已经是proposed、active、burned或destroyed，Core必须在同一target-ledger decision CAS中写入`rejected(identity_collision)`，不创建新reservation且不在同一OperationId内换样重试；exact retry只返回byte-equivalent saved rejection、`allocationReads:0`，新OperationId才可采样新candidate。stage 7–11的primary failure属于pre-sample recorded rejection；stage 13–15的primary failure属于post-sample recorded rejection。两者都经decision CAS保存，但后者已经发生且必须报告一次allocation-history read，不能用generic rejection抹掉该观察。

candidate在stage 12读取时free且stage 13–15全部通过后，唯一planning decision CAS才同时：(1) 重新核验expected authority generation、continuation cut与target ledger unseen；(2) 重新比较candidate仍未reserved/proposed/active/burned/destroyed；(3) 建立`ContinueAuthorityReservation(authorityInstanceId, operationId, requestFingerprint, continuationCutToken, expectedGenerationToken)`；(4) 把同一ID写入唯一`planned` decision。planning CAS观察generation/continuation-cut loss或candidate allocation-state loss时，当前**attempt**的唯一结果是internal `restart`：零ledger write、零reservation、不得发布error或保存decision；两种loss都已在stage 12采样，因此该attempt固定`ledgerReads:2,allocationReads:1`。restart重新从§4.1.1 authority/ledger gate进入；最小的generation-loss **completed operation**在额外一次authority/ledger observation后返回`identity_authority_unavailable`，固定`ledgerReads:3,ledgerWrites:0,allocationReads:1,newReservations:0`。allocation-loss restart则重新读取authoritative state并可采样新的ephemeral candidate；若随后在stage 12权威看到占用，才写`identity_collision`。不得把attempt计数冒充completed-operation计数，也不得把restart冒充recorded rejection。planned crash recovery与exact retry必须读取并复用同一reservation，不能再次分配；同OperationId不同fingerprint固定`operation_id_conflict`且不产生第二reservation。并发continue请求由Workspace-local planning CAS线性化，每个OperationId/fingerprint至多一个reservation winner。commit把reserved ID原子转为active authority、把TargetLedgerCustodyRecord移交给它，并在receipt的`authorityInstanceId`回显byte-equal值；若planned decision进入`terminal_failed`，同一决议必须把reservation永久转为burned，之后任何operation都不得再分配。preflight/recorded rejection没有reservation可burn。D6可自由选择物理表、锁、lease与事务机制，但不得改变sample/reject/restart/reserve/recover/activate/burn/no-reuse的D3可观察语义。

本节机器计数必须携带`observationScope=attempt|completed_minimal_restart|saved_replay|terminalization|concurrent_schedule`；不同scope的计数禁止相加、替换或比较为同一个operation outcome。`ledgerReads`只计对目标`(WorkspaceId,OperationId)` ledger key的逻辑观察，并**包括每次CAS expected-state compare**，即使存储实现把lookup与CAS合并为一次物理事务；custody、proposal family与allocation history读取分别计数。因而unseen/current plan-and-reserve是stage-5 lookup + planning CAS compare=`ledgerReads:2`，pre-sample recorded rejection是`2/1/0`，post-sample recorded rejection与stage-12 authoritative collision是`2/1/1`（依次为ledger reads/writes/allocation reads），stable planning是`2/1/1`且恰一reservation。generation/allocation CAS-loss attempt是`2/0/1`并只返回restart；generation-loss minimal completed restart是`3/0/1`并返回unavailable。saved collision replay、saved planned retry与saved fingerprint conflict都不重采样，故allocationReads为0。saved/early branch没有CAS时维持`1/0`或更早gate的`0/0`。该定义是conformance observation，不替D6冻结物理read次数。

#### 4.1.3.1 当前wire11义务与历史机器证据

新decision的唯一当前版本矩阵是wireVersion11接受，0–10及未知版本拒绝；旧v9/v10仅按§4.1.3a授权的保存决议重放或恢复，不建立新decision。当前mutation binding为Result/9、Annotation Value/3、D3-CJ/3；十数组per-mode与B/C/N/M/E/S/Q及scope/mapping规则以本稿规范为准。

当前conformance义务覆盖全部closed union和per-mode正负格、primary/member owner、existing preimage/result pair、fresh typed slots、完整C/Field流与prepared map、S target/reply/restored source lifecycle、整数/排序/UTF-8/locator、stage/ledger/reservation/restore/purge、copy/fork/import及完整receipt。完整新decoder/corpus尚未实施，不宣称下述537旧向量或28个旧review goldens证明新组合。

以下引用块逐字来自原冻结v9权威的机器语料章节，所有“当前/current/冻结/Gate”仅指那个历史版本；其中v9、Result/7、数组数、旧generator/validator路径、bytes/digest/count均是原历史记录，不能用于选择本稿的新decision decoder，也不要求为本轮包装重新计算其hash。

> #### 4.1.3.1 单源机器语料与可读向量目录
>
> 冻结时的唯一机器语料由Research中的exact-base-SHA-bound review-only generator从本节closed schema constants、typed fixture factories与continue transition function确定性生成到 D3 Wire Conformance Corpus v21.json（外部控制记录未随本输入发布）（887410 bytes）。
>
> 该corpus accepted output的结构化摘要为 [D3-DIGEST:accepted_output:wire-corpus]`70B0978DC6DD7D6B3CE1C39F01BF25F17F98DAD6C1B93EFB0DB7AAD70F509048`。
>
> 验证明确分两层：冻结v20由matching `validate_d3_wire_corpus_v23.py`以完整独立closed ref/locator/symbolic decoder、typed comparator、reject expectation derivation、Annotation outer materializer与continue transition function在normal/-O/-OO下复算；v21由不导入generator的`validate_d3_wire_corpus_v21.py`独立重建exact-base v20→v21递归D3 version transform、五个Annotation `d2Outer` wire 1→2、0..8/10 reject与9 accept矩阵、逐fixture canonical bytes/length/SHA/expectation及28个七切片goldens，并从冻结classifier及final D2 Facet lexical grammar/product推导7条Task operation paths、9个Task semantic cells与2个Template/Facet product cells。最终review-package manifest可锁定script与accepted bytes/hash，但v21 generator本身只受exact base bytes/SHA/schema/ID约束，不是manifest-bound；Candidate不手抄自引用向量。corpus revision 21以v20的536 fixtures为受guard的非回归基线，机械生成537个current-wire fixtures、保留32个base groups/101个reject oracles/20个canonical comparator domains，其中仅五个`annotation_outer_materialization_case`的nested `d2Outer.wireVersion`按final D2 generation从1升2，并另列28个`review-semantic-canonical-json/1`七切片goldens；其中Task证据明确区分7条operation paths与9个semantic cells，另有2个Template/Facet product cells；这些语义必须由冻结classifier及final D2 Facet lexical grammar/product机械推导；额外prefix-vector group不扩张comparator domain。authority-generation由5个ledger state×3个generation relation×适用的same/different或not_applicable fingerprint形成27个exact cells；continue authority allocation由`observationScope,phase,validationOutcome,candidateObservation,planningCasResult,savedDecision,fingerprintRelation,terminalAction`推导19个exact reachable cells；materialization intent shape为16个exact cells，state/effect为14个exact cells，deterministic re-import另有9个OriginBinding lifecycle cells。v20 matching generator与v23独立validator分别持有domain input constants和transition function并要求exact key-set equality；v21 generator/validator只对exact-base绑定、递归D3 wireVersion transform、五个Annotation `d2Outer` wire 1→2、version matrix与七切片review goldens作独立重建，删除或手改任何受检项即失败。
>
> independent locator decoder只从token bytes验证alphabet、padding、magic/NUL、canonical JSON、closed variant、nested NodeRef/ResourceRef、UUIDv4、D3Integer与range/region invariant；fixture name、expected outcome与metadata都不能改变validity，metadata仅可在成功typed decode后比较expected value。mutation harness把同一合法token重标negative时必须报告“negative accepted”，并证明四个malformed nested-owner token即使携带actual decoded body仍拒绝。symbolic decoder只读取`D3-Symbolic-Result/7` bytes，递归验证全部PayloadSubjectKey与ownerSubject、slot/ordinal、S segment的Annotation subjectRef与resultSlot.container相等，以及内嵌start/end partition从0到baseLength严格连续、无洞、无重叠、count>0；optional metadata不参与validity。C0、empty/adjacent B、missing/gapped/overlap partition、nested bool/string/MAX+1 ordinal及S mismatch逐一独立拒绝。Candidate、生成器、corpus与validator作为同一Gate输入；任何手工编辑corpus、不能byte-for-byte复现、hash/count或exact coverage set不符都使Gate fail。
>
> corpus的537个继承wire fixtures仍是closed union：`encoding=D3-CJ/3|D3-Symbolic-Result/7|locator-token|raw-negative`；另列的28个amendment fixtures固定`encoding=review-semantic-canonical-json/1`且只属review evidence，不是product wire kind。继承的`gate_v8_*` fixture names与`D3-Gate-v8-Closure/1` metadata只是不改写的base provenance labels，绝不能决定decode或current wire；D3 envelope accept bytes固定wireVersion 9，version-cutover保留0..8/10 reject。五个`annotation_outer_materialization_case`的nested `d2Outer.wireVersion`恰为2且其`value`仍排除`wireVersion`；这不改变D3 envelope或独立`D3-Symbolic-Result/7`、`D3-Annotation-Value/3`、`l1` version domains。每个base fixture并恰含`name,group,encoding,expectation,bytes|bytesBase64,length,sha256`及该encoding要求的typed metadata。`expectation`恰含`validity=accept|reject, family, stage, reason`；pure encoder向量使用`family=encoder,stage=encode,reason=none`，不能被误读为operation decoder接受结论。negative fixture必须给出唯一expected family/stage/reason。所有name唯一，文本fixture的length/hash按UTF-8 exact bytes，raw-negative按base64 decode bytes；文件UTF-8无BOM、LF only、恰一尾随LF。
>
> 必需fixture catalog至少包括：
>
> - base envelopes：allocation issue/replace/proposal/outcomes、create/fork fingerprint body、operation request、receipt、error、三个resolver与transfer；
> - 所有refs、subject、locator-token body、intent、八个plan array element、十二个receipt array element、restore三种location family、Annotation result context及所有closed union/null/value branch；
> - strings：short control escape、lowercase `\\u00xx`、uppercase-escape negative、quote、backslash、non-BMP、U+2028/U+2029、NFC/NFD distinct、lone-surrogate与invalid-UTF-8 negatives；
> - integer/version：MAX-1/MAX/MAX+1、`-1/-0/1.0/1e0`，wireVersion 0–8与10 reject、9 accept；
> - request/receipt八/十二数组的至少三元素permutations，全部canonicalize到同bytes/fingerprint；
> - Annotation Value/3 full payload：target-only、target+reply、P→Q、P→null、null→Q、target/reply adjacent span、双表示negative；
> - Symbolic Result/7的B/M/N/E/S、内嵌baseLength与start/end、首尾/相邻slot、explicit resultSlot、missing/duplicate/overlap/gap/length/count/version negatives，以及递归owner ordinal bool/string/MAX+1、C0、相邻/空B与S subject/container mismatch negatives；
> - D2 non-regression：create/copy/import/fork的node_link/citation + AuthorAnchorAddress全部reject，NodeRef equivalents accept；
> - lifecycle：source-unchanged suspension、purge B+rewrite live A、trash P+reparent live C、restore A+delete invalid slot、nested Trash simultaneous removal；
> - owner-local copy：same-owner Resource copy+occurrence rewrite accept，cross-owner foreign Document rewrite reject，缺preimage/result revision evidence reject；
> - proposal digest：create/fork lowercase-hex fingerprint accept，uppercase/base64/length negatives。
>
> 当前corpus accepted output：`887410` UTF-8 bytes，[D3-DIGEST:accepted_output:wire-corpus]`70B0978DC6DD7D6B3CE1C39F01BF25F17F98DAD6C1B93EFB0DB7AAD70F509048`，`537` inherited/current-wire fixtures，`32` base groups，另有`28` review-only amendment fixtures覆盖恰七切片。
>
> Generator/validator source role 的accepted bytes与摘要只由不可变Gate package/frame manifest机械列出并核对；Candidate不承载source-role digest prose authority。独立validator重新计算每项length/hash、D3-CJ/3 canonical round-trip、bytes-only closed locator/Result/7 decode、expectation completeness、fixture/group coverage、20个typed comparator domain、101个reject oracle、27-cell authority matrix、19-cell transition-derived continue allocation matrix、16-cell materialization shape matrix、14-cell materialization state/effect matrix、9-cell deterministic re-import matrix、完整D2 Annotation v2 outer materialization及mutation harness，不信任载体自报字段。

OperationId授权/重放矩阵以固定总序压缩为下表；`任意`表示该维度不得被探测或不得改变结果：

| ledger state | current authorized | authority/ledger continuity | fingerprint | generation relation | 唯一primary outcome与ledger effect |
| --- | --- | --- | --- | --- | --- |
| 任意 | false | 任意 | 任意 | 任意 | `identity_not_visible`；不读ledger，不交付bytes，不改state |
| 任意 | true | unavailable/unproven | 任意 | 任意 | `identity_authority_unavailable`；不改state |
| 任意 | true | available但唯一authority/ledger integrity检查失败 | 任意 | 任意 | `workspace_integrity_conflict`；不改state |
| existing任意 | true | available/proven | different | 任意 | `operation_id_conflict`；不披露旧request/state，不改state |
| rejected | true | available/proven | same | unchanged或changed但continuity已验证 | byte-equivalent saved recorded rejection；不改state |
| committed | true | available/proven | same | unchanged或changed但continuity已验证 | byte-equivalent saved receipt；不改state |
| terminal_failed | true | available/proven | same | unchanged或changed但continuity已验证 | byte-equivalent saved error；不改state |
| planned | true | available/proven | same | unchanged | 复用原reservation恢复执行；只可转一个terminal state |
| planned | true | available/proven | same | changed且continuity已验证 | 复用连续ledger中的原reservation恢复；不得重新reserve |
| unseen create/fork | true | issuer/source available/proven | n/a | P1 authenticity/audience/binding或P2 current-family失败 | preflight `workspace_identity_conflict`；target ledger zero-read，family/successor零effect |
| unseen ordinary | true | available/proven | n/a | token byte-equal current generation且stage 7–15通过 | planning CAS重验TL/ledger/current generation后写planned+reservation并执行 |
| unseen ordinary | true | available/proven | n/a | token属于continuity-proven older generation | stage5 `identity_authority_unavailable + preflight_rejection`；ledger read=1，write/reservation/decision=0 |
| unseen create/fork | true | available/proven | n/a | 适用P1/P2、stage 5、7–15全部通过 | planning CAS重验P2/TL/ledger后写planned+reservation并执行；proposal同boundary claim |
| stage5 unseen | true | available/proven | n/a | stage 7–15任一primary failure | 写入rejected；current proposal转decision_bound_rejected并burn IDs，exact retry可经P2(c)重放 |

所谓`generation changed但continuity已验证`只表示新的active authority通过§8.5 continue/failover gate继承同一Workspace ledger；它不把generation从fingerprint中移除。调用方重放时必须提交原canonical request及原expectedAuthority字段，才能得到same fingerprint；以新generation改写request即different fingerprint并conflict。不同Workspace中的同UUID落不同key，各自按unseen/existing独立判断。

#### 4.1.3a v10 显式既有源组合与符号源物化

本节定义继承自v10的既有源组合/C物化语义，由当前wire11原样消费，并加上§21的D7准备绑定、definitionTransfers与Q段。当前新decision只接受wire11。wire10和wire9只走各自原closed decoder、原canonical fingerprint及原授权/authority/P/TL门之后的保存决议重放/恢复；先按请求版本选择legacy decoder，不读取ledger选版本。stage5 existing same fingerprint可重放原bytes或恢复原planned；unseen固定legacy unsupported_wire_version preflight，零write/reservation且不进入7–15。同Workspace同OperationId跨版本输入不同仍冲突。旧版本proposal不再新签发；已有family/decision只按历史绑定恢复，不加新D7条件。D2/2、CJ/3、AnnotationValue/3、proposal认证/7各自版本不变。历史v9/v10模型仅证明原分支，不是当前wire11/Result9新分支证据。

**existingPayloadEdits。** 每个当前wire11 plan必含该数组，元素closed `{ref,expectedRevisionToken,potentialChanges,artifactBinding?}`。ref为完整Node/Resource/Annotation Ref；token是该源非空opaque revision，禁止只靠sha256防ABA。artifactBinding仅ordinary import必需，其余mode禁止；闭集为`{kind:"materialized_object",artifactObjectKey}`或`{kind:"companion_effect",sourceArtifactKeys}`。前者表示该artifact对象明确更新existing对象，后者是由已绑定artifact事实/创建结果派生的附带源修改，sourceArtifactKeys为非空、按ArtifactObjectKey排序唯一的真实输入keys，不占第二个artifact对象。键使用既有ArtifactObjectKey decoder。只允许create_node/create_resource/create_annotation/import_new+ordinary_format非空，其余mode强制空且保留本来已有的compound能力，不扩大copy/fork/生命周期矩阵。按RefKey排序、ref唯一；该数组与新增的existing subject preimage/result payload pair精确双射，缺、重、孤立pair静态拒绝。声明仅允许既有live payload、适用Annotation reply及同源Field/framing修改，禁止改变identity、owner、lifecycle或Node placement。Annotation reply必须同owner并由S唯一表示。

ordinary import中artifact manifest occurrence恰分fresh与existing两组；两组ArtifactObjectKey联合完整、唯一、无洞，freshSubjects只覆盖fresh组，existingPayloadEdits中的materialized_object仅覆盖由当前可信SourceBinding/OriginBinding及显式映射意图选定的existing组；companion_effect不参加对象分区，必须由所列输入事实/创建结果严格推导其源修改，不授权任意upsert。不能按title/path/任意UUID自动upsert。fresh ordinal按fresh组key相对排序连续分配；root forest只从fresh组推导，仍至少一个fresh Node root。全部既有的batch不合法D3 import，走D6 owner。mixed batch的binding/mappingVersion/watermark由该同一规范输入派生并同一decision发布。

existing preimage slot不得伪装为result-only/absent；删除用准确resultContext，新slot才用result-only，old与new全部coverage。raw no-op允许绑定existing pair，仍用于依赖/潜在owner授权，但不增source revision、不列preserved。preserved恰列实际改变existing对象；fresh仍仅allocated/resultAllocations或identityMap；D6 companion记录真实source versions，不能代替D3 preserved/reference/structure证据。

stage落点唯一：1检查十数组closed形状、模式、pair与canonical重复；2验证target Workspace role；3对每个明确existing声明及符号规则可能写入的既有容器验证本意图实际可能改动的Field/body/control能力和适用locator披露；7检查encoded owner；9检查exact ref live；11检查expected revision及原locator；12验证下面候选map唯一性；14在同cut核对actual preimage、符号源全量物化、实际footprint属于已授权可能范围，以及D2/D4完整语义和source/slot coverage；15检查完整引用/结构/owner闭包。越权效果先按not_visible遮蔽，不能先返回泄露隐藏Field的语义详情。失败沿原family和decision CAS，不占stage6。对符号canonical owner选择，stage3保守要求**全部明确列出的潜在existing owner**具备相应效果能力，未选中分支缺权也拒绝。这是新fresh组合入口的明确取舍；既有D4纯编辑仍按实际canonical owner授权，不改变其合同。

potentialChanges是非空closed union数组，元素恰为`{kind:"field",fieldId}`或`{kind:K}`，K闭集`body|document_metadata|node_declaration|resource_payload|annotation_payload|annotation_reply|raw_source`；按此顺序rank（field=0后FieldId ASCII，其余依次1..7）排序、无重复。ref与类别须静态相容；FieldId用D4完整词法，不读取inner Entry。stage3只读取这份外层声明及当前policy作保守授权：field→对应field_write/source_write且无该Field deny；body/metadata/declaration依D6能力矩阵；Resource/Annotation依其相应写权；raw_source要求source_write以及该Ref范围没有任何适用细项deny，且包含声明变化时还要求node_control。raw_source不豁免D2/D4门禁。carrier必要framing属于其确切Field修改；只能声明field来更改对应Entry及其必要framing。stage14独立从完整preimage/result重算全部actual changes，必须是该有限声明允许的子集；缺任何潜在既有容器或分类伪报均不得提交，也不向caller泄露隐藏解析详情。原计划可能修改的范围不能由后验effects自报。

**候选map与reservation分离。** 只有stage5确定unseen且有权继续后，stage12为全部fresh/mapped content subjects由Core私有采样一个ephemeral候选map，资源/批注owner先由其Node subject物化。此时无持久allocation、无reservation、无entity及对外候选ID输出。map按subject键唯一绑定，不从hash或用户ID选择。stage12对完整allocation/burn history查唯一性；collision沿既有identity uniqueness family记录rejection，当前未决attempt不重采样。stage14–15使用该同一个map和拟议source revision物化并完成全部门禁，选择哪个canonical owner不能成为retry抽签理由。

planning CAS再验原cut、当前权限、候选仍free，同时保存完全物化的canonicalPlan、该map、planned和同一组reservations；CAS loser按原restart。若发现same-op赢家，丢弃本地map并重放赢家；仍unseen时本attempt保留map，候选被别的operation占用则在stage12作recorded rejection。decision前进程崩溃没有持久candidate，下一次新attempt可重新采样；不得宣称未落盘值跨crash永久绑定。planned以后只有保存的map/源/refs，禁止重新选择或首次运行D4验证。该规则也用于copy/fork/import的content映射；Workspace proposal和continue authority candidate沿原各自规则。

**Result/9 typed carrier分段。** 外层沿§4.1.3的B/M/N/E/S partition语法（当前magic /9；历史Result/8只用于原保存决议），新增tag `C`，只允许Node exact_source_document。C覆盖骨架中一个完整D2 carrier（包括opener、namespace line、entry line前缀/EOL和closer）；它不是D2 token/Entity/Locator。C body恰为CJ/3 `{container,registryBindings,baseCarrierBytes,entries}`；container必须等于payload subject；registryBindings为按WorkspaceRef排序唯一的`{workspaceRef,registryBinding}`数组，完整覆盖该C所解释source与target Workspace，使用D4完整closed binding且逐项等于受信Registry cuts；bytes字段用canonical无padding base64url编码raw bytes。baseCarrierBytes长度等于end-start，strict UTF-8且完整满足D2 carrier grammar，最多32blocks/8192entries等上限在完整结果仍重验。entries按base carrier真实entry-line顺序逐项完整覆盖，不能跳行、重叠、改namespace或靠caller span猜位置。

每个entry描述恰为`{kind:"literal_entry",rawEntrySource,referenceTemplates}`或`{kind:"relation_entry",rawEntrySource,referenceTemplates,factKey,endpoints,allowedOwners}`。rawEntrySource逐字等于对应base entry payload；literal prefix和该行EOL从base取得，不能把raw JSON误当完整line。referenceTemplates按pointer UTF-8排序，元素恰为`{pointer,template}`；pointer是以`$`为根的RFC6901完整typed Ref/Locator根路径。stage14先校验C.registryBindings逐项等于受信cuts并完成namespace/schema gate，再由同Registry schema独立解释raw Entry，验证这些pointer恰覆盖所有已知typed根，无重复/嵌套、locator不再拆内ref。普通string/note看似UUID也永不替换。未知schema源只按原raw保留能力处理；需要无法证明的重写则typed操作不可用，不能猜。

template闭集：`{kind:"exact_typed_value",value:<完整D3 Ref或Locator>}`、`{kind:"subject_ref",subject:<existing/mapped/new同kind subject>}`、`{kind:"subject_locator",locatorKind:"element"|"range",ownerSubject,sourceSpan,elementKind?}`、`{kind:"subject_resource_region",resourceSubject,regionToken}`、`{kind:"opposite_relation_endpoint"}`。elementKind仅element必需；span/regionToken及所有非身份定位事实使用原D3 closed decoder。subject_locator/region仅重发其同plan有明确完整result源及拟议revision的target，否则须用exact_typed_value；existing unchanged target保持完整exact locator并核验当前revision。AnnotationRef适用owner-local规则；未知kind或裸UUID拒绝。opposite仅relation_entry真正relation value.target的typed NodeRef根允许，literal target不使用它。exact模板的实际decoded value必须等于原raw根，并输出原根bytes；subject模板只替换该typed根全部bytes为物化对象的CJ/3，所有其他raw Entry bytes逐字保留，不重序列化整个Entry。D4 provenance/qualifier内已知typed根同样完整处理。

**关系单事实与carrier assembly。** factKey恰为`{sourceOwner:<existing/mapped/new Node subject>,fieldId,occurrenceKey}`，只作本plan内容地址。endpoints恰为`{left:<Node subject>,right:<Node subject或literal typed target>}`；schema必须证明关系方向、target variant及source fact/initial explicit Entry真实对应。directed/literal owner固定为物化sourceOwner；symmetric NodeRef按D4完整NodeRef comparator选唯一canonical owner，target由opposite模板机械取另一端。self-edge按Field规则拒绝，不能靠重复emit规避。allowedOwners是此mode允许的结果Node subjects，按subjectKey排序唯一：create compound可包含声明过existing edits与fresh，copy/fork/partial transfer只允许D4已冻结的fresh closure。集合必须恰列两端中适用的可写owner候选，不把forbidden existing owner写进copy plan；实际canonical owner不在该集合则whole-operation在planning前拒绝。每个factKey在每个allowedOwner恰有一个相同事实定义的relation_entry，其余重复或遗漏拒绝；对该实际container只emit选中owner的一份，其他entry输出零bytes。共用事实所有非引用raw bytes、键/qualifiers/note/provenance必须相同，target只是同一对endpoints的方向投影，不生成反向作者副本。

C物化采用整个结果Document的有序source assembly，不能把每个carrier当独立Field stream。C的base位置只是已绑定的事实输入位置；是否为迁入由fact sourceOwner经同map得到的原结果owner与最终canonical owner独立比较，不由caller自报。先保留所有未迁移Entry的原line位置及完整未修改bytes，原位retarget只替换选中typed根；收集跨owner迁入的唯一事实，移除所有非选中owner副本及选中owner中的迁入占位line，然后统一插入。每个受影响namespace的全部carrier须进入同Document受信解析视图，并由C逐Entry绑定；不得在B中藏另一个同Field occurrence来改变定位。

迁入按旧source owner RefKey、FieldId、occurrenceKey排序；brand-new sourceOwner用subjectCanonicalKey、FieldId、occurrenceKey，二者混合时existing/mapped来源组在前、new来源组在后，组rank及原key进入唯一规范顺序。排序只作用迁入队列，不重排任一原有作者stream。逐项在已保留原位项加先前迁入项组成的完整owner stream上执行D4既有定位：目标Field存在时，插在跨全部同namespace carrier的该Field最后一个occurrence之后；不存在时追加在owner全部作者Entries之后。不能替换成首个或最后一个namespace carrier。如果owner最后carrier恰为所需namespace，可在其最后Entry之后追加；否则使用该输入骨架已显式绑定在D2 carrier prefix末尾的所需namespace潜在carrier。需要潜在carrier却未绑定时在planning前拒绝；Core准备器必须提供完整所需骨架，不在执行时暗造新framing或扩大写域。迁入line的raw Entry、literal prefix与EOL取选中owner已绑定的C占位line，不能从另一owner猜换行。

所有迁移与插入完成后才决定carrier是否为空：最终零Entry时整体删除其opener/namespace/closer，非零时保留绑定的framing bytes。一个被移空但又收到合法迁入项的carrier可保留，不能先删后遗失其source锚点。潜在carrier最后为空则整体消失。所有非carrier B bytes（包括comments、空白和body）逐字保持且相对位置不变；不得合并namespace或将其重编码。由已绑定的真实完整preimage/initial source、fact graph及该唯一算法独立重算最终source，并验证D4作者Field顺序、Entry非引用bytes、owner-local refs、requiredness、unknown namespace原样、D2全部framing/32-carrier/8192-entry及byte/inspection预算。D4SourceMaterializationEffects列真实最终sources和按事实导出的变化，不得反过来用effects自报作为合法source证明。M/N/E的最终slot ordinal与locator在整个assembly结束后的真实D2解码结果上计算；C本身不制造D2引用slot。

C内部typed根不是D3 authored slot，不进入D3 rewrittenReferences；由D4SourceMaterializationEffects/1与完整source binding证明。B不得藏入已知需要重写的typed carrier根；若所有根都exact且无迁移，可用一个C或对不需要任何typed语义的raw-preserving操作保留B，后者不宣称typed重写成功。普通D2 reference仍M/N/E且ordinal从最终D2 decoded authored slot序得到，C不制造D2引用slot。关系schema的Entry必须使用relation_entry，literal_entry不能绕过canonical owner/单事实规则。raw Entry的fieldId/key必须独立解码并与factKey及下述外层声明匹配。每个C的完整body进入原payload sha256和canonical request fingerprint；从骨架partition可重建其全部绑定bytes，字段模板/framing/owner选择任一变化均改变请求。

**Result/9 S目标。** annotation_reply structuralPlan.replyTo现为`AnnotationRef|null|{kind:"fresh_annotation_reply",subject:<new/mapped Annotation subject>}`。fresh variant仅同plan且owner物化为existing Annotation的同一owner时合法。S body仍显式绑定完整structuralEntry，base span按骨架reply value的完整CJ bytes定义；fresh variant的end-start比较其骨架完整variant bytes而非最终AnnotationRef长度，输出为map物化的concrete AnnotationRef。StructuralPlanKey nullRank改为null=0,concrete=1,fresh=2，fresh分支比较subjectCanonicalKey。receipt toReplyTo只能concrete Ref/null，必须等于同map物化结果；禁止在reference evidence重复列该reply。所有其余S/slot/owner/cycle规则保留。

C及S是请求绑定的符号骨架，不是可直接交付的作者source。typed根raw形状必须符合D4 constructor，但fresh/mapped身份位置只能由template解释；其骨架UUID不成为caller-chosen候选ID或state lookup。拟议revision由Core按准确source preimage与最终改变规则签发：fresh source初值1，实际既有source改变checked-add1，raw no-op保持；需要subject_locator的目标须有唯一可证明的最终revision与完整source span，无法闭合则planning前拒绝，不以搜索/近似anchor补齐。物化后独立验证全部locator与source revisions互相一致。

existingPayloadEdits的canonical comparator为RefKey；artifactBinding及potentialChanges属于该closed entry且同ref不能重复，修改其任一key、潜在效果或revision必然改变CJ/fingerprint。原plan示例若省略existingPayloadEdits只是历史fragment，完整v11请求必须补上[]，缺省不接受。D4SourceMaterializationEffects是同decision受管结果扩展，任何该操作必需的扩展在commit前未完整生成和验证即不得发布D3成功receipt。

v11 decoder、完整D2/D4解析器与D4 effects独立重建最终sources；只有哈希相符不足以证明source合法。本候选的本地检查必须明确其有界覆盖，完整产品引擎、跨语言全量v11 conformance和host验收仍是相应实施义务，不把旧v9向量改版本数字当新分支证据。

#### 4.1.4 Common wire integer domain

`D3Integer`是所有D3 v11 JSON integer的共同数学域：`0..9223372036854775807`（`MAX=2^63-1`），decoder必须先以数学整数解析，禁止依赖宿主语言number精度。负数（包括lexical `-0`）、浮点、指数形式、超出共同域或无法无损解析固定为static invalid：request/receipt/proposal/transfer的ordinal、sequence、count与observationSequence使用`invalid_integer_shape -> invalid_identity_ref`；locator line/column/offset使用`invalid_locator_shape -> invalid_identity_ref`；envelope `wireVersion`非整数或超域为`invalid_closed_shape`，域内但不等于11为`unsupported_wire_version`。

字段子域闭合为：`wireVersion == 11`；proposal `sequence`、new-result/artifact/slot ordinals、destination/placement ordinals、transfer `observationSequence`均为D3Integer；line、column、offset/end coordinate也为D3Integer。所有派生算术先在无限精度数学整数上求值再比较MAX，永不wrap/clamp/扩大域：allocation replacement的MAX分支为A11；`destinationOrdinal+i>MAX`为stage10 `invalid_ordinal`；symbolic segment count/length>MAX为stage14 `identity_map_incomplete`。transfer在`observationSequence=MAX`时必须已经是terminal observation，`pending`/可继续状态在MAX为invalid closed outcome，且terminal后禁止新observation；因此MAX-1只可前进到MAX terminal。通过共同域但超出当前parent合法区间的ordinal也只在stage10返回`invalid_ordinal`。`-1`、`-0`、`9223372036854775808`、任意更大JSON integer和`1.0`不得进入semantic stage。

### 4.2 Workspace identity

WorkspaceId 标识逻辑内容 authority namespace。WorkspaceRef 的闭合结构：

~~~json
{
  "kind": "workspace_ref",
  "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283"
}
~~~

WorkspaceId 不等于 root NodeId、路径、repository ID、backup lineage、Server database row、URL、账户、租户、设备或 AuthorityInstanceId。

AuthorityInstanceId 是 D3 暴露给 D6 的最小控制身份，不是 content identity。其持久化、lease、epoch、fencing 或选主机制由 D6 决定：

- 每个被激活为可写的物理 replica 在首次激活时获得一个 AuthorityInstanceId；
- NodeRef 不含 AuthorityInstanceId，因此合法 failover、重启或 exclusive disaster recovery 不改变 content ref；
- 一个 byte-for-byte 副本在显式选择 continue 或 fork 前不得自动取得写资格；
- D6 必须以 fencing、单调 generation 或等价机制阻止 stale/并发 AuthorityInstanceId commit；无法证明独占的副本只能只读/reconciliation；
- backup/export artifact 不是 AuthorityInstance，不携带写权。

关闭、卸载、换设备、换路径、Server failover 不改变 WorkspaceId。D3 不创建 Workspace Trash。整个 Workspace 的销毁属于 D6 管理动作；销毁终止该 Workspace authority，之后其 typed refs统一解析为workspace_unavailable而非tombstoned。第7.5节的known-purged保证只在该Workspace authority存续期间成立；workspace_unavailable不证明某ref曾存在、从未存在或已purge。外部系统不得把旧路径或同名新Workspace猜成原Workspace，也不得复用其WorkspaceId。

### 4.3 NodeRef

NodeRef 是唯一 Node durable ref：

~~~json
{
  "kind": "node_ref",
  "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
  "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
}
~~~

规则：

- root、普通 Node、包含exact `tasks/task` Facet的Task Node、`coreKind=template` Template Node使用同一 NodeRef kind。
- 不存在 DocumentRef、TaskRef、TemplateRef、FolderRef、BlockRef、HeadingRef 或 ViewRef。
- NodeRef 在同 Workspace move、reparent、reorder、rename、title edit、core-kind/facet-membership change、Document revision change时保持不变。
- NodeRef 的 workspaceId 必须与 authoritative owner Workspace 完全相等；foreign NodeRef 不经当前 resolver 自动 fetch、mount、search 或 coerce。
- D2 source 中的 Node targetToken 使用唯一 canonical token n1.WORKSPACE_UUID.NODE_UUID；token 只是 NodeRef 的 source serialization，不是第二 identity。
- Node targetToken 的 workspace UUID 与 owning Document Workspace 不同，D2 same-Workspace validation 必须 fail closed。

### 4.4 ResourceRef

ResourceRef 是 owner-local 复合 ref：

~~~json
{
  "kind": "resource_ref",
  "owner": {
    "kind": "node_ref",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
    "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
  },
  "resourceId": "93d826e7-237d-47f6-9e14-04fe9cc65a13"
}
~~~

规则：

- ResourceId 只在 owner NodeRef 内有意义；bare ResourceId 不是 ref。
- Resource owner 永不可变。所谓跨 Node move 是 copy-to-new-owner + optional trash-source，必得新 ResourceRef。
- Resource bytes、filename/path、media type、digest、caption、alt、page/width/height 都不是 Resource identity。
- bytes replacement 在同一个 Resource 对象上保留 ResourceRef，但产生新的 resource revision/digest；region locator 失效。
- owning exact-source Document 中 source token使用 r1.RESOURCE_UUID。decoder必须从D2冻结的Node:Document恰一existential ownership结构推导唯一owner NodeRef；这是source结构的一部分，不是UI selection、调用参数或ambient context。
- r1 token只在该owner Node的exact-source Document内合法；另一个Node中的相同token绑定另一个owner，不能表达跨owner ResourceRef。在全部D2/D4受管authored state中，任何直接Resource ref的containing/owning Node必须byte-equal `ResourceRef.owner`；未来D4不得用“完整ResourceRef”绕过此上游边界。跨Node只能复制为目标owner下fresh ResourceRef，或用普通NodeRef引用原owner Node；若产品要允许跨owner Resource关系，必须先显式重开D2。
- ResourceRef 的 JSON owner 必须完整；不得用当前 UI selection、path、parent parameter 或 ambient context 补齐。

### 4.5 AnnotationRef

AnnotationRef 是 owner-local 复合 ref：

~~~json
{
  "kind": "annotation_ref",
  "owner": {
    "kind": "node_ref",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
    "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
  },
  "annotationId": "7d5e5be7-ab9d-456a-9067-2dc18d92bcc3"
}
~~~

规则：

- Annotation owner 永不可变；跨 Node copy 必须生成新 AnnotationRef并重建 target。
- replyToAnnotationRef 必须与当前 AnnotationRef 具有完全相同 owner，并保持无环。
- Annotation target 从 resolved 变 stale 不改变 AnnotationRef。
- purpose、body、author display、reply position、target locator、suggestion payload、时间和状态都不是 identity。
- 删除一个带 replies 的 Annotation 默认作用于该 reply subtree；不允许留下 live reply orphan。未来 UI 若要保留 replies，必须先显式、原子地 reparent并重新验证无环。

### 4.6 D2 Resource、Annotation 与 non-durable occurrence

| D2 对象 | D3 identity 决议 |
| --- | --- |
| Document | 无独立 ID；使用 owning NodeRef 地址 |
| DocumentMetadata / DocumentBody | 无 durable identity |
| section/heading/paragraph/list_item/table/row/cell/citation/bibliography placement | occurrence，仅 RevisionLocator 或 AuthorAnchorAddress |
| AttributeCarrierBlock / LexicalAttributeEntry | non-durable lexical occurrence；只有 owning Document + current revision source range；无 EntityRef、lifecycle、cross-revision continuity、locator token、AuthorAnchorAddress 或 Annotation target |
| SavedQueryViewDefinition | occurrence，仅 NodeRef + locator/anchor；无 ViewRef |
| Task | ordinary Node whose available Facet set contains exact `tasks/task`；只使用 NodeRef，无 TaskRef、Task wire kind或mirror |
| Template | `coreKind=template` Core meta-kind；只使用 NodeRef；Task Template只在D9 target plan要求fresh ordinary + `tasks/task` |
| NodeCollectionResult | 无 durable identity；result handle 属于 D6/D7 短期 envelope |
| UnmanagedItem | 无 Workspace entity ref；path-based inventory entry 不能进入 content ref |
| Record/RecordCollection | D3 不定义；D5 若选择必须另建闭合 domain，与 NodeRef 不复用、不 coercion |

## 5. 所有权与结构

### 5.1 总体图

    WorkspaceRef
      └─ owns every Node identity in one connected ordered tree
           ├─ structural parent NodeRef + siblingOrdinal
           ├─ existentially owns exactly one Document address
           ├─ strictly owns ResourceRef*
           └─ strictly owns AnnotationRef*

Workspace 是 Node 的最终 aggregate owner。Node parent 是结构 containment，不是 identity namespace：

- child NodeId 不在 parent namespace 内；move 到另一个 parent 保留 NodeRef；
- non-root live Node 必须恰有一个 live structural parent；
- root parent/ordinal 为 null；root 不能被 reparent、trash 或 purge 为普通 Node；
- delete parent 默认处理完整 subtree；若用户只删 parent，必须先显式原子 move children，不能产生 orphan；
- parent title/path/label变化不影响 child identity。

Document 是 Node 的 existential component：与 Node 同生、同 live/trashed/tombstoned边界，不得 detach、transfer或单独拥有 identity。

Resource/Annotation 是严格 owner-local：

- ref 编码 owner；
- owner 不可修改；
- owner trash时随 aggregate进入 trashed；
- owner purge时其 payload一起 purge并各留 tombstone；
- owner tombstoned或不存在时，Resource/Annotation 不得恢复为 live。

### 5.2 无孤儿不变量

- live Node tree 从 root 全连接；missing parent、cycle、duplicate NodeRef、duplicate/gapped live `siblingOrdinal`是workspace structural invalid/reconciliation，不是可自动修复状态。每个Trash sibling list同样必须具有从0连续且唯一的`trashOrdinal`；duplicate、gap或其他非canonical sibling list也是authority/integrity corruption。任一ordinary trash/restore/purge在授权后的stage 4 integrity preflight发现这些事实，都固定返回`workspace_integrity_conflict + preflight_rejection`：target OperationId ledger content零读写、零allocation/reservation、零payload/placement/lifecycle mutation且无receipt。`RefKey`只可稳定排序repair-channel diagnostics，不得决定普通operation的committed poststate。siblingOrdinal只是D6结构排序值，不进入任何ref；对已证明canonical的有效列表做renumber或合法compaction不改变identity，restore位置占用只按当前live siblings判断。
- live Resource/Annotation 必须有 live owner Node。
- trashed Resource/Annotation 必须有live或trashed owner。
- tombstoned Resource/Annotation保留完整encoded owner NodeRef，但owner当前可以是live、trashed或tombstoned；owner状态不改变该owner-local ref，也不能让对象restore。若owner后来purge，Core复用既有object tombstone事实并保证owner closure完整，不能发布冲突身份事实。tombstone不是trashed对象。
- live Annotation reply parent必须 live且同 owner；trashed reply subtree不能被普通 resolver当 live thread。
- Core 不按路径、名称、source digest或唯一候选自动 reparent、reowner、rekey或恢复。

### 5.3 Annotation target 与 reply slot

D2 已冻结 Annotation 只能 target owning Node 自己的整个 Document、Document element、Document range、owner-local Resource 或 Resource region。D3 不扩张该集合。用于resolver、Value/3、copy plan与receipt的规范化`D3-Annotation-Target-Projection/1`（简称`AnnotationTargetProjection`）是以下closed union；这是唯一D3 identity/locator projection，不是D2 outer payload，也不存在省略owner或混用outer token字段的hybrid schema：

| kind | required fields | owner invariant |
| --- | --- | --- |
| document | `owner: NodeRef` | owner = AnnotationRef.owner |
| document_element | `locator: DocumentElementLocator` | locator.owner = AnnotationRef.owner |
| document_range | `locator: DocumentRangeLocator` | locator.owner = AnnotationRef.owner |
| resource | `resourceRef: ResourceRef` | resourceRef.owner = AnnotationRef.owner |
| resource_region | `locator: ResourceRegionLocator` | locator.resourceRef.owner = AnnotationRef.owner |

每个variant只允许表中字段与`kind`，unknown/missing/null/extra拒绝。Annotation target 不允许 NodeRef、AnnotationRef、AuthorAnchorAddress 或跨 owner locator/ref。target owner mismatch 在 D2 Annotation 创建/编辑 envelope 中按 D2 固定顺序返回`cross_owner_resource_reference`（Resource/region）或`invalid_annotation_target`（Document locator）；在 D3 copy/lifecycle operation 中映射为`owner_mismatch`。

从Projection/1到冻结D2 AnnotationTarget outer wire的materialization恰为：`document{owner}`→`{"kind":"document"}`；`document_element{locator}`→`{"kind":"document_element","locatorToken":l1(locator)}`；`document_range{locator}`→D2 range variant，其中start token body恰为`document_range_endpoint(owner,documentRevisionToken,edge=start,point=sourceSpan.start)`、end token body恰为同owner/revision的`edge=end,point=sourceSpan.end`，按§6.2各自经l1编码；`resource{resourceRef}`→同ref的D2 resource variant；`resource_region{locator}`→`{"kind":"resource_region","resourceRef":locator.resourceRef,"regionLocatorToken":l1(locator)}`。Annotation顶层`ownerNodeRef`唯一来自AnnotationRef.owner；materialization不得从path、label、ambient owner或registry补字段。Projection/1与D2 outer wire必须双向唯一：outer token decode后必须重建byte-equal Projection/1，否则stage14 `identity_map_incomplete`。

D2 v2 的 AnnotationTarget 仍严格只有上述五成员；`AttributeCarrierBlock`、`LexicalAttributeEntry`、field occurrence、`namespaceToken`、`rawEntrySource`和它们的source range都禁止进入该union。未来若D4需要跨revision durable field target，必须另行正式reopen D2/D3，而不能从本amendment推导。

D2 `targetStatus`只表示target identity/locator的exactness，不承载D3 entity lifecycle：Projection/1 owner/ref/locator在准确revision与coordinate仍成立时固定`resolved`，即使target entity当前`trashed`且D3 reference lifecycle为`suspended`；只有revision/coordinate/region不再exact时固定`stale`。trashed target不可普通导航或Action只由D3 Entity/Locator resolver和`referenceLifecycleTransitions`表达，绝不得把`suspended`发明为第三个D2 status、把exact trashed target误报stale或令Annotation projection unavailable。target restore不改Value/3 bytes或revision时，D2 status保持resolved，D3 transition显式`suspended→resolved`。

`reply_to`不是AnnotationTargetProjection：它只能是与source AnnotationRef同owner的AnnotationRef，并必须acyclic。target slot与reply slot的 expected union 永不互换；receipt decoder在解析宽泛target内容前必须先由slot选择唯一expected union。

AnnotationRef是durable identity，但可变Annotation value必须有D6提供canonical equality语义的`annotationRevisionToken`。它是non-empty opaque JSON string，不进入identity equality；任何body、target或reply_to committed变化都必须产生对同一AnnotationRef从未用于另一state的新token，即使value bytes后来回到旧值也不得复用旧token。D3不冻结token编码；任何非空JSON string都lexically valid，只有current-token equality决定准确preimage。ReferenceSlotAddress、rewrite result context与identity-preserving reply change必须分别携带preimage/postimage token，因而ABA不能被当作同一slot state。

## 6. Locator、anchor、path、label 和 span

### 6.1 DocumentElementLocator

DocumentElementLocator 绑定 exact Document revision：

~~~json
{
  "kind": "document_element_locator",
  "owner": {
    "kind": "node_ref",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
    "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
  },
  "documentRevisionToken": "rev:opaque-d6-token",
  "elementKind": "paragraph",
  "sourceSpan": {
    "startLine": 8,
    "startColumn": 0,
    "endLine": 8,
    "endColumn": 19
  }
}
~~~

elementKind 必须精确属于 D2 可地址kind闭集：`section|heading|paragraph|unordered_list|ordered_list|list_item|table|table_row|table_cell|literal_block|source_block|quote_block|thematic_break|resource_block|bibliography_placement|saved_query_view_definition`。unknown/future kind即使是non-empty string也固定`invalid_locator_shape`；fixture name、metadata或外部schema声明不得扩张该集合。sourceSpan 使用 D2 的 0-based logical line、Unicode scalar column、end-exclusive 规则。Locator equality 只表示同 owner、同 revision、同 kind、同 exact span，不表示同一跨修订 occurrence。

D2 wire 中每个locator token是self-describing deterministic value，不是registry handle或durable identity。exact token bytes恰为ASCII `l1.`后接RFC 4648 base64url alphabet对`ASCII "D3-Locator/1" + NUL + D3-CJ/3(LocatorTokenBody)`的编码；`=`padding严格禁止，空body、非canonical alphabet、能decode但re-encode不byte-equal、错误magic/NUL、非D3-CJ/3 body或unknown field/variant全部`invalid_locator_shape`。`l1` body内部的任何structural/lexical failure也被本token decoder吸收为`invalid_locator_shape`，明确包括nested owner/ref缺字段、wrong kind、extra field、UUID case/variant/version/grammar与owner Workspace不一致；通用`invalid_owner_shape`或`malformed_identifier`不得抢占一个`l1`输入。同一body只能有一个token；Core decode后必须按同一规则重编码并byte-compare，不能信任客户端或查询第二个mapping authority。

`LocatorTokenBody`是以下closed union，所有field都必需且不得为null：

| kind | exact fields（含kind） | use |
| --- | --- | --- |
| `document_element` | `kind, owner, documentRevisionToken, elementKind, sourceSpan` | D2 `document_element.locatorToken`与Body element `locatorToken` |
| `document_range_endpoint` | `kind, owner, documentRevisionToken, edge, point`；edge=`start|end`，point=`{line,column}` | D2 `document_range.startLocatorToken/endLocatorToken` |
| `resource_region` | `kind, resourceRef, resourceRevisionToken, regionToken` | D2 `resource_region.regionLocatorToken` |

element body逐字段等于本节DocumentElementLocator；resource body逐字段等于§6.3 ResourceRegionLocator。AuthorAnchorAddress没有`l1` token variant，也不得进入D2 link/citation/Annotation target；它只通过§6.4 typed symbolic address解析为新签发DocumentElementLocator。

### 6.2 DocumentRangeLocator

~~~json
{
  "kind": "document_range_locator",
  "owner": {
    "kind": "node_ref",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
    "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
  },
  "documentRevisionToken": "rev:opaque-d6-token",
  "sourceSpan": {
    "startLine": 8,
    "startColumn": 3,
    "endLine": 8,
    "endColumn": 11
  }
}
~~~

它只用于 exact text flow/range操作和 Annotation target。D2 range pair的唯一映射为：start token body必须`edge=start, point={startLine,startColumn}`，end token body必须`edge=end, point={endLine,endColumn}`；两个body的owner与documentRevisionToken必须byte-equal，且按D2 logical coordinate总序`start <= end`。该pair机械组成唯一`DocumentRangeLocator(owner,revision,sourceSpan)`；反向编码也只能产生这两个token。交换edge、混用另一owner/revision、用两个完整range body、padding或registry handle全部invalid，不进入stale判定。跨 revision 时必须 stale；D8 可以提供明确reanchor proposal，但成功后必须由Core按新revision/points签发新pair，旧token不被改写。

### 6.3 ResourceRegionLocator

~~~json
{
  "kind": "resource_region_locator",
  "resourceRef": {
    "kind": "resource_ref",
    "owner": {
      "kind": "node_ref",
      "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
      "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
    },
    "resourceId": "93d826e7-237d-47f6-9e14-04fe9cc65a13"
  },
  "resourceRevisionToken": "resource-rev:opaque-d6-token",
  "regionToken": "opaque-profile-region"
}
~~~

regionToken 的几何或格式 profile 由 D8/D9，D3 只冻结它必须绑定 exact ResourceRef + resource revision。D2 outer `resourceRef`必须byte-equal decoded token body.resourceRef；不等是`invalid_annotation_target`或适用copy wire的`owner_mismatch`。bytes改变即 stale；不得按相同像素、文件名或 digest fallback到另一 Resource。

token continuity规则闭合：same-Workspace continue只有在owner ref与D6 revision token都保持时token byte-equal；Node copy、Workspace fork、owner-changing Resource/Annotation copy以及ordinary export/import re-entry都产生fresh owner/revision并按同一normalized locator重新签发token，绝不复制旧token或维护隐藏registry mapping。exact snapshot export只携带token作为被导出bytes的一部分；再进入选择continue才可在continuity成立时保留，选择fork/import则按相应fresh identity规则重写。

### 6.4 AuthorAnchorAddress

~~~json
{
  "kind": "document_anchor_address",
  "owner": {
    "kind": "node_ref",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
    "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
  },
  "anchor": "acceptance"
}
~~~

AuthorAnchorAddress 是 authored symbolic locator，不是 entity identity。每次在当前 valid Document revision重新解析：

- 恰一匹配：`LocatorResolveOutcome(status=resolved)`必须在同一closed outcome中返回该revision新签发的`resolvedLocator: DocumentElementLocator`；
- 缺失：stale；
- duplicate 在 D2 本身 invalid，返回`invalid(reason=anchor_ambiguous)`，不返回任意一个；
- owner Document unavailable/trashed/tombstoned：传播 owner状态。

anchor 不跨 Document；同名 anchor 可存在于不同 Node。

### 6.5 Path、display label 与 source span

- D6 path/portable locator 只定位存储或 unmanaged inventory，不进入 identity equality。
- Node title、Resource occurrence caption/alt、文件名、UI breadcrumb、Server URL和collection label只用于显示或输入。
- 合法的Document title source edit、D6 path/file rename、D8 UI-only label变化及Resource occurrence caption/alt edit都不改durable identity；D3不提供可混淆这些authority的泛化rename operation。
- source span没有 owner+revision就不是 locator；即使加上也只是 exact-revision coordinate。
- API 不得接受 path-or-id、name-or-ref、anchor-or-uuid 等模糊 union。

### 6.6 D2 v2 exact-source、control/carrier spans 与 artifact binding

D2 v2 的完整 decoded exact source 仍是 Document 唯一作者权威；A+ Node control header、namespace-scoped lexical carrier、body、trivia与line endings都只是该同一 payload 的组成字节。D3/D6 envelope对`exact_source_document`绑定的subject、owner、current document revision与SHA-256必须覆盖从scalar 0到EOF的完整source，不能只hash body、control或carrier projection，也不能另存可写header/carrier sidecar。`AttributeCarrierBlock`与`LexicalAttributeEntry`属于同一source，不成为第二payload authority、EntityRef、D3 authored slot或独立revision subject；它们可作为Result/9 C的完整lexical载体，其中D4真实typed根接受受限模板物化，详见§4.1.3a。

A+ control spans、carrier block range、namespace range与entry raw source range必须绑定同一owner NodeRef、同一current Document revision与同一exact-source digest。它们只用于解析、diagnostic、repair与lossless evidence；D3不得为其签发`l1` locator token、AuthorAnchorAddress、cross-revision continuity或hidden registry identity。source edit后旧range只能stale/unavailable，不能按相同`namespaceToken`、`rawEntrySource`、FieldId、position或closest text复活。

| operation/artifact | binding requirement | forbidden inference |
| --- | --- | --- |
| same-Workspace Node copy | fresh NodeRef；copy完整exact source bytes并按既有identityMap规则重写真正的typed ref slots | carrier/entry不是新slot；相同raw entry不合并identity |
| Workspace fork | fresh Workspace/Node/owner-local refs；完整source digest与control/carrier bytes进入mapped result payload proof | 不保留旧owner的range/locator token或建立carrier registry |
| partial/ordinary import | fresh target identity；artifact manifest绑定输入bytes/hash，target full parse后签发新的current-revision ranges | 不从namespace/field spelling/source UUID导入identity |
| canonical export | artifact binding保留exact bytes/hash及control/carrier内容 | artifact digest不是Document/Node identity |
| plain/content-only export | 显式loss report；再进入只能fresh import | 丢失control/carrier后不得声称same canonical Node |

这些要求复用现有D3 payloadBindings、identityMap、rewrittenReferences与artifact evidence语义，不新增operation mode、reference union、symbolic segment或一般identity algebra。

## 7. Entity lifecycle

### 7.1 状态机

Node、Resource、Annotation 使用共同逻辑状态：

    unallocated
      -> live
          -> trashed
              -> live        (restore)
              -> tombstoned  (permanent purge)

tombstoned 是终态。ID 永不回到 unallocated，也不能 reassign。D3 选择严格两阶段模型：public `purge` 只接受 trashed target；live 对象必须先以独立 committed `trash` operation 进入 Trash，之后才能 purge。UI 可以连续引导两个动作，但不得把它们宣称为单一原子 commit，也不得跳过可观察的 trashed prestate。

reservation/burn属于allocator history，不是上述entity lifecycle：

    never_seen
      -> reserved(OperationId, requestFingerprint)
          -> live       (该OperationId committed)
          -> burned     (该OperationId terminal_failed)

burned永远不能进入live；它没有EntityResolveOutcome，authorized lookup表现为not_found。只有已经进入live的ID才可进入trashed/tombstoned。exact Workspace fork是唯一允许fresh result在首次外部可观察时已经为trashed的创建路径：同一原子commit必须先完成不可观察的`reserved -> live -> trashed`内部遍历并保留完整Trash/result lifecycle evidence，任何resolver或订阅者都不能观察中间live publication。ordinary create、Entity Copy、import/adopt/promote及其他mode若请求或返回fresh+trashed result均fail closed；不得把该fork-only规则泛化为`unallocated -> trashed`。

Document 跟随 Node 状态；没有独立 state machine。

Workspace 本身没有 Trash state。Workspace artifact 在未激活时只是 snapshot/copy evidence；激活必须选择continue或fork。销毁整个Workspace后没有当前authority负责保存内部tombstone；全部内部ref只报告workspace_unavailable且不泄露历史状态，不能复用其WorkspaceId创建同名新authority。

### 7.2 live

- ordinary read/query/action只处理 live且已授权对象；
- Node owner chain必须连接 root；
- Resource/Annotation owner必须 live；
- all mutations bind current authoritative revision/state and are stale-checked by D6。

### 7.3 trashed

- soft delete 原子移动 logical closure 到 Workspace Trash domain，但 Trash 不是 Node、不是 content graph、没有 NodeRef；
- Node subtree、其 Documents、Resources、Annotations保留原 refs；
- 每次trash operation保留一个规范trash graph：记录进入Trash的closure、内部parent/owner/reply边、Trash forest parent/ordinal和原结构位置。该graph只服务restore/purge/fork evidence，不成为第二content graph或identity authority。进入ordinary lifecycle计算前，stage 4必须先证明全部相关live sibling list与Trash sibling list的ordinal从0连续且唯一；损坏输入按§5.2 fail closed，绝不以`RefKey`修复。对已证明canonical的输入，新进入Trash的closure内部child顺序精确保留prestate live `siblingOrdinal`；新Trash roots在现有root之后append，同一operation的多个new roots按NodeRef canonical key排序（这里只排序多个合法selected roots，不修复已有ordinal）。若一个trashed parent在restore或purge中离开Trash而其child仍trashed，Core把每个surviving direct child原子splice到被移除parent在原siblings列表中的位置：保持child的prestate `trashOrdinal`顺序，最近surviving trashed ancestor成为trashParent，否则为null；多个removed parent按已验证的prestate sibling order同时展开，existing siblings相对顺序不变，所有受影响list压紧为0..n-1。禁止“一律重根到root”、按数据库枚举merge或产生special Trash Node。receipt用`node_trash_reparent`逐Node证明任何trashParent/trashOrdinal变化，包括仅ordinal shift。原结构位置是独立恢复提示：original parent仍有live/trashed identity时保留准确ref/ordinal；original parent已purged/tombstoned/not_found时固定成为`unavailable_original_location`，不得用path、label或tombstone猜parent；

多层同时离开Trash只使用以下规范递归；其前置条件是`children`已由stage 4证明按连续且唯一的prestate `trashOrdinal`形成canonical list，算法不得接受或归一化损坏输入。`removed`是本operation将restore或purge的Node集合，算法在完整prestate snapshot上一次求值，不按request数组顺序逐项修改：

~~~text
flatten(list, nearestSurvivingParent):
  out = []
  for node in list:                         # prestate order
    if node in removed:
      out.extend(flatten(node.children, nearestSurvivingParent))
    else:
      node.children = flatten(node.children, node)
      out.append(node)
  assign ordinals 0..len(out)-1 in out order
  assign every out node's parent = nearestSurvivingParent
  return out

newTrashRoots = flatten(prestateTrashRoots, null)
~~~

最小嵌套golden：roots=`[X,G,Y]`，G.children=`[P,Q]`，P.children=`[C0,C1]`，同operation removed=`{G,P}`；唯一poststate roots=`[X,C0,C1,Q,Y]`。receipt必须为C0、C1、Q列从各自old parent到null及new ordinal 1/2/3的`node_trash_reparent`，并为Y列ordinal 2→4；X parent/ordinal不变不列。若C0/C1内部还有removed parent，递归同式展开；不得得到`[X,Q,C0,C1,Y]`、`[X,Y,C0,C1,Q]`或按数据库枚举的任何次序。
- ordinary Query/View、Node tree、link navigation和default resolver不得把 trashed对象当 live；
- 具有 lifecycle/trash 权限的调用可得到 trashed状态与最小恢复信息；无权调用得到 not_visible；
- restore只能从准确 ref或trash operation item选择，不能按路径/标题猜测；
- Trash item ID是 operation/container ID，不是被删 entity identity。
- live内容指向trashed target的typed ref/locator称为suspended reference：它是可观察但不可普通导航/Action的合法状态，不是dangling；trash不得静默rewrite closure外的live Document，receipt必须枚举因此形成的referenceLifecycleTransitions。

### 7.3.1 已独立进入Trash的owner-local成员

trash本次进入closure只包含本次从live转trashed的对象。Node/subtree进入Trash时，把其当时live的Resources/Annotations及按原reply规则进入的live成员记录为该次owner恢复closure；此前已独立trashed的owner-local对象保持原独立Trash归属，不因owner相等或owner后来trash而合并、重标或再次产生lifecycle transition。保存的owner边表示所有权，不等同于恢复membership。既有独立trashed Node也不因旧original parent后来trash而自动并回新subtree。

仅restore owner Node按其当前规范Trash恢复membership恢复成员；此前独立trash的R/A继续trashed，不列成此次恢复或already_live/tombstoned omission。用户以独立R/A为restore primary且owner亦trashed时，仍按原owner_restore_location恢复所需owner closure并显式包含该primary及必需reply chain；不得借owner单独restore偷偷加入其它独立删除项。具体selectedObjects、原位置/owner/reply门与stage15完整闭包照常适用。

purge owner的不可逆owner闭包与restore membership分开：永久purge Node必须处理其全部仍trashed的owned Resource/Annotation，包括早先独立进入Trash者，完整列入所需selectedObjects/真实tombstone效果；既有tombstone按原规则保持，不再次分配或复活。不能保留一个owner已tombstoned而对象仍trashed的不可恢复半状态。只purge独立owner-local对象不purge owner，其closure外live引用仍须按原门显式处理。exact fork机械映射源cut已证明的Trash归属/恢复连接和所有owner/reply边，不按owner相等重新合并独立项；这些是原Trash graph控制事实，不新增EntityRef、公共operation mode或作者源。

最小序列：先独立trash R与无reply A，再trash N及当时live的R2/A2；restore N只恢复N/R2/A2，R/A保持trashed；之后显式restore R才恢复R。若改为purge N，R/A/R2/A2必须一同永久处理。每一步都有单一原OperationId、完整前后状态与实际receipt/effects，缺声明/权限/引用处理时whole operation失败。

### 7.4 restore

恢复规则：

- restore preserving identity；被恢复的Node/Resource/Annotation refs与删除前完全相同；
- restore一个Node时，closure恰为trash graph中仍处trashed、仍连接在该Node之下且可恢复的当前subtree。已经单独恢复为live的descendant保持其当前位置并从祖先trash closure脱离；已经tombstoned的descendant保持tombstoned并列入omittedObjects，二者都不构成identity_collision；
- trashed descendant在原parent仍trashed时可以单独restore，但调用必须给出显式live destination parent与ordinal；commit后它从原祖先trash closure脱离；
- 原parent仍live且位置可用时可恢复原parent/ordinal；否则调用必须显式给出live destination parent和ordinal。name/path/ordinal冲突可用显式新label/location/order解决，但不能rekey；
- restore request的location必须逐subject kind服从§4.1.3 exact matrix：Node使用original/explicit placement；owner已live的Resource/Annotation只用owner_local_no_placement；owner也需恢复时只用owner_restore_location表达唯一owner-root placement。location不得被忽略、套到owner-local对象本身或与structuralPlan重复表示同一Node placement；
- restore preflight验证恢复后的live graph没有dangling reference。指向仍trashed target的suspended reference可以保留并在receipt中报告；指向not_found、tombstoned、foreign或owner-mismatch target的live ref必须在同一plan显式rewrite/delete/include合法closure，否则whole operation fail closed；
- live inbound ref指向被恢复对象不阻止restore，成功后必须以lifecycle-only `suspended→resolved`及receipt `referenceLifecycleTransitions`显式证明；trashed source中的inbound ref不参与live graph阻断，但该source自身恢复时必须以`non_live_source→resolved|suspended`显式证明每个postimage live slot；
- Resource/Annotation单独恢复要求owner live；owner trashed时必须在同一plan恢复所需owner closure并由owner_restore_location唯一给出owner placement；owner tombstoned则不可恢复。owner restore不要求已purge的owned object复活；
- Annotation恢复要求其stored reply parent为live、同owner且无环，或在同一plan恢复完整reply chain/显式移除reply。parent missing/trashed/tombstoned且无合法plan固定stage 9 `entity_not_restorable`；parent live但compound plan造成cycle固定stage 10 `structural_cycle`；不使用二选一family。已单独恢复或purge的reply依上述规则留在当前状态；
- 同ref已经live但不属于同一trash item、同ref tombstoned、owner缺失、cycle、所需payload不完整或上述引用规则失败时whole restore fail closed；
- restore不从备份、export或另一Workspace偷取相同ref；那属于continue/fork/import规则。

### 7.5 permanent purge 与 tombstone

permanent purge：

- 非root对象只对trashed closure执行：live target固定`entity_not_live`，tombstoned target固定`entity_not_restorable`，不存在单operation live→tombstoned捷径。Workspace root不进入这些lifecycle分支；任何root purge在§13 stage 8先固定`root_operation_forbidden`；
- purge可以选择一个仍trashed的sub-closure；被purge成员从任何祖先trash closure移除并成为tombstoned，祖先以后restore时把它们列为omittedObjects而不复活；
- 删除 Document source、Resource bytes、Annotation body和其他 payload，无法通过 Weftext restore；
- 为 closure中每个 Node、Resource、Annotation发布最小 tombstone；
- tombstone只含 typed ref、kind以及 owner-local ref中已经自带的 owner；不得含 title、path、body、bytes、caption、author display、ACL、source span或秘密；
- tombstone在Workspace authority存续期间永久保留，阻止ID reuse并使authorized resolver区分known-purged与never-known；
- tombstone不是 live entity、Query row、Document、Resource bytes或Annotation；
- D6可压缩、索引或分片tombstone，但压缩后仍必须足以：(a)阻止typed ref复用；(b)在第9节授权矩阵允许时返回tombstoned与entity kind。最小规范事实是完整typed ref与entity kind；purge时间不是D3必需事实且不得因压缩而扩大披露。

### 7.6 Resource 与 Annotation 的独立 delete

- Node trash默认处理完整subtree。保留child需要在同一原子plan先move child到显式parent。closure外live inbound refs保留为suspended并在receipt枚举，不得隐式改写其Document。
- Resource trash允许owning Document/Annotation中的live occurrence暂时成为suspended；purge Resource前必须在同一原子plan显式删除/替换全部closure外live occurrence，否则inbound_reference_conflict。
- Annotation trash默认处理reply subtree。保留reply必须在同一原子plan先显式reparent并验证同owner、acyclic；reparent与parent trash/delete必须同成同败。
- 对任何Node/Resource/Annotation执行purge，都必须显式处理全部closure外live inbound refs；不得仅因方便把它们静默改写或留下dangling。
- 接受suggestion本身不隐式改变Annotation lifecycle；是否另行delete由D7决定，若选择delete则服从本节原子门禁。

## 8. Create、Move、Copy、Workspace Fork、Workspace Continue 与 Import

### 8.1 create

- 创建 Workspace：新 WorkspaceId、新 AuthorityInstanceId、新 root NodeRef。
- 创建 Node：Core生成新 NodeId，指定一个live parent和ordinal；Document在同一 commit创建。
- 创建 Resource/Annotation：Core在准确live owner内生成新local ID。
- create在stage 1–5失败不占用OperationId；create_workspace proposal在P1/P2失败也不route/read target ledger，tampered-first后genuine-second仍可继续；stage5 unseen请求在stage7–15失败写rejected但不产生content reservation。写入`planned`后的terminal failure不产生可解析live对象并burnreservation。只有当前authorization、authority availability与ledger continuity gate通过后，相同fingerprint/Workspace-local OperationId重试才交付保存结果。

### 8.2 同 Workspace move/reorder/rename

| 操作 | identity |
| --- | --- |
| Node reparent | NodeRef与descendant refs全部保持 |
| sibling reorder | refs保持，只改ordinal |
| Node title rename | refs保持 |
| D6 storage path rename | refs保持 |
| Resource bytes replace或文件名变化 | ResourceRef保持，resource revision变化 |
| Resource跨Node“move” | `copy_resource`到new owner + optional source `trash`；必得fresh ResourceRef |
| Annotation body/target edit | AnnotationRef保持，target locator可变化 |

move/reorder/compound Node placement的`destinationOrdinal`统一表示commit后final sibling index。算法唯一为：(1) 从各live source parent同时移除所有moving Nodes；restore Node没有live removal；(2) 按destination parent分组，组内声明final ordinal必须唯一且在该parent最终child count范围；(3) 把moving/restored Nodes放入这些final indexes，未移动children保持prestate相对顺序并依序填满其余indexes；(4) 对所有parent同时验证cycle、root、missing parent与continuous ordinals。数组输入顺序不参与结果。例：`[A,B,C]`把A移动到2得到`[B,C,A]`；把B声明到其final index 1是成功no-op且不产生`node_placement_change`。duplicate final index或越界固定stage10 `invalid_ordinal`。receipt必须列每个显式移动/恢复且parent/ordinal实际改变的operation subject，以及本operation显式表示的structural side effect；同一parent中仅因别的Node移动造成的unselected sibling compaction不进入`preserved`或`structuralChanges`，但算法可由prestate+listed moves机械重算。`[A,B,C], A→2`的canonical receipt只列A与A的唯一`node_placement_change`；把B/C也列入必须拒绝。path只是D6执行影响。

### 8.3 Entity Copy

规范API只有copy_node_subtree；CLI/UI只使用Lexicon控制的copy/复制标签。Workspace分叉只使用workspace fork/分叉工作区，不能与copy做成同一identity模式；[D3-TERM-EXCLUDE:migration_deletion_note]`clone|duplicate`均是退役alias。

同 Workspace copy：

1. source root与所有被纳入成员必须live。默认closure是选中live Node subtree及其当前live Resources/Annotations；trashed成员不属于copy closure，不能借copy复活；
2. 为closure中每个Node、Resource、Annotation生成fresh ID；
3. 建立完整typed oldRef -> same-kind newRef identityMap。owner-local entry的`from.owner`是原owner，`to.owner`必须等于同map中映射后的owner；
4. closure内容中的每个typed ref/locator位置默认执行internal-rewrite/external-preserve：target在identityMap内则重写；target在closure外但仍是当前Workspace的合法live/trashed target则保留；foreign/not_found/tombstoned/owner-mismatch target必须显式transform/omit/reject。唯一例外是下一条的owner-derived `r1` slot，它不能跨owner external-preserve；
5. copied Document中的Node link/citation依上条处理。`r1.RESOURCE_UUID`由所在exact-source Document推导owner，因此任何owner-changing Node copy中：source Resource在identityMap内时必须重写为destination owner下的新ResourceRef；source Resource不在map内（包括trashed Resource被copy closure排除）时必须在同一plan显式删除/替换该occurrence或拒绝whole copy。不得原样保留token、不得产生跨owner含义，也不得因destination owner恰有相同ResourceId leaf而静默rebind；
6. Annotation生成fresh AnnotationRef；target locator按转换后的target revision重新签发。subtree copy中无法唯一重建者只能按`omittedObjects`显式omit，且omit必须沿reply descendants闭合，除非先在同一plan显式reparent并验证同owner、acyclic；standalone `copy_annotation`不得把请求subject omit为committed no-op，无法重建必须whole operation拒绝；
7. reply关系按identityMap重写；
8. D4未来关系必须逐字段显式选择是否覆盖默认规则；D3不替D4定义关系payload；
9. commit receipt必须返回完整identityMap、rewrittenReferences与omittedObjects。

copy不是identity延续，即使source bytes完全相同。D2 v2 control/carrier/entry bytes属于同一exact-source payload并随copy/fork的payload proof传递；D2/D3 authored slot按identityMap重写，D4已知typed根与关系carrier通过§4.1.3a C物化，carrier/entry range本身绝不进入`rewrittenReferences`。

owner-local copy 规则：

- `copy_resource`要求准确live source ResourceRef与显式live destination owner；无论同owner还是跨owner都生成fresh ResourceRef和fresh target resource revision。它只复制所选payload，不自动改写任何occurrence。请求可以在**同一atomic plan**显式修改零到多个existing container，但每个container必须是target Workspace中由destination owner拥有的Node exact-source Document或Annotation Value/3，且必须有同一`existing_entity_subject`的准确preimage/result payload pair、逐slot plan/result evidence，并在receipt `preserved`中出现。same-owner `R→R'`因而可原子改写owner Document中的`r1.R`；destination owner不同或source Workspace foreign时，source/foreign-owner Document绝不属于可修改container，不能把new ResourceId写回foreign owner。未显式绑定的occurrence保持原样；source trash仍是独立`trash`operation。
- `copy_annotation`要求准确live source AnnotationRef与显式live destination owner，并生成fresh AnnotationRef。请求source必须恰出现一次于identityMap.from，且不得出现在omittedObjects；target locator必须针对destination owner当前revision唯一重建并由Core重新签发，不能重建则whole copy拒绝。复制reply时，reply parent必须同时在identityMap内、已经是destination owner的live Annotation，或在同一plan显式reparent并验证同owner、acyclic；不能把source owner的reply ref或locator原样带入。
- `copy_resource`请求source同样必须恰出现一次于identityMap.from且禁止omit；两种standalone owner-local copy均不得提交空identityMap或omitted-only no-op receipt。
- 两种owner-local copy都返回same-kind typed identityMap；`to.owner`必须等于显式destination owner，不存在reowner-in-place。对copy_annotation，同plan existing-container edit服从相同target-Workspace/destination-owner/preimage-result规则；其structural reply side effect还必须有唯一S与structuralChanges。任何请求修改foreign Workspace、非destination owner或未带成对revision evidence的container固定stage2 `workspace_ref_misbound`或stage1 `invalid_request_matrix`，不得由实现自行扩张matrix。

### 8.4 跨 Workspace transfer

跨 Workspace不存在保留NodeRef的move。唯一合法语义：

    target fresh-identity copy
      + verified identityMap/ref rewrite
      + optional source soft delete in a separate authorized commit

两个Workspace之间没有D3原子提交保证；UI不得在target成功、source失败时声称原子move。每个结果必须有独立receipt。

target必须为每个Node/Resource/Annotation生成fresh ID和完整ref map。transfer closure中的全部typed ref与locator位置——包括Node link/citation、Resource occurrence、完整ResourceRef/AnnotationRef、Annotation target、Document range与Resource region locator——若仍指向source Workspace且target不在identityMap中，必须在preview中：

- 扩大include closure；或
- 通过显式Document source transformation删除/替换；或
- reject。

不得把任何foreign-Workspace typed ref/locator保留为可提交D2内容，也不得自动转为HTTPS、path或title link。source soft delete是独立trash operation，必须服从第7.6/10.1节；只有其receipt committed后才可显示“source removed”。target成功但source未尝试/失败时必须显示“copied; source not removed”，并由第12节non-atomic transfer outcome关联两个OperationId。

### 8.5 Workspace continue 与 fork

完整Workspace snapshot、backup、raw copy或export重新进入时必须显式选择：

#### continue

- 保留WorkspaceId和全部Node/Resource/Annotation refs/tombstones；
- 只用于已知同一逻辑Workspace的exclusive continuation、failover或disaster recovery；
- 新物理位置取得新的AuthorityInstanceId，但该ID不来自artifact或caller；只可由Core按§4.1.3 planning CAS fresh reserve，commit激活，terminal failure永久burn；
- snapshot必须包含一个D6可验证的continuation cut，并完整包含截至该cut的allocation、burn、live/Trash、tombstone与transaction事实。D6还必须证明旧authority已停止且该cut之后不存在已提交事实，或取得足以合并这些事实的authoritative ledger；否则不能以同一WorkspaceId继续写，只能fork或只读/reconciliation；
- 因此continue不得让已经发布的tombstone重新变live，也不得丢失cut之前的burn/allocation history；无法证明“备份之后没有提交”的stale backup不能continue；
- continue不由root NodeId、path、digest、文件名或“最近备份”猜测。

#### fork

- 新WorkspaceId、新AuthorityInstanceId；
- fork closure包含snapshot中全部live与trashed Node/Resource/Annotation；为每个成员生成fresh local IDs并保留其live/trashed状态；
- fork request的`resultLifecyclePlan`逐member承诺准确source两态；live Node只在`resultPlacementPlan`进入new live tree，trashed Node只在`resultTrashPlacementPlan`进入canonical Trash forest，二者严格分区。exact fork逐Node复制source cut已经规范化的trashParent/trashOrdinal，不重新merge或按payload枚举；trashed Node的当前trashParent若仍trashed则映射，否则source cut中的canonical parent已为null。original parent仍可映射时以`mapped_original_location`保存，已purged/tombstoned/not_found时必须以`unavailable_original_location`保存，恢复只能要求显式live destination，绝不自动挂到Workspace root；
- 对fork payload中的全部typed ref与locator逐一处理：target在live/trashed identityMap内则重写；指向source tombstone、not_found、owner-mismatch或closure外foreign target必须显式transform或whole fork拒绝，不能omit任何source live/trashed member，也不能提交foreign或dangling target；
- 重写全部owner-local tokens、anchor owner与可唯一重建的revision locator；任何Annotation target或reply无法在target唯一、同owner、无环重建时，whole fork拒绝；
- 旧tombstone不作为新Workspace已分配ID历史直接复制；只可作为非authoritative provenance摘要；
- 返回完整identityMap；`identityMap.from`恰等于snapshot全部live/trashed Node/Resource/Annotation，`omittedObjects`必须为空；receipt的`resultLifecycles`逐mapped target证明保留两态，`initialPlacements`与`trashPlacements`分别证明live tree与Trash forest且对mapped Node精确分区；root及任何成员都不可omit；
- fork后的Workspace与source没有共同提交权威。

Workspace fork是唯一分叉名称与identity模式；Node层使用copy，不使用clone/duplicate。

### 8.6 import/export re-entry

re-entry mode只由调用方显式选择的闭合artifact class决定，绝不从path、digest、title、root NodeId、同名UUID或“看起来是备份”推断：

| artifact class | 允许mode | identity规则 |
| --- | --- | --- |
| formal workspace backup snapshot | continue_workspace 或 fork_workspace（二选一） | continue满足§8.5才保留；fork全部fresh |
| full-workspace identity-bearing snapshot | fork_workspace | 必须含唯一root、完整live/trashed tree、全部owner/reply closure和可验证snapshot cut；target全部fresh |
| partial identity-bearing transfer bundle | 仅`import_new + partial_identity_bearing_transfer_bundle` | 不能使用在线`copy_node_subtree`、不能fork或把subtree root隐式提升为Workspace root；artifact manifest/source_artifact bindings和source cut是唯一preimage evidence，source typed refs只作provenance/rewrite输入，target全部fresh |
| ordinary format export / UnmanagedItem / Import IR / worker output | import_new | fresh分区全部fresh；existing分区只按§4.1.3a受信SourceBinding/OriginBinding及显式existingPayloadEdits保持身份；至少一个fresh Node root，全existing batch走D6；IR ID不能成为content ID |
| 当前Workspace authority中的准确Trash对象或trashed typed ref | restore | 仅§7.4允许preserve；它不是export artifact |

所有artifact class都必须把D2 v2完整exact source（含A+ control与namespace-scoped carriers）作为一个hash-bound payload处理；canonical export保留exact bytes/hash，ordinary/plain re-entry分配fresh identity并重新建立current-revision ranges。carrier/entry不能作为artifact member identity、dedupe key或symbolic ref slot。

unknown artifact class/mode组合拒绝。`copy_node_subtree`只表示在线、可授权且可取得authoritative source Workspace cut的copy；离线bundle绝不能进入该variant。`fork_workspace`可先持有未claim proposal，但必须在target Workspace成为allocated entity、proposal claim与ledger planned之前验证full-workspace closure；partial bundle用于fork固定`domain_kind_mismatch`且不claim/allocate。普通Document/resource export不携带可主张target identity的能力；再次进入source Workspace仍是copy/import_new。restore document history到同一live Node可以保留NodeRef；Restore as new Node等于copy/create并分配fresh identity。

## 9. 引用解析

三个resolver outcome都是closed JSON tagged union。所有variant只允许本节列出的字段，unknown/missing/duplicate/null拒绝；`wireVersion`固定11且canonical object key order只服从D3-CJ/3。成功decode后的variant必须回显调用方已经提供的canonical ref/locator；decode失败的`invalid`不得回显未成功decode的raw payload，只有成功decode后判定的`anchor_ambiguous`按§9.2回显canonical AuthorAnchorAddress。

`WorkspaceResolveOutcome`的`kind`固定`workspace_resolution`：

| status | required fields | forbidden fields | 语义 |
| --- | --- | --- | --- |
| available | `wireVersion, kind, status, requestedRef: WorkspaceRef` | `reason` | 当前authority可用且允许披露 |
| not_visible | 同上 | `reason` | `mayDiscloseWorkspaceState=false`；不披露可用性 |
| workspace_unavailable | 同上 | `reason` | 已销毁、未挂载、不能取得或authority/integrity冲突；不细分原因 |
| invalid | `wireVersion, kind, status, reason` | `requestedRef` | WorkspaceRef decode失败 |

Workspace没有`trashed`、`tombstoned`或`not_found`结果。顺序固定为closed decode → `mayDiscloseWorkspaceState` → authority availability/integrity。两个AuthorityInstance冲突、live+tombstone collision、duplicate record或无法证明唯一authoritative projection时，普通resolver在有披露权后统一`workspace_unavailable`，不得选winner；具体reconciliation事实只经D6独立受权repair channel暴露，不进入D3普通resolver wire。

### 9.1 EntityResolveOutcome

resolver绑定一个准确Workspace authority与D6 authorization context。D3不冻结ACL角色，但冻结一个供本API使用的workspace/ref-domain scope谓词`mayDiscloseEntityState`：只有当D6允许调用方一致区分live/trashed/tombstoned/never-known四态时才为true；内容snapshot读取仍可由更窄的D6权限单独控制。

`EntityResolveOutcome.kind`固定`entity_resolution`。`resolved | trashed | tombstoned | not_found | not_visible | workspace_unavailable`每个variant required字段恰为`wireVersion, kind, status, requestedRef`，其中requestedRef是成功decode的NodeRef、ResourceRef或AnnotationRef；`reason`禁止。`invalid` required字段恰为`wireVersion, kind, status:"invalid", reason`，`requestedRef`禁止。snapshot、Trash metadata和repair detail若获授权必须由独立envelope返回，不能作为本outcome的可选字段。

| status | 语义 |
| --- | --- |
| resolved | typed ref存在、live、允许披露entity state且kind/owner匹配；不自动附带内容读取权 |
| trashed | known recoverable但非live；只在允许披露entity state时返回 |
| tombstoned | known permanently purged；只在允许披露entity state时返回最小tombstone |
| not_found | well-formed ref在当前Workspace分配历史中未知；只在允许披露entity state时返回 |
| not_visible | authorization/policy要求遮蔽存在性；不得附带path/title/kind detail |
| workspace_unavailable | ref指向的Workspace不是当前bound authority或当前authority不可获得；不自动route/fetch |
| invalid | ref closed decode失败；只带closed reason，不回显raw input |

示例：

~~~json
{
  "kind": "entity_resolution",
  "requestedRef": {
    "kind": "node_ref",
    "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283"
  },
  "status": "tombstoned",
  "wireVersion": 11
}
~~~

授权矩阵闭合为：

| `mayDiscloseEntityState` | authoritative state | outcome |
| --- | --- | --- |
| false | live / trashed / tombstoned / never-known | not_visible |
| true | live | resolved；若另有content-read权限，snapshot只能通过独立closed content envelope另行返回，本outcome字段不变 |
| true | trashed | trashed；若另有trash-detail权限，恢复metadata只能通过独立closed trash-detail envelope另行返回，本outcome字段不变 |
| true | tombstoned | tombstoned；只带第7.5节最小事实 |
| true | never-known、burned reservation、reserved/planned但尚未commit的proposed ID | not_found；不得暴露reservation、OperationId或planned状态 |

D6不得提供“可见never-known但不可见任何allocated state”的混合能力给该resolver API；否则会形成existence oracle。任何authorized snapshot、Trash detail或tombstone审计事实使用独立envelope；本outcome不带identity之外的事实。

### 9.2 LocatorResolveOutcome

`LocatorResolveOutcome.kind`固定`locator_resolution`，`wireVersion`固定11。required/forbidden字段闭合为：

| variant | required fields | forbidden fields |
| --- | --- | --- |
| `resolved`且requestedLocator.kind=`document_anchor_address` | `wireVersion, kind, status, requestedLocator, resolvedLocator: DocumentElementLocator` | `reason`；resolvedLocator.owner必须等于requestedLocator.owner并绑定当前Document revision，elementKind/span必须是唯一anchor结果 |
| `resolved`且其他locator kind | `wireVersion, kind, status, requestedLocator` | `reason, resolvedLocator` |
| `trashed | tombstoned | not_found | not_visible | workspace_unavailable | stale` | `wireVersion, kind, status, requestedLocator` | `reason, resolvedLocator` |
| `invalid(reason=anchor_ambiguous)` | `wireVersion, kind, status, reason, requestedLocator: AuthorAnchorAddress` | `resolvedLocator` |
| 其他`invalid` | `wireVersion, kind, status, reason` | `requestedLocator, resolvedLocator` |

`resolvedLocator`使用第6.1节DocumentElementLocator canonical encoding，没有数组或额外evidence；同一authoritative revision和anchor index必须签发byte-equivalent locator。无`mayDiscloseLocatorState`时在anchor index lookup之前返回not_visible，禁止resolvedLocator。anchor missing固定stale；duplicate固定invalid(anchor_ambiguous)；两者都不得签发locator。locator额外允许status=stale：

- locator内嵌的owner/entity ref先按§9.1传播resolved/trashed/tombstoned/not_found/not_visible/workspace_unavailable/invalid；只有owner/entity resolved且live后才可能判断stale；
- owner/ref当前可见且live，但绑定Document/Resource revision不等于当前revision；
- revision相同但exact kind/span/region无法验证；
- anchor当前缺失；
- Annotation原target的owner/entity仍live但revision/coordinate不能精确解析。

在owner/entity resolved之后、任何current revision、coordinate、anchor、elementKind、span或region lookup之前，必须执行独立`mayDiscloseLocatorState` gate。false时统一`not_visible`并停止；不得由`mayDiscloseEntityState`、content-read或mutation权限隐式推导此谓词，具体ACL映射由D6。因而entity state可见但locator state不可见时，真实resolved、revision mismatch、anchor missing/duplicate、wrong span和wrong region均不可区分。

stale不自动fuzzy reanchor、不返回nearest match。D8可请求独立reanchor proposal；成功后产生新locator和明确evidence，失败/ambiguous保持stale。

### 9.3 固定失败关闭顺序

1. 闭世界decode：wireVersion、字段集合/JSON type、kind、expected union、owner shape、UUID/token与locator lexical/range shape；失败按§9.4唯一返回invalid。
2. Workspace binding：NodeRef.workspaceId以及ResourceRef/AnnotationRef/ResourceRegionLocator内嵌owner.workspaceId都必须等于当前resolver authority；否则返回workspace_unavailable，不访问外部网络/路径。
3. D6 entity-state disclosure gate：`mayDiscloseEntityState=false`时返回not_visible，并停止存在性、Trash、tombstone或owner探测。
4. authority/integrity gate：duplicate writer、duplicate typed record、active+tombstone collision、missing owner或无法形成唯一authoritative projection时统一workspace_unavailable；repair detail只进入独立受权channel。
5. 若调用面提供ambient expected-owner/slot-owner，在不做存在性查询的前提下，只比较已decode请求bytes中的encoded owner；不相等固定owner_mismatch并停止，不做跨owner候选搜索。普通resolver没有ambient expected-owner，跳过此步。
6. exact typed-key lifecycle lookup：以完整NodeRef或(owner NodeRef, localId)查询，不做跨owner候选搜索；返回resolved、trashed、tombstoned或not_found。已定位存储记录若与typed key/encoded owner不一致属于authority/integrity损坏，普通resolver投影workspace_unavailable，operation返回workspace_integrity_conflict，不是owner_mismatch。
7. locator-state disclosure gate：locator请求在owner/entity resolved后执行`mayDiscloseLocatorState`；false统一not_visible且不访问current revision/coordinate/anchor/region。
8. Locator revision与exact coordinate验证：revision/token equality由D6提供canonical比较；missing或mismatch返回stale；anchor duplicate返回invalid(anchor_ambiguous)。

UI、CLI、Server、Mobile不得改变顺序或用path/title/search/previous cache补救。

### 9.4 invalid reason

invalid.reason闭合且由同一pre-discriminant decoder总序选择：

1. `invalid_closed_shape`：raw value非JSON object、duplicate key；所有closed union共同discriminant `kind` missing/null/非string；有实际`wireVersion`字段的D3 envelope中，该字段missing/null/非数学整数或超出D3Integer。此步只检查共同envelope，不检查未知variant的其他字段。
2. `unsupported_wire_version`：只适用于实际携带`wireVersion`的request/receipt/resolver outcome/error/proposal/allocation outcome/transfer envelope；wireVersion先通过D3Integer decode后不等于11。NodeRef/locator本身没有wireVersion字段，此reason对它们不成立；不得把token prefix称为wireVersion。
3. `unsupported_ref_kind`：kind string不是当前expected closed union已知variant。命中后停止，忽略其他variant-specific extra/missing字段的reason选择；例如`{"kind":"future_ref","extra":1}`唯一为unsupported_ref_kind。
4. `invalid_closed_shape`：kind已知后，按该variant检查missing/extra/null/JSON type或数组duplicate key。对非locator且非wireVersion的已声明D3Integer成员，本步只要求值是JSON number；Boolean/string/null等错误JSON类型仍在本步失败，number的负数、浮点/指数词法与范围只由第9项专门判定，不被本步一般shape规则抢先归类。wireVersion保留第1/2项，完整Locator/l1保留第10项专门规则。
5. `domain_kind_mismatch`：kind已知且自身shape合法，但放入错误expected union。
6. `invalid_owner_shape`：owner字段kind正确但其nested NodeRef closed shape非法。
7. `malformed_identifier`：前述通过后UUID不为canonical lowercase UUID。
8. `malformed_token`：D2冻结的`n1/r1`外层token grammar不成立，或任一D6 opaque token是空string。D6 revision/generation/region/profile token只要是非空JSON string就lexically valid；`rev:α`合法，D3不再声称存在未冻结的允许字符集。`l1`不进入此reason：其prefix、alphabet、padding、base64url decode、canonical re-encode、magic、NUL、D3-CJ body、variant及range静态不变量的任一失败都唯一归入下一项`invalid_locator_shape`。
9. `invalid_integer_shape`：非locator的D3 integer字段为负数、浮点/指数、超出`0..2^63-1`或不能无损解析。operation固定映射`invalid_identity_ref`；wireVersion使用第1/2项专门规则。
10. `invalid_locator_shape`：无需访问Workspace即可判定的locator负坐标、浮点、超出D3Integer、start>end或其他静态locator invariant失败；也包括全部`l1` outer/static decode failures及body内nested owner/ref/UUID lexical/closed-shape失败。对任何`l1`输入，第6项`invalid_owner_shape`、第7项`malformed_identifier`与第8项`malformed_token`都被本项专用decoder reason遮蔽。
11. `invalid_request_matrix`：v11 request元素本身closed，但只依赖request bytes的per-mode shape/empty/cardinality、duplicate equality、expectedAuthority shape或action冲突。Workspace role不属于此reason；authoritative payload/slot/object coverage也不属于此reason。
12. `anchor_ambiguous`：只在成功decode/binding/authorization/authority/entity/locator disclosure且当前anchor lookup为duplicate时成立。

current-revision semantic invalidity与stale边界固定为：locator已decode、owner live、revision匹配，但ReferenceSlotAddress声明slotKind与该span唯一occurrence不符，operation为`invalid_locator`；duplicate anchor为`invalid(anchor_ambiguous)`。revision/annotationRevisionToken不匹配，或revision相同但element/span/region在当前exact source中不存在，resolver为stale、operation为`stale_locator`。不得改报invalid_locator_shape。

到operation primary family映射固定为：`domain_kind_mismatch → domain_kind_mismatch`；`anchor_ambiguous → invalid_locator`；`invalid_request_matrix`及其余decode reasons → `invalid_identity_ref`。一个输入不得同时报告reason/family数组。golden：known variant的非Locator D3Integer成员使用1.0或1e0→invalid_integer_shape；同成员使用true或字符串→invalid_closed_shape；若同时extra member则第4项仍先于第9项。wireVersion使用1.0仍走第1项，Locator坐标使用1.0仍走第10项；unknown kind+extra → unsupported_ref_kind；known kind+extra → invalid_closed_shape；`start=9,end=3` → invalid_locator_shape/invalid_identity_ref；空opaque token → malformed_token/invalid_identity_ref；`rev:α`通过lexical后才可resolved/stale；same-revision wrong slotKind → invalid_locator；旧revision/Annotation token或不存在span → stale/stale_locator。

## 10. 悬空引用、损坏和可见性

### 10.1 authoritative live graph

引用状态定义：

- suspended reference：live source指向准确typed ref/locator，但target当前trashed；这是合法可恢复状态，resolver返回trashed，普通navigation/Action fail closed；
- dangling reference：live source指向not_found、tombstoned、foreign-workspace、owner-mismatch或无法closed-decode的target；默认非法。

D3默认不允许新commit制造dangling live D2 reference：

- trash/move/copy/restore/purge preflight必须枚举受影响的全部D2 Node link、citation、Resource occurrence和Annotation target；
- trash可以让既有live inbound ref变为suspended，但必须在receipt列出，不能隐式rewrite closure外Document；
- copy/restore若产生dangling必须同时rewrite/delete/include required closure或reject；
- purge必须在同一原子plan显式处理全部closure外live inbound ref，否则reject；
- Core不能以允许broken link为方便跳过D2 gate。

若外部损坏、partial sync、旧缓存或未经Core字节修改造成missing owner/target：

- Workspace进入D6 repair/reconciliation；
- exact source按D2 repair surface暴露；
- 不返回old/partial projection；
- 不自动创建placeholder Node、orphan folder、Resource或Annotation；
- 普通Workspace/Entity/Locator resolver在授权gate后统一投影`workspace_unavailable`，直到D6恢复唯一authoritative projection；不得把冲突状态伪装成not_found/tombstoned/stale。具体损坏类型、候选记录与修复动作只进入独立受权repair channel。

D4以后必须为每个ref-valued关系字段显式决定live-only、suspended/tombstone-aware或其他closed resolution policy；在D4冻结前，D3不替任何尚不存在的D4字段选择默认delete policy，也不得把所有ref字段默认为allow-dangling。

### 10.2 observable state 与信息遮蔽

- tombstoned表示当前Workspace authority知道该typed ref曾存在且已永久purge。
- not_found表示`mayDiscloseEntityState=true`的resolver确认从未分配、只有burned reservation或没有规范entity记录。
- not_visible表示不能披露是否存在；调用方不得据此猜测not_found。
- stale只属于locator/evidence/handle，不属于durable entity ref；NodeRef本身不会因Document revision变化而stale。
- workspace_unavailable不表示目标不存在；只表示当前authority不能解析。
- trashed不是resolved；普通navigation/action必须拒绝，恢复入口除外。
- 只有owner/entity先按§9.2–9.3解析为resolved/live后，`mayDiscloseLocatorState=false`才使locator outcome为not_visible；不得用resolved/stale/anchor_ambiguous泄露Document或Resource revision/coordinate状态。较早的已授权trashed/tombstoned/not_found等结果仍按原总序传播，不读取位置。
- 离线cache/replica投影必须标记`non_authoritative`并携带snapshot revision与authorization generation；它可以用于展示，但不得冒充本节outcome或作为mutation证据。commit前Core必须重新执行第9.3节。

## 11. 非领域 identity classes

| 类 | 最小D3门禁 |
| --- | --- |
| LogicalOccurrenceKey | 必须绑定semantic major、query invocation namespace和operator/occurrence path；只在一次D7 execution closure比较；不得持久到Document/View或跨query invocation暴露 |
| ResultRowHandle | opaque、Core签发；绑定WorkspaceId、result snapshot/revision、authorization generation、row occurrence；分页/重启/权限变化可失效；不得构造EntityRef |
| Provenance | 可以包含typed refs、revision locators、transform steps；仅解释，不授权、不建立ownership、不改变各成员lifetime |
| ActionEvidence | 由Core从authorized result与当前state重新签发；绑定actor/auth generation、exacttarget ref/locator、revision、action kind、expiry；一次性/窄scope；不得长期持久 |
| OperationId | idempotency和执行关联；不是entity，不能resolve成Node/Resource/Annotation |
| Commit/TransactionId | D6 evidence；不进入content ref |
| HistoryEntry/Snapshot/Repository/LineageId | history/backup evidence；不能作为WorkspaceId或NodeRef |
| AuditEventId | D10/D6 event identity；不赋予内容读取或write capability |

转换禁令：

- locator -> EntityRef：禁止；
- ResultRowHandle -> EntityRef：只有handle中显式公开的typed EntityRef可原样读取；handle自身不可coerce；
- Provenance -> ActionEvidence：禁止客户端转换，必须向Core重新签发；
- ActionEvidence -> EntityRef：只能读取它明确绑定的target ref，不能扩大scope；
- event/snapshot/operation ID -> content ref：禁止；
- foreign UID/RECURRENCE-ID/ForeignIdentityKey/SourceBinding/OriginBinding -> EntityRef：禁止；只有OriginBinding中显式保存的Weftext typed ref可原样读取，foreign side自身不可coerce；
- bare UUID -> any ref：禁止。

## 12. 最小操作 wire 与 receipt

D3不冻结D6事务transport，但任何分配、copy/fork、identity-preserving结构变化、trash/restore/purge操作必须产生closed `identity_change_receipt`。示例：

~~~json
{
  "wireVersion": 11,
  "kind": "identity_change_receipt",
  "operationId": "bc550154-f436-4f5f-adcb-7458df568d34",
  "mode": "copy_node_subtree",
  "sourceWorkspaceRef": {
    "kind": "workspace_ref",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283"
  },
  "targetWorkspaceRef": {
    "kind": "workspace_ref",
    "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283"
  },
  "identityMap": [
    {
      "from": {
        "kind": "node_ref",
        "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
        "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
      },
      "to": {
        "kind": "node_ref",
        "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
        "nodeId": "8e4c0c8e-eed4-4dcc-a154-05fed18d4e18"
      }
    }
  ],
  "allocated": [
    {
      "kind": "node_ref",
      "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
      "nodeId": "8e4c0c8e-eed4-4dcc-a154-05fed18d4e18"
    }
  ],
  "resultAllocations": [],
  "resultLifecycles": [],
  "initialPlacements": [
    {
      "subject": {
        "kind": "mapped_result_subject",
        "sourceRef": {
          "kind": "node_ref",
          "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
          "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
        }
      },
      "nodeRef": {
        "kind": "node_ref",
        "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
        "nodeId": "8e4c0c8e-eed4-4dcc-a154-05fed18d4e18"
      },
      "parentRef": {
        "kind": "node_ref",
        "workspaceId": "9dfadab9-5f7a-4ba9-a6d4-681b29645283",
        "nodeId": "c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"
      },
      "ordinal": 0
    }
  ],
  "trashPlacements": [],
  "preserved": [],
  "tombstoned": [],
  "rewrittenReferences": [],
  "referenceLifecycleTransitions": [],
  "structuralChanges": [],
  "omittedObjects": [],
  "result": "committed"
}
~~~

receipt对象是closed wire：`wireVersion`固定为11，`kind`固定为`identity_change_receipt`，`operationId`为canonical UUID，`mode`来自下列闭集，`targetWorkspaceRef`必需，`result`固定为`committed`。任何rejection/failure都不生成receipt：stage1–4、proposal P1/P2、TL1/TL2与stage5 conflict返回preflight error；stage5 unseen后的stage7–15失败记录rejected error；已planned authoritative abort记录terminal_failed error。所有十二个数组字段`identityMap, allocated, resultAllocations, resultLifecycles, initialPlacements, trashPlacements, preserved, tombstoned, rewrittenReferences, referenceLifecycleTransitions, structuralChanges, omittedObjects`即使为空也必须出现。

- `sourceWorkspaceRef`在`copy_node_subtree, copy_resource, copy_annotation, fork_workspace, continue_workspace`中必需；same-Workspace copy与continue必须等于target，fork必须不等于target，跨Workspace copy必须不等于target。`import_new`只在`artifactClass=partial_identity_bearing_transfer_bundle`时必需；ordinary-format import中禁止。其他mode禁止。
- `destinationOwnerRef`只在`copy_resource`与`copy_annotation`中必需，必须是target Workspace内显式live NodeRef；其他mode禁止。
- `issuerAuthorityInstanceId`与`targetAuthorityInstanceId`只在`create_workspace`、`fork_workspace`必需，必须分别byte-equal request.workspaceProposal同名字段；target字段表示commit后target Workspace初始active authority。`authorityInstanceId`只在committed `continue_workspace` receipt必需，必须byte-equal该OperationId的planned ContinueAuthorityReservation；request/intent中出现该字段固定stage1拒绝。三者都是canonical UUID控制字段，不是content ref，不能进入任何identity array/map；任一跨角色借用或额外出现拒绝。
- `continuation`只在`continue_workspace`必需，closed为`{"workspaceRef": <WorkspaceRef>, "continuationCutToken": <non-empty opaque D6 token>}`；workspaceRef必须等于source/target WorkspaceRef。receipt表示Core已验证该cut，布尔`verified`字段反而禁止。
- `artifactClass`只在`import_new`必需，闭合为`ordinary_format | partial_identity_bearing_transfer_bundle`；full-workspace snapshot必须使用`fork_workspace`，不能编码为import_new。
- `allocated`元素为WorkspaceRef、NodeRef、ResourceRef或AnnotationRef；精确列本commit在target中新分配并成为live/trashed authoritative object的refs。它只是canonical集合，不承载ordinal映射。`create_workspace`恰列target WorkspaceRef与该graph全部resultAllocations.ref；copy/fork/identity-bearing import恰列identityMap.to，且fork额外列target WorkspaceRef；ordinary import及其他create恰列resultAllocations.ref；不得漏列或重复。Document没有可列ref。
- `resultAllocations`元素closed为`{"subject":<new_result_subject>,"ref":<same-kind NodeRef|ResourceRef|AnnotationRef>}`；每个request freshSubjects declaration恰一entry、不得有undeclared entry、ref唯一、按subject canonical key排序。Node subject禁止ownerSubject；Resource/Annotation subject必需ownerSubject，且receipt ref.owner必须byte-equal该ownerSubject经同receipt identityMap/resultAllocations或existing subject机械物化的NodeRef。ordinary import验证者用canonical request中的ArtifactObjectKey→subject bijection机械关联每个ref；create验证者用role=primary/member declaration关联。无owner allocation、owner swap、从`allocated`顺序猜owner、artifact/provider顺序分配或相同digest合并subjects一律拒绝。它是brand-new subject到fresh typed ref的唯一映射；identityMap只服务mapped_result_subject。
- `resultLifecycles`元素closed为`{"subject":<mapped_result_subject>,"ref":<NodeRef|ResourceRef|AnnotationRef>,"state":"live"|"trashed"}`；只在fork非空，必须逐项由identityMap把subject.sourceRef映射到同一ref，并与request resultLifecyclePlan准确相等；它恰覆盖identityMap.to且按subjectCanonicalKey排序，证明每个mapped entity的committed lifecycle，不依赖外部snapshot或payload推导。其他mode必须为空。
- `initialPlacements`元素closed为`{"subject":<mapped_result_subject|new_result_subject(entityKind=node)>,"nodeRef":<NodeRef>,"parentRef":<NodeRef|null>,"ordinal":<D3Integer>}`；除fork中的trashed Node外，每个fresh Node恰一entry，subject必须通过identityMap或resultAllocations机械得到同一nodeRef；fork中只覆盖resultLifecycles.state=live的Node。root唯一允许parentRef=null且ordinal=0，其中0只是wire sentinel，root没有semantic siblingOrdinal。其materialized parent/ordinal必须与request resultPlacementPlan逐项typed相等，按subjectCanonicalKey排序且subject唯一。该数组证明fresh live tree，不使用structuralChanges伪装create。
- `trashPlacements`元素closed为`{"subject":<mapped_result_subject(NodeRef)>,"nodeRef":<NodeRef>,"trashParentRef":<NodeRef|null>,"trashOrdinal":<D3Integer>,"restoreLocation":<ConcreteForkRestoreLocation>}`；只在fork非空且恰覆盖resultLifecycles.state=trashed的Node。subject/nodeRef由identityMap机械绑定；trashParentRef若非null必须是同receipt中state=trashed的mapped Node，若null表示canonical Trash forest root；同parent的ordinal从0连续、无洞、唯一。`ConcreteForkRestoreLocation`恰为`{"kind":"mapped_original_location","parentRef":<NodeRef>,"ordinal":<D3Integer>}`或`{"kind":"unavailable_original_location"}`，并逐项物化request resultTrashPlacementPlan。它不形成live parent edge，也不能被resolver当作NodeRef来源。其他mode必须为空。
- `identityMap`元素只允许NodeRef、ResourceRef、AnnotationRef的same-kind `from`/`to`；Workspace映射由source/target WorkspaceRef表达。
- `preserved`与`tombstoned`元素只允许NodeRef、ResourceRef、AnnotationRef。`preserved`恰列本operation中identity保持不变、但authoritative payload、显式placement/structure、lifecycle或显式表示的structural side effect至少一项实际改变的**existing entity**；move/reorder/compound placement的唯一例外是未被选择的siblings因final-index算法产生的ordinal compaction，它们可由prestate与显式moves机械重建，绝不进入`preserved`或`structuralChanges`。未改变对象不得为“证明存在”而列入，fresh/mapped结果只由identityMap/resultAllocations表示。`tombstoned`恰列本operation新发布的准确tombstone closure。`existing_entity_subject(ref)`自身直接、唯一物化为encoded ref，不依赖preserved membership；preserved只是完整effect evidence，不是identity lookup表。
- `rewrittenReferences`是closed reference-result evidence union，而不是只允许有preimage的改写：
  - `preimage_reference_result`字段恰为`kind,fromSource,from,result`；result为`{"kind":"rewritten_slot_result","resultSlot":<ReferenceResultSlotKey>,"toSource":<ReferenceSlotAddress>,"to":<SlotCompatibleTarget>}`或`{"kind":"deleted_slot_result","resultContextKey":<ReferenceResultContextKey>,"resultContext":<ReferenceResultContext>}`。
  - `created_reference_result`字段恰为`kind,resultSlot,toSource,to`，用于result-only-existing/fresh plan entry；fresh plan的to必须等于targetTemplate机械物化出的完整slot target，而不只是subject；没有伪造from/fromSource。
  `ReferenceResultContext`恰为`{"kind":"document_result_context","owner":<NodeRef>,"documentRevisionToken":<non-empty opaque D6 token>}`或`{"kind":"annotation_result_context","annotationRef":<AnnotationRef>,"annotationRevisionToken":<non-empty opaque D6 token>}`。deleted表示Core已在committed postimage context验证preimage slot不存在。
- request中的container subject按variant机械物化：existing_entity_subject直接取其encoded ref；mapped_result_subject查identityMap；new_result_subject查resultAllocations。preserved不参与该lookup。Core再以committed revision和slotOrdinal唯一物化`toSource`。resultSlot.slotKind必须等于toSource slot family，且to必须等于对应existingTarget/FreshSlotTargetTemplate/map materialization。每个非lifecycle-only reference plan resultSlot恰一receipt reference entry、每个receipt resultSlot恰一reference plan entry；structural S的resultSlot只对应annotation_reply_change且禁止reference entry。即使两个slot target byte-equal也不能交换。preimage的from只按fromSource验证，to只按toSource验证；deleted的contextKey必须物化为同一个resultContext。
- `referenceLifecycleTransitions`元素closed为`{"source":<ReferenceSlotAddress>,"target":<SlotCompatibleTarget>,"prestate":"absent"|"non_live_source"|"resolved"|"suspended","poststate":"resolved"|"suspended"}`。source总是committed postimage slot，但prestate由与该slot唯一配对的request evidence origin决定，而不是由postimage container identity的新旧决定：result-only entry固定`absent`；非delete preimage entry先读取typed preimage `fromSource`的source lifecycle：trashed取`non_live_source`，live取准确target的`resolved|suspended`；lifecycle-only固定使用显式`expectedPrestate`，其中`non_live_source`只表示preimage container trashed但同slot存在；structural S reply origin则读取同一structuralPlan的准确preimage Value/3：reply为null取absent；非null且source preimage trashed取non_live_source，source live才取该preimage slot的resolved/suspended，非null postimage必须resolved到同map的concrete reply parent，null postimage不列。S的lifecycle transition不构成第二reference result。它完整列出本operation影响的全部postimage live source slots，不论poststate是resolved还是suspended；“影响”包括source payload/revision改变、source fresh创建/restore、plan显式覆盖，或target lifecycle在本operation中trash/restore，即使source bytes/revision完全不变。`preserve_exact`、`preserve_suspended`、result-only-existing/fresh、map/rewrite、`suspend_with_result`及lifecycle-only transition都按此规则双侧验证；delete没有postimage slot因此不列。fresh mapped result仍有typed preimage时绝不得使用`absent`；lifecycle-only entry必须与该数组中完全相同source/target/pre/post一一对应且没有orphan resultSlot/payload；target restore且preimage source live时必须显式列`suspended→resolved`；source同时由Trash恢复时列`non_live_source→resolved`。postimage source仍非live则不列live graph transition，不得对需列的transition以数组缺席表达成功。
- `structuralChanges`是以下closed union：
  - `node_placement_change`：`nodeRef,from{parentRef,ordinal},to{parentRef,ordinal}`，只用于live→live且from/to不同；
  - `node_trash_placement`：`nodeRef,from{parentRef,ordinal},to{trashParentRef,trashOrdinal,restoreLocation}`，只用于live→trashed；
  - `node_restore_placement`：`nodeRef,from{trashParentRef,trashOrdinal,restoreLocation},to{parentRef,ordinal}`，只用于trashed→live；
  - `node_trash_reparent`：`nodeRef,from{trashParentRef,trashOrdinal,restoreLocation},to{trashParentRef,trashOrdinal,restoreLocation}`，只用于仍trashed但任一Trash placement事实改变；
  - `annotation_reply_change`：`annotationRef,fromAnnotationRevisionToken,toAnnotationRevisionToken,fromReplyTo,toReplyTo`；它必须逐项匹配request唯一annotation_reply structuralPlan，toReplyTo等于replyTo按同map物化后的concrete Ref/null，且result Annotation Value/3的ordinal 1 S segment携带该structural entry。receipt不得另列同reply@1 slot reference result；同Annotation target@0的reference证据仍必需，lifecycle证据仅在postimage source live时必需。其toSource.annotationRevisionToken、reply transition source.annotationRevisionToken及toAnnotationRevisionToken必须逐字等于这一个最终Annotation revision，不能按target/reply分别增版。
  `restoreLocation`恰为`{"kind":"original_location","parentRef":<NodeRef>,"ordinal":<D3Integer>}`或`{"kind":"unavailable_original_location"}`。只有trash parent与reply fields允许其schema声明的null。Node restore或owner_restore_location中的owner Node必须恰有一条node_restore_placement；original location不能伪装为from==to的live placement。trash/restore/purge receipt必须枚举每个改变trashParent/trashOrdinal/restoreLocation的Node，包括ordinal shift；fresh Node仍只由initialPlacements/trashPlacements表达。
- `ReferenceSlotAddress`闭合为：`{"kind":"document_slot_address","locator":<DocumentRangeLocator>,"slotKind":"node_link"|"citation"|"resource_occurrence"}`；或`{"kind":"annotation_slot_address","annotationRef":<AnnotationRef>,"annotationRevisionToken":<non-empty opaque D6 token>,"slot":"target"|"reply_to"}`。Core必须验证Document locator在准确source revision/span唯一对应声明slotKind，或Annotation token等于该Annotation准确current revision，否则分别invalid_locator/stale_locator；裸DocumentRangeLocator不再是slot address。
- 每个slot独立决定唯一expected target union：`document_slot_address/node_link|citation`只允许NodeRef，严格等于冻结D2的Node-only authored target；AuthorAnchorAddress绝不进入这两个slot。`document_slot_address/resource_occurrence`只允许与该slot locator.owner同owner的ResourceRef；`annotation_slot_address/target`只允许§5.3 AnnotationTargetProjection且其owner等于该slot annotationRef.owner；`annotation_slot_address/reply_to`只允许同owner AnnotationRef。`from`必须通过fromSource family，`to`必须通过toSource family，两者不得共享ambient owner或revision；known wrong family固定domain_kind_mismatch。live Annotation的target是mandatory，`deleted_slot_result`对target永远禁止；standalone copy无法重建target时whole reject。只有reply_to可deleted为无parent。若整个Annotation被omitted或tombstoned，用omittedObjects/tombstoned表达对象消失，不为其自身target伪造live annotation_result_context。D4未来新增relation slot必须用新wireVersion或自身closed receipt extension，D3不预选其payload。
- `omittedObjects`元素closed为`{"ref": <NodeRef|ResourceRef|AnnotationRef>, "reason": "already_live"|"tombstoned"|"unreconstructable_target"|"explicitly_omitted"|"reply_closure"}`；per-mode reason必须满足§4.1.3 omission algebra，特别是restore只允许`already_live|tombstoned`，Node永远不得以copy/import omission reason出现。

receipt十二数组只按§4.1.3 `D3-Canonical-Comparator/1` registry排序；本节不定义第二套简写顺序。每个fromSource最多一次，每个reference-plan resultSlot及toSource最多一次，每个structural S resultSlot最多一次且不得进入rewrittenReferences，每个postimage source在referenceLifecycleTransitions最多一次；toSource与lifecycle transition source只可在同一slot、byte-equal target时重合。同一existing Annotation的annotation_value/annotation_reply@1 slot不得同时出现于reference result evidence和structuralChanges；同一Annotation的annotation_target@0必须继续拥有其完整reference证据，不能因subject相同而拒绝或省略；同一identity每个适用structural kind最多一次，同一数组canonical key重复拒绝。所有receipt数组输入置换必须生成同一D3-CJ/3 bytes，不允许完整JSON或缺失字段tie-break。

闭合mode：

- create_workspace
- create_node
- create_resource
- create_annotation
- copy_node_subtree
- copy_resource
- copy_annotation
- fork_workspace
- continue_workspace
- move_node
- reorder_node
- trash
- restore
- purge
- import_new

closed字段与per-mode语义：

| mode/variant | 顶层可选字段 required；其余forbidden | 数组精确语义；未提及数组为空 |
| --- | --- | --- |
| create_workspace | issuerAuthorityInstanceId + targetAuthorityInstanceId | 两字段分别等于proposal；resultAllocations映射全部declared graph subjects；每个Node有initialPlacement且primary root null/0；allocated = target WorkspaceRef + resultAllocations.ref；resultLifecycles/trashPlacements空 |
| create_node/create_resource/create_annotation | none | resultAllocations映射全部declared graph subjects；owner-local ref.owner精确物化ownerSubject；每个Node有initialPlacement且primary满足intent；fresh created references与existing preimage/result references分别完整覆盖；preserved恰列实际改变的existing声明；Annotation reply结构变化单列且S互斥；allocated = resultAllocations.ref；resultLifecycles/trashPlacements空 |
| copy_node_subtree | sourceWorkspaceRef | identityMap完整；allocated = identityMap.to；每个fresh Node有initialPlacement；rewritten/suspended/omitted均完整；resultLifecycles/trashPlacements/structural为空 |
| copy_resource/copy_annotation | sourceWorkspaceRef + destinationOwnerRef | identityMap完整且请求source恰映射一次；allocated = identityMap.to；rewritten/suspended完整；每个实际修改的target-Workspace existing Node/Annotation container在preserved恰一次并有preimage/result payload pair；copy_annotation可有显式reply structural side effect；omitted为空 |
| fork_workspace | sourceWorkspaceRef + issuerAuthorityInstanceId + targetAuthorityInstanceId | 两authority字段分别等于proposal；exact fork：identityMap.from恰覆盖source全部live/trashed对象；allocated = target WorkspaceRef + identityMap.to；resultLifecycles恰覆盖identityMap并保留两态；每个live Node有initialPlacement，每个trashed Node有trashPlacement且两数组精确分区；rewritten/suspended完整；omitted与structural为空；任一member无法重建则whole reject |
| import_new + ordinary_format | artifactClass | identityMap空；request ArtifactObjectKey→subject bijection逐项验证resultAllocations；owner-local ref.owner物化ownerSubject；allocated = resultAllocations.ref；每个new Node有initialPlacement且top-level forest按ArtifactObjectKey形成唯一连续插入block；fresh created references与existing preimage/result references分别完整；preserved恰列实际改变的existing声明；适用Annotation reply structuralChanges完整；resultLifecycles/trashPlacements空；IR/worker ID不得进入任何ref数组 |
| import_new + partial_identity_bearing_transfer_bundle | artifactClass + sourceWorkspaceRef | identityMap覆盖全部纳入source typed refs；allocated = identityMap.to；每个include Node有initialPlacement；rewritten/suspended/omitted均完整 |
| move_node/reorder_node | none | preserved只列实际parent/ordinal改变的operation subject；若改变，structuralChanges恰列node_placement_change；成功no-op两处都不列；final-index算法可机械重算unselected sibling compaction |
| trash | none | preserved = live→trashed closure，加上所有payload/structure/Trash placement实际改变且identity保持的existing side-effect containers；每个Node有node_trash_placement，surviving Trash graph变化另有node_trash_reparent；suspended列因target lifecycle或source变更而受影响的全部live slots；同commit slot/reply修复完整列入reference/structural evidence |
| restore | none | preserved = trashed→live成员，加上所有payload/structure/Trash placement实际改变且identity保持的existing side-effect containers；每个恢复Node有node_restore_placement，surviving Trash graph变化有node_trash_reparent；omitted只列请求closure中未恢复成员；suspended按“受影响”完整列出 |
| purge | none | tombstoned = trashed→tombstoned完整closure；preserved恰列所有因inbound rewrite/delete或surviving Trash reparent而实际改变的existing containers；同commit的reference/structural evidence完整列出 |
| continue_workspace | sourceWorkspaceRef + Core-reserved authorityInstanceId + continuation | authorityInstanceId必须等于planned reservation并在同commit激活/接管custody；十二个数组全部为空，不枚举全Workspace preserved |

`rewrittenReferences`在产生source改变的mode中必须列出本commit的全部D2/D3 reference slot改写/删除，但identity-preserving Annotation reply edit是唯一排除项且只进入structuralChanges；fresh Annotation reply初始化仍进入rewrite evidence。`structuralChanges`必须列出本commit所有identity-preserving Node placement与Annotation reply reparent副作用；`initialPlacements`列出全部fresh live Node结构，fork的fresh trashed Node只列`trashPlacements`；`resultLifecycles`证明fork每个mapped entity两态；`referenceLifecycleTransitions`必须完整。trash/restore/purge中的lifecycle transition、rewrite与structural evidence属于同一receipt、同一OperationId、同一commit decision，必须同成同败。`omittedObjects.reason`按mode闭合：copy/identity-bearing import只允许owner-local `unreconstructable_target | explicitly_omitted | reply_closure`；restore只允许`already_live | tombstoned`；fork、standalone copy及其他mode必须为空。一个omitted ref不能同时出现在allocated、preserved或tombstoned。

三个lifecycle compound golden只有以下单receipt表达：

- restore A并删除A Document中指向tombstoned B的slot：`mode=restore`，A因lifecycle与payload都改变在preserved恰一次；rewrittenReferences含`fromSource(A,trash preimage revision) + from=B + deleted_slot_result(document_result_context(A,restored revision))`；request缺该exact slot/result coverage固定stage14拒绝。
- purge trashed B并删除或替换live A→B：B在tombstoned，A因payload/revision改变在preserved；A的preimage/result slot完整出现在rewrittenReferences。closure外live A→B未处理固定stage15；B tombstone与A新revision只可同一commit decision发布。
- trash Annotation P并把live reply child C原子reparent到Q：P因lifecycle改变、C因Value/3/reply structure改变，二者都在preserved；structuralChanges含`annotation_reply_change(C, fromRevision=C1, toRevision=C2, fromReplyTo=P, toReplyTo=Q)`，且rewrittenReferences不得另含C.reply_to。缺S/structural entry固定stage15，双表示固定stage1。

上述golden不引入D6 compound transaction wire；D6可选择任意物理事务机制，但只能对外发布这一份D3 v11 receipt与一个terminal decision。

identityMap entry的from/to都必须是NodeRef、ResourceRef或AnnotationRef且kind相同。source typed ref可以来自另一个Workspace；这不是target-local `from`，而是完整foreign source ref。`copy_node_subtree`、`fork_workspace`和owner Node随bundle映射的identity-bearing import中，owner-local entry的to.owner必须等于同map中from.owner对应的NodeRef target。`copy_resource`/`copy_annotation`不要求也禁止为了owner correspondence加入未被复制的Node mapping；其每个local to.owner必须恰等于receipt.destinationOwnerRef。target ref不得重复，source ref也不得重复；identity-preserving`N→N` entry禁止。普通IR/provenance ID不进入identityMap。

owner-changing rewrite的四个canonical golden形状固定为：

1. Node copy：`fromSource=document_slot_address(owner=Ns,rev=S1)`、`from=ResourceRef(owner=Ns)`；result为`rewritten_slot_result`，`toSource=document_slot_address(owner=Nt,rev=T1)`、`to=ResourceRef(owner=Nt)`，且Ns→Nt及Rs→Rt都在identityMap。
2. Workspace fork：与上式相同，但from/to Workspace不同，source/target WorkspaceRef在receipt顶层显式绑定；不得出现omitted member。
3. partial import：foreign fromSource与from可作为验证过的bundle preimage；toSource/to必须属于target Workspace，owner映射由同receipt identityMap证明。
4. standalone Annotation copy：`fromSource=annotation_slot_address(annotationRef=As,annotationRevisionToken=AS1,slot=target|reply_to)`只验证旧slot；`toSource=annotation_slot_address(annotationRef=At,annotationRevisionToken=AT1,slot=target|reply_to)`只验证新slot，As→At为请求subject唯一map entry。target必须使用rewritten result并保持D2五成员union；只有reply_to可用`deleted_slot_result(annotation_result_context(annotationRef=At,annotationRevisionToken=AT1))`表示无parent。不得以单一source或revision token验证两侧。

对每个golden，preimage/result slot各自的expected union、owner、revision/span与唯一occurrence独立验证；任一侧失败whole operation拒绝。in-place edit同样使用两个revision-bound slot：identity可相同，revision token不得借用。

typed ref、identityMap与全部receipt排序只使用§4.1.3 RefKey/registry；identityMap固定`(RefKey(from),RefKey(to))`，不得改按destination、JSON bytes或容器枚举顺序；destination/source唯一性按完整typed ref比较。unknown mode/field/version、错误kind、mode-specific owner-map不闭合、canonical key重复或数组交叠均拒绝；合法输入置换必须接受并canonicalize。

OperationId replay严格服从§4.1.1的前置交付gate与Workspace-local scope：未授权不得读取saved bytes；authority/ledger不连续不得猜测结果；gate通过后rejected返回byte-equivalent recorded rejection、committed返回byte-equivalent原receipt、terminal_failed返回byte-equivalent terminal error，planned仅对相同fingerprint恢复；同一Workspace-local key的不同fingerprint固定operation_id_conflict。任何返回都不得改变既有decision、重新分配或把replay改报identity_collision。receipt、OperationId与transfer summary都不是写能力。

跨Workspace transfer没有联合commit receipt。协调层可以在两个authority的独立结果之上发布closed非权威证据：

~~~json
{
  "wireVersion": 11,
  "kind": "cross_workspace_transfer_outcome",
  "transferOperationId": "e1d3e52e-87ab-43e5-9936-f25bd0bea3f2",
  "observationSequence": 2,
  "targetWorkspaceRef": {"kind":"workspace_ref","workspaceId":"b6b8917e-f226-4e2a-8e2d-5a956ef16ea1"},
  "targetOperationId": "bc550154-f436-4f5f-adcb-7458df568d34",
  "targetDisposition": "committed",
  "sourceWorkspaceRef": {"kind":"workspace_ref","workspaceId":"9dfadab9-5f7a-4ba9-a6d4-681b29645283"},
  "sourceOperationId": "8735f383-504e-4e17-88cc-3146e38e937d",
  "sourceDisposition": "failed"
}
~~~

三个OperationId均为canonical lowercase UUID；source/target WorkspaceRef必需、不同且在同一transfer中固定。targetOperationId只在`(targetWorkspaceId,targetOperationId)` ledger中有意义，sourceOperationId只在`(sourceWorkspaceId,sourceOperationId)` ledger中有意义；UUID相同也不冲突。`observationSequence`是从0开始、对同一transferOperationId严格递增的D3Integer。每个outcome是immutable observation；协调层可以追加更高sequence，projection只取已验证的最高sequence，不得覆盖旧证据。若sequence=MAX，target/source disposition必须已terminal，且此后禁止任何新observation；MAX上的pending/not_attempted可继续组合是invalid outcome。targetOperationId在同一transfer中固定；sourceOperationId一旦首次出现也固定。

sourceDisposition闭合为not_attempted、pending、committed、failed；它只描述协调结果，不改变任一Workspace identity事实。`targetDisposition`闭合为committed、failed；target failed时sourceDisposition必须为not_attempted。`sourceOperationId`在not_attempted时必须省略，在pending/committed/failed时必需；sourceWorkspaceRef始终必需，以证明尚未尝试的是哪个Workspace。状态只能`not_attempted → pending → committed|failed`，或在target failed时保持not_attempted；terminal disposition不得回退。target/source均committed仍只证明两个独立Workspace-local commit，不得解码成跨authority原子receipt；unknown字段、重复sequence、WorkspaceRef/ID变化、状态回退或不可能组合拒绝。

## 13. 错误与 operation fail-closed

D3 operation error wire固定为`{"wireVersion":11,"kind":"identity_operation_error","operationId":<canonical UUID>,"family":<closed family>,"disposition":"preflight_rejection"|"recorded_rejection"|"terminal_failure"}`，通常只允许这五个字段。若OperationId本身未成功decode，`operationId`必须省略，其余四字段仍必需。error不附带ref、owner、path、候选对象、原request fingerprint、prestate或repair detail。

- `preflight_rejection`只用于stage 1–5、proposal P1/P2、TL1/TL2，或已有planned decision在stage 3/4/适用P2/TL无法安全交付/恢复时；它不创建或改变target ledger，exact retry重新评价当前preflight gate。P1/P2 error可回显已成功decode的OperationId，但不得route/read target ledger；create/fork的TL在P2成功后运行，ordinary的TL在stage4成功后直接运行；两者都只读custody metadata，仍不读ledger key content。
- `recorded_rejection`只用于stage5 unseen后的stage7–15 primary failure；Core使用§4.1.1 decision CAS原子比较expected ledger/family/formal role后写`rejected`，create/fork winner同时转decision_bound_rejected并burn target IDs。CAS loser没有errorBytes decision、effect或burn，必须从唯一restart point重判；exact same-fingerprint retry仍可过P2(c)+TL+stage5取回保存bytes，迟到predecessor不能触发该effect。
- `terminal_failure`只用于已planned计划的authoritative abort，family必须且只能是`identity_commit_aborted`；它原子写`terminal_failed(errorBytes)`并burn全部reservation。D6的物理原因只可进入独立受权audit/repair channel。
- disposition/family不匹配、recorded rejection没有成功decode的OperationId/fingerprint、terminal failure没有既存planned decision均为authority integrity conflict；实现不得降级成私有transport error。

下表的stage与行序就是唯一全局primary family总序；编号stage、§4.1矩阵与测试均由此表生成，后文不得定义第二套优先级：

| stage | family | 条件 | 结果 |
| --- | --- | --- | --- |
| 1 | invalid_identity_ref | §9.4 pure decode、static invariant、static request matrix/cross-field equality；不读authoritative state | preflight_rejection；不创建ledger |
| 1 | domain_kind_mismatch | known ref/locator/evidence/artifact class放错expected union/mode | preflight_rejection；不创建ledger |
| 2 | workspace_ref_misbound | 唯一Workspace role矩阵违规，包括target/foreign preimage role | preflight_rejection；不route/fetch |
| 3 | identity_not_visible | ordinary当前operation，或create/fork issuer/current principal/fork source及所需locator-state disclosure authorization拒绝 | preflight_rejection；不读target ledger/closure/revision/existence |
| 4 | identity_authority_unavailable | stage3已授权的existing authority/source未挂载、已销毁，generation/cut/ledger continuity不可证明 | preflight_rejection；不读target ledger |
| 4 | workspace_integrity_conflict | authority可达但duplicate writer/record、active+tombstone collision、missing owner、非唯一projection，或任一相关live/Trash sibling list存在duplicate/gapped ordinal或非canonical次序 | preflight_rejection；resolver投影workspace_unavailable；target OperationId ledger content零读写、零allocation/reservation/mutation且无receipt |
| P1 | workspace_identity_conflict | create/fork token body真实性、current-principal audience、issuer与request纯字节binding不成立 | preflight_rejection；不route/read/write target ledger，不读/改family |
| P2 | workspace_identity_conflict | proposal既非current active、同OperationId winner，也非同fingerprint decision_bound_rejected；或planning CAS时family已改变 | preflight_rejection；target ledger zero-read/write，不改变family |
| TL1 | identity_authority_unavailable | create/fork在P2成功后、ordinary在stage4成功后，TargetLedgerCustodyRecord/custodian不可达，或custody generation、ledger/allocation/burn/saved-decision continuity不可证明 | preflight_rejection；只读custody metadata，不读ledger key content，不创建decision |
| TL2 | workspace_integrity_conflict | TL1已证明reachable/continuous，但custody或ledger namespace存在duplicate record、key/state collision、saved bytes非唯一 | preflight_rejection；不读目标OperationId content，不创建decision |
| 5 | operation_id_conflict | existing Workspace-local ledger key收到different fingerprint | preflight_rejection；不披露原request/state |
| 5 | identity_authority_unavailable | ordinary ledger key为unseen，expectedAuthority token已证明属于连续ledger但older than current active generation | preflight_rejection；恰一次ledger-key read，零decision/write/reservation；不进入stage7 |
| 6 | — | v11保留的空stage；不得插入实现私有gate/family/read/effect | 无结果；直接进入stage7 |
| 7 | owner_mismatch | expected owner/slot owner与request bytes encoded owner不一致 | recorded_rejection；typed-key lookup前停止 |
| 8 | root_operation_forbidden | decoded subject是bound Workspace root且mode为trash/purge/move/reorder | recorded_rejection；先于lifecycle，root当前态不改变family |
| 9 | identity_not_resolvable | 允许披露但exact typed ref为not_found/burned | recorded_rejection |
| 9 | entity_not_live | 普通action/copy source非live，或restore/purge subject仍为live（两者只接受trashed） | recorded_rejection |
| 9 | entity_not_restorable | restore/purge target已tombstoned，或owner tombstoned、stored reply parent不可用且无显式合法plan、payload不完整 | recorded_rejection |
| 10 | invalid_ordinal | D3Integer已decode但不在当前parent合法插入/重排区间 | recorded_rejection |
| 10 | orphan_creation | move/delete/restore/compound structure将产生live orphan | recorded_rejection |
| 10 | structural_cycle | Node parent或Annotation reply成环 | recorded_rejection |
| 11 | invalid_locator | current-revision locator/slot semantic唯一性无效，包括anchor_ambiguous/wrong slotKind | recorded_rejection，无fallback |
| 11 | stale_locator | Document/Resource/Annotation revision token或exact coordinate不再成立 | recorded_rejection，无fallback |
| 12 | identity_collision | target/reservation已有相同typed ref、tombstone或burn/proposal history；continue时Core内部ephemeral candidate AuthorityInstanceId命中proposed/active/burned/destroyed allocation history | decision CAS原子写recorded_rejection；不创建ContinueAuthorityReservation、不在同OperationId内重采样；exact retry重放原拒绝，new OperationId可重新采样 |
| 13 | cross_workspace_identity_preservation | 跨Workspace试图保留content ref | recorded_rejection |
| 14 | identity_map_incomplete | authorized cut中copy/fork/import缺subject/ref/payload/resultAllocation/resultLifecycle/initialPlacement/trashPlacement、owner-map、slot或exact partition | recorded_rejection；不写planned |
| 14 | operation_precondition_failed | 带preparationBinding的unseen请求已授权，但原完整绑定request、期限、D7条件或当前依赖不成立；在identity_map_incomplete之后 | recorded_rejection；不写planned；availability不伪装为确定业务拒绝 |
| 15 | inbound_reference_conflict | omission/reply/owner closure不闭合，或purge/transfer/restore将留下dangling live ref/single-receipt修复不完整 | recorded_rejection；不写planned |

一个operation一次只返回一个primary family。机械validation stages与表逐项相同：

1. closed request/OperationId/ref/slot decode、common integer、数组canonicalization、只依赖request bytes的static matrix与cross-field equality；绝不枚举authoritative closure；
2. 按source preimage/target result/destination/ambient role做Workspace binding；
3. ordinary验证当前operation/saved-result；create/fork只验证issuer、current principal、fork source及所需locator disclosure authorization；
4. 只对stage3允许的existing authorities验证availability、generation/cut/ledger continuity与integrity；integrity包含不读取target OperationId ledger content的结构索引证明，逐相关live/Trash sibling list验证ordinal从0连续且唯一。duplicate/gapped/noncanonical ordinal固定在此停止，不进入P1/P2/TL/stage5，也不产生allocation、reservation、mutation或receipt。ordinary旧generation token若可证明属于同一连续ledger，只获得通往stage5 saved-decision lookup的provisional admission，不授权unseen decision；partial offline artifact source永不查询；
P1. create/fork按精确token preimage验证authenticity、audience、issuer及request byte binding；失败停止且target ledger zero-read；
P2. 读取issuer allocation family，验证current proposal或同winner关系；失败停止且target ledger zero-read；
TL1. create/fork在P2成功后、ordinary在stage4成功后解析唯一custody record并验证custodian可达及完整ledger/allocation/burn连续性；unavailable/unproved先于任何reachable-record integrity；
TL2. 只在TL1通过后检查custody/namespace/ledger metadata唯一完整；
5. 只有前述gate通过后才读取`(boundWorkspaceId,OperationId)`content并比较fingerprint：different固定operation_id_conflict；authorized saved same-fingerprint返回保存bytes并停止；planned进入恢复；unseen ordinary必须再比较expectedAuthority token与current generation，older固定identity_authority_unavailable且零write/reservation，current才继续；unseen create/fork按proposal路径继续；
6. 保留空stage，不读状态、不产生family/effect；
7. 只比较request bytes的expected/slot/encoded owner；
8. 在exact typed-key lifecycle前识别root subject；
9. exact typed-key与mode lifecycle；存储record/key owner矛盾归stage 4 integrity；
10. ordinal、orphan、cycle；
11. 已在stage 3获locator disclosure后检查semantic uniqueness、revision与coordinate；
12. identity/proposal/reservation uniqueness；content candidates严格按§4.1.3a采样、验证并在后续planning CAS reserve；continue只检查Core本次内部ephemeral candidate，collision以target-ledger rejection decision CAS原子写recorded rejection、零reservation、同OperationId不重采样，绝不读取caller-supplied ID；candidate当前free只允许继续stage13–15，尚不形成reservation；
13. cross-Workspace preservation；
14. 读取同一authorized cut，验证identityMap/object/ref/payload/result allocation/result lifecycle/live placement/trash placement/result-slot exact closure与symbolic bounds；
15. inbound refs、omission、reply、owner、rewrite/suspended/structural closure；全部通过后才进入§4.1.1 planning boundary。

早gate一旦成立，所有晚gate不再探测；同stage多条件按表中行序。stage4与TL gate都固定availability/unproved continuity先于reachable integrity；两者同时成立只返回identity_authority_unavailable。ordinary existing key + different fingerprint只有在generation/continuity已通过stage4与TL后才到stage5 conflict；ordinary old-generation token在stage5只有existing same-fingerprint replay或unseen identity_authority_unavailable两条闭合分支。proposal字段flip则按stage3/4/P1/P2/TL1/TL2/5精确矩阵，不得一概stage5。未授权+缺mapping只得到stage3；forged/audience-wrong proposal无论hidden target ledger是否存在都在P1返回byte-equal family且target ledger zero-read；late burned predecessor在P2停止且successor zero-effect。Workspace-role只stage2；root purge只stage8；cross-owner ref在四生命周期态都stage7且不查询key。stage1–4、P1/P2、TL1/TL2、stage5 conflict或old-generation unseen不写新decision；stage5 current-generation unseen后的stage7–15失败只可经decision CAS写rejected。全部通过并在planning decision CAS中对create/fork重验P2/TL/ledger expected state、对ordinary重验TL/ledger/current generation，同时对content map及continue重验stage-12 ephemeral candidate仍free后才写planned+reservation。allocation race造成的CAS loser在没有decision时按§4.1.1唯一restart rule重新进入，不得发出unrecorded private error。没有实现私有family。

## 14. 边界案例与最小反例

1. 同名Node：相同title和path仍是不同NodeRef。
2. Node rename/reorder：ref不变；收藏、关系和Query entity identity不变。
3. heading文本相同：跨revision两个heading不能因文本相同获得同一entity identity。
4. explicit anchor保留：可在新revision重新解析并签发新locator，但anchor address仍不是entity。
5. Resource bytes相同：不同owner或不同ResourceId始终不同ResourceRef。
6. Resource跨Node拖动：产生new ResourceRef；若UI只显示move，也必须交付copy+optional source trash receipt。
7. Annotation target stale：AnnotationRef保持，accept/replay失败。
8. 删除parent Node：subtree一起trashed；child不能留在live tree成为orphan。
9. purge后新建：即使title/source相同也生成fresh ID；旧ref解析tombstoned。
10. unknown UUID：`mayDiscloseEntityState=true`时not_found；false时无论真实状态均not_visible。
11. raw Workspace copy：未选择continue/fork前不能获得write authority。
12. disaster restore：只有含verified continuation cut且无未知later commit的exclusive continue保留refs；stale backup无法证明时只能fork/只读，new AuthorityInstanceId不进入content ref。
13. Workspace fork：live+trashed成员全部fresh且identityMap完整并保留两态；旧tombstone不复制为allocation history；旧/new裸Node UUID绝不能比较。
14. cross-Workspace transfer外链：未复制target必须explicit transform或reject，不能留下合法D2悬空token。
15. duplicate active identity：两个AuthorityInstance声称同WorkspaceId时D6必须阻止至少一方commit。
16. restore mixed closure：同一trash closure中已单独恢复的live descendant保持现位并被祖先restore排除；tombstoned descendant保持终态并列omitted；真正foreign live collision拒绝且不自动rekey。
17. unauthorized tombstone：返回not_visible，不泄露曾存在或purge时间。
18. ResultRowHandle重放：revision/auth变化后stale；不能从row key构造NodeRef。
19. provenance含NodeRef：只说明来源，不能直接批准Action。
20. D5 future Record：即使leaf UUID碰巧等于NodeId也因ref kind/domain不同而不相等。
21. allocation abort：OperationId A reserve X后terminal failure，X必须burn；A+same fingerprint重放同一error且不能提交X，A+different fingerprint固定operation_id_conflict；OperationId B永不能分配X，resolver仍not_found。
22. suspended vs dangling：trash B后A->B成为suspended且不隐式改写A；purge B在A未显式rewrite/delete时拒绝。
23. re-entry discriminator：同一export按ordinary format只能import_new；只有formal backup+explicit continue可保留Workspace identity；path/digest不能改变mode。
24. receipt owner-local map：subtree/fork中to.owner来自owner Node mapping；standalone Resource/Annotation copy则来自required destinationOwnerRef且禁止虚假Node mapping；foreign source typed ref合法，IR ID非法。
25. privacy matrix：无state-disclosure时live/trashed/tombstoned/never-known全部not_visible；有state-disclosure时四态机械区分但不自动赋予content read。
26. cross-Workspace partial transfer：target receipt成功、source trash失败时outcome为copied/source failed，不能显示atomic move。
27. annotation copy omit：parent Annotation无法重建时reply descendants一起omit，或在同一plan显式reparent；不得留下dangling reply。
28. Workspace destruction：旧ref统一workspace_unavailable，不保留tombstone/never-known差异，也不允许同WorkspaceId重新激活。
29. owner-local independent purge：Resource/Annotation单独purge后其tombstone保留encoded owner，即使owner仍live/trashed；owner后续restore/purge均不复活或冲突重建该对象。
30. Workspace resolver：available/not_visible/workspace_unavailable/invalid闭合；不得返回Workspace trashed/tombstoned/not_found，也不得在duplicate writer间选winner。
31. owner-derived r1 copy：N的trashed Resource R留下suspended occurrence后copy N；新Document必须delete/replace该occurrence或whole reject，即使destination owner恰有相同ResourceId也不得rebind。
32. Annotation target owner：N1-owned Annotation携带N2 Document/Resource locator固定拒绝；reply_to携带NodeRef或target携带AnnotationRef固定domain_kind_mismatch。
33. locator oracle：owner/entity已resolved/live且entity state可见但locator state不可披露时，valid、stale、missing anchor、duplicate anchor、wrong span/region全部not_visible且不访问coordinate index。
34. authority corruption：live+tombstone collision、duplicate record与missing owner对普通resolver统一workspace_unavailable，对operation统一workspace_integrity_conflict；repair detail不外泄。
35. purge prestate：live target只得entity_not_live，trashed target可purge，tombstoned target只得entity_not_restorable；不存在直接live→tombstoned receipt。
36. receipt canonical：同一map按destination排序但未按`(from,to)`排序必须拒绝；allocated漏列或重复identityMap.to必须拒绝。
37. recorded purge rejection：OperationId O对live B执行purge得到`recorded_rejection/entity_not_live`；B后来由另一个operation trash后，O的exact retry仍byte-equal replay原rejection，不得purge。只有新OperationId可重新评价trashed prestate。
38. allocation issuer/target split：issuer A签发fork proposal时target authority B是fresh reserved ID；committed receipt必须同时回显A/B并使B成为target初始authority，replacement同时burn旧target WorkspaceId与B。continue request禁止authorityInstanceId，planning CAS由Core另mint/reserve C，exact recovery复用C，commit receipt回显并激活C，terminal failure burn C。
39. already-suspended copy：live A的node_link指向closure外、同Workspace、已trashed B；copy A必须以`preserve_suspended`保留byte-equal B，result payload用E segment，rewrittenReferences证明A旧slot→A'新slot且target仍B，referenceLifecycleTransitions唯一为A'→B的`suspended→suspended`。A'虽是fresh Node，但该slot属于preimage-backed branch，`absent→suspended`必须拒绝。若同一场景是owner-derived Resource occurrence，则因A→A'改变owner而不得preserve，仍只可map到fresh local R'、delete/rewrite或whole reject。
40. fork missing original parent：trashed B的original parent P已purged；exact fork仍必须映射B且state=trashed，resultTrashPlacementPlan把B'置于trashParent=null/ordinal canonical位置并写`unavailable_original_location`，receipt以resultLifecycles+trashPlacements逐项证明。之后restore B'若不用explicit_location必须拒绝，绝不把P tombstone、path或Workspace root猜成parent。
41. proposal tampered-first/ledger oracle：active P的token或audience被改一byte；P1返回byte-equal conflict且target ledger zero-read。genuine P可继续。已replaced predecessor在P2失败且successor零effect；current P的recorded failure转decision_bound_rejected，只有同proposal+OperationId+fingerprint可过P2(c)重放。
42. ordinary import forest：artifact产生ArtifactObjectKey K0/K1与top-level roots r0/r1且intent.destinationOrdinal=4；Core先按K0/K1规范序决定key→subject bijection，再按所得subjectCanonicalKey把二者放到同一destinationParent的4/5。二者都写4、写4/6、交换为5/4、使用第二parent，或按transport/provider/artifact输入枚举顺序分配subject/ordinal都拒绝；不得暗建wrapper Node。
43. duplicate same-target slots：新Document的slot0与slot1都指向B；plan必须有两个不同ReferenceResultSlotKey，receipt不得把slot0/slot1的toSource互换，即使to bytes相同。缺slot、多slot或交换绑定固定stage14 identity_map_incomplete。
44. create N+E：create Node N0的slot0指向同operation new Node N1（result_only_fresh/N），slot1指向既存live B（result_only_existing/E）；两个entry、两个created_reference_result与symbolic segments逐ordinal一一对应。旧preimage-only实现必须明确拒绝不支持的当前wire11，也不能猜测历史v10的N/E语义。
45. owner-local fresh swap：ordinary import创建N0/N1及相同bytes的R0/R1；R0.ownerSubject=N0、R1.ownerSubject=N1。交换receipt Ref.owner、漏ownerSubject或因digest相同合并subject都拒绝；owner不会从payload/数组位置推断。
46. restore placement：trashed A的stored original为P/2，恢复到P/2仍必须用node_restore_placement从Trash facts到live facts；不得用from==to node_placement_change。恢复parent造成surviving trashed child splice时，另逐child列node_trash_reparent。
47. final index：`[A,B,C]` move A→2得到`[B,C,A]`；A→0是成功no-op且无placement change；compound `{A→2,C→0}`按最终indexes一次求值，不按input order依次移动。
48. Trash splice：root trash siblings`[X,P,Y]`，P的trashed children`[C0,C1]`；purge P但保留children后roots必须`[X,C0,C1,Y]`，不是`[X,Y,C0,C1]`或数据库枚举序。receipt列C0/C1/Y所有变动Trash ordinals。
49. integer exhaustion：allocation sequence MAX replacement返回allocation_sequence_exhausted且P保持active；transfer MAX必须terminal；forest destinationOrdinal=MAX且第二root导致invalid_ordinal；symbolic count MAX+1导致identity_map_incomplete。任何wrap到0均为conformance failure。
50. canonical strings：U+001A只能以lowercase `\u001a`作为canonical escape；uppercaseescape重编码不等evidence。`é`与`e`+combining acute保持不同bytes/hash，non-BMP直接UTF-8。
51. offline bundle：artifact-local foreign ref恰等source中真实/不存在/trashed三种状态时，stage3/4/9/14的source authority read count都必须为0，outcome只由bound manifest与target facts决定。

## 15. 替代方案与裁决

| 方案 | 结论 | 最小拒绝理由 |
| --- | --- | --- |
| 全产品只用裸UUID | 拒绝 | 无Workspace/domain/owner，容易跨集合和事件ID碰撞 |
| Path或slug是Node identity | 拒绝 | move/rename破坏ref，并依赖D6物理布局 |
| content hash是identity | 拒绝 | edit改变identity，相同内容错误合并 |
| Document另设独立标识 | 拒绝 | 违反D2 Node:Document 1:1和无第二identity |
| Block/Heading有durable ID | 拒绝 | 正文编辑造成entity churn，违反D2 occurrence边界 |
| ResourceRef是workspace-global UUID | 拒绝 | 破坏owner-local封闭和跨owner防误用 |
| Resource跨owner保留identity | 拒绝 | owner是ref组成，reparent会改变引用含义 |
| soft delete后立即not_found | 拒绝 | 无法安全restore、诊断或区分never-known |
| purge后删除全部identity事实 | 拒绝 | ID可能复用，旧ref无法区分known-purged |
| tombstone永久保存title/path | 拒绝 | 扩大隐私和被遗忘数据面；identity诊断不需要payload |
| raw copy自动continue | 拒绝 | 可产生两个相同WorkspaceId的独立writer |
| 移除AuthorityInstanceId并完全留给D6内部 | 拒绝 | duplicate-writer、continue激活与审计需要一个跨D6实现可传递但不进入content ref的最小控制身份；仅有WorkspaceId不能区分并发物理writer |
| 所有restore都fork | 拒绝 | disaster recovery破坏合法长期ref |
| cross-Workspace move保留NodeRef | 拒绝 | authority namespace改变却声称同一entity，并需跨authority原子提交 |
| fuzzy locator自动reanchor | 拒绝 | ambiguity可能把Annotation/Action指向错误内容 |
| Query row handle就是entity ref | 拒绝 | aggregate/project/unnest产生occurrence，权限和revision作用域不同 |
| Trash是特殊Node | 拒绝 | 与D2普通Node代数混淆，可能进入tree/Query/Document |
| 复制时所有外部ref一律改成copy | 拒绝 | closure外没有copy，且会改变作者意图 |
| 复制时所有内部ref一律指向original | 拒绝 | copied subtree内部语义断裂；规范采用internal rewrite/external preserve |
| ICS UID直接作为NodeId/NodeRef | 拒绝 | UID只在SourceBinding内持久，跨源可重复且不能承担Workspace/owner/lifecycle语义 |
| 每个recurrence occurrence默认建Node | 拒绝 | 无限/长期series会无界materialize，并给可重建occurrence伪造durable identity |
| recurring override必定是Node | 拒绝 | override默认可为series-owned value；只有显式adopt且确需独立内容能力才fresh-create Node并同决议写OriginBinding |
| recurring override必定是Record | 不冻结 | D5保留从零决定独立Record域；D3只禁止与NodeRef/foreign key coercion |
| derived occurrence无独立对象 | 接受为默认 | 可由recurring-series rule/cut重建，满足最小identity面且避免无界Node扩张 |
| 同UID跨calendar/source自动merge | 拒绝 | source scope不同，可能是无关事件；必须显式用户/映射决策且不能保留同一Node identity假象 |
| adopt保留UID为Node identity | 拒绝 | provenance关联不等于identity continuity；adoption必须fresh NodeRef并保留分离OriginBinding |

## 16. 对 D4–D10 的稳定门禁

### 横切术语与命名

- 下游所有受控identifier/symbol/locale key必须映射D3 Lexicon stable concept ID；需要新concept或不同含义时先建立新entry，不复用裸term。
- D4/D7若冻结Relation或Library citation更窄语义，必须保留Reference/Typed Reference/Node Link/Citation排除边界；D5若定义Record不得叫item/entity/node。
- D6/D9对SourceBinding/OriginBinding的wire/persistence不能把owner/authority/source/provenance重新合并；D8可调整用户显示文案但locale key仍映射canonical concept。
- R0负向gate仅作用受控surface并给出migration target；历史review、用户内容与第三方format字段不在denylist执行面。

### D4 属性、schema、关系

- ref-valued字段必须使用显式typed ref kind，不接受bare UUID、path或label。
- Node relation是否允许suspended/tombstone-aware语义必须逐字段显式定义；D4冻结前D3不替尚不存在字段选择delete/resolution policy。
- relationship owner、direction、cardinality、delete policy由D4决定，但不得reparent Resource/Annotation或coerce NodeRef。
- D2/D4受管authored state中的Resource direct ref必须owner-local：containing Node等于ResourceRef.owner。D4不得新增跨owner Resource direct relation；只能建NodeRef-to-owner关系或触发copy-to-fresh-owner。该负向门禁不可由字段级resolution policy放宽。
- schema migration不能改变既有entity identity；若domain kind改变必须copy/new identity。
- Calendar/Event/Task/journal/time-note 的typed schema与mapping由D4决定；VEVENT、VTODO、VJOURNAL、VFREEBUSY、VTIMEZONE不得因同属VCALENDAR而自动共享Event/Node identity。VFREEBUSY/VTIMEZONE在任何authority mode都固定没有Node identity，D4/D9只能选择非Node mapping；Task复用其Node identity，checklist item只是Document occurrence且无独立identity。

### D5 记录域

- D3不决定Record是否存在。
- D5从零决定record domain的scoping、entrypoint、authority、owner、identity结构、persistence、lifecycle与resolver；D3不得要求它挂在Workspace、不得替它选择copy/delete/purge操作集。
- D3只冻结identity隔离门禁：若D5定义任何RecordRef/RecordCollectionRef，它们必须closed-decode，且不得复用WorkspaceRef、NodeRef、ResourceRef、AnnotationRef、locator、evidence或bare UUID equality，也不得相互coerce。
- Record与Node之间若未来允许转换，只能由D5/D4显式定义copy/create + mapping；不得原地kind mutation。
- recurring override/occurrence若由D5选择Record表达，其foreign key与RecordRef必须分域；D3既不要求Record，也不允许把UID/RECURRENCE-ID当作RecordRef或NodeRef。

### D6 存储、事务、权限、同步

- 以下是D3要求D6保证的identity/lifecycle不变量，不是物理实现选择：WorkspaceId、AuthorityInstanceId、single active authority、commit fence、continue/fork gate、continuation cut、reservation/burn ledger和tombstone persistence。
- path/database row/index/cache不得成为identity authority。
- lifecycle/copy/restore/purge是revision-bound原子plan；crash recovery不能报告partial success；相同OperationId按ledger状态恢复planned或重放原receipt/error，different fingerprint固定operation_id_conflict。
- documentRevisionToken/resourceRevisionToken必须有canonical equality语义；D3不规定token编码。
- authorization在existence/trash/tombstone探测前；D6把调用上下文映射到第9节state-disclosure谓词，not_visible不得泄露。
- external byte changes造成reconciliation，不自动rekey/reparent。
- snapshot/history/backup IDs与content refs分域。
- SourceBinding、provider token、cursor、etag、sync watermark与OriginBinding的物理持久化、retention、解绑/retire和冲突事务由D6决定，但必须实现§3.1每ForeignIdentityKey至多一条active binding、retired审计状态、non-live target零auto-restore/零duplicate及损坏/不唯一fail closed；source scope、content identity、authority及provenance四域始终可区分。

### D7 Query、View、Action

- selectors默认只返回live authorized typed refs；include-trash若以后存在由D7决定payload/UX，但必须显式请求并服从第9节state-disclosure门禁。
- Saved Query/View只通过NodeRef+locator/anchor地址，无ViewRef。
- LogicalOccurrenceKey、ResultRowHandle、Provenance、ActionEvidence遵守第11节，不得提升为entity。
- Action在commit前重新resolve ref、revision、auth；tombstoned/trashed/stale fail closed。
- query invocation不得暴露callee内部occurrence key或把row handle持久化；D3不定义QueryRef。
- derived calendar occurrence可作为D7 occurrence/value进入结果，但不是StableEntityRef；foreign series anchor/occurrence/override只有显式adopt、fresh NodeRef并同决议建立OriginBinding后才可走entity action。managed Document occurrence仍只可使用promote。View/Action payload由D7另行冻结。

### D8 编辑器

- breadcrumb/path/title/selection不产生identity。
- reanchor只能提出proposal并由Core签发new locator；ambiguous不选closest。
- move/copy/delete UI必须让用户可识别identity-preserve还是fresh-copy语义，并准确呈现partial cross-Workspace transfer outcome；具体呈现由D8决定。
- 跨Node粘贴含r1 owner-local token时，编辑器必须依据identityMap重写到new owner ResourceRef或拒绝；不能让相同local UUID在destination ambient owner下静默重绑定。
- stale Annotation不能静默应用。

### D9 import/export/template/worker

- IR/worker IDs是temporary；target entity IDs由Core分配。
- 第8.6节artifact class与continue/fork/import-new/restore组合必须显式，不能按path/digest猜；unknown组合拒绝。
- Template是 coreKind=template 的Core meta-kind。实例化始终产生fresh Node/Resource/Annotation refs，原身份结果严格按实际D3 mode：copy/fork/identity-bearing import返回其完整identityMap；create或ordinary-format import的identityMap为空，返回完整resultAllocations及适用placements/reference证据。D9可从完整来源到symbolic subject的准备证据与原resultAllocations机械关联显示来源到实例，但不得增设第二身份权威或改写receipt。Task Template只可由D9 target plan声明targetCoreKind=ordinary且target facets包含exact tasks/task；Template自身禁止因此成为Task。每个真实typed slot按该mode的fresh result/identityMap规则完整处理；Facet/carrier/entry occurrence不成为reference slot。
- worker不得选择或写Workspace identity，不得把source UUID变target ref。
- export身份承载策略必须在receipt中声明；普通format export re-import是fresh。
- ICS mapping必须有界且先preview：subscribe/sync、bounded `initial_import`与`adopt` selected foreign object三种external-authority mode不可互相猜测；managed occurrence的`promote`和managed live Node的`managed_copy`是另外两个identity intent。同源upsert使用SourceBinding+UID(+RECURRENCE-ID)并逐格遵守§3.1 shape/state矩阵；第二份内容只声明`managed_copy`且activeBindingWrites=0；跨源相同UID不自动合并；长期/无限 recurrence 禁止默认展开为Nodes；VFREEBUSY/VTIMEZONE任何mode均不得建Node。
- VTODO→Task、VJOURNAL→journal/time note及其他VCALENDAR component mapping由D4/D9显式选择；D9不得把UID、SEQUENCE、时间戳、provider token或path写入NodeRef。

### D10 Agent、自动化、外部能力

- Agent/automation只能携带typed ref/locator作为proposal target；Core重新resolve。
- transcript、provider object ID、tool call ID、audit ID、approval ID不是content ref。
- provenance不授权，ActionEvidence不扩大主体权限且revision/auth变化即失效。
- 外部连接器不得按path/title猜Node，也不得将foreign ID coercion为NodeRef。

## 17. 实现影响图

    typed identity foundation
      -> WorkspaceId + AuthorityInstanceId
      -> NodeRef
      -> owner-local ResourceRef / AnnotationRef
      -> canonical source tokens

    exact locator foundation
      -> DocumentElementLocator / DocumentRangeLocator
      -> AuthorAnchorAddress
      -> ResourceRegionLocator
      -> stale/fail-closed resolver

    lifecycle authority
      -> live / trashed / tombstoned
      -> subtree and reply closure
      -> canonical Trash splice/order evidence
      -> live/trash/restore placement receipt variants
      -> restore preserving identity
      -> purge minimal tombstones

    copy and activation
      -> issuer proposal token body + principal audience pre-ledger gate
      -> Workspace-local OperationId ledger; no cross-family registry
      -> result-slot / ownerSubject commitments
      -> same-Workspace fresh subtree copy
      -> cross-Workspace transfer map
      -> Workspace continue vs fork
      -> import/export identity receipts

    foreign identity and provenance
      -> source-scoped ForeignIdentityKey / SourceBinding
      -> external vs Weftext authority modes
      -> derived occurrence / series-owned override
      -> explicit adoption with fresh NodeRef + same-decision OriginBinding
      -> deterministic re-import without UID coercion

    terminology governance
      -> stable concept IDs + canonical Chinese/English terms
      -> wire/API/code/CLI/UI/locale one-to-one mapping
      -> retired alias registry + explicit deletion targets
      -> controlled-surface negative gate; user content excluded

    downstream envelopes
      -> D4 typed refs
      -> D5 independent optional record refs
      -> D6 revision/auth/transaction/storage
      -> D7 handles/provenance/evidence
      -> D8 reanchor UX
      -> D9 worker/import/export mapping
      -> D10 proposals/audit isolation

未来实现至少影响：

- crates/weftext-core 的 NodeId、workspace scope、document revision、navigation、transactions、Trash、history、links、citation、Resource/Annotation边界；
- canonical encoder必须实现D3-CJ/3 exact scalar escaping和D3Integer无wrap算术，并产出可跨Rust/TypeScript/.NET复算的golden；
- allocation bootstrap必须把P1/P2置于target ledger read之前、记录principal audience/token body、处理MAX exhaustion与late predecessor零effect；
- plan/receipt schema必须实现ReferenceResultSlotKey、created reference result、ownerSubject、四类Node lifecycle placement evidence与final-index placement算法；
- weftext-import 的 IR ID、proposal ID、target allocation、receipt和resource path映射；
- calendar connector/import 的SourceBinding、foreign key、origin binding、bounded preview、series/occurrence policy及conflict projection；
- Core/public schemas/CLI/locales 的Lexicon concept mapping、退役identifier删除清单与受控负向命名gate；
- backup/history 的 repository/lineage/snapshot/store/entry ID分域；
- Desktop/CLI/Server/WebUI/Mobile 的ref wire、resolver outcome和no-fallback错误投影；
- Query candidate中的StableEntityRef/Locator/LogicalOccurrenceKey/ResultRowHandle/Provenance/ActionEvidence；
- 旧公开规范中source weftext.id、special Trash Node、path-based resource identity和bare node UUID。

本决议不授权当前任务修改这些区域。

## 18. 测试轮廓

1. typed ref round-trip：WorkspaceRef/NodeRef/ResourceRef/AnnotationRef闭世界JSON，unknown/missing/duplicate/null/case/version/kind均拒绝；nested owner Workspace也必须绑定。
2. token grammar：n1与r1 canonical source token；r1 owner由exact-source Document结构推导；malformed、foreign workspace、wrong owner、跨Node ambient paste、bare UUID拒绝。
3. equality性质：move/rename/reorder/edit保持NodeRef；copy/import/fork fresh；leaf UUID跨domain不相等。
4. allocation/OperationId：Workspace-local `(WorkspaceId,OperationId)` 的unseen→rejected或unseen→planned→committed|terminal_failed；当前授权/authority/ledger连续性gate通过后same-fingerprint rejected/terminal replay返回byte-equivalent bytes、planned recovery复用reservation；gate失败遮蔽交付但不改decision；different fingerprint仅在同Workspace key内conflict；burned resolver为not_found且永不能live。
5. tree ownership：root、parent、ordinal、cycle、subtree delete、explicit child move；无live orphan。
6. owner-local：Resource/Annotation不能reowner；cross-owner ambient mismatch拒绝；owner trash/purge cascade。
7. locator：element/range/region exact revision；revision change、kind/span mismatch均stale；无fuzzy fallback；locator disclosure gate在任何coordinate lookup之前。
8. anchor：same-document unique resolve的v11 outcome必需resolvedLocator且owner/revision/span闭合；missing stale；duplicate invalid(anchor_ambiguous)并回显canonical requestedLocator；cross-document拒绝；owner/entity已resolved/live而无locator disclosure时三者全为not_visible且禁止resolvedLocator；非live owner仍先传播原entity结果。
9. lifecycle：live->trashed->restore preserve；trashed->purge tombstone；mixed restore排除已live/tombstoned成员；tombstone终态且ID永不复用。
10. resolver矩阵：v11三个outcome的每个status required/forbidden字段；只有decoded anchor_ambiguous回显requestedLocator，其他invalid不回显raw payload；单一canonical优先级和跨五表面一致；authority/integrity conflict统一workspace_unavailable。
11. privacy：`mayDiscloseEntityState=false`时live/trashed/tombstoned/not-found全部not_visible；owner/entity已resolved/live而`mayDiscloseLocatorState=false`时全部locator位置状态不可区分且无lookup，非live entity结果仍在前一门传播；content snapshot仍独立授权。
12. reference preflight：trash形成suspended并receipt枚举；restore删除dangling slot、purge删除/改写live inbound、trash Annotation原子reparent reply均在同一v11 receipt以rewrittenReferences/structuralChanges证明同成同败；identity-preserving reply仅structural单表示；未处理live inbound拒绝。
13. same-Workspace copy：source只含live closure；typed identityMap覆盖Node/Resource/Annotation且owner-map闭合；internal rewrite、external live用preserve_exact、external already-trashed用preserve_suspended且receipt rewrite+suspended双证据；owner-changing r1不得external-preserve，trashed target必须delete/replace/reject；same leaf destination collision不得rebind；Annotation omit沿reply closure。
14. cross-Workspace transfer：全部fresh；全部typed refs/locators的unmapped source target include/transform/reject；target成功/source失败由transfer outcome准确呈现。
15. Workspace continue：verified continuation cut完整含allocation/burn/tombstone ledger；stale backup或unknown later commit不得continue；new AuthorityInstance与duplicate writer fence验证。
16. exact Workspace fork：live+trashed全部fresh refs、保留两态且complete map；resultLifecycles逐mapped entity证明两态，initialPlacements/trashPlacements对mapped Node精确分区；trashed subtree、Trash forest root、original parent已purged的unavailable location均有canonical request/receipt；omittedObjects必空，任一unreconstructable Annotation/member导致whole reject；旧tombstone不成为new allocation history。
17. raw copy：未选择mode前只读/reconciliation。
18. import/export：ordinary、partial identity-bearing、full-workspace snapshot class/mode矩阵；ordinary单root/forest及同parent连续destinationOrdinal block；partial bundle不得fork；IR ID不进入target；format re-import fresh；same-node history restore preserve；restore-as-new fresh；unknown组合拒绝。
19. corruption：duplicate ID、active+tombstone collision、missing owner、partial Trash、cycle进入reconciliation；普通resolver统一workspace_unavailable，operation统一workspace_integrity_conflict，无rekey/reparent。
20. Query/evidence负向：locator、row handle、provenance、action evidence、operation/history/snapshot/audit ID不能coerce为entity ref。
21. D5模拟：独立RecordRef即使UUID相同也不能进入D2/D3 Node decoder。
22. receipt round-trip：v11全部closed mode/variant、十二数组、reference-result union、resultSlot/context双侧验证、四类Node lifecycle placement、ownerSubject物化、duplicate/cross-array冲突拒绝；authorized committed replay byte-equivalent。
23. transfer outcome：v11必需source/target WorkspaceRef并分别绑定Workspace-local OperationId；observationSequence immutable递增、MAX terminal边界、not_attempted→pending→terminal单调状态；任一状态都不改变两个独立receipt或声称原子提交。
24. Workspace destruction：任何旧entity ref只得workspace_unavailable，不能泄露tombstone/never-known差异或复用WorkspaceId。
25. offline projection：cache必须non_authoritative并携带snapshot/auth generation；mutation前Core重解析。
26. error parity/property：invalid reason全覆盖unknown/missing/extra/duplicate/null/malformed/wrong-domain多错；reason→family唯一映射；operation family全表总序，owner_mismatch优先于invalid/stale locator、unauthorized优先于integrity/OperationId/not_found。
27. retirement门：仓库扫描最终删除bare UUID跨API、path-or-id、special Trash Node、source identity authority、resource path ref和fallback resolver。
28. receipt/error closed wire：source/destination owner/artifact/issuer-target authority/continuation字段矩阵、十二数组、document/annotation preimage/result slot expected union、result allocation/lifecycle/live placement/trash placement、structural change、omitted reason、canonical排序，以及preflight/recorded/terminal三error disposition全部验证。
29. owner-local copy：Resource/Annotation同owner与跨ownercopy均fresh；receipt必须有destinationOwnerRef且禁止虚假Node mapping；请求source恰映射一次且禁止omit/空no-op；Annotation target/reply必须在destination唯一重建或whole reject/reparent；source occurrence不隐式改写。
30. fork引用闭包：live/trashed成员映射完整且omitted为空；source tombstone、not_found、owner-mismatch和closure外foreign target全部transform或whole reject，不能进入target payload。
31. locator lifecycle传播：owner/entity的trashed/tombstoned/not_found/not_visible/workspace_unavailable/invalid优先于stale；anchor duplicate固定为invalid(anchor_ambiguous)。
32. OperationId crash/replay矩阵：unseen/rejected/planned/committed/terminal_failed × authorized/unauthorized × authority available/unavailable × generation unchanged/changed × same/different fingerprint全积；每格单一primary outcome，未授权不读saved bytes，ledger decision不变；同Workspace相同ID的mode/body/generation变化conflict，不同Workspace相同ID独立合法。
33. suspended copy partition：external trashed NodeRef link使用preserve_suspended并在rewritten+suspended双列；live Node + trashed Resource + owner-derived suspended occurrence则copy只能rewrite到mapped fresh local ref、delete/replace或reject，绝不ambient rebind。
34. AnnotationTarget穷举：D2五个kind逐一round-trip；Document/Resource owner必须等于Annotation owner；target/reply slot wrong-family与cross-owner canonical reject。
35. slot address穷举：document slotKind三值与annotation slot两值各自唯一expected union；from/to兼容；裸range、wrong slotKind、非唯一source span拒绝。
36. purge prestate：live→entity_not_live、trashed→committed tombstone、tombstoned→entity_not_restorable；UI连续trash+purge仍保留两个OperationId/receipt且不宣称原子。
37. per-mode receipt golden：每个mode/variant一份v11 canonical bytes和字段翻转negative cases；fork allocated含WorkspaceRef且omit为空、resultLifecycles覆盖map、live/trash placements分区，copy allocated恰等map.to，ordinary create/import的new subject逐项映射，continue十二数组空。
38. reconciliation privacy：duplicate writer/live+tombstone/missing owner在无披露权时not_visible；有权普通resolverworkspace_unavailable；repair detail仅独立受权channel。
39. D2 immutable binding：final D2 target SHA与D2 v2 §5.10 Annotation target/outer grammar一致性测试；本amendment绑定D2 target SHA `873D2487AC27C7C579D64C14CAB106215FBD462286416860225F2F404BC85137`、D2 wire v2与outer grammar，历史上曾把D3 wireVersion升级为9，本次wire11仍绑定相同D2 v2；任何后续D2升级仍必须触发新的D3 wireVersion review。
40. canonical request union：所有mode intent required/forbidden字段、null/extra/unknown拒绝；plan十数组及mode矩阵穷举，proposal/allocation outcome/request/receipt/error/resolver/transfer wireVersion均为11；Candidate v1–v18中的旧wire从未冻结或发布，明确撤回且不得接受。
41. fingerprint property：D3-CJ/3 golden bytes与SHA-256向量；十数组集合置换得到同fingerprint；任何规范相关字段或完整proposal byte改变得到不同fingerprint；top-level OperationId自身不进入fingerprint。
42. owner-changing rewrite goldens：Node copy、Workspace fork、partial import、standalone Annotation copy均同时绑定fromSource/from与toSource/to或deleted result context；两侧独立owner/revision/expected-union验证。
43. lifecycle receipt goldens：restore selected payload缺exact rewrite固定stage14 identity_map_incomplete；purge closure外live inbound未处理与trash Annotation reply closure未处理固定stage15 inbound_reference_conflict；重复reply双表示固定stage1；三者各只有一份committed receipt且无partial state。
44. exact fork partition：`allSourceLiveTrashed == identityMap.from`，`omittedObjects == []`，root/member不可omit；unreconstructable target、reply或locator whole reject。
45. owner total order：cross-owner claimed ref在存在/not-found/trashed/tombstoned四态均不访问key并固定owner_mismatch；record/key owner损坏固定workspace_integrity_conflict。
46. resolver planned projection：never-known/burned/reserved-planned-uncommitted在有披露权时均not_found，且outcome不含reservation或OperationId差异。
47. invalid/stale goldens：静态range invariant、profile token lexical、same-revision wrong slotKind、旧revision/不存在span分别唯一映射到invalid_identity_ref、invalid_identity_ref、invalid_locator、stale_locator。
48. saved decision retrieval after generation change：只有已验证continue/failover ledger continuity与当前授权可取回rejected/committed/terminal_failed旧bytes；同一旧token碰到unseen key在stage5固定identity_authority_unavailable且read=1/write=0/reservation=0；无连续性为stage4 authority unavailable，撤权为not_visible，均不改decision。
49. Workspace bootstrap proposal：Core-issued create/fork proposal v11 closed round-trip；issuer/target AuthorityInstanceId分域、allocationIntent family replacement/winner、caller-chosen/forged/mismatched/burned/duplicate claim；lost-response exact retry只命中同一winner；unclaimed/terminal-failed/destroyed WorkspaceId与target AuthorityInstanceId永不reuse且resolver为workspace_unavailable。
50. subject-bound payload：A/B payload互换改变fingerprint；A/B同digest保留两条不同subject binding；preimage/result/source_artifact role互换改变fingerprint；static subject cardinality在stage1，authoritative逐主体coverage少一/多一固定stage14 identity_map_incomplete。
51. request canonicalization：十数组所有permutation均canonicalize接受并得到同D3-CJ/3 bytes/fingerprint；static duplicate/matrix错误stage 1，Workspace role stage 2，authoritative coverage stage 14/15；不得存在按输入顺序拒绝或授权前读取closure的实现分支。
52. request per-mode algebra：逐mode exact/empty/cardinality/action/artifact/Workspace-role矩阵；move intent+structural重复、foreign preimage放错role、preserve_exact/preserve_suspended/suspend_with_result/lifecycle-only四态混用分别按stage1/2唯一拒绝且不写ledger；authoritative exact closure只在stage14/15。
53. Annotation ABA：同AnnotationRef从A1→A2→A3且target bytes回到原值时三个annotationRevisionToken均不同；fromSource/resultContext/toSource分别绑定准确token；旧token固定stale_locator。live target delete固定invalid request/receipt，reply_to delete合法。
54. error total order Cartesian：family table/stages/ledger matrix同源；ordinary existing+different fingerprint只在stage4 generation/continuity通过后到operation_id_conflict；proposal flip按stage3/4/P1/P2/TL1/TL2/5；authority/custody unavailable或continuity未证明与reachable-integrity症状并存时固定identity_authority_unavailable，只有连续可达后才可返回workspace_integrity_conflict；unauthorized遮蔽后续。
55. invalid partition coverage：unknown kind+extra/missing、known kind+extra、envelope wireVersion、common integer边界、empty opaque token、D2 source token、root trash/purge/move/reorder、semantic ordinal逐一恰一reason/family；non-empty non-ASCII opaque token如`rev:α`lexically valid并只由后续state决定resolved/stale。
56. canonical evidence bytes：allocation issue/replace、proposal、allocation success/error outcomes、request/receipt/error/Workspace-Entity-Locator outcomes/transfer全部有D3-CJ/3 exact bytes/hash golden；所有nested object服从UTF-8 key order；独立实现首份bytes一致，saved replay完全byte-equal，noncanonical bytes不得作为authoritative evidence。
57. unique stage source：从§13单表生成family/stage/ledger tests；unauthorized+missing mapping不读closure；proposal P1/P2在target ledger前；stage1–4/P1/P2/stage5 conflict无新decision，stage5 unseen后的stage7–15才写rejected。
58. proposal family concurrency：initial response完全丢失时byte-equal issue retry返回同一proposal；同一family的Pn response延迟、显式Pn+1 replacement、两者并发claim全排列；线性化后至多一个claimed/activated，late predecessor对successor零effect。
59. proposal saved replay：逐字段翻转nested operationId、mode、targetWorkspaceRef、issuer/target authority、principalAudienceToken、source/cut、issuanceToken；每格按stage1/2/3/4/P1/P2/5精确裁决，伪造/audience mismatch绝不因hidden saved ledger改变结果；byte-equal真实proposal才可取回saved bytes。
60. expectedAuthority exact matrix：每个operation mode × existing/create/continue × token absent/present × unseen/rejected/planned/committed/terminal_failed × generation current/older-continuous/unrelated-unproven全积；每格只有表中唯一outcome，并断言ledger read/write/reservation计数。
61. fresh-result evidence：两个同kind new subjects A/B交换payload、result allocation/lifecycle/live placement/trash placement中的任一个都改变fingerprint/receipt或stage14拒绝；`allocated`排序变化不能改变subject映射；每个fresh live Node有唯一initialPlacement，fork fresh trashed Node有唯一trashPlacement且绝不双列。
62. symbolic grammar：D3-Symbolic-Result/9的B/M/N/E/S framing、NUL/LF、positive count/baseLength、start/end partition、双length E、explicit full resultSlot、octet length、zero/raw merge、overlap/gap、missing/duplicate slot、nested owner integer与S subject/container equality跨Rust/TypeScript/.NET相同；segment顺序只按真实span，logical ordinal绝不由位置推断，concrete slot不得留在B。
63. integer boundary：每个integer field对`-1,-0,0,MAX,MAX+1,10^100,1.0,1e0`逐一唯一reason/family；allocation/transfer/forest/symbolic派生值覆盖MAX-1/MAX/MAX+1且不得wrap；lone surrogate固定invalid_closed_shape。
64. artifact/omission closure：partial bundle使用copy_node_subtree固定domain_kind_mismatch；partial import在stage3/4/9/14及所有stage的source-authority read count为0；Node omit固定stage1；owner-local omit/ref/reply闭合；任何自动reparent禁止。
65. Annotation reply single representation：identity-preserving P→Q、P→null、null→Q只在structuralPlan/Changes与唯一S；null postimage仍占用`replyTo` value的ASCII `null` structural span但不是resolver reference slot；同时提交reply_to rewrite固定stage1；漏structural、stale token、cross-owner、cycle分别唯一stage/family；fresh-copy reply仍由rewrite/postimage evidence验证。
66. placement canonicalization：两个及以上resultPlacementPlan/resultLifecyclePlan/resultTrashPlacementPlan entry的全排列均canonicalize为subjectCanonicalKey顺序并产生同request/fingerprint；duplicate subject固定stage1；existing parent只按wrapper kind+ref+ordinal typed equality，异构JSON不得byte-compare。
67. rejection/terminal wire：live purge rejection→later trash→exact retry始终replay recorded_rejection；stage1–4、proposal P1/P2、TL1/TL2及stage5 existing-key conflict不新建decision；stage5 unseen后的stage7–15失败才写recorded rejection；planned abort只得terminal_failure/identity_commit_aborted并burn，三种disposition字段矩阵互斥。
68. restore kind matrix：Node original/explicit、owner-live Resource/Annotation no-placement、owner-trashed owner_restore_location逐格正负；subject-kind wrong variant、ignored field、owner placement双表示stage1，owner prestate/variant不相容或owner tombstoned stage9；receipt placement evidence唯一。
69. allocation control total order：issue/replace的malformed、unauthorized/wrong principal、authority unavailable、integrity、same-request replay、different issue bytes、retired、claimed、stale sequence、MAX exhausted全笛卡尔；每格唯一outcome/family/effect，成功及A7–A11失败exact retry byte-equal。串行golden `replace_absent → issue(sequence=0) → same replace(expectedSequence=1)` 精确得到 `allocation_not_visible, issued, allocation_sequence_conflict`，写计数`0,1,1`、proposal计数`0,1,0`；并发线性化的每个合法序列也按current state重算A1–A5，绝不历史重放A5。
70. nested golden corpus：refs、PayloadSubjectKey所有variant（含ownerSubject）、M/N/E/S resultSlot、result-only/preimage/lifecycle-only plan、created/preimage receipt、ReferenceSlotAddress、Document/Annotation result context、五类structural change、三类restore location、placement、proposal/audience/allocation outcome都必须由单一生成器产生机器可读exact bytes/length/hash/expectation；正文不维护第二份exact bytes。
71. proposal authority split：issuer A/target B逐字段proposal→request digest→receipt equality；replacement/retire/terminal_failed burn B、commit激活B、continue另mint C；issuer/target交换、复用或receipt漏字段全部拒绝。
72. proposal authenticity/audience non-consumption：篡改issuanceToken、token-body或principal audience先到，P1必须byte-equal preflight且target ledger zero-read、family/IDs不变；genuine proposal仍可commit。与replacement/late predecessor全排列中，P2失败对successor零effect，只有current proposal在planning CAS或自身recorded failure可claim/retire/burn。
73. existing suspended golden：external trashed Node link的preserve_suspended canonical entry固定length/hash；pre/post target byte-equal且两态都suspended，缺rewritten或suspended任一证据stage14/15唯一失败；Resource r1同形状固定不适用。
74. fork Trash graph property：任意live/trashed/tombstoned历史下，mapped live Node==initialPlacements subjects、mapped trashed Node==trashPlacements subjects、二者不交且并集等于mapped Node；trashParent闭于trashed set或null；siblings ordinal连续；original parent purged时必须unavailable且fork仍可提交。
75. ordinary import forest：1/2/N roots，request数组全排列、artifact输入顺序翻转仍按subject key分配同一连续ordinal block；duplicate/hole/second parent静态拒绝，parent容量越界stage10。
76. fork receipt lifecycle：每个identityMap entry恰一resultLifecycle，同subject/ref/state机械绑定；漏项、重复、state flip、owner-local ref错配或从snapshot外推而receipt缺证据均stage14 identity_map_incomplete。
77. result-slot bijection：0/1/N slots、同target重复slots、M/N/E混合、delete context分别验证plan↔symbolic↔receipt一一对应；交换、漏项、多项、wrong slotKind/ordinal均stage14失败。
78. N+E create/import：create Node/Annotation与ordinary import至少各一golden同时含fresh和existing result-only refs，无preimage仍可闭合；旧preimage-only实现拒绝不支持的当前wire11；历史v10也不能靠猜测接受。
79. new owner binding：按mode与primary/member的联合矩阵检查existing/new/mapped owner；create Resource/Annotation的primary只取destinationOwnerRef，member可用合法existing/new owner；ordinary import只取同graph声明的new Node owner。mapped owner只用于适用identityMap模式，不能由generic subject union放行；wrong/missing/cyclic ownerSubject、receipt owner swap、no owner occurrence均拒绝。
80. restore structural variants：live→trash、trash→live、trash→trash reparent、live→live各正负golden；original same live location仍必须trash→live variant，wrong variant/from==to拒绝。
81. final-index placement property：单move、same-parent reorder、cross-parent compound、restore+move、input permutation在模型算法下唯一；duplicate final index/out-of-range唯一invalid_ordinal。
82. Trash splice property：对已证明canonical的valid-state输入，new roots append、parent restore/purge递归splice、multiple removals、root/child ordinal compaction及receipt完整性保持既有golden；另对live sibling与Trash sibling分别注入duplicate和gap，在trash/restore/purge三种operation、normal/`-O`/`-OO`下全部于stage 4得到`workspace_integrity_conflict + preflight_rejection`，并断言target OperationId ledger content read/write=`0/0`、allocation/reservation=`0`、payload/placement/lifecycle mutation=`0`且无receipt。`RefKey`只排序repair diagnostics，不能使任一mutation通过。
83. D3-CJ/3 strings：U+000B/U+001A/short escapes/quote/backslash/slash/U+2028/U+2029/non-BMP/normalization-distinct exact bytes/hash；uppercasehex或over-escaped evidence拒绝。
84. proposal oracle instrumentation：forged token、wrong audience、stale predecessor在target ledger absent/unseen/rejected/planned/committed五态的target read/write count均0且error bytes equal；genuine current proposal随后仍可用。
85. allocation family OperationId scope：同family不同OperationId冲突；不同family复用同UUID且不同target Workspace合法；实现不得要求global registry。
86. counter exhaustion：A11保存byte-equal outcome且family active；transfer MAX pending无效/MAX terminal后无next；ordinal/symbolic MAX+1对应指定family；宿主64位边界无panic/private status。
87. reference lifecycle affected coverage：target live→trashed的`resolved→suspended`、source restore的`non_live_source→resolved|suspended`、target restore的`suspended→resolved`在bytes/revision unchanged时必须使用lifecycle-only plan并列同pre/post receipt transition，request fingerprint随entry变化，payload read/write/revision计数全为0且无resultSlot；source改变时使用相应result action并仍列transition；完全无关slot不列。
88. payload role/subject matrix：existing preimage/result合法，mapped/new仅result，artifact仅source_artifact；每种wrong role/kind在stage1唯一拒绝。
89. TargetLedgerGate：create/fork按stage4→P1→P2→TL1→TL2→5，ordinary（含continue）按stage4→TL1→TL2→5；未激活、decision_bound_rejected、planned、committed、continued/failover状态逐一验证custodian所有权与原子移交；TL1不可达/连续性未证明遮蔽TL2 integrity，create/fork的P1/P2失败时custody和target ledger读取计数都为0；ordinary不同fingerprint遇TL1 unavailable必须先返回identity_authority_unavailable而非读取ledger后返回operation_id_conflict；older-continuous token对existing same fingerprint可replay、对unseen固定stage5 authority unavailable。
90. recorded-decision CAS：两个fingerprint从unseen并发到recorded-rejection或planning boundary的所有交错；至多一个winner。loser零写/零burn/零retire，并按create/fork从P2、ordinary从TL1重新进入，最终只得到same-fingerprint replay或相应workspace_identity_conflict/operation_id_conflict。
91. D2 owner-local non-regression：Node A authored state直接嵌入ResourceRef(owner=B)在live/not_found/trashed/tombstoned四态均于typed lookup前拒绝；D4关系字段不能把完整ResourceRef当作绕过。合法替代只有NodeRef(B)或copy-to-fresh-owner。
92. v11 cutover：同一envelope的wireVersion 0..8及12逐一unsupported_wire_version，新决议只接受11；v9/v10仅走§4.1.3a保存决议重放/恢复；resolver、allocation、operation、receipt、error与transfer不得各自漂移版本。
93. creation graph completeness：create Node N0+member N1、Resource R0、Annotation A0，以及create Resource/Annotation的primary属于既有P、member Node N及属于N的member Resource/Annotation两组完整图，均按freshSubjects、payload、Node placement、ownerSubject与resultAllocations形成exact bijection。primary Annotation target仍须属于P；任何undeclared/duplicate/unbound member、fresh target template丢variant事实或从分配顺序猜owner都拒绝。
94. Annotation postimage：Value/3五target、三purpose、target-only、target+reply、P→Q、P→null、null→Q、explicit slot-keyed adjacent non-B、fresh target template物化及identity-preserving S逐一round-trip；S与reference plan双表示拒绝；A1→A2→A3 target bytes ABA不允许旧revision token重放，targetStatus和D6 token不进入projection hash。
95. comparator registry：每个request十数组与receipt十二数组至少三个entry的全排列经唯一rank/tuple排序后bytes相同；delete context对rewrite slot、fresh target各variant、structural change五variant和null/value分支都不使用absent/null sentinel、完整JSON或宿主顺序tie-break。另以rewritten与deleted各一组longest-common-prefix bytes向量故意让postimage key逆序，证明ReferenceResultKey仍先比较fromSource/from，再比较toSource/to或resultContext；duplicate result slot另行拒绝。
96. artifact mapping：ArtifactObjectKey跨part/object的全排列得到同key→subject→resultAllocation join；相同payload digest不合并object，不同provider/文件名/输入顺序不改变identity，duplicate/hole/missing join固定拒绝。
97. 完整wire corpus：旧v9的537继承fixtures、28 amendment fixtures及其原运行/摘要保留为历史证据。新v11需要新增十数组、C完整carrier/typed根、S fresh reply、管理组合及legacy-replay版本矩阵；不能用旧输出计数或替换版本数字宣称新分支通过。完整跨语言产品conformance由实施阶段实际运行并绑定真实结果。
98. nested Trash simultaneous removal：roots `[X,G,Y]`、G children `[P,Q]`、P children `[C0,C1]`、removed `{G,P}`只得`[X,C0,C1,Q,Y]`；receipt精确列C0/C1/Q reparent和Y ordinal shift，输入removed顺序置换不改变结果。
99. D2 Node-only authored target：create/copy/import/fork × node_link/citation × existing/fresh各正负；NodeRef accept，AuthorAnchorAddress在typed lookup前固定stage1 domain_kind_mismatch，D2 bytes不变。
100. locator token exactness：element、range start/end、resource region跨实现byte-equal；无paddingbase64url、magic/NUL/CJ body重编码；padding、alternate alphabet、wrong edge、owner/revision mismatch、registry handle全部invalid；range pair双向唯一。
101. locator lifecycle：continue同owner/revision保留token；copy/fork/import fresh owner或revision重签；旧token stale或misbound，绝无hidden registry continuity。
102. proposal digest lexical：create/fork lowercase 64-hex golden；uppercase、base64、63/65位、raw digest固定stage1；同OperationId不同lexical形式不得进入ledger conflict。
103. existing materialization/preserved：existing_entity_subject不依赖preserved直接物化；purge B+rewrite A、trash P+reparent C、restore A+delete slot中所有modified existing entities恰列一次，未改变entity不得列；missing/extra分别拒绝。
104. standalone owner-local copy：same-owner Resource copy+occurrence rewrite的source/mapped payload及existing Document preimage/result/revision evidence完整；cross-owner foreign Document edit、未成对binding、隐式未声明occurrence修改拒绝；copy_annotation structural S同形。
105. 历史v9/v10 corpus保持原记录；不得机械替换版本号称为wire11运行。当前wire11按完整closed union、十数组、C/Q及相邻边界独立验证。
106. rename domain boundary：wire mode `rename_display`固定unsupported/invalid；D2 title source action、D6 path/file rename、D8 UI label与Resource occurrence caption分别只改变自身authoritative field且durable ref保持。
107. lifecycle-only fingerprint：相同OperationId下漏entry、source/target/pre/post任一flip得到不同fingerprint；exact retry byte-equal；receipt漏/多referenceLifecycle transition或出现orphan resultSlot/payload固定拒绝；`non_live_source→suspended`与`suspended→resolved`各有exact vector。
108. ICS foreign identity：两个SourceBinding有相同UID必须是两个foreign keys且不得自动合并；同SourceBinding+UID title/path改变仍通过OriginBinding upsert同Node，不新建Node；UID/RECURRENCE-ID/SEQUENCE/DTSTAMP/etag逐一不得通过NodeRef decoder。
109. recurrence boundedness：无限RRULE、长期RDATE集合与bounded series preview都默认产生零occurrence Nodes；materialized recurring series至多一个series Node；RECURRENCE-ID override默认仍是series-owned value/optional future Record，不能暗建Node。
110. materialization intent shape product：对`initial_import|adopt|promote|managed_copy × ForeignIdentityKey present/absent × managed locator present/absent`的16个shape逐格验证，只有§3.1四个admitted shape可进入下一门；其余12格固定`materialization_intent_mismatch + preflight_rejection`且零allocation/binding write/mutation/receipt。特别证明FIK present+locator absent在声明`initial_import`时不被adopt/promote classifier误拒绝，FIK/locator同时出现对四verb均reject。
111. materialization state product：`initial_import`与`adopt`逐格覆盖`never_bound|active_live|active_non_live|retired|conflict`；`promote`只接受`not_applicable + resolved managed locator`；`managed_copy`只接受`not_applicable + live managed source`并覆盖non_live/unresolvable rejects。每格精确核对outcome、allocations、activeBindingWrites、authority/provenance effect；retired只允许新的显式Adopt重建sole active binding，initial_import/ordinary re-import均零allocation。
112. deterministic re-import：同ForeignIdentityKey的SEQUENCE上升、相等、倒退、离线并发、取消与source deletion逐格得到upsert/no-op/conflict/unbind候选，不靠title/path匹配；active_live、active_non_live、ordinary missing、conflict与retired逐格验证allocation/binding write/auto-restore计数。ordinary re-import miss或retired、scope不唯一或版本不可比较均fail closed；second content只声明`managed_copy`且activeBindingWrites=0；跨SourceBinding同UID不merge。
113. authority mode：subscribe/sync、bounded `initial_import`、`adopt` selected foreign object的相同ICS输入得到不同authority/provenance但始终独立identity；managed Document occurrence的`promote`不接受ForeignIdentityKey，managed live Node的`managed_copy`不建立OriginBinding；解绑、Weftext删除、source取消、双向编辑与offline conflict各维状态互不代替。
114. VCALENDAR component gate：VEVENT、VTODO、VJOURNAL、VFREEBUSY、VTIMEZONE混合输入不得批量获得Event Node identity；VFREEBUSY/VTIMEZONE在subscribe/sync、initial_import、adopt及任何未来authority mode均精确零Node，即使显式mapping，也只能路由非Node value/control/future D5 candidate；VTODO→Task与VJOURNAL→time note在D4/D9 mapping preview前不执行；VTODO→Task成功映射只能创建fresh ordinary Node并声明exact `tasks/task` Facet，Task复用Node identity且checklist item无独立durable identity。
115. terminology uniqueness：Lexicon stable concept ID、canonical中文名、canonical英文名各自唯一；machine registry与当前42个entry集合精确相等，owned wire/API/code/CLI/UI/locale name只能映射一个concept；重复bare term跨concept或仅有字段标题而registry mapping损坏都使Gate fail。
116. ontology negatives：[D3-TERM-EXCLUDE:quoted_counterexample]`note|item|object|instance`逐一不能成为开放domain kind/ref；Node/Document/Entity/DocumentOccurrence/DerivedOccurrence的class/instance口语不能改变identity algebra。
117. reference vocabulary：Typed Reference、Reference、Node Link、Citation、Relation的wire/code/locale全排列只映射各自concept；将citation叫relation、NodeRef叫reference fact、generic ref叫link都由受控gate拒绝；自然语言引用不扫描。
118. resource vocabulary：Resource UI attachment role仍映射Resource concept；[D3-TERM-EXCLUDE:quoted_counterexample]`AttachmentId|FileRef|BlobId|path-as-ref|URI-as-resource`受控identifier拒绝，用户文件名与historical evidence保留。
119. ownership vocabulary：Owner/Authority/Source/SourceBinding/Provenance/OriginBinding每对互换的wire field/code type/locale key均拒绝；裸中文“来源”只映射Source，Provenance唯一controlled label为“来源证据”，任何context复用裸label都使terminology Gate fail。
120. operation vocabulary：copy/fork/continue/move/import/adopt/promote逐项golden验证fresh/preserve/binding effect；[D3-TERM-EXCLUDE:migration_deletion_note]`clone` controlled alias拒绝；subscribe/sync/connect不能暗建Node、选择authority或执行import。
121. terminology migration gate：分类顺序固定为exact owned name→global retired identifier→expected-concept/type mismatch→unknown/error；concept-relative semantic non-alias不进入flat denylist。扫描只遍历schema/wire/public symbols/CLI/locale/normative headings；Document、Annotation、review evidence、migration note、quoted counterexample与third-party field只有精确closed typed span可排除，JSON/table/heading始终扫描。每个controlled hit必须给actual/expected stable concept ID（若适用）及唯一replacement/deletion target；合法owned name、negation或ICS token都不得被裸token误拒或豁免同线违规identifier。
122. terminology and naming gate：逐entry核对stable concept ID、canonical中英文term、owner/layer、排除边界、wire/API、code convention、CLI/UI/locale、允许简称、semantic non-alias、全局retired identifier、例/反例与migration target；当前42个concept（39继承及3新增）、D1/D2 frozen entries、ICS `ForeignIdentityKey`/provenance/derived occurrence边界与受控名称集合必须保持一词一义。本次冻结以independent product-conformance evidence证明Candidate、Lexicon、wire示例与所有受控surface双向一致；删除或协同移动entry/name、跨concept owned-name冲突、retired name与owned name相交、同retired name多replacement、expected-concept旁路、给no-wire concept暗加identifier、陈旧产品代次或依赖缺失必须fail closed。负向gate只扫描受控identifier/schema/wire/symbol/locale key，ordinary prose、用户内容、第三方format、历史evidence、migration/deletion note与closed typed counterexample不在禁止面。具体证据载体、sandbox、Unicode数据、scanner、mutant、manifest、builder、validator与replay算法是review-infrastructure-only evidence，不由D3冻结，也不得成为下游产品实现前提。
123. Annotation outer materialization：独立`materialize_annotation_outer` oracle从Value/3、receipt分配的AnnotationRef/owner、committed revision、target exactness/lifecycle唯一产生完整D2 Annotation v2 closed object；五个accept vectors联合覆盖五target、三purpose、null/non-null reply、resolved/stale与live/trashed，且顶层字段恰为`wireVersion,kind,annotationRef,ownerNodeRef,purpose,replyToAnnotationRef,target,body,suggestion,targetStatus`。六个negative分别把outer `wireVersion`、`kind=annotation`、`annotationRef`、`ownerNodeRef`、revision、`targetStatus`注入Value/3并固定materialize reject；ambient owner或客户端字段永不被读取。
124. mapped reference evidence-origin prestate：copy/fork的`preserve_exact|preserve_suspended|map_target|rewrite_to_existing`各有完整plan→M/E segment→rewrittenReferences→referenceLifecycleTransitions join；mapped result container为fresh identity仍按typed preimage source lifecycle使用non_live_source/resolved/suspended prestate；postimage仍trashed时不列live graph transition。每向量只把prestate改为absent固定stage14 identity_map_incomplete。
125. authority-generation exact product：unseen/rejected/planned/committed/terminal_failed × current/older-continuous/unrelated-or-unproven × same/different或明确not_applicable形成closed 27-cell key set；每格由独立transition function导出outcome与ledger read/write/reservation计数，删除/重复/改一cell均失败。ledgerReads计入CAS compare，故unseen/current plan-and-reserve为2；older unseen固定stage5 unavailable；unrelated任何existing state在stage4零ledger read；planning CAS generation-change零reservation并restart。
126. OriginBinding lifecycle partition：initial_import仅never_bound分配并写sole binding；adopt在never_bound或retired可分配并写sole binding且retired保留audit；active_live零fresh；active_non_live零write/零auto-restore并遮蔽trashed/tombstoned/not_found/not_visible/unprovable；ordinary missing、retired与conflict均fail；retire-before-adopt与adopt-before-retire分别线性化；managed_copy仅live source fresh且activeBindingWrites=0。
127. LocatorResolveOutcome exact matrix：resolved anchor必需唯一DocumentElementLocator且owner相等；resolved non-anchor禁止resolvedLocator；trashed/tombstoned/not_found/not_visible/workspace_unavailable/stale逐一注入resolvedLocator必失败；anchor_ambiguous必需requested AuthorAnchorAddress，其他invalid禁止requestedLocator；generator corpus与独立mutation harness双重验证。
128. continue authority issuance：19-cell exact key set只包含transition-derived reachable inputs。caller-supplied ID与preflight零读/零reservation；pre-sample rejection为`2/1/0/0`，post-sample rejection为`2/1/1/0`；stable fresh为`2/1/1/1`。generation-loss与allocation-loss attempt均为internal restart `2/0/1/0`；generation-loss minimal completed restart为unavailable `3/0/1/0`。reserved-by-other/proposed/active/burned/destroyed stage-12 collision固定`2/1/1/0`并保存byte-equivalent rejection，saved collision replay固定`1/0/0/0`且不重采样。同OperationId planned retry复用，different fingerprint不二次分配，并发恰一winner，terminal burn永久no-reuse，commit receipt ID等于reservation。每格显式携带observation scope和transition input；generator/validator独立推导outcome与计数，删除、重复、手改任一cell都失败。
129. locator evidence independence：合法locator token仅改fixture name/expectation或删除decodedBody不得成为合法negative；四个malformed nested-owner token和一个unknown elementKind token即使提供actual decoded body仍由bytes拒绝；精确16-kind闭集、alphabet/padding/magic/NUL/closed fields/UUID/ref/span全部由一个decoder裁决。
130. Result/9 evidence independence：C0、baseLength0、empty/adjacent B、gap/overlap/missing last coverage、nested owner ordinal bool/string/MAX+1及S subject/container mismatch全部只由stream bytes失败；新增artifact/Resource container、resource_bytes/import_artifact payload、cross-family slot、wrong Annotation ordinal、M target kind、empty/non-string S revision与cross-owner S reply negatives；fixture metadata与name不得参与decoder分支。
131. Result/9 closed-reader matrix：Node的D3 authored slot只接受node_link/citation/resource_occurrence，另允许独立C carrier分段；Annotation annotation_value只接受target@0/reply@1；S只接受existing same-subject Annotation container与same-owner concrete/fresh物化reply，null removal通过。逐negative只改name/metadata/expectation不能把相同bytes变成accept。

132. D2 v2 binding：final D2 authority SHA、稳定ID、outer grammar与D2 wire v2必须与本target/corpus manifest exact-equal；旧D2 digest或candidate/package digest均拒绝。
133. lexical occurrence inventory：AttributeCarrierBlock/LexicalAttributeEntry逐项证明owner Document/current revision range，EntityRef/lifecycle/cross-revision/locator/Annotation target全部为否。
134. exact-source artifact binding：control/carrier/body/trivia同一完整hash；copy/fork/import/export逐格验证carrier/entry随source bytes而非作为payload authority或symbolic ref slot。
135. Task Facet operation cells：existing ordinary Node assign/remove exact `tasks/task`保持NodeRef；checklist promote fresh NodeRef、ForeignIdentityKey absent、activeBindingWrites=0、无OriginBinding/mirror；VTODO initial_import/adopt只在ForeignIdentityKey与same-decision sole binding闭合时fresh且无mirror。
136. Terminology companion gate：Facet/FacetId/attribute carrier block/lexical attribute entry与`facetId|facetMemberships|attributeCarrierBlocks|namespaceToken|rawEntrySource`必须由同代Lexicon拥有；Facet membership可在ordinary/template Node上，template + lexically valid non-`tasks/task` Facet接受而template + exact `tasks/task`拒绝；`NodeSpecialization::Task`只可作为retired negative出现。
137. Template boundary：Template meta-kind与Task Template target plan逐格证明fresh identity/ref rewrite，Template自身不成为Task；positive template + `project/project`与negative template + `tasks/task`必须机械闭合。
138. field target negative：field occurrence、carrier、entry及其range进入EntityRef/locator/AnnotationTarget均拒绝；未来durable field target需要独立formal reopen。

## 19. 本次replacement接受边界

本候选只正式演进D6所必需的创建/ordinary mixed import既有源组合、D4符号源物化和相应closed wire/阶段/回执，D1/D2及D4/D5领域决定不回退。D3身份、owner、四态、原阶段顺序、proposal/custody和账本决议语义继续适用；其新增精确行为见§4.1.3a与全文同步矩阵。

历史D6联合接受边界（仅记录）：旧v10与D4桥由D6流程接受，历史报告与当时停止点保持历史归属。当前D7联合候选的接受边界由本稿顶部状态及§21定义，不以旧接受票或当时停止点代替新评审。


### D6 revision03：mode/owner和Annotation证据的联合约束

creation的primary限制不传染member：create_workspace成员构成新Workspace内部graph；create_node/resource/annotation成员可用本mode合法existing/new Node owner，new owner必须完整声明Document、placement、allocation；primary Resource/Annotation仍只属于destinationOwnerRef，且其Annotation target不得越到member的另一owner。copy_node_subtree/fork/partial identity import的owner-local结果只由完整identityMap重写source owner；standalone copy_resource/copy_annotation的唯一mapped primary归destinationOwnerRef，不伪造Node mapping；它们的destination-owner existing compound沿原pair/slot/S矩阵且existingPayloadEdits为空。ordinary import identityMap为空，其fresh owner-local对象只能属于声明的new Node；含existing Node result pair不改变此规则。任何generic subject union都不能超越该mode admission。

existing Annotation的target@0与reply@1独立覆盖但同一最终Value/3、map和revision：target保留或合法改写均列完整reference证据，postimage source live时另列lifecycle证据；reply null→fresh、concrete→fresh、concrete→null仅S/annotation_reply_change，非null postimage且source live时另列唯一S-origin lifecycle transition；reply reference result与S双记拒绝。fresh new/mapped Annotation初始reply仍走reference plan，绝不成为S container。create_node/resource/annotation可由合法new member Annotation提供同owner fresh reply；copy_annotation可用唯一mapped Annotation作为destination-owner既有Annotation的fresh reply；copy_resource不存在可分配的fresh Annotation；ordinary import的new-only owner规则使其fresh imported Annotation不能作existing Annotation的同owner fresh reply。Node copy/fork/partial identity import不得增加existing S。缺owner声明、mapped owner无identityMap、primary owner错配、owner swap、无Document/placement/allocation、跨owner reply、cycle、target删除、slot family互换、双增revision、target遗漏、reply双记、S lifecycle来源遗漏都必须拒绝。测试79/93及S/slot/receipt覆盖均逐格使用此模式矩阵，不能统称existing/mapped/new全部模式都合法。

restore的联合边界：owner live、Annotation A trashed、mandatory target=owner Document、旧reply=P；restore A同时S改为同owner live Q，target@0与reply@1均使用non_live_source前态、resolved后态以及唯一最终A revision。旧reply=null时只有reply取absent，target仍为non_live_source。该操作修改Value/revision，禁止用lifecycle-only替代typed preimage reference/S证据；target/reply的current授权、完整payload pair和restore closure仍全部验证。

D4关系的受信读取使用RelationReadContext/2及RelationReadBinding/2，显式区分真实源与无源端点状态；D3 restore/purge/copy的原mode授权、最小tombstone和同decision边界不变。清理旧tombstoned target不用伪造其Document，实际新目标仍live/domain完整，symmetric purge仍须同事务显式清理全部incident facts。

## 20. D6观察授权与初始控制绑定

D6控制接口§13将本稿stage3的当前授权具体化：ordinary identity/lifecycle模式先证明target Workspace全域潜在约束可观察；online copy还证明source全域，offline artifact不访问source。全部潜在existing写权继续按§4.1.3a检查，不扩大真正写集。未经观察授权，两种隐藏事实状态均在stage3返回同一identity_not_visible，零ledger/作者范围读取；stage6仍空。prepared source/state/incidence/负范围及Calendar真实scope受固定scope覆盖，完整语义仍在stage14/15，planning/commit重验当前权利和版本；saved receipt/recovery也先过当前scope而非旧成功继承权限。

create/fork的stage3只按公开mode/外层target角色及当前issuer allocate_workspace给临时prepared_workspace资格，不先读family/target policy或生成stage12计划。原stage4、P1/P2、TL、stage5顺序不改；新意图在这些门通过后依family固定profile与真实map/cut产生完整WorkspaceBootstrapPlan，并在stage14/15验证。saved replay只核对原profile绑定和当前issuer/source资格，不重初始化已激活target。A2具体为issuer allocate_workspace且在family lookup前执行；host认证自身不足。初始creator policy、target Registry与Calendar配置/scope绑定同全部源、authority、receipt、custody一次激活，不要求尚不存在target的当前source_write/policy_admin。无新proposal/request/receipt member或fingerprint算法。普通period/copy的scope规则及显式管理解除依赖由D6接口§8.1、完整fork由§14提供；控制facts由原输入与cut唯一派生，不能作为任意companion写入。continue/failover保留现policy/control状态。

managed copy内部node scope若映射为本次fresh Node，其所需配置按D6接口§8.1从真实source配置确定派生、保留multiplicity并同copy提交；只建立该fresh scope配置，不改既有配置，不需先激活scope Node或第二次管理提交。完整prepared范围、target Registry和负inventory仍全部验证。

## 21. D7准备证据与保存定义转移（wire11新增）

本节属于完整D3替换正文。definitionTransfers是第十个必填plan数组；除copy_node_subtree、fork_workspace及identity-bearing import_new可非空之外，其余mode必须空。其静态closed shape、duplicate与mode matrix在stage1；真实source/目标D2 occurrence及slot coverage在stage14，不在授权前解析源。其唯一comparator为(subjectCanonicalKey(resultContainer),resultOccurrence)，相同key重复拒绝；输入数组任意排列先canonicalize。slot path比较逐segment，member text rank0按UTF8，array index rank1按D3Integer，公共prefix后短路径在前；同path重复或祖先/子孙重叠拒绝。

preparationBinding是唯一新增可选顶层成员，其存在和值均进入原fingerprint，OperationId仍排除；raw D3省略不能取得D7条件保证。D3 receipt仍原十二数组、不额外插入D7作者slot；新D7语义效果按原同decision companion保存。

以下为完整D7准备绑定和转移规范的同包合入正文；D7持有payload schema，D3持有stage/Result/ledger。两处同名条款必须逐字一致，任何修订需共同评审。
## 21B.1. 一份不可变准备记录

Core受管`PreparedActionBinding/2` exact semantic members为`kind,version,bindingToken,protocolOwner,operationId,workspaceRef,principalAudienceToken,action,canonicalCallInputs,definitionInputs,registryInputs,ruleInputs,sourceInputs,constructionInput,proposedInputs,dependencyProof,observationProof,budgetBinding,expiresAt,request,preview`。kind=`d7_prepared_action_binding`，version=2，bindingToken为独立tag=d7_preparation的D6 Token。其余语义类型如下，不能使用自由dictionary来遗漏证据：

- action为原完整ActionSpec；canonicalCallInputs是按实际调用路径排序的完整QueryCall数组，包含有效arguments/context，普通无Query动作为空；definitionInputs逐项是`{definition:DefinitionAddress,ownerVersion:SourceVersion,payload:text}`，保存实际解析wrapper/Query/View的完整source payload，不由调用方自报。
- registryInputs为原完整D4 RegistrySnapshot及RegistryBinding、完整展开所用定义；ruleInputs为原完整D4 RecurrenceReadContext/Binding及实际使用的其它已closed规则贡献。没有使用时对应空数组，不填假context。
- sourceInputs是原D3/D6源pin目录，每项含完整subject、SourceVersion、exact source/bytes binding及actual role；proposedInputs是同原D3 Result/9或D6完整源变换的已验证符号/具体输入。完整类型逐字引用所属D3/D6 closed类型，不重新定义Ref/Locator或允许任意JSON。
- constructionInput 是 null 或 D9 Templates §6a 的完整 TemplateConstructionInput/1。只有Core内置D9 node-template adapter可建立非null值；公共ActionSpec/d7_action_prepare不接受额外construction字段。非null仅用于D9指定的ordinary-import d3_operation或简单collection_create；完整输入pins、版本、参数、loss、sourceSubjectBindings独立重编译后须等于request及proposedInputs。sourceInputs原类型不变，不塞新的role或私有artifact variant。
- dependencyProof为实际完整正/负依赖，包括全部pre/post Query、定义解析、Registry/rules、authority/state/placement/ForeignBinding、授权与源Envelope元数据；observationProof为同包D6选定scope与当前潜在范围资格、逐constraint保持证明。原D6 budgetBinding包含已耗账户，不能在replay重新初始化。
- request为准备完成后返回的完整原D3/D6 request bytes；preview为效果规范的完整不可变语义清单、原字节pins及初始交付投影，包含适用conditional_source_change；不能把一个采样map当作确定后像，尚无identity reservation。交付handle重签发不改变语义清单、原request或此准备记录。expiresAt遵守D6 clock epoch/期限域，不从设备时间猜。

记录在返回任何prepared响应/token前原子保存并验证所有引用的pins齐全。bindingToken生成与request构造只有以下次序：先生成随机token（无作者/identity effect），构造包含该token的最终request，再保存完整记录及最小授权定位映射，最后返回；不把“request包含token、token需hash request”变成循环散列。完整相等用原canonical request与原规范对象/byte pins，不能以一个相同hash替代证据。

最小定位映射/2 exact {bindingToken,protocolOwner,operationId,workspaceRef,principalAudienceToken,observationScope,registryBinding,constructionReadRefs}，受保护且无作者值。constructionReadRefs是按D3 RefKey排序唯一的EntityRef数组，非模板为空；非null construction恰覆盖其inputPins、omittedAnnotations及构造所需的其他完整来源读取Ref，不含source bytes。Core记录版本选择/1或/2 decoder，不按字段缺失猜版本。D3 stage3与D6原授权步骤先读此映射，证明current audience、固定scope及这些来源的完整当前读取资格，再读完整record/ledger；missing/wrong-tag/wrong-audience或资格不足一律not_visible（D3映identity_not_visible），不泄露记录存在。此增加的是D9实际输入的读取检查，不授新权限、不改变原阶段先后。

## 21B.2. D3显式wire11承接

D3 wire11顶层在原wire10 exact members之外增加唯一可选`preparationBinding`，其exact值为`{kind:"d7_preparation_binding",bindingToken}`。此成员进入原requestFingerprint；绑定的规范原request包括它，顶层OperationId仍按原规则不进入fingerprint。D7发出的D3准备请求必须包含它；独立raw D3请求可以省略，但只能表示自身完整D3意图，不取得D7 postcondition、preview或保存策略证明。

stage1验证closed形状、Token词法与known kind；stage2验证原Workspace roles；stage3在原D3全域/issuer/source授权之外，按最小定位映射证明准备记录current audience与相同固定scope，原P1/P2/TL顺序不变。stage5只比较已包含preparationBinding的原fingerprint：同OperationId同D3 payload而不同D7条件必然使用不同bindingToken，不能混用同一saved request；不同fingerprint仍按原stage5冲突，不读取/回显原条件。

unseen请求继续原stages，stage14在原identity_map_incomplete检查之后增加唯一family `operation_precondition_failed`：已授权的绑定记录过期、其完整request与所收请求不相等、原D7 pre/post及非null construction完整proof与当前cut依赖不成立、原D7完整postcondition不满足，均recorded_rejection。不能证明pin/authority/连续性属于availability，按原availability路径不伪造确定业务拒绝；原stage15 inbound closure仍随后完整执行。未知/丢失授权定位记录已在stage3遮蔽，完整记录损坏在连续性/完整性检查停止，不能仅因记录丢失生成一个新计划。

同一planning CAS比较原ledger unseen/family/current authority及绑定记录和完整依赖，保存原request、PreparedActionBinding完整内容、pins、scope、实际D3计划与reservations。已有planned恢复只使用这份保存内容，不重跑漂移后的Query或重新选择targets。已有saved decision的stage5重放不检查旧preview/preparation TTL，但仍先当前原授权和scope；返回原receipt/error bytes。撤权不写永久rejection；重获权仅恢复原decision。

prepare不会领取或烧毁最终content IDs；fresh内容保持D3 symbolic subjects，到原stage12临时candidate及原planning reservation才按已有D3规则产生真实映射。原D7 post-query的proposed evaluation在同一已绑定symbolic candidate namespace进行；fresh equality/NodeRef输入只能由Core typed替代环境提供，不把symbolicToken当公开NodeRef。stage14以最终candidate map重新验证同一语义与完整结果，不能把用伪UUID运行的一次样本当证明。

## 21B.3. D6承接与寿命

D6 request继续通过原planToken唯一选择PreparedIntent；该PreparedIntent完整嵌入PreparedActionBinding及对应D6 request，不增加D3模式或第二commit入口。planToken与bindingToken是不同tag，两者一次绑定且不可互换。相同planToken永不指向另一个action、post-query、输入或targets。D6原步骤6/7逐项CAS和保存这份完整D7证据，D6错误/disposition顺序保持。

未被任何decision引用的准备记录在有限TTL后不再允许新提交，可以释放大pins；保留最小token/tag/audience/scope及expired标志至受管token失效保留期，以区分已知本人过期与未知。任何planned或terminal saved decision引用后的最小授权映射和完整record/pins按该原ledger保存与恢复规则保留；不受preview TTL清理。ledger恢复所需pin不能删；preview丢失仅preview不可用，不改commit事实。若连续性不可证明，保持原planned或遮蔽交付，不重新prepare代替。

显式重新prepare产生新tokens与新OperationId；d3_operation已给OperationId时不改其值，但不会在同一ID下修改现存不可变记录。未提交的多个独立records不占原ledger key；最终只有原ledger CAS winner能形成decision。没有“准备成功就保证最终提交”的预留授权。调用者改动返回request后，它成为另一canonical输入，原证据不再匹配，且不能自动使用旧preview确认新请求。

lost receipt只能重发原request；相同saved decision在当前资格通过后返回原bytes，不再次执行Query/字段修改或增加commitSequence。未知是否提交时不能以新OperationId重做。完整post Query（包括sort/take、scalar、QueryRef、所有正负依赖）在准备与原planning/commit证据中都固定；显示200条preview不减小条件范围。

## 21B.4. 版本及历史

本联合版本的新准备记录一律为PreparedActionBinding/2，新增constructionInput并使用最小定位映射/2；非模板明确null/空refs，不自动从作者字符串推断模板。已有/1 records及其planned/saved decisions继续其原decoder、原最小映射和byte-equal恢复/重放，不填字段、不换token或重新解释历史授权。新模板准备只使用/2。切换后尚未形成decision的旧/1准备不能建立新author effects：原授权门后，D3 unseen在stage14按operation_precondition_failed、D6在原步骤6按semantic_rejected记录准备失效，用户须明确重新prepare。已planned/saved在原ledger门先恢复/重放，不被此unseen版本检查回溯拒绝。D3 wire11、preparationBinding对象形状及D6提交wire均不变：新不可变bindingToken选择/2记录并进入原fingerprint。无法加载对应版本或完整pins时按原availability/完整性门停止，不能按空construction处理。


D3 wire10与wire9已保存decision仅以各自原decoder、原fingerprint、原门和原byte receipts重放/恢复；禁止新wire10/9 decision。不能把旧request改成11、补token或按新语义重新解释保存的旧结果。新wire11的request/receipt/error及Result/9需完整新conformance，历史v10测试不证明这些新增合同。D6 Policy/2及新metadata资格只作用实际采纳新profile的请求/准备，不悄改原D3 create/fork已明确授权的历史receipt重放范围。

## 21Q.1. 唯一typed引用遍历

按完整D7 schema解码SavedQueryDefinition、ViewSpec、DynamicBlock后，以静态schema路径遍历：DefinitionAddress.owner及其已识别D3 Locator；TypedLiteral.type指明的node/resource/annotation_ref值（递归Optional/object/list/union）；CollectionCreationPolicy.parent；保存View/Query调用地址；Query Selector literals、QueryRef arguments、parameter defaults、固定View Domain TypedLiteral及DynamicBlock literal/context bindings。普通CEL expression字符串、labels、任意普通text和lexical this bindings不在遍历内；即使其字符串含UUID也逐字保持。不能递归扫描未知JSON并猜ref。

每个typed Ref都用原D3完整decoder，且按原slot/owner/locality门判断。来自identity map内的Ref必须完整改为对应fresh Ref；同Workspace map外的完整Ref按显式preserve策略保持；跨Workspace map外Ref不能继续作为当前Workspace typed Ref，整操作拒绝cross_workspace_identity_preservation，不静默删默认值、替换none或把它改成text。source map遗漏不靠owner/path猜。Resource/Annotation映射包含其完整owner；new owner不相容拒绝，不能只换leaf UUID。this.node仍lexical，在目标Node实际执行时解析，不在copy中固化旧NodeRef。

TypedLiteral中的普通object仅由其TypeSpec声明的Ref成员受此规则；一个像Locator的普通object不会仅凭成员名被升级为可解析Locator。D7真实DefinitionAddress.at.locator才接受下段Locator重发。D4 authored provenance的复制继续由D4/D3原typed carrier合同处理；不能由D7的普通object去补签其ActionEvidence或重新解释历史原值。

## 21Q.2. 源、结果与显式slot清单

D3 wire11的plan在原九数组之外增加第十数组`definitionTransfers`，完整元素exact：`{kind:"d7_definition_transfer",sourceContainer,inputOccurrence,resultContainer,resultOccurrence,payloadFormat,slots}`。

sourceContainer为existing_entity_subject(NodeRef)或artifact_part_subject；inputOccurrence为非负Counter，指完整已绑定D2 source/artifact part中按source order的SavedQueryViewDefinition ordinal；resultContainer为existing/mapped/new Node subject；resultOccurrence为结果完整D2源同类ordinal。payloadFormat只能weftext.saved-query/weftext.view/weftext.dynamic-block；未知format不能成为本adapter输入。一般copy/fork不得借用ordinary导入转换旧QuerySpec：其源须是当前完整已验证payload。

slots按唯一typed-path comparator排序且路径唯一：逐segment比较，member text的tag rank=0并按原UTF8 bytes比较，Counter index的tag rank=1并按D3Integer数值比较；共同prefix后短路径在前。禁止用整条path的D3-CJ/3序列化字节排序；索引2必须先于10，元素exact `{path,before,after}`。path是`member name text|Counter index`数组，仅在完整schema遍历找到的typed Ref/DefinitionAddress Locator根合法，禁止祖先/子孙重叠；before为原D3 Ref或原完整DefinitionAddress Locator（原decoder确认variant）；after为`{kind:"map",subject:PayloadSubjectKey}`、`{kind:"preserve"}`或`{kind:"reissue_locator",ownerSubject:PayloadSubjectKey,targetOccurrence:Counter}`。map只能mapped/new同kind subject；preserve必须满足上段同Workspace外部保留；reissue_locator只接原DefinitionAddress DocumentElementLocator，并绑定其当前精确目标SavedQueryViewDefinition occurrence，owner映射机械确定。全payload全部typed Ref/Locator必须恰有一项，不能只列需要改变的引用。locator root已包含owner，禁止另列其owner嵌套路径。anchor地址的owner是普通Ref slot，anchor name原样保留并在目标验证唯一解析。

definitionTransfers按`subjectCanonicalKey(resultContainer),resultOccurrence`排序，result pair唯一，source pair可在显式独立copy映射中多次出现但仍逐一绑定各result。其全部成员进入原D3 canonical request/fingerprint。完整source payload不是slots声称的对象：Core从原preimage/source_artifact payloadBinding对应的完整source pin按inputOccurrence独立恢复，再由D7 schema重新枚举并与slots精确比对；result同样由完整结果独立恢复，缺/多slot、before不等、wrong type、owner互换均stage14 identity_map_incomplete。无Ref的合法saved payload也须有一条slots=[] transfer，证明已读其完整format而非当opaque跳过。

### 新作者定义与转移范围

definitionTransfers只描述已有payload的copy/fork/identity-bearing import；新增Node里完全新写的SavedQueryDefinition/View/DynamicBlock不是“从不存在的源转移”。此类新作者payload若只含当前已存在的合法Ref/DefinitionAddress可留B，但仍在stage14完整D7解码、读权限、所有引用/Locator及全D2/D4门验证；它不能宣称获某个旧定义的transfer效果。D9 ordinary import必须先产出当前完整D7 wrapper，不能由本合同隐式猜旧format。

新作者payload直接互引本次尚未分配的fresh Node不是本版输入形状：TypedLiteral与DefinitionAddress没有symbolic public variant，整请求拒绝unsupported authored payload，映原stage14 identity_map_incomplete；不能伪造UUID。copy/fork中已有定义相互引用则由下面Q+已有source mapping完整支持，不能以这项新写入限制拒绝它们。新建多个定义互引的作者工具可在D9提出另一显式版本，但本版不会拆成两个提交冒称原子。

## 21Q.3. Result/9的Q segment

Result/9完整继承Result/8的B/M/N/E/S/C分区、长度、显式slot、C carrier与原decoder限制，新增唯一Q tag；magic改为ASCII `D3-Symbolic-Result/9`+NUL，后续header及partition算法不变。Q只允许exact_source_document的D2 SavedQueryViewDefinition **payload span**，不含外层attributes/opener/closer或anchor；不能用于Resource、Annotation、Field carrier、普通literal/body。与B/M/N/E/S/C spans不重叠。

Q body是D3-CJ/3 exact `{transfer:<完整definitionTransfers元素>,inputPayload:<原inert UTF8 payload文本>}`；start/end绑定结果base中的完整payload span，bodyLength按实际Q body字节计。inputPayload必须与sourceContainer/inputOccurrence的真实原bytes相等，且完整解码format等于payloadFormat。Q与definitionTransfers一一对应；copy/fork/identity-bearing import中的每个已识别保存payload都必须Q承载，不能留在B规避转移；payloadBindings.result按完整Result/9 bytes绑定。语义结果为下面唯一materialize函数输出的UTF8 JSON，替换该payload span；外层D2 raw、普通CEL/text值逐字保持，JSON格式化可变仅限已明确列出的inert payload整体，preview须显示这一格式化变化。

算法materialize(inputPayload, slots, original cut, original D3 candidate map)：先完整解码和枚举，按map/preserve机械代换typed Refs；对reissue_locator使用同一原D3完整source与目标occurrence映射；以D3-CJ/3 canonical JSON写回这个portable payload（UTF8原text值及换行字符保留，JSON源码空白在该已选择payload内规范化）。完整最终Query/View/DynamicBlock解码、Workspace/ref/Locator、参数型、D2全source/载体及D4验证全部通过才可接受；不能把JSON合法当Query合法。

## 21Q.4. revision与位置闭环

内部Ref映射先由原D3 stage12的ephemeral candidate map固定。fresh源revision=1，existing改变源revision=原值+1；原D3/D6为每个拟议源固定其将要使用的revision token，token由受管revision身份生成，不取尚未确定的payload内容散列。原token域/真实性仍由D3/D6，不允许D7调用方指定。只有完整原planning CAS及最终author commit才发布这些tokens，不提前授予Locator能力。

reissue_locator目标必须是原已验证的SavedQueryViewDefinition occurrence，在本次完整变换中有唯一对应的结果occurrence；deleted/ambiguous/非当前preimage Locator拒绝，不最近似定位。D7只重发指向SavedQueryViewDefinition的DocumentElementLocator。D2规定保存块的opener与closing delimiter独占physical line；Q按D3-CJ/3把完整JSON payload写为一行，string内部换行均escaped。因此坐标数字的宽度不会改变保存块边界的line/column，采用确定的两遍物化：

1. 固定完整Ref/revision token及其它原D3/C非D7-position变换。Q中的待重发位置先用内部零坐标模板，整体canonical JSON仍一行；拼成完整source，用D2 parser取得每个映射后的SavedQueryViewDefinition的完整真实SourceSpan。内部模板不得交付或当合法Locator解析。
2. 将这些真实span同时代入相应Locator，再次序列化Q并拼成完整source。重新用D2 parser读取每个目标element span，要求与第一遍逐字相等；随后每个最终Locator用原D3 decoder/resolver证明same owner/revision/elementKind/exact target occurrence。任何不等、目标消失/多义或预算溢出整操作拒绝，不迭代猜测或交付中间值。

该算法的适用边界是完整saved-definition element位置：不是payload某byte或任意正文range。普通preserve的外部Locator逐字保持并验证原当前指向，不更新latest。D3/C处理的Annotation或D4 provenance坐标继续各自原接受器；它们须绑定同一最终source，不能分两次提交各自正确，也不能修改Q canonical JSON一行的前提。若一个同plan变换不能证明与这个两遍边界兼容，整计划在stage14失败；没有自由脚本参与重排源。

## 21Q.5. 完整效果与未知payload

原D3 receipt及D6效果同decision附带D7自己的`D7DefinitionTransferEffects/1`，exact `{kind:"d7_definition_transfer_effects",version:1,operationId,transfers}`。transfers逐项exact `{sourceContainer,inputOccurrence,resultOwner:NodeRef,resultOccurrence,beforePayload:text,afterPayload:text,slots}`；slots逐项`{path,before,after}`，after是实际完整Ref或重发Locator，不再是symbolic subject。数组顺序与request definitionTransfers一致；每项由实际完整before/after源独立恢复并逐项验证，不能仅复述request。该扩展不进入D3 authored node_link/citation slot union，不造D3 referenceLifecycleTransitions或持久definition身份。

copy/fork/import的包含Node若有未知或不可解码D7 payload，原raw可被普通D2只读保留，但**typed transfer整体拒绝**，不能假称已保留可执行引用含义；full fork不省略该Node或删block。用户可以在另一个明确author编辑中修复/改为普通inert literal后重新提交新operation，这里不自动做该转换。已识别payload没有Ref时slots=[]仍通过完整schema验证并保留text值；普通body/source/literal中的同UUID字符串完全不参与。旧result handles、ActionEvidence、preview/effects tokens本来不是合法portable schema成员，遇到它们拒绝unknown member而非搬到新Workspace。

### 21.1 字节grammar与验证顺序补齐

Result/9的Q使用共同segment前缀Q<start>:<end>:<bodyLength>:及后续恰bodyLength bytes；无E的额外exactLength尾部。Q body必须先strict decode再D3-CJ/3重编码byte-equal，start/end是所绑定base完整D2保存payload span。definitionTransfers的source preimage/source_artifact binding必须完整；result base由B原bytes及non-B原模板共同恢复，Q.inputPayload等于该源真实payload，且end-start等于base payload span实际长度。slot mismatch、extra transfer、非保存块Q或typed转移payload藏B均stage14 identity_map_incomplete。Q不进入ReferenceResultSlotKey或原referenceLifecycleTransitions，不能冒充node_link。

stage14先全部原D3/D2/D4完整物化与coverage，再检查带token的D7 bound pre/post condition；前者identity_map_incomplete优先，后者operation_precondition_failed随后。stage15原引用/结构闭包仍完整执行。stage3失权不读内容，stage4/P/TL availability先行，不能因新条件改变早门先后。saved重放不受旧准备TTL，所需最小授权记录与完整record随原decision耐久保存。
