---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：b80e6c6f-648a-46f9-b5eb-8235f237b5f6。

候选状态：D7 文件权威 PL-IR-01 修复消费者；未独立接受、未激活、未实施。固定 S 的历史标签/证据只保留原范围。本候选消费 D3/D6 稳定生产地址/current Observation 作者修复；该 exact 修复仍待独立复核，不自行关闭 P1。新增 D3 conflict preparation 与 D7 /3 binding 保持各自独立联合复核门。

# D7 预览与完整效果运输


这是D7准备动作、D8明确编辑适配器与D3/D6原decision的只读运输，不产生第三ledger。preview明确表示拟议效果，committed表示原decision已保存的效果；不能将fresh symbolic subject包装为已提交Ref或将preview伪造为D3 receipt。本文件是Execution §8的完整规范正文。

## 1. Manifest与打开入口

Core保存完整、不可变的语义效果和每个字节slot的exact pinned payload；这份同decision证据不含会随交付重签发而改变的授权含义。`EffectManifest/2`是该完整语义效果在一个交付epoch中的closed运输投影，exact `{format:"weftext.effects",version:2,phase,protocolOwner,operationId,workspaceRef,profile,items,decisionKey:DecisionKey/2}`，phase=`preview|committed`，profile=`full|owner_fields`。语义items、完整pins及该epoch所有EffectBytes投影在任何首page前已形成、验证完毕且immutable；取页不能再执行Query、源变换、D4 gate或追加效果。preview绑定D7 PreparedActionBinding/3（历史/1、/2 仅按原规则恢复/重放），或D8 Editor Interfaces定义的PreparedEditBinding/2；D8 producer严格只允许protocolOwner=D6与profile=full，并由完整D6 PreparedIntent绑定原request、前后源、scope、依赖与预算，不伪造D7 ActionSpec/bindingToken。两类producer都须满足本文件完全相同的完整性/授权/epoch/期限门，D8准备成功使用d8_edit_prepared承载同一header及tokens；committed绑定原D3/D6相同OperationId、原request、完整plan、receipt及原子D6 companion。transport tokens不是作者source、结果身份或额外授权。 header 的 decisionKey 与 request、record、OperationId、Workspace 和 protocolOwner 全部相等；CommitDomain 是原决定域，生产 sourceVersion 的域可以不同。D8 当前完整 PreparedEditBinding/2 必须按其冻结 Interfaces §4–5 解码，request=d6_commit_request/2、preview=this D6/full header，嵌入原 PreparedIntent/2；D8 没有 D7 bindingToken，也不消费 D3 ConflictInstallInput。

D7 prepare或D8明确编辑prepare返回previewToken（tag action_preview）及previewCursorToken（tag effects_cursor）。原D6 receipt已有effectsToken；D3 receipt保持自身closed形状，通过新closed读取入口取得其原companion token：

- `d7_effects_resolve` exact `{wireVersion:2,kind:"d7_effects_resolve",protocolOwner,request}`。request是原完整D3 wire12请求或D6 commit request，protocolOwner必须相符；不接受只有OperationId的ledger探测。按该原协议的decode/current authorization/authority/custody/原key与fingerprint gate做**只读**查找，绝不执行未完成操作或新建decision。只有当前获准读取完整manifest、原key为committed且request相等才返回其已保存effectsToken。planned/rejected/terminal/unseen在已授权之后统一effects_unavailable；更早原权限/continuity门仍优先，错误映射下节。不得因为resolve读取去创建token所指的另一份临时效果。
- `d7_effects_resolved` exact `{wireVersion:2,kind:"d7_effects_resolved",effectsToken}`。token来自原同decision保存值，反复resolve不会生成不同语义清单。
- `d7_effects_open` exact `{wireVersion:2,kind:"d7_effects_open",effectsToken}`；成功`{wireVersion:2,kind:"d7_effects_opened",effectsToken,cursorToken,manifest}`。manifest为除items外的上述完整header，phase只能committed。preview header随prepared响应新增`previewManifest`，同形但phase=preview。header也可能披露scope/operation，须先完整授权。
- `d7_effects_page_request` exact `{wireVersion:2,kind:"d7_effects_page_request",token,cursorToken,pageSize}`，token仅action_preview/effects；pageSize=1..200。成功`{wireVersion:2,kind:"d7_effects_page",token,items,terminal,nextCursorToken?}`，terminal=true禁止next，false必有next。允许空nonterminal；完整消费者必须直到真正terminal，且逐item验证与原header一致，不按200条截断。

同一交付epoch重复同cursor/pageSize在当前资格下重放同item bytes；不同pageSize只改变运输切片，不改变清单。cursor绑定token/phase/manifest/位置/当前交付epoch，不能跨preview/committed使用；没有客户端offset跳过未看效果后声称完整审阅。完整manifest含原始效果全部items，细分profile由prepare前的先验证明唯一确定，不在交付时偷偷删项。

## 2. EffectItem闭集

subject逐字复用D3 PayloadSubjectKey中的existing/mapped/new entity三类，不接受artifact作为被修改对象；committed item须用existing subject持实际结果Ref，preview可用符号subject。source是某对象的作者bytes；生命周期/结构/控制单独列项，不能靠“源没变”漏掉它们。

| item exact shape | 语义与完整性 |
|---|---|
| `{kind:"source_change",subject,before:SourceImage,after:SourceImage}` | 覆盖所有fresh源及确定的源变化；committed恰为实际 source-state 变化；只有 bytes/value 与 source-state 均保持的 true no-op 才不列。相同 raw 的 external→managed 或 D3 canonical claim/生产版本 admission 仍列真实 source_change；preview中existing C源改用下一variant，不重复列；before/after完整，允许absent |
| `{kind:"conditional_source_change",subject,before:SourceImage,result:{payloadKind:"exact_source_document",bytes:EffectBytes,revisionRule:"preserve_if_same_state_else_current_domain_h_plus_one",versionBasis:D7ProposedVersion/1,forceAdmission:bool}}` | 仅full preview的existing Node且before为完整present；恰覆盖所有含C的existing结果源；不是actual edit，规则见下段；committed禁止 |
| `{kind:"entity_state_change",subject,before:EntityImage,after:EntityImage}` | 覆盖fresh、lifecycle、placement、owner结构；纯move/Trash/restore即使source revision不变也有项 |
| `{kind:"d3_plan",request:D3Request}` | 仅preview，完整原请求/所有十数组及symbolic mapping；不是已planned receipt；一个D3 preview恰一项 |
| `{kind:"d3_receipt",receipt:D3Receipt}` | 仅committed，原完整byte-equivalent receipt；一个D3 committed manifest恰一项 |
| `{kind:"semantic_extension",format,bytes:EffectBytes}` | format闭集d4_relation_copy_effects/1、d4_source_materialization_effects/1、d7_definition_transfer_effects/2、d3_canonical_effects/1；仅committed的完整extension，不能截断owners/facts/entries；preview的完整作者变换由source_change、conditional_source_change与原D3 C/Q计划表达，不伪造最终Ref版extension |
| `{kind:"period_scope_change",subject,before:PeriodBindingImage,after:PeriodBindingImage}` | 原D6唯一派生控制scope，subject必须Node；purge删除/新period建立/managed copy均覆盖 |
| `{kind:"series_configuration_change",seriesScope,before:SeriesConfigImage,after:SeriesConfigImage}` | 同D6完整seriesScope；仅原请求允许的完整fork/bootstrap/fresh scope初始化或独立已授权管理结果 |
| `{kind:"workspace_bootstrap",bytes:EffectBytes}` | 完整D6控制计划：profile/3用WorkspaceBootstrapPlan/2；真实已签发的profile/1或/2 family用原Plan/1，均含其准确policy及Registry/config/binding；preview的fresh NodeRefs按下段symbolic encoding呈现 |
| `{kind:"authority_change",before:AuthorityImage,after:AuthorityImage}` | create/fork/continue当前authority及预期切换；preview的未分配continue authority明确pending，不捏造ID |
| `{kind:"field_change",ownerNodeRef,fieldId,beforeVersion:SourceVersion/2,afterVersion:<SourceVersion/2|D7ProposedVersion/1>,before:EffectBytes,after:EffectBytes}` | 仅owner_fields；两个bytes解码完整FieldEntryImage/2，实际版本与源相等 |
| `{kind:"conflict_branch_source",head:ChangeId/1,subject:EntityRef,sourceVersion:SourceVersion/2,payloadKind,bytes:EffectBytes/2}` | 仅 D3 resolution full preview 的完整已读 head source；细则见 §6 |
| `{kind:"conflict_resolution_change",conflictId:ConflictId,before:ConflictRecord/2,after:ConflictRecord/2,selection:D3ConflictSelection/1,canonicalStates:[D7CanonicalState/1...]}` | 仅 portable D3 resolution 的完整冲突与 canonical 控制改变；preview/committed 同一语义，细则见 §6 |
| `{kind:"canonical_plan",plan:D3CanonicalPlanProjection/1,payloads:[{entityRef:EntityRef,before:EffectBytes/2,selected:EffectBytes/2,result:EffectBytes/2}...]}` | 仅 D3 resolution full preview/committed；完整公开 canonical 计划与对应全部 bytes，见 §6.1 |

当前 SourceImage/2 closed 三 variant：`{state:"absent"}`；`{state:"present",sourceVersion:SourceVersion/2,payloadKind,bytes:EffectBytes/2}`；`{state:"proposed",versionBasis:D7ProposedVersion/1,payloadKind,bytes:EffectBytes/2}`。payloadKind 只允许 exact_source_document/resource_bytes/annotation_value。present 是真实 before 或 committed after，完整 sourceVersion 与 subject 实体相等；它可能为普通真实 managed/external 前像，但 committed 新 managed after 必须来自原 seal，不能把外部 sequence 当 revision。proposed 只用于 preview after，绝不是未 seal 的 SourceVersion，也没有伪造 ChangeId。Document 是 exact UTF8、Resource 是 exact binary、Annotation 是完整 D3 Annotation-Value/3；symbolic Result/9 编码仍不能当可解析的最终 source。

D7ProposedVersion/1 的 closed 两 arm 为 `{kind:"existing",sourceStamp:SourceStamp/1}` 与 `{kind:"fresh",decisionKey:DecisionKey/2,subject:<mapped-or-new PayloadSubjectKey>,revision:1}`。existing 必须对应同一实际 existing source 的原 D6 /1 或 conflict-only /2 拟议 stamp，revision=该生产域连续历史 H+1，生产 observationEpoch 来自目标安装世代，不抄 foreign before。fresh 在 final Ref 尚未由原 stage12 产生时只表达原 fresh allocation 的完整空历史后置要求，不谎称已有完整 stamp/epoch/Ref；其 symbolic subject 必须逐字等于 item.subject，decisionKey 与 header 相同，revision=1 只有 winning candidate 真正 fresh 且其生产域空历史得到完整证明才可成立。实际 stage12/14 把这个唯一 subject 映射到原 candidate，得到并冻结真正 /1 SourceStamp；不能换成别的 ID、重采 H 或预先公开 candidate。candidate 不满足完整空历史即原计划失败，不静默调整 revision。present 保留生产域/生产世代；任何 proposed 不能当 current source、portable Locator 资格或已提交版本交付。

EntityImage先按subject entity kind区分：absent=`{state:"absent"}`；purged=`{state:"tombstoned"}`；Node live=`{state:"live",placement:{parent:NodeSubject|null,ordinal:Counter}}`；Node trashed=`{state:"trashed",trashPlacement:{trashParent:NodeSubject|null,trashOrdinal:Counter,restoreLocation}}`，restoreLocation exact为`{kind:"original",parent:NodeSubject,ordinal:Counter}`或`{kind:"unavailable"}`，从原D3 restoreLocation机械投影；显示kind按以下闭表机械映射，原receipt内kind/成员/bytes不变：ordinary `original_location(parentRef,ordinal)`→`original(existing parentRef,ordinal)`；fork preview `mapped_original_location(parent,ordinal)`→`original(同一mapped parent subject,ordinal)`；fork committed `mapped_original_location(parentRef,ordinal)`→`original(existing mapped结果parentRef,ordinal)`；`unavailable_original_location`→`unavailable`。父Ref/subject和ordinal完全保留，不回退source parent/root/null；此映射同样适用于trash后像、restore/purge前像和surviving Trash结构效果。Resource/Annotation live/trashed=`{state:"live"|"trashed",owner:NodeSubject}`。父/owner在committed必须existing真实Ref，preview允许同plan mapped/new；root/null只按D3原sentinel规则合法，不赋予null新的D7一般值含义。Annotation reply是其完整source字段，同时须与原D3 structural/references effects相等。

PeriodBindingImage是`{state:"absent",revision:Counter}`或`{state:"present",scope:EffectScope,revision:Counter}`。EffectScope是D7只读投影，exact为`{kind:"workspace",workspaceRef}`或`{kind:"node",subject:NodeSubject}`；它从D4 workspaceId/scopeNodeRef机械映射，preview可在subject中使用已绑定fresh symbol，committed只能真实existing Node subject。它不是D4作者scope或D6控制请求的替代wire。revision保留原独立控制防ABA含义。SeriesConfigImage同样absent/present，present另有multiplicity=`unique|many`；本运输的seriesScope是exact `{series,scope:EffectScope,policyBinding}`，series及policyBinding逐字复用D6原七成员identity与完整三方绑定，只有scope机械投影为EffectScope，不能只给名字。Control generation和实际依赖在内部完整保存，不能把只读projection当另一可写配置源。

AuthorityImage是`{state:"absent"}`、`{state:"active",authorityInstanceId,authorityGenerationToken}`、仅create/fork preview可用`{state:"proposed",authorityInstanceId}`或仅preview可用`{state:"pending_continuation",sourceWorkspaceRef}`；IDs/tokens使用原D3域。current authority状态披露须原权限，committed绝无pending/proposed。Bootstrap已有proposal给出的targetAuthority与Workspace不受content fresh identity影响；不能在preview调用continue的stage12来提前分配AuthorityInstanceId。

`FieldEntryImage/2` exact `{format:"weftext.field-entries",version:2,ownerNodeRef,fieldId,sourceVersion:<SourceVersion/2|D7ProposedVersion/1>,entries}`，entries 为原 author order 的完整 `{occurrenceKey,rawEntrySource}` 数组，非差异片段。before 用真实完整 production SourceVersion，preview after 用本节 existing proposed basis，committed after 用原 CP3/receipt 对应的真正 sealed 版本；对应 field_change.beforeVersion/afterVersion 逐项相等。每项原 D4 inner occurrence/revision 仍在同一完整源及版本基础内验证，raw 不重序列化，未选条目显示且逐字不变。它是明确 Field projection，不是 exact SourceSnapshot；runtime 当前资格另由完整 SourceObservation 证明，不把 stable production version 当 current token。

### 2.1. 条件源物化的完整预览

conditional_source_change不接受用户predicate、任意脚本或采样出的Ref。subject必须是原请求已声明的existing Node；before是其同一绑定 preimage 与完整生产 SourceVersion/2；result.bytes的encoding仅d3_symbolic_result9，必须逐字等于原D3 payloadBindings所绑定的该subject完整Result/9，完整解码后至少含一个C，所有C.container及Registry/源绑定与原plan相等。每个含C的existing结果源恰一项；fresh源仍用 source_change 与上述有明确空历史前提的 fresh proposed basis。此项同时覆盖C涉及的全部潜在owner/迁出/迁入/原位及非carrier原bytes，不能只给delta或某一map的输出。

准备不产生最终content ID，也不选一个“典型”map。消费者显示完整符号源变换、完整 before 与拟议生产域/versionBasis，并明确现存源可能保留原生产版本或在当前生产域 admission；不能把baseCarrier/占位骨架显示为确定的最终源。最终仅使用原stage12的同一个Core candidate map及原C物化算法，先经原全部D2/D3/D4规则，再将完整结果UTF8 bytes与before比较：forceAdmission=false 且最终完整 bytes/value 与 source-state 都保持时保留原完整生产版本，committed 不列 source_change；否则消费原计划冻结的当前生产域 H+1 versionBasis，在同一 seal 后恰列一个完整 actual source_change。forceAdmission 只由真正 owner 从 external→managed admission 或 D3 canonical claim/selected production version 改变推导，不能由用户选择；为 true 时相同 raw 也发生一次真实 source-state admission，为 false 才允许完整相等分支。versionBasis 对 existing subject 只能是 existing arm，与原被比较 H/lastIssued/目标世代及 exact after pin 相等；不能用 before.revision+1 或 selected historical head 的 revision 捐号。任何原语义/预算/溢出/最终CAS失败均沿原原子失败规则，不部分发布或重新采样挑选分支。

由此选择得到的actual-only committed集合必须与原receipt、D4SourceMaterializationEffects及实际源独立相等；D4内部效果按原规则保留C的no-op owner，不把它假造为public source edit。条件项从preview消失或精化成actual source_change是本variant唯一明示的集合变化，不是允许commit追加未预览效果。preview语义、原字节与准备记录都不因此改变。含C但可证明所有map均修改的existing源仍使用同一variant，避免由实现自行选择不同wire。

## 3. 拟议字节和独立EffectBytes

EffectBytes/2 exact `{handleToken,encoding,byteLength}`。tag=`effect_bytes/2`，不能拿D6 resource_bytes/1 ByteHandle、Query result或effects cursor替代。record绑定manifest token+phase+item ordinal+该kind的唯一字节slot角色（如before、after、result或extension）+交付epoch、exact pinned bytes、encoding、audience/observation scope、有限读取预算、期限/epoch。Resource原D6 ByteHandle继续其旧合同；新handle可以承载fresh/proposed二进制而不伪造已提交ResourceRef。

encoding闭集：`exact_source_utf8|resource_bytes|d3_annotation_value3|d3_symbolic_result9|d4_relation_copy_effects1|d4_source_materialization_effects1|d7_definition_transfer_effects2|d3_canonical_effects1|d6_workspace_bootstrap_plan1|d6_workspace_bootstrap_plan2|d7_symbolic_json2|field_entries2`。这些encoding唯一决定完整decoder，不允许generic_json或任意format dispatch。ordinary exact数据按相应完整原decoder验证；D3 symbolic输入按Result/9，consumer逐B/M/N/E/S/C/Q分区以symbolic subject显示、审阅全部占位关系及原bytes，不能自行分配UUID后冒称author source。

`d7_symbolic_json2`只用于本节preview Bootstrap中具有fresh refs的完整closed control对象。内容exact `{format:"weftext.symbolic-effect",version:2,payloadFormat,template,slots}`，payloadFormat闭集为d6_workspace_bootstrap_plan1|d6_workspace_bootstrap_plan2，由受保护family及其完整真实计划decoder决定，不允许调用方降级选择；template为对相应schema按typed Ref位置机械替换成`{kind:"symbolic_subject",subject:NodeSubject}`的完整对象，slots为`{path,subject}`列表；path的表示与排序逐字采用同包《Definition Transfer》§2的唯一typed-path规则（member text或Counter index的数组，text UTF8 rank0、index数值 rank1、共同prefix后短路径在前，故index 2先于10）；slots必须与所有替换处一一对应。不得允许literal位置出现placeholder或普通object冒充Ref。Core按原plan candidate map对完整模板替换后必须通过原D6/D4 decoder；最终committed encoding使用真实原格式而非symbolic。这个只读展示模板不能作为修改请求或证据输入返回Core。真实已经签发且固定profile/1或/2的family，仍可按原Plan/1完成原授权replacement及尚未提交的create/fork。当前运输显式支持这条真实family分支及其saved/planned/unknown恢复；不得把Plan/1改标为Plan/2或插入Policy/3，也不推定其它原型均已部署。新profile/3产生Plan/2。BootstrapPlan/2 的所有字段逐字采用 D6 Control §10.2 实际 decoder，initialPolicy 为完整 Policy/3，profile 为冻结 family 的 /3 副本；creatorBinding、target Registry、series configurations 与 period bindings 不被 preview 裁剪。目标 W/B 仍由原 D3 proposal 决定，issuer A 暂管不改变 DecisionKey，也不增加 targetDomain 成员。

`d7_effect_bytes_read` exact `{wireVersion:2,kind:"d7_effect_bytes_read",handleToken,offset,maxBytes}`，offset Counter，maxBytes1..1048576。成功`{wireVersion:2,kind:"d7_effect_bytes_chunk",handleToken,offset,totalBytes,data,terminal}`，data为canonical无padding base64url；decoded长度恰min(maxBytes,totalBytes-offset)，terminal恰offset+len=totalBytes；offset==totalBytes可空terminal，超界cursor_invalid。完整消费者必须验证0..totalBytes无洞且使用header encoding完整decode；seek最后一byte不是完整审阅。I/O short read、pin缺失、epoch/资格变化不返回部分data或假EOF。

完整D4 extensions可能包含全部raw source，必须按full profile交付其整份bytes；不能将其切成owner_fields并仍使用原format。窄profile的field_change由完整已验证原扩展/源独立推导，只在下一节证明它覆盖本action全部作者效果时合法。原内部完整D4证据仍同decision保存，并不因窄公开投影被丢弃。

## 4. 先验资格、错误与完整性

full profile要求整份所有before/after源、适用lifecycle/结构/控制及semantic extension的当前完整读取/ObservationScope资格；未知或读不全必须在prepare前拒绝，不能等用户看到一半preview再隐藏某item。D3 bootstrap preview依据原prepared_workspace issuer资格仅披露本次明确创建内容；committed额外target源/控制读取仍须当前target权限，原D3 receipt重放继续它自己的特殊边界。

owner_fields仅用于《D7 Narrow Field Qualification》静态证明成功的单owner Field动作。实际作者footprint恰为已授权Fields，identity/lifecycle/placement/其它source/控制值不变；只列完整field_change。所有D4全源语义效果保存在内部，但public field projection必须独立证明覆盖全部实际修改，不能隐藏另一个Field或结构变化。beforeVersion绑定真实preimage，afterVersion绑定最终源版本并与原receipt.sourceVersions中该owner的实际修改记录一致。只列该Field完整author image确实变化的项；只有完整raw字节/值与source-state均保持的true no-op，field_change集合与原receipt.sourceVersions才均为空；相同字节的source admission不属此例。若其真实效果无法由先验合格的owner_fields完整表示，按下述范围规则拒绝准备，不隐藏admission、不默切full，也不伪造Field修改；sourceEnvelope与commitSequence的观察成本须已有明确资格。若实际footprint扩大或有无法投影的效果，整个prepare拒绝，不在commit后才发现preview不完整。

完整性总门：preview由原前像、完整proposed/Result9及原plan独立恢复全部确定/条件作者效果，committed由同一真实candidate map、实际源及原receipt独立恢复actual效果；比较完整item集合和各payload bytes，不让effects自称覆盖范围。多余、遗漏、重复、相互矛盾或错误phase均拒绝；条件项只按§2.1精化，不能据其存在豁免其它entity/control/source效果。所有blob完整decode后才可宣称完整审阅。

唯一排序与重复规则如下，先rank再secondary key。数组必须已经按此顺序，不在消费者端悄悄修复：
| rank / kind | secondary key与cardinality |
|---|---|
| 0 source_change | 原D3 subjectCanonicalKey；每subject≤1 |
| 1 conditional_source_change | 同subjectCanonicalKey；每subject≤1，且与source_change互斥；仅preview |
| 2 entity_state_change | 同subjectCanonicalKey；每subject≤1 |
| 3 d3_plan / d3_receipt | 无secondary key；D3 preview恰一个plan，D3 committed恰一个receipt；D6两者禁止 |
| 4 semantic_extension | 固定format rank：d4_relation_copy_effects/1=0，d4_source_materialization_effects/1=1，d7_definition_transfer_effects/2=2，d3_canonical_effects/1=3；每format≤1，仅committed；各适用format按原协议要求恰一份完整operation级扩展，不按owner拆项 |
| 5 period_scope_change | Node subjectCanonicalKey；每subject≤1 |
| 6 series_configuration_change | 完整seriesScope按D3-CJ/3编码后的UTF8 bytes；包含七成员series、机械投影后的完整scope和完整policyBinding；每完整key≤1，不按seriesKey、label或handle排序 |
| 7 workspace_bootstrap | 单Workspace操作级singleton，适用时恰一项 |
| 8 authority_change | 单Workspace操作级singleton，适用时恰一项 |
| 9 field_change | 完整owner NodeRef规范key，再完整FieldId规范bytes；每(owner,FieldId)≤1，仅owner_fields |
| 10 conflict_branch_source | 完整 ChangeId 原 comparator，再完整 RefKey；每 (head,subject) 恰一项，仅 full preview |
| 11 conflict_resolution_change | 单 conflictId singleton，只有本次 portable resolution 适用 |
| 12 canonical_plan | D3 resolution full profile singleton，preview/committed 均恰一项，true no-op 也保留空计划 |

其它singleton是否适用由原mode/控制合同及完整实际效果决定，不能为凑数添加空或假效果。每个kind的全字段、subject域、phase、前后像和mandatory范围仍须验证；去重后数量相同不能代替集合相等。

错误exact `{wireVersion:2,kind:"d7_effects_error",code}`，code闭集`invalid_request|not_visible|preview_expired|reset_required|cursor_invalid|effects_unavailable|budget_exceeded`。所有入口固定：closed decode→当前tag/audience与全manifest/profile授权→preview期限（committed不受preview TTL）→交付epoch/continuity→cursor/range→pin/decoder/预算→完整输出前current gate。missing/wrong-tag/wrong-audience/撤权均not_visible；continuity不可证明或缺完整pins为effects_unavailable；已证明epoch失效reset_required。原resolve的authority/custody不可证明映effects_unavailable，原key/fingerprint冲突在当前资格后同样effects_unavailable，不返回另一个request的信息。任何失败无items/data/terminal。

committed effects保持同decision耐久语义，不因原preview过期消失。每次成功d7_effects_open在当前完整资格与有限预算下，从原effectsToken所指不可变语义清单及完整pins建立一个新的完整交付epoch；在返回header/cursor前为每个EffectBytes slot固定该epoch的handle并完整验证投影。后续page只切片这份投影，不边翻页边换handle。重复打开可产生不同transport随机bytes，但原effectsToken、receipt、语义item顺序、encoding、byteLength及exact payload不变，不重跑Query/变换/作者effects。保存的原语义证据和既有epoch投影都不原地修改。

每个cursor/byte handle仅在自己epoch、phase及audience下有效；新epoch不自动撤销先前仍在自己期限内的交付。旧epoch到期后按原错误总序返回reset_required，不能借新open让旧cursor继续或混入新handles。全部committed pins按原decision耐久保留；缺失或continuity不可证明为effects_unavailable，无部分items/data。preview在prepare产生一份完整初始投影并由原PreparedActionBinding或PreparedEditBinding不可变保存；不因新交付延长原preview TTL，原record和原request绑定不变。preview取消/超时不回滚已committed事实；撤权遮蔽交付，不改原decision。

## 5. D3模式覆盖

| 原mode | 必需preview/committed效果 |
|---|---|
| create_workspace / fork_workspace | 完整d3_plan/receipt、全部fresh source与entity结构/两态、原Bootstrap Registry/policy/config/bindings、authority；commit包含适用D4与D7全部extensions；preview显示其完整作者源C/Q变换及适用conditional_source_change |
| create_node / create_resource / create_annotation | 全部primary/member fresh与compound existing源（含C者使用条件项）、placement/owner及period控制；互相引用用Result9 symbolic完整表达 |
| copy_node_subtree / copy_resource / copy_annotation | 完整身份映射、全部选中与省略处置、新建源与结构、必需的新范围配置、提交后的 D4 复制及物化扩展与 D7 定义转移扩展；预览提供完整 C/Q 源变换 |
| continue_workspace | 完整d3_plan/receipt与authority_change，源不变不假造source_change |
| move_node / reorder_node | 完整entity_state_change与原结构计划/回执，未改源不增加source revision |
| trash / restore | 全部实际closure的entity状态/placement和reference生命周期回执；有cleanup时才列源变化及对应D4 effects |
| purge | 完整tombstone/源absent、scope binding删除及原D3回执；不省略closure成员或隐式清控制引用 |
| import_new | 全部artifact fresh分区→fresh subjects/ownership/placement，ordinary-format existing分区及companion按原§4.1.3a完整列源效果；含C的existing源用条件项；完整mapping/loss dispositions、commit的适用D4/D7 extensions；preview完整C/Q源变换 |

十五个原mode共享这套有限类型。独立D6 policy/issuer/attempt管理变更不是D7 ActionSpec的kind，本版本effects入口对其不支持的完整控制清单在授权后返回effects_unavailable；不得将其包装成D7 Query rows。上述D7所允许的D3 mode带来的必要Bootstrap/period/fresh-scope控制效果已经纳入，不能用这项独立管理限制跳过。


完整预览与最终语义证明的区别：D4/D7 committed extensions是带最终identity/revision的验证证据；它们没有独立于已列source/entity/control变更的额外作者效果。preview在尚无最终映射时不发送假造extension，而必须完整发送所有现存前像、所有拟议Result9 C/Q原字节及subject归属、Node/Resource/Annotation/结构/控制变化；消费者显示每个原Entry迁出/原位/迁入及typed reference替换，不能仅显示payload digest。commit后最终extensions逐字证明这些预览模板在原candidate map下的实际物化，并与原receipt和源一致。条件C的existing源按§2.1唯一规则确定是否实际修改及版本，除此之外任何extension带出预览未覆盖的作者变化，视为原准备与提交绑定完整性冲突，不是允许追加的效果。
## 6. D3 resolution 的完整分支与 canonical 投影

只有实际 D3ConflictResolutionPrepare/1 → 原 wire12 → 受保护 PreparedActionBinding/3 的 resolutionInput 可产生本节两类 conflict kind 和 canonical_plan；裸 d3_operation、D8/full 或 D6 source/policy resolution 不因此取得这些身份效果。新增 kind 使用本文件 current manifest/transport/EffectBytes 版本，不向旧 /1 闭集暗加 arm。

conflict_branch_source 只用于 full preview，subject 是完整 existing EntityRef，payloadKind 为 exact_source_document/resource_bytes/annotation_value 且匹配实体 decoder。它恰覆盖 D3ResolutionInput.branchEvidence.sourcePins 实际读取的每个 (head,subject)，无重复或省略；原 pin 原 bytes 逐字绑定对应 EffectBytes/2。sourceVersion 必须是该 head 完整 sealed history 已证明、对该 subject 有效的 managed SourceVersion/2，sourceVersion.changeId 可以是 head 的祖先，不能仅因相同 revision/hash 推断。head 不等于 sourceVersion 生产版本，生产域也不等于当前观察域。这是明确授权的历史分支运输，不签 current SourceVersionRef，不能将此 byte handle 作为普通 Query/Action/D8 source 资格。

conflict_resolution_change 在 portable resolution 的 preview 与 committed 中恰一项：before 为实际 open/resolution_prepared ConflictRecord/2；after 同 conflictId、完整 key、createdAtFrontier 与 supersedes，只按原 owner 转 resolved，不造 pre-seal ChangeId 或重开 history。selection 与原 D3ResolutionInput 完全相同，canonicalStates 由完整 original plan/pins 独立构造。已 resolved exact 同选择且全效果空的 no-op 不列该 item，但 D3 preview 仍有原 d3_plan，committed 仍有 d3_receipt。fresh copy 必有实际 fresh native 效果，不冒作 no-op。

~~~text
D7RestoreMembershipImage/1 =
{state:"absent"} | {state:"present",members:[EntityRef...]}
D7CanonicalState/1 =
{entityRef:EntityRef,
 installed:{head:ChangeId/1,image:EntityImage,restoreMembership:D7RestoreMembershipImage/1},
 selected:{head:ChangeId/1,image:EntityImage,restoreMembership:D7RestoreMembershipImage/1},
 result:{image:EntityImage,restoreMembership:D7RestoreMembershipImage/1}}
~~~

全部对象 closed，所示字段必需；canonicalStates 按完整 RefKey 唯一排序，恰覆盖 canonical 选择/物化的 existing 实体和实际改变的恢复 membership owner，不重复列普通 native fresh 结果。installed 来自真实物理安装 metadata 与相应 ConflictInstallInput/原 installation proof，不能以选定 head 的旧图像替代 actual before；selected 是真实用户所选 head 的语义图像，placement 只取其位置 basis、不因此回滚 source/claim/lifecycle；result 来自同一个最终计划。三幅 EntityImage 均使用本文件真实 closed union，必须是该实体相应 live/trashed 图像，不用 absent 或假 tombstone 填缺失证据。已证明独立 tombstone/omission 仍依 D3 原 closure，不被此投影复活。

restoreMembership 的 present 只用于该 Node 当前 active Trash 的完整 saved restore_membership；members 按 RefKey 唯一排列，来自原真实 Trash 决定及 D3 所选分支矩阵，缺证明不能填 []。live Node 和 Resource/Annotation **自身**此项为 absent，并不否认它们属于 owner Node 的 membership，也不删除保留的历史 pins。真正 empty membership 必须有完整空集证明。installed、selected、result 之间的 membership 改变都完整展示，不能只给 head、choice、fingerprint 或不可读 metadataPin。

有 entity_state_change 时其 before/after 与 installed/result.image 逐项相等；原 source_change/conditional_source_change 另运输实际 full source 前后与真实生产/拟议版本，branch_source 运输全部已读 head bytes，不用 canonicalStates 代替 source。相同 bytes 但 changed canonical claim/selected production version 的 source admission 仍给 source_change（若含真正 C 条件则依 §2.1 的 forceAdmission），CP3 before 取实际 wrapper.sourceVersion、after 取唯一真实 /2 source stamp + seal ChangeId。metadata-only open resolution 也有完整 canonical/control 投影；true resolved no-op 不再把旧 head 版本重新 admission 到当前 after。

portable Locator PL-IR-01 现已有稳定生产地址/current Observation 作者算法；Preview/Effects 只保留新 preparation 选中的当前 Observation，不把稳定 Locator 地址变成运行时证据。该 exact 修复仍待独立复核，未闭合 owner-plan/effects 整合门继续独立开放。

### 6.1 Canonical 计划、字节与公共语义扩展

D3 主文 §10.1.1.3 唯一拥有 D3CanonicalEffectPlan/1、D3CanonicalPlanProjection/1 与 D3CanonicalEffects/1。本 producer 在准备成功前完整解码 protected resolutionInput.canonicalPlan、验证两个分量及全部 original typed gates，再机械产生 canonical_plan；不能用实现方自报 effects 替代该 owner gate。plan 逐成员等于受保护完整计划的具名 pin→hash 投影，decisionKey/expectedKey/conflictId 与 request/header/selection 相符。canonicalStates 与 conflict_resolution_change 的同名数组逐项相等；true no-op 两处适用状态依 D3 精确规则，canonical_plan 的 states/lifecyclePlan/containers 均为空。

payloads 按完整 RefKey 排序唯一，恰覆盖 plan.containers；每项 entityRef 相等，before/selected/result 三个必需 EffectBytes/2 slot 分别运输 protected beforePin/selectedPin/resultPin 的完整 bytes，hash 等于 projection 的同名 sha256。before 是真实 installed，不换成 selected head。before/selected encoding 按 entity payloadKind 为 exact_source_utf8/resource_bytes/d3_annotation_value3；result encoding 恰为 plan.container.resultEncoding，Result/9 必须是完整 d3_symbolic_result9。payloadBindings 的 preimage/result hash 同时验证；未知/错 decoder、缺 role、空占位、重复实体、hash 相等却来源不符均不能准备成功。公共消费者可逐页及逐 bytes read 完整查看 actual A、selected B、全部 E/delete/result-only/S/lifecyclePlan 和 final source 条件，不依赖读取内部 PinRef。canonical_plan 是只读证据，不可转作新的修改请求或 current Observation。

committed 的 semantic_extension(format=d3_canonical_effects/1,encoding=d3_canonical_effects1) 在此 profile 恰一项，即使 true no-op 也交付已封存的空扩展。其 bytes 用完整 D3CanonicalEffects/1 decoder，requestFingerprint 对原完整请求、receiptSha256 对同 manifest 的完整原 primary receipt D3-CJ/3 bytes、decisionKey/effectClass/条件 changeId 对 receipt/companion/effectsToken 逐项验证。extension.plan 等于同 manifest canonical_plan.plan；sourceVersions 恰覆盖其 containers，before/selected 完整生产版本相等，after 等于 source_change 的 sealed managed after 与 CP3 唯一 /2 after；source 不变的 metadata-only/lifecyclePlan 无伪 sourceVersions。扩展四数组分别按原 D3 comparator/唯一性、一一 evidence-origin 规则独立重建，不能只复述请求。

原 d3_receipt 十二数组只属于 native component，canonical arrays 只在该语义扩展；同一历史 fromSource 各为 native mapped 与 canonical existing 的真实不同结果时可分属两个命名组件。不能把两个数组拍平成一个仍声称 fromSource 唯一的旧回执，也不能以另一 head 替换 actual before。所有物理 source/entity/control items 则从 actual installation 到 final cut 独立重建一次；D3 仅允许 exact before/final/version/evidence 等价的 existing overlap，alias 不产生第二 item、write、source version 或 CP3 entry。native receipt 的 selected-branch 逻辑前态与真实 installed 前态分别按各自 owner 比较，不把逻辑中间状态当第二次安装。

preview 不发送 committed extension 或虚构最终 Ref/ChangeId；完整 canonical_plan 与全部 byte slots 加原 native plan、source/entity/control/conflict items 必须足以审阅完整效果。committed 的 canonical_plan 保留原准备计划及 pins，与最终扩展及真实 after 分开。新 full consumer 必须同时验证两者、原 receipt 和其它适用 D4/D7 extension；未知格式/缺项/不可读字节不能按旧 receipt-only 宣称完整成功。prepare 时不完整则无 prepared 响应；真实 P seal 后缺交付 evidence 为 effects_unavailable，不能把已封存 commit 改成失败或触发新作者操作。所有 semantic bytes/pins 与原 primary receipt/companion 同 P 保留，epoch 只重建运输 handles；saved/planned/unknown 不重做 slot 对应、typed gate、head 选择或 H 分配，仍按原责任恢复。

## 7. 历史运输与完整 current 准入

真实旧 EffectManifest/1 exact `{format:"weftext.effects",version:1,phase,protocolOwner,operationId,workspaceRef,profile,items}` 没有 DecisionKey；其 wireVersion1 的 resolve/open/page/bytes/error 采用上述同名 kind 的原成员，解析原 D3/D6 request 版本，不补域、不把旧 request 改成12。旧 EffectBytes shape 同 handleToken/encoding/byteLength，tag=effect_bytes/1，只绑定原 manifest1/epoch。旧 SourceImage 为 absent、`{state:"present",revision:Counter,payloadKind,bytes}` 或 `{state:"proposed",revision:Counter,payloadKind,bytes}`；旧 conditional_source_change.result 只有 payloadKind、bytes、revisionRule=preserve_if_raw_equal_else_increment；旧 field_change 及 FieldEntryImage/1 用其原 SourceVersion/Counter，后者 exact `{format:"weftext.field-entries",version:1,ownerNodeRef,fieldId,revision,entries}`。旧 semantic extension 格式 d7_definition_transfer_effects/1、编码 d7_definition_transfer_effects1/field_entries1 保持原 decoder，旧 closed kind 集合仅 rank0–9。旧 d6_workspace_bootstrap_plan1、d7_symbolic_json1 继续精确 Plan/1 与 format=weftext.symbolic-effect/version1、payloadFormat=d6_workspace_bootstrap_plan1 的原闭集，不补 Policy/3 或新能力。真实旧 /1-/2 PreparedActionBinding 与旧 PreparedEditBinding/1 的 saved/planned/unknown 留存职责不取消，旧效果只从原完整 plan/receipt/pins 重建，不能从 current 文件补伪历史版本。

当前完整运输选择 manifest2/EffectBytes2/wire2 及这些明确更新的 SourceImage/FieldEntryImage/扩展 decoder。新 code 不能仅按 handle lexical shape 或字段缺失猜原版本；受保护 record 的真实版本唯一分派，当前 handle 不接受旧 token 抄入。旧保存的原 request/receipt 恢复和当前语义投影是两条有界 decoder 责任，不要求将无部署证据的所有过往草稿常驻兼容；有真实 record 则继续其原期限/last-reference/unknown/授权责任。

整个 /2 manifest 先证明实际 profile 的当前披露、完整原输入/前后/计划与 pins，再成功返回任一 header/page；pin 丢失、分支/owner 未闭合、读取权限不全、当前域或交付连续性不明都不生成“部分完整”成功。结果不表示作者已经提交；只有真实 P seal/原 receipt 决定 commit，source生产版本只在该时点有真实 ChangeId。本候选仍须真实 owner 联合独立接受，机器检查或 page 可读不等于语义已接受。

## 8. planned 预览的只读恢复

D7 拥有本节完整当前生产者，与 D10 Candidate §15.2 协调。入口仅接受真实 planned 决议保留 D7 PreparedActionBinding/3 的原 D6 请求。不新增提交、决议或批准，也不因共享效果运输便将此入口扩到 D3 或 D8 记录。请求与响应为以下准确闭集：

```text
{wireVersion:2,kind:"d7_planned_preview_open",protocolOwner:"D6",
 request:<complete original d6_commit_request/2>}
{wireVersion:2,kind:"d7_planned_preview_opened",request,
 previewToken,previewCursorToken,previewManifest}
```

额外、缺失、重复、null 成员，以及不支持的外层或请求版本均为 invalid_request。请求派生完整原 DecisionKey/2，禁止只凭 OperationId 查账。依序执行闭集解码、当前认证主体与受保护最小定位映射、原 D6 当前授权及 ObservationScope 和整份原预览 profile 披露资格、权威/托管/账本连续性，再检查同 key 的准确 canonical 请求字节及 fingerprint、状态恰为 planned；随后才加载原完整 PreparedActionBinding/3、语义预览及每个保留 pin。映射、主体或权限不足为 not_visible；资格通过后，非 planned、请求不等或账本/pin 连续性不可证统一 effects_unavailable，不披露另一请求或决议状态。这是读取，不继续安装、不改变批准或预留、不 abort/seal，也不分配新 OperationId。原 saved/planned/unknown 恢复仍归 D6。若真实保留记录中存在旧 /1 入口，只用其原 decoder，不改标进入此 /2 producer。

返回前按 §1–7 完整验证原 Manifest/2 与载荷，保留 phase=preview、protocolOwner=D6、原 profile、item 顺序和完整 DecisionKey；建立一个有限恢复交付世代，产生新的 action_preview token、effects_cursor 和全部 effect_bytes/2 句柄。响应 request 准确等于原请求，previewManifest 是原语义 manifest 除 items 外的完整 header。新世代使用当前可信时钟、预算、主体及授权，在返回 header 前绑定所有原语义项与准确 pins；不延长或复活旧 token/cursor。原准备或交付期过期不删除 planned 决议保留输入，也不阻止这个独立获权恢复读取；新期限不授权新的 unseen commit。随后页面和字节读取采用现有 wire2 入口、完整覆盖与错误规则：此恢复预览期限到达为 preview_expired，已证明世代失效为 reset_required，pin/连续性不可用为 effects_unavailable。失败不返回部分成功或载荷。

重开只改变运输句柄。不重跑 Query、不用漂移后的定义替换原定义、不重选目标、不变换新源、不分配 fresh identity、不重算拟议版本，也不修订原计划。完整原语义相等包括每个分型载荷的编码、长度和准确字节。D10 Control §8.2 可对该完整投影依其穷尽的分型 EffectBytes 替换计算 D10-Author-Preview/1 关联摘要，不改变 D7 wire；仅有摘要不算完整查阅。调用方只有在同一世代读取整份 manifest、所有页面及必要载荷后，才能进入另行授权的 D10 交互批准。D7 不创建批准或作者写权限。原 effects_resolve/open 仍只服务 committed，不可读 planned 预览不能变成新准备。
