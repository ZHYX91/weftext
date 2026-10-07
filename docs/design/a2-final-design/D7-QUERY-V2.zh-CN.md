---
source_language: zh-CN
translation_status: source
---

[English](D7-QUERY-V2.md)
# A2 D7 QuerySpec/2 与当前元数据读取源

状态：这是 A2-D7-2F89-P1-02 的作者修订；属于当前 schema successor 候选，仍需独立复核。

## 1. 版本边界

QuerySpec/2 是 A2 新建 Query 的当前作者 schema。它保留 QuerySpec/1 的顶层成员、参数规则、DAG 规则、CEL profile、结果族以及所有 v1 算子；只有本文明确列出的当前读取源和 union_all 是新增项。外层执行 carrier 仍为 wireVersion2。

QuerySpec/1 不原地扩宽。若真实保存记录中存在 QuerySpec/1，就继续按准确的 /1 grammar 与错误规则解码和执行。copy、fork、import、export、QueryRef 与历史恢复都保留记录时的 QuerySpec 版本；不存在从 /1 自动改写到 /2 的部署迁移。用户明确编辑并另存为 /2 时，才按普通 D2/D3 规则产生新的 definition revision。

## 2. QuerySpec/2 的闭合新增

在 QuerySpec/2 中，read 算子保持 v1 的准确形状，并在全部 v1 source 之外只增加下列当前含义：

| Source | 适用 from | 准确输出 |
| --- | --- | --- |
| {kind:"title"} | NodeRef | Optional<text>；当前 D2 原生 title；合法无标题 Document 返回 none |
| {kind:"subtitle"} | NodeRef | Optional<text>；当前 D2 原生 subtitle；缺席时为 none |
| {kind:"node_file_name"} | NodeRef | text；当前 present D6 FileObjectBinding.relativePath 的准确 basename |
| {kind:"node_relative_path"} | NodeRef | text；使用 "/" 分隔的准确当前 D6 PortableRelativePath |
| {kind:"resource_file_name"} | ResourceRef | text；当前 present D6 FileObjectBinding.relativePath 的准确 basename |

其它 v1 read source 的形状与类型在 /2 中保持不变。尤其 body_text 仍消费 D2 语义产品，resource_descriptor 仍不暴露 path。

QuerySpec/2 还只增加一个通用 relation 算子：

~~~text
{id,op:"union_all",inputs:[relationId...]}
~~~

inputs 含 2..16 个 relation ID。每个输入的公共 schema 必须在 column 顺序、名称与 TypeSpec 上逐字节相等。union_all 保留全部 row 与重复项，不做 coercion 或 dedup，所有语义可达输入都必须执行，输出为无序 bag。任一输入错误都按普通 canonical error order 使整个 Query 失败。内部 occurrence identity 在 parent K 前加入 canonical input ordinal，因此不同分支的相同公共 row 仍是不同 occurrence。union_all 不跨分支授予 Action lineage；后续 Action 仍必须明确选择 Ref 并重新授权。

这个新增通用算子的稳定推导 feature ID 是 query.union_all.v1。它属于 QuerySpec/2，不是领域专用或 Search 专用算子。generic join、window 与 quantile 继续不支持。

## 3. D2 title 与 subtitle producer

title 与 subtitle 来自同一 current D2DocumentSnapshot/3 内的同一 D2DocumentMetadata/3。先完成 Ref-state disclosure 与既有 source_read gate，之后才允许读取 source。snapshot root SourceObservation/1、processor environment、document_format dependency、include 以及 metadata projection 实际使用的全部依赖，都必须在最终 D7 read barrier 仍然 current。

返回值就是这一次 D2 evaluation 的语义值。filename、path、first paragraph、placeholder text 或 NodeRef 都不能补造 title/subtitle。合法无标题 Document 的 title 为 none；不存在 subtitle 时 subtitle 为 none。D2 projection 非法或不可用时，在普通 disclosure 顺序之后返回既有 unavailable/not_visible，不能用 none 假装成功。

## 4. D6 Node 文件元数据 producer

node_file_name 与 node_relative_path 消费该 Node 真实 current D6 SourceObservation/1 以及其中 present FileObjectBinding/1。顺序闭合为：

1. 解码 QuerySpec/2 与 Ref；
2. 先做 D3/D6 Ref-state disclosure；
3. 要求当前 source_read；node_relative_path 还必须在任何 placement-sensitive path disclosure 前取得 current structure_state；
4. 在同一 observer cut 取得完整 current SourceObservation/1 与 present FileObjectBinding/1；
5. 验证 PortableRelativePath 和普通 D6 continuity/dependency proof；
6. 派生所需 text，并在发布前重验完整 Query barrier。

node_file_name 是 PortableRelativePath 最后一个非空 "/" segment。不得去扩展名、Unicode normalization、大小写折叠、percent decoding 或套用 host path 规则。node_relative_path 返回准确 canonical PortableRelativePath 文本，绝不是 host-native path。

absent、placeholder、conflict、gap 或其它无法证明 current binding 的状态，按既有 owner mapping 返回 source_unavailable/proof_unavailable。隐藏或未授权 Ref 返回 not_visible。旧 Observation 的相同 bytes、digest、basename 或 path 都不能恢复 current 资格。

## 5. D6 Resource 文件名 producer

resource_file_name 只适用于 ResourceRef。先做 Ref/owner disclosure，再执行既有 resource_read gate；随后 Core 在同一 cut 取得该 Resource 的完整 current SourceObservation/1 与 present FileObjectBinding/1，并按 §4 的相同算法派生 basename。

这个 source 不会额外授予 resource bytes；投影出的 basename 也不授 Resource identity、owner change、copy 或写权限。缺失或未 materialize 的 bytes/binding 返回 unavailable，不返回空名称。

## 6. 同一 cut 的依赖与失效

每个 metadata read 都记录真实 authorization dependency，以及实际产生它的准确 source/observation dependency。D2 title/subtitle 还记录 document_format 与完整 D2 evaluation dependency。文件元数据绑定准确 SourceObservation/1 与 FileObjectBinding/1。selector/query_scan dependency 继续独立证明选中的可见 population 与 negative range。

同一个 read batch 的全部 binding 必须来自一个完整 Query cut。rename、move 或外部 rename 只要改变 FileObjectBinding.relativePath，就会使旧 result 失效，即使 Ref 与文件 bytes 未改变。D2 metadata 改变通过 D2 snapshot dependency 使 title/subtitle 失效。若结构变化改变 selected subtree，即使单个 file path 未变，也由普通 placement/query_scan dependency 使结果失效。

这些 source 都不会把 filename/path 升格成 identity、Locator、Provenance、SourceVersion、ActionEvidence 或写权限。

## 7. 保存、复制、导入与 consumer 分派

SavedQueryDefinition、DynamicBlock 与 QueryRef 保留内嵌 QuerySpec 版本。Definition Transfer 按版本对应 schema 遍历 typed Ref/DefinitionAddress slot，不扫描 text、path 或 filename。D9 的 query_json copy/export/import 保留 QuerySpec 的准确 bytes 与版本；只有用户明确作者化新 revision 时才可改变。D8 editor 与 D7 Search compiler 在需要这些 current source 时，为新定义生成 QuerySpec/2。

v1 decoder 永不接受 subtitle、node_file_name、node_relative_path、resource_file_name 或 union_all。未知未来版本按既有 definition/version 边界失败。本修订不引入第二 Query registry、Search executor 或 migration ledger。

## 8. 验收 oracle

正例包括：titleless Node 只靠 node_file_name 命中；原生 title 与 basename 不同；Node move 改变 node_relative_path 但保留 NodeRef；缺失 subtitle 表示为 Optional none；Resource 只靠 resource_file_name 命中。

负例包括：隐藏 Ref、source_read/resource_read deny、没有 structure_state 却读取 path、placeholder 或 observation gap、rename 后 stale FileBinding、QuerySpec/1 携带 v2-only source、以及 union_all 输入 schema 不相同。任何一种都不能退回 index 值、shell 私下过滤、旧 Observation 或空结果。
