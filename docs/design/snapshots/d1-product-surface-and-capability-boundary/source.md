---
_weftext:
  id: "a96b68da-72b7-434b-826e-e14d07c96077"
---

# D1 产品表面与能力边界

状态：**frozen**。D1 的 OpenCode 独立评审、最小反例裁决、定向复审和 Sol Pro 最终门禁均已完成；不存在未解决的 P0/P1。本决议是 D2–D10 的冻结上游输入，但尚未表示 `repos/weftext` 已实现。

日期：2026-08-27。

## 1. 范围、输入和判定方式

本决议只回答 D1：Weftext 的正式产品表面、运行模式、能力归属、进程/信任/网络边界、状态所有权、能力探测、无生产发行签名的 GitHub 发布声明和依赖顺序。它不定义 D2–D6 的领域对象、身份、持久化布局、权限代数、事务日志、同步算法或冲突数据模型，也不定义 D9/D10 的 worker、Agent、自动化或连接器内部协议。

冻结输入：

- Core 是工作区语义和写入的唯一权威；所有写入都经过 Core 验证的计划或事务。
- Desktop、WebUI、Server、CLI、Mobile 都是正式产品表面，不能各自发明工作区语义。
- Mobile 必须支持，但首期只承担设备本地可完成能力；不做转换，不提供 AI Agent。
- 当前唯一公开发行渠道是 GitHub Releases；不制作或发布带生产发行身份的平台签名、公证或应用商店工件，也不把这些事项放入当前完成条件。仅为本地开发、CI、模拟器或登记测试设备 conformance 所必需的临时 debug/development signing 与 provisioning 可以使用，但不构成发行能力、Supported 声明或未来签名承诺。
- 不保留任何未发布原型兼容；保留现有 Logo，不修改 `brand/`。

本文中的规则按两类标注：

- **M（mechanical）**：必须能由构建图、能力清单、协议测试、依赖扫描、负向测试或发布证据机械验证。
- **P（product decision）**：产品选择本身；实现和发布必须忠实呈现，不能由 UI 或调用方猜测改写。

除明确写为“未来由 Dn 定义”外，“必须”“不得”“仅”均为规范性要求。

## 2. 自包含问题定义

Weftext 要同时满足四个互相拉扯的用户目标：

1. 用户可在没有账户、没有 Server、没有网络时，把一个可移植工作区当作本地产品使用；
2. 同一产品也可由 Server 托管，通过浏览器、Desktop、CLI 和 Mobile 安全访问，并最终支持多用户协作；
3. 格式转换、AI Agent、自动化和连接器可扩展，但不能获得第二条工作区写路径，不能把外部 Provider 状态变成工作区权威；
4. 在只通过 GitHub、没有生产发行签名、公证或商店分发的阶段，发布声明必须窄、可复现、对安装阻力和缺失能力诚实；开发测试所需的临时签名不能被表述为发行能力。

最危险的失败不是“某个按钮暂时没有”，而是：同一工作区出现两个写入权威；浏览器或 UI shell 自行解释文件；共享文件夹被误称为协作；远端缓存或 Agent transcript 被当作内容；worker/Provider 获得可写工作区；没有实际包证据却泛称支持 Windows/macOS/Linux/Mobile；正式表面被永久降格为另一表面的薄皮。

设计目标因此是：**一个语义核心、两类权威承载位置（本机或 Server）、五个正式表面、显式能力协商、默认拒绝的跨边界调用，以及按证据逐行发布的支持矩阵。**

规模、威胁和故障假设：

- 工作区可大于单设备内存，文件和外部同步可部分到达；具体限制由 D5/D6 冻结。
- UI、浏览器、Agent、外部 Provider、输入文件和网络均不可信；本机用户授予的工作区路径也可能包含恶意或异常内容。
- 进程可崩溃、设备可离线、Server 可重启、权限可变化、worker 可超时；D1 只冻结谁能拥有和改变状态，不冻结恢复算法。
- 无签名包会触发操作系统警告；这不是可隐藏的安装细节。

## 3. 总体不变量

| ID | 规则 | 类型 | 机械检查 |
| --- | --- | --- | --- |
| D1-I01 | 任一活跃工作区副本在任一时刻只有一个提交持有者：本机 Core 或 Server 内 Core。同一物理本地副本不得同时由两个进程取得提交资格；对同一托管工作区后端，提交资格必须跨全部 Server 进程、实例和重启重叠窗口唯一授予，第二实例必须拒绝写入或保持只读；客户端也不得与 Server 同时写同一托管存储。 | P+M | 本机打开/占用竞争；双 Server 同后端启动、重启重叠、故障切换与托管目录写入负向测试 |
| D1-I02 | 所有正式表面共享 Core 产生的领域结果、诊断、计划、提交和能力标识；表面只能选择交互，不得重写语义。 | P+M | 跨表面 conformance fixtures |
| D1-I03 | WebUI 是 Server 客户端，不是浏览器本地文件工作区；浏览器从不获得工作区目录句柄或托管路径。 | P+M | Web 构建依赖/权限扫描；API 负向测试 |
| D1-I04 | Desktop、CLI、Mobile 在本地模式承载本机 Core；在远端模式只调用 Server，不以本机 Core 改写远端响应或提交。 | P+M | 调用图与模式矩阵测试 |
| D1-I05 | 共享文件夹同步只复制字节；同步 Provider 不拥有 Weftext 语义，Weftext 也不把该模式声称为实时或安全多用户协作。Core 发现提交资格取得后发生的权威外部变化时，必须使当前提交资格失效并停止写入，不能基于旧状态继续提交或静默合并。 | P+M | 文案负向门禁；外部修改失效与端到端冲突案例 |
| D1-I06 | worker、Agent、自动化和连接器都在 Core 之外；它们只能提交输入、结果或变更意图，由 Core/Server 重新验证后提交。 | P+M | 依赖图、挂载和直接写入负向测试 |
| D1-I07 | 能力是否可用由版本化能力描述产生；UI 可见性、操作系统检测、成功过一次或网络猜测都不是能力权威。 | P+M | capability fixture 和 unknown-feature 测试 |
| D1-I08 | 对每个已安装或可到达的执行入口，任何不支持、未交付、未安装、未配置、离线、版本不兼容、用户未授权或策略拒绝都返回显式不可用语义，不得静默降级为另一实现、空结果或近似写入。没有安装且没有可达地址的表面没有虚构运行时响应义务。 | P+M | 错误矩阵 |
| D1-I09 | “支持某平台/CPU/包”必须逐行绑定实际构建、干净安装/启动和场景证据；源码可编译、CI 绿灯或存在图标都不能单独产生支持声明。 | P+M | release manifest gate |
| D1-I10 | 当前公开工件明确没有生产发行身份的平台签名、公证或商店分发；不得以未来签名、商店或自动更新承诺补足首发。临时 debug/development signing 与 provisioning 仅可用于开发和 conformance，不能进入公开工件或 Supported 声明。 | P+M | 发布资产、签名身份、下载入口与文案扫描；开发工件不得发布的负向门禁 |

## 4. 产品表面职责

### 4.1 Desktop

Desktop 是完整的个人本地工作区表面，也是可选的 Server 客户端。它负责原生工作区授权、窗口/会话/草稿等设备状态、完整编辑和阅读交互、调用本机 Core，以及在能力存在时协调本地转换 worker、Agent broker、自动化和连接器。

Desktop 不拥有文档解析、身份、查询、动作、事务或冲突语义；UI WebView 不得直接读写工作区。远端模式不得挂载或直写 Server 工作区；Server 管理和基础设施运维不属于 Desktop 首期职责。

### 4.2 WebUI

WebUI 是由 Weftext Server 提供的日常工作和授权管理表面，支持浏览器中的阅读、编辑、查询、视图、批注、协作以及 Server 管理。它不是只做管理的控制台，也不是可以脱离 Server 打开本地文件夹的 PWA。

WebUI 不运行本地 Core，不拥有本地工作区，不运行转换 worker、模型、Agent runner、自动化调度器或连接器；它可以在相应 Server 能力可用且有权时发起或管理这些 Server 能力。浏览器缓存、草稿、选择和页面状态均非工作区权威。

### 4.3 Server

Server 是托管工作区的唯一在线写入边界。它负责认证会话、授权执行、审计、托管备份/恢复、部署健康、并发协调、WebUI 和版本化客户端 API；在托管模式内调用同一 Core。对同一托管工作区后端，Server 控制面必须跨全部进程、实例和重启重叠窗口唯一授予提交资格，不能把“每个实例内都有 Core”解释为每个实例都可提交。后续实时协作由 Server 协调，但持久结果仍经 Core 成为规范工作区状态。

Server 不发明独立领域模型、解析器、Query、Action 或文件操作；数据库/控制平面不成为内容副本。Server 可协调可选 worker、Agent、自动化和连接器，但这些能力不得成为 Server 基础启动或读取工作区的硬依赖。

### 4.4 CLI

CLI 是无界面的本地和远端正式表面，用于脚本化、批处理、诊断、备份/恢复和 Server 管理。其本地命令直接调用 Core，远端命令调用 Server API；同一动作必须使用同一能力标识、预览、错误和提交结果。

CLI 不提供富文本可视编辑器、实时光标/在线状态 UI 或隐藏交互式确认。需要确认的破坏性或外部操作必须有显式参数/输入并保持非零失败状态。CLI 可以在以后发起本地或 Server 转换、Agent、自动化和连接器操作；本地 Agent 由 CLI 可选启动的同一能力族 broker 承载，而不是嵌入 CLI 的第二套 Agent 权威、工具语义或提交边界。具体 Agent 协议仍由 D10 决定，最终写入仍由本机 Core 或 Server Core 重新验证并提交。

### 4.5 Mobile

Mobile 是 iOS/Android 原生本地客户端和 Server 客户端，不是缩窄 WebUI。首期本地能力包含打开/创建移动端可安全承载的工作区、阅读、编辑、导航、搜索、查询与视图、Core 动作、批注、草稿、冲突呈现，以及不改变内容语义的工作区备份/导出。远端能力在 Server API 稳定后提供相同领域语义和协作消费。

当前 D1、G1/G1.1 与 G2 范围内，Mobile 明确排除转换执行/委托/监控/批准、AI Agent 入口/运行/审批、自动化调度和连接器凭据管理。它可以读取这些能力已经通过合法工作区提交产生的普通结果，也可把安全文件作为普通附件保存；不得因此探测、转换或采用附件内容。Mobile 的资源、电池、后台和文件 Provider 限制必须通过能力不可用语义暴露，不能改写 Core 规则。未来若改变这些排除项，必须重开 D1 并与 D8/D9/D10 联合审查，不能由实现或 UI 悄悄加入。

## 5. 平台/能力矩阵

符号：`L` 本机承载；`R` 作为 Server 客户端；`H` Server 承载；`I` 仅发起/管理已承载能力；`C` 只消费已提交结果；`—` 当前 D1/G1/G1.1/G2 明确不进入该表面。括号内为阶段边界。此表是产品能力归属目录，不是 Release 支持声明；只有通过第 11.3 节证据门并列入当次 release manifest 的 OS/CPU/工件行才能称为 Supported。

| 能力 | Desktop | WebUI | Server | CLI | Mobile |
| --- | --- | --- | --- | --- | --- |
| 本地工作区 Core 读/写 | L | — | — | L | L |
| 托管工作区 Core 读/写 | R | R | H | R | R（Server 可用后） |
| 文档/领域读取、编辑、Query、View、Action | L/R | R | H | L/R | L/R |
| 原生富交互编辑与无障碍 UI | L/R | R | — | — | L/R |
| 本地设备草稿/会话状态 | L | 浏览器本地 | — | 最小临时输入 | L |
| Server 账户、成员、权限、审计管理 | 会话消费；非基础设施管理 | I | H | I | —（首期） |
| 实时协作、在线状态、光标 | R（G2 后续） | R（G2 后续） | H（G2 后续；G2 不交付） | — | R（G2 后续） |
| 本地备份/恢复 | L | — | — | L | L（移动端安全范围） |
| 托管备份/恢复 | — | I | H | I | — |
| 转换 worker | L（可选、后续） | I（Server 可选、后续） | H（可选、后续） | L/R 发起（后续） | — |
| AI Agent | I/L broker（可选、后续） | I（Server 可选、后续） | H broker（可选、后续） | I/L broker（可选、后续） | — |
| 自动化调度 | I/L（可选、后续） | I（Server 可选、后续） | H（可选、后续） | I/L（后续） | — |
| 外部连接器 | I/L broker（可选、后续） | I（Server 可选、后续） | H broker（可选、后续） | I/L（后续） | C；不管理凭据或运行连接器 |
| 普通附件保存/读取 | L/R | R | H | L/R | L/R |
| 经 Core 验证的工作区提交 | L/R | R | H | L/R | L/R |
| 绕过 Core 的原始路径/字节直写 | — | — | — | — | — |

“—”是当前 D1/G1/G1.1/G2 的产品边界，不是暂时没做的按钮；对应能力描述必须返回 `unsupported_surface`。`后续`表示正式产品方向，但不能进入对应阶段的支持声明。I/C 是能力归属角色，不是额外 wire state。

## 6. 运行模式与状态所有权

| 模式/状态 | 唯一可提交权威 | 其他状态所有者 | 必须拒绝的解释 |
| --- | --- | --- | --- |
| Desktop/CLI 本地 | 取得该物理副本提交资格的唯一 Core 和本地工作区后端 | 调用表面拥有设备草稿/UI/命令状态；派生索引由本机重建。其他进程未取得协调资格时只能失败或只读 | 两个本机进程同时成为提交持有者；UI、索引、worker、备份或同步 Provider 是内容权威 |
| Mobile 本地 | 取得该物理副本提交资格的 Mobile 包内 Core 和经证明满足所需文件/原子语义的设备后端 | Mobile 拥有草稿、缓存、平台文件授权和生命周期状态；后端不能满足时能力明确不可用 | 因平台受限而另创移动语义、假定任意 File Provider 都安全或建立隐式云权威 |
| 完全离线本地 | 同上；网络不存在不削弱本地提交能力 | 外部能力显示 `offline` 或 `missing_component` | 为保持“功能可用”而调用未声明远端服务 |
| Server 远端 | 获得该托管后端唯一提交资格的 Server 实例内 Core 和托管工作区后端 | 客户端只有缓存、草稿、选择、分页/会话等非权威状态；Server 控制面拥有账户/ACL/审计。该远端工作区的读取、计划、Query 和提交均由取得资格的 Server Core 执行；同后端的其他实例只能拒绝写入或保持只读 | 两个 Server 进程/实例或重启重叠窗口同时提交；本机 Core、客户端缓存、离线副本或浏览器数据库解释或提交托管工作区 |
| 远端断网 | Server 仍是最后可提交权威 | Desktop/WebUI/Mobile 可保存明确标记的非权威草稿；CLI 失败 | 把草稿称为已保存、已同步或已合并 |
| 共享文件夹复制 | 每个物理副本由取得资格的本地 Core 负责各自独立提交；不同物理副本之间不存在 Weftext 协调者 | 文件同步 Provider 只负责字节传输；Core 负责检测到达后的不完整/外部变化并在无法证明安全时停止写入。同一物理副本仍受单提交持有者规则约束 | 同步 Provider 是事务、权限或协作 Server；两个进程可安全并发写同一副本；检测到外部变化后继续基于旧状态提交 |
| Server 实时协作 | Server 内 Core 是持久权威；协作协调器只拥有短暂会话/传输状态 | 客户端拥有未提交输入和在线展示状态 | CRDT/操作日志/在线状态本身自动成为工作区作者源 |
| worker 任务 | 发起侧 Core/Server 在最终提交前仍是唯一工作区权威 | 协调器拥有任务状态；worker 只拥有隔离临时输入/输出 | worker 输出、缓存或回执可直接写工作区 |
| Agent/自动化/连接器 | 本机 Core 或 Server Core，取决于工作区模式 | broker/调度器/Provider 拥有非权威运行状态和凭据引用 | transcript、模型输出、第三方对象或计划即为工作区变更 |

本地工作区不得同时作为 Server 托管目录被 Desktop/CLI/Mobile 直接打开。同一物理本地副本的提交资格必须由 D6 定义的占用/协调机制唯一授予；对同一托管工作区后端，这一资格也必须跨全部 Server 进程、实例和重启重叠窗口唯一授予，第二实例必须拒绝写入或保持只读。D1 不提前固定锁文件、OS 锁、租约、路由、串行化或 revision 结构。任何外部修改被发现后，现有提交资格必须失效，直到 D6 的协调/恢复流程建立新资格。托管或本地工作区的导出/备份只复制合法工作区内容，不与原工作区形成共同提交权威；副本身份由 D3/D6 决定，普通附件通道不得冒充备份。

## 7. 部署与进程拓扑

### 7.1 本地 Desktop

```text
OS user
  -> Desktop UI/WebView (untrusted presentation)
  -> narrow native command boundary
  -> Desktop host + in-process Core (trusted workspace authority)
  -> granted local workspace backend

Desktop coordinator
  -> optional isolated worker / Agent or connector broker
  -> staged output or proposed action
  -> Core preview/validate/commit
```

### 7.2 远端 Desktop/CLI

```text
Desktop / CLI remote client
  -> authenticated versioned Server API
  -> Server transport/authz/audit boundary
  -> Server in-process Core -> hosted workspace backend

optional local broker -> proposed intent only -> Server re-authz + Server Core commit
```

远端客户端可同时因为另一个本地工作区而装载本机 Core，但该 Core 不得读取、解释、计划或提交当前远端工作区。

### 7.3 本地 CLI

```text
OS user/script -> CLI argument/parser -> in-process Core -> granted workspace
                                      -> optional broker/worker -> Core commit boundary
```

### 7.4 Mobile

```text
Mobile UI -> narrow application boundary -> packaged Core -> app/file-provider workspace
Mobile UI -> authenticated HTTPS -> Weftext Server (remote mode)
```

### 7.5 托管 Server 与 WebUI

```text
Browser / Desktop / CLI / Mobile
  -> TLS + authenticated versioned Server API
  -> Server transport/authz/audit boundary
  -> in-process Core
  -> hosted workspace backend

Server coordinator
  -> optional isolated worker / Agent / automation / connector broker
  -> staged output or proposed action
  -> Server re-authz + Core preview/validate/commit
```

WebUI 与 Server 作为同一协调发布集交付；WebUI 资产可以嵌入 Server 包，但浏览器执行环境始终位于网络信任边界之外。同一托管后端即使被多个 Server 实例发现，也只有取得唯一提交资格的实例能进入 Core 提交路径；其他实例在资格移交完成前拒绝写入或保持只读。

## 8. 进程、信任、权限与网络边界

| 发起方 → 目标 | 允许 | 禁止 | 机械门禁 |
| --- | --- | --- | --- |
| Desktop WebView/UI → Desktop host | 版本化、最小、类型化命令；只传用户意图、授权句柄和展示输入 | 任意路径读写、shell、直接数据库/索引写入、绕过预览的工作区写入 | Tauri/IPC allowlist 与负向调用测试 |
| Desktop/CLI/Mobile host → Core | 本地模式内的共享语义调用 | 调用方自算身份、权限、冲突或提交结果 | 跨调用方 fixture |
| WebUI/远端客户端 → Server | 经 TLS、认证、授权和版本协商的 API | 文件系统协议、数据库访问、托管目录挂载、worker 直连 | API/网络拓扑测试 |
| Server → Core | 认证上下文内的读取、计划、查询、提交 | Server 旁路直接改工作区或重写 Core 诊断 | 依赖扫描和故障注入 |
| Core → 工作区后端 | 受 D2–D6 冻结合同约束的唯一读写 | 任意外部网络、模型/Provider SDK、UI 回调决定语义 | 构建依赖/网络 deny 测试 |
| 协调器 → worker | 单任务、显式能力、工作区后端之外的只读/复制输入与隔离临时输出、限制资源和网络 | 可写工作区挂载、在工作区内暂存、继承全部用户环境/凭据、自动监听并提交输出、输出即提交 | 沙箱、路径/挂载、崩溃清理和自动提交负向测试 |
| Agent → broker/Server | 有范围读取、建议、预览请求、获批动作请求；本地 broker 向 Core 或远端 broker 向 Server 提交时携带发起主体和获批范围，效力不大于其交集 | 原始工作区、shell、Core 私有 API、永久令牌、以 broker 自身扩大权限或无主体提交 | capability、主体/范围和审计负向测试 |
| 自动化 → broker/Server | 明确主体和能力下的定时调用；写入仍过 Core | 无主体后台写、跳过批准/策略、把定时器状态放进文档权威 | 调度/撤销/重放测试 |
| connector broker → Provider | 仅为已声明能力使用显式网络出口和凭据引用 | Provider 直接回调工作区写入口；凭据进入工作区/日志/Agent 上下文 | egress allowlist、secret scan |
| 浏览器 → worker/Provider | 无直接调用；必须经 Server 能力边界 | 暴露 Server 凭据、工作区路径或可写临时目录 | CSP、端点和渗透测试 |

权限的主体、角色、字段过滤和非披露细节由 D6 冻结。D1 先冻结：权限检查发生在拥有工作区权威的一侧；客户端提供的“已授权”标志没有效力；外部能力的有效权限不大于发起主体与被授能力的交集。

## 9. 能力归属

| 能力族 | 产品归属 | 可部署位置 | 不属于 |
| --- | --- | --- | --- |
| 工作区解析、领域、Query/View 语义、计划、验证、提交、规范诊断 | Core 基础能力 | Desktop/CLI/Mobile 本机进程；Server 进程 | UI、worker、Agent、数据库、Provider |
| 本地设备草稿、窗口、选择、最近项、凭据句柄 | 表面设备能力 | 对应设备 | 可移植工作区权威 |
| 认证、ACL、会话、托管审计、协作协调、托管备份运维 | Server 基础能力 | Server 控制平面 | Core 领域内容、WebUI 本地状态 |
| 转换 | 可选适配能力 | Desktop/CLI 本地协调或 Server 协调；隔离 worker/外部 Provider | Core；Mobile；浏览器进程 |
| AI Agent | 可选委托能力 | Desktop/CLI broker 或 Server broker；模型可为外部 Provider | Core；Mobile；浏览器进程；直接写入者 |
| 自动化 | 可选调度/委托能力 | Desktop/CLI 本地控制面或 Server 控制面 | Core；Mobile 首期；无主体写入者 |
| 连接器 | 可选外部适配能力 | Desktop/CLI broker 或 Server broker；外部 Provider 保存其自身状态 | Core；Mobile 首期凭据/运行时；工作区权威 |

D9 决定转换 worker 的具体协议和 Provider Profile；D10 决定 Agent、自动化、连接器的能力、批准和审计协议。D1 只确定 CLI 可与 Desktop 一样启动可选本地 Agent broker；该 broker 不拥有第二套 Agent 权威、工具语义、权限或提交边界。D1 禁止这些能力迁入 Core 或 Mobile，但不抢先规定请求/响应字段、审批等级、持久化格式或 Provider 列表。

## 10. 能力探测与不可用语义

每个已安装或可到达的执行入口在接触工作区前，必须先经过不依赖业务 `contractMajor` 的稳定启动协商入口，再消费由实际承载边界生成的能力描述：本地由宿主/Core 组合生成，远端由 Server 生成。启动协商只发现双方支持的 major 集、选择共同 major 或返回 `incompatible_version`；它不得列出业务 capability、解释工作区、授权操作、修改状态或形成第二套产品合同权威。选择完成后，唯一业务合同权威是所选 major 下由实际承载边界产生的能力描述；作者、UI 或客户端不能自行声明能力形成第二权威。

能力描述的最小语义形状如下；字段名是 D1 的跨表面合同，传输封装可由后续主题补充：

```json
{
  "productVersion": "0.1.0",
  "contractMajor": 1,
  "surface": "mobile",
  "mode": "local",
  "capabilities": {
    "workspace.read": { "state": "available" },
    "workspace.write": { "state": "available" },
    "conversion.execute": {
      "state": "unavailable",
      "reason": "unsupported_surface"
    },
    "agent.session": {
      "state": "unavailable",
      "reason": "unsupported_surface"
    }
  }
}
```

规范状态只有 `available` 和 `unavailable`。`unavailable.reason` 封闭为：

- `unsupported_feature`：请求 ID 不属于已经选择的 `contractMajor`；禁止旧别名、同名 UI 或近似操作回退；
- `unsupported_surface`：能力 ID 已知，但产品明确禁止进入该表面；不能通过安装或登录解决；
- `not_in_release`：能力和表面组合是正式方向，但当前协调版本没有交付；
- `policy_denied`：当前主体或部署策略不可使用；必须遮蔽组件、配置、网络、版本和暂时故障等更细部署信息；
- `user_action_required`：用户尚未授予本地文件、网络或其他显式设备权限，修复需要该用户操作；
- `missing_component`：可选 worker/broker/依赖未安装或已移除；
- `not_configured`：所需组件已交付且存在，但管理员或本机尚未配置；
- `offline`：所需本地或网络承载边界不可达，因此不能判断其内部运行状态；
- `incompatible_version`：承载边界已到达，但调用方、Server、worker 或 Provider 的版本组合不被所选合同允许；
- `temporarily_unavailable`：承载边界已到达、版本兼容且配置仍存在，但其内部能力暂时失败，预计无需重新安装或重新配置即可恢复。

对一个 capability 的一次探测只能返回一个 reason。所有正式表面按以下固定顺序选择第一个成立项，不能按 UI、平台或调用方改变：

1. ID 不属于已选择 major → `unsupported_feature`；
2. 产品禁止该表面 → `unsupported_surface`；
3. 当前 release 未交付 → `not_in_release`；
4. 主体或部署策略拒绝 → `policy_denied`，并停止探测或披露后续部署状态；
5. 缺少用户显式授权 → `user_action_required`；
6. 所需组件不存在 → `missing_component`；
7. 组件存在但未配置 → `not_configured`；
8. 所需承载边界不可达 → `offline`；
9. 边界已到达但版本组合不允许 → `incompatible_version`；
10. 边界已到达且前述条件均不成立，但能力内部暂时失败 → `temporarily_unavailable`。

这是一条归一/优先级规则，不授权探测者越权收集被遮蔽信息。静态产品与发布目录决定前三项；`policy_denied` 必须在查询组件、配置、网络、版本或临时健康详情之前遮蔽它们。组件曾经可用但随后消失仍归入 `missing_component`，不能泄露“从未安装”与“后来移除”的历史。`offline` 与 `temporarily_unavailable` 由承载边界是否已到达互斥。

未知 major 由稳定启动协商入口返回 `incompatible_version`；未知 capability ID 由所选 major 的能力边界返回 `unsupported_feature`。二者都不得回退。`workspace_busy`、`stale_revision`、目标冲突等针对具体工作区/操作的失败不属于 capability reason，由 D6/D7 的操作错误代数冻结。每次实际调用也必须重新验证能力；探测不是授权票据。UI 的隐藏/禁用/说明、CLI 的退出码、Server 的错误响应和 Mobile 的提示都只是同一规范状态、reason 和修复类别的投影。

产品层要求所有不可用结果至少携带：稳定 reason、受影响 capability ID、是否可重试、不会泄露秘密的修复类别。具体 wire、HTTP 状态和本地错误类型由实现主题冻结，但不得减少这些语义。全部单一原因和重叠原因组合都必须有跨 Desktop、WebUI、Server、CLI、Mobile 的 conformance fixtures。

Desktop 本地安装了受支持 worker 时，承载边界可产生如下状态；它不证明某个具体格式或 Provider 可用：

```json
{
  "surface": "desktop",
  "mode": "local",
  "capabilities": {
    "conversion.execute": { "state": "available" }
  }
}
```

## 11. 版本与发布声明边界

### 11.1 协调版本

Desktop、CLI、Server、嵌入 WebUI、Mobile 源码包和可选能力组件使用同一产品版本集合；各工件仍记录自己的构建目标和组件版本。`contractMajor` 表示跨表面语义不兼容边界，能力 ID 表示可单独探测的功能。任何旧原型名、别名、迁移解析或双读写都不进入第一版公开合同。

启动协商入口具有跨 major 稳定、固定且只读的产品语义：双方在任何工作区读取、能力描述或提交之前，交换支持的 major 集并选择共同 major；没有共同 major 时只返回 `incompatible_version` 和双方支持的 major 集。它不承载领域对象、capability 列表、权限结论或工作区数据，不能成为第二产品合同权威。具体 HTTP、IPC 或本地类型封装由实现主题决定，但每个正式表面必须能解析同一协商结果。

客户端/Server major 不匹配且当次 release manifest 未显式批准该组合时，读取与写入均默认拒绝；只有发布证据列出的只读互操作组合才可读取，不能由客户端猜测“看起来能用”。可选 worker/Provider 版本不匹配只使对应能力不可用，不应阻止基础工作区打开。

### 11.2 首个 GitHub 公共发布应声称的工件

首发采用窄的本地产品切片；正式产品表面不等于首发全部交付。

| 发布阶段 | OS / CPU | 工件 | 声称 | 明确不声称 |
| --- | --- | --- | --- | --- |
| G1 本地首发 | Windows 11 x86_64 | 无签名 NSIS `.exe` Desktop；配套 x86_64 `.zip` CLI | 本地 Desktop 和 CLI 的已列能力 | Windows ARM64、macOS/Linux Desktop、Server/WebUI、Mobile、转换、Agent、协作 |
| G1.1 本地 CLI | Ubuntu 24.04 LTS x86_64 | `.tar.gz` CLI | 本地无界面工作区能力 | Linux GUI、其他发行版/CPU、Server |
| G2 托管发布 | Ubuntu 24.04 LTS x86_64 | Server + 内嵌 WebUI 的 `.tar.gz`；可另附同内容 OCI image tar | 经认证托管工作区、WebUI、备份/恢复和已列 API | 其他 Linux/CPU、托管 SaaS、实时共同文本编辑，除非各自另有证据 |

G1 不等待 Server、WebUI、实时协作、Mobile、转换或 Agent；G1.1 只在独立 Linux CLI 证据行通过后发布。G2 等待 D6 权限/事务/备份边界和 Server/WebUI 端到端证据，但不等待实时共同编辑。Linux ARM64 Server/CLI、macOS、Linux Desktop 和任何其他包只有新增完整证据行后才能声称支持；源码构建说明不得写成受支持二进制。

Mobile 仍是必须实现和验证的独立正式表面；Desktop 的 macOS/Linux/其他 CPU 只是同一 Desktop 表面的平台端口。两者都不能绕过第 11.3 节证据门产生 Supported 声明。由于“仅 GitHub Releases、无生产发行签名/公证/商店分发”约束，D1 不声称存在面向普通用户的 iOS/Android 生产安装包；Mobile 源码和开发构建只证明工程实现与 conformance。允许 SDK 或平台为本地开发、CI、模拟器和登记测试设备强制使用临时 debug/development signing 与 provisioning；这些开发工件不得进入公开 GitHub Release，不产生生产支持、下载可用性、发行渠道或未来签名计划。未来若改变渠道，必须另立决策；本文不新增签名计划。

WebUI 不单独下载；它随 Server 协调版本交付。G2 的浏览器支持范围必须在每个 release manifest 中列出经实际自动化和人工测试的浏览器名称与精确版本，不使用“现代浏览器”泛称。

### 11.3 每个支持行的证据门槛

任何一行只有同时具备以下证据才可出现在“Supported”表：

1. 从精确 tag/commit、锁定工具链和锁文件在对应原生 OS/CPU 干净 runner 构建；交叉编译只能作为附加证据；
2. 工件名、版本、目标 triple、字节数、SHA-256、SBOM、许可证/NOTICE 和已知限制进入机器可读 release manifest；
3. 在没有源码仓库和开发工具的干净 VM/机器安装或解压并启动，完成该表面最小端到端场景、重启和卸载/清理检查；发布页提供该平台原生命令的 SHA-256 校验步骤并绑定同一工件摘要；
4. 自动化包级 smoke 与独立人工 clean-install 记录都绑定工件摘要；GUI 还需键盘、IME、缩放/高对比度和系统警告记录，Server 还需持久卷、认证、备份恢复和支持浏览器矩阵；
5. 能力描述与实际包内容一致；缺失组件返回规范不可用状态；
6. 发布页只陈述已通过的行，失败或缺证据的行移至 `Experimental/Unsupported`，不得以其他平台的 Core 测试替代。

### 11.4 无生产发行签名的安装体验

- 对每个公开工件，文件名、release manifest、发布页、安装说明和应用 About 都明确写其未使用生产发行身份的平台代码签名，并提供校验摘要、平台原生校验命令和源 commit；不得称为“已验证发布者”。开发测试所用的临时 debug/development signing 不得出现在公开工件或被称为发行签名。
- Release 首屏必须直接列出 `Included`、`Not in this release` 和 `How to verify`，不得把正式表面、未交付能力或无生产发行签名状态藏在路线图或二级页面。
- Windows 安装说明记录该受测版本真实出现的系统警告和逐步继续/取消路径；不得要求关闭 SmartScreen、降低全局安全设置或忽略摘要不符。
- G1 不承诺静默安装、自动更新或后台提升权限。更新通过 GitHub Release 手工下载安装；回滚和工作区备份步骤必须可见。
- 缺少 Server、Mobile、转换、Agent、某 CPU 或某 OS 不是隐藏在路线图中的细节，而是发布 manifest 和运行时 capability 的显式 `not_in_release` 或 `unsupported_surface`。
- GitHub/Sigstore provenance 或校验信息若存在，只是供应链完整性证据，不等于生产发行身份的平台签名，也不得改变无生产发行签名文案。

## 12. 发布依赖顺序和首发/后续边界

```text
冻结并实现共享 Core 契约
  -> G1 Windows Desktop + Windows CLI 本地首发
      -> G1.1 Ubuntu x86_64 CLI（独立证据行，不阻塞 G2）
      -> Server 安全基础（authz/audit/backup/deploy）
      -> WebUI 日常与管理表面
      -> G2 托管发布
          -> 实时协作
          -> Mobile 远端模式

共享 Core + D8 移动交互
  -> Mobile 本地实现与开发构建验收（允许仅测试所需临时 debug/development signing）
  -> 生产分发等待未来独立渠道决策（本 D1 不规划生产发行签名）

共享 Core + D9
  -> Desktop/CLI 可选本地转换
  -> Server 可选转换（还依赖 Server 安全基础）

共享 Action/权限/审计 + D10
  -> Desktop/CLI 可选本地 Agent/自动化/连接器
  -> Server 可选 Agent/自动化/连接器
```

G1.1 与 Server 安全基础可在 G1 后并行，G1.1 不阻塞 G2。实时协作不得阻塞安全的非实时 Server/WebUI 发布，但必须明确标为不可用；它依赖 Server 会话、权限、事务、恢复和多客户端测试。Mobile 本地不依赖 Server，Mobile 远端依赖稳定 Server API。转换不决定 Core 模型，也不阻塞 G1/G2 基础读取编辑。Agent、自动化和连接器不阻塞任何基础表面，且必须晚于可复用的动作、权限和审计边界。

## 13. 明确非目标

- 不定义 D2–D6 的对象、身份、引用、存储文件、事务日志、ACL、同步/冲突或 CRDT。
- 不定义 D9 的转换 IR、Provider/worker wire、格式支持矩阵或模板语法。
- 不定义 D10 的 Agent 工具协议、审批级别、自动化表达式、连接器目录或 transcript 格式。
- 不实现代码，不修改公开规范、实现、fixture、`repos/weftext` 或 `brand/`。
- 不把 WebUI 变成本地文件 PWA，不把 Mobile 变成 WebUI 包装，不把 CLI 变成另一个语义实现。
- 不把共享文件夹复制宣传成协作，不承诺离线远端提交或自动合并。
- 不承诺 macOS/Linux Desktop、Windows ARM64、Linux ARM64、iOS/Android 生产包、应用商店、生产发行签名、公证、自动更新或商用发布；仅测试所需的临时 debug/development signing 不构成例外发行承诺。
- 不为未发布原型保留任何兼容入口。

## 14. 替代方案与拒绝理由

| 方案 | 结论 | 最小拒绝理由 |
| --- | --- | --- |
| Desktop 是唯一产品，其他都是薄工具 | 拒绝 | 违反五个正式表面基线，Server/WebUI/Mobile 会长期成为二等客户端。 |
| 在有生产安装包前不把 Mobile 当正式表面 | 拒绝 | 直接违反 Mobile 必须支持的冻结输入，也会允许 D2–D8 先形成桌面/浏览器专用语义，事后难以补齐移动端。正式地位不等于当前 Release 已支持。 |
| 每个表面各自实现最合适的解析/写入 | 拒绝 | 同一输入可得到不同身份、错误或提交，是直接的第二权威。 |
| WebUI 通过 File System Access API 打开本地工作区 | 拒绝 | 浏览器权限/原子文件语义不一致，并与 Server WebUI 身份混淆。 |
| 所有客户端都以 Server 为唯一权威 | 拒绝 | 破坏无账户、离线、本地优先和 Mobile 本地能力。 |
| 同步文件夹即协作层 | 拒绝 | 字节复制没有主体、事务顺序、权限、在线状态或确定合并。 |
| 浏览器/客户端离线副本可直接提交，稍后同步 | 拒绝当前 D1 | 在 D6 未定义 revision/冲突前会创建隐式双权威。只允许非权威草稿。 |
| worker、Agent 或自动化直接写暂存工作区后原子替换 | 拒绝 | 替换仍绕过 Core 的语义验证、授权和审计，崩溃时还会扩大破坏面。 |
| 把转换/Agent 编入 Core 以统一能力 | 拒绝 | 引入不可信输入、网络、凭据、模型和重依赖，使 Core 不再可离线、可嵌入、可最小审计。 |
| 首发一次交付五表面、Server、协作、转换和 Agent | 拒绝 | 一个失败层会掩盖其他层，且现有实际证据只接近 Windows Desktop/本地 Core；窄发布能保持声明真实。 |
| 从 Linux/macOS CI 编译通过推导 Desktop 支持 | 拒绝 | 源构建不证明包安装、WebView、权限、IME、无障碍、升级或卸载。 |
| 为改善无生产发行签名体验加入生产签名/商店路线 | 拒绝 | 违反当前渠道基线；开发测试临时签名不能被转化为发行计划，未来渠道变化必须是独立产品决策。 |

## 15. 端到端场景与最小反例

### S1 本地离线编辑

Windows Desktop 断网后打开本地工作区、编辑、预览、提交、重启并恢复。网络型能力均显式 `offline`；本地 Core 行为不变化。另一个本机进程不能同时取得该物理副本的提交资格。反例：断网导致文档保存按钮消失，或 CLI 与 Desktop 都报告提交成功。

### S2 同一机器双进程

Desktop 与 CLI 尝试同时写同一物理工作区。只有 D6 允许的协调方式可提交，另一方得到结构化冲突/占用；不能各自成功。反例：两边都报告成功但后写覆盖前写。

### S3 共享文件夹部分到达

设备 A 提交后同步 Provider 先传文档、后传控制字节；设备 B 的 Core 将其视为不完整/外部变化并使现有提交资格失效、停止相关写入，不把 Provider 状态当事务完成证据。具体检测、占用、恢复和冲突表示由 D6 定义。产品文案不得称该模式“实时协作”。

### S4 WebUI 断网草稿

浏览器编辑托管文档后断网。草稿留在设备并明确未提交；刷新或另一浏览器看不到这项变更。重连后必须重新取得 Server 能力/授权/修订再进入 D6/D8 定义的解决流程。反例：localStorage 被当作已保存托管版本。

### S5 远端权限变化

Desktop、WebUI、CLI、Mobile 同时打开托管工作区。Server 权限变化后，每个表面下一次调用都以 Server 结果为准；本地缓存或旧 capability 不能继续提交。具体错误和非披露由 D6 定义。

### S6 转换 worker 崩溃

Desktop 发起可选转换；worker 只拿到隔离输入并崩溃。工作区字节不变，能力返回暂时不可用/任务失败，临时输出清理。反例：worker 拥有可写工作区挂载并留下半成品。

### S7 Mobile 遇到 PDF 和 Agent 动态块

Mobile 可把 PDF 保存为普通附件，也可读取已经由合法提交产生的普通结果；`conversion.execute` 与 `agent.session` 永远返回 `unsupported_surface`，不创建远程任务、不显示可批准动作。反例：移动端把 PDF 上传 Server 自动转换，或为 Agent 保留隐藏入口。

### S8 Agent 提议跨节点变更

模型 Provider 只返回提议；Desktop/Server broker 把提议转换为受范围动作请求，Core/Server 重新验证，策略要求时由人批准后才提交。Provider 超时、重放或返回路径字符串不能写入。具体批准模型由 D10 定义。

### S9 G1 干净安装

用户在干净 Windows 11 x86_64 VM 下载 NSIS，先核对 SHA-256，看到未签名警告，按发布说明安装，创建/打开/编辑/提交/重启工作区，再卸载。About 显示版本、commit 和 unsigned。若只有构建产物没有这条证据，不得列 Supported。

### S10 错误平台下载

Windows ARM64、macOS、Linux GUI 或 iOS 用户访问 G1 发布页，得到明确 Unsupported/No artifact；网站或 README 不使用“跨平台桌面”。反例：图标、Tauri 配置或源码可编译被用作支持声明。

## 16. 对 D2–D10 的输入约束

| 后续主题 | D1 强制输入 |
| --- | --- |
| D2 文档/领域对象 | 对象定义必须跨五表面一致；不得出现 Desktop-only、Server-only 或 Mobile-only 的同名领域对象；设备/Server 控制面状态不属于工作区对象。 |
| D3 身份/生命周期 | 身份不能依赖 UI、浏览器 URL、本地绝对路径或 Server 数据库行；本地与托管切换/导出不得产生隐式双权威。 |
| D4 属性/schema/关系 | 凭据、在线状态、worker/Agent/Provider 状态、设备能力不得进入可移植领域属性；类型错误跨表面一致。 |
| D5 结构化数据 | Core 是规模和编辑语义权威；Mobile 可资源降级但不能改变对象种类、行身份或结果；不得用 UI 表格形态决定领域。 |
| D6 存储/事务/权限/同步 | 必须为本地、远端断网、共享文件夹复制和 Server 协作分别实现本表的唯一状态所有者；为同一物理副本以及同一托管后端跨全部 Server 进程/实例/重启重叠窗口提供单提交持有者协调；第二 Server 实例拒写或只读；外部变化使旧资格失效；派生缓存/草稿非权威；托管目录禁止客户端直写；权限在权威侧重验。 |
| D7 Query/View/Action | 同一 contract major 和 capability ID 下跨表面语义一致；不可用不能伪装成空结果或降级 Query；View 不探测 UI 猜测的能力。 |
| D8 编辑器/交互 | 所有按钮/命令从共享操作和 capability 描述派生；设备差异只能影响交互与可用性，不影响计划/错误/提交；WebUI 仅 Server，Mobile 禁止的入口必须做包级负向检查。 |
| D9 转换/worker | worker 在 Core 外、无可写工作区挂载、默认无网络、显式能力/依赖；Desktop/CLI 或 Server 协调，WebUI 仅发起，Mobile 不执行/委托/监控/批准。 |
| D10 Agent/自动化/连接器 | 都是 Core 外的可选委托能力；只部署于 Desktop/CLI 本地控制面或 Server 控制面；权限不超过主体交集；Mobile 无 Agent/自动化/连接器运行和凭据管理；不得把 Provider 状态变成工作区权威。 |

## 17. 实现影响图

本节只列未来影响，不授权本任务修改代码。

```text
stable bootstrap major negotiation (discovery only)
  -> selected contractMajor
  -> shared capability catalog + deterministic unavailable reason
  -> weftext-core public operation/result boundary
  -> Desktop native command allowlist + UI registry
  -> CLI local/remote command registry and exit errors
  -> Server capability endpoint/API/authz gate + embedded WebUI
  -> future Mobile package and forbidden-component gate

optional capability coordinators
  -> conversion worker launcher
  -> Agent/automation/connector broker
  -> staged output / proposed action
  -> Core or Server authoritative commit

release support manifest
  -> native build jobs
  -> artifact hashes/SBOM/licenses
  -> package smoke + clean-install evidence
  -> GitHub Release page and in-product About/capability display
```

当前仓库需要在实现阶段审计和替换的区域至少包括：`crates/weftext-core`、`apps/desktop`、`crates/weftext-cli`、`crates/weftext-server` 与其 `webui`、`crates/weftext-agent*`、`crates/weftext-import`/worker、发布 evidence schema/workflow、公开中英文产品/运行时/发布文档，以及尚不存在的 Mobile 包。现有 `prototypes/webui` 只能删除或重建，不能形成兼容义务。现有 Windows-only Tauri 目标和 NSIS 配置只能证明 G1 候选形态，不能代替发布证据。

## 18. 测试轮廓

1. **静态依赖门**：Core 依赖图无 UI、Tauri、Server、worker、模型/Provider SDK 或网络客户端；Mobile 包无 conversion/agent/automation/connector 运行代码；WebUI 无文件系统/worker 直连。
2. **调用边界门**：UI/WebView 只能调用 allowlist；同一托管后端只有取得唯一提交资格的 Server 实例可写，第二实例拒写或只读；worker 沙箱无可写工作区挂载；Agent/Provider 不能调用 Core 私有提交；CLI 本地 Agent 只能经可选 broker 提议并由 Core 重新验证。
3. **跨表面一致性**：相同 fixture/意图在 Desktop、CLI、Server/WebUI、Mobile 得到相同规范结果、诊断、计划摘要和提交效果；不适用表面得到明确不可用。
4. **模式所有权**：本地离线、远端、远端断网、共享文件夹部分到达、双本机进程、双 Server 同后端启动、Server 重启重叠/故障切换和协作重连均断言只有一个可提交权威；第二 Server 实例只能拒写或只读。
5. **能力状态矩阵**：每个 capability 对每个 surface/mode/release 的 available/unavailable + reason 与静态矩阵一致；未知 ID 返回 `unsupported_feature`，旧 major 只经稳定启动协商返回 `incompatible_version`；全部重叠原因按固定优先级归一，`policy_denied` 遮蔽部署详情，`offline` 与 `temporarily_unavailable` 由边界是否到达互斥；各表面投影一致且均无回退。
6. **安全负向**：路径穿越、绝对路径泄露、浏览器直读、worker 写挂载/工作区内暂存、Provider 回调、旧 capability 重放、权限变化、secret/transcript 写入工作区全部失败；Desktop IPC allowlist 负向样本必须在 G1 实际包运行。
7. **发布逐行证据**：G1/G2 每行在原生 runner 构建、包级自动 smoke、独立 clean-install/operation、摘要/SBOM/许可证、已知限制和无生产发行签名文案齐全；G2 还必须包含双 Server 同后端竞争和重启重叠证据；任一缺失自动从 Supported 移除。Mobile 临时 debug/development signing 只进入开发 conformance 记录，绝不进入公开 release manifest。
8. **文案负向门**：不出现“跨平台 Desktop”“Mobile 生产包”“同步即协作”“WebUI 本地文件”“生产已签名/已验证发布者”“转换/Agent 已内置”等超出证据的声明；开发测试临时签名不得被称为发行能力。
9. **端到端场景**：可自动化的 S1–S10 场景形成可重复脚本；需要人工判断的 OS 警告、物理设备和实体可访问性场景形成绑定工件摘要的人工验收记录。
10. **退役门**：实现切换时仓库级扫描删除旧原型 surface/capability 名、别名、fallback、双路径和 prototype bridge；不建立兼容测试。

## 19. 冻结状态与后续边界

- 冻结日期：2026-08-27。
- OpenCode 阶段：四个有效独立首审、两项真实争议定向复审；最终无未关闭 P0/P1。
- Sol Pro 对话：`架构评审结论`，（原评审会话地址不公开）。
- Sol Pro 首轮：`accept-with-changes`、P0 无、五条 P1、`Top-level material change: no`。
- 同对话定向关闭：P1-1 至 P1-5 全部 `closed: yes`，`New P0/P1: 无`，`Top-level material change: no`，`Final gate: pass`。
- 六条 Sol Pro P2 保留为非阻塞残余建议；它们不改变本决议，也不作为 D2 的隐藏前置条件。
- 下一项且唯一允许开始的主题是 D2“文档、内容和领域对象模型”。D2 只能读取本决议等 Weftext-Control 冻结材料，不继承模型会话，也不得反向修改 D1；若发现成立的上游反例，必须显式重开 D1。
- 本决议不授权实现、公开文档更新或发布；这些动作仍受 A2 与后续实施门控制。
