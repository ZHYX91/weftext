---
source_language: zh-CN
translation_status: source
---

[English](D4-SOURCE-MAP.md)
# A2 D4 来源、catalog、intake 与 current-consumer 审计映射

状态：**A2-D4D5:P2-01** 与 **P2-02** 仅为 author-resolved-pending-independent。本文只记录作者侧审计闭合，不独立接受 D4、D6、D10 或全局 A2。

## 1. fixed-S D4 来源覆盖

三份 fixed-S D4 文本来源的 **906/906 个非空行**全部保留精确 source path/blob/line/text。每个非空行恰好属于一个连续、来源限定的义务组，并写明 current file/section 落点、`retain | named-supersede | explicit-defer | historical-only` disposition、真实 owner、basis 与 oracle。front matter、revision history 与旧 review sequencing 只作为历史 provenance，不冒充业务语义。

不可变 reference catalog 继续是唯一静态值权威。`D4-CATALOG-MAP.json` 通过 7 个无重叠 subtree group 覆盖 **2,854/2,854 个 JSON Pointer**、**95 个具名记录**和 **7 个 global limit**。按 RFC 6901，文档根指针是 `""`；`"/"` 表示空键成员。映射只保存 pointer/record identity 与真实 current consumer/validator/constructor 落点，不复制第二份规范值。

## 2. Mandatory 与 fixed97 来源限定

Mandatory source 1–924 行中的 **716/716 个非空行**已分入 29 个来源限定 group。跨模块事项明确写真实 owner 与目标：D6 persistence defer 给 D6，D9 template/conversion defer 给 D9，D10 package/provider/connector control 归 D10；D4 继续拥有 Semantic Namespace、Field、Relation 与 Calendar 语义。925–1141 行仍留给后续 D7–D10/global。

**72 条 D4 相关 fixed97 acceptance row**现在同时保存真实英文 row 与真实中文 row，并带 source path/blob/line、current A2 target、disposition、owner、basis 和 oracle。数量只是库存证明，不能替代 row 内条件。

## 3. 真实 D10 direct source qualification

CONTROL-CONTRACT 中英、CANDIDATE 中英、UPSTREAM-AMENDMENTS 中英共 6 份真实 current 文件均按精确 blob 登记，`readStatus=direct_partial_not_full_D10`。其中与 Package/Contribution、Registry/activation、provider availability、schedule/temporal、connector/credential、execution custody 和 unchanged boundary 直接相交的条款映射到现有 D4 current clause；这不表示 D10 A2 完整模块已完成。

D10 原文中的历史/原 owner `DependencyProof/2`、`DependencyKey/2`、`wire12`、`PreparedActionBinding/3` 保持逐字原样，只增加来源限定。fresh/current D4 继续由 `D3-SCHEMAS.md §2/§6.1` 与 `D4.md §0` 的真实 current successor 闭合；真实 historical record 继续按 recorded decoder/bytes/pins/recovery 恢复，不建立第二套 current authority。

## 4. 最小可追溯反例

- Organizations relation/inverse：Mandatory 第 183、210 行直接覆盖 inverse 语义；current 落点为 D4 §8/§16.4，D5 §14 只是投影/编辑消费者。inverse UI 必须回到唯一 canonical authored side，不能另存一份 membership。
- People phone：Mandatory 第 111 行为 `phones[]`；current 落点为 D4 §10.1/§16.2，D5 §4/§14 为编辑/消费层。两个同值 phone 可以是不同 current-revision occurrence，并拥有不同 note；selector 不能跨 revision 变成 durable identity。
- Calendar recurrence：Mandatory 第 251、276、303–306 行覆盖 Event/recurrence/series/occurrence；current 落点为 D4 §9/§10.3/§16.6/§17.5 与真实 current dependency producer。recurrence/series projection 是派生状态；强 managed-Document consumer 在实际语义解析该 Document 时还必须绑定 `document_format`。

## 5. 评审状态

D1/D2 fixed-446 findings 继续 independent CLOSED；两个有界 D3 P1 已在 fixed-a62 被独立 CLOSED。已完成的 fixed-a62 D4/D5 非作者报告（`a62aa7d4eaf56113cd1e9e32336ae8814f83589b`，request `7b6e1f16-fa41-499c-b078-647f1942e94e`）为 P0=0/P1=0/P2=2：D4/D5 语义 LIMITED PASS，audit package REVISE。本作者修复不自行关闭两个 P2，均等待新的 fixed-stop 独立复核。

D6 仍是停在 829 的作者候选；fixed-829 只读工作不构成 D6 acceptance。D7–D10 完整模块、Mandatory 925–1141 与 fresh Pro/global 仍 pending。runtime、OS、GUI、真实 replica、migration、activation、deployment 全部 UNRUN。
