---
source_language: zh-CN
translation_status: source
---

[English](D5-SOURCE-MAP.md)
# A2 D5 来源、intake、supersession 与 current-consumer 审计映射

状态：**A2-D4D5:P2-02 仅在 fixed4282 的六份 D10 真实来源有界范围内独立 CLOSED；A2-D4D5:P2-01 的 R1/R2 仅为 author-resolved-pending-independent。** 本次只修审计映射，不重写 D5 已正确的规范语义。

## 1. fixed-S D5 来源覆盖

三份 fixed-S D5 来源的 **170/170 个非空行**全部保留精确 source path/blob/line/text；每行恰好归入一个来源限定义务组，并写明 current file/section、disposition、真实 owner、basis 与 oracle。

关键历史 native-table group 是 fixed-S D5 主文 **81–92 行**。其中 Inline-only / one-logical-line / no span / no cell block / no header option 限制明确标为 **named-supersede**，不能继续冒充 current grammar。fresh current Source 服从 current D2 接纳的完整合法 Asciidoctor 2.0.26 native table surface；D5 §0/§3/§19.1 只负责 current structured editing。合法 Source 形态可以让结构化编辑 unavailable，但不能因此把 Source 判非法。

current D5 仍保持六种互不混同的 row domain，也没有 persistent Record/RecordCollection/RecordRef identity domain。

## 2. Mandatory 与 fixed97 来源限定

Mandatory source 1–924 行按 **716/716** 个非空行闭合为 33 个 actual-owner group；不再用泛化的 “D4/D5 intake” 当 current target。D6 persistence、D9 Office/template/conversion、D10 package/provider/connector/credential/execution custody、D3 binding 与 D4 typed fact 均保留各自真实 owner。

**130 条 D5 相关 fixed97 row**继续逐条保存真实 EN/ZH row 与 source path/blob/line。R1 已对 **130/130** 逐 case 重核真实 current holder/consumer，并记录 actual file+section target、disposition、owner、basis 与精确 source-row oracle；计数不能替代原 row 条件。

## 3. D10 direct qualification

CONTROL-CONTRACT、CANDIDATE、UPSTREAM-AMENDMENTS 的中英文共 6 份真实文件均按精确 blob 登记为 `direct_partial_not_full_D10`。Package/Contribution identity、provider availability、connector credential、schedule/execution control 与 external-effect custody 都不能成为 D5 author source 或 row identity。

D10 原文的 Proof2/Key2/wire12/PAB3 只做来源限定的历史/原 owner 证据，逐字不改。fresh/current D5 使用 `D3-SCHEMAS.md §2/§6.1` 的真实 current successor family 与 D5 §0；历史 bytes 不重编码。

## 4. 最小可追溯反例

- fixed-S D5 81–92 行 → D5 §0/§3/§19.1，`named-supersede`：旧 Inline-only/no-span/no-block/no-header 限制不是 current native-table grammar。
- Mandatory 第 111 行 `phones[]` → D5 §4/§14/§19.2 + D4 §10.1/§16.2：同值 phone 可在同一 revision 中对应不同 occurrence/note，不产生 durable row identity。
- Mandatory 第 183/210 行 → D5 §14 + D4 §8/§16.4：Organizations inverse display/edit 必须回到真实 authored relation side，不能持久化 inverse membership row。
- Mandatory 303–304 → D5 §14/§19.2/§19.5 + D4 Calendar；derived occurrence 默认不具 Node identity，只有显式 D3 promote/adopt 才创建 fresh Node。305 只保留为分组 metadata；306 明确让 D5 只做 domain/import consumer，而 provider token/etag/cursor/credential/fetch state 留在 D10 connector/subscription control。
- fixed97 `AD2-18` 现回 D2 native video semantics；`FC34C-D6-01` 回 D6-CONTROL §21.4/§21.5 + D10 execution responsibility；D8 presentation-policy 行回 SPEC §8.2/SCHEMAS §6.4；D9 export 行回 SPEC §8.3/SCHEMAS §6.5，不再用 D5 §17 当语义兜底。

## 5. 评审状态

D1/D2 fixed-446 findings 继续 CLOSED；D3 P1-01/P1-02 已在 fixed-a62 被独立 CLOSED。后续 fixed4282 独立复核仅在“六份真实 D10 来源资格”有界范围内独立 **CLOSED P2-02**，并让 P2-01 以 R1/R2 残余继续 OPEN。本次只修 R1/R2，P2-01 仍为 author-resolved-pending-independent，等待新的 fixed-stop 非作者复核。

D6 作者写入已停在 `01cc40b819df78fbe724f1c64c27284ad60fc6c8`，五项 D6 finding 仅为 author-resolved-pending-independent；未 accepted，也不由本 D5 审计自动关闭。D7–D10 完整模块、Mandatory 925–1141 与全新的全局复核仍待完成。运行时、OS、GUI、真实副本、迁移与激活证据全部未运行。
