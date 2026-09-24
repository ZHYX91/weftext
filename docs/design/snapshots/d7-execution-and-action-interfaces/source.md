---
_weftext:
  id: "cf00b979-ef4d-4909-80a2-f9d9e163f1ef"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 Execution、Action 与 DynamicBlock — revision06-draft

## 1. 编译、执行与 typed result

本文件所有D7 object closed，采用主稿strict decoder；嵌入D3/D4/D6 object使用原decoder，不重新命名其成员。Token为D6固定43字符canonical token词法，服务器保存独立tag，不因同形可互换。authenticated PrincipalContext来自host，不在任何作者JSON自报。

`d7_query_execute` exact `{wireVersion:1,kind:"d7_query_execute",workspaceRef,call:QueryCall,budget}`。QueryCall exact `{query,arguments,context}`，三者只在该层出现一次；query二variant `{kind:"inline",spec:QuerySpec}` 或 `{kind:"saved",definition:DefinitionAddress}`。arguments使用主稿TypedLiteral object；context exact是推导需要的成员集合（now/timeZone/calendarVersion/tzdbVersion）：now用TypedLiteral zoned_instant，其余nonempty text；缺项context_unbound，多余项context_unused。无device locale/time/timezone默认。definition closure、D4Registry/temporal rules从同一Core AuthorizedCut取；不能由客户端提交trusted read context。

budget exact `{maxRows,maxValueBytes,maxResultBytes,maxWork,maxDependencies,maxTemporaryBytes,maxElapsedMs}`，均Counter；取request、受管policy、host每维min，任何0明确拒绝，不用0表示无限。协议硬上限：QuerySpec1MiB，含保存闭包总8MiB；每Query relations≤128、scalars≤64、参数≤64、全部invocation≤64、深度≤16；单expr UTF8≤65536、AST≤4096、嵌套≤64；终端/中间每行columns≤64；maxRows≤1,000,000，maxValueBytes≤16MiB，maxResultBytes≤256MiB，maxDependencies≤1,000,000，maxTemporaryBytes≤1GiB，maxWork≤1,000,000,000，maxElapsedMs≤3,600,000。checked add/multiply计费在分配前；graph nodes+edges/recurrence/unnest实际展开均计maxRows。算法可流式/spill，但不交付不完整结果。

work定义为逻辑AST求值节点、逐输入operator处理、关系候选检查、完整range扫描项、解析UTF8 octet各计1，按规范batch逻辑计；优化不以少扫描免除必要逻辑计费，cache命中也按相同逻辑成本资格检查。为资源可实现性可在预算确定不足时提前拒绝，不能先读取隐藏事实决定所需额度然后输出其数目。耗费与失败只有budget_exceeded类别，无未授权精确计数。Elapsed是执行attempt墙钟可导致不同availability结果，不改变成功结果数学语义；并行/重启不得累计取得无限重试额度，使用D6预算绑定。

响应在完整成功后为 `d7_query_result` exact `{wireVersion:1,kind:"d7_query_result",resultToken,cursorToken,schema}`；cursorToken选择该result运输起点。producing期间仅异步runtime pending状态，无rows、count、schema-derived秘密或可复用token输出。规范接口可阻塞到complete；取消由host请求关联处理，不构成新authorAction。失败 `d7_query_error` exact `{wireVersion:1,kind:"d7_query_error",diagnostic}`；diagnostic由主稿定义。

TerminalSchema closed：

- rows `{kind:"rows",ordered:bool,columns:[{columnId,type}]}`，columnId为终端名，顺序按终端schema。
- scalar `{kind:"scalar",type:T}`。
- graph `{kind:"graph",nodes:RowsSchema,edges:RowsSchema,nodeRef,sourceRef,targetRef,relationColumn,derivationColumn}`，后五值为相应columnId。

D6 `d6_result_page` envelope不变，其rows由D7定义：rows结果每item exact `{kind:"row",rowHandle,cells:[V...]}`，cells数目及型严格schema，rowHandle为tag=result_row的Token；scalar结果仅一个item `{kind:"scalar",value:V}`（none也是明确一项，不是零行）；graph每item `{kind:"graph_node"|"graph_edge",rowHandle,cells:[V...]}`按nodes后edges两段运输。D6可给空nonterminal页；消费者必须一直取到terminal，不通过空数组/少于pageSize判断EOF。首次cursorToken由d7_query_result给出，D6既定request无新增字段；它不是Query author member或语义limit。

scalar page聚合不得重复/遗漏value；graph完整终端时nodes/edges必须各自完整并符合schema；任何wrong tag/错误cells/重复rowHandle/跨result rowHandle使整个consumer拒绝。rowHandle与Token没有durable语义，Core映射绑定result+epoch+internalkey，不以输入row value的hash冒充唯一行；同值bag两行句柄不同。

## 2. Graph terminal 与完整验证

Query result增加closed variant `{kind:"graph",nodes,edges,nodeRef,sourceRef,targetRef,relationColumn,derivationColumn}`；nodes/edges引用relation ID，成为联合graph两根，主稿CanonicalGraph按键字节序遍历两根，Scalar result规则不变。五column bindings必须存在。nodeRef/sourceRef/targetRef全部NodeRef，relationColumn和derivationColumn为text，后者只允许asserted/derived。nodes中nodeRef唯一，edges每个endpoint必须存在于完整nodes；重复edge occurrence允许，不使用endpoint pair去重。nodes、edges都必须Query sort的ordered relation。节点可以孤立，self/cycle受实际关系定义与Query完整验证约束；View network不再做关系完整性推导。

每条asserted edge必须有Core保留的单一actual D4 relation/core结构或NodeLink provenance，且source/target/relationColumn与该事实精确吻合（symmetric允许原规范两端视角）；计算、组合、聚合后无法证明者必须declared derived，不能仅改字面标签取得asserted资格。derived同样需要输出值的完整可观察依赖，不能对隐藏endpoint生成placeholder。graph构造仅验证/整合两个完整关系，不自动删除missingendpoint edge或补node、sort或aggregate。

query_ref只接callee rows，scalar query_ref只接scalar；callee graph在v1返回unsupported_feature（feature为graph-query-ref），不拆成偷偷多次调用。graph terminal feature为weftext.query.graph-result/1；derived_period_range read推导weftext.query.derived-period-range/1，结果仍是已有typed rows，不新增Result/wire版本或错误码；普通业务新Facet不增加feature。

## 3. 交付、cache 与订阅

ResultHandle由D6保存完整query/canonicalGraph、有效arguments、完整definition closure、Registry binding、规则版本、输入AuthorizedCut、schema、内部occurrence/provenance、依赖和audience/epoch/TTL/budget。SemanticStateKey绑定canonicalGraph+实际used arguments/context+definition/Registry/规则语义版本+授权partition；SnapshotResultKey再加cut及完整正负DependencyProof。两者均不是content identity。物理cache读取必须证明依赖/构建覆盖到当前cut，不能用mtime/path/firstpage替代。无索引时完整扫描；范围空也有query-scan/relation-incidence/ref-inbound/placement-range负版本。

授权、定义、Registry、rules、lifecycle与continuity按D6各自依赖处理。Result交付每次按D6当前audience/auth→expiry→epoch/reset→cursor→state次序；**授权generation任何变化使旧Query result整体reset，即使重新授权仍能读同source**，不得套用ByteHandle对无关auth变化可继续读旧资源的例外。D6wire code是reset_required；业务含义包括历史非回退输入中的stale_authorization，不新造第二错误alias。元数据、schema、page、导出、ActionEvidence签发均同闸门；旧rowHandle不能跨epoch映射到新行。

Subscription是受管runtime、无作者身份。`d7_subscribe` exact `{wireVersion:1,kind:"d7_subscribe",resultToken}`。只能当前完整result创建。成功 `{wireVersion:1,kind:"d7_subscription",subscriptionToken,epoch,sequence}`，epoch为独立Token tag subscription_epoch，sequence Counter从0开始。`d7_subscription_next` exact `{wireVersion:1,kind:"d7_subscription_next",subscriptionToken,epoch,afterSequence}`。当前auth/TTL/reset优先，再检查请求epoch/sequence；不返回hidden counters。

事件闭集：`d7_delta` exact `{wireVersion:1,kind:"d7_delta",subscriptionToken,epoch,sequence,fromResultToken,toResultToken,changes}`；`d7_reset` exact `{wireVersion:1,kind:"d7_reset",subscriptionToken,epoch,sequence,code:"reset_required"}`；`d7_subscription_idle`仅同token/epoch/sequence；error使用D7queryerror，仅not_visible/result_expired/reset_required/cursor_invalid/result_unavailable。delta sequence严格+1且from必须上次完整acceptedresult。changes数组为 `{kind:"remove",rowHandle}` 或 `{kind:"upsert",rowHandle,cells,position}`；仅rows结果适用，position是目标完整结果零起Counter位置。scalar/graph变化一律reset，不谎称有增量支持。

rows delta采用完整替换的保守合法算法：只有Core证明同schema、同auth/definition/Registry/rules/continuity，且两个完整batch结果相等语义范围可比时，可把旧全部rowHandle remove，再把新全部rows按目标序upsert（新tagged rowHandles绑定toResult）。changes至多2*maxRows并受结果/输出预算；客户端先在临时完整副本验证oldset、唯一handles、position连续无洞、类型和目标结果完整性，再一次切换。不得边收delta边暴露混合epoch。最小变更优化需相同证明；失去任何dependency、订阅gap、schema变化、资源不足、未知equivalence、counter溢出或source读取失败一律reset，不发猜测delta。当前epoch旧证据在reset实际发送前已经失效；丢reset也不能继续旧操作。重新订阅须新Query完整结果。

此delta对bag重复以内部occurrence区分、公开opaque handles定位；新result不能复用旧handle，故本版remove-all/upsert-all有明显带宽成本，但没有无依据稳定行身份保证。sequence是订阅私有序，不暴露D6全局contentSequence或commitSequence。

next精确语义：afterSequence若等于当前最后已形成event的sequence且无新event，返回idle；否则只返回sequence=afterSequence+1的已保存完整event。重复相同afterSequence在当前资格仍有效且event保留时重放同event bytes；afterSequence大于当前或所需event已不在有限保留log中为reset_required，不猜客户端已收到了什么。subscription继承原result的有限TTL/预算，不自动延长；eventlog及新result pins计入该预算，容量不足立即使epoch失效并reset。`d7_unsubscribe` exact `{wireVersion:1,kind:"d7_unsubscribe",subscriptionToken}`，当前audience通过后撤销runtime subscription并释放其可释放pins，返回`{wireVersion:1,kind:"d7_unsubscribed"}`；不改任何作者source/decision，未知/wrongaudience token仍not_visible。

## 4. 错误闭集与 caller 一致性

D7 query code rank顺序（同逻辑执行阶段用index）：invalid_request、unsupported_version、unsupported_feature、not_visible、unknown_definition、definition_unavailable、definition_cycle、definition_kind_mismatch、invalid_graph、unbound_parameter、type_mismatch、context_unbound、context_unused、source_unavailable、none_value、numeric_overflow、division_by_zero、invalid_order、duplicate_key、missing_endpoint、invalid_provenance、budget_exceeded、cancelled、result_expired、reset_required、cursor_invalid、result_not_complete、result_unavailable。阶段优先于rank，当前授权永远优先于内容diagnostic。D2/D4详细parse/constraint错误不作为Query隐藏数据回显；已授权来源坏值统一source_unavailable，详细audit/repair由原权限入口。

Core/CLI/Desktop/Server/WebUI/Mobile六个caller使用同JSON、typechecker、schema、错误、complete语义和动作adapters；host传输可不同，不得CLI int64截断/JS Number舍入/移动端自动少取/Server额外where。CLI导出精确typedJSON是本版规范，CSV/Office布局由D9；用户明确的CSV adapter也必须完整结果、当前export+inputs read，不将none空字符串混为事实。architecture fixture可定义六caller expected相同，但未实际运行这些caller不得计产品通过。

## 5. ActionSpec 与准备入口

ActionSpec exact `{format:"weftext.action",version:1,intent}`，是运行时明确意图，不是Query节点或可存入D2的SavedAction实体。`d7_action_prepare` exact `{wireVersion:1,kind:"d7_action_prepare",workspaceRef,action,budget,evidenceToken?}`；budget使用D6受管PreparedIntent预算，数量上界D5：每原子action最多1000目标、新Node及native rows。user看到preview后通过原D3/D6提交入口确认；准备不reserve identity/写作者源/占用binding。prepare错误exact `{wireVersion:1,kind:"d7_action_error",code}`，code=invalid_request/not_visible/stale_target/action_not_applicable/unsupported_action/definition_unavailable/semantic_rejected/budget_exceeded/authority_unavailable；全部为无ledger的准备失败，不覆盖正式D3/D6错误。stale只在适用locator-state授权之后；其它未获资格一律not_visible。

EntityTarget exact `{ref:EntityRef,expectedRevision:Counter}`用于源编辑（Node表示其Document），没有以path/title/UUID猜测。FieldSelector exact `{owner:NodeRef,fieldId,expectedRevision,occurrenceKey,rawEntrySource}`，后三项与D5实际前像匹配，rawEntrySource为完整原Entry JSON文本而非重序列化object；Core在完整原源中定位，不让客户端提交trustedContext。key相同而raw/revision变更也拒绝。NativeSelector exact `{owner:NodeRef,locator:D3Locator}`，是revision-bound原生内容选择器；locator必须为D3 DocumentElementLocator且locator.owner逐字等于owner。具体kind按动作精确限定：set_native_cell只接table_cell；remove_native_row/promote_native_row只接table_row；toggle_checklist/promote_checklist只接当前D2解析确认为checklist的list_item，普通list_item不能冒充checklist。所有位置须在同一完整当前Document中解析并验证原源，禁止Field occurrence、任意range、table外层、另一owner或过期revision冒充。tableLocator参数另只接elementKind=table。此处没有新Locator身份/语法；D3原解码、当前授权与版本门仍先行。

intent闭集：

| kind / exact余下成员 | 效果与协议owner |
|---|---|
| set_title: target:EntityTarget, value:text | target Node；完整D2合法title，不自动trim；D6 |
| append_field: target:EntityTarget, fieldId, entry:D4Entry | 完整新Entry，key须在owner+Field+revision未占用；D4 gate；D6 |
| replace_field: selector:FieldSelector, entry:D4Entry | 只替换所选Entry，保持相同occurrenceKey；其它raw不动；D6 |
| remove_field: selector:FieldSelector | 仅删该Entry，empty carrier处理D2/D4；D6 |
| set_field_member: selector:FieldSelector, memberPath:[name...], value:TypedLiteral | 静态schema object member路径，不能index任意list/union；末端type严格，member删除使用Optional.none且仅D4optional成员；whole proposedEntry再验证；D6 |
| toggle_checklist: selector:NativeSelector, checked:bool | 仅当前checklist marker改为显式checked状态，原inlines/nestedLists/其它raw保持；D6，无Task查找/新建 |
| assign_facet: target:EntityTarget, facetId, initialEntries:[{fieldId,occurrenceKey,rawEntrySource}...] | D4 AssignFacet/FacetOperationRequest2，完整initial author order与requiredness；D6 |
| remove_facet: target:EntityTarget, facetId | D4 RemoveFacet，只改declared membership且复验依赖/domain；保留全部Entry raw；D6 |
| cleanup_facet_fields: target:EntityTarget, selectors:[FieldSelector...] | D4 CleanupFacetFields，完整唯一selectors只删用户选择且证明未被任何effective Facet使用的Entry；D6 |
| set_native_cell: selector:NativeSelector, value:text | D5 inert原生cell编码，不允许注入delimiter/multiline；D6 |
| remove_native_row: selector:NativeSelector | 精确row删除，整table/D2结构复验；D6 |
| reorder_native_rows: owner:NodeRef, expectedRevision, tableLocator:D3Locator, rows:[D3Locator...] | 完整无重复同table行排列，trivia未证明保持即unsupported；D6 |
| create_node: parent:NodeRef, source:text, ordinal:Counter | 完整D2源与D4classification；fresh Node；D3 create_node |
| promote_native_row: selector:NativeSelector, parent:NodeRef, source:text, label:text, ordinal:Counter | 原row改显式NodeLink+freshNode同一D3 create_node compound，不留mirror |
| promote_checklist: selector:NativeSelector, parent:NodeRef, source:text, label:text, ordinal:Counter | locator须checklist item，完整source ordinary+tasks/task；原item替换NodeLink，同一D3 compound |
| apply_suggestion: annotation:EntityTarget, targetLocator:D3Locator | D2 purpose=suggestion且target/revision精确，replace_plain_text原replacement（可空），Document及Annotation既有源同D6；不自动删除Annotation |
| d3_operation: request:D3Request | 完整同包wire11，由D3 decoder/adapters；不加planToken/私有mode；lifecycle/move/copy/restore/purge/bootstrap走原请求 |
| collection_create: collection:CollectionCall, parentPolicy:ParentPolicy, source:text, ordinal:Counter, requireMembership:bool | 本节下段；freshNode+D7完整postcondition同D3 |
| collection_remove: collection:CollectionCall, target:EntityTarget, change:FieldChange | FieldChange仅replace/remove_field同形；完整postquery确认target不再属于集合，不能隐式Trash；D6 |
| bulk_field: targets:TargetSet, fieldId, operation:BulkFieldOperation | 同型显式append或replace_single，不隐式replace_all；完整1000以内targets原子D6 |

没有general patch/raw arbitrarycallback、嵌套Action、脚本或混合任意D3/D6请求batch。D5新增native row的最小合同也提供 `insert_native_row`：exact owner:NodeRef,expectedRevision,tableLocator,at:Counter,cells:[text...]，列数必须等于实际table固定列数且全部inert，at=0..rowCount；D6。大批数据显式多action，任一action不部分成功。

BulkFieldOperation exact `{kind:"append",value:TypedLiteral,qualifiers:D4Qualifiers,note?:text,authoredProvenance?:D4ProvenanceArray}`或`{kind:"replace_single",value:TypedLiteral}`。append为每owner生成独立合法occurrenceKey；所有实际D4Entry必须在同Registry下通过，缺note/provenance保持原D4缺省。replace_single要求每owner该Field恰一Entry，保留其key/其它members；0或>1统一semantic_rejected，不挑第一条或清空列表。FieldChange/相同对象冲突/两条意图重叠都在prepare拒绝；不是按数组后写覆盖。

TargetSet exact `{kind:"explicit",targets:[EntityTarget...]}`，或 `{kind:"result",resultToken,columnId,rowHandles:[Token...]}`，或 `{kind:"all_result",resultToken,columnId}`。explicit无重复完整Ref；result只接受完整rows result的nonoptional NodeRef column，rowHandles必须同result、无重复；all_result读取全部terminal rows，非当前页。先current授权/expiry/epoch再取cells；按完整Ref去重生成EntityTarget并取得实际current SourceVersion，保存全部targets/revisions。若同Ref在结果多行中出现，bulk只修改一次；preview明确目标集合，不把occurrence数当目标数。column是用户明确选择，不从第一个Ref猜目标。

result/all_result选择及任何从result重签ActionEvidence，还必须在当前cut复验原完整query的全部正/负依赖、定义和rules；不匹配返回stale_target（当前state披露资格之后）。因此依赖R1的旧显示行不能默默绑定R2，原membership范围变更不能在commit时扩大或缩小目标。source-free exact-ref query未读源时可在此第一次取得source版本，但仍复验其lifecycle/selector依赖。此保守条件不改变旧ResultHandle对原cut的只读有效期；它只拒绝用已失准依据生成新作者动作。

## 6. Evidence、prepare、commit 与重放

`d7_issue_action_evidence` exact `{wireVersion:1,kind:"d7_issue_action_evidence",resultToken,rowHandle,actionKind,targetColumn?}`。先D6 result current gate，再所请求Action的当前资格。targetColumn存在须为该row明确nonoptional EntityRef且动作适用，Corefresh解析并绑定版本；不存在时只允许仍保留唯一无歧义source/occurrence lineage的terminal行。project/derive/filter/sort/take不会误擦除此lineage；aggregate/distinct/query_ref/traverse/unnest/recurrence后无implicit evidence。0或多目标为action_not_applicable；不暴露隐藏候选。

成功 `{wireVersion:1,kind:"d7_action_evidence",evidenceToken,actionKind,target}`，target是已经允许披露的EntityTarget或NativeSelector/FieldSelector（完整read权限另验）；token tag action_evidence，保存principal/session/delegation、result/epoch、exactkind、target/occurrence/revision、完整read/dependency范围、有限TTL。可将返回target填入相应ActionSpec，prepare请求的可选`evidenceToken`；Core验证exact actionKind/target对应，无extra权利。未给token的显式action仍可通过完整fresh授权/定位/版本证据prepare，不强制必须先运行Query。

evidence不能授权overwrite/delete、拓宽Field/member、换owner、更新版本或从一row操作所有query成员。准备、preview取页、commit、replay都重验当前权利和实际D6ObservationScope；旧evidence不因A→B→A、移动恢复、auth generation变化或相同resultcells复活。签发不是reserved permission，token不可跨audience；没有token的内容访问亦不降低上游门槛。

Coreprepare固定顺序：closed intent decode→当前主体及潜在对象/约束观察资格（不读作者值决定profile）→当前authority/cut→精确source/selector/定义/规则/正负依赖→实际拟议完整source和MutationFootprint→D2/D4/D5/D7全门→immutable PreparedIntent/preview。D6 owner用workspace_constraints或《D7 Narrow Field Qualification》证明的owner_fields；D3 owner依原全域/prepared_workspace gate。preview必须在准备前已有完整交付资格；owner_fields交付完整Field projection，full交付完整源/实体/控制效果。profile不能在取页时按隐藏内容选择。

准备成功`d7_action_prepared` exact `{wireVersion:1,kind:"d7_action_prepared",protocolOwner,request,previewToken,previewCursorToken,previewManifest}`。protocolOwner=D6时request为完整d6_commit_request；D3时为完整wire11 identity_operation_request，必须含preparationBinding。previewManifest是EffectManifest去除items的完整header，phase=preview。完整准备记录、token生成/保存顺序、D3 stage3/5/14与planning CAS、D6原planToken嵌入、TTL及恢复合同由《D7 Prepared Action Binding》逐项规定，该文件为本节规范组成部分。

OperationId由准备器为新意图生成UUIDv4；d3_operation已给ID则保留。重放只能发送原完整request，修改输入必须新prepare；既有key下变化依原operation冲突。准备不reserve最终identity，fresh内容只以原D3 symbolic subject展示。正式提交直接进入原D3/D6入口；D7没有第三ledger或commit outcome。原receipt丢失时重放同请求；当前撤权遮蔽交付，不能写成永久业务拒绝。

## 7. 集合与完整 post-query

CollectionCall exact `{call:QueryCall,nodeColumn}`；call.query须rows terminal、nodeColumn nonoptional NodeRef，全集为完整结果该列去重live NodeRef。没有CollectionRef、成员缓存或第二份arguments/context。ParentPolicy exact `{kind:"explicit",parent:NodeRef}`或`{kind:"resolve_saved"}`。explicit直接选定parent；resolve_saved只接saved Query，先读取下段版本绑定的creationPolicy，有则取其parent，无则取DefinitionAddress.owner。inline/adhoc只能explicit。优先级因此唯一为显式请求parent > 保存策略parent > 定义所在Node；选定parent失效/不合格时拒绝，不回退下一层。

SavedQueryDefinition exact `{format:"weftext.saved-query",version:1,query:QuerySpec,creationPolicy?}`；creationPolicy exact `{parent:NodeRef}`，是D5 CollectionCreationPolicy的D7源内承载，不是实体或Query运算。无策略须省略，不用null/空object。它保存在D2 `[weftext-query]` inert payload，通过原DefinitionAddress定位。其address解析、完整owner source revision、wrapper raw bytes、QuerySpec及creationPolicy均绑定同cut，同一原提交的正负依赖覆盖保存策略及最终parent状态/placement范围。重启只能重新从当前source解析或恢复原完整PreparedIntent，不从UI缓存parent当保存策略。policy并发变化使旧prepare依赖冲突；旧saved定义若变成DynamicBlock/View则definition_kind_mismatch，不按标题寻找替代定义。creationPolicy没有新的权限，选择parent仍需原D3当前资格与实际write footprint。

collection_create source必须完整，模板实例化由D9未来提供完整确定输出后才能传入；本Action没有自由template callback。Core把fresh symbolicNode及其全部拟议source/结构/Fields组成proposed workspace，在相同授权观察域完整重新执行CollectionCall，包含sort/take、saveddefs/scalars/负依赖，验证新Node membership。requireMembership=true不成立则整个create拒绝，identity未公布；false明确只是创建Node，不承诺自动出现。集合top1例：新Node合法但排第二时true拒绝，不能用where局部检查代替。

collection_remove只修改明确作者membership事实或该Field。修改后完整postquery必须不含target，另一路关系仍命中、sort/take变化再进入等都按真实结果；不允许单纯移除View显示行冒充修改成功。修改saved Query定义应使用它owner的明确完整source编辑（D8/D9未来adapter），不隐式影响所有共享用户。Trash/删除native row/删Field/移出集合四种语义不合并。

所有postcondition完整proof记录读到的正/负范围、Registry、definitions、auth、rules和scope配置依赖，并在D3/D6planning/commit CAS检查。preview200行仅运输，证明覆盖全部结果；超1000目标或预算整体拒绝。Core内部临时proposed evaluation无新永久Query/Node/索引权威。

## 8. Preview/effects 运输

《D7 Preview and Effects Transport》是本节完整规范：EffectManifest/1、full与owner_fields两种先验profile、全部source/entity/control/semantic effects、existing C的conditional_source_change、D3 companion读取、完整交付epoch投影及独立effect_bytes/1字节运输。D6 resource_bytes/1仅表示已提交Resource读取，不能用于fresh/proposed bytes。preview不是receipt；committed效果必须来自原同一decision，不能重跑Query推测。

所有15种D3模式由该合同§5逐项覆盖。pure lifecycle/placement即使source bytes不变也有完整entity_state_change；fresh/既有源、D4 C及D7 Q物化、bootstrap、period scope及authority变化均由原前后状态独立计算并核对。清单在第一页之前完整形成，缺项/多项/错phase整体拒绝；分页200只是运输。

## 9. DynamicBlock 与 D2承载

DynamicBlock exact `{format:"weftext.dynamic-block",version:1,query,view,arguments,contextBindings}`。query是inline QuerySpec/saved DefinitionAddress二选一；view是`{kind:"inline",spec:ViewSpec}`或`{kind:"saved",definition:DefinitionAddress}`二选一。arguments按参数名映射TypedLiteral或LexicalBinding：`{kind:"literal",literal:TypedLiteral}`、`{kind:"lexical",name:"this.node"|"this.heading_title"}`。this.node由包含block的NodeRef提供；this.heading_title当前最近包含section的text，不存在报context_unbound；没有implicit collection、selection或currentTask。只允许匹配calleeType，SavedQuery本身仍无this。

contextBindings exact为推导依赖的键，值`{kind:"literal",literal:TypedLiteral}`或`{kind:"runtime",name:"now"|"timeZone"|"calendarVersion"|"tzdbVersion"}`。runtime由用户明确的执行context传入，不从device猜；首次refresh为新invocation，now只取一次，不在每row变化。多余/缺失binding拒绝，saved query/view解析闭包先完成再验证。

D2 `[weftext-query]` 的inert payload由D7按format区分SavedQueryDefinition（含完整QuerySpec及可选保存创建策略）或DynamicBlock（执行嵌入）；`[weftext-view]`只接ViewSpec。复用原D2 SavedQueryViewDefinition occurrence、`....`分隔和Locator/anchor，无新增D2元素/wire/identity。QueryRef address若指DynamicBlock返回definition_kind_mismatch，不递归执行其View或把它当SavedQuery；作为View输入的DefinitionAddress只能指ViewSpec。该内容错误是D7，不改D2 lexical valid判据。

portable block不存result/schema快照、cursor、diagnostic、pageSize、刷新间隔、选择/展开/颜色隐藏等UI状态、resolved this、actions、actionQuery/writeQuery。设备可维护绑定document revision+occurrence的可丢弃UI偏好，不参与Query语义、事务或作者source；切换owner/copy后不得携带旧evidence。多个block各自完整Query/View调用，不组成隐式dashboard大查询、跨block参数联动或共享写权。D8负责编辑/布局/RTL交互，本阶段到此合同为止。


## 10. 明确Field occurrence的读取与选择

一般unnest、aggregate、QueryRef的implicit Action lineage仍擦除。需要编辑多值Field时，D8编辑器以用户明确选定的owner/Field调用`d7_field_selection_open`，exact `{wireVersion:1,kind:"d7_field_selection_open",workspaceRef,ownerNodeRef,fieldId,budget}`；这是受管编辑选择入口，不是新的Query operator、FieldRef或持久记录。先按§11的潜在Field读取资格及适用metadata授权，再读当前完整Field、原owner source revision、Registry和全部实际正负依赖；整个Field成功后才返回结果，不先暴露前几条。

成功exact `{wireVersion:1,kind:"d7_field_selection",selectionToken,selectors}`。selectionToken为tag=field_selection的D6 Token，绑定完整Field source cut、按原author order的完整FieldSelector数组、principal/session/delegation、auth generation、Registry、依赖与有限TTL/预算。selectors每项就是上文FieldSelector，包含当前revision及exact rawEntrySource供完整审阅，不披露其它Field/body；其中合法作者Ref/Locator/provenance按Value§5.1只返回原值，实际选择、preview/effects及重放不查询目标状态，后续动作目标仍独立当前解析；8192是D2/D4已有限范围，仍受完整输出字节预算。超预算整体budget_exceeded，不截断selectors。空Field成功selectors=[]，不能选择不存在item。failed响应复用d7_action_error；无权早于existence/schema/源有效性。

`d7_field_selection_evidence` exact `{wireVersion:1,kind:"d7_field_selection_evidence",selectionToken,occurrenceIndex,actionKind}`；index为Counter，只是这个短期选择列表中的位置。actionKind仅replace_field/remove_field/set_field_member。先current audience/read/write/profile、TTL、auth generation和原完整依赖复验，再核对index；过期/变化统一stale_target（已通过适用state披露），越界action_not_applicable。成功复用d7_action_evidence，target为选择列表该项的原FieldSelector；不按value、key单独或最新row补签，两个同值电话的不同occurrenceKey/raw仍各自可选。证据不会将整个Query/Field改成可写集合。用户给完整显式FieldSelector也可fresh prepare，仍byte-match原源并走相同门。

native checklist/table的编辑选择由D2已受权完整DocumentSnapshot里的实际list_item/table_row/table_cell Locator取得；checked=null的普通item不适用toggle/promote。toggle的checked是期望目标状态，必须精确替换原D2 `[ ]`/`[x]`标记，保留其它bytes并完整复验D2/D4；与当前相同是raw no-op，不增加source revision。toggle/promote争用同source revision，至多一个原CAS成功；另一个必须显式重规划，不能去找同标题Task或在已替换NodeLink上继续toggle。

Facet adapters由Core从当前EntityTarget与真实Registry/RelationReadBinding/RecurrenceReadBinding构造完整D4 FacetOperationRequest/2。客户端不给trusted context；expected值来自同cut并逐项绑定。assign的initialEntries是完整raw references，append顺序严格保持；remove一次只移除一个declared Facet；cleanup selectors全部同owner/revision、唯一且raw匹配，数量1..1000，未选Entry与trivia逐字保持。Core将显式Facet/Entry修改共同送入D4后状态/关系/recurrence gate，再生成原D6 PreparedIntent；不能只改Facet header后跳过typed gate。所有适用D4 null成员只存在嵌入原D4对象中，不扩散成D7可选null。任一步失败完整rollback，不发部分effects/receipt。
## 11. 窄Field编辑资格

《D7 Narrow Field Qualification》与同包D6 Policy/2共同定义本版owner_fields正向路径、source_envelope_state及commit_sequence_state的明确授权成本。新capability不由Field read/write推导；source_read也不会自动授权这两个metadata类别。Registry全图先验证明、全D2/D4前后源有效性、全部公开outcome和原CAS/replay仍是必要条件。不能以constraint数组为空或一次运行未碰到秘密值替代证明。

Field selection/evidence与prepare采用同一潜在Field范围；只读选择需要source_envelope_state及完整Field读取，提交/receipt再需要commit_sequence_state。显式的FieldSelector路径不降低这些门。真实用户场景为两个同值phone Entries之一的text修改；同包有界模型必须从真实Registry和完整D2 source构造成功路径，并分别检验所有已授权metadata的影响。
