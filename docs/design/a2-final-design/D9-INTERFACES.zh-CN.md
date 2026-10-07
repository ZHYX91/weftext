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

D9 冻结 semantic flow `prepare→inspect→publish/state/cancel`；本 A2 候选**不虚构额外 public export wire envelope**。各 surface host route 可以不同，但必须消费相同 current `ExportPlan/3`、`D9ExportConfirmation/1`、staged-byte 与 publication-intent 语义。

prepare 在返回前完成全部 input/catalog/projection/loss/staged bytes。inspect 只读并重验 current disclosure。confirmation 绑定 exact Plan/loss choices。publish create-only 并绑定原 destination intent。state/cancel 只操作该 publication responsibility，不成为 author commit。

## 8. Export to Resource

把一个 staged dataFile 保存成 Resource 是独立 D7/D3 author operation，消费 exact staged bytes、显式 owner/name 与 current write authorization。Export confirmation 不授 write capability。external publication 与 Resource creation 展示两个不同 outcome/receipt。

## 9. Office-template compiler interface

compiler 消费不可变的 Office template 字节、完整授权 projection 与 Plan policy。它只扫描一次可见 token，编译精确 binding record，验证 style/repeat/layout/safety，并返回 staged output 与精确 loss evidence。它没有 Workspace path、任意 callback 或 permission handle。

qualified native-table authoring 由 compiler 读取可见的 `native.table[...]::column[...]` token。它可以派生内部 `nt_...` / `nc_...` Plan key，但可见 template token 仍是 authoring authority。

## 10. Worker interface

唯一面向 worker 的控制对象是 `WorkerInvocation/1`。slot/handle 由 host 分配。Worker 不能选择 executable、path、network、retry、loss choice、final publication destination 或 author operation。Host 必须验证每个 declared output byte，并拒绝任何 undeclared output。

## 11. Region interface

只有在原 D3 disclosure/currentness gate 允许后，D9 才验证/sign inner `RegionBody` geometry profile。outer `ResourceRegionLocator/l1` 继续归 D3。D9 不提供独立 durable region identity。

## 12. Surfaces

Desktop/CLI local mode 可以协调本地经过评审的 provider。Remote Desktop/CLI 与 WebUI 均调用 Server；Server 不接受 client 提供的任意 filesystem path。Mobile 没有 conversion execution/delegation/approval path。presentation difference、CJK/RTL 与 transport framing 都不得改变 request bytes 或 domain outcome。
