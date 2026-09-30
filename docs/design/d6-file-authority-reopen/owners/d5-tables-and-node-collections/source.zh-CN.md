---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：b83a76f3-6d5d-463b-9f51-ed8144d24602。

# D5 Tables and Node Collections — D6-FA-r01

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 的 D5 v1 作为来源与兼容历史；本文件是完整 D5 owner 后像，直接消费 H2 D3 wire12、H2 D6 SemanticState/SourceVersion/CommitDomain 和本批 D4 后像。没有建立 Record/RecordCollection durable domain。

固定来源：
- S=7e18168dad3e6d120fce0dd607dc10fa7894e252
- source blob=166b1aebd43c00bb9c1152c567efbd46c7b5ffb8

## 1. 问题、选择与上游

D5 负责表格和 Node 集合的结构语义，不拥有 Document bytes、Field schema、Node identity、parent/order或 Query execution。

冻结选择继续是：

1. D4 Field Value Occurrence 不是 Record；
2. D2 Document Table Row/Cell 是 current-revision Document occurrence，
  没有 durable row/cell identity；
3. Node Collection 是 D7 Query 求值出的 live authorized NodeRef集合，不是 parent、owner、保存 membership 列表或 RecordCollection；
4. 外部/IR records 不取得 Workspace identity，除非显式 import/create产生 fresh Node；
5. D5 不建立 RecordRef、TableRowId、CollectionRef、ViewRef 或数据库式 row store。

P/I不可建立第二份 table row、collection membership、Field occurrence或Node parent/order current truth。

## 2. 完整替代比较与行域

| row-like thing | authoritative domain | identity/lifetime | allowed operation |
|---|---|---|---|
| D4 Field Value Occurrence | owning Document source + D4 Entry | revision-bound occurrenceKey selector；非durable identity | D4 Field edit |
| D2 Document Table Row | exact-source table occurrence | current Document revision locator/ordinal | D5 native table edit |
| D2 Document Table Cell | row内 Inline* occurrence | current revision locator/column position | D5 native cell edit |
| NodeCollectionResult row | NodeRef | Node durable identity保持 | D7/D5 collection Action |
| Import/worker row | external/IR | 无 Workspace identity | D9 preview/import；fresh Node on commit |

UI呈现为“表格”不改变上述 domain。尤其 names/phones/addresses 等 list-of-object仍是 D4 typed values/occurrences；不能因为编辑器像表格就暗建 Record identity。

## 3. Document Table

### 3.1 grammar与source authority

Document Table 仍是 D2 native AsciiDoc table occurrence，
  作者权威是 `.adoc` exact source。D5 structured edit必须 round-trip D2 grammar、
  delimiter、cell separator、header/body/footer、ragged rows、
  multi-line content、block/inline boundaries与原 trivia。

缺失 trailing cells是真正 absent，不自动补 empty。Cell内容是 Inline*，不能从 `0012`、`true` 或日期样式推断 D4 type。

table/row/cell locator均 revision-bound；相同文本/位置在新 SourceVersion不产生 identity continuity。I中的 parsed table可删重建。

### 3.2 native table local edit

以下可以是普通局部 source operation，不需要全 Workspace Query cut：

- 修改一个 representable cell；
- append/remove row；
- append/remove logical column；
- 在安全条件下 reorder rows；
- 修改表头/标题；
- raw source body edit。

每个 structured operation至少绑定：

- current full Document source + SourceVersion/2；
- exact table locator/current revision；
- D2 table parse和目标row/cell/column；
- actual MutationFootprint；
- current write permission；
- 唯一 proposed full source；
- D6 file install qualification。

如果同一 source 中 D4 unavailable/invalid namespace不相交，仍须保持其 raw bytes byte-equal；D5 structured edit不能顺手重排/重写它。

### 3.3 ragged/trivia/reorder

ragged rows合法；D5不把短行扩成长行。column insertion/removal必须给每行确定 source transformation，不能靠 renderer补位。

row reorder 只有在 D2 表内没有会被移动语义破坏的 inter-row blank/comment/trivia 时才是 structured-safe；否则固定 `unsupported_table_reorder`，用户可转 Source editor。不得移动comment到另一row或丢失CRLF。

### 3.4 unrepresentable cell

如果 structured UI value不能无损表示为当前 D2 cell grammar，则返回 `unrepresentable_cell`，零source write。不得转义到另一个隐藏语法、HTML blob或私有sidecar。

## 4. D4 Field occurrences 与“表格式”编辑

D4 repeatable Field occurrences可以由表格式/列表式UI编辑，但 D5不把它们变成 Document Table Row或Record。

对一个 occurrence 的 add/remove/replace/reorder/note仍走 D4 Entry/1、OccurrenceKey、TypeSpec、RegistryBinding、SourceVersion和Field权限。D5 UI可以提供 row-like interaction，但保存结果仍是 D4 source transformation。

D4 retained_unavailable/invalid Field不可因 D5 表格UI而变 empty。
  partial typed projection可显示 raw/status，但 structured typed mutation仍服从 D4 operation matrix。

## 5. Node Collection

### 5.1 成员语义

Node Collection 是一个 Query 定义在准确 params/context/auth/cut 下得到的去重 live authorized NodeRef集合。

membership是 derived：
- 不写 `memberOfCollection`；
- 不创建 CollectionRef；
- 不改变 D3 parent；
- 删除 collection definition不删 Node；
- 同一 Node可同时属于多个 collections；
- result row顺序来自 Query，不是 membership identity。

transport pagination不是 membership。Query `take`若在 semantic Query中则真正限制 membership；只传前200 rows的 page不改变集合。

### 5.2 completeness

collection membership若用于显示探索，可以显式标 partial/pending并只展示已证明的 rows；但下面操作要求 future D7 complete result/cut：

- “全部成员” mutation；
- remove-from-collection需要反演 membership；
- bulk update；
- collection postcondition；
- create with requireMembership；
- Automation以集合全集选targets。

I coverage、当前 page、未下载 cloud placeholder、旧cache都不能冒充完整 membership。

D7新 Prepared contract尚未冻结时，上述强入口保持 unavailable/owner_update_required；D5不得私造一个 PreparedActionBinding/3 或 generic token。

## 6. Collection Creation Policy

CollectionCreationPolicy仍属于保存定义中的明确构造意图，不是parent owner或持续 Template authority。它可以声明：

- destination parent；
- title/body/source defaults；
- ordinal；
- optional Template/constructor引用；
- optional explicit author facts；
- `requireMembership` postcondition。

create本身使用 D3 wire12 create_node。replica_local create可以在local source/parent/Facet合法时可靠保存；如果 collection membership完整性尚未证明，不能宣称 requireMembership Action完成。对 `requireMembership=true` 的 collection-create强Action必须 managed/complete，并在 proposed state用同 definition/params/auth/dependencies重新求值，证明 fresh Node确实在完整结果中。

destination parent失效拒绝，不fallback到root，不从排序邻居猜 parent。

## 7. Remove from Node Collection

`RemoveFromNodeCollectionIntent` 是 D5语义 intent，不是新 D3 operation kind。

可用性取决于 selector能否有确定、有限、可预览的 author change：
- explicit refs selector可修改保存定义；
- 某个 D4 Field predicate可提出明确 Field edit；
- arbitrary filter若无法唯一反演为author change则 unavailable。

“Remove from collection”不等于 Trash Node。UI必须分开显示 collection membership edit、D4 fact edit 和 D3 Trash。Trash仍服从 D3 wire12 local/managed规则。

## 8. Native table row to Node / import

把 table row 转成 Node 是 identity-changing conversion：

- source row仍是 Document occurrence，无 row identity；
- target NodeRef fresh；
- 默认不把 row locator当 identity；
- Resource/Annotation若复制到new owner则 fresh；
- preview展示 source row、target source、字段映射、normalization/loss；
- commit由 D3 create/import + D9映射；
- source row是否保留/删除必须显式，不能生成镜像后自动同步。

跨 Workspace transfer仍是 target fresh-copy receipt + optional source Trash receipt 两个独立 authority结果，不声称原子 move。

## 9. delete、batch 与 failure

### 9.1 distinct delete modes

D5必须区分：

1. delete table row：改 Document source；
2. delete D4 occurrence：D4 Field edit；
3. remove from Node Collection：改变定义或真实membership fact；
4. Trash Node：D3 lifecycle；
5. purge Node：D3 managed_atomic irreversible operation。

一个 `Delete` 按钮不能凭当前View上下文猜哪种语义。

### 9.2 batch target freeze

批量preview后，targets必须固定到 exact NodeRefs/table locators/occurrence selectors和版本。commit不得重新执行“当前所有选中/当前前N条”扩大 target set。

partial result不能生成 all_result target list。权限/SourceVersion/Frontier变化使旧preview stale或需要重新prepare，不用最新page悄换targets。

### 9.3 atomicity

单 Workspace strong batch必须同一 managed decision中全部成功或全部失败，服从 D3/D4/D6/D7 owner gates。跨 Workspace不存在一份原子receipt；每个 Workspace独立receipt，协调结果必须呈现target/source各自状态。

## 10. D6-FA proof matrix

### 10.1 proof dimensions

D5操作可能需要：

A. current Document source/SourceVersion/2；
B. table/row/cell locator或 D4 occurrence selector；
C. actual MutationFootprint；
D. local D2/D4/D5 structural validity；
E. complete Query membership/negative range；
F. proposed full source/post-state；
G. current authorization/policy；
H. CommitDomain/Frontier/install；
I. D7 new preparation，仅强 collection/bulk consumer。

### 10.2 operation matrix

| operation | local proof | complete proof | result |
|---|---|---|---|
| raw/native table cell edit | A-D/F-G/H | 无全库membership | ordinary可靠save |
| append/remove table row | A-D/F-G/H | 无全库membership | ordinary可靠save |
| structured row reorder | A-D + trivia-safe | 无 | ordinary或 unsupported_table_reorder |
| column edit | A-D/F-G/H | 无 | ordinary save；不得推断D4 type |
| D4 occurrence edit | D4完整local Entry proof | 依Field跨对象义务 | 由D4 complete/pending决定 |
| D3 replica_local move/reorder | D3 parent/sibling proof | Node Collection membership不因此证明 | local success；collection consumer仍pending |
| D3 replica_local Trash | D3 local closure | collection/inbound全集可缺 | local lifecycle pending，不等于bulk集合Action |
| collection local read | authorized rows +明确coverage | 不要求写 | partial/exploratory可用 |
| collection requireMembership create | local create prerequisites | complete Query cut/postcondition | D7新版前 unavailable；以后 complete only |
| remove from collection | explicit invertible plan | complete membership/postcondition | complete only |
| bulk update/all_result | frozen complete targets | future D7 complete cut | complete only |
| restore/purge | D3 managed_atomic | D4/D5 complete + purge Frontier | complete only |
| typed row import to Nodes | D9 mapping + D3 identity | 完整target plan | managed path；无D7旁路 |

D6 `collection` obligation表示 collection-related complete proof未完成，不是空集合。r5 pending(collection)后来r6完整求值，只证明r6/current SourceVersion/cut，不修改r5 receipt。

## 11. dynamic schema、nested values 与 import/export

D5不从 UI row shape发明 Record。D4 nested object/list/union保持其自己的 Typed Value；
  entry-local selector/occurrenceKey不是 RecordRef。

People names/phones/addresses/engagements等 list-of-object继续由 D4 catalog定义；D5比较Record替代的历史结论仍是“不建立Record domain”。稳定局部编辑通过 D4 occurrence/Entry semantics，而非隐藏row identity。

导入表格/CSV/worker rows是外部IR；只有D9 explicit import commit后才生成 fresh Node。IR row number/provider ID不是 NodeId。

普通 XLSX/CSV export从当前 source/Query显式读取；partial collection export必须标 partial 或拒绝 complete-export mode，不因 page size小于结果集就截断并声称完整。

## 12. limits 与 budgets

fixed D5 hard limits保持：

| contract | maximum |
|---|---:|
| explicit Node targets in one collection mutation | 1000 |
| native table structured row targets | 1000 |
| preview detail page | 200 |
| fetched result page | 200 |
| one import batch new Nodes | 1000 |

更窄 runtime/policy budget可以降低，不得放宽这些上限。1000是target count，不是读取全库的许可；complete range仍按实际语义证明。

大表/大库操作必须有 bounded parser、streaming/分页和cancel；不能为了消除pending把整个几十GB workspace无界读入内存。

## 13. multi-replica、I/P 与 conflicts

I中 table parse、collection result、membership candidate都是 derived，可删除重建。I重建不产生作者source、membership decision或 old complete proof。

P丢失不能从当前table/query结果恢复旧 batch decision、Prepared binding或 approval/Money。普通 native table source仍按 D6/D3 recovery规则继续。

两个replica：
- 同table同cell/row source change → source conflict；
- 不同rows若文件CAS冲突，可在 exact base +两分支source上提出明确merge；
- collection定义与member facts并发变化 →重新取得complete cut；
- 一个replica只有page/placeholder不能声称全集。

hash相同、row文本相同、I cache相同不能恢复旧 SourceVersion/locator/cut资格。

## 14. mandatory domain dispositions

### People

names/phones/addresses/relationships/engagements的表格式UI不创建 Record。相同 phone value的多个 occurrence仍按 D4 selector/notes/provenance区分。

### Organizations

memberships/roles/relationships同样不创建 Record。Organization-side inverse UI修改必须最终落到唯一 D4 authored relation owner，不双写。

### Calendar

recurrence derived occurrence不创建table row/Record identity。series override继续由 D4/D3/D7/D9边界处理，不以View row身份持久化。

### Library

bibliographic lists/creators/resources仍使用 D4 Fields、NodeRefs/Resources；Citation row不变成Record。

这些 mandatory scenarios是压力义务，不是新增产品功能批准。

## 15. D7/D8/D9边界

D7拥有 Query execution、complete result、new Prepared/Action evidence；D5不定义成功新版binding。

D8可以用 table/list/board UI呈现 Node Collection，
  但View pagination/filter device state不改变 portable membership semantics。
  native Document table editor仍写 exact source。

D9拥有 row import/export mapping、Office template/export和loss report；D5只提供真实 table/collection domain，不让模板column绑定反向要求普通 Node保存 export-only metadata。

## 16. legacy compatibility

旧 D5 v1 semantic names与旧D7 PreparedActionBinding/1,/2 saved decisions保持历史decoder/bytes。D3 v9/v10/v11及D6 wire1旧decision按原规则恢复。

D6-FA新 consumer将 SourceVersion/2、CommitDomain/Frontier、SemanticState接入 operations，但不新造 TableRowId/RecordRef/CollectionRef。

新 D7 Prepared未完成前，collection strong Action保持unavailable；不能借ordinary D6 source-save产生“旧Action已完成”的receipt。

## 17. completion conditions

未来实现必须验证：

1. native local cell/row/column edit在无关全库index缺失时仍可可靠保存；
2. unrepresentable cell拒绝且source不变；
3. ragged rows/trivia保留；
4. unsafe reorder返回unsupported；
5. Field occurrence edit仍走D4；
6. collection page 200不冒充membership；
7. `take` semantic与transport page分开；
8. partial exploration不能bulk/all_result写；
9. create requireMembership缺complete cut拒绝；
10. remove arbitrary noninvertible filter unavailable；
11. batch targets固定，不commit时扩大；
12. cross-Workspace两个receipt；
13. multi-replica conflict显式；
14. r5 replay/r6 current分开；
15. rebuild I/lost P不恢复proof；
16. legacy saved bytes不变。

这是设计候选；未声称实现、性能或产品测试通过。

## 18. 候选接受边界

本后像不建立Record domain，不修改D2 grammar/D4 catalog/D3 identity。后继不可变candidate仍需fresh独立联合审查和协调接受。

旧D10 B13保持REVISE、术语/双语FAIL、11 OPEN；本文件不关闭任何finding。

## 19. 规范性 D5 v1 exact-contract 恢复

本节恢复 fixed-S 中必须逐项保留的 native table、collection create/remove、bulk、import/export、budget 和 stage-adapter 细节。若前文概述与本节有张力，以本节为准；D6-FA-r01只改变外层source/version/qualification binding，不增加Record identity或新版D7 Prepared。

### 19.1 native AsciiDoc table grammar 与 structured intent

D5 native table继续完整服从 D2 v2：

~~~text
delimiter           := |===
row                 := one logical line beginning with |
cell separator      := unescaped |
escaped pipe        := \|
escaped backslash   := \\
cell content        := Inline*
unsupported in D5 v1 native structured model:
  span
  cell block
  nested table
  header option as a schema/type declaration
~~~

第一行从不自动成为schema，显示数字/日期不推断D4 type，读取不把ragged rows补成矩形。

每个 native structured edit绑定：

~~~text
owning NodeRef
current SourceVersion/2
current Document revision represented by that SourceVersion
current table locator
exact source range
actual MutationFootprint
one explicit operation:
  insert row at explicit ordinal
  remove selected rows
  replace one existing cell with explicit Inline source
  insert/remove logical column
  reorder selected/all rows only when trivia-safe
~~~

insert row显式给全部cells；replace要求cell确实存在。ragged table的column edit必须逐row明确所有变换，不能自动补padding。

plain-text cell intent必须证明D2 parse后恰为请求的inert text；高级Inline source中的link/ref继续过D2/D3验证。无法无损表达newline/reserved syntax/complex content固定：

~~~text
unrepresentable_cell
~~~

并保持zero author write。不得截断、偷偷拆行、执行macro、使用未冻结escape或写隐藏sidecar。

row reorder只有没有inter-row blank/comment trivia时允许整表reorder；否则固定：

~~~text
unsupported_table_reorder
~~~

完整source保留。纯view sort不写source。

### 19.2 行域、Field-cell 与 occurrence edit

D5五种row-like域保持互斥：

~~~text
native Document table row  -> D2 table_row occurrence
Node Collection row        -> NodeRef
D4 repeatable fact row     -> Field Value Occurrence
Query aggregate/join row   -> D7 derived row
Resource preview row       -> derived view of Resource bytes
~~~

NodeCollectionResult只含NodeRef，不能混入Field/table/aggregate row。排序、pagination、hidden column不改变domain或权限。

一个D4 Field cell的可编辑mode必须显式：
- zero occurrences -> append；
- exactly one -> replace that current selector；
- multiple -> select one occurrence OR explicit append OR explicit replace-all。

replace-all必须展开全部被删除occurrences、notes、qualifiers、provenance和relation effects并过同一gate。空显示不等于missing/unknown/invalid/null/empty text；D4没有generic null。清空输入必须明确合法empty text或remove occurrence。

replace target exact绑定：

~~~text
owner NodeRef
FieldId
SourceVersion/2 / current source revision
occurrenceKey
expected raw Entry
edit mode
~~~

只改note保留Entry其他作者内容及所有未选中source/trivia。旧selector不因key bytes相同跨revision继续有效；若Core提出replan，必须保留base/current/proposed并重新证明目标与不相交变化，不能靠(FieldId,value)、row number、nearest text或key单独承接。

### 19.3 Collection Creation Policy 与 parent priority

Node Collection membership来自 D7 在明确 Workspace、snapshot、authorization、compiled Query与params上的求值。去重按完整NodeRef。集合不是parent、owner或lifecycle container。

“在集合中新建”的reviewed plan至少包含：

~~~text
destinationParent: NodeRef
final sibling ordinal
explicit title
initial Document source
required initial Facet/Field author facts
optional completed Template construction input
requireMembership: Boolean
definition/query binding + params + authorization/dependencies
~~~

parent唯一优先级：

~~~text
explicit destination in this request
  >
explicit parent in saved creation policy
  >
Node containing the saved definition
~~~

ad-hoc Query没有containing definition，调用方必须显式给destination；禁止猜Workspace root或当前selected/sorted row。default parent仍需live、
  same Workspace、authorized、structurally legal。
  default ordinal是commit-plan pre-state中parent child count；并发变化必须replan。

Template只做一次性D9 source construction，不成为实例持续authority。冲突时拒绝，不last-wins；无Template也可由caller给显式title/body/facts创建。

requireMembership=false时动作明确是“create Node”，预览必须说明它可能不在当前result。requireMembership=true时在完整proposed post-state、相同Query/params/auth/dependencies下重新完整求值，证明fresh Node属于semantic result后才commit。transport page不参与；Query本身top/limit等semantic operators参与。无法证明则collection-create strong action unavailable。

### 19.4 Remove from Collection、Delete 与 batch

RemoveFromNodeCollectionIntent只在存在一个明确、有限、可preview的作者变换时可用：
- 修改显式refs selector；
- 修改确定的D4 Field fact；
- 修改保存定义。
任意无法唯一反演的filter/aggregate固定unavailable。

以下动作必须在UI/intent层明确分开：

~~~text
delete Document table row
delete D4 Field occurrence
remove from Node Collection
Trash Node
purge Node
~~~

Trash preview展示真实D3 subtree、Resources/Annotations、relation effects和隐藏children closure；同一Node出现在多个collections也只执行一次该NodeRef lifecycle。

bulk preview先冻结完整targets和每个target version；commit不重新执行“当前全选/当前前N”来扩大目标。要对新结果操作必须重新preview。page前N不是whole-result范围。

同Workspace strong batch整体成功或拒绝；拒绝无成功receipt且作者source保持。跨Workspace遵守两份独立authority result：

~~~text
target fresh-copy receipt
source Trash receipt (optional and separately authorized)
~~~

target失败不删source；source Trash失败显示copied/source retained，不自动伪装atomic move。

### 19.5 row→Node、import/export 与 dynamic schema

table row→Node是fresh identity-changing conversion。计划包含title、parent、完整Field mapping、losses、每个原occurrence disposition。保留原row只是一次性copy；promotion则同一允许的原子plan把原row替换为普通Node link row，不能留下声称同一对象事实的cell mirror。Node→table只是export/snapshot，NodeRef永不变row identity。

“新增列”可只选择已有Field显示；创建新Field是Registry evolution，不在首次填值时推断type。breaking change使用fresh ID+explicit migration。unknown/uninstalled/incompatible namespace保留raw，typed column unavailable而不是empty。

CSV/XLSX mapping至少显式：

~~~text
input format + encoding
sheet/table/range
header presence
exact header names and duplicate-column selection
space / empty cell / missing column / null distinctions
target FieldId per column
conversion per column
title construction
destination parent
initial Facets
duplicate-row policy
source/provenance/losses
formula policy
quantity/resource budgets
~~~

不得按display column label/locale猜decimal/date/ref。formula/external links不执行；cached value、formula text或reject必须显式选择并记录evidence freshness。普通import不按title/path/phone/email/external UUID合并现有Node。SourceBinding比较域与active OriginBinding upsert分开，SourceBinding本身不授权target update。

长import由显式有限batch构成；每batch原子，但整个job不冒充单一原子commit。job保存已提交batch receipts和input binding，续作重验证未提交部分。D9不能用相同row text猜重复并跳过。

### 19.6 hard limits、budget 与 partial result

fixed D5 hard limits：

~~~text
one Node collection mutation explicit Node targets <= 1000
one native table structured edit row targets      <= 1000
one preview detail page                           <= 200
one fetched grid/result page                      <= 200
one import batch new Nodes                        <= 1000
~~~

请求超过固定上限返回：

~~~text
limit_exceeded
~~~

不得自动truncate或silent split。更窄runtime/policy budget可提前拒绝，但不能放宽上限。

数量上限不是充分budget；Core还要绑定source bytes、decoded value bytes、read dependency count、write bytes、time/cancellation和materialization budget。达到任何较窄界限整体拒绝。preview 200只是显示截面，不能使未显示effects变“已审阅”。

10,000 overlapping-engagement Person Nodes的colleague查询必须按需派生，不能持久化约5000万edges。大型同构import验收至少10,000 Nodes、10 batches、failure around batch6、repeated resume和working-memory小于完整workspace input；这是未来D6/D7/D9 evidence义务，不是性能已通过。

### 19.7 D5 intent adapter 最小语义

后续实现必须一一提供closed request/plan/error/receipt adapter；D5不提前定义D7 Prepared wire。

~~~text
edit native table:
  binds owner NodeRef, SourceVersion/Document revision,
        table locator, exact source ranges, explicit transforms
  success -> one complete valid proposed Document source
  reject  -> stale locator/revision, invalid/unrepresentable cell,
             limit/auth/D2/D4 full-source failure

edit Field occurrence:
  binds owner, FieldId, RegistryBinding, SourceVersion,
        occurrenceKey, expected raw Entry, explicit edit mode
  success -> D4 proposed state + relation effects
  reject  -> stale/ambiguous/unavailable schema/
             constraint/auth/relation conflict

create through collection:
  binds saved-definition locator/revision or ad-hoc Query binding,
        parent/title/source/ordinal, requireMembership,
        complete dependencies
  success -> fresh NodeRef + valid source/placement +
             membership proof when required
  reject  -> parent/stale/auth/template/schema/membership/budget

remove membership:
  binds selected NodeRef, Query+params,
        saved-definition locator/revision or ad-hoc binding,
        actual read dependencies,
        explicit author-fact/definition transform
  success -> complete post-query proves Node no longer a member
  reject  -> noninvertible/no real write target/stale/auth/
             cannot prove complete result

trash selected Nodes:
  binds dedup exact NodeRef target set + D3 closure/state
  success -> D3 lifecycle receipt
  reject  -> root/hidden/unauthorized/stale/incomplete closure

import batch:
  binds exact input, finite mapping, title/parent/facts, budgets
  success -> fresh Nodes + per-row mapping + D3/D9 receipts
  reject  -> unresolved format/loss/schema/limit/stale/auth
             with zero author writes for the batch
~~~

row handle、column index、caption、
  caller自报before-image或cached decoded value都不是write authorization。
  Core重验证真实source、authority、visibility、revision与完整read-set。preview不是commit；timeout/disconnect不能由UI猜success。

### 19.8 D6-FA-r01 current composition

所有native table source mutation绑定 D6 SourceVersion/2、CommitDomain/2、current Policy和实际FileObjectBinding/install能力。仅hash相同不证明row/cell locator或source continuity。

collection strong action还需要 future D7 complete cut/Prepared。当前D7新版未完成时它固定 unavailable/owner_update_required；D5不创建替代token。explicit partial/pending rows只用于获权exploration，不参与all_result/bulk/requireMembership/Automation writes。

D6 `collection` obligation对应的保存状态可表现为 semantic_pending(collection)：它表示完整collection proof尚未成立，不表示empty。r6后来complete只证明r6；r5 pending receipt保持原bytes。I重建、P丢失、新replica、placeholder或A→B→A都不恢复旧membership/cut proof。
