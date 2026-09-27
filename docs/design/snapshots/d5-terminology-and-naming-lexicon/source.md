---
_weftext:
  id: "295b1806-1a36-4c87-80c6-8c5aee5e21e7"
---

# D5 Terminology and Naming Lexicon

接受证据：D5总控验收（外部控制记录未随本输入发布）；Pro完整原报告（外部控制记录未随本输入发布）；四项P2修正与逐字差异（外部控制记录未随本输入发布）。正文保留完整接受设计及其语义；候选称谓、完成条件或首次命名版本反映其来源，不重启已完成评审。此权威与控制索引须在协调切换journal（外部控制记录未随本输入发布）为committed后一起生效，准备中或已回滚整批不生效。

状态：已冻结的D5架构权威，revision02-editorial-01。D5已冻结revision02-editorial-01：独立Chat GPT-6 Pro对revision01明确accept/pass、P0=0/P1=0，完整阅读/语义覆盖及术语通过；四项非阻断P2已由总控作保持既有语义的编辑修正，未新增评审轮次。 本文不证明产品实现或性能；实际生效以同批committed切换journal为准。

继承完整 D3 与 D4 Lexicon，不改写既有 term IDs、wire 或非身份边界。下表补全 D2 已有表格/节点集合概念的受控命名，并为 D5 创建策略与成员移除提供精确映射。

每行依次定义 stable concept ID、中文/英文、定义及 owner/layer、排除、wire/API/code、CLI/UI/locale、简称、退役/迁移与实例。名称检查只用于声明为受控的标识符、schema/API/locale 和操作标签，不对用户内容、第三方格式、普通 prose、历史报告、反例或本文件退役清单执行词汇禁令。

| Stable ID | 正式中英文 | 定义、owner/layer | 排除 | Wire/API/code 与变量 | CLI/UI、locale、简称 | 迁移/例子 |
| --- | --- | --- | --- | --- | --- | --- |
| `weftext.term.document-table` | 文档表格 / Document Table | D2 Document 内的原生 AsciiDoc table occurrence；D5定义其显式结构编辑 | 非 RecordCollection、NodeCollectionResult、数据库、独立 owner | 既有 D2 `table` kind；`DocumentTable`、`document_table`；不新增 durable ref | 文档上下文 UI“表格”、CLI `document table`、`term.documentTable`；仅该上下文简称 table | 首次命名候选D5 v1；旧“表格数据库”改回真实域。例：`|===`包围的source；CSV Resource不是它。 |
| `weftext.term.document-table-row` | 文档表格行 / Document Table Row | D2 table内一个当前revision的row occurrence | 非独立 Record、Node、持久row key | 既有 `table_row`；`DocumentTableRow`、`document_table_row`；current locator/ordinal | UI“表格行”、CLI `document table row`、`term.documentTableRow`；table明确时 row | D5 v1；删除 `TableRowId` durable用法。例：两个相同文本row仍是两个source occurrences；重排不产生EntityRef。 |
| `weftext.term.document-table-cell` | 文档表格单元格 / Document Table Cell | D2 row内Inline* cell occurrence | 非D4 Field、schema列、单独content identity | 既有 `table_cell`；`DocumentTableCell`、`document_table_cell` | UI“单元格”、CLI `document table cell`、`term.documentTableCell`；table明确时 cell | D5 v1；删除把cell位置称FieldId的用法。例：文本`0012`不是推断integer。 |
| `weftext.term.node-collection` | 节点集合 / Node Collection | D2定义的去重live authorized NodeRef求值结果；D7求值/视图 | 非structural parent、RecordCollection、保存定义identity、持久membership owner | 既有 `NodeCollectionResult`；`node_collection_result`；不新增collectionRef/ViewRef | UI“节点集合”、CLI `node collection`、`term.nodeCollection`；无裸“记录集”简称 | D5 v1；持久化在saved Query/View occurrence的定义中。例：同一Person属于两个结果；删除定义不删Person。 |
| `weftext.term.collection-creation-policy` | 集合创建策略 / Collection Creation Policy | 保存定义内明确的parent/source/default构造意图；D5语义、D7/D9 payload实施 | 非membership谓词、持续Template authority、parent owner | `CollectionCreationPolicy`、`collection_creation_policy`；`destinationParent`、`requireMembership`是相应intent成员；最终D7wire仍待其阶段冻结 | UI“新节点位置与默认值”、CLI `collection creation policy`、`term.collectionCreationPolicy`；明确时创建策略 | D5 v1；不从排序邻居猜目录。例：显式parent优先；失效parent拒绝，不回退root。 |
| `weftext.term.collection-membership-removal` | 移出节点集合 / Remove from Node Collection | 对明确作者事实或保存定义执行变更，使选中Node不再是结果成员 | 非Trash Node、删除文档行、删除Field occurrence的无条件别名 | `RemoveFromNodeCollectionIntent`、`remove_from_node_collection_intent`；只是D5语义意图，不是新增D3operation kind | UI“移出节点集合”、CLI `node collection remove`、`action.nodeCollectionRemove`；无模糊delete别名 | D5 v1；任意filter无法反演则不可用。例：移出手工refs selector只改定义；Trash须另选并展示subtree。 |

以上新英文/中文名称与既有D3/D4精确受控名称的任何碰撞都必须在冻结前消除。`table/table_row/table_cell`和`NodeCollectionResult`是为已有D2语义提供term归属，不修改其wire形状；没有新union discriminator。

## 退役与语义非同义词

| 旧受管 Record 标识或说法 | disposition | 完整目标 |
| --- | --- | --- |
| `RecordRef`, `RecordCollectionRef`, `RecordId`, `RecordCollectionId`, `RecordSet`, `record_ref`, `record_collection_ref`, `record_set` | retire as independent managed-domain names | 删除domain、decoder、params、scan、schema、equality、provenance、View/Action、导入导出专用分支；不能alias/coerce到NodeRef。 |
| `recordCollection(...)`, `records` Query domain, `row.record.fields` | retire managed-domain branches | D7按D2/D3/D4/D5重建明确Node或非身份值/occurrence入口。 |
| 文档表格行=Record、Field occurrence=entity、节点集合=folder/owner | split/reject | 使用本表和D3/D4已有概念；view改变不改变真实域。 |
| 数据结构语言的record/object、第三方CSV record、历史讨论“记录集” | keep-qualified | 不是独立受管Record，不执行全局文字替换，不禁止普通用户文字。 |

## 横切碰撞矩阵

1. Profile：keep-qualified；Weftext AsciiDoc Profile属于D2，Facet是Node membership；D5不恢复裸Profile。
2. type/class/schema/facet/role：split；D4 ValueType/Field/Facet与relationship role保持各自定义；视图列不是schema authority。
3. plugin/module/pack/provider：keep-qualified；D5没有私有记录provider或新产品表面，D10冻结机制。
4. Template/default/export template：split；一次性创建、明确默认值与Office输出绑定不同，均不接管实例。
5. Node/Document/Record/occurrence：split + retire独立Record；本文件行域表适用，phone不是Node。
6. attribute/field/metadata：keep-qualified；D2 header无自动D4类型，table cell不是Field。
7. relation/link/reference/citation：keep-qualified；inverse编辑写原事实，literal target无edge，row link不成为RecordRef。
8. Resource/attachment/file：keep-qualified；CSV/XLSX附件只是opaque Resource，解析行非作者域。
9. Calendar/event/period/journal：keep-qualified；series、derived occurrence、scholarly journal与Diary不混同。
10. calendar system/view/source：split；规则、派生布局、外部authority/binding不同。
11. assertion/state/observation/repeatable/relation：keep-qualified；共享row editor不抹去D4类型含义。
12. occurrence key/locator/ref：split；value内当前revision selector、Document locator、durable NodeRef彼此不可coerce；字段永久Annotation target仍须上游重开。

这些裁决是当前候选的术语判断，不以表格存在代替独立terminology gate。R0实现须对受控token/AST/locale做正反例：合法`NodeCollectionResult`、`DocumentTableRow`保持归属；旧RecordRef discriminator拒绝；历史Record讨论和用户文本不误报；D3 NodeRef/FieldId/occurrenceKey原有归属不变。
