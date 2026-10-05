---
source_language: zh-CN
translation_status: source
---

[English](SOURCE-MAP.md)
# A2 D1/D2 来源与 disposition 图

状态：第一批作者映射。它只证明本批实际读取了什么、D1/D2 来源义务落在何处；不是独立接受。

## 1. 本批完整读取的固定输入

| 固定 S 来源 | 阅读状态 | A2 目标 | Disposition |
| --- | --- | --- | --- |
| D1 Product Surface and Capability Boundary | 全文 | D1 第 1–13 节 | retain，并具名整合 multi-replica/concurrency afterimage |
| D1 Implementation Impact and Test Outline | 全文 | D1 第 12 节 | 原 13 项 test obligation 全部保留 |
| D2 Document Content and Domain Objects | 全文 | D2 第 1–14 节 | domain decision retain；historical Profile-v2 language/wire rule 具名 versioned |
| D2 Implementation Impact and Test Outline | 全文 | D2 第 10、13–14 节 | 12 组 test 全部映射；旧 subset/plain-Annotation 假设只对 fresh current 路径 supersede |

精确 source path、S blob 与每个输入的 TODO 状态见 SOURCE-MAP.json。

## 2. Current D1 owner 阅读

current D1 两对双语 owner（Product Surface/Capability 与 Implementation Impact/Test Outline）均已全文阅读。它们提供 registered-replica 普通离线编辑、per-physical-replica commit-holder scope、Server multi-session、execution-responsibility separation、observed-only 产品状态、range-scoped invalidation 与 large-workspace milestone。

current owner 中“original capability negotiation remains”“existing G1/G1.1/G2 gates remain”不能作为继承捷径。D1 第 7–8 节已把固定 S 的十个 unavailable reason、stable major/read-only rule、artifact row、evidence gate、signing/warning/integrity/rollback 和 G1.1/Server-security 并行排期完整写回 A2。

## 3. D1 同名但不同来源的 scenario ID

固定 S D1 §15 与 later D1 owner §15 都使用 S1–S10，但含义不同。A2 在 D1 §11 分别保留为 fixed-S:S1..S10 与 current-owner:S1..S10；任何一组都不能靠同名 ID 删除另一组。

## 4. Current D2 overlay 阅读

fixed97 AsciiDoc/Annotation candidate 已读取完整 D2 language/processor/Witness/CSP/product 相关 section、完整 D2 schema region、D2 registry name 与 D2 相关 acceptance group；A2 D2 直接内嵌 current semantic/schema 规范副本。

已读取 D3 create_node §8.1 及其 create/copy/fork/import 邻接段，用于 title/admission 与 source-identity 交集；current create_node 只要求 D2-valid source，没有 title gate。

已读取 D6 current authorization/ordinary-save/installation/seal/recovery 相关段，用于 exact-source save、strict/observed-only、pin、SourceRevisionPlan、no-op 与 recovery 边界。

已读取 D8 committed-read、Draft/edit prepare/submission 与 historical recovery 相关段，用于 current exact-source read/save。fresh current projection 由 fixed97 D2 overlay 替换 historical limited document_snapshot product。

D7/D9 的 D2 direct-consumer 行为通过 fixed97 current overlay 实际读取：D7 heading/body 消费 D2DocumentSnapshot/3；D9 exact source/resource/query_json 的 generationPolicy none 不依赖 provider。D10 已读取 TASK/导航，但本批不整合 D10。

## 5. D2 具名 before/after disposition

| Before | A2 After |
| --- | --- |
| Profile-v2 强制 explicit title admission | current native title 可 absent；禁止 path/filename/paragraph 合成 |
| Profile-v2 closed language subset | 完整 fixed Asciidoctor 2.0.26 observable core + 具名 Weftext extension |
| heading 只允许 1–5 | WeftextManaged authored 1–9；effective level 无语言级上限 |
| old allowlist 外 include/pass/body-attribute/extension 全域非法 | native validity 按 fixed baseline + frozen environment/authorization/provider gate |
| Annotation 只有 plain text | portable Value/4 + R6 inline body/profile + aggregate CAS |
| historical document_snapshot wire2 作为 current product | 只作 historical recovery；fresh product 使用 D2DocumentSnapshot/3 |
| old Profile-v2 diagnostic/limit corpus 作为全局语法 | 与 native baseline 冲突处只作 historical-profile test |
| effective document attr 被 flatten/approximate | 同一次 evaluation 的 typed null/Boolean/text/integer/textArray + origins |
| structured write 从粗粒度 node range 猜 target | 只有 edited slot 的唯一 current authored writable origin 可写 |

## 6. Acceptance mapping 规则

D1 fixed-S scenario、D1 current-owner scenario、fixed-S D1 Impact tests、fixed-S D2 16 test groups、fixed-S D2 Impact 12 test groups，以及 fixed97 current D2 acceptance ID 全部 source-qualified；计数不能替代 row body 或行内条件。

SOURCE-MAP.json 记录机器可读 obligation group、disposition、source path/blob/section、A2 target 与 current oracle group。

## 7. 明确未读或未完成

本批虽已知道 49-file inventory、route graph 与 blob，但其它固定 S 输入仍是 semantic full-read TODO。D3–D10 current owner 尚未 global integrate；本批只读取上面具名的 D1/D2 直接相交段。

source map 不把任何 D3/D4/D5/D6/D7/D8/D9/D10 模块写成 done。下一次授权作者批次是 D3。
