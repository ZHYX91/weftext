---
source_language: zh-CN
translation_status: source
---

[English](D5-SOURCE-MAP.md)
# A2 D5 来源、intake、supersession 与 current-consumer 审计映射

状态：**A2-D4D5:P2-01** 与 **P2-02** 仅为 author-resolved-pending-independent。本次只修审计映射，不重写 D5 已正确的规范语义。

## 1. fixed-S D5 来源覆盖

三份 fixed-S D5 来源的 **170/170 个非空行**全部保留精确 source path/blob/line/text；每行恰好归入一个来源限定义务组，并写明 current file/section、disposition、真实 owner、basis 与 oracle。

关键历史 native-table group 是 fixed-S D5 主文 **81–92 行**。其中 Inline-only / one-logical-line / no span / no cell block / no header option 限制明确标为 **named-supersede**，不能继续冒充 current grammar。fresh current Source 服从 current D2 接纳的完整合法 Asciidoctor 2.0.26 native table surface；D5 §0/§3/§19.1 只负责 current structured editing。合法 Source 形态可以让结构化编辑 unavailable，但不能因此把 Source 判非法。

current D5 仍保持六种互不混同的 row domain，也没有 persistent Record/RecordCollection/RecordRef identity domain。

## 2. Mandatory 与 fixed97 来源限定

Mandatory source 1–924 行按 **716/716** 个非空行闭合为 29 个 actual-owner group；不再用泛化的 “D4/D5 intake” 当 current target。D6 persistence、D9 Office/template/conversion、D10 package/provider/connector/credential/execution custody、D3 binding 与 D4 typed fact 均保留各自真实 owner。

**130 条 D5 相关 fixed97 row**现在逐条保存真实 EN 与 ZH row、source path/blob/line，以及 current A2 target/disposition/owner/basis/oracle。计数不能替代原 row 的条件。

## 3. D10 direct qualification

CONTROL-CONTRACT、CANDIDATE、UPSTREAM-AMENDMENTS 的中英文共 6 份真实文件均按精确 blob 登记为 `direct_partial_not_full_D10`。Package/Contribution identity、provider availability、connector credential、schedule/execution control 与 external-effect custody 都不能成为 D5 author source 或 row identity。

D10 原文的 Proof2/Key2/wire12/PAB3 只做来源限定的历史/原 owner 证据，逐字不改。fresh/current D5 使用 `D3-SCHEMAS.md §2/§6.1` 的真实 current successor family 与 D5 §0；历史 bytes 不重编码。

## 4. 最小可追溯反例

- fixed-S D5 81–92 行 → D5 §0/§3/§19.1，`named-supersede`：旧 Inline-only/no-span/no-block/no-header 限制不是 current native-table grammar。
- Mandatory 第 111 行 `phones[]` → D5 §4/§14/§19.2 + D4 §10.1/§16.2：同值 phone 可在同一 revision 中对应不同 occurrence/note，不产生 durable row identity。
- Mandatory 第 183/210 行 → D5 §14 + D4 §8/§16.4：Organizations inverse display/edit 必须回到真实 authored relation side，不能持久化 inverse membership row。
- Mandatory 第 251/276/303–306 行 → D5 §14/§19.10 + D4 Calendar/current proof：recurrence occurrence 默认保持派生；provider/connector 状态不成为 row identity。

## 5. 评审状态

D1/D2 fixed-446 findings 继续 CLOSED；D3 P1-01/P1-02 已在 fixed-a62 被独立 CLOSED。已完成的 fixed-a62 D4/D5 非作者复核（`a62aa7d4eaf56113cd1e9e32336ae8814f83589b`，request `7b6e1f16-fa41-499c-b078-647f1942e94e`）给出 semantic LIMITED PASS，但 audit package 因这两个 P2 为 REVISE。本次作者修复仍把两项 P2 保持为 author-resolved-pending-independent。

D6 仍是已停止写入的作者候选，并未被接受。D7–D10 完整模块、Mandatory 925–1141 与全新的全局复核仍待完成。运行时、OS、GUI、真实副本、迁移与激活证据全部未运行。
