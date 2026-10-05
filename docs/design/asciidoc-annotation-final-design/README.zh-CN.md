---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# AsciiDoc / Annotation 最终设计候选

状态：**candidate-design-not-implemented**。

本目录把已完成独立方案 Gate 的完整 AsciiDoc、Weftext 必要扩展、独立 JSON Annotation，以及它们对 fixed-e8aa D2–D10 actual owners 的 FC-3.4→d 协调，整理为一个可公开、可独立复核的设计候选。

- parent commit：`e8aa0b341630a57c786c0891d4bbd1620247441d`
- parent PR branch：`fix/pl-ir-01-portable-locator-requalification`
- immutable accepted input S：`7e18168dad3e6d120fce0dd607dc10fa7894e252`
- S49 snapshots 和 `docs/design/inputs.json`：**不修改**
- 本目录：设计候选，不是实现、发布或 A2/global 接受。

## 阅读顺序

1. [REVIEW-ENTRY.zh-CN.md](REVIEW-ENTRY.zh-CN.md)
2. [SPEC.zh-CN.md](SPEC.zh-CN.md) — 行为、算法与 owner replacement
3. [SCHEMAS.zh-CN.md](SCHEMAS.zh-CN.md) — current closed shapes、排序、历史分派
4. [ACCEPTANCE.zh-CN.md](ACCEPTANCE.zh-CN.md) — 750 条逐项未运行设计义务
5. [terminology-registry.json](terminology-registry.json)
6. [replacements.json](replacements.json)

`replacements.json` 将 fixed-e8aa actual owner/blob/规范关注点精确路由到 SPEC/SCHEMAS/ACCEPTANCE/registry 的 replacement surfaces。SPEC 的摘要不能扩张或缩减 SCHEMAS 的 closed shape；ACCEPTANCE 的逐项场景是规范义务而不是测试通过记录。未被点名的 owner 正文保持 fixed-e8aa 语义；被点名的冲突合同由本候选替换，而不是在旧正文末尾追加私有补丁。

本候选采用一个联合 PR，因为 managed document format、PAB4、EffectManifest3、D10 recovery 与 BootstrapPlan4 共用同一 closed successor family，硬拆会要求临时同版本扩臂。

## 验收记账

- 核心设计 oracle：438
- actual-owner coordination fixture：312
- 合计：750
- 状态：**全部未运行**。这些是设计验收义务，不是产品测试结果。

作者只执行文档/JSON/router一致性检查；最终设计接受必须由非作者对停止写入后的 exact PR head SHA 完成。
