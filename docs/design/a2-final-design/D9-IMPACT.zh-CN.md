---
source_language: zh-CN
translation_status: source
---

[English](D9-IMPACT.md)

# A2 D9 实现影响与测试轮廓

状态：**author-resolved-pending-independent-review**。本文列实现义务，不是实现证据。

## 1. 替换边界

Core 继续保留 D2/D4 领域解释和 D3/D6 事务权威。conversion/Office/XML/model/runtime dependency 留在可选 coordinator/worker package。实现必须把未发布 legacy ImportIr/YAML proposal decoder、free provider command/fallback alias、旧 attr/record/H1–H9/formula-reorder/broad-view template parser、free export dictionary、rowHandle identity 与 D9-private Locator alias 作为一个 migration set 清理；historical research file 只保留历史证据。

current implementation 必须面向 D3 wire13、PAB4、Effect3、D2Snapshot3 与 ExportPlan/Receipt3。真实已存 historical record 仍用 exact decoder，而不是 current/historical 双写。

## 2. I01–I12 gates — 全部 UNRUN

| Gate | 必需证据 |
| --- | --- |
| I01 IR/codec | 完整 strict decoder：closed member/union、duplicate/unknown/null/Unicode/Counter/order/budget、format coverage，以及独立 parser/render 对照。 |
| I02 hostile files | ZIP alias/traversal/link/device/bomb、ODF repeat、XML DTD/XXE/entity、active/encrypted/unknown variant、恶意 stdout/slot/exit-zero truncated output 与全部 limit。 |
| I03 OS sandbox | 具名 Windows/Linux/macOS build 的 file/network/process-tree/resource/cleanup 隔离证据，不能只看 timeout/container 名。 |
| I04 mapping/admission | 完整 D2 parse/product、D4 Registry/type/qualifier/cardinality/reference/control、fresh-root authority，以及 ConversionInput/Result9/PAB4/Effect3 字节闭合。 |
| I05 real ImportJob | 10k Nodes/10 batches/超过内存输入、SCC/coupling group、真实 D6 commit 前后 fault、lost receipt、restart、revoke、cancel、TTL/pins。 |
| I06 Office template | 真实 Word/WPS/LibreOffice DOCX/ODT/XLSX/ODS，split/mixed style、escape、非法 XML scalar、typed none/complex value、visible native selector、0/1/N repeat、merge/limit。 |
| I07 document output | H1–H9 current source semantics、target loss、body/bibliography 单一放置、style bundle、CJK/RTL/AT/font/pagination 的具名 Office 版本证据。 |
| I08 typed export | D7 各 result domain、reset/order/bag/tie/none/graph、任意精度值、date/instant、公式注入反例与 typed spreadsheet output。 |
| I09 publication | 可证明的 create-only filesystem boundary；ENOSPC/name conflict/user move/flush/rename crash/revoke race；unknown 不换名重发。 |
| I10 region | Crop/MediaBox/UserUnit/Rotate/EXIF/density/rounding、d9rg1 + l1 currentness、copy/fork、stale/not-visible、keyboard/AT。 |
| I11 surfaces | Desktop/CLI/Server/WebUI parity、Mobile negative capability、D1 overlap reason priority、nondisclosing error、D8 generated proposal/Draft conflict。 |
| I12 release/naming | 完整 dependency/SBOM/license/model/font/platform install/uninstall、无 provider/config backdoor、legacy-name scan、capability 与真实 gate 绑定。 |

某个 profile 通过只开放精确 format × operation × variant × route × platform × version 组合。library/profile semantics 变化必须重跑受影响 gate。

## 3. Mandatory §14 Office binding 实现

D9 Main/Schemas 的 visible qualified selector grammar 是实现义务。compiler 必须从 template bytes 解析 exact JSON-string component 和 canonical occurrence suffix，编译成 D9NativeTableSelector/1，强制 shortest-unique qualification，并验证一个 repeat band 的全部 token 指向同一 table/rowset/order。

fixture 必须覆盖：
1. 相同 leaf 的重复 table；
2. multi-row header 的 shortest suffix；
3. 相同 title + full path 仍需 table/column occurrence；
4. CJK、RTL、combining、emoji、space、slash、double-colon、quote、bracket；
5. repeat band same-source unification；
6. 后续碰撞使 fresh selector ambiguous，但 frozen Plan 保持；
7. ordinary Document table、schema-bearing collection、ordinary no-template XLSX 三域；
8. copy/move/style 与 stale template；
9. no-template XLSX 正向；
10. Mobile unavailable、corrupt/malicious input 与 stale authority。

hash-derived nt_/nc_ 只测试为 internal Plan key 或 genuine historical authoring，不再是新的 qualified user spelling。

## 4. Worker 与 provider 证据

Docling Lite current install evidence 的 completeForExecution=false，因此继续 unavailable。XPS/OXPS、OFD、CAJ/HN 不能从候选 library、extension recognition 或其它 variant 继承 availability。每个 provider record 必须绑定 exact dependency version、license/distribution decision、model/font asset、sandbox evidence 与 corpus test。

Worker budget 在 route step/retry 间累计。cleanup 必须证明整个 process tree 已终止；不确定 cleanup 隔离 output 并 disable route。Worker output 在 pin 前独立 decode/validate。

## 5. Import 与 mapping 证据

CSV fixture 包含 quoted newline、double quote、duplicate header、ragged record、empty file 与 exact Unicode。Workbook fixture 区分 numeric lexeme、blank/absent/empty、formula/cache、hidden sheet/row/column、merge anchor/covered cell 与 source coordinate。Page/flow fixture 保留完整 geometry/reading order/unrepresented issue。coverage 漏项即使 IR decoder 通过也必须失败。

D4 mapping test 使用真实 Registry definition，覆盖 type/qualifier/cardinality/relation/Calendar 正反例。不得按 label 映 Field，也不得让 fresh root 借 existing-owner 权限。

## 6. Node Template 证据

测试覆盖 duplicate source、self/cross-template fresh subject rewrite、owner-local Resource remap、external-current reference policy、只枚举 index 而不读 body 的 Annotation omission、parameter type、title/body_text/field_append overlap、D2 reparse equality。

parent-import 与 simple collection 分支按原 D3/D7 request 测试。sourceSubjectBindings 与 receipt resultAllocations 必须唯一 join，不保存第二 identity map。

## 7. PAB4 与 recovery 证据

fresh current 测试使用 PAB4 与 wire13 mode-legal shape。旧 PAB1/2/3/wire11/12 只测试 recovery，不静默升级。读取大 record 前证明 MinimumMapping。page 1 前已有完整 preview bytes/effects，pagination 零 semantic rerun。

saved/planned/unknown 保留原 request/OperationId/pins/owner，不依赖 expired preview TTL 或 current business validation，除非 current disclosure/custody fence 本身要求。

## 8. Export 证据

覆盖 renderer registry unavailable 时 exact-source/resource/query_json generationPolicy=none；rendered document 的 D2Snapshot3 + D8 presentation binding；body/bibliography selection；narrow Field/Query/native_table 不额外读 body；graph/scalar/rows 完整值；canonical set ordering；current output-name validity/PortableAlias/reserved-name rule；Plan3/Receipt3 与 historical strict dispatch。

initial loss report、data bytes、loss-report.json、manifest.json、stagedOutputs 在 confirm 前冻结并验证。confirm 后任何变化必须失败，不能 rerender。

## 9. Typed format 与 image 证据

CSV dangerous starter 包含 ASCII/fullwidth 及 leading Unicode White_Space。TSV 内嵌 separator/newline 拒绝。XLSX/ODS 的 text 保持 text，任意精度 numeric policy 与 none/empty/absent 分离，永不计算 formula。

image physical size 只用真实 density fact、exact ratio 与 half-even rounding。missing/invalid/conflicting density 分支分离。explicit imageSizes 是 output layout policy，不是 source evidence。

## 10. Publication 与 Resource handoff

真实 filesystem test 覆盖 create-only atomic bundle、flush order、conflict、rename 前后 crash、user move、restart 与 revoke race。recovery 不换 final name。external PublicationReceipt/3 与 author Resource receipt 分离并独立恢复。

## 11. Evidence ledger

historical bounded evidence 继续分开：57、63、59、82、90、130 以及 12-scenario chain。本作者批不重跑，也不升级这些证据。current Design documents workflow 最多证明 repository consistency。

I01–I12、真实 Worker、Office/WPS/LibreOffice、OS sandbox、真实 D6 fault injection、performance、release、deployment 在实际实现证据出现前全部 **UNRUN**。
