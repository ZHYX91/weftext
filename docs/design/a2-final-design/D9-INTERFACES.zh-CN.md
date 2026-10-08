---
source_language: zh-CN
translation_status: source
---

[English](D9-INTERFACES.md)

# A2 D9 接口

状态：**author-resolved-pending-independent-review**。这些接口不在 D7/D3/D6 之外建立 author submit protocol。

## 1. Public artifact interfaces

D9/1 pure artifact request 保持封闭：
- `d9_probe` → `d9_probe_result`
- `d9_convert` → `d9_conversion_started`
- `d9_conversion_state` → `d9_conversion_state_result`
- `d9_conversion_cancel`

它们只消费 immutable `SourceArtifact`、已安装 route/profile/options 与 budget，不读取 Workspace author fact。probe/conversion result 只是 evidence，不是 author receipt。

## 2. Workspace analysis/import interfaces

current Workspace orchestration 保持 D9/2：
- `d9_import_analyze` 绑定 Workspace、CommitDomain、完整 Frontier、IR、mapping 与 budget；
- `d9_import_choose` 绑定 immutable analysis 与完整 loss-choice set；
- `d9_import_prepare` 返回 `d9_import_prepared`；prepared arm 内逐字是**实际 current D7 prepared outcome**，不是 D9 preview 替身；
- `d9_import_next` 只在 authoritative predecessor recovery 后物化唯一下一有限 batch；
- `d9_import_state` / `d9_import_state_result` 操作 D6-owned ImportJob 与原 receipts；
- `d9_template_analyze` 以 `TemplateConstruct/2` 复用同一 analysis/loss/job 路径。

WorkspaceRef 与 CommitDomain 必须 exact match。static potential scope/authorization 先于 hidden record read。失败后不得 downgrade，不得先读后遮蔽，不得事后扩 scope，也不得把 not-visible/unavailable 当空。

## 3. Current author handoff

D9 自己永不提交 author change。current prepared author operation 通过 D7 current `PreparedActionBinding/4`、current `D3IdentityOperationRequest/13` 或既有 `d6_commit_request/2`、`DependencyProof/3`、current PIntent 与 `EffectManifest/3` / `EffectBytes/3` 交回原 owner。

D3 先执行原 saved/planned/unseen request/input 查找，再执行 concrete payload/effects/currentness 的最终复验，最后只封存原 request。D6 planning CAS/P 仍唯一。D9 wrapper 不得添加 planToken、第二 DecisionKey、第二 receipt 或第二 CAS。

读取大型 binding/PIntent record 前先验证 MinimumMapping。只有 built-in D9 Node-template adapter 能生产非 null 的 D9 construction；任意 public construction kind 都拒绝。

## 4. Error priority

D9 public error 只有在 D1 surface/capability 处理之后才使用 current closed D9 family。顺序为：
封闭解码/version → D1 静态 surface/release capability → 当前 audience/entry authorization → 适用时检查当前 Workspace/domain/P continuity → 精确 pins/dependencies/currentness → format/business validation → budget。

一旦进入 D3/D6/D7/D8，就逐字返回原 owner error。D10 不包装 D9 error。无权 caller 不得获知 format/profile/provider/version、hidden object count 或详细 dependency state。

## 5. Analysis/inspect boundary

完整 analysis immutable，只有通过自身完整 disclosure qualification 后才能 inspect。inspect 可在有限输出预算内返回 fixed IR/mapping 或 Template construction、source/resource catalog、initial loss report、choices、groups/batches 与 proposed source/resource objects；它不是任意 public streaming source API。

任何 fixed semantic input 变化都建立 successor analysis，不原地改 token。

## 6. ImportJob recovery

`d9_import_prepare` 与 `d9_import_next` 不承诺 job-wide atomicity。每 batch 带一个原 DecisionKey/OperationId/request。saved 在 current disclosure 下返回原 receipt/error bytes；planned/unknown 恢复原 request/responsibility；只有 unseen 可形成 fresh plan。

pause/cancel 只阻止尚未 planned 的后续 batch，不回滚 committed batch，也不取消真实 planned author decision。

## 7. Export semantic interface

D9 冻结 semantic flow `prepare→inspect→publish/state/cancel`；本 A2 候选**不虚构额外 public export wire envelope**。各 surface host route 可以不同，但 fresh unseen export 必须消费相同现任 `ExportPlan/4`、`D9ExportConfirmation/2`、staged-byte 与 publication-intent 语义。已记录的 Plan/Confirmation1-/3 family 继续作为精确 recovery input。

每条 export 路径先遵守 §4：封闭解码/version → D1 静态能力 → 现任 audience/entry authorization → 适用的 Workspace/domain/P 连续性，之后才按原操作状态分派。已有 saved/planned/unknown 或 inspect/confirmation/publish/state/receipt 通过 token tag/version 与原控制责任查找真实受保护 ExportPlan，重验精确 pin/dependency/currentness、原 owner/confirmation/恢复责任，不改记录字节。fresh unseen prepare 则不要求已有 Plan：Core 从获权的 D8 Annotation 或 D7 Query 输入私下构造候选 catalog/selection/projection。对于 View，在进入 D7 §7 之前，只允许内部已获权结构定位：所选 viewInput 在实际目录中存在、payload.kind=query_result 且有真实 D7ResultPin；index=99 不存在或选中 field/annotation 类型时直接拒绝候选，不能要求不可能的 D7 调用。选中真实 result 后，D7 §7 保留封闭解码 → result 授权/epoch → 完整结果 → schema → layout/binding → 全量结构/顺序 → domain/numbers → budget → 交付的原顺序；D9 layout/profile/业务错误不能抢在 result 授权、epoch、完整性之前。Annotation 原 owner 门禁也保持。随后 final-FC §6.6.1 的 catalog/payload、selection↔projection/recordPin、View hash/complete_data、target/receipt、canonical、profile、loss 与 budget 关系全部强制执行。现任失权同时携坏 index/layout 时，先返回不披露的原 not_visible/reset，绝不暴露私有细节。首次准备按 FC SPEC §8.3 和既有 D9 workers/export §3a 先核候选 dataFile 与 report/manifest 保留名称冲突，再按原顺序在 token 分配前排序、由 Core 随机分配尚不可公开的唯一 d9_export_plan/4 planToken，先于引用它的 report/manifest/Plan 构造，生成验证 dataFiles、冻结 loss/staged pin 后原子保存完整 Plan，成功后才公开 token；失败不披露。不能新增公共 wire、store、CAS、重跑 Query 或修补受保护字节。inspect/publish/receipt 继续关联现任可披露的原已确认 Plan；print 使用真实 planToken，对 Plan 的 print target/destination、View binding、实际 output 与 loss 逐字核对（回执没有 target 字段），Resource author 和 server download 仍独立。

## 8. Export to Resource

把一个 staged dataFile 保存成 Resource 是独立 D7/D3 author operation，消费 exact staged bytes、显式 owner/name 与 current write authorization。Export confirmation 不授 write capability。external publication、print delivery 与 Resource creation 展示三种不同 outcome；三者互不替代。

## 9. Office-template compiler interface

compiler 消费不可变的 Office template 字节、完整授权 projection 与 Plan policy。它只扫描一次可见 token，编译精确 binding record，验证 style/repeat/layout/safety，并返回 staged output 与精确 loss evidence。它没有 Workspace path、任意 callback 或 permission handle。

qualified native-table authoring 由 compiler 读取可见的 `native.table[...]::column[...]` token。它可以派生内部 `nt_...` / `nc_...` Plan key，但可见 template token 仍是 authoring authority。

## 10. Worker interface

唯一面向 worker 的控制对象是 `WorkerInvocation/1`。slot/handle 由 host 分配。Worker 不能选择 executable、path、network、retry、loss choice、final publication destination 或 author operation。Host 必须验证每个 declared output byte，并拒绝任何 undeclared output。

## 11. Region interface

只有在原 D3 disclosure/currentness gate 允许后，D9 才验证/sign inner `RegionBody` geometry profile。outer `ResourceRegionLocator/l1` 继续归 D3。D9 不提供独立 durable region identity。

## 12. Surfaces

Desktop/CLI local mode 可以协调本地经过评审的 provider。Remote Desktop/CLI 与 WebUI 均调用 Server；Server 不接受 client 提供的任意 filesystem path。Mobile 没有 conversion execution/delegation/approval path。presentation difference、CJK/RTL 与 transport framing 都不得改变 request bytes 或 domain outcome。
