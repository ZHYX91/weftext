---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：b83a76f3-6d5d-463b-9f51-ed8144d24602。

# D5 Tables and Node Collections — D6-FA-r01

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 的 D5 v1 继续作为来源与兼容历史；本文件是完整 D5 owner 后像，当前消费 actual A D6 的 qualified SourceVersion/2、SourceObservation/1、SourceVersionRef/1、CommitDomain/2、Frontier/2 与 SemanticState，actual B D1/D3 当前规范接口，以及本批当前 D4 后像。H2 仅保留为历史兼容与 saved-decoder 背景，不作为当前 producer 基础。本文件没有建立 Record/RecordCollection durable domain，也不据此声称独立接受、激活、实现或额外完整阅读。

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
| D4 字段值出现项 | 所属 Document source + D4 Entry | 绑定 revision 的 occurrenceKey selector；非持久 identity | D4 Field 编辑 |
| D2 文档表格行 | exact-source 表格出现项 | 当前 Document revision 的 locator/ordinal | D5 原生表格编辑 |
| D2 文档表格单元格 | row 内 Inline* 出现项 | 当前 revision 的 locator/column position | D5 原生单元格编辑 |
| NodeCollectionResult row | NodeRef | Node durable identity保持 | D7/D5 collection Action |
| Import/worker row | external/IR | 无 Workspace identity | D9 preview/import；fresh Node on commit |

UI呈现为“表格”不改变上述 domain。尤其 names/phones/addresses 等 list-of-object仍是 D4 typed values/occurrences；不能因为编辑器像表格就暗建 Record identity。

完整替代方案及取舍如下：

| 方案 | 实际优势 | 决定与成本 |
|---|---|---|
| 持久 Record 加 RecordCollection | 可紧凑保存大量同构短行 | 本代不采用：它会在 Node 之外新增作者存储、引用、schema、CRUD、权限、Trash、copy、fork 和迁移域 |
| 把每个电话、表格行或 recurrence override 都建成 Node | 统一取得独立身份 | 不采用：产生不必要的 ID 和存储，无限 recurrence 也不可能全部物化成 Node |
| 用普通 Document 表格作为持久数据库 | 源码可读且可编辑 | 不作为身份模型：持久行 ID 造成双重权威，无法保持外部编辑、重排和复制的语义 |
| 只用 CSV/JSON Resource | 适合可携带的原始数据与不透明附件 | 保留这种 Resource 用途，但不作为唯一模型：行没有独立 CRUD、引用和生命周期 |
| Node 加 D4 事实、无身份的 Document 行和派生行 | 只保留一个持久内容身份域 | 采用；短行仍承担明确的标题、parent 和源码成本，百万 Node 性能要由 D6 实测证明，不能承诺 OLAP 能力 |

只有新增“无需 Document 但必须独立持久化行”、且现有模型无法以合理成本满足的硬需求，才重新打开 Record 决策。插件、缓存、行句柄或文件格式不能暗建该域，D5 也不为它预留 RecordRef discriminator。

## 3. Document Table

### 3.1 grammar与source authority

Document Table 仍是 D2 原生 AsciiDoc 表格出现项，作者权威是 `.adoc` 精确源码。D5 按 §19.1 每行一个逻辑行的准确语法，保留 delimiter、separator、Inline* 单元格、不齐行、换行形式和未修改的 trivia；不新增多行或块级单元格、嵌套表、跨度或 header/body/footer schema 模型。

缺失 trailing cells是真正 absent，不自动补 empty。Cell内容是 Inline*，不能从 `0012`、`true` 或日期样式推断 D4 type。

table/row/cell locator 继续是绑定 revision 的稳定生产地址，而 structured-operation selector 还必须绑定完整 current SourceObservation/1。持久 managed locator 在另一副本只能通过 D3 已认证 canonical RevisionTokenBinding/2、该副本新 current Observation 的 exact production-version equality、准确 coordinate/table parse 与最终 same-cut 门重新取得新读取资格。watcher gap/replacement 会使旧 sourceToken 和依赖它的 structured selector/preparation 失效，但不会因此改写 persistent managed 地址。相同文本/位置、裸 revision、hash 或 I 都不能证明当前 source 或恢复旧 selector。

### 3.2 native table local edit

以下可以是普通局部 source operation，不需要全 Workspace Query cut：

- 修改一个 representable cell；
- append/remove row；
- append/remove logical column；
- 在安全条件下 reorder rows；
- 编辑普通首行单元格文本，或单独修改 Document 标题；
- raw source body edit。

每个 structured operation 至少绑定以下证据；持久 locator 的读取成功本身不是可复用 structured-write selector，新 prepare 必须冻结本次新的 Observation/selector：

- current full Document source + SourceVersion/2，其中不透明 Document revision token 通过真实 RevisionTokenBinding/2/RevisionTokenSource/2 先解析稳定生产版本；
- 与该生产版本逐字相等的完整 current SourceObservation/1，并通过 SourceVersionRef/1.sourceToken 保护本次当前观察连续性；
- 当前 observation cut 内真实 fileObjectBinding、evidencePins、author-control、Registry、incidence 依赖；
- exact table locator/current revision；
- D2 table parse和目标row/cell/column；
- actual MutationFootprint；
- current write permission；
- 唯一 proposed full source；
- D6 file install qualification。

这些只是局部 structured source operation 的真实证据，不要求无关 Workspace 全 Query cut，也不自动取得 weak save 资格。current auth、local typed/full-source 检查必须实际通过；任何已绑定观察、pins、control、Registry 或 incidence 依赖变化都使旧计划 stale/reprepare；Frontier 推进按 §19.9 的 exact/scope_dependencies 规则判断，不能用裸 version/hash/I 或最新 page 静默替换目标。

如果同一 source 中 D4 unavailable/invalid namespace不相交，仍须保持其 raw bytes byte-equal；D5 structured edit不能顺手重排/重写它。

### 3.3 ragged/trivia/reorder

ragged rows合法；D5不把短行扩成长行。column insertion/removal必须给每行确定 source transformation，不能靠 renderer补位。

只有表内不存在行间空行或注释 trivia，才允许结构化行重排；否则固定 `unsupported_table_reorder`，用户可转 Source editor。不得移动comment到另一row或丢失CRLF。

### 3.4 unrepresentable cell

如果 structured UI value不能无损表示为当前 D2 cell grammar，则返回 `unrepresentable_cell`，零source write。不得转义到另一个隐藏语法、HTML blob或私有sidecar。

## 4. D4 Field occurrences 与“表格式”编辑

D4 repeatable Field occurrences可以由表格式/列表式UI编辑，但 D5不把它们变成 Document Table Row或Record。

对一个 occurrence 的 add/remove/replace/reorder/note仍走 D4 Entry/1、OccurrenceKey、TypeSpec、RegistryBinding、SourceVersion和Field权限；其既有 sourceRevision/OccurrenceKey/Entry selector 不改 wire，并额外由同一 current SourceObservation/1、SourceVersionRef/1.sourceToken、当前 observation cut、fileObjectBinding/evidencePins、author-control、Registry/incidence 依赖保护。watcher gap、replacement 或 discontinuous rematerialization 后，旧 token/selector 不因 production version、hash、Entry bytes 或 row-like text 相同而继续有效；I 不能恢复资格。D5 UI可以提供 row-like interaction，但保存结果仍是 D4 source transformation，并继续执行当前 auth、local typed 与完整 source 检查。

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

新的 unseen 强集合或批量操作必须取得实际已协调的 D7 完整结果及 Prepared 生产、消费合同；缺少所需合同则返回 unavailable/owner_update_required，不建立新决议。D5 不自造 Prepared binding 或旁路 token。这个新执行门禁不阻止真实 saved/planned 记录的交付或原计划恢复（§19.11）；当前 D7 协调仍按 §19.12 明确保留待完成项。

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

partial result不能生成 all_result target list。preview必须冻结完整目标、实际readSet、evidence pins与所依赖的current Observation/cut；已绑定权限、SourceObservation/1、SourceVersionRef/1.sourceToken、SourceVersion/2、control、Registry、incidence 或 pins 变化使旧 preview stale 并要求重新 prepare。Frontier/2 扩展服从 §19.9 及实际 consumer 的重置规则，不用最新page、当前选择或重新求值结果悄换targets。Frontier/2只表示sealed causal/dependency prefix，不是全集Query、Registry完整性或payload物化证明。

### 9.3 atomicity

单 Workspace strong batch必须同一 managed decision中全部成功或全部失败，服从 D3/D4/D6/D7 owner gates。跨 Workspace不存在一份原子receipt；每个 Workspace独立receipt，协调结果必须呈现target/source各自状态。

## 10. D6-FA proof matrix

### 10.1 proof dimensions

D5操作可能需要：

A. current Document source + SourceVersion/2生产版本，以及由 SourceVersionRef/1.sourceToken 选择并由 InputDescriptor/2.sourceInputs[].observation 承载的完整 current SourceObservation/1；生产 SourceVersion/2 的 commitDomain/版本历史保持原义，operation CommitDomain/2可以不同于生产域；
B. table/row/cell locator 保持既有不透明 revision-token 语义，D4 occurrence selector 则使用真实 managed 内层整数；
C. actual MutationFootprint；
D. local D2/D4/D5 structural validity；
E. complete Query membership/negative range；
F. proposed full source/post-state；
G. current authorization/policy，以及current Observation cut内的fileObjectBinding、evidencePins、control、Registry、incidence依赖；
H. operation CommitDomain/2、Frontier/2和实际install evidence；Frontier/2只证明sealed causal/dependency prefix，不代替全集Query、Registry完整性或payload物化；
I. D7 new preparation，仅强 collection/bulk consumer。

这些证明仍保留原B-D/F等局部要求；已绑定 current Observation 或 readSet/pins 变化必须 stale/reprepare；Frontier 扩展按 §19.9 判断，不能用裸version/hash/I或最新page替换原目标。

### 10.2 operation matrix

| operation | local proof | complete proof | result |
|---|---|---|---|
| raw/native table cell edit | A-D/F-G/H | 无全库membership | ordinary语义；原生结构化单元格编辑使用strict保护 |
| append/remove table row | A-D/F-G/H | 无全库membership | ordinary语义；native structured row edit使用strict保护 |
| structured row reorder | A-D + trivia-safe | 无 | ordinary语义下strict保护，或 unsupported_table_reorder |
| column edit | A-D/F-G/H | 无 | ordinary语义下strict保护；不得推断D4 type |

ordinary与strict|observed_only是两条独立轴。ordinary操作可以使用strict；不要求无关Workspace全集Query并不产生weak资格。仅actual A§4.1允许的人工raw整源existing-Document save可以在planning前显式选择observed_only：必须是trusted interactive_source_save、恰一个既有live Document、ordinary + replica_local、完整source read/replace，author write set为空或仅该Document，无applicable body/Field/Node-control deny，无identity/parent/order/lifecycle/sharedPolicy/Registry/Calendar-scope/other-entity mutation，并且DraftBase等于当前选定Observation。该选择冻结在profile中；strict失败、已知冲突、授权失败、耐久失败或strong obligation失败都不得fallback为observed_only。structured table row/column/cell/reorder、bulk、collection、promotion、automation、server checkpoint、Approval、Money及所有strong Action都继续使用strict保护。这两条轴都不扩大授权、local typed/full-source检查、proposed full source或D6 install要求。

observed_only只保证耐久保留已读前像B与用户输入N：安装N可能覆盖从未观察到的外部C，后来C也可能再次替换current file，但B/N不能因此丢失。已观察competition、stale Base、watcher gap或continuity gap必须conflict/reprepare；unknown install保持recovery_unknown。prepare只保存proposal/read-before/pins等准备证据，不是Saved。任何要求strict或strong保存的路径都必须取得对应真实D6 install/evidence，不能伪造耐久receipt。
| D4 occurrence edit | D4完整local Entry proof | 依Field跨对象义务 | 由D4 complete/pending决定 |
| D3 replica_local move/reorder | D3 parent/sibling 证明 | Node Collection membership 不因此得到证明 | 局部成功；collection consumer 仍 pending |
| D3 replica_local Trash | D3 局部 closure | collection/inbound 全集可缺 | 局部 lifecycle pending，不等于 bulk 集合 Action |
| collection local read | authorized rows +明确coverage | 不要求写 | partial/exploratory可用 |
| collection requireMembership 创建 | 局部创建前提 | 完整 Query cut/postcondition | D7 新版前 unavailable；以后仅 complete |
| remove from collection | explicit invertible plan | complete membership/postcondition | complete only |
| bulk update/all_result | 已冻结的完整 targets | 未来 D7 complete cut | 仅 complete |
| restore/purge | D3 managed_atomic | D4/D5 complete + purge Frontier | complete only |
| typed row import to Nodes | D9 mapping + D3 identity | 完整target plan | managed path；无D7旁路 |

D6 `collection` obligation表示 collection-related complete proof未完成，不是空集合。semantic_pending(collection)只表示缺少该全集证明；它不能把typed invalid、source invalid、缺失strong evidence或其它实际失败洗成成功，也不能授权Action、all_result或批写。r5 pending(collection)的历史receipt和原bytes保持不变；后来r6完整求值只证明r6及其current SourceObservation/SourceVersion/cut，不修改r5 receipt。

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
| fetched editable-grid page | 200 |
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

D6-FA新 consumer将 SourceVersion/2、SourceObservation/1、SourceVersionRef/1、CommitDomain/2、Frontier/2 和 SemanticState 接入 operations，但不新造 TableRowId/RecordRef/CollectionRef。SourceVersion/2 继续保留生产版本原有的 commitDomain、observationEpoch、revision 或 externalSequence、changeId 语义；生产域可以不同于当前 observerDomain，而 current SourceObservation/1 的 observerDomain 必须等于 operation CommitDomain，entityRef 必须等于 sourceVersion.entityRef，并以当前 fileObjectBinding、evidencePins、control、Registry、incidence 与 cut 完成外层资格。SourceVersionRef/1.sourceToken 以 d6_source_observation/1 选择完整当前 Observation；它不替换 D5/D4 既有 inner sourceRevision、OccurrenceKey、Entry selector 或 locator wire。Frontier/2 只表示 sealed causal/dependency prefix，不证明全集 Query、Registry 完整性或 payload 物化。watcher gap、replacement 或 discontinuous rematerialization 会使旧 SourceVersionRef/current Observation 以及依赖它们的 selector/preparation 失效。managed persistent Locator token 只继续表示稳定生产地址；后续新的读取只有通过 D3 的 canonical binding、exact production version、新 current Observation 与原 coordinate 门，才可重新取得资格。I 既不能恢复地址真实性，也不能恢复当前资格。旧 D5 v1、D7 PreparedActionBinding/1,/2、D3 v9/v10/v11 和 D6 wire1 的 saved decoder、receipt bytes 与恢复规则仍按历史版本处理，不机械改写成当前 token，也不据此宣称新版 D7 success。

只有缺少实际已协调 D7 producer 的受影响 unseen 强集合操作才保持 unavailable；真实 saved/planned/unknown 记录先按 §19.11 恢复。ordinary D6 source-save 不能伪造旧 Action receipt。

## 17. completion conditions

未来实现必须验证：

1. native local cell/row/column edit在无关全库index缺失时，仍可凭完整真实局部证据完成ordinary语义保存；structured cell/row/column/reorder使用strict保护。仅人工raw全文已有一个live Document的source save可在actual A §4.1全部资格满足时，于planning开始前显式选择observed_only并冻结profile；ordinary与strict|observed_only两轴不得混淆为无条件可靠或严格耐久，strict失败不得fallback，strong Action、bulk、automation、Approval、Money等路径不放宽；
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
  any table attribute or header option (including presentation-only headers)
~~~

第一行从不自动成为schema，显示数字/日期不推断D4 type，读取不把ragged rows补成矩形。

每个 native structured edit绑定：

~~~text
owning NodeRef
current SourceVersion/2
current opaque Document revision token resolved by RevisionTokenBinding/2
current SourceObservation/1 selected through SourceVersionRef/1.sourceToken
current observation cut + fileObjectBinding/evidencePins/author-control/Registry/incidence dependencies
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



原生表格的首行仍是普通 cell；不存在“设为显示表头”动作或独立 header 类型。D2 无条件禁止 table attribute/header option，即使它仅用于展示而非 schema/type。普通首行 cell 文本编辑和独立 Document title 修改仍分别使用其既有操作；不得为兑现旧“表头编辑”措辞新增语法。

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
complete managed SourceVersion/2 / its actual inner sourceRevision
current SourceObservation/1 selected through SourceVersionRef/1.sourceToken
current observation cut + fileObjectBinding/evidencePins/author-control/Registry/incidence dependencies
occurrenceKey
expected raw Entry
edit mode
~~~

只改note保留Entry其他作者内容及所有未选中source/trivia。旧selector不因key bytes相同跨revision继续有效；若Core提出replan，必须保留base/current/proposed并重新证明目标与不相交变化，不能靠(FieldId,value)、row number、nearest text或key单独承接。

集合的可编辑列仅限标题、完整 D4 Field，或 D4 明确支持编辑的结构成员。NodeRef、路径、投影有效性、逆关系及聚合列都不是通用可写单元格。新增观察应追加出现项；明确纠错才替换选定出现项。多个尚未解决的值保持可展开，偏好单值摘要不删除历史。关系编辑只写一次真实 canonical 作者端，逆向展示不能双写。

D4 内层 sourceRevision 取自真实完整 managed 生产 SourceVersion/2 的整数 revision。external SourceVersion/2 只有 externalSequence，没有 managed revision；externalSequence、epoch、Locator token 和 incidence-range token 都不能冒充该整数。确实需要内层整数的 D4 结构化操作，先完成显式获权的 managed admission/save，再从成功封存的 managed 源码准备新的结构化请求。admission 结果未知就保持未知。读取不能自动接纳或写入；raw/read/repair/Draft 与独立合格的 ordinary 操作仍可用。

### 19.3 Collection Creation Policy 与 parent priority

Node Collection membership来自 D7 在明确 Workspace、snapshot、authorization、compiled Query与params上的求值。去重按完整NodeRef。集合不是parent、owner或lifecycle container。

“在集合中新建”的reviewed plan至少包含：

~~~text
destinationParent: NodeRef
final sibling ordinal
explicit title
initial Document source
required initial Facet/Field author facts
optional revision-bound completed Template construction input
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

Template 只负责一次性 D9 源码构造，必须绑定预览已审阅的准确 Template revision 和完整构造结果。Template 或冲突输入发生变化使新计划失效；提交不能追随最新 Template 或悄悄重算构造。调用方也可以不用 Template，显式提供标题、正文和事实。planning CAS 一旦成功，parent 子项数量或 Template 的变化按原计划恢复规则处理，不能改写已保存的计划（§19.11）。

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

同 Workspace 强批次整体成功或拒绝。安装前已确定的拒绝没有作者写入，但保留真实 D3 recorded-rejection/决议历史；安装或封存结果未知时按 §19.11 恢复，不能声称零副作用或换新操作重试。跨 Workspace 遵守两份独立 authority result：

~~~text
target fresh-copy receipt
source Trash receipt (optional and separately authorized)
~~~

target失败不删source；source Trash 失败显示 copied/source retained，不冒充原子移动，也不自动重试删除。

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

D4 semantic-major 身份不可变，沿用其演进账本。Field/type 的破坏性变化必须使用新 ID，并明确迁移、损失及完整拟议状态验证。闭合对象、union 与有界集合只用于 D4 允许的位置，并遵守最大嵌套深度 8、对象成员数 64、集合项数 256。导入与本地编辑都必须经过 Entry/schema admission，不能靠导入绕过限制。未知 JSON 成员必须明确保留或损失处置，不能静默丢弃；嵌套值不构成通用 record store。

导出分别处理普通 Document 表格、Node 集合和 Query 值表。Office 映射属于明确的导出模板，普通 Node 不持久化仅供导出的 schema。重新导入创建新 Node，不复活行身份。保存的 Query/View 定义是所属 Node 内的出现项，没有 ViewRef；复制定义不复制派生成员。退役 Record token 必须拒绝，不能强制转换为 NodeRef。

ICS 或其他同步来源中，D3 SourceBinding 定义外部来源实例、scope、映射命名空间和外键比较域；active OriginBinding 才将准确 ForeignIdentityKey 映射到 NodeRef。initial_import/adopt/upsert 及 retired/non-live 情况服从真实 D3 状态矩阵；SourceBinding、UID、标题或路径本身都不能授权 CRUD。未绑定的只读 provider 仍可用于订阅，但不能自称已有可更新的同步绑定。有限 import/adopt 以明确映射和绑定创建新 Node，不能为无限 recurrence 的每次出现创建 Node。UID、RECURRENCE-ID 和 LogicalOccurrenceKey 不是 NodeRef；VFREEBUSY/VTIMEZONE 不创建 Node，VEVENT/VTODO/VJOURNAL 必须明确映射为 event、Task 或 journal。

### 19.6 hard limits、budget 与 partial result

fixed D5 hard limits：

~~~text
one Node collection mutation explicit Node targets <= 1000
one native table structured edit row targets      <= 1000
one preview detail page                           <= 200
one fetched editable-grid page                    <= 200
one import batch new Nodes                        <= 1000
~~~

请求超过固定上限返回：

~~~text
limit_exceeded
~~~

不得自动truncate或silent split。更窄runtime/policy budget可提前拒绝，但不能放宽上限。

数量上限不是充分budget；Core还要绑定source bytes、decoded value bytes、read dependency count、write bytes、time/cancellation和materialization budget。达到任何较窄界限整体拒绝。preview 200只是显示截面，不能使未显示effects变“已审阅”。

Template 构造产生的每个新 Node（包括全部 descendants）都计入导入批次上限：单个输入行产生 1001 个 Node 也返回 limit_exceeded。这些是 D5 编辑、grid 和导入边界，不限制 D2 语法、通用 D7 Query 分页合同或 Workspace 总规模。有限 effects 全集必须可以跨预览页完整取得；详情页上限不能截断 effects 或悄悄拆开原子意图。

10,000 overlapping-engagement Person Nodes的colleague查询必须按需派生，不能持久化约5000万edges。大型同构import验收至少10,000 Nodes、10 batches、failure around batch6、repeated resume和working-memory小于完整workspace input；这是未来D6/D7/D9 evidence义务，不是性能已通过。

### 19.7 D5 intent adapter 最小语义

后续实现必须一一提供closed request/plan/error/receipt adapter；D5不提前定义D7 Prepared wire。

~~~text
edit native table:
  binds owner NodeRef, complete SourceVersion and opaque Document revision token,
        current SourceObservation/1 via SourceVersionRef/1.sourceToken,
        current observation cut + fileObjectBinding/evidencePins/control/Registry/incidence,
        table locator, exact source ranges, explicit transforms
  success -> one complete valid proposed Document source
  reject  -> stale locator/revision, invalid/unrepresentable cell,
             limit/auth/D2/D4 full-source failure

edit Field occurrence:
  binds owner, FieldId, RegistryBinding, SourceVersion,
        current SourceObservation/1 via SourceVersionRef/1.sourceToken,
        current observation cut + fileObjectBinding/evidencePins/control/Registry/incidence,
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

所有 native table source mutation 都绑定 D6 `SourceVersion/2`、当前 operation 的 `CommitDomain/2`、current Policy，以及实际 `FileObjectBinding` 与 install 能力。`SourceVersion/2` 继续表示生产版本，并保留真实 `commitDomain`、`observationEpoch`、`revision` 或 `externalSequence`、`changeId` 语义；其生产域可以不同于当前 operation 的观察域，不新增任何 producing-domain 字段。

当前资格由 `SourceObservation/1` 额外保护：其真实形状为 `kind=d6_source_observation`、`version=1`，`observerDomain` 必须等于 operation `CommitDomain`，`entityRef` 必须等于 `sourceVersion.entityRef`，并包含对应 `sourceVersion`、当前 `observationEpoch`、`fileObjectBinding`、`evidencePins`，同时与当前 control、Registry、incidence 依赖及当前 cut 一致。`InputDescriptor/2.sourceInputs[].observation` 承载该完整 Observation；`SourceVersionRef/1` 的 `sourceToken` 使用 `d6_source_observation/1` 标记并选择完整当前 Observation，而不是裸 revision、hash、I cache、row text 或 production version。`Frontier/2` 只表示已 seal 的 causal/dependency prefix，不单独证明全集 Query、payload 已物化或 Registry 完整。

D5 既有绑定 revision 的 table/row/cell Locator 与 D4 数字 sourceRevision/occurrence inner selector 形状保持不变。managed Locator token 是稳定生产版本地址；每次新读取/操作仍以 current `SourceObservation/1` 为额外资格。watcher gap/replacement 会使旧 sourceToken/selector/preparation 失效，即使 production version 未变。之后对同一个 persistent locator 的另一次新读取，只有 D3 重新验证 canonical binding、相同 exact production version、新 current Observation 与原 coordinates 后才可成功；绝不复活旧 selector。I 既不能提供地址真实性，也不能提供当前资格。

新的 unseen 强集合操作还需要实际 D7 complete cut/Prepared 合同；缺少所需 owner 时返回 unavailable/owner_update_required，不建立新决议。D5 不创建替代 token。明确标出的 partial/pending 行只供探索，不参与 all_result/bulk/requireMembership/Automation 写入。真实 saved/planned/unknown 记录沿用 §19.11 恢复分支；§19.12 记录剩余 D7 协调。

D6 `collection` obligation对应的保存状态可表现为 semantic_pending(collection)：它只表示完整collection proof尚未成立，不表示empty，也不能把typed invalid、source invalid、缺失strong evidence或其它真实失败洗成成功，更不能授权Action、all_result、bulk或Automation。r6后来complete只证明r6及其current SourceObservation/SourceVersion/cut；r5 pending receipt及其历史原bytes保持不变。I重建、P丢失、新replica、placeholder或A→B→A都不恢复旧membership/cut proof。


### 19.9 原依赖与 Frontier 扩展

D5 消费真实 D6 DependencyProof/2，包括十四种闭合 DependencyKey/2，以及 placement_range 下九种 StructureRange；不登记一个包揽所有依赖的 D5 key。每个意图绑定 owner 实际要求的 source、control、权限、Registry、关系和集合的正负范围、pins 及物化证据。完整空范围与非空范围的证明标准相同；隐藏、不可读、未物化输入、范围连续性丢失、未知 schema 或 partial I 都不等于空。披露权限、ObservationScope 和 owner 可用性检查先于受保护读取、Entry 解码及业务验证。仅丢失可重建 I 缓存，不必使仍有真实保护证据的输入失效；丢失真实范围或当前观察证据则会失效。

exact 要求原 expectedFrontier 不变；所有 D3 managed_atomic 路径继续使用 exact。只有实际操作 profile 允许 scope_dependencies 时，才可接受有完整中间记录和 proof 链的连续、已验证封存、因果不回退扩展。原来绑定的每个 Observation/token、selector、pin、权限 generation、DependencyProof stamp、Registry/rule 和正负范围都必须保持有效且不变；扩展与它们无关的证明须保存在原 P 计划中。更大的向量数值、provider 同步状态、相同最终 bytes 或 I 都不够。expectedFrontier、Notice.baseFrontier、targets、Query、拟议源码、版本基础和 token 均不改写；重新签名不能修好过期绑定。无法证明无关则 conflict/reprepare。D7 完整结果的权限及依赖重置规则仍然适用，这不是旧结果或句柄普遍存活的许可。

### 19.10 生产 revision 与无改动效果

managed 生产 revision 只由 D6 分配。实体 E 在生产域 D 中的 H(D,E)，是该域连续封存历史中的最大 managed revision；epoch 变化不重置 H。外域 before 的 revision 为 90，也不能给已证明完整空历史、H=0 的新域提供版本值；后者的新 managed after 为 1。P 或可携带历史缺失、有缺口都不等于空；到 MAX 时检查式递增失败，不能回绕。返回旧生产域时继续该域自己的 H，externalSequence 永不成为 managed revision。

每个真正的新 managed after，都在原计划赢得 CAS 时冻结 SourceRevisionPlan/1 的 before、lastIssued 或已证明空历史基础、after SourceStamp/1 及 afterPin。D3 新身份和 D7 两遍物化共用原 candidate map 与版本基础。预览和 planning 不产生 ChangeId 或已封存 SourceVersion；只有 P seal 才把拟议 stamp 与该决议唯一 ChangeId 结合，递增一次 H 并产生真实 managed SourceVersion/2。D5 不建立另一个分配器或账本。

真正 raw no-op 保留完整旧 source version 和精确 bytes，即使它来自另一生产域；它不建立 SourceRevisionPlan、source revision、H 增量、内容 ChangeId 或 Frontier head。源码未变的可携带结构或 lifecycle 效果也保留该源码基础，不产生新 source version/H 增量，但可以有该决议真实的 portable ChangeId。删除使用 after absent，不伪造“已删除源码版本”。相同 bytes 的 external-to-managed admission 是显式接纳，不是 raw no-op：它有真实 SourceRevisionPlan，并使用 after 域的 H+1。混合操作中，未变 owner 保留自己的完整旧基础，变化或新建 owner 使用各自冻结的计划；不能借另一 owner 的 revision。

新的可携带发布消费 ContentCompletionProof/3，其 before/after 是真实生产 SourceVersion，并使用原来保留的 Frontier 扩展证据。回执交付或发布失败不能重复安装、分配、递增 H、收费或执行效果；恢复只重发原已封存 proof/outbox。历史 proof 版本保留原 bytes 和 decoder。

### 19.11 实际入口与 saved/planned/unseen 恢复

共同的闭合解码、当前最低披露权限和 ObservationScope、适用的 domain/authority/fence、P custody/continuity 检查均先于业务查账。先比较原 DecisionKey、protocol owner 和完整规范 request/fingerprint，再按状态分支；同一个 key 的不同请求或 owner 是 mismatch，不能另建决议。

| 原记录状态 | D5 consumer 义务 |
|---|---|
| saved，包括已耐久记录的拒绝或 terminal 结果 | 按原效果或结果披露范围检查当前交付权限，交付准确原 receipt/error/effects 或恢复原发布；不再要求旧 before 仍为 current、Frontier 相等、TTL 未过期、当前 r6 完整性或新版 consumer；撤权只能隐藏交付，不能改历史 |
| planned 且未 seal | 恢复同一 request、descriptor/owner version、identity map、适用 SourceRevisionPlan/H 基础、pins、reservations/write set、Notice、保护模式、attempts/budget 和原 preparation TTL/clock；当前权限、依赖连续性及真实安装来源只决定原计划继续、暂停、冲突或 recovery_unknown；不重选 page/Query/target，不另 prepare，不重抽身份或修改 H/after |
| unseen | 才应用真实当前 producer/consumer 门禁及 planning CAS；缺少 strong owner 时用既有 unavailable/owner_update_required/proof_unavailable 返回且不建立新决议；不依赖该 owner、已完整合格的 ordinary/local 路径仍可用 |

原生 D3 identity_operation_request wire12 使用闭合 InputDescriptor/2、d3_identity_operation/12 owner descriptor、受保护输入和原生私有计划；没有 planToken、D6 prepare 要求、PreparedIntent/2 或 expectedDomainFenceToken。只有真实 D6 d6_commit_request/2 入口才检查它实际声明的这些成员。D7 preparationBinding 只在真实 D3/D7 合同允许且要求的路径检查，不改变唯一原生提交入口或单一 P planning CAS/seal。D5 不虚构成功 preparation wire。

install/P/seal 结果未知时，保留原记录、pins 和未清责任；这不是业务拒绝，也不是新重试。不能从当前文件、hash、I 或丢失的 P 推断成功、零副作用、退款、配额重置、reservation 释放、再次收费或 Approval/Money 使用，也不能换新 OperationId。真实历史 D3/D6/D7 记录保留原版本、decoder 及恢复义务；保留实际历史不意味着声称未部署原型曾经激活。跨 Workspace 的 target copy 与 source Trash 各自保留 domain、授权、DecisionKey 和 receipt，不能用一方推断另一方成功。

### 19.12 内部协调与接受边界

本批此前的一个依赖现已有同包作者修复候选：PL-IR-01 使用 D6 稳定生产地址 binding 加 D3 fresh-current-Observation 新读取算法，并由上文消费；在本新 exact candidate 获独立复核前，**不得自行宣称已接受或关闭 P1**。另一项仍是 D7 complete result、preparation、preview/effects 的最终协调 producer/consumer 后像。D5 不重签旧 runtime binding、不虚构 D7 新版；真实历史记录保持原 saved/planned/unknown 恢复。独立接受前必须核对 PL01–PL18 与实际 D7 producer；已合格 raw/read/repair/Draft/local 路径继续可用。
