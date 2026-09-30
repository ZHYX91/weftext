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
| SourceVersion/2 | D6 | current table/source binding | row identity |
| CommitDomain/2 | D6 | local/managed scope | collection identity |
| Frontier/1 | D6 | complete cut/batch dependency | table revision |
| SemanticState/1 | D6 | ordinary save pending/complete | collection completeness |
| ConflictRecord | D6 | source/placement/lifecycle conflict | Record domain |
| NodeRef | D3 | collection member identity | collection membership fact |
| Field Value Occurrence | D4 | row-like Field editor source | table row |
| PreparedActionBinding future version | D7 | strong collection/bulk preparation | D5-owned token |

D5不把任何 imported name登记为 owned alias。

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
