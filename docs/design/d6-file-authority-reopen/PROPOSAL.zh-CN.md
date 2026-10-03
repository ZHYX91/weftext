---
source_language: zh-CN
translation_status: source
---

[English](PROPOSAL.md)

# D6-FA-r01 文件权威重开联合候选说明

状态：**候选；P1 作者配套已补齐；整体联合候选仍未完成；未接受、未激活、未实现、未合并**。

固定上游仍为 S=7e18168dad3e6d120fce0dd607dc10fa7894e252；旧 D10 作者候选基线仍为 C8=d99f053b9386c9c9e1664251fdec9f00e33fac2c。本目录只保存 D6-FA-r01 候选后像，不修改 snapshots、inputs 或原 D10 十八份。只有全部真实 replacement owner、D10 消费修订、fresh 独立联合审查和协调接受完成后，才能讨论激活。

本文件只做路由、版本与差异说明；规范操作语义唯一归 owners/ 下对应 owner 后像。

## 1. 已确定产品取舍

1. Document 当前 exact source 由普通 `.adoc` 文件唯一承载；Resource 当前 bytes 由普通资源文件唯一承载。
2. 可删 SQLite I 与耐久 SQLite P 均在同步目录外；I 可重建，P 保存不可从文件重建的 decision/recovery/unknown/approval/claim/Money/pins；两者都不是正文 authority。
3. 可移植 identity、parent/order、lifecycle、shared configuration/ACL/trust 由 portable metadata物理承载，逻辑 owner仍归 D3/D4等真实领域 owner。
4. 多台登记 replica 可离线普通 edit/create/move/reorder/Trash，随后显式处理 source/placement/lifecycle/policy conflicts。
5. portable file复制不复制 global execution responsibility。
6. Server 保留多人 Draft/read/prepare/edit；一个托管后端的持久 commit holder唯一不等于单用户。
7. 实时协作仍 post-G2；未冻结 OT/CRDT，不采用每键 author commit。
8. 大库 ordinary file save 不等待无关 full index/OCR；complete Query/Action仍必须完整证明。
9. 用户已批准：受信人工单一既有 live Document 的 ordinary save 可在完整授权、局部门、耐久输入/已读取前像和唯一P决议链保持时使用 WriteProtection=observed_only；它只放宽未观察外部竞争排除，不放宽已观察冲突、失权、幂等、耐久或强Action资格。

## 2. 当前实际 replacements：16

`replacements.json` 只登记实际存在的 16 份 fixed-S owner replacement：

- D6 五份：Storage、Control、Terminology Lexicon、Terminology Registry、Implementation Impact/Test。
- D1 两份：Product Surface/Capability Boundary、Implementation Impact/Test；G0-B 已生成对应 owner 后像。
- D3 三份：Identity/References/Ownership/Lifecycle、Terminology Lexicon、Implementation Impact/Test；G0-B 已生成对应 owner 后像。
- D4 三份：Attribute Types/Schema/Relations、Terminology Lexicon、Implementation Impact/Test。
- D5 三份：Tables/Node Collections、Terminology Lexicon、Implementation Impact/Test。

D4 Reference Catalog 与 mandatory scenarios 只是固定只读输入，不登记 replacement；本批不改 catalog typed definitions 或 limits。

## 3. 当前 owner 路由

| fact / capability | logical owner | D6-FA-r01 physical/control |
|---|---|---|
| Document/Resource current bytes | D2/Resource domain | ordinary files |
| identity/parent/order/lifecycle | D3 | portable metadata + D3 wire12 |
| Field/Facet/关系/Calendar Registry 语义 | D4 | 可移植 Registry 元数据 + D6 生产 SourceVersion/2 与当前 SourceObservation/SourceVersionRef 外层资格 |
| native table structure / Node Collection semantics | D5 | exact source / D7 result + D6 当前 SourceObservation 外层资格 |
| durable decision/recovery | protocol owner + D6 | local P |
| search/parser/index candidates | source owners | rebuildable I |
| global execution responsibility | D6/D10 | continuous fenced control |

P/I不能成为第二份 Field、relation、collection、parent/order作者真相。

## 4. 三层资格、当前生产者与下游消费边界

- **普通副本内容（ordinary replica content）**：普通 source save 与 D3 create_node/move/reorder/Trash 只证明本次真实触及的来源、身份、结构、策略和安装范围。D5 原生表格 cell/row/column/reorder 可以依完整真实局部证据工作，不等待无关 I 或全 Workspace Query，但结构化编辑继续使用 strict 保护。缺少无关的强证明不能永久把合格 ordinary/local/offline 路径变成只读。
- **完整语义/动作证明（complete semantic/action proof）**：D4 relation、Facet 强修改、Calendar 唯一性，D5 collection membership、bulk、requireMembership、restore、purge、copy、fork、import，以及 D7 all_result、强 Action、Automation 等继续要求各自 owner 的完整正负范围、当前授权和完整 cut；semantic_pending、局部成功或 partial index 都不能替代。
- **全局执行责任（global execution responsibility）**：Automation、ApprovalUse、claim、Money、external unknown、stop 等继续要求独立、连续且可 fence 的责任链，普通 replica registration、ChangeId/Frontier 或文件复制都不会取得这些消费资格。

本 P1 的 Storage、Control、Terminology Lexicon、machine Registry 与 Implementation Impact/Test 五个 owner 已形成实际私人作者后像，但都仍是未独立接受、未激活、未实现的候选。固定 C 中 D1/D3 的 G0-B 后像和 D4/D5 的 A/B/C 候选也确实存在；“已经有候选”只表示相应旧边界有文本后像，不表示它们已经消费本 P1 后来补齐的全部生产规则。依赖这些新规则的 D3/D4 路径仍要 P2 实际补齐 native descriptor、companion、range key、版本和恢复消费者，D5 仍要 P3 实际补齐 locator/cut/保存保护消费者，然后再进入 fresh 联合接受。

来源版本与当前观察继续分层。SourceVersion/2 是生产历史：managed 分支记录生产 CommitDomain、生产 observationEpoch、revision、ChangeId；external 分支记录生产 CommitDomain、生产 observationEpoch、externalSequence，且没有 managed revision/ChangeId。生产域可以不同于当前 operation/observer 域。对生产域 D 与实体 E，H(D,E) 是该域连续 sealed managed 历史中的最大 revision；只有完整证明空历史才允许 H=0，真实 managed after 使用 checked H+1，MAX 不回绕，同一生产域跨生产 observationEpoch 不重置，跨域往返各自续接本域 H。equal-byte external admission 仍是 managed 接纳；source deletion 与 source-unchanged portable placement/lifecycle/structure effect 不推进 H，其中真实 portable effect 仍在 seal 获得本 decision 的 ChangeId。

SourceObservation/1 另以当前 operation CommitDomain 作为 observerDomain，绑定完整生产 SourceVersion/2、当前 observationEpoch、FileObjectBinding 和 evidencePins；SourceVersionRef/1.sourceToken 只选择这份完整当前 Observation。watcher gap、replacement 或不连续 rematerialization 会使旧 Observation、Ref 和受保护 revision token 的当前资格失效，即使 production SourceVersion、revision、hash 或最终文本相同。只有会产生 managed after 的原 plan 才在 P 中冻结内部 SourceRevisionPlan/1、SourceStamp/1 与 after pin；seal 前没有本 decision 的新 ChangeId。RevisionTokenBinding/2 与 d6_source_revision/2 绑定 observerDomain/current observationEpoch 与 managed SourceStamp 或完整 external SourceVersion；它不改 D3 Locator 的 opaque token 词法，也不替换 D4 的 sourceRevision、OccurrenceKey、Entry/Type/RelationReadContext/Binding/Recurrence 或 D5 revision-bound locator 等原内层 wire。

DependencyProof/2 使用十四类闭合 DependencyKey/2：source（来源）、lifecycle（生命周期）、placement_range（结构范围）、ref_inbound（入站引用）、relation_incidence（关系关联）、calendar_scope（日历范围）、registry（注册表）、temporal_rules（时间规则）、authorization（授权）、foreign_binding（外部绑定）、query_scan（查询扫描）、replica_registry（副本登记目录）、conflict_record（冲突记录目录）和 execution_resource（执行资源）。每个 key 使用 owner 定义的真实范围、受保护 evidence pins 与 `{epoch,revision}` 连续性 stamp；完整性来自当前授权下的一致 snapshot/range barrier，或无缺口连续变更链加最终复验。I 只缓存候选，删除/重建 I 不会凭空破坏仍保存在真实 P/M owner 中的完整连续性事实；真正丢失 proof、发生 gap、owner rule/decoder 改变或连续性无法证明时，旧 proof 才失效并建立新 epoch。partial/building index、index miss、placeholder、unknown、provider“已同步”、最终 hash 相同或 Frontier 向量数字增长都不能证明 empty 或完整。

Frontier/2 只表示每个 CommitDomain 已验证连续 sealed 的因果前缀，不证明 payload 已物化、placeholder 已下载、Registry/index 完整或 D7 全集 Query。frontierPolicy=exact 要求原完整 Frontier 相等，D3 managed_atomic 继续 exact。scope_dependencies 只接受从原 expectedFrontier 到当前 cut 的真实连续、已验证、非回退且能证明与原 source/control/authorization/全部正负 DependencyKey 无关的扩展；原 canonical request、expectedFrontier、DependencyProof.baseFrontier、targets、Query/selector、pins、proposed bytes、WriteProtection、owner input 和版本依据都不重签、不重采样。真实绑定依赖变化或未知 gap 不得洗成“无关”，但真正只依赖完整局部证据的 ordinary/local 操作也不被无关全 Workspace gate 误禁用。

ordinary 语义资格与 strict|observed_only 写入保护是两个独立轴，ordinary 也可以使用 strict。observed_only 只允许受信人工 interactive_source_save 在 planning 开始前显式选择并冻结：恰一个 existing live Document、ordinary+replica_local、完整 source read/replace、author source write set 为空或仅该 Document、无适用 body/Field/node-control deny、不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar scope 或其它 entity，并且 Draft Base 等于选定 current SourceObservation。noninteractive、managed_atomic、D3 结构/生命周期、D5 structured cell/row/column/reorder、bulk/collection、D7 strong Action、Automation、server checkpoint、Approval 与 Money 一律 strict。B 与 N 按原 plan 耐久保留；最后可信检查后仍未观察到的外部 C 可能被 N 覆盖且没有恢复副本，后来另一个 C 也可以替换 current file，但不能丢 B/N。已观察竞争、stale Base、watcher gap、失权或其它资格缺失都不受弱保护豁免；unknown install 保持 recovery_unknown，prepare/retained 不是 Saved。

新 portable 安装与发布按真实 owner 版本解释：InstallationNotice/2 在首个 portable-current install 前绑定 DecisionKey、原 baseFrontier、WriteProtection 和 component before/after；baseFrontier 可以包含旧的已 sealed ChangeId，但 notice 不含这个尚未 seal decision 的新 ChangeId。唯一 P seal 才分配该 portable decision 的 ChangeId，并只在真实 managed source change 时把 SourceRevisionPlan/1 的 SourceStamp 与 ChangeId 合成 managed SourceVersion/2。新路径随后发布 ContentCompletionProof/3，运输真实生产 SourceVersion before/after 与 actual frontierBefore/frontierAfter；接收端验证完整连续 sealed 链后建立自己的 SourceObservation，不能复制发送方 token。当前冲突记录使用 ConflictRecord/2+Frontier/2，ConflictKey/1、ConflictId 与 `D6-ConflictKey/1` hash domain 不变。

历史 ContentCompletionProof/1,/2、ConflictRecord/1、Frontier/1、InstallationNotice/1、旧 revision-token profile、Policy/1/2、D6 wire1、D3/D7/D8 既有 binding/receipt，以及真实存在的 saved/planned/unknown records 继续按原 bytes、decoder、pins、授权、期限与 continuity 履约，绝不因新版本静默重解。ContentCompletionProof/2 的 SourceVersionRef sourceChanges 不迁成 /3 的生产 SourceVersion；ConflictRecord/1 的 createdAtFrontier 不按 Frontier/2 解释。存在 decoder 不等于所有历史 prototype 都曾部署或 active，但真实存在的历史恢复责任也不能被新 wire、I 重建或重新授权抹掉。

## 5. 版本与兼容

P1 当前作者候选实际登记如下；本节只路由 owner 已冻结的版本边界，不创造第二套 wire：

- D6 Control 的控制协议由 1→2，用于当前提交与恢复接口；PreparedIntent 由 1→2，用于冻结准备态输入和计划；Policy 由 2→3，用于当前能力与披露规则；SourceVersion 由 1→2，用于区分完整生产版本；Frontier 由 1→2，用于表示已验证连续封存的因果前缀；ObservationScope 由 1→2，用于限定先验可观察范围；DependencyProof 由 1→2，用于承载闭合依赖完整性证明；InstallationNotice 由 1→2，用于在安装前记录原基线和组件前后像。WriteProtection 新增 strict|observed_only 保护轴；SourceObservation/1 继续表示当前观察，SourceVersionRef/1 继续选择完整当前观察，SourceStamp/1 继续表示拟议受管版本地址，三者保持既有版本号。
- 新生产路径把 ContentCompletionProof 的当前版本由 /2 推进到 /3：/3 的 sourceChanges 使用生产 SourceVersion/2|absent，并记录 actual frontierBefore/frontierAfter；历史 /1 与 /2 的原 bytes、SourceVersionRef sourceChanges、decoder、pins 和恢复门继续保留，不回填或重编码。
- 新当前冲突记录使用 ConflictRecord/2+Frontier/2；历史 ConflictRecord/1+Frontier/1 继续原 decoder。ConflictKey/1、ConflictId 格式和 `D6-ConflictKey/1` hash domain 没有升版。
- 新决议的受保护 revision-token 路径使用 RevisionTokenBinding/2 与 d6_source_revision/2；旧 d6d/d6r/d6a profile 和 DocumentRevision/ResourceRevision/AnnotationRevision 的 opaque decoder 原样保留。SourceRevisionPlan/1 是 P 内部 plan 类型，SourceStamp/1 是拟议版本地址；二者都不是新的公开 current-source wire，也不会改变 D3/D4/D5 的内层 selector 形状。
- DependencyProof 仍是 /2；本 P1 补齐的是 DependencyKey/2 的十四类 closed union、排序、stamp、owner/range/continuity 与错误边界，不因此再升 DependencyProof wire。machine Registry 根 schema 仍是 D6-Terminology/2；48 个 concept identity、13 个 crossStageBinding、ownedNames 与 firstFreeze 保持，P1 只同步选定定义和 binding contract，不人为再升 Registry schema。

D3 identity operation/receipt/resolver 的 11→12 候选已存在，ledger key 已加入 CommitDomain；这不等于 P2 已完成对 SourceRevisionPlan/RevisionTokenBinding/DependencyKey/ContentCompletionProof/ConflictRecord/恢复分流的消费。D4 公开 Entry/Type/RelationReadContext/Binding/Recurrence/effect shapes 与其 sourceRevision/OccurrenceKey/Entry selector wire 保持不变，D5 v1 domain/limits 与 revision-bound locator 也保持不变；新的 D6 外层资格通过完整 SourceObservation/1、SourceVersionRef/1、DependencyProof/2 和 owner input 组合，而不是改写这些内层 wire。

D1、D3、D4、D5 现有候选继续只表示其已经形成的候选消费面。P2 D3/D4、P3 D5 的新后像必须实际消费本 P1 最终 producer 后才能解除相关 owner_update_required/proof_unavailable gate。D7 也不能只补一个新版 Prepared：Query Algebra、Value/CEL、View、Narrow Field Qualification、Definition Transfer、Preview/Effects、Execution/Action、Prepared Action Binding、Scenario Dispositions、Terminology Lexicon/Registry 与 Implementation Impact/Test Outline 必须共同配套；D8、D9 与 D10 十八份 owner 也仍需真实消费者修订。

历史 D3 v9/v10/v11、D6 wire1、Policy/1/2、SourceVersion/1、Frontier/1、InstallationNotice/1、ContentCompletionProof/1,/2、ConflictRecord/1、旧 revision-token profile、D7 PreparedActionBinding/1,/2、D8 PreparedEditBinding/1 以及实际存在的 Result/ByteHandle、saved/planned/unknown 记录继续按原版本、原 bytes、原授权/pins/continuity 履约；没有部署证据的 prototype 不自动提升为 active compatibility，真实存在的历史责任也不因新版本取消。

任何上面的作者版本登记都不表示 accepted/activated/implemented。PROPOSAL/replacements 只是路由；最终是否共同生效仍取决于 P2/P3、D7–D10 实际消费者、旧十一项 OPEN 与 U6/U7 的后续处置、fresh 独立全量联合审查和协调接受，以及之后 A2 自包含重建与另一轮 fresh Pro 全局终审。

## 6. D4 catalog 不变

固定 catalog blob：
`ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb`

继续定义 4 QualifierSetSpec、22 aliases、61 Fields、7 Facets、1 CalendarSeriesScopePolicy及原 limits。本候选不修改 catalog。

特别保留：
- `people/labeled-text-value.label` 是 optional `semantic_code` contribution-set；
- codes `people/other|people/personal|people/work`；
- `people/phone` 等复用 alias；
- label absent不是empty/unknown/default other；
- display label不等 SemanticCodeId；
- relation canonical owner/inverse与 Calendar comparator/series-scope保持。

## 7. D5 domain 与 hard limits

仍无 Record/RecordCollection durable domain。Document table row/cell是 revision-bound occurrence；
  Node Collection membership是 D7 Query派生，不是 parent/owner/persistent membership。

hard limits继续：
- collection mutation explicit Node targets ≤1000；
- native table structured row targets ≤1000；
- preview page ≤200；
- fetched page ≤200；
- import batch new Nodes ≤1000。

更窄budget可降低，不得放宽；pagination前N不能冒充全集。

## 8. 当前已配套与仍待 consumer/owner

| Owner | 当前候选状态 / 尚待协调 |
|---|---|
| P1 D6 | Storage、Control、Terminology Lexicon、machine Registry、Implementation Impact/Test 五个 owner 私人作者后像已实际补齐，本 PROPOSAL/replacements 负责最终路由登记；整体仍未独立接受、未激活、未实现 |
| D1 | G0-B 已存在普通保存产品状态/指标与 strict、durable_observed_only 边界候选；不因 P1 路由完成而自动激活 |
| D3 | 固定 C 已有 wire12、owner descriptor、SourceObservation、companion/receipt 等候选边界；P2 仍需实际消费 SourceRevisionPlan/1、RevisionTokenBinding/2+d6_source_revision/2、D3 所属 DependencyKey/2 范围、saved/planned/unseen 恢复与同 P seal 规则，然后接受 |
| D4 | 现有三份候选已保留 production SourceVersion 与 observerDomain 分离、current Observation 外层资格及原 inner selector；P2 仍需实际消费 relation_incidence、calendar_scope、registry、temporal_rules 的完整范围/stamp、revision binding、CP3 接收后本地 Observation 与恢复分流 |
| D5 | 现有三份候选已保留 source/locator 外层资格、native structured strict 与 collection/bulk strong gate；P3 仍需实际消费新的 currentness、DependencyProof cut、strict|observed_only 普通保存与恢复规则 |
| D7 | Query Algebra（查询代数）、Value/CEL（值与表达式求值）、View（视图）、Narrow Field Qualification（窄字段资格）、Definition Transfer（定义转移）、Preview/Effects（预览与效果）、Execution/Action（执行与动作）、Prepared Action Binding（准备动作绑定）、Scenario Dispositions（场景处置）、Terminology Lexicon/Registry（术语与注册表）及 Implementation Impact/Test Outline（实现影响与测试）均需真实 consumer afterimage；complete Query/Result/Action 还要消费 query_scan、当前授权世代、完整正负范围和 reset 规则 |
| D8 | Source/Live/Read 负责来源、实时状态和读取配套，Draft/Edit Map 负责草稿与编辑映射，IME/Undo 负责输入法组合与撤销，ReliableSaveState 与可移植发布负责保存和发布状态，冲突、多会话和协作检查点仍须绑定完整当前 SourceObservation；同时继续按原合同解码历史 SourceVersion/1 与 PreparedEditBinding/1 |
| D9 | 仍需形成真实消费者后像：为范围受限的固定证据建立配套，令 ImportJob 消费完整的构造、导入与导出 cut，冻结精确导出输入和发布依据，并由真实 owner 处理外部绑定与版本比较 |
| D10 | 十八份实际 owner 仍需完成消费者配套：对 recipient/target/payload 提供可检查后再同意的审批绑定，持续证明 ApprovalUse 与 sourceOccurrenceKey 的连续性；Run、Lease、Automation、Workspace 和 deployment 层级的 Money 谱系、provider 外部结果 unknown、claim、stop 与执行责任都必须按原 owner 保持耐久连续，U6/U7 仍未闭合 |
| review/acceptance | 原 P0=0、P1=3、P2=8 共十一项 OPEN 保留；最终组装字节仍需 fresh 独立全量联合审查和协调接受，之后还要 A2 自包含重建及 A2 后另一轮 fresh Pro 全局终审 |

依赖新 P1 producer 的 managed/strong success 在对应 P2/P3/D7–D10 consumer 和 fresh 联合接受完成前继续使用 owner_update_required、proof_unavailable 或该 owner 原有 unavailable 结果；不能因为 D6 文件、fixture、作者检查或 CI 存在就半包激活。反过来，已经批准且不依赖缺失 strong producer/consumer 的 ordinary `.adoc`/Resource 读取、Draft、完整合格人工整源保存和局部离线操作不得被永久取消。ordinary 成功也不升级为 complete Query/Action/Automation 资格。

## 9. 大库、冲突与历史结果

T_first_open、T_first_edit、T_first_reliable_save、T_full_search_ready、T_OCR_ready继续分开，不承诺未经实测秒数；G0-B 已登记 ReliableSaveState/InputRetention 与 strict 可靠保存、durable_observed_only 耐久保留的指标边界。durable_observed_only 仍不计入旧 strict T_first_reliable_save，不解除后续 consumer、联合审查和激活 gate。

D4/D5 local operations不因无关 I coverage永久只读；strong complete consumer可扫描source，无法完成则 unavailable。

r5 receipt、current r6、I rebuild、P loss、external A→B→A分别处理；任何后续补验都不改写旧 receipt。多replica source/table/Field/collection冲突显式记录，不LWW。

## 10. D10 旧终审状态

固定 B13 继续是 **REVISE**，术语/双语 FAIL，P0=0、P1=3、P2=8，共 11 OPEN：

- P1：`R08-B13-P1-01`、`R08-B13-P1-02`、`R08-B13-P1-03`
- P2：`R08-B01-P2-01`、`R08-B02-P2-01`、`R08-B02-P2-02`、`R08-B02-P2-03`、`R08-B05-P2-01`、`R08-B11-P2-01`、`R08-B12-P2-01`、`R08-B13-P2-01`

本候选不关闭、重分类或独立接受任何 finding。后继不可变候选仍须 fresh 完整联合审查：新提案、D10 十八份、固定 S49 与全部真实 replacement owner 后像。作者检查/CI不是独立接受。
