---
source_language: zh-CN
translation_status: source
---

[English](TASK.md)

# D10：Agent、自动化与外部能力

## 目标和当前状态

状态：candidate。形成一个完整、可实现、可独立审查的设计，定义 Agent 可读取的数据、上下文选择、提案动作、批准、审计和撤权；定义 automation、调度、connector、MCP、model/tool adapter 与 conversion coordination 怎样共享安全边界而不共享第二套作者权威。

当前作者候选采用窄 Broker、typed Capability Catalog 与专用 executors；Core 继续是唯一 author transaction authority。候选同时包含必要的 D6/D7 Standing Approval 配套修订提案，但这些修订在独立接受和协调激活前不生效。

## 固定输入

使用总控给定的固定 Git commit。必须完整读取 ../inputs.json 所列设计输入，并维护实际阅读覆盖。当前作者会话已经完成 48/48 输入阅读；START 记录该事实及曾发生的截断补读。

上游 D1–D9 在协调修订真正接受前持续权威。快照中的旧模型、旧阶段、旧授权与历史 candidate 标签仅作为原文历史，不改变当前任务包的执行边界。

## 约束和问题

Core 是唯一作者事务权威。复用 D3 identity/lifecycle，D6 authorization/transactions/recovery，D7 exact Actions/result/effects，D8 Draft/confirmation，以及 D9 worker/proposal/publication。外部 model output、transcript、secret、credential、Provider provenance、Run 和 tool result 均不能变成作者权威或取得第二写路径。

安装、D4 Registry、D10 Capability Catalog、运行健康和作者内容必须分域。disable、uninstall、failed upgrade、credential rotation 和 provider outage 不删除作者事实。任何 Registry/Catalog 激活必须有明确代际和完整历史，不按安装顺序或 display name 建立 namespace ownership。

Desktop/CLI 可以承载同一个本地 Broker/control domain；Server 承载托管能力；WebUI 只能经 Server 发起/管理。初始 Mobile 没有 Agent、automation、connector/conversion execution 或 credential management，也没有这些能力的批准/委托入口。

read、content egress、workspace mutation、external side effect 和 secret use 是独立授权维度。网页、Document、tool result、MCP descriptor/prompt/resource 和 model text 都是不可信输入，不能扩大 principal、delegation、tool allowlist、egress recipient、network/file/process、secret、budget 或 approval。

必须定义 principals、Delegation Lease、Standing Approval、grant lifetime、revocation、exact-input binding、idempotency、unknown-outcome recovery、cancellation/restart、queue/scheduling/concurrency、audit/retention/export、resource budget 和 cost ceilings。过去一次交互确认不得变成无限期后台授权。

外部效果不能描述成与 Core transaction 原子。要区分 package install、capability availability、permission denial、temporary unavailability 与 non-disclosing diagnostics。MCP 只作为 Tool Adapter transport，不成为 Weftext 权限或身份系统。

## 交付物

作者阶段在 docs/design/d10/ 形成并保持中英文同步的完整候选：

- CANDIDATE.zh-CN.md / CANDIDATE.md
- TERMINOLOGY.zh-CN.md / TERMINOLOGY.md
- SCENARIO-DISPOSITIONS.zh-CN.md / SCENARIO-DISPOSITIONS.md
- IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md / IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md
- UPSTREAM-AMENDMENTS.zh-CN.md / UPSTREAM-AMENDMENTS.md
- 本 TASK 中英文件
- START 中英文件

CANDIDATE 必须自包含最终架构、数据类型、操作合同、状态机、错误与竞争、权限/批准/出站/secret、激活/升级/回滚、Agent/Automation/Connector/MCP、budget/cost/audit 及五端边界。必须比较可信的 A/B/C/D 完整替代并说明选择理由和非目标。

SCENARIO-DISPOSITIONS 必须逐项覆盖 mandatory intake 的 D10 路由和本 TASK 的故障边界，给来源定位、accept/revise/reject/defer-with-owner、执行模式、候选落点和验证义务。

UPSTREAM-AMENDMENTS 必须精确给出必要 D6 主文、D6 Control Interfaces、D7 Execution/Action 的拟新增/替换规范，以及保持不变的 wire/owner/版本边界；不能把作者提案写成已生效上游。

## 作者执行阶段

同一作者会话先完成方案规划，再在同一候选分支和既有 PR 中写入完整候选、进行仓库校验并核实实际提交实物。需要重大设计收敛时可以回到作者规划，但不得因为阶段切换另建候选分支或重复 PR。

作者只修改 docs/design/d10/ 中必要设计材料和既有 PR 的标题/正文。不得修改输入快照、产品实现、brand、仓库权限或分支保护；不得合并、发行或开始 A2。

作者完成条件是：48/48 输入覆盖真实；七对双语文档完整；必要 upstream amendment 明确未激活；diff 只在授权目录；实际文档检查/CI 已读取并如实报告；任何 pending/failure 不被写成 pass。

## 独立审查

完整候选形成固定提交后，交给新的独立 Chat GPT-6 Pro 从零审查。作者自查不算独立 Gate。

独立审查至少要求：完整 accept/pass 判断；P0/P1=0 才可建议协调激活；terminology pass；检查是否存在更简单且完整的替代；检查 D6/D7 amendment 是否必要且充分；验证 mandatory scenarios、依赖闭合与证据边界。

有限 simulation、模型或 CI 不建立产品支持。真实 Core、durable fault、OS sandbox、真实 protocol provider、UI/device 和 release evidence 继续按 Implementation Impact 分层，未完成项必须保留 pending。
