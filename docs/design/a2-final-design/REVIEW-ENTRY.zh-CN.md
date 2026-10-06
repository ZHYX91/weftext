---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1–D6 作者候选复核入口——D4/D5 审计映射交回

状态：仅作者候选；未独立接受、未实现、未合并、未发布、未部署、未全局冻结。

## 1. 固定对象与绑定

Base branch：docs/asciidoc-annotation-final-design。
固定 base SHA：97f4734f82a760cb6716c8122b84494da2b61164。
固定历史输入 S：7e18168dad3e6d120fce0dd607dc10fa7894e252。
受保护 inputs blob：787d03c31a55496f81ed03fd54a6fdfff50a2ad4。
候选 branch：docs/a2-final-design-integration。
本次窄修从 stopped head 829efce6aacbe944714e093c98065b01d50b2593 开始。

下一位复核者必须绑定 PR5 本批真实 final stop head；不得跟随 moving branch，也不得把任何旧 fixed-SHA 结论外推到其原范围之外。

## 2. 既有独立状态

- fixed-446 已独立 CLOSED 有界 D1/D2 findings；继续保留该结论。
- fixed-a62 已独立 CLOSED D3:P1-01 与 D3:P1-02；更早 fixed-446 的 OPEN 只作为历史状态。
- 已完成的非作者报告绑定 fixed-a62 `a62aa7d4eaf56113cd1e9e32336ae8814f83589b`（复核请求 `7b6e1f16-fa41-499c-b078-647f1942e94e`），对 D4/D5 给出 P0=0/P1=0/P2=2：规范语义 LIMITED PASS，audit package REVISE。
- 当前两项 finding 为 **A2-D4D5:P2-01**（source→current clause 审计映射）与 **A2-D4D5:P2-02**（真实 D10 direct-source qualification）。本作者修复只标 **author-resolved-pending-independent**。

D6 继续是保存至 829 的 stopped 作者候选；fixed-829 只读复核不使 D6 自动 accepted。

## 3. P2-01 复核目标

请审机器映射本身，不能只看计数：
- D4 fixed-S 文本：906/906 个非空行，保留精确 source path/blob/line/text，每行唯一归属一个 source-qualified obligation group。
- D5 fixed-S 文本：170/170，规则相同。
- Mandatory 1–924 行：716/716 个非空行映射到真实 owner/target；metadata/review 行不得冒充业务语义。
- D4 catalog：按 RFC 6901 覆盖 2,854/2,854 个 JSON Pointer，并无重叠分入 7 个子树组；文档根指针是空字符串 `""`，而 `"/"` 表示空键成员；95 个具名记录与 7 个全局限制继续只由固定 catalog 提供规范值。
- fixed97：72 条 D4 与 130 条 D5 selected row，逐条保存真实英文 row、真实中文 row及 current A2 target/disposition/owner/basis/oracle。

最小反例/正例：fixed-S D5 81–92 行必须被 D5 §0/§3/§19.1 具名 supersede；Organizations inverse 必须回到 D4 §8/§16.4，不能生成第二 authored inverse；phone occurrence 必须回到 D4 §10.1/§16.2 与 D5 §4/§14，不能变成 durable row identity；Calendar recurrence 必须回到 D4 §9/§10.3/§16.6/§17.5 与 current dependency proof。

## 4. P2-02 复核目标

机器映射按真实 blob 直接限定以下 6 份 D10 文件：
- CONTROL-CONTRACT.md / .zh-CN.md
- CANDIDATE.md / .zh-CN.md
- UPSTREAM-AMENDMENTS.md / .zh-CN.md

每份都标 `direct_partial_not_full_D10`，并把与 Package/Contribution、namespace/anti-spoof、provider availability、schedule/temporal、connector/credential、external effect 与 execution custody 直接相关的条款映射到现有 D4/D5 current clause。

D10 原文 Proof2/Key2/wire12/PAB3 等 predecessor/original-owner 引用逐字保留并做来源限定，禁止盲改。fresh/current authority 继续来自 D3-SCHEMAS §2/§6.1 的真实 A2 successor family 加 D4/D5 §0；真实 historical record 继续按 recorded decoder/bytes/pins/recovery。

## 5. 范围护栏

本次只改 audit/source map 与 review/progress 入口，不改 D1–D6 正确规范业务语义，不改产品实现、依赖、checker、CI、source snapshots 或 protected inputs，也不建立第二套 catalog/Registry/current authority。

D7–D10 完整 A2 所有者模块整合仍待完成；Mandatory 925–1141 仍待完成；全新的 Pro 级全局复核仍待完成。运行时、操作系统、GUI、真实副本、迁移、激活、部署、提供方、崩溃恢复与性能证据全部未运行。

### 5.1 Mandatory intake 与当前工作流资格

Mandatory source §1–§14 继续只是场景/压力输入，不会自行成为当前权威。历史 `$council`、OpenCode panel、controller/task 与聊天路由指令只保留为来源限定的工作流 provenance；当前仓库流程是单一作者 branch、明确的 source→current-clause disposition，再由新的非作者复核绑定一个精确 stop SHA。若来源与已有 current owner 冲突，仍须明确 owner/reopen disposition，不能在本批静默改写。

### 5.2 冻结与最终接受资格

Mandatory §10 列出的产品义务仍是各真实 owner 的有效义务，本审计修复不能把它们整体降为历史豁免。有界映射修复只有在精确来源/current target 审计闭合、该有界范围没有未关闭 P0/P1 或其他 finding 后，才可由独立复核接受。全局 A2 冻结还必须完成 D7–D10 完整 owner integration、Mandatory 925–1141、全部适用产品/测试义务，以及全新的 Pro/global 非作者复核。文档检查只能证明仓库一致性，不能替代语义接受。

### 5.3 Supersession 与交接资格

Mandatory §11–§11.3 继续作为 supersession 与产品意图的来源证据；其中旧 D3 时期的 controller/session 调度只属于历史工作流 provenance，不是当前执行命令。当前交接以 GitHub branch/PR 状态和上文记录的精确固定复核对象为准。fixed-a62 的 D4/D5 结论不得扩大到 D6；D6 仍单独绑定 stopped author candidate 829 及其尚未完成的只读复核。任何旧 council/session 指令都不会恢复此前中断的批次。

全局 A2 设计仍未接受。
