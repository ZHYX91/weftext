---
source_language: zh-CN
translation_status: source
---

[English](D7-SEARCH.md)
# A2 D7 搜索语义与 SEARCH-01–08

状态：这是当前 D7 作者候选中的规范搜索设计。fixed85bdadf 独立复核已关闭此前 Registry/QuerySpec coordination finding，只留下 A2-D7-2F89-P2-01；本修订只完成该 machine-oracle 残余的作者修复，仍需 fixed-SHA 独立复核。D8 的具体交互实现继续属于后续完整模块批次。

## 1. 方案裁决与唯一执行权威

产品方向保持不变：默认使用普通文本，可视化筛选承担常规结构化条件，熟练用户可以显式进入快捷条件模式。普通用户无需学习快捷语法；普通搜索框也不会把所有冒号自动解释成操作符。

所有成功搜索都由 Core QuerySpec/2 执行。不存在第二个全文搜索执行器、shell 私下过滤、搜索专用数据存储或第二持久权威。Node 与 Resource 同时启用时，一个 QuerySpec/2 分别建立同 schema 分支，再用 union_all 合并，最后统一 sort/project。

保存时只保存 canonical Query definition，不保存快捷文本、可视化 chip 顺序或设备交互状态。

## 2. 搜索对象与当前 Query source

搜索词汇继续严格分域：

- document title：current D2 native title，类型为 Optional；
- document subtitle：current D2 native subtitle，类型为 Optional；
- Node file name：由 QuerySpec/2 的 node_file_name source 从当前 FileBinding 派生 basename；
- Node relative path：由 node_relative_path 返回准确 PortableRelativePath；
- Resource file name：由 resource_file_name 返回 basename；
- logical hierarchy：D3 selector/placement scope，不是稳定 path identity；
- body：D2 semantic body_text；
- contributed text：显式选择的 current SearchContribution Field/textPath。

前五个内建 source 的授权、SourceObservation/FileObjectBinding、同 cut、rename/move reset 以及“不授身份/写权限”规则由 D7-QUERY-V2 统一定义。无标题 Document 继续无标题，filename/path 永不补造 title。相同 title 或 file name 也不会合并不同 Ref。

## 3. Preset、空输入与 source enablement

File List filter 默认只看当前选中 parent 的直接 live Node，不递归。普通文字默认匹配 title 与 node_file_name。body、subtitle、path、Resources、Annotation body、Trash、attachment extraction/OCR 默认关闭，只有显式打开才参与。没有普通 needle 且没有 structured condition 时，此 surface 是 browse mode，不执行 Search Query。

Quick Open 默认搜索 selected Workspace/root 下获权的 live Node，并启用 title、subtitle、node_file_name。body 与 path 默认关闭。Resources 控件会加入 Resource domain 与 resource_file_name。没有 needle 且没有 structured condition 时，Quick Open 也是 browse mode，不把空 needle 当搜索。

Global Search 默认搜索 selected Workspace/root 或明确 subtree 中的获权 live Document，并启用 title、subtitle、body。Node filename/path、Resources、attachment extraction/OCR、Annotation body 与 Trash 都必须显式开启。没有 needle 且没有 structured condition 时，Global Search 不枚举整个 Workspace；它保持未执行 draft，提交时返回 invalid_request。

即使没有普通 needle，只要有 structured field/source condition 就是合法搜索。快捷 source term 还会显式启用对应 source：例如 Quick Open 中的 @body:x 会开启 body，@path:x 开启 Node path，@resource-name:x 会开启 Resource domain 与 resource_file_name。对应的可视化控件修改同一 compiler input。权限或 provider 失败仍然是 Query 失败，显式 source 绝不能被静默丢弃。

source applicability 是闭合的。title/subtitle/filename/path/body/@field 只适用于 Node；resource-name 只适用于 Resource。NOT 保持 child 的 domain set，AND 取交集，OR 取并集。AND 的 domain intersection 若为空，返回 source_not_applicable，而不是偶然得到零结果。这样 NOT @title:x 也不会变成“匹配所有 Resource”。

## 4. 匹配、Unicode、Boolean 组合与排序

当前 matching boundary 不变。内建 title/subtitle/body 与文件元数据使用各自声明的 exact comparison。D4 text path 若声明 normalization:"exact"，就执行区分大小写的精确比较；若声明 normalization:"nfc-for-compare"，只在比较时分别对 candidate 与 needle 做 NFC，不改写 source bytes。contains 在选定 comparison basis 上执行精确 substring。系统不自动 case-fold、fuzzy、stemming、tokenizer、transliteration 或 Pinyin。

CJK 与 RTL 都按普通 Unicode scalar 文本处理；RTL 只改变呈现。普通 plain-search mode 把整段输入当一个 literal phrase，AND、OR、NOT、colon、@、URL 语法与 parentheses 在这里都没有特殊含义。

默认 rank 中，title exact 为 0；subtitle、Node/Resource file name 以及 contribution role=name 的 exact 为 1；alias exact 或 Node relative path exact 为 2；非 body 的 substring 为 3；body/content-only 为 4。只有已经获权的值可以进入 rank。rank 相同后先使用明确的用户 sort。canonical subject/source-occurrence 的稳定 tie 由 Query Algebra §3 已规定、始终追加的升序 LogicalOccurrenceKey 承担，不是编译器临时伪造的 CEL helper。当前 fixtures 没有显式 user sort，因此真实 QuerySpec/2 的 sort 只有 `row.rank`，随后直接使用该强制 K tie；`source_key` 继续只是投影到 search row 的数据，不替代 K。UI 顺序与 locale collation 不提供隐藏排序。

## 5. Shortcut lexer 与递归 grammar

只有用户显式选择 shortcut mode 后才启动快捷解析。输入是一行 UTF-8；NUL 与换行非法。

~~~text
Shortcut mode v1

shortcut-input := ws? or-expr ws?
or-expr        := and-expr (ws1 OR ws1 and-expr)*
and-expr       := unary-expr ((ws1 AND ws1 | adjacency) unary-expr)*
adjacency      := ws1
unary-expr     := NOT ws1 unary-expr | primary
primary        := field-condition | literal-term | "(" ws? or-expr ws? ")"
field-condition := builtin-field ":" value
                 | field-ref ":" value
builtin-field  := @title | @subtitle | @filename | @path | @body | @resource-name
field-ref      := @field "(" field-id ("," member-path)? ")"
value          := quoted-value | bare-token
literal-term   := quoted-value | bare-token
~~~

lexer 先于 grammar 执行：

- quote 外的 ASCII space/tab 分隔 token。
- parenthesis 只有在 quote 外且未 escape 时才是结构符号。
- colon 通常只是普通文本；只有紧跟 recognized field operator head 的那一个 colon 才具有语法意义。
- bare token 是非空 Unicode scalar 序列，但不能含未 escape 的 space/tab/quote/parenthesis。backslash 后面若是 backslash、quote、parenthesis、@、colon、space 或 tab，就把这两个字符解码成后一个 escaped scalar；若后面是其它 scalar，则 backslash 本身就是 literal。bare token 的最后一个 scalar 若是 backslash，它**始终是一个完整的字面反斜线**，绝不属于 incomplete escape。
- quoted-value 由 double quote 包围。内部只把 backslash+double quote 与 backslash+backslash 当 escape；其它 scalar，包括 whitespace、colon、@、CJK、RTL，都是 literal。已经进入 quote 后遇到 EOF 时，draft 为 incomplete；若 open quote 内最后一个 scalar 是 backslash，则唯一分类为该位置的 `incomplete_quoted_escape`，否则是 `incomplete_quote`。
- 未 escape 的 uppercase ASCII AND、OR、NOT 只有作为完整 lexer token 时才是 keyword。lowercase 形式是 literal。若要搜索大写关键字本身，使用 quote。
- primary 开头若出现未 escape 的 @ 并匹配 recognized operator head，就解析为 source term。未知 @name: 返回 unknown_shortcut_field，不能 fallback 成 literal。escape leading @ 后，整段按 literal 处理。
- field-id 使用准确 D4 FieldId grammar。member-path 由 1..8 个 dot-separated lowerCamel ASCII ObjectMemberSpec name 组成，每段 1..64 bytes，并必须使用 Query 同一 current Registry/Field TypeSpec 验证。label 或 localized name 不能替代。

优先级从低到高是 OR、AND/adjacency、递归 NOT、primary/parentheses。NOT NOT A 合法，并保留两个显式 not node；A AND B AND C 与 adjacency A B C 都是合法链。

unmatched parenthesis、missing operand、missing field value、未完成 @field(...)、`incomplete_quote`、`incomplete_quoted_escape` 或其它未完成 structured token 都是 draft-incomplete，不执行 Query。bare token 末尾的 backslash 不在此列，它就是 literal scalar。用户显式提交 incomplete state 时返回带准确 source span 的 invalid_request。语法已经完整但 field/operator/value 未知时，返回对应 parse/compile error。任何 error path 都不会执行另一条 literal Query。

## 6. 唯一 condition AST 与确定性 Query 编译

shortcut input 与 visual control 都生成同一份 ephemeral compiler AST：

~~~text
SearchConditionAst/1 :=
    {kind:"literal",value:text}
  | {kind:"source_term",source:<closed source selector>,value:text}
  | {kind:"not",child:SearchConditionAst/1}
  | {kind:"and",children:[SearchConditionAst/1...]}
  | {kind:"or",children:[SearchConditionAst/1...]}
~~~

这个 AST 不持久化，也不是第二查询语言权威。and/or 只把相邻同类 operator flatten，并保留 source order；NOT 不做代数抵消。visual editor 使用同样 node kind 与 source selector，因此 surface 切换不能丢条件。

编译顺序固定：

1. 先确定 preset、显式 scope 和已启用的数据源；
2. 用 QuerySpec/2 与当前 Registry 验证全部 source 和 Field；
3. 按 §3 计算每个 AST 节点适用的 subject domain；
4. 建立 Node 分支，以及启用时的 Resource scan/read/filter/project 分支；
5. 每个有效分支都投影到同一 search-row schema，其中包含 Optional<NodeRef>、Optional<ResourceRef>、Optional<text> display、rank 和稳定 source key；
6. 存在多个分支时使用 QuerySpec/2 union_all；
7. 最后执行明确且确定的 sort 与 project。

可空 title/subtitle 只有在 some 分支才参与匹配，none 永远不会被强制转换成空文本。无标题 Document 通过正文命中时可以得到 display=none；界面可显示非作者占位文字，但不能把占位文字写回 Query 数据。

快捷/可视化子集无法表达的完整 Query condition，必须作为 opaque advanced Query condition 保留在可视化编辑器中，绝不能丢失；只有完整 Query editor 能修改它。保存时始终保存 QuerySpec/2，不保存 SearchConditionAst/1。

D7-SEARCH-FIXTURES.json 是 parser、visual 与 compiler 等价的 machine oracle。每个正例都实际保存 shortcut AST、明确且与具体 UI widget 无关的 visual-condition input、visual AST、expected canonical AST、明确标为非 wire 的 `compilerReviewDescription`、可按 QuerySpec/2 严格解码的真实 `expectedQuerySpec`，以及直接序列化既有 Query/View/Action §5 算法的完整 `expectedCanonicalGraph`，而不是无法解释的 digest。`expectedQuerySpec` 顶层准确只有 `format/version/parameters/relations/scalars/result`，并使用真实复数 scan domain/selector、read 的 `input/from/bindings`、CEL filter predicate、`{name,expr}` derive/project field、`{expr,direction,none}` sort key、TypedLiteral/Optional parameter、result shape，以及需要时的 QuerySpec/2 `union_all`。review description 只记录便于人工检查的 preset/scope/source compiler input，不能被当成 Query 解码或执行。

`expectedCanonicalGraph` 从 result relation 开始，删除作者 relation ID，按既有 dependency order 分配 `canonicalOrdinal`，记录 non-reference parameter、canonical `weftext.cel/1` AST、reference ordinal 与 declared field order，按参数名 bytes 排序 ParameterSpec，并原样保留 `union_all.inputs` 顺序。其中 CEL-AST object 只是在 fixture 中对既有 profile 做结构化证据呈现，不是新的 CEL 语法、wire format 或 graph protocol。Node/Resource 混合 OR fixture 会先按合法 domain 专门化 SearchConditionAst，再编译为各自合法 CEL predicate，投影相同 public schema，最后使用真实 `union_all` 与最终 sort/project。

正例继续要求 shortcutAst、visualAst 与 canonicalAst 按规定 canonical AST serialization 逐字节相等；`expectedQuerySpec` 与 `expectedCanonicalGraph` 是两种 surface 共用的预期编译结果。negative、incomplete 与 browse/no-query fixture 的两个对象都明确为 null，且不会执行替代 Query。这些仍只是设计 oracle：真实产品 QuerySpec/2 decoder、CEL parser/compiler 与 Search compiler 全部保持 UNRUN。

## 7. 权限、索引状态、命中数、排名与摘要

授权和最低披露检查必须先于任何敏感 source、Field、contribution、附件、Annotation 或索引私有数据读取。隐藏对象不得通过命中数量、排名缺口、摘要、补全提示或不可用原因泄露。

Derived Index 仍只用于候选加速。索引处于 building、partial、stale 或 unavailable，必须与“已经证明完整且结果为零”的 Query 区分。完整搜索需要当前获权的 query_scan，以及所有真实的正负 source、Registry、contribution、提取结果和授权依赖。部分探索不能签发完整 ResultHandle/ActionEvidence，也不能把当前 page 冒充完整结果。

已选择的 source、Field、SearchContribution、provider 或提取数据缺失时，在普通披露门之后返回既有 unavailable/reset 结果；不能退化为空 contribution。预算耗尽会让完整结果失败，不能返回前 N 条后再宣称完整。

snippet/highlight 只能从已经获权的语义文本派生；其中的 scalar offset 只是结果显示偏移，不是 Locator。打开命中项时必须重新执行 fresh current resolution/read；结果或来源已经 stale 时必须 reset。

## 8. 保存、重开、复制与导入

新的当前保存搜索使用 QuerySpec/2。saved definition 记录明确 scope、parameters、稳定 FieldId、SearchContribution 的 contributionId/version 依赖和作者排序。shortcut text、parser cursor、source popover、recent history 与 device direction 都只是交互状态。

重开时先按记录的 QuerySpec version 分派，再做当前资格检查。QuerySpec/1 保持准确旧语法，永远不按 /2 重新解释。Field 删除或改型、contribution 移除或换版、权限变化、FileBinding 变化，都按各自真实 Registry/currentness 规则触发 reset。

Definition Transfer 按对应版本的 Query schema 映射 typed Ref 与 DefinitionAddress slot；saved-definition copy/fork/import 走这个 D7/D3 作者路径，并保留 embedded QuerySpec version/bytes。D9 的 `query_json` 只负责把完整 terminal result 静态导出为 `weftext.query-result-export/1`，绝不包含 QuerySpec author bytes，也不能创建或迁移 saved Query。filename/path 字符串永远不会变成 DefinitionAddress 或 identity。

## 9. 跨 surface 与跨 device 等价

只要 QuerySpec/2 bytes、parameters、scope 与当前 dependency cut 相同，File List、Quick Open、Global Search 和 saved Query 就必须得到相同的成员集合与排序语义。不同 surface 可以提供不同 preset control，但任何可用 control 都必须映射到上述同一 source/AST 规则。

键盘、指针、触摸与辅助技术操作调用同一个 condition model。focus 绑定逻辑条件或结果身份，而不是 DOM position。IME 预编辑不执行 Query。CJK/RTL 输入、双向文本隔离、读屏 label、移动端面板和硬件键盘仍属于 D8 实现义务；本 D7 设计不声称已经实测。

## 10. SEARCH-01–08 验收

| ID | 当前设计义务 |
| --- | --- |
| SEARCH-01 | File List、Quick Open、Global Search 使用 §3 的 preset 范围与空输入行为；递归、Resources、Annotation、Trash、body、attachment/OCR 都必须显式选择。 |
| SEARCH-02 | title、subtitle、Node 文件名、Node 路径、Resource 文件名与 hierarchy 分域；QuerySpec/2 提供真实 producer；titleless 不补 title；rename/move 通过真实 dependency 使旧结果 reset，但不按文字改变 identity。 |
| SEARCH-03 | exact 与 nfc-for-compare 按 §4 分开；比较区分大小写，基于 Unicode scalar 且结果确定；不声称 fuzzy、tokenizer、Pinyin 或 stemming。 |
| SEARCH-04 | shortcut mode 必须显式开启，完整 lexer/grammar 见 §5；plain mode 中普通 title:、URL、盘符冒号和 prose 都按 literal 处理；shortcut error 或 incomplete draft 不执行 fallback Query。 |
| SEARCH-05 | visual 与 shortcut 生成同一 SearchConditionAst/1 和 §6 CanonicalGraph；advanced condition 必须保留；D8 的平台交互继续等待后续 evidence。 |
| SEARCH-06 | 权限先于敏感读取；隐藏 count/rank/snippet 不泄露；索引状态与完整零结果分开；source/provider 缺失和预算耗尽都有明确失败；普通 open/edit/save 独立。 |
| SEARCH-07 | save/reopen/copy/import 持久化带版本的 canonical Query，不保存 UI state；/1 与 /2 分开 dispatch；Field、contribution、FileBinding 或权限变化都会重新资格化或 reset；旧 result evidence 不续权。 |
| SEARCH-08 | 打开 hit 时执行 fresh current authorized resolve；snippet offset 不是 Locator；相同 QuerySpec/2 语义跨 surface/device 保持一致，只允许明确标示的 unavailable capability 差异。 |

以上都是设计义务。Product Search、GUI/IME/AT、provider、performance 与跨设备执行继续 UNRUN。
