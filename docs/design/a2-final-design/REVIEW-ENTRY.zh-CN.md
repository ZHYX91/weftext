---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1-D5 作者候选复核入口

状态：D4/D5 整合批次后停止写入的作者候选；未独立接受、未实现、未合并、未发布、未部署、未全局冻结。

## 1. 固定 base 与复核绑定

Base branch：docs/asciidoc-annotation-final-design。
固定 base SHA：97f4734f82a760cb6716c8122b84494da2b61164。
固定历史输入 S：7e18168dad3e6d120fce0dd607dc10fa7894e252。
受保护 inputs blob：787d03c31a55496f81ed03fd54a6fdfff50a2ad4。
候选 branch：docs/a2-final-design-integration。

复核者必须绑定本批产生的 exact PR5 head；不得跟随 moving branch，也不得把任何旧 fixed-SHA 结论外推到其原范围之外。

## 2. 既有独立评审状态

非作者 fixed-44617a4 报告在其有界范围内独立 CLOSED D1/D2 的 P1-01 与 P2-01。

同一报告把 D3:P1-01 保持 OPEN，问题是 current successor 与 predecessor dispatch / fourteen-key 混用；把 D3:P1-02 保持 OPEN，问题是 current canonical Annotation 仍保留 Value3 enum。该报告只评 44617a4ddf3bbe538c7a9e2bab9fe8a4013fe706，不评 edaf446、D3 文档机械 delta 或本次 D4/D5 作者 delta。本作者在本批不关闭这两个 D3 finding。

## 3. D4/D5 已完成作者工件

D4.zh-CN.md、D4-IMPACT.zh-CN.md、D4-LEXICON.zh-CN.md 构成 D4 current 作者候选。D4-SOURCE-MAP.json 保存 fixed input、mandatory intake、acceptance row 与 direct producer/consumer provenance。D4-CATALOG-MAP.json 映射全部 catalog JSON Pointer，而固定不可变 catalog 继续是唯一静态 catalog authority。

D5.zh-CN.md、D5-IMPACT.zh-CN.md、D5-LEXICON.zh-CN.md 构成 D5 current 作者候选。D5-SOURCE-MAP.json 保存 fixed D5 输入、mandatory intake、acceptance row、direct producer/consumer provenance，以及 full-native-table/titleless/six-row-domain 的显式 supersession。

不新增单独的 D4/D5 schema authority。D4 闭合 schema 继续自包含在 D4 主文 exact-contract 段；D5 table/collection/intent 合同继续自包含在 D5 主文 exact-contract restoration。历史 inner version 与 SourceRevisionPlan 分支保持来源限定，不机械改数字。

## 4. 阅读与映射范围

本批 fixed-S 全文 intake：
- D4 main、Impact、Lexicon 与完整 reference catalog；
- D5 main、Impact、Lexicon。

当前 parent 的 D4/D5 main、Impact、Lexicon 均按完整双语文件消费，并在其上叠加 current overlay。

Mandatory scenario input §1–§14、1–924 行已为 D4/D5 阅读并映射，包含未编号 prose 与 Office table binding section；925–1141 行留给后续 D6–D10。

fixed97 acceptance 表整体已解析。D4-SOURCE-MAP 保留 72 条 D4 相关 row；D5-SOURCE-MAP 保留 130 条 D5 相关 row。D4 catalog 映射包含 2,854 个 JSON Pointer、95 个具名记录与 7 个 global limits。

D6–D9 owner 文件只读取 D4/D5 direct producer/consumer 交叉；属于 partial，不是完整模块整合。parent owner 树没有独立 D10 owner 文件，因此 D10 只做 fixed97 SPEC/SCHEMAS direct-partial。

## 5. 待完成与未运行范围

D6–D10 完整 A2 owner 整合仍待完成。Mandatory input 925–1141 行仍待完成。D3:P1-01 与 D3:P1-02 继续 OPEN，留待之后绑定 exact head 的窄复核/修复周期。

Runtime、OS、GUI、真实 replica、产品实现、部署、migration、activation 场景全部 UNRUN。文档/source/schema 映射检查不是产品证据。

最终 A2 仍需完成 D6–D10，固定 final SHA，fresh non-author global review，以及 same-accepted-design-SHA freeze/startup 工作。
