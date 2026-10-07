---
source_language: zh-CN
translation_status: source
---

[English](D7-QUERY-V2.md)
# A2 D7 QuerySpec/2 与当前元数据读取源

状态：这是 A2-D7-2F89-P1-02 的作者修订；属于当前 schema successor 候选，仍需独立复核。

## 1. 版本边界

QuerySpec/2 是 A2 新建 Query 的当前作者 schema。它保留 QuerySpec/1 的顶层成员、参数规则、DAG 规则、CEL profile、结果族以及所有 v1 算子；只有本文明确列出的当前读取源和 union_all 是新增项。外层执行 carrier 仍为 wireVersion2。

QuerySpec/1 不原地扩宽。若真实保存记录中存在 QuerySpec/1，就继续按准确的 /1 grammar 与错误规则解码和执行。QueryRef、reopen、历史恢复以及 D7 Definition Transfer 都先按记录中的 embedded QuerySpec version 分派。saved definition 的 copy/fork/import 因而沿既有 D3/D7 Definition Transfer 作者路径保留原 author payload。不存在从 /1 自动改写到 /2 的部署迁移。用户明确编辑并另存为 /2 时，才按普通 D2/D3 规则产生新的 definition revision。

D9 的 `query_json` 明确**不是** QuerySpec carrier。其真实 owner 只把已经完整成功的 D7 terminal result 序列化成 `weftext.query-result-export/1`：保留 TerminalSchema、typed terminal data、bag/order 语义以及 value 中真实作者 Ref，同时去掉 runtime handle/paging。它既不复制也不迁移 saved Query definition。

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

`inputs` 含 2..16 个 relation ID。每个输入的公共 schema 必须在 column 顺序、名称与 TypeSpec 上逐字节相等。union_all 保留全部 row 与重复项，不做 coercion 或 dedup，所有语义可达输入都必须执行，输出为无序 bag。任一输入错误都按普通 canonical error order 使整个 Query 失败。内部 occurrence identity 在 parent K 前加入 canonical input ordinal，因此不同分支的相同公共 row 仍是不同 occurrence。union_all 不跨分支授予 Action lineage；后续 Action 仍必须明确选择 Ref 并重新授权。

这个新增通用算子的稳定推导 feature ID 是 query.union_all.v1。它属于 QuerySpec/2，不是领域专用或 Search 专用算子。generic join、window 与 quantile 继续不支持。

## 3. D2 title 与 subtitle producer

title 与 subtitle 来自同一 current D2DocumentSnapshot/3 内的同一 D2DocumentMetadata/3。先完成 Ref-state disclosure 与既有 source_read gate，之后才允许读取 source。snapshot root SourceObservation/1、processor environment、document_format dependency、include 以及 metadata projection 实际使用的全部依赖，都必须在最终 D7 read barrier 仍然 current。

返回值就是这一次 D2 evaluation 的语义值。filename、path、first paragraph、placeholder text 或 NodeRef 都不能补造 title/subtitle。合法无标题 Document 的 title 为 none；不存在 subtitle 时 subtitle 为 none。D2 projection 非法或不可用时，在普通 disclosure 顺序之后返回既有 unavailable/not_visible，不能用 none 假装成功。

## 4. D6 Node 文件元数据 producer

`node_file_name` 与 `node_relative_path` 只消费 D6 §19 / D6-SCHEMAS §11 定义的 D6-owned `D6FileBindingMetadataObservation/1`。D7 不直接读取 FileObjectBinding，也不自行发明 path capability。

顺序闭合为：

1. 解码 QuerySpec/2 与 NodeRef；
2. 执行普通 D3/D6 Ref-state disclosure；
3. D6 在读取 FileBinding locator metadata 前，按 deny-before-allow/default deny 对该准确 Ref 要求既有 `entity_state` 与 `locator_state`；
4. D6 在一个 observer cut 内建立完整 current 受保护 SourceObservation/1 与 present FileObjectBinding/1，只返回受保护 metadata observation/value；
5. D7 验证 projection，并绑定该准确 observation 与当前 authorization dependency；
6. publication 前重验完整 Query barrier。

仅为了披露 FileBinding locator，这些 source 不要求 `source_read`、`resource_read`、`structure_state` 或 `source_envelope_state`。同一 Query 若另外读取 body/source/Resource bytes，那个独立 read 仍须通过原 content capability。metadata producer 可以为正确性内部验证受保护 SourceObservation/FileObjectBinding evidence，但这些受保护 bytes/token 不会作为 Query cell 暴露。

`node_file_name` 取 PortableRelativePath 最后一个非空 "/" segment，不去扩展名、不做 Unicode normalization、大小写折叠、percent decoding 或 host path 规则。`node_relative_path` 返回准确 canonical PortableRelativePath 文本，绝不是 host-native path。

absent、placeholder、conflict、gap 或其它无法证明 current binding 的状态，在授权后按 D6 owner mapping 返回 source_unavailable/proof_unavailable；隐藏或未授权 Ref 在 locator metadata 读取前返回 not_visible。旧 Observation 的相同 bytes、digest、basename 或 path 都不能恢复 current 资格。

## 5. D6 Resource 文件名 producer

`resource_file_name` 只适用于 ResourceRef，并使用同一个 D6-owned metadata producer 的 projection=basename。Ref/owner disclosure 与准确的 `entity_state` + `locator_state` gate 必须先于 FileBinding metadata 读取。仅为披露获权 locator label 时不要求 `resource_read`；另行读取 Resource bytes/descriptor 仍使用其原 gate。

投影出的 basename 不授 Resource identity、owner change、copy right、ByteHandle 或写权限。binding 缺失、未 materialize 或存在 gap 时返回 unavailable，而不是空名称。

## 6. 同一 cut 的依赖、直接 consumer 与 union identity

每个 metadata read 都记录真实 authorization dependency，以及实际产生它的准确 D6 受保护 SourceObservation/FileObjectBinding。D2 title/subtitle 还记录 document_format 与完整 D2 evaluation dependency。selector/query_scan dependency 继续独立证明选中的可见 population 与 negative range。

同一个 read batch 的全部 binding 必须来自一个完整 Query cut。Policy/auth generation 改变、rename、move 或外部 rename 只要改变 FileObjectBinding.relativePath，就会使旧 result 失效，即使 Ref 与文件 bytes 未改变。D2 metadata 改变通过 D2 snapshot dependency 使 title/subtitle 失效。若结构变化改变 selected subtree，即使单个 FileBinding path 未变，也由普通 placement/query_scan dependency 使结果失效。

这些 source 都不会把 filename/path 升格成 identity、D3 Locator、Provenance、SourceVersion、ActionEvidence 或写权限。

### 6.1 QuerySpec/2 title consumer

fresh QuerySpec/2 的 `title` 类型是 Optional<text>。任何仍需要 nonoptional display label 的 retained current consumer，都必须显式适配，不能把 source type 偷偷改回 text。

Temporal Query/View 正向链在 fresh /2 witness 中读取 `title:Optional<text>`，在 terminal/a11y data 中继续保留这个 Optional column，并在 Calendar/Timeline projection 前显式派生 `displayTitle = row.title.orValue('')`。View 绑定 nonoptional `displayTitle`。空 display text 只是 presentation value：D8 可以依据独立的 Optional title state 显示本地化“无标题”占位，但空值/占位都不能回写成作者 title、参与 identity，也不能 fallback 到 filename/path。

link display 仍先使用 explicit source label；否则 fresh authorized target-title read 返回 Optional<text>：some(v) 投影 v，none 投影 titleless/empty display label。target 不可读时继续使用 call site 已知的 unavailable-link presentation state。绝不能查询 filename/path 来补造 target title。

真实 QuerySpec/1 definition 保留准确 /1 title 类型与 decoder，不因为 current D2 允许 titleless Document 就重解释为 Optional，也没有自动 migration。

### 6.2 `union_all` 的 LogicalOccurrenceKey 与顺序

QuerySpec/2 只给 retained closed K tagged tree 增加下面一个 constructor：

~~~text
{kind:"union_all",
 base:{invocation:I,operator:o},
 inputOrdinal:Counter,
 parent:K}
~~~

`inputOrdinal` 是作者 `inputs` array 中对应 entry 的从 0 开始 index，范围 0..15。array order 属于语义，因此 canonicalization 必须保留。允许重复 relation ID：`union_all [r,r]` 会把每个 parent occurrence 复制两份，分别标为 inputOrdinal 0 与 1；即使 public cells 和 parent K 完全相同，也仍是两个 bag occurrence。

共享 upstream relation node 按普通 DAG 只求值一次；每个 union input occurrence 把完整 rows 复制到自己的 branch，并按既有 checked budget 计 union output row/byte/work。parent relation error 继续使用原 canonical operator/error ordering；union schema mismatch 是 union node 上的静态 QuerySpec/2 type/graph error。`union_all` 输出无序，所以显式 `sort` 之前不能使用 `take`。sort key 完全相等后，包含 `inputOrdinal` 的完整 K tree 提供既有 deterministic internal tie，因此 paging 不会把重复 branch 合并或改序。

`union_all` 仍擦除 implicit Action lineage。后续 Action 必须选择 explicit Ref 并 fresh authorize；新的 K 绝不是 public identity 或 ActionEvidence。

## 7. 保存、复制、导入/导出与 consumer 分派

SavedQueryDefinition、DynamicBlock 与 QueryRef 保留 embedded QuerySpec version。Definition Transfer 按版本对应 schema 遍历 typed Ref/DefinitionAddress slot，不扫描 text、path 或 filename。saved-definition copy/fork/import 走该 D7 Definition Transfer 与普通 D3 author operation；除非用户明确作者化新的 definition revision，否则 embedded QuerySpec bytes/version 保持不变。

D9 的 `query_json` 继续只是一种 terminal-result export profile：它序列化完整结果的 TerminalSchema 与 data，绝不承载 QuerySpec author bytes。因此导入这种 result file 不能创建、迁移或刷新 saved Query definition。

D8 editor 与 D7 Search compiler 在需要这些 current source 时，为新定义生成 QuerySpec/2。v1 decoder 永不接受 subtitle、node_file_name、node_relative_path、resource_file_name 或 union_all。未知未来版本按既有 definition/version 边界失败。本修订不引入第二 Query registry、Search executor 或 migration ledger。

## 8. 验收 oracle

正例至少包括：主体只有 `entity_state+locator_state`、没有 `source_read` 时，titleless Node 仍可仅凭 node_file_name 命中；没有 `resource_read` 时可按 Resource basename 命中；原生 title 与 basename 不同；Node move 改变 node_relative_path 但保留 NodeRef；缺失 subtitle 表示为 Optional none；titleless temporal item 的 QuerySpec/2 Calendar witness 派生非作者 displayTitle 而不写 title；可读但无标题的 link target 永不 fallback 到 filename；以及 `union_all [r,r]` 经显式 sort/paging 后仍保留两个 occurrence。

负例至少包括：隐藏 Ref、缺少 `locator_state`、placeholder 或 observation gap、rename 后 stale FileBinding、另行读取 body/Resource bytes 时缺少 content capability、QuerySpec/1 携带 v2-only source、union_all 输入 schema 不相同，以及对 unordered union_all 直接使用 `take`。任何一种都不能退回 index 值、shell 私下过滤、更强 content permission、filename-as-title、旧 Observation 或空结果。

saved-definition copy/fork/import 必须通过 Definition Transfer 保留记录中的 embedded QuerySpec version。D9 query_json 必须继续只是 terminal result serialization，绝不能被当作 QuerySpec author source。
