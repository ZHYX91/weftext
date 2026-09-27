---
_weftext:
  id: "6375684c-8a5e-4c9e-ae16-c840233f77cb"
---

# D9 配套 D3/D7 准备绑定修订

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


revision: D9-r04-2026-09-24；candidate；未激活。此联合候选明确重开下面必要接口及其机械镜像/版本引用。D3 wire11、Result/9、D7 ActionSpec/public prepare形状、D6提交与唯一ledger不改；D1/D2/D4/D5/D8及其余D6/D7合同保持。完整本地replacement位于candidate-upstream，只有独立门禁通过后与D9协调激活。

## 1. D3下游模板承接条款的精确替换

仅替换D3主文“D9 import/export/template/worker”中Template一段；除下述21B镜像外其余正文不变。旧句把所有实例化一律描述为identityMap，与其严格per-mode矩阵矛盾。本替换以原mode的实际receipt为准，不增加身份分配路径。

替换后的完整条款：

- Template是 coreKind=template 的Core meta-kind。实例化始终产生fresh Node/Resource/Annotation refs，原身份结果严格按实际D3 mode：copy/fork/identity-bearing import返回其完整identityMap；create或ordinary-format import的identityMap为空，返回完整resultAllocations及适用placements/reference证据。D9可从完整来源到symbolic subject的准备证据与原resultAllocations机械关联显示来源到实例，但不得增设第二身份权威或改写receipt。Task Template只可由D9 target plan声明targetCoreKind=ordinary且target facets包含exact tasks/task；Template自身禁止因此成为Task。每个真实typed slot按该mode的fresh result/identityMap规则完整处理；Facet/carrier/entry occurrence不成为reference slot。

## 2. D7 Prepared Action Binding 完整规范 replacement

下面从第一个规范节起为新完整正文。既有文档ID保留，历史接受标签不构成本候选已经接受的证据。

## 1. 一份不可变准备记录

Core受管`PreparedActionBinding/2` exact semantic members为`kind,version,bindingToken,protocolOwner,operationId,workspaceRef,principalAudienceToken,action,canonicalCallInputs,definitionInputs,registryInputs,ruleInputs,sourceInputs,constructionInput,proposedInputs,dependencyProof,observationProof,budgetBinding,expiresAt,request,preview`。kind=`d7_prepared_action_binding`，version=2，bindingToken为独立tag=d7_preparation的D6 Token。其余语义类型如下，不能使用自由dictionary来遗漏证据：

- action为原完整ActionSpec；canonicalCallInputs是按实际调用路径排序的完整QueryCall数组，包含有效arguments/context，普通无Query动作为空；definitionInputs逐项是`{definition:DefinitionAddress,ownerVersion:SourceVersion,payload:text}`，保存实际解析wrapper/Query/View的完整source payload，不由调用方自报。
- registryInputs为原完整D4 RegistrySnapshot及RegistryBinding、完整展开所用定义；ruleInputs为原完整D4 RecurrenceReadContext/Binding及实际使用的其它已closed规则贡献。没有使用时对应空数组，不填假context。
- sourceInputs是原D3/D6源pin目录，每项含完整subject、SourceVersion、exact source/bytes binding及actual role；proposedInputs是同原D3 Result/9或D6完整源变换的已验证符号/具体输入。完整类型逐字引用所属D3/D6 closed类型，不重新定义Ref/Locator或允许任意JSON。
- constructionInput 是 null 或 D9 Templates §6a 的完整 TemplateConstructionInput/1。只有Core内置D9 node-template adapter可建立非null值；公共ActionSpec/d7_action_prepare不接受额外construction字段。非null仅用于D9指定的ordinary-import d3_operation或简单collection_create；完整输入pins、版本、参数、loss、sourceSubjectBindings独立重编译后须等于request及proposedInputs。sourceInputs原类型不变，不塞新的role或私有artifact variant。
- dependencyProof为实际完整正/负依赖，包括全部pre/post Query、定义解析、Registry/rules、authority/state/placement/ForeignBinding、授权与源Envelope元数据；observationProof为同包D6选定scope与当前潜在范围资格、逐constraint保持证明。原D6 budgetBinding包含已耗账户，不能在replay重新初始化。
- request为准备完成后返回的完整原D3/D6 request bytes；preview为效果规范的完整不可变语义清单、原字节pins及初始交付投影，包含适用conditional_source_change；不能把一个采样map当作确定后像，尚无identity reservation。交付handle重签发不改变语义清单、原request或此准备记录。expiresAt遵守D6 clock epoch/期限域，不从设备时间猜。

记录在返回任何prepared响应/token前原子保存并验证所有引用的pins齐全。bindingToken生成与request构造只有以下次序：先生成随机token（无作者/identity effect），构造包含该token的最终request，再保存完整记录及最小授权定位映射，最后返回；不把“request包含token、token需hash request”变成循环散列。完整相等用原canonical request与原规范对象/byte pins，不能以一个相同hash替代证据。

最小定位映射/2 exact {bindingToken,protocolOwner,operationId,workspaceRef,principalAudienceToken,observationScope,registryBinding,constructionReadRefs}，受保护且无作者值。constructionReadRefs是按D3 RefKey排序唯一的EntityRef数组，非模板为空；非null construction恰覆盖其inputPins、omittedAnnotations及构造所需的其他完整来源读取Ref，不含source bytes。Core记录版本选择/1或/2 decoder，不按字段缺失猜版本。D3 stage3与D6原授权步骤先读此映射，证明current audience、固定scope及这些来源的完整当前读取资格，再读完整record/ledger；missing/wrong-tag/wrong-audience或资格不足一律not_visible（D3映identity_not_visible），不泄露记录存在。此增加的是D9实际输入的读取检查，不授新权限、不改变原阶段先后。

## 2. D3显式wire11承接

D3 wire11顶层在原wire10 exact members之外增加唯一可选`preparationBinding`，其exact值为`{kind:"d7_preparation_binding",bindingToken}`。此成员进入原requestFingerprint；绑定的规范原request包括它，顶层OperationId仍按原规则不进入fingerprint。D7发出的D3准备请求必须包含它；独立raw D3请求可以省略，但只能表示自身完整D3意图，不取得D7 postcondition、preview或保存策略证明。

stage1验证closed形状、Token词法与known kind；stage2验证原Workspace roles；stage3在原D3全域/issuer/source授权之外，按最小定位映射证明准备记录current audience与相同固定scope，原P1/P2/TL顺序不变。stage5只比较已包含preparationBinding的原fingerprint：同OperationId同D3 payload而不同D7条件必然使用不同bindingToken，不能混用同一saved request；不同fingerprint仍按原stage5冲突，不读取/回显原条件。

unseen请求继续原stages，stage14在原identity_map_incomplete检查之后增加唯一family `operation_precondition_failed`：已授权的绑定记录过期、其完整request与所收请求不相等、原D7 pre/post及非null construction完整proof与当前cut依赖不成立、原D7完整postcondition不满足，均recorded_rejection。不能证明pin/authority/连续性属于availability，按原availability路径不伪造确定业务拒绝；原stage15 inbound closure仍随后完整执行。未知/丢失授权定位记录已在stage3遮蔽，完整记录损坏在连续性/完整性检查停止，不能仅因记录丢失生成一个新计划。

同一planning CAS比较原ledger unseen/family/current authority及绑定记录和完整依赖，保存原request、PreparedActionBinding完整内容、pins、scope、实际D3计划与reservations。已有planned恢复只使用这份保存内容，不重跑漂移后的Query或重新选择targets。已有saved decision的stage5重放不检查旧preview/preparation TTL，但仍先当前原授权和scope；返回原receipt/error bytes。撤权不写永久rejection；重获权仅恢复原decision。

prepare不会领取或烧毁最终content IDs；fresh内容保持D3 symbolic subjects，到原stage12临时candidate及原planning reservation才按已有D3规则产生真实映射。原D7 post-query的proposed evaluation在同一已绑定symbolic candidate namespace进行；fresh equality/NodeRef输入只能由Core typed替代环境提供，不把symbolicToken当公开NodeRef。stage14以最终candidate map重新验证同一语义与完整结果，不能把用伪UUID运行的一次样本当证明。

## 3. D6承接与寿命

D6 request继续通过原planToken唯一选择PreparedIntent；该PreparedIntent完整嵌入PreparedActionBinding及对应D6 request，不增加D3模式或第二commit入口。planToken与bindingToken是不同tag，两者一次绑定且不可互换。相同planToken永不指向另一个action、post-query、输入或targets。D6原步骤6/7逐项CAS和保存这份完整D7证据，D6错误/disposition顺序保持。

未被任何decision引用的准备记录在有限TTL后不再允许新提交，可以释放大pins；保留最小token/tag/audience/scope及expired标志至受管token失效保留期，以区分已知本人过期与未知。任何planned或terminal saved decision引用后的最小授权映射和完整record/pins按该原ledger保存与恢复规则保留；不受preview TTL清理。ledger恢复所需pin不能删；preview丢失仅preview不可用，不改commit事实。若连续性不可证明，保持原planned或遮蔽交付，不重新prepare代替。

显式重新prepare产生新tokens与新OperationId；d3_operation已给OperationId时不改其值，但不会在同一ID下修改现存不可变记录。未提交的多个独立records不占原ledger key；最终只有原ledger CAS winner能形成decision。没有“准备成功就保证最终提交”的预留授权。调用者改动返回request后，它成为另一canonical输入，原证据不再匹配，且不能自动使用旧preview确认新请求。

lost receipt只能重发原request；相同saved decision在当前资格通过后返回原bytes，不再次执行Query/字段修改或增加commitSequence。未知是否提交时不能以新OperationId重做。完整post Query（包括sort/take、scalar、QueryRef、所有正负依赖）在准备与原planning/commit证据中都固定；显示200条preview不减小条件范围。

## 4. 版本及历史

本联合版本的新准备记录一律为PreparedActionBinding/2，新增constructionInput并使用最小定位映射/2；非模板明确null/空refs，不自动从作者字符串推断模板。已有/1 records及其planned/saved decisions继续其原decoder、原最小映射和byte-equal恢复/重放，不填字段、不换token或重新解释历史授权。新模板准备只使用/2。切换后尚未形成decision的旧/1准备不能建立新author effects：原授权门后，D3 unseen在stage14按operation_precondition_failed、D6在原步骤6按semantic_rejected记录准备失效，用户须明确重新prepare。已planned/saved在原ledger门先恢复/重放，不被此unseen版本检查回溯拒绝。D3 wire11、preparationBinding对象形状及D6提交wire均不变：新不可变bindingToken选择/2记录并进入原fingerprint。无法加载对应版本或完整pins时按原availability/完整性门停止，不能按空construction处理。


D3 wire10与wire9已保存decision仅以各自原decoder、原fingerprint、原门和原byte receipts重放/恢复；禁止新wire10/9 decision。不能把旧request改成11、补token或按新语义重新解释保存的旧结果。新wire11的request/receipt/error及Result/9需完整新conformance，历史v10测试不证明这些新增合同。D6 Policy/2及新metadata资格只作用实际采纳新profile的请求/准备，不悄改原D3 create/fork已明确授权的历史receipt重放范围。

## 3. 必须同批生效的镜像及版本消费

D3主文21B.1–21B.4用上面D7规范§1–§4逐字替换，仅章节号变成21B.n；不得保留旧/1 exact shape与新/2并列为新请求权威。D3 21Q定义转移及其他stage/Result条款全部保持。

### 02 Decisions\D6 Storage Transactions Permissions and Sync\D6 Control Interfaces.md

仅替换含PreparedActionBinding版本的原段落，完整新段如下：

D6 PreparedIntent额外承载完整PreparedActionBinding/2（历史/1仅按原规则恢复/重放）及其原D6 request，原planToken不可变；D3 wire11的preparationBinding由原D3 stages处理，D6不创建另一ledger。按D7《Prepared Action Binding》的完整同包正文消费；其所有input pins、pre/post Query dependencies、受权scope和preview在准备返回前固定，planning/commit的原CAS逐项比较。记录被原decision引用后不受preview TTL回收。当前权限不足遮蔽重放，不改变decision。

### 02 Decisions\D7 Query View Action\D7 Preview and Effects Transport.md

仅替换含PreparedActionBinding版本的原段落，完整新段如下：

Core保存完整、不可变的语义效果和每个字节slot的exact pinned payload；这份同decision证据不含会随交付重签发而改变的授权含义。`EffectManifest/1`是该完整语义效果在一个交付epoch中的closed运输投影，exact `{format:"weftext.effects",version:1,phase,protocolOwner,operationId,workspaceRef,profile,items}`，phase=`preview|committed`，profile=`full|owner_fields`。语义items、完整pins及该epoch所有EffectBytes投影在任何首page前已形成、验证完毕且immutable；取页不能再执行Query、源变换、D4 gate或追加效果。preview绑定D7 PreparedActionBinding/2（历史/1仅按原规则恢复/重放），或D8 Editor Interfaces定义的PreparedEditBinding/1；D8 producer严格只允许protocolOwner=D6与profile=full，并由完整D6 PreparedIntent绑定原request、前后源、scope、依赖与预算，不伪造D7 ActionSpec/bindingToken。两类producer都须满足本文件完全相同的完整性/授权/epoch/期限门，D8准备成功使用d8_edit_prepared承载同一header及tokens；committed绑定原D3/D6相同OperationId、原request、完整plan、receipt及原子D6 companion。transport tokens不是作者source、结果身份或额外授权。

本修订不将D8 PreparedEditBinding改成D7记录；D8生成提案与效果producer保持原门。历史/1已保存decision保持原bytes、原恢复资格和原decoder。
