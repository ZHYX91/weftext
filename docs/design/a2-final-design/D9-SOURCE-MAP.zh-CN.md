---
source_language: zh-CN
translation_status: source
---

[English](D9-SOURCE-MAP.md)

# A2 D9 来源映射

状态：**author-resolved-pending-independent-review**。

机器权威为 D9-SOURCE-MAP.json。

- fixed S：八份 D9 source 均 FULL 阅读，并以连续无缺口的 section/line range 从第 1 行覆盖到各文件末行。
- current D9 owner afterimage：八组中英文件均 FULL。
- Mandatory source §14 的 884–924 行：FULL。
- Mandatory source §15.5–15.8 的 1042–1141 行：对 D9 适用的 export/View 交叉为 FULL；这不批准 D7/D8 deferred layout。
- current D7 D9-binding 与 PreparedActionBinding owner 中英对：FULL。
- final AsciiDoc/Annotation successor：已完整阅读 D9 直接相关的 SPEC/SCHEMAS/ACCEPTANCE 中英文；fresh author submission 继续使用 wire13/PAB4/Effect3。fresh export 改用具名 final-FC Plan4/Receipt4 successor；Plan/Receipt1–3 保留精确历史恢复。
- D10：只读取具名 D9 direct intersection，记为 PARTIAL；完整 D10 仍 UNREAD，留后续。

本修复批新增三项仅作者已修、待独立复核的 disposition，且不改 fixed input：（1）final FC §6.6/§16a 增加唯一封闭 Annotation-content carrier，annotation_index 继续只用于 omission；（2）增加一个基于完整 D7 result/ViewSpec/runtime gate 的有限 current View render binding，消费 Mandatory §15，且只对具名 metric/bar/line/scatter/pie/heatmap 的 DOCX/XLSX/PDF/SVG/PNG/print profile 给正向路径；（3）修正 predecessor/current source range。此前 visible native-table Office authoring 修订保持不变：唯一 lowercase-ASCII leaf 继续使用 data.native_table.COLUMN；fresh qualified/non-ASCII 使用可见 native.table[...]::column[...] bytes，nt_/nc_ 仍只是内部 Plan key。

受保护 S49 与 docs/design/inputs.json blob 787d03c31a55496f81ed03fd54a6fdfff50a2ad4 保持 immutable。historical 有界 evidence set 57/63/59/82/90/130/12 分开记账。source coverage、count、hash、绿色 docs check 都不是语义接受。
范围修正：在 ff10，final-FC SCHEMAS §6.5.1 精确是 EN L2224–2260 / ZH L2328–2364，不是此前宽泛范围。原误带入的后续 §7 Annotation owner 继续单独保留（ff10 EN L2261–2421 / ZH L2365–2528）。SPEC §16 精确是 EN L940–949 / ZH L1002–1011；后续 §17 recovery 边界另行保留（EN L950–957 / ZH L1012–1019）。本批 current successor 另以新 blob 映射 §6.6 与 §16a；旧 bytes 不改写。

受保护 S49 与 docs/design/inputs.json 保持 immutable。source coverage、count、hash、绿色 docs check 都不是语义接受。D9-BAF-P1-01、D9-BAF-P1-02、D9-BAF-P2-02 均只是 author-resolved-pending-independent-review；ROOT-D9-ZH-P2-01 仍是单独的 ff10 修复并正在独立复核。产品/runtime 证据继续 UNRUN。
