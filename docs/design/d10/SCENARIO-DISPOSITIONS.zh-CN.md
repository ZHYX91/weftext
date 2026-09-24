---
source_language: zh-CN
translation_status: source
---

[English](SCENARIO-DISPOSITIONS.md)

# D10 场景裁决

revision: D10-r01-candidate-2026-09-25；状态：candidate。本文件把固定上游输入中的 D10 路由、TASK 强制故障和本候选新增竞争边界逐项落到可执行裁决。disposition 只评价 D10 候选如何承接该场景，不表示测试已经通过。

执行模式列只有四类：automatic 表示在本文 closed 规则下允许无需逐次人工确认继续；interactive 表示可以准备但必须逐次确认；unsupported 表示本代明确不可用；deferred 表示由具名 owner 的未来合同冻结后才能开放。

## 1. 上游强制路由与领域边界

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-U01 | D1 §2–§4，D1-I01/I06 | Agent、automation、connector 直接写工作区或与 Core 同时写 | reject | unsupported | Core 仍为唯一 author commit point；executor 无 author store/workspace mount | 依赖图、文件/DB 句柄负向、双 writer/fence 测试 |
| D10-U02 | D1 §4.1/4.4 | Desktop 与 CLI 各启动独立 scheduler 导致同一 occurrence 重复 | revise | automatic | 同一 local control domain 与唯一 occurrence claim；CLI 连接同一能力族 | 双进程竞争、崩溃接管、same key 仅一个 active Run |
| D10-U03 | D1 §4.2/4.3 | WebUI 直接跑 model/connector 或浏览器持 credential | reject | unsupported | WebUI 只经 Server Broker；secret 留 Server secret store | 浏览器 bundle/API 扫描、credential 泄漏负向 |
| D10-U04 | D1 §4.5，D8 Direction §7 | Mobile Agent、automation approval、conversion/connector 管理 | reject | unsupported | 保留 D1 unsupported_surface；只消费 committed facts | 五端 capability fixture，Mobile 不出现隐藏批准入口 |
| D10-U05 | Mandatory Intake §2.4/§2.5 与 D4 Calendar | Calendar recurrence/后台提醒从设备时钟或未绑定规则猜测 | revise | automatic | Automation schedule 使用已验证 D4 temporal rules、有限 horizon/limit 与 exact source dependency | tzdb/rule generation 改变、DST/超域、restart 反例 |
| D10-U06 | Mandatory Intake §2.6 Organizations | organization provider ID/服务名变成 Node identity | reject | unsupported | Connector ID 留控制域；作者关系仍 D3/D4 identity | 同名/改名/provider ID 重用不得合并 Node |
| D10-U07 | Mandatory Intake §2.7–§2.8 Calendar packs | pack 安装即改变作者字段或 provider 失效删除事实 | reject | unsupported | 激活只改变 Registry/Catalog availability；raw source 与 semantic ledger 保留 | disable/uninstall/failed update 后作者 bytes 不变 |
| D10-U08 | Mandatory Intake §9 A2-08 | connector sync 把 cursor/credential 写入作者 source | reject | unsupported | cursor/etag/credential 是控制状态；写回需具名 closed adapter | author source 搜索无 secret/cursor；commit/control 原子关联 |
| D10-U09 | Mandatory Intake §9 A2-09/A2-15 | 各 surface 对同 capability 自己猜是否可用 | reject | unsupported | D1 capability 为唯一产品可用性；D10 只提供 downstream facts | Desktop/CLI/Server/WebUI 相同 reason precedence |
| D10-U10 | Mandatory Intake §9 A2-21 | schema、connector、runtime state 合并为一个 provider 状态 | reject | unsupported | D4 Registry、D10 Catalog、runtime health 三域分离 | schema available/runtime unavailable 与反向组合 ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-U11 | Mandatory Intake §9 A2-28 | holiday/workday provider 变更后旧派生结果继续有效 | revise | automatic | rule/contribution generation 进入依赖；失效 reset/recompute | 旧 result/cache/automation schedule 不能混 generation |
| D10-U12 | Mandatory Intake §9 A2-32 | ICS UID/RECURRENCE-ID 自动成为 D3 identity/upsert key | reject | unsupported | 只经已冻结 SourceBinding/OriginBinding adapter 才可 lookup/upsert | 相同 UID 两次 ordinary import 保持 fresh，未开放 sync 明确 unavailable |
| D10-U13 | Mandatory Intake §9 A2-33 | subscribe、sync、copy、adopt 混成同一个“同步”动作 | reject | unsupported | 每个 authority/effect 走其 owner closed protocol | 不同 intent 的 request/receipt/approval 不互用 |
| D10-U14 | Mandatory Intake §9 A2-36 | package 获得 whole-package 工作区权限 | reject | unsupported | 权限按 Contribution，五类授权维度分开 | 同包纯数据可用、network connector denied 的组合 |
| D10-U15 | Mandatory Intake §9 A2-37 | Settings/Marketplace UI 状态成为 capability authority | reject | unsupported | UI 只是 Activation/Policy/Catalog 的投影 | 隐藏/显示按钮不改变 Core eligibility |
| D10-U16 | Mandatory Intake §9 A2-43 | localized module/manifest 名改变 canonical ID | reject | unsupported | canonical namespace/contribution ID 独立 locale | 中英/RTL 切换 request bytes 不变 ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-U17 | Mandatory Intake §9 A2-46 | Mobile 上传文件后自动委托 Server 转换/Agent | reject | unsupported | Mobile 只能普通附件；无转换/Agent 委托/审批 | 上传不触发 worker/model，返回 unsupported_surface |
| D10-U18 | Mandatory Intake §9 A2-49 | 未接纳 contribution 动态注入 SearchContribution/schema | reject | unsupported | D10 认证并 generation-bind Contribution；D7 grammar 不变 | runtime discovery 新 contribution 只 pending，不进入 current query |
| D10-U19 | Mandatory Intake §9 A2-50 | 禁用规则 pack 后把作者值删除/默认化 | reject | unsupported | 相关派生能力 unavailable/reset，作者 source 保留 | disable/reenable 与 raw source digest 不变 |
| D10-U20 | Mandatory Intake §9 A2-53/54 | D10 新术语覆盖 D1–D9 owned name 或 wire alias 漂移 | revise | automatic | Terminology 文件与 controlled-name gate | 受控 positive surface 扫描；历史 prose 排除 |
| D10-U21 | Mandatory Intake §9 A2-56，D9 Templates | Node/Office Template 被当作长期 Agent 脚本或任意 callback | reject | unsupported | Template 仍 D9 一次性构造/渲染；D10 不执行其普通文字 | template text 含 tool 指令只作不可信文字 |
| D10-U22 | D7 Algebra §6 SearchContribution | SearchContribution 内带脚本/network 或 provider unavailable 时静默跳项 | reject | unsupported | 保持 D7 纯数据 closed descriptor；D10 只认证来源/generation | 缺 contribution 整体按原 D7 unavailable，不改 grammar |
| D10-U23 | D7 Execution §5–§6 | Query row/evidence 被 Agent 当长期写授权 | reject | unsupported | evidence/selector 仍 TTL/revision/dependency/current auth-bound | A→B→A、auth change、result reset 后旧 evidence 失败 ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-U24 | D8 Main §2–§3/Interfaces §4–§5 | model 输出伪装成人类 Draft、自动点击确认 | reject | unsupported | D8 edit 仍 interactive；Automation 不创建 EditSession | dirty Draft、composition、stale preview、unknown receipt 全链 |
| D10-U25 | D9 Workers §1–§2 | 把 D9 conversion worker 变成有网络的通用工具 host | reject | unsupported | D9 worker 继续专用无 workspace/network 默认；D10 executor 独立 | worker sandbox 不因 D10 网络能力扩大 |
| D10-U26 | D9 Export §4 | external publication receipt 当作 Resource/author receipt | reject | unsupported | PublicationReceipt 与 D3/D6 receipt 分域 | publish success + Resource create fail 两结果并存 ；并核对本行边界不会扩大权限、身份或作者写入语义 |

## 2. TASK 强制故障边界

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-F01 | TASK Constraints/Acceptance | 恶意 Document 要求把其它 workspace/secret 上传 | accept | automatic | untrusted-data precedence、ContextBundle 最小化、recipient-specific egress | 两个隐藏世界相同 tool catalog；无 secret/extra read |
| D10-F02 | TASK | prompt injection 要求启用新 MCP tool 或扩大 budget | accept | unsupported | remote descriptor/output 无控制权；allowlist/budget 由受管记录 | 注入文本与控制记录 diff 必须为零 |
| D10-F03 | TASK | grant/Delegation Lease 到期后 queued Run 继续 | accept | automatic | 每 protected step current lease check；queued→blocked | 到期前后线性化、restart 不延长期限 |
| D10-F04 | TASK | 撤权后旧 context/cache/result 继续交付 | accept | automatic | current D6 generation/owner gate；各上游 cache 规则保持 | revocation 与 chunk/page/tool-call 竞争 |
| D10-F05 | TASK | 同一 schedule occurrence 重复 claim | accept | automatic | `AutomationOccurrenceKey/1` + durable unique claim | 两进程/重启/enable-disable race ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-F06 | TASK | crash 时 external request 可能已发 | accept | automatic | durable ExternalEffectIntent + send fence；恢复 outcome_unknown | durable-before-send、send-before-response 两侧 fault ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-F07 | TASK | cancel 与 D6 planning 竞争 | accept | automatic | 已 planned 不被 Run cancel 冒充 abort；按原 D6 recovery | cancel-before-plan、plan-before-cancel、commit-before-cancel ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-F08 | TASK | cancel 与 external send 竞争 | accept | automatic | 仅证明未 send 才 cancelled；submitting 后三态 | send fence 两侧 fault injection |
| D10-F09 | TASK | credential rotation 后旧 request 自动用新 credential 重发 | reject | unsupported | old attempt 绑定实际 secretGeneration；新 credential 仅可受权 reconcile | rotation + unknown request；禁止 mutation resend |
| D10-F10 | TASK | failed package upgrade 半激活 Registry/Catalog | accept | automatic | staged validation + one ActivationBinding switch | 每个激活步骤 crash；旧 binding 完整存活 ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-F11 | TASK | 已激活版本回滚时 Registry 指针倒退 | reject | unsupported | rollback 是 successor activation；semantic ledger 累计 | 三代 schema/history/revival attack |
| D10-F12 | TASK | audit collector offline 就停止全部本地工作 | revise | automatic | local durable spool 是安全门；remote collector 可延迟 | collector offline、spool full、disk failure 分支 |
| D10-F13 | TASK | local durable audit 写失败仍执行 protected step | reject | unsupported | protected step fail closed；安全停止有 reserve | audit failure before read/egress/secret/send/author submit ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-F14 | TASK | Desktop/CLI/Server/WebUI 对相同请求得不同目标/错误 | accept | automatic | shared Core semantics；host 只运输/认证 | identical fixture 跨四端；Mobile 为负能力 |
| D10-F15 | TASK | capability probe 泄露 package 未安装/账户状态给 denied principal | accept | automatic | D1 reason precedence，policy_denied 先遮蔽 deployment detail | 重叠 reason 矩阵 |
| D10-F16 | TASK | external effect 与 Core write 被展示为一个“原子成功” | reject | unsupported | 两个独立 outcome/receipt；无 composite author receipt | Core success/external unknown 与反向组合 |
| D10-F17 | TASK | unknown external result 换新 idempotency key 自动重试 | reject | unsupported | 原 EffectIntent/key；无可靠 reconcile 则停止 | timeout、eventual consistency、window expiry ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-F18 | TASK | idempotent effect 被认为 retry 不收费 | reject | unsupported | 每 attempt 独立 CostReservation | 重试费用、billing delay、unknown charge |
| D10-F19 | TASK | 并发 Runs 都消费 Standing Approval 最后一次 | accept | automatic | ApprovalUse count reservation 与 D6 planning CAS | N=1 双并发，至多一个 reservation/commit |
| D10-F20 | TASK | 并发 Runs 都消费最后一笔费用预算 | accept | automatic | 多账户原子 CostReservation | Run/Lease/Automation/deployment 四账户竞争 |
| D10-F21 | TASK | billing unknown 后 TTL 释放额度 | reject | unsupported | CostReservation=uncertain 持续占上限 | crash/timeout/restart/retention ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-F22 | TASK | provider 超过已接纳价格上限仍静默继续 | revise | automatic | 记录 anomaly、冻结 capability、要求管理处理 | simulated overcharge，不自动提高 ceiling |

## 3. Standing Approval 与确认边界

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-A01 | D7 Execution §5–§6 + D10 Candidate §14–§15 | 单 Node/Field 恰一 Entry 的 bool member 更新，值在有限集合内 | revise | automatic | fresh `set_field_member` prepare +完整 owner_fields preview + ApprovalUse | 真实 D7 Narrow Field/preview/D6 commit 集成 |
| D10-A02 | 同上 | 同一 Field 两个 Entry 值完全相同 | accept | interactive | `require_exactly_one_entry` 失败，不自动 first | 两同值 occurrenceKey 反例 |
| D10-A03 | 同上 | Field 从一 Entry 并发变两 Entry 后消费旧批准 | accept | interactive | fresh selection/source revision + dependency conflict | prepare/approve/commit 三阶段 race |
| D10-A04 | 同上 | proposed change 同时改变 note/provenance/Facet/关系 | reject | unsupported | footprint 必须仅一个 member | mutant footprint 必须拒绝 |
| D10-A05 | 同上 | append/remove/replace whole Entry 试图使用 standing envelope | reject | interactive | 首版无人值守 profile 不覆盖；可重新走交互 D7 | action-kind negative matrix |
| D10-A06 | D8 Interfaces §4–§5 | 自动整源 document edit | accept | interactive | 仍走 D8 Draft/preview/explicit confirmation | Agent 不创建 fake EditSession ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-A07 | D3/D7 create/lifecycle | Agent 自动创建/Trash/restore/copy Node | accept | interactive | 原 D3/D7 preview + 逐次确认 | fresh identity、closed modes、不消费 standing envelope ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-A08 | proposed D6/D7 amendment | standing approval 已过期但 request 已 committed，用户重放 receipt | accept | automatic | saved decision 在 current auth 下重放原 bytes，不重复扣 approval | lost receipt after approval expiry ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-A09 | proposed D6/D7 amendment | request 已 planned 后 approval 被撤销 | revise | interactive | 保持 planned/blocked；可对 exact 原 plan 新增一次性授权，不换 request | 不 burn、不自动换 envelope |
| D10-A10 | proposed D6/D7 amendment | raw no-op author decision | accept | automatic | committed no-op 仍消耗一次 successful commit count | replay 不重复计数 ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-A11 | D8 Main §3 | 后台合法更新与当前 dirty Draft 同 owner | accept | automatic | author commit 有效；D8 Draft 进入 stale/conflict，不被覆盖 | current Draft bytes/selection 保持 |
| D10-A12 | D8 Main §3 | composition 期间 Agent proposal 自动提交 | reject | interactive | D8 composition/preview gate 保持 | composition trace + delayed proposal ；并核对本行边界不会扩大权限、身份或作者写入语义 |

## 4. MCP、tool、secret 与出站

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-T01 | D10 Candidate §10 | MCP server 自称 delete tool 为 readOnly | reject | interactive | remote annotation 不授 effect class；本地 Contribution 决定 | hostile MCP descriptor fixture ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-T02 | D10 Candidate §10 | MCP runtime 新发现一个 tool/schema | accept | deferred | 只进入 pending admission；current allowlist 不变 | discovery diff 不改变可调用目录 |
| D10-T03 | D10 Candidate §10 | remote JSON 把 2^63+1 经 double 舍入 | reject | unsupported | ToolValue exact integer；无法精确适配即 unsupported | integer/decimal/null/unknown member corpus ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-T04 | D10 Candidate §10 | tool 参数中传 EntityRef/token 作为“普通工具能力” | reject | unsupported | 首版 ToolValue 排除 Ref/Locator/control token | decoder negative |
| D10-T05 | D10 Candidate §10 | 文件工具取得任意 host path | reject | unsupported | 仅 InputSlot exact bytes，无 path capability | ../、symlink、home/workspace path 负向 ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-T06 | D6 §5 + D10 Candidate §16 | credential 被写入 Document、prompt、transcript | reject | unsupported | SecretRef + trusted transport injection | secret canary across source/context/log/export ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-T07 | D10 Candidate §9 | 可读 workspace context 发给未批准的新 Model Provider | reject | interactive | recipient-specific egress；read 不蕴含 egress | provider switch requires new match ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-T08 | D10 Candidate §9 | tool output 指令要求访问隐藏 Field | reject | unsupported | tool output 是不可信数据，不能扩大 readScope | hidden-world noninterference |
| D10-T09 | D10 Candidate §11 | executable package 继承 SSH agent/browser cookie/env secret | reject | unsupported | runtime 默认无这些 host capabilities | OS sandbox real tests ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-T10 | D9 Worker §2 + D10 Runtime | D9 conversion route 借 D10 transport 获得网络 | reject | unsupported | D9 no-network 默认保持，transport capability 不继承 | dependency/handle/network scan ；并核对本行边界不会扩大权限、身份或作者写入语义 |

## 5. Connector、外部效果与恢复

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-E01 | D3 bindings + D10 Candidate §16 | connector cursor/etag 作为作者 Field/identity | reject | unsupported | 控制状态分域；只有 closed SourceBinding/OriginBinding adapter 可改变绑定 | author-source/control-store diff ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-E02 | D10 Candidate §17 | external request 在 durable intent 后、send 前 crash | revise | automatic | 保守 outcome_unknown，除非 transport 能证明未发送 | fault injection at send fence ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-E03 | 同上 | send 已完成、success response 丢失 | accept | automatic | same EffectIntent/key reconcile；不新发 | provider idempotency/reconcile fixture ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-E04 | 同上 | provider 不提供 idempotency/conditional proof | accept | interactive | mutation write capability 对 unattended execution unavailable；人工处理 | capability matrix ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-E05 | 同上 | eventual-consistent readback 暂时未找到目标 | accept | deferred | 保持 unknown；不能判 failed_no_effect | delayed visibility service |
| D10-E06 | 同上 | 补偿删除外部对象 | revise | interactive | compensation 是新 ExternalEffectIntent/approval/cost | original success + compensation fail |
| D10-E07 | D9 Publication + D10 | 外部 publication 成功、Core Resource create 失败 | accept | interactive | 两个独立结果；不删除用户文件补偿 | two-stage failure |
| D10-E08 | D10 Candidate §16 | credential rotation 后只读 reconcile | accept | automatic | 可在 current permission 下用新 secret 读同账户；不 mutation resend | account continuity proof ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-E09 | 同上 | credential rotation 后旧 mutation 自动 resend | reject | unsupported | old EffectIntent 不换 generation 重发 | rotation race |
| D10-E10 | D10 Candidate §17 | revoke 与 send gate 并发 | accept | automatic | revoke 先赢不 send；send 先赢不可声称未执行 | linearization test |

## 6. Package、Registry 与 trust

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-P01 | D4 §3.2/Registry handshake | 两个 publisher claim 同 namespace | accept | unsupported | NamespaceClaim 唯一 owner；D4 owner conflict fail closed | claim conflict before inner parse ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-P02 | D4 Registry evolution | 同 semantic ID 回滚到旧 digest | reject | unsupported | semantic ledger 累计；rollback 是 successor activation | three-generation mutation/revival ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-P03 | D10 Candidate §7 | self-signed package 首装即 claim namespace | reject | unsupported | PublisherIdentity ≠ NamespaceClaim | trust-root/claim negative ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-P04 | D10 Candidate §7 | publisher key 合法轮换 | revise | automatic | old-key continuity + current trust policy + successor activation | old/new key chain ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-P05 | D10 Candidate §6 | runtime binary 更新而 Registry 未变 | accept | automatic | successor ActivationBinding/Catalog；RegistryBinding 可保持 | exact catalog digest generation ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-P06 | D10 Candidate §6 | health outage 每次生成 semantic generation | reject | unsupported | runtime health 与 semantic activation 分离 | flapping health no Registry churn ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-P07 | D10 Candidate §6 | uninstall 后已保存 unknown Field 被清理 | reject | unsupported | raw source 与 history 保留；typed state unavailable | uninstall/reinstall roundtrip ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-P08 | D10 Candidate §6 | 激活切换中 crash | accept | automatic | old 或完整 new ActivationBinding，不半态 | crash at every staging/commit point ；并核对本行边界不会扩大权限、身份或作者写入语义 |

## 7. Deferred boundaries with named owners

| ID | 来源定位 | 场景 | disposition | 执行模式 | owner/理由 | 再开放前要求 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-D01 | Mandatory Intake Calendar holiday/anniversary | 未冻结 holiday/anniversary provider 算法 | defer-with-owner | deferred | D4 temporal semantics + D10 provider admission | closed rule contribution、version/coverage、D7 consumer ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-D02 | Mandatory Intake ICS sync | 通用双向 ICS subscription/upsert | defer-with-owner | deferred | D3 binding + D9 mapping + D10 connector | SourceBinding/OriginBinding、conflict/watermark、idempotency ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-D03 | D10 Candidate §13 | 通用 multi-step workflow DAG | defer-with-owner | deferred | future D10 | typed DAG、recovery、budget、approval composition ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-D04 | D10 Candidate §14 | 无人值守 create/delete/Facet/native-table/bulk | defer-with-owner | interactive | future D10 + owning D3/D7 adapters | 机械 approval envelope 与 D6 atomic use ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-D05 | D10 Candidate §10 | 任意 JSON Schema/OpenAPI 自动生成工具 | defer-with-owner | deferred | future D10 ToolValue profile | exact numeric/null/additionalProperties/recursion contract ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-D06 | D10 Candidate §11 | 浏览器本地 MCP/process execution | defer-with-owner | unsupported | D1 surface boundary | 新 D1 surface/review ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-D07 | D10 Candidate §22 | Mobile Agent/connector/approval | defer-with-owner | unsupported | D1 + D8 + D10 joint | 重开 D1 与真实 Mobile evidence ；并核对本行边界不会扩大权限、身份或作者写入语义 |
| D10-D08 | TASK/A2 | A2 系统级验收 | defer-with-owner | deferred | A2 | D10 independent acceptance/activation 后才开始 |

## 8. 当前证据状态

这些行是规范 disposition，不是执行结果。本 D10 作者阶段没有宣称产品 Core、OS sandbox、真实 MCP/model/connector 服务、credential store、费用系统或 UI 已实现。

本轮候选的机械文档检查只能证明仓库格式、双语同步和输入清单一致性；真正的 D10 有限状态机/竞争模型若在后续提交实际运行，必须单独给源码、fixture 和真实结果，并明确其不证明生产实现。Implementation Impact 文件列出完整后续证据门。
