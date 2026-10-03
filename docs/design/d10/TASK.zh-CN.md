---
source_language: zh-CN
translation_status: source
---

[English](TASK.md)

# D10：Agent、自动化与外部能力

## 目标和当前状态

revision: D10-FA-r01-2026-10-02；状态：协调作者候选，未接受、未激活、未实现。最近一次完整历史 R08 评审绑定 C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` 与 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`，结论 REVISE（P0=0、P1=3、P2=8）。十一项历史最终处置仍为 OPEN。具名修订已有限定独立复核，实际跨 owner 整合及 fresh 全局接受仍未完成；REVIEW-DISPOSITIONS 区分各层证据。

当前作者候选采用窄 Broker、typed Capability Catalog 与专用 executors；Core 继续是唯一 author transaction authority。候选同时包含必要的 D6/D7 Standing Approval 配套修订提案，但这些修订在独立接受和协调激活前不生效。

## 固定输入

使用总控给定的固定 Git commit，并按 actor/lineage 分开记录阅读证据，不能合并继承。原作者 lineage 历史完成 S49/49；历史 R08 接续作者亲自完成 S16/49 全文，D9 workers/export 与 templates 只做 dependency-scoped 局部读取，不计全文；固定 R07 的独立联合评审另行完成 S49/49。三套覆盖互不替代，workflow/tool summary 不计全文；START 记录准确来源。

上游 D1–D9 在协调修订真正接受前持续权威。快照中的旧模型、旧阶段、旧授权与历史 candidate 标签仅作为原文历史，不改变当前任务包的执行边界。

## 约束和问题

Core 是唯一作者事务权威。复用 D3 identity/lifecycle，D6 authorization/transactions/recovery，D7 exact Actions/result/effects，D8 Draft/confirmation，以及 D9 worker/proposal/publication。外部 model output、transcript、secret、credential、Provider provenance、Run 和 tool result 均不能变成作者权威或取得第二写路径。

安装、D4 Registry、D10 Capability Catalog、运行健康和作者内容必须分域。disable、uninstall、failed upgrade、credential rotation 和 provider outage 不删除作者事实。任何 Registry/Catalog 激活必须有明确代际和完整历史，不按安装顺序或 display name 建立 namespace ownership。

Desktop/CLI 可以承载同一个本地 Broker/control domain；Server 承载托管能力；WebUI 只能经 Server 发起/管理。初始 Mobile 没有 Agent、automation、connector/conversion execution 或 credential management，也没有这些能力的批准/委托入口。

读取、内容出站、工作区修改、外部副作用和 secret 使用是独立授权维度。网页、Document、工具结果、MCP 描述/提示/resource 和模型文本都是不可信输入，不能扩大 principal、delegation、工具 allowlist、出站 recipient、网络/文件/进程、secret、budget 或 approval。

必须定义 principal、Delegation Lease、Standing Approval、授权寿命、撤销、精确输入绑定、幂等、未知结果恢复、取消/重启、队列/调度/并发、audit/retention/export、资源预算和费用上限。过去一次交互确认不得变成无限期后台授权。

外部效果不能描述成与 Core transaction 原子。要区分 package install、capability availability、permission denial、temporary unavailability 与 non-disclosing diagnostics。MCP 只作为 Tool Adapter transport，不成为 Weftext 权限或身份系统。

## 交付物

作者阶段在 docs/design/d10/ 形成并保持中英文同步的完整候选：

- CANDIDATE.zh-CN.md / CANDIDATE.md
- TERMINOLOGY.zh-CN.md / TERMINOLOGY.md
- SCENARIO-DISPOSITIONS.zh-CN.md / SCENARIO-DISPOSITIONS.md
- IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md / IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md
- UPSTREAM-AMENDMENTS.zh-CN.md / UPSTREAM-AMENDMENTS.md
- CONTROL-CONTRACT.zh-CN.md / CONTROL-CONTRACT.md
- REVIEW-DISPOSITIONS.zh-CN.md / REVIEW-DISPOSITIONS.md
- TASK.zh-CN.md / TASK.md
- START.zh-CN.md / START.md

CANDIDATE 必须自包含最终架构、数据类型、操作合同、状态机、错误与竞争、权限/批准/出站/secret、激活/升级/回滚、Agent/Automation/Connector/MCP、budget/cost/audit 及五端边界。必须比较可信的 A/B/C/D 完整替代并说明选择理由和非目标。

SCENARIO-DISPOSITIONS 必须逐项覆盖 mandatory intake 的 D10 路由和本 TASK 的故障边界，给来源定位、accept/revise/reject/defer-with-owner、执行模式、候选落点和验证义务。

UPSTREAM-AMENDMENTS 必须精确给出必要 D6 主文、D6 Control Interfaces、D7 Execution/Action 的拟新增/替换规范，以及保持不变的 wire/owner/版本边界；不能把作者提案写成已生效上游。

## 作者执行阶段

当前设计工作将本目录与 `../d6-file-authority-reopen/` 中显式 owner 后像及唯一路由协调。固定 S 快照保持原字节；每份正文只有一位指定作者，创作与独立审查分工保持。候选不代表产品实现、合并、发布或激活；设计冻结之前仍须 A2 整合及全新普通 Chat Pro 全局终审。

完成条件是九对双语 D10／18 个准确路径与全部必要 D1–D9 实际 owner 后像在同一不可变候选上一致；固定 S49 及其清单保持不变；既有125个场景 ID 与完整义务可追溯，新增案例明确列出。适用文档/输入检查、真实阅读覆盖、术语、中英语义以及历史/当前发现都绑定该候选。旧 S49/49 覆盖、具名差分 PASS 和 CI 均不能继承为全文接受。设计冻结要求零开放 P0/P1、剩余 P2 明确处置以及 fresh 独立全局 Pro 接受，仍不等于实现或发布。

## 当前分批修订约束

本候选经过分批独立审查后继续由同一作者修订；作者修订不改变“独立 Gate 仍未完成”的状态。当前需要保持以下明确合同：

- Delegation Lease 的 `maxRuns` 在 Run 第一次获准进入受保护执行的原子准入点消费一次；此前取消不消费，此后失败、取消、崩溃不退款，同 Run 恢复不重复消费。
- 同一 Automation occurrence 的 claim、Run identity 与 terminal proof 必须耐久防重；restart、rescan、disable→enable 和缓存重建不得另起 Run。
- Lease 是否过期由可信当前时间决定，不依赖 cleanup task；时间连续性不可证明时 fail closed，不能假定未过期。
- Standing Approval 进入正式 D6 后发生竞争失效，必须按 UPSTREAM-AMENDMENTS 提出的 D6 `approval_unavailable/preflight` 表达；不能伪装成 D10 前置 approval error，也不能保存永久 semantic rejection。
- planned decision 的原 preview transport 过期后，必须通过 UPSTREAM-AMENDMENTS 的 planned-preview 只读恢复入口重新签发有限交付 epoch；不得 reprepare、换 OperationId、重新 Query 或换 target。
- authoritative `terminal_failed` 可以在同一 abort transaction 释放 approval count reservation，但取消、TTL、临时撤权和 Lease 过期不能释放；费用 reservation 另按 settled/released/uncertain 裁决。
- raw no-op 与实际 member change 都属于首版 single_field_member 自动 profile，但 no-op 的 MutationFootprint、field_change、sourceVersions 必须保持为空，不能伪造效果。
- 费用 `released` 只表示能证明收费执行从未开始；实际发送且最终账单为零必须 `settled(0)`。

这些条款在 D6/D7 配套修订共同接受前仍只是候选提案，不改变当前上游权威。

R08 还冻结以下已经由 CONTROL/CANDIDATE/UPSTREAM/SCENARIO/IMPLEMENTATION 拥有的作者阶段边界：

- 无人值守作者路径只能使用具名第一方 Core field-member adapter 与 closed `FieldMemberTask/1`；task intent 与 Standing Approval 分域，任意 ToolValue 绝不能成为 NodeRef/FieldId/selector/author request；
- control history 内部保留完整 canonical B 防重，public history 只返回七-kind summary，绝不返回 A/B/M；
- external consent 绑定稳定 effect Ref 与 requestDigest；生命周期 revision 与不可变请求语义分域；public current view 不披露请求 payload、幂等 key、secret 或 reservation identity；
- emergency stop 是专用同库 D10 safety transaction，使用 exact-target 防重、预留容量、receipt/result replay，不走普通 prepare，也不建第二 author ledger；
- D8 13 kind 与 D9 17 public kind 各有一个 technical-interface owner；consumes/returns/operates-on 数据继续保留原 domain owner，包括 D6 ImportJob。

## 独立审查

完整候选形成固定提交后，交给新的独立 Chat GPT-6 Pro 从零审查。作者自查不算独立 Gate。

历史 C=`35fab950dabedfb92c9f12858701be8afe6faa74` 与固定 R06 C=`32a0868ae9443a3f839cfb4f5e9bbcace308314d` 继续只作为历史 REVISE 记录。固定 R07 C=`cf46461848d5dfe4dcd0f482ede934243cd098a4` 的完整独立联合终审完成候选18/18、S49/49，结论 REVISE：P0=0、P1=1（`B03-P1-01`）以及 REVIEW-DISPOSITIONS 中记录的十项 P2；整体、术语与中英语义均需修订。该完整 R07 review 只为作者修订提供证据，不代表 R08 已接受。

完成条件是九对双语 D10／18 个准确路径与全部必要 D1–D9 实际 owner 后像在同一不可变候选上一致；固定 S49 及其清单保持不变；既有125个场景 ID 与完整义务可追溯，新增案例明确列出。适用文档/输入检查、真实阅读覆盖、术语、中英语义以及历史/当前发现都绑定该候选。旧 S49/49 覆盖、具名差分 PASS 和 CI 均不能继承为全文接受。设计冻结要求零开放 P0/P1、剩余 P2 明确处置以及 fresh 独立全局 Pro 接受，仍不等于实现或发布。

有限 simulation、模型或 CI 不建立产品支持。真实 Core、durable fault、OS sandbox、真实 protocol provider、UI/device 和 release evidence 继续按 Implementation Impact 分层，未完成项必须保留 pending。
