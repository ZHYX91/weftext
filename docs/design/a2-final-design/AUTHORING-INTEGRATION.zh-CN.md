---
source_language: zh-CN
translation_status: source
---

[English](AUTHORING-INTEGRATION.md)

# 作者源集成：准确来源、版本与消费者

状态：本文件是 [AUTHORING](AUTHORING.zh-CN.md) 的规范集成部分，仍是未实现的设计候选。这里只替代下列具名 current 槽位；原真实记录及未列出的语义保持。它不是第二份 Field/schema 目录，也不自动关闭原 21 项全局问题。

## 1. 显式内层分派与历史域

先识别外层真实 kind/version，再只沿本文件枚举的类型槽选择内层 decoder；不能递归搜字段或使用“latest”。新 ActionSpec3、Snapshot2、Binding2、Recipe3、Environment4 均有显式版本。保留外层形状的对象，按所列内层 discriminator 区分新 profile 与原 profile；旧 arm 的闭合 decoder、原 pins/bytes/恢复责任不变。保存/已计划/unknown 首先定位原决议和原 arm，绝不按当前软件重编码。未见的新请求必须满足当前 profile，缺新版本字段不能降级旧 arm。空的未消费槽不会凭空要求新版本。

新增语法不增加新的权限、账本、CAS、Notice 或 CP。受保护 canonical descriptor、完整 preview、原 source/registry proof 和最终 read barrier 必须绑定同一确切内层对象。未知或混合不合法的 arm 拒绝，不以可解析 JSON 代替完整 schema 验证。

## 2. 原生 Field 的真实选择与效果

```text
FieldSelector/2 =
  {kind:"entry",owner:NodeRef,fieldId:FieldId,expectedRevision:Counter,
   occurrenceKey:UUID,rawEntrySource:text}
 |{kind:"native_header",owner:NodeRef,fieldId:FieldId,expectedRevision:Counter,
   occurrenceKey:UUID,rawAttributeSources:[text...]}

FieldSource/3 =
  {kind:"absent"}
 |{kind:"entry",entries:[{occurrenceKey:UUID,rawEntrySource:text}...]}
 |{kind:"native_header",occurrenceKey:UUID,rawAttributeSources:[text...],
   resolution:{state:"value",value:TypedValue}|{state:"absent"}}

FieldSourceImage/3 = {
  format:"weftext.field-sources",version:3,ownerNodeRef:NodeRef,fieldId:FieldId,
  sourceVersion:SourceVersion/2|D7ProposedVersion/1,source:FieldSource/3
}
```

所有对象闭合。entry 的 raw 是完整原 Entry JSON。native 的 rawAttributeSources 是同一 root 文档头中、该 Field 已绑定名称的全部 set/unset 原文，按真实作者顺序，保留每个事件的准确 UTF-8 内容及自身换行，不夹入事件之间其它字段/正文。名称来自同 cut 的 authoringBindings，调用方不另交名字。绝对 sourceRange 留在内部 NativeFieldOrigin，Field-only 输出不披露隐藏前文长度。重复赋值只产生一个有效 Field 事实；absence 无 selector，FieldSource.absent 指该 Field 无作者载体而不是整个 Node source 不存在。Entry 空集规范为 absent。native raw 集非空时，即使最终 unset 也必须保留 native_header，resolution.state=absent；此时 occurrenceKey 只定位载体，不产生事实 selector。只有 resolution=value 才有 native 事实 selector。非法 native 值不伪装 absence 或 TypedValue；其修复走完整 Source 的 exact_source_utf8 前后像，不发可用窄 selector。Recipe field_set 从有效 absence 创建值时，以保留 raw 集的 native-absent 作为完整前像。

selection/read 先核原 source_envelope_state、完整 Field 读取及真实 native 依赖的披露资格，再产生完整 raw 集与 typed value；提交另核原 commit_sequence_state/写权限。含不可披露 substitution 来源、非 root/生成来源或不完整依赖时不能发出可用窄 selector，也不能泄漏隐藏原因、offset 或 raw。Core 在完整源中重定位全部事件并 byte-match，绑定原完整 SourceVersion/Observation、Registry 和正负依赖；derived key 或 raw 相同不能代替连续性。完整输出超预算整体失败，不截断。无依赖的普通 task status 仍必须有 Field-only read/write 成功路径，无须授予正文读取。

D7 field selection 的 open、selection、selection_evidence、action_evidence 新 family 使用 wireVersion3，除版本与 selector target 为 FieldSelector2 外保留原准确成员；selection_evidence.actionKind 增加 set_field_source。原 wire2 保持原 Selector。selectionToken/evidenceToken 仍原运行时 token、audience、TTL、预算、完整 source cut 与授权世代；先复验原门再取 occurrenceIndex，不能给旧 token 重新签发新资格。Query 一般行不因此获得 Field lineage。

D7 EffectBytes3 的当前 encoding union 增加唯一 field_sources3，精确解码 FieldSourceImage3；原 field_entries2 只用原 image2。EffectManifest3 的 owner_fields.field_change before/after 均使用新 image3，不能混合两个 image 代际；两个 sourceVersion 必须逐项等于该 effect 的 beforeVersion/afterVersion。显示完整前后 Field，不是差异或合成 JSON。preview after 仍为 proposed basis，committed 才是真实 sealed version。D8 当前 preview/bytes decoder 明确接受此 encoding，完整 Source 仍为 exact_source_utf8；外层不新增第二份效果。

## 3. 一个字段写入器与模板参数

```text
D7ActionSpec/3 = {format:"weftext.action",version:3,intent}

new intent:
{kind:"set_field_source",selector:FieldSelector/2,
 replacement:{kind:"native_header",value:TypedLiteral}
            |{kind:"entry",entry:D4Entry}}

TemplateRecipe/3 = {format:"weftext.node-template",version:3,parameters,nodes}
new binding slot:
{kind:"field_set",fieldId:FieldId,carrier:"native_header"|"entry"}
```

Spec3 保留 Spec2 的全部未变 intent；replace_field/set_field_member 仍只接 entry arm，native 调用为 action_not_applicable。remove_field 与 cleanup_facet_fields 接 Selector2，native 删除覆盖全部同名 set/unset，防止前值复活；cleanup 仍须证明无 effective Facet 依赖。append_field/field_append 保持 Entry-only，有 header 即 carrier conflict。bulk replace_single 复用同一写入器，要求每个 owner 恰一事实；不自动改为 append。新 set_field_source 覆盖 native 改值和显式两向表示转换，不引入任意 source patch。

native→native 通过已验证 codec 生成可逆原生字面，仅改最终有效值的准确 span，保留其它赋值、位置和 trivia；完整重解析证明新 typed 值及所有效果。header→Entry 删除完整同名集，完整 Entry 使用所选 derived UUIDv5 作为固定 occurrenceKey（D4 已允许 UUID1–5），不再把它视为 header 连续性。Entry→native 必须是唯一 occurrence、codec 可逆，且无不能保留的 qualifier/note/provenance/union 信息；否则拒绝并保留 Draft。Entry→Entry 保留 key。表示转换与删除必须完整证明其它属性/正文 native 引用的影响，无法在获权 Field 范围内证明时只可走独立完整 Source 授权或拒绝，不通过隐藏扫描的布尔承诺放行。具有全源权的用户可预览并显式展开相关 native 引用为等效文本后转换，不能建立 shadow 属性。

新建/无值的 Field 由已获权 Source constructor 或 Template field_set 构造；不存在的 selector 不伪造。AssignFacet 原 initialEntries 仍精确 Entry 构造器，未填已有 native 必填字段时可保留它；若同时给同 Field 的 Entry 则冲突。RemoveFacet 只更新 原生 weftext-types 的已声明类型并验证 closure，保留所有业务值。FacetOperationRequest2 的 cleanup 原 Entry-only 形状不扩宽：新 FacetOperationRequest3 保留其余成员，wireVersion=3，仅 cleanupSelectors 改为完整 Selector2 数组，其余 null/空数组及 operationKind 适用规则不变，Core 从真实选择构造并提交同一 D4 gate。

D7ActionPrepareRequest3.action、D7ActionInput3.action、PAB4.action 的新 current arm 精确为 Spec3；按 action.version 分派，Spec2 真实记录保持旧域。其 canonical descriptor 与 PAB 对应成员 byte-equal 规则不变。D8 属性面板/任务控件使用同一 Action；D10 core_field_member 的现有 object-member 动作仍 Entry-only，不能把 scalar status 冒充 object member。普通任务 status 修改用明确 Spec3 set_field_source；其它 Automation action 能力未因此扩张。

Recipe3 的参数及旧 title/body_text/field_append slot 不变，field_set 用原参数绑定并设置一个 singleton 默认事实，无则创建；多 occurrence 拒绝，carrier 改变执行同一无损转换。native 参数必须匹配展开 ValueTypeSpec 与完整 codec/wrappers，日期不猜历法。TemplateConstruct2.recipe 本来是受保护的 Resource 版本引用，故其精确 pin decoder 按 Recipe.version 选择2或3；ConstructionInput2/sourceSubjectBindings/PAB4 不增成员、不预分配最终 NodeId。import 的 typed Field mapping 同样在 Core 选取显式 carrier 并产生真实 raw；未知或非可逆值进入原 loss/拒绝路径。

D4SourceMaterializationEffects2 保留原 /1 的 kind、operationId、registryBindings、owners 与完整版本依据；wireVersion=2。entries 项保留 sourceSubject/resultOwnerNodeRef/fieldId/referenceChanges，将单一 occurrenceKey 替换为 beforeOccurrenceKey/afterOccurrenceKey（各为 UUID 或 null），两个 rawEntrySource 槽替换为 beforeSource/afterSource：选定载体的 FieldSource3，fresh 或完全删除一端为 absent。每个非 absent source 的 Entry arm 恰一项，或为一个 native arm，分别与同侧 occurrenceKey 相等；真正 absent 侧的 key 为 null。前后 key 不必相等：Entry UUIDv4→native UUIDv5 必须保留两者，header→Entry 可按既定规则保留 key。项按 sourceSubject 的原 canonical key、resultOwnerNodeRef、FieldId、beforeOccurrenceKey、afterOccurrenceKey 排序唯一；null 排在 UUID 前，UUID 按准确 ASCII 字节。不得把转换拆成未关联的删除/新增来丢失前像。每项及 owners 从同一完整前后源重算；所有未变 C owner 仍覆盖。新 semantic_extension format 为 d4_source_materialization_effects/2、encoding 为 d4_source_materialization_effects2；D7/D8 decoder 明确接受；在 retained effects §4 的 format rank 中 /2 与 /1 同属 materialization rank=1，同一 manifest 二者互斥，旧 /1 完全保持。此定位不关闭原全局效果排序问题。

### 3.1 具名 native 窄适配器与依赖状态权限

本节精确替代 d7/owners/narrow-field/source 的 §2 首段和条件2–5在新 native 字面改值分支的限制。原 Entry 窄适配器及其它排除仍原样有效；不能凭 Selector2 就绕过静态证明。

原 source_envelope_state 只披露列出的聚合有效性/容量，不自动授权 native 依赖拓扑。新增 Policy4，精确继承 D6-CONTROL §10/§10.1 Policy3 的 {version,revision,grants} 及 grant/deny/原 capability 语义，version=4，仅 capability union 增加 {kind:"native_dependency_state",fieldId:FieldId}。此能力只允许 workspace scope，FieldId 必须是当前 schema 的精确 ID，默认不授予，不由其它读写/管理员能力隐含；实际安装/修改仍要求原 policy_admin 管理路径。旧 Policy3 不补 grant。D6 policy decoder、policy key/pin、authorization DependencyKey3/proof 的对应 policy 槽按 policy.version=3或4精确分派，同一授权世代/原 seal 冻结完整对象；旧 recorded1/2/3 原恢复。既有 bootstrap 可以保留 Policy3；未显式安装此 grant 时新 native 窄路径不可用，全源编辑保留原授权路径。

Policy4 的准确 current 消费槽包括 D3-SCHEMAS/D6-SCHEMAS 的 WorkspaceAuthorizationBundle2.policy、ConflictResolutionInput3.resolution.policy、ConflictResolutionPreview2.resolution.policy，以及 D6-CONTROL §10.1 管理入口的完整 proposed policy/结果 Bundle2.policy。它们严格按 policy.version 接收 Policy3|4；3→4 必须是经旧 policy_admin 授权的显式策略后继，按原规则 checked-increment Policy.revision、auth generation 和 bundle.authorizationRevision，完整保留原 trust histories，不能只改 version 或偷偷补 grant。conflict choice 与 resultBundle.policy 同样保持实际原版本或合法显式后继并逐字匹配；真实旧 Bundle1/recorded 域不扩大。当前 bootstrap.initialPolicy 仍可使用原 Policy3，不自动生成新能力；Bundle2、CP 与 pin prefix 不升版。

该能力仅允许获知指定 Field 的聚合隔离资格和由它影响的选择/准备结果，不授权名字、数量、路径、位置、raw、其它 Field 值或正文。复用原 selectionToken/proof 与授权失效机制，不新增状态端点、token 类型或 proof。只读 typed Query 不需要此能力。wire3 Field selection 对 Registry 中存在 native binding 的目标 Field 必须在读取作者源/拓扑前检查该 grant；无 grant 统一 not_visible。显式 native selector/intent 同样先验检查；不是先扫描后根据“碰巧隔离”决定所需权限。Entry-only wire2 原合同不扩大，不能借旧入口取得 native selector。

仅新增 set_field_source 的 native→native 直接字面改值（和同形 bulk replace_single 分解）进入此适配器。先按原条件1和完整 Registry 图建立潜在 Field/constraint 闭包，再核原 Field read/write、entity/state、source_envelope_state、commit_sequence_state、source_write deny 以及新增 capability。然后由同一 D2 evaluation 的完整 attribute-use/origin 证据证明：目标是唯一可写 root 直接 codec 字面，不含 substitution/import/generated 输入；其它 header 字段、正文、条件、macro/include 目标、URL、extension/provider 没有读取这个属性名。必须追踪完整实际原生读取与负范围，不能只搜索字面引用；不能完整追踪时不可用。外部读取非空或证明不可用一律仅报告聚合不合格，不能泄漏具体原因/拓扑。有正文依赖时本轮只允许独立完整 Source 授权；其它 Field 依赖也不由此适配器放行，须其完整扩大获权路径。

对原条件2，读取完整 FieldSource3 及 codec/union/code/qualifier/provenance 相关贡献，证明原/拟议状态都是恰一个合法有效事实。对条件3，required_field 和 cardinality 由 present 数量不变保持，所有可能 Facet constraints 仍逐个静态证明；root union variant 必须不变，未知 constructor 或依赖未获权则拒绝，不看当前隐藏 membership 来放宽。对条件4，唯一允许变化是目标最终有效值的准确 byte span；其它 set/unset、weftext-id/weftext-types、namespace、trivia、header 其它字段和正文逐字不变。codec 必须生成合法安全的原生字面，完整重解析不得新增属性/控制行或改变 parse 结构。对条件5，原计划保存完整 FieldSource3 前后像、真实 byte patch、逐约束保持证明、实际 source/Registry/environment 与完整正负原生读取证据。内部全源仅用于保全和已获权聚合验证，公开只返回原 Field/元数据范围。

该证明绑定 before 和拟议 after 的同一固定 processor 环境/源 pins，最终 read barrier 重验。unset/absence/非法值、remove、表示转换、多来源或新增字段没有自动窄资格，使用各自完整 Source 授权或拒绝保留 Draft。成功/失败的隔离差异是新增能力明确授权的聚合元数据，不伪称完全隐藏源活动。INT-01 正例明确授予 native_dependency_state(tasks/status)：先验图证明及权限→完整无外部读取证明→一个有效 status 的安全 open→done patch→同 cut FieldSource3 image→原 P seal；无此 grant 在读拓扑前拒绝，body 引用和未知 extension 分支聚合不合格，绝不自动授予。

## 4. 注册表与实际控制槽位

Snapshot2 按 AUTHORING §4 精确十八成员解码。RegistryEvolutionProof2 的闭合形状为 {version:2,previousSnapshot,currentSnapshot}，previousSnapshot 是已认证真实 Snapshot1 或2，currentSnapshot 必为2；同样校验原 predecessor、完整摘要、所有者、semantic ledger。这里新增 version 是分派标记，不把旧 Proof1 无版本对象放宽。catalog 和 RegistryBinding1 不变。

下表是实际 current 槽位的规范补丁。new-current 必须完整保存 Snapshot2 bytes/digest/pin，不能删除 authoringBindings 以塞进旧槽。跨现有 holder 的新分派依据正是列出的固定路径上的 snapshot.version=2 或 evolution.version=2；若是旧的被记录 holder 则先走原恢复 arm，不能在原 pins 中换对象。相同 holder 内的 registryReads 不得混合1/2；evolution 的真实 previousSnapshot 例外已具名。

| 实际 owner / 槽 | 新 current 类型与规则 |
| --- | --- |
| D4 §2.2 handshake、ValidatedCatalogContext、registry/temporal proof | Snapshot2 + 每个非 bootstrap 代必需的 EvolutionProof2；原已认证 bootstrap 保留 predecessor 的 registry_bootstrap marker，没有 previousSnapshot/演进 proof；固定 catalog 保持。 |
| D3-SCHEMAS WorkspaceBootstrapTargetRegistry1.snapshot 与 D6 bootstrap producer | Snapshot2；原目标信任/授权/安装边界保持，既有 Plan4/policy 不复制。 |
| D3-SCHEMAS 与 D10-CONTROL §0 D10WorkspaceReadDependencies2.registryReads[].snapshot | Snapshot2；真实完整 binding/pin 同 cut；空数组不造读取。 |
| D3-SCHEMAS 与 D10-CONTROL §0 D10ControlEffectPlan2.registryChange | before.snapshot 为真实1或2，after.snapshot 为2，evolution 为 Proof2；sourceChange/权限/排序规则保持。 |
| D10-CONTROL §7 activation body、§8.1 activation supplement 与 D3-SCHEMAS 对应槽 | registrySnapshot 为2、registryEvolution 为可空 Proof2；实际 before image 保留原 recorded arm，after 带显式内层 version2。 |
| D10-CONTROL §0.2 ScheduleRecurrenceEvidence2.registrySnapshot/registryEvolution | Snapshot2/可空 Proof2，完整相同 scheduling cut；历史 evidence/Witness/Step 不重新签名。 |
| D6-CONTROL document-format、registry/period/bootstrap proof；D10-CONTROL §12 bootstrap 消费 | 真实 Snapshot2/Proof2 及完整资格，不根据文件或 plugin 缺失推断空表。 |

实际 D10 activation 仍是 Image1/Pin1 的 activation arm，不是只承载 automation/run 的 Image2。它只在 supplement.registrySnapshot.version=2 及对应 Proof2 处选择新嵌套域；D10-Control-Record/1 pin prefix 保持，完整 pinned canonical bytes/digest 覆盖 version 和绑定表。逐槽递归分派，不因一个 Snapshot2 放宽整个外层。旧 before/initial/checkpoint/image/pin 各自保留准确原版本；一条有证据的显式演进链可以跨1→2，不能把 registryReads 同次读取不得混版的规则误套到整条历史链。

## 5. 格式生产、接收与渲染

Profile2/Binding2 精确按 AUTHORING §7。新 managed2 的链接地址解释同时遵守 [LINKS](LINKS.zh-CN.md)；新库根 codec 不在真实旧 profile 的重读/恢复中隐式激活。AsciiDocProcessorEnvironment4 完整继承 D2 Environment3 的成员，version=4，documentProfile 增加 weftext_managed/2；其它 environment 成员不变。新 managed2 evaluation 必须等于已验证 Binding2.profile，包括原 languageBaseline。实际 render 例为 snapshot.evaluation.processorEnvironment.documentProfile 等于 semanticQualification.documentFormat.binding.profile.managedProfile，两处 languageBaseline 也相等。DocumentFormatCurrentQualification2 继承原 /1 全部成员，version=2，binding 为严格 Binding1|Binding2，componentImage.version 仍等于 bindingRevision，绝不能写入 decoder 的1/2。ManagedDocumentSemanticQualification2 保持原 sourceObservation/documentFormat 成员，documentFormat 使用 Qualification2。真实旧 Qualification1/Semantic1/Environment3 永远按原类型。

D2 evaluation/snapshot 的 processor-environment 槽显式接受 Environment4，由内层 version 分派；旧环境 pin 不重编码。D3-SCHEMAS/D6-SCHEMAS 的 format qualification、D6-CONTROL §0.1.2 component decoder/§0.1.3 receiver admission 与 document_format dependency 都接上述精确类型。receiver 按实际 binding.version=1或2 选择闭合 decoder，未知版本/错误 profile 拒绝；portable_metadata pin 精确为 D3-CJ/3(binding)，原 CP4 component rank、Notice3、ChangeRecord1、P seal 与完整关联验证不变。managed2 缺 binding/registry 仍 incomplete，不能凭控制属性可读自授 admission。

D9DocumentRenderBinding1 的新 current semanticQualification 槽采用 SemanticQualification2，准确 D2 snapshot pin 消费 Environment4；以其 semanticQualification.documentFormat.version=2 与 snapshot.evaluation.processorEnvironment.version=4 选择新 rendering profile，不篡改旧 ExportPlan4 中已冻结的 Semantic1/Environment3。新 Plan/confirmation 的输入并集与摘要完整覆盖新内层对象；prepared 以后不得转 profile。exact-source/resource/query-json 的 null render binding 路径不受影响，真实旧 Plan/receipt/recovery 保持。

## 6. 原生 ID 和类型的构造与原 C/Q

原 Result9/C/Q 闭集不接受新的控制声明 tag。本候选使用 Core 内部 ManagedControlConstruction1 作为完整源构造的具名中间表示，不能直接作为原 Result9 字节提交，不能被 worker 或文件 UUID 自行认证。它不解决原全局“symbolic Document 输入”P1：正式 D3 提交 framing 仍须与该问题联合完成独立验收。在此之前，新建/template/import 的 managed2 端到端依赖仍未关闭，已有身份属性编辑不依赖新 ID 构造。

```text
ManagedControlConstruction/1 = {
  kind:"managed_control_construction",version:1,
  subject:PayloadSubjectKey,
  preimagePin:PinRef/2,
  idSite:{kind:"existing_value",range:D2SourceRange/2,rawValue:text}
        |{kind:"insert_header",byteOffset:Counter,eol:"LF"|"CRLF"},
  declaredTypes:[text...]
}
```

仅 Core 由同次 D2 完整原生解析、header 边界和来源证据构造上述闭合中间表示。existing_value 指向已验证 copy/template 前像中的唯一直接 weftext-id value span；insert_header 只用于无该保留声明的准确 AsciiDoc skeleton 或真实 managed1 的显式格式转换。插入点固定为可选 BOM 后，插入完整控制行及冻结 EOL，但不添加会终止 header 的空行；原文若以空行/正文起始，仍必须完整重解析，证明原标题/正文/author/revision 语义不被重分类，否则要求明确的完整源修复预览而不盲插。原 types 若已存在则独立验证/patch，不能随新 ID 插入重复 types；异常、重复、unset、非直接控制或已有外来 envelope 不能靠此构造绕过验证。类型位置与准确原文来自 preimage 的同次解析，不信任调用方给的 offset。subject 仅 Node existing/mapped/new；declaredTypes 来自实际获权选择/Recipe，按 same-cut schema 展开验证。中间表示受原预算/权限/保留约束，不公开预分配 UUID。

mapped/new subject 只从原 D3 stage12 私有 candidate map 取得 fresh NodeId；existing same-identity conversion 只用受保护原 NodeRef 和 birth/file binding，不能新分配。已有 ID 仅 patch 准确字面；types 使用独立 patch，变更时以 AUTHORING §2 的逗号协议编码，相同展开集合时保留原短名/顺序/空白。新增生成器优先使用固定 task 短名，其它默认完整 facet ID；空 types 省略新增行，已有行清空值而不生成 unset。缺 types 时紧邻 ID 完整行之后插入；若 ID 位于 EOF 且没有 EOL，先补一个分隔 EOL 再插类型行，保持原 ID value span 不变；新补 EOL 使用文档首个真实 EOL，全文无 EOL 时固定 LF，并冻结在完整拟议源中。模板实例必须使用 Recipe 目标类型并去掉 template，模板自身仍保留其原身份。普通复制无类型变更时保留声明。原生 ID/type 不属于 Field，字段 materializer、MapAttribute、Recipe field_set、remove/bulk 都不能触碰它们；类型变更依原 Facet/Source gate，ID 外改不能成为身份转移。

ID、types 与业务 patch 一同进入完整 preview/SourceTransform，保留所有无关字节、注释、BOM/EOL 与原生来源依赖。完整重解析证明 ID 与唯一分配/原身份、birth/file binding 一致，类型集与验证后的 declaredTypes 一致，再由 D4 求 effective closure。控制 patch 的真实位移先映射原 C/Q pinned 前像到中间/最终 source；C/Q 两遍复用同一 revision token，Q 验最终位置，afterPin 覆盖 bytes0..EOF。原唯一 planning CAS/P seal 同时冻结和安装 format/source/metadata；矛盾整体拒绝，planned recovery 复用原 bytes/map/UUID，不重抽或按 current 重算。

真实 managed1 普通 copy/fork 保留 Profile1/Binding1 和原源码；显式 copy+format-conversion 才在 fresh subject 上生成新控制声明，Binding2.bindingRevision=1。same-identity conversion checked-increment bindingRevision；所有真实源变更产生 sourceChanges，不能用 profile-only 空变更掩盖。restore 保留原历史版本。未部署的 YAML 前稿无新增 runtime decoder 或迁移责任。

## 7. 集成验收与剩余边界

以下 INT-01–06 是待实施的设计案例，产品 UNRUN；每项正常、拒绝、旧记录恢复均需审查。

| 编号 | 必须覆盖 |
| --- | --- |
| INT-01 | 已有合法 managed2 task，无 body read 的 Field-only status open→done：wire3 selection、Spec3、真实 image3、同一提交、D8读；隐藏 substitution/ABA/撤权拒绝。 |
| INT-02 | 多个同名 set/unset 的 remove 不显露前值；无损两向转换；两个同值复杂事实不合并，无法保全 note/正文引用时拒绝。 |
| INT-03 | Recipe3 field_set native status 覆盖默认值；field_append 遇 header 冲突；新实例 ID 构造仍依 §6 未关闭的原全局 framing。 |
| INT-04 | 真实 Snapshot1→2 明确演进，完整 bootstrap/activation/proof/recurrence 消费；缺 version/绑定表、混合新旧读取、错误 digest 拒绝。 |
| INT-05 | Binding2 原 pin→seal→第二副本 admission→Qualification2/Environment4→D2/D9；unknown version拒绝，真 Binding1 原恢复和普通复制不偷偷转换。 |
| INT-06 | 控制声明 patch 改变全文 offset 后 C/Q 和最终 afterPin 一致；candidate/ID mismatch 或部分 metadata 不安装；真实 saved/planned/unknown 不重编码。 |
