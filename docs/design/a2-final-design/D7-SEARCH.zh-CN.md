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

正例继续要求 shortcutAst、visualAst 与 canonicalAst 按规定 canonical AST serialization 逐字节相等；`expectedQuerySpec` 与 `expectedCanonicalGraph` 是两种 surface 共用的预期编译结果。为了在不发明 Ref literal、也不引入可覆盖 Optional<Ref> 输入的前提下构造另一 domain 的 absent Ref，fixture 使用类型为 `list<NodeRef,0>` 或 `list<ResourceRef,0>` 的 version-2 parameter；这种类型唯一合法值就是空列表，默认值也是 `[]`，因此 `at(param.empty_*_refs, 0)` 会得到所需的强类型 Optional none，而 invocation argument 不可能把它替换成 present Ref。negative、incomplete 与 browse/no-query fixture 的两个对象都明确为 null，且不会执行替代 Query。这些仍只是设计 oracle：真实产品 QuerySpec/2 decoder、CEL parser/compiler 与 Search compiler 全部保持 UNRUN。

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

## 11. QuerySpec/3：生命周期搜索与保存贡献绑定

本节是新准备和新作者定义的唯一 current owner，具名替代上文 QuerySpec/2 保存/编译说明及 retained Query Algebra §§1–2 的 live-only scan/read，且只在 QuerySpec/3 invocation 中生效。原生身份/业务值仍遵守 AUTHORING/集成；typed Field 不退回 Entry-only。QuerySpec/1、/2 的真实作者字节、错误与记录恢复保持原解码域，不加默认成员，不读时升级。旧版本仍 live-only，不能用旧编译结果宣称完成 Trash 搜索。本修订是设计候选，所有运行测试 UNRUN。

### 11.1 唯一闭合作者形状

```text
QuerySpec/3 = {
  format:"weftext.query", version:3,
  parameters:<原 ParameterSpec 数组>,
  relations:<完整 QuerySpec/2 算子 DAG>,
  scalars:<原 ScalarSpec 数组>, result:<原 result union>,
  lifecycleScope:"live"|"trash"|"live_and_trash",
  searchBindings:SearchBindingSet/1
}
SearchBindingSet/1 = {
  version:1, mode:"none"|"explicit"|"all",
  selections:[{
    contribution:<完整 SearchContribution>,
    namespaceOwner:{ownerKind:"core"|"first_party"|"publisher"|"workspace_user",ownerId:text}
  }...]
}
```

八个 Query 顶层成员全部必填，对象闭合。lifecycleScope 不可由参数、调用方、UI 或环境覆盖；一般新 Query 默认为作者显式保存的 live，searchBindings 为 `{version:1,mode:"none",selections:[]}`。none 恰要求空数组；explicit 要求1..256项；all 允许0..256项且代表作者保存时明确选择的完整可披露激活 SearchContribution 集，不代表将来任意新贡献自动加入。超限整体 budget_exceeded，不截断。每项 contribution 是原五成员闭合对象，正 Counter version、静态 textPath/role/FieldId与原规则不变。selections 按 contributionId 原UTF-8排序唯一，同ID不同version也拒绝重复；namespaceOwner采用D4精确所有者类别/ownerId，必须在同cut证明为该贡献ID命名空间的所有者，不能用Field的命名空间代替。

绑定不创建另一份 FieldDefinition，也不是新的 SearchContribution 实例/注册表。保存的完整 descriptor 是语义约束；当前执行从 D10 重新取得其接纳真实性、版本及权限，不把保存对象当接纳证明。显式绑定须静态证明 contribution.fieldId/textPath 确实由本Query可达 read与匹配/rank表达式消费；Core的 CEL AST/类型路径证明失败则 invalid_request，不能用unused binding给普通 Query加 ambient package gate。无绑定的通用Field Query不依赖SearchContribution。编译器为明确选择的完整贡献产生对应绑定；all先冻结当时完整集合再编译同一DAG。是否搜索Trash只修改lifecycleScope；原AST/快捷词法不增加新executor。

CanonicalGraph3 精确为 `{version:3,lifecycleScope,searchBindings,graph:CanonicalGraph}`，searchBindings使用上述规范数组及原D3-CJ/3值编码。Query语义identity、SemanticStateKey及完整definition闭包必须包含此完整对象，不只散列旧DAG；原逻辑operator ordinal/K和错误排序不改。历史CanonicalGraph1/2不重编码。

### 11.2 生命周期范围与真实读取资格

live只允许live；trash只允许trashed；live_and_trash允许两者，永不含tombstoned、never-known、冲突/placeholder或未完成状态。scope适用于本invocation的scan和read，不传播为QueryRef callee的默认值，也不影响definition lookup：DefinitionAddress仍要求其原获权live source/版本，不能为了读callee的scope先执行隐藏定义。输入/参数Ref的read同样检查本scope，不以“scan外来的Ref”绕过它。

workspace在同一Workspace当前规范inventory与live/Trash forest中选择所需状态；entities按完整Ref去重，在最低state披露之前不查存在，无权/missing为零成员。subtree仍沿指定root的**当前**规范forest，不按原parent/restore-membership/路径重建历史树。root不在scope或不可披露统一not_visible；先structure_state及root披露，再读其准确live或Trash subtree。两态范围不将live root曾删除的后代重新接回；同时搜两forest可使用workspace或显式entities。headings以选中Node为source owner，仍须完整D2投影；Resource/Annotation按自身真实lifecycle选择，owner/关系披露独立验证，不因owner相等自动加入，独立Trash成员不变成restore成员。

本scope不授任何权限。先trusted audience/Workspace→原entity_state/适用owner/state披露→结构或所选source的原准确source_read/field_read/resource_read/annotation_read与deny/最小闭包→可用性→读取。Trash只增加D7在已获权trashed对象上的只读source-adapter资格，D6同一个current SourceObservation/1/FileObjectBinding/源pin producer在该明确只读目的下可观察保留源；不创建managed版本，不选旧历史pin伪装current，不读完再授权。合法trashed Node正文非空needle正例复用同一D2 body_text；原生属性Field仍经当前FieldSource/Registry adapter。typed Ref/provenance按原value读规则，不隐式解析引用目标。权限不足not_visible；已获权但源/pin/完整性不可得source_unavailable或原owner unavailable，不变空。

本增量不扩宽traverse/related/graph的live endpoint/关系门；它们在本scope中仍按原live资格，trashed起点无法取得原live关系资格则not_visible，不恢复已suspended边。普通已保存live Query、Calendar与NodeCollection含义不变。任何ActionEvidence、collection_create/remove、bulk/all_result或postQuery写入consumer必须先证明完整调用闭包全为live scope，真实/静态不能证明则原action_not_applicable，哪怕最后filter只剩live行；Trash只读结果绝不授写。显式restore/purge只接受原D3新prepare/完整确认及自己的owner proof，不把Query result当恢复闭包或写权。saved/unknown原动作恢复仍用其真实版本，不重新评估本scope。

### 11.3 同一query_scan key的闭合覆盖证据

保留D6-CONTROL §3.4的query_scan key成员、十五kind/rank、DependencyProof/3和PinRef/2，不新造DependencyKey。其domain/selector仍恰为原四domain及原三selector。D6-CONTROL的该key定义并未在成员中编码live；D7以前只签live枚举。本节用具名受保护证据区分新覆盖，不靠相同key或旧stamp猜测。

```text
D7QueryScanCoverage/1 = {
  kind:"d7_query_scan_coverage",version:1,
  key:<准确 DependencyKey/3.query_scan>,
  stamp:{epoch:Token,revision:Counter}, baseFrontier:Frontier/2,
  lifecycleScope:"trash"|"live_and_trash",
  authorization:{key:<原 authorization DependencyKey>,stamp:{epoch:Token,revision:Counter}},
  inventoryEvidencePins:[PinRef/2...],
  members:[{ref:EntityRef,state:"live"|"trashed",lifecyclePin:PinRef/2}...]
}
```

仅受信Core/D3/D6 producer可生成此闭合对象，以D3-CJ/3保存于原PinRef/2.payloadKind=portable_metadata、retentionClass=query_preview的受保护pin record；pin record内部kind分派到本version1 decoder，不新增公共pin member/tag/store。内部portable_metadata decoder只在该Query3 coverage consumer接受准确根kind/version的上述完整闭合对象，不扩宽任何旧metadata类型，不接受任意JSON。Core在原授权后的Query preparation中生成，pin不授公开元数据读取，不能经Resource ByteHandle打开；生命周期/成员详情不越过原D7诊断披露。每个member计maxRows/work，每个inventory/lifecycle pin计maxDependencies和实际bytes计maxTemporaryBytes/maxResultBytes；先checked预算再分配，超限整体budget_exceeded。pin保留不超原result的有限TTL/预算，query_preview释放/restart沿原受保护记录，保存author bytes不保存pin。key/ref域及Workspace一致，members按D3RefKey排序唯一，headings的ref是Node source owner；每项state须属于scope，lifecyclePin证明该同cut真实D3 birth/canonical/lifecycle及适用owner。members只列完整获权population，隐藏成员只在受保护真实inventory/authorization proof中核验，不交客户端。inventoryEvidencePins非空且排序唯一，逐pin为该key准确potential范围的D3当前完整identity/forest/owner目录与连续证据；缺负范围、gap或仅命中source pins不得生成Coverage。headings的全部实际source/D2依赖另外保留在原Proof与sourceInputs中，不因members列表而免除。

对任何non-live scope的scan，consumer须在DependencyProof/3.entries对应**同key**的evidencePins找到唯一覆盖该key+stamp+scope+baseFrontier+当前authorization项的Coverage/1；相同key多个invocation/scope可共享一项Proof entry并含各scope各一枚准确Coverage pin。严格解码、摘要/长度、真实受保护pin身份、对应authorization key/stamp、所有inventory/lifecycle证据和当前连续性必须全部成立，再独立按同scope枚举比较members逐Ref/state相等，不能只信“complete=true”、count/digest或caller列表。缺失/旧live-only证据/错误scope失败原source_unavailable，不返回全集/ActionEvidence。静态entities=[]仍零QueryScan、零Coverage，不由它断言Workspace为空。

不因启用Coverage decoder或软件升级创建新的continuity epoch，不改生产H或重签旧record。新producer在原D6真实同cut证明下保留合法key/stamp连续性；对Trash所需inventory/lifecycle/owner/forest负范围，必须把实际原D3 lifecycle、placement_range（trash_roots/trash_children/subtree/owner目录等适用项）及authorization的完整key/stamp/evidence加入同一Proof3，并要求inventoryEvidencePins是这些准确entry的真实证据pin并完整覆盖潜在范围。Coverage没有自身独立stamp，只重复父entry的真实值；consumer逐key重验这些原dependency，不能仅以query_scan stamp未变判current。Trash恢复/purge/两态转移/独立成员变化使相关真实依赖失效；无法证明连续性按原D6 gate，不让client制造新epoch绕过gap。QuerySpec3/live和真实QuerySpec1/2继续其原live证明规则，不强加Coverage或Trash目录门。缓存键包含CanonicalGraph3及scope；即使字节相同，也不得拿旧live Proof替代non-live Coverage。

### 11.4 保存贡献的冷重开与当前D10查找

保存Query只保存上述纯数据绑定，不保存activationGeneration、CapabilityCatalog digest、Registry pin、Token或Run身份。重开/inline/QueryRef每次先当前请求和定义披露，再按embedded query.version严格解码和静态图/类型，之后在同AuthorizedCut通过D10-CONTROL §4**现任完整Catalog**定位每个D7 contributionId，验证激活且version及完整五成员descriptor逐字相等、D4 namespace owner tuple与保存namespaceOwner相等、原immutable资产digest/trust及准确Registry文本路径合格。D10 package-local ID/contractVersion不是D7 ID/version，不按字段/名称猜同义项。

explicit仅要求所选项；all还要求当前完整可披露SearchContribution集合与保存selections逐项相等，无新增/遗漏项。变更不自动重编译DAG或刷新author bytes：需要用户显式编辑再保存新definition revision。字段相同但C/v1停用、移除或升v2不能由C2代替。缺所选贡献/版本或语义改变走已披露后的definition_unavailable；权限不足先not_visible；坏已获权源走source_unavailable。不存在新error code或fallback。新语义错误在完整canonical闭包中按原阶段/ordinal次序；preexecution contribution错误按invocation路径及selections原ID顺序确定最小项，对外不输出隐藏ID/path/count。read-only定义不携保存者权。

runtime依赖绑定D10当前activationGeneration+capabilityCatalogDigest+RegistryBinding和完整所选descriptor/owner真实性证据，沿原ResultHandle闭包留存；Catalog/授权/Registry变化先使旧result reset，冷重开重新取得资格。仅无关connector/包变更不能让新执行的explicit保存定义definition_unavailable；可能保守reset旧runtime，但不改已保存语义。mode none不引入D10 Catalog gate，普通Field Query继续独立，关闭UI不等于停用贡献。

### 11.5 实际producer/consumer槽位与历史边界

| 实际owner / 槽 | 精确current规则 |
| --- | --- |
| D7-SCHEMAS / Query author decoder，retained Query/View/Action §4 | current QuerySpec3八成员；原QuerySpec1/2严格原闭合对象。feature由Core推导query.lifecycle-scope.v1与query.search-bindings.v1，仅确实使用non-live或非none时加入，不由作者requires声明。 |
| D7 d7_query_execute.call.query.inline.spec、SavedQueryDefinition/1.query、DynamicBlock/1.query.inline.spec、relation/scalar QueryRef解析出的query | 以嵌入query.version=1/2/3选择唯一完整decoder；外层Query wire2、SavedQueryDefinition1、DynamicBlock1的其余成员不变。记录recovery先按真实外层/嵌入版本，不把旧wrapper内query改成3。 |
| D7 Compiler/Search/D8保存搜索与builder | 新建输出QuerySpec3完整scope/绑定；basic UI无法表示仍保留advanced source；保存定义走原D2/D3/D6，不保存AST/快捷文本。 |
| D6 query_scan、Proof3.entries[].evidencePins、原ResultHandle依赖/缓存 | non-live只接受§11.3精确Coverage1 pin；source/lifecycle/结构/auth各真owner同cut证明，旧live-only proof不能替代；所有分页/订阅/导出先原audience/expiry/epoch/reset gate。 |
| D3/D6只读trashed source producer，D7 read | §11.2明确只读qualification消费原Observation/pin，不能扩宽source_write或restore/purge；public source save普通live路径不受影响。 |
| D10-CONTROL §4 Search descriptor lookup | 本节§11.4的精确D7 ID/version/descriptor/namespace owner当前查找及运行dependency；不改D10D7SearchDescriptor1或ContributionKind，保存bindings不是激活对象。 |
| DefinitionTransfer、Result Q、copy/fork/import | 先完整按embeddedQuery.version解码，遍历原typed Ref/DefinitionAddress slots，scope与纯descriptor保持。mode/ID/version/FieldId/path/role非Ref文本不猜替换；publisher/core/first_party所有者在目标Registry/Catalog精确重新验证。workspace_user.ownerId须等于目标WorkspaceId，跨库不同不能静默改绑定：本版拒绝definition_unavailable，用户另显式编辑新definition；同库可保持。无typed Ref仍有原slots=[] transfer，不省Q。 |
| D5 NodeCollection/Action、D9 Query结果导出、D10查询调用 | D5写入/Action全闭包只live；D9 query_json仍仅完整terminal result，不承载Query作者格式；D10只消费同Query和当前权限，无专用Search executor。 |

本节是QuerySpec/3 successor本体与跨owner协调，不改变旧Registry术语ID或conceptId；D7现有query-spec/search-contribution概念足够，new controlled types D7QueryScanCoverage/1与SearchBindingSet/1属于D7该概念的闭合内部承载，非新持久实体。原34 concepts及13 bindings历史计数不被本节冒充新全局通过。原SavedDefinition wrapper明确是embedded-version dispatcher，不把允许的新嵌入版本解释为旧recorded decoder忽略未知member。

### 11.6 可执行正例与拒绝/恢复oracle

以下准确QuerySpec3 JSON可在同Workspace获权Trash Node正文needle正例执行：

```json
{"format":"weftext.query","version":3,"parameters":[],"lifecycleScope":"trash","searchBindings":{"version":1,"mode":"none","selections":[]},"relations":[{"id":"n","op":"scan","domain":"nodes","selector":{"kind":"workspace"},"as":"node"},{"id":"b","op":"read","input":"n","from":"row.node","bindings":[{"name":"body","source":{"kind":"body_text"}}]},{"id":"f","op":"filter","input":"b","predicate":"contains(row.body, 'needle')"},{"id":"p","op":"project","input":"f","fields":[{"name":"node","expr":"row.node"}]}],"scalars":[],"result":{"kind":"rows","relation":"p"}}
```

显式保存C/v1正例在同形Query中设置 searchBindings 为 `{version:1,mode:"explicit",selections:[{contribution:{contributionId:"people/search-name",version:1,fieldId:"people/name",textPath:["text"],role:"name"},namespaceOwner:{ownerKind:"first_party",ownerId:"weftext.people"}}]}`，read源为people/name values，filter明确使用 `row.names.exists(v, contains(v.text, 'needle'))`，同cut已接纳people/name object.text schema与全部静态路径匹配。这里snippet只是说明替换成员，不是另一wire；编译器必须产完整八成员Query，再原保存/执行。

| Case | 正向与拒绝/恢复判据 |
| --- | --- |
| SR-01 | Trash T获权+保留源/格式完整+needle→scan/read命中；live Query不命中；非liveScope拒绝tombstone、placeholder，不用旧pin补值。 |
| SR-02 | 无entity/state披露或source_read→读取前not_visible；Field-only不偷读body；hidden T不增count/rank/snippet/coverage公开成员。 |
| SR-03 | 旧live-only proof具有同key但无Coverage、wrongScope、错误auth/key/stamp/cut、缺负inventory→无complete；准确覆盖空集成功，partial I不能当空。 |
| SR-04 | Trash→restore/purge/source replacement/auth变化→旧Result/coverage reset；重新授权/字节ABA只允许新invocation，旧page/action不复活。 |
| SR-05 | subtree选择真实forest；原parent不包含已移入Trash后代；entities[]零scan proof；独立Trash资源/批注不因owner恢复变live。 |
| SR-06 | 保存C/v1→清I/D→冷重开/QueryRef/DynamicBlock保持八成员与C绑定；同Field/path的C2不能代替；C停用/删除/v2/owner变而Field完好→definition_unavailable。 |
| SR-07 | 无关connector变更→旧runtime可reset但fresh explicit C/v1可执行；none的普通Field Query完全不需Catalog；all添加新贡献→不得自动扩大作者语义。 |
| SR-08 | 同库copy/fork与跨库合法first_party定义transfer保留scope/绑定及typed Ref映射；目标缺C或user owner不相容→整体明确拒绝，不静默改名/丢条件；无Ref仍Q/slots=[]。 |
| SR-09 | QueryRef各callee拥有自己的scope/绑定；caller live后续read不读取callee Trash Ref；whole closure非live的all_result/collection write拒绝，restore/purge另原prepare，不从scan授权。 |
| SR-10 | QuerySpec1/2注入新成员、3缺成员/未知enum/duplicate binding/unknown member皆拒绝；旧saved/planned/unknown原bytes不升级，query_json不变Query carrier。 |

这些是设计oracle，产品/GUI/Catalog/缓存/冷重开/Transfer/Trash/真实权限测试UNRUN；不因本文存在自行关闭任何global finding。
