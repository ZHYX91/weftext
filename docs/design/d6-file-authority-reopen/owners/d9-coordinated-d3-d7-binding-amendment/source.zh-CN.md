---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：6375684c-8a5e-4c9e-ae16-c840233f77cb。

候选状态：D9 文件权威协调后像，未独立接受、未激活、未实施。以固定 S 7e18168dad3e6d120fce0dd607dc10fa7894e252 的完整本篇为来源，保留其能力、限制、场景与实际历史证据边界；历史阶段标签不决定本轮状态。当前跨域来源、D3/D6 单决议及 D7 准备/预览消费按同包真实 owner 协调。PL-IR-01 的便携 Locator 资格与尚未接受的跨 owner producer 仍为具名整合门，不因本篇已落盘而关闭。

# D9 配套 D3/D7 准备绑定修订

本篇是固定 S 配套修订的完整当前消费后像。保留原模板身份规则、准备证据、先验授权、symbolic namespace、单 planning CAS 与真实历史恢复；当前 D7 准备记录由其唯一 owner 定义，不再复制一份随时间漂移的完整 schema。D9 仅拥有本篇明确的构造输入和消费约束，没有第二身份分配、ActionSpec、提交或 ledger。

## 1. D3 模板结果的完整承接

Template 是 coreKind=template 的 Core meta-kind。实例化始终产生 fresh Node/Resource/Annotation refs，原身份结果严格按实际 D3 mode：copy/fork/identity-bearing import 返回其完整 identityMap；create 或 ordinary-format import 的 identityMap 为空，返回完整 resultAllocations 及适用 placements/reference 证据。D9 可从完整来源到 symbolic subject 的准备证据与原 resultAllocations 机械关联显示来源到实例，但不得增设第二身份权威或改写 receipt。

Task Template 只可由 D9 target plan 声明 targetCoreKind=ordinary 且 target facets 包含 exact tasks/task；Template 自身禁止因此成为 Task。每个真实 typed slot 按该 mode 的 fresh result/identityMap 规则完整处理；Facet/carrier/entry occurrence 不成为 reference slot。当前 Node Template 首代仍不复制 Annotation；上面的完整 mode 规则不扩大该 profile。

## 2. D7 唯一准备 owner 与 D9 输入

当前完整 PreparedActionBinding/3、MinimumMapping/3、D7DefinitionInput/2、D7ProposedInput/2 及其 closed 成员、排序与 pins 由同包 D7 Prepared Action Binding §5 唯一定义。本篇不把固定 S 的 /2 镜像重复注册为另一 current decoder；真正旧 /1、/2 记录继续 §5 的历史规则。

当前 constructionInput 只能为 null 或 D9 Templates §6a 的完整 TemplateConstructionInput/2。非 null 只由 Core 内置 D9 node-template adapter 建立，只用于其 ordinary-import d3_operation 或受限简单 collection_create；公共 ActionSpec/d7_action_prepare 不收追加 construction JSON。它与 D3 resolutionInput 互斥：D9 构造时 resolutionInput=null，最小映射 resolutionAccess=null。文件普通 import 的 constructionInput=null，其输入/IR/mapping/loss/route 全部仍由主文 ConversionInput/2 绑定。

TemplateConstruct/2 使用真实 SourceVersionRef/1 选择当前 Template、recipe 与资源；持久 Recipe/2 固定完整 managed SourceVersion/2，不能跟随 latest 或保存当前 token。inputPins 的 sourceVersion、observation、payloadKind、pin 与实际完整 source bytes、生产域和当前观察逐项相等。omittedAnnotations 只保存真实同 cut 的受权 D9EntityVersionAddress/2 目录，不要求读取 Annotation body。Templates §6b 的显式旧配方升级、原始 fixed-source 含义及 PL-IR-01 gate 全部适用。

Core 独立从完整 recipe/源/pins 重新编译，完整初始 loss/choices、sourceSubjectBindings、参数、资源选择、proposedInputs 与原请求逐项相等；不信任 worker、调用方 AST、payload hash 或 preview 采样。所有实际源读取的完整观察进入原 InputDescriptor/2.sourceInputs；其 exact {entityRef,observation,role} 项只用 before/dependency，不塞 artifact/私有 role/冲突安装 wrapper。proposedInputs 保存原 D3 Result/9 或合法具体 payload 的真正 typed pins；input descriptor owner 始终为 d3_identity_operation/12。

definitionInputs、Registry/规则、canonicalCallInputs、全部正负依赖、pre/post Query、完整 observation scope、BudgetBinding/1 和 preview 按 D7 的实际当前类型保存；没有使用才为空，不填假 context。scope 是读取上界，不是任意源/写入权。constructionReadRefs 按完整 RefKey 排序唯一，恰覆盖实际构造输入与省略目录涉及的 Ref；当前 /3 最小映射的资格按真实用途验证，源 bytes 的真实使用者需相应读取资格，仅省略目录项需完整 state/annotation 目录披露，不自动扩成 body 读取。先映射与当前权限，再大记录/隐藏内容；未知 token、wrong tag/audience 或无权统一 not_visible（D3 按原 identity_not_visible 映射）。

准备的保存次序保持随机 bindingToken→构造包含该 token 的完整原 request→原子保存完整 record、minimum mapping 和全部 pins→返回 prepared。没有 token 从包含自己的 request 散列的循环。完整相等比较 canonical request、完整规范对象和 byte pins；相同 hash 不替代来源或连续性。任何一个 pin/字节 slot 缺失都不得返回 prepared 成功。

## 3. 原生 D3 提交、候选与单 P seal

当前请求为原 D3 identity_operation_request wire12；preparationBinding 仍仅 {kind:"d7_preparation_binding",bindingToken}。该字段按原规则进入 fingerprint，顶层 operationId 按 D3 原规则排除；完整 key 由 Workspace、CommitDomain、OperationId 组成。同 key 改 token/输入/条件仍是原 operation_id_conflict，没有新的 D9 操作 key 或提交 RPC。原生 D3 没有 D6 planToken、PreparedIntent 或 expectedDomainFenceToken。

stage1 closed decode；stage2 原 Workspace roles；stage3 当前 audience/原 scope、minimum mapping 与真实 constructionReadRefs 用途授权；stage4 及原 P/TL 顺序证明域/authority/P/custody 连续性后，stage5 才比较原 key/fingerprint 并分 saved/planned/unseen。当前源/完整 Query、construction、写权和 TTL 新业务门只属于 unseen，不放在旧决议查找之前。saved 只在原当前披露资格下返回原 receipt/error bytes；planned/unknown 恢复原 request/plan/record/pins/预算/安装责任，不重新 Query 或选择来源。

unseen 按原阶段解码全部输入并验证实体、typed slots、完整 D2/D4 源与 Result9。在原 stage14 的 identity_map_incomplete 检查之后，才验证完整 D7 pre/post 与 construction 证明，已证明的业务失败按原 operation_precondition_failed recorded rejection；未能证明 authority/pin/连续性则走原 availability，不把未知写成确定失败。原 stage15 inbound/结构闭包仍完整执行。不能先运行昂贵/有披露的构造，再反过来挑授权 scope。

prepare 不领取或烧毁最终 content IDs。fresh 来源到目标保持原 symbolic subjects；原 D3 stage12 的唯一私有 candidate map 及 winning planning reservation 分配实际身份。collection requireMembership 的完整 post Query 在同一绑定 symbolic namespace 求值，完整 sort/take、QueryRef、scalar 和全部正负依赖保持；fresh equality/NodeRef 输入仅由 Core typed 环境提供，不用 fake UUID 的样本充证明。stage14 用最终 candidate map 重验相同完整语义；1000 是所有 descendants 的每 batch 上限，不能 preview 截到 200 条就减小条件。

一个原 planning CAS 同时保存完整原请求、/3 record、source/control/authorization/Query 依赖、scope、真实 candidate、pins、SourceRevisionPlan/1、预算与原 reservation。普通或 fresh source 的 H/空历史来自实际生产域；raw no-op/未变 source 不造 plan，externalSequence 不当内层 revision。只有同 P seal 可生成该真实 portable decision 的 ChangeId、实际 managed SourceVersion/2、原 receipt/effects/companion 和 Job 前缀。D9 没有 conflict-only SourceRevisionPlan/2 消费资格，也不从文件像 after 推断 seal。

## 4. D6 与完整效果运输

D9 普通 import、parent 模板、合法 collection 模板和另存 Resource 均走其原 D3 mode。其它本来由 D7/D8 拥有的既有源变换仍由它们的真实 adapter 处理，D9 不提供 generic patch。真正 D6 request 才由 planToken 选择 PreparedIntent/2 并完整保存原 D7 /3 或 D8 PreparedEditBinding/2；D3 本身只走原 native 请求。这两条路径共用同一个 DecisionKey/P 决议边界，不能另设 D6 identity submit。

当前 full preview 与 committed effects 由 D7 Preview and Effects Transport 的完整 EffectManifest/2、当前 runtime2、EffectBytes/2 及其受保护 pin producer 提供。header 的完整 DecisionKey 与原 request 相等；全部语义 items、精确字节 slots、完整 source/production/current 关系、依赖与初始交付投影在第一页前已固定，取页不再执行 Query/D4 gate 或补效果。handle 重签仅改变交付 epoch，不改变原 request/semantic manifest/pins。

D8 producer 仍仅 protocolOwner=D6、profile=full，使用其真实 PreparedEditBinding/2 和 d8_edit_prepared/2；不能伪造 D7 ActionSpec/bindingToken。D9 prepared batch 嵌入完整原 d7_action_prepared/2，而不是仅一个 header/pin/摘要或不存在的 preview 对象。任何当前完整 producer 尚未落实或未获联合接受，只门控依赖它的新 unseen 成功，不回溯取消真实 saved/planned/unknown。

## 5. 寿命、历史和协调边界

未被决议引用的准备按真实有限 TTL 失效，保留原最小 token/tag/audience/scope/expired 定位责任至原规定期限；当前不可用不是随意销毁全部证据。任何 planned、saved、unknown、publication 或其它原 last-reference 持有的完整 record/mapping/pins 继续其原保留责任，不受 preview TTL、I 重建或当前版本升级清理。预览失效只影响可用交付，不改变作者 commit 事实。

真实 PreparedActionBinding/1、/2、旧最小映射、TemplateConstruct/1、TemplateConstructionInput/1、ConversionInput/1、Recipe/1、D3 wire9/10/11 与旧 D6 请求均以其原 decoder、fingerprint、授权/custody、期限、pins 和 byte-equal receipt 恢复。没有部署证据的历史候选不因此变成额外活跃兼容层；反之，确有保存记录时责任不能被当前文档取消。禁止补新字段、换 token、重编译、按当前 source 回填旧 after 或新 OperationId 重做 unknown。

当前新模板只由 /2 construction/recipe 与 /3 preparation 的实际 owner 联合生产。旧尚未形成决议且真正过期的准备可以走明确新分析；一般未提交旧配方保留 Templates §6b 的显式“按明确来源另建配方”正向入口，不能用同 Counter 猜迁移，也不永久禁用普通合法模板。

本次不再把 D3 固定 S §21B 镜像复制成 current 主定义；D3 当前 §21.1 精确引用 D7 owner，D6 Control/Storage 与 D7 效果消费相同实际版本。D3/D6 IR-06/07 后像已存在但仍待独立复核，D7 当前 producer 仍在协调，PL-IR-01 不在本篇裁决。完整 source/version/pin/preview 与旧恢复门必须同批独立接受才可激活；字节哈希、作者检查或本篇引用不能代替该结论。
