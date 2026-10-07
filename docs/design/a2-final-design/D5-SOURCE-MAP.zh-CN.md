---
source_language: zh-CN
translation_status: source
---

[English](D5-SOURCE-MAP.md)
# A2 D5 来源、intake、supersession 与 current-consumer 审计映射

状态：**A2-D4D5:P2-02 继续仅在 fixed4282 的六份 D10 真实来源有界范围内独立 CLOSED。fixed1fc4 独立复核后，A2-D4D5:P2-01 现为 bounded CLOSED：R2 保持 CLOSED，原 188/202 条 R1 通过项加全部 14 条残余映射（其中 D5 13 条）均通过独立复核。** 本次只修审计映射，不重写 D5 已正确的规范语义。

## 1. fixed-S D5 来源覆盖

三份 fixed-S D5 来源的 **170/170 个非空行**全部保留精确 source path/blob/line/text；每行恰好归入一个来源限定义务组，并写明 current file/section、disposition、真实 owner、basis 与 oracle。

关键历史 native-table group 是 fixed-S D5 主文 **81–92 行**。其中 Inline-only / one-logical-line / no span / no cell block / no header option 限制明确标为 **named-supersede**，不能继续冒充 current grammar。fresh current Source 服从 current D2 接纳的完整合法 Asciidoctor 2.0.26 native table surface；D5 §0/§3/§19.1 只负责 current structured editing。合法 Source 形态可以让结构化编辑 unavailable，但不能因此把 Source 判非法。

current D5 仍保持六种互不混同的 row domain，也没有 persistent Record/RecordCollection/RecordRef identity domain。

## 2. Mandatory 与 fixed97 来源限定

Mandatory source 1–924 行按 **716/716** 个非空行闭合为 33 个 actual-owner group；不再用泛化的 “D4/D5 intake” 当 current target。D6 persistence、D9 Office/template/conversion、D10 package/provider/connector/credential/execution custody、D3 binding 与 D4 typed fact 均保留各自真实 owner。

**130 条 D5 相关 fixed97 row**继续逐条保存真实 EN/ZH row 与 source path/blob/line。fixed32cf 独立复核通过 **117/130** 条 D5 R1 映射并留下 13 条残余；这 13 条连同 D4 的 1 条残余随后在 fixed1fc4 独立复核中全部通过，因此合并后的 A2-D4D5:P2-01 现为 bounded CLOSED。6 条 D2 language/Witness/CSP/product row 继续归 D2，5 条 native-table export row 继续归 D9，两条 holder/proof row 继续归真实 D5/D4 inner consumer 加 D6 outer `/3` proof。计数不能替代原 row 条件。

## 3. D10 direct qualification

CONTROL-CONTRACT、CANDIDATE、UPSTREAM-AMENDMENTS 的中英文共 6 份真实文件均按精确 blob 登记为 `direct_partial_not_full_D10`。Package/Contribution identity、provider availability、connector credential、schedule/execution control 与 external-effect custody 都不能成为 D5 author source 或 row identity。

D10 原文的 Proof2/Key2/wire12/PAB3 只做来源限定的历史/原 owner 证据，逐字不改。fresh/current D5 使用 `D3-SCHEMAS.md §2/§6.1` 的真实 current successor family 与 D5 §0；历史 bytes 不重编码。

## 4. 最小可追溯反例

- fixed-S D5 81–92 行 → D5 §0/§3/§19.1，`named-supersede`：旧 Inline-only/no-span/no-block/no-header 限制不是 current native-table grammar。
- Mandatory 第 111 行 `phones[]` → D5 §4/§14/§19.2 + D4 §10.1/§16.2：同值 phone 可在同一 revision 中对应不同 occurrence/note，不产生 durable row identity。
- Mandatory 第 183/210 行 → D5 §14 + D4 §8/§16.4：Organizations inverse display/edit 必须回到真实 authored relation side，不能持久化 inverse membership row。
- Mandatory 303–304 → D5 §14/§19.2/§19.5 + D4 Calendar；derived occurrence 默认不具 Node identity，只有显式 D3 promote/adopt 才创建 fresh Node。305 只保留为分组 metadata；306 明确让 D5 只做 domain/import consumer，而 provider token/etag/cursor/credential/fetch state 留在 D10 connector/subscription control。
- fixed32cf 的残余现按真实 owner 分流：`AD2-29`、`N36-06`、`M37-08`、`M38-11`、`M39-06`、`FC4R-PROD-10` 以 D2 language/Witness/CSP/product-projection 条款为主；`FC4R-TABLE-01/02/05/06/08` 以 D9 SPEC §8.3 + SCHEMAS §6.5/§6.5.1 为主并依赖 D2 grid；`FC34B-HOLDER-04/05` 保留 D5/D4 原 inner locator/context version，由 D6-CONTROL §3.4 + D6-SCHEMAS §5.1 提供 fresh outer source+format proof。

## 5. 评审状态

D1/D2 fixed-446 findings 继续 CLOSED；D3 P1-01/P1-02 继续在 fixed-a62 被独立 CLOSED。fixed4282 继续只在“六份真实 D10 来源资格”有界范围独立 **CLOSED P2-02**。P2-01 方面，fixed32cf 已独立 CLOSED R2 并通过 188/202 条 R1；后续 fixed1fc4 独立复核通过全部 14 条残余映射（其中 D5 13 条），因此 P2-01 现为 bounded CLOSED。这不构成 D4/D5 全局接受。

D6 当前评审状态不再在本文复制易漂移的摘要；统一以 [A2 REVIEW-ENTRY](REVIEW-ENTRY.zh-CN.md) 与 [D6-SOURCE-MAP](D6-SOURCE-MAP.zh-CN.md) 为状态/provenance 入口。D7–D10 完整模块、Mandatory 925–1141 与全新的全局复核仍待完成。运行时、OS、GUI、真实副本、迁移与激活证据全部未运行。
