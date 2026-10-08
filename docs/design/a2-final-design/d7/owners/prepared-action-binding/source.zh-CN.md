---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：5a533466-be80-4de0-987e-2db0086fda87。

候选状态：D7 文件权威协调修复候选，未独立接受、未激活、未实施。历史阶段标签与有界证据仅保留原范围。本篇现已消费 PL-IR-01 的稳定生产地址/current Observation 作者修复及当前 D3/D6 producer，但该新 exact candidate 仍待独立复核，不自行关闭 P1，也不意味着全包可激活。

# D7 准备证据与原事务绑定


本规范消费同包D3 wire12与D6 Control当前生产合同。当前记录的完整 /3 形状及来源类型见§5；§1的 /2 字段基线完整保留，/3 只作那里明确的扩展和类型升级。D3的identity/stages/ledger owner和D6唯一author commit继续成立；准备记录是不可变输入证据，没有committed/rejected结果状态，不占用OperationId ledger，不保留另一份作者权威。

## 1. 一份不可变准备记录（历史 /2 形状及当前 /3 扩展）

Core受管`PreparedActionBinding/2` exact semantic members为`kind,version,bindingToken,protocolOwner,operationId,workspaceRef,principalAudienceToken,action,canonicalCallInputs,definitionInputs,registryInputs,ruleInputs,sourceInputs,constructionInput,proposedInputs,dependencyProof,observationProof,budgetBinding,expiresAt,request,preview`。kind=`d7_prepared_action_binding`，version=2，bindingToken为独立tag=d7_preparation的D6 Token。其余语义类型如下，不能使用自由dictionary来遗漏证据：

- action为原完整ActionSpec；canonicalCallInputs是按实际调用路径排序的完整QueryCall数组，包含有效arguments/context，普通无Query动作为空；definitionInputs逐项是`{definition:DefinitionAddress,ownerVersion:SourceVersion,payload:text}`，保存实际解析wrapper/Query/View的完整source payload，不由调用方自报。
- registryInputs为原完整D4 RegistrySnapshot及RegistryBinding、完整展开所用定义；ruleInputs为原完整D4 RecurrenceReadContext/Binding及实际使用的其它已closed规则贡献。没有使用时对应空数组，不填假context。
- sourceInputs是原D3/D6源pin目录，每项含完整subject、SourceVersion、exact source/bytes binding及actual role；proposedInputs是同原D3 Result/9或D6完整源变换的已验证符号/具体输入。完整类型逐字引用所属D3/D6 closed类型，不重新定义Ref/Locator或允许任意JSON。
- constructionInput 是 null 或 D9 Templates §6a 的完整 TemplateConstructionInput/1。只有Core内置D9 node-template adapter可建立非null值；公共ActionSpec/d7_action_prepare不接受额外construction字段。非null仅用于D9指定的ordinary-import d3_operation或简单collection_create；完整输入pins、版本、参数、loss、sourceSubjectBindings独立重编译后须等于request及proposedInputs。sourceInputs原类型不变，不塞新的role或私有artifact variant。
- dependencyProof为实际完整正/负依赖，包括全部pre/post Query、定义解析、Registry/rules、authority/state/placement/ForeignBinding、授权与源Envelope元数据；observationProof为同包D6选定scope与当前潜在范围资格、逐constraint保持证明。原D6 budgetBinding包含已耗账户，不能在replay重新初始化。
- request为准备完成后返回的完整原D3/D6 request bytes；preview为效果规范的完整不可变语义清单、原字节pins及初始交付投影，包含适用conditional_source_change；不能把一个采样map当作确定后像，尚无identity reservation。交付handle重签发不改变语义清单、原request或此准备记录。expiresAt遵守D6 clock epoch/期限域，不从设备时间猜。

记录在返回任何prepared响应/token前原子保存并验证所有引用的pins齐全。bindingToken生成与request构造只有以下次序：先生成随机token（无作者/identity effect），构造包含该token的最终request，再保存完整记录及最小授权定位映射，最后返回；不把“request包含token、token需hash request”变成循环散列。完整相等用原canonical request与原规范对象/byte pins，不能以一个相同hash替代证据。

最小定位映射/2 exact {bindingToken,protocolOwner,operationId,workspaceRef,principalAudienceToken,observationScope,registryBinding,constructionReadRefs}，受保护且无作者值。constructionReadRefs是按D3 RefKey排序唯一的EntityRef数组，非模板为空；非null construction恰覆盖其inputPins、omittedAnnotations及构造所需的其他完整来源读取Ref，不含source bytes。Core记录版本选择/1或/2 decoder，不按字段缺失猜版本。D3 stage3与D6原授权步骤先读此映射，证明current audience、固定scope及这些来源的完整当前读取资格，再读完整record/ledger；missing/wrong-tag/wrong-audience或资格不足一律not_visible（D3映identity_not_visible），不泄露记录存在。此增加的是D9实际输入的读取检查，不授新权限、不改变原阶段先后。

## 2. D3显式wire12承接

D3 wire12顶层继续保留从wire11引入的唯一可选`preparationBinding`，其exact值为`{kind:"d7_preparation_binding",bindingToken}`。此成员进入原requestFingerprint；绑定的规范原request包括它，顶层OperationId仍按原规则不进入fingerprint。D7发出的D3准备请求必须包含它；独立raw D3请求可以省略，但只能表示自身完整D3意图，不取得D7 postcondition、preview或保存策略证明。

stage1验证closed形状、Token词法与known kind；stage2验证原Workspace roles；stage3在原D3全域/issuer/source授权之外，按最小定位映射证明准备记录current audience与相同固定scope，原P1/P2/TL顺序不变。stage5只比较已包含preparationBinding的原fingerprint：同OperationId同D3 payload而不同D7条件必然使用不同bindingToken，不能混用同一saved request；不同fingerprint仍按原stage5冲突，不读取/回显原条件。

unseen请求继续原stages，stage14在原identity_map_incomplete检查之后增加唯一family `operation_precondition_failed`：已授权的绑定记录过期、其完整request与所收请求不相等、原D7 pre/post及非null construction完整proof与当前cut依赖不成立、原D7完整postcondition不满足，均recorded_rejection。不能证明pin/authority/连续性属于availability，按原availability路径不伪造确定业务拒绝；原stage15 inbound closure仍随后完整执行。未知/丢失授权定位记录已在stage3遮蔽，完整记录损坏在连续性/完整性检查停止，不能仅因记录丢失生成一个新计划。

同一planning CAS比较原ledger unseen/family/current authority及绑定记录和完整依赖，保存原request、PreparedActionBinding完整内容、pins、scope、实际D3计划与reservations。已有planned恢复只使用这份保存内容，不重跑漂移后的Query或重新选择targets。已有saved decision的stage5重放不检查旧preview/preparation TTL，但仍先当前原授权和scope；返回原receipt/error bytes。撤权不写永久rejection；重获权仅恢复原decision。

prepare不会领取或烧毁最终content IDs；fresh内容保持D3 symbolic subjects，到原stage12临时candidate及原planning reservation才按已有D3规则产生真实映射。原D7 post-query的proposed evaluation在同一已绑定symbolic candidate namespace进行；fresh equality/NodeRef输入只能由Core typed替代环境提供，不把symbolicToken当公开NodeRef。stage14以最终candidate map重新验证同一语义与完整结果，不能把用伪UUID运行的一次样本当证明。

## 3. D6承接与寿命

D6 request继续通过原planToken唯一选择PreparedIntent；该PreparedIntent完整嵌入PreparedActionBinding及对应D6 request，不增加D3模式或第二commit入口。planToken与bindingToken是不同tag，两者一次绑定且不可互换。相同planToken永不指向另一个action、post-query、输入或targets。D6原步骤6/7逐项CAS和保存这份完整D7证据，D6错误/disposition顺序保持。

未被任何decision引用的准备记录在有限TTL后不再允许新提交，可以释放大pins；保留最小token/tag/audience/scope及expired标志至受管token失效保留期，以区分已知本人过期与未知。任何planned或terminal saved decision引用后的最小授权映射和完整record/pins按该原ledger保存与恢复规则保留；不受preview TTL清理。ledger恢复所需pin不能删；preview丢失仅preview不可用，不改commit事实。若连续性不可证明，保持原planned或遮蔽交付，不重新prepare代替。

显式重新prepare产生新tokens与新OperationId；d3_operation已给OperationId时不改其值，但不会在同一ID下修改现存不可变记录。未提交的多个独立records不占原ledger key；最终只有原ledger CAS winner能形成decision。没有“准备成功就保证最终提交”的预留授权。调用方改动返回request后，它成为另一canonical输入，原证据不再匹配，且不能自动使用旧preview确认新请求。

lost receipt只能重发原request；相同saved decision在当前资格通过后返回原bytes，不再次执行Query/字段修改或增加commitSequence。未知是否提交时不能以新OperationId重做。完整post Query（包括sort/take、scalar、QueryRef、所有正负依赖）在准备与原planning/commit证据中都固定；显示200条preview不减小条件范围。

## 4. 版本及历史

历史 /2 引入constructionInput与最小定位映射/2；当前新准备一律为§5的PreparedActionBinding/3与最小定位映射/3。非模板constructionInput=null/空refs，非冲突resolutionInput/resolutionAccess=null；不从作者字符串猜adapter。已有/1 records及其planned/saved decisions继续其原decoder、原最小映射和byte-equal恢复/重放，不填字段、不换token或重新解释历史授权。当前新模板准备使用/3，消费本包已经落盘的 D9 /2 typed construction；完整组合仍须独立接受。切换后尚未形成decision的旧/1或/2准备不能建立新author effects：原授权门后，D3 unseen在stage14按operation_precondition_failed、D6在原步骤6按semantic_rejected记录准备失效，用户须明确重新prepare。已planned/saved在原ledger门先恢复/重放，不被此unseen版本检查回溯拒绝。D3 wire12、preparationBinding对象形状及D6提交wire均不变：新不可变bindingToken选择/3记录并进入原fingerprint。无法加载对应版本或完整pins时按原availability/完整性门停止，不能按空construction处理。


真实D3 wire11、wire10与wire9已保存decision仅以各自原decoder、原fingerprint、原门和原byte receipts重放/恢复；禁止新wire11/10/9 decision。不能把旧request改成12、补token或按新语义重新解释保存的旧结果。新wire12的request/receipt/error及Result/9需完整新conformance，历史v10测试不证明这些新增合同。D6 Policy/3及新metadata资格只作用实际采纳新profile的请求/准备，不悄改原D3 create/fork已明确授权的历史receipt重放范围。
## 5. 当前 PreparedActionBinding/3 的完整类型

当前新准备只由本节 /3 decoder 接收，不能把 §1 的历史简写当成新输入。下面所有成员必需；除 constructionInput/resolutionInput/resolutionAccess 及最小映射 registryBinding 明示的 null 外，不接受额外、重复、缺失或非法 null。Token、Counter、Ref、原 ActionSpec/QueryCall 和全部 D3/D4/D6 导入类型各按其真实 closed decoder，不接受同名自由对象。

~~~text
{kind:"d7_prepared_action_binding",version:3,bindingToken:Token,
 protocolOwner:"D3"|"D6",operationId:UUIDv4,workspaceRef:WorkspaceRef,
 principalAudienceToken:Token,action:ActionSpec,canonicalCallInputs:[QueryCall...],
 definitionInputs:[D7DefinitionInput/2...],registryInputs:[ValidatedCatalogContext...],
 ruleInputs:[RecurrenceReadContext...],sourceInputs:<InputDescriptor/2.sourceInputs>,
 constructionInput:null|TemplateConstructionInput/2,
 proposedInputs:[D7ProposedInput/2...],dependencyProof:DependencyProof/2,
 observationProof:<PreparedIntent/2.observationProof>,budgetBinding:BudgetBinding/1,
 expiresAt:<D6 protected deadline>,request:<D3 wire12|d6_commit_request/2>,
 preview:<complete immutable EffectManifest/2 semantics,pins,initial delivery>,
 resolutionInput:null|D3ResolutionInput/1}
D7DefinitionInput/2 =
{definition:DefinitionAddress,ownerVersion:SourceVersion/2,payload:text}
D7ProposedInput/2 =
{subject:PayloadSubjectKey,payloadKind:"exact_source_document"|"resource_bytes"|"annotation_value",
 encoding:"exact_source_utf8"|"resource_bytes"|"d3_annotation_value3"|"d3_symbolic_result9",
 pin:PinRef/2}
MinimumMapping/3 =
{bindingToken:Token,protocolOwner:"D3"|"D6",operationId:UUIDv4,
 workspaceRef:WorkspaceRef,principalAudienceToken:Token,
 observationScope:ObservationScope/2,registryBinding:null|RegistryBinding/1,
 constructionReadRefs:[EntityRef...],resolutionAccess:null|D7ResolutionAccess/1}
D7ResolutionAccess/1 =
{workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
 conflictId:ConflictId,readRefs:[EntityRef...]}
~~~

request 的完整 DecisionKey/2 唯一确定此 record 的 Workspace、operationId、decision owner 与 CommitDomain；preview header 的 decisionKey 与之逐项相等，protocolOwner 不是 OwnerInputBinding 的 descriptor owner。budgetBinding 使用 D6 真实 BudgetBinding/1 的真实成员与数值域，expiresAt 使用同 PreparedIntent 的可信 deadline/clock epoch；预算已耗值及 PinBudget/1 按原 D6 记录保存，重放不清零。preview 是本包《Preview and Effects Transport》定义的完整 protected preview 记录，绝非一个客户端传来的 header 或任意 dictionary；所引用的 semantic manifest、全部 slots/pins 和初始交付投影都按该 owner 完整解码，并与本 request/record 相等。

sourceInputs 逐项等于最终 request 所绑定的 InputDescriptor/2.sourceInputs；每项真实为 `{entityRef,observation:SourceObservation/1,role:"before"|"dependency"}`，完整 role、排序、唯一性与跨字段规则逐字服从 D6。当前 observerDomain 等于请求 CommitDomain；观察中 sourceVersion 保留真正生产域/生产世代，不能由当前观察重签改写。当前任何 SourceVersionRef 只能解析到这份完整观察，未签 caller Ref/revision 不补成证据。sourceInputs 不是 preview bytes、历史冲突分支或 D9 artifact 的容器。

persistent Locator 可以在本次 preparation 之前刚通过新读取取得资格，但本 record 必须把由此得到的**当前** Observation 保存到 sourceInputs。之后任何再次 requalification 或观察变化只能作为另一个新 preparation 的证据，绝不能换入这个不可变 /3 record、request、preview 或 saved/planned recovery。

definitionInputs 按 DefinitionAddress 的 D3-CJ/3 UTF8 bytes 唯一排序，恰覆盖实际读取的全部 wrapper/Query/View；同 owner 的完整当前观察已在 sourceInputs，ownerVersion 与观察中生产 SourceVersion/2 逐字相等，payload 是其中真实完整 payload，不凭定义 label 取最新。registryInputs 是实际使用的不可变 D4 ValidatedCatalogContext 数组，包含其完整 Registry/绑定闭包；ruleInputs 恰为实际递归展开使用的 D4 RecurrenceReadContext（包括真实 Binding/规则贡献），各按 D4 owner 的完整 canonical key 唯一排序。当前语法未定义的其它规则上下文不能被自由 JSON 添加；实际未使用才为空。完整 QueryCall 继续按真实调用路径顺序保存，不能只留顶层调用或去掉负依赖。

proposedInputs 按 D3 subjectCanonicalKey 唯一排序，恰覆盖真实完整拟议源（含条件源），不含 artifact subject。D6 分支只允许 existing Entity subjects 与实体对应 concrete encoding；D3 可有同一次 native plan 的 existing/mapped/new，exact payloadKind 与实体 kind 对应。concrete pin 的 payloadKind 等于该 source kind，完整 bytes 用该实体 owner decoder；只有完整 Result/9 使用 encoding=d3_symbolic_result9、pin.payloadKind=effect_bytes，其 pin bytes 必须完整通过 D3 Result/9 decoder 且等于原 result payload binding，不能把内部符号当 exact source。没有 C/Q/typed slot 的原结果按真实 concrete bytes 绑定。D3 canonical 分量的 existing source 另与完整受保护 canonical plan 的最终 source 相等；同实体与 native result 重叠只有 D3 §10.1.1.3 的 complete before/final/version/typed-evidence 精确相等才可去重为唯一 proposed input，否则 stage14 identity_map_incomplete；不允许双份按序覆盖。每项 pin 都是 Core 实际受管对象，digest 相同不代替来源、用途、完整 decoder 或原 payload binding。所有使用的 pins 都进入对应 OwnerInputBinding.pinRefs 和原 decision 的恢复保存集合。

非 null resolutionInput 只由 D3ConflictResolutionPrepare/1 生产，protocolOwner=D3，action.intent.kind=d3_operation，constructionInput=null，canonicalCallInputs=[]、definitionInputs=[]。action 的完整 native request、record.request 和返回 request 逐字相等；这里只附加原 bindingToken，没有新的 ActionSpec arm、ownerKind 或提交入口。D3ResolutionInput/1、D3CanonicalInput/1、D3CanonicalEffectPlan/1、D3ResolutionInputUse/1 由 D3 主文 §10.1 唯一定义；D6 ConflictInstallInput/1 由其 Control §3.1.1 生产。它们在 resolutionInput 中单独保存，不冒充 sourceInputs 的 current Observation；普通 Query、D8、D7 非 resolution action 及 raw wire12 都不能消费此安装资格。source/conflict/完整当前范围的 DependencyProof/2 仍全部需要，只在具名 guard 上下文以 wrapper 证明该 source entry。ordinary action 的 resolutionInput/resolutionAccess 必须 null；不是根据作者字符串或复制 pins 推断 resolution。

非 null constructionInput 只由实际 D9 node-template adapter 生产，resolutionInput=null；其完整当前类型 TemplateConstructionInput/2 由同包 D9 Templates §6a 实际正文拥有，输入版本/观察/pin 与目录级 omittedAnnotations 按该闭集逐项验证。真实历史 TemplateConstructionInput/1 依旧原 decoder/恢复；完整 D9 consumer 组合未独立接受时只门控新模板激活，不用未定义对象替代，也不阻断普通 D7 action。D9 portable recipe 不因本记录存在取得当前 Observation：其所选真实来源须按 D9 当前 producer 显式资格映射，再进入本 current sourceInputs；模板 text、路径/hash、旧 Counter 或 artifact 不能自动补签当前 SourceVersionRef。

最小映射的 constructionReadRefs 对非模板为空；模板的源bytes读取须完整source披露，D9 omittedAnnotations 仅按其实际 D9EntityVersionAddress/2 与同cut目录/state用途作最小披露，不因列在 omission 目录而强制读取 Annotation body；resolutionAccess 对 resolution 恰是完整潜在 subject/source/owner/structure 披露集合，readRefs 按完整 RefKey 唯一排序、无 branch values。resolution 用完整 workspace_constraints scope；普通 record 为 null，不能把任意 action 升成冲突入口。mapping 中 registryBinding 与实际 record 的 D4 绑定一致；registryBinding=null 当且仅当该当前动作真实不读取/验证任何 Registry 事实且 record.registryInputs=[]；否则必为与全部实际 contexts 相等的完整 RegistryBinding/1（expectedRegistryGeneration 与 expectedSnapshotDigest），没有“空RegistryBinding”arm，不能伪造 active Registry。此 null 只属于 current MinimumMapping/3，真实旧 /1,/2 decoder 不改。原子保存顺序为随机 bindingToken→含 token 的完整 request→record/mapping/guard/所有 pins→prepared，避免循环 fingerprint；尚无最终 fresh identity 或 OperationId reservation。

## 6. 当前 owner descriptor、完整预览与恢复联动

D6 分支的 native source mutation 由 D7 Execution 的 d7_action/2 owner descriptor 完整绑定 ActionSpec、调用、定义/Registry/rules 和 D7ProposedInput/2；外层 InputDescriptor/2 带完整来源/控制/观察范围/Frontier，PreparedIntent/2 带全部依赖与安装计划。D3 分支保持 d3_identity_operation/12，既有 native intent、完整 payload evidence 与原 D7 record/guard 分工，不要求 D6 planToken。D6 planToken 仍 tag=d6_plan/2，D7 bindingToken 仍 tag=d7_preparation；前者只能选择一个不可变 PreparedIntent，后者只能选择本完整 record，不能互换。

D3 resolution 的阶段顺序依其真正 owner：stage3 最小 audience/conflict_read/披露和用途关联，stage5 原 fingerprint 的 saved/planned/unseen，只有 unseen 才加载/重验当前选择、写权、期限与完整 branch proof；stage14 在 identity_map_incomplete 后最终复验 exact key/head/record/request/preview/所有后置条件，stage15 完整 inbound。input guard 缺失或转移不能将 historical copy/canonical pins 变成 raw 资格；同 key 改请求仍按原 stage5 冲突，不先做当前 key 检查。一个 native planning CAS 保存全部原输入、guard、/1 fresh 与 conflict-only /2 canonical source plans、candidate 和责任；同 P seal 保存原 native-component D3 receipt/companion、mandatory 公共 D3CanonicalEffects/1、resolved conflict、CP3 与完整效果，没有第二业务决议。

完整 preview 必须实际可读每页和每个 EffectBytes/2 slot。source、selected branch 和 canonicalStates 的真实 installed/selected/final 元数据及 active restore membership 由该效果 owner 完整投影；PinRef 不能作为公共读取接口。普通/full、narrow owner_fields 与 D8 PreparedEditBinding/2 各依实际 producer 的先验观察/披露资格，不在交付时删项或把 wrapper 变成 ordinary current source。committed 从原 final plan/receipt/CP3 独立重建，交付 epoch 的随机 token 变化不改语义。

当前准备所有旧/新有决议引用的输入仍服从原 last-reference 与 unknown 责任。相同字节改 canonical claim/version 的真实 admission 只在 D3/D6 原计划封存一次；metadata-only open 与 exact already-resolved no-op 按 D3 唯一规则，不在 D7 重新决定版本。新的 preparation expiry 只影响 unseen；saved/planned/unknown 先恢复原 decoder、原请求、原 guard/pins，不用新 Query、头选择或当前来源取代原计划。候选包含这些接口不等于它们已经激活；完整 D3/D6/D7 与其它实际消费者联合接受后才进入新执行。

### 6.1 Canonical namespace 的原子绑定

非 null resolutionInput.canonicalPlan 必须逐字按 D3 §10.1.1.3 closed D3CanonicalEffectPlan/1 解码，包含完整 canonicalStates、lifecyclePlan 和 source containers；不得用 canonicalInputs 或自由 metadata pin 替代 typed payload/reference/S 计划。before/selected/result pins、pin→hash 公共投影、完整 canonical_plan preview item 与全部 EffectBytes/2 slots 和原 native preview 同一次原子准备保存。每个实际 physical source 的 proposedInput 唯一，并与两个 component 的 exact-overlap 规则及同 /2 source plan 对齐。

原十二 receipt 数组只证明 native 分量；新 resolution 的 committed complete consumer 必须同时取得 D3CanonicalEffects/1、原 receipt/companion 和完整 D7 /2 公共 effects；preview 只要求完整 canonical_plan 投影及 before/selected/result 字节，不要求尚未 seal 的最终扩展字节。extension 与 canonical plan/inputs/request/fingerprint/selection 不能分离或换 head。缺任一 mandatory producer/typed gate 不得 prepared；真实 saved commit 之后缺当前完整 delivery 只返回 effects_unavailable，保留原 commit 与恢复责任，不重建新 binding/decision。普通 D7、D8/D6 和真实历史 /1,/2 records 不被该新 profile 暗增字段或重新解释。

当前 MinimumMapping/3 registryBinding=null 的唯一正例是实际无 Registry 用途且 record.registryInputs=[] 的原生动作。D3 typed prepare、完整 record/guard、原 wire12 request、canonical plan 和 preview 的共同校验同时断言此条件；不能在任一层删除 binding 或换 preview 后取得 raw 资格。凡真实 D4/C carrier、D7/Q definition、classification/Field/关系或其它 typed Registry用途，必有真实非 null RegistryBinding/1 与完整 immutable contexts。null 不代表 Registry 空、默认 schema 或已验证某个不可见贡献。该新 /3 最小映射规则仍须与 D3 冻结引用作最终联合核对，不能改真实旧 /1,/2 最小映射。
