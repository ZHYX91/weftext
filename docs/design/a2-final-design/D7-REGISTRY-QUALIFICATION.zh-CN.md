---
source_language: zh-CN
translation_status: source
---

[English](D7-REGISTRY-QUALIFICATION.md)
# A2 D7 术语 Registry 资格映射

状态：这是 A2-D7-2F89-P1-01 的作者修订。D7-REGISTRY.json 继续是准确的 parent-current machine Registry；本文只是资格 overlay，不是第二套 Registry 权威。

## 1. 三个不同层次

immutable fixed-S Registry 的 blob 是 4615d8c03df22ba9630728f946715de8e4308438，实际包含 34 个 concept 和 8 个 cross-stage binding。

协调后的 parent-current Registry blob 是 3cab5124f46822729093d5d955908b05eb65bb87，实际包含 34 个 concept 和 13 个 cross-stage binding。D7-REGISTRY.json 与这个 parent-current blob 逐字节相同。

A2 不创建另一份 Registry 对象。A2 保留 parent-current 的 34/13，并且只应用下表具名 successor 资格。历史记录继续按产生它们的 concrete version 分派。

## 2. 十三条 binding 的逐项资格

| ID | Parent-current owner / 含义 | A2 disposition | fresh A2 concrete rule | 历史保留 |
| --- | --- | --- | --- | --- |
| B01 | D3 identity algebra / D7 runtime payload | retained-current | EntityRef、Locator、LogicalOccurrenceKey、ResultRowHandle、Provenance 与 ActionEvidence 这六类对象继续由原语义 owner 分别负责，彼此不合并 | 已记录的各种形式都留在各自原 owner 下 |
| B02 | D4 schema and authorship | retained-current | FacetId、FieldId、FieldSelector、authoredProvenance 继续由 D4/D5 拥有 | 不做版本改写 |
| B03 | D6 control and transport | named-current-successor-overlay | 保留 SourceObservation/1、SourceVersionRef/1、CommitDomain/2、Frontier/2、OwnerInputBinding/2 与 ObservationScope/2 的原职责；新的完整 proof 使用 DependencyProof/3，不沿用 parent 文本中的 /2 | 真实 Proof2 与旧 protected record 继续按准确 decoder 和 pins 恢复 |
| B04 | D2 occurrence / D7 payload | named-current-successor-overlay | saved Query/View 仍是 D2 occurrence，但 fresh A2 carrier 使用 current D2 product/SavedDefinition，并可承载 QuerySpec/2；不创建 ViewRef | Profile2 措辞只作来源历史；真实 old payload 仍按原字节 |
| B05 | D3 identity submit / Definition Transfer | named-current-successor-overlay | 新的 identity submit 使用 D3 wire13，跨 owner 的 preparation 使用 PAB4；DefinitionTransfer 与 Result/9 Q 仍保持各自真实 inner type | wire12/PAB3 association 只在确有历史记录时恢复 |
| B06 | D6 capability split | retained-current | Policy/3 继续把 source_envelope_state 与 commit_sequence_state 分开，不新增 implication | Policy/1-/2 历史记录保持原语义 |
| B07 | D4 DerivedDuration | retained-current | D7 继续通过通用 adapter 消费已经冻结的 D4 result | 不变 |
| B08 | D5 CollectionCreationPolicy | retained-current | D5 拥有 saved collection creation semantics；D7 只承载，不另造 creation policy type | 不变 |
| B09 | D7 protected record and delivery | named-current-successor-overlay | 新的 A2 准备链使用 PAB4、D7ActionInput/3、D7ProposedInput/3、EffectManifest/3、EffectBytes/3 与 ownerKind d7_action/3；MinimumMapping/3、D7DefinitionInput/2、D7ResolutionAccess/1、FieldEntryImage/2、D7DefinitionTransferEffects/2 继续保持真实版本 | PAB3/Input2/Effect2/d7_action2 与更早记录继续按原 bytes 和 pins 恢复 |
| B10 | D3 resolution / D6 installation / D7 read-only canonical effects | retained-current with current outer carriers | D3ResolutionInput/1、D3CanonicalEffectPlan/1、D3CanonicalPlanProjection/1、D3CanonicalEffects/1、ConflictInstallInput/1 与 D7 canonical/restore image 都保持原 inner version；只有适用的当前 outer carrier 使用 wire13、Proof3、Effect3 | inner/outer 版本组合始终按真实记录准确分派 |
| B11 | D8 edit consumer / D7 transport | named-current-successor-overlay | 新的 D8 edit consumer 使用 PreparedEditBinding/3，并绑定 PreparedIntent3 与 EffectManifest/3、EffectBytes/3 | PreparedEditBinding/2 以及 Manifest2/EffectBytes2 只在真实历史记录中恢复 |
| B12 | D9 template consumer / D7 preparation | named-current-successor-overlay | TemplateRecipe/2、TemplateConstruct/2、TemplateConstructionInput/2、D9EntityVersionAddress/2 继续由 D9 拥有；当前 construction adapter 进入 PAB4，不再进入 PAB3 | 历史 PAB3 construction association 原样保留 |
| B13 | D6 bootstrap / D7 display / D10 consumer | named-current-successor-overlay plus exact historical dispatch | 新的 bootstrap 使用 WorkspaceBootstrapPlan/4、WorkspaceBootstrapProfile/4、WorkspaceTrustGenesis/2，Policy/3 不变；当前 D7 effect transport 使用 EffectManifest/3 与 EffectBytes/3；既有 d7_planned_preview_open/opened surface 保留真实 outer contract，并按记录本身重新打开对应 current 或 historical protected record | Plan1/Plan3/Profile3/symbolic2 与 Manifest1-2 保留准确 family/custody；historical Manifest2 不改标签为 Effect3，新的 current preview payload 才使用 Effect3 |

**B05/B09/B13 具名现任生产/消费 overlay：**直接 interactive Run start 是 D10 Core §7.1 可信到场 runtime 生产者，不是 D7 Action 或第八个 control action。此 Run 经原 Run-targeted LeaseRunUse CAS 准入之后，才可用现任 `PreparedActionBinding/4`、`EffectManifest/3`、`EffectBytes/3` 及 D10 Link2/ApprovalUse2 关联进入原 D7 作者 step；D6 仍保留原作者 request/DecisionKey/P。真实已记录历史 PAB3/Manifest2/Link1/ApprovalUse1 使用**自身**保存的 decoder、准确 pins 与 preview semantic digest，冷启动也不变。当前 PAB4 不经 PAB3-only pin 获取，历史 Manifest2 不按 Manifest3 重算，Link1 不冒充 Link2 恢复。`d7_planned_preview_open` 按实际记录版本分派，保持原 request/OperationId/完整 preview 与闭合错误/披露次序。D7 owner/Registry 唯一，不创 D10 第二 registry/作者账本。B13 仍用准确 D6 Profile4/Plan4/Genesis2 + 必需 D8 初始 presentation policy、唯一 final P，不采用旧 D10 Plan3 consumer。

## 3. Owner 与优先级

parent-current Registry 继续是十三条 binding 的唯一 machine 列表。本 overlay 不把十三条 parent row 改写成新版本字符串，也不重新发布一份 Registry；它只说明 A2 fresh current 采用哪些 concrete version，以及 parent 文字中哪些版本是历史、哪些保持不变。

表中标为 named-current-successor-overlay 的条目，只对 fresh unseen work 由 D7-SCHEMAS 与 fixed97 的准确 current schema 优先。retained-current 不发生机械升版。historical-record-only 也绝不表示删除：saved/planned/unknown/committed 记录保留原 decoder、pins、request fingerprint、custody 与 recovery。

## 4. 复核 oracle

复核者必须能够从 B01-B13 任一条出发，找到唯一 semantic owner，区分 fixed-S 34/8 与 parent-current 34/13，识别 A2 fresh successor 而不改写未变化的 inner version，并找到历史 decoder 分支。只有计数正确不构成接受证据。
