---
_weftext:
  id: "b83a76f3-6d5d-463b-9f51-ed8144d24602"
---

# D5 Tables and Node Collections

接受证据：D5总控验收（外部控制记录未随本输入发布）；Pro完整原报告（外部控制记录未随本输入发布）；四项P2修正与逐字差异（外部控制记录未随本输入发布）。正文保留完整接受设计及其语义；候选称谓、完成条件或首次命名版本反映其来源，不重启已完成评审。此权威与控制索引须在协调切换journal（外部控制记录未随本输入发布）为committed后一起生效，准备中或已回滚整批不生效。

状态：已冻结的D5架构权威，revision02-editorial-01。D5已冻结revision02-editorial-01：独立Chat GPT-6 Pro对revision01明确accept/pass、P0=0/P1=0，完整阅读/语义覆盖及术语通过；四项非阻断P2已由总控作保持既有语义的编辑修正，未新增评审轮次。 本文不证明产品实现或性能；实际生效以同批committed切换journal为准。

## 1. 问题、选择与上游

用户既要在文档里写普通表格，也要从表格式界面管理人员、组织、任务和大量结构化资料。显示成一行并不足以说明它拥有独立身份；编辑一个单元格也不能绕过真实作者源。D5 决定行域、集合成员、读写意图、规模边界及与后续实现阶段的接口。

本候选选择：**不新增持久 Record/RecordCollection 域**。有独立正文、引用、附件、关系或生命周期的受管对象使用 Node；Node 内独立重复事实使用 D4 Field Value Occurrence；普通文档表格仍使用 D2 table/row/cell occurrence；Query 产生的值行和派生 recurrence occurrence 保持无持久身份。“记录集”作为独立受管产品对象及公共 API 名称退出本代设计，不能作为 Node Collection、普通表格、CSV 附件或任意 Query result 的别名保留。

冻结输入为 D1 产品表面边界、最终 D2 v2、D3 wireVersion9 及其39项 Lexicon、D4 revision37-preservation-01 决议/术语/Impact。D4 的类型、Field、Facet、关系、Registry 和 source admission 原样承接。它们不要求每个 phone、name、appointment 或 recurrence occurrence 成为 Node。本候选的“统一 Node”仅指需要独立内容实体的受管数据，绝不是把所有值、表格行、外部事件都物化为 Node。

D1 五表面使用同一 Core；本机或 Server Core 是写入权威。D2 Node 恰有一份 exact-source Document，body 可空但 title 必须显式；NodeCollectionResult 不拥有成员，保存 Query/View 只能是 Node Document 内 occurrence。D3 NodeRef 跨同 Workspace 移动/改名保持，复制/普通导入 fresh；Resource/Annotation 严格 owner-local。D4 occurrenceKey 仅是绑定当前 owner、Field 和 Document revision 的值内 selector，不能跨 revision 自动延续目标。本候选不修改这些合同，不要求正式重开 D1–D4。

## 2. 完整替代比较

| 方案 | 权威/身份/编辑闭合方式 | 收益 | 代价与本候选裁决 |
| --- | --- | --- | --- |
| 独立 Record 与 RecordCollection | 新内容入口、独立作者序列/row store、与 NodeRef 分域的复合引用、新 schema/CRUD/授权/Trash/copy/fork/迁移合同；进入正文只能显式转换 | 海量同构短行不承担一份 Document、title 和 Node tree placement；集合级列 schema 自然 | 本项目已要求的人员/任务/组织是 Node，重复事实已有 D4；再加 Record 会引入两套受管实体及跨域转换。大数据存储收益真实，但当前场景没有证明它足以承担此额外领域合同。reject for this generation。 |
| 所有行和值都成为 Node | 每个 table row、phone、override 都 fresh Node，关系连接父对象 | 独立引用/lifecycle 统一 | 作者意图并不要求这些身份；膨胀、无限 recurrence 和注释迁移成本违反场景。reject。 |
| 普通表格兼任记录数据库 | 给 Document row 生成持久 ID/row schema；正文和索引共同写 | 初看交互简单 | 违反 D2 row occurrence 与单一作者权威；重排、外部编辑、删除与跨文档复制后身份无唯一意义。reject。 |
| 所有结构化资料只保留为 CSV/JSON Resource | owner-local bytes 唯一权威；解析结果只读 | 大型外部文件便携、无隐式物化 | 无独立作者行编辑/引用/生命周期；只能作为保留原始输入的明确选择，不能替代受管对象管理。accept as opaque attachment, reject as sole data model。 |
| Node + D4 独立事实 + 无身份 Query/文档行 | 每个具有内容实体需求的对象是一份 Node；事实和派生结果使用现有非身份承载 | 复用唯一 source/schema/ref/lifecycle；从表格到正文不需要“提升 Record” | 短行也需要显式 title/parent，一百万短行的存储和索引成本仍须 D6 证明；不能宣称通用 OLAP/电子表格数据库。chosen。 |

选择基于所需语义，不以历史实现或已投入成本否决 Record。以后若出现必须独立持久行身份、无需 Document、且现有模型无法经济承载的硬需求，应正式重开 D5 比较 Record 全套合同，不能在插件、cache、Query row handle 或新文件格式里悄悄加入。D5 现在不预留可被当成合法值的 RecordRef discriminator。

## 3. 行域与生命周期

| 可见表格内容 | 真实行域 | 作者权威/目标 | 新增、移除与删除 |
| --- | --- | --- | --- |
| 原生 AsciiDoc 表格 | D2 table_row occurrence | containing Node 的 exact Document source；D3 当前 revision locator + row/cell ordinal | 插入/删除 source occurrence；不创建/删除 Node，不生成 row ID。 |
| 人员/任务等节点集合 | 去重的 live、authorized NodeRef set | 各成员自己的 Document/Node control；集合定义位于另一个或同一个 Node Document | 新增是显式 create Node；移除成员必须修改明确的作者事实或集合定义；trash Node 是另一个有具体闭包的动作。 |
| names、phones、任职、关系编辑器 | D4 独立 Field Value Occurrence | owner Node、完整 FieldId、绑定当前 source revision 的 occurrenceKey | append/replace/remove/reorder D4 Entry；note 内嵌；不生成 Record/Node/Annotation identity。 |
| join/group/aggregate/展开后的 Query 表格 | D7 typed value/relation row，可能有重复 | 派生值与短期 D7 evidence；无作者行容器 | 默认只读；只能通过明确命名且重新验证目标的 Action 写真实作者源，不因有 NodeRef 列而自动可编辑。 |
| recurrence 日历行 | D4/D7 derived occurrence | series authored facts + rule/provider snapshot | 改 series/override 是相应显式操作；单次采用/提升遵守 D3/D4/D9，绝不把屏幕行号作为身份。 |
| CSV/JSON/XLSX 附件预览 | Resource bytes 的派生行 | owning Node 的 opaque Resource bytes | 附件替换是 Resource 操作；导入为 Nodes 是 fresh create，预览本身不“管理”外部行。 |

NodeCollectionResult 只含 NodeRef，不能混入 Field row、table row 或 aggregate row；D7 可把两种结果共同显示在一个 dashboard，但每个 result/table 的 domain 必须显式且稳定。排序、分页、列隐藏不改变 domain，也不赋予行权限。保存的 Query/View 无 ViewRef，复制其 owning Node 服从 D3；复制定义不复制查询成员，删除定义不删除成员。

## 4. 集合成员与新增节点

### 4.1 成员语义

Node collection 的成员由 D7 对一个明确 snapshot、workspace、授权上下文和已编译 Query 的求值产生。selection 去重按完整 NodeRef；title、路径、同名、值相等不去重身份。一个 Node 可在多个集合中，集合不是 parent、owner 或生命周期容器。

属性/Facet/关系谓词不隐式包含物理目录条件，因此同 Workspace move 保留身份且不改变这些谓词的真值。若 Query 明确要求某 parent/subtree/path，移动可能改变 membership；它仍是 Query 重算结果，不是从集合“搬走”了一个被拥有对象。源字段编辑、Facet Remove 或关系删除也可能使行消失，预览须显示这种后果，提交仍针对真实 owner。

手工选定 NodeRef 的集合也必须通过保存的 Query/View payload 表达选定 refs，没有第二份 membership sidecar。D7 冻结具体 payload；外部错误/权限变化不得被解释成空集合并保存。完整求值是否可交付、分页/缓存/授权失效由 D6/D7 保证，无法证明当前结果有效时不得据旧行发起未绑定写入。

### 4.2 创建策略

“在集合中新建”必须在 Core 可审阅计划中给出：`destinationParent: NodeRef`、final sibling ordinal、显式 title、初始 Document source、所需 Facet/Field 作者事实、可选已完成的 Template 实例化输入，以及希望创建后仍在该集合内的要求。UI 不从当前排序行邻居推断 parent，不把字段排序序号当 Node sibling ordinal。

选择 parent 的唯一优先级：本次显式 destination > 已保存创建策略中显式绑定的 parent > 包含该保存定义的 Node。ad-hoc Query 没有 containing definition，必须由调用方给出 destination；不能猜 Workspace root 或当前选中行。默认 parent 也必须 live、同 Workspace、有授权且在当前结构中合法。默认 ordinal 为实际 commit plan 绑定 pre-state 中该 parent 的 child count，作为 append 的 final index；并发改变必须重规划，不能悄悄插到旧位置。

Template 是 D9 的一次性 source construction，不成为实例的持续权威。D5 只接受明确 revision-bound 的完整结果；模板要求与请求 Facet/Field 冲突时拒绝并呈现冲突，不 last-wins。未提供模板仍可用 caller 明确提供的普通 Node title/body/事实创建；不得要求用户为了列表录入先制作模板。

不对任意 Query 反求满足条件的 source。创建策略须显式列出实际要写的事实。`requireMembership` 是明确的 Boolean：true 时，在完整 proposed post-state、相同 Query/参数/授权/依赖下证明新 Node 属于该 Query 的完整语义结果后才能 commit；transport page 不参与成员判断，但 Query 自身的 top/limit 等语义算子仍参与，不能仅证明某个 filter 为真。不能验证（例如外部只读数据、非确定 snapshot 或不可写聚合）则该集合内创建不可用。false 时，操作明确叫“创建节点”，可以创建后不出现在当前结果，预览须告知；不得把 false 冒充保证集合内新增。

创建父节点不是被创建节点必须携带的 membership；parent 策略改变不移动既有成员。新的 filter、排序或显示列不 retroactively 改写任何成员 source。

## 5. 单元格和重复事实编辑

可编辑列分为 title、一个完整 D4 Field、或 D4 明确操作支持的结构成员；NodeRef/path/projection validity/inverse relation/聚合列不能用通用“写单元格”伪造。列定义引用完整 FieldId 和当前 RegistryBinding，不以显示 label 或列序号识别 schema；同名“状态/日期”必须消歧到对应 Fields。

多值 Field 单元格显示摘要不能建立单值假象。零条目可以执行 append；恰一条时可显式 replace 该当前 revision selector；多条时必须选择具体 occurrence 或明确选择 append/replace-all。replace-all 必须展开完整待删除条目、notes/qualifiers/provenance/关系效果并经过同一 gate，不得在普通输入时默认执行。空显示不等于缺失、unknown、invalid、null 或空字符串：D4 没有通用 null 值；清空输入必须明确是设置合法空 text 还是删除选中 occurrence。删除 required Field 最后一项、超 cardinality、Facet composition 冲突均拒绝。

replace 的目标为当前 owner + FieldId + source revision + occurrenceKey，并绑定 expected raw Entry；值相同仍是不同 occurrence。只改 note 必须保留该 Entry 的其余作者内容，完全不触碰未选择 Entry、namespace trivia、顺序和未知 namespace；未经验证的缓存不能变成作者源。D4 relation 走其 canonical owner、完整 read-set 和 proposed-state gate；inverse UI 只定位原事实并授权真实 owner，不在展示端保存 inverse 副本。

并发时，旧 selector 不因 key bytes 相同而跨 revision 继续有效。Core 可以在保留 base/current/proposed 三者后提出新 revision 的显式重规划，但只能以完整 source/read-set 证明目标和不相交变化；不能依 `(FieldId,value)`、行号、最近似文本或 key 单独自动承接。不能证明时返回冲突并保留用户提案。不同 Field 的变更不得被编码成替换整个 people object/namespace；D6 的细粒度 patch/rebase 必须保留不相关事实，D5 不声称当前纯设计已经证明物理并发合并。

当前/首选出生、姓名、测量等摘要沿用 D4 领域语义：新的 observation 使用 append；明确纠正某断言才 replace；多个未裁决断言必须展开选择；不能因为摘要只显示一个值就覆盖历史。共享 field-row editor 可以统一 add/remove/reorder/note 交互，但 event/state/measurement/relation 类型和约束仍由各自 schema 唯一决定。

## 6. 原生表格编辑与转换

原生表格完整服从 D2 v2：`|===` delimiter、每个 row 为一个以 `|` 开始的 logical line、未转义 pipe 分 cell、`\|` 和 `\\` 单次 escape、Inline* cell、无 span/cell block/嵌套 table/header option。D5 不把第一行自动升级成 schema，不根据显示数字/日期猜类型，不把表格补齐成长方形作为读取副作用。

原生编辑意图均绑定 owning NodeRef、Document revision、当前 table locator，以及原始 source 范围：insert row at explicit ordinal、remove selected rows、replace one cell 的明确 Inline source、insert/remove column。row ordinal 只在这一 revision 的 table 内有效。row insert 给出全部 cells；replace 必须存在该 cell。对于 ragged table，列操作必须明确给出每个受影响 row 的完整编辑；没有的 cell 不能自动创建或删除邻格，调用方可另行 preview 明确的 padding。blank/comment trivia 不计 row，未触碰 trivia 和行结束符逐字保留。

单元格写入接收明确且可由 D2 验证的 Inline source，不把任意 CSV 文本当 AsciiDoc 语法。不能无损表达的 newline、受保留语法影响的文字或复杂内容，必须返回 `unrepresentable_cell` 并让已授权映射选择 D4 text/Document/Resource 等适当承载；不能截断、偷偷拆行、执行宏、用未冻结转义或声称 lossless。plain-text intent 必须证明解析后确为请求的 inert text，否则拒绝；高级 Inline source intent 的 link/ref 还须完整 D2/D3 验证。

排序只是暂时 view 时不改 source；若用户明确请求“重排行”，计划必须是当前全部或明确选中 row 的置换，并给出 trivia 附着策略和完整 source delta，不能把排序展示当成保存。D5 v1 的明确策略是：只在没有 inter-row blank/comment trivia 的表格允许整表 row reorder；有 trivia 返回 `unsupported_table_reorder`，保留完整源，用户可通过普通 Document edit 明确处理。此限制是该编辑动作的边界，不使合法 D2 source 变 invalid。

表格 row → Node 是显式 fresh create 与 source 转换。计划给出 title、parent、完整字段映射、losses、每个原 occurrence 的 disposition。选择保留原 row 时只是一次性复制，不能声称两份持续同步；选择 promotion 时，同一原子计划把原 row 替换为只含普通 Node link 的 row，保留 surrounding table；不得留下仍声称同一对象事实的 cell mirror。D3 原有 checklist promotion 不受本规则改变。Node → table 是导出/快照复制；NodeRef 不变为 row identity，重新导入 fresh。

## 7. 删除、移除、批量与失败

必须区分四个动作：删文档行、删 D4 occurrence、从查询中移除、Trash Node。表格里的 Delete 键不能统一实施其中任意一种。一个 node collection 中“移除”只能选择明确的事实变更或修改包含该 ref 的保存定义；不能反演任意 filter。Trash Node 的预览须给出真实 Node subtree、Resources/Annotations、关系效果和 D3 gate；集合中未展示的 children 也不能隐瞒。多个集合显示同一 Node，只执行一次完整 NodeRef 对应的 lifecycle 动作。

批量操作预先固定明确 target 集合和每个目标 revision；不能把“全选当前过滤结果”在 commit 时重新求值扩大为新增对象。要对新结果操作须重新预览。按排序/分页取到的前 N 行不是隐含 whole-result 范围。一个批次原子成功或拒绝，拒绝保留全部作者状态并无成功 receipt；D3 planned/rejected/重试 allocation/receipt 规则仍按其原合同，不使用本句覆盖其 ledger 记录。

跨 Workspace 遵守 D3：target fresh copy 与 source trash 是分别授权、分别有 receipt 的两个提交。不存在 D5 的跨 Workspace 原子 move。目标 copy 失败不删源；源 trash 失败必须显示 copied/source retained，不自动重试删除。

## 8. 动态 schema、嵌套值与导入导出

“新增列”可只是选择已有 Field 的显示；创建新 Field 是另一个经 Core 的显式 Registry 演进计划。不能在填入首个数值时推断或改变 Field type。D4 semantic-major identity、永存 ledger、new FieldId/FacetId 的 breaking-change 规则完整保持。旧/new schema 的数据迁移必须有逐条映射、loss、权限和 complete post-state，不能修改同 ID 已绑定语义。unknown/uninstalled/incompatible namespace 保存 raw source，typed 列标记不可用，不显示为空且不因删除视图列而删除数据。

嵌套 JSON 只能显式映射到 D4 closed object/union/允许位置的 bounded collections、多个独立 Entries、Document 或 opaque Resource。D4 depth8、members64、items256、每 Entry/schema admission 等不因导入而放宽。独立 phone/names 不塞 generic array；额外 unknown JSON member 不静默丢弃，也不建立 record store。原始附件可保留用于 lossless 原件保管，与已导入 Node 是明确两个内容对象，不宣称自动同步。

CSV/Excel mapping 至少明确：输入格式与编码、sheet/table/range、header 是否存在及 exact 名字/重复列选择、空格/空单元格/缺列/null 的区分、每列目标 FieldId 和转换、每行 title construction、destination、初始 Facets、重复行策略、source/provenance/losses、formula 处理与数量预算。不得按列 label/locale 猜 decimal、date、ref；公式与外部链接不执行，cached value、formula text、拒绝三者必须显式选择且标记证据新鲜度。普通导入不按标题、路径、phone、email 或外部 UUID 合并既有 Node；fresh import 和已验证SourceBinding比较域及精确ForeignIdentityKey→NodeRef的active OriginBinding的同步更新分开；SourceBinding存在本身不授权更新目标。

长导入由显式有限 batches 构成；每个 batch 是原子动作，但整个 import job 不冒充一次原子提交。完整 job 保存已提交批次 receipts 和输入绑定，失败后只报告实际结果，续作须重验证尚未提交部分。D9 冻结 parser/IR/template 和 job wire，D6 冻结持久 journal；D5 要求 D9 不把中途失败变成成功总数、不通过相同 row text 去重重试。

导出普通表格、节点集合和 Query value table 必须显式不同 binding domain。普通 Node 无 export-only schema；映射/歧义消除保存在 Office 模板内。节点集合的 typed columns 来自 D4/D7，不能反向要求为普通 Document table 定义 FieldId。删除 Record domain 后模板必须拒绝旧 record token，不能偷转 node。CSV/Excel renderer、formula escaping、复杂源语法 loss 与 DOM/格式保真由 D9 验证，不以本架构表格断言其已完成。

## 9. 规模与能力合同

D5 不把所有数据同时载入内存，不定义“超过 N 行改成另一种身份”。Workspace 允许大于设备内存，实际 storage/Index/Query 必须有受预算约束的迭代/磁盘计划。没有一百万 Node 的实机证据就不能宣称同等性能；Node-only 的存储代价是公开的取舍。

本代 D5 编辑合同的硬上限是：一次 Node collection mutation 最多1000个显式 Node targets；一次 native table structured edit 最多1000个 row targets；一次 preview 页最多展示200行明细并提供完整有限 effects 的可获取清单；D5 editable grid 的每次 fetched page 最多200行；导入一个 batch 最多1000个新 Node。后者计入模板产生的全部 descendant Nodes，不只计导入行数。值/源更窄上限继续服从 D2/D4；这些数值是 D5 编辑协议边界，不是 Document grammar、一般 D7 Query 输出或 Workspace total-Node 上限。请求超过限额显式 `limit_exceeded`，不得自动截断或静默拆批。

上述数量还不是充分预算：Core 必须公布并绑定本次可用的 source bytes、decoded value bytes、read dependency count、write bytes、time/cancellation 与 materialization budget，任何达到较窄资源界限的计划整体拒绝。实际 product-wide budget schema、页句柄生命周期、全结果完成条件属于 D6/D7；D5 不自行发明另一个 ResourceLimit/Query wire。不同表面可有较低可用容量，必须在同一 capability 合同中明确不可用，不用近似结果改变语义。preview 的200行只是展示截面，不能据其把未展示 effects 视为已审阅；批量确认须提供总范围、完整可获取明细及实际限额。

必须以10,000个有 overlapping engagements 的 Person Nodes 验证派生 colleague 按需查询，不能创建约5,000万条持久关系或 Node；limit/cancel 不能发布不完整结果为成功。大型同构导入的阶段验收至少包含10,000 Node、10批次、故障在第6批前/后、重复续作与内存上限，且完整 workspace input 大于给执行器的可用 working memory。此为 D6/D7/D9 的未来实际证据义务，不是当前性能通过声明。

## 10. People、Organizations、Calendar 的 D5 裁决

People names/phones/emails/accounts/addresses、Organizations identifiers/classifications/sites/contacts、Person engagements/roles/relations 逐项选择 D4 独立 typed Entry；类型代码、validity、note/provenance 不是 Record 字段表。独立 Record 会给每次 phone/reference 引入新 resolver/Trash/owner；Document occurrence 缺少当前 D4 typed fact gate；每项 Node 引入不需要的独立 Document/parent；内容导出/并发编辑仍需原 owner。因此不修改 D4 架构。任职事实保持 D4 的唯一 authored side，Organization inverse 不能另写一份 membership；10,000参与者不能派生成新的持久 edges。

Schema 的 member/constraint/alias 数组是封闭 schema value，不是隐含记录集合；当前 Registry 所有权/演进不因此改变。D4 Entry note 内嵌；不选择独立 Annotation target。重复 value 的 note 准确绑定当前 occurrence；跨 revision 的永久 field link/Annotation 仍不可用，需要正式 D2/D3/D4 reopen，不能由 D5 UI 命名绕过。

对 ICS：已受管 series Node 使用 D4 rule/override structured facts；occurrences 无持久 identity。外部 subscribe/sync 仍以外部 source 为 authority，D10 connector cache/derived snapshot 不是新增本地 Record domain，也不获得 Workspace author CRUD；D3 SourceBinding确定外部来源实例、scope和mapping namespace的foreign-key比较域；OriginBinding才持久关联精确ForeignIdentityKey与NodeRef。initial_import、adopt、upsert以及retired/non-live情形继续逐项遵守D3 intent/state矩阵；不得仅凭SourceBinding、UID、title或path猜更新目标。无 binding 的只读 provider view 可作为明确能力，但不能满足承诺的可更新同步。有限 import/adopt selected components 根据 D3/D4/D9 获得 fresh Node 与明确 binding；不能把每个 recurrence 展开项自动物化。UID/RECURRENCE-ID 与 LogicalOccurrenceKey 均不变为 NodeRef。VFREEBUSY/VTIMEZONE 不创建 Node；VTODO/Task、VJOURNAL、VEVENT 保持各自显式映射。

比较结果：series-owned structured values 最好保持本地独立事实/修正与单一 series authority；独立 Record 增加另一身份域仍不能替代 foreign scope；Document occurrence 缺少 recurrence typed semantics；每次 Node 在无限序列不成立；纯派生无 override 源则不能保存作者例外。选择 series Node + typed rule/override + derived occurrence + 明确外部 binding 的组合。此裁决不自动赋予 D9/D10 尚未实现的同步、floating-time、URI fetch、credentials 或转换能力。

## 11. D5 意图、证据与阶段接口

D5 规定语义意图与结果，不扩张 D3 已封闭 Operation kind，也不提前冻结 D7 ActionSpec、D6 transaction wire。后续实现必须为下表每个意图提供 closed request/plan/error/receipt adapter，无法一一保留含义不得宣称完成 D5 实现。

| D5 意图 | 最少绑定 | 成功效果 | 拒绝/失效 |
| --- | --- | --- | --- |
| edit native table | owner NodeRef、Document revision、table locator、exact source ranges、每项具体变换 | 同一个 Document 的完整 valid proposed source；其他字节保留 | stale locator/revision、invalid/unrepresentable cell、越界、权限、D2/D4完整源校验失败 |
| edit Field occurrence | owner、FieldId、RegistryBinding、Document revision、occurrenceKey、expected raw Entry、显式 edit mode | D4完整 proposed-state/关系效果；保留其余source | stale/ambiguous、unavailable schema、constraint/auth/relation conflict |
| create through collection | definition locator/revision或ad-hoc Query binding、明确parent/title/source/ordinal、requireMembership、完整依赖 | fresh NodeRef、合法source/placement、必要成员证明 | parent/stale/权限、模板或schema冲突、membership未证、预算失败 |
| remove membership | 选中NodeRef、原Query及参数、保存定义locator/revision或ad-hoc Query binding、实际读取依赖、明确的作者事实变换或保存定义变换 | 完整post-query证明选中Node不再是结果成员；传输page不参与成员判断；执行所列source mutation且不隐式Trash | Query不可逆、无真实写目标、revision/权限冲突，或无法证明移出完整结果时拒绝；普通修改事实须另用明确意图，不得报告移出成功 |
| trash selected Nodes | 去重完整NodeRef target集、D3 closure及expected状态 | D3完整subtree/lifecycle/relations receipt | root、hidden/unauthorized依赖、stale、closure不完整 |
| import batch | 明确输入绑定、有限映射、title/parent/facts、数量/资源预算 | fresh Nodes与逐行mapping和D3/D9receipt | format/loss未裁决、schema错误、超限、stale/权限；该batch全无作者写入 |

任何表面不得把 row handle、column index、caption、调用者自报 before-image 或 cached decoded value 当写入授权。Core 对真实 source、authority、visibility、revision 与完整 read-set 重验证。拒绝不能泄露未授权对象存在性；预览不构成提交；超时/断网不能单靠 UI 猜 success。以上是复用 D1/D3/D4 的边界要求，其物理执行和 recovery 由 D6/D7 接口验收。

## 12. 完成条件

本候选须有独立 Chat GPT-6 Pro 对完整 D5 候选、上游约束、全部适用强制场景、术语与实现影响的明确判断。成立的领域反例和更优完整方案须裁决并整合；本地文件检查或场景表自身不能证明语义。只有 accept/pass、零未决P0/P1、完整required reading/semantic coverage与terminology pass后，才可形成 D5 决议/术语/Impact和协调控制验收。D6 及以后仍需另行授权。
