---
source_language: zh-CN
translation_status: source
---

[English](IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md)

# D10 实现影响与测试轮廓

revision: D10-r01-candidate-2026-09-25；状态：candidate。本文件描述未来实现义务和证据门，不表示当前仓库已经实现 Agent、自动化、Connector、MCP、standing approval 或 D10 runtime。本文不授权修改产品代码；当前 PR 只包含设计材料。

## 1. 实现切片与状态所有者

目标实现保持“窄控制面 + 专用 executor”，不建立一个包含所有领域语义的通用 tool host。

```text
Desktop / CLI local                     Server / WebUI remote
        |                                      |
        v                                      v
  D10 Broker / Scheduler                D10 Broker / Scheduler
        |                                      |
        +---- Core-managed D10 control adapters+
        |              |
        |              +--> D6 Policy / ledger / author commit
        |              +--> D4 Registry validation/evolution
        |              +--> D7 Action prepare/effects
        |              +--> D8 edit proposal/confirmation
        |              +--> D9 conversion/export
        |
        +--> Agent runtime
        +--> Model Adapter
        +--> Tool Adapter / MCP Adapter
        +--> Connector
        +--> managed transport / secret injection
```

Broker 不读写 authority DB 表，不解释 D2/D4 source，也不接受自由 callback 写入。Core-managed control adapter 是所有 durable delegation、approval、ActivationBinding、occurrence claim、ApprovalUse 和 author-submit 资格的唯一入口。

建议实现顺序：

1. S1：D10 strict value/control decoders、受管 token tags、ActivationBinding/Capability Catalog、publisher/namespace trust。。上述技术名称均只表示本文定义的受控边界，不增加额外权限、身份或作者写入语义。
2. S2：DelegationLease、ContextBundle、egress、SecretRef、audit spool 与原子预算/费用 reservation。
3. S3：Run/Automation Definition、occurrence claim、serial scheduler、cancel/restart。
4. S4：Model/Tool/MCP adapters 与 runtime isolation；先只 read/compute，不开放 external mutation。
5. S5：ExternalEffectIntent、send fence、idempotency/reconciliation、credential rotation。
6. S6：协调实现 UPSTREAM-AMENDMENTS 中 D6/D7 standing-approval 分支；在此之前无人值守 author commit 继续 unavailable。
7. S7：Connector 的具名 read-only profile；只有已有 SourceBinding/OriginBinding closed adapter 的写回才可逐 profile 开放。
8. S8：Desktop/CLI/Server/WebUI surfaces、diagnostics、audit/export/retention；Mobile 只做 negative capability conformance。。上述技术名称均只表示本文定义的受控边界，不增加额外权限、身份或作者写入语义。
9. S9：清理旧原型/别名/自由 JSON/tool callback、更新公开规范；只有真实实施/平台证据完成后才能公开声称支持。

## 2. 数据与存储影响

### 2.1 Authority Store

需要新增受管 D10 control records，但不能增加另一 author commit root。逻辑上至少包括：

- activation/trust/catalog records；
- Delegation Lease 与 Standing Approval；
- Automation Definition、occurrence claim、Run/step；
- ApprovalUse 与 approval-count reservation；
- budget/cost account 与 reservation；
- ExternalEffectIntent/attempt/reconciliation evidence；
- Audit Started/terminal link 与本地 protected spool metadata；
- SecretRef/account generation metadata，不含 secret bytes。

这些记录应与 Workspace/authority identity、fence、current principal 和 version/CAS 一起管理。SQL table layout、index 和 GC 由实现决定，但不得改变候选的 version、atomicity、replay、masking、retention 语义。

D6 author ledger 仍是 Workspace+OperationId 唯一 author decision namespace。D10 Run/Approval/ExternalEffect 记录不得在恢复时生成另一个“作者 committed”事实。

### 2.2 Secret Store

Desktop/CLI 使用符合平台能力的 OS secret store；Server 使用部署提供的受保护 secret store/KMS 等价物。实现必须证明：

- secret bytes 不写 author DB、普通日志、crash dump fixture、transcript、export；
- SecretRef 不能在另一个 account/contribution/audience 下重放；
- rotation generation、disable/revoke、restart、backup/restore 行为可验证；
- Server 多前端不会让浏览器取得 raw secret。

若所声称平台无法提供合格 secret store，该依赖 secret 的 contribution 为 unavailable；不能回退到明文配置文件或环境变量继承。

### 2.3 Immutable assets

package、manifest、definition、runtime、model、font/other dependency 按 exact digest/version 存于不可变资产区。激活前完整校验，激活后不能就地覆盖相同 identity 的 bytes。GC 只能删除不再被 current/historical binding、planned decision、unknown effect 或 audit/recovery pin 引用的资产。

## 3. 激活、Registry 与 package 实施义务

Package verifier 实现 SHA-256 目录绑定、Ed25519 package signature、PublisherIdentity、NamespaceClaim、dependency closure 与平台约束。具体 key storage、revocation list distribution 和管理员 UX 可由实现选择，但必须满足以下规范行为：

- 自签不产生 namespace ownership；
- reserved namespace 永不被 third-party claim；
- key rotation 保持 publisher continuity；
- revoke 后新的 trusted context/execution 失效；
- 历史 D4 semantic ledger、saved decision、author source 不删除；
- ActivationBinding 切换要么旧、要么完整新，不能半态；
- semantic rollback 只能作为 successor activation，不能倒退 Registry history。

D4 candidate Registry 仍必须通过原 `RegistrySnapshot/1`、`RegistryBinding/1`、evolution proof 与 catalog loader。不能为了实现方便把 D10 package metadata 塞进 D4 closed snapshot。

## 4. Agent、Tool 与 MCP 实施义务

Agent runtime 只持 Run-local state、ContextBundle refs 和已允许 tool catalog，不缓存 ambient workspace authority。Context builder 必须从 Core 当前授权输入产生 exact source/result pins，并在 egress 前再次匹配 recipient。

ToolValue decoder/encoder 必须逐类型证明 exact 语义；严禁：

- JS double 代替 arbitrary integer/decimal；
- JSON null 同时表示 Optional.none、missing member 和 invalid；
- additionalProperties=true 的自由 map；
- 未界定 recursive schema；
- 任意 path、URL fetch、shell command、secret/token 作为通用值；
- 远端 MCP annotation 自动决定 effect class。

MCP adapter 测试需要恶意 server：descriptor 前后漂移、tool name 冲突、schema 变更、extra field、巨大递归 schema、伪 readOnly、prompt injection、资源内容带 tool 指令、断流/重复响应、超预算结果。发现变化只能 pending/reject/reset，不得在一个 Run 中热换 schema。

## 5. Runtime 与 OS sandbox

每个可执行 contribution 的支持声明必须逐平台绑定真实 sandbox 证据。最低检查：

- no workspace/authority-store mount；
- no user home/browser profile/SSH agent/system clipboard；
- no inherited arbitrary environment secret；
- child process tree 受控且 cancel/crash 后可确认终止；
- network 默认关闭；只允许 managed transport；
- InputSlot 只读、output/temp 私有；
- CPU、memory、disk、process-count、wall-time 预算；
- symlink/hardlink/device/reparse/path alias 拒绝；
- stdout/stderr 有界并脱敏；
- runtime crash 不改变 author state。

Windows、macOS、Linux 分开验收；存在 container/sandbox 名称不算通过。Server 所宣称的 architecture/platform 也必须逐行有相同等级证据。

## 6. Automation 与 scheduler 实施义务

Scheduler 必须持久化 definition revision、next finite schedule horizon、sourceOccurrenceKey、claim owner、Run link 和实际 skipped/started outcome。首版 serial：。上述技术名称均只表示本文定义的受控边界，不增加额外权限、身份或作者写入语义。

- 同 `AutomationOccurrenceKey/1` 至多一个 active Run；
- enable/disable 不改变 definitionRevision；
- definition semantic change 生成新 revision 和 activation point；
- restart 恢复原 claim，不重算成第二 occurrence；
- run_once 只取当前有限窗口最新遗漏 occurrence；
- rule/source/authorization/ActivationBinding 变化在新 step 前重验；
- clock epoch/continuity 不可证明时暂停，不延长期限。

测试不能只 mock “scheduler returned one row”，必须真实制造双进程、双 Server frontend、crash before/after claim、process pause、clock epoch loss、source/rule generation change 和 enable/disable 竞争。。上述技术名称均只表示本文定义的受控边界，不增加额外权限、身份或作者写入语义。

## 7. Standing Approval 协调实现

UPSTREAM-AMENDMENTS 是 S6 前置条件。实现不得在 amendment 尚未共同接受时把该分支隐藏在 Broker 中先上线。

Core 必须拥有 StandingApprovalEnvelope validator、ApprovalUse builder 和原子 reservation/consume 逻辑。Broker 只能请求“尝试机械批准”，不能提交审批结论。

必须从真实 D7 `set_field_member`、FieldSelection/Narrow Field Qualification、PreparedActionBinding/preview、D6 request/plan 运行完整路径。不能只构造简化 JSON fixture 声称闭合。

核心竞争测试：

1. exactly-one Entry 在 prepare 后变为两个；
2. source A→B→A；
3. approval 最后一次两个 Run 同时 reserve；
4. approval revoke/expire 与 D6 planning CAS 竞争；
5. planning 后 current D6 write permission revoke/regrant；
6. commit 成功但 receipt 丢失；
7. committed raw no-op；
8. process 在 approval reserved、D6 planned、author commit 三个点崩溃；
9. stale preview/cursor/EffectBytes delivery epoch；
10. mutant 在 note/provenance/另一 member/Facet/body 偷带变化。

只有真实 D6 commit 事务同时保存 author result 与 approval consumed 后才算一次自动提交完成。

## 8. External effect 与 Connector 实施义务

每个可写 External Service 必须有具名 adapter profile，声明：

- exact request payload/value types；
- target/account binding；
- current read/write authorization classes；
- secret use；
- accepted success proof；
- failed_no_effect proof；
- idempotency key 生成、server scope、retention/window；
- safe reconciliation query；
- cost model；
- cancellation/timeout semantics。

缺任一项时可保留 read-only connector，mutation 对 unattended execution unavailable。不能用 HTTP method 名或 status 2xx 泛化成所有服务的成功/幂等合同。

send fence 需要 fault injection：durable intent 前、intent 后 send 前、write syscall/HTTP send 中、remote accepted 后 response 前、response 后 terminal audit 前。没有可证结果的一律 outcome_unknown。。上述技术名称均只表示本文定义的受控边界，不增加额外权限、身份或作者写入语义。

Connector sync 若修改 SourceBinding/OriginBinding/watermark 必须另有 owner-stage closed adapter；普通 ExternalEffectIntent 或 single_field_member approval 不得直接写这些控制字段。

## 9. Budget 与费用实现义务

D10 cost engine 必须做多账户原子 reservation，而不是先读余额后分别扣减。至少验证 Run、DelegationLease、Automation、deployment 四层上限，并与 D6 原 work/attempt budget 独立累计。

Money 只能使用同 account currency 与 Counter microUnits。pricing rule 必须版本化并冻结：

- 固定收费；
- token/unit 线性收费时，最大输入/输出/work limit 能推导有限上限；
- 无法给出有限上限的 dynamic/auction price 不提供 hard ceiling。

reservation 状态必须耐久。uncertain 在 billing truth 不可证明前不得自动 release。overcharge anomaly 要冻结 capability 并进入管理恢复；不能修改历史 reservation 使其“合法”。

真实 provider 测试至少包括：success billing、error billing、retry billing、delayed invoice、missing usage、usage disagreement、over-ceiling simulation、currency mismatch 与 crash recovery。。上述技术名称均只表示本文定义的受控边界，不增加额外权限、身份或作者写入语义。

## 10. Audit、retention 与 export

本地/Server protected audit spool 必须先于受保护操作耐久记录 started；remote collector 可以异步。实现必须有：

- append/sequence/integrity 或等价防篡改机制；
- 与 Workspace/principal/Run/step/effect/author request 的受限关联；
- secret redaction 与结构化 allowlist，不用日志后处理猜 secret；
- cancellation/revocation/emergency-stop 的 reserve；
- storage full、permission failure、corruption、collector offline 的不同状态；
- planned/unknown/uncertain 证据的 retention pin；
- 当前 `audit` 权限下的导出，不泄露普通用户不可见 source/secret。

候选不冻结具体 retention 天数；部署必须给有限、可见 policy，并且不能早于恢复/计费/安全 evidence 的最低存活条件删除。

## 11. Capability 与错误 conformance

D1 capability reason 和固定优先级必须从真实部署组合执行测试，特别是：

- policy_denied + missing component；
- policy_denied + offline；
- missing component + not configured；
- offline + incompatible version；
- temporarily unavailable 与 offline 互斥；
- Mobile unsupported_surface 高于后续安装状态。

D10 control error 必须验证同一请求在 hidden object exists/missing 两个世界中仍返回相同 not_visible，直到 caller 已有当前可见资格。D3/D6/D7/D8/D9 原 error 必须逐字沿原 owner 传递，不能被一个通用 AgentError 包装掉。

## 12. 跨表面实施矩阵

| 能力 | Desktop local | CLI local | Server | WebUI | Mobile |
| --- | --- | --- | --- | --- | --- |
| D10 Broker/control | 本地 | 同一本地能力族 | 托管 | 仅 Server 客户端 | unavailable |
| Agent runtime | 能力可用时 | 同一本地能力族 | 能力可用时 | 仅发起/管理 Server | unavailable |
| Automation scheduler | 一个本地 scheduler | 管理同一 scheduler | 一个 authority-domain scheduler | 仅 Server 客户端 | unavailable |
| Connector/model/tool executor | 本地 sandbox/transport | 同一本地能力族 | Server sandbox/transport | 不直接运行 | unavailable |
| Secret management | OS secret store | 同本地 store | Server secret store | 只经 Server 管理 | unavailable |
| Standing-approval author submit | amendment 激活后 | 同 Core | amendment 激活后 | 仅经 Server | unavailable |
| 普通 committed facts | 原 Core | 原 Core | 原 Core | Server Core | 原 Mobile Core/Server |

表中“能力可用时”仍必须满足 D1 release/platform evidence；设计存在不产生 supported 声明。

## 13. 真实实现替换与禁止兼容层

后续实现应成套替换任何现有自由 tool callback、任意 JSON argument、UI 自报 approval、进程继承环境 secret、按 extension/name 自动调用工具、Agent 直写文件或 cursor/provider state 混入 author source 的原型。若旧原型未发布，不保留 serde alias、fallback parser 或双读双写。

历史研究 fixture 可以保留并明确 non-authoritative；public/API/CLI/schema/positive fixture 不得同时接受新旧两套受控语义。

## 14. 测试与证据分层

| 层 | 能证明 | 不能证明 |
| --- | --- | --- |
| D10 静态/文档检查 | 双语结构、controlled names、scenario/reference 完整性 | runtime 正确、安全隔离、真实协议 |
| 有限模型 | 指定状态机、reservation、claim、race 的有限反例 | 完整 Core、OS、provider、任意规模 |
| real Core conformance | D3–D8 decoder/permission/plan/commit/replay 实际组合 | OS sandbox、外部 provider 行为 |
| durable fault model | SQLite/fence/crash/restart/atomic reservation | 真实云/网络服务保证 |
| OS sandbox evidence | 特定 build/platform 的文件/network/process isolation | 其它平台/版本 |
| protocol-service evidence | 具名 MCP/model/connector 的实际 idempotency/schema/billing | 其它 provider 或未来版本 |
| UI/device evidence | Desktop/WebUI/CLI 交互、a11y、状态显示 | Core 内部正确性本身 |
| release evidence | 当前 package/platform/profile 可声明 Supported | 未覆盖组合 |

本候选作者阶段没有运行新的 D10 有限状态机模型，因此不存在可报告的 D10 模型 pass 数。现有 CI 只在提交后按实际结果报告，不能把文档绿灯写成产品 conformance。

## 15. 必须实现的最小 hostile/race corpus

1. prompt injection 请求扩大 read/egress/secret/tool/budget；
2. MCP descriptor/schema drift、伪 readOnly、巨大输出；
3. two-entry same-value standing approval；
4. approval-count N=1 双并发；
5. cost N=1 双并发；
6. revocation 与 context delivery/author planning/external send 三种 race；
7. crash at occurrence claim、ApprovalUse reservation、D6 planned、author commit；
8. external unknown + idempotency window active/expired；
9. credential rotation + unknown mutation；
10. audit local failure vs collector offline；
11. package activation crash/failed upgrade/successor rollback；
12. D4 three-generation semantic revival attack；
13. cancelled Run with prior author committed/external succeeded；
14. dirty D8 Draft + background legal author commit；
15. Mobile upload/Agent/approval negative capability；
16. hidden object exists/missing non-disclosure；
17. provider billing uncertain/overcharge；
18. package disable with author raw unknown namespace retained。

每个 case 同时给正例和 mutant/negative；不能只比较字符串日志。

## 16. 完成门

作者实现计划只有在以下条件都明确记录后才可交独立评审：

- CANDIDATE、TERMINOLOGY、SCENARIO-DISPOSITIONS、UPSTREAM-AMENDMENTS 与本文互相一致；
- 48/48 upstream input coverage 保持；
- D6/D7 amendment 明确为未激活提案；
- 没有把 unsupported/deferred 写成 available；
- 自动 author commit 只限 single_field_member profile；
- External effect unknown、cost uncertain、audit failure、cancel/planned 恢复均有单一规范结论；
- D1 surface/reason、D3 identity、D4 Registry、D8 confirmation、D9 worker/publication 不被暗改；
- 任何实际运行证据精确分层，pending 项不被写成 pass。

这些是候选完整性门，不是独立 Gate verdict。
