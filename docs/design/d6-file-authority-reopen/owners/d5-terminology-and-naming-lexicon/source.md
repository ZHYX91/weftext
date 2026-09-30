---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 295b1806-1a36-4c87-80c6-8c5aee5e21e7.

# D5 Terminology and Naming Lexicon — D6-FA-r01

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. All six fixed-S D5 concepts preserve conceptId, owner, owned wire/code/UI/locale names, and firstFreeze. This batch introduces no Record/RecordCollection terminology domain.

## 1. Control rules

D5 owns structural terminology for Document Table and Node Collection only. D4 Field Value Occurrence, D3 NodeRef/parent/lifecycle, D6 SourceVersion/ConflictRecord, and D7 Query/Action/Prepared remain owned elsewhere.

The bare word row is always qualified by domain: Document Table Row, Node Collection result row, or external import row are not identity-equivalent. A tabular UI never transfers terminology ownership across those domains.

## 2. Complete controlled concepts

| conceptId | canonical names | owner/layer | definition and exclusion | owned wire/code | UI/locale | firstFreeze |
|---|---|---|---|---|---|---|
| `weftext.term.document-table` | 文档表格 / Document Table | D2 occurrence + D5 structure | native AsciiDoc table occurrence inside a D2 Document; not RecordCollection, NodeCollectionResult, database, or independent owner | existing D2 `table`; `DocumentTable`, `document_table`; no durable ref | UI 表格, CLI `document table`, `term.documentTable` | D5 v1 |
| `weftext.term.document-table-row` | 文档表格行 / Document Table Row | D2 occurrence + D5 structure | current-revision row occurrence; not Record, Node, or durable row key | existing `table_row`; `DocumentTableRow`, `document_table_row`; current locator/ordinal | UI 表格行, CLI `document table row`, `term.documentTableRow` | D5 v1 |
| `weftext.term.document-table-cell` | 文档表格单元格 / Document Table Cell | D2 occurrence + D5 structure | Inline* cell occurrence in a row; not D4 Field, schema column, or independent identity | existing `table_cell`; `DocumentTableCell`, `document_table_cell` | UI 单元格, CLI `document table cell`, `term.documentTableCell` | D5 v1 |
| `weftext.term.node-collection` | 节点集合 / Node Collection | D5/D7 result semantics | deduplicated live authorized NodeRef result defined by D2/D7; not structural parent, RecordCollection, saved-definition identity, or persistent membership owner | existing `NodeCollectionResult`; `node_collection_result`; no CollectionRef/ViewRef | UI 节点集合, CLI `node collection`, `term.nodeCollection` | D5 v1 |
| `weftext.term.collection-creation-policy` | 集合创建策略 / Collection Creation Policy | D5 semantics; D7/D9 payload consumers | explicit parent/source/default construction intent in a saved definition; not membership predicate, persistent Template authority, or parent owner | `CollectionCreationPolicy`, `collection_creation_policy`; intent members `destinationParent`, `requireMembership` | UI 新节点位置与默认值, CLI `collection creation policy`, `term.collectionCreationPolicy` | D5 v1 |
| `weftext.term.collection-membership-removal` | 移出节点集合 / Remove from Node Collection | D5 semantic intent | changes explicit author fact/saved definition so a Node leaves the result; not generic alias for Trash, row deletion, or Field-occurrence deletion | `RemoveFromNodeCollectionIntent`, `remove_from_node_collection_intent`; not D3 operation kind | UI 移出节点集合, CLI `node collection remove`, `action.nodeCollectionRemove` | D5 v1 |

## 3. Controlled non-aliases

- Document Table is not database table or RecordCollection.
- Document Table Row is not Record or Node.
- Document Table Cell is not D4 Field/Field Value Occurrence.
- Node Collection is not folder/parent or stored membership list.
- Collection Creation Policy is not Template identity or parent owner.
- Remove from Node Collection is not Trash, purge, or table-row deletion.
- Result page is not collection membership.
- Query `take` is semantic membership limiting; transport page size is not.
- row number/ordinal/locator is never durable identity.

## 4. D6-FA imported names

| name | owner | D5 use | not |
|---|---|---|---|
| SourceVersion/2 | D6 | current table/source binding | row identity |
| CommitDomain/2 | D6 | local/managed scope | collection identity |
| Frontier/1 | D6 | complete cut/batch dependency | table revision |
| SemanticState/1 | D6 | ordinary-save pending/complete | collection completeness |
| ConflictRecord | D6 | source/placement/lifecycle conflict | Record domain |
| NodeRef | D3 | collection-member identity | collection membership fact |
| Field Value Occurrence | D4 | source for row-like Field editor | table row |
| future PreparedActionBinding | D7 | strong collection/bulk preparation | D5-owned token |

D5 registers none of these imported names as owned aliases.

## 5. Compatibility and Gate

Mechanically prove exact preservation of all six fixed-S conceptIds, owners, owned wire/code/UI/locale names, and firstFreeze values. There is no new public `TableRowId|RecordRef|CollectionRef|ViewRef`.

Later D7/D8/D9 projection/batch/import names are registered by their actual owners.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings. This file closes none.

## 6. fixed-S controlled-surface exact snapshot

The JSON below is only the mechanical exact-preservation inventory for fixed-S controlled surfaces; semantic definitions remain the owner entries above. Any field drift fails the terminology Gate and is never auto-migrated by implementation.

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
