---
source_language: zh-CN
translation_status: source
---

[English](D6-IMPACT.md)

# A2 D6 实施影响与测试大纲

状态：A2 D6 作者候选；不是独立接受，也不是已运行产品测试。

## 1. 实施切片与唯一 owner

D6-FA-r01 的实现必须按逻辑 owner 分开，禁止为了方便重新合并成“一个数据库保存一切”。

| 切片 | 实施职责 | 明确禁止 |
|---|---|---|
| F1 portable file backend | .adoc/Resource current bytes、FileBinding、safe install primitive、外部变化观察 | 用P/I中的body覆盖用户文件；hash+rename冒充CAS |
| F2 可移植元数据 | 身份、子节点顺序、生命周期/Trash、Annotation、共享策略/信任、变更/前沿/冲突记录 | 两份可独立写 parent/order；把 path/title 当 Ref |
| P 耐久控制 | SQLite 决议/恢复/未知/批准/claim/Money、PreparedIntent、pins、执行责任 | 保存全库当前正文作为读取回退；被同步器合并 |
| I 派生索引 | 清单/解析/搜索/OCR 可删 SQLite，分层覆盖/检查点 | 身份/策略/回执 owner；部分索引冒充全集 |
| D Draft/会话 | Draft/输入/选择/IME、本地或 Server 协作临时状态 | 作为作者 revision 或 commit evidence |
| C coordinated consumers | D3/D4/D5/D7/D8/D9/D10新版本消费 | 半包激活、generic D6绕过原owner |

Implementation 必须有静态 owner audit：每个 current truth字段恰有一个逻辑写owner。任何兼容双写、启动时silent migration、旧authority.sqlite body mirror、portable metadata↔P双向同步current state均不合格。

## 2. 分阶段实施顺序

### S1 Portable reader + no-write open

- 打开现有文件型Workspace，读取portable metadata入口并验证路径containment/closed records。
- 当前 Document/Resource bytes从普通文件读取；index不存在也能打开指定文件。
- 外部invalid bytes可进入Source/repair inventory，不被replacement字符静默保存。
- 不允许产生新版managed commit，直到S2–S5满足。

### S2 Durable control + legacy replay

- 建立库外 control.sqlite3、WAL/SHM 与私有 pin 区，并保存 canonical request/fingerprint、DecisionKey、planned/committed/unknown 恢复、预算/attempt、安装状态、必要 pins、ApprovalUse/claim/Money/external unknown 与执行责任；P 不是 current body 或 portable structure 的第二 owner。
- 完整支持实际历史 D3 v9/v10/v11、D6 wire1 commit/receipt/error、Policy/1/2、SourceVersion/1、Frontier/1、InstallationNotice/1、ContentCompletionProof/1 与已经存在的 ContentCompletionProof/2、ConflictRecord/1、旧 revision-token profile、D3 primary receipt/companion、D7 PreparedActionBinding/1,/2、D8 PreparedEditBinding/1 以及实际历史 Result/ByteHandle 记录。按记录真实保存时的版本、原 bytes/fingerprint、授权、pins、clock/continuity 与恢复规则分派，禁止自动重编码或升级。
- saved、planned、unknown 必须是不同恢复分支。saved 在共同 disclosure、CommitDomain/P continuity 与原 request/fingerprint 定位后，只按原实际 effect/mode 或结果披露范围做当前交付授权，再重放原 receipt/error/effects bytes 或补原版本 publication/outbox；不得要求旧 source/Frontier/业务依赖重新成为 current，也不得回写后来 current source、重新收费或重新分配版本。planned 只恢复原冻结 plan、InputDescriptor、适用 SourceRevisionPlan、pins、OperationId、预算/attempt、安装状态与原版本依据；未 seal 时没有本次 ChangeId，也不能 new-prepare 第二个成功。unknown 保留原 pins、外部 effect、费用/批准/claim 与执行连续责任，不从当前文件、相同 hash、I 或重新授权猜结果。
- historical/predecessor v2 fixture 可以覆盖其 recorded SourceRevisionPlan/1、RevisionTokenBinding/2、d6_source_revision/2、ContentCompletionProof/3、ConflictRecord/2 和 DependencyKey/2，但仍必须在 feature gate 下保持 not_in_release/unsupported_version 或 owner gate；fixture 存在不表示历史 prototype 全部 active，也不表示新 producer 已被消费者接受。
- P-loss 测试必须证明：不能从 portable/current files、Derived Index、相同 digest 或空 control DB 合成旧 receipt、ApprovalUse、Money、unknown、clock/pin continuity 或旧生产域 H(D,E)。若 portable current 完整可验证且没有未决安装风险，可按 replica registration 建新 ReplicaEpoch/CommitDomain 继续不依赖旧全局责任的 ordinary content；旧域历史责任仍不可重建。

### S3 Portable metadata + replica model

- 此阶段要实现 ReplicaRecord、CommitDomain/2、ChangeId/1、Frontier/2、完整 managed/external SourceVersion/2、SourceObservation/1、SourceVersionRef/1、内部 SourceRevisionPlan/1、SourceStamp/1、RevisionTokenBinding/2+d6_source_revision/2，以及 current DependencyKey/3 + DependencyProof/3 + InputDescriptor/3 + PreparedIntent/3、InstallationNotice/3、ContentCompletionProof/4、ChangeRecord/1 和 current ConflictRecord/2。历史 Frontier/1、ContentCompletionProof/1,/2 与 ConflictRecord/1 必须继续使用原有解码规则和原始字节；旧版令牌配置也保持原语义。
- SourceVersion/2 的生产 CommitDomain 与生产 observationEpoch 和当前 SourceObservation/1 的 observerDomain、当前 observationEpoch 分开。对生产域 D 和实体 E，H(D,E) 是连续 sealed managed 历史中的最大 revision；只有从域 birth/registration、P continuity 与已验证 portable sealed history 证明完整空历史时才允许 H=0。真实 managed after 使用 checked H+1，MAX 不 wrap，同一生产域跨 production observationEpoch 不重置 H；跨域往返各自继续各自 H，相同裸 revision 不可比较。
- true raw no-op 保留原 SourceVersion/2；纯 placement/lifecycle/control 且 source unchanged 不推进 H；source deletion 的 after=absent，不创建删除版 SourceVersion 或 H 增量，但作为真实 portable effect 仍在 seal 获得 ChangeId；equal-byte external admission 仍是 external→managed 显式接纳，按当前生产域 H+1 形成 managed after，externalSequence 永不充当 managed sourceRevision。
- 只有确实会产生 managed after 的原 plan 才冻结 SourceRevisionPlan/1：实际 before Observation 或明确 absent、同生产域 lastIssued/完整空史证明、拟议 SourceStamp 与 exact after pin。winning plan 后这些版本依据不可重采样；seal 前重验同域历史。plan/staging/InstallationNotice 都不分配本次 ChangeId；唯一 P seal 才把 SourceStamp 与真实 ChangeId 合成为 managed SourceVersion/2 并原子推进 H。
- RevisionTokenBinding/2 使用受保护 d6_source_revision/2，形状只含 token+RevisionTokenSource/2，专职稳定生产版本地址，与当前 SourceObservation 分离。计划在 C/Q 位置物化前固定一条拟议 token；winning CAS 将它冻结，唯一 seal 即使当时没有 Locator 也把它选为该 managed SourceVersion 的唯一 canonical binding，并通过 original sealed-outbox 关联认证后运输。loser/aborted token 不能借另一 seal 生效。新的 observer 只有独立证明 current SourceObservation 的完整 sourceVersion 与已解析地址相等，才能取得一次新读取资格；watcher gap 会废掉旧 observation/runtime 证据，但不会因此改写稳定地址。D3 Locator 外层 opaque 词法、D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence 以及 D5 revision-bound locator wire 保持原形。
- Frontier/2 只表示每个 CommitDomain 已验证连续 sealed 的因果前缀；它不证明 payload 已物化、placeholder 已下载、Registry/index 完整或 D7 complete Query。远端记录接纳不重放 remote OperationId，也不为纯运输另造本域 ChangeId。
- 新 FA transport 使用 ContentCompletionProof/4：它携真实生产 SourceVersion before/after 或 absent、actual frontierBefore/frontierAfter 和真实 components。接收端必须验证 InstallationNotice、proof、全部 components、生产版本与完整连续 sealed 链后才推进 Frontier，并用自己的 CommitDomain、FileObjectBinding、observationEpoch、evidence pins 与当前 control/Registry/incidence cut 建立新的 SourceObservation/SourceVersionRef；禁止复用发送端 sourceToken。
- 新 conflict 使用 ConflictRecord/2+Frontier/2；ConflictKey/1、ConflictId 与 D6-ConflictKey/1 hash domain 不变，历史 Record/1 仍用 Frontier/1。child order 继续只有一个有序列表 owner；新设备注册新 ReplicaEpoch，retired epoch 不复活。

### S4 Safe file installation

强路径至少实现并证明 conditional_replace 或真正 exclusive_write_window；advisory lock、read-hash-then-rename 或最终 digest 相同都不能作为 strict CAS/排他证明。expected-absent 新文件仍用 create_only。

observed_replace/WriteProtection=observed_only 只允许受信人工 interactive_source_save 在 planning 开始前显式选择并冻结，而且必须同时满足：恰一个既有 live Document；ordinary+replica_local 整源保存；完整 source read/replace；author source write set 为空或仅该 Document；没有适用 body/Field/node-control deny；不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar scope 或其它 entity；Draft Base 等于当前选定 SourceObservation。noninteractive、managed_atomic、D3 identity/structure/lifecycle、D5 structured cell/row/column/reorder、bulk/collection、D7 strong Action、Automation、server checkpoint、Approval 与 Money 都保持 strict。planning 开始后 strict capability失败、known conflict、失权、durability failure、strong obligation失败或其它资格缺失都不得 fallback 成 observed_only。

observed_only 原 plan 必须耐久保留实际 read-before B 与用户输入 N。唯一放宽是最后可信检查后到 N 安装前从未观察的外部 C 可能被 N 覆盖且没有可恢复副本；以后另一个 C 也可以替换 current file，但 B/N 的耐久保留责任不消失。已观察竞争、stale Base、watcher gap、third_state 或失权不属于该风险；unknown installation/provenance 固定 recovery_unknown，相同 hash/文本不得猜 success。

故障注入必须覆盖：
- stage write 前；
- stage data flush 前/后；
- SourceRevisionPlan/版本依据 durable planning 前/后；
- InstallationNotice/3 flush 前/后，并验证其中没有本 decision 尚未 seal 的 ChangeId；
- 每个 component install 前/后；
- file data flush；
- directory entry flush；
- installed verification；
- P seal transaction 前/中/后，并验证 seal 是唯一 decision commit point；
- ContentCompletionProof/4 与 outbox 写/flush 前/后；
- response/receipt delivery 丢失。

strict 路径在每个点验证不会静默丢失已观察竞争字节；observed_only 验证不覆盖任何已观察竞争、B/N 已耐久、未观察 C 不被虚构、unknown 不猜 success。prepare/retained 都不是 Saved；只有 strict install+P seal 得到 reliable，合格 observed_only install+P seal 得到 durable_observed_only。seal 后 publication/delivery 失败只补同一原 proof/outbox 或交付原 receipt，不重装 N、不换 OperationId、不重新分配 ChangeId/managed revision、不推进 H、也不重复收费。

### S5 Ordinary save 与 semantic pending

- D6 source-save 继续区分 ordinary 与 complete 语义资格，并与 strict|observed_only WriteProtection 分轴；ordinary 成功不自动取得 complete Query/Action 资格，strong consumer 失败也不能改写成同一 Action 的 ordinary success。
- 普通既有 Document 整源保存绑定完整 current SourceObservation/1（含真实 production SourceVersion/2）、FileObjectBinding/evidence pins、当前 lifecycle/policy、实际 MutationFootprint、D2 parse 和实际修改的 local typed facts；合法但尚未证明跨对象/全集 obligation 时只能 semantic_pending。semantic_pending 可供明确 local Source/edit/read consumer 使用，但不能进入 relation/unique/Calendar 强 mutation、all_result/post-query、Automation、purge 或其它 complete consumer。
- DependencyProof/3 必须消费十五类闭合的 DependencyKey/3：source、document_format、lifecycle、placement_range、ref_inbound、relation_incidence、calendar_scope、registry、temporal_rules、authorization、foreign_binding、query_scan、replica_registry、conflict_record、execution_resource。每个 key 的 owner、范围、epoch/revision stamp、evidence pins、正负集合与披露规则都服从最终 Control；禁止用 free JSON、I row、provider 的“已同步”状态、最终 hash 或 Frontier 数字替代真实完整证明。
- 完整范围来自当前授权下的一致 snapshot/range barrier，或无缺口 continuous change chain 加最终复验。空范围和非空范围使用同一标准；partial/building index、index miss、placeholder、unknown decoder、I/O failure、缺 shard 或隐藏但无权读取都不能证明 empty。只删除/重建 I 不会破坏仍完整保存在真实 P/M owner 中的 range epoch/revision 与连续链；这些 correctness facts 真正丢失或发生 gap 才使旧 proof 失效并要求新 epoch+完整重枚举。
- frontierPolicy=exact 必须保持原完整 Frontier equality；D3 managed_atomic 仍走 exact。scope_dependencies 只能接纳从原 expectedFrontier 到当前 cut 的真实连续、已验证 sealed、非回退且能证明与原 source/control/authorization/全部正负 DependencyKey 无关的扩展。原 canonical request、InputDescriptor.expectedFrontier、DependencyProof.baseFrontier、targets、Query/selector、pins、proposed bytes、WriteProtection、owner input 与版本依据不重签、不重采样；实际绑定依赖变化或未知 gap 不得洗成“无关”。反过来，只依赖完整真实局部范围的 ordinary/local/replica_local 操作不应被无关全 Workspace Query/index gate 永久阻断。
- 当前 Observation 的连续性和版本令牌连续性必须与生产 SourceVersion 分开检查；即使生产版本、修订号、摘要或文本相同，发生监视器事件缺口、对象替换或不连续重新物化后，也不能恢复旧 SourceObservation、SourceVersionRef 或 d6_source_revision/2 的当前有效性。
- submit/recovery 必须先经过共同 closed decode、state disclosure、CommitDomain/fence/P continuity 与 DecisionKey/request-fingerprint 防 probe，再分 saved/planned/unseen。saved 只按原实际效果/结果披露范围做当前交付授权并重放原 bytes；planned 复用原 plan、SourceRevisionPlan、pins、预算、安装状态和版本依据且没有 pre-seal ChangeId；只有 unseen 才按当前 owner/Observation/DependencyProof 建新 business decision。当前 r6 新证明不得回溯拒绝或升级 r5 saved decision。
- installed write set 用原 planned after 验证，未写 dependency 继续对原 before/cut；自己的安装不能自冲突。late competition、撤权、unknown provenance 与 third_state 保持 conflict/paused/recovery_unknown，不能新写业务 rejection。
- 真实 fixed-C wire12 继续作为 historical input；current A2 D3 已是 wire13/Descriptor3/Proof3，且两个有界 D3 finding 已独立 CLOSED。D4/D5 继续保持各自独立 review 状态；本 D6 候选的生产域 H/SourceRevisionPlan/revision-token profile/Key3/ContentCompletionProof/4/ConflictRecord/2/U5 恢复规则仍需本次 fixed829 五 finding 修复的独立复核及后续真实 joint/global acceptance；不能把旧候选解释为自动支持新 producer，也不能因此永久取消不依赖缺失 strong consumer 的 ordinary 文件读取、Draft、合格人工整源保存或局部离线操作。

### S6 Sync/conflict

- 两副本离线正文/正文、创建/创建、移动/编辑、移动/移动、Trash/编辑、策略冲突、部分传输、placeholder、身份碰撞继续逐项测试；普通多设备/offline 内容能力不因 complete Action consumer 尚未配套而被全局冻结。
- 每个新 FA portable change 以 InstallationNotice/3、ContentCompletionProof/4、全部 listed component bytes/metadata、真实生产 SourceVersion before/after 及从 notice.baseFrontier 经 actual frontierBefore 到 frontierAfter 的完整连续 sealed 记录链共同证明完成。任一部分缺失、placeholder 未 materialize、生产版本 metadata 不全或链有 hole 都是 incomplete，不能 mint identity、推进 Frontier、从 I 补齐或把 index/provider 状态当完成。
- 接收端验证生产历史后建立自己的 current SourceObservation/SourceVersionRef；不得复制 sender sourceToken，也不得仅因相同 production revision/hash/text 跳过本地 FileObjectBinding、observer observationEpoch、pins 与 continuity。
- 并发 portable heads 使用 ConflictRecord/2+Frontier/2；ConflictKey/1、ConflictId 与 D6-ConflictKey/1 hash domain 保持不变。历史 ConflictRecord/1+Frontier/1 原 bytes/decoder 保留；unsealed external competition、third_state 或 unknown install 没有真实 ChangeId 时不得捏造 head。
- conflictId 必须稳定；new sealed head 改变完整 key 时旧 open/resolution_prepared 记录按其版本进入 superseded 并建立后继，不按 mtime/LWW 自动选 winner。source merge保留原 bytes，policy 不做 allow union。
- current A2 D3 wire13 已提供 D3-owned placement/lifecycle/identity resolution 的 native Descriptor3/companion consumer；真实 wire12 只作 historical exact dispatch。D6 仍不得用 generic payload 绕过 D3，current success 必须匹配 wire13 owner contract 与 Key3/SourceRevisionPlan recovery；该有界 D3 closure 不等于 D6 或全局接受。
- purge 仍是强操作：必须证明完整 inbound/owner closure、complete semantic proof，并取得 registered replica 对 purge Frontier 的真实接纳确认或显式 retirement；provider“已同步”、replica 数量或单一 Frontier row 都不能代替 acknowledgement。

### S7 Server multi-user

- 多用户并行read/prepare/Draft；
- 不同文档commit可并发计算，仅必要range lock和P seal短暂串行；
- 同文档两Draft不互相覆盖；
- 权限撤销与seal竞争；
- Server failover同时fence P与author file path；
- 第二Server实例不能绕P锁直接rename托管文件。

实时协作仍not_in_release，直到后续OT/CRDT adapter及D1 release evidence；但candidate conformance要有抽象session model测试，确保receive/broadcast/checkpoint三状态不混淆。

### S8 Index/large workspace

- index删除重建、分层coverage、parser/search/OCR独立失效；
- 10k/100k/1M小文件；
- 十几/几十GB以附件为主与正文为主各一组；
- cold/warm filesystem cache；
- SSD/HDD或具名慢速磁盘，记录CPU/RAM/OS/filesystem；
- sync placeholder/按需下载；
- 重启续建、批量外部修改、watcher gap；
- exact/NFC/regex candidate回读与漏召回回退scan；
- no-body-replica检查。

## 3. 必须测量的五个时间

所有大库报告必须分别给出：

- T_first_open：Workspace入口/活动目录可响应；
- T_first_edit：活动Document Draft可输入；
- T_first_reliable_save：只统计 strict 安全install+P seal；observed_only 的 durable_observed_only 单独记录，D1 更新前不填入旧 strict 指标；
- T_full_search_ready：指定search profile/范围具备complete coverage；
- T_OCR_ready：指定附件/OCR profile完成或明确失败。

不得只给一个“启动时间”。T_first_edit不等于author save；T_first_reliable_save不等于portable published；T_full_search_ready不等于OCR ready。

还必须记录峰值RSS、index DB大小、P DB大小、protected pins、读取字节、文件数、解析吞吐、P seal延迟、portable publication延迟、重启续建工作量、单文件和批量增量代价。

没有实际数据不得声称“达到具名同类应用的速度”“几秒重建”或任何具名性能通过。

## 4. no-body-replica 验收

每个候选包实际创建Workspace后，扫描所有：

- durable control SQLite表/索引/FTS shadow；
- derived index表/FTS shadow；
- temp/staging/pin区；
- portable metadata；
- log/audit；
- crash recovery文件。

断言：

1. I不存在完整current Document body列或完整全库AST序列化；
2. P不存在可作为全库current source读取fallback的body镜像；
3. 每个完整source pin都有明确owner/purpose/retention/容量，可追到具体plan/conflict/history；
4. source pin达到可释放条件后可物理回收而不损坏canonical decision/receipt；
5.旧协议要求decision-lifetime pins的历史fixture仍保留；
6. F中的普通文件被外部改变后，P/I旧bytes不能自动覆盖current。

检查只证明实际schema/bytes，不允许根据表名“contentless”推断无敏感内容；token/gram/position同样纳入泄露和容量审计。

## 5. 正反例验收矩阵

下列 FA01–FA30 身份保持不变，全部都是未来实现必须取得的证据，不是本作者候选已运行或已通过的测试。每项都同时服从当前 disclosure、CommitDomain/P continuity、真实 owner 与历史恢复边界。

| ID | 场景 | 必须结果 |
|---|---|---|
| FA01 | I 全删，10 万无关文档未解析，编辑一个 D2-valid 普通笔记 | 只重建本次 ordinary save 真正依赖的 current SourceObservation、局部 DependencyProof 与安装资格；有合格 strict primitive 时可达 T_first_reliable_save，不等待无关 index/OCR 或全 Workspace Query proof。全集 Action 仍必须等待自身 complete proof |
| FA02 | 既有目标缺 strict 条件/排他原语 | strict 返回 install_unavailable；只有受信人工在 planning 前显式选择并冻结、且满足 existing single live Document、ordinary+replica_local、完整 source read/replace、无适用 deny、无结构/其它 entity mutation、Draft Base=current Observation 的保存才可 observed_only，并仅在 durable install+P seal 后得到 durable_observed_only |
| FA03 | Base=A 后第三方写 B | 若竞争 B 在安装前已被观察，strict 与 observed_only 都停止并保留当前竞争状态、原 read-before 与输入；observed_only 只承担最后可信检查后仍未观察的 C 可能被 N 覆盖的风险，unknown installation 保持 recovery_unknown |
| FA04 | external A→B→A 加 watcher gap | 当前 observer observationEpoch 变化，旧 SourceObservation/1、SourceVersionRef/1、selector/map/prepared/evidence 资格失效。managed d6_source_revision/2 canonical binding 只继续表示稳定生产地址；只有受保护历史证明仍为同一 exact production SourceVersion，且新的 current Observation 独立合格时，后续新读取才可使用它。相同 bytes/hash 或 external event 不构成该证明 |
| FA05 | install after 成功，但实现仍把 written target 与 before 比较 | 测试必须抓出该错误；规范实现对 written component 验证原 planned after+installation provenance，只让 unwritten dependencies 继续对原 before/cut，并按 exact 或合格 scope_dependencies 重验 |
| FA06 | P seal 成功，ContentCompletionProof/4 写失败 | decision、真实 ChangeId、适用 managed SourceVersion/H、receipt/charge 与 ReliableSaveState 已固定；strict 为 reliable，合格 observed_only 为 durable_observed_only，portablePublication=pending；retry 只补同一 proof/outbox，不重装、不重新收费、不再次推进 H/ChangeId |
| FA07 | response 丢失，r5 已 committed，后来 r6 编辑 | 共同 continuity 与原 request/fingerprint 定位后，只按 r5 原实际效果范围做当前交付授权并返回原 receipt bytes；不要求 r5 source/Frontier/业务 dependency 等于 r6，不重写 current r6，不重复收费；撤权只可遮蔽交付 |
| FA08 | 两设备离线同笔记不同编辑 | 两个真实 sealed heads 形成 source_concurrent；新记录使用 ConflictRecord/2+Frontier/2，不 LWW，原分支 bytes/pins 可取；无 sealed ChangeId 的外部竞争不得伪造 head |
| FA09 | A move Node，B 改其正文 | 只有 placement 与 source 维度独立、相关 placement_range/source/authorization proof 均完整且 destination policy/structure 重验通过才组合；否则 conflict，不能用同一 SourceVersion revision 推断结构未变 |
| FA10 | A Trash，B edit | 不默认 delete-wins；保留 edit branch 与 Trash intent，lifecycle/placement/source 依赖分别证明，未选择 branch 的 ordinary read 不随机取一个 current |
| FA11 | purge vs 旧 replica restore | tombstone 不复活；旧 bytes 只能 fresh-copy/reconciliation。purge 还必须有完整 inbound/owner closure 及每个参与 replica 的 purge-Frontier acknowledgement 或 retirement |
| FA12 | child_list/Document/ContentCompletionProof 分批到达 | 只有 InstallationNotice/3、ContentCompletionProof/4、全部 listed components、生产 SourceVersion before/after 与完整连续 sealed 链互相验证后才接纳并推进 Frontier；任一部分缺失都是 incomplete，不 mint identity、不把 Frontier/provider“已同步”或相同 hash 当完成，receiver 自建 Observation 而不复制 sender token |
| FA13 | placeholder size 已知但 bytes 未下载 | not_materialized/source_unavailable；不能当 empty/not_found、不能从旧 locator/I/cache 补 current source，也不能生成完整 source/query_scan proof |
| FA14 | Ref 重复但 birth 不同 | identity_collision；解析器不得选择第一项；resolution 继续由 D3 owner 负责。真实 fixed-C wire12 只保留为历史版本，不能代替当前 wire13 的 native Descriptor3/companion consumer |
| FA15 | 构建中的索引漏掉一个关系目标 | relation_incidence 与 query_scan 的完整消费者必须返回 proof_unavailable、拒绝完整 Query/Action，或回读真实来源做完整扫描；索引未命中、只有部分结果行或重复得到相同摘要，都不能据此证明范围为空 |
| FA16 | semantic_pending source 进入 auto Action | complete consumer 明确拒绝；Source/明确 local read/edit 仍可在自身授权与局部真实证据下继续，不能因缺无关全 Workspace proof 而永久禁用 |
| FA17 | P 丢失、files/portable intact | 旧 production domain 的 H、planned/saved/unknown、approval/Money/external responsibility 不可从 files/I 猜回；portable current 完整可验证且无未决安装风险的范围可注册新 ReplicaEpoch/CommitDomain 继续 ordinary content，新域只从自身已证明空历史开始 H |
| FA18 | I 丢失、P intact | 若真实 P/M range epoch/revision、pins 与连续 change facts 仍完整，只 rebuild I cache，不重签 proof、不换 epoch、不重放 decision、不 mint Ref、不改收费；若 correctness facts 实际缺失，则旧 proof 失效并须当前授权下完整重枚举，ordinary local 不依赖该 strong range 的路径仍可用 |
| FA19 | copied P 数据库在两设备打开 | 不自动产生两个 execution holder；takeover 必须证明完整 continuity 并 fence 旧 holder，普通 replica registration 不能取得 Approval/Money/external consumption authority |
| FA20 | approval 一次、网络 response unknown、程序 crash | restart 先恢复 original request、provider binding、pins、Money/approval/claim/unknown continuity；不新 OperationId、不再 consume、不从当前文件或相同 hash 猜 provider outcome |
| FA21 | Money work batch 先 charge 后 crash | charge 保留；attempt/restart 不 refund，不因 P/summary absence 重置额度；continuity 或 clock 不明保持 planned/paused/unknown，不能伪造 terminal success/failure |
| FA22 | Server Alice/Bob 编辑不同 Document | 两 Draft 与 prepare 并行；commit 各一次，仅真实 dependency range lock 与 P seal 短串行；无关 coarse sequence/Frontier head 变化若已证明与原依赖无关，不应制造全 Workspace UI 互斥 |
| FA23 | Server Alice/Bob 同 Document 旧 base | first seal 成功；second 保留 Draft 并 stale/conflict，不覆写。相同文本或 revision 数字不能绕过当前 Observation/FileObjectBinding continuity |
| FA24 | Bob revocation 与 checkpoint seal 并发 | 唯一线性化；revocation 先赢则 Bob 未提交贡献不能借 Alice 身份 seal，seal 后的已提交历史只允许按当前 delivery authorization 交付，不被撤权改写 |
| FA25 | Server 故障切换只隔离 control DB、未隔离文件写入 | 验收失败；旧实例必须同时失去 P 与 author-file rename/write 能力，只有 SQLite fence 不足 |
| FA26 | 协作仅收到接收确认但没有检查点 | UI/API 不能显示 reliable、durable_observed_only 或 committed；仅接收或广播、Draft 持久化以及输入已留存都不是 Saved |
| FA27 | IME preedit 多事件+最终 input | preedit 不进入 author op；最终确认至多一个 Draft/checkpoint input transaction，后续 checkpoint 仍走当前授权、Observation、DependencyProof 与 seal |
| FA28 | protected conflict pin 超容量 | 新 prepare/install 受限或 planned 进入 paused_capacity；不能删 last-reference conflict/recovery evidence 后从 I 猜回 |
| FA29 | expired 新历史 effect pin | 按其新 retention 返回 effects_unavailable；不得用 current file、新 Observation 或相同 digest 伪造 old after。旧合同承诺的 decision-lifetime pin 不追溯删除 |
| FA30 | legacy wire1 saved decision | 按真实旧 decoder byte-equal replay，旧 pin/授权/continuity promise 保留；不得用 SourceVersion/2、d6_source_revision/2、ContentCompletionProof/3、ConflictRecord/2 或当前 stronger proof 重编码/升级旧记录 |

### PL-IR-01 便携位置设计验收表

PL01–PL55 仅是本修复候选的设计/一致性义务，**尚未作为产品测试执行**，也不因此自行关闭 P1、接受联合设计或证明实现。本修订进一步覆盖 PR3-PLIR-P1-01 的 authenticated-carrier chain；signature/canonicalization oracle 仍只是设计要求，不是密码学产品测试证据。

| ID | 场景 | 必须结果 |
|---|---|---|
| PL01 | A seal managed V 和 locator token t；完整运输把 V/源同步到已登记副本 B，源未改变 | B 验证历史 portable-trust declaration、exact canonical RevisionTokenSealArtifact/1 bytes 与 Ed25519 signature，把 association DecisionKey/ChangeId/V/SourceStamp 与 current CP4/ChangeRecord1 逐项核对，再建立自己的 current Observation O_B 且 O_B.sourceVersion=V。新的受权读取使用原 locator/token；合法 forwarder 只需转发原 artifact bytes，B 不信任新的 sender 声明 |
| PL02 | 同一 sealed V 仍是真实当前生产版本，但 B 的 current observation generation 合法换代并完整重建 | 新读取可从新的完整 Observation 重新取得同一稳定地址资格；旧 sourceToken、selector、ActionEvidence、PAB、Draft/map、PreparedIntent 仍失效且绝不被改写 |
| PL03 | 真实生产史为 V_A1→V_B→V_A2，最终 bytes/坐标与 V_A1 相同 | 指向 V_A1 的 locator 对当前 V_A2 仍 stale；比较完整生产版本/历史，不能比较文本/hash/裸 revision |
| PL04 | watcher gap、object replacement 或 discontinuous rematerialization 最后得到相同 bytes，但 exact production-version continuity 无法证明 | 旧当前资格仍 unavailable/stale；rescan/hash/I 不得声称持久地址等于当前源 |
| PL05 | 两个 preparation 的 SourceStamp 字段相同但 token 分别为 t/t2，只有 t 的 plan 赢 planning CAS 并 seal | seal 只签名 t 的 winning binding。为 t2 构造其它字段相同的 association 但没有历史 private key、复制 t 的 signature、或仅给相同 stamp/V/digest，都会在 signature 或 cross-field validation 失败并成为 unavailable；loser t2 绝不 canonical，也不构成 conflict |
| PL06 | 拟议 plan aborted/terminal-failed 或 seal 不可证明，后续 decision 复用同数值 H+1 | 旧拟议 token 没有有效 RevisionTokenSealArtifact/1，永久非 canonical；后续 decision 只为自身 binding/DecisionKey/ChangeId 签名，revision 数值或相同 SourceStamp 字段不能借该 artifact |
| PL07 | 一个 decision 同时 seal 多个 managed after V1..Vn，其中部分版本当时没有 Locator；各副本后来才第一次需要其中某些 Locator | 单一 P seal 对每个 non-absent managed after 都建立恰一份独立 keyed RevisionTokenSealArtifact/1 + outbox item；它们共享本 decision ChangeId，但各自绑定自己的 EntityRef/SourceStamp/V。之后首次 Locator 使用解析保留 artifact；任何副本都不得延迟 mint、派生或重签 token |
| PL08 | 两份非逐字相等的 RevisionTokenSealArtifact/1 对同一 exact sealed managed SourceVersion 声称不同 token | 只有两份 artifact 都独立通过历史 trust-key lookup、canonical-byte/signature verification 及全部 current CP4/ChangeRecord1/association cross-fields 时，才属于可达 integrity 矛盾/repair 状态。伪造、invalid 或 untrusted 的第二份 artifact 只被拒绝/不可用；不得 first/last wins，也不得静默 alias |
| PL09 | B 独立观察到与 A external source 相同 bytes，但没有 A 的原 external-event 证据 | B 不继承 A external token/event；普通 external read 继续使用 B 自己的 Observation |
| PL10 | 原 external-event 证据存在，且 B 独立 current Observation 证明 exact 同一完整 external SourceVersion | 新的受权读取可使用该 external 地址而不转 managed；当前 selector/write 仍需各自证据 |
| PL11 | Annotation target/document_range 指向同步到 B 的 managed V；目标 Node 后续按普通 lifecycle 进入 Trash/恢复 live | 源版本/坐标合格时 B 新读取可解析 exact target；D2 target exactness 与 D3 lifecycle 分开。新 Annotation edit 只有 fresh qualification 后才可 preserve target；旧 suggestion/edit preparation 不复活 |
| PL12 | QueryRef DefinitionAddress.locator 指向同步 V 中 saved definition | 新 invocation 依授权、exact Frontier、canonical version address、B 当前 Observation 和 exact saved-definition occurrence 一次解析；cycle identity 用真实 owner/version/occurrence 归一，不用 token 拼写；旧 ResultRowHandle/ActionEvidence 不复用 |
| PL13 | TemplateRecipe/2 固定 V，body_text 保存原 DocumentRangeLocator | B 的当前 selected Observation 必须等于 V，locator 还要通过 new-read 算法及 expectedText/inert-paragraph 门；版本变化为 dependency_conflict/stale；不使用该 locator 的模板不被全局禁用 |
| PL14 | PDF/image V 的 ResourceRegionLocator 同步到 B | 当前 Resource 授权/Observation 等于 V 时，原 page/profile/geometry 重新验证后同一地址可读；转换 Resource 或 bytes/orientation/version 变化仍 stale；d9rg1 单独不授能力 |
| PL15 | 调用方缺 owner/locator/source/definition 适用披露权限 | 在读取受保护 binding/source/definition/geometry 前返回原 non-disclosing outcome；不能泄露 stale/version/conflict/count |
| PL16 | publication retry、P recovery 或 I 删除/重建时，原 RevisionTokenSealArtifact/1、outbox item/pin 与历史 trust declaration 仍完整 | 只复用/重发 exact 原 artifact bytes 并重新验证，I 仅重建 token→version cache。不得从 stamp/V/digest 重构、选择更新 trust key、mint 或重签；若必要 admission 前真实 artifact/trust history 已丢失，相同 hash/rescan 不能重建真实性 |
| PL17 | 原 decision 为 saved、planned 或 outcome-unproved，随后当前观察/版本改变 | saved 按当前原范围交付授权返回原 bytes；planned 恢复原 proposed binding/token/map/pins/OperationId/version basis；unknown 保留原责任。都不能把新 Observation/token 换进旧 record |
| PL18 | D7 Q 两遍物化为拟议 managed after 生成绑定 revision 的 locator | 两遍及 winning CAS 之前先固定一个 token；seal 形成 V/ChangeId 后才把该已冻结 binding 放入 RevisionTokenSealArtifact/1 签名。signature 覆盖的 domain-separated body 不含 signature/pin/outbox address，因此不产生自引用或第三遍 Q；Notice3/CP4 不增加其 closed current schema 之外的 binding/artifact member，也没有第二个 CP4、第二 ledger 或第二 CAS |

| PL19 | fresh Workspace create/fork 到第一次 managed source seal | WorkspaceBootstrapPlan/4 携 WorkspaceTrustGenesis/2。issuer/target custody 建立受保护 root anchor；root self-signature、root-signed exact W/B domain declaration 与 PoP 全验证；同一 bootstrap P seal 提交 WorkspaceAuthorizationBundle/1 与 managed after，只使用狭窄 same-P genesis 例外，不要求 target 预先已有自身 trust history |
| PL20 | 同步/复制来的 Workspace 提供不同 self-signed root，或显式 import fingerprint 不匹配 | 拒绝 root/anchor 建立；不得把它当替换 anchor，不验证其 domain declaration，也不允许 revision seal；self-signature 只证明持钥，不授 authority |
| PL21 | fresh device 已合法锚定 root，注册 ReplicaEpoch R 后进行第一次 managed edit | registration 的一个 P seal 以同一 DecisionKey/ChangeId 原子提交 active ReplicaRecord 与 root-signed exact R-domain authorize declaration/PoP；只有接纳后 joining DomainSealKeyHandle usable，之后第一 seal 的 history_at 解析该 key 并成功 |
| PL22 | 普通 sync/copy 把全部 Workspace files、policy/trust history 和 revision-seal artifact 复制到没有受保护 private-key handle 的机器 | 合法 root anchoring 后可做历史验签/读取，但 authorize_new_sign 必须失败；文件占有、sender identity、Registry seed、package signature 或复制 public key 都不授旧域签名权 |
| PL23 | K1 seal V；之后普通 root-authorized rotation K1→K2 提交；V/artifact 在 rotation 后才晚到接收端 | validate_historical 以 V 的 proved frontierBefore 选择 K1 并验原 artifact；rotation cut 后的新 seal 必须 K2。不能因为 current key=K2 重签或拒绝旧 artifact |
| PL24 | revoke/rotate 先进入适用 current cut，旧 key author decision 后到 P seal | authorize_new_sign 重验看到新 bundle，在 commit 前拒绝旧 K。预计算 signature 只是 staging；install/recovery 保留原 plan/pins，走 paused/conflict/recovery，不伪造 seal |
| PL25 | 旧 key author P seal 先 commit，随后 ordinary revoke/rotation 在 publication retry 前提交 | committed decision 与原 artifact bytes 仍历史有效；outbox/recovery 只在当前 delivery authorization 下发布 exact 原 bytes，后续 key 状态不触发重签或再次分配 ChangeId/H |
| PL26 | domain private key 丢失；另一个场景是 Workspace root private key 丢失 | domain-key loss 阻止该域新 managed seal，直到 root-authorized loss_recovery rotate/add 创建新 usable handle；历史验签不受影响。root-key loss 阻止新的 trust mutation/registration/authority-domain 变更，但不使既有已授权 usable domain key 或历史签名失效；两者都不能从 portable bytes 修复 |
| PL27 | 某 trust declaration/predecessor/root signature/PoP/version-qualified DecisionKey→CP3-or-CP4 activation binding 损坏、缺失或重排 | 受影响 history prefix unavailable/integrity-conflicted，不能产生 RevisionTokenSealVerificationKey/1；不能按 arrival order、current key、I row 或相同 digest 补洞 |
| PL28 | 攻击者为 V 构造同 SourceStamp 但 token=t2，而 t 是真实 winning binding | 没有 anchored root/history 授权的 domain key 与 t2 自身 valid artifact signature/cross-fields 时，t2 只被拒绝/unavailable，不成为第二 canonical mapping；相同 stamp/version/hash/bytes 均不足 |
| PL29 | 对同一 exact V 的两份非逐字相等 artifact、不同 token 都真实通过 anchored root/history 与 current CP4/ChangeRecord1 | 可达 integrity 矛盾；Core 不按 arrival order/current host/current key 选择，必须进入 repair/conflict；与伪造第二 artifact 的普通拒绝严格区分 |
| PL30 | K1→K2 rotation/revoke 后删除并重建 Derived Index，但 public trust history 与 artifacts 完整 | I rebuild 只能重验受保护 anchor、累计 declarations、activation ChangeRecords 与 exact artifacts 后派生 current/historical projection；绝不创建 anchor、private handle、declaration、token、signature 或 signing authority |

| PL31 | 显式 anchor import 输入 root declaration R、expected fingerprint F，而 caller 误把或恶意把 R.rootKeyId 当成 declaration fingerprint | Core strict-decode R，复核 Workspace/key/selfSignature，按包含 selfSignature 的 canonicalRootDeclarationBytes 和 D6-Workspace-Trust-Root-Declaration/1 域计算 WorkspaceTrustRootFingerprint/1，再与 F 逐字比较。rootKeyId 属于不同值/域，不能匹配或 coercion；revision-1 predecessor 使用同一个 fingerprint object |
| PL32 | 共同 bundle n 为 Policy P、current K1；branch A 合法 ordinary rotate K1→K2，branch B 合法 revoke K1 mode=compromise；两者均为 root-signed n+1 successor，形成 policy_concurrent | 当前 wire3 policy_bundle_choice 精确选择 A 的 head+bundle address 与 P。Core 从共同 root/prefix 验证两支，把 B compromise 派生为 TrustConflictCarry/1，在 A 上 root-sign 一条 resolve_conflict declaration；由于 inherited compromise 指向 K1 而非 K2，结果 keep_current K2。两支/losing history 均保留；K1 artifact 只有在 B 原 compromise activation cut 之前才历史有效。不存在 arrival/LWW/allow-union |
| PL33 | 同一 trust conflict 中 selected bundle digest/revision 与 head 不符、某 branch version-qualified CP3-or-CP4/declaration bytes 缺失、roots 不同、trust 分叉但无 usable root handle，或 receiver 收到遗漏 B 的 inheritedCompromises 的 resolve_conflict | prepare 或 receiver admission 以 unavailable/integrity-conflict 失败，零 canonical successor。carry set 由 Core 派生且必须完整，caller 无法通过省略抹掉 compromise；不得 fallback 成 policy-only、猜 head、换 current key 或另做第二 repair commit |
| PL34 | old authority 已 fenced 后 continue 到 fresh server B2 | continuationCut 上：current rotated K1b → 同 decision 追加 revoke(K1b)+authorize(B2)；此前 revoke(K1,loss) 已使状态 none → 同 decision 只 append authorize(B2)；conflicted/gapped/unproved → 不激活。一或两条 declaration 共用一个 DecisionKey/activation ChangeId且无 current 中间 prefix；此前 loss/compromise facts 保持 public history |
| PL35 | 仅信任变化的策略组件更新、全新副本注册、随后停用副本 | 仅信任变化时，bundle.authorizationRevision 与 policy ComponentImage.version 各推进一次，而 Policy/3.revision 不变。replica_register 只能创建自身全新 ReplicaEpoch，并在同一决议追加单条固定 profile 的 authorize；不能管理其它提交域或密钥。replica_retire 只把记录置为 inactive，authorize_new_sign 随后因 active-domain gate 失败；不得隐式 revoke 或删除历史授权 |
| PL36 | 共同状态为 K1；selected branch 只是无关策略变化并仍保持 K1，而 losing branch 执行 compromise rotate K1→K3 | 递归 compromise extraction 必须从 losing rotate 的 replacesTrustKeyId 产生原始 K1 fact，并保留该 rotate 的原 activation cut。完整 effective union 禁止 selected branch 继续 keep K1；没有合格 fresh recovery 时结果为 none，后续 resolver cut 绝不能冒充原 compromise cut |
| PL37 | R1 已经携带一条原始 K1 compromise fact；之后 R1 所在分支与一条从未包含原始 compromise declaration 的分支再次冲突，第二次 resolver 选择后者 | 第二次 resolver 必须验证 R1 并递归折叠其 inheritedCompromises，重新加载原 root-signed declaration 与 CP3 activation evidence；若 selected chain 尚无该 fact，就再次携带同一原始 fact。经过两次或更多 resolution，原 activation cut 始终不变 |
| PL38 | 同一个原始 compromise fact 同时通过直接 declaration 和一个或多个旧 resolve_conflict 到达新 resolver | 所有逐字相同实例使用同一 factId，只保留一条并按 factId ASCII 升序规范排列；同 factId 但 bytes 不同属于 integrity conflict。不得按 arrival order 选择 source metadata，且原 declaration/activation evidence 必须继续保留 |
| PL39 | selected current key K1 被任一 effective compromise fact 命中，包括只来自 losing branch 或旧 inherited carry 的事实 | keep_current K1 非法。只有 exact affected domain/profile 符合 fresh eligibility 且显式请求恢复时才可 authorize_fresh，否则结果为 none；安全且无关的 current key 不能因为 freshAuthorizations 中出现就被 rotate |
| PL40 | 同一 resolve_conflict 为两个不同 exact domain/profile/key tuple 生成两条 authorize_fresh outcome，随后交换两份 possessionSignature | 每份签名只能验证由父 declaration 的 workspaceRef/revision/predecessor/decisionKey 加该 outcome 自身 domain/profile/key tuple 重构出的 PoP body。交换后必须失败；possessionSignature 与 rootSignature 都不进入 PoP body，因此不存在签名递归 |
| PL41 | 冲突只涉及策略，所有已验证 heads 对无关 domain D 都一致为 none，但 caller 把 D 放入 freshAuthorizations | D 不属于 affectedDomainProfiles，wire3 prepare 必须在生成 key 或 plan 前拒绝该无关 fresh 请求；resolver 不能退化成通用 add/rotate 入口 |
| PL42 | source_merge 的 wire3 prepare 成功，随后客户端预览并提交 | OwnerInputBinding/2 与 InputDescriptor/3 都使用 d6_conflict_resolution/2；exact branch/base/head/proposed-source/semantic pins 固定在 ConflictResolutionInput/2 与 previewBinding。成功只返回既有 d6_prepared_intent/planToken，最终只能走 d6_commit_request/2，由唯一 P seal 提交 source 与 conflict-record effect |
| PL43 | choose_source_head 的 wire3 prepare 成功，但提交前 chosen head 或 ConflictKey 改变 | exact selected-head source pin 与 expectedKey 已冻结在同一个 PreparedIntent/3/preview；最终重验发现变化后必须 stale/conflict_changed，commit 不能替换另一 head/source，也不能绕过既有 d6_commit_request/2 |
| PL44 | policy_bundle_choice 已 prepare，随后在 planned 状态崩溃，或变成 saved/unknown 后重试 | immutable descriptor 必须保留全部 branch version-qualified CP3/CP4 与 bundle pins、selected address、effective compromise union、inherited carries、outcomes、fresh PoPs、result-bundle pin 与 previewBinding。§5 saved/planned/unseen 顺序和 §8 recovery 只能恢复同一 plan/result 责任；不得用另一 Resolution2 重解析，也不得建立第二 submit、ledger/CAS 或重新派生新的 carry |
| PL45 | source_merge 解决合法 source_concurrent(H1,H2)，而冲突 subject 没有普通 current Observation | D6 从真实物理已安装 sealed head 生产 exact SourceConflictBefore/1，并从 base 加全部 head production versions 生产完整 SourceConflictVersionBasis/1；二者绑定到 d6_conflict_resolution/2。若 merged bytes 改变则冻结 SourceRevisionPlan/3。subject 不进 sourceInputs；step 6 复验 guarded before+basis；唯一 P seal 产生唯一 managed after/H+1 与 conflict effect |
| PL46 | choose_source_head 在权限/pins/ConflictKey 都合法但 A 无普通 Observation 时选择 H1 | actual installed before 与 chosen-head production version 分别从 sealed history/pins 完整证明，并绑定同一 DecisionKey/audience/key/arm，经既有 planToken 与 d6_commit_request/2 提交；不伪造 branch-current Observation，也不借用 D3 ConflictInstallInput |
| PL47 | 实际安装 H2 与所选 H1 的 source bytes 完全相同，但完整 production SourceVersion/2 不同 | 仍是真实 source admission：SourceRevisionPlan/3 冻结 H1 为 versionBasis、H2 真实物理版本为 before；seal 前无 ChangeId，唯一 seal 才产生当前 domain H+1，且 CP4 before=H2 物理版本、after=新 managed version。相同 bytes 不得抹掉 production-version 选择 |
| PL48 | source_merge 输出恰等于 installed bytes 且不选择另一个 production version，或 choose_source_head 选择已经安装的 exact production version | source 真正未变：不产生 SourceRevisionPlan/3、sourceChanges 或 H increment。ConflictRecord resolve/supersede 仍是真实 portable control effect，可使用 decision ChangeId；只有原规则下所有效果均空才是 raw no_op |
| PL49 | conflict heads 合法，但本地 physical installed head/FileObjectBinding/sourcePin/metadataPin 或其来源无法完整证明 | SourceConflictBefore/1 不可得，prepare 不能 source-success。历史 head bytes、相同 digest、selected branch evidence 或 I 都不能替代物理 before；不得伪造 H/after token/plan |
| PL50 | complete source resolution 真实需要跨单 source 的 D4 relation_incidence 或 D7 complete query_scan | 在任何 author/range read 前，owner profile 选择能覆盖该 closure 的最小既有 wider ObservationScope，通常 workspace_constraints，随后在 current 十五 key proof 中记录所有真实依赖，并在实际语义解析 managed Document 时包含 document_format；saveProfile=complete、strict、exact 均不降低 |
| PL51 | preparation 选择 local_source，但真实 owner-required closure 需要 foreign relation/query/member range | Core 必须在越界 read 与 planning 前返回 owner_update_required/proof_unavailable；不得先读后扩 scope、静默丢 dependency 或降级 complete/strict/exact |
| PL52 | 冲突 subject A 与另一个无冲突 source B 同时是实际依赖 | A 不进入 sourceInputs，其 source key 由 SourceConflictBefore/1+versionBasis guard；B 仍以真实 current SourceObservation 进入 sourceInputs，并有匹配 source DependencyKey/controlInputs entry。特殊路径不得吞并无关 current read |
| PL53 | caller/transplant 提供复制 before、错误 DecisionKey/audience/expectedKey/arm、缺 metadata pin，或 choose_source_head versionBasis 指向错误 production version | cross-field/guard 在 planning 前或 step 6 按既有 invalid/unavailable/conflict 路径拒绝。相同 bytes/hash、另一 head source pin 或 fake guard 都不能授权安装 |
| PL54 | 两个 source-resolution prepare 竞争；一个输 planning CAS，或 winner 在安装/seal 前后 crash 并 retry | loser/aborted 永不取得 ChangeId/H/token。planned recovery 只恢复 winner 原 InputDescriptor、SourceConflictBefore/VersionBasis、适用时 SourceRevisionPlan/3、pins/H/DecisionKey/preview/install state；saved recovery 只重放唯一 sealed result。不 reprepare、不重选 head、不改 H、不第二次 seal |
| PL55 | policy_bundle_choice 构造 InputDescriptor | controlInputs 只包含真实 conflict_record、authorization，以及当前十五 key union 中确实读取的其他成员。Frontier 由 expectedFrontier、DependencyProof.baseFrontier 与 frontierPolicy 表示；policy/trust history 保存在 OwnerInputBinding、branchEvidence、bundle/declaration pins 与 derivedPlan。portable_frontier_state 和 policy-history 永远不得成为 DependencyKey kind |
## 6. 权限与非披露测试

至少覆盖：

- caller持有ConflictId但无state disclosure → not_visible，不能知道record存在；
- replica_register但无source_write → 可登记但不能编辑正文；
- conflict_resolve但无source/policy/D3实际write → prepare在对应原权限门失败；
- execution_custody_admin不授Money增额/approval新建/source read；
- phone-only Field edit不能因ordinary source profile获得body/name；
- hidden sibling不因计算ordinal被枚举；
- current source external_invalid的详细parse/bytes仅按repair/source权限交付；
- receipt/effects replay撤权遮蔽但decision不改变。

## 7. Legacy 与版本矩阵

实现测试必须按“实际记录保存时的版本与 owner”同时保留并区分：

- D3 v9/v10/v11 historical saved request/receipt/error，以及实际旧 D3 primary receipt/companion；
- D6 wire1 commit/receipt/error 与旧 PreparedIntent/Token tag；
- Policy/1/2；
- SourceVersion/1，以及实际历史 d6d/d6r/d6a revision-token profile、DocumentRevision/ResourceRevision/AnnotationRevision opaque decoder；
- Frontier/1 与 InstallationNotice/1；
- ContentCompletionProof/1 和已经存在的 ContentCompletionProof/2；/2 的 sourceChanges 继续是 SourceVersionRef/1|absent，禁止按 /3 的 production SourceVersion/2 解码；
- ConflictRecord/1；createdAtFrontier 继续只按 Frontier/1，ConflictKey/1、ConflictId 与 D6-ConflictKey/1 hash domain 不变；
- D7 PreparedActionBinding/1,/2；
- D8 PreparedEditBinding/1；
- 已有 Result/ByteHandle 历史 token/record，以及它们原授权、pin、clock、continuity 与 expiry/reset 规则；
- 任何实际由旧 decision/pin 引用的 SourceObservation/1、SourceVersionRef/1 或受保护 token binding，只按生成它们的原合同解析。

decoder 或历史草稿文字存在，不等于对应 prototype 曾部署或已经 active；测试不得把“有 decoder”机械扩大成产品兼容承诺。反过来，一旦 saved、planned、unknown 记录真实存在，其原 request/decision/receipt/error、pins、期限、授权、unknown、费用/批准/claim、外部 effect 与 no-duplicate-effect 责任不能因新版本、I 重建、重新授权或缺其它 prototype 部署证据而取消。

新 v2/new-FA fixture 不能通过只改 wireVersion 或复制旧 bytes 生成。fresh-current fixture 必须独立按 current closed schema 构造，并覆盖 SourceVersion/2+SourceObservation/1、SourceRevisionPlan/1、RevisionTokenBinding/2+d6_source_revision/2、DependencyProof/3+十五类 DependencyKey/3（含 document_format）、InstallationNotice/3、ContentCompletionProof/4、ChangeRecord/1、ConflictRecord/2；另以真实 recorded decoder 单独覆盖 historical Proof2/Key2/Notice2/CP3，禁止重编码。old→new 隐式 upgrade 与 new→old fallback 都必须失败；unknown owner version 返回 unsupported_version/owner_update_required 或对应 unavailable，不走旧近似路径。

planned fixture 必须证明未 seal 时没有本 decision 的 ChangeId，恢复只续原 plan/pins/版本依据；saved fixture 证明当前 r6 业务 proof 不回溯拒绝 r5，交付撤权只遮蔽响应；unknown fixture 证明相同 hash/current file/Derived Index 不能猜 success/failure。新 current Observation 或新 revision token 不能重新签名旧 binding。

旧 canonical request 若保存完整 source/pin 是旧协议历史证据，不迁移；新 PreparedIntent/2 使用 InputDescriptor、closed owner binding 与 purpose-bound PinRef。测试须证明相同 hash、相同裸 revision 或相同最终文本但 production domain、SourceObservation、DependencyKey stamp、owner descriptor 或 pin continuity 不同，绝不能被视为 same input/currentness。

## 8. D3/D4/D5/D7/D8/D9/D10 后续消费门

Storage、Control、Lexicon 与 machine Registry 的 P1 私人作者后像已经真实存在，本 Impact/Test Outline 只把它们转换为未来验收义务；它不能单边把任何下游 owner 标为已接受。依赖新 producer 的 managed/strong 路径在对应 consumer afterimage 与 fresh 联合接受完成前保持 owner_update_required、proof_unavailable 或该 owner 原有 unavailable；不依赖缺失 strong producer/consumer 的 ordinary `.adoc`/Resource 读取、Draft、完整合格人工整源保存与局部离线操作不得被永久禁用。

- D3：current A2 D3 使用 wire13，并真实消费 `d3_identity_operation/13` OwnerInputBinding/2、DecisionKey/2、Frontier/2 策略、完整 SourceObservation/1 与十五类中 D3 所拥有的 lifecycle、placement_range、ref_inbound 范围；真实 wire9–12 继续 historical exact dispatch；会产生 managed after 时还要消费同一原 plan 的 SourceRevisionPlan/1 与 d6_source_revision/2，并把 primary receipt 与 D3DecisionCompanion/2 放在同一 P seal。D3 Locator 原 opaque revision-token 词法与实际 WriteScope 不得被 D6 扩张。
- D4：固定 C 已有 local-vs-complete 和较早 A/B/C 的 SourceObservation/Frontier/WriteProtection 消费候选，但 P2 仍须配套 production SourceVersion 与 observerDomain 分离、relation_incidence、calendar_scope、registry、temporal_rules 的实际枚举/stamp、RevisionTokenBinding/2 边界、saved/planned/unseen 恢复分流以及 ContentCompletionProof/4 接收后的新 Observation。RelationReadContext/2、RelationReadBinding/2 与 Recurrence /1 的原 closed wire 不改。
- D5：现有 local-vs-complete 候选仍须在 P3 消费新的 current Observation/currentness、revision-bound locator、DependencyProof cut、strict|observed_only 普通保存边界和恢复分流；D5 structured cell/row/column/reorder 一直 strict，不能借 ordinary weak save 降门。
- D7：查询代数 Query Algebra 要消费完整 query_scan 与 cut；值和表达式 Value/CEL 要消费真实版本/授权依赖；视图 View 要保持结果完整性与 reset；窄字段资格 Narrow Field Qualification 要保持静态独立性与非披露；定义转移 Definition Transfer 要复用同一 candidate map、SourceStamp 与 revision binding；预览/效果 Preview/Effects 要绑定原 plan、pins 与历史 retention；执行/动作 Execution/Action 要保留 complete evidence 和 current authorization generation；准备动作绑定 Prepared Action Binding 要形成真实新 consumer 后像；场景处置 Scenario Dispositions 要覆盖 positive/negative/unknown/recovery；术语与注册表 Terminology Lexicon/Registry 要同步真实 owner；实现影响与测试 Implementation Impact/Test Outline 要同步验收面。上述任一项都不能由 D6 free JSON、只改 Prepared 一页或粗粒度 Frontier 推断代替。
- D8：后续 Source/Live/Read 负责来源、活动状态与读取配套，Draft/Edit Map 负责草稿与编辑映射，IME/Undo 负责输入法组合与撤销，ReliableSaveState 负责可靠保存状态；可移植发布、冲突和协作会话也要在同一消费者后像中配套。这些消费者必须绑定完整的当前 SourceObservation，不能只绑定生产 SourceVersion；发生观察缺口或对象替换时必须重置或重建基线，同时继续保留 SourceVersion/1 与 PreparedEditBinding/1 的历史解码规则。
- D9：ImportJob/ExportPlan 的新 pins/version、construction/import/export cut 与 foreign version/binding consumer 仍需真实 afterimage；D6 不替 D9 发明外部比较器或 free payload。
- D10：十八份实际 owner 文件仍须完整消费 sourceOccurrenceKey continuity、recipient/target/payload 可审阅批准、ApprovalUse、Run/Lease/Automation/Workspace/deployment Money 谱系、claim、provider external request/result unknown、stop 与 execution responsibility。execution_resource DependencyKey 不吸收这些 D10 子合同；U6/U7 继续开放。
- 旧十一项 OPEN、A2 前后 fresh gate 与最终另一轮 fresh Pro 全局终审均不因上述 producer 文档、machine Registry 或 fixture 存在而解除。

不能因 D6 文件存在、作者形式检查通过或候选目录可读，就模拟这些 consumer “已经支持”、把它们计入产品 conformance，或把历史 decoder 全部假定为 active。

## 9. 文档/机器检查

候选文档自身至少检查：

- 中英文标题/章节编号与所有closed enum/key集合一致；
- terminology registry conceptId唯一、ownedNames冲突检测、旧firstFreeze逐项不变；
- replacements.json只列实际存在的后像，sourceBlob精确等于固定S；
- 不出现指向不存在owner后像的“已定义/已激活”语言；
- D6 v2 JSON例子可被严格JSON模板/人工schema检查，bool与Counter不混用；
- 新ConflictId、Frontier排序、ChangeId overflow、Policy/3能力矩阵有正反例；
- docs/design/snapshots、inputs、D10十八份在本批未变。

文档CI成功也不等于产品conformance或独立review通过。

## 10. G0-A 保存/接口增量验收

以下全部是未来实现与 conformance 必须取得的证据，不是本作者文档已经运行或通过的测试。

- WriteProtection 必须在 planning 开始前由受信人工显式选择并冻结；strict request 不能原地变 observed_only。observed_only 只能用于恰一个 existing live Document 的 ordinary+replica_local 整源保存，且要求完整 source read/replace、author source write set 为空或仅该 Document、无适用 body/Field/node-control deny、不修改 identity/parent/order/lifecycle/shared policy/Registry/Calendar scope/其它 entity、Draft Base=current SourceObservation。noninteractive、managed_atomic、结构、多对象、D5 structured、bulk/collection、D7 strong Action、Automation、server checkpoint、Approval、Money 一律 strict。
- fault injection 必须区分 inputRetentionState=retained、InstallationState、ReliableSaveState 与 PortablePublicationState。prepare/retained 不是 Saved；strict durable install+P seal 才是 reliable，完整合格 observed_only durable install+P seal 才是 durable_observed_only。B 与 N 按原 plan 耐久保留；只有最后检查后仍未观察的 C 可以被 N 覆盖且可能没有恢复副本，later C 可再替换 current file但不得丢 B/N。已观察竞争、stale Base、watcher gap、third_state、撤权或其它资格缺失不能 fallback；unknown install/provenance 固定 recovery_unknown。
- SourceVersion/2 测试必须分别验证 managed 与 external 生产版本、production CommitDomain/production observationEpoch 和 current observerDomain/current observationEpoch。对 H(D,E) 覆盖：完整空史才允许 H=0；fresh managed=1；同生产域 checked H+1；跨 production observationEpoch 不重置；MAX 不 wrap；跨域首次写使用新域自身 H；返回旧域继续旧域 H；equal-byte external admission 仍产生 managed H+1；true raw no-op 保留原版本；source deletion 与 source-unchanged portable structure/lifecycle 不推进 H，但真实 portable effect 仍在 seal 获取 ChangeId。
- SourceRevisionPlan/1 测试必须证明只有会产生 managed after 的原 plan 才冻结 before Observation/absent、lastIssued/完整空史依据、SourceStamp 与 exact after pin；winning plan 后 revision、target、H 依据不可重采样。plan、stage、InstallationNotice/3 与 recovery_unknown 都没有本 decision 新 ChangeId；只有唯一 P seal 把 SourceStamp 与 seal ChangeId 合成 managed SourceVersion/2 并推进 H。seal 后 publication/delivery 失败不得二次分配 ChangeId/revision 或二次收费。
- RevisionTokenBinding/2 与 d6_source_revision/2 是稳定生产地址记录 `{kind,version,token,source}`。每个真实 sealed managed after 都必须验证：原 P seal 把 winning-plan binding 放入 strict RevisionTokenSealAssociation/1，并在同一 seal 生成唯一 RevisionTokenSealArtifact/1。必须验证 exact D3-CJ/3 canonical bytes、`D6-Revision-Token-Seal/1` domain separation、trustKeyId/public-key equality、生产 CommitDomain 在该 seal cut 的历史 portable-trust declaration 下 Ed25519 signature、完整 DecisionKey/ChangeId/SourceVersion/SourceStamp cross-fields、恰一 RevisionTokenSealOutboxItem/1 与 exact portable_metadata pin。同 stamp 的伪造 token 若没有该 signature，只是 unavailable，不是 canonical，也不是 integrity conflict。loser/aborted/seal 不可证明的 token 不能借另一 seal；publication/retry/I rebuild 只复用 exact artifact bytes，不能选 current key 或重签。watcher gap 只使旧 current Observation/runtime evidence 失效，不使 signed stable address 本身失效；新读取资格仍独立。external 仅 bytes 相同不继承旧 event/token。D3 Locator 与 D4/D5 inner selector wire 保持原 owner 形状。
- SourceObservation/1 与 SourceVersionRef/1 必须绑定真实 FileObjectBinding 与 evidence pins；receiver 接纳 ContentCompletionProof/4 后必须以自己的 CommitDomain、当前 FileObjectBinding、observationEpoch、pins 和当前 control/Registry/incidence cut 建新的 Observation/Ref，禁止复用 sender sourceToken。
- Frontier/2 只证明 verified continuous sealed causal prefix。exact 必须与原 expectedFrontier 完整相等；D3 managed_atomic 仍 exact。scope_dependencies 只有在从原 base 到当前 cut 的每个新增 head 都有完整连续 sealed 记录链、无回退/无 hole，并且原 source/control/authorization/全部正负 DependencyKey 仍有效且新增 effect 被证明无关时才可继续。原 canonical request、expectedFrontier、DependencyProof.baseFrontier、targets、Query/selector、pins、proposed bytes、WriteProtection、owner input 与版本依据都不能重签或重采样；provider“已同步”、vector 数字变大、最终 hash/bytes 相同都不够。
- DependencyProof/3 的 stamp 保持 `{epoch,revision}`：epoch 是该完整 DependencyKey 范围的 continuity generation，新完整枚举可从 revision=0 开始；这不表示 unknown/empty/source revision0。可证明连续的同一 epoch 内，任何会影响范围的变化先使旧 proof 不再 current，再 checked 增 revision；gap、owner decoder/rule 改变、真实 proof 目录丢失或 continuity 无法证明必须新 epoch。I-only 删除/重建在受保护 P/M correctness facts 与 event chain 仍完整时只重建 cache，不换 epoch、不重签 proof；这些事实真的丢失时必须当前授权下完整重枚举。
- 十五类 closed DependencyKey/3 都必须有正例、负例、unknown 与授权先行测试，且任何 key 都禁止 free JSON：
  - source：正例绑定完整 current SourceObservation/准确 bytes或value pin/FileObjectBinding；absent 必须由真实 identity/lifecycle/FileBinding+受管 absent 证明。placeholder、I miss、I/O failure、无 source_read 时的对外正文披露都不能伪造成成功。
  - lifecycle：由 D3 完整证明 birth、claim、live、Trash、tombstone、never-known 各状态及其所有者关系；必须先通过状态披露门，再读取这些生命周期事实，派生索引未命中不能证明对象不存在。
  - placement_range：由 D3 对 StructureRange 的九类范围做完整枚举，包括同级子项列表、回收站根项、祖先链、子树、所有者本地资源和注解目录、回复闭包以及恢复成员关系；在读取隐藏的同级子项前，必须先通过结构状态披露门。
  - ref_inbound：由 D3/实际 slot owner 完整枚举 target inbound；零入站必须有完整目录/stamp，不能从当前页或 I 空结果推断。
  - relation_incidence：由 D4 按 RelationReadContext/2 的 fieldId、endpointNodeRef、revisionToken、完整 factSelectors 证明正负 incidence；masked/unprovable 不得 complete，原 D4 wire 不变。
  - calendar_scope：由 D4 语义+D6 配置持久层按 binding、series、period、scope_inbound 四类 CalendarRange，完整 SeriesScope/RegistryBinding、period membership、负范围与 control inbound 证明；index empty/current hits 不决定 unique/many。
  - registry：绑定完整同 Workspace RegistrySnapshot/1、RegistryBinding/1、必要 RegistryEvolutionProof 及 Field/Facet/alias/namespace/contribution 目录；unavailable/unknown definition 与已证明完整空集合分开。
  - temporal_rules：绑定实际使用的 comparator、period rule、timezone/tzdb、RecurrenceReadBinding/1 与有限 horizon coverage；缺 segment、未知 rule provenance 与 unsupported business value 分开，设备时区不能补洞。
  - authorization：principalAudienceToken 来自受信 principal/session/delegation；proof 绑定当前 Policy/3/auth generation、delegation、ObservationScope 与适用 capability，且隐藏 grant/deny 不外泄；任何影响披露/写资格的授权变化必须变 stamp。
  - foreign_binding：D3 绑定语义、D6 目录 continuity、D9/D10/实际来源版本比较器各归原 owner；unknown profile/decoder 使 strong path proof_unavailable，不能用 UID/etag/path/hash/文本或 free wrapper 代替。
  - query_scan：D7 owner 必须证明指定 principalAudienceToken、domain、selector 下完整可见枚举与隐藏策略 generation；headings 还要逐 source 验证准确 D2 内容。partial page、ResultHandle/cursor、CEL 或无命中不得当完整 scan。
  - replica_registry：D6 必须证明 active/retired ReplicaRecord 全目录、registrationSequence 与无 gap continuity；purge 逐 replica 证明指定 Frontier acknowledgement 或 retirement，provider sync/count/单一 Frontier 不代替。
  - conflict_record：选择器为 id 时，证明对应记录及其版本完全一致；选择器为 subject 时，证明覆盖该主体的完整冲突目录和真实已封存头。读取前必须先通过 conflict_read 与 subject 披露门；尚未封存的外部竞争、third_state 或安装结果未知都不得伪造 ChangeId。
  - execution_resource：D6 只读取指定 DecisionKey+protocolOwner 原操作的 resource policy、attempt、累计 work、pins/capacity、pause category；不得枚举所有 OperationId，也不得把 absence/P loss 当退款/额度重置，更不得吸收 D10 Money、provider unknown 或 sourceOccurrenceKey。
- 完整 range 的 empty 与 non-empty 使用同一 snapshot/range barrier 或无漏 continuous change chain+final revalidation；partial/building index、index miss、unknown decoder、placeholder、I/O failure、缺 shard、隐藏未授权对象不能 empty-success。只依赖真实完整局部证据的 ordinary/local 操作应继续按自身资格工作，不因无关 complete-query proof 不可用而永久停止。
- ContentCompletionProof/4 测试必须验证通知、证明与组件键逐项对齐，ChangeId 确实来自封存，frontierBefore/frontierAfter 是实际封存前后的前沿，生产 SourceVersion 的前后版本正确，并覆盖 external→managed 接纳、删除后为 absent、来源未变化的可移植效果不伪造 sourceChanges，以及接收端自行建立 Observation。/3 本身不授予 Query/Action 的完整性资格。ContentCompletionProof/1,/2 继续使用原字节、解码规则和固定证据，/2 中 SourceVersionRef 的 sourceChanges 禁止按 /3 重新解释。
- ConflictRecord/2 测试必须验证 createdAtFrontier=Frontier/2、真实 sealed heads、new-head supersession；ConflictKey/1、ConflictId 与 D6-ConflictKey/1 hash domain不变。ConflictRecord/1 仍按 Frontier/1，不能仅因支持 /2 就重编码历史。
- D3-native OwnerInputBinding/2 与 D3DecisionCompanion/2 必须共享同一 DecisionKey/P seal，不建立第二 receipt/ledger，也不得扩张 D3 WriteScope；current wire13 native consumer 已存在；依赖这些 D6 producer 的新 success 仍须本 D6 repaired candidate 独立复核及后续 owner/global acceptance。
- saved/planned/unseen 顺序必须有专门 fixture：共同 disclosure/P/domain continuity 先于业务状态；saved 重放原 bytes且当前 r6 business proof 不回溯拒绝；planned 无 pre-seal ChangeId并只续原 plan/pins/版本依据；unknown 不从 files/hash/I 猜结果；撤权只遮蔽已 committed 交付，不改历史。
- Windows、Linux、同步盘/placeholder 后端、Server failover 的 conditional/exclusive/create/observed primitive、flush、rename、event-gap 与 crash-window 都仍是未来真实平台故障注入义务；文档、API 名、fixture 清单或 CI 不能代替实测。

current A2 D3 wire13 已具备有界 current-consumer afterimage，真实 fixed-C wire12 继续 historical；D4/D5 保留其各自独立 review 状态。依赖本次 repaired D6 production revision、Proof3/Key3、Notice3/CP4/ChangeRecord1、Record3/Inventory2 与恢复规则的新 strong success，在本 D6 候选 fresh independent review 与后续 joint/global acceptance 完成前继续 gate。D7、D8、D9、D10 的下游门保持 §8 所列完整范围。

### 10.1 Schedule continuity 容量设计场景——UNRUN

| ID | 设计场景 | 必须结果 |
|---|---|---|
| D6-SCHED-CAP-01 | 一次 producer update 恰需要 4,096 个 retained pin，canonical evidence metadata 不超过 16 MiB，且另行 reserved 的 source/component PinBudget 足够 | 其它 authorization/continuity gate 均通过时容量门可通过；不得把 4,096 解释成 transition 数量上限 |
| D6-SCHED-CAP-02 | 1,000 个 retained transition 每个需要 5 个 pin，共 5,000 pin | 即使 1,000 < 4,096 也必须 capacity fail/gap；同一 producer transaction 先原子记录 invalidation/gap，之后才可 reference-safe 释放历史或继续无关 source work |
| D6-SCHED-CAP-03 | retained pin 数不超过 4,096，但 canonical evidence metadata 超过 16 MiB | 走同一原子 gap/capacity 路径；禁止截断、静默 eviction 或 incomplete continuity 的成功结果 |
| D6-SCHED-CAP-04 | evidence 两项计数均满足，但独立 reserved PinBudget 无法保留所需 source/component bytes | evidence count/metadata 成功不授权 update；先 gap/invalidation，source/component history 只可 reference-safe 释放，旧责任/历史仍可恢复 |

这些只是设计 oracle。runtime、crash、provider、replica 与 performance execution 全部 UNRUN。

## 11. Fresh review gate

本次 fixed829 五 finding 修复首先需要一轮**绑定真实 final D6 repair SHA 的新非作者复核**。该窄复核必须把 current D6 Main/Control/Schemas/Impact/Lexicon/Registry/SourceMap 合并阅读，并读取本 package 实际引用的 load-bearing fixed-S、parent/current owner 段和 direct holder，对五项 finding 的正例、负例、unknown、恢复与 historical decoder 路径重新攻击；不能因为本批读取了 direct intersection 就声称 D7–D10 完整模块已全文复核。更晚的完整 A2/global review 仍需完整适用 input/owner corpus、完成后的 D7–D10 模块、Mandatory 925–1141 与真正 fresh Pro/global pass。作者产物、形式检查、design_inputs、docs、tests、diffcheck 或 CI 即使通过，也只证明实际执行的检查，不构成独立语义接受、产品实现或激活。

本作者当前阅读 provenance 只能如实记录每个作者任务真正读取的文件/范围；不能把旧独立审查的 49/49、过去某次双语覆盖、D10 all18 或其它对话的阅读升级成本候选已经完成的 fresh 独立全量阅读。本 Impact artifact 的 reading 只证明本次实际读取的两份原 C Impact 与附件中的最终私人 Storage/Control/Lexicon/Registry producer。

旧 B13 的历史 REVISE 与当时术语/双语 FAIL 仅作历史状态记录；不能因为当前术语/机器检查后续改善就由作者自行宣布旧发现关闭。原 P0=0、P1=3、P2=8 共十一项 OPEN、U6/U7 均继续保持，直到其各自证据由后续 fresh 独立联合审查处置。本批不关闭、不重分类、不接受它们。

两个有界 D3 finding 已独立 CLOSED，本批不重开。D4/D5 audit finding 属于另一固定对象工作流，不计入这五项 D6 finding。对 D6 而言，fixed829 五项 finding 在真实 final repair SHA 接受新的非作者复核之前都只保持 author-resolved-pending-independent。D7–D10 完整 A2 模块、Mandatory 925–1141、A2 完成以及随后真正 fresh Pro/global 终审仍待完成。在这些门完成前，候选仍未接受、未激活、未实现、未合并或发布。

## 12. D3 canonical-resolution 安装资格

新增 Control §3.1.1/§6.2.1 producer 测试：真实 installed Resource A=a，选择 canonical B=b/fresh-copy A，以及镜像方向，都以一个 D3 decision 安装，完整绑定物理 before 与 final preview。Node/Annotation、同 bytes 改 claim/version、metadata-only source 不变和 exact resolved no-op 分别验证，断言唯一 H/ChangeId/current CP4 分支，同时 genuine historical CP3 recovery 继续按版本限定。普通 SourceObservation/current_source/SourceVersionRef、D8/Query/source-save/raw wire12 必须拒 ConflictInstallInput 或移植 pins，不得用 wrapper 填补普通 source 证据。覆盖 malformed/mismatched key/entity/head/version/FileObjectBinding、伪造 audience/use 关联、equal hash 无来源、缺 metadata、external/third-state/unknown install 及完整 source/conflict/range proof，断言读前遮蔽与 unavailable 先于 reachable integrity。guard 剥离或跨 key/token/audience 移植不扩大资格。逐 install/seal 边界崩溃、lost response、撤权、head 改变、wrapper/preview TTL 和 last-reference 压力下，planned/saved/unknown 保留原 /1、/2 分量、唯一 candidate/H 基础和原 receipt，不 reprepare 或重复 canonical admission。以上是设计 oracle，不是已执行 fixture 或产品 conformance；实际最终 D3/D6/D7 producer 字节仍须联合独立审查。

## D6 与 D10 联合生产者实施义务

本批只修改设计文档。下列是必需未来测试，不是已经执行的证据；不增加实现、依赖或公开通用控制入口。真实 D7 符号引导及预览消费者与新的独立联合接受仍是独立门。

| 场景 | 必需证据 |
| --- | --- |
| 完整不可变控制输入 | 保留内嵌原 A，拒绝在 ownerInput 放入生成的 M 或未来确认；崩溃后完整历史图像和准确负向范围仍可恢复。 |
| 原两个 CAS 点 | planning 恰预留最后一个名额，seal 包括逐字无操作在内只消费一次；r6 之后仍重放原 r5，不重新准入或收费。 |
| 安装与封存之间停止 | 真实物理安装后停止胜出，保留原屏障、B/N 和来源；第三状态暂停，只有安全权威中止释放计次，费用不自动退回。 |
| 外部确认 | 受信事件只改变独立确认记录，两个 CAS 都重验；直接原 D6 请求不能绕过，保存 consent 先重放。 |
| 引导版本 | fresh current Profile4/Plan4 在原 D3 决议产生准确 Policy3、含两条 Declaration2 profile 的 WorkspaceTrustGenesis2、mandatory initialPresentationPolicy 与完整 Registry/Calendar 初始化；真实 historical Plan1/Plan3 替换或重放保持 recorded family，不获得 current 字段或授权。 |
| 真实 Registry 激活 | 可移植 Registry 改变获得唯一严格 ChangeId + Notice3/CP4/ChangeRecord1 与原子目录选择器；Registry 不变的纯 P 变化没有内容版本，源与 H 均不增加。 |
| 调度连续性 | 无关正文或 Entry 变化可推进；删除重建或规则复原永久改变绑定；观察或收件箱缺口暂停；容量不足先原子标缺口再允许普通源继续。 |
| 执行责任 | 验证完整及空范围，保留共享账户债务、剩余次数为零的原准入、全部发生项和发送未知及停止容量；新执行前隔离旧持有者。 |

## 13. A2 current successor 实施义务

除非另有精确外部运行记录，本文全部案例仍只是未执行的设计义务。当前实现还必须：
- 实现第 3 版依赖键、依赖证明、输入描述和准备记录，以及 `document_format` 的失效规则；不得扩大历史第 2 版依赖键；
- 以一条精确的当前可移植链产生第 3 版安装通知、第 4 版完成证明和第 1 版变更记录；仅格式变化时 `sourceChanges=[]`，不得虚构 H；
- 保持来源修订计划的角色分派：/1 用于普通或新鲜写入，/2 仅用于 D3 规范冲突，/3 仅用于 D6 来源冲突；
- 实现来源变换第 3 版事件的精确回放几何、完整前像哈希、已签名规范制品与发件箱、历史密钥验证，以及恢复时禁止重新签名；
- 实现第 2 版声明、授权包、密钥句柄和持有证明的双配置档；策略冲突必须递归折叠混合的第 1/2 版携带事实，并保留精确冲突头证据、固定证据目录和第 2 版预览；来源分支使用第 2 版输入，策略分支使用第 3 版输入；
- 专用副本注册必须产生一个 ReplicaEpoch、两个暂存的第 2 版密钥句柄和两条第 2 版声明，配置档顺序固定，并共享唯一的安装通知、完成证明、变更记录和 P 提交；
- 实现当前 D10 的混合版本持有者路由、执行责任记录与连续性证明的清单逐字节一致性，以及调度 /1 与 /2 制品域的严格分派；
- 完整保留结果、游标、字节句柄与字节读取合同，以及导入任务、来源绑定、同步上限、服务器草稿/准备/栅栏、预算/尝试/时钟语义和全部既有正向、负向、未知与恢复案例。

这些条目都只是设计义务，不表示已经运行运行时、密码学、提供方、崩溃恢复、性能、操作系统、图形界面或真实副本测试。

## 14. Review boundary

D6 作者不独立关闭本候选。D7-D10 完整模块仍待后续。文档与机器 source 检查只验证其具名机械性质。
