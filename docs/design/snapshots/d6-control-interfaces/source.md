---
_weftext:
  id: "592603c4-c7ef-4572-aee6-256aa3aa7955"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。

# D6 Control Interfaces

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


本文件补充D6候选，不是已发布API。D3采用同包正式修订候选wireVersion11及Result/9；D2 wireVersion2保持，D4现有实际值decoder保持并由同包符号桥生成其合法输入。D6版本号为独立域。以下object全部closed：未列成员、重复JSON key、null代替必填、错variant、非canonical Ref均拒绝。可选成员仅在明确列出时存在；不得以null占位。

## 1. 公共基础类型

- `Counter`：non-Boolean JSON integer，0..9223372036854775807；不得经double舍入后检查，不接受浮点、指数表示或字符串。溢出拒绝，不wrap。所有版本、计数、字节长度和位置均使用此域。
- `Uuid`：D3 canonical lowercase UUIDv4；Ref和FieldId直接使用D3/D4 decoder，不新增同名近似结构。
- `Token`：Core签发的32个密码学随机bytes的无padding base64url文本，恰43个ASCII字符；末字符满足canonical编码尾位。token是不可猜控制句柄，比较完整bytes，不是identity或能力凭证。解析token形状不访问记录。已认证调用可先访问仅含token/tag/audience及授权定位键的受保护控制映射；作者状态、source、rows和完整记录内容仍在对应当前授权以后读取。各入口的明确顺序优先，不能把控制映射查询当作内容授权。不同token种类由其调用接口与记录tag共同绑定，不能互换。
- `SourceVersion`：closed `{entityRef, revision}`；Document用其owner NodeRef，Resource/Annotation用各自完整Ref。revision使用Counter；类别与Ref一起比较。
- 规范请求比较采用D3-CJ/3对本文件已成功decode的object产生的完整规范bytes，记录协议域`D6-Control/1`；D6不新增包装hash或改变D3 requestFingerprint。

## 2. PreparedIntent与提交入口

D6不提供任意JSON callback、SQL、文件路径写入或自由代码事务接口。D2/D4操作、D5集合意图和D9/D10导入先由其对应Core adapter验证现有closed输入。D7尚未冻结的Action不能通过“通用D6 JSON”提前开放。有效adapter可生成受管PreparedIntent，保存：真实authenticated principal、Workspace、完整canonical输入、唯一语义kind、准确before cut、全部proposed payload bytes、实际MutationFootprint、正/负依赖、适用D2/D4/D7验证证据、§13固定潜在ObservationScope及授权定位记录、预算、到期时间和preview可披露结果。全部结构由Core保存，客户端仅持planToken。

PreparedIntent不reserve content identity，不写OriginBinding，不产生author effects。D3 identity/lifecycle操作直接使用同包D3 v11请求/plan/stages，不经下列D6入口；它可利用同一内部准备器，但不能把D6 token加进D3 closed对象。新建Resource/Annotation、copy/promotion、lifecycle、create/fork/continue仍归D3。D6提交入口只接受既有实体源编辑或D6独立受管控制修改，不能产生D3 identity/lifecycle变化。

`d6_commit_request`恰为：

```json
{"wireVersion":1,"kind":"d6_commit_request","operationId":"canonical-uuid-v4","workspaceRef":{"kind":"workspace_ref","workspaceId":"canonical-uuid-v4"},"expectedAuthorityToken":"opaque-D3-generation-token","planToken":"core-issued-token"}
```

示例的占位值说明类型，并非合法测试实例。expectedAuthorityToken使用D3已有opaque token domain，不能误用本文件Token长度限制。

输入只选定一份immutable PreparedIntent，不能加patch/预算/效果覆写。相同planToken绑定的规范意图永远不变；重规划产生新token及新OperationId。D6 canonical request key包含除operationId外的上述所有成员；ledger同时保存其byte-equal输入、完整规范意图与依赖/plan。planToken本身不能选择另一个Workspace或principal。ledger key仍是WorkspaceId+OperationId，protocolOwner=D6；跨D3/D6 owner同key冲突，D3的比较算法不变。

D6唯一顺序：

1. closed decode和静态关系检查，失败preflight `invalid_request`；零author-state读取。
2. 当前principal对该Workspace本入口授权及§13潜在观察资格；仅可先访问受保护最小授权定位映射以选定固定profile，失败preflight `not_visible`，不读完整plan/ledger/existence。
3. 当前authority与ledger custody/continuity资格；不可证明返回preflight `authority_unavailable`，证实破坏返回preflight `integrity_conflict`。
4. 读同key ledger：不同protocolOwner或canonical request返回preflight `operation_id_conflict`；same saved decision在重新验证其实际效果范围的当前授权后重放原bytes；planned只恢复原plan。先有decision时不要求旧planToken的preview TTL仍有效。原request generation通过连续性证明可重放/恢复，但不能用于unseen的新decision。
5. unseen须expectedAuthorityToken等于current；再解析本principal的planToken并核对tag、Workspace、期限与完整记录。跨principal/missing/wrong-tag统一preflight `not_visible`，不暴露token存在与否；已知本人过期plan可返回preflight `plan_expired`。验证实际footprint授权，失败preflight `not_visible`。
6. 验证原业务依赖、完整语义证据/预算仍可用；确定语义冲突可记录`rejected`，不确定availability不记录。recorded rejection的CAS同时比较ledger unseen、当前资格/授权与该cut；CAS loser零写，回第3步。不能保存一次撤权为永久负决议。
7. 全部通过才原子写planned及完整固定plan和pins。D6无content reservations；同一CAS比较ledger unseen、current authority与全部业务依赖/授权。loser回第3步。
8. 按D6主稿唯一author commit point写全部effects、outbox和committed receipt；暂不可恢复保持planned，确定原plan永不提交才写terminal_failed。物理fence、逻辑authority continuation与业务依赖分别处理。

授权第2步包括先于业务事实的固定潜在观察资格，但不替代真实写集和效果披露授权；第4/5步必须检查实际saved/planned effect范围，防止仅有另一个Field的写权获得原receipt。内部可以受信读取检查，不向调用者披露原意图。D3 replay仍按原D3授权谓词及阶段，不套本顺序。

## 3. 回执与错误

`d6_commit_receipt`恰为`wireVersion,kind,operationId,workspaceRef,commitSequence,sourceVersions,effectsToken`；version=1，kind固定，commitSequence为Counter，sourceVersions为按完整Ref规范bytes排序、无重复的SourceVersion数组。只列实际改变的source；raw no-op可为空。effectsToken引用同decision原子持久的完整效果清单，清单不是隐藏的第二作者源；领取每页仍当前授权，不能用一个token逃逸Field遮蔽。显示200行不截断清单。receipt本身会披露变更对象与版本，须满足上述范围授权；不满足则拒绝整份重放，不擅改已保存receipt。

`d6_error`恰为`wireVersion,kind,code,disposition`，version=1、kind固定。code闭集为`invalid_request|not_visible|authority_unavailable|integrity_conflict|operation_id_conflict|plan_expired|dependency_conflict|semantic_rejected|budget_exceeded|transaction_aborted`；disposition闭集`preflight|recorded|terminal`。前三个阶段、key冲突、权限、过期和availability仅preflight；dependency_conflict/semantic_rejected/budget_exceeded在第6步可recorded；planned后的永久abort仅transaction_aborted+terminal。内部具体原因进单独受权audit，不进入此error。response与调用上下文关联，不回显可能错误的operationId。saved bytes不可变，失败请求不暴露隐藏Field或原request。

D3操作需要的D6 companion只属于其同一decision；保存`protocolOwner=D3,operationId,workspaceRef,commitSequence,effectsToken`与原D3 receipt的精确关联。它不是第二提交成功回执，不改变D3原对象形状、顺序和primary outcome。

## 4. 权限Policy结构

Core受管policy恰为`version,revision,grants`；新Policy/2的version=2，revision为Counter。原Policy/1保留原decoder/能力闭集，不自动升级；当前配置可明确安装Policy/2。grant恰为`subject,effect,scope,capabilities`。subject是host/D10认证映射后的principal Token；外部请求不能声明当前subject。effect=`allow|deny`。scope三variant：`{kind:"workspace"}`、`{kind:"subtree",root:NodeRef}`、`{kind:"ref_set",refs:[EntityRef...]}`；refs非空、无重复、按完整Ref规范bytes排序，全部属于本policy Workspace，不要求实体已存在。capabilities为非空、无重复capability union数组。

Field capability恰为`{kind:"field_read"|"field_write",fieldIds:[FieldId...]}`，集合非空、无重复，按规范FieldId排序。其他capability恰为`{kind:K}`，K闭集：`workspace_state|entity_state|locator_state|source_read|source_write|body_write|node_control|node_create|resource_read|resource_write|annotation_read|annotation_write|lifecycle|registry_admin|binding_admin|policy_admin|export|repair|audit|source_envelope_state|commit_sequence_state`。Field集合不隐含body/title/Facet或control写权。角色展开只能产生这些同形grants；不另引role优先级。workspace级操作只接受workspace scope，不能用subtree的policy_admin管理全Workspace。

workspace_state/commit_sequence_state只允许workspace scope；entity_state/locator_state/source_envelope_state允许workspace或ref_set，禁止subtree。ref_set只配这三个明确state capability，不配Field或其它capability；包含不适用scope/capability的grant由decoder整体拒绝。其授权只看当前principal、policy及请求的完整Ref，在查entity存在、lifecycle、parent、locator或index之前完成，同一ref-domain一致覆盖live/trashed/tombstoned/never-known。workspace allow可被exact-ref deny遮蔽，所以parent隐藏/spouse可见不依赖当前树或对象存在。最小tombstone不得为授权而增存parent/旧ACL。以下按当前parent匹配的subtree规则仅适用于已经通过适用state-disclosure闸门之后的内容/Field/效果权限，不用于定义existence visibility。按子树动态隐去存在仍不属于该首版state grant。

匹配按当前受信principal、Workspace、完整Ref或适用的当前parent关系；deny优先于全部allow，默认拒绝。source_read/source_write是完整source能力，但显式某Field read deny阻止包含该Field的全源披露，write deny阻止实际修改该Field的proposal；原始源输出必须证明全部内容可读，Field API只投影允许事实。title/coreKind/Facet声明改变须按下面footprint矩阵取得资格并通过适用typed gates；body_write不能修改这些结构。resource/annotation还需完整owner scope，状态披露与内容读取分别判断。export需要export及其所有输入读取权。policy修改要求旧policy下policy_admin，原子保存整个closed policy并增revision/auth generation；撤销自身管理员权限是显式效果，可提交后立即生效。D8可设计防误操作preview，但不引入隐藏的永不可撤销管理员。

修改source、control或policy后Core计算实际effect并更新受影响授权范围generation；move需old/new域资格和D3现有规则。未授权subject不能通过计数、错误详因、缓存、效果清单或subscription得知隐藏facts。

能力矩阵是唯一蕴含规则，未列蕴含均不存在；匹配的explicit deny压过直接或派生allow：

| 实际footprint | 所需能力 | deny裁决 |
|---|---|---|
| 已证明仅普通body | body_write或source_write | body_write deny或source_write deny中实际适用的源范围禁令均阻止该效果 |
| 某Field的值/Entry note/key | 该Field的field_write或source_write | 该Field field_write deny优先；仍执行D4各具体变换门禁 |
| 普通文档title/subtitle元数据 | source_write | source_write deny阻止，不由body_write授予 |
| coreKind/declared Facets等影响Node分类的声明 | source_write且node_control | 任一deny均阻止；不能凭整源写越过Node control禁令 |
| parent/ordinal/lifecycle | D3请求以及node_control/lifecycle等其适用授权 | D6 source edit禁止产生此类effects，source_write不蕴含它们 |
| Resource/Annotation现有payload | resource_write或annotation_write | 不被source_write蕴含；新identity仍D3 |
| 无法完整分类的原字节差异 | source_write且证明没有任何适用细项deny被隐藏 | 无法证明则拒绝；不以source_write跳过typed检查 |

source_write deny表示该范围完整源禁止修改，不能绕由field/body细项allow；field/body deny仅限制相应细项，但一份包含其修改的整源proposal也被阻止。未改动的隐藏Field可由Core原样保留；只有§13可证明结果与其独立的owner_fields路径才可继续局部Field edit。不能以内部保留和错误遮蔽替代潜在结果观察资格。

write不蕴含read。D6正常提交/重放回执还要求其sourceVersions的entity-state披露资格；效果清单中before/after值分别按对应read能力交付，缺权返回遮蔽结果而非原文。受信内部footprint不受此输出裁剪影响。D3回执披露仍按D3原合同，不以此表新增D3阶段。

### 4.1. 作者引用原值与目标状态

已获权作者内容中的普通typed Ref/Locator原值与其指向对象的当前状态是不同读取对象。对当前主体完整获权读取的Field/source，只按原D3/D4 closed decoder验证并保留其中完整作者Ref、历史Locator、revision token及坐标原值；此投影不查询目标Workspace、existence、lifecycle、title、content或当前位置，也不声称引用当前有效。授权来自该完整作者事实/源的读取权；Field/source读取不蕴含目标state/content资格。此规则适用于普通非relation Ref值及D4现有provenance atoms，包括跨Workspace Node provenance和合法owner-local Resource provenance；AnnotationRef仅在原schema已有的普通值位置适用，不新增Annotation provenance atom。D4原domain/locality不变。

relation实际endpoint的domain/state、关系可见性、incidence、graph/遍历/join及点击解析仍先通过目标当前state/content和适用locator授权，再读取目标。D4语义角色区分actual relation endpoint与附属作者provenance，不能仅因JSON内出现node_ref就混同。renderer不得附加自动resolved标志、目标当前标题/缩略图/状态或位置；显示作者原值不授遍历权。依赖目标状态的Query/Action显式列入原潜在观察范围、真实依赖与CAS，不能使用只保留作者bytes的窄证明。

同一规则贯穿Query read、Field selection/evidence、preview、effects和重放。完整Field/source失权按原当前门阻止披露；只有被引用目标失权不改变仍获权的作者原值，但禁止该目标后续解析。任何阶段不得删Locator、改none、丢atom或改transform inputIndex。此处不会开放Query内部occurrence/source-lineage。

## 5. 结果句柄与分页

内部ResultRecord的必备绑定由主稿§8定义，状态闭集`producing|complete|failed|cancelled|expired`；这些是受管控制状态，不是Node lifecycle。结果来源schema和row值使用D7最终冻结的typed result；D6不得新增any/JSON row fallback。

`d6_result_page_request`恰为`wireVersion,kind,resultToken,cursorToken,pageSize`；version=1，kind固定，两token为本文件Token，pageSize为1..200的Counter。首cursor随已complete的result描述返回；cursor不由调用方提供数值offset，记录绑定同result/epoch/位置。`d6_result_page`恰为`wireVersion,kind,resultToken,rows,terminal,nextCursorToken?`；terminal为真正JSON Boolean，与Counter分域。terminal=true禁止nextCursorToken；false必需nextCursorToken，即使rows=[]。rows严格使用已complete且当前获权的D7 schema，不允许截断一行内部值。producing只能查询进度，不能通过page入口获得partial rows。

取页先验audience/当前授权，再验handle state、expiry/pin与cursor绑定；失败不返回任何rows。完整结果在发布前已完成所有语义和预算检查，后续运输错误不会变成EOF；取消或expiry后不得默默重跑到新cut。订阅delta wire留D7，但D6要求每次交付当前授权、epoch和无缺口sequence；无法证明则reset，禁止拼接旧epoch结果。

§3的d6_error仅用于commit/plan入口。结果读取的错误对象恰为`wireVersion,kind,code`，version=1，kind=`d6_result_error`，code闭集`invalid_request|not_visible|result_expired|reset_required|cursor_invalid|result_not_complete|result_unavailable`，禁止rows/terminal/nextCursorToken或内部cause。顺序固定：closed decode→当前authenticated principal下的opaque handle audience查验及完整读取授权→过期→epoch失效/reset→cursor绑定→结果状态→实际读取。opaque handle查验只访问受保护的token/tag/audience控制映射，不查entity、作者内容或结果rows；missing/wrong-tag/wrong-audience及撤权统一not_visible。仅对已知本人且当前仍获权的handle可公开expired/reset等状态。结果producing→result_not_complete，failed/cancelled/pin不可用或实际I/O失败→result_unavailable；expired优先于reset，reset优先于旧cursor错误。任何错误均不是成功EOF；返回空nonterminal页只适用于已complete结果的合法运输游标。

## 6. 预算

BudgetBinding恰为`version,maxInputBytes,maxSourceBytes,maxWriteBytes,maxDecodedBytes,maxWorkUnits,maxDependencies,maxNewEntities,maxTemporaryBytes,maxElapsedMillis,maxResultBytes,maxOutputBytes`；version=1，其余都是Counter，0表示不允许消耗该资源而非无限。实际预算取request限制、Workspace policy、host能力的逐项min；服务不得将0解释成默认大值。预算生成时host另存单调时钟deadline与取消标记，它们不是用户可写JSON，也不将wall clock回拨当扩额。

所有子操作共用一个累计账户，计数增量在运算/分配前checked-add；不能先超限后截断。D5固定实体/row/page上限再取min，不能以maxNewEntities覆盖1000新Nodes的batch上限。Snapshot pins、WAL增长、staging、Result spool和导出共用临时磁盘上限，history/作者数据保留不能被此预算当cache删掉。限制不可满足则显式budget_exceeded，不把source变语法invalid，不以partial结果成功。

恢复必须保持预算语义：input/source/write/new-entity/result/output是绑定整个固定plan的规模上限，不因attempt重启扩大。work units按执行批次**先持久charge再执行**，崩溃后已charge但未完成的批次也算已耗，不能回退；同一plan所有子任务/恢复共享已耗counter。临时磁盘按仍存活的全部pins/staging实际占用与已reservation额度计费，不按某个worker的局部视图重置。

maxElapsedMillis是该次执行attempt的上限，非整个planned等待墙钟期限。每个attempt有持久attempt序号、clock epoch、起点及结果；同epoch可证明时从原起点继续，epoch丢失则原attempt结束为interrupted，不能假装时间尚未经过。Core policy另设持久有限attempt allowance，签发/消费attempt资格先落盘且与plan绑定；崩溃不自动补回，额度耗尽保持暂停。只有当前Workspace scope的policy_admin可按§8闭合控制意图明确新增有限attempt allowance，它不改变canonical plan、源/实体限额、已耗work units、IDs、effects或原request/decision，也不是重新规划。无法证明剩余额度或当前clock时停止该attempt、保持planned，不把预算不确定转成D3 terminal abort；若要改意图/提高原plan语义规模上限则必须新OperationId。执行资源policy不是ordinary客户端可覆盖的BudgetBinding字段。

## 7. 导入Job与下游边界

ImportJob是内部持久控制记录：authenticated owner、输入artifact完整descriptor及pins、mapping版本、有限dependency graph/SCC与atomic group、显式batch顺序、每batch稳定OperationId、明确protocolOwner及其canonical request/完整plan、已commit receipts、binding/version/watermark前缀、状态和预算。D9/D10冻结各source输入/比较器/映射closed结构，D6禁止在其冻结前提供任意provider callback写权限。

批次的author effects必须全部属于其protocolOwner对应canonical request绑定的完整plan，job/binding/cursor是从该输入与实际结果派生且同事务保存的控制effects；不能通过job为任一协议添加未绑定的作者修改。零fresh identity且仅既有payload/control修改的batch使用D6；有fresh且无既有修改使用D3原create/import语义；fresh加既有源修改的atomic group按下述D3 per-mode矩阵选择其合法组合，不以fresh数量直接选择新增数组。不能把一个不可拆group分成两个协议decision。每batch的owner、input descriptor、mappingVersion、group成员及其完整proposed bytes在planning前固定；D6与D3共用Workspace+OperationId ledger，owner冲突不会落第二条decision。batch receipt、SourceBinding/OriginBinding变更、local notes保留证明和watermark必须在同一author commit事务保存。只有已commit且前驱完整的batch可推进job前缀；预先下载、preview或planned不推进。same request精确重放不重复推进；mapping/input/groups变化创建新job意图，不回写旧batch。改变mapping/input/atomic groups是新显式job意图，已commit batch不得改写。读取进度需当前授权；暂停/取消只阻止未planned的后续batch，不将planned偷偷burn或回滚已commit前缀。用户可见job partial成功与单batch完整事务分开。

上述fresh/既有分类不能替代D3的per-mode admission：新增existingPayloadEdits仅用于create_node/create_resource/create_annotation及ordinary_format import_new。copy_resource/copy_annotation允许的destination-owner既有源组合继续使用原payload pair、slot/S及receipt矩阵，新增数组仍为空；Node copy/fork/partial identity import不得增加原矩阵禁止的既有写入。ordinary import的fresh Resource/Annotation owner明确只取其声明的new Node subject；在既有owner下创建附件/批注应选择适用的create_resource/create_annotation意图，而非暗改import owner规则。任一batch不能由“含fresh”推导不存在的mode或混合两个decision。

上述控制记录的SQL布局、RPC运输和host clock实现由实现切片决定；不得改变本文件closed外部接口、依赖/授权/幂等顺序，或把未来D7/D9/D10的未冻结输入伪装成当前已支持能力。它们是明确下游实现义务，不是本次运行验收。

## 8. 受管配置与执行资源意图

本节及§8.1的四个version=1 closed输入只进入Core准备器，成功返回§2的planToken；提交仍使用唯一d6_commit_request及同一Workspace+OperationId ledger。没有独立无账本管理旁路。管理意图自己的operationId由提交请求提供；不得等于被管理计划的OperationId。planToken绑定本意图全部规范bytes。unknown member、null替代下列variant、Boolean/浮点Counter均拒绝。

`d6_series_configuration_intent`恰为`wireVersion,kind,workspaceRef,seriesScope,expectedConfiguration,expectedRangeRevision,targetMultiplicity,expectedPolicyRevision`。kind固定；seriesScope恰为`{series,scope,policyBinding}`：series是D4七成员`{calendarId,calendarVersion,timeZone,tzdbVersion,periodKind,periodRuleId,seriesKey}`，scope复用D4 workspace/node闭集，policyBinding恰为`{registryBinding,policyId,policyVersion,policySchemaDigest}`且与同snapshot的CalendarSeriesScopePolicy/1三方相等。配置适用于这一series+scope的全部periodKey；它不是D4冲突key。单次创建的冲突key仍恰为`{series,periodKey,scope}`，各periodKey独立unique，绝不禁止同series不同period共存。配置管理range覆盖同series+scope的全部period及其完整负范围revision；验证unique时逐个完整D4 key分组，任一组重复则拒绝。Registry/policy更换必须新意图绑定当前精确policy并重新证明完整范围，不能只凭label或客户端散列；targetMultiplicity=`unique|many`。expectedConfiguration闭集`{kind:"absent"}`或`{kind:"revision",revision:Counter}`。range/policy revision为Counter。Core从同一cut取得对应配置、完整scope与当前policy，逐项相等才可准备；不存在配置须显式absent，禁止默认值。新建unique或many→unique必须证明整个scope无冲突；unique→unique也复验现有unique不变量。首次many仍读完整scope并绑定版本，不伪造空范围。现配置与目标相同属于显式no-op，保存幂等decision但不增配置revision；新建revision=1，实际改变checked-add 1，MAX拒绝。commit同时CAS旧policy/config/range及当前授权，并使依赖该配置的plans/results失效。

`d6_attempt_allowance_intent`恰为`wireVersion,kind,workspaceRef,targetOperationId,targetProtocolOwner,expectedResourcePolicyRevision,increment`。kind固定；targetProtocolOwner=`D3|D6`；increment为1..MAX的Counter。targetOperationId选择同Workspace已planned的原decision；不是新意图的operationId，也不能把D3/D6 owner互换。Core在授权后读取原plan及其独立execution-resource-policy revision，准备前核对planned与exact owner；committed/rejected/terminal_failed不可补充。新增额度为checked-add，有限host上限、Workspace资源上限以及仍存活pins/磁盘总占用继续生效；不能通过policy_admin超出host可证容量。补充不改变原request、原plan、fresh refs/reservations、已耗work、原语义规模预算或source版本。resource-policy revision因实际补充、attempt签发/消费/中断状态变化及已耗work/pins计费账户变化每个实际事务均checked-add1；每项变更CAS同时比较旧revision和相关exact旧counter。准备期间任一次消耗都会使旧管理计划失效，不能用预先算好的after覆盖新消耗；任一上溢或容量不足整体拒绝。

**唯一授权映射。** 本节两入口以及相应commit/replay都要求当前policy下、同Workspace、workspace scope的policy_admin。registry_admin、binding_admin、node_create、source_write、Field写权及subtree/ref_set内同名grant均不授此权；deny压过allow。初始入口先验该Workspace管理权限；Calendar配置还须先验§13全域观察资格，之后才读configuration、target ledger或资源额度，拒绝统一not_visible，不透露对象是否存在。scope/target内容的受信检查不向管理员附带隐藏作者值；preview只交付其具有读取权限的详情。policy revision是CAS依赖而非授权凭证，当前撤权先赢时拒绝新交付/提交；不能凭旧preview继续写。

两种管理提交复用§2顺序与错误闭集，expected version/state失配采用dependency_conflict；非法目标kind/owner静态失配invalid_request，授权后发现target不是planned采用semantic_rejected；不向外披露内部分类。规划后遇到并发原计划commit/abort，管理计划按确定依赖冲突terminal abort，不向已经terminal的原计划追加额度；若补充先赢，原plan可继续且原decision不被改写。原计划attempt消耗与额度管理共用resource-policy CAS，不能lost update。管理请求的same-key canonical replay返回保存的同一receipt，绝不再次补额；不同输入同key冲突。管理receipt的sourceVersions为空；effectsToken绑定对应exact control before/after，effect交付仍需当前Workspace policy_admin。

### 8.1 Period范围绑定与显式解除

CalendarPeriodScopeBinding是D6控制事实，绑定一个具有有效calendar/period Entry的NodeRef到准确D4 scope；series与periodKey仍每次从唯一完整作者源解释，不能另存可编辑副本。每Node绑定的控制revision属于独立范围版本，首次1，实际scope/有无绑定变化checked-add1，删除后保留仅用于防ABA的控制版本；MAX时整体拒绝。它不进入D3最小tombstone，也不保留已purge源/分类。若没有有效period Entry则没有活动binding；Trash期间保留绑定及原D4范围归属。

**确定选择。** 普通新period（D3 create、ordinary-format import，或既有Node首次增加period）固定选择其Workspace scope；这条规范规则必须作为显式scope写入原plan，不能从当前冲突或index结果决定。所选series+scope配置必须已通过显式管理建立，不默认many，不在普通源写中自动建配置。对已有period的源编辑保留准确scope，即使series或periodKey改变，也必须读取新旧完整范围和新配置；删除period的源变换同事务移除自身binding。显式改变scope使用下述唯一控制意图，不偷偷改Field或Node identity。

managed Node copy按原source cut携带每个copied period的控制scope：workspace改为target Workspace；node scope按D3完整Node map重绑定，same-Workspace map外合法live/trashed Ref可保留；跨Workspace map外或不可解释scope使整copy失败，不能改成workspace逃过原约束。普通artifact不携带可凭空信任的控制binding，按上述新period规则；完整fork按§14同时重绑定配置与binding。copy/create不改变已有配置。任一适用D3 mode须在原stage14/15验证这些确定规则与完整范围，仍不新增wire字段；D6生成的控制effects由原request、真实source cut和当前受管配置唯一派生并在plan固定。

`d6_period_scope_intent`是第三种closed准备输入，exact为`{wireVersion:1,kind:"d6_period_scope_intent",workspaceRef,nodeRef,expectedSourceRevision,expectedBindingRevision,targetScope,expectedPolicyRevision}`。nodeRef必须同Workspace；targetScope复用D4的workspace/node union且同Workspace；三个revision都是Counter。它只重新选择已有有效period的范围，不增删作者Entry或identity。先当前Workspace policy_admin及§13 workspace_constraints观察资格，再读取该Node/source及绑定；同cut核对这些expected值，解释真实period/series，读取旧/新配置与完整负范围及其版本，验证新scope Node的live/trashed状态和D4 key/unique语义。缺period、缺配置或无合法旧binding拒绝semantic_rejected；expected失配dependency_conflict；不能用不存在的Node初始化绑定。PreparedIntent保存完整旧/新scope及所有实际source/state/Registry/config/range版本，commit逐项CAS。目标scope相同仍复验有效性，保存no-op decision不增版本；实际改变只增binding及旧/新range控制版本，sourceVersions为空，原Document版本不变。effects包含准确控制before/after，只向当前全域观察且policy_admin的主体交付。

`d6_series_configuration_remove_intent`为第四种closed准备输入，exact为`{wireVersion:1,kind:"d6_series_configuration_remove_intent",workspaceRef,seriesScope,expectedConfigurationRevision,expectedRangeRevision,expectedPolicyRevision}`；seriesScope与§8同型，三个revision均Counter。当前policy_admin及全域观察资格通过后，同cut验证配置存在及expected值，证明其完整series+scope范围没有任何live/trashed period binding；空index不能证明空范围。只可删除不再使用的配置，不连带删/改源或重绑period；有任一使用者拒绝semantic_rejected，失配dependency_conflict。删除与range/config负版本变更、receipt/outbox同一commit，保留独立控制世代防止删除/重建ABA；随后显式新配置的逻辑revision重新从1开始，但全部prepared依赖还绑定该不复用的范围/control世代，不能只比较逻辑revision。无作者source增版。

两个新增意图都进入§10 d6_prepare_request/§2 d6_commit_request的同一closed union、同一ledger和错误顺序，不是自由管理callback。§8配置创建/修改与新增scope/删除入口均要求完整观察证明；attempt allowance保持其明确可披露控制授权，不要求完整作者读权。scope Node是控制依赖：建立/保持node scope要求同cut证明该Node live/trashed；purge检查控制inbound，仍被任何binding或配置指向时整purge拒绝。可先按scope意图移走相关period（或通过合法源编辑删除其period），再删除空配置，最后按D3 purge；每步显式、受权，不能猜测重绑定。若purge集合内部period一起被purge，其自身binding在同一plan删除，不算最终剩余inbound，但配置不会在purge中自动删除，须先解除。该有意成本换取配置可解释性，不授予purge隐式管理权。

**managed copy的fresh scope配置。** 上述“copy不改变已有配置”不禁止一个必要且封闭的新配置初始化分支：若node scope的原Node恰在本次D3 Node map中，其target scope Node是已证明的fresh prepared对象，Core必须将source cut的对应完整配置重绑定到该fresh scope，保留multiplicity及完整D4 policy含义，作为原copy计划的控制effects。不能预先要求尚不存在的新Node live或要求调用第二次管理提交。只有原映射scope的负inventory/配置负范围、原source配置及所有prepared period候选都完整可证，且target Registry可验证其policy时才可准备；source配置缺失/不可迁移、target配置碰撞或prepared unique冲突均整copy拒绝。首份配置revision=1、binding=1；source配置历史版本作为原plan读依赖，target新control世代独立。该分支仅建立本次映射fresh scope所需的配置，不改既有target配置、不为workspace scope或map外既有scope隐式建配置；后者缺配置仍须显式管理。所有fresh scope Node的状态按D3受保护prepared-origin和原reservation验证，提交后必须live/trashed；全部源/配置/binding及原receipt一次commit。配置派生由原copy输入、map、真实source cut唯一决定，不是可编辑companion或新增D3 wire。fullfork仍按§14完整处理所有source配置。

§10控制读取target额外接受`{kind:"period_scope",nodeRef}`；成功state恰为`{kind:"period_scope_state",binding,sourceRevision,policyRevision}`，binding为`{kind:"bound",scope,revision}`，只有当前有效period可返回；不存在/无period/无binding统一control_unavailable。三个revision为Counter，scope同D4。该读取及series_configuration读取均先证明当前policy_admin和workspace_constraints，再访问绑定/范围与返回其版本；execution_resource仍仅其原控制权限。控制读取不新增通用作者源入口，不向局部管理员泄露隐藏范围的变动计数。

## 9. 永久暂停的可观测成本

原plan工作额度耗尽或重启丢失已计费工作后可能永远无法完成，即使补充attempt次数也是如此。保留planned、reserved/burn-history语义和全部必要pins；不添加abandon API、不把用户取消/TTL/磁盘紧张当确定业务冲突，不回收其必要作者源证明。当前Workspace policy_admin可读取受管执行资源摘要：targetOperationId/owner、planned状态、已耗与上限、pins字节、attempt余量和可证明的暂停原因类别；源值与实体明细仍单独受权。普通subject不可枚举该摘要。

持续保留会消耗存储，甚至使新增计划因总容量不足而拒绝。这是首版明确成本，不承诺自动解锁或无限运行；管理可补充实际host容量，不能提高原plan语义额度或删pins。工作量永久耗尽时，资源摘要明确标记不可由attempt补充恢复。新OperationId可表达新的业务意图，但不会释放旧plan或复用其reservations。后续若要设计放弃协议，须独立冻结commit/abandon CAS、授权和burn规则；不属于当前能力。

## 10. 准备与控制状态读取

四个§8/§8.1意图的外部准备入口恰为`{wireVersion:1,kind:"d6_prepare_request",intent:<§8/§8.1四个closed意图之一>}`；成功恰为`{wireVersion:1,kind:"d6_prepared_intent",planToken}`。先closed decode→当前Workspace policy_admin及适用§13潜在观察资格→当前authority/custody→实际输入/版本/约束/预算→保存immutable PreparedIntent。失败使用d6_error且仅preflight；没有OperationId、没有ledger decision或content reservation。PreparedIntent保存其Workspace及主体，TTL为Core执行资源policy给定的有限值，过期提交按既有plan_expired；读取配置版本不等于准备成功。

`d6_control_read_request`恰为`{wireVersion:1,kind:"d6_control_read_request",workspaceRef,target}`。target闭集`{kind:"series_configuration",seriesScope}`、`{kind:"period_scope",nodeRef}`或`{kind:"execution_resource",targetOperationId,targetProtocolOwner}`，成员复用§8。先closed decode→当前Workspace scope policy_admin（series/period再加全域观察，execution_resource按control_only）→当前authority/custody→同一read cut取得完整控制状态；输出前重验当前权限。missing target与wrong owner在已授权后统一control_unavailable，不暴露作者内容。series配置缺失是明确合法absent，不等于范围为空。

成功`d6_control_state`恰为`{wireVersion:1,kind:"d6_control_state",workspaceRef,target,state}`，target逐字回显已经成功decoded的请求target。series state恰为`{kind:"series_configuration_state",configuration,rangeRevision,policyRevision}`；configuration为`{kind:"absent"}`或`{kind:"configured",revision,multiplicity}`，全部Counter/multiplicity复用§8。rangeRevision取整个series+scope的完整range，不返回隐藏Node数量或Entry值。resource state恰为`{kind:"execution_resource_state",targetState,resourcePolicyRevision,attemptsRemaining,workCharged,maxWorkUnits,pinnedBytes,pauseCategory}`；targetState=`planned|committed|rejected|terminal_failed`，pauseCategory=`none|attempts_exhausted|semantic_work_exhausted|continuity_unavailable|capacity_unavailable|authorization_unavailable`，数量均Counter，已terminal目标仍可受权读取历史资源摘要但不能补充。

失败恰为`{wireVersion:1,kind:"d6_control_error",code}`，code闭集`invalid_request|not_visible|authority_unavailable|integrity_conflict|control_unavailable`，不得附带state或内部cause。这里的管理控制读取不提供通用作者source读取或枚举所有OperationId；输入必须有准确target。§9的摘要由此closed入口交付，UI可由自身已授权操作清单选择target。失败不生成ledger decision，不使用d6_result_page处理管理控制状态。

## 11. D2 Resource ByteHandle与字节读取

D2 wireVersion2的resource_snapshot及其byteEnvelope保持原形；byteEnvelope恰为`{kind:"d6_byte_handle",handleToken:<ByteHandle>}`。ByteHandle的文本域是§1 Token，语义tag固定`resource_bytes/1`；不是D3 EntityRef、43字符的源revision、上传staging、plan、effects、Query result或cursor。其他tag的合法Token也不可在本入口互换。D2外层resourceRef及ownerNodeRef必须分别等于签发记录的完整ResourceRef及其owner，不靠同leaf UUID、名称、路径或digest匹配。

**签发与不变绑定。** Core仅在受权构造D2 Resource snapshot时签发；它不是客户端任意创建的handle记录。先D3适用的Workspace/entity-state披露门，再当前完整resource_read及owner scope，再同一可信cut取得已提交完整Resource版本和descriptor。签发成功原子保存受保护的handle/tag、audience（principal+session/delegation）、Workspace、完整ResourceRef、SourceVersion、对应resourceRevisionToken、原immutable cut、完整有序payload descriptor及其确切总长度、所需pin、有限期限/clock epoch、lifecycle/continuity交付epoch、预算绑定和已耗counter。descriptor覆盖全部已提交bytes并已可证明完整耐久，不能签发receiving staging、未提交plan或不完整upload。不能只保存当前payload指针，也不能每次读时重新解析latest。D2 derived.length/digest等仍是其原可空派生成员，不代替本绑定，不向D2新增revision字段。

Resource从R1更新到R2时，已签发H继续指向pin住的R1，读取成功回显R1 token；source更新本身不使这个immutable快照重绑或reset。Ref/owner的lifecycle变化或已证明不再延续原快照的authority epoch变化使交付epoch失效；权限变化先重新授权，不因generation数字变化就假定仍可读或永久拒绝。合法holder/authority连续接管可保留完整记录及pin；无法证明原record/clock/pin连续性则停止读取。相同bytes/digest、Trash再restore、重新授权、同名新资源都不能复活已失效/过期H。释放pin不改作者数据；已回收控制映射的token按missing处理，不承诺永远保留expired记录。

**closed Core读取协议。** `d6_byte_read_request`恰为`{wireVersion:1,kind:"d6_byte_read_request",handleToken,offset,maxBytes}`。offset为Counter，maxBytes为1..1048576的Counter；无客户端ResourceRef/最新版本选择器、URL、SQL、callback或可增额预算。offset是同一固定byte sequence中的octet位置，不是Unicode列；允许独立重读/并行读取，但全部请求共享该handle的累计预算。此Core协议的HTTP/CLI/native封装不得改变形状、整数或可观察语义。

成功`d6_byte_chunk`恰为`{wireVersion:1,kind:"d6_byte_chunk",handleToken,resourceRef,resourceRevisionToken,offset,totalBytes,data,terminal}`。data为本次原bytes的canonical无padding base64url文本（空bytes唯一编码为空串）；它不是§1固定32bytes Token。decoded data长度恰为`min(maxBytes,totalBytes-offset)`，totalBytes是固定descriptor长度，offset逐字等于成功decoded请求。terminal为真正Boolean，恰当`offset+decodedLength==totalBytes`为true。offset==totalBytes允许空terminal成功，包括总长0；offset>totalBytes固定range_invalid。非terminal必须非空，实际short read、缺chunk、损坏或I/O异常不得伪装空页或EOF；本次完整预期chunk已读取核验且过交付门后才输出，失败不交部分data。terminal只说明该请求到达固定字节序列末端；完整下载消费者仍须证明同handle/version的0..totalBytes无洞覆盖，不能从直接seek末尾的空terminal声称取得全Resource。

错误恰为`{wireVersion:1,kind:"d6_byte_error",code}`，code闭集`invalid_request|not_visible|byte_expired|byte_reset_required|range_invalid|byte_unavailable|budget_exceeded`；禁止data、terminal、ResourceRef、总长度、源token或内部cause。唯一顺序为：(1) closed decode/Counter/Token词法；(2) 当前authenticated principal下查询受保护tag/audience映射，随后对记录所绑定完整Ref重新过D3披露及完整resource_read/owner-scope授权；(3) 期限；(4) 已证明失效的lifecycle/continuity交付epoch；(5) offset范围；(6) pin/descriptor/backend连续可读性与资源预算；(7) 读取和核验完整所请求chunk；(8) 与当前撤权/expiry/reset串行的最终交付门，再输出一份成功chunk。missing/wrong-tag/wrong-audience、无法证明当前授权和撤权统一not_visible，不能先泄露旧版本存在性。只对已知本人且当前获权H公开其他状态；expired先于reset，reset先于range。任何gate在首次需要的事实不可证明时当场停止，不可跳过后再报告后序错误：当前授权不可证明为not_visible；已获权但期限gate的clock epoch不可证明立即byte_unavailable，先于reset/range。pin缺失、无法证明连续性、clock epoch丢失、存储损坏及I/O失败统一byte_unavailable；不暴露其内部区别。最终交付门若状态改变，按同一当前授权→expiry→reset优先级拒绝整份chunk。故障不是成功EOF。此读取不生成OperationId或改变D3/D6 author decision。

**有界计费。** 签发时选定有限BudgetBinding，取Workspace/host限制的逐项min并验证完整descriptor/pin规模，0不是无限。所有同H读取共用持久原计数，重读、并行请求、传输重试和进程恢复不重新发一份额度。每请求在读/编码/分配前checked-add并保留足够source/decoded/work/output额度；output包含base64扩张和完整成功envelope，不能只计原bytes。预留和扣费以同一预算record的旧revision/counters做CAS；每次失败后已charge的工作不退款，不能用并发读绕过额度。每次分配/读取不超过maxBytes及剩余host能力，预算不足拒绝整chunk，不截断成短成功。期限由有限TTL和可证明clock epoch给出，等待不延长TTL；pin/实际存储占用按§6计费。输出前并发撤权失败可消耗已执行额度但绝不披露bytes。另行重新读取Resource snapshot可获得当前受权的新H及新policy配额，不会恢复旧H或承诺仍是旧版本。

## 12. 跨阶段继承类型与未开放的运输边界

SourceVersion是完整Ref+numeric Counter；D2的`<D6:DocumentRevision>`及D3既有document/resource/annotationRevisionToken使用主稿§4的d6d/d6r/d6a类别token。三种代码类型DocumentRevision、ResourceRevision、AnnotationRevision归SourceVersion概念，均不是43字符随机Token。构造/比较需要对应完整Ref、类别、storeIncarnation及numeric revision；上游D3原opaque lexical decoder和closed member名字保持，不能在旧D3入口偷偷加入D6词法前置门。D6自己签发的token必须满足其本类别规范，不能以只相等的revision数字跨Ref比较。

D3的sourceSnapshotCutToken、continuationCutToken及generation/custody token保留D3 wire所有权；D6提供分别可证的immutable source cut、包含完整ledger/custody的continuation cut及authority连续性，不把普通MVCC cut或ResultHandle TTL代入。D4 RelationReadContext/2与RelationReadBinding/2的source/entity-state/incidence三类依赖，以及保持原版的RecurrenceReadContext/1及其范围revision，由D6可信读取器提供，shape仍归D4，不能当新的公开任意JSON读取API。D7拥有ResultRowHandle/row schema，D6 cursor只作运输定位。以上继承映射与ByteHandle实际入口一同纳入跨阶段术语库存。

stageInput仅为Core内部受限语义接口；未来Web/移动文件输入的上传/续传协议由D9/D10适配契约另行冻结，不作为当前D6已定义的外部opaque upload handle接口。ByteHandle只读已提交Resource，绝不用于读取其他主体staging、封存upload或授权提交。D2 strict UTF-8失败后的原Document byte/repair envelope是D6 repair层义务；普通Resource ByteHandle不冒充损坏Document payload，也不自行扩大为通用Document下载。repair运输在相应实现入口冻结前须单独定义完整受权byte envelope，保留原bytes并禁止D2伪attestation。

effectsToken仍只指同decision的完整效果清单，D3 companion精确保存§3列出的五个关联成员；sourceVersions属于已保存效果证据，不在companion外层增成员。其未来运输必须独立tag/closed schema/完整分页终点/当前授权，禁止放入D7 Query row fallback或200行截断；这是实现接口冻结前明确未完成的运输义务，不宣称本稿已提供新的effects RPC。

D4新版stateToken是受保护DependencyProof中的非空scalar字符串引用，词法由D4 state binding定义，不能作为§1固定32-byte Token、SourceVersion、ByteHandle或客户端授权。它只进入可信关系上下文和完整commit依赖，不向D3/D6 sourceVersions或source-write effects加入无源端点。旧RelationReadContext/1不得通过填revision=0或空Facet自动升级。

## 13. 潜在约束观察范围与准入

写入权、Core内部读取权、对外观察约束结果的资格是三个条件。后者覆盖成功/拒绝、是否产生新对象、no-op、preview、业务错误、receipt/effects、恢复后结果及订阅产生的语义事实；不只是错误details。只有真实写权不能试探隐藏unique键、incoming、基数或跨Field谓词。Core不能在扫描命中隐藏行后才决定是否需要观察授权，也不能因当前范围为空、只有可见行或index无结果便授予资格。该规则适用于有效作者状态之间的秘密事实差异，不宣称消除恶意OS破坏、物理耗时或已明确公开的资源容量带来的所有信息。

`ObservationScope`是Core受管计划内的closed技术投影，不是新权限、token、D3 request member或客户端自报证明。`kind="d6_observation_scope",wireVersion=1`，profile四variant：

| profile | exact members（包括kind/wireVersion/profile） | 固定资格 |
| --- | --- | --- |
| workspace_constraints | kind,wireVersion,profile,workspaceRef | 下述全Workspace潜在内容和控制观察证明 |
| owner_fields | kind,wireVersion,profile,workspaceRef,ownerNodeRef,fieldIds | 同一完整owner及有限sorted unique FieldIds；只有下述静态独立性证明成立才能选用 |
| control_only | kind,wireVersion,profile,workspaceRef | 仅纯policy修改、attempt allowance及其他已closed且可证明无作者条件依赖的受管控制意图；依原控制观察与管理权限 |
| prepared_workspace | kind,wireVersion,profile,workspaceRef | 仅D3 create/fork的全fresh target；stage3按公开mode/外层target角色和当前issuer准入形成临时资格，原P1/P2与stage12–15才完成§14的真实绑定；不提前信任proposal/family |

**首版全域证明算法。** 在作者源、entity existence、实际范围成员、Query/index和ledger内容读取之前，读取受保护当前认证/授权定位记录及policy，按完整Workspace作用域检查当前principal/delegation具有workspace scope的`workspace_state,entity_state,locator_state,source_read,resource_read,annotation_read,policy_admin`全部allow。对该主体匹配的任一deny，如果其capability与上述能力或任意Field read相关，无论scope是workspace、subtree还是ref_set、其中Ref是否实际存在，都使全域证明失败；不查parent或扫描当前实体来证明deny“碰巧没有命中”。其他主体的deny不计，写入deny继续独立按真实footprint判断。资源/批注的所有owner因此同样受覆盖。此算法刻意保守：有限ref_set、局部subtree、当前完整可见行列表、无命中索引均不能证明任意潜在Ref的资格。policy_admin用于观察可能影响操作的受管配置结果，不因其存在而推导任何作者读权或忽略deny。不存在Document的Ref不要求伪造源或成功source lookup；这里只证明先验授权范围。

**狭窄Field路径。** owner_fields必须先由已认证完整Registry及closed意图构造潜在范围和保持证明，具体采用本稿§15合入的《D7 Narrow Field Qualification》。其资格包含完整Field读取/实际Field写、source_envelope_state，以及提交/receipt所需commit_sequence_state；不由source_read或field_read/write隐含授予。实际作者值只能在先验资格后读取，不能由当前秘密membership决定profile。全源D2/D4 validity、capacity、source revision/CAS和receipt序号差异有明确metadata授权；其余秘密内容仍不可影响公开outcome。无法证明就拒绝或另以完整workspace_constraints资格重新准备；不自动补权。


该静态上界不能靠模型布尔值或adapter声明“安全”成立：实施必须从closed操作和完整Registry constraint algebra生成依赖图，并用实际验证器读取追踪证明所有作者/控制依赖属于上界。某一实施入口未完成这种证明时固定使用全域profile。普通不受其他Field/membership/范围约束影响的phone追加或编辑可以只观察phone并仅写phone；删除最后一个可能required的Field、改Facet或任意不透明源必须采用其更大上界。新增schema约束使原静态证明失效，不能从旧Registry缓存恢复狭窄资格。

**D3唯一映射。** 首版所有ordinary D3 identity/lifecycle模式在stage3要求target的workspace_constraints；涉及在线source Workspace的copy另对source证明同一全域观察资格，离线artifact仍不访问source authority。create_workspace的target用prepared_workspace，fork source用workspace_constraints且保留其完整source/export授权，fork target用prepared_workspace。continue是ordinary，不重新bootstrap。stage3只使用模式/外层Refs及受保护当前policy，不解析隐藏源、lookup target ledger或执行stage14语义来发现范围；原stage6仍空。stage14/15和planning CAS验证实际source/state/negative/incidence/Registry/control读取全部受所保存scope覆盖；不能用晚发现的隐藏依赖先求值再返回不同业务结果，D3 wire11只增加本稿§16规定的显式preparationBinding；scope本身仍由可信Core派生并保存，不能作为未声明request member。

prepared_workspace的临时资格既不证明proposal有效，也不允许任何既有target内容/ledger读取；仍按原stage4、P1/P2、TL、stage5处理。只有这些原门通过，且stage12–15完整证明目标均来自同一proposal的fresh集合与固定bootstrap控制计划时，才可形成提交所用完整proof。不得为stage3提前读取family/profile、采样或物化候选。

**D6入口的授权定位。** 准备时先从closed adapter输入确定静态scope；与planToken同事务保存最小、受保护、不可公开的授权定位记录，exact为`{planToken,workspaceRef,principalAudienceToken,observationScope,registryBinding}`。其中scope/profile及Registry仅供受信授权器选择资格，不含作者值、匹配结果、preview、receipt或完整plan。§2步骤2允许只读取这一定位记录，验证当前audience及固定scope，禁止先读完整PreparedIntent或ledger来选profile。定位记录与规范request/原plan绑定，planned或任何saved decision引用后按该ledger的保存寿命保留，不随preview TTL清理；缺失/不完整统一not_visible，不降级为猜测scope。重放继续使用同一窄profile且重新证明当前Registry下静态上界覆盖原范围；旧schema不能豁免新增约束。没有同权新提交成功而因旧preview过期永远无法授权重放的旁路。定位查询不是plan/ledger读取或存在性对外披露；其完整绑定在后续原步骤再次核对。

此首版代价明确：局部写权或node_create本身不足以调用这些D3修改入口；具有隐藏内容的共享Workspace可能拒绝创建、copy、restore、purge或mixed fresh import，即使“这一次”不会冲突。管理员也必须具备完整内容观察权，不能只凭policy_admin绕过隐藏Field。该代价不增加实际写集，不取消局部D6 Field编辑路径；未来若交付更细D3 profile或显式约束结果披露权限，须另行版本化评估，不在这里隐式解密隐藏事实。

| 阶段/表面 | 观察资格和失败 |
| --- | --- |
| D3 stage3 | 按固定mode profile先证明；失败沿identity_not_visible/preflight，零target ledger/作者范围读取，零新decision |
| D6准备/preview及首次提交授权门 | closed adapter先选潜在上界，失败not_visible/preflight；不能先查询unique/incoming再给preview |
| 已保存decision重放 | 先当前profile资格，再按原协议到saved key；核对saved scope和当前真实效果披露，资格不足不给原成功/业务拒绝receipt或effects |
| planning及author commit | 同一事务比较policy/current delegation及全部业务依赖；固定scope和plan不换；撤权胜出拒绝或暂停，不保存永久业务rejection |
| planned recovery | 用原scope/原plan和当前fence/连续性；观察权暂失保持planned，不burn；重获权后仅恢复原意图 |
| receipt/effects/订阅/导出publication | 当前观察资格与实际内容披露同时通过才交付；与撤权线性化，已交付内容不能追回 |
| 多批/混合导入 | 每个固定batch均按其D3或D6 owner入口执行；作业开始授权不覆盖后续批；已committed前缀不抹除，未获权批不得用success/failure探测隐藏匹配 |

完整性证明`DependencyProof`继续负责准确扫描及负范围/CAS；ObservationScope负责潜在结果可披露，两者都必须成立。scope与RegistryBinding、policy generation、当前principal/delegation、实际source/state/incidence/control依赖一起保存于固定plan。scope不能用来扩大sourceVersions、actual owners或D3 closure。一般schema错误仍按原decoder/diagnostic顺序；授权不足永远不能由业务约束是否命中决定。当前授权缺失导致的公开结果在双状态中均为同一外层not_visible，内部不向调用者泄露原因、进度成员数、已生成IDs或部分成功。对有完整观察权的主体继续真实检查全部unique/基数/incoming，不能过滤隐藏行后假成功。

只读Query和ByteHandle保留其独立入口/完整结果/当前内容授权，不因为本节自动要求policy_admin；它们若被用于Action或requireMembership约束，修改入口另证明相应ObservationScope。D6配置管理虽然已有policy_admin，涉及完整Calendar范围真假时也必须证明workspace_constraints。其他管理预算状态使用既有明确可披露的控制schema，不冒充作者约束查询。

control_only不读取作者事实或以其存在性/值作为成功条件；当前policy_admin仍可读取并显式修改closed policy，即使其没有source_read或存在read deny，从而保留撤权后的受权管理恢复路径。管理员自身被撤销时不能靠此恢复；仍须另一当前获权主体明确修改。Registry/binding修改若依赖作者范围则不属于此profile。issuer管理属于§14独立控制域，按其自身管理权限，不借不存在的Workspace policy授权。内部读取追踪发现任何未覆盖作者条件时在求值前停止，不能以control_only得到隐藏业务判定。

D3 create/fork原保存receipt的重放是明确分域：按原D3当前issuer/source准入、proposal及本节保存的prepared范围验证后返回原bytes，不因target已激活或其权限后来撤销而要求重建BootstrapPlan、重初始化或增加target policy门。原receipt本来公开的有限新identity/结果由受权allocation协议披露；额外target source、control详情、effects值及后续内容读取仍需当前target授权和适用观察范围。不能把通用效果交付规则反向改写原D3 receipt合同。

## 14. Issuer控制授权与Workspace初始控制状态

首次建立存储与创建Workspace不是同一授权。host/D10认证只给主体身份，不自动给issuer控制权限，更不赋予既有Workspace读写权。`IssuerControlPolicy`由现存issuer的独立控制域拥有，exact为`{kind:"d6_issuer_control_policy",wireVersion:1,issuerAuthorityInstanceId,revision,grants,bootstrapProfile}`。revision为Counter；grant exact `{subject,effect,capabilities}`，subject为该issuer控制域的认证principal Token，effect为allow/deny，capabilities为非空、sorted unique的`allocate_workspace|administer_issuer`数组。只作用于这个issuer，不接受Workspace/subtree/ref_set scope。deny优先、默认拒绝；administer_issuer不自动蕴含allocate_workspace，两个能力都不蕴含任何既有source读取或target激活后的能力。

**信任起点与管理。** 本地首次issuer只能在用户明确建立的全新空存储域，由受信host将当前OS登录身份映射为初始issuer principal；Server由已有部署operator的受信配置指定已认证身份，不能由普通HTTP请求自报subject或因首次连接抢占管理员。Core在一次初始化事务内mint issuer AuthorityInstanceId、建立连续性/fence控制、issuer policy revision=1及其认证映射；初始issuer principal被明确授予上述两种可撤销能力。初始配置必须提供完整bootstrapProfile及可信Registry seed。marker/authority/ledger/配置存在、缺损、冲突或旧备份不能再走“空域”初始化；失败保持未初始化，不能覆盖或补造旧grant。恢复、continue、failover、repair都不得触发首次issuer规则。

issuer policy变更是这个issuer控制域自己的显式管理操作：仅当前administer_issuer可准备/执行/重放，绑定完整旧revision及新policy/profile，当前认证/delegation和fence在同一控制事务CAS；实际改变checked-add revision，MAX拒绝。以issuer内的管理operation key保存不可变请求/结果，same-key重放不重复改变，different input conflict；这个控制账本不是D3 Workspace-local OperationId或跨Workspace全局注册表。issuer管理的host/CLI/RPC运输须在该实施入口冻结前提供closed schema，当前D6 Workspace提交API不能冒充此入口；本节已经冻结主体/权限、幂等、CAS和效果的完整语义，不授权自由代码管理回调。撤销最后一个issuer管理员或其allocate能力可以是显式效果，不能靠重新open恢复初始超级管理员；host上重新认证只恢复同一身份，不新增grant。

`bootstrapProfile`当前新签发exact为`{kind:"d6_bootstrap_profile",wireVersion:2,profileRevision,registrySeedBinding,newSeriesMultiplicity,initialPeriodScope}`；profileRevision为Counter，registrySeedBinding为D4 RegistryBinding/1，指向同一受保护、完整且经D4 trust root验证的不可变seed，newSeriesMultiplicity为显式unique或many，initialPeriodScope在本版固定为workspace；都不能缺省或从请求/冲突结果猜测。当前profile的固定policy规则是：新target只为本次经issuer认证的创建主体建立target-authority-local principal映射；不复制source principal Tokens、credentials、delegation或source ACL。profile/2的initialPolicy.version=2，其初始Workspace grant为当前Policy/2明确全部§4非Field capability，以及target Registry所列全部FieldId的field_read/field_write（集合为空时不生成空Field capability）。deny为空。历史profile/1固定使用Policy/1和原非Field闭集，禁止补source_envelope_state/commit_sequence_state；既有family保持原副本，只有显式issuer profile更新才影响新family。这是明确的首份owner policy，不是永久超级管理员。后续按普通Workspace policy修改可完全撤销。新增Registry Field不自动获得新Field grant，仍按当前policy与普通管理修改；source_read的既有完整源规则不变。

**issue/replacement绑定。** D3 A2在任何family lookup前只按当前issuer principal的allocate_workspace判断；它包含issue、replace和该issuer创建/fork目标的准入，故replace不需要先读family才能知道mode。D3 A3/A4仍分别处理issuer不可用/可达完整性，A5–A11及其先后不改。issue成功在原family/proposal/custody事务内固定一份受保护bootstrap profile副本、真实issuer principal/audience映射、该targetAuthority的初始principal映射，以及Registry seed完整输入；profile不变更D3 proposal/request shape或认证framing。已创建family永不重新套用“最新profile”。replacement继承同一family的固定profile和同一已认证人的身份，但为新target建立独立target principal映射并随原规则retire旧target控制映射；不能让一个旧target Token跨authority取得权限。无成功issue不得凭replace创建profile/family。配置更新只影响以后新family，旧family能否操作仍由当前issuer授权和原D3 family/custody gate决定。

**正式请求与唯一bootstrap计划。** D3 stage3先检查当前issuer allocate_workspace；fork还检查source的当前完整观察及source/export授权，stage4/P1/P2/TL/stage5次序不变。此时prepared_workspace只是公开mode与外层角色导出的临时资格，不能要求尚未在stage12生成的WorkspaceBootstrapPlan，也不提前认证family。target尚未激活，不能先读它的policy/exists/ledger来验证当前source_write或policy_admin。原proposal门与完整计划验证通过后，受权bootstrap资格仅允许验证和发布该proposal的有限全fresh target闭包；不授权任何既有Workspace改写。普通body/Facet/关系所需的source与域约束仍须在原stage14/15完整验证，不能以初始化为由接受invalid source。

`WorkspaceBootstrapPlan`是D3原plan内D6拥有的受保护控制部分，exact语义成员为`kind,wireVersion,operationId,proposalId,issuerAuthorityInstanceId,targetWorkspaceRef,targetAuthorityInstanceId,profile,creatorBinding,targetRegistry,initialPolicy,initialSeriesConfigurations,periodScopeBindings`，kind固定`d6_workspace_bootstrap_plan`、version=1。creatorBinding恰为`{issuerPrincipal,targetPrincipal,principalAudienceToken}`，只有受信认证映射/原proposal可提供；它不是新credential或客户端principal声明。profile为上述immutable完整副本；targetRegistry exact为`{snapshot,binding}`，分别是完整D4验证后的RegistrySnapshot/1及RegistryBinding/1；initialPolicy是§4完整closed policy，revision=1；初始auth generation=1。此计划不是额外可编辑请求或第二decision，不出现在D3 closed wire。它的每项都由已绑定family profile、原proposal/source cut、stage12真实candidate map与原formal request确定派生；同fingerprint不得因ambient配置、重试或新principal产生另一份控制结果。planning同时保存完整派生结果/输入与原request/源plan/pins和reservation，提交只发布保存内容，不能靠未绑定companion补grant。

**Registry和Calendar初始状态。** create target Registry从固定seed经过D4一次性registry_bootstrap建立，Workspace/user owner与target authority正确绑定；fork使用完整source Registry输入和同一trust-root规则为新target重新认证/绑定，source和target Binding分别保存，不能把source context直接当target。无法证明定义可移植、owner binding、规则或完整解释时原操作在planning前按适用D3/D4门拒绝，不能偷偷安装网络贡献或改作者FieldId。

initialSeriesConfigurations每项exact为`{seriesScope,multiplicity,revision}`，seriesScope逐字复用§8的完整closed对象，multiplicity为unique/many，revision=1；按seriesScope canonical bytes排序、唯一。series和periodKey来自完整prepared作者Entry，scope则是独立控制输入，不能声称calendar/period Entry中本来含有scope。periodScopeBindings项exact为`{nodeRef,scope,revision}`，revision=1，scope复用D4闭集；按NodeRef排序、每个有效period Node恰一项。它只保存D4范围选择这一必要控制事实，不复制series/periodKey/Field值、不形成第二作者源或用户可写的新Field。普通已激活Workspace的period创建/修改按§8.1确定选择规则在同一源plan中绑定显式scope，持久保存该控制选择；删除period或purge其Node同时移除该Node的binding，Trash保持，scope改变按当前Workspace policy_admin授权且改变配置/范围控制版本。普通Field修改不隐式改变scope。scope自身若为Node，必须在同Workspace指向完整已证明live/trashed Node；D6控制inbound检查阻止把仍被scope binding/config使用的Node直接purge，必须先按§8.1的closed scope变更与空配置删除意图解除依赖，不能在purge中猜重绑定。

create_workspace为所有prepared period按该family的initialPeriodScope显式绑定target workspace scope，并为每个不同series建立配置，采用已固定newSeriesMultiplicity；不是普通create缺配置时的隐式默认，也不冒充Node scope初始化接口。fork把source cut中当前SeriesScopeConfigurations及periodScopeBindings完整带入，保留unique/many；Node主体与node scope都按完整D3 Node map重绑定，Workspace scope换成target WorkspaceId，再按target Registry/policy验证。不得把source unique改成profile many。任一配置或binding不能完整解释/重绑定时整fork在planning前失败，禁止省略、保留foreign scope或靠第二次管理提交补洞。所有prepared period候选按其准确binding恰受一个配置覆盖，不允许漏配或从是否冲突选择multiplicity。

target尚无作者事实，但不能以数据库空表证明最终范围为空：D4唯一性校验读取该D3 proposal的完整prepared结果集合和全部scope，观察/当前range版本来自原target负inventory/custody及allocation依赖，全部同cut。unique按每个完整period key分别比较，many允许多个Nodes；与普通已激活Workspace相同。初始配置与policy/Registry、全部源、authority activation、ledger/receipt、custody移交及失效版本在同一个author commit原子发布。初始源revision仍为1；policy/config初始化不另增源版本。没有“先激活再补管理员/配置”的可观察窗口。

**普通管理、重放与恢复。** §4旧policy_admin修改以及§8配置意图只作用于已经激活的Workspace；不能在未激活target预先运行D6 management decision。本节bootstrap是一次D3 create/fork的确定控制初始化分支，激活后不再适用。claim/replace race、recorded rejection、planning、terminal failure、burn、custody和D3原receipt保持原规则；失败不产生半policy或可登录的半target。source或target Registry/profile派生证据不能恢复时保持原availability/恢复状态，不重选“最新seed”。已committed的原create/fork重放只按D3当前issuer/source授权及原proposal返回保存bytes，不再次执行policy/config初始化，更不覆盖用户后改policy；控制详情/target内容交付另按当前target权限，失去target权限不能借旧bootstrap结果读内容。continue/failover保留当前policy、principal映射和全部配置及其版本；换holder不恢复creator权限。fork的新grant不会授予对source的任何能力。

profile创建/更新时必须证明Registry seed是D4允许的无退役/迁移历史的完整bootstrap-eligible种子，且target owner重绑定后仍须真实trust-root认证，不能只换WorkspaceId。fork不是把source Registry重新当无历史seed bootstrap：保留source累计语义历史、退役/迁移约束和必需RegistryEvolutionProof，重新认证其target Workspace/user owner与target Binding；定义/历史/证明不可移植或不可验证时在原planning前拒绝，绝不清空历史来通过bootstrap。

## 15. 窄Field资格与公开metadata（完整合入）

## 15.1. 显式元数据权限

D6 Policy/2在原全部capability之外增加两个closed无参数capability：`{kind:"source_envelope_state"}`和`{kind:"commit_sequence_state"}`。它们不被source_read、field_read、field_write或policy_admin蕴含，也不蕴含任何这些能力。source_envelope_state只配workspace或ref_set scope，与entity_state一样先按请求完整Ref和当前policy验证，不用subtree/先查parent；commit_sequence_state只配workspace scope。deny覆盖allow；缺显式allow不合格。既有Policy/1不会自动升级或补grant；正常受权policy修改才可安装Policy/2。bootstrap仍按冻结的完整profile版本建立policy，不得回放时偷偷扩大旧grant。

source_envelope_state明确授权观察该完整源的revision及其变化、source字节长度、物理行/深度/数量容量边界、整个D2结构/容器是否有效、当前Registry下整个源的D4验证是否可用/有效，以及这些有限状态对源编辑成功/失败和CAS的影响。它不允许返回body、title、其它Fields、Facet名称、具体隐藏错误位置、值、ref或约束命中成员；公开精确schema如下。这个权限有意允许观察同owner源是否被其它编辑改变，不能宣传为隐藏源活动完全不可观察。

commitSequence明确为同一Workspace下D3和D6成功author commit共享的非负Counter：fresh Workspace初始化0，每个最终D3/D6 committed decision（包括控制意图与保存raw no-op成功decision）checked+1；与receipt/companion、outbox和源指针同事务写入。规划、准备、拒绝、重放、恢复尝试、物理fence不增加；同一OperationId至多增加一次。已有MAX时不新提交，按原预算/资源失败与恢复边界处理，不能回绕。它不同于contentSequence：后者只作内部范围完整性证明，不向普通receipt附带。commit_sequence_state显式授权整个Workspace的这个活动序号；不授权读取其它decision内容。具此资格的phone编辑者可观察别人的提交数量，因此对这种元数据作授权后比较，不把它伪装成严格秘密。

`d7_source_envelope_read` exact `{wireVersion:1,kind:"d7_source_envelope_read",workspaceRef,ownerNodeRef}`，先workspace/entity_state及source_envelope_state，后当前source存在、revision及验证。成功exact `{wireVersion:1,kind:"d7_source_envelope",ownerNodeRef,sourceRevision,sourceBytes,maximumPhysicalLineBytes,maximumDepth,entryCount,validation}`，数量均Counter，validation=`valid|invalid|unavailable`。上述完整member set只用于valid；invalid/unavailable variant禁止maximumDepth/entryCount（因为完整结构或typed计数未被证明），其它成员保持。只公开聚合状态，不公开invalid的原因，不以0冒充未解码计数。实际I/O不可证明用source_unavailable错误；不存在/不live按已获state资格的既定拒绝。此读取仍不授予Query任意隐藏Field，详细diagnostic只走原repair/audit权限。

## 15.2. 从真实Registry构造潜在范围

输入只允许显式单owner的Field append/replace/remove/set_field_member或同型bulk分解；native、Facet、identity、Query membership、Calendar、relation/inbound/foreign-binding及任意raw变换不能利用此证明。先只读已认证的完整Registry定义图，不读取owner作者值或当前membership来决定资格。图枚举所有Field/Facet/alias及constraint constructor；未知、不可用、未覆盖新constructor直接不可证明。scope不是调用者传入的qualified布尔值。

对触及Field F计算如下闭包，所有条件来自已验证schema及closed意图形状：

1. F及其完整alias、qualifier、unit/code/external-scheme等贡献依赖进入范围。若任意可达值/qualifier/provenance可能引入typed Ref/Locator或relation，且当前adapter不能从意图证明这些成员逐字保持，则本版窄证明拒绝；需要更大真实范围。潜在输入provenance仍按D4完整验证，不能删掉后继续。
2. Field cardinality与at_most_one_preferred仅影响F，故读取F完整Entry集合。mutually_exclusive_members与measurement_unit_dimension读取该Field完整value及相关贡献，不扩大到其它作者Fields。
3. 所有可能含F的Facet constraints必须逐个检查。required_field在存在合法旧Entry的replace/set_member下由条目数量不变证明保持；append由数量不减保持；remove不能从未读membership猜required是否适用，本版窄路径拒绝。union_variant_equal：若closed mutation不能证明F原root union variant保持，则加入另一Field的完整数据且仍不能隐藏实际Facet适用性；本版最小路径保守拒绝此类可能变更。不得因当前owner碰巧未声明该Facet而跳过潜在约束。
4. Classification及其它Field必须逐字不变，源变换仅在已验证F的精确Entry区间；任何namespace/trivia/body/header/其它Field差异使证明失败。已有源的完整D2/D4有效性由source_envelope_state授权的真实验证状态决定，不由请求或cache自报。有效前像+保持不变量的局部变换，对未变Fields保持其所有原验证结果；涉及重新判断不可证明的跨Field/分类条件时不能走此路径。
5. 计划保存潜在Field上界、完整Registry定义路径与逐constraint保持证明、源Envelope状态及版本、F完整raw前像/拟议Entry、实际全源byte patch、所有当前授权与依赖。内部读取分成用于原字节保留的完整源、已授权Envelope聚合验证、以及进入业务结果的F事实；追踪必须能验证业务/公开效果从不读取上界外作者值。未知读取类别在求值前停止。

通过上述图证明后，才检查当前principal对F及闭包全部Fields的field_read、实际修改的field_write、对应源owner的完整entity_state和source_envelope_state（不是给所含作者Ref目标增授状态权）；需要提交/receipt的Action还须workspace commit_sequence_state。不能证明则在作者读取前not_visible；不得试运行后看这次结果是否安全。source_write deny仍阻止实际修改。结果可见性与原source版本/CAS始终另验；不因为完整源被内部读取而输出整份source。

## 15.3. 正向phone构造

使用实际D4 catalog的people/phone，text成员在其已展开closed object内修改。选择两个完整合法、同值但不同occurrenceKey的phone Entries之一；set_field_member只改required text路径，保持同key、qualifiers、note、provenance和全部其它members。Registry全图证明它不是relation，不改变root variant和任何Facet membership，Field计数不变；at_most_one_preferred输入保持，requiredness保持，text的nonEmpty和code域仍按D4验证。本构造同时涵盖省略provenance、合法同/跨Workspace Node provenance、历史Locator和owner-local Resource provenance；资格不由当前值是否为空决定。Value§5.1的统一作者原值规则允许在完整Field读取权下逐字返回这些原作者成员，不查询其目标Workspace/state/content。closed text变换证明全部Ref/Locator/provenance保持，因此这些目标状态不是本窄操作的依赖，也不要求对任意foreign Workspace作静态授权。原D4仍完整检查形状、Locator owner和Resource locality；它明确允许普通foreign NodeRef，不得宣称全被拒绝。新建/改写Ref、依赖目标状态或relation不属本局部text证明；需要其真实更大范围。点击仍走独立当前resolver/content门。

主体显式获得：workspace的workspace_state/entity_state/locator_state、owner的source_envelope_state、workspace scope的people/phone field_read/field_write（原D6 Field capability不允许ref_set；本例不偷偷增加该scope），以及workspace commit_sequence_state；没有source_read/body读取、其它Field读取或policy_admin。read/选择返回两个完整phone selectors；第二个selector的revision和raw匹配后prepare，preview只包含该Field两条Entry的完整before/after及owner SourceVersion；提交经原D6 CAS恰修改第二条，第一条、body和隐藏Field逐字保持；effects与原receipt可完整审阅。此构造必须由实际source/Registry/权限/提交模型执行，不能仅把本段expected写入报告当pass。

## 15.4. 全公开outcome比较

两有效世界在输入、F事实、已授权Envelope元数据、明确获权commitSequence及当前policy/Registry/authority上相同，仅隐藏body/其它Fields内容不同：读取、选择、preview、成功/no-op/拒绝、receipt.sourceVersions、effects、replay结果必须逐字相同（opaque token随机性按同一随机币耦合）。source前像和后像秘密区域保持各自原文，但不交付给此主体。

若两世界只因隐藏源变化导致sourceRevision、长度、容量、验证状态或Workspace commitSequence不同，这些差异由上述显式元数据capability预先授权；其公开影响可不同，必须在证据中逐项指出原因。不能新增未列的隐藏success oracle。D2容器接近capacity、CRLF/Unicode字节差、其它Field并发引发source CAS、新Registry约束、source_validity改变、撤权、source A→B→A与lost receipt重放都必须覆盖。去掉source_envelope_state或commit_sequence_state的负例在读取作者源/ledger前统一not_visible；不能为了让窄路径成功而自动授予。

这是明确减少内容读取面的权限配置，保留必要的源元数据与提交活动观察成本。它不声称不同物理I/O/OS故障/墙钟耗时不可区分；这些已在D6范围外，但不能用该限制豁免本文件列出的业务输出、计数、CAS与preview/effects。

## 16. D7准备、效果与提交序号的唯一消费

D6 PreparedIntent额外承载完整PreparedActionBinding/2（历史/1仅按原规则恢复/重放）及其原D6 request，原planToken不可变；D3 wire11的preparationBinding由原D3 stages处理，D6不创建另一ledger。按D7《Prepared Action Binding》的完整同包正文消费；其所有input pins、pre/post Query dependencies、受权scope和preview在准备返回前固定，planning/commit的原CAS逐项比较。记录被原decision引用后不受preview TTL回收。当前权限不足遮蔽重放，不改变decision。

effectsToken继续指同decision完整效果。D7《Preview and Effects Transport》是d7_effects_resolve/open/page及effect_bytes/1唯一完整closed运输规范；D6原resource_bytes/1不因此支持拟议或fresh bytes。D3/D6原receipt shape不变；D3 companion保存commitSequence/effectsToken及完整D4/D7扩展，与原receipt/源/outbox原子提交。preview包含全部symbolic source/entity/control计划和含C的existing源条件项；按D7效果合同在同一原candidate map下唯一精化no-op/实际修改与revision，不能只列旧source项或采样一个map。owner_fields只输出可证明覆盖全部实际修改的完整Field前后像，原内部完整源证据不丢失。

commitSequence是本Workspace所有最终D3/D6 committed decision共享的Counter，包含raw no-op和控制意图成功；fresh Workspace初始化0后每个成功checked+1，重放、准备、拒绝、恢复attempt不增。旧Workspace切换时必须从连续完整ledger/companions和原持久counter核验已有最大已签发序号，取其最大为新counter基线并与版本切换原子保存；缺任何连续性证明不得初始化为0或重用序号。旧saved receipt bytes从不改写，旧planned最终提交只增一次；MAX无新成功提交。原contentSequence仍内部，不混为同一counter。

Policy/1请求按历史能力和原scope消费，不自动获新窄路径；新的D7 owner_fields操作必须采用Policy/2。独立D3完整workspace/prepared观察的原receipt资格保持，新增commit_sequence_state不是反向修改原D3历史receipt授权；新增D7/D6窄profile显式要求它。控制管理员只读管理路径不因新metadata能力丢失原control_only资格。
