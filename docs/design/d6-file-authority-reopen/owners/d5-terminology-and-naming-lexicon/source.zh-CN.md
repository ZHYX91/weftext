---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：295b1806-1a36-4c87-80c6-8c5aee5e21e7。

# D5 Terminology and Naming Lexicon — D6-FA-r01

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 的 D5 六个 concept 全部保留 conceptId、owner、owned wire/code/UI/locale names 与 firstFreeze；本批不新增 Record/RecordCollection 术语域。

## 1. 控制规则

D5 只拥有 Document Table 与 Node Collection 的结构语义术语。D4 Field Value Occurrence、
  D3 NodeRef/parent/lifecycle、D6 SourceVersion/ConflictRecord、D7 Query/Action/Prepared均保持原 owner。

“row / 行”必须带域：Document Table Row、Node Collection result row、外部 import row 不互为 identity。UI表格布局不能让一个概念取得另一个域的名字。

这些限制适用于受控产品 UI、API/wire、代码标识符、CLI 和 locale key；不禁止用户正文、第三方格式里的 record 用语，或明确标出的历史及反例讨论。这些内容不登记为 Weftext 身份别名。

## 2. 完整受控 concepts

| conceptId | 正式名 | owner/layer | 定义与排除 | owned wire/code | UI/locale | firstFreeze |
|---|---|---|---|---|---|---|
| `weftext.term.document-table` | 文档表格 / Document Table | D2 occurrence + D5 structure | D2 Document 内的 native AsciiDoc table occurrence；不是 RecordCollection、NodeCollectionResult、数据库或独立 owner。 | existing D2 `table`; `DocumentTable`, `document_table`; no durable ref | UI“表格”、CLI `document table`, `term.documentTable`; table仅在Document上下文简称 | D5 v1 |
| `weftext.term.document-table-row` | 文档表格行 / Document Table Row | D2 出现项 + D5 结构 | 当前 revision 的表格行出现项；不是 Record、Node 或持久 row key。 | 既有 `table_row`; `DocumentTableRow`, `document_table_row`; 当前 locator/ordinal | UI“表格行”、CLI `document table row`, `term.documentTableRow` | D5 v1 |
| `weftext.term.document-table-cell` | 文档表格单元格 / Document Table Cell | D2 occurrence + D5 structure | row内 Inline* cell occurrence；不是 D4 Field、schema column 或独立 identity。 | existing `table_cell`; `DocumentTableCell`, `document_table_cell` | UI“单元格”、CLI `document table cell`, `term.documentTableCell` | D5 v1 |
| `weftext.term.node-collection` | 节点集合 / Node Collection | D5/D7 result semantics | D2定义的去重 live authorized NodeRef求值结果；不是 structural parent、RecordCollection、saved definition identity 或 persistent membership owner。 | existing `NodeCollectionResult`; `node_collection_result`; no CollectionRef/ViewRef | UI“节点集合”、CLI `node collection`, `term.nodeCollection` | D5 v1 |
| `weftext.term.collection-creation-policy` | 集合创建策略 / Collection Creation Policy | D5 semantics; D7/D9 payload consumers | saved definition 中明确 parent/source/default 构造意图；不是 membership predicate、persistent Template authority 或 parent owner。 | `CollectionCreationPolicy`, `collection_creation_policy`; intent members `destinationParent`, `requireMembership` | UI“新节点位置与默认值”、CLI `collection creation policy`, `term.collectionCreationPolicy` | D5 v1 |
| `weftext.term.collection-membership-removal` | 移出节点集合 / Remove from Node Collection | D5 semantic intent | 对明确作者事实/保存定义执行变更使Node不再是结果成员；不是 Trash Node、delete row或 delete Field occurrence 的通用别名。 | `RemoveFromNodeCollectionIntent`, `remove_from_node_collection_intent`; not a D3 operation kind | UI“移出节点集合”、CLI `node collection remove`, `action.nodeCollectionRemove` | D5 v1 |

## 3. 受控非同义词

- Document Table ≠ database table ≠ RecordCollection。
- Document Table Row ≠ Record ≠ Node。
- Document Table Cell ≠ D4 Field/Field Value Occurrence。
- Node Collection ≠ folder/parent ≠ saved membership list。
- collection creation policy ≠ Template identity/parent owner。
- remove-from-collection ≠ Trash ≠ purge ≠ delete table row。
- result page ≠ collection membership。
- Query `take` 是 semantic membership限制；transport page size不是。
- row number/ordinal/locator都不是 durable identity。

### 3.1 完整退役 Record 分支

退役 active 受控名称 RecordRef、RecordCollectionRef、RecordId、RecordCollectionId、RecordSet、record_ref、record_collection_ref、record_set，以及 recordCollection(...)、records Query 域和 row.record.fields。其 active decoder、参数、scan、schema、equality、group、distinct、cache、export、provenance、View Action、import 分支及专用权限和 capability 必须成套删除；只删 selector 标签而保留执行域不算退役。任何一个都不能成为 NodeRef 别名或强制转换。真实 saved/planned/unknown 记录恢复所需的原 decoder 和 bytes 继续承担原义务；只有原型或 decoder 存在，不证明曾经部署。

### 3.2 跨阶段易混名称矩阵

| 易混表面 | 必须区分的含义 |
|---|---|
| Profile | D2 WeftextAsciiDocProfile 用于语法准入，不是 D4 Facet |
| type/class/schema/facet/role | D4 保持各自 schema、语义和 Facet 含义；显示列不是类型声明 |
| plugin/module/pack/provider | 明确实际 D10 contribution、runtime 或 provider 域，不能笼统当作可互换 module |
| Template/default/export template | 绑定 revision 的一次性构造、初始默认值和 Office 导出映射分别处理 |
| Node/Document/Record/occurrence | 持久 Node 身份、所属 Document、已退役 Record 域及无身份出现项分别处理 |
| attribute/field/metadata | D2 头部 attribute 不是 D4 Field；表格单元格也不是 Field 或可携带 control 事实 |
| relation/link/ref/citation | canonical 关系事实与逆向展示只对应一份作者事实；literal 不产生实体边，行链接不创建 RecordRef |
| Resource/attachment/file | CSV/XLSX 可以是 Resource 的不透明 bytes；预览行不取得新作者身份 |
| Calendar/event/period/journal | series、派生出现、学术期刊和 Diary 属于不同域 |
| calendar system/view/source | 日期规则、展示布局和外部来源权威分别处理 |
| assertion/state/observation/repeatable/relation | 共用编辑器不合并 D4 语义，也不覆盖历史 |
| occurrenceKey/Locator/Ref | 当前值 selector、绑定 revision 的 Document locator 与持久实体引用分别处理；永久 Field 出现项 Annotation 必须显式重开上游设计 |


## 4. D6-FA imported names

| name | owner | D5 use | 不是 |
|---|---|---|---|
| SourceVersion/2 | D6 | 完整 managed/external 生产版本；仅 managed 有内层整数 | row identity或当前观察token |
| SourceObservation/1 | D6 | 当前完整受保护source观察资格 | production SourceVersion或row identity |
| SourceVersionRef/1 | D6 | sourceToken选择完整当前Observation | 裸revision/hash/I连续性 |
| InputDescriptor/2 | D6 | sourceInputs[].observation承载当前Observation | D5 plan或row identity |
| CommitDomain/2 | D6 | 当前operation域/decision scope | collection identity |
| Frontier/2 | D6 | 已封存的因果/依赖前缀与当前证明截面 | 完整 Query/Registry/物化证明 |
| SemanticState/1 | D6 | ordinary-save semantic pending/complete轴 | save-protection mode或collection completeness |
| ConflictRecord | D6 | source/placement/lifecycle conflict | Record domain |
| NodeRef | D3 | collection member identity | collection membership fact |
| Field Value Occurrence | D4 | row-like Field editor source | table row |
| PreparedActionBinding | D7 | 实际版本的强准备与原记录恢复 | D5-owned token |
| RevisionTokenBinding/2 + RevisionTokenSource/2 | D6 | 受保护不透明 token 作为到 tagged managed stamp 或 external version 的稳定生产地址绑定；当前 observer 资格另由 SourceObservation/1 证明 | D4 整数、权限、运行时 selector 证据或第二套 portable 重资格 token family |
| SourceRevisionPlan/1 + SourceStamp/1 | D6 | 原计划冻结的拟议 managed-after 基础 | 已 seal 的当前 source 或 D5 独立分配器 |
| DependencyProof/2 + DependencyKey/2 | D6 与实际范围 owner | 十四种闭合 key 和九种 placement StructureRange | partial index 或猜出的完整空范围 |
| ContentCompletionProof/3 | D6 | 具有真实生产 before/after 的已 seal 可携带效果 | sender 当前观察 token 或新的执行权限 |

D5不把任何 imported name登记为 owned alias，也不新增持久 TableRowId、RecordRef、CollectionRef 或其它row/Record/Collection identity。

SourceVersion/2仍是生产版本/history：managed variant保留原有 entityRef、commitDomain、observationEpoch、revision、changeId，external variant保留原有 entityRef、commitDomain、observationEpoch、externalSequence。生产commitDomain可以不同于当前operation域。当前资格由完整 SourceObservation/1 提供：observerDomain必须等于operation CommitDomain，entityRef必须等于sourceVersion.entityRef，并携带对应sourceVersion及当前observationEpoch、fileObjectBinding、evidencePins；author-control、Registry、incidence依赖与cut也必须属于同一当前观察资格。InputDescriptor/2.sourceInputs[].observation实际承载该Observation，SourceVersionRef/1.sourceToken以 d6_source_observation/1 标记选择完整当前Observation，而不是裸production version、revision、hash、I cache或相同行文字。

watcher gap、replacement 或 discontinuous rematerialization 会使旧 sourceToken 及依赖该 Observation 的 D5 runtime selector/preparation 失效，即使 production SourceVersion 相同也一样。managed persistent Locator 仍只是稳定生产地址，只能通过 canonical binding、新 current Observation 与原 coordinates 取得一次新的读取资格；I 既不能恢复地址真实性，也不能恢复当前资格。SourceObservation/1只是外层当前观察保护，不替换D5/D4既有inner sourceRevision、OccurrenceKey、Entry selector或revision-bound locator wire。

D3/D5 不透明 Locator revision token 通过已认证 canonical RevisionTokenBinding/2 与 tagged RevisionTokenSource/2 解析为稳定生产地址。managed source 的唯一 canonical token 由原 winning plan/seal 与 sealed-outbox 关联建立；一次新读取再独立证明 current SourceObservation/1 的完整 sourceVersion 与该地址逐字相等，之后才使用准确 table/range 坐标。watcher gap 会使旧 sourceToken/runtime selector 失效，但不会 mint/改写稳定地址；新读取成功也不复活旧 structured operation。D4 数字内层 sourceRevision 仍须真实 managed 生产 revision，externalSequence 永不提供该整数。external persistent position 还需原 external-event 证据与当前 exact external SourceVersion Observation；普通 raw/read/repair/Draft 保留其合法 external-source 路径。

H(D,E) 是 D6 连续封存的生产域历史，不是新的 D5 counter。新 managed after 消费同一计划的 SourceRevisionPlan 和检查式 H+1；epoch 不重置 H，外域 revision/externalSequence 不捐值。真正 raw no-op 及源码未变 owner 保留完整旧基础，不新增 source version/H；相同 bytes 的 external admission 则产生真实 managed after。可携带效果与当前 Observation 仍分别处理。

owner 使用 exact 时 Frontier 必须相等。允许 scope_dependencies 的路径，只有在扩展连续封存、已验证且与原依赖无关，原 observations、pins、control/auth/Registry 及完整正负依赖不变，并在 P 保留证明时，才可沿用原计划；原基线和 token 都不改写。D3 managed_atomic 继续 exact，D7 完整结果重置规则不放宽。

Frontier/2仅表示已seal的causal/dependency prefix和相应proof cut，不单独证明complete Query、Registry完整性或payload物化。历史Frontier/1 decoder、旧saved bytes与其它历史恢复仍按其原版本解释，不机械替换为Frontier/2。

ordinary语义与strict|observed_only保存保护是独立两轴，ordinary可以使用strict；不要求无关完整index或Workspace Query并不产生weak资格。只有满足A §4.1全部条件的人工existing-live-Document整源保存，才能在planning开始前显式选择observed_only并冻结profile：trusted interactive_source_save、恰一个既有live Document、ordinary + replica_local、完整source read/replace、author write set为空或仅该Document、无applicable body/Field/Node-control deny、无identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation，并且DraftBase等于当前选定Observation。structured、bulk、collection、promotion、Automation、server checkpoint、Approval、Money及所有strong Action都不能使用weak保护；strict失败、已知冲突、授权、耐久或strong义务失败也不能fallback为weak。

observed_only的B/N耐久、未观察外部C可能被N安装覆盖、后续C可能再次替换current file、已观察competition或gap要求conflict/reprepare以及unknown install进入recovery_unknown，均继续由D6定义，D5不新增保存保证。semantic_pending(collection)只表示缺少collection全集proof，不表示empty，也不能把typed invalid、source invalid或缺少strong evidence洗成成功，更不能授权Action、all_result、bulk或Automation；后来r6的新证明只适用于r6及其当前Observation/SourceVersion/cut，不改写r5历史receipt。

ConflictRecord 仍归 D6，NodeRef 归 D3，Field Value Occurrence 归 D4，PreparedActionBinding 归 D7。PL-IR-01 现已有 D6 稳定地址 + D3 fresh-current-Observation 作者修复，等待本 exact candidate 的独立复核，而不再是 owner 算法未定义依赖。另一项 current complete-result/preparation/preview producer 协调仍开放。D5 不自造合同、不重签旧 runtime evidence；saved/planned/unknown 继续先于 unseen 当前业务门恢复。

必须机械证明 fixed-S 六个 conceptId、owner、owned wire/code/UI/locale names、firstFreeze逐项保持。不存在 `TableRowId|RecordRef|CollectionRef|ViewRef` 新 public identifier。

后续 D7/D8/D9如新增 projection/batch/import名称，由其真实owner登记，不回填D5概念表。

旧 D10 B13仍 REVISE、术语/双语 FAIL、11 OPEN；本文件不关闭任何finding。

## 6. fixed-S 受控表面 exact snapshot

以下 JSON 只作为 fixed-S 受控表面的机械 exact-preservation inventory；语义定义仍以上文 owner 条目为准。任何字段差异都使术语 Gate 失败，不能由实现自动迁移。

~~~json
{
  "format": "weftext.controlled-surface-snapshot",
  "version": 1,
  "sourceCommit": "7e18168dad3e6d120fce0dd607dc10fa7894e252",
  "entries": [
    {
      "conceptId": "weftext.term.document-table",
      "definitionOwnerLayer": "D2 Document 内的原生 AsciiDoc table occurrence；D5定义其显式结构编辑",
      "ownedWireApiCode": "既有 D2 `table` kind；`DocumentTable`、`document_table`；不新增 durable ref",
      "uiLocaleShort": "文档上下文 UI“表格”、CLI `document table`、`term.documentTable`；仅该上下文简称 table",
      "migrationExamples": "首次命名候选D5 v1；旧“表格数据库”改回真实域。例：`|===`包围的source；CSV Resource不是它。",
      "firstFreeze": "D5 v1"
    },
    {
      "conceptId": "weftext.term.document-table-row",
      "definitionOwnerLayer": "D2 table内一个当前revision的row occurrence",
      "ownedWireApiCode": "既有 `table_row`；`DocumentTableRow`、`document_table_row`；current locator/ordinal",
      "uiLocaleShort": "UI“表格行”、CLI `document table row`、`term.documentTableRow`；table明确时 row",
      "migrationExamples": "D5 v1；删除 `TableRowId` durable用法。例：两个相同文本row仍是两个source occurrences；重排不产生EntityRef。",
      "firstFreeze": "D5 v1"
    },
    {
      "conceptId": "weftext.term.document-table-cell",
      "definitionOwnerLayer": "D2 row内Inline* cell occurrence",
      "ownedWireApiCode": "既有 `table_cell`；`DocumentTableCell`、`document_table_cell`",
      "uiLocaleShort": "UI“单元格”、CLI `document table cell`、`term.documentTableCell`；table明确时 cell",
      "migrationExamples": "D5 v1；删除把cell位置称FieldId的用法。例：文本`0012`不是推断integer。",
      "firstFreeze": "D5 v1"
    },
    {
      "conceptId": "weftext.term.node-collection",
      "definitionOwnerLayer": "D2定义的去重live authorized NodeRef求值结果；D7求值/视图",
      "ownedWireApiCode": "既有 `NodeCollectionResult`；`node_collection_result`；不新增collectionRef/ViewRef",
      "uiLocaleShort": "UI“节点集合”、CLI `node collection`、`term.nodeCollection`；无裸“记录集”简称",
      "migrationExamples": "D5 v1；持久化在saved Query/View occurrence的定义中。例：同一Person属于两个结果；删除定义不删Person。",
      "firstFreeze": "D5 v1"
    },
    {
      "conceptId": "weftext.term.collection-creation-policy",
      "definitionOwnerLayer": "保存定义内明确的parent/source/default构造意图；D5语义、D7/D9 payload实施",
      "ownedWireApiCode": "`CollectionCreationPolicy`、`collection_creation_policy`；`destinationParent`、`requireMembership`是相应intent成员；最终D7wire仍待其阶段冻结",
      "uiLocaleShort": "UI“新节点位置与默认值”、CLI `collection creation policy`、`term.collectionCreationPolicy`；明确时创建策略",
      "migrationExamples": "D5 v1；不从排序邻居猜目录。例：显式parent优先；失效parent拒绝，不回退root。",
      "firstFreeze": "D5 v1"
    },
    {
      "conceptId": "weftext.term.collection-membership-removal",
      "definitionOwnerLayer": "对明确作者事实或保存定义执行变更，使选中Node不再是结果成员",
      "ownedWireApiCode": "`RemoveFromNodeCollectionIntent`、`remove_from_node_collection_intent`；只是D5语义意图，不是新增D3operation kind",
      "uiLocaleShort": "UI“移出节点集合”、CLI `node collection remove`、`action.nodeCollectionRemove`；无模糊delete别名",
      "migrationExamples": "D5 v1；任意filter无法反演则不可用。例：移出手工refs selector只改定义；Trash须另选并展示subtree。",
      "firstFreeze": "D5 v1"
    }
  ]
}
~~~
