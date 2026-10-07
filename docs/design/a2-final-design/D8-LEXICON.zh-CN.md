---
source_language: zh-CN
translation_status: source
---

[English](D8-LEXICON.md)

# A2 D8 术语与命名

状态：author-resolved-pending-independent-review。固定 S 的九个 D8 concept identity 继续保留。D10 R08 提供的 technical-interface-owner 元数据仅作为当前协调输入；真正的 current schema version 仍由现任 D8 owner 决定。Prepared Edit Binding 当前是 /3，不能把旧 /2 提案重新解释为 current。

## 1. 九个 D8 domain concept

| Concept ID | 中文 / English | 当前 owner/type | 明确排除 |
| --- | --- | --- | --- |
| weftext.term.edit_session | 编辑会话 / Edit Session | D8 的 UI 状态；类型为 EditSession | 不是 Workspace/Document identity，也没有 commit authority |
| weftext.term.edit_draft | 编辑草稿 / Edit Draft | D8 proposal；Draft + draftSerial | 不是 author Source，也不是 source revision；本行只定义当前命名边界，不新增身份、权限或执行语义 |
| weftext.term.draft_projection | 草稿投影 / Draft Projection | Core D8 projection；d8_draft_projection | 不是 D2 Snapshot/payload，也不是 Locator；本行只定义当前命名边界，不新增身份、权限或执行语义 |
| weftext.term.draft_edit_map | 草稿编辑映射 / Draft Edit Map | 由 flows、plainRegions、sites 组成 | 不是持久段落 identity、Locator 或 client parser |
| weftext.term.prepared_edit_binding | 编辑准备绑定 / Prepared Edit Binding | 受保护的 PreparedEditBinding/3 | 不是 D7 PreparedActionBinding，也不是第三套 ledger |
| weftext.term.composition_transaction | 组合输入事务 / Composition Transaction | D8 host 的 IME 状态 | 不是 D6 author transaction，也不是 receipt |
| weftext.term.caret_affinity | 光标亲和位置 / Caret Affinity | upstream/downstream | 不是 source offset，也不是 Locator |
| weftext.term.layout_epoch | 布局代 / Layout Epoch | 可丢弃的 layout generation | 不是 result epoch、auth generation 或 source revision；本行只定义当前命名边界，不新增身份、权限或执行语义 |
| weftext.term.direction_preference | 方向偏好 / Direction Preference | 设备/会话的 ltr、rtl、auto 偏好 | 不是 locale、portable author direction 或 Query order；本行只定义当前命名边界，不新增身份、权限或执行语义 |

不得新增第十个 D8 domain concept，例如 Record、Row、DocumentRef 或 SearchUIState。D2 Snapshot、D3 Value、D6 SourceObservation 与 D7 Query/View/Action 继续属于原 owner；D8 UI 只是消费这些数据，不会改变 owner。

## 2. 十三个 kind 与唯一 technical interface owner

| kind | technical interface owner | 消费或操作的对象 | 返回值与 owner 边界 |
| --- | --- | --- | --- |
| d8_document_read | D8 Document Read Interface | NodeRef + 当前 D6 read qualification | 返回 d8_document |
| d8_document | D8 Document Read Interface | 精确 read result | 返回 Snapshot/3 envelope |
| d8_draft_project | D8 Draft Projection Interface | Draft proposal + 当前 cut | 返回 d8_draft_projection |
| d8_draft_projection | D8 Draft Projection Interface | 单个 proposal | 返回 projection + Draft Edit Map |
| d8_draft_text_replace | D8 Draft Text Replace Interface | Draft + 一个已证明的 scalar segment | 返回 d8_draft_text_replaced；本行只定义当前命名边界，不新增身份、权限或执行语义 |
| d8_draft_text_replaced | D8 Draft Text Replace Interface | 精确 replace result | 返回完整 projection；没有隐式 caret |
| d8_draft_write | D8 Draft Write Interface | Draft + region/site mapping | 返回 d8_draft_written |
| d8_draft_written | D8 Draft Write Interface | 精确 command result | 返回 projection + caret |
| d8_edit_prepare | D8 Edit Prepare Interface | D8EditPrepareRequest/3 | 返回 d8_edit_prepared |
| d8_edit_prepared | D8 Edit Prepare Interface | PreparedEditBinding/3 的结果 | 返回 original request + preview；不声称 commit |
| d8_undo_prepare | D8 Undo Prepare Interface | original decision/effects + 精确 current after | 为 fresh inverse 返回 d8_edit_prepared；本行只定义当前命名边界，不新增身份、权限或执行语义 |
| d8_editor_error | D8 Error Interface | D8 public entry failure | 返回 closed D8 owner error |
| d8_prepared_edit_binding | D8 Prepared Edit Binding Internal Interface | 不可变的 /3 preparation state | 仅受保护内部使用；从不成为 public response kind |

多个 kind 可以共享一个 interface owner。Draft Edit Map 等 data concept 只是被接口消费的数据，不与接口共同拥有 kind。document|annotation 只是 D8 intent discriminator，不是 EntityKind。

current successor 还包含由 D8 owner 管理的 Annotation read/draft-open 与 presentation-policy kind；这些接口不会产生新的 domain concept identity：

- d8_annotation_read；
- d8_annotation_draft_open；
- d8_presentation_policy_set；
- d8_presentation_policy_no_change / d8_presentation_policy_prepared；
- internal presentation-policy record/head/outbox kinds。

## 3. Mode/status vocabulary

Source / Write / Read 是 mode，不是三份 Document。
readonly 表示 current interaction permission，不是第四份 content copy。
Draft saved locally、pending submission、planned、outcome unknown 与 committed 必须分别表达；preview 不是 receipt。

clean/dirty 与 authority/version 相互独立。Draft 即使与 committed after 精确相等而变成 clean，serial、production version 与 Observation 仍保持新的 current 值。

## 4. Direction vocabulary

“RTL support” 不能作为单一通过项。必须分别说明 Unicode 内容保全、bidi interaction、RTL shell 与 RTL localization。dir=auto 只是 presentation mechanism，不等于完整 RTL 能力。Caret Affinity 不能称作 source side；Layout Epoch 不能称作 result epoch。

Panel Partition 的中文固定为“分面板”，不得简称 Facet，以免与 D4 Facet 混淆。View textDirection、App locale 与 author source direction 必须分开。

## 5. Search vocabulary

File List Filter、Quick Open 与 Global Search 是 surface/preset，不是新的 Query kind。
plain、visual、shortcut 是 compiler-input UI mode，不是三种执行器。
SearchConditionAst/1、QuerySpec/2、CanonicalGraph 与 SearchContribution 继续由 D7 owner 管理。

title、subtitle、node_file_name、node_relative_path、resource_file_name 与 displayTitle 是不同概念；这些名称即使同屏展示也不能互换。
filename/path 不得称作 title。
snippet offset/highlight offset 不得称作 Locator。

## 6. Annotation vocabulary

AnnotationRef、Annotation Value/4、AnnotationInlineBody/1、Suggestion、target 与 reply 继续由 D3 owner 管理。
D8 Annotation Draft 只表示 proposal state。

manual reattach、reconfirm suggestion、apply suggestion 与 reject suggestion 是不同 operation class。
requalification 不等于 reattach。

resource_region 中的 d9rg1 只是 inner geometry token，不是完整 Locator。完整 ResourceRegionLocator 继续由 D3 owner 管理。

## 7. Structured row vocabulary

Native Document Table Row、Field Value Occurrence、NodeRefCollection row、Query row、Import IR row 与 Resource preview row 是六个不同 domain，不得统一称作 Record；同一表格 UI 也不能抹掉这些 domain 差异。
禁止引入 RecordRef、TableRowId 或 CollectionRef。
OccurrenceKey 不是持久 row identity；rowHandle 不是 author ID。

## 8. View/renderer vocabulary

renderer 只是 View consumer，不是 Query 或 aggregate owner。wide-to-long 是 Query transform，不能称作 renderer melt。D7 已 defer 的 layout 不得标记为 supported。

table/list fallback 必须描述为“使用同一份完整数据的替代表达；graphical renderer unavailable”，不能描述成 sampled fallback。

**View builder** 指 D8 在 current D7 `ViewSpec/1` 与 current SavedDefinition/DynamicBlock owner 之上的无损编辑 surface；其定义保存校验只检查静态条件，不消费 ResultHandle，完整结果/数据校验继续由既有 D7 运行期 View 门负责。它不是 schema、author object、parser、registry 或 persistence kind。**advanced ViewSpec route** 指 basic controls 无法表达合法 member 时编辑 exact current owner representation 的路径；它不是 migration，也不能绕过任一校验门。

## 9. Controlled aliases

没有已发布的 compatibility alias。实现采用这些命名时，应删除任何未列出的 controlled type/kind alias；历史研究文字和用户 prose 不迁移。

禁止以下 alias：

- EditorSession wire alias；
- author_draft/source_draft；
- 用 document_snapshot 充当 Draft Projection alias；
- 用 locator/parser 充当 Draft Edit Map alias；
- 用 PreparedActionBinding 充当 Prepared Edit Binding alias；
- sourceAffinity/sourceSide；
- 用 Query epoch 充当 LayoutEpoch alias；
- 用 localeDirection 充当 DirectionPreference alias；
- Record/row identity aliases；
- D8SearchEngine / D8QuerySpec。

## 10. Naming evidence boundary

本 lexicon 只冻结 owner 命名和 collision rule，不证明 UI locale 已实现。除既有 Direction Preference selector 与 Draft status phrase 外，不能因为设计文档出现一个概念就新增 CLI verb 或 locale key。
