---
_weftext:
  id: "b80e6c6f-648a-46f9-b5eb-8235f237b5f6"
---

当前权威状态：D8 D8-r03-p2-2026-09-24，仅由总控验收（外部控制记录未随本输入发布）和committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。下面送审正文中的candidate/未激活为历史标签，不覆盖本段。架构接受不代表产品实现；D9未启动。


# D7 预览与完整效果运输 — D8-r03组合修订候选

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


这是D7准备动作、D8明确编辑适配器与D3/D6原decision的只读运输，不产生第三ledger。preview明确表示拟议效果，committed表示原decision已保存的效果；不能将fresh symbolic subject包装为已提交Ref或将preview伪造为D3 receipt。本文件是Execution §8的完整规范正文。

## 1. Manifest与打开入口

Core保存完整、不可变的语义效果和每个字节slot的exact pinned payload；这份同decision证据不含会随交付重签发而改变的授权含义。`EffectManifest/1`是该完整语义效果在一个交付epoch中的closed运输投影，exact `{format:"weftext.effects",version:1,phase,protocolOwner,operationId,workspaceRef,profile,items}`，phase=`preview|committed`，profile=`full|owner_fields`。语义items、完整pins及该epoch所有EffectBytes投影在任何首page前已形成、验证完毕且immutable；取页不能再执行Query、源变换、D4 gate或追加效果。preview绑定D7 PreparedActionBinding/2（历史/1仅按原规则恢复/重放），或D8 Editor Interfaces定义的PreparedEditBinding/1；D8 producer严格只允许protocolOwner=D6与profile=full，并由完整D6 PreparedIntent绑定原request、前后源、scope、依赖与预算，不伪造D7 ActionSpec/bindingToken。两类producer都须满足本文件完全相同的完整性/授权/epoch/期限门，D8准备成功使用d8_edit_prepared承载同一header及tokens；committed绑定原D3/D6相同OperationId、原request、完整plan、receipt及原子D6 companion。transport tokens不是作者source、结果身份或额外授权。

D7 prepare或D8明确编辑prepare返回previewToken（tag action_preview）及previewCursorToken（tag effects_cursor）。原D6 receipt已有effectsToken；D3 receipt保持自身closed形状，通过新closed读取入口取得其原companion token：

- `d7_effects_resolve` exact `{wireVersion:1,kind:"d7_effects_resolve",protocolOwner,request}`。request是原完整D3 wire11请求或D6 commit request，protocolOwner必须相符；不接受只有OperationId的ledger探测。按该原协议的decode/current authorization/authority/custody/原key与fingerprint gate做**只读**查找，绝不执行未完成操作或新建decision。只有当前获准读取完整manifest、原key为committed且request相等才返回其已保存effectsToken。planned/rejected/terminal/unseen在已授权之后统一effects_unavailable；更早原权限/continuity门仍优先，错误映射下节。不得因为resolve读取去创建token所指的另一份临时效果。
- `d7_effects_resolved` exact `{wireVersion:1,kind:"d7_effects_resolved",effectsToken}`。token来自原同decision保存值，反复resolve不会生成不同语义清单。
- `d7_effects_open` exact `{wireVersion:1,kind:"d7_effects_open",effectsToken}`；成功`{wireVersion:1,kind:"d7_effects_opened",effectsToken,cursorToken,manifest}`。manifest为除items外的上述完整header，phase只能committed。preview header随prepared响应新增`previewManifest`，同形但phase=preview。header也可能披露scope/operation，须先完整授权。
- `d7_effects_page_request` exact `{wireVersion:1,kind:"d7_effects_page_request",token,cursorToken,pageSize}`，token仅action_preview/effects；pageSize=1..200。成功`{wireVersion:1,kind:"d7_effects_page",token,items,terminal,nextCursorToken?}`，terminal=true禁止next，false必有next。允许空nonterminal；完整消费者必须直到真正terminal，且逐item验证与原header一致，不按200条截断。

同一交付epoch重复同cursor/pageSize在当前资格下重放同item bytes；不同pageSize只改变运输切片，不改变清单。cursor绑定token/phase/manifest/位置/当前交付epoch，不能跨preview/committed使用；没有客户端offset跳过未看效果后声称完整审阅。完整manifest含原始效果全部items，细分profile由prepare前的先验证明唯一确定，不在交付时偷偷删项。

## 2. EffectItem闭集

subject逐字复用D3 PayloadSubjectKey中的existing/mapped/new entity三类，不接受artifact作为被修改对象；committed item须用existing subject持实际结果Ref，preview可用符号subject。source是某对象的作者bytes；生命周期/结构/控制单独列项，不能靠“源没变”漏掉它们。

| item exact shape | 语义与完整性 |
|---|---|
| `{kind:"source_change",subject,before:SourceImage,after:SourceImage}` | 覆盖所有fresh源及确定的源变化；committed恰为实际变化，unchanged raw不列假source edit；preview中existing C源改用下一variant，不重复列；before/after完整，允许absent |
| `{kind:"conditional_source_change",subject,before:SourceImage,result:{payloadKind:"exact_source_document",bytes:EffectBytes,revisionRule:"preserve_if_raw_equal_else_increment"}}` | 仅full preview的existing Node且before为完整present；恰覆盖所有含C的existing结果源；不是actual edit，规则见下段；committed禁止 |
| `{kind:"entity_state_change",subject,before:EntityImage,after:EntityImage}` | 覆盖fresh、lifecycle、placement、owner结构；纯move/Trash/restore即使source revision不变也有项 |
| `{kind:"d3_plan",request:D3Request}` | 仅preview，完整原请求/所有十数组及symbolic mapping；不是已planned receipt；一个D3 preview恰一项 |
| `{kind:"d3_receipt",receipt:D3Receipt}` | 仅committed，原完整byte-equivalent receipt；一个D3 committed manifest恰一项 |
| `{kind:"semantic_extension",format,bytes:EffectBytes}` | format闭集d4_relation_copy_effects/1、d4_source_materialization_effects/1、d7_definition_transfer_effects/1；仅committed的完整extension，不能截断owners/facts/entries；preview的完整作者变换由source_change、conditional_source_change与原D3 C/Q计划表达，不伪造最终Ref版extension |
| `{kind:"period_scope_change",subject,before:PeriodBindingImage,after:PeriodBindingImage}` | 原D6唯一派生控制scope，subject必须Node；purge删除/新period建立/managed copy均覆盖 |
| `{kind:"series_configuration_change",seriesScope,before:SeriesConfigImage,after:SeriesConfigImage}` | 同D6完整seriesScope；仅原请求允许的完整fork/bootstrap/fresh scope初始化或独立已授权管理结果 |
| `{kind:"workspace_bootstrap",bytes:EffectBytes}` | 完整D6 WorkspaceBootstrapPlan/1，含实际Registry/policy/config/binding；preview的fresh NodeRefs按下段symbolic encoding呈现 |
| `{kind:"authority_change",before:AuthorityImage,after:AuthorityImage}` | create/fork/continue当前authority及预期切换；preview的未分配continue authority明确pending，不捏造ID |
| `{kind:"field_change",ownerNodeRef,fieldId,beforeVersion:SourceVersion,afterVersion:SourceVersion,before:EffectBytes,after:EffectBytes}` | 仅owner_fields；两个bytes解码完整FieldEntryImage/1，实际版本与源相等 |

SourceImage closed三variant：`{state:"absent"}`；`{state:"present",revision:Counter,payloadKind,bytes:EffectBytes}`；`{state:"proposed",revision:Counter,payloadKind,bytes:EffectBytes}`。payloadKind仅exact_source_document/resource_bytes/annotation_value；present只能是实际before或已committed after，proposed只能preview after，revision是将发布的确定source version（fresh=1、actual edit checked+1，raw no-op不造edit项）。Document bytes是exact UTF8源，Resource bytes是原exact binary，Annotation是D3 Annotation-Value/3完整bytes，由subject唯一给owner/identity，targetStatus按D2/D3派生。symbolic bytes不是合法已物化SourceSnapshot；header encoding必须明确，不能让D2 decoder吞入placeholder。

EntityImage先按subject entity kind区分：absent=`{state:"absent"}`；purged=`{state:"tombstoned"}`；Node live=`{state:"live",placement:{parent:NodeSubject|null,ordinal:Counter}}`；Node trashed=`{state:"trashed",trashPlacement:{trashParent:NodeSubject|null,trashOrdinal:Counter,restoreLocation}}`，restoreLocation exact为`{kind:"original",parent:NodeSubject,ordinal:Counter}`或`{kind:"unavailable"}`，从原D3 restoreLocation机械投影；显示kind按以下闭表机械映射，原receipt内kind/成员/bytes不变：ordinary `original_location(parentRef,ordinal)`→`original(existing parentRef,ordinal)`；fork preview `mapped_original_location(parent,ordinal)`→`original(同一mapped parent subject,ordinal)`；fork committed `mapped_original_location(parentRef,ordinal)`→`original(existing mapped结果parentRef,ordinal)`；`unavailable_original_location`→`unavailable`。父Ref/subject和ordinal完全保留，不回退source parent/root/null；此映射同样适用于trash后像、restore/purge前像和surviving Trash结构效果。Resource/Annotation live/trashed=`{state:"live"|"trashed",owner:NodeSubject}`。父/owner在committed必须existing真实Ref，preview允许同plan mapped/new；root/null只按D3原sentinel规则合法，不赋予null新的D7一般值含义。Annotation reply是其完整source字段，同时须与原D3 structural/references effects相等。

PeriodBindingImage是`{state:"absent",revision:Counter}`或`{state:"present",scope:EffectScope,revision:Counter}`。EffectScope是D7只读投影，exact为`{kind:"workspace",workspaceRef}`或`{kind:"node",subject:NodeSubject}`；它从D4 workspaceId/scopeNodeRef机械映射，preview可在subject中使用已绑定fresh symbol，committed只能真实existing Node subject。它不是D4作者scope或D6控制请求的替代wire。revision保留原独立控制防ABA含义。SeriesConfigImage同样absent/present，present另有multiplicity=`unique|many`；本运输的seriesScope是exact `{series,scope:EffectScope,policyBinding}`，series及policyBinding逐字复用D6原七成员identity与完整三方绑定，只有scope机械投影为EffectScope，不能只给名字。Control generation和实际依赖在内部完整保存，不能把只读projection当另一可写配置源。

AuthorityImage是`{state:"absent"}`、`{state:"active",authorityInstanceId,authorityGenerationToken}`、仅create/fork preview可用`{state:"proposed",authorityInstanceId}`或仅preview可用`{state:"pending_continuation",sourceWorkspaceRef}`；IDs/tokens使用原D3域。current authority状态披露须原权限，committed绝无pending/proposed。Bootstrap已有proposal给出的targetAuthority与Workspace不受content fresh identity影响；不能在preview调用continue的stage12来提前分配AuthorityInstanceId。

`FieldEntryImage/1` exact `{format:"weftext.field-entries",version:1,ownerNodeRef,fieldId,revision,entries}`，entries为原author order的完整`{occurrenceKey,rawEntrySource}`数组，不只差异片段；每项用原D4 decoder验证，raw不重新序列化。before/after必须与原完整source独立恢复的该Field精确一致，未选条目仍显示且逐字不变。它明确是Field projection，不标为exact SourceSnapshot。

### 2.1. 条件源物化的完整预览

conditional_source_change不接受用户predicate、任意脚本或采样出的Ref。subject必须是原请求已声明的existing Node；before是其同一绑定preimage及真实revision；result.bytes的encoding仅d3_symbolic_result9，必须逐字等于原D3 payloadBindings所绑定的该subject完整Result/9，完整解码后至少含一个C，所有C.container及Registry/源绑定与原plan相等。每个含C的existing结果源恰一项；fresh源仍用source_change，revision固定1。此项同时覆盖C涉及的全部潜在owner/迁出/迁入/原位及非carrier原bytes，不能只给delta或某一map的输出。

准备不产生最终content ID，也不选一个“典型”map。消费者显示完整符号源变换，并明确现存源可能保持原版或修改后增版；不能把baseCarrier/占位骨架显示为确定的最终源。最终仅使用原stage12的同一个Core candidate map及原C物化算法，先经原全部D2/D3/D4规则，再将完整结果UTF8 bytes与before比较：完全相同则revision保持且committed不列source_change；不同则沿原checked+1并恰列一个完整actual source_change。任何原语义/预算/溢出/最终CAS失败均沿原原子失败规则，不部分发布或重新采样挑选分支。

由此选择得到的actual-only committed集合必须与原receipt、D4SourceMaterializationEffects及实际源独立相等；D4内部效果按原规则保留C的no-op owner，不把它假造为public source edit。条件项从preview消失或精化成actual source_change是本variant唯一明示的集合变化，不是允许commit追加未预览效果。preview语义、原字节与准备记录都不因此改变。含C但可证明所有map均修改的existing源仍使用同一variant，避免由实现自行选择不同wire。

## 3. 拟议字节和独立EffectBytes

EffectBytes exact `{handleToken,encoding,byteLength}`。tag=`effect_bytes/1`，不能拿D6 resource_bytes/1 ByteHandle、Query result或effects cursor替代。record绑定manifest token+phase+item ordinal+该kind的唯一字节slot角色（如before、after、result或extension）+交付epoch、exact pinned bytes、encoding、audience/observation scope、有限读取预算、期限/epoch。Resource原D6 ByteHandle继续其旧合同；新handle可以承载fresh/proposed二进制而不伪造已提交ResourceRef。

encoding闭集：`exact_source_utf8|resource_bytes|d3_annotation_value3|d3_symbolic_result9|d4_relation_copy_effects1|d4_source_materialization_effects1|d7_definition_transfer_effects1|d6_workspace_bootstrap_plan1|d7_symbolic_json1|field_entries1`。这些encoding唯一决定完整decoder，不允许generic_json或任意format dispatch。ordinary exact数据按相应完整原decoder验证；D3 symbolic输入按Result/9，consumer逐B/M/N/E/S/C/Q分区以symbolic subject显示、审阅全部占位关系及原bytes，不能自行分配UUID后冒称author source。

`d7_symbolic_json1`只用于本节preview Bootstrap中具有fresh refs的完整closed control对象。内容exact `{format:"weftext.symbolic-effect",version:1,payloadFormat,template,slots}`，payloadFormat只允许d6_workspace_bootstrap_plan1；template为对相应schema按typed Ref位置机械替换成`{kind:"symbolic_subject",subject:NodeSubject}`的完整对象，slots为`{path,subject}`列表；path的表示与排序逐字采用同包《Definition Transfer》§2的唯一typed-path规则（member text或Counter index的数组，text UTF8 rank0、index数值 rank1、共同prefix后短路径在前，故index 2先于10）；slots必须与所有替换处一一对应。不得允许literal位置出现placeholder或普通object冒充Ref。Core按原plan candidate map对完整模板替换后必须通过原D6/D4 decoder；最终committed encoding使用真实原格式而非symbolic。这个只读展示模板不能作为修改请求或证据输入返回Core。

`d7_effect_bytes_read` exact `{wireVersion:1,kind:"d7_effect_bytes_read",handleToken,offset,maxBytes}`，offset Counter，maxBytes1..1048576。成功`{wireVersion:1,kind:"d7_effect_bytes_chunk",handleToken,offset,totalBytes,data,terminal}`，data为canonical无padding base64url；decoded长度恰min(maxBytes,totalBytes-offset)，terminal恰offset+len=totalBytes；offset==totalBytes可空terminal，超界cursor_invalid。完整消费者必须验证0..totalBytes无洞且使用header encoding完整decode；seek最后一byte不是完整审阅。I/O short read、pin缺失、epoch/资格变化不返回部分data或假EOF。

完整D4 extensions可能包含全部raw source，必须按full profile交付其整份bytes；不能将其切成owner_fields并仍使用原format。窄profile的field_change由完整已验证原扩展/源独立推导，只在下一节证明它覆盖本action全部作者效果时合法。原内部完整D4证据仍同decision保存，并不因窄公开投影被丢弃。

## 4. 先验资格、错误与完整性

full profile要求整份所有before/after源、适用lifecycle/结构/控制及semantic extension的当前完整读取/ObservationScope资格；未知或读不全必须在prepare前拒绝，不能等用户看到一半preview再隐藏某item。D3 bootstrap preview依据原prepared_workspace issuer资格仅披露本次明确创建内容；committed额外target源/控制读取仍须当前target权限，原D3 receipt重放继续它自己的特殊边界。

owner_fields仅用于《D7 Narrow Field Qualification》静态证明成功的单owner Field动作。实际作者footprint恰为已授权Fields，identity/lifecycle/placement/其它source/控制值不变；只列完整field_change。所有D4全源语义效果保存在内部，但public field projection必须独立证明覆盖全部实际修改，不能隐藏另一个Field或结构变化。beforeVersion绑定真实preimage，afterVersion绑定最终源版本并与原receipt.sourceVersions中该owner的实际修改记录一致。只列该Field完整author image确实变化的项；整体raw no-op时field_change集合与原receipt.sourceVersions均为空，不伪造版本或修改；sourceEnvelope与commitSequence的观察成本须已有明确资格。若实际footprint扩大或有无法投影的效果，整个prepare拒绝，不在commit后才发现preview不完整。

完整性总门：preview由原前像、完整proposed/Result9及原plan独立恢复全部确定/条件作者效果，committed由同一真实candidate map、实际源及原receipt独立恢复actual效果；比较完整item集合和各payload bytes，不让effects自称覆盖范围。多余、遗漏、重复、相互矛盾或错误phase均拒绝；条件项只按§2.1精化，不能据其存在豁免其它entity/control/source效果。所有blob完整decode后才可宣称完整审阅。

唯一排序与重复规则如下，先rank再secondary key。数组必须已经按此顺序，不在消费者端悄悄修复：
| rank / kind | secondary key与cardinality |
|---|---|
| 0 source_change | 原D3 subjectCanonicalKey；每subject≤1 |
| 1 conditional_source_change | 同subjectCanonicalKey；每subject≤1，且与source_change互斥；仅preview |
| 2 entity_state_change | 同subjectCanonicalKey；每subject≤1 |
| 3 d3_plan / d3_receipt | 无secondary key；D3 preview恰一个plan，D3 committed恰一个receipt；D6两者禁止 |
| 4 semantic_extension | 固定format rank：d4_relation_copy_effects/1=0，d4_source_materialization_effects/1=1，d7_definition_transfer_effects/1=2；每format≤1，仅committed；各适用format按原协议要求恰一份完整operation级扩展，不按owner拆项 |
| 5 period_scope_change | Node subjectCanonicalKey；每subject≤1 |
| 6 series_configuration_change | 完整seriesScope按D3-CJ/3编码后的UTF8 bytes；包含七成员series、机械投影后的完整scope和完整policyBinding；每完整key≤1，不按seriesKey、label或handle排序 |
| 7 workspace_bootstrap | 单Workspace操作级singleton，适用时恰一项 |
| 8 authority_change | 单Workspace操作级singleton，适用时恰一项 |
| 9 field_change | 完整owner NodeRef规范key，再完整FieldId规范bytes；每(owner,FieldId)≤1，仅owner_fields |

其它singleton是否适用由原mode/控制合同及完整实际效果决定，不能为凑数添加空或假效果。每个kind的全字段、subject域、phase、前后像和mandatory范围仍须验证；去重后数量相同不能代替集合相等。

错误exact `{wireVersion:1,kind:"d7_effects_error",code}`，code闭集`invalid_request|not_visible|preview_expired|reset_required|cursor_invalid|effects_unavailable|budget_exceeded`。所有入口固定：closed decode→当前tag/audience与全manifest/profile授权→preview期限（committed不受preview TTL）→交付epoch/continuity→cursor/range→pin/decoder/预算→完整输出前current gate。missing/wrong-tag/wrong-audience/撤权均not_visible；continuity不可证明或缺完整pins为effects_unavailable；已证明epoch失效reset_required。原resolve的authority/custody不可证明映effects_unavailable，原key/fingerprint冲突在当前资格后同样effects_unavailable，不返回另一个request的信息。任何失败无items/data/terminal。

committed effects保持同decision耐久语义，不因原preview过期消失。每次成功d7_effects_open在当前完整资格与有限预算下，从原effectsToken所指不可变语义清单及完整pins建立一个新的完整交付epoch；在返回header/cursor前为每个EffectBytes slot固定该epoch的handle并完整验证投影。后续page只切片这份投影，不边翻页边换handle。重复打开可产生不同transport随机bytes，但原effectsToken、receipt、语义item顺序、encoding、byteLength及exact payload不变，不重跑Query/变换/作者effects。保存的原语义证据和既有epoch投影都不原地修改。

每个cursor/byte handle仅在自己epoch、phase及audience下有效；新epoch不自动撤销先前仍在自己期限内的交付。旧epoch到期后按原错误总序返回reset_required，不能借新open让旧cursor继续或混入新handles。全部committed pins按原decision耐久保留；缺失或continuity不可证明为effects_unavailable，无部分items/data。preview在prepare产生一份完整初始投影并由原PreparedActionBinding或PreparedEditBinding不可变保存；不因新交付延长原preview TTL，原record和原request绑定不变。preview取消/超时不回滚已committed事实；撤权遮蔽交付，不改原decision。

## 5. D3模式覆盖

| 原mode | 必需preview/committed效果 |
|---|---|
| create_workspace / fork_workspace | 完整d3_plan/receipt、全部fresh source与entity结构/两态、原Bootstrap Registry/policy/config/bindings、authority；commit包含适用D4与D7全部extensions；preview显示其完整作者源C/Q变换及适用conditional_source_change |
| create_node / create_resource / create_annotation | 全部primary/member fresh与compound existing源（含C者使用条件项）、placement/owner及period控制；互相引用用Result9 symbolic完整表达 |
| copy_node_subtree / copy_resource / copy_annotation | 完整map、全部selected/omitted dispositions、fresh源/结构、必要fresh scope配置、commit的D4 copy/materialization与D7 transfer extensions；preview的完整C/Q源变换 |
| continue_workspace | 完整d3_plan/receipt与authority_change，源不变不假造source_change |
| move_node / reorder_node | 完整entity_state_change与原结构计划/回执，未改源不增加source revision |
| trash / restore | 全部实际closure的entity状态/placement和reference生命周期回执；有cleanup时才列源变化及对应D4 effects |
| purge | 完整tombstone/源absent、scope binding删除及原D3回执；不省略closure成员或隐式清控制引用 |
| import_new | 全部artifact fresh分区→fresh subjects/ownership/placement，ordinary-format existing分区及companion按原§4.1.3a完整列源效果；含C的existing源用条件项；完整mapping/loss dispositions、commit的适用D4/D7 extensions；preview完整C/Q源变换 |

十五个原mode共享这套有限类型。独立D6 policy/issuer/attempt管理变更不是D7 ActionSpec的kind，本版本effects入口对其不支持的完整控制清单在授权后返回effects_unavailable；不得将其包装成D7 Query rows。上述D7所允许的D3 mode带来的必要Bootstrap/period/fresh-scope控制效果已经纳入，不能用这项独立管理限制跳过。


完整预览与最终语义证明的区别：D4/D7 committed extensions是带最终identity/revision的验证证据；它们没有独立于已列source/entity/control变更的额外作者效果。preview在尚无最终映射时不发送假造extension，而必须完整发送所有现存前像、所有拟议Result9 C/Q原字节及subject归属、Node/Resource/Annotation/结构/控制变化；消费者显示每个原Entry迁出/原位/迁入及typed reference替换，不能仅显示payload digest。commit后最终extensions逐字证明这些预览模板在原candidate map下的实际物化，并与原receipt和源一致。条件C的existing源按§2.1唯一规则确定是否实际修改及版本，除此之外任何extension带出预览未覆盖的作者变化，视为原准备与提交绑定完整性冲突，不是允许追加的效果。
