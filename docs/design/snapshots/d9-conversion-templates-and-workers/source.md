---
_weftext:
  id: "9e6c35cb-3c31-4c1d-8d64-32a9a10a1021"
---

# D9 转换、模板与 Worker

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


revision: D9-r04-2026-09-24；candidate，未接受、未激活。规范组成：本文、Import IR and Mapping、Templates、Workers and Export、Acceptance Matrix、Terminology and Naming Lexicon、Implementation Impact and Test Outline、Coordinated D3 D7 Binding Amendment。有限研究证据不是产品实现验收。

## 1. 产品选择与范围

用户需要把 PDF、工作簿和办公文档带入资料工作区，也需要用办公室现有软件制作普通文本占位符模板，输出能继续使用的文档和表格。选择 Weftext 拥有的转换协调器、封闭的分类型 Import IR、Core 映射与原事务协议；不要求转换器纯 Rust。默认文件级文档导入是一份文档级 Node，逐页 Node 是明确结构变换，不是提取默认值。

D9 冻结的是架构合同和有界第一代 profiles：共同探测/worker/IR/损失/提案链，页面与工作簿两种不同结构的表示，显式表格映射，普通 Office 模板，Node Template 一次性构造，DOCX/ODT 标题映射，导出发布及资源区域 profile。每种格式/功能/平台仍需独立实现证据才可用。本次没有宣布任何新转换为 available，没有交付生产 worker。

暂不开放：任意脚本或表达式模板；自动富 HTML/RTF 剪贴板导入；Office 公式求值/改写；无条件无损往返；持续外部同步/双向写；ICS 的生产转换 profile；CAJ 全变体支持；未经证明的语义重定位；Mobile 转换执行、委托、监控或批准。后续条款给出其明确归属和失败结果，不能以“通用 IR 支持”替代缺失的格式 profile。

本候选包含明确配套修订：D3模板结果承接与其镜像准备条款、D7 PreparedActionBinding/2、D6/D7效果的版本消费引用。完整变更见配套修订篇；未接受前当前上游仍权威，不能仅激活D9而遗漏这些文件。原wire11/Result9/Action/receipt、阶段顺序和唯一事务权威保持。

## 2. 不变量与上游承接

| ID | 必须保持 |
|---|---|
| U01 | D1 五表面；Desktop/CLI 可承载可选协调器，Server 可承载 worker，WebUI 仅调用 Server；Mobile 连转换委托/审批也排除，只消费已提交普通结果及普通附件。 |
| U02 | D2 Profile2 的唯一 exact source；标题中的冒号不拆副标题，subtitle 来自显式 header；body heading 仅 H1–H5。未知/不可表示构造不可偷偷扩展语法。 |
| U03 | D3 wire11 / Result9 是唯一 durable identity、owner、locator 和 fresh/preserve 权威。IR key、source ordinal、内容摘要、path、Office 单元格坐标不变为 Ref。 |
| U04 | D4 Field 由完整 FieldId、Registry、typed Entry 解释；普通 header 不是 Field；无通用 null、动态 Record schema 或按列名自动类型推断。 |
| U05 | D5 本代没有持久 Record 域。文档行、Field occurrence、Node Collection、Query value rows、附件预览必须显式分域。 |
| U06 | D6 唯一 authority store、PreparedIntent、ledger、预算、授权和原子提交。转换进程不得取得工作区挂载或数据库句柄；设备文件路径不是内容目标。 |
| U07 | D7 只消费 complete 结果；空非 terminal 页不等于 EOF；Query auth generation 变化整体 reset。保持原 ActionSpec 与 D3/D6 request、effects 形状。 |
| U08 | D8 Source/Write/Read 共享 exact Draft；自动生成的提案不能伪称用户原生输入。完整源编辑需要 source_read、workspace_constraints 及实际 footprint 资格。 |
| U09 | 原七种 Native Actions 原形状/gates 保持。column、多行生成、ragged 自动修补和 TSV UI Source 路线仍关闭；不得拆为多次既有 Action 规避。 |
| U10 | 转换结果、Provenance、来源坐标与 preview 不授写入权；只有已确认原请求的真实 D3/D6 receipt 可证明 author commit。 |
| U11 | 所有输入/目标/模板/Provider/版本/映射/损失选择/结果顺序均绑定准确版本；改变后新准备。结果未知先恢复原请求，不能换 OperationId 重做。 |
| U12 | 源/模板不可写，预览零作者写入；准备可写隔离暂存及控制记录，不能把这叫完全零磁盘 I/O。 |

原型公开文档中的旧 Record、YAML envelope、H1–H9、`attr.中文键`等不能覆盖上游。本代没有未发布兼容承诺：未来实施删除旧解析器/别名，不能双读双写。当前私有架构阶段保留产品仓库原样。

## 3. 状态所有者及执行链

```text
显式输入授权 → host 复制/上传 immutable artifact → 隔离 probe
→ 固定 route/options/budget → 受限 worker → Weftext IR 验证
→ Core 显式映射/完整上游 admission → 可审阅精确提案与损失
→ 独立确认 → 原 D7/D3/D6 事务 → 原 receipt
```

导出：Core 当前授权读取/完整 Query → 冻结显式消费范围、ExportContentSelection/1、输入目录及 RenderSnapshot/ExportProjection 或原 D7ResultPin → worker 创建外部 staging → Core 验证准确输出与 ExportLossReport → 绑定同Plan/bytes的明确发布确认 → 当前授权的外部发布或独立 Resource 创建。后台等待不续期权限。原件保留、解析文本和输出是不同产物，保留原件不等于转换无损。

| 对象 | 持有者/寿命 |
|---|---|
| SourceArtifact | host/Server 控制区 immutable bytes + lease/pin；不是 Node、SourceBinding 或身份承诺 |
| Probe / IR / MappingProposal | job 控制区；受预算、audience、TTL 与输入绑定约束，均可丢弃重建 |
| ImportJob | D6 已定义的持久控制记录；固定有限 groups/batches、原请求、实际 receipts；不是第二事务账本 |
| Template | D2 Template Node 及其 Document/明确 Resource；Office 文件也是普通 Resource，不因扩展名自动成为 Template |
| RenderSnapshot / D7ResultPin / StagedOutput | 已授权数据的私有 pin；独立输出发布状态，无 author revision |
| ResourceRegionLocator | 只有 Core 在已提交 Resource 的准确 revision 上签发；IR 区域仅候选 |

上传/本地选择必须复制到工作区外隔离输入后再探测；复制期间前后文件身份/长度不一致则重选，不边读取可变文件边解释。完全相同 bytes 的两次显式输入仍是两个 occurrence，不能按摘要自动去重。输入目录递归、网络下载、URI 获取不在本 profile；用户先提供明确有限文件。Server 不读取客户端任意路径。

## 4. 新接口与原提交边界

以下为 D9/1 业务准备接口的 closed 语义对象；host HTTP/IPC 路由可以不同，不能改变字段或提供另一个提交语义。JSON duplicate key、未知/缺失字段、wrong union、非法 Unicode scalar、非法 null 全拒绝；数字直接用 D6 Counter，禁止 Boolean、浮点、指数、负零。`Token`、`WorkspaceRef`、`EntityTarget`、`BudgetBinding`、Ref/Locator 各用原 decoder。optional 明列；其余必填。

共同 `SourceArtifact` exact `{artifactToken,byteLength,sha256,displayName,originClass}`。token 是 D6 Token 的独立 tag，绑定实际 audience 与 immutable pin；sha256 为完整输入 bytes 的 SHA-256（产品内容绑定，不是身份）。originClass=`local_file|upload|clipboard_plain|generated`，由 host 确认而非 worker 自报；不含原始路径/凭据。displayName 为显示文本，不参与路径拼接。客户端回传 descriptor 必须与 host 保存值相同。

- `d9_probe` exact `{wireVersion:1,kind:"d9_probe",sources:[SourceArtifact],hints,budget}`，sources 非空；hints为按sourceIndex唯一排序的`{sourceIndex,format:"csv"|"tsv"}`数组，可空。纯文本无法可靠自识别，未明确hint时报告unknown，不按扩展名猜。hint只选择严格文本decoder，不能绕过内容验证或将二进制伪装为文本。仅探测这些 bytes，不读作者内容；结果 `d9_probe_result` exact `{wireVersion:1,kind,probeToken,items}`。item exact `{sourceIndex,format,variant,encryption,activeContent,diagnostics}`；format 闭集为 `pdf|image|csv|tsv|xlsx|ods|docx|odt|xps|oxps|ofd|caj|unknown`，variant 为已知 format profile 内枚举或 `unrecognized`，不能自由文本选择可执行程序。encryption=`none|required|unsupported|unknown`，activeContent=`absent|present|unknown`。后两者不是安全证明；生产执行还须 route profile 及隔离。probeToken 保存完整输入与实际证据。
- `d9_convert` exact `{wireVersion:1,kind:"d9_convert",probeToken,routeId,profileId,options,budget}`。本代 options 只有四 variant：`{kind:"page",ocr:"disabled"|"local",languageTags:[text...]}`、`{kind:"grid",sheets:[Counter...]}`、`{kind:"flow"}`、`{kind:"delimited",encoding:"utf-8",delimiter:"comma"|"tab",lineEnding:"crlf"|"lf",header:bool}`。四者与 profile 精确匹配；本代grid.sheets必须为空数组，表示完整读取全部sheet，源索引保持连续；具体sheet/range在Core mapping选择，非空选项unsupported_profile。languageTags在OCR disabled时必须空；local时须非空唯一、按精确BCP47注册语言tag匹配已安装模型，不按locale猜。语言模型必须已安装且许可审查，不自动下载。执行响应只有 `{wireVersion:1,kind:"d9_conversion_started",jobToken}`，完成由状态读取；它没有 commit 成功含义。
- `d9_conversion_state` exact `{wireVersion:1,kind:"d9_conversion_state",jobToken}`。成功 exact `{wireVersion:1,kind:"d9_conversion_state_result",state,detail}`；state=`queued|running|converted|failed|cancelled`。queued/running 的 detail exact `{completedUnits,totalUnits}`（totalUnits 可 null，completedUnits Counter）；converted exact `{irToken}`；failed exact `{diagnostics}`；cancelled exact `{}`。进度不交付可提交 partial IR。转换取消 exact `{wireVersion:1,kind:"d9_conversion_cancel",jobToken}`，返回相同状态对象；converted 之后只能丢弃未使用租约，不能取消已建立的 author request。
- `d9_import_analyze` exact `{wireVersion:1,kind:"d9_import_analyze",workspaceRef,irToken,mapping,budget}`；mapping 使用 Import IR and Mapping 的封闭 mapping。成功 `{wireVersion:1,kind:"d9_import_analysis",analysisToken,lossReport,groups}`。Core形成完整不可变分析，groups为按index的 `{index,newNodeCount,dependsOn}` 数组，dependsOn是更早group index sorted unique；完整不可拆组和所有分区/目标的准确语义保存于analysisToken并可通过完整提案inspect读取，绝不只凭计数确认。没有fresh Node的全omit分析只可结束，不得prepare普通import。
- d9_import_choose exact {wireVersion:1,kind:"d9_import_choose",analysisToken,lossChoices}；lossChoices按lossKey排序唯一，元素exact {lossKey,choice}，一次覆盖全部requires_choice与blocking，notice不得提供choice，blocking只准reject，不能假通过。choice仅accept_loss/reject；reject返回d9_error.cancelled且无新author request；全accept_loss时Core重新验证相同输入/固定映射与完整提案未漂移，保存初始LossReport和完整lossChoices，返回新d9_import_analysis，真实loss保留为已接受notice。依赖变化返回dependency_conflict或原授权错误。选择不改转换语义；要保留原件、flatten、include hidden或omit，必须显式改mapping/construction重新analyze，全部loss接受重置。旧token不原地改，不能以choice重置预算。
- `d9_import_prepare` exact `{wireVersion:1,kind:"d9_import_prepare",workspaceRef,analysisToken,budget}`；仅当前完整授权、依赖不变、无未裁决loss的分析可准备。结果 exact `{wireVersion:1,kind:"d9_import_prepared",importJobToken,batches,lossReport}`；prepared batch exact `{index,state:"prepared",prepared}`，prepared逐字为完整原`d7_action_prepared`对象，含protocolOwner/request/previewToken/previewCursorToken/previewManifest，不重新命名为不存在的preview对象。尚未满足前驱实际Ref/revision的batch是 `{index,state:"waiting_for_predecessor"}`。完整groups、输入、映射与固定后继语义在首次确认前均可取得，但每batch提交前仍需原完整精确preview，不能一次同意未知实际bytes。
- `d9_import_next` exact `{wireVersion:1,kind:"d9_import_next",workspaceRef,importJobToken,budget}`；只在已恢复的committedPrefix之后物化唯一下一batch，返回d9_import_prepared，其batches仅该index，并保持同job固定分组/输入/映射。资格/依赖变化返回dependency_conflict或原授权错误，不暗改已确认语义；相同已prepared下一batch在有效期限内重放同prepared，期限外只能原协议允许的重新准备，planned/unknown先恢复原request。
- `d9_import_state` exact `{wireVersion:1,kind:"d9_import_state",workspaceRef,importJobToken}`。结果 exact `{wireVersion:1,kind:"d9_import_state_result",state,committedPrefix,batches}`；state=`prepared|running|partial|complete|paused|cancelled`，prefix Counter；batch exact `{index,status}`，status=`unprepared|prepared|submitted_unknown|planned|rejected|terminal_failed`，或 `{index,status:"committed",protocolOwner,receipt}`，receipt逐字原对象。committed以外不得带成功数量。完整job状态只在其完整观察资格下交付；receipt若不具原读取资格整体not_visible，不返回遮蔽数组伪称完整。pause/cancel为host job控制操作，仅阻止未planned后续，返回同状态对象；不新增author wire，也不撤销原decision。

格式/语义处理错误统一 `d9_error` exact `{wireVersion:1,kind:"d9_error",code}`，code=`invalid_request|not_visible|source_unavailable|unsupported_profile|encrypted_input|unsafe_input|provider_unavailable|conversion_failed|invalid_output|mapping_required|unrepresentable|template_invalid|dependency_conflict|budget_exceeded|cancelled|result_expired|reset_required|publication_failed|publication_unknown`。先 closed decode→当前 audience/入口授权→authority（若读 workspace）→pins/依赖→格式/业务/预算；不向未获权者泄露版本、格式或详细原因。授权后诊断清单另为 `{code,sourceIndex,location,message}`，location 是 IR 的 SourceLocation 或 null；用户内容不成为诊断模板/脚本。D1 capability unavailable 仍用 D1 原闭集/优先级，不把 d9_error 塞进其 reason。D3/D6 commit errors 保持原对象，不转换为 D9 伪回执。

这些接口不增加 D6 policy capability：入口按 D1 conversion 能力及输入读权、export（导出时）、实际写目标/潜在观察资格的交集运行；导入至少按 D3 所需 workspace_constraints 及完整写集授权。无权不运行昂贵 provider 后再遮蔽错误。

所有D9 token遵循D6词法，但保存独立tag `d9_artifact/1|d9_probe/1|d9_conversion_job/1|d9_ir/1|d9_analysis/1|d9_import_job/1|d9_route_revision/1|d9_export_plan/1|d9_publication/1`；不能与action/receipt/region token互换。完整analysis inspect是host只读业务读取，按source kind返回原IR+mapping或TemplateConstruct+完整来源目录，以及loss/选择、分组目录和完整拟议源/资源列表，先全资格/有限总输出预算后交付；具体流式IPC不在本版增加公共wire，不能复用D7 effects声称已有author准备。实际author preview仍仅原D7。

## 5. 准备证据如何绑定原 D3/D6

转换器只能返回源观察；普通import由Core建立完整source_artifact、ArtifactObjectKey→symbolic subject、Result9、placement/ref/carrier plans。ConversionInput/1的manifest保存准确输入目录、profile版本、目标、初始LossReport/完整lossChoices和全部group/batch语义：文件分支另存完整IR、mapping及固定route/version；模板parent分支另存完整TemplateConstructionInput/1的可逆JSON投影及recipe编译profile。模板投影仅把每个inputPins元素的bytes替换为partOrdinal，元素exact {sourceVersion,payloadKind,partOrdinal}；所有其他成员原样，准确bytes保存在该source part，Core按part绑定恢复原完整内部记录。manifest不内嵌raw二进制，不省略其来源版本，也不把Base64任意数据交给公共API。完整拟议输出另由原D3 result payload pins/Result9绑定和独立重编译证明，不重复制造另一输出权威。输入内容使用产品定义的真实SHA-256；不把reviewer/package/source-map哈希混入。

ConversionInput 不预分配fresh作者 ID。它的对象 manifest 只列本 batch 拟物化对象，ordinal 按明确 sourceIndex、profile 内位置、输出 role 排序，完整声明 fresh Node 与其 Resource/Annotation；不能从 payload digest 合并对象。Core 独立复算 manifest 及映射，不信任 worker 自报完整性。该 part 的任何映射、loss、route、目标变更都会改变原 request binding。原始 source byte parts 本身不被重写为 IR。

物理来源绑定闭合为Core创建的一份immutable `ConversionInput/1` ZIP容器，内部只允许manifest.json、按sourceIndex排列的source/N.bin、按IR resource key排列的resource/N.bin三类entries；ZIP用stored、不加密，无外部关系/links/重复part，套用相同有限容器安全门。manifest保存上段完整规范JSON但不含容器自身SHA，故无自引用。D3的artifactSha256是整份实际容器bytes SHA；partOrdinal固定manifest=0、所有source按index接续、所有resource按key接续。每个实际part按原payloadBindings另有其真实bytes SHA。只有part0有待物化object manifest，其他parts为原始证据且object集合为空；两个相同source bytes仍有各自partOrdinal/输入occurrence，不合并。part0对象按(documentKey, source内部位置, roleRank)枚举，roleRank=node=0/resource=1/annotation=2；同位置同role若多个，追加稳定原resource key或显式选择序号，完整tuple唯一，否则拒绝。rows使用(sheet,row)，document root使用空位置先于内部位置，资源挂接紧随其owner位置。objectOrdinal连续0..N-1；Core再按D3 ArtifactObjectKey唯一规则分配subjects，不能直接沿IR key分配ID。Node Template的parent入口使用同一容器与严格ZIP/part规则，node source位置为recipe node index；source parts按inputPins的RefKey顺序，resource parts按所选Resource RefKey顺序，不按输出摘要。对象位置以recipe index与资源选择序号形成唯一tuple。collection入口不建source_artifact，完整输入只按配套PreparedActionBinding/2保存，原create plan保持合法。

容器只用于本次产品准备，审查材料无此包装要求。本代同一batch的fresh roots必须同destinationParent且形成D3连续插入block；映射含不同parent或不连续目标时规划为不同显式batch并展示，不能在一个普通import intent藏第二destination。所有根的顺序由part0 enumeration与原D3规则确定，不由worker/哈希字典顺序确定。

普通导入至少一个 fresh Node root；其 fresh Resource/Annotation 只能归本次 fresh Node，且 forest 插入按 D3 原 contiguous ordinals。在既有 Node 上保存原件/输出 Resource 必须用 `create_resource`，不能伪装普通 import。只有合法 `create_node|create_resource|create_annotation|ordinary_format import_new` 才使用其已冻结的 existingPayloadEdits；copy/fork/partial transfer 不扩权。D9 第一代普通文件导入不提供自动 upsert、OriginBinding 或 existing Node 匹配；完全相同文件再次显式新导入仍 fresh。D6 checkout/正式 snapshot 的再进入各走原专门流程，不被 D9 猜模式。

author 准备经 D7 `d3_operation` 包住完整原 D3 请求，消费其 PreparedActionBinding、FullPreview、effects；mapping / losses 在上述输入 part 中，受同一准备绑定和 D3 fingerprint 约束。D3 stage14 对真实输入、完整 Result9 和上游 gates 复验。D9不往D3 preparationBinding增加新kind；新受管record按配套PreparedActionBinding/2选择，模板constructionInput完整而普通文件为null。纯既有源变换只允许本代已经 closed 的 D7 Field 或 D8 source adapter；不提供任意 D9 callback 或 generic patch。D8 编辑器仍需明确接收该 generated proposal，保留 producer 来源；旧 Draft 存在时先冲突处理，不能直接 overwrite。

独立确认绑定完整准备结果；CLI 同样提供显式确认材料。收到 request 后只经原 ledger 提交；worker exit0、d9 converted、preview 或 HTTP 200 均不代表 commit。unknown 按原 OperationId/request 恢复。receipt 不可读时显示无法确认，不能以本地计数猜成功。

## 6. 有界导入 Job

完整输入有限冻结后，先建立 D6 定义的依赖图及不可拆组：完整 Template 实例、Node+其 Resources/Annotations、来源绑定（若未来 profile 允许）、promotion 源替换和真正 fresh 相互依赖 SCC 必须同组。单向可先建目标后建引用源；不能因为 input 顺序相反就把无环图强行合成一组。组/批次均已明确列入提案且在确认前计算。

每 batch 最多 1000 新 Nodes，含所有 descendants；更窄 D2/D4/D6 预算取 min。一个不可拆组超过限制即整 job 规划拒绝；不先导入一半，不默拆组。多个独立组可以由用户确认显式 batch 划分；本代默认串行执行，不承诺整个 job 原子。后续 batch 只引用已 commit 前驱真实 Ref 及当前版本，依赖变化暂停并重新预览，不静默修改已确认分组/输入/映射。

每 batch 的稳定 OperationId、protocolOwner、输入和 group binding 在准备时写入 D6 job；同批 author effects、receipt、job 前缀和适用 binding/version/watermark 在原 commit 原子保存。worker 重跑不产生新的 author request；已提交前缀精确重放不重复创建。第六批 commit 前失败保留五批，commit 后丢回执通过原 ledger 得到六批；不能显示零或全部完成。

取消/暂停只阻止尚未 planned 的后续 batch；planned 先恢复原 decision，committed 不回滚。改变 mapping/输入/分组另建显式 job，旧 receipts 保留。清理不可删 planned 所需 pins；未提交临时输出的 TTL 清理由 audience policy 控制，不能删除用户源或 author history。10,000 Nodes、10 batches、大于 working memory 的真实实施验证仍为必需后续门，本轮有限模型不代替。

## 7. D8 后续边界的逐项归属

| 输入/意图 | D9 裁决 |
|---|---|
| 资源区域 | 冻结 PDF/image 区域语义和 token profile；签发必须实际支持该格式的 Core validator，未经实施保持 unavailable。OFD/XPS/CAJ 原件区域暂 unsupported，转出的 PDF 是另一 Resource，不继承原 token。 |
| 富剪贴板 | 本代仍只取 D8 text/plain；没有 plain flavor 返回不可用。不把 HTML/RTF 转为 source。后续需显式转换输入、隔离解析、mapping/loss 和独立编辑 adapter，不能放开现有粘贴默认行为。 |
| CSV/TSV/Office 导入 | 独立文件/显式 clipboard_plain 转换入口可产生 fresh Node/native-table-in-new-Document/D4 mapping提案；不能用于现有 grid 自动批贴。 |
| 模板实例化 | Core独立construction adapter：parent入口ordinary import，受限简单集合入口原collection_create；完整来源用PreparedActionBinding/2，requireMembership完整post Query。不增加持续绑定或新Action。 |
| 模板导出 | 完整 typed snapshot → 外部结果；重新导入是新的明确转换，不保留隐藏 Node/row identity。 |
| 已有完整源编辑 | 原 D8 full scope + workspace_constraints + 实际 footprint；D9 不提供免权通道，窄 Field 仍 D7。 |
| Agent 提升/联网/OCR 外传 | D9 第一代 worker network=denied；AI 增强、凭据、连接器、调度在 D10，未冻结前 unavailable，不能把 agent-produced 标成 human。 |

## 8. 完整替代与接受条件

直接复用第三方 AST 最省适配工作，但把其 schema/版本/未覆盖单元格或身份误带入产品，拒绝。万能单一 paragraph/table 树会丢 workbook 的空位、公式与合并及 PDF 几何，拒绝。每格式各建 Core transaction 增加第二写权威，拒绝。先全部转 PDF 再导入不能满足 Office 结构/表格类型需求，只接受为独立、明确损失的页面 route。模板执行 Query/CEL/脚本或增强 Office 数据模型增加隐藏作者能力，拒绝。选择有限 profiles、分型 IR、纯模板、显式映射与原事务的代价是部分格式/富格式/公式保真尚不可用，必须在能力和 preview 中可见。

独立终审须检验更简单完整替代、上游/命名/语义与 wire 一致、路径/数据权限、事务/重启/副作用、真实格式事实以及全部场景。冻结需要明确 accept/pass、零开放 P0/P1；本地有限测试、厂商列出支持格式和架构文字均不等于产品验收。D10 只在本任务完成后且另获阶段授权才开始。

导出报告域及图片尺寸政策由Workers and Export §3a、Import IR图片尺寸profile与Templates共同约束：导出不借用Import LossReport的地址，不使用宿主DPI默认。原D3/D6/PreparedActionBinding组合不因这两项导出控制修订改变。
