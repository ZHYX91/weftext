---
source_language: zh-CN
translation_status: source
---

[English](SOURCE-MAP.md)
# A2 D1–D5 来源与 disposition 图

状态：D1–D5 作者候选的来源/disposition 图；只记录阅读与映射证据，不是独立接受。

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

本批所需 fixed-S D4/D5 来源现已全文读取并映射：D4 主文/Impact/Lexicon 与不可变 catalog，以及 D5 主文/Impact/Lexicon。Mandatory scenario input §1–§14（1–924 行）已按 D4/D5 读取；925–1141 行留给后续 D6–D10。

D3、D4、D5 都是作者候选，并有模块级 source map。D6–D10 完整模块仍 TODO；D4/D5 direct producer/consumer 阅读明确只是 partial，不能据此把这些 owner 模块标成完成。

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

fixed-5c 的非作者 finding 本作者不自行关闭。当前 head 已包含 Record-boundary 作者修复，仍待非作者复核；本次 source-map 窄修同样等待非作者复核。



## 9. D3 source/case 整合

D3-SOURCE-MAP.json 是案例级审计记录，保存 fixed-S main §14 cases 1-51、fixed-S §18 outlines 1-138、current-parent main §19 cases 1-79、Impact §20 的真实 56-79 结构、fixed-S Lexicon 全部 42 concept/普通 section，以及 current PR4 的 73 条 D3-direct 双语 acceptance row。

三个 fixed-S D3 输入与 current parent D3 main/Impact/Lexicon 中英双语都已全文读取。具名 D6-D10 中文 direct-owner 文件只为闭合 D3 interface 而全文读取；对应完整 A2 owner 模块仍待后续。fixed97 SPEC/SCHEMAS 除此前 D2 完整区域外，只声称 D3 相关 partial read。

fixed-446 独立评审已经关闭 D1/D2 P1-01/P2-01 finding；D3:P1-01 与 D3:P1-02 仍 OPEN，且没有评 edaf 或更晚作者 delta。全部产品/runtime scenario 仍未运行。

## 10. D4 整合

D4 fixed-S 主文、Impact、Lexicon 已逐非空行映射。不可变 catalog 继续是唯一静态数据权威：7 个 global limits、4 个 qualifier sets、22 个 aliases、61 个 Fields、7 个 Facets、1 个 Calendar series-scope policy，共 95 个具名记录。D4-CATALOG-MAP.json 映射全部 2,854 个 JSON Pointer，不建立第二份 registry 或 activation ledger。

当前 unseen/fresh 外层分派使用 wire13 / InputDescriptor/3 / DependencyProof/3 / 十五分支 DependencyKey/3，并包含 document_format。Entry/1、RelationReadContext/2、RelationReadBinding/2、recurrence 内层类型和 D4 effect extensions 都不机械升版。SourceRevisionPlan/1、/2、/3 继续是不同 owner 分支。

## 11. D5 整合

D5 fixed-S 主文、Impact、Lexicon 已逐非空行映射。当前候选没有持久 Record/RecordCollection/RecordRef 域，并保持六种不同 row domain。

历史 Inline-only、单逻辑行、禁止 span/块 cell 的 native-table 限制被显式 supersede；当前 D2 接纳完整合法 Asciidoctor 2.0.26 table surface。合法 shape 可以没有 structured control，但 Source 不能因此 invalid。合格的 titleless native create/save 继续合法；filename/path/placeholder 不能伪造 title。

D6–D9 producer/consumer 交叉按精确 path/blob 记录为 direct partial。parent owner 树没有独立 D10 owner 文件，所以 D10 只保留 fixed97 SPEC/SCHEMAS direct-partial。
