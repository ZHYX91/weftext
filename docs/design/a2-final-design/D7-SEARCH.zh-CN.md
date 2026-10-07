---
source_language: zh-CN
translation_status: source
---

[English](D7-SEARCH.md)
# A2 D7 搜索语义与 SEARCH-01–08

状态：current D7 作者候选中的规范搜索设计；D8 interaction implementation 仍属于后续完整模块批次。

## 1. 裁决与替代方案

本批比较三种完整 interaction 方案。

| 方案 | 优点 | 成本 / 风险 | 裁决 |
| --- | --- | --- | --- |
| 仅 plain text + visual filter | 普通用户零语法学习、无词法歧义 | 熟练用户键盘效率较低；复杂条件不便复制 | 保留为默认 ordinary experience |
| ordinary box 自动解析常见 field-colon 文本 | 短、熟悉 | 会混淆正文 title:、URL、drive letter、CJK punctuation 与未完成输入；错误回退可能执行另一条 Query | 拒绝 |
| plain text 默认 + visual filter + 显式 shortcut-condition mode | 无需学语法，同时提供确定的 expert path，并 round-trip 到同一 Query | 需要一个小型 parser 与清楚的 mode indicator | 选择 |

选择后的模型只有一个 execution authority：每个成功 search state 都编译到既有 D7 Query algebra，并走相同 D6 authorization、complete-range、budget、result、paging 与 reset 规范。不存在第二 full-text executor，也不存在不同于 saved Query 的 durable search object。

## 2. Search object vocabulary

current D3/D2 没有一个可把所有文件 label 都等同进去的 generic authored Node name，因此搜索词汇明确分开：

- document title：current D2 native document title，可 null；
- document subtitle：current D2 native subtitle，可 null；
- file name：在有权读取 metadata 时使用 current D6 physical FileBinding basename；
- path：在有权 disclosure path 时使用 current D6 physical relative path；
- resource name：Resource FileBinding basename，与 Document title 分域；
- logical hierarchy：用来枚举 Node 的 D3 parent/order scope，不是 stable path identity；
- body：完整 current D2 evaluation barrier 下的 D2 semantic body_text；
- contributed text：explicit selected current SearchContribution 的 Field/textPath。

titleless Document 继续 titleless。filename、path、list placeholder 或 first paragraph 可以作为 UI fallback 显示，但绝不能成为 title data。duplicate title、不同允许位置中的 duplicate filename、相等 contributed string 都不会合并 identity。

rename/move 在 D3 保持 identity 的情况下改变 current filename/path match，而 NodeRef 保持不变。因此 old result 按普通 currentness rule stale；saved Query 重新对 current authorized FileBinding 执行，而不是把旧 display path 冻结成 identity。

## 3. 三种产品搜索 preset；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

File-list filter 只在已经选定的 logical list scope 上运行。默认 scope 是 current D3 parent、nonrecursive、只含 live Node；不读 body text、Annotation body、Trash、attachment extraction 或 OCR。UI 可明确打开 recursive subtree 或 Resources；这些开关必须编译成 explicit Query scope/source choice，并分别取得 disclosure。这个 surface 的 empty input 表示 browse selected list scope，不是 empty full-Workspace search。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

Quick open 默认在 selected Workspace/root scope 搜 authorized live managed Node，读取 title/subtitle 与独立获权的 filename metadata。body、Annotation body、attachment extracted text、OCR 与 Trash 默认关闭。明确开启 Resources 才加入 authorized Resource 与 resource-name match。Quick open 仍是 Query preset，不是按文字做 identity lookup。

Global text search 默认在 selected Workspace/root 或 explicit subtree 搜 authorized live Document，title/subtitle/body 默认开启。Resource、attachment extraction/OCR、Annotation body 与 Trash 都是 explicit opt-in scope。attachment content 只有真实 current extraction/SearchContribution dependency 成立时可用。Annotation text 必须先过 annotation disclosure/read 再取得 body。missing provider、incomplete extraction、hidden source 或 unmaterialized placeholder 对 complete search 是 unavailable，不是 successful empty match。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 4. Match、Unicode、Boolean composition 与 sort

current generation 保留 D7 SearchContribution 的 matching boundary：deterministic NFC comparison + exact scalar substring，不自动 fuzzy、stemming、token-language inference、transliteration 或 Pinyin。comparison 不改 source bytes。case 使用 retained case-sensitive comparison；未来 case-fold profile 必须通过 explicit versioned semantic addition，不能仅由 UI toggle 引入。

CJK 直接按 Unicode scalar substring 工作，不需要 whitespace tokenizer。RTL 只影响 presentation。quoted phrase 是一条 exact scalar sequence。普通 plain search text 是一个 literal phrase，不会自动按词拆分。visual filter row 默认 AND；underlying Query expression 可表示时，visual builder 提供 explicit OR group 与 NOT。

shortcut mode 中相邻 primary term 是 AND，OR、NOT 与 parentheses 按下述 grammar。match rank 与 role order 继续以复制到本候选的 Query Algebra owner text 中 retained D7 SearchContribution rank 为 exact 权威；UI 不能加入 private fuzzy score。若 retained comparator 完全相等，Query 以 canonical subject key，再以 selected contribution identity 作为明确 stable tie，保证 paging deterministic。general Query sort 仍由作者定义，绝不跟 viewport order。

## 5. Shortcut-condition mode

shortcut parsing 只在 opt-in 后启用；单纯输入 colon 永远不会开启。visible mode indicator 是 interaction state；parse 成功后 semantic object 是 compiled Query condition。

```text
Shortcut mode v1

plain-token        := escaped-token | quoted-value
field-condition    := @title:value
                    | @subtitle:value
                    | @filename:value
                    | @path:value
                    | @body:value
                    | @resource-name:value
                    | @field(field-id[,member-path]):value
value              := quoted-value | escaped-token
quoted-value       := "..." with backslash escaping for quote and backslash
boolean-expression := primary
                    | NOT primary
                    | primary AND primary
                    | primary OR primary
primary            := plain-token | field-condition | ( boolean-expression )

precedence: parentheses > NOT > AND > OR
whitespace between adjacent primary terms is AND
a colon is syntax only inside a recognized @ operator
unknown @ operator or invalid field/member path is a parse error
incomplete input is a draft parse state and executes no Query
```；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

@ prefix 是刻意选择。ordinary title:、https://example.test、C:\\notes\\a.adoc、time 12:30、quoted prose 与 CJK full-width punctuation 都保持 literal；只有用户明确进入 shortcut mode 并使用 recognized @ operator 才产生语法。若想在 shortcut mode 搜以 operator spelling 开头的文字，可 quote 或 escape leading @。

unknown @ operator、unknown FieldId、unavailable Field definition、illegal member path、unmatched quote/parenthesis 或 invalid value 都在 exact draft span 报 error，UI 不执行 fallback literal Query。incomplete input 保留 editable draft，同样执行零 Query；这样错误不会 silent 变成更宽搜索。

第一代 built-in keyword set 是 title、subtitle、filename、path、body、resource-name 与 field。刻意不设 generic file 或 name，因为它们会把不同 domain 合并。field 必须给 stable D4 FieldId；若还有 member path，则必须用 Query 同一份 current Registry 验证。localized label 与 alias 不能替代 stable ID。

## 6. Visual filter 与 round-trip

plain input、visual filter 与 shortcut mode 都编辑同一个 condition model。可表示的 Query condition 可以 lossless round-trip 成 visual chip 与 shortcut text；presentation chip 重排不能改变 semantics。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

完整 Query 可能包含 convenience grammar 表示不了的 condition。该条件必须作为 opaque-but-editable advanced condition node 留在 visual model；切 shortcut mode 时要明确显示它不能转换成文本，绝不能丢掉。用户可打开 full Query editor 继续编辑；保存绝不能只 serialize 当前可见 chips。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

keyboard、pointer、touch 与 assistive-technology control 都调用同一 condition operation。focus 绑定 logical condition ID，而不是 DOM index。search refresh、sort 或 epoch reset 使旧 result-row focus/selection/evidence stale，并把 focus 放回 stable container，不能把 reused visual row 交给另一对象。IME preedit 执行零 Query；只有 finalized input 可以更新 search draft。

CJK IME、RTL text、bidi isolation、screen-reader label、mobile sheet 与 hardware keyboard path 是 D8 interaction obligation。本 D7 批冻结其 semantic target 与 acceptance requirement，但不声称 platform execution。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 7. Permission、index state、count、ranking 与 snippet；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

authorization 与 minimum disclosure 必须先于任何 sensitive source、Field、contribution、attachment、Annotation 或 index-private read。Search 不得通过 hit count、ranking gap、snippet、completion suggestion 或 unavailable reason 泄露 hidden existence。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

Derived Index 只做 candidate accelerator。building、partial、stale、unavailable 与 proved complete zero-result Query 完全不同。complete search 必须具备 current authorized query_scan，以及全部真实 positive/negative source、Registry、contribution、extraction 与 authorization dependency。partial exploration 只可显示 explicit covered range，并持续标为 incomplete；不能签发 complete ActionEvidence，也不能把 current page 当 whole result。

selected Field/SearchContribution/provider/extraction missing 时，在普通 disclosure gate 后返回 retained unavailable/reset result，绝不能 fallback-empty。budget exhaustion 按 retained D7 budget error 使 complete result 失败；不能返回 first N rows再称 complete。cancellation 保持 retained explicit outcome。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

hit count 只从 authorized complete result 计算。building 时 UI 可以不显示 count 或显示 unknown，不能估计 hidden matches。ranking 只用 authorized matched value 与 retained D7 comparator。

snippet/highlight 只能从已经 authorized 的 matched semantic text 派生。其 scalar offset 是 result value 内的 ephemeral presentation offset，不是 D3 Locator coordinate。打开 hit 时沿真实 Ref/provenance/Locator rule 做 fresh current resolution/read；若 result/source stale，就 revalidate/reset，不能把 display offset 套到新 bytes。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 8. Persistence、reopen、copy 与 import；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

保存 search 只保存既有 canonical Query definition，以及 explicit scope、parameter、stable D4 FieldId、selected SearchContribution contributionId/version dependency 与 author-defined ordering。shortcut string 不是 durable search authority。非作者 device 可以为 convenience 记 parser version 或 last text，但它们可丢弃，不能进入 DynamicBlock、DefinitionTransfer、result cache 或 ActionEvidence。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

reopen 先按 recorded author schema decode saved Query，再资格化 current definition、Registry、contribution 与 permission。Field deletion 或 incompatible type change 走 Registry migration/unavailability；SearchContribution removal/version change 走 D10 activation dependency；permission change reset current result。rename/move 改 current filename/path value，但不按文字改变 stable content identity。

copy/fork/import 对 canonical Query payload 使用既有 D7 Definition Transfer。typed Ref 与 DefinitionAddress root 按 D3 规则 map；FieldId 与 SearchContribution stable identity 继续作为 semantic dependency，不从 label 猜。unknown payload 不自动变成新 search syntax。old result row、snippet 与 evidence 不会被复制成 current authority。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 9. List、full search 与 saved Query 的 hit equivalence

当 file-list、global search 与 saved Query 配置成 byte-equivalent canonical condition、scope、contribution set、ordering 与 current dependency cut 时，它们具有相同 D7 result semantics。surface pagination 或 virtualization 不改变 membership/order。

capability difference 必须显式。无法取得 required provider、complete range、secure snippet、bidi interaction 或 assistive navigation 的 surface，要报告对应 capability unavailable/incomplete，不能改跑另一条 hidden Query。跨 device equality 是 canonical Query 与 qualified dependency 的 semantic equality，不是 pixel equality。

## 10. SEARCH-01–08 acceptance；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。
；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。
| ID | Normative D7 closure 与后续 D8 acceptance ；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。 |
| --- | --- ；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。 |
| SEARCH-01 | File-list filter、quick open、global text search 使用 §3 的 scope；empty input browse 与 explicit search 分开；recursion、Resource、Annotation、Trash、body、attachment/OCR 都必须 explicit。 |
| SEARCH-02 | title/subtitle/filename/path/resource-name/hierarchy 分域；titleless 不补 title；duplicate display value 保留 distinct Ref；rename/move 使 old result qualification stale，但不按 text 改 identity。 ；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。 |
| SEARCH-03 | current match 为 case-sensitive NFC exact substring over Unicode scalars；CJK/RTL deterministic；Boolean AND/OR/NOT explicit；不声称 fuzzy/tokenizer/Pinyin/stemming；unsupported future mode 不 silent run。 ；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。 |
| SEARCH-04 | Shortcut mode 必须 explicit，并采用 §5 的 @ operator、quote、escape、precedence 与 error；ordinary colon text、URL、drive letter、title: 继续 literal；invalid/incomplete shortcut 执行零 alternate Query。 ；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。 |
| SEARCH-05 | Visual/shortcut condition 编辑同一 model；无法表示的 advanced condition 保留不丢；focus/keyboard/touch/AT/IME/CJK/RTL 语义按 §6，并等待后续 D8 platform evidence。 |
| SEARCH-06 | permission 先于 sensitive read；hidden count/rank/snippet 不泄露；index building/partial/stale/unavailable 与 complete zero 分开；missing contribution/provider 与 budget 有 explicit unavailable/error；ordinary open/edit/save 独立。 ；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。 |
| SEARCH-07 | save/reopen/copy/import 持久化 canonical Query 与 stable semantic dependency，不保存 UI state；Field/contribution/version/permission change 重新资格化或 reset；old result 不取得新 current eligibility。 |
| SEARCH-08 | hit 打开时 fresh current authorized resolve source；snippet/highlight offset 不是 Locator；同 canonical condition 在 list/full/saved surface 产生同 membership/order，同时 capability difference 明示。 |

以上是 design acceptance obligation；本批不宣称任何 product search、platform interaction、tokenizer、provider、accessibility、performance 或 cross-device test 已 PASS。
