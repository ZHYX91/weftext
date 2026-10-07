---
source_language: zh-CN
translation_status: source
---

[English](SOURCE-MAP.md)
# A2 D1–D6 来源与 disposition 图

状态：D1–D6 作者候选的来源/disposition 图；只记录阅读与映射证据，不是独立接受。

## 1. 本批完整读取的固定输入

| 固定 S 来源 | 阅读状态 | A2 目标 | Disposition |
| --- | --- | --- | --- |
| D1 Product Surface and Capability Boundary | 全文 | D1 第 1–13 节 | retain，并具名整合 multi-replica/concurrency afterimage |
| D1 Implementation Impact and Test Outline | 全文 | D1 第 12 节 | 原 13 项 test obligation 全部保留 |
| D2 Document Content and Domain Objects | 全文 | D2 第 1–14 节 | domain decision 继续保留；historical Profile-v2 的 language/wire rule 具名划入历史版本 |
| D2 Implementation Impact and Test Outline | 全文 | D2 第 10、13–14 节 | 12 组 test 全部映射；旧 subset/plain-Annotation 假设只对 fresh current 路径 supersede |

精确 source path、S blob 与每个输入的 TODO 状态见 SOURCE-MAP.json。

## 2. Current D1 owner 阅读

current D1 两对双语 owner（Product Surface/Capability 与 Implementation Impact/Test Outline）均已全文阅读。
它们补入 registered-replica 的普通离线编辑、每个物理副本各自的 commit-holder scope、Server 多会话，以及独立的 execution-responsibility domain。
同时还补入 observed-only 的产品状态、按依赖范围失效和 large-workspace milestone。

current owner 中“original capability negotiation remains”“existing G1/G1.1/G2 gates remain”不能作为继承捷径。D1 第 7–8 节已把固定 S 的十个 unavailable reason、stable major/read-only rule、artifact row、evidence gate、signing/warning/integrity/rollback 和 G1.1/Server-security 并行排期完整写回 A2。

## 3. D1 同名但不同来源的 scenario ID

固定 S D1 §15 与 later D1 owner §15 都使用 S1–S10，但含义不同。A2 在 D1 §11 分别保留为 fixed-S:S1..S10 与 current-owner:S1..S10；任何一组都不能靠同名 ID 删除另一组。

## 4. Current D2 overlay 阅读

fixed97 AsciiDoc/Annotation candidate 已读取完整 D2 language/processor/Witness/CSP/product 相关 section、完整 D2 schema region、D2 registry name 与 D2 相关 acceptance group；A2 D2 直接内嵌 current semantic/schema 规范副本。

已读取 D3 create_node §8.1 及其 create/copy/fork/import 邻接段，用于 title/admission 与 source-identity 交集；current create_node 只要求 D2-valid source，没有 title gate。

已读取 D6 中与 current authorization、普通保存、installation、seal 和 recovery 直接相交的段落，用于确认 exact-source save、strict/observed-only、pin、SourceRevisionPlan、no-op 与恢复边界。

已读取 D8 中 committed read、Draft/edit prepare、submission 和旧版本恢复的相关段，用于确认 current exact-source 的读取与保存路径。
fresh current projection 使用 fixed97 D2 overlay 的当前模型；historical limited document_snapshot product 只继续服务旧版本恢复。

D7/D9 的 D2 direct-consumer 行为通过 fixed97 current overlay 实际读取：D7 heading/body 消费 D2DocumentSnapshot/3；D9 exact source/resource/query_json 的 generationPolicy none 不依赖 provider。D10 已读取 TASK/导航，但本批不整合 D10。

## 5. D2 具名 before/after disposition

| Before | A2 After |
| --- | --- |
| Profile-v2 强制 explicit title admission | current native title 可 absent；禁止 path/filename/paragraph 合成 |
| Profile-v2 closed language subset | 完整 fixed Asciidoctor 2.0.26 observable core + 具名 Weftext extension |
| heading 只允许 1–5 | WeftextManaged authored 1–9；effective level 无语言级上限 |
| 旧 allowlist 外的 include/pass/body-attribute/extension 被全域判非法 | current native validity 按 fixed baseline，并继续经过 frozen environment、authorization 与 provider gate |
| Annotation 只有 plain text | portable Value/4 + R6 inline body/profile + aggregate CAS |
| historical document_snapshot wire2 作为 current product | 只作 historical recovery；fresh product 使用 D2DocumentSnapshot/3 |
| old Profile-v2 diagnostic/limit corpus 作为全局语法 | 与 native baseline 冲突处只作 historical-profile test |
| effective document attribute 被 flatten 或近似化 | 同一次 evaluation 必须保留 typed null、Boolean、text、integer、textArray 与 origins |
| structured write 从粗粒度 node range 猜 target | 只有 edited slot 的唯一 current authored writable origin 可写 |

## 6. Acceptance mapping 规则

D1 的 fixed-S scenario 与 current-owner scenario 都保留来源限定。fixed-S D1 Impact tests、fixed-S D2 的 16 组 tests、fixed-S D2 Impact 的 12 组 tests，以及 fixed97 current D2 acceptance ID 也都必须带来源资格。
任何计数都不能替代 row body 或其中的行内正反条件。

SOURCE-MAP.json 记录机器可读的 obligation group 与 disposition，并逐项保存 source path、blob、section、A2 target 和对应 current oracle group。

## 7. 明确未读或未完成

本批所需 fixed-S D4/D5 来源现已全文读取并映射：D4 主文/Impact/Lexicon 与不可变 catalog，以及 D5 主文/Impact/Lexicon。Mandatory scenario input §1–§14（1–924 行）已按 D4/D5 读取；925–1141 行留给后续 D7–D10。

D3、D4、D5 都是作者候选并有模块级 source map。D7 已在 fixed26be 的 D7 范围独立 PASS。D8 继续是未接受的作者候选；c98b 五项 finding 的作者修订现覆盖验收结构、View builder/旧入口路由、source-map applicability/navigation 与双语断裂，统一状态为 `author-resolved-pending-independent-review`。D9–D10 完整模块仍 TODO；direct producer/consumer 阅读不能把这些 owner 模块标成完成。

## 8. D1/D2 source-map 完整性窄修

fixed-5c 非作者复核指出的是**映射遗漏，不是新的语义缺陷**：若干已经实际读过的 fixed-S D1/D2 section 没有各自的 sourcePath/blob/section → A2 section/disposition/basis 行。本窄修只补这些行，不重写已经完成的 D1/D2 current semantic 正文。

| Source-qualified section | A2 落点 | Disposition |
| --- | --- | --- |
| fixed-S D1 main §13 非目标 | D1 §§1–3,13 | 保留 Core/五表面/不实施边界 |
| fixed-S D1 main §14 替代方案 | D1 §§2–6,10,13 | 保留拒绝第二权威、浏览器本地 Workspace、同步即协作、worker 直写等裁决 |
| fixed-S D1 main §16 下游约束 | D1 §§2–7,10,12.2 | retain；current per-replica/server-holder 与 execution-custody 是具名 afterimage |
| fixed-S D1 main §§17–18 | D1 §§11–12 | 作为未实施/未运行 implementation 与 test obligation 保留 |
| fixed-S D1 main §19 | D1 §§13–14 | 保留 review/freeze provenance；当时“下一主题”顺序只属历史 |
| fixed-S D2 main §6 | D2 §§10–13 | 保留 exact diagnostic span、repair 与 AND commit gate；Profile-v2/plain-Annotation 细节在 current native/Value4 明确替代处只作历史 |
| fixed-S D2 main §11 | D2 §§10,13 | retirement proof 保留为未实施；current native AsciiDoc、Value4 与 D5 no-Record 是具名 supersession |
| fixed-S D2 main §13 | D2 §§14–15 | 旧 activation/review evidence 只作 generation provenance；不虚构 migration |
| fixed-S D1 Impact §§1–6 | D1 §§8,12–14 | 每节补独立 machine mapping；原 13 条 test row 继续逐条映射 |
| fixed-S D2 Impact §§1–6 | D2 §§1,7–10,13–15 | 每节补独立 machine mapping；原 12 条 tests 与 12 条 retirement item 继续逐条映射 |

D2 Impact source blob 精确为 `956b3b768b97704c2e95dd8242b69507698ad3a7`；不采用早先私人报告中的 typo。current D5 的“本代无 persistent Record/RecordCollection、无 RecordRef”仍是 A2 current 边界，本次 source-map 修复不会复活 fixed-S 的 optional Record-domain 说法。

在 fixed-5c 作者修订时点，本作者没有自行关闭两项 finding。后续 fixed446 独立复核已在有界范围 CLOSED Record-boundary 与 source-map completeness 修订。该结论只适用于其固定 D1/D2 mapping scope，不构成 D6 或 global A2 acceptance；旧句子现在只作为历史时间线，不再表示 current pending。



## 9. D3 source/case 整合

D3-SOURCE-MAP.json 是案例级审计记录，保存 fixed-S main §14 cases 1-51、fixed-S §18 outlines 1-138、current-parent main §19 cases 1-79、Impact §20 的真实 56-79 结构、fixed-S Lexicon 全部 42 concept/普通 section，以及 current PR4 的 73 条 D3-direct 双语 acceptance row。

三个 fixed-S D3 输入与 current parent D3 main/Impact/Lexicon 中英双语都已全文读取。具名 D6-D10 中文 direct-owner 文件只为闭合 D3 interface 而全文读取；对应完整 A2 owner 模块仍待后续。fixed97 SPEC/SCHEMAS 除此前 D2 完整区域外，只声称 D3 相关 partial read。

fixed-446 已独立关闭有界 D1/D2 findings。后续 fixed-a62 非作者窄复核已独立 CLOSED D3:P1-01 与 D3:P1-02；更早 fixed-446 的 OPEN 只作为历史评审 provenance。全部产品/runtime scenario 仍未运行。

## 10. D4 整合

D4 fixed-S 主文、Impact、Lexicon 已逐非空行映射。不可变 catalog 继续是唯一静态数据权威：7 个 global limits、4 个 qualifier sets、22 个 aliases、61 个 Fields、7 个 Facets、1 个 Calendar series-scope policy，共 95 个具名记录。D4-CATALOG-MAP.json 映射全部 2,854 个 JSON Pointer，不建立第二份 registry 或 activation ledger。

当前 unseen/fresh 外层分派使用 wire13 / InputDescriptor/3 / DependencyProof/3 / 十五分支 DependencyKey/3，并包含 document_format。Entry/1、RelationReadContext/2、RelationReadBinding/2、recurrence 内层类型和 D4 effect extensions 都不机械升版。SourceRevisionPlan/1、/2、/3 继续是不同 owner 分支。

## 11. D5 整合

D5 fixed-S 主文、Impact、Lexicon 已逐非空行映射。当前候选没有持久 Record/RecordCollection/RecordRef 域，并保持六种不同 row domain。

历史 Inline-only、单逻辑行、禁止 span/块 cell 的 native-table 限制被显式 supersede；当前 D2 接纳完整合法 Asciidoctor 2.0.26 table surface。合法 shape 可以没有 structured control，但 Source 不能因此 invalid。合格的 titleless native create/save 继续合法；filename/path/placeholder 不能伪造 title。

D6–D9 producer/consumer 交叉继续记录为 direct partial。D10 现也通过真实 CONTROL-CONTRACT、CANDIDATE、UPSTREAM-AMENDMENTS 中英文文件按精确 blob 做 direct qualification；这些读取均为 direct_partial_not_full_D10，不等于 D10 完整模块完成。

## D6 整合更新

后续非作者复核绑定 fixed26be（`26be071d4c2075343d9ebf272e00f23769ce0d64`），结论 PASS：P0=0 / P1=0 / P2=0，并 CLOSED 最终 D7 残余。D7 因而在 D7 范围 accepted；D8、D9–D10 与 global A2 仍分别 pending。产品/runtime 证据仍为 UNRUN。


## 12. D4/D5 审计映射复核谱系

D4/D5 方面，fixed4282 仅独立 CLOSED P2-02 的六份 D10 真实来源资格有界范围。fixed32cf 已独立 CLOSED P2-01 R2 并通过 188/202 条 R1；fixed1fc4 随后独立通过全部 14 条残余 R1 映射。因此 A2-D4D5:P2-01 现为 bounded CLOSED，但不构成 D4/D5 全局接受。

D4-SOURCE-MAP.json 已将 906 个 fixed-S D4 非空行、D5-SOURCE-MAP.json 将 170 个 fixed-S D5 非空行全部放入来源限定的连续义务组，并逐项写 current target、disposition、owner、basis 和 oracle。Mandatory 1–924 行为 716/716，按真实 owner/defer 路由。fixed97 selected rows 同时保存真实 EN/ZH 原 row（D4 72/72；D5 130/130）。D4-CATALOG-MAP.json 按 RFC 6901 用 7 个无重叠 subtree 覆盖 2,854/2,854 pointer，文档根使用空字符串而不是 slash，同时保持不可变 fixed catalog 为唯一规范值权威。Mandatory 的跨文件 target 已改为逐文件 target array；current workflow/freeze/handoff 三组分别落到 REVIEW-ENTRY §5.1–§5.3，不再指向不存在的 §7。

旧 fixed-S D5 Inline-only/no-span/no-block/no-header 限制已明确由 current D5 §0/§3/§19.1 与 current D2 full native-table surface 具名 supersede。6 份真实 D10 双语 direct file 均按 direct_partial_not_full_D10 做来源限定；原 Proof2/Key2/wire12/PAB3 继续是 historical/original-owner source，fresh current dispatch 仍由真实 A2 successor owner 闭合。

fixed2f89（`2f89a55cb1f924a47281f59e6419fff7c0c206ed`）完整 D7 独立复核结论为 REVISE：P0=0 / P1=2 / P2=2。该复核已在有界范围独立 CLOSED `A2-D6-01CC-P2-01`、`A2-NAV-454E-P2-01` 与 `COORD-D7-DOC-QUALITY-01`。本作者修复处理仍 OPEN 的四项 D7 finding：`A2-D7-2F89-P1-01`、`A2-D7-2F89-P1-02`、`A2-D7-2F89-P2-01`、`A2-D7-2F89-P2-02`；四项都只是“作者修复已存在、等待 fixed-SHA 独立复核”，作者不自行关闭。不声称 D7 或全局 A2 已接受。D8–D10 完整模块与全新的独立 Pro/global 复核继续 pending；产品/runtime 证据仍为 UNRUN。

## 13. D7 整合

D7 现已整合为完整作者候选。fixed-S 的十三份 D7 来源以及额外的 D9 coordinated D3-D7 binding 来源均已完整读取。当前双语 D7 owner afterimage 按原 blob 复制到 d7/owners。immutable fixed-S Registry 是 34 个 concept / 8 条 cross-stage binding；parent-current Registry 与逐字节相同的 D7-REGISTRY.json 是 34/13。D7-REGISTRY-QUALIFICATION 只叠加 A2 的具名 successor/current-history 资格，不建立第二 Registry。D7.md 与 D7-SCHEMAS.md 只对具名 fixed97/current-D2 successor 应用补充规则，其余保留条款仍直接由复制的 owner 正文承担。

D7-SEARCH.md 已整合 SEARCH-01 至 SEARCH-08，但没有建立第二个搜索执行器或持久权威。普通文本与可视化筛选是默认路径；可选快捷模式必须显式进入，并使用带 @ 前缀的操作符，因此普通冒号文字、URL、Windows 盘符和 title: 不会被静默解析。保存时只保存 canonical Query 语义，不保存设备侧解析器状态。当前匹配保留 exact 与显式 `nfc-for-compare` 两种比较 basis，substring/equality 均区分大小写，并且不声称已经提供自动模糊、拼音或分词能力。

D7-SOURCE-MAP.json 记录完整 D7 provenance 与 fixed26be accepted scope。D8-SOURCE-MAP.json 保留 fixed D8 prose/case、Mandatory §15/RTL、SEARCH-01–08、current View/FC 协调与 direct producer/consumer 库存。c98b 作者修订已把 54/54 fixedProseSections 映射到 section-level current target（宽泛整文件 0），并对 760/760 FC 行写入显式 current owner/basis（direct 167、upstream 142、保留 non-D8 owner 451）；这些结果仍全部等待 fixed-SHA 独立语义复核。D9–D10 仍只在真实 D8 intersection 上 PARTIAL；其完整模块留后续。

fixed2f89 独立关闭两项 navigation finding 与 COORD-D7-DOC-QUALITY-01；fixed1068244 关闭 A2-D7-2F89-P2-02；fixed85bdadf 关闭两个 P1 与 Impact-sync P2；后续 fixed26be 非作者复核关闭最终 A2-D7-2F89-P2-01，并给出 PASS 0/0/0。

产品 runtime、OS、GUI、renderer/export、database、真实 replica、provider、performance、migration 与 activation 行为在本批全部为 UNRUN。


## 14. fixed1068244 残余 D7 修复

fixed1068244（`1068244d982a22140d8753b0483c63135b092999`）独立增量复核结论为 REVISE：P0=0 / P1=2 / P2=2，并已独立 CLOSED `A2-D7-2F89-P2-02`（immutable source trace）。当前残余 finding 是 `A2-D7-2F89-P1-01`、`A2-D7-2F89-P1-02`、`A2-D7-2F89-P2-01`，以及新增的 `A2-D7-1068244-P2-01`（Impact 中英同步）。本作者批修复这四项残余，但不自行关闭。此前有界关闭的 `A2-D6-01CC-P2-01`、`A2-NAV-454E-P2-01` 与 `COORD-D7-DOC-QUALITY-01` 继续保持 CLOSED。不声称 D7 或 global A2 已接受。

本批保持已经关闭的 immutable source trace 不变；修正顶层 Registry 导航中的 fixed-S 34/8 与 parent-current 34/13，加入 QuerySpec/2 消费的 D6-owned 最小权限 FileBinding metadata producer，闭合 QuerySpec/2 的 Optional-title/union_all/D9-query_json 直接 consumer，修复 shortcut machine oracle 与 escape 案例，并同步 D7-IMPACT 中英文。S49 snapshots 与 inputs 继续受保护。

## 15. fixed85bdadf 唯一 D7 残余与真实 compiler oracle

fixed85bdadf（`85bdadf448e0475715b62debe215fa4d04e30ab2`）独立把 D7 repair ledger 收敛到 P0=0 / P1=0 / P2=1。P1-01、P1-02 与 A2-D7-1068244-P2-01 均在该 SHA CLOSED；P2-02 继续保持 fixed1068244 CLOSED。本继任作者 delta 只触碰唯一 P2-01 Search oracle 与必要状态/导航文字：19 个正例现在都带真实可严格解码的 QuerySpec/2 与直接 §5 CanonicalGraph serialization，8 个 negative/incomplete/browse case 继续保持零 Query。已关闭的 Registry/D6 metadata/D9 query_json 合同均不重开。


## 16. D8 c98b 五项 finding 作者修订

D8 已保全候选首次真实推送在 7e3f；本五项修订精确从 c98b（`c98b1b161751c5a01220da37568c78b5cef13841`）开始。不可变库存继续是 54 个 fixed prose-section row、160 个 fixed case + FA17 继承、RTL intake、Mandatory §15、SEARCH-01–08 与 27 个 current Search fixture、两层 replacement router，以及严格 760 条 current AsciiDoc/Annotation acceptance row；S49/inputs 不重新生成。

本修订把 `D8-ACCEPTANCE.json` 设为六字段唯一结构源，恢复 ANNOT-01 与 WRITE-08 的完整正反极性，把 current View overlay 改到 `VIEW-CUR-*`，增加十个 `VIEW-BLD-01..10` Mandatory §15.7 oracle，得到 213/213 唯一 current obligation。正文补齐基于 current D7 ViewSpec/Definition 的无损 builder lifecycle，并明确 historical `.weftext-query view=...` 与 current `DynamicBlock/1` 路由，不建立第二 View/Query schema/store。同时把 54/54 fixed prose row 映射到 section-level target，并完成 760/760 FC 的逐行 applicability/current-owner 作者审计（direct 167、upstream 142、保留 non-D8 owner 451；110 行 applicability 改动）。中文 §6/§9 断句按完整英文合同修复。

五项 finding 全部仍只是 `author-resolved-pending-independent-review`。数量、schema/列数检查和 docs CI 只提供机械证据，不代表语义接受。current D8 继续消费完整 Asciidoctor 2.0.26/D2 product、exact Source/Draft/edit/currentness、Annotation Value4/R6、presentation-policy owner 与已接受 D7 Search/View 语义。产品/runtime/GUI/IME/AT/Office/OS/多副本/性能/migration/activation/deployment 全部 UNRUN。D9/D10 仅在具名 D8 边界 PARTIAL；其完整模块与 global A2 留后续。

## 17. D9 完整作者整合

D9 从 exact `5eca16c40cdf2e1892f6930d51c632ea720a460c` 开始，现已是完整作者候选，不再是 TODO module。八份 fixed-S D9 source 为 FULL；八组 current D9 owner 中英为 FULL；Mandatory §14 的 884–924 行 FULL；current D7 D9-binding/PAB owner 中英 FULL；D9 适用 final AsciiDoc/Annotation successor FULL。D9 必要 read gap 为 0。D10 仍仅在具名 D9 direct intersection 上 PARTIAL，不构成完整 D10 整合。

fresh current author 工作消费 D3 wire13、D7 PAB4、DependencyProof3、Effect3。fresh export/publication 消费 ExportPlan3/PublicationReceipt3 以及 current D2Snapshot3/D8 presentation binding。historical wire11/12、PAB1/2/3、Recipe/ConversionInput1、Plan/Receipt1/2 只按原 decoder/bytes 恢复。；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。

Mandatory §14 在作者设计层闭合，并有一项具名 selector 修订：唯一 lowercase-ASCII native-table leaf 继续使用 `data.native_table.COLUMN`；qualified/non-ASCII fresh Office template bytes 使用可见 `native.table[...]::column[...]`，按 shortest suffix/title/occurrence 逐级限定。final-FC `nt_/nc_` 只作 internal compiled Plan key，不是 hidden authoring authority。ordinary Node 不增加 export-mapping schema。

current Annotation 整合也已明确：Node Template omission 只局限 construction；`annotation_index` 只作 omission-directory evidence。真正 export/copy/import current Annotation content 的 profile 必须通过唯一 R6 AnnotationInlineProfile 消费完整 PortableAnnotationRecord/4 / Value4，并在 current disclosure 下保持 target/reply/attribution 边界。

`D9-ACCEPTANCE.json` 是 80 条结构权威；`D9-SOURCE-MAP.json` 保存 fixed/current/successor provenance。historical finite evidence 57/63/59/82/90/130/12 分开记账、不可相加。I01–I12 与全部产品/runtime/GUI/Office/OS/performance/migration/activation/deployment 证据继续 UNRUN。D9 仍为 `author-resolved-pending-independent-review`；docs/JSON 绿色检查不能接受它。
