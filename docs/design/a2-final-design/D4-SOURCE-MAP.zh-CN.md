---
source_language: zh-CN
translation_status: source
---

[English](D4-SOURCE-MAP.md)
# A2 D4 来源、catalog、intake 与 current-consumer 审计映射

状态：**A2-D4D5:P2-02 仅在 fixed4282 的六份 D10 真实来源有界范围内独立 CLOSED；A2-D4D5:P2-01 的 R1/R2 仅为 author-resolved-pending-independent。** 本文不独立接受 D4、D6、D7–D10 或全局 A2。

## 1. fixed-S D4 来源覆盖

三份 fixed-S D4 文本来源的 **906/906 个非空行**全部保留精确 source path/blob/line/text。每个非空行恰好属于一个连续、来源限定的义务组，并写明 current file/section 落点、`retain | named-supersede | explicit-defer | historical-only` disposition、真实 owner、basis 与 oracle。front matter、revision history 与旧 review sequencing 只作为历史 provenance，不冒充业务语义。

不可变 reference catalog 继续是唯一静态值权威。`D4-CATALOG-MAP.json` 通过 7 个无重叠 subtree group 覆盖 **2,854/2,854 个 JSON Pointer**、**95 个具名记录**和 **7 个 global limit**。按 RFC 6901，文档根指针是 `""`；`"/"` 表示空键成员。映射只保存 pointer/record identity 与真实 current consumer/validator/constructor 落点，不复制第二份规范值。

## 2. Mandatory 与 fixed97 来源限定

Mandatory source 1–924 行中的 **716/716 个非空行**已分入 29 个来源限定 group。跨模块事项明确写真实 owner 与目标：D6 persistence defer 给 D6，D9 template/conversion defer 给 D9，D10 package/provider/connector control 归 D10；D4 继续拥有 Semantic Namespace、Field、Relation 与 Calendar 语义。925–1141 行仍留给后续 D7–D10/global。

**72 条 D4 相关 fixed97 acceptance row**继续逐条保存真实英文/中文 row 与 source path/blob/line。R1 已对 **72/72** 逐 case 重核真实 current owner/consumer；每行都写 actual file+section target、来源限定 disposition、owner、basis 与精确 source-row oracle。数量只证明库存，不能替代原 row 条件。

## 3. 真实 D10 direct source qualification

CONTROL-CONTRACT 中英、CANDIDATE 中英、UPSTREAM-AMENDMENTS 中英共 6 份真实 current 文件均按精确 blob 登记，`readStatus=direct_partial_not_full_D10`。其中与 Package/Contribution、Registry/activation、provider availability、schedule/temporal、connector/credential、execution custody 和 unchanged boundary 直接相交的条款映射到现有 D4 current clause；这不表示 D10 A2 完整模块已完成。

D10 原文中的历史/原 owner `DependencyProof/2`、`DependencyKey/2`、`wire12`、`PreparedActionBinding/3` 保持逐字原样，只增加来源限定。fresh/current D4 继续由 `D3-SCHEMAS.md §2/§6.1` 与 `D4.md §0` 的真实 current successor 闭合；真实 historical record 继续按 recorded decoder/bytes/pins/recovery 恢复，不建立第二套 current authority。

## 4. 最小可追溯反例

- Organizations relation/inverse：Mandatory 第 183、210 行直接覆盖 inverse 语义；current 落点为 D4 §8/§16.4，D5 §14 只是投影/编辑消费者。inverse UI 必须回到唯一 canonical authored side，不能另存一份 membership。
- People phone：Mandatory 第 111 行为 `phones[]`；current 落点为 D4 §10.1/§16.2，D5 §4/§14 为编辑/消费层。两个同值 phone 可以是不同 current-revision occurrence，并拥有不同 note；selector 不能跨 revision 变成 durable identity。
- Mandatory 303–304 现把 recurrence set/override 与 derived-occurrence identity 落到 D4 §9/§10.3/§16.6/§17.5；source binding/import 仍归 D3/D9，显式 promote/adopt 创建 fresh Node identity 仍归 D3。305 只作为“导入意图”分组 heading；306 保持外部 Calendar authority，同时把 provider token/etag/cursor/credential/fetch state 明确留在 D10 connector/subscription control。
- fixed97 `FC34-TR-01` 现落 D6-CONTROL §10.2/§20/§20.2，`FC34B-TR-05` 落 §9.4/§20.1，均不再冒充 Calendar；D2 media/provenance 与 D3/D8 Annotation 行也回到各自真实 holder。

## 5. 评审状态

D1/D2 fixed-446 findings 继续 independent CLOSED；两个有界 D3 P1 已在 fixed-a62 被独立 CLOSED。后续 fixed4282 独立复核对本审计包给出 P0=0/P1=0/P2=2：仅在“六份真实 D10 来源资格”有界范围内独立 **CLOSED P2-02**，P2-01 仍以 R1/R2 残余 OPEN。本次只修这两个残余，并把 P2-01 标为 author-resolved-pending-independent；是否关闭必须由新的 fixed-stop 非作者复核裁决。

D6 作者写入已停在 `01cc40b819df78fbe724f1c64c27284ad60fc6c8`，五项 D6 finding 仍只是 author-resolved-pending-independent；其独立 exact-SHA 复核与本 D4/D5 审计分离，本次不构成 D6 acceptance。D7–D10 完整模块、Mandatory 925–1141 与 fresh Pro/global 仍 pending。runtime、OS、GUI、真实 replica、migration、activation、deployment 全部 UNRUN。
