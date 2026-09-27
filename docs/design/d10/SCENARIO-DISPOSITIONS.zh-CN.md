---
source_language: zh-CN
translation_status: source
---

[English](SCENARIO-DISPOSITIONS.md)

# D10 场景裁决

revision: D10-r04-final-review-fixes-2026-09-28；状态：candidate。本文件把固定上游输入中的 D10 路由、TASK 强制故障和本候选新增竞争边界逐项落到可执行裁决。disposition 只评价 D10 候选如何承接该场景，不表示测试已经通过。

执行模式列只有四类：automatic 表示在本文 closed 规则下允许无需逐次人工确认继续；interactive 表示可以准备但必须逐次确认；unsupported 表示本代明确不可用；deferred 表示由具名 所有者 的未来合同冻结后才能开放。

来源定位分四类：**Intake 原要求**表示场景直接来自 Mandatory Intake；**上游合同**表示直接来自已接受 D1–D9；**候选推导攻击**表示作者从上游不变量构造的负向攻击，不冒充 Intake 原文；F/A/T/E/P 等其余条目是 TASK 或候选自增故障。来源标签只说明出处，不改变 disposition。

## 1. 上游强制路由与领域边界

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-U01 | 上游合同：D1 §2–§4，D1-I01/I06 | Agent、automation、connector 直接写工作区或与 Core 同时写 | reject | unsupported | Core 仍为唯一 author 提交 point；executor 无 author store/workspace mount | 依赖图、文件/DB 句柄负向、双 writer/fence 测试 |
| D10-U02 | 上游合同：D1 §4.1/§4.4 | Desktop 与 CLI 各启动独立 scheduler，或 terminal occurrence 被重扫后重复运行 | revise | automatic | 同一本地 control domain、唯一 occurrence claim 与 durable terminal proof；CLI 管理同一能力族 | 双进程 claim；K terminal 后重启、重扫、disable→enable、缓存重建均恢复原 Run |
| D10-U03 | 上游合同：D1 §4.2/§4.3 | WebUI 直接跑 模型/connector 或浏览器持 凭据 | reject | unsupported | WebUI 只经 Server Broker；secret 留 Server secret store | 浏览器 bundle/API 扫描、凭据 泄漏负向 |
| D10-U04 | 上游合同：D1 §4.5；D8 Direction §7 | Mobile Agent、automation approval、conversion/connector 管理 | reject | unsupported | 保留 D1 unsupported_surface；只消费 committed facts | 五端 capability fixture，Mobile 不出现隐藏批准入口 |
| D10-U05 | Intake 原要求：§6.1、§6.6、§13.1.2；D4 Calendar | Calendar recurrence/后台提醒从设备时钟或未绑定规则猜测 | revise | automatic | Automation 调度 使用已验证 D4 temporal rules、有限 horizon/limit 与 exact 源 dependency | tzdb/rule 代际 改变、DST/超域、重启 反例 |
| D10-U06 | Intake 原要求：§4.4、§13.1.3 | organization 提供方 ID/服务名变成 Node 身份 | reject | unsupported | Connector ID 留控制域；作者关系仍 D3/D4 身份 | 同名/改名/提供方 ID 重用不得合并 Node |
| D10-U07 | Intake 原要求：§6.5、§8.3 Packs、§9 第49–50项 | pack 安装即改变作者字段或 提供方 失效删除事实 | reject | unsupported | 激活只改变 Registry/Catalog availability；raw 源 与 semantic ledger 保留 | disable/uninstall/失败 update 后作者 字节 不变 |
| D10-U08 | Intake 原要求：§9 第8项 | connector sync 把 cursor/凭据 写入作者 源 | reject | unsupported | cursor/etag/凭据 是控制状态；写回需具名 closed 适配器 | author 源 搜索无 secret/cursor；提交/control 原子关联 |
| D10-U09 | Intake 原要求：§9 第9、15项 | 各 surface 对同 capability 自己猜是否可用 | reject | unsupported | D1 capability 为唯一产品可用性；D10 只提供 downstream facts | Desktop/CLI/Server/WebUI 相同 reason precedence |
| D10-U10 | Intake 原要求：§9 第21项 | 模式定义、connector、运行时 状态 合并为一个 提供方 状态 | reject | unsupported | D4 Registry、D10 Catalog、运行时 health 三域分离 | 模式定义 可用/运行时 不可用 与反向组合  |
| D10-U11 | Intake 原要求：§9 第28项 | holiday/workday 提供方 变更后旧派生结果继续有效 | revise | automatic | rule/贡献项 代际 进入依赖；失效 reset/recompute | 旧 结果/缓存/automation 调度 不能混 代际 |
| D10-U12 | Intake 原要求：§9 第32项 | ICS UID/RECURRENCE-ID 自动成为 D3 身份/upsert key | reject | unsupported | 只经已冻结 SourceBinding/OriginBinding 适配器 才可 lookup/upsert | 相同 UID 两次 ordinary import 保持 fresh，未开放 sync 明确 不可用 |
| D10-U13 | Intake 原要求：§9 第33项 | subscribe、sync、copy、adopt 混成同一个“同步”动作 | reject | unsupported | 每个 authority/效果 走其 所有者 closed protocol | 不同 intent 的 请求/回执/approval 不互用 |
| D10-U14 | Intake 原要求：§9 第36项 | package 获得 whole-package 工作区权限 | reject | unsupported | 权限按 Contribution，五类授权维度分开 | 同包纯数据可用、网络 connector denied 的组合 |
| D10-U15 | Intake 原要求：§9 第37项 | Settings/Marketplace UI 状态成为 capability authority | reject | unsupported | UI 只是 Activation/Policy/Catalog 的投影 | 隐藏/显示按钮不改变 Core eligibility |
| D10-U16 | Intake 原要求：§8.5.1；§9 第37、43、54项 | 本地化 模块/清单 名改变 规范 ID | reject | unsupported | 规范 命名空间/贡献项 ID 独立 区域设置 | 中英/RTL 切换 请求 字节 不变  |
| D10-U17 | 候选推导攻击：D1 §4.5/§9 Mobile 边界 + D9 conversion；非 Intake 直接条目 | Mobile 上传文件后自动委托 Server 转换/Agent | reject | unsupported | Mobile 只能普通附件；无转换/Agent 委托/审批 | 上传不触发 worker/模型，返回 unsupported_surface |
| D10-U18 | Intake 原要求：§13.1.5；上游 D7 Query Algebra §6 SearchContribution | 未接纳 贡献项 动态注入 SearchContribution/模式定义 | reject | unsupported | D10 认证并 代际-bind Contribution；D7 grammar 不变 | 运行时 discovery 新 贡献项 只 pending，不进入 当前 query |
| D10-U19 | Intake 原要求：§6.5、§8.3 Packs、§9 第28、48、50项；上游 D4 unknown/unavailable 保留 | 禁用规则 pack 后把作者值删除/默认化 | reject | unsupported | 相关派生能力 不可用/reset，作者 源 保留 | disable/reenable 与 raw 源 digest 不变 |
| D10-U20 | Intake 原要求：§8.5.1–§8.5.4；§9 第53、54、57项 | D10 新术语覆盖 D1–D9 owned name 或 wire alias 漂移 | revise | automatic | Terminology 文件与 controlled-name gate | 受控 positive surface 扫描；历史 prose 排除 |
| D10-U21 | Intake 原要求：§2.5、§8.3 第6项、§9 第39、56项；上游 D9 Templates | Node/Office Template 被当作长期 Agent 脚本或任意 callback | reject | unsupported | Template 仍 D9 一次性构造/渲染；D10 不执行其普通文字 | template text 含 工具 指令只作不可信文字 |
| D10-U22 | 上游合同：D7 Query Algebra §6 SearchContribution | SearchContribution 内带脚本/网络 或 提供方 不可用 时静默跳项 | reject | unsupported | 保持 D7 纯数据 closed descriptor；D10 只认证来源/代际 | 缺 贡献项 整体按原 D7 不可用，不改 grammar |
| D10-U23 | 上游合同：D7 Execution §5–§6 | Query row/evidence 被 Agent 当长期写授权 | reject | unsupported | evidence/selector 仍 TTL/revision/dependency/当前 授权-bound | A→B→A、授权 change、结果 reset 后旧 evidence 失败  |
| D10-U24 | 上游合同：D8 Main §2–§3 / Interfaces §4–§5 | 模型 输出伪装成人类 Draft、自动点击确认 | reject | unsupported | D8 edit 仍 interactive；Automation 不创建 EditSession | dirty Draft、composition、stale 预览、unknown 回执 全链 |
| D10-U25 | 上游合同：D9 Workers §1–§2 | 把 D9 conversion worker 变成有网络的通用工具 宿主 | reject | unsupported | D9 worker 继续专用无 workspace/网络 默认；D10 executor 独立 | worker sandbox 不因 D10 网络能力扩大 |
| D10-U26 | 上游合同：D9 Export §4 | 外部 publication 回执 当作 Resource/author 回执 | reject | unsupported | PublicationReceipt 与 D3/D6 回执 分域 | publish 成功 + Resource create fail 两结果并存  |
| D10-U27 | 上游合同：D3 Lexicon §origin-binding / §adopt | Adopt 把 `adoption_binding` 登记成自身 code convention，与既有 `OriginBinding` / `origin_binding` 所有权重叠 | revise | deferred | UPSTREAM-AMENDMENTS 的 B10-01 最小词表勘误：Adopt 只保留 `adopt_*`；关联绑定归 origin-binding 所有，不加 alias/wire/identity/capability | 正向 `adopt_*`→`OriginBinding/origin_binding`；反向受控正向面不存在 `adoption_binding`，scanner 不加豁免 |

## 2. TASK 强制故障边界

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-F01 | TASK Constraints/Acceptance | 恶意 Document 要求把其它 workspace/secret 上传 | accept | automatic | untrusted-data precedence、ContextBundle 最小化、recipient-specific egress | 两个隐藏世界相同 工具 catalog；无 secret/extra read |
| D10-F02 | TASK | prompt injection 要求启用新 MCP 工具 或扩大 budget | accept | unsupported | remote descriptor/output 无控制权；allowlist/budget 由受管记录 | 注入文本与控制记录 diff 必须为零 |
| D10-F03 | TASK | Delegation Lease 到期后 queued/paused Run 继续执行 | accept | automatic | 每个受保护步骤与最终提交都检查当前 Lease 和可信时间；cleanup 是否运行不决定 expiry | 暂停跨过 notAfter 后拒绝新步骤/最终提交；重启不延长期限 |
| D10-F04 | TASK | 撤权后旧 context/缓存/结果 继续交付 | accept | automatic | 当前 D6 代际/所有者 gate；各上游 缓存 规则保持 | revocation 与 chunk/page/工具-call 竞争 |
| D10-F05 | TASK | 同一调度 occurrence 重复 claim 或 terminal 后重新建 Run | accept | automatic | `AutomationOccurrenceKey/1`、durable unique claim、terminal dedup proof | 两进程竞争；K terminal 后重启/重扫/disable-enable 不另起 Run |
| D10-F06 | TASK | 崩溃时外部请求可能已经发送 | accept | automatic | 先耐久保存 ExternalEffectIntent，再经过发送栅栏；无法证明发送结果时恢复为 outcome_unknown | 在“耐久意图→发送→响应”三个边界分别做故障注入 |
| D10-F07 | TASK | 取消 与 D6 planning 竞争 | accept | automatic | 已 planned 不被 Run 取消 冒充 abort；按原 D6 恢复 | 取消-before-plan、plan-before-取消、提交-before-取消  |
| D10-F08 | TASK | 取消 与 外部 send 竞争 | accept | automatic | 仅证明未 send 才 cancelled；submitting 后三态 | send fence 两侧 fault injection |
| D10-F09 | TASK | 凭据 rotation 后旧 请求 自动用新 凭据 重发 | reject | unsupported | old attempt 绑定实际 secretGeneration；新 凭据 仅可受权 reconcile | rotation + unknown 请求；禁止 mutation resend |
| D10-F10 | TASK | 失败 package upgrade 半激活 Registry/Catalog | accept | automatic | staged validation + one ActivationBinding switch | 每个激活步骤 崩溃；旧 binding 完整存活  |
| D10-F11 | TASK | 已激活版本回滚时 Registry 指针倒退 | reject | unsupported | rollback 是 successor activation；semantic ledger 累计 | 三代 模式定义/history/revival attack |
| D10-F12 | TASK | audit collector offline 就停止全部本地工作 | revise | automatic | local durable spool 是安全门；remote collector 可延迟 | collector offline、spool full、disk failure 分支 |
| D10-F13 | TASK | 本地耐久 audit 写失败后仍执行受保护步骤 | reject | unsupported | 本地受保护审计写失败时 fail closed；取消、撤权和紧急停止保留独立控制写入容量 | 在读取、出站、secret 使用、外部发送和作者提交之前分别注入审计故障 |
| D10-F14 | TASK | Desktop/CLI/Server/WebUI 对相同请求得不同目标/错误 | accept | automatic | shared Core semantics；宿主 只运输/认证 | identical fixture 跨四端；Mobile 为负能力 |
| D10-F15 | TASK | capability probe 泄露 package 未安装/账户状态给 denied principal | accept | automatic | D1 reason precedence，policy_denied 先遮蔽 deployment detail | 重叠 reason 矩阵 |
| D10-F16 | TASK | 外部 效果 与 Core write 被展示为一个“原子成功” | reject | unsupported | 两个独立 outcome/回执；无 composite author 回执 | Core 成功/外部 unknown 与反向组合 |
| D10-F17 | TASK | unknown 外部 结果 换新 idempotency key 自动重试 | reject | unsupported | 原 EffectIntent/key；无可靠 reconcile 则停止 | 超时、eventual consistency、window expiry  |
| D10-F18 | TASK | idempotent 效果 被认为 retry 不收费 | reject | unsupported | 每 attempt 独立 CostReservation | 重试费用、billing delay、unknown charge |
| D10-F19 | TASK | 并发 Runs 都消费 Standing Approval 最后一次 | accept | automatic | ApprovalUse count reservation 与 D6 planning CAS；authoritative terminal_failed 才释放为 released_terminal | N=1 双并发至多一个 reserved；terminal replay 不二次释放 |
| D10-F20 | TASK | 并发 Runs 都消费最后一笔费用预算 | accept | automatic | 多账户原子 CostReservation | Run/Lease/Automation/deployment 四账户竞争 |
| D10-F21 | TASK | 费用未知或 author terminal_failed 后错误释放额度 | reject | unsupported | CostReservation=uncertain 持续占原上限；费用状态与 approval reservation 分域 | 崩溃/超时/重启/author abort 均不自动 release；只有证明收费尝试未开始才 released |
| D10-F22 | TASK | 提供方 超过已接纳价格上限仍静默继续 | revise | automatic | 记录 anomaly、冻结 capability、要求管理处理 | simulated overcharge，不自动提高 ceiling |
| D10-F23 | TASK / Candidate §18 | 已实际发送请求且可靠账单为 0，被错误标成 released | reject | unsupported | 发送后有最终计费事实一律 settled(actual)，零费用为 settled(0)；released 仅证明收费执行从未开始 | 发送前取消→released；实际发送零账单→settled(0)；费用不可证→uncertain |
| D10-F24 | TASK / Candidate §8 | `maxRuns=1`，R1 首次受保护执行后模型调用收费并失败，R2 仍开始 | reject | unsupported | Run-admission CAS 在第一次受保护执行前建立 `LeaseRunUse/1` 并消费一次；失败/取消不退款 | R1 失败后 R2/下一 K 得到 `delegation_exhausted`；同 R1 重启不重复消费 |
| D10-F25 | TASK / Candidate §13 | terminal K 经重启、重扫或 disable→enable 后另起 Run | reject | unsupported | claim/terminal 防重事实在 definition revision 可恢复期间持续可证，压缩不得丢失“不再执行”证明 | K terminal 后四种恢复路径都返回原 Run/outcome |
| D10-F26 | TASK / Candidate §8/§20 | Run 暂停，Lease 跨过 notAfter，但清理任务未运行 | accept | automatic | trusted 当前 time 是准入/提交门；cleanup 不拥有 expiry 语义 | 推进可信时间后拒绝新 context/模型/工具/外部 step 与最终 author submit |
| D10-F27 | TASK / Candidate §8/§20 | clock epoch/时间连续性丢失却假定 Lease 仍有效 | reject | unsupported | 返回 `state_unavailable` 并暂停；恢复可信时间后按真实当前时间判 active/expired | 丢 epoch、重启、恢复时间三段状态机 |
| D10-F28 | Candidate §8/§13/§20 | `maxRuns=1` 已由 R1 首次准入消费，R1 的第二个受保护步骤因为 remaining=0 被误判成新 Run | reject | unsupported | 同 Run 已有完整 `LeaseRunUse/1` 时不再比较 remaining count，也不再次消费；仍逐步验证当前授权、准确 leaseRevision、可信时间、ActivationBinding、批准与预算 | 正向同 Run 第二步继续；负向 lease revoke/expire/revision-change 时仍阻止 |
| D10-F29 | Candidate §8/§20 | 同 Run 的原 D6 planned request 恢复时，因为 lease 已无剩余次数而返回 `delegation_exhausted` | reject | unsupported | planned 恢复先证明原 `LeaseRunUse/1` 连续性，复用既有准入，不重新做新 Run count gate；若记录缺失/连续性不可证则 `state_unavailable`，不是重新消费 | `maxRuns=1`、remaining=0 的原 planned 恢复可继续；新 Run 同时应被 `delegation_exhausted` 拒绝 |

## 3. Standing Approval 与确认边界

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-A01 | D7 Execution §5–§6 + D10 Candidate §14–§15 | 单 Node/Field 恰一 Entry 的 bool member 更新，值在有限集合内 | revise | automatic | fresh `set_field_member` prepare +完整 owner_fields 预览 + ApprovalUse | 真实 D7 Narrow Field/预览/D6 提交 集成 |
| D10-A02 | 同上 | 同一 Field 两个 Entry 值完全相同 | accept | interactive | `require_exactly_one_entry` 失败，不自动 first | 两同值 occurrenceKey 反例 |
| D10-A03 | 同上 | Field 从一 Entry 并发变两 Entry 后消费旧批准 | accept | interactive | fresh selection/源 revision + dependency conflict | prepare/approve/提交 三阶段 race |
| D10-A04 | 同上 | proposed change 同时改变 note/provenance/Facet/关系 | reject | unsupported | footprint 必须仅一个 member | mutant footprint 必须拒绝 |
| D10-A05 | 同上 | append/remove/replace whole Entry 试图使用 standing envelope | reject | interactive | 首版无人值守 profile 不覆盖；可重新走交互 D7 | action-kind negative matrix |
| D10-A06 | D8 Interfaces §4–§5 | 自动整源 document edit | accept | interactive | 仍走 D8 Draft/预览/explicit confirmation | Agent 不创建 fake EditSession  |
| D10-A07 | D3/D7 create/lifecycle | Agent 自动创建/Trash/restore/copy Node | accept | interactive | 原 D3/D7 预览 + 逐次确认 | fresh 身份、closed modes、不消费 standing envelope  |
| D10-A08 | proposed D6/D7 amendment | standing approval 已过期但 请求 已 committed，用户重放 回执 | accept | automatic | saved decision 在 当前 授权 下重放原 字节，不重复扣 approval | lost 回执 after approval expiry  |
| D10-A09 | D6/D7 配套修订提案 | request 已 planned、旧 preview 已过期且 Standing Approval 不再可用 | revise | interactive | `d7_planned_preview_open` 在当前 audience/ObservationScope/权限/连续性下从原 pins 重签有限 recovery epoch；完整查阅后建立 `PlannedDecisionApproval/1` 绑定原 request | 新客户端无旧副本可查原 preview；不重算 Query/target；确定 dependency conflict 不能复活 |
| D10-A10 | D6/D7 配套修订提案 | `set_field_member` 请求值已等于 current member，完整 source 逐字 no-op | accept | automatic | 仍验证唯一 Entry/member、类型、valueConstraint、当前权限与依赖；MutationFootprint、field_change、sourceVersions 均为空 | 不伪造 effect/version；若原 D6 committed 则消费一次批准，replay 不重复 |
| D10-A11 | D8 Main §3 | 后台合法更新与当前 dirty Draft 同 所有者 | accept | automatic | author 提交 有效；D8 Draft 进入 stale/conflict，不被覆盖 | 当前 Draft 字节/selection 保持 |
| D10-A12 | D8 Main §3 | composition 期间 Agent proposal 自动提交 | reject | interactive | 保持 D8 composition/预览门；未完成输入法事务不能被 Agent 提案绕过 | 组合输入轨迹与迟到 Agent proposal 的竞争测试 |
| D10-A13 | D6/D7 配套修订提案 | R1 前置批准有效，R2 抢占最后次数；R1 进入 D6 后只有 approval 失败 | revise | automatic | 新 D6 `approval_unavailable/preflight`；unseen 不写 ledger，planned 保持 planned；D10 不包装 | 原 D6 permission/business error 优先；批准变化后同 请求 可按规范重试 |
| D10-A14 | D6/D7 配套修订提案 | planned 已 reserved，依赖改变并被原 D6 authoritative abort 为 terminal_failed | revise | automatic | abort 同事务把 approval count `reserved→released_terminal`；历史保留，replay 不双释放 | 取消/TTL/临时撤权/Lease 过期均不得 release；费用状态另行裁决 |

## 4. MCP、工具、secret 与出站

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-T01 | D10 Candidate §10 | MCP server 把删除工具自报为 readOnly | reject | interactive | 远端 annotation 不决定效果类别；本地接纳的 Contribution 才定义工具是读取还是外部 mutation | hostile MCP descriptor 中伪造 readOnly，验证授权门不改变 |
| D10-T02 | D10 Candidate §10 | MCP 运行时 新发现一个 工具/模式定义 | accept | deferred | 只进入 pending admission；当前 allowlist 不变 | discovery diff 不改变可调用目录 |
| D10-T03 | D10 Candidate §10 | 远端 JSON 把 2^63+1 经 double 舍入 | reject | unsupported | ToolValue 必须保留 exact integer；适配器不能证明精确语义就拒绝 | 覆盖 integer、decimal、null、未知成员和越界数值 |
| D10-T04 | D10 Candidate §10 | 工具 参数中传 EntityRef/token 作为“普通工具能力” | reject | unsupported | 首版 ToolValue 排除 Ref/Locator/control token | decoder negative |
| D10-T05 | D10 Candidate §10 | 文件工具取得任意 宿主 path | reject | unsupported | 仅 InputSlot exact 字节，无 path capability | ../、symlink、home/workspace path 负向  |
| D10-T06 | D6 §5 + D10 Candidate §16 | credential 被写入 Document、prompt 或 transcript | reject | unsupported | SecretRef 只由受信 transport 注入认证通道，secret bytes 不进入作者源、上下文或普通日志 | 用 canary secret 扫描 source、context、log、transcript 和 export |
| D10-T07 | D10 Candidate §9 | 已获权读取的 workspace context 被发送给未批准的新 Model Provider | reject | interactive | 出站授权按 recipient 精确绑定；读取资格不蕴含发送到另一个提供方 | 切换模型提供方时必须重新匹配 egress recipient |
| D10-T08 | D10 Candidate §9 | 工具 output 指令要求访问隐藏 Field | reject | unsupported | 工具 output 是不可信数据，不能扩大 readScope | hidden-world noninterference |
| D10-T09 | D10 Candidate §11 | 可执行 package 继承 SSH agent、browser cookie 或环境 secret | reject | unsupported | runtime 默认没有这些宿主能力，只能使用显式 InputSlot 与受管 transport | 在真实 OS sandbox 中探测 home、浏览器资料、SSH 与环境变量 |
| D10-T10 | D9 Worker §2 + D10 Runtime | D9 conversion route 借 D10 transport 获得网络 | reject | unsupported | D9 no-网络 默认保持，transport capability 不继承 | dependency/handle/网络 scan  |

## 5. Connector、外部效果与恢复

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-E01 | D3 bindings + D10 Candidate §16 | connector cursor/etag 作为作者 Field/身份 | reject | unsupported | 控制状态分域；只有 closed SourceBinding/OriginBinding 适配器 可改变绑定 | author-源/control-store diff  |
| D10-E02 | D10 Candidate §17 | 外部 请求 在 durable intent 后、send 前 崩溃 | revise | automatic | 保守 outcome_unknown，除非 transport 能证明未发送 | fault injection at send fence  |
| D10-E03 | 同上 | send 已完成、成功 response 丢失 | accept | automatic | same EffectIntent/key reconcile；不新发 | 提供方 idempotency/reconcile fixture  |
| D10-E04 | 同上 | 提供方 不提供 idempotency/conditional proof | accept | interactive | mutation write capability 对 unattended execution 不可用；人工处理 | capability matrix  |
| D10-E05 | 同上 | eventual-consistent readback 暂时未找到目标 | accept | deferred | 保持 unknown；不能判 failed_no_effect | delayed visibility service |
| D10-E06 | 同上 | 补偿删除外部对象 | revise | interactive | compensation 是新 ExternalEffectIntent/approval/cost | original 成功 + compensation fail |
| D10-E07 | D9 Publication + D10 | 外部 publication 成功、Core Resource create 失败 | accept | interactive | 两个独立结果；不删除用户文件补偿 | two-stage failure |
| D10-E08 | D10 Candidate §16 | 凭据 rotation 后只读 reconcile | accept | automatic | 可在 当前 permission 下用新 secret 读同账户；不 mutation resend | account continuity proof  |
| D10-E09 | 同上 | 凭据 rotation 后旧 mutation 自动 resend | reject | unsupported | old EffectIntent 不换 代际 重发 | rotation race |
| D10-E10 | D10 Candidate §17 | revoke 与 send gate 并发 | accept | automatic | revoke 先赢不 send；send 先赢不可声称未执行 | linearization test |

## 6. Package、Registry 与 trust

| ID | 来源定位 | 场景/风险 | disposition | 执行模式 | D10 候选落点 | 验证义务 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-P01 | D4 §3.2/Registry handshake | 两个 publisher claim 同 命名空间 | accept | unsupported | NamespaceClaim 唯一 所有者；D4 所有者 conflict fail closed | claim conflict before inner parse  |
| D10-P02 | D4 Registry evolution | 同一个 semantic ID 试图回滚到旧 digest | reject | unsupported | semantic ledger 只允许累计后继；rollback 也是 successor activation | 三代演进中验证 mutation、tombstone 与复活攻击 |
| D10-P03 | D10 Candidate §7 | self-signed package 首次安装就要求 claim namespace | reject | unsupported | PublisherIdentity 与 NamespaceClaim 分开；自签只能证明某把 key 签过包 | 信任根和 namespace claim 的负向测试 |
| D10-P04 | D10 Candidate §7 | publisher key 合法轮换 | revise | automatic | 必须证明旧 key 到新 key 的连续性，并由当前 trust policy 接纳为 successor activation | 旧/新 key chain、撤销和错误接管反例 |
| D10-P05 | D10 Candidate §6 | 运行时 binary 更新而 Registry 未变 | accept | automatic | successor ActivationBinding/Catalog；RegistryBinding 可保持 | exact catalog digest 代际  |
| D10-P06 | D10 Candidate §6 | health outage 每次都生成新的 semantic generation | reject | unsupported | runtime health 与 semantic activation 分域；短暂故障不改 Registry 历史 | 反复健康抖动时 RegistryBinding 保持不变 |
| D10-P07 | D10 Candidate §6 | uninstall 后已保存 unknown Field 被清理 | reject | unsupported | raw 源 与 history 保留；typed 状态 不可用 | uninstall/reinstall roundtrip  |
| D10-P08 | D10 Candidate §6 | 激活切换中 崩溃 | accept | automatic | old 或完整 new ActivationBinding，不半态 | 崩溃 at every staging/提交 point  |
| D10-P09 | S D4 reference catalog + 上游 D7 Narrow Field | `single_field_member` 因为“只改一个 Field”而跳过真实 Facet/Relation/constraint 图 | reject | unsupported | 当前第49份输入证明目录含 27 relation Fields、Calendar 跨 Field `union_variant_equal` 与 required/local constraints；必须运行完整 Registry 图证明，`people/phone` 只作实际正向构造 | 正向 `people/phone`；负向 relation Field、`calendar/range`/`calendar/recurrence` 跨字段约束与 unknown constructor 都拒绝窄证明 |
| D10-P10 | Intake 原要求：§6.5、§8.3、§9 第49项 | Pack 已安装但父 domain/extension point 缺失 | accept | automatic | package/config/source 保留，dependent contribution inactive；无更高优先级 D1 reason 时 capability 投影 `missing_component` | 不注册领域 rule/View/Action；已接纳 D4 history/raw source 不删除 |
| D10-P11 | Intake 原要求：§6.5、§9 第49–50项 | 父 extension point 显式停用，但 dependent pack 仍暗中运行 | reject | unsupported | contribution inactive；组件存在但配置停用时 D1 投影 `not_configured`；accepted schema retention 与 activation 分域 | 禁用 Calendar 后 holiday/rule 输出消失，作者 period/range/event facts 与配置保留 |
| D10-P12 | Intake 原要求：§6.5、§9 第49项 | parent version 不兼容仍沿旧规则继续运行 | reject | unsupported | dependency resolution=`incompatible`，dependent contribution inactive；D1 投影 `incompatible_version`，必须 successor activation 后恢复 | parent upgrade/downgrade 竞争不能让旧 binding 跟随 latest |
| D10-P13 | Intake 原要求：§6.5、§8.3 + D1 capability | 仅隐藏父模块 UI 就把 pack 当 disabled，或用 UI 显示强行启用 | reject | unsupported | UI visibility 与 dependency activation 分离；parent semantic capability 仍 ready 时 activation 不变 | 隐藏/恢复导航入口前后 RegistryBinding、Contribution activation 和作者 facts 不变 |
| D10-P14 | Intake 原要求：§6.5、§9 第49项 + D1 | 当前 surface 不支持父 domain/规则运行 | accept | unsupported | 该 surface contribution inactive，D1 固定 `unsupported_surface`；其它 surface/global config 不删除 | Mobile 不运行规则仍按其 Core/D4 能力保留 portable author facts |
| D10-P15 | Intake 原要求：§6.5、§8.3；Candidate §6.1 | parent extension-point version/binding 改变但 dependent Catalog binding 未失效 | reject | unsupported | resolved parent binding 进入 Capability Catalog digest；变化必须 successor ActivationBinding，禁止 ambient latest | 两个 activation generation 不能混用旧 parent rule 与新 Catalog |

## 7. Deferred boundaries with named owners

| ID | 来源定位 | 场景 | disposition | 执行模式 | 所有者/理由 | 再开放前要求 |
| --- | --- | --- | --- | --- | --- | --- |
| D10-D01 | Intake §13.1.2 + §6.5；仅具体算法/规则语义 | 未冻结 holiday/anniversary 提供方 算法 | defer-with-owner | deferred | D4 temporal semantics + D10 提供方 admission | closed rule 贡献项、version/coverage、D7 consumer  |
| D10-D02 | 强制输入 ICS sync | 通用双向 ICS subscription/upsert | defer-with-owner | deferred | owner 为 D3 binding + D9 mapping + D10 connector | 再开放前必须闭合 SourceBinding/OriginBinding、冲突/watermark 和幂等语义 |
| D10-D03 | D10 Candidate §13 | 通用多步骤 workflow DAG | defer-with-owner | deferred | owner 为未来 D10 | 再开放前必须定义 typed DAG、恢复、预算和 approval 组合 |
| D10-D04 | D10 Candidate §14 | 无人值守 create/delete/Facet/native-table/bulk | defer-with-owner | interactive | owner 为未来 D10 与相应 D3/D7 adapters | 每种动作都要独立机械 approval envelope 与 D6 原子消费合同 |
| D10-D05 | D10 Candidate §10 | 从任意 JSON Schema/OpenAPI 自动生成可调用工具 | defer-with-owner | deferred | owner 为未来 D10 ToolValue profile | 必须先闭合 exact numeric、null、additionalProperties 和递归边界 |
| D10-D06 | D10 Candidate §11 | 浏览器本地运行 MCP 或进程型执行器 | defer-with-owner | unsupported | owner 为 D1 产品端边界 | 必须先重开 D1，并重新审查浏览器执行、文件/网络/进程隔离和权限模型 |
| D10-D07 | D10 Candidate §22 | Mobile 上提供 Agent、connector 或 approval | defer-with-owner | unsupported | owner 为 D1 + D8 + D10 联合边界 | 必须重开 D1，并提供真实 Mobile 平台、交互和权限证据 |
| D10-D08 | TASK/A2 | A2 系统级验收 | defer-with-owner | deferred | A2 | D10 independent acceptance/activation 后才开始 |

## 8. 当前证据状态

这些行是规范 disposition，不是执行结果。本 D10 作者阶段没有宣称产品 Core、OS sandbox、真实 MCP/模型/connector 服务、凭据 store、费用系统或 UI 已实现。

本轮候选的机械文档检查只能证明仓库格式、双语同步和输入清单一致性；真正的 D10 有限状态机/竞争模型若在后续提交实际运行，必须单独给源码、fixture 和真实结果，并明确其不证明生产实现。Implementation Impact 文件列出完整后续证据门。
