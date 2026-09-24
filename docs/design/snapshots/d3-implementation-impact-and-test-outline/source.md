---
_weftext:
  id: "04f16e0f-a8ca-4d4a-8d75-007e46d44975"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。


# D3 实现影响与测试轮廓 — revision05


日期：2026-08-31。

权威输入：

- [D3 身份、引用、所有权和生命周期决议](../d3-identity-references-ownership-and-lifecycle/source.md)
- [D3 Terminology and Naming Lexicon](../d3-terminology-and-naming-lexicon/source.md)
- [D1 产品表面与能力边界](../d1-product-surface-and-capability-boundary/source.md)
- [D2 文档内容与领域对象](../d2-document-content-and-domain-objects/source.md)

历史v9冻结证据原范围（其中版本/激活措辞只描述历史，当前wire11不沿用此历史门）：同代generation binding：final D2 SHA-256 `873D2487AC27C7C579D64C14CAB106215FBD462286416860225F2F404BC85137`、D2 wire v2/outer grammar（含可独立传输Annotation outer `wireVersion:2`）、D3 wireVersion 9与39-concept Lexicon。matching corpus/goldens必须在激活前同代通过；当前Impact不单独激活。

## 1. 实施边界

D3 只固定对象如何被识别、被谁拥有、引用怎样封闭解析、生命周期状态怎样观察，以及各操作何时保留或产生 fresh identity。它不选择 D4 的类型/schema/关系，不选择 D5 Record domain，不冻结 D6 的物理存储、事务、权限或同步机制，不冻结 D7 payload，不设计 D8 编辑器，不规定 D9 worker/模板，不规定 D10 Agent 自动化。

实现必须继续保留D2 v2：Node是content graph中唯一Document-bearing内容实体；每个Node恰一exact-source Document；Task是ordinary Node+exact `tasks/task` Facet；Template是Core meta-kind；AttributeCarrierBlock/LexicalAttributeEntry与其他Document occurrences没有独立durable identity；Resource/Annotation是owner-local非节点对象；Saved Query/View只是Document occurrence；NodeCollectionResult无durable identity。Record/RecordCollection仍由D5从零决定。

## 2. Core 最小身份层

未来 R0 实现至少需要以下封闭能力；具体语言类型名可以不同，但必须一对一映射冻结 Lexicon：

1. `WorkspaceRef`、`NodeRef`：权威身份值，不由path、title、display label、source span或外部UID推导。
2. `ResourceRef`、`AnnotationRef`：包含owner `NodeRef`与owner-local object id的封闭值；裸local id不可跨owner解析。
3. `EntityRef = NodeRef | ResourceRef | AnnotationRef`：只有真实接受该union的接口才可使用；不得出现wire `kind:"entity"`。
4. `Locator`及其封闭成员：只定位D2 v2允许的Document element/range/resource region；AttributeCarrierBlock、LexicalAttributeEntry、field occurrence及其range没有locator kind，locator失效不制造新的authoritative identity。
5. `OriginBinding(ForeignIdentityKey, authoritative ref)`与Provenance：证明来源关联，不证明foreign identity等于Weftext identity；provider token/etag/cursor属于下游控制面。
6. 生命周期观察：`live | trashed | tombstoned | not_found | not_visible | unprovable`及冻结的解析结果/错误族；没有权限时不得把不可见错误降格成不存在。

## 3. 操作影响图

| 用户意图 | 身份效果 | 所有权/位置 | 必须失败关闭的情况 |
| --- | --- | --- | --- |
| create | 产生 fresh identity | 在目标owner/parent一次建立 | owner、parent或precondition不可证明 |
| promote | 只接受resolved managed Document occurrence；产生fresh NodeRef，ForeignIdentityKey absent，`activeBindingWrites=0`，无OriginBinding/foreign provenance；checklist同时声明exact `tasks/task`且无mirror | 原occurrence按原子plan改写；carrier/entry不是identity或ref slot | foreign key present、locator unresolved、binding state非not_applicable或任何binding write |
| initial_import | ForeignIdentityKey present；`never_bound`才fresh Node + same-decision sole OriginBinding；`active_live`只做deterministic re-import/upsert且不fresh | 按frozen classifier/binding-state matrix，不迁移foreign identity | key缺失、retired/active_non_live/conflict或required binding缺失 |
| adopt | ForeignIdentityKey present；`never_bound|retired`可fresh Node + same-decision sole OriginBinding并保留foreign provenance | 按frozen classifier/binding-state matrix，不把foreign key变成NodeRef | key/binding缺失、active_live/non_live/conflict或authority不明 |
| managed_copy | 只接受managed live Node source；产生fresh NodeRef，`activeBindingWrites=0`且无OriginBinding | copy lineage不是foreign provenance | source non_live/unresolvable或任何binding write |
| workspace fork/continue | 按决议的closed mode保留或重建workspace/node identity | 不能由显示名或path暗示continuity | mode未知、基线/后继不可证明 |
| move/reorder/rename | 同Workspace内保持NodeRef | 改parent/ordinal/label/path，不改identity | cycle、orphan、重复/缺口ordinal或完整性异常 |
| trash | identity仍可由授权观察为trashed | 不进行隐式复制或解绑 | live/Trash ordinal损坏、引用后置条件不闭合 |
| restore | 恢复同一identity | 必须显式得到合法parent/ordinal | parent不存在、cycle/orphan、ordinal损坏 |
| purge | 终止普通可解析性并保留冻结tombstone语义 | owner-local children按闭合规则处理 | retention/gate不满足或对象状态不可证明 |
| export/re-enter | 外部载体不得携带可冒充本Workspace identity的权威身份 | re-entry按copy/import/adopt规则 | 同UID跨source自动合并、path/title dedup |

D2 v2 exact-source payload binding覆盖完整control/carrier/body/trivia bytes与SHA；copy/fork/import/export随同一payload proof，只有真实typed reference slots进入rewrite evidence，carrier/entry ranges不进入identityMap或symbolic results。

所有普通写操作必须先完成身份、owner、lifecycle与完整性preflight，再产生allocation或mutation。duplicate/gapped live或Trash ordinal在authority/integrity gate即返回`workspace_integrity_conflict + preflight_rejection`：零target-ledger content read/write、零allocation、零placement/lifecycle mutation、无receipt。repair-channel diagnostic不得作为普通commit poststate。

## 4. 引用解析接口

引用解析按封闭顺序实现：

1. 解码并验证ref kind、workspace scope、owner-local组成和canonical encoding；
2. 验证caller声明的expected concept/type，拒绝known-wrong-union；
3. 检查authoritative object及owner/lifecycle observation；
4. 区分resolved、trashed、tombstoned、not_found、not_visible、unprovable及stale/invalid locator；
5. 任何未知kind、缺字段、额外字段、scope错配、owner错配、revision证明不足或权限不可证明均fail closed。

普通API不得把path、display label、filename、source span、ICS UID或provider locator升级为Ref；也不得把无权限的`not_visible`投影为`not_found`。

## 5. ICS / RFC 5545 压力案例门禁

- `UID`、`RECURRENCE-ID`、`SEQUENCE`和时间戳是source-scoped foreign identity/version/provenance材料，永不替代`NodeRef`。
- recurring series即使materialize，也至多一个series Node；derived occurrence默认可重建且无Node identity；series-owned override默认是下游structured value/Record候选。
- foreign derived occurrence只有显式adopt且确需独立正文、Resource、Annotation、relation/reference、权限或生命周期时，才可按binding-state matrix创建fresh NodeRef与same-decision sole OriginBinding并保留Provenance；promote专用于managed Document occurrence，绝不产生foreign binding/provenance。
- 同源重导入用SourceBinding+ForeignIdentityKey确定upsert/conflict；title/path改变不重复创建；跨source相同UID不自动合并。
- subscribe/sync、initial_import/adopt、managed_copy与managed promote必须保持content authority、source binding、provenance和Weftext identity分域；四个materialization intents不得互相猜测。
- VTODO/VJOURNAL/VFREEBUSY/VTIMEZONE不自动取得Event Node identity；D4/D9只可在preview后按冻结D2/D3边界映射。VTODO→Task只有声明`initial_import|adopt`并通过ForeignIdentityKey/binding-state matrix时才可fresh ordinary NodeRef + exact `tasks/task` Facet + same-decision sole OriginBinding；不能保留UID为identity或创建mirror，且不得称为promote。

## 6. 下游门禁

- D4：任何property/schema/relation endpoint必须声明接受的冻结ref/locator domain；不得给Document occurrence、heading、citation、AttributeCarrierBlock、LexicalAttributeEntry、field occurrence或derived occurrence补durable identity/locator/Annotation target。未来durable field target必须正式reopen。
- D5：可以定义独立Record domain，也可以拒绝；若定义，必须有与NodeRef分域的owner/identity/persistence，不得冒充Node/Document或第二正文权威。D3没有预设RecordRef tuple。
- D6：选择物理布局、事务、权限、同步和retention机制时，必须实现D3可观察状态、preflight原子性、not-visible遮蔽及tombstone语义；不得改变身份合同。
- D7：Query/View/Action payload只能消费冻结ref/locator；ResultHandle/row handle/cursor不是D3 durable content identity，具体协议仍待D6/D7。
- D8：UI rename/reorder/trash/restore必须显示identity-preserving或fresh-identity effect；不得用路径或标题匹配代替ref。
- D9：import/export必须给出preview、authority mode、source binding与fresh identity effect；不得按展开occurrence无界建Node。Template是Core meta-kind；Task Template target plan产生fresh ordinary Node+`tasks/task`及完整identity/ref rewrite，Template自身不成为Task。
- D10：Agent/automation必须通过同一Core action/preflight/receipt边界；不得持有第二identity或commit authority。

## 7. 术语与代码命名约束

所有公共schema/wire/API/code symbol/CLI/UI/locale key必须从当前完整Lexicon的42个concept entries投影。`Node`、`Document`、`occurrence`、`Resource`、`Annotation`、`Reference`、`Relation`、`Citation`、`Authority`、`SourceBinding`、`Provenance`、`OriginBinding`以及copy/fork/continue/move/import/adopt/promote不得裸词多义。

负向gate只检查受控identifier/symbol/locale surface：先exact owned name，再Lexicon §6 exact global-retired table，再按expected concept/type检查concept-relative semantic nonalias，最后unknown/error。自然语言、用户内容、第三方format、历史evidence、migration note和closed counterexample不在粗暴禁词面。

## 8. 测试轮廓

### 8.1 构造与作用域

- Workspace/Node/owner-local Resource/Annotation的canonical round trip与malformed/unknown/extra-field拒绝。
- 两个owner拥有相同local id时Ref不相等；裸local id解析失败。
- path/title/display label/source span变化不改变authoritative identity；删除后复用path/title不复活旧identity。

### 8.2 生命周期与操作

- create/copy/fork/continue的fresh/preserve矩阵逐格测试；materialization另对promote/initial_import/adopt/managed_copy按shape、binding state、allocation、activeBindingWrites与provenance逐格测试。
- move/reorder/rename保持NodeRef；跨Workspace不能保持NodeRef。
- trash→restore保持同一identity；purge/tombstone的普通解析与观察结果闭合。
- duplicate/gapped live/Trash ordinal对trash/restore/purge在preflight阶段失败且零读写/分配/突变/receipt。
- cycle、orphan、owner mismatch、stale locator、unknown revision与not-visible fail closed。

### 8.3 引用与错误

- NodeRef/ResourceRef/AnnotationRef/EntityRef各正例和known-wrong-union反例。
- lifecycle × visibility × revision × owner scope矩阵；客户端不得从错误差异推断隐藏对象。
- locator失效不会创建新identity或自动retarget；悬空引用保持可观察但不隐式删除source occurrence。

### 8.4 ICS 与重导入

- 同source相同UID+RECURRENCE-ID确定upsert；不同source相同UID不合并。
- recurring master不展开无限Nodes；foreign derived occurrence无NodeRef，只有显式adopt可按binding-state matrix产生fresh NodeRef与sole OriginBinding；promote只处理managed Document occurrence。
- SEQUENCE/etag冲突只影响version/conflict，不改变NodeRef。
- cancel/delete/unbind/offline conflict分别保持authority/provenance/identity边界。

### 8.5 术语

- 当前42个concept IDs（39个继承及3个新增）、owned-name exact-set、surface一对一映射、全局retired与owned disjoint；原五个wire names精确归属Facet/FacetId/carrier/entry，本次Preparation Binding、Definition Transfer、Definition Result Segment的名称按当前Lexicon归属；`NodeSpecialization::Task`只命中global retired rule。
- `NodeRef→Node`；`NoteRef→global-retired/NodeRef`；ForeignIdentityKey-as-NodeRef→`expected-concept-mismatch`；`CloneWorkspace|master`不因裸token进入global-retired。
- 每条global-retired identifier唯一replacement/deletion target；受控surface中的退役词、未知词和同形合法owned词不会互相误判。

### 8.6 D2 v2 七切片同代验收

历史v9冻结证据原范围（其中版本/激活措辞只描述历史，当前wire11不沿用此历史门）：- final D2 SHA、D3 wireVersion 9、corpus/goldens/negative matrix exact绑定；0..8与10拒绝、9接受。
- AttributeCarrierBlock/LexicalAttributeEntry只有owner Document/current revision range；EntityRef/lifecycle/cross-revision/locator/AnnotationTarget全否。
- exact source、control/carrier spans与copy/fork/import/export artifact hash闭合；无第二payload authority或symbolic ref slot。
- existing ordinary Node assign/remove exact `tasks/task`保持NodeRef；checklist promote fresh NodeRef/零binding/no mirror；VTODO initial_import/adopt fresh NodeRef + required sole binding/no mirror；Template target plan fresh rewrite。
- 当前42-concept terminology registry/negative Gate完整覆盖39个继承及3个新增entry；template + non-task Facet接受、template + `tasks/task`拒绝；field occurrence identity/target保持禁止。

## 9. 实施顺序与完成条件

当前只完成final-capable设计staging，没有改动代码或active authority；corpus/validators与外部Gate必须在激活前同代闭合。未来R0按“冻结term/ref value → resolver/state machine → operation preflight → persistence/transaction realization →各surface adapter”顺序切换；D4–D10各自在自己的冻结任务中补充上层合同。

一个实现切片只有在Core、所有受影响调用方、fixtures、normal/optimized validation、公开中英文文档和负向退役词gate原子一致时才算完成。任何实现细节若要求改变D3 identity/lifecycle合同，必须显式reopen D3，而不是在D4–D10或代码中静默覆盖。原D3阶段的“D4 paused/not started”只属于历史推进状态；本次是D7及D3/D4/D6联合架构候选，不启动D8或产品实施。

## D6所需完整版本演进实施义务

当前目标为同包D3 v11/Result9和D4§15源物化桥。上列既有机制与场景继续非回退；其中旧wire/corpus计数仅说明历史覆盖，不能作为当前版本验收结果。实施必须统一十数组per-mode decoder、现有源revision/potentialChanges授权、artifact fresh/update/companion分区、candidate map与planning CAS、C carrier完整源物化、S fresh reply和回执/效果双向验证。原v9只保存决议重放，unseen零写拒绝；新版本不向旧请求补默认数组或悄悄转换已存decision。

逐实际输入验证row/checklist promotion、Resource创建+existing occurrence、existing Annotation reply→fresh同owner、pure existing upsert归D6、mixed D3批次、fresh Task SCC、完整Template、copy内部D4 typed refs、symmetric迁移/空carrier/禁写既有端、provenance/locator所有typed根、权限不足/撤权/ABA/collision/crash/replay/watermark。全量D2/D4语义和真实源解析必须使用独立真实输入，不能从待验effects反推期望值；本次有界模型不等于上述完整产品验收。本文件不授权产品实现或公开规范修改。


D6 revision03接入门：逐格验证mode×primary/member×existing/new/mapped owner，ordinary import new-only owner；creation多owner member图与standalone copy旧compound均须完整payload/placement/allocation/receipt。Annotation target@0和S reply@1共用最终revision，null/concrete→fresh/null、target同时保持/改写、S-origin lifecycle transitions及reply双记/target遗漏负例完整覆盖。

联合恢复验收补充：trashed Annotation restore加target reference/S reply改写时，存在的typed preimage slot取non_live_source；null旧reply单独取absent；target与reply共用一次最终revision。ByteHandle当前授权缓存因generation变化失效，但仍获权时旧版本pin可继续，lifecycle/continuity reset与auth重验严格分开。

当前实现入口按D3 wire11/Result9/十数组及D4 RelationReadContext2/Binding2构建新conformance。旧v9/v10仅按各自原decoder重放或恢复保存决议，不建立新旧版decision。旧corpus版本/数目/哈希不改写为新执行事实。

受信D3 fresh Create conformance须验证结果revision=1、expectedOwnerRevision=1、request声明/initialEntries与完整暂存结果精确一致，成功不二次append/increment；revision0假前像、结果2、缺失/重复initial Entry、现存owner冒用fresh origin都拒绝。普通existing实际修改仍加一次。

## 当前观察与初始化实施义务

接入D6接口§13固定潜在观察scope、D3 stage3与D6最小授权定位、空范围前置权限、当前policy/Registry CAS、replay/recovery/delivery；成对隐藏状态在无权时必须同一not_visible且零业务读取/decision，有权时仍完整检查真实约束。纯policy管理保留control_only恢复路径。实现§14 issuer首次信任与可撤销grant、family固定profile、独立target principal、完整Registry seed/source重绑、初始policy/Calendar配置和scope绑定一次activation；完整fork保留multiplicity且不复制ACL。实现§8.1 scope选择与迁移、空配置删除、控制inbound和ABA版本。模型不是完整decoder/host验收，需在对应实施入口冻结并实际验证所有运输、认证与故障前提。

## D7联合revision03消费补充（未接受）

当前协议基线为D3 wire11/Result9；D4 C及全部领域类型语义不变。新增受控名称和exact位置：PreparationBinding=D3.identity_operation_request.preparationBinding；DefinitionTransfer=D3.intent.plan.definitionTransfers[]；D3-Symbolic-Result/9.Q（定义结果分段）=D3-Symbolic-Result/9的Q；PreparedActionBinding=D7受管不可变准备输入；SourceEnvelopeStateCapability/CommitSequenceStateCapability=D6 Policy/2.capabilities；EffectManifest/EffectBytes=D7只读效果运输。它们均非内容实体或第二ledger。完整定义以同包D3主稿§21、D6 Control Interfaces §15–16及D7专项正文为准。

必须新增实际版本解码/legacy replay、fingerprint差异、planned恢复、保存定义typed引用转移/Locator、完整效果及窄Field正向outcome测试；历史wire10与Result8测试保留历史归属，不以数字替换声称新版本通过。其它原实现影响和必需检查不因本补充被删除。
