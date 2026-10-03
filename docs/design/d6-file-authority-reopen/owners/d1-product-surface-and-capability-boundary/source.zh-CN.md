---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：a96b68da-72b7-434b-826e-e14d07c96077。

# D1 产品界面与能力边界

候选状态：D6-FA-r01；部分联合候选；未接受、未激活、未实现。固定 S 的 2026-08-27 冻结决议继续作为历史来源；本后像只在完整替代 owner、D10 消费者、fresh 独立联合审查和协调接受完成后才可激活。本文不授权代码、依赖、发布、部署或 A2。

## 1. 范围、输入和版本边界

D1 仍只拥有正式产品界面、运行模式、能力归属、进程/信任/网络边界、状态所有权和发布顺序。D2–D10 分别拥有内容、身份、类型、结构、存储、Query/Action、编辑器、转换和外部能力的内部协议。

D6-FA-r01 对 D1 的材料变化只有五项：

- 普通文件成为本地工作区当前内容字节的权威载体，外部程序与文件同步服务可以修改或运输这些字节。
- 同一逻辑 Workspace 可以有多个已登记物理副本，每个副本可在离线时独立完成普通编辑、创建、移动、重排和 Trash。
- 单提交持有者规则改为“每个物理副本或每个托管后端恰有一个持久提交持有者”，而不是“整个 Workspace 在全球只有一个可编辑设备”。
- Server 保持多客户端、多主体和并发编辑会话；持久提交排序不等于只允许一个人编辑。
- Automation、批准次数、Money、external unknown 与 stop 的连续执行责任独立于普通内容副本，文件复制不复制其消费资格。

本后像不改变五个正式产品界面的身份，不提前交付实时协作，也不改变当前发行平台声明。

## 2. 自包含问题定义

Weftext 同时满足：

1. 没有账户、Server 或网络时，用户可把普通文件工作区作为本地产品使用。
2. 同一个逻辑 Workspace 可复制到多个设备，普通内容操作可以离线发生；同步后必须显式处理并发版本、结构、生命周期和策略冲突。
3. 托管部署通过 Server 向 Browser、Desktop、CLI、Mobile 提供同一 Core 语义，并保留多人同时工作。
4. worker、Agent、自动化、连接器、同步服务和索引都不能获得第二条作者写入语义。
5. 全局执行资源只由连续、可证明且被 fence 的执行责任域消费；普通内容可用性不依赖该责任域持续在线。
6. 大库可先打开、先编辑并先完成活动内容的文件保存，再渐进完成 metadata、全文搜索和 OCR；其中 strict 的 reliable 与 observed_only 的 durable_observed_only 必须分开呈现和测量。

最危险的失败包括：同一物理副本两个进程都认为自己能提交；文件同步被宣传为实时协作；一个离线副本复制全局费用资格；部分索引被当成全集；Server 的数据库单写者被误解为单用户产品；客户端 Draft、输入留存或同步上传被误报成文件保存；以及 observed_only 被展示成 strict reliable 或强 Action 资格。

## 3. 总体不变量

| ID | 规则 |
|---|---|
| D1-I01 | 任一物理本地副本在任一时刻只有一个持久提交持有者；同一托管后端跨全部 Server 进程、实例和重启重叠窗口也只有一个持久提交持有者。不同已登记物理副本可以各自离线提交普通内容，不因此成为同一个全局执行 authority。 |
| D1-I02 | Desktop、WebUI、Server、CLI、Mobile 共享 Core 的领域结果、诊断、计划和提交语义；界面只能选择交互，不能自造身份、冲突或提交结果。 |
| D1-I03 | WebUI 永远是 Server 客户端，不打开本地目录，不持有托管路径或数据库。 |
| D1-I04 | Desktop、CLI、Mobile 在本地模式调用本机 Core；远端模式只调用 Server，不以本机缓存重解释 Server 结果。 |
| D1-I05 | 普通同步服务只复制文件和可移植元数据。检测到外部变化后，依赖旧 SourceObservation、观察世代或受影响安装范围的资格失效；Core 保留 Draft/分支并重新取得局部资格。已观察竞争绝不能由 observed_only 覆盖，但一个局部变化也不得把整个副本永久变为只读。 |
| D1-I06 | worker、Agent、自动化和连接器只能产生输入、结果或提议，最终作者提交仍由本机 Core 或 Server Core。 |
| D1-I07 | 能力状态来自版本化能力描述，不从按钮、OS 检测、缓存或一次成功推测。 |
| D1-I08 | 不支持、未交付、未配置、离线、版本不兼容、无权或策略拒绝都必须返回明确不可用结果，不用近似操作补位。 |
| D1-I09 | 平台支持声明必须绑定实际构建、安装、运行和场景证据。 |
| D1-I10 | 当前公开工件没有生产发行身份的平台签名、公证或商店分发；开发签名不能被包装成发布能力。 |
| D1-I11 | 产品必须区分 input retained、not_saved、reliable、durable_observed_only、recovery_unknown 与 portable publication pending/published。只有受信交互的单一既有 live Document 普通 source-save 才可能使用 observed_only；Draft 自动留存不授予无人值守弱安装资格。 |

D1-I01 约束“谁能把一个持久结果写入某个物理后端”，不约束“多少用户可以同时持有 Draft、阅读、准备、评论或编辑输入”。

## 4. 产品界面职责

### 4.1 Desktop

Desktop 是完整本地文件工作区界面，也是 Server 客户端。它负责用户文件授权、窗口、设备 Draft、选择和 UI 状态，调用本机 Core 或远端 Server。

本地模式可以打开一个已登记副本并离线编辑；同步到达的变化必须交给 Core/D6 协调。Desktop WebView 不直接改工作区文件、可移植元数据、P 或 I。

### 4.2 WebUI

WebUI 只通过 Server 工作。浏览器缓存、Draft、选区、光标和 presence 都不是工作区权威。网络断开时可以保存明确标记的本地 Draft，但不能声称已保存到托管工作区。

### 4.3 Server

Server 是托管工作区的唯一在线持久写入边界。它负责认证会话、授权、审计、备份恢复、并发协调、WebUI 和版本化客户端 API，并在托管模式调用同一 Core。

同一托管后端的提交资格必须跨全部 Server 进程与实例唯一；未取得资格的实例拒绝写或保持只读。这个限制只作用持久提交路径。多个用户可以同时打开同一或不同文档，各自持有 Draft、读取、准备和交互状态。

不同文档的读取、解析和准备可以并行；持久提交只在必要目标与依赖范围内排序。同一文档的两个普通非实时编辑会话可以并存：先保存的一方发布新版本，另一方的旧 Base Draft 保留并进入 stale/conflict 处理，不被覆盖。

实时共同文本编辑仍属于 G2 之后的后续能力。届时 Server 协调会话输入、广播和检查点，但持久结果仍经 Core；OT/CRDT 算法本轮不冻结，协作日志也不成为第二作者源。

### 4.4 CLI

CLI 支持本地和远端模式。它可以在本地文件副本上执行同一 Core 普通操作，也可以调用 Server。需要确认的破坏性、外部或冲突解决操作必须显式输入，不能隐藏交互式批准。

### 4.5 Mobile

Mobile 的领域语义与其它界面一致。其本地后端只有在能证明所需文件安装和耐久条件时才提供相应写能力；否则明确不可用。Mobile 远端仍是 Server 客户端。现有转换、Agent、自动化和连接器限制不因文件权威重开而扩大。

## 5. 平台与能力矩阵

| 能力 | Desktop | WebUI | Server | CLI | Mobile |
|---|---|---|---|---|---|
| 本地文件工作区 Core 读/写 | L | — | — | L | L |
| 托管工作区 Core 读/写 | R | R | H | R | R |
| 普通编辑、Query、View、Action | L/R | R | H | L/R | L/R |
| 多副本文件同步后的冲突呈现 | L | — | — | L | L |
| Server 多用户会话 | R | R | H | R | R |
| 实时协作、presence、光标 | R（G2 后续） | R（G2 后续） | H（G2 后续） | — | R（G2 后续） |
| 本地可重建索引 | L | 浏览器非权威缓存 | H 的派生缓存 | L | L |
| 本地备份/恢复 | L | — | — | L | L |
| 托管备份/恢复 | — | I | H | I | — |
| 自动化/Agent/连接器 | 原 D1 边界 | 原 D1 边界 | 原 D1 边界 | 原 D1 边界 | 原 D1 边界 |
| 绕过 Core 直写 | — | — | — | — | — |

L/R/H/I 的含义保持固定 S：本机承载、远端客户端、Server 承载、仅发起/管理。

## 6. 运行模式与状态所有权

| 模式 | 持久提交持有者 | 非权威状态 | 必须拒绝的解释 |
|---|---|---|---|
| Desktop/CLI/Mobile 本地副本 | 该物理副本唯一 Core + 合格文件后端 | Draft、I、UI、同步进度 | 同一副本两个进程都可靠保存 |
| 完全离线本地 | 同上 | 网络能力不可用 | 因无执行托管者而禁止无关普通编辑 |
| 多设备文件复制 | 每个已登记物理副本各自一个本地提交持有者 | 同步服务仅运输 F/M | 文件复制产生一个跨设备原子事务或复制 Money 资格 |
| Server 托管 | 取得托管后端资格的 Server Core | 客户端 Draft/缓存，Server 会话/presence | 单个后端提交序列等于一个用户会话 |
| Server 实时协作 | Server Core 仍是持久权威 | 协作输入流和临时检查点 | OT/CRDT/operation log 自动成为作者源 |
| 远端断网 | Server 仍为托管权威 | 客户端非权威 Draft | 把 Draft 叫已保存、已同步 |
| worker/Agent/automation | 发起侧 Core/Server | 任务、凭据引用、transcript | 外部结果直接写文件或 P |

本地文件副本不能同时被同机 Desktop/CLI 两个 Core 当作可提交后端，也不能同时作为 Server 托管目录被本地客户端直写。

## 7. 部署与进程拓扑

### 7.1 本地副本

用户界面或 CLI → 本机 host → Core → 文件后端 F/M；同一 host 另有库外 P 和可删 I。同步程序只看允许同步的普通文件与可移植元数据，不接触活动 P/I/WAL/SHM。

### 7.2 多副本同步

设备 A 与设备 B 各自拥有独立 replicaEpoch 和本机提交域。同步服务运输文件和完整 portable records。接收侧验证内容完成证明、实际组件和冲突后再接纳；它不重放另一设备 OperationId，也不导入另一设备的 execution responsibility。

### 7.3 托管 Server

Browser/Desktop/CLI/Mobile → authenticated Server API → authorization/audit → Core → 托管文件后端 + Server P/I。多个前端连接和多个 Draft 可以并存；只有完成 Core seal 的受管结果进入唯一持久提交序列，Draft 或广播确认都不是保存成功。

### 7.4 故障切换

Server failover 必须同时 fence 控制库和作者文件写能力。只阻止旧实例写 SQLite、却仍允许它 rename/replace 托管文件，不满足唯一提交持有者。control custody 连续也不表示 revision-seal private key 连续：fresh AuthorityInstanceId 只能通过 D6 已锚定 WorkspaceAuthorizationBundle transition 与新 host 上合法受保护 key handle 获得 fresh exact-domain signing key；复制 Workspace/control files 永远不提供该 handle。

## 8. 信任、权限与网络边界

权限在持有工作区后端的一侧重验。客户端提供的“已授权”、同步服务的“已上传”和本地索引的“已完成”都不是权限证据。D6 WorkspaceTrustAnchor/root/domain-seal trust 与 D10 publisher/package signing、D4 Registry seed authenticity 相互独立；后两者都不能替代 Workspace author/seal authority。

普通本地操作只读取其实际需要的 Source、identity/lifecycle、结构和 policy 范围。强 Action 需要的完整正/负范围仍按 D4–D7 证明；索引未完成时可以扫描源，无法证明时只拒绝该强操作。不得为了证明一个普通段落保存而先读取无关隐藏文档。

撤权与提交以拥有 authority 的边界线性化。Server 协作中，每个参与者在持久检查点前重新验证自己的未提交贡献；A 的权限不能替 B 的输入提供提交权。

## 9. 能力归属

Core 继续拥有领域解释、计划、验证和提交。D6 拥有 F/M/P/I 的物理与控制合同、WriteProtection、ReliableSaveState、input retention、便携发布、冲突记录和执行责任连续性。D3 拥有 identity、parent/order、lifecycle。Server 控制面拥有认证、账户、会话、审计和协作协调。任何数据库或同步程序都不是领域 owner。

## 10. 能力探测与不可用语义

原 D1 capability negotiation 和不可用 reason 顺序保持。D6 的 workspace_busy、domain_unavailable、install_unavailable、conflict、owner_update_required 等属于具体工作区/操作结果，不扩张 D1 capability reason。

能力探测不是授权票据；实际提交仍重验当前 policy、后端资格、SourceObservation、Frontier/2 和所需依赖。strict 请求不能在提交途中原地降级为 observed_only。

## 11. 协调版本与发布声明

D6-FA-r01 是设计 generation，不自动改变当前 release support matrix。新 D1/D3/D6/D7/D8/D9/D10 afterimage 只有完整联合接受后才能进入一个产品 contract major；本批不以“contractMajor 2”绕过具体 wire/version 消费。

G1/G1.1/G2 的既有平台证据门保持。没有实际多人协作、性能和故障注入证据时，发布说明必须明确其未交付状态。

## 12. 发布依赖顺序和实时协作排期

既有顺序保持：

共享 Core → G1 本地首发 → G1.1 CLI 与 Server 安全基础 → WebUI → G2 托管发布 → 实时协作 → Mobile 远端能力。

实时协作不得阻塞安全的非实时 Server/WebUI 发布；它也不能被文件同步模式冒充。D6-FA-r01 只冻结未来协作必须接入的提交、权限、Draft 和冲突边界，不实现 OT/CRDT。

## 13. 明确非目标

- 不把同步目录称为实时协作。
- 不要求所有副本同时在线才能普通编辑。
- 不让未登记或已退役副本自动取得执行责任。
- 不冻结自动 LWW、mtime winner 或任意 CRDT/OT 算法。
- 不把每次按键变成作者提交。
- 不把浏览器变成本地文件 PWA。
- 不扩大当前平台、签名或商店承诺。
- 不把大库完整索引或 OCR 作为打开和普通文件保存的统一前置条件；弱保护路径的耗时也不能冒充 strict 可靠保存指标。

## 14. 替代方案与拒绝理由

| 方案 | 裁决 | 原因 |
|---|---|---|
| 整个 Workspace 全球只有一个可写设备 | 拒绝 | 破坏已确认的多设备离线普通内容能力 |
| 一个物理副本允许多个进程同时写 | 拒绝 | 无法形成确定安装、版本和恢复边界 |
| 文件同步就是协作层 | 拒绝 | 没有主体、权限、提交顺序、presence 或确定冲突协议 |
| Server 单写事务意味着单用户 | 拒绝 | 持久排序与前台并发会话是不同层 |
| 浏览器离线 Draft 直接成为 Server commit | 拒绝 | 绕过 Server 当前权限、版本和冲突门 |
| 复制 portable metadata 就复制全局额度 | 拒绝 | 内容副本与 execution responsibility 分域 |
| 全库索引完成前全部只读 | 拒绝 | 无关范围不应阻断普通局部操作 |
| 静默 last-writer-wins | 拒绝 | 丢失并发原字节、身份或用户意图 |

## 15. 端到端场景与最小反例

### S1 本地离线编辑

断网后 Desktop 打开活动文档并编辑。若后端具备 strict 安装资格，可显示“已可靠保存”；若只具备 observed_only 且同时满足受信交互、单一既有 live Document、完整读取和整文件替换权限、零或单 source 写集及无细项 deny，可显示“已保存 · 普通文件模式”，并稳定说明不能排除其它程序同时写入。已观察冲突、未知安装结果或不满足资格时只保留输入并显示冲突/待恢复/不可用；Draft 自动留存不得触发无人值守 observed_only 安装。

### S2 同一机器双进程

Desktop 与 CLI 争用同一物理副本。只有一个能取得提交持有者；另一方失败或只读。两个进程都报告可靠保存是反例。

### S3 两设备文件同步

A、B 离线修改同一笔记后同步。两边已经 seal 的普通保存结果都保留各自真实 WriteProtection，接收端形成显式冲突，不按 mtime 选 winner，也不把 observed_only 升级为 reliable。同步先到正文后到 metadata 时状态为 incomplete，不把缺 metadata 当新身份或删除。

### S4 WebUI 断网草稿

浏览器断网后保留明确未提交的 Draft。重连后以 Server 当前权限和版本重新进入协调；localStorage/IndexedDB 不是托管提交。

### S5 远端权限变化

两个用户正在同一托管文档编辑。A 已形成提交、B 尚未提交时撤销 B 权限；B 的 Draft 保留为本地数据，但不能借 A 的会话或旧 capability 提交。已提交事实不因撤权被改写。

### S6 不同文档并发

A 编辑 N1，B 编辑 N2。读取和准备可并行，持久 commit 只按真实写集与依赖排序。N1 提交不能仅因全局序号变化让无关 N2 Draft 消失。

### S7 同文档非实时并发

A、B 都从 revision r5 编辑。A 先保存为 r6；B 的旧 Base 已属于已观察冲突，必须得到 stale/conflict，保留 B Draft 和 r5/r6 差异。observed_only 也不能覆盖 A。

### S8 后续实时协作

G2 之后多个参与者可收到临时协作输入。广播收到不等于可靠保存；检查点经 Core 验证后才成为规范状态。未实现该能力时 capability 明确 not_in_release。

### S9 全局执行责任

A、B 都有文件副本，但只有连续执行责任 holder 可消费同一 ApprovalUse/Money。B 仍可普通编辑；它不能因为复制文件获得第二份余额。

### S10 大库首次打开

数十 GB 库可以先完成 T_first_open、T_first_edit；strict 路径达到 T_first_reliable_save 后仍可继续 T_full_search_ready 与 T_OCR_ready。observed_only 到 durable_observed_only 的延迟必须单独记录，不能填入 T_first_reliable_save。未建立性能证据前不承诺具体秒数。

## 16. 对 D2–D10 的输入约束

| Owner | D1-FA 输入 |
|---|---|
| D2 | exact source 与对象语义跨副本/Server一致；外部 invalid bytes 不因路径而升级为合法 Document |
| D3 | identity 与 parent/order 不依赖路径；副本登记与 Workspace continue 分开；replica_local 只缩小语义证明范围，D3 identity/structure/lifecycle 安装仍必须 strict |
| D4 | semantic_pending 的 typed/cross-object消费必须显式；不能把 pending 当全部约束已通过 |
| D5 | native/collection/bulk 的完整范围门不能由部分索引代替 |
| D6 | F/M/P/I/Draft 分域、WriteProtection、ReliableSaveState、input retention、便携发布、冲突、replicaEpoch 和 execution responsibility |
| D7 | Query/Action 绑定 CommitDomain/frontier；complete 结果只能由完整范围证明产生 |
| D8 | Source/Live/Read、Draft/IME/Undo/selection连续；多会话与协作检查点不成为第二作者源 |
| D9 | pins/import/export/publication服从新域与版本，不用 worker 成功冒充作者 commit |
| D10 | approval/claim/Money/sourceOccurrenceKey/stop 与普通内容副本分域并保持连续 responsibility |

## 17. 实现影响图

未来实现至少需要：本地文件后端资格与占用、portable replica registry、Server 文件写 fence、按范围失效的外部变化观察、冲突呈现、D8 多会话 Draft 绑定、Server 广播 outbox，以及大库分层索引状态。

这些都是未来实施影响，不授权本批修改产品源码。

## 18. 验收与性能轮廓

必须验证：

1. 同一物理副本双进程只有一个持久提交 holder。
2. 两个登记副本可各自离线普通编辑，不复制 execution responsibility。
3. 外部变化只使相关 SourceVersion/cut/install 范围失效，无关笔记仍可重新取得局部资格。
4. Server 两用户可同时持有同文档或不同文档 Draft，最终提交仍唯一排序。
5. 撤权、重连、重启和 failover 不产生第二提交 holder。
6. 文件同步部分到达、placeholder 和冲突不伪造完整状态。
7. T_first_open、T_first_edit、T_first_reliable_save、
  T_full_search_ready、T_OCR_ready 分别测量；durable_observed_only 的保存延迟另列，不计入 T_first_reliable_save。
8. 基准记录文件数、正文量、附件量、冷暖缓存、CPU、RAM、磁盘、峰值内存、I/P 大小与重启续建。
9. no-body-replica 检查确认 P/I 没有全库当前正文或完整 AST 副本。
10. 实时协作未实现时所有界面都明确报告未交付，不以文件同步代替。

## 19. 候选状态与后续门

本后像是 D6-FA-r01 的 D1 owner 候选，不替代独立接受。D1 的历史 frozen 决议只保留为来源；新语义必须与 D3/D6 以及后续 D4/D5/D7/D8/D9/D10 afterimage 共同接受。

旧 D10 B13 仍为 REVISE，3 个 P1 与 8 个 P2 共 11 项 OPEN；本文件不关闭任何 finding。
