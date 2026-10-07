---
source_language: zh-CN
translation_status: source
---

[English](D8-INTERFACES.md)
# A2 D8 当前接口与状态机

状态：author-resolved-pending-independent。本文是 D8 当前接口 owner；只在具名 successor 上替代 fixed-S / D6-FA 旧 current 版本，真实历史 decoder 保持原样。

## 1. 公共 decoder 与 owner 边界

所有 D8 JSON 都是 strict UTF-8 单对象：duplicate/unknown/missing member、非法 null、unknown kind/version 拒绝。D8 structural integer 复用 D6 Counter，拒绝 Boolean、fraction、exponent、-0、overflow。嵌入 D2/D3/D4/D6/D7 值继续使用各自 decoder，D8 不改写其 null/number/version 规则。

D8 不定义 EntityRef、Locator、SourceVersion、SourceObservation、Query、Field、Action、Effect、receipt 或 PublicationReceipt 的新版本。D8 public errors 不能包装/重命名进入 D3/D6/D7/D9 后的 owner error。

`D8SourceTarget/2 := {ref:EntityRef,expectedObservation:SourceObservation/1}`。ref 必须等于 observation.entityRef；Workspace/observerDomain 必须等于 request 的 workspaceRef/commitDomain。生产 domain/version 与当前 observer domain/epoch 分开。这个对象不是 permission ticket；Core 逐项重建/验证。

## 2. Document read

当前 read request：

```text
{wireVersion:2,kind:"d8_document_read",
 workspaceRef,commitDomain,ownerNodeRef,budget}
```

成功：

```text
{wireVersion:2,kind:"d8_document",
 workspaceRef,commitDomain,ownerNodeRef,
 sourceObservation:SourceObservation/1,
 documentRevisionToken,
 domainFenceToken,
 snapshot:D2DocumentSnapshot/3}
```

顺序：closed decode → Workspace/entity disclosure → complete source_read + source-envelope/current-format qualification → authority/cut → exact source + D2 fixed-2.0.26 evaluation → final delivery barrier。managed token 必须经真实 RevisionTokenSeal/Binding 证明与 sourceObservation.sourceVersion 同一生产版本；外部 source 使用其原 external-event/version 证明。

D2-invalid source 可在授权后通过完整 invalid Snapshot/3 分支返回；physical decode failure 是 source_unavailable。成功 read 不更新已有 Draft Base，不重签 Locator，也不授 write。

## 3. Draft projection

```text
d8_draft_project/2 = {
 wireVersion:2,kind:"d8_draft_project",
 workspaceRef,commitDomain,ownerNodeRef,
 expectedSourceObservation,draftSerial,source,budget
}
```

valid success 保留 exact source、baseObservation、serial、current D2 metadata/attribute carriers/body、`editMap={flows,plainRegions,sites}` 与 navigation `origins={elements,flows}`；invalid success 只返回 exact source + diagnostics，不泄露 partial metadata/body/map/origins。

`mapBinding={ownerNodeRef,baseObservation,draftSerial,source}` 四项 exact。任何一项不同都不能 relabel 旧 map。

BodyPath 只走 current D2 body schema 的结构 member/index；用户 payload 中相同 key 不参与。flow 每个 Inline slot 一个；plainRegion 是单 parent 的 maximal exact raw ordinary-text region；site 是 parser 证明的 sibling boundary。navigation origin 独立于 editability。

## 4. TextReplace 与 DraftWrite

`d8_draft_text_replace/2` 只对一条 nonnull flow segment 工作；请求显式携带 flowPath、segmentIndex、start/end、replacementText。start/end 必须是完整 flow 中的 Unicode18 grapheme boundary，replacementText 不含 CR/LF。成功返回 `d8_draft_text_replaced`，projection serial=inputSerial+1；它没有隐式 caret。

`d8_draft_write/2` 的 command union 只有：

```text
{kind:"splice_plain",regionIndex,start,end,text}
{kind:"break_plain",regionIndex,offset}
{kind:"insert_at_site",siteIndex,text}
```

success 是 `d8_draft_written` + complete projection + proven caret。region splice 保留 selected range 外每个 byte；insert_at_site 在 0/1/2 × 0/1/2 EOL 九候选里用 full reparse 选 minimum generated EOL，再最小 prefix。EOL policy 仅用于生成字符；existing/pasted EOL 原样。

Source no-op 可以返回新 serial/caret，但不产生 content Undo group。serial 不回绕。old map/serial 不因 Undo 复活。

## 5. Async input protocol

每个 editor controller 最多一个 transform in-flight。host 记录：

```text
InputTransaction {
 owner, generation, inputSerial, draftSerial,
 beforeSource, beforeSelection,
 nativeAfter, nativeSelection,
 compositionState, successorLog
}
```

这是 UI state，不是 portable wire。结果只有在 owner/Base/source/inputSerial/generation 都匹配其 transaction 时可接纳。迟到结果不覆盖后继 native input。

IME begin/update=preedit；commit/end/final input 才形成 Draft transaction。active composition 下禁止 prepare/submit/structural command。cancel/blur/background 不制造 source write。final composition 形成一个 Undo group。

## 6. 当前 Edit prepare /3

current request：

```text
D8EditIntent/3 =
  {kind:"document",target:D8SourceTarget/2,source:text}
| {kind:"annotation",target:D8SourceTarget/2,
   expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
   value:AnnotationEditableProposal/1,
   targetPolicy:"preserve"|"replace_current"}
| {kind:"annotation_reconfirm_suggestion",target:D8SourceTarget/2,
   expectedAnnotationRevisionToken:AnnotationRevisionToken/1}

D8EditPrepareRequest/3 = {
 wireVersion:3,kind:"d8_edit_prepare",
 workspaceRef,commitDomain,
 saveProfile:"ordinary"|"complete",
 guarantee:"replica_local"|"managed_atomic",
 writeProtection:"strict"|"observed_only",
 intent:D8EditIntent/3,budget:BudgetBinding/1
}
```

document proposed pin 是 exact UTF-8 source。Annotation caller 只能提供 `AnnotationEditableProposal/1`；Core 在 operation-class gate 后构造完整 Value/4，caller 不能提供 actor/time/trusted/state/confirmation/basis/expectedText/pointAffinity。

`PreparedEditBinding/3` exact current semantic members：

```text
{kind:"d8_prepared_edit_binding",version:3,
 workspaceRef,commitDomain,operationId,principalAudienceToken,
 inputDescriptor:InputDescriptor/3,
 intent:D8EditIntent/3,
 origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request},
 sourceInputs:InputDescriptor/3.sourceInputs,
 proposedInputs:[{entityRef,pin:PinRef/2}],
 registryInputs:[ValidatedCatalogContext...],
 dependencyProof:DependencyProof/3,
 observationProof:PreparedIntent/3.observationProof,
 budgetBinding,expiresAt,
 request:d6_commit_request/2,
 preview:EffectManifest/3}
```

OwnerInputBinding current `ownerKind=intentKind=d8_edit/3`，canonicalDescriptorBytes=D3-CJ/3(D8EditInput/3)。current effect bytes 使用 EffectBytes/3，Annotation pin encoding 是 `d3_annotation_value4`。

ordinary/complete 与 strict/observed_only 是独立轴。observed_only 仅 trusted interactive single existing live Document whole-source save；Annotation/Undo/structured/bulk/automation/managed_atomic 不能使用。

prepare 成功只说明 immutable plan/preview 已建立，不等于 Saved。提交仍是原 D6 request；计划/提交阶段 revalidate descriptor/proof/dependencies/authorization/install。

## 7. Annotation read / Draft

```text
D8AnnotationReadRequest/1 = {
 wireVersion:1,kind:"d8_annotation_read",
 workspaceRef,commitDomain,annotationRef
}

D8AnnotationReadResponse/1 = {
 kind:"d8_annotation_read",version:1,
 workspaceRef,commitDomain,annotationRef,
 sourceObservation,
 annotationRevisionToken,
 value:D3-Annotation-Value/4,
 targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
 body:D8AnnotationBodyRead/1
}

D8AnnotationDraftOpenRequest/1 = {
 wireVersion:1,kind:"d8_annotation_draft_open",
 workspaceRef,commitDomain,annotationRef
}
```

body absent→exactSource=null/semanticText=""；valid→exact source + semantic text；invalid→exact source + diagnostics、semanticText=null。read 返回完整 Value/4，包括 attribution。Draft 只保留 editable fields；annotation_write 缺失时 access=readonly，readonly 不能进入 prepare/3。

ordinary_edit、manual_reattach、reconfirm_suggestion 是由 intent arm/targetPolicy 机械决定的 operation class，不是 caller flag。

## 8. Preview / confirmation / recovery

D8 confirmation UI 消费当前 D7/D6 full preview transport：`EffectManifest/3`、`EffectBytes/3`、preview token/cursor/epoch 与完整 effect pages。UI 必须在确认前证明所有 pages/bytes 可取得且属于同一 plan/epoch。manifest summary、当前 page 或缓存旧 bytes 不足以确认。

TTL/authorization/epoch/reset 后禁止 submit old preview。planned/saved/unknown 先恢复 original request + original decoder/pins/OperationId，再继续原责任。新 prepare 不能假装解决 unknown old decision。

## 9. committed Undo prepare

`d8_undo_prepare` 是新 operation 的 prepare entry，不是 ledger rollback。输入恢复原 request/receipt/effect before/after 并证明：
- original decision 只有一个 source change；
- 无其它 control/identity/entity effect；
- current production version+bytes exact 原 after；
- current authorization/format/Observation/install fresh；
- complete pins 仍在。

成功返回普通 current `d8_edit_prepared` family，使用新 OperationId/planToken/preview。Redo 同样是 fresh operation。

## 10. Presentation policy

current shared presentation policy 是 D8-owned complex P-only configuration，不是 D6 Policy/3 或 PortableComponentKey。

主要 current types：
- `D8WorkspacePresentationPolicy/1`：唯一 head 的逻辑 current view；
- immutable `D8WorkspacePresentationPolicy/2` record：parents、revision、defaultPresentation、activationChangeId；
- protected `D8PresentationPolicyHeadSet/1`；
- `D8PresentationPolicySetRequest/2`；
- `D8PresentationPolicyPrepareResult/2`；
- `D8PresentationPolicyEffect/1`；
- `D8DocumentRenderBinding/1`。

SetRequest 顺序：decode → presentation disclosure → policy_admin → expectedFrontier/domain → protected head set → exact expectedHeads → parent record/pin/ancestry → semantics/budget。相同单 head value 是 no-change；[] 仅在证明未初始化 owner state 时合法；多 head conflict 不因 values 相同而 LWW。

prepare 不创建 ChangeId/current record；final P after all checks 才分配一个 ChangeId，构造/哈希/pin record，提交 effect/receipt/ChangeRecord/outbox/head transition。offline concurrent writers 可产生两个 revision=2 heads；revision/arrival/hash 不能自行选 winner。resolution 显式以全部 current heads 为 parents。

## 11. Error 与 disclosure 顺序

继承的 D8 editor error family 保持 closed：

`invalid_request|not_visible|authority_unavailable|domain_unavailable|integrity_conflict|source_unavailable|proof_unavailable|owner_update_required|stale_target|ambiguous_selection|unrepresentable_text|semantic_rejected|budget_exceeded`。

current coordinated path 若其 owner 已有真实 unavailable/conflict 结果，必须返回该结果；`owner_update_required` 不能作为通用 implementation 占位。静态 decode 在先，敏感存在/版本/Locator/detail 只有在对应 disclosure 后可观察。进入 D3/D6/D7/D9 后返回该 owner 原 error，不由 D8 wrapper 改名。

## 12. Surface parity

Desktop/CLI/Mobile local Core 和 Server Core 调相同 interfaces；WebUI 仅 Server。一个 surface 缺 renderer/IME/table editor 时返回真实 capability unavailable，不得改 request meaning。Remote/offline hosted author source 不因本地有 parser 就获得 commit authority。
