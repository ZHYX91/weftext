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
   value:TypedValue}

FieldSourceImage/3 = {
  format:"weftext.field-sources",version:3,ownerNodeRef:NodeRef,fieldId:FieldId,
  sourceVersion:SourceVersion/2|D7ProposedVersion/1,source:FieldSource/3
}
```

所有对象闭合。entry 的 raw 是完整原 Entry JSON。native 的 rawAttributeSources 是同一 root 文档头中、该 Field 已绑定名称的全部 set/unset 原文，按真实作者顺序，保留每个事件的准确 UTF-8 内容及自身换行，不夹入事件之间其它字段/正文。名称来自同 cut 的 authoringBindings，调用方不另交名字。绝对 sourceRange 留在内部 NativeFieldOrigin，Field-only 输出不披露隐藏前文长度。重复赋值只产生一个有效 Field 事实；absence 无 selector，FieldSource.absent 指该 Field 无作者载体而不是整个 Node source 不存在。Entry 空集规范为 absent。

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

新建/无值的 Field 由已获权 Source constructor 或 Template field_set 构造；不存在的 selector 不伪造。AssignFacet 原 initialEntries 仍精确 Entry 构造器，未填已有 native 必填字段时可保留它；若同时给同 Field 的 Entry 则冲突。RemoveFacet 只更新 YAML 的已声明类型并验证 closure，保留所有业务值。FacetOperationRequest2 的 cleanup 原 Entry-only 形状不扩宽：新 FacetOperationRequest3 保留其余成员，wireVersion=3，仅 cleanupSelectors 改为完整 Selector2 数组，其余 null/空数组及 operationKind 适用规则不变，Core 从真实选择构造并提交同一 D4 gate。

D7ActionPrepareRequest3.action、D7ActionInput3.action、PAB4.action 的新 current arm 精确为 Spec3；按 action.version 分派，Spec2 真实记录保持旧域。其 canonical descriptor 与 PAB 对应成员 byte-equal 规则不变。D8 属性面板/任务控件使用同一 Action；D10 core_field_member 的现有 object-member 动作仍 Entry-only，不能把 scalar status 冒充 object member。普通任务 status 修改用明确 Spec3 set_field_source；其它 Automation action 能力未因此扩张。

Recipe3 的参数及旧 title/body_text/field_append slot 不变，field_set 用原参数绑定并设置一个 singleton 默认事实，无则创建；多 occurrence 拒绝，carrier 改变执行同一无损转换。native 参数必须匹配展开 ValueTypeSpec 与完整 codec/wrappers，日期不猜历法。TemplateConstruct2.recipe 本来是受保护的 Resource 版本引用，故其精确 pin decoder 按 Recipe.version 选择2或3；ConstructionInput2/sourceSubjectBindings/PAB4 不增成员、不预分配最终 NodeId。import 的 typed Field mapping 同样在 Core 选取显式 carrier 并产生真实 raw；未知或非可逆值进入原 loss/拒绝路径。

D4SourceMaterializationEffects2 保留原 /1 的 kind、operationId、registryBindings、owners 与完整版本依据；wireVersion=2。entries 项保留 sourceSubject/resultOwnerNodeRef/fieldId/occurrenceKey/referenceChanges，将两个 rawEntrySource 槽替换为 beforeSource/afterSource：恰一个事实的 FieldSource3，fresh 或删除一端为 absent。Entry arm 最多一项且 key 等于该项 key；native arm 同样相等。每项及 owners 从同一完整前后源重算；所有未变 C owner 仍覆盖。新 semantic_extension format 为 d4_source_materialization_effects/2、encoding 为 d4_source_materialization_effects2；D7/D8 decoder 明确接受，原 /1 完全保持。

## 4. 注册表与实际控制槽位

Snapshot2 按 AUTHORING §4 精确十八成员解码。RegistryEvolutionProof2 的闭合形状为 {version:2,previousSnapshot,currentSnapshot}，previousSnapshot 是已认证真实 Snapshot1 或2，currentSnapshot 必为2；同样校验原 predecessor、完整摘要、所有者、semantic ledger。这里新增 version 是分派标记，不把旧 Proof1 无版本对象放宽。catalog 和 RegistryBinding1 不变。

下表是实际 current 槽位的规范补丁。new-current 必须完整保存 Snapshot2 bytes/digest/pin，不能删除 authoringBindings 以塞进旧槽。跨现有 holder 的新分派依据正是列出的固定路径上的 snapshot.version=2 或 evolution.version=2；若是旧的被记录 holder 则先走原恢复 arm，不能在原 pins 中换对象。相同 holder 内的 registryReads 不得混合1/2；evolution 的真实 previousSnapshot 例外已具名。

| 实际 owner / 槽 | 新 current 类型与规则 |
| --- | --- |
| D4 §2.2 handshake、ValidatedCatalogContext、registry/temporal proof | Snapshot2 + 必要 EvolutionProof2；固定 catalog 保持。 |
| D3-SCHEMAS WorkspaceBootstrapTargetRegistry1.snapshot 与 D6 bootstrap producer | Snapshot2；原目标信任/授权/安装边界保持，既有 Plan4/policy 不复制。 |
| D3-SCHEMAS 与 D10-CONTROL §0 D10WorkspaceReadDependencies2.registryReads[].snapshot | Snapshot2；真实完整 binding/pin 同 cut；空数组不造读取。 |
| D3-SCHEMAS 与 D10-CONTROL §0 D10ControlEffectPlan2.registryChange | before.snapshot 为真实1或2，after.snapshot 为2，evolution 为 Proof2；sourceChange/权限/排序规则保持。 |
| D10-CONTROL §7 activation body、§8.1 activation supplement 与 D3-SCHEMAS 对应槽 | registrySnapshot 为2、registryEvolution 为可空 Proof2；实际 before image 保留原 recorded arm，after 带显式内层 version2。 |
| D10-CONTROL §0.2 ScheduleRecurrenceEvidence2.registrySnapshot/registryEvolution | Snapshot2/可空 Proof2，完整相同 scheduling cut；历史 evidence/Witness/Step 不重新签名。 |
| D6-CONTROL document-format、registry/period/bootstrap proof；D10-CONTROL §12 bootstrap 消费 | 真实 Snapshot2/Proof2 及完整资格，不根据文件或 plugin 缺失推断空表。 |

实际 D10 activation 仍是 Image1/Pin1 的 activation arm，不是只承载 automation/run 的 Image2。它只在 supplement.registrySnapshot.version=2 及对应 Proof2 处选择新嵌套域；D10-Control-Record/1 pin prefix 保持，完整 pinned canonical bytes/digest 覆盖 version 和绑定表。逐槽递归分派，不因一个 Snapshot2 放宽整个外层。旧 before/initial/checkpoint/image/pin 各自保留准确原版本；一条有证据的显式演进链可以跨1→2，不能把 registryReads 同次读取不得混版的规则误套到整条历史链。

## 5. 格式生产、接收与渲染

Profile2/Binding2 精确按 AUTHORING §7。AsciiDocProcessorEnvironment4 完整继承 D2 Environment3 的成员，version=4，documentProfile 增加 weftext_managed/2；其它 environment 成员不变。新 managed2 evaluation 必须等于已验证 Binding2.profile，包括原 languageBaseline。DocumentFormatCurrentQualification2 继承原 /1 全部成员，version=2，binding 为严格 Binding1|Binding2，componentImage.version 仍等于 bindingRevision，绝不能写入 decoder 的1/2。ManagedDocumentSemanticQualification2 保持原 sourceObservation/documentFormat 成员，documentFormat 使用 Qualification2。真实旧 Qualification1/Semantic1/Environment3 永远按原类型。

D2 evaluation/snapshot 的 processor-environment 槽显式接受 Environment4，由内层 version 分派；旧环境 pin 不重编码。D3-SCHEMAS/D6-SCHEMAS 的 format qualification、D6-CONTROL §0.1.2 component decoder/§0.1.3 receiver admission 与 document_format dependency 都接上述精确类型。receiver 按实际 binding.version=1或2 选择闭合 decoder，未知版本/错误 profile 拒绝；portable_metadata pin 精确为 D3-CJ/3(binding)，原 CP4 component rank、Notice3、ChangeRecord1、P seal 与完整关联验证不变。managed2 缺 binding/registry 仍 incomplete，不能凭 YAML 可读自授 admission。

D9DocumentRenderBinding1 的新 current semanticQualification 槽采用 SemanticQualification2，准确 D2 snapshot pin 消费 Environment4；以其 documentFormat.version=2 与 snapshot.environment.version=4 选择新 rendering profile，不篡改旧 ExportPlan4 中已冻结的 Semantic1/Environment3。新 Plan/confirmation 的输入并集与摘要完整覆盖新内层对象；prepared 以后不得转 profile。exact-source/resource/query-json 的 null render binding 路径不受影响，真实旧 Plan/receipt/recovery 保持。

## 6. YAML ID 的构造与原 C/Q

原 Result9/C/Q 闭集不接受额外 YAML tag。本候选新增 Core 内部 ManagedEnvelopeConstruction1，作为完整源构造的具名中间表示，不能直接作为原 Result9 字节提交，不能被 worker 或文件中的 UUID 自行认证。它不解决原全局“symbolic Document 输入”P1：接入其正式 D3 提交 framing 仍须和该问题联合完成独立验收。在该依赖完成前，新建/template/import 的 managed2 完整端到端状态明确为设计依赖未关闭，不因以下算法或空 C/Q 正例宣称可实现启动。已有身份的属性编辑不依赖新 ID 构造。

```text
ManagedEnvelopeConstruction/1 = {
  kind:"managed_envelope_construction",version:1,
  subject:PayloadSubjectKey,
  preimagePin:PinRef/2,
  envelopeSite:{kind:"existing_id",range:D2SourceRange,rawScalar:text}
              |{kind:"new_envelope",byteOffset:Counter,eol:"LF"|"CRLF"},
  declaredTypes:[text...]
}
```

仅 Core 由真实输入构造：existing_id 用于已验证 copy/template 前像；new_envelope 在可选 BOM 后唯一插入，前像是准确 ID-free AsciiDoc skeleton，尚不是合法 managed Document。declaredTypes 来自实际获权类型选择/Recipe，按 same-cut schema 校验；非法或含另一 envelope 的输入不能以 new_envelope 绕过验证。该中间表示及原 pin 受原 operation 预算/权限/保留约束，不产生公开预分配 UUID。

正式生产阶段使用原 D3 stage12 私有 candidate map，subject 解析到同一唯一 NodeId。existing_id 只替换确切 UUID scalar，保留引号、注释、BOM/EOL；new_envelope 使用确定的最小 envelope、每个 type 的安全引用编码及所冻结 EOL。完整重解析证明 id==candidate subject==birth/file binding。先计算准确 envelope edit/displacement，再把原 C/Q pinned 前像范围按完整 SourceTransform 映射到中间/最终源；两遍 C/Q、同一 revision token 和 Q 最终位置验证保留。afterPin 覆盖0到EOF全部字节；新 format binding 与 source/metadata 在唯一 winning planning CAS/P seal 内一致冻结安装。任一步不一致整体失败；planned recovery 复用原 frozen bytes/map/UUID，不重抽或从当前源重算位置。

真实 managed1 的普通 copy/fork 保持 Profile1/Binding1 与无 YAML 的原源；显式 copy+format-conversion 预览才使用 fresh subject 及上述 envelope，Binding2.bindingRevision=1。相同身份转换 checked-increment 原 bindingRevision；源改变必须产生真实 sourceChanges。restore 始终按原历史版本。不得用 profile-only/sourceChanges 空数组掩盖插入 envelope。

## 7. 集成验收与剩余边界

以下 INT-01–06 是待实施的设计案例，产品 UNRUN；每项正常、拒绝、旧记录恢复均需审查。

| 编号 | 必须覆盖 |
| --- | --- |
| INT-01 | 已有合法 managed2 task，无 body read 的 Field-only status open→done：wire3 selection、Spec3、真实 image3、同一提交、D8读；隐藏 substitution/ABA/撤权拒绝。 |
| INT-02 | 多个同名 set/unset 的 remove 不显露前值；无损两向转换；两个同值复杂事实不合并，无法保全 note/正文引用时拒绝。 |
| INT-03 | Recipe3 field_set native status 覆盖默认值；field_append 遇 header 冲突；新实例 ID 构造仍依 §6 未关闭的原全局 framing。 |
| INT-04 | 真实 Snapshot1→2 明确演进，完整 bootstrap/activation/proof/recurrence 消费；缺 version/绑定表、混合新旧读取、错误 digest 拒绝。 |
| INT-05 | Binding2 原 pin→seal→第二副本 admission→Qualification2/Environment4→D2/D9；unknown version拒绝，真 Binding1 原恢复和普通复制不偷偷转换。 |
| INT-06 | envelope 改变全文 offset 后 C/Q 和最终 afterPin 一致；candidate/ID mismatch 或部分 metadata 不安装；真实 saved/planned/unknown 不重编码。 |
