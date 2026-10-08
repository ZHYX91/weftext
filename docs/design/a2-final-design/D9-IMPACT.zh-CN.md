---
source_language: zh-CN
translation_status: source
---

[English](D9-IMPACT.md)

# A2 D9 实现影响与测试轮廓

状态：**author-resolved-pending-independent-review**。本文列实现义务，不是实现证据。

## 1. 替换边界

Core 继续保留 D2/D4 的领域解释以及 D3/D6 的事务权威。conversion、Office、XML、model 与 runtime dependency 留在可选 coordinator/worker package。实现时必须把未发布 legacy ImportIr/YAML proposal decoder、free provider command/fallback alias、旧 attr/record/H1–H9/formula-reorder/broad-view template parser、free export dictionary、rowHandle identity 与 D9-private Locator alias 作为一个 migration set 清理；historical research file 只保留历史证据。

current implementation 必须面向 D3 wire13、PAB4、Effect3、D2Snapshot3 与 fresh ExportPlan/Receipt4。真实已存 Plan/Receipt1-/2-/3 与其它 historical record 继续使用 exact decoder，不能双写或迁移。

## 2. I01–I12 gates — 全部 UNRUN

| Gate | 必需证据 |
| --- | --- |
| I01 IR/codec | 完整严格解码器：覆盖封闭成员/联合体、重复/未知/null/Unicode/Counter/顺序/预算、格式覆盖，并与独立 parser/render 结果对照。 |
| I02 hostile files | 覆盖 ZIP 名称别名/路径穿越/link/device/压缩炸弹、ODF 重复项、XML DTD/XXE/entity、active/encrypted/unknown variant、恶意 stdout/slot、exit-zero truncated output 以及全部限制。 |
| I03 OS sandbox | 对具名 Windows/Linux/macOS build 提供 file/network/process-tree/resource/cleanup 隔离证据，不能只凭 timeout 或 container 名称。 |
| I04 mapping/admission | 覆盖完整 D2 解析/产物、D4 Registry 的类型、限定符、基数、引用与控制校验、fresh-root 权威，以及 ConversionInput/Result9/PAB4/Effect3 的字节闭合。 |
| I05 real ImportJob | 覆盖 10k Nodes、10 个 batches、超过内存的输入、SCC/coupling group、真实 D6 commit 前后 fault、lost receipt、restart、revoke、cancel，以及 TTL/pins。 |
| I06 Office template | 使用真实 Word/WPS/LibreOffice DOCX/ODT/XLSX/ODS，覆盖拆分/混合样式、escape、非法 XML scalar、类型化 none/complex value、可见 native selector、0/1/N repeat，以及 merge/limit。 |
| I07 document output | 覆盖 H1–H9 现任 source semantics、target loss、body/bibliography 单一放置、style bundle，以及 CJK/RTL/AT/font/pagination 的具名 Office 版本证据。 |
| I08 typed export | 覆盖 D7 各 result domain、reset/order/bag/tie/none/graph、任意精度值、date/instant、公式注入反例与 typed spreadsheet output。 |
| I09 publication | 验证 create-only 文件系统边界；覆盖 ENOSPC、名称冲突、用户移动、flush、rename 前后 crash 与 revoke race；unknown 状态不得换名重发。 |
| I10 region | 覆盖 PDF 的 Crop/MediaBox/UserUnit/Rotate、EXIF、density/rounding 等几何与图像元数据，d9rg1 + l1 的现任性、copy/fork、stale/not-visible，以及键盘/AT 交互。 |
| I11 surfaces | 覆盖 Desktop/CLI/Server/WebUI 的语义一致性、Mobile 负向能力、D1 overlap reason priority、非泄露 error，以及 D8 generated proposal 与 Draft 的冲突。 |
| I12 release/naming | 覆盖完整依赖/SBOM/许可/模型/字体/平台安装卸载、无 provider/config 后门、历史名称扫描，并把 capability 与真实 gate 绑定。 |

某个 profile 通过只开放精确 format × operation × variant × route × platform × version 组合。library/profile semantics 变化必须重跑受影响 gate。

## 3. Mandatory §14 Office binding 实现

D9 Main/Schemas 的 visible qualified selector grammar 是实现义务。compiler 必须从 template bytes 解析 exact JSON-string component 和 canonical occurrence suffix，编译成 D9NativeTableSelector/1，强制 shortest-unique qualification，并验证一个 repeat band 的全部 token 指向同一 table/rowset/order。

fixture 必须覆盖：
1. 相同 leaf 的重复 table；
2. multi-row header 的 shortest suffix；
3. 相同 title + full path 仍需 table/column occurrence；
4. CJK、RTL、combining、emoji、space、slash、double-colon、quote、bracket；
5. repeat band same-source unification；
6. 后续碰撞使 fresh selector ambiguous，但 frozen Plan 保持；
7. ordinary Document table、schema-bearing collection、ordinary no-template XLSX 三域；
8. copy/move/style 与 stale template；
9. no-template XLSX 正向；
10. Mobile unavailable、corrupt/malicious input 与 stale authority。

hash-derived nt_/nc_ 只测试为 internal Plan key 或 genuine historical authoring，不再是新的 qualified user spelling。

## 4. Worker 与 provider 证据

Docling Lite 的现任 install evidence 中 completeForExecution=false，因此继续不可用。XPS/OXPS、OFD、CAJ/HN 不能从候选 library、extension recognition 或其它 variant 继承 availability。每个 provider record 都必须绑定精确 dependency version、license/distribution decision、model/font asset、sandbox evidence 与 corpus test。

Worker budget 在 route step/retry 间累计。cleanup 必须证明整个 process tree 已终止；不确定 cleanup 隔离 output 并 disable route。Worker output 在 pin 前独立 decode/validate。

## 5. Import 与 mapping 证据

CSV fixture 覆盖带引号换行、双引号、重复表头、不齐记录、空文件与精确 Unicode。Workbook fixture 区分 numeric lexeme、blank/absent/empty、formula/cache、隐藏 sheet/row/column、merge anchor/covered cell 与源坐标。Page/flow fixture 保留完整几何、阅读顺序与未表示问题。只要 coverage 有漏项，即使 IR decoder 通过也必须失败。

D4 mapping test 使用真实 Registry definition，覆盖 type/qualifier/cardinality/relation/Calendar 正反例。不得按 label 映 Field，也不得让 fresh root 借 existing-owner 权限。

## 6. Node Template 证据

测试必须覆盖重复 source、同模板/跨模板的 fresh subject 重写、owner-local Resource remap、external-current reference policy、只枚举 index 而不读取 body 的 Annotation omission、参数类型、title/body_text/field_append 的重叠，以及 D2 重解析相等性。

parent-import 与 simple collection 分支按原 D3/D7 request 测试。sourceSubjectBindings 与 receipt resultAllocations 必须唯一 join，不保存第二 identity map。

## 7. PAB4 与 recovery 证据

fresh current 测试使用 PAB4 与 wire13 mode-legal shape。旧 PAB1/2/3/wire11/12 只测试 recovery，不静默升级。读取大 record 前证明 MinimumMapping。page 1 前已有完整 preview bytes/effects，pagination 零 semantic rerun。

saved/planned/unknown 三类恢复状态必须保留原始请求、OperationId、固定引用与原 owner；它们不得依赖已过期的预览 TTL 或现任业务校验，只有现任披露/保管边界本身要求时例外。

## 8. Export 证据

测试必须覆盖：首次 unseen exact-source/resource 以及完整 graph（原 TerminalSchema 列绑定、data={nodes,edges} 与全部实际 typed 列/V/顺序）、rows/scalar query_json 使用 generationPolicy=none，不要求预存 Plan，也不依赖 renderer registry；随机不公开的 token 必须先于引用它的 report/manifest，只有完整原子保存的 Plan 才能返回；渲染文档使用 D2Snapshot3 与 D8 呈现绑定；显式选择 body/bibliography；窄 Field/Query/native_table 不得额外读取 body；完整保留 graph/scalar/rows 值；集合按 canonical 规则排序；现任输出名执行 validity、PortableAlias 与 reserved-name 检查；fresh Plan4/Receipt4，以及 Plan/Receipt1-/2-/3 的严格历史分派。 完整分派回归必须逐一覆盖 document/native_table/node_collection/query_rows/query_json/resource/annotation/view 八域和七种 catalog payload，包括不属于公开 inputDomain 的窄 field、只用于 omission 的 annotation_index、annotation_content，以及仅在实际消费时要求各自权限的 template artifact/resource/route_asset。尤其首次 asciidoc_source 的 Document 有 source_read、完整 SourceVersion/2、现任 SourceObservation/1 和精确 source pin 时，即使没有 Annotation、Query 或预存 Plan 也必须成功；首次 resource_exact 取得 resource_read 与精确 bytes/version/Observation/pin 后同理。原读取权限缺失、source bytes 不可得、Observation 过期或 pin 错误均须拒绝，绝不能虚构 Query 或静默扩大 scope。native-table/Field 保留真实 D2/窄 D4 门禁，Collection/Query 仍需完整 result、正负依赖与 graph 的真实列/V 与原顺序；route/asset 只在使用时核查。已获权 graph 有孤立 node、合法端点/关系 schema、edges=空时，即使没有 ViewSpec 或可选 network ViewSpec.nodeDetails，仍必须以 generationPolicy=none 的 query_json 成功导出。若 nodes 实际有 displayName 列，则每个值、出现次数都必须保留；漏该列、分页截断、缺 nodes/edges 或改表顺序均失败。query_json graph data 只含 nodes 和 edges；ViewSpec.nodeDetails 只是可选 network 展示设置，不是 Query 数据权威。

initial loss report、data bytes、loss-report.json、manifest.json、stagedOutputs 在 confirm 前冻结并验证。Annotation selection/projection 只在 freeze 前按 inputIndex canonicalize 一次；View renderer assets 使用 owner 定义的 role/id/version/pin comparator；renderer evidence pins 按 pinToken 排序。frozen/received/recovery record 若 array 非 canonical 必须拒绝，不能读取时排序。D7 result row 与 Annotation disclosure fragment 是有语义顺序的 sequence，绝不能全局排序。confirm 后任何变化必须失败，不能 rerender。

## 8a. Annotation 内容与 Mandatory §15 View 证据

Annotation conformance 必须覆盖：首次 unseen 的 *Alice* 完整 Value/4 便携备份，不要求预存 Plan，使用真实获权 D8 carrier、在 report/manifest 前分配未公开 token，并原子保存 Plan；还要覆盖普通可读 Value/4 导出；最小 target-hidden 反例，其中 R6 body/attribution/reply 仍可导出而 target context 为 unavailable；portable backup 的 canonical record bytes；source-history 与 target-context 独立 disclosure；revision/Observation/record-pin 改变导致 stale；pin 矛盾；授权丢失；以及精确 saved/planned/unknown 分派。还必须覆盖 final-FC §6.6.1：两个已获权 Annotation input 按 [0,1] canonical order 选择时成功；[0,1,0]、duplicate/cross-mode selection、portable_backup selection 却配 review_bundle_r6 projection、未选择却出现 projection，或 portable backup 任一 context flag=true，都必须在 reread 前拒绝。具有语义顺序的 disclosure fragment 不能为了 canonicalization 被重排。便携备份 A/PA 只有在 projection.recordPin 等于选中 A 的 payload.recordPin 时才成功；若选择 A 却在同一 index 投影 PB，即使 B 和两份 pin 都获权且在并集中也必须拒绝。入口失权又叠加错 pin 或坏 index 时，应先返回不披露的 not_visible/reset，而不是关系错误细节。annotation_index 必须作为 body source 失败，也不能用 D7 annotation_body semantic string 替代完整 record carrier。

View conformance 必须覆盖全部 D9 适用的 Mandatory §15 fixture，不能只增加一个 bar 示例。首次 bar(state,task_count) → PDF 使用真实获权且正确选中的 query_result D7ResultPin，经过 D7 §7 与 Plan 冻结成功，不要求预存 Plan。首次获权请求选中 viewInput=99 而 catalog 无条目，或 payload.kind=field/annotation_content 而非 query_result 时，必须在进入 D7 前返回原 wire2 d9_error.invalid_request，因为根本没有可选的 D7ResultPin。相同失配若来自真实已认证的 retained/frozen Plan，仅在可达的记录/pin 证据证明矛盾时才可用 integrity_conflict；证明/来源不可得分别归 proof_unavailable/source_unavailable，域不可用也要保留原处置。invalid_output 不能用于该请求方 View 目录错误，但绝非只能用于 worker：原 D9 Import IR §2 Host/Core 独立格式覆盖校验即使 worker 成功、IR schema 合格，发现 XLSX 漏了 sheet 或隐藏对象仍须按 S33 返回 invalid_output；原 D9 Region §8 新签发格式/几何校验同样保留其适用的 invalid_output，且不得虚构 D7 错误或补读。选中真实合法 result 后，原 D7 owner 错误仍保持原优先级；失权再叠加坏 index/layout，必须在披露结构前返回不泄露的 not_visible/reset。测试必须证明：Plan 冻结前已经取得完整 result 并按原顺序完成 D7 运行期验证；long-form、顺序、坐标域、单位、empty/none/zero、panel、颜色与无障碍语义都保持；负值、重复 key、无效顺序、首个分页不完整、authorization reset、renderer/profile 不可用、font/color/page/alt-text loss、CJK/RTL/print 等价，以及 Dashboard block 隔离都有明确结果。Plan4 关系测试必须验证 contentSelection.viewInput=viewRenderBinding.resultInput=projection.view.resultInput、renderer.layout=ViewSpec.layout、binding/projection 的 outputScope 都是 complete_data、target/profile 精确兼容、print 当且仅当使用 print destination，并产生匹配的 receipt class。projection 的 viewSpecSha256 必须等于 binding 中的值以及精确 ViewSpec/1 的 SHA-256(D3-CJ/3(...))；来自另一份合法 ViewSpec 的 hash 即使 result index/layout/scope 相同也必须拒绝。无权 caller 叠加坏 layout 时先返回不泄露的 not_visible/reset；获权 caller 的同一失配仍按原 owner/关系门禁拒绝。print receipt 没有 target 字段：通过 planToken 解析原受保护且已确认的 print Plan，在 Plan 上检查 target 与 destination，再核 binding、真实 print output 与 loss；错误 Plan/binding/output 是反例。本地 legend hide/show 不能让 PDF/SVG 或同数据 accessible table 少掉某个 series。第一代 D9 chart profile 只支持 metric、bar、line、scatter、pie、heatmap。延期的 tree/treemap/sunburst、Gantt、boxplot/quantile 必须在现任 ViewSpec 封闭解码阶段返回 unsupported_layout；现任 D7 network 在这个封闭六成员 D9 route 上固定返回 renderer_unavailable，与是否安装 profile 无关，只有未来真实 versioned renderer/schema successor 才能增加 network graphics。六种图表缺少 backend/profile 时同样返回 renderer_unavailable。两条路径都不能用普通数据行替代图表。Office View route 还必须重跑适用的 Mandatory §14 模板、样式与重复区 fixture。

## 9. Typed format 与 image 证据

CSV dangerous starter 覆盖 ASCII/fullwidth 以及前导 Unicode White_Space。TSV 含内嵌 separator/newline 时必须拒绝。XLSX/ODS 中的 text 必须保持 text；任意精度 numeric policy 与 none/empty/absent 分离，并且永不计算 formula。

image physical size 只使用真实 density fact、精确 ratio 与 half-even rounding。missing/invalid/conflicting density 分支必须分开。显式 imageSizes 是 output layout policy，不是 source evidence。

## 10. Publication 与 Resource handoff

真实文件系统测试必须覆盖 create-only atomic bundle、flush 顺序、conflict、rename 前后 crash、用户移动、restart 与 revoke race。recovery 不得更换 final name。fresh external PublicationReceipt/4、D9PrintReceipt/1 与 author Resource receipt 是三种不同 outcome；historical PublicationReceipt/3 继续独立恢复。

## 11. Evidence ledger

historical bounded evidence 继续分开：57、63、59、82、90、130 以及 12-scenario chain。本作者批不重跑，也不升级这些证据。current Design documents workflow 最多证明 repository consistency。

I01–I12、真实 Worker、Office/WPS/LibreOffice、OS sandbox、真实 D6 fault injection、performance、release、deployment 在实际实现证据出现前全部 **UNRUN**。
