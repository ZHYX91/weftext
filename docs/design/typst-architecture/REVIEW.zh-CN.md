---
source_language: zh-CN
translation_status: source
---

[English](REVIEW.md)

# 验证与评审状态

状态：作者候选，尚未全局接受，尚未启动产品重构。前轮有界通过不转移到本方案；本文记录实际证据及其适用范围，不能把未运行场景记为通过。

## 1. 证据分层

需求核对已覆盖当前 REQUIREMENTS、历史 D4 全文及完整类型目录，形成 SOURCE §5.1 的七类能力清单。历史业务输入用于发现遗漏；旧字段封装与运行协议不自动继承。尚未逐字重新审计所有历史 D1–D10 文件。

| 实际工作 | 结果与范围 | 不能据此声称 |
| --- | --- | --- |
| 原生结构化数据 | 当前完整 [schema](examples/schema.json) 的 31 项有限 [设计检查](examples/design-checks.json) 通过；官方 Typst 0.15.1 运行 23 个 [编译用例](examples/syntax-experiments.json)，22 成功、heading/Bib 同键按预期失败。语义负例仍可编译 | Python 检查是实际 Rust 安装器、CEL parser、provider、权限、表单或导出；原生编译成功证明字段语义正确 |
| 标题与混排 | 前轮七组有限实验验证同级独立/同行标题的编号、目录、引用及增删移动改级；副标题不进入 heading counter | 完整编辑器、深层/长文/所有自定义 show 通过 |
| Word | 独立样例十项 OOXML 结构检查通过，包含整段 Heading N、正文有效字符格式重置和字段 | Word/WPS/LibreOffice 实机字段更新、目录、保存重开通过；参考图不是 Office 渲染 |
| 来源反例 | 本轮七个官方 CLI 样例说明 query.body 与实际 post-show 输出不同；删除显示仍可能保留 heading introspection；重复 include 不能从根 AST 猜出现项 | AST 或 CLI query 足以生成完整流式阅读树 |
| 本地文件保护 | Windows NTFS 有限探针验证 share=0 阻止测试中的写开、替换、删除；已有 writer/mapping 使保护获取失败 | 所有平台/驱动/过滤器、断电、外部 ABA 或自动恢复已证明 |
| 中断恢复 | 八个两文件进程中断场景检查 before/after、保留第三态和原 operationId 的零重复写 | 实际断电、日志撕裂或释放保护后同对象同字节的来源连续性 |
| 目录反例 | 目录 share=0 不能排除新增/写子文件；子文件句柄可能阻止父 rename；no-replace 保留占用目标 | capture 是全目录 CAS，或开放目录可仅靠扫描证明完整负范围 |
| 普通目录移动 | 同对象 no-replace 移动保持目录/子文件身份，晚加子项和外部新稿随树保留；已有 writer 导致 busy，关闭后重试成功；Node 双改名成功，竞争旧名称使回退拒绝并保两稿 | 对外部工具的多对象瞬时原子、跨卷同等原语、断电/祖先改动/完整故障恢复 |
| 目录保护、创建与清理修订 | [独立可运行探针](probes/core-repairs/README.zh-CN.md) 在本机 NTFS 验证实际 LIST 访问保护祖先/目标父、持原句柄移动、NtCreateFile 返回原目录创建句柄、同名冲突、计划文件保护及非空目录保留未知新增 | 所有祖先链/别名/过滤器、目录项耐久、断电、多步骤恢复、Server 隔离或生产后端通过 |
| Query 共同快照模型 | 同探针包的有限内存模型复现逐对象混读 sum20；共同快照得 sum10，冻结分页保持，模拟撤权停止后续交付 | 真实文件日志、完整库存、当前发布、权限、持久保留或性能已实现 |
| 库级样式 | 五次官方编译核正文/列表字号、表格边框、局部恢复和真实相对资产；改模块不改 note bytes/AST；转换 show 内部 set 不可一般覆盖的反例成立 | 自动库设置 UI、Core 依赖/撤权/迟到失效、导出或富编辑已实现 |
| 官方求值观察桥 | [可复核原型](probes/semantic-bridge/README.zh-CN.md)：11 例 observer on/off/官方发布 CLI 的 PDF 逐字节相同；on/off 的 World 请求序列与最终 Frame 相同；tracked Sink 随缓存重放最终稳定轮事件 | 完整来源值流、所有布局/表格/公式、renderer、逆向富编辑、AT、性能或全部 E1 通过 |
| Html 最终消费与原生样式 | [16 例可复现探针](probes/html-flow/README.zh-CN.md)：关闭适配的 Html 与官方字节相同、开启适配串行/并行 Html 与最终树相同、三种 Paged PDF 相同；有界正文语义投影保持，World 顺序为 15/16 例相同 | 浏览器 renderer、所有样式/表格拓扑、生产来源/writer、局部 frame 的阅读/复制树、AT 或 E1 全部通过 |

这些实验没有修改产品实现或真实资料。完整原始运行资料保留在私有工作区；公开观察桥附固定 patch、lock、11 例输入、runner 和精简实测结果，不含下载缓存、编译目录或私人聊天原文。

观察桥最初的直接日志在缓存命中时丢失内层事件；实际改为官方 tracked Sink 的独立观察槽，在原顺序中合并并由缓存重放，未禁用缓存或额外执行 show。默认样例最终稳定轮由一次物理 realization 补齐为四项，重复 include 为七项，同行标题/列表/表格/公式组合为四十二项。独立反例随后证明稳定 trace 仍包含被丢弃的测量内容和重复布局，不能直接作为最终阅读树；这些计数只证明观察完整性，不证明可见出现项。当前路线改为 EDITOR §8 的官方 Html 最终消费树与 Paged 输出，样式/角色适配另行验证。计算字符串 `text(left + right)` 只有调用 span，不能据此恢复两个输入的来源或授予 writer；这类内容保留为生成/不可直接编辑，除非另有经过验证的来源证明。原 11 例等值证据继续有效，但不关闭 E1。

## 2. 设计风险与产品验收的区别

设计复核须判断机制是否具体、可实现并满足需求；产品验收须验证实际实现和声明支持的平台。不能要求设计阶段先写完整产品，也不能用“以后测试”掩盖尚无可行算法。

| 项目 | 当前必须核清的设计问题 | 后续真实实现验收 |
| --- | --- | --- |
| E1 官方语义桥 | 最终 Html 消费树排除测量/丢弃工作，保真实重复；有界原生样式/角色及来源适配，Html/Paged target 边界明确；默认内容可流式，歧义不伪造 writer | 完整 renderer、全部编辑逆映射、AT/RTL、资源/预算、安全、升级回归 |
| C1 文件与结构 | 文件保护和对象移动各有明确操作边界；不能把普通 move 错当复制全树，不能暗称强 negative scope | 支持的文件系统、权限/rename 失败、断电、恢复、外部竞争和 reader barrier |
| C2 权限与后台服务 | 字段窄读写静态隔离、真实文件 writer fencing、当前交付与缓存失效 | 隐藏值侧信道、双 holder、撤权/分块/提交竞争、进程隔离 |
| S1 类型及查询 | 一套 Type/Value；所有已选能力有具体形状/消费者；缺值、同项集合和精度一致 | 完整 seed/provider、CEL profile、表单/Query/导入导出垂直验收 |
| F1 批注和复制 | 准确来源与同一分配/结果映射；源和批注接受状态同事务；权限分域 | 崩溃/并发/媒体精度、复杂源码选区、重附及导出 |
| E5 可编辑输出 | 原生标题/正文格式和目标限制明确，准确工件确认/下载 | 真实 Office/WPS/LibreOffice、XLSX 精度、损失报告和内网续下载 |

普通本地使用、直接外部编辑、完整合法 Typst 与服务端多用户需求都保留。生产后端尚未通过验收不能伪称支持；也不能通过把所有正常操作标为 unavailable 来规避需求。

## 3. 独立评审与准入

四份独立 Sol 报告均绑定 `b895b350d4b4ceda5f2317f2d4b1d8efb42bd11e`，结论均为 **REVISE**。范围分别为需求/类型、Core/文件/权限、编辑/语义消费、功能/Query；没有全量重读所有历史协议，也没有声称产品验收。去重后为 7 项 P1、5 项 P2，无已报 P0。批注英文披露差异由两个 reviewer 发现，只计一项。完整私有报告已收取阅读，以下保留独立可理解的发现及修订入口，不公开私人会话。

| 独立发现 | 具体损害与修订责任 | 当前处置 |
| --- | --- | --- |
| C-P1-01 | 只保护 source 目录仍可把它移入被替换的 target parent；CORE 须给真实祖先/父对象保护闭包及 same-handle 操作 | c53af36 已在该发现范围内独立设计 CLOSED |
| C-P1-02 | 同用户 private staging 不隔离外部 writer，已枚举集合不排除新 child；CORE 须给正常 copy/purge 的可落实算法并保未知新增 | c53af36 已在该发现范围内独立设计 CLOSED |
| C-P1-03 | Field-only 资格缺生产算法，未改隐藏字段的合法性可能决定公开写结果；SOURCE/CORE 须明确静态依赖、公开资格和获权局部校验 | c53af36 已在该发现范围内独立设计 CLOSED |
| P1-REQ-01 | 闭合 schema 无法登记声称已选的关系和 Event 跨字段规则；SOURCE 负责可安装定义、签名、目标域与各消费者 | c53af36 已在该发现范围内独立设计 CLOSED |
| P1-REQ-02 | 保存 Query 只有短名/CEL，清索引后无法恢复原 Field/Type 绑定；SOURCE/FEATURES/CORE 负责持久解析基准与迁移 | c53af36 已在该发现范围内独立设计 CLOSED |
| FRESH-FEATURES-P1-01 | 同事务 A10/B0→A0/B10 可混读成20，分页可能重漏；CORE/FEATURES 负责共同读取快照与冻结结果顺序 | c53af36 已在该发现范围内独立设计 CLOSED |
| P1-EDITOR-01 | 稳定执行 trace 包含测量后丢弃内容及重复布局；EDITOR 负责最终消费选择机制和必要反例证据 | c53af36 最终 Html 路线和有限证据已独立设计 CLOSED |
| P2-REQ-01 | 原生 citation key 没有稳定论文 NodeRef 的可重建来源，改 key/碰撞后可漂移；SOURCE/FEATURES 负责持久引用输入 | c53af36 已在该发现范围内独立设计 CLOSED |
| P2-REQ-02 | 独立可写 birthday 与带来源 birth event 形成两份事实；SOURCE/FEATURES 负责单一作者数据和简洁表单入口 | c53af36 已在该发现范围内独立设计 CLOSED |
| FRESH-FEATURES-P2-01 | 先 EXDATE 再 replacement 可能吞掉已明确改期；SOURCE/FEATURES 负责逐原发生键决策与窗口一致性 | c53af36 已在该发现范围内独立设计 CLOSED |
| FRESH-FEATURES-P2-02 / P2-EDITOR-03 | 英文普通批注读取误交 sourcePreimage，可能泄漏私有 URI；FEATURES 同步为获权 stored visibleExcerpt | c53af36 已在该发现范围内独立设计 CLOSED |
| P2-EDITOR-02 | 原型 runner 只遍历带 elem 对象，深度截断哨兵漏检；诊断完整性须检查全部容器 | c53af36 已独立设计 CLOSED：原 11 份 trace 无截断，真实深层 trace 被新 oracle 拒绝 |

四份后续独立报告均绑定 `c53af36afc674b16dfcde1acbc95c839c37d1348`，对照前一候选，在实际审查范围内关闭上述 12 项去重后的原发现，不将通过外推到未审合同。Core 和编辑报告无新增问题；需求和功能报告合计 4 项新的 P2，无新增 P0/P1。完整报告已收取阅读。Core 独立重跑公开 namespace/common-cut 脚本；编辑独立重跑公开 16 例、额外 7 个输入及真实深层 trace 的 oracle。这些是有限可行性证据，不是产品验收。

| 新发现 | 协调修订 | 当前处置 |
| --- | --- | --- |
| P2-C53-REQ-01 | FEATURES 引用实际已登记 temporal/range，不增加虚构别名。 | d9e63fd 已在本差量范围内独立设计 CLOSED |
| P2-C53-REQ-02 | 两版 paper 样例补 CalendarDate 必需 version。 | d9e63fd 已在本差量范围内独立设计 CLOSED |
| P2-C53-REQ-03 | 保存 Query 的 select 对可选电话 label 加 guard；QOPT01 覆盖有/无 label 混合数据。 | d9e63fd 已在本差量范围内独立设计 CLOSED |
| C53-FEATURES-P2-01 | SOURCE、FEATURES、schema 区分 During 缺端未知与原生 none 明示无界；DUR01–09 覆盖表单/provider/Query，保留 Range 独立规则。 | d9e63fd 已在本差量范围内独立设计 CLOSED |

需求和编辑 reviewer 已在 c53 独立确认五档：纯源码、渲染并始终显示语法、按需显示语法、隐藏语法并通过界面编辑、只读阅读。共享 Draft/Undo、切换不改源、IME/焦点和不支持构造的明确源码入口均保留；此处接受需求的设计保全，不代表五档已实现。

两位非作者 reviewer 已分别完整阅读 `d9e63fd12b0cc33786cccd568ac4729d50da506e` 对照 c53 的十文件差量，包括 schema、During/Range/provider/Cell 消费及编号设计 oracle，均关闭四项 P2，窄范围无新增发现。完整报告已收取阅读，未运行 provider/CEL/UI 产品行为。未改 Core/编辑的有限关闭仍绑定 c53 及实际未受影响合同。冻结重构基线前仍须新的 Pro 全局终审；没有全局接受或产品已实现声明。

评审记录必须写明准确提交、实际阅读范围、发现与证据，不以作者自检、编译成功或旧 CLOSED 自动关闭。检查器检查文档配对、链接和保护输入，不替代语义评审。仅在重大设计缺口已解决、关键选型已有足够证据且独立接受后，才能冻结重构基线；具体顺序见 [INTEGRATION](INTEGRATION.zh-CN.md)。

## 4. 中文默认补充的评审边界

2026-10-13 按用户确认补 REQUIREMENTS §5.5、EDITOR §8.6 和 CORE/FEATURES 消费约定：不逐篇启用中文，区分正文/raw/math 字体与语言，明确离线字体交付、原生覆盖及缺字/导出边界。CJK01–08 是新增产品验收要求，尚未运行；本次没有安装字体、实现 renderer 或验证 Office。既有有限样式实验不覆盖这批场景。

未参与作者的编辑 reviewer 已完整核查固定 `c4a37874114b333442a9fd531e4aff824097061b` 对照 cdb 的十文件双语差量及受影响的五档、模块覆盖/退出、Html/Paged/DOCX、Environment 和资源交付合同，结论为有界 PASS，无新增 P0/P1/P2。完整报告已收取阅读；这只接受该补充的需求保全与接口自洽，不接受 CJK01–08 产品行为。

已完成的全局终审（REVISE、两个OPEN P2，见§5）固定于 `cdb05a3f7c5fe74fe147f34d6f91f7ee0eda0a0d`，不包含本补充；其结论不能自动接受后续差量。本次有界结论仍须与全局裁决合并形成完整设计接受判断，不要求重开已完成且未受影响的阅读。

## 5. cdb 全局终审与成组修复候选

非作者 Pro 全局终审固定对象为 `cdb05a3f7c5fe74fe147f34d6f91f7ee0eda0a0d`，结论 **REVISE**：实际完整当前架构/需求/配置与所列相关历史业务范围内未发现P0/P1，确认以下两个OPEN P2。协调者已收取全文报告；这里保留自足损害、修复入口和证据边界，不公开私人会话或把原报告结论改成作者ACCEPT。中文默认补充的c4a有界PASS仍仅覆盖§4范围。

| 原Pro发现 | 自足问题与本轮修复候选 | 评审状态 |
| --- | --- | --- |
| TYPST-GLOBAL-CDB-P2-01 | During/Range实际仅DateOrInstant，2020年3月开始任职不能保未知日；SOURCE §3.1.1与实际TemporalEndpoint复用CalendarDate，并贯通Task.start/due既有date精度、provider/Query、UI/导出。相同粒度比较与years/months时长可用，混精度有据判断或unavailable，缺端/none差别保留，recurrence精确域不放宽。 | 固定7dcff6c有界CLOSED；原Pro在固定f456独立确认CLOSED（见§6）。 |
| TYPST-GLOBAL-CDB-P2-02 | 一般人物关系role/customLabel未给分类判别/码域，同名friend可漂移；SOURCE §4.1与三真实Field专用Shape明确required relationship的preset/custom分支及各Field闭合集合，守方向/单源，消费者往返保分支，不猜分类。 | 固定7dcff6c有界CLOSED；原Pro在固定f456独立确认CLOSED（见§6）。 |

本轮从干净 `c048626acb89654a4d6c986aa8bd798a3a3a3c53` 作成组候选；没有改snapshots/inputs或撤销中文默认补充。CORE“授权未变化、仅相关数据变化”与FEATURES“venue/container及未定义独立publisher”同步澄清，不自动扩大文献模型。

实际运行：原闭合/路径/ABI参考31检查通过；新增实际Field路径的69个 [有限模型检查](examples/precision-relation-checks.json) 通过；官方0.15.1共23个 [编译例](examples/syntax-experiments.json)，22成功、heading/Bib同键按预期失败，新增两个语义负例仍合法Typst。模型结构往返不是Office，签名参考不是受信Rust安装器；没有运行完整provider/CEL、UI/权限/事务、字体/CJK/Office产品验收，也没有重跑未修改的Html/Core平台探针。全局接受仍须由原Pro核对最终固定候选的修复和中文补充后裁定。

非作者需求 reviewer 已完整核查固定 `7dcff6cdd1c0dcb49576fa5d7977e0a100fe7f9d` 对照 c048 的23文件差量及受影响的真实Field、期间/分类、Query、Task和精确recurrence合同，结合原Pro全文与必要历史业务来源，结论为有界PASS：原两个P2及Task.start/due同族影响面设计CLOSED，无新增P0/P1/P2。完整报告已收取阅读；在私有固定副本独立复跑31配置、69实际Field模型及另23个边界/分类反例均通过，不等同于真实provider/CEL/Office或全局接受。官方Typst编译结果仅按既有实际版本化记录引用，未由该reviewer重跑。

该复核另建议澄清FEATURES的colleague旧句。本次双语明确：任职重叠计算的视图按需派生，显式professional_relations断言独立保留；无engagement不妨碍按真实Field记录，不自动双写或合并来源。此句与既有SOURCE §4.1及原生正例一致，不增加第二权威。最新差量与§4中文默认补充一并交原Pro，不以本有界关闭替代最终裁决。

## 6. f456 全局终审与共同时间基准修复候选

原非作者Pro已完整复核固定 `f4560ddd4f93421db1f6c070889150cbc22e9bdb`，结论 **REVISE**：原两项TYPST-GLOBAL-CDB-P2均独立CLOSED，中文默认补充在设计层接受；无P0/P1，只有新增 **TYPST-GLOBAL-F456-P2-01 OPEN**。报告及证据已完整收取阅读。这不等于全局接受或CJK/Office产品通过。

新增问题是SOURCE §3.1.1、FEATURES §8.3与有限模型只各自验证Range，两个only-start或only-end的civil/instant Range分别有效，却在没有有限跨参数比较时返回overlap=true；交换参数同样错误。原独立70组检查66通过、4失败。粗精度跨历法/版本同侧开放范围也可被洗成true，属于同一根因。

从上述干净基线修订的候选在SOURCE/FEATURES双语与实际模型中先核完整获权输入、有限端点、provider覆盖及共同时间基准，再处理无界和三值合取。共同域不要求端点可证明先后：同历法/版本month/day同侧开放仍true；native date与CalendarDate(day)、受支持的跨历法完整日仍可用。civil/instant混域invalid，粗精度跨历法/版本或缺provider unavailable；不补时区、日期或改变During缺端/none与精确recurrence。schema/ABI形状、签名不变，受信builtin合同的调用级预检落实到算法。

实际有限检查：31配置/路径/设计ABI检查通过；[96项模型](examples/precision-relation-checks.json)通过（原69加27，含SOURCE RDOM01–07）；作者用原独立70组脚本及不改的期望在新候选复跑，70通过；额外8组同族原期望复跑均通过。后两项是作者复跑，不伪称固定f456已通过或独立接受。没有重跑未改的23个原生编译例、Html/Core平台探针；完整Rust provider/CEL、权限/事务/UI、CJK/字体/Office仍UNRUN。

当前新增项仅 **repaired-pending-independent**，由非作者对最终固定差量复核后才能关闭；原两项CLOSED和中文设计接受不重开，不由作者自行宣布全局ACCEPT。
