---
source_language: zh-CN
translation_status: source
---

[English](D7-SEARCH.md)
# A2 D7 搜索语义与 SEARCH-01–08

状态：这是当前 D7 作者候选中的规范搜索设计。本修订处理 A2-D7-2F89-P1-02 与 A2-D7-2F89-P2-01，但仍需独立复核。D8 的具体交互实现继续属于后续完整模块批次。

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

默认 rank 中，title exact 为 0；subtitle、Node/Resource file name 以及 contribution role=name 的 exact 为 1；alias exact 或 Node relative path exact 为 2；非 body 的 substring 为 3；body/content-only 为 4。只有已经获权的值可以进入 rank。rank 相同后先使用明确的用户 sort；再以 canonical subject key 与 contribution/source identity 做稳定 tie。UI 顺序与 locale collation 不提供隐藏排序。

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
- bare token 是非空 Unicode scalar 序列，但不能含未 escape 的 space/tab/quote/parenthesis。backslash 只有在后面是 backslash、quote、parenthesis、@、colon、space 或 tab 时才作为 escape；这些 pair 解码成被 escape 的 scalar。其它位置的 backslash 是普通字面字符，因此 Windows path 不需要把每个 backslash 都改写。若输入停在一个已经开始但尚未完成的 reserved escape，则是 incomplete draft。
- quoted-value 由 double quote 包围。内部只把 backslash+double quote 与 backslash+backslash 当 escape；其它 scalar，包括 whitespace、colon、@、CJK、RTL，都是 literal。没有 closing quote 时为 incomplete。
- 未 escape 的 uppercase ASCII AND、OR、NOT 只有作为完整 lexer token 时才是 keyword。lowercase 形式是 literal。若要搜索大写关键字本身，使用 quote。
- primary 开头若出现未 escape 的 @ 并匹配 recognized operator head，就解析为 source term。未知 @name: 返回 unknown_shortcut_field，不能 fallback 成 literal。escape leading @ 后，整段按 literal 处理。
- field-id 使用准确 D4 FieldId grammar。member-path 由 1..8 个 dot-separated lowerCamel ASCII ObjectMemberSpec name 组成，每段 1..64 bytes，并必须使用 Query 同一 current Registry/Field TypeSpec 验证。label 或 localized name 不能替代。

优先级从低到高是 OR、AND/adjacency、递归 NOT、primary/parentheses。NOT NOT A 合法，并保留两个显式 not node；A AND B AND C 与 adjacency A B C 都是合法链。

unmatched quote/parenthesis、missing operand、missing field value、未完成 @field(...)、unfinished reserved escape 或其它未完成 token 都是 draft-incomplete，不执行 Query。用户显式提交时，同一状态返回带准确 source span 的 invalid_request。语法已经完整但 field/operator/value 未知时，返回对应 parse/compile error。任何 error path 都不会执行另一条 literal Query。

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

compiler 的顺序固定：

1. resolve preset、显式 scope 与 enabled sources；
2. 按 QuerySpec/2 与 current Registry 验证全部 source/Field；
3. 按 §3 计算每个 AST node 的 applicable subject domains；
4. 建立 Node branch，以及启用时的 Resource scan/read/filter/project branch；
5. 每个有效 branch 投影到同一 search-row schema：Optional<NodeRef>、Optional<ResourceRef>、display text、rank、stable source key；
6. 多 branch 时使用 QuerySpec/2 union_all；
7. 应用明确的 deterministic sort 与 final project。

快捷/可视化子集无法表达的完整 Query condition，必须作为 opaque advanced Query condition 留在 visual editor，绝不能丢失；只能通过 full Query editor 修改。保存时始终保存 QuerySpec/2，不保存 SearchConditionAst/1。

D7-SEARCH-FIXTURES.json 是 parser/visual equivalence 的 machine oracle。每个 positive fixture 的 shortcut AST 与 visual AST 在 canonical AST serialization 后必须逐字节相等，并编译成逐字节相等的 CanonicalGraph 描述。negative/incomplete fixture 不产生任何 Query。

## 7. 权限、索引状态、命中数、排名与摘要

authorization 与 minimum disclosure 必须先于任何 sensitive source、Field、contribution、attachment、Annotation 或 index-private read。hidden object 不得通过 hit count、rank gap、snippet、completion 或 unavailable reason 泄露。

Derived Index 仍只做 candidate accelerator。building、partial、stale、unavailable 与 proved complete zero-result Query 分开。complete search 需要 current authorized query_scan，以及全部真实 positive/negative source、Registry、contribution、extraction 与 authorization dependencies。partial exploration 不能签发 complete ResultHandle/ActionEvidence，也不能把 current page 当作完整结果。

selected source/Field/SearchContribution/provider/extraction data 缺失时，在普通 disclosure gate 后返回既有 unavailable/reset result；不能 fallback 成 empty contribution。budget exhaustion 使完整结果失败，不能返回前 N 条再宣称 complete。

snippet/highlight 只能从已获权的 semantic text 派生。它们的 scalar offset 只是 result-display offset，不是 Locator。打开 hit 时重新执行 fresh current resolution/read；result/source stale 时必须 reset。

## 8. 保存、重开、复制与导入

新的 current saved search 使用 QuerySpec/2。saved definition 记录 explicit scope、parameters、stable FieldId、SearchContribution contributionId/version dependency 与作者 ordering。shortcut text、parser cursor、source popover、recent history、device direction 都只是 interaction state。

reopen 先按 recorded QuerySpec version 分派，再做 current qualification。QuerySpec/1 保持准确旧语法，永远不按 /2 重新解释。Field deletion/type change、contribution removal/version change、permission change、FileBinding change 都走各自真实 Registry/currentness reset。

Definition Transfer 按版本对应 Query schema 映射 typed Ref/DefinitionAddress slot。D9 exact query_json copy/export/import 保留 recorded version 与 bytes。filename/path string 永远不会变成 DefinitionAddress 或 identity。

## 9. 跨 surface 与跨 device 等价

同一 QuerySpec/2 bytes、parameters、scope 与 current dependency cut，在 File List、Quick Open、Global Search 和 saved Query 上必须得到相同 Query membership/order semantics。不同 surface 可以提供不同 preset control，但只要某个 control 可用，就必须映射到上述同一 source/AST 规则。

keyboard、pointer、touch 与 assistive-technology operation 调用同一个 condition model。focus 绑定 logical condition/result identity，而不是 DOM position。IME preedit 不执行 Query。CJK/RTL input、bidi isolation、screen-reader label、mobile sheet、hardware keyboard 仍是 D8 implementation obligation；本 D7 design 不声称已经实测。

## 10. SEARCH-01–08 验收

| ID | 当前设计义务 |
| --- | --- |
| SEARCH-01 | File List、Quick Open、Global Search 使用 §3 的 preset scope 与 empty-input behavior；recursive、Resources、Annotation、Trash、body、attachment/OCR 都必须显式。 |
| SEARCH-02 | title/subtitle/Node filename/Node path/Resource filename/hierarchy 分域；QuerySpec/2 提供真实 producer；titleless 不补 title；rename/move 通过真实 dependency reset，但不按 text 改 identity。 |
| SEARCH-03 | exact 与 nfc-for-compare 按 §4 分开；比较区分大小写、基于 Unicode scalar 且 deterministic；不声称 fuzzy/tokenizer/Pinyin/stemming。 |
| SEARCH-04 | shortcut mode 必须显式，完整 lexer/grammar 见 §5；plain mode 中普通 title:、URL、drive colon 与 prose 都是 literal；shortcut error/incomplete draft 不执行 fallback Query。 |
| SEARCH-05 | visual 与 shortcut 生成同一 SearchConditionAst/1 和 §6 CanonicalGraph；advanced condition 保留不丢；D8 platform interaction 继续等待后续 evidence。 |
| SEARCH-06 | permission 先于 sensitive read；hidden count/rank/snippet 不泄露；index state 与 complete zero 分开；missing source/provider 与 budget 有明确失败；ordinary open/edit/save 独立。 |
| SEARCH-07 | save/reopen/copy/import 持久化 versioned canonical Query，不保存 UI state；/1 与 /2 分开 dispatch；Field/contribution/FileBinding/permission 变化重新资格化或 reset；old result evidence 不续权。 |
| SEARCH-08 | hit 打开时 fresh current authorized resolve；snippet offset 不是 Locator；相同 QuerySpec/2 semantics 跨 surface/device 保持一致，只允许显式 unavailable capability 差异。 |

以上都是设计义务。Product Search、GUI/IME/AT、provider、performance 与跨设备执行继续 UNRUN。
