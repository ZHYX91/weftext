---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：042833a6-ecb2-4909-8712-ce4ec464ff34。

# D1 Implementation Impact and Test Outline

候选状态：D6-FA-r01；部分联合候选；未接受、未激活、未实现。固定 S 的 D1 Implementation Impact 仅作历史来源。本文件定义未来实现/验收义务，不授权实现、依赖变更、发布或 A2。

## 1. 影响范围

D1-FA 只改变运行模式与状态所有权，不创造新的领域对象。需要实施的主要切片：

| 切片 | 未来职责 | 不得推导 |
|---|---|---|
| 本地文件副本 | 占用、后端资格、外部变化失效、replicaEpoch 接入 | 一个逻辑 Workspace 只能有一台设备 |
| 多副本同步 | transport F/M、完整记录接纳、冲突呈现 | 同步服务是事务或 execution authority |
| Server | 多会话、范围并发、唯一持久提交 holder、文件与 P 双重 fence | SQLite 单写者等于单用户 |
| 大库启动 | 活动文档优先、渐进 I、分层 parser/search/OCR | 全库索引完成前不可编辑 |
| 实时协作 | G2 后会话输入、广播、checkpoint 接口 | 本批已交付 OT/CRDT |

## 2. 产品状态机与界面投影

各界面必须区分以下用户可观察里程碑：

- T_first_open：可进入库、浏览已发现结构并打开已物化目标。
- T_first_edit：活动 Draft 可以输入；它不等于可靠保存。
- T_first_reliable_save：活动目标通过文件安装、D6 决议 seal 后得到可靠保存。
- T_full_search_ready：声明的完整搜索范围可由完整扫描或合格候选索引+回读证明。
- T_OCR_ready：选定附件/模型/配置的 OCR 工作完成，失败项明确。

同步上传、Preview、Prepared、worker success、HTTP success、
  Draft persistence、P planned 都不能投影成 T_first_reliable_save。

## 3. 本地实现义务

Desktop/CLI/Mobile 本地 host 必须：

1. 对同一物理副本提供进程级排他提交资格。
2. 把活动 P/I 放在同步目录之外。
3. 把外部文件变化转为 D6 当前观察，不由 UI 自行合并。
4. 在目标后端缺少安全 conditional/exclusive 安装原语时保留 Draft/after，并明确 install_unavailable。
5. 只使受影响 SourceVersion、cut、prepare 和安装范围失效；重新验证后允许无关普通操作继续。
6. 删除 I 后能够从 F/M 渐进重建，不重新 mint identity、不补 decision。
7. P 丢失时保留可证明的普通内容，暂停无法恢复的 execution responsibility，不从文件猜旧 receipt。

## 4. Server 多用户实现义务

Server 后端要同时支持多会话与单持久提交边界：

- 每个 authenticated principal 有独立授权上下文、Draft/selection/session。
- 同一文档可有多个 Draft；不同文档的读取与准备可并行。
- 持久 checkpoint/commit 只在实际 write/dependency scope 上排序。
- 旧 Base 提交绝不能覆盖已发布新版本；冲突保留所有未提交输入。
- 撤权在 checkpoint 前重新验证，不能借另一参与者身份提交。
- 广播只从已形成的受管结果/outbox 产生，接收者仍过读取门。
- failover 同时 fence P 与工作区文件写路径。
- 后端串行 SQL 事务不是一个前台用户锁。

## 5. 多副本同步实现义务

同步器只运输普通文件和可移植元数据记录。接收方必须能区分：

- 完整 ChangeId + ContentCompletionProof 已到达；
- InstallationNotice 或部分 components 到达但完成证明缺失；
- placeholder/not_materialized；
- 同一 subject 的并发 source/placement/lifecycle heads；
- identity collision；
- retired replica 带回旧内容。

任何一项都不能由 mtime、path、文件名、相同 digest 或“最后上传者”自动裁决。

## 6. 实时协作后续接口

实时协作仍为 G2 后续能力。实施前至少必须提供：

- session epoch 与参与者身份；
- 每参与者输入序号和准确 Base；
- IME preedit 与 final commit 的分离；
- transient broadcast 与 durable checkpoint 的分离；
- 撤权、断线、重连和 gap reset；
- 无法证明 rebase 时保留输入并停止 checkpoint；
- 协作算法输出到 Core 的唯一受管 source proposal。

OT/CRDT 的选择、编码与优化不由 D1-FA 冻结。

## 7. 大库与索引实施轮廓

首开不做全库完整 hash，也不把全库正文或 AST 复制进 I。实施使用：

- 流式目录/portable metadata 枚举；
- 活动文档优先队列；
- 有界在途字节、解析对象、worker 和 I 批事务；
- 可续建 coverage/checkpoint；
- metadata、parser、候选搜索、附件提取、OCR 的独立版本与失效域；
- 需要全集的 Action 在完整 coverage 缺失时扫描源或明确拒绝。

默认 token/FTS 命中只可作为候选；exact/NFC/regex 最终语义由对应 owner 和 source 回读决定。

## 8. 性能验证矩阵

未来基准至少覆盖：

| 轴 | 最小集合 |
|---|---|
| 文件数 | 1 万、10 万、100 万小文件 |
| 数据构成 | 附件为主几十 GB、正文为主几十 GB |
| 下载状态 | 全本地、同步器按需下载 |
| 缓存 | 冷缓存、暖缓存 |
| 硬件 | 记录 CPU、RAM、OS、文件系统、磁盘 |
| 生命周期 | 初次打开、删除 I、已有库新设备、建设中重启、单文件/批量修改 |
| 输出 | 五个时间里程碑、峰值 RAM、I/P/pin 大小、读字节数、增量时间 |

这些是验收设计，不是现有性能结果；不得写“几秒完成”或具名竞品速度等未经实测结论。

## 9. 安全与负向测试

必须覆盖：

1. 同一本地副本 Desktop/CLI 同时争用，至多一个可靠保存。
2. 两已登记副本都离线保存普通内容，之后产生显式冲突而非全库锁死。
3. 一端移动、另一端编辑；一端 Trash、另一端编辑；正文/metadata 分批到达。
4. 复制 portable metadata 后 global approval/Money 仍不可重复消费。
5. 删除 I 后 identity/parent/order/policy/receipt 不丢。
6. 丢 P 后不可从当前源恢复 external unknown/原 receipt/费用；普通无疑点内容仍可打开。
7. Server 两人同文档、不同文档并发，持久结果有唯一顺序。
8. Server 撤权与 commit 竞争只有一个规范结果。
9. failover 只 fence DB、不 fence 文件时验收失败。
10. partial index 对 complete Action 不能产生“全集已证明”。
11. no-body-replica 检查 P/I/schema/FTS shadow state，拒绝全库当前正文和完整 AST 镜像。
12. G2 前任何“实时协作已可用”声明都失败。

## 10. 跨 owner 后续义务

- D3：wire12、CommitDomain scoped ledger、本地 create/move/reorder/Trash、强 purge、legacy replay。
- D4：semantic_pending 的 relation/unique/calendar/cross-object 类型消费。
- D5：native/bulk/collection 的局部与完整范围。
- D7：Query/Action scoped cut、Prepared 新版本、effects/pin retention。
- D8：Source/Live/Read、多人 Draft、IME/Undo/selection 和协作 checkpoint。
- D9：ImportJob/export/publish 的 scoped pins 与新版 request/effects。
- D10：approval/claim/Money/sourceOccurrenceKey/stop 的 execution responsibility。

在这些 afterimage 未共同完成前，不允许半包激活新版 managed success。

## 11. 接受边界

本文件只定义未来实施和验证。文档 CI、作者自查、代码能编译或旧测试成功都不构成独立接受。

旧 D10 B13 保持 REVISE、术语/双语 FAIL、3 P1 + 8 P2 共 11 OPEN。
