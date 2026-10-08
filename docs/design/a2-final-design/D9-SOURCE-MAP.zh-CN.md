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
- final AsciiDoc/Annotation successor：已完整阅读 D9 直接相关的 SPEC/SCHEMAS/ACCEPTANCE 中英文；fresh author submission 继续使用 wire13/PAB4/Effect3。fresh export 使用 final-FC ExportPlan/4，并按交付类型使用 PublicationReceipt/4 或 D9PrintReceipt/1；Plan/Receipt1–3 保留精确的记录型恢复。
- D10：只读取具名 D9 direct intersection，记为 PARTIAL；完整 D10 仍 UNREAD，留后续。

本修复批新增三项仅作者已修、待独立复核的 disposition，且不改 fixed input：（1）final FC §6.6/§16a 增加唯一封闭 Annotation-content carrier，annotation_index 继续只用于 omission；（2）增加一个基于完整 D7 result/ViewSpec/runtime gate 的有限 current View render binding，消费 Mandatory §15，并只对具名 metric/bar/line/scatter/pie/heatmap 的 DOCX/XLSX/PDF/SVG/PNG/print profile 给正向路径；不属于 D7 现任封闭集合的布局先返回 unsupported_layout，合法 D7 布局若缺少这条 D9 profile/backend 才返回 renderer_unavailable；（3）修正 predecessor/current source range。此前 visible native-table Office authoring 修订保持不变：唯一 lowercase-ASCII leaf 继续使用 data.native_table.COLUMN；fresh qualified/non-ASCII 使用可见 native.table[...]::column[...] bytes，nt_/nc_ 仍只是内部 Plan key。

受保护 S49 与 docs/design/inputs.json blob 787d03c31a55496f81ed03fd54a6fdfff50a2ad4 保持 immutable。historical 有界 evidence set 57/63/59/82/90/130/12 分开记账。source coverage、count、hash 与绿色 docs check 都不是语义接受。

范围修正继续固定在已复核的 ff10 前身：当时 final-FC SCHEMAS §6.5.1 精确为 EN L2224–2260 / ZH L2328–2364，后续 §7 单独保留；SPEC §16 精确为 EN L940–949 / ZH L1002–1011，后续 §17 单独保留。current successor 映射现在指向实际最终 blob：SCHEMAS EN §6.5 L1997–2225、§6.5.1 L2226–2262、§6.6 L2263–2574；ZH §6.5 L2101–2336、§6.5.1 L2337–2375、§6.6 L2376–2687。SPEC EN §8.3 L439–463、§8.4 L464–471、§16 L946–955、§16a L956–972、§17 L973–980、§18 L981–995；ZH §8.3 L449–483、§8.4 L484–491、§16 L1012–1021、§16a L1022–1038、§17 L1039–1046、§18 L1047–1153。

协调者发现的版本预检已经在原 D9 三修 scope 内协调，不新增 finding 计数：FC §8.3 继续保留 exact Source/Resource/query_json 的 generationPolicy=none、受控名称与 canonical ordering、template/route/style、递归 pin 和 unknown-publication 义务，但 fresh unseen work 统一使用 Plan4/Receipt4 或 PrintReceipt1。SPEC §17 的 unseen-current 分派、§18 类型清单、SCHEMAS §§6.5–6.6、terminology registry、FC acceptance 与 replacement router 已一致。真实记录的 Plan/Receipt1–3 保留精确 decoder/token/bytes/pins/confirmation/saved-planned-unknown recovery，绝不迁移。

D9-BAF-P1-01、D9-BAF-P1-02、D9-BAF-P2-02 仍只是 author-resolved-pending-independent-review。ROOT-D9-ZH-P2-01 已在 ff10 独立 CLOSED，不重开。产品/runtime 证据继续 UNRUN。
