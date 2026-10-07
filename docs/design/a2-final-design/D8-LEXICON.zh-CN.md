---
source_language: zh-CN
translation_status: source
---

[English](D8-LEXICON.md)
# A2 D8 Terminology and Naming

状态：author-resolved-pending-independent。固定 S 的九个 D8 concept identity 保留；D10 R08 的 technical-interface-owner 元数据作为当前协调输入吸收，但 current schema version 以现任 D8 owner 为准（Prepared Edit Binding 当前是 /3，不把旧 /2 提案重新变 current）。

## 1. 九个 D8 domain concepts

| Concept ID | 中文 / English | 当前 owner/type | 明确排除 |
| --- | --- | --- | --- |
| weftext.term.edit_session | 编辑会话 / Edit Session | D8 UI state; EditSession | Workspace/Document identity、commit authority |
| weftext.term.edit_draft | 编辑草稿 / Edit Draft | D8 proposal; Draft + draftSerial | author source/revision |
| weftext.term.draft_projection | 草稿投影 / Draft Projection | Core D8 projection; d8_draft_projection | D2 Snapshot/payload、Locator |
| weftext.term.draft_edit_map | 草稿编辑映射 / Draft Edit Map | flows/plainRegions/sites | durable paragraph identity、Locator、client parser |
| weftext.term.prepared_edit_binding | 编辑准备绑定 / Prepared Edit Binding | protected PreparedEditBinding/3 | D7 PreparedActionBinding、third ledger |
| weftext.term.composition_transaction | 组合输入事务 / Composition Transaction | D8 host IME state | D6 author transaction/receipt |
| weftext.term.caret_affinity | 光标亲和位置 / Caret Affinity | upstream/downstream | source offset、Locator |
| weftext.term.layout_epoch | 布局代 / Layout Epoch | discardable layout generation | result/auth/source revision |
| weftext.term.direction_preference | 方向偏好 / Direction Preference | device/session ltr|rtl|auto | locale、portable author direction、Query order |

不得新增第十个 “Record/Row/DocumentRef/SearchUIState” D8 domain concept。D2 snapshot、D3 Value、D6 SourceObservation、D7 Query/View/Action 是 consumed owner data，不因 D8 UI 使用而改 owner。

## 2. 十三个 kind 与唯一 technical interface owner

| kind | technical owner | consumes | returns |
| --- | --- | --- | --- |
| d8_document_read | D8 Document Read Interface | NodeRef + current D6 read qualification | d8_document |
| d8_document | D8 Document Read Interface | exact read result | Snapshot/3 envelope |
| d8_draft_project | D8 Draft Projection Interface | Draft proposal + current cut | d8_draft_projection |
| d8_draft_projection | D8 Draft Projection Interface | one proposal | projection + Draft Edit Map |
| d8_draft_text_replace | D8 Draft Text Replace Interface | Draft + one proven scalar segment | d8_draft_text_replaced |
| d8_draft_text_replaced | D8 Draft Text Replace Interface | exact replace result | complete projection; no implicit caret |
| d8_draft_write | D8 Draft Write Interface | Draft + region/site mapping | d8_draft_written |
| d8_draft_written | D8 Draft Write Interface | exact command result | projection + caret |
| d8_edit_prepare | D8 Edit Prepare Interface | D8EditPrepareRequest/3 | d8_edit_prepared |
| d8_edit_prepared | D8 Edit Prepare Interface | PreparedEditBinding/3 result | original request + preview; no commit claim |
| d8_undo_prepare | D8 Undo Prepare Interface | original decision/effects + exact current after | d8_edit_prepared for a fresh inverse |
| d8_editor_error | D8 Error Interface | D8 public entry failure | closed owner error |
| d8_prepared_edit_binding | D8 Prepared Edit Binding Internal Interface | immutable /3 preparation state | protected only; never public response kind |

多个 kind 可共享一个 interface owner；Draft Edit Map 等 data concept 不是 kind 的共同 owner。document|annotation 只是 D8 intent discriminator，不是 EntityKind。

Annotation read/draft open 与 presentation-policy 当前 kinds 是现任 successor 的额外 public interfaces，继续由 D8 owner；它们不产生新 domain concept identity：
- d8_annotation_read；
- d8_annotation_draft_open；
- d8_presentation_policy_set；
- d8_presentation_policy_no_change / d8_presentation_policy_prepared；
- internal presentation-policy record/head/outbox kinds。

## 3. Mode/status vocabulary

Source / Write / Read 是 mode，不是三份 Document。readonly 描述 current interaction permission，不是第四份 content。Draft saved locally、pending submission、planned、outcome unknown、committed 必须分开；preview 不是 receipt。

clean/dirty 与 authority/version 独立：Draft exact after 可以 clean，但 serial/version/Observation 仍是新/current 值。

## 4. Direction vocabulary

“RTL support” 禁止作为单一通过项；必须分别写 Unicode content preservation、bidi interaction、RTL shell、RTL localization。dir=auto 是 presentation mechanism，不是完整能力。Caret Affinity 不称 source side；Layout Epoch 不称 result epoch。

Panel Partition 中文“分面板”，不得简称 Facet，以免与 D4 Facet 冲突。View textDirection 与 App locale、author source direction 分开。

## 5. Search vocabulary

File List Filter、Quick Open、Global Search 是 surface/preset，不是新的 Query kinds。plain、visual、shortcut 是 compiler-input UI，不是三种执行器。SearchConditionAst/1、QuerySpec/2、CanonicalGraph、SearchContribution 均保持 D7 owner。

title、subtitle、node_file_name、node_relative_path、resource_file_name、displayTitle 分开。filename/path 不得称 title。snippet offset/highlight offset 不得称 Locator。

## 6. Annotation vocabulary

AnnotationRef、Annotation Value/4、AnnotationInlineBody/1、Suggestion、target、reply 保持 D3 owner。D8 Annotation Draft 只描述 proposal。manual reattach、reconfirm suggestion、apply suggestion、reject suggestion 是不同 operation class；requalification 不是 reattach。

resource_region 的 d9rg1 是 inner geometry token，不是完整 Locator；完整 ResourceRegionLocator 仍 D3 owner。

## 7. Structured row vocabulary

Native Document Table Row、Field Value Occurrence、NodeRefCollection row、Query row、Import IR row、Resource preview row 六类不得统称 Record。禁止 RecordRef/TableRowId/CollectionRef。OccurrenceKey 不是 durable row identity；rowHandle 不是 author ID。

## 8. View/renderer vocabulary

renderer 是 View consumer，不是 Query/aggregate owner。wide-to-long 是 Query transform，不叫 renderer melt。D7 deferred layout 不得标 supported。table/list fallback 必须写 “same complete data alternative; graphical renderer unavailable”，不能叫 sampled fallback。

## 9. Controlled aliases

没有发布兼容 alias。实现采用本命名时，删除任何未列出的 controlled type/kind alias；历史研究文字和用户 prose 不迁移。禁止：
- EditorSession wire alias；
- author_draft/source_draft；
- document_snapshot 作为 Draft Projection alias；
- locator/parser 作为 Draft Edit Map alias；
- PreparedActionBinding 作为 Prepared Edit Binding alias；
- sourceAffinity/sourceSide；
- Query epoch 作为 LayoutEpoch alias；
- localeDirection 作为 DirectionPreference alias；
- Record/row identity aliases；
- D8SearchEngine / D8QuerySpec。

## 10. Naming evidence boundary

本 lexicon 只冻结 owner 名称与 collision rule，不证明 UI locale 已实现。除 Direction Preference 既有 selector 与 Draft status phrase 外，不凭设计文本新增 CLI verb/locale key。
