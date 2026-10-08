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

一旦进入 D3/D6/D7/D8，就逐字返回原 owner error。D10 不包装 D9 error。无权 caller 不得获知 format/profile/provider/version、hidden object count 或详细 dependency state。 D9 自有的现任 Workspace 业务错误只采用原 §4a wire2 d9_error {wireVersion:2,kind:"d9_error",code} 闭集。获权后请求方选错 inputIndex/payload kind 才是 invalid_request；已认证受保护记录的真实矛盾经过证明才可为 integrity_conflict，证明缺失用 proof_unavailable，来源不可得用 source_unavailable。请求方 View catalog 选择错误不能以任何 owner 的 invalid_output 替代；但原 worker 输出校验、Host/Core 独立的 Import IR 格式覆盖验证（IR schema 合格却漏 XLSX sheet/隐藏对象，S33），以及 D9 Region 新签发格式/几何校验仍保留各自适用的 invalid_output 和原错误信封；其它错误也不能一概归 invalid_output。历史 D9/1 错误信封不变。

## 5. Analysis/inspect boundary

完整 analysis immutable，只有通过自身完整 disclosure qualification 后才能 inspect。inspect 可在有限输出预算内返回 fixed IR/mapping 或 Template construction、source/resource catalog、initial loss report、choices、groups/batches 与 proposed source/resource objects；它不是任意 public streaming source API。

任何 fixed semantic input 变化都建立 successor analysis，不原地改 token。

## 6. ImportJob recovery

`d9_import_prepare` 与 `d9_import_next` 不承诺 job-wide atomicity。每 batch 带一个原 DecisionKey/OperationId/request。saved 在 current disclosure 下返回原 receipt/error bytes；planned/unknown 恢复原 request/responsibility；只有 unseen 可形成 fresh plan。

pause/cancel 只阻止尚未 planned 的后续 batch，不回滚 committed batch，也不取消真实 planned author decision。

## 7. Export semantic interface

D9 冻结 semantic flow `prepare→inspect→publish/state/cancel`；本 A2 候选**不虚构额外 public export wire envelope**。各 surface host route 可以不同，但 fresh unseen export 必须消费相同现任 `ExportPlan/4`、`D9ExportConfirmation/2`、staged-byte 与 publication-intent 语义。已记录的 Plan/Confirmation1-/3 family 继续作为精确 recovery input。

每条 export 路径都先按 §4 执行严格封闭解码/version → D1 静态能力 → 现任 audience/entry authorization → 适用的 Workspace/domain/P 连续性，然后才按原操作状态分派。已有 saved/planned/unknown 或 inspect/confirmation/publish/state/receipt 按原 token tag/version 与控制责任查找真实受保护 ExportPlan，保留精确 pin/dependency/currentness、confirmation、字节及恢复责任；fresh unseen prepare 绝不要求预存 Plan。首次准备先按所选 D9ExportInputDomain/2 固定原潜在 ObservationScope，再按 final FC SCHEMAS §6.6.1 的唯一八域分派表取得实际消费的原 owner 输入：document 用 source_read 与真实 SourceVersion/Observation/source pin，resource 用 resource_read 和原 Resource bytes/pin，native_table 用获权 D2 table 与实际选中的窄 D4 Field/Resource，node_collection 消费真实完整 Collection/Query producer 及所选窄 Field，query_rows/query_json 消费完整 D7 result（graph query_json 保留原 TerminalSchema graph 列绑定、仅含 {nodes,edges} 的 data，以及全部实际 node/edge 列和 typed V；不要求 ViewSpec.nodeDetails），annotation 消费现任 D8AnnotationReadResponse/1，view 则选真实 D7ResultPin/ViewSpec 并进入原 D7 View §7。field 不是第九个 inputDomain。七种封闭 catalog payload（document、resource、field、annotation_index、annotation_content、template、query_result）以及 template/route/style/asset/resource 依赖只在实际消费时分别授权。无 Query 的精确 Source/Resource/Field 读取不得无端增加全 Workspace Query 或 Annotation 要求；generationPolicy=none 的 Source/Resource/query_json 不受无关 renderer/profile 限制。

View 只有完成公共入口授权，才能在私有目录中定位 viewInput 存在、所选 payload.kind=query_result 并且有真实 D7ResultPin。若已获权请求方选中不存在的 index=99，或选中 field/annotation_content 而不是 query_result，D9/2 Workspace 原封闭 wire2 错误为 {wireVersion:2,kind:"d9_error",code:"invalid_request"}；因为没有所选 result，不能调用 D7，也不能伪造 D7 错误。若已认证的旧受保护 Plan 中确有可达记录与 pin，且经过授权证明 index/kind/record/pin 自相矛盾，才可返回原 wire2 integrity_conflict。pin/proof 缺失或无法证明归 proof_unavailable；真实 Source/Observation 不可得归 source_unavailable；原 domain 连续性缺失按实际情况归 domain_unavailable 或原 owner 恢复结果，不能虚构数据损坏。invalid_output 不能用于这里的请求方 View catalog 选择错误；它继续适用于原有各输出验证路径，包括 worker 校验、独立 Host/Core Import IR 格式覆盖（XLSX IR schema 合格却漏已选 sheet → invalid_output，S33），以及 D9 Region 新签发的格式/几何校验。不得将其它 D9 错误一概重映射为 invalid_output。当前失权叠加坏 index/layout 必须在 D9 私有诊断前返回原不披露的 not_visible/reset；D1/封闭解码优先级、已进入 D3/D6/D7/D8 的原错误不变。有真实所选 D7 result 时，View §7 仍按封闭解码 → result 授权/epoch → 完整结果 → 精确 schema → layout/binding → 全量结构/顺序 → domain/numbers → budget → 交付的原顺序，D9 renderer/profile/业务不能抢先。

原 owner 门禁通过后，final FC §6.6.1 的 selection↔projection/recordPin、View hash/complete_data/无障碍、target/destination/print 回执、canonical asset/pin、route/loss/budget 全部仍必须校验。首次 §8.3 与原 D9 workers/export §3a 要先核完整 dataFile/report/manifest 名称冲突，再由 Core 在构造引用它的 loss report、manifest、Plan 前随机分配唯一尚不公开的 d9_export_plan/4 token；生成验证完整 dataFiles，冻结精确 staged output/proof 并原子保存完整 Plan 后才披露 token。不新增公共 wire/store/CAS，不能将不可读当空、补读/重跑 Query 或修复已冻结字节。Print 只按真实原确认受保护 Plan 检查 Plan 的 print target/destination、View binding、实际 output 与 loss，不增加 receipt.target；Resource author 与 server-download 仍分离。

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
