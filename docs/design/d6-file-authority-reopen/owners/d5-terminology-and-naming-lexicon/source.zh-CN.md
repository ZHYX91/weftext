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

## 4. D6-FA imported names

| name | owner | D5 use | 不是 |
|---|---|---|---|
| SourceVersion/2 | D6 | table/source生产版本与内层revision绑定 | row identity或当前观察token |
| SourceObservation/1 | D6 | 当前完整受保护source观察资格 | production SourceVersion或row identity |
| SourceVersionRef/1 | D6 | sourceToken选择完整当前Observation | 裸revision/hash/I连续性 |
| InputDescriptor/2 | D6 | sourceInputs[].observation承载当前Observation | D5 plan或row identity |
| CommitDomain/2 | D6 | 当前operation域/decision scope | collection identity |
| Frontier/2 | D6 | 已封存的因果/依赖前缀与当前证明截面 | 完整 Query/Registry/物化证明 |
| SemanticState/1 | D6 | ordinary-save semantic pending/complete轴 | save-protection mode或collection completeness |
| ConflictRecord | D6 | source/placement/lifecycle conflict | Record domain |
| NodeRef | D3 | collection member identity | collection membership fact |
| Field Value Occurrence | D4 | row-like Field editor source | table row |
| PreparedActionBinding future version | D7 | strong collection/bulk preparation | D5-owned token |

D5不把任何 imported name登记为 owned alias，也不新增持久 TableRowId、RecordRef、CollectionRef 或其它row/Record/Collection identity。

SourceVersion/2仍是生产版本/history：managed variant保留原有 entityRef、commitDomain、observationEpoch、revision、changeId，external variant保留原有 entityRef、commitDomain、observationEpoch、externalSequence。生产commitDomain可以不同于当前operation域。当前资格由完整 SourceObservation/1 提供：observerDomain必须等于operation CommitDomain，entityRef必须等于sourceVersion.entityRef，并携带对应sourceVersion及当前observationEpoch、fileObjectBinding、evidencePins；author-control、Registry、incidence依赖与cut也必须属于同一当前观察资格。InputDescriptor/2.sourceInputs[].observation实际承载该Observation，SourceVersionRef/1.sourceToken以 d6_source_observation/1 标记选择完整当前Observation，而不是裸production version、revision、hash、I cache或相同行文字。

watcher gap、replacement或discontinuous rematerialization会使旧sourceToken以及依赖该观察的D5 locator失效，即使production SourceVersion相同也不能续认；I不能恢复这种资格。SourceObservation/1只是外层当前观察保护，不替换D5/D4既有inner sourceRevision、OccurrenceKey、Entry selector或revision-bound locator wire。

Frontier/2仅表示已seal的causal/dependency prefix和相应proof cut，不单独证明complete Query、Registry完整性或payload物化。历史Frontier/1 decoder、旧saved bytes与其它历史恢复仍按其原版本解释，不机械替换为Frontier/2。

ordinary语义与strict|observed_only保存保护是独立两轴，ordinary可以使用strict；不要求无关完整index或Workspace Query并不产生weak资格。只有满足A §4.1全部条件的人工existing-live-Document整源保存，才能在planning开始前显式选择observed_only并冻结profile：trusted interactive_source_save、恰一个既有live Document、ordinary + replica_local、完整source read/replace、author write set为空或仅该Document、无applicable body/Field/Node-control deny、无identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation，并且DraftBase等于当前选定Observation。structured、bulk、collection、promotion、Automation、server checkpoint、Approval、Money及所有strong Action都不能使用weak保护；strict失败、已知冲突、授权、耐久或strong义务失败也不能fallback为weak。

observed_only的B/N耐久、未观察外部C可能被N安装覆盖、后续C可能再次替换current file、已观察competition或gap要求conflict/reprepare以及unknown install进入recovery_unknown，均继续由D6定义，D5不新增保存保证。semantic_pending(collection)只表示缺少collection全集proof，不表示empty，也不能把typed invalid、source invalid或缺少strong evidence洗成成功，更不能授权Action、all_result、bulk或Automation；后来r6的新证明只适用于r6及其当前Observation/SourceVersion/cut，不改写r5历史receipt。

ConflictRecord继续由D6拥有，NodeRef继续由D3拥有，Field Value Occurrence继续由D4拥有。future PreparedActionBinding继续由D7拥有；新版D7 Prepared尚未冻结时，strong collection/bulk入口仍为unavailable/owner_update_required，D5不创建替代token。
## 5. compatibility 与 Gate

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
