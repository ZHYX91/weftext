---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：21e90c5d-66a2-476b-b852-5a77ac4e7f13。

# D5 Implementation Impact and Test Outline — D6-FA-r01

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 的 D5 v1 实施/验收义务继续保留；本文件当前消费 actual A D6 的 SourceVersion/2 生产历史、SourceObservation/1、SourceVersionRef/1、CommitDomain/2、Frontier/2 与 SemanticState，actual B D1/D3 规范接口，以及本批当前 D4 与 D5 main；并继续覆盖 local-vs-complete、partial-index 和 multi-replica 验证。H2 D3/D6 与真实 saved decision 仅作为历史兼容和恢复背景，不作为当前 producer，也不据此声称接受、激活、实现、完整阅读或独立验收完成。

## 1. implementation effect graph

~~~text
D2 exact source
  -> native table parser / current-revision locators
  -> D5 structured table intent
       -> local source transformation
       -> D6 SourceVersion/2 + SourceObservation/1
       -> SourceVersionRef/1.sourceToken + InputDescriptor/2.sourceInputs[].observation
       -> current cut / MutationFootprint / strict install
  -> D4 Field Value Occurrence editor
       -> D4 Entry/Registry proof + inner sourceRevision/OccurrenceKey/Entry selector
       -> current SourceObservation/1 outer qualification
  -> D7 Query -> NodeCollectionResult
       -> exploratory partial presentation
       -> strong collection/bulk operation
            -> future D7 complete cut / Prepared
            -> Frontier/2 sealed causal/dependency prefix
            -> D3/D4/D6 managed commit
~~~

SourceVersion/2继续表示生产版本历史：managed_source_version/2保留entityRef、commitDomain、observationEpoch、revision、changeId，其中changeId.commitDomain等于该生产commitDomain；external_source_version/2保留entityRef、commitDomain、observationEpoch、externalSequence；生产域可以不同于当前operation域。完整current SourceObservation/1要求observerDomain等于operation CommitDomain、entityRef等于sourceVersion.entityRef，并以当前observationEpoch、fileObjectBinding、evidencePins及同一cut内的control、Registry、incidence依赖提供外层保护。SourceVersionRef/1.sourceToken以d6_source_observation/1选择该完整Observation，InputDescriptor/2.sourceInputs[].observation实际承载它。watcher gap、replacement或discontinuous rematerialization即使production version、hash或row text相同也使旧token和依赖它的locator失效；I不能恢复资格。D5 current Document revision/locator及D4 inner selector wire保持不变。

Frontier/2只表示已seal的因果/依赖前缀，不替代全集Query、Registry完整性或payload物化证明。D7 strong collection/bulk仍等待未来完整Prepared/current cut；当前入口保持unavailable/owner_update_required，D5不制造success binding。

没有 Record/RecordCollection durable storage。I中的table parse、collection result和membership candidate全部可重建。

## 2. implementation slices

### 2.1 native Document Table

需要：
- exact source table parser；
- current SourceVersion/2，并从其真实生产revision取得current Document revision；
- 完整current SourceObservation/1，以及SourceVersionRef/1.sourceToken和InputDescriptor/2.sourceInputs[].observation承载的当前观察资格；
- 当前cut内真实fileObjectBinding、evidencePins、control、Registry、incidence依赖；
- table/row/cell locator validation；
- ragged row model；
- representable cell encoder；
- exact trivia/line-ending preservation；
- source-patch overlap detection；
- 完整current source与唯一proposed full source；
- current authorization与D2/local typed检查；
- D6 safe-install、P seal/recovery和实际file-install evidence。

native table editor不是 D4 schema editor，也不依赖无关全 Workspace Query；这只缩小所需语义证明范围，不产生weak保存资格。structured cell/row/column/reorder继续要求strict保护；只有满足actual A §4.1全部条件的人工raw整源已有一个live Document保存，才能在planning开始前显式选择observed_only并冻结profile。ordinary与strict|observed_only是独立两轴，不能由UI形态或缺少无关全库证明推导weak。

### 2.2 D4 occurrence editor

表格式 property editor必须调用 D4 Entry/Occurrence semantics，而不是创建 TableRow/Record。每个 add/remove/reorder/note继续绑定current source、D4 Registry以及原有inner sourceRevision、OccurrenceKey、expected Entry selector，并额外消费完整current SourceObservation/1、SourceVersionRef/1.sourceToken、InputDescriptor/2.sourceInputs[].observation及同一current cut内的fileObjectBinding、evidencePins、control、Registry、incidence依赖。watcher gap、replacement或discontinuous rematerialization后，旧token/selector不能因production version、hash、I或相同值重新续认；current authorization、typed admission、完整post-state与relation effects仍须实际验证。这里不创建新的request/plan wire，也不因表格式UI取得observed_only资格。

### 2.3 Node Collection

collection viewer消费 D7 result + explicit coverage。强 collection action等待 D7新版 complete result/Prepared；当前不能用 old Prepared/1,/2、page或cache补成功。

### 2.4 conversion/import/export

row→Node/import由D9负责mapping/loss，D3负责fresh identity。D5只提供 source row domain。跨Workspace保持两个独立receipts。

## 3. component impacts

| area | future work | forbidden |
|---|---|---|
| table parser | native D2 grammar + locators + ragged/trivia | row ID/database row |
| table editor | exact source patch + D6 install | sidecar/table DB |
| Field list UI | D4 Entry adapter | Record inference |
| collection engine | D7 result + coverage | page as membership |
| collection actions | frozen targets + complete cut | re-evaluate all at commit |
| import | D9 map + D3 fresh Node | IR row ID as NodeId |
| I | parse/result cache | authority |
| P | decision/recovery only | membership/current source |

## 4. test outline

### 4.1 native source

1. simple table cell edit preserves delimiter/header/footer.
2. ragged rows remain ragged.
3. missing trailing cells remain absent.
4. CRLF/LF preserve.
5. inline formatting and escaped separator preserve.
6. inter-row comments/blanks block structured reorder.
7. representable cell exact round-trip.
8. unrepresentable cell returns `unrepresentable_cell`, zero write.
9. table locator old revision returns stale.
10. same row text after external edit is not old occurrence continuity.

### 4.2 row/column operations

- append/remove one row；
- batch rows up to 1000；
- column add/remove across ragged rows；
- header/title edit；
- reorder safe table；
- reorder with unsafe trivia；
- overlapping patch rejection；
- exact post-source reparse；
- current SourceVersion CAS，并保持其生产 revision 与 table/row/cell Locator 的内层版本绑定；
- 同一操作还必须绑定完整 current SourceObservation/1、SourceVersionRef/1.sourceToken、InputDescriptor/2.sourceInputs[].observation，以及同一 current cut 内的 fileObjectBinding、evidencePins、control、Registry、incidence 和 strict install 资格；
- 任一当前观察、授权、pins、依赖或 cut 变化都使旧计划 stale/reprepare，不能用裸 SourceVersion、hash、I 或相同行文字续认旧 Locator；
- 局部 structured row/column 操作不要求无关 Workspace 全集 Query，但 structured cell/row/column/reorder 仍使用 strict 保护，不能由局部性或 UI 形态推导 weak 保存；
- crash before/after install under D6 recovery。

### 4.3 Field occurrence UI

- repeated equal D4 values remain distinct occurrences；
- note follows occurrence selector, not value；
- D4 type/cardinality/requiredness failures reject；
- unavailable namespace can be displayed raw but not typed-mutated；
- local valid Field + cross-object missing proof becomes D4/D6 pending only；
- no TableRowId/RecordRef emitted。

### 4.4 collection membership

1. deduplicate same NodeRef.
2. unauthorized/trashed Node excluded by Query contract.
3. Node in multiple collections.
4. deleting saved definition leaves Node intact.
5. transport page 200 is not full membership.
6. semantic `take` changes membership.
7. partial index exposes explicit partial only.
8. placeholder/unmaterialized source never means absent membership.
9. rebuilding I recomputes result without author writes.

### 4.5 Collection Creation Policy

- valid explicit parent；
- parent stale/not-visible/trashed；
- no root fallback；
- default title/body/source；
- Template/default fact mapping；
- requireMembership=false local create；
- requireMembership=true complete post-query；
- same definition/params/auth/dependency reevaluation；
- query would exclude new Node → whole strong Action rejects；
- D7 new Prepared absent → unavailable。

### 4.6 Remove from collection

- explicit refs selector invertible；
- simple D4 Field predicate with one previewed edit；
- arbitrary derived/filter/aggregate noninvertible；
- UI separation from Trash；
- stale preview does not retarget；
- permission change blocks commit without deleting Node。

### 4.7 delete modes

Mechanical tests ensure:
- table-row delete changes Document only；
- D4 occurrence delete uses D4 gate；
- remove-from-collection changes definition/fact only；
- Trash changes D3 lifecycle；
- purge only managed_atomic and irreversible；
- generic Delete cannot switch meaning silently after View change。

### 4.8 batch and limits

For target counts 0,1,200,201,999,1000,1001:
- enforce appropriate target/page limits；
- narrower budgets can reject earlier；
- never auto-sample/truncate；
- preview target set byte/identity-equal at commit；
- all_result requires complete cut；
- partial result cannot feed mutation。

Import batch tests 1000/1001 fresh Nodes separately.

### 4.9 cross-Workspace

target copy success + source Trash failure yields explicit partial transfer state,
  not atomic move. Both receipts preserve separate OperationId/CommitDomain/authority.

### 4.10 multi-replica / conflicts

- A/B edit same cell；
- A/B edit different rows but file generation conflicts；
- explicit three-way source merge；
- collection definition changed on A, member fact on B；
- page/placeholder on one replica；
- identical hash but new observationEpoch；
- production SourceVersion相同但 current observationEpoch、fileObjectBinding、evidencePins、control、Registry、incidence 或 cut 改变时，旧 SourceVersionRef/1.sourceToken 与依赖它的 Locator 必须 stale/reprepare；
- watcher gap、replacement 或 discontinuous rematerialization 即使 production version 相同也使旧 token/Locator 失效；裸 hash、I、相同行文字或 ABA 都不能恢复 continuity；
- SourceVersion/2 的 production commitDomain 可以不同于当前观察域，但 SourceObservation/1.observerDomain 必须等于 operation CommitDomain，entityRef 必须等于 sourceVersion.entityRef；
- no mtime/LWW winner；
- merge never creates hidden row identity。

### 4.11 r5/r6, I/P

- r5 pending(collection) committed；semantic_pending(collection)只表示缺少 collection 全集 proof，不表示 empty，也不能把 typed invalid、source invalid 或缺失 strong evidence 洗成成功，更不能授权 Action、all_result、bulk 或 Automation；
- r6 current complete result later，并且只证明 r6 及其 current SourceObservation/SourceVersion/cut；
- replay r5 returns original bytes，后来的 r6 proof 不修改 r5 历史 receipt；
- delete/rebuild I doesn't upgrade r5，也不能恢复旧 membership/cut proof；
- lose P doesn't recreate batch decision/approval，也不恢复 Money 或 execution custody；
- new device gets fresh replicaEpoch, same NodeRefs, no execution custody。

### 4.12 partial index / complete action

Partial metadata or query index may render a local list,
  but fixed negative tests reject its use for:
- complete membership；
- requireMembership postcondition；
- remove-from-collection inverse；
- bulk/all_result；
- collection negative constraint；
- Automation target set。

A full source scan may produce the required
  complete cut when D7 contract allows;
  otherwise strong action unavailable.

### 4.13 domain fixtures

People duplicate phone/name rows do not become Records.
Organizations inverse relation rows resolve to D4 authored owner.
Calendar derived occurrences remain derived.
Library citation/work/resource rows keep their domains.
Native table row conversion always fresh Node.

## 5. legacy and terminology

- fixed D5 six conceptIds/owners/owned names/firstFreeze exact；
- no TableRowId/RecordRef/CollectionRef/ViewRef；
- old D5 v1 meaning preserved；
- saved D7 PreparedActionBinding/1,/2 replay only under legacy；
- new D7 Prepared not guessed；
- D3 v9/v10/v11 and D6 wire1 historical decisions unchanged。

## 6. performance and resource validation

Future benchmarks record table row/cell count, source bytes,
  parser time, patch size, peak RAM, index coverage,
  collection candidate/result counts,
  page count and cancellation. No seconds-level performance guarantee is made by this design.

The implementation must remain bounded for large tables/workspaces and must
  not scan unrelated content merely to make ordinary local edits look complete.

## 7. completion boundary

A future implementation slice requires Core/adapters/fixtures/bilingual docs/legacy replay
  and real execution evidence. Documentation CI or author self-review is not product conformance.

旧D10 B13仍REVISE、术语/双语FAIL、11 OPEN。

## 8. fixed-S 架构影响与完整回归义务保留

本节把 fixed-S D5 Impact 中仍有效的架构影响、Record 分支退役和完整测试案例并入当前候选。D6-FA-r01 当前映射消费 actual A D6 的 SourceVersion/2 生产版本、SourceObservation/1、SourceVersionRef/1、InputDescriptor/2、CommitDomain/2、Frontier/2 与 SemanticState，actual B D1/D3 规范接口，以及本批当前 D4/D5；只替换外层文件权威、当前观察资格、SourceVersion/CommitDomain/semantic-pending/多replica消费，不删除下列 fixed-S 语义与fixture义务。SourceVersion/2 保留原有生产版本字段：managed_source_version/2包含entityRef、commitDomain、observationEpoch、revision、changeId，且changeId.commitDomain等于该生产commitDomain；external_source_version/2包含entityRef、commitDomain、observationEpoch、externalSequence，生产域可以不同于 current observerDomain；SourceObservation/1 的 observerDomain 必须等于 operation CommitDomain，entityRef 必须等于 sourceVersion.entityRef。Frontier/2 只表示 sealed causal/dependency prefix，不替代全集 Query、Registry 完整性或 payload materialization proof。本节仍是未接受、未激活、未实现的候选映射，不声称产品实现、独立接受或额外完整阅读完成。

### 8.1 跨阶段架构影响

D2继续提供 native table/row/cell grammar、revision locator、exact source局部编辑；Saved Query/View只作Node内occurrence。禁止row identity、Record正文、ViewRef和读取时矩形化。

D3继续提供NodeRef、owner-local Resource/Annotation、copy/fork/continue、Trash/restore和receipt；
  D6-FA新decision使用wire12，但不增加Record identityMap arm，也不让occurrenceKey跨revision resolve。

D4继续提供Registry、FieldId、TypedValue、Facet/relation公共post-state、
  raw source保留和Entry/schema admission。
  table column不覆盖Field schema，不恢复people blob或Entry Annotation。

D6负责单物理副本/Server后端持久提交资格、SourceVersion/2生产版本历史、SourceObservation/1当前观察资格、SourceVersionRef/1、CommitDomain/2、Frontier/2、source patch、完整read-set/negative dependency、safe install、P seal/recovery和有限权限。SourceVersion/2保留原有生产版本字段：managed_source_version/2包含entityRef、commitDomain、observationEpoch、revision、changeId，且changeId.commitDomain等于该生产commitDomain；external_source_version/2包含entityRef、commitDomain、observationEpoch、externalSequence；生产域可以不同于当前operation域。SourceObservation/1以observerDomain等于operation CommitDomain、entityRef等于sourceVersion.entityRef以及当前fileObjectBinding、evidencePins、control、Registry、incidence和cut保护当前操作，InputDescriptor/2.sourceInputs[].observation承载该完整观察。Frontier/2只表示sealed causal/dependency prefix，不替代全集Query、Registry完整性或payload物化证明。D6不建立Record store，不用whole-namespace replacement扩大冲突/权限，也不把ordinary与strict|observed_only混为同一轴；structured/bulk/collection/Action/automation/Approval/Money不因本地UI或局部证明而获得weak保护。

D7负责去重Node result、typed/occurrence result、editable columns、creation/membership proof、
  frozen bulk targets、paging/revocation；Record分支必须完整退役，join/group row不默认可编辑Node。

D8五表面必须区分删table row、删Field fact、移出collection、Trash；展开multi-value summary，保留conflict Draft，不能用grid row index写入或因Mobile容量改变语义。

D9显式CSV/Excel/JSON mapping、loss、fresh import、finite batch、Office三类binding分域；
  不执行external formula、不implicit upsert、不让ordinary Node持久export-only schema。

D10 external source authority/cache分域，SourceBinding与schema/contribution真实性保持；connector cache不是受管Record author store，也不建立另一CRUD/reference域。

### 8.2 旧 Record 分支必须成套退役

未来D7/实现不得只删一个records字符串后保留旧语义。必须同时消除：
- RecordCollectionRef/RecordRef及其TypeSpec/Query参数；
- records scan domain、recordCollection selector、record schema identity；
- record/node relation专支；
- row.record.fields等Field/CEL overload；
- collection UUID和record compound-ref equality/group/distinct/cache/export专支；
- persistent Record occurrence/provenance seed；
- record schema权限/read-set/lens/action target/View passthrough；
- 对应decoder、capability、fixtures、locale/API aliases；其中成套退役的是新的 active Record 执行域及其专用 API/capability/aliases，不机械删除已经承诺用于 saved decision、receipt、unknown recovery 的原版本 decoder 与原 bytes。旧 D5、D3、D6 wire1 和 D7 PreparedActionBinding/1,/2 的有效历史恢复继续按各自原协议解释；这些历史 decoder 不能恢复当前 Query/strong Prepared、approval 或 Money 资格，也不能仅凭缺少部署证据就把 prototype 自动提升为全 active compatibility。

必须保留的通用语义是NodeRef和普通TypedValue准确类型、D4 FieldId/RegistryBinding、通用authorized Query envelope、准确target/revision重解析、derived row无durable identity和ActionEvidence分域。

### 8.3 native table完整检查

必须覆盖：
- pipe/backslash escaping；
- CRLF/CR/LF；
- duplicate rows；
- ragged table；
- blank/comment trivia；
- empty table；
- stale locator；
- valid Inline/ref；
- unrepresentable text；
- untouched bytes exact；
- failure zero author write。

structured patch后必须重parse完整proposed source，并同时保留内层 production revision/selector 与外层 current SourceObservation/1、SourceVersionRef/1.sourceToken、fileObjectBinding、evidencePins、control、Registry、incidence、cut 和 structural strict install 资格；I、hash 或相同行文字不能补造这些证明。任何可确定的 source CAS、当前观察、授权、依赖或 durable-install 资格失败都必须在作者写入前 reject 或 conflict-reprepare，不得部分保存。若 install/ack 结果未知，则必须进入真实 recovery_unknown，并按原 operation 对账实际结果；不能伪称零副作用、Saved，也不能把未知结果当成新的 operation 重试。

### 8.4 Collection creation/membership

必须覆盖：
- path predicate与普通property predicate在move后的差异；
- empty collection默认parent；
- ad-hoc Query必须显式parent；
- Template conflict；
- append ordinal并发；
- filter为true但Query top/limit排除fresh Node；
- definition/params/schema/auth变化使旧plan stale；
- requireMembership true/false；
- page/paging/cache与semantic membership分开。

### 8.5 Field 与并发

必须覆盖：
- 两个相同phone值但notes不同；
- 三条历史assertions；
- same value different occurrenceKeys；
- external copy key collision；
- multiple carrier blocks；
- unknown namespace；
- deleting last required occurrence；
- inverse UI必须写真实canonical owner；
- D4 selector/read-set稳定。

并发fixture：A修改一个name note，B新增phone，C只有phone权限。不能用whole namespace覆盖；old revision plan不能直接重放；replan后不相关facts必须保留；无法证明时保留conflict proposal。delete+readd same key ABA不能误命中。

### 8.6 lifecycle 与 cross-Workspace

同一Node在多个collections只代表同一identity。hidden subtree必须进入Trash preview；root Trash拒绝；restore保留原identity；copy fresh。

Workspace transfer的target copy与source Trash是两个独立receipts。target失败不动source；source Trash失败显示target copied / source retained。不得构造D5跨Workspace原子move。

### 8.7 scale、limits 与 batch recovery

target fixtures必须覆盖999/1000/1001，grid page 199/200/201，wide row触发更窄bytes/dependencies预算；Template descendants计入import count。loaded count从不等于total，禁止partial-success冒充complete。

大型import未来验收至少：
- 10,000 Nodes；
- 10 batches；
- workspace input > available working memory；
- crash before/after batch 6 commit；
- process restart；
- network loss；
- repeated resume。

每batch原子；job准确报告partial committed state且不能重复create。当前文档不声称这些产品测试已通过。

### 8.8 ICS 与 recurrence

必须覆盖：
- series rule/override；
- infinite recurrence with bounded query；
- external subscribe/sync 与 adopt/import分开；
- same UID under different SourceBinding；
- cancel/delete/unbind/reimport/offline/timezone；
- no Record/implicit occurrence Node materialization；
- VFREEBUSY/VTIMEZONE zero Node；
- VTODO→Task和VJOURNAL/VEVENT显式mapping。

SourceBinding只定义foreign-key比较域；active OriginBinding才关联ForeignIdentityKey到NodeRef。UID/RECURRENCE-ID/LogicalOccurrenceKey都不成为NodeRef。

### 8.9 Query / Office 与 zero-row语义

必须覆盖：
- empty result仍有schema；
- nonterminal zero-row page不当EOF；
- join/aggregate columns默认read-only；
-普通Document table、Node collection、Query value table的Office binding分域；
- ordinary Node零export-only持久配置；
- legacy Record token明确拒绝。

partial collection export标partial或complete-export拒绝，不能静默截断到transport page。

### 8.10 terminology 与五表面一致性

D5 controlled names唯一归属；Document Table/row/cell、Node Collection、Field occurrence、derived row、Resource preview分域。退役API成套删除，不能误杀合法用户内容、历史prose或第三方format。

Desktop/WebUI/Server/CLI/Mobile在同一Core语义下处理上述意图；surface差异只影响interaction/capability，不改变identity、membership、limit或error语义。

### 8.11 D6-FA-r01新增回归

在上述 fixed-S matrix 上额外覆盖：
- ordinary 与 strict|observed_only 是独立两轴；offline native structured cell/row/column/reorder 在无关 I 未完成时仍可凭完整真实局部证据保存，但使用 strict 保护，不因缺少无关全集 Query 或 UI 形态取得 weak 资格；
- 人工 raw 整源保存仅在 actual A §4.1 全部资格满足时可选择 observed_only：trusted interactive_source_save、恰一个 existing live Document、ordinary + replica_local、完整 source read/replace、write set 为空或仅该 Document、无 applicable body/Field/Node-control deny、无 identity/parent/order/lifecycle/sharedPolicy/Registry/Calendar-scope/other-entity mutation，并且 DraftBase 等于 selected current Observation；必须在 planning 开始前人工明确选择并冻结 profile；
- 任一 weak 资格缺失、strict 失败、known competition、stale Base、watcher gap 或 continuity gap 都不得 fallback 为 weak，并要求 reject 或 conflict-reprepare；unknown install 保持 recovery_unknown，prepare 只代表 proposal/read-before/pins 等准备证据，不是 Saved；
- observed_only 路径验证已读前像 B 与用户输入 N 的耐久保留；安装 N 可以覆盖从未观察到的外部 C，后来 C 也可能替换 current file，但不得因此丢弃已耐久的 B/N；
- structured/bulk/collection/promotion/Action/Automation/server checkpoint/Approval/Money 全部保持 strict，不因本组验收条款放宽；
- SourceVersion/2 stale/ABA，以及生产版本相同但当前 SourceObservation/1、fileObjectBinding、evidencePins、control、Registry、incidence 或 cut 已改变的情况；发生 watcher gap、replacement 或 discontinuous rematerialization 后，即使生产版本相同，也不能继续认可旧 SourceVersionRef/1.sourceToken 或 Locator；
- multi-replica same/different-row conflict；
- explicit three-way merge proposal；
- semantic_pending(collection)只表示缺少全集 collection proof，不表示 empty，不洗掉 typed invalid、source invalid 或缺失 strong evidence，也不喂 Action、all_result、bulk 或 Automation；
- rebuild I不恢复old proof；
- lost P不恢复batch decision/approval/Money；
- new replica保留NodeRefs但没有execution custody；
- r5 pending replay与r6 current完整结果分开，r6只证明其current Observation/SourceVersion/cut且不修改r5历史receipt原bytes；
- Server多人table/collection Draft并存而commit仍由同一持久边界排序。

这些均是未来可复核验收条款，不表示产品测试已经执行或通过。

### 8.12 接受边界

这些是未来可复核合同案例。mechanical preflight、文档CI、作者自查、历史Pro审查都不等于D6–D10、A2或产品host实现验收。

旧D10 B13仍REVISE、术语/双语FAIL、11 OPEN。
