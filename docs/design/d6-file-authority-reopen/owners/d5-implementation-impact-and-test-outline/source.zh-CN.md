---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：21e90c5d-66a2-476b-b852-5a77ac4e7f13。

# D5 实现影响与测试轮廓 — D6-FA-r01

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 的 D5 v1 实施/验收义务继续保留；本文件当前消费 actual A D6 的 SourceVersion/2 生产历史、SourceObservation/1、SourceVersionRef/1、CommitDomain/2、Frontier/2 与 SemanticState，actual B D1/D3 规范接口，以及本批当前 D4 与 D5 main；并继续覆盖 local-vs-complete、partial-index 和 multi-replica 验证。H2 D3/D6 与真实 saved decision 仅作为历史兼容和恢复背景，不作为当前 producer，也不据此声称接受、激活、实现、完整阅读或独立验收完成。

## 1. 实现影响图

~~~text
closed intent + minimum disclosure/ObservationScope + domain/custody continuity
  -> original DecisionKey/request lookup -> saved delivery / original planned recovery
  -> unseen operation-applicable owner/Registry availability
       -> complete current SourceObservation + real source/pins/dependencies
       -> D2 exact-source parse / opaque revision-token locator
            -> D5 native structured transform -> full proposed source / strict install
       -> D4 carrier / strict Entry decode -> real managed inner revision / selector
            -> typed proposed state + relation effects
       -> D7 Query with explicit coverage
            -> partial exploration OR actual complete result / Prepared producer
       -> exact or permitted proved-unrelated scope_dependencies validation
       -> one native D3 or actual D6 prepared entry / single P planning CAS
       -> original SourceRevisionPlan where needed / install / single seal / CP3
~~~

SourceVersion/2继续表示生产版本历史：managed_source_version/2保留entityRef、commitDomain、observationEpoch、revision、changeId，其中changeId.commitDomain等于该生产commitDomain；external_source_version/2保留entityRef、commitDomain、observationEpoch、externalSequence；生产域可以不同于当前operation域。完整current SourceObservation/1要求observerDomain等于operation CommitDomain、entityRef等于sourceVersion.entityRef，并以当前observationEpoch、fileObjectBinding、evidencePins及同一cut内的control、Registry、incidence依赖提供外层保护。SourceVersionRef/1.sourceToken以d6_source_observation/1选择该完整Observation，InputDescriptor/2.sourceInputs[].observation实际承载它。watcher gap、replacement或discontinuous rematerialization即使production version、hash或row text相同也使旧token和依赖它的locator失效；I不能恢复资格。D5 current Document revision/locator及D4 inner selector wire保持不变。

Frontier/2 只表示已封存的因果和依赖前缀，不替代全集 Query、Registry 完整性或 payload 物化证明。只有受影响的 unseen 强操作等待实际 D7 完整结果及 Prepared producer；D5 不制造成功 binding。真实 saved/planned/unknown 记录先按原合同恢复。主文 §19.9–12 定义允许的 Frontier 扩展、producer 消费和剩余内部协调，不放宽 D7 完整结果重置规则。

没有 Record/RecordCollection durable storage。I中的table parse、collection result和membership candidate全部可重建。

## 2. 实现分块

### 2.1 原生 Document Table

需要：
- 精确源码表格解析器；
- 完整 current SourceVersion/2，以及通过 RevisionTokenBinding/2 和 tagged RevisionTokenSource/2 解析 managed 或 external 来源的不透明 Document revision token；
- 完整current SourceObservation/1，以及SourceVersionRef/1.sourceToken和InputDescriptor/2.sourceInputs[].observation承载的当前观察资格；
- 当前cut内真实fileObjectBinding、evidencePins、control、Registry、incidence依赖；
- 验证 table/row/cell 定位器；
- 保留不齐行的模型；
- 可表示单元格的编码器；
- 精确保留 trivia 与换行形式；
- 检测源码补丁重叠；
- 完整current source与唯一proposed full source；
- current authorization与D2/local typed检查；
- D6 safe-install、P seal/recovery和实际file-install evidence。

native table editor不是 D4 schema editor，也不依赖无关全 Workspace Query；这只缩小所需语义证明范围，不产生weak保存资格。structured cell/row/column/reorder继续要求strict保护；只有满足actual A §4.1全部条件的人工raw整源已有一个live Document保存，才能在planning开始前显式选择observed_only并冻结profile。ordinary与strict|observed_only是独立两轴，不能由UI形态或缺少无关全库证明推导weak。

### 2.2 D4 出现项编辑器

表格式 property editor必须调用 D4 Entry/Occurrence semantics，而不是创建 TableRow/Record。每个 add/remove/reorder/note继续绑定current source、D4 Registry以及原有inner sourceRevision、OccurrenceKey、expected Entry selector，并额外消费完整current SourceObservation/1、SourceVersionRef/1.sourceToken、InputDescriptor/2.sourceInputs[].observation及同一current cut内的fileObjectBinding、evidencePins、control、Registry、incidence依赖。watcher gap、replacement或discontinuous rematerialization后，旧token/selector不能因production version、hash、I或相同值重新续认；current authorization、typed admission、完整post-state与relation effects仍须实际验证。这里不创建新的request/plan wire，也不因表格式UI取得observed_only资格。

### 2.3 Node 集合

集合查看器消费 D7 result 和明确 coverage。新的 unseen 强集合操作必须使用实际已协调的完整结果及 Prepared producer；page/cache 或历史 Prepared/1,/2 都不能伪造当前资格，真实历史记录仍按原合同恢复。最终 D7 owner 后像仍是具名协调依赖，不由 D5 自造新版。

### 2.4 转换、导入与导出

row→Node/import由D9负责mapping/loss，D3负责fresh identity。D5只提供 source row domain。跨Workspace保持两个独立receipts。

## 3. 组件影响

| 范围 | 后续工作 | 禁止项 |
|---|---|---|
| 表格解析器 | D2 原生语法、定位器、不齐行与 trivia | 行 ID 或数据库行 |
| 表格编辑器 | 精确源码补丁与 D6 安装 | 附属文件或表格数据库 |
| Field 列表界面 | D4 Entry 适配器 | 推断出 Record |
| 集合引擎 | D7 结果与覆盖范围 | 把分页当集合成员范围 |
| 集合动作 | 固定目标与完整 cut | 提交时重新计算所有目标 |
| 导入 | D9 映射与 D3 新 Node | 把 IR 行 ID 当 NodeId |
| I | 解析与结果缓存 | 内容权威 |
| P | 仅决议与恢复 | 集合成员或当前源码权威 |

## 4. 测试轮廓

### 4.1 原生源码

1. 简单单元格编辑保留 delimiters、普通行和未修改的 trivia。
2. 不齐行保持不齐。
3. 缺少的尾部单元格仍缺少。
4. 保留 CRLF/LF。
5. 保留行内格式与转义分隔符。
6. 行间注释或空白阻止结构化重排。
7. 可表示单元格精确往返。
8. 无法表示的单元格返回 `unrepresentable_cell`，零写入。
9. 旧 revision 的表格定位器返回 stale。
10. 外部修改后相同行文字不能证明旧出现项的连续性。

### 4.2 行列操作

- 追加或移除一行；
- 不超过 1000 行的批量操作；
- 跨不齐行增删列；
- 编辑普通首行单元格文本，或单独修改 Document 标题；
- 重排满足安全条件的表格；
- 拒绝无法安全保留 trivia 的重排；
- 拒绝重叠补丁；
- 精确重新解析完整后像源码；
- current SourceVersion CAS，并保持 table/row/cell Locator 的不透明受保护 revision-token 绑定；
- 同一操作还必须绑定完整 current SourceObservation/1、SourceVersionRef/1.sourceToken、InputDescriptor/2.sourceInputs[].observation，以及同一 current cut 内的 fileObjectBinding、evidencePins、control、Registry、incidence 和 strict install 资格；
- 已绑定当前观察、授权、pins 或依赖变化都使旧计划 stale/reprepare；Frontier 扩展按主文 §19.9 和实际 owner 重置规则判断，不能用裸 SourceVersion、hash、I 或相同行文字续认旧 Locator；
- 局部 structured row/column 操作不要求无关 Workspace 全集 Query，但 structured cell/row/column/reorder 仍使用 strict 保护，不能由局部性或 UI 形态推导 weak 保存；
- 按 D6 规则恢复安装前后的崩溃。

即使只用于展示、没有 schema 声明，table header option 也必须拒绝。普通首行单元格编辑仍可通过既有 cell 操作；Document 标题使用独立操作。本次不新增 header 语法或动作。

### 4.3 Field 出现项界面

- 相同 D4 值的重复项仍是不同出现项；
- note 跟随出现项选择器，不跟随值；
- D4 类型、基数或必需性不满足时拒绝；
- 不可用命名空间可原样显示，但不能作 typed 修改；
- 本地 Field 合法但缺跨对象证明时，仅可进入 D4/D6 允许的 pending 状态；
- 不产生 TableRowId/RecordRef。

### 4.4 集合成员

1. 对相同 NodeRef 去重。
2. 按 Query 规范排除未授权或已 Trash 的 Node。
3. 同一 Node 可以属于多个集合。
4. 删除保存定义不改 Node。
5. 运输分页 200 项不等于完整成员范围。
6. 语义 `take` 改变成员范围。
7. 部分索引只能交付明确标注的 partial。
8. 占位或未物化源码不代表成员不存在。
9. 重建 I 重新计算结果，不写作者内容。

### 4.5 集合创建策略

- 合法的显式 parent；
- parent 已过期、不可见或已 Trash；
- 不回退到根节点；
- 默认标题、正文与源码；
- Template 与默认事实映射；
- requireMembership=false 的本地创建；
- requireMembership=true 的完整后状态 Query；
- 复验相同定义、参数、授权与依赖；
- Query 排除新 Node 时，整个强 Action 拒绝；
- 缺少 D7 新 Prepared producer 时不可用。

### 4.6 从集合移除

- 显式 refs 选择器可求逆；
- 简单 D4 Field 谓词对应一个已预览修改；
- 任意派生、过滤或聚合不可据此求逆；
- 界面与 Trash 明确区分；
- 过期预览不重新选择目标；
- 权限变化阻止提交，不删除 Node。

### 4.7 删除方式

机械测试应验证：
- 删除表格行只改变 Document；
- 删除 D4 出现项仍经 D4 门；
- 从集合移除只改变定义或事实；
- Trash 改变 D3 生命周期；
- purge 仅允许 managed_atomic，且不可逆；
- 通用 Delete 不得在 View 改变后悄悄改变含义。

### 4.8 批量与限额

对目标数量 0、1、200、201、999、1000、1001 分别验证：
- 执行适用的目标和分页限额；
- 更小预算可提前拒绝；
- 不自动抽样或截断；
- 提交时目标集合与预览在字节及身份上相等；
- all_result 要求完整 cut；
- partial 结果不能作为修改输入。

导入批次另测 1000 与 1001 个新 Node。

### 4.9 跨 Workspace

目标复制成功而源 Trash 失败时，明确显示部分转移状态，不能称原子移动。两份回执分别保留各自 OperationId、CommitDomain 与 authority。

### 4.10 多副本与冲突

- A/B 修改同一单元格；
- A/B 修改不同行，但文件 generation 冲突；
- 显式三方源码合并；
- A 修改集合定义，B 修改成员事实；
- 一个副本只有分页或占位；
- hash 相同但 observationEpoch 已变化；
- production SourceVersion相同但 current observationEpoch、fileObjectBinding、evidencePins、control、Registry 或 incidence 依赖改变时，旧 SourceVersionRef/1.sourceToken 与依赖它的 Locator 必须 stale/reprepare；
- watcher gap、replacement 或 discontinuous rematerialization 即使 production version 相同也使旧 token/Locator 失效；裸 hash、I、相同行文字或 ABA 都不能恢复 continuity；
- SourceVersion/2 的 production commitDomain 可以不同于当前观察域，但 SourceObservation/1.observerDomain 必须等于 operation CommitDomain，entityRef 必须等于 sourceVersion.entityRef；
- 不按 mtime/LWW 决定胜者；
- 合并不创建隐藏行身份。

### 4.11 r5/r6, I/P

- r5 以 pending(collection) 状态提交；semantic_pending(collection)只表示缺少 collection 全集 proof，不表示 empty，也不能把 typed invalid、source invalid 或缺失 strong evidence 洗成成功，更不能授权 Action、all_result、bulk 或 Automation；
- r6 后来取得当前完整结果，并且只证明 r6 及其 current SourceObservation/SourceVersion/cut；
- 重放 r5 返回原始字节，后来的 r6 proof 不修改 r5 历史 receipt；
- 删除或重建 I 不会升级 r5，也不能恢复旧 membership/cut proof；
- 丢失 P 不会重建批次决议或批准，也不恢复 Money 或 execution custody；
- 新设备取得新 replicaEpoch，保留相同 NodeRef，不自动取得执行 custody。

### 4.12 部分索引与完整动作

部分元数据或 Query 索引可呈现本地列表，但固定反例必须拒绝将其用于：
- 完整成员范围；
- requireMembership 后置条件；
- 从集合移除的逆操作；
- bulk/all_result；
- 集合负约束；
- Automation 目标集合。

在 D7 规范允许时，完整源码扫描可以形成所需完整 cut；否则强动作不可用。

### 4.13 领域用例

People 的重复电话或姓名行不成为 Record。Organizations 的逆关系行解析到 D4 的实际作者 owner。Calendar 派生出现项继续保持派生性质。Library 的引文、作品和资源行保留各自领域。原生表格行转换始终创建新 Node。

## 5. 历史兼容与术语

- 准确保留 D5 六个 conceptId、owner、所属名称及 firstFreeze；
- 不引入 TableRowId/RecordRef/CollectionRef/ViewRef；
- 保留原 D5 v1 语义；
- 已保存 D7 PreparedActionBinding/1,/2 只按真实原版规则重放；
- 不猜测 D7 新 Prepared；
- D3 v9/v10/v11 与 D6 wire1 的历史决议保持不变。

## 6. 性能与资源验证

后续基准测试记录表格行数与单元格数、源码字节数、解析耗时、补丁大小、内存峰值、索引覆盖、集合候选与结果数量、页数及取消行为。本设计不作秒级性能保证。

实现必须对大型表格与 Workspace 保持有界资源开销，不能仅为了让普通局部编辑看起来完整而扫描无关内容。

## 7. 完成边界

后续实现批次必须同时提供 Core、适配器、用例、双语文档、历史重放与真实执行证据。文档 CI 或作者自审不是产品符合性证明。

旧D10 B13仍REVISE、术语/双语FAIL、11 OPEN。

## 8. fixed-S 架构影响与完整回归义务保留

本节把 fixed-S D5 Impact 中仍有效的架构影响、Record 分支退役和完整测试案例并入当前候选。D6-FA-r01 当前映射消费 actual A D6 的 SourceVersion/2 生产版本、SourceObservation/1、SourceVersionRef/1、InputDescriptor/2、CommitDomain/2、Frontier/2 与 SemanticState，actual B D1/D3 规范接口，以及本批当前 D4/D5；只替换外层文件权威、当前观察资格、SourceVersion/CommitDomain/semantic-pending/多replica消费，不删除下列 fixed-S 语义与fixture义务。SourceVersion/2 保留原有生产版本字段：managed_source_version/2包含entityRef、commitDomain、observationEpoch、revision、changeId，且changeId.commitDomain等于该生产commitDomain；external_source_version/2包含entityRef、commitDomain、observationEpoch、externalSequence，生产域可以不同于 current observerDomain；SourceObservation/1 的 observerDomain 必须等于 operation CommitDomain，entityRef 必须等于 sourceVersion.entityRef。Frontier/2 只表示 sealed causal/dependency prefix，不替代全集 Query、Registry 完整性或 payload materialization proof。本节仍是未接受、未激活、未实现的候选映射，不声称产品实现、独立接受或额外完整阅读完成。

### 8.1 跨阶段架构影响

D2继续提供 native table/row/cell grammar、revision locator、exact source局部编辑；Saved Query/View只作Node内occurrence。禁止row identity、Record正文、ViewRef和读取时矩形化。

D3继续提供NodeRef、owner-local Resource/Annotation、copy/fork/continue、Trash/restore和receipt；
  D6-FA新decision使用wire12，但不增加Record identityMap arm，也不让occurrenceKey跨revision resolve。

D4继续提供Registry、FieldId、TypedValue、Facet/relation公共post-state、
  raw source保留和Entry/schema admission。
  table column不覆盖Field schema，不恢复people blob或Entry Annotation。

D6负责单物理副本/Server后端持久提交资格、SourceVersion/2生产版本历史、SourceObservation/1当前观察资格、SourceVersionRef/1、CommitDomain/2、Frontier/2、source patch、完整read-set/negative dependency、safe install、P seal/recovery和有限权限。SourceVersion/2保留原有生产版本字段：managed_source_version/2包含entityRef、commitDomain、observationEpoch、revision、changeId，且changeId.commitDomain等于该生产commitDomain；external_source_version/2包含entityRef、commitDomain、observationEpoch、externalSequence；生产域可以不同于当前operation域。SourceObservation/1以observerDomain等于operation CommitDomain、entityRef等于sourceVersion.entityRef以及当前fileObjectBinding、evidencePins、control、Registry、incidence和cut保护当前操作，InputDescriptor/2.sourceInputs[].observation承载该完整观察。Frontier/2只表示sealed causal/dependency prefix，不替代全集Query、Registry完整性或payload物化证明。D6不建立Record store，不用whole-namespace replacement扩大冲突/权限，也不把ordinary与strict|observed_only混为同一轴；structured/bulk/collection/Action/automation/Approval/Money不因本地UI或局部证明而获得weak保护。

D7负责去重Node result、typed/occurrence result、editable columns、creation/membership proof、
  frozen bulk targets、paging/revocation；Record分支必须完整退役，join/group row不默认可编辑Node。

D8五表面必须区分删table row、删Field fact、移出collection、Trash；展开multi-value summary，保留conflict Draft，不能用grid row index写入或因Mobile容量改变语义。

D9显式CSV/Excel/JSON mapping、loss、fresh import、finite batch、Office三类binding分域；
  不执行external formula、不implicit upsert、不让ordinary Node持久export-only schema。

D3 拥有 SourceBinding/OriginBinding 身份、准确外键比较范围和 active/retired 转换语义；D6 保存受管目录及其连续性。D10 拥有 provider contribution 和真实外部来源/版本 profile，D9 负责映射与导入。provider cache 不是受管 Record 作者存储，不建立第二 CRUD/引用域。

### 8.2 旧 Record 分支必须成套退役

未来D7/实现不得只删一个records字符串后保留旧语义。必须同时消除：
- RecordCollectionRef/RecordRef及其TypeSpec/Query参数；
- records 扫描域、recordCollection 选择器与 record schema 身份；
- record/node relation专支；
- row.record.fields等Field/CEL overload；
- collection UUID和record compound-ref equality/group/distinct/cache/export专支；
- 持久 Record 出现项或 provenance 种子；
- record schema权限/read-set/lens/action target/View passthrough；
- 对应decoder、capability、fixtures、locale/API aliases；其中成套退役的是新的 active Record 执行域及其专用 API/capability/aliases，不机械删除已经承诺用于 saved decision、receipt、unknown recovery 的原版本 decoder 与原 bytes。旧 D5、D3、D6 wire1 和 D7 PreparedActionBinding/1,/2 的有效历史恢复继续按各自原协议解释；这些历史 decoder 不能恢复当前 Query/strong Prepared、approval 或 Money 资格，也不能仅凭缺少部署证据就把 prototype 自动提升为全 active compatibility。

必须保留的通用语义是NodeRef和普通TypedValue准确类型、D4 FieldId/RegistryBinding、通用authorized Query envelope、准确target/revision重解析、derived row无durable identity和ActionEvidence分域。

### 8.3 native table完整检查

必须覆盖：
- 竖线与反斜杠转义；
- CRLF/CR/LF；
- 重复行；
- 不齐行表格；
- 空白与注释 trivia；
- 空表格；
- 已过期的定位器；
- 合法 Inline 与 Ref；
- 无法表示的文本；
- 未修改字节精确保留；
- 失败时零作者写入。

structured patch后必须重parse完整proposed source，并同时保留不透明 Locator revision-token 或真实 managed D4 内层 selector 与外层 current SourceObservation/1、SourceVersionRef/1.sourceToken、fileObjectBinding、evidencePins、control、Registry、incidence、cut 和 structural strict install 资格；I、hash 或相同行文字不能补造这些证明。任何可确定的 source CAS、当前观察、授权、依赖或 durable-install 资格失败都必须在作者写入前 reject 或 conflict-reprepare，不得部分保存。若 install/ack 结果未知，则必须进入真实 recovery_unknown，并按原 operation 对账实际结果；不能伪称零副作用、Saved，也不能把未知结果当成新的 operation 重试。

### 8.4 集合创建与成员关系

必须覆盖：
- path predicate与普通property predicate在move后的差异；
- empty collection默认parent；
- ad-hoc Query必须显式parent；
- Template 冲突；
- append ordinal并发；
- filter为true但Query top/limit排除fresh Node；
- definition/params/schema/auth变化使旧plan stale；
- requireMembership true/false；
- page/paging/cache与semantic membership分开。

### 8.5 Field 与并发

必须覆盖：
- 两个相同phone值但notes不同；
- 三条历史assertions；
- 相同值使用不同 occurrenceKey；
- 外部复制引起 key 冲突；
- 多个 carrier block；
- 未知命名空间；
- 删除最后一个必需出现项；
- inverse UI必须写真实canonical owner；
- D4 selector/read-set稳定。

并发fixture：A修改一个name note，B新增phone，C只有phone权限。不能用whole namespace覆盖；old revision plan不能直接重放；replan后不相关facts必须保留；无法证明时保留conflict proposal。delete+readd same key ABA不能误命中。

### 8.6 lifecycle 与 cross-Workspace

同一Node在多个collections只代表同一identity。hidden subtree必须进入Trash preview；root Trash拒绝；restore保留原identity；copy fresh。

Workspace transfer的target copy与source Trash是两个独立receipts。target失败不动source；source Trash失败显示target copied / source retained。不得构造D5跨Workspace原子move。

### 8.7 scale、limits 与 batch recovery

target fixtures必须覆盖999/1000/1001，grid page 199/200/201，wide row触发更窄bytes/dependencies预算；Template descendants计入import count。loaded count从不等于total，禁止partial-success冒充complete。

大型import未来验收至少：
- 10,000 Nodes；
- 10 batches；
- Workspace 输入超过可用工作内存；
- 第 6 批提交前后崩溃；
- 进程重启；
- 网络断开；
- 重复恢复。

每batch原子；job准确报告partial committed state且不能重复create。当前文档不声称这些产品测试已通过。

### 8.8 ICS 与 recurrence

必须覆盖：
- 系列规则与 override；
- 用有界 Query 处理无限 recurrence；
- external subscribe/sync 与 adopt/import分开；
- 不同 SourceBinding 下使用相同 UID；
- cancel/delete/unbind/reimport/offline/timezone；
- 不物化 Record 或隐式 occurrence Node；
- VFREEBUSY/VTIMEZONE 不产生 Node；
- VTODO→Task和VJOURNAL/VEVENT显式mapping。

SourceBinding只定义foreign-key比较域；active OriginBinding才关联ForeignIdentityKey到NodeRef。UID/RECURRENCE-ID/LogicalOccurrenceKey都不成为NodeRef。

### 8.9 Query / Office 与 zero-row语义

必须覆盖：
- empty result仍有schema；
- nonterminal zero-row page不当EOF；
- join/aggregate columns默认read-only；
-普通Document table、Node collection、Query value table的Office binding分域；
- ordinary Node零export-only持久配置；
- legacy Record token明确拒绝。

partial collection export标partial或complete-export拒绝，不能静默截断到transport page。

### 8.10 terminology 与五表面一致性

D5 controlled names唯一归属；Document Table/row/cell、Node Collection、Field occurrence、derived row、Resource preview分域。退役API成套删除，不能误杀合法用户内容、历史prose或第三方format。

Desktop/WebUI/Server/CLI/Mobile在同一Core语义下处理上述意图；surface差异只影响interaction/capability，不改变identity、membership、limit或error语义。

### 8.11 D6-FA-r01新增回归

在上述 fixed-S matrix 上额外覆盖：
- ordinary 与 strict|observed_only 是独立两轴；offline native structured cell/row/column/reorder 在无关 I 未完成时仍可凭完整真实局部证据保存，但使用 strict 保护，不因缺少无关全集 Query 或 UI 形态取得 weak 资格；
- 人工 raw 整源保存仅在 actual A §4.1 全部资格满足时可选择 observed_only：trusted interactive_source_save、恰一个 existing live Document、ordinary + replica_local、完整 source read/replace、write set 为空或仅该 Document、无 applicable body/Field/Node-control deny、无 identity/parent/order/lifecycle/sharedPolicy/Registry/Calendar-scope/other-entity mutation，并且 DraftBase 等于 selected current Observation；必须在 planning 开始前人工明确选择并冻结 profile；
- 任一 weak 资格缺失、strict 失败、known competition、stale Base、watcher gap 或 continuity gap 都不得 fallback 为 weak，并要求 reject 或 conflict-reprepare；unknown install 保持 recovery_unknown，prepare 只代表 proposal/read-before/pins 等准备证据，不是 Saved；
- observed_only 路径验证已读前像 B 与用户输入 N 的耐久保留；安装 N 可以覆盖从未观察到的外部 C，后来 C 也可能替换 current file，但不得因此丢弃已耐久的 B/N；
- structured/bulk/collection/promotion/Action/Automation/server checkpoint/Approval/Money 全部保持 strict，不因本组验收条款放宽；
- SourceVersion/2 stale/ABA，以及生产版本相同但已绑定当前 SourceObservation/1、fileObjectBinding、evidencePins、control、Registry 或 incidence 依赖已改变的情况；发生 watcher gap、replacement 或 discontinuous rematerialization 后，即使生产版本相同，也不能继续认可旧 SourceVersionRef/1.sourceToken 或 Locator；
- 多副本修改相同或不同行产生冲突；
- 显式三方合并提案；
- semantic_pending(collection)只表示缺少全集 collection proof，不表示 empty，不洗掉 typed invalid、source invalid 或缺失 strong evidence，也不喂 Action、all_result、bulk 或 Automation；
- rebuild I不恢复old proof；
- lost P不恢复batch decision/approval/Money；
- new replica保留NodeRefs但没有execution custody；
- r5 pending replay与r6 current完整结果分开，r6只证明其current Observation/SourceVersion/cut且不修改r5历史receipt原bytes；
- Server多人table/collection Draft并存而commit仍由同一持久边界排序。

这些均是未来可复核验收条款，不表示产品测试已经执行或通过。

### 8.12 恢复的固定 S 案例与当前 producer 回归

以下都是未来一致性验收义务，不是已经执行的产品测试：

| 输入或竞争条件 | 必须得到的结果 |
|---|---|
| 旧 Task Node/checklist union，或删除标签后仍保留 Record selector、参数、cache 分支 | 拒绝退役 active 域；Task 是带 Facet 的普通 Node，不是 checklist 出现项 union；按真实上游域重建 D7 provenance 和 ActionEvidence |
| 标题、完整 Field、D4 支持的结构成员，与 NodeRef、路径、逆关系、聚合、投影状态列比较 | 仅前三者可暴露实际 owner 编辑能力，不把派生列变成通用可写单元格 |
| 三条历史 assertion、值相同但 notes 不同的电话、偏好摘要或新增观察 | 保留每个出现项和历史；新观察追加，明确纠错才替换；逆关系编辑只写 canonical 作者端一次 |
| 预览后 Template revision 变化、新计划前 parent 子项数变化、已有获胜计划 | 新提案拒绝或重新准备并显式审阅，不跟随最新 Template；已获胜计划按原恢复规则处理，不改源码或 ordinal |
| D4 depth 8/9、members 64/65、items 256/257，或禁用位置的嵌套 union/集合 | 本地编辑和导入都执行真实 D4 admission；未知 JSON 成员明确保留或损失，不静默删除 |
| 一个输入行连同 Template descendants 产生 1001 个 Node；1000 个有限效果分多页 | 前者 limit_exceeded；后者保留每项效果且可完整取得，不截断或暗拆批次；不能用 D5 grid 上限禁止合法大型 D2 文本或通用 Query |
| 不同来源实例、scope、映射命名空间内相同 UID；retired/non-live OriginBinding；未绑定只读 provider | 执行准确 D3 外键及状态转换，不按 UID/标题/路径暗更新；只读订阅可用但没有可更新绑定 |
| external Document 源码没有 managed 整数 | 原生不透明 Locator 使用真实 external RevisionTokenSource 分支；D4 内层 selector 只等待明确获权 managed admission 后的新请求；raw/read/repair/Draft 独立可用；未知 admission 不猜 revision |
| domain A 的 managed before revision 90，domain B 已证明 H=0，随后 B epoch 变化或返回 A | B after=1；epoch 不重置 B 历史，返回 A 时继续 A 自己的 H；外域 before/externalSequence 不给 after 捐值；空历史不明或 MAX 时失败 |
| 外域 managed before 的 raw no-op、某 owner 未变而另一 owner 变化、等 bytes external admission、源码删除 | 未变 owner 保留完整旧版本，无 SourceRevisionPlan/H；变化 owner 用原计划；admission 真正产生 managed after；删除 after absent，不造删除 revision |
| 允许 scope_dependencies，具有连续已封存的无关扩展且原全部输入不变 | 保留 P 证明后同一原计划可继续；原 expectedFrontier/Notice 基线、targets、pins、tokens 不变 |
| 向量同样增长但缺中间证明、负向 membership/auth/Registry/pin 变化，或 watcher gap 后 Observation 变化 | 扩展不能成功；stale/conflict/reprepare，不靠重签名恢复；D3 managed_atomic exact 和 D7 完整结果重置保持 |
| 仅丢 I 与真实 source/range 连续性丢失；完整空范围与隐藏/未物化范围比较 | 保护证据仍在时可不写作者数据重建 I；真实证据缺失不等于完整空范围或旧证明有效 |
| 使用真实原生 descriptor 的 raw wire12 create，与 D6 d6_commit_request/2 比较 | 原生输入其余资格完整即可无 planToken/PreparedIntent 成功；额外未声明成员拒绝；真实 D6 入口校验所携 token；必需 D7 resolution binding 不能删成 raw 输入 |
| 同 key 不同 owner 或规范请求；r6 后重放 saved r5、旧 TTL 过期或当前 D7 缺失 | mismatch 先于新业务；saved 按原披露范围的当前权限交付准确原 bytes，不重执行或重查旧业务完整性 |
| install 前后 planned、seal 未知、seal 后发布失败 | 保留原 candidate/H/after/pins/budget/TTL/Notice 和原操作；不另 prepare 或假定零效果；已 seal 发布只重发原 proof/outbox，不重装、不新增 ChangeId/H、不再收费或使用 Approval/Money |
| target copy 成功但 source Trash 拒绝或结果未知 | 保留各自域绑定 key 和 receipt，报告 copied/source retained 或明确 unknown；不自动另起删源，也不推断原子移动 |

资格顺序用例还包括隐藏 source 和 Registry 不可用的成对情景：先取得披露权限、ObservationScope 和实际 Registry owner，再读受保护 source 或解码 Entry；不能产生禁读、存在性泄露或按超时猜状态。无效 source/本地 typed 事实不能进入 semantic_pending。仅因 unseen 新 consumer 尚未协调，不能拒绝 saved 交付和原恢复。

PL-IR-01 portable Locator 跨副本资格现已有 D6 稳定地址 + D3 fresh-current-Observation 作者规则；它是**针对本 exact candidate 的独立复核义务**，不再是基础算法未决。应按 D6 PL01–PL18 与 D5 主文 §19.8/§19.12 核对，特别覆盖纯同步正向读取以及“新读取绝不复活旧 table/occurrence selector”。另一项最终 D7 complete-result/Prepared/preview producer 仍是真实内部协调依赖。上述本地测试都不能自行接受任一依赖，仍需对新的准确 hash 独立复核。


### 8.13 接受边界

这些是未来可复核合同案例。mechanical preflight、文档CI、作者自查、历史Pro审查都不等于D6–D10、A2或产品host实现验收。

旧D10 B13仍REVISE、术语/双语FAIL、11 OPEN。
