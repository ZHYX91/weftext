---
source_language: zh-CN
translation_status: source
---

[English](START.md)

# D10 启动阅读检查点

状态：candidate-preparation。本文件只记录 D10 第一批实际阅读、边界和待比较结构；不是完整 D10 候选，不代表设计接受、实现完成或发布许可。

固定输入提交：`f205831c848729f7ddbc3ba0cf32b689459c0c98`。作者分支：`docs/d10-start`。后续 Draft PR 目标分支：`docs/chat-collaboration`。快照中的旧模型、阶段和授权文字只作为历史输入保留；当前 D10 任务包决定本次执行方式。

## 已完成阅读

以下 11 个文件已经从固定输入提交直接读取到文件末尾，没有以搜索摘要替代正文：

- `AGENTS.zh-CN.md`
- `docs/DOCUMENTATION.zh-CN.md`
- `docs/design/AGENTS.zh-CN.md`
- `docs/design/README.zh-CN.md`
- `docs/design/inputs.json`
- `docs/design/d10/TASK.zh-CN.md`
- `scripts/check_docs.py`
- `docs/design/snapshots/d1-product-surface-and-capability-boundary/source.md`
- `docs/design/snapshots/d1-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d4-d10-a2-mandatory-scenario-inputs-2026-08-28/source.md`
- `docs/design/snapshots/d9-conversion-templates-and-workers/source.md`

其中后四项属于 48 份设计输入清单。本批因此实际完整读取 4/48 份设计输入；强制场景 intake 已分段覆盖至末尾。一次较大分段返回曾被工具截断，该范围随后以更小分段重新完整覆盖，截断结果没有被计入已读证据。

## 尚未读取的设计输入

以下 44 份是 `docs/design/inputs.json` 中尚未完整读取的输入。完整 D10 候选形成前不得把它们视为已覆盖：

- `docs/design/snapshots/d2-document-content-and-domain-objects/source.md`
- `docs/design/snapshots/d3-identity-references-ownership-and-lifecycle/source.md`
- `docs/design/snapshots/d3-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d4-attribute-types-schema-and-relations/source.md`
- `docs/design/snapshots/d4-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d5-tables-and-node-collections/source.md`
- `docs/design/snapshots/d5-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d6-control-interfaces/source.md`
- `docs/design/snapshots/d6-storage-transactions-permissions-and-sync/source.md`
- `docs/design/snapshots/d6-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d6-terminology-registry/source.json`
- `docs/design/snapshots/d7-definition-transfer/source.md`
- `docs/design/snapshots/d7-execution-and-action-interfaces/source.md`
- `docs/design/snapshots/d7-narrow-field-qualification/source.md`
- `docs/design/snapshots/d7-prepared-action-binding/source.md`
- `docs/design/snapshots/d7-preview-and-effects-transport/source.md`
- `docs/design/snapshots/d7-query-algebra/source.md`
- `docs/design/snapshots/d7-query-view-action/source.md`
- `docs/design/snapshots/d7-scenario-dispositions/source.md`
- `docs/design/snapshots/d7-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d7-terminology-registry/source.json`
- `docs/design/snapshots/d7-value-and-cel-profile/source.md`
- `docs/design/snapshots/d7-view-contract/source.md`
- `docs/design/snapshots/d8-acceptance-matrix/source.json`
- `docs/design/snapshots/d8-acceptance-matrix/source.md`
- `docs/design/snapshots/d8-direction-and-accessibility/source.md`
- `docs/design/snapshots/d8-editor-and-cross-surface-interaction/source.md`
- `docs/design/snapshots/d8-editor-interfaces/source.md`
- `docs/design/snapshots/d8-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d9-acceptance-matrix/source.md`
- `docs/design/snapshots/d9-coordinated-d3-d7-binding-amendment/source.md`
- `docs/design/snapshots/d9-import-ir-and-mapping/source.md`
- `docs/design/snapshots/d9-templates/source.md`
- `docs/design/snapshots/d9-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d9-workers-and-export/source.md`
- `docs/design/snapshots/d2-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d3-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d4-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d5-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d6-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d7-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d8-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d9-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d8-rtl-and-bidirectional-interaction-mandatory-review-intake-2026-09-01/source.md`

## D10 范围

D10 要定义 Agent、自动化与外部能力怎样在不产生第二写入权威的前提下工作。范围至少包括 Agent 可读取的数据和上下文选择、建议动作、批准、撤权、审计与恢复；自动化的调度、队列、重复与取消；连接器的外部协议、凭据和同步控制状态；Extension Package、manifest、contribution、runtime、namespace 注册和 provider 生命周期；以及这些能力与 D9 转换协调器共享而不混淆的授权、隔离、预算、审计和外部副作用边界。

Desktop 与 CLI 可承载本地 broker/control plane，Server 可承载托管能力，WebUI 只能经 Server 发起或管理。Mobile 首期不运行 Agent、自动化、连接器或转换执行，也不管理这些能力的凭据；它只能消费合法提交后的普通结果。

本批不定义产品实现，不修改 D1–D9 快照，不把任何候选 package、provider、manifest 或 runtime 语法冻结成兼容合同。

## 已识别的硬约束

- Core 是唯一作者事务权威。Agent、自动化、连接器、模型、网页、工具输出、transcript、provider 对象和运行缓存都不能成为作者事实或第二条工作区写路径。
- 本地与托管模式必须沿用 D1 的唯一提交持有者和能力探测边界；同一能力在产品端之间不能由 UI 或调用端自行改写语义。
- D3 的身份、owner、locator、来源与生命周期边界必须复用；外部 provider ID、cursor、etag、UID 或名称不能替代 Weftext identity。
- D6 的授权、预算、事务、恢复和信息不披露必须是最终工作区修改的门。旧 capability 探测、旧交互同意或后台定时器都不是持续授权票据。
- D7 的 Action、准备、精确输入绑定、preview/effects、结果消费和 unknown recovery 必须复用；D10 不建立另一个通用提交协议。
- D8 的 Draft、确认和跨产品端交互边界必须复用；自动或模型生成的提案不能伪称用户原生输入。
- D9 的 worker、隔离输入、提案、发布和外部输出边界保持权威。D10 可复用其隔离与外部副作用经验，但不能把转换 worker 改造成通用作者。
- 用户读取、内容出站、工作区修改和外部副作用是不同授权维度；一个维度的许可不能隐式扩大到另一个维度。
- 网页、文档、工具返回和模型文本全部是不可信输入，不能扩大主体、范围、网络、文件、进程、secret 或写入权限；提示注入必须在能力与数据边界上默认拒绝。
- 上下文选择遵循最小必要和数据最小化；secret 使用句柄或受控引用，不进入工作区内容、模型上下文、普通日志或可导出 transcript。
- 定时和后台执行必须有明确主体、能力范围、授权寿命、撤权和重验。一次交互批准不能自动变成无限期后台授权。
- 外部副作用与 Core 事务不能假装原子。设计必须覆盖幂等键、结果未知、重试、补偿或人工恢复、取消竞争和崩溃重启。
- 安装、registry 定义、provider 可用性和作者内容互相正交。禁用、卸载、版本不兼容或升级失败不得删除、默认化或改写作者事实。
- Extension、pack、connector、adapter、provider、module、contribution 与 runtime 的术语必须通过横切术语门；官方 namespace、publisher namespace 和用户自定义 namespace 需要 anti-spoof 与冲突规则。
- 运行环境必须覆盖网络、文件、进程与资源隔离，来源和依赖验证，更新与回滚，资源预算与费用上限，审计失败，以及权限变化后的缓存读取。
- capability 不可用、权限不足、组件缺失、未配置、离线、版本不兼容和暂时不可用必须保持可诊断但不泄露被遮蔽部署信息。

## 建议候选文档结构

完整候选可按以下逻辑组织，实际文件拆分在完整阅读后再决定：

1. 范围、非目标、上游合同与总体不变量。
2. Terminology、package/contribution taxonomy、namespace 与 registry。
3. 主体、委托、capability、授权寿命、撤权和精确输入绑定。
4. Agent 上下文选择、工具调用、提案、批准与结果消费。
5. 自动化调度、队列、并发、重复、取消、重启与授权重验。
6. 连接器、provider、secret、同步控制状态与外部副作用。
7. runtime 隔离、网络/文件/进程边界、依赖来源、更新和回滚。
8. Desktop/CLI 本地控制面、Server 托管控制面、WebUI 管理入口和 Mobile 不可用矩阵。
9. 状态机、幂等、unknown recovery、审计、保留、导出、预算与费用。
10. 与 D9 worker/转换发布、D7 Action、D8 确认的接口组合。
11. 强制场景 disposition、恶意输入与故障注入、跨产品端一致性。
12. 上游修订清单、实现影响、测试轮廓、待验证证据和接受条件。

## 关键开放决策与竞争性顶层方案

当前不冻结任何方案。至少保留以下三种完整顶层方案进入后续比较：

- 方案 A：共享 broker/control plane，加专用 Agent runtime、automation scheduler 和 connector adapters。公共层只拥有主体、授权、capability、secret 引用、审计、预算、job 生命周期和外部副作用恢复；各领域运行器保留各自封闭协议。需要证明公共层足够统一，又不会变成开放的万能工具系统。
- 方案 B：统一 capability host，以类型化 contributions 驱动 Agent、自动化、连接器和部分外部执行。manifest 声明输入、输出、权限、sandbox、网络和副作用类别，host 统一调度与审计。需要证明 closed schema、最小权限、升级兼容和 provider 隔离不会被开放 contribution blob 破坏。
- 方案 C：Agent、自动化和连接器维持独立控制子系统，只共享 D6 授权、D7 Action/结果、统一审计标识与少量 capability vocabulary。需要证明重复状态机和诊断不会造成语义漂移，同时避免为了统一而引入过大的公共 runtime。

必须进一步比较的开放问题包括：交互 Agent session 与后台 automation 的主体/委托关系；一次性批准、会话批准、策略批准和定时授权的边界；读取上下文与内容出站是否需要不同 grant；外部副作用结果未知后的恢复模型；connector sync 与一次性 Action 的共用/分离程度；package 安装信任与运行时授权是否分层；registry 定义与已安装 provider 版本如何绑定；审计失败时哪些操作必须阻断；本地与 Server 是否共享一套 wire 还是只共享语义对象；以及跨 capability 的预算、费用与并发仲裁由谁拥有。

## 下一批完整阅读顺序

下一批继续只读，并维持已读/未读清单：

1. D2 主文与 D2 实施影响，先固定 Document、作者源、领域对象和 Task/Template 边界。
2. D3 主文、D3 术语与 D3 实施影响，固定 identity、ownership、provenance、origin 和生命周期词义。
3. D4 主文、D4 术语与 D4 实施影响，固定 typed value、schema/field/relation、namespace 与 registry 相关上游语义。
4. D5 主文、D5 术语与 D5 实施影响，固定 Record、collection、occurrence 与规模边界。
5. D6 两份主接口、术语、registry 与实施影响，重点读取主体、权限、事务、审计、预算、恢复、控制状态和缓存非披露。
6. D7 全部 Action/Query/View/结果消费接口、术语、registry 与实施影响，重点闭合准备、确认、unknown、effects 和 capability contribution 的接口可复用范围。
7. D8 主文、editor interfaces、direction/accessibility、acceptance matrix、术语、实施影响与 RTL intake，固定 Agent/自动化提案如何进入 Draft、确认、可访问交互和五端一致性。
8. 最后补齐 D9 其余 acceptance、binding amendment、IR/mapping、templates、workers/export、术语与实施影响，再开始完整 D10 候选写作。

完整输入未覆盖前，只允许继续扩展候选结构、问题清单和反例；不声明设计门通过，不冻结顶层选择，也不据当前阅读修改任何上游快照。
