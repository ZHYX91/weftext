---
source_language: zh-CN
translation_status: source
---

[English](CORE.md)

# Typst Core：存储、身份、事务与授权编译

状态：本轮协调设计已完成独立 Sol 复核，未实现、未全局接受。输入 `af6737a057c7bb69763dfaad0a57da4652e085f3`；本候选只定义新目标，不表示旧原型已经迁移。产品、文件系统条件安装、同步、崩溃、密码学、Server、性能、包隔离和 UI 验收均 **UNRUN**。本文的类型标识是协议名，不是作者必须手写的对象。

## 1. 唯一格式与库存

新目标只有 `X/X.typ` 一份 managed Document。根 `.weftext-format` 的全部字节必须恰为 ASCII `weftext.typst.v1` 加一个 LF，即 `weftext.typst.v1\n`；无 BOM、CR、空格或额外行。它选择本候选 profile，不证明 Workspace、Node birth、文件绑定或权限。无标记、错误标记、未知 profile 分别按普通未管理内容或显式 incompatible/incomplete 诊断处理，不能猜格式或改写标记。

Core 是唯一库存 authority。目录树决定父子关系；路径是 locator，ID 是 lowercase UUIDv4；不持久化 parent/path/name/child-path 列表。自然名称顺序和父所有排序策略、子稀疏 manual rank 保留。根 `.weftext-rules` 是独立 portable authority，不继承 `.gitignore`；unmanaged 目录是完整不可重入子树，没有 UUID/envelope；ignored 内容仍属于备份字节。一个 managed Node 的同名 `.typ` 不能单独分类。非 canonical 普通文件是资源，仅 managed owner boundary 内才可派生 owner UUID。独立批注仍用 `weftext.annotations.json`，不增加节点内 metadata 目录。SQLite/index/graph/backlinks/thumbnail 都可重建，不成为内容源。

文件管理器的 create、rename、move、copy、trash、restore、purge、资源 replacement 和 Server 中的同名动作均调用同一个 Core plan/transaction。结构目标在 ignored/unmanaged 内，或被操作 managed subtree 包含此边界时，Core 拒绝；不由 shell 私下执行移动再补元数据。canonical containment、symlink/reparse/硬链接别名、大小写及跨卷 rename 由受信后端证明；不能证明唯一对象与边界则 installation unavailable。旧 root AGENTS 的 .adoc 与 `weftext.asciidoc.v1\n` 描述现有原型，未被本设计改成已实现 `.typ`。旧资料转入必须显式导入、保留输入和损失预览，不能仅改扩展名、隐式双主源或启动时转换。

## 2. 同一 profile 的闭合身份

```text
TypstProfileCarrier/1 = {
  kind:"weftext_typst_profile",version:1,
  profileId:"weftext.typst.v1",documentLanguage:"typst",
  languageBaseline:"0.15.1",rootMarker:"weftext.typst.v1\n",
  sourceContract:"typst_source/1"
}
TypstDocumentFormatBinding/1 = {
  kind:"weftext_typst_document_format",version:1,
  ownerNodeRef:NodeRef,bindingRevision:Counter,
  profileCarrier:TypstProfileCarrier/1
}
TypstDocumentQualification/1 = {
  kind:"weftext_typst_document_qualification",version:1,
  sourceObservation:SourceObservation/1,
  documentFormat:{key:DocumentFormatDependencyKey/1,
    stamp:{epoch:Token,revision:Counter},
    componentImage:ComponentImage/1,
    binding:TypstDocumentFormatBinding/1,bindingPin:PinRef/2}
}
```

所有成员必需，unknown/duplicate/missing/null 拒绝。Counter、Ref、Token、Pin、canonical JSON 都用既有 owner 的精确 decoder；D3-CJ/3 编码完整新对象，包含 carrier，而不是删除新字段后获得旧摘要。bindingRevision 从1开始；格式真实改变 checked +1；restore 用历史准确 binding；新 identity 的 create/copy/fork 为1。component 是既有 document_format key，present bytes 恰为完整 binding 的 D3-CJ/3，componentImage.version 等于 bindingRevision，pin 绑定同 NodeRef、用途和准确全部字节。Qualification 的 observation.entityRef、binding.ownerNodeRef、key.ownerNodeRef 同一；key.workspaceRef 等于完整 NodeRef 的 Workspace；present image、stamp、pin、同 cut observation 都完整核验。

新生产者在库存→源读取→Field/Body projection→Query/编辑→prepare→pin→install→seal→receiver→render/export 的每一步都使用此准确 carrier。新 receiver 根据已认证 Workspace 根 profile 与准确 component binding.kind/version 选择新 decoder；原 ManagedDocumentFormatBinding1/2 decoder 不接受 Typst 常量。root marker、文件名、包声明、调用方 carrier 或编译 metadata query 不能单独选 decoder。现有相同 source bytes/version 但 format stamp/binding 改变仍使旧 proof stale。旧 profile 恢复不能因为当前根选择 Typst 而改绑。

## 3. 控制声明不代替 birth 和授权

[SOURCE](SOURCE.zh-CN.md) 拥有 TypstRootDeclaration/1、原生 Field/Schema 规则。本文消费它的 authenticated_structure | invalid_control | incomplete_source，只把 authenticated_structure 视为准确 root 作者结构证据。唯一根作者 `#metadata((id: ..., types: ...)) <weftext>`、SOURCE规定的允许前缀、保护的标准函数绑定、完整根保留标签排歧及 direct literal ID/types 均须成立；标题可以在声明之前，不能由Core擅自改成“首表达式”限制。include/import/template/生成结果不能提供身份。控制声明有效后还必须验证原 Node birth、Workspace/NodeRef、lifecycle、file-object binding、完整 portable history、format qualification 和当前 Policy。改 ID、重复 ID、复制带 ID 的外部文件、partial sync 缺 metadata 都不获得新 birth，不能自动重新分配 UUID。

新 ID 仅由原 D3 stage12 单一私有 candidate map 生成；mapped/new 使用该 map，existing 使用已认证原身份。prepare 不公开 fresh ID；复制/模板/导入对所有 Node/Resource/Annotation 和 references 用同一 map，普通复制不能借保留旧 ID 偷渡身份。same-identity restore/repair/显式转换按原身份和准确历史证明；不以 path/hash/相同字符串转移身份。types 不是 Field、业务 setter 不修改 id/types；Facet/type change 仍需 node-control/Source 与完整 schema gate。

```text
TypstControlConstruction/1 = {
  kind:"typst_control_construction",version:1,
  subject:PayloadSubjectKey,sourcePin:PinRef/2,
  declarationRange:{start:Counter,end:Counter},
  idRange:null|{start:Counter,end:Counter},
  typesRange:null|{start:Counter,end:Counter},
  declaredTypes:[text...]
}
TypstDocumentConstruction/1 = {
  kind:"typst_document_construction",version:1,
  profileCarrier:TypstProfileCarrier/1,subject:PayloadSubjectKey,
  sourcePin:PinRef/2,control:TypstControlConstruction/1,
  transferPlanPin:null|PinRef/2
}
```

这两个对象只由 Core 从 SOURCE 官方 AST、准确 skeleton/source pin 和同 cut 获权 Recipe/Facet selection 构造，不接受 caller offset/UUID/信任标志。subject 只能是 Node existing/mapped/new，sourcePin 为该 subject 的完整 UTF-8 Document 前像。range 为该官方 AST 的 byte 半开区间，BOM/EOL/注释无损；已有 declaration 正宽且 idRange 指唯一 direct string token，typesRange 指完整 direct tuple/array；缺 types 时 null，仅由验证后的完整 dictionary 插入方案处理。fresh skeleton 的 declarationRange 可为 BOM 后唯一零宽插入点且两个值 range 都 null；非 fresh 不能据缺声明自动新建身份。声明插入不改变原正文分类。declaredTypes 顺序及作者短名保留规则由 SOURCE 唯一拥有，验证后的 effective closure 由 schema owner 给出。

stage12 后物化只用原同一 map；control patch 与 transferPlanPin 中 FEATURES 具名 TypstDefinitionTransferPlan/1 的 AST reference 改写合并成无重叠、准确 byte edits，冲突整体拒绝。声明或 reference 变长后重算最终官方 AST 和位置。原 AsciiDoc Result10 H、单行 JSON Q 的 framing 不用于新 Document。需要最终生产地址的 transfer 使用同一 SourceRevisionPlan/1、同一拟议 revision token 做原“先物化，后验证最终地址”的两遍：第一遍构造，第二遍对完整最终源验证所有 typed addresses/范围和 self-reference；不能以旧 line/span 约束替代。control 对 map/birth、types 对 schema、最终声明唯一性、完整 typed coverage、未声明字节及 AST 分类不变全部证明后才得到完整 afterPin。CAS 冻结 construction、transfer pin、最终 bytes、map、version basis 与 token；恢复不再解析 current 文件生成替代方案。

## 4. 复用通用事务；只替换语言耦合槽

D6-CONTROL §§1–9、15、18–20 和 D3 §§4、10–15 的唯一算法继续拥有 DecisionKey2、CommitDomain2、ChangeId1、Frontier2、SourceVersion2、SourceObservation1、PinRef2、SourceRevisionPlan1/2/3、RevisionTokenBinding2、签名 artifact、P ledger、安装、恢复、错误顺序。d6_source_save_prepare wire2、d6_commit_request wire2、Notice3/CP4/ChangeRecord1、PortableComponentKey2、OwnerInputBinding2 均保持精确 generic 形状；不增加 parser 名、carrier 字段、Typst encoding 或另一个提交入口。唯一新增的通用 dependency 族为下述 compile_world 所需4代，不重定义其它15种依赖。source-save 对新 profile 通过 SOURCE validation，body 编译错误不自动等于 source syntax/control invalid；普通保存允许明确 semantic_pending，完整保存仍核所有实际 obligations。strict/observed-only 的权限、单 existing live Document、明确受信人工选择和禁止降级规则不扩大。

```text
CompileWorldDependencyKey/1 = {
  kind:"compile_world",workspaceRef:WorkspaceRef,ownerNodeRef:NodeRef,
  target:"html"|"paged",principalAudienceToken:Token
}
DependencyKey/4 = DependencyKey/3 的原十五臂 | CompileWorldDependencyKey/1(rank15)
DependencyProof/4 = DependencyProof/3 逐成员原样，version=4，entries[].key=DependencyKey/4
InputDescriptor/4 = InputDescriptor/3 逐成员原样，version=4，controlInputs[].key=DependencyKey/4
PreparedIntent/4 = PreparedIntent/3 逐成员原样，version=4，
  inputDescriptor=InputDescriptor/4，dependencyProof=DependencyProof/4
```

这四个新闭合successor保留原kind字面量，planToken受保护tag为 d6_plan/4；不在原3代decoder增加第16臂。新候选所有新prepare统一4代，同holder不可混3/4；未消费body/World的静态Field/普通source路径可以没有compile_world项，不因此被迫编译或读取正文。prepare wire2的request无内嵌proof/descriptor，最终commit wire2只取受保护token，因此无需机械改wire。actual current槽是 D3 identity envelope.inputDescriptor、PAB adapter.dependencyProof/observationProof、EDITOR prepared binding.inputDescriptor/dependencyProof、D9 export input、D10 workspace_reads.dependencyProof/source保存holder以及它们的PreparedIntent backing；ownerInput/pinDirectory/preview仍原generic decoder。

语言耦合 closed holder 使用一个新 nominal adapter；以下是完整的 schema 定义方法，不是向旧类型添加未知成员：

```text
FormatAdapterEnvelope/1 = {
  kind:"weftext_format_adapter",version:1,
  role:Role,profileCarrier:TypstProfileCarrier/1,payload:Payload(Role)
}
Role = identity_input | identity_request | proposed_input |
       action_input | prepared_action | effect_manifest | effect_bytes |
       edit_input | prepared_edit | recipe | construction_input |
       render_binding | export_plan | bootstrap_plan |
       workspace_reads | control_effect_plan | recurrence_evidence |
       activation_image | author_step
```

Payload(Role) 分两种明示定义：表列“原schema逐槽替代”的角色严格复制原fixed-input成员集合和适用矩阵，原内层kind/version字面量保留为新nominal payload结构标识，不交给旧decoder；表列“完整新payload”的角色直接采用具名新owner的完整closed type，不同时套旧成员集合。Envelope kind/version/role是新nominal类型分派，carrier必须与所有source format qualifications相同。逐槽替代表外成员及optional/null/排序/权限约束原样；没有反射扫描、任意JSON、递归“发现版本”、generic any或自行增加Role。所有嵌套表中命名的语言耦合类型用相应envelope，generic atoms始终原类型。此有限替代避免第二事务authority，也避免“retained”隐去格式槽。

| Role / 原准确 schema owner | 本候选 payload 的准确替代路径 |
| --- | --- |
| identity_input / D3-SCHEMAS §6 D3IdentityInput14（AUTHORING-INTEGRATION §6.1） | 意图中的文档构造字节引用TypstDocumentConstruction1；其余原意图、模式及权威矩阵不变。ownerKind/intention精确为typst_identity/1，不能把作者构造误当旧文档字节载体。 |
| identity_request / 同处 D3IdentityOperationRequest14 | inputDescriptor 为4；intent 同上；preparationBinding仍恰为原 `{kind:"d7_preparation_binding",bindingToken}`，只把token受保护backing选新prepared_action，不内嵌该对象或造成循环；完整outer envelope bytes构成fingerprint，receipt/error选择envelope domain，不冒充wire14。 |
| proposed_input / D7ProposedInput3 | `{subject,payloadKind,encoding,pin}`；文档编码仅exact_source_utf8或typst_document_construction1；资源仅resource_bytes；批注仅typst_annotation_value1或typst_annotation_construction1。此为全部合法组合，新载体禁止全部旧Result9/10，批注新构造按§4.1通过同一身份映射产生完整结果，不能交给旧解码器。 |
| action_input / D7ActionInput3；prepared_action / PAB4 | action为FEATURES.TypstActionSpec1，canonicalCallInputs逐项完整采用FEATURES.TypstQueryCall1，不保留旧TypedLiteral/参数域。QueryRef、inline/saved定义、参数类型/default、arguments及context.now等真实typed槽按FEATURES闭合TypedLiteral/ParameterSpec/native Query族选择，不是仅改两个地址槽。definitionInputs为TypstDefinitionInput1，registryInputs为SOURCE.TypstSchemaQualification1；构造、提议、请求、完整效果按相应角色选择。依赖证明4代，原观察/预算/期限保持。 |
| effect_manifest / EffectManifest3；effect_bytes / EffectBytes3 | 精确采用§4.1闭合效果及编码矩阵；保留原完整集合、phase/披露门，不用摘要或construction前像冒充committed after。 |
| edit_input / D8EditInput3；prepared_edit / PreparedEditBinding3 | 完整新payload分别为 EDITOR TypstEditInput1、TypstPreparedEditBinding1，不复制旧D8 member集合；其inputDescriptor/dependencyProof/preparedIntent为4，preview为effect_manifest envelope，sourceInputs原型不变。原request不假造preparedIntent的未知member：最终d6_commit_request2由其受保护planToken/operationId和current fence按原owner生成，受保护prepare record保存准确request bytes。禁止旧D8 map/locator判写资格。 |
| recipe / Recipe3；construction_input / TemplateConstructionInput3（AUTHORING-INTEGRATION §2） | 完整新payload分别为FEATURES TypstRecipe1与TypstTemplateConstructionInput1；identity构造选本文对象；完整同cut schema/environment及原执行/冻结顺序保持。 |
| render_binding / D9DocumentRenderBinding1；export_plan / ExportPlan4 | render binding 选 FEATURES TypstExportBinding1，精确 binding/source snapshot/environment/world 与 route map；plan 的相应 rendering/source/Recipe/definition/schema 槽应用此表。原 route/预算/损失确认/pins/worker/proposal/下载和 receipt 恢复语义不变。 |
| bootstrap_plan / WorkspaceBootstrapPlan4 | targetRegistry的snapshot/binding精确选SOURCE新快照/绑定；profile.registrySeedBinding也选TypstSchemaBinding1。完整获权seed及digest、目标标记/布局/源格式效果由原custody计划冻结。删除initialPresentationPolicy；双信任、初始权限、创建者与系列/时段规则保持§7。 |
| workspace_reads / D10WorkspaceReadDependencies2 | registryReads[i]的snapshot/binding分别选SOURCE新快照/绑定，snapshotPin仍准确全量pin；sourceInputs原样，dependencyProof为4。 |
| control_effect_plan / D10ControlEffectPlan2 | registryChange.before.snapshot 为真实 recorded schema，after.snapshot 为 TypstSchemaSnapshot1，evolution 为 SOURCE 定义完整显式演进；changes 仅 activation/author language 槽应用本表，其余 control images 原样。 |
| recurrence_evidence / ScheduleRecurrenceEvidence2 | payload.dependencyProof精确为DependencyProof4；registrySnapshot/registryEvolution采用SOURCE新类型，其余observation/sourcePin/metadataPin/recurrenceContext逐成员保持原定义。当前生产、步骤前后证明及全部祖先的准确槽位、引用和历史分派见§4.3。 |
| activation_image / D10ControlRecordImage1 activation arm；author_step / D10AuthorStepResponsibility2 | 补充资料的注册表/演进、请求、准备绑定、输入描述及呈现资格采用表中对应适配器。其他十九类控制投影、执行声明、费用、停止和外部同意保持原职责与权限。 |

新 OwnerInputBinding/2.canonicalDescriptorBytes 精确为完整相应 envelope 的 D3-CJ/3；ownerKind=intentKind 分别为 typst_identity/1、typst_action/1、typst_edit/1、typst_bootstrap/1 或原 control intent（其 descriptor 包含具名 adapter 槽）。D6 处理 action/edit 的既有 protocolOwner=D7/D8，D3 identity 的既有 protocolOwner=D3，最终决议仍只有原 D3|D6 owner 互斥。pinRefs 为所有 proposed、schema、world、transfer、preview 与原必需证据 pins 的准确排序唯一 union；PreparedIntent4.pinDirectory 再包含 DependencyProof4/SourceObservation/installationPlan 全部真实 pins。preparation token 的受保护 backing record 类型由 envelope 选择，tag/audience/expiry 不给新权限。previewBinding 是原类型，但其 owner preview pin 现在绑定完整 envelope；after bytes/format/manifest/route/schema 不得另选 profile。

CollectionCall的准确原形是原execution-action owner §7的call/nodeColumn二成员；本profile的action.collection.call（collection_create臂）及其他真实CollectionCall位置逐项选FEATURES.TypstQueryCall1，nodeColumn仍准确终端列名，ParentPolicy保持原闭合域。canonicalCallInputs、action内调用和prepared_action对应成员使用同一完整native调用，不能保留第二份旧arguments/context。Recipe/TemplateConstructionInput/DefinitionTransferPlan/DefinitionTransferEffects的参数、值、资源、损失、源subject和typed effect slot完整形状只由FEATURES §§6–7定义；本文引用其完整新payload，不以旧Recipe或SOURCE Field定义自行补出参数类型，也不复制第二份effect decoder。

在本表和被引用的新 owner 完整 decoder 下，新实例有正向 prepare→原 planning CAS→原唯一 P→原 receipt/publication 路径；这不是 owner_update_required 的永久占位。未知 adapter role/新 owner version 或缺完整证据才 unavailable。确实未触及语言的 Resource、structure、trust、Money、stop 操作仍用原 generic current 类型，无需包装。

原生Query请求、页面与View选择FEATURES完整TypstQueryResult1/TypstQueryType1/TypstQueryV1/TypstTerminalSchema1/TypstViewValue1；旧D7TypeSpec/V闭合集不扩。通用result handle/cursor/epoch/cut/authorization/预算生命周期仍唯一Core producer，新的结果只能由已获权完整native Query生成。render_binding/export_plan的resultPins槽对新结果采用下述完整记录，viewSpecs采用TypstViewValue1；旧结果仍选原真实D7ResultPin及ViewSpec1，不能混页或按字段名猜profile。准确query_json选择FEATURES具名weftext.typst-query-result-export1全量schema/data，静态字段结果不因导出被迫整篇编译或读正文。

```text
TypstResultPin/1 = {
 kind:"typst_result_pin",version:1,profileCarrier:TypstProfileCarrier/1,
 resultToken:Token,producerEpoch:Token,principalAudienceToken:Token,
 authorizationGeneration:Token,frontier:Frontier/2,
 schemaPin:PinRef/2,dataPin:PinRef/2,dependencyProof:DependencyProof/4
}
```

schemaPin恰绑定完整TypstTerminalSchema1，dataPin按该schema准确解码完整rows/scalar/graph结果，保持bag、order与所有native精确值。受保护resultToken选择同一producerEpoch和完整Query输入/cut/pins；记录与当前授权及所有正负依赖逐项对应，不是调用方新权限。renderer/export消费此记录和配对View，不能重跑Query、补查、丢null/array项或近似浮点/日期。失效、错profile或缺完整pin整体拒绝；旧pin/domain保存恢复不换成此型。

### 4.1 完整效果与Annotation构造

effect_manifest payload严格保持EffectManifest3全部成员，items改为下表穷尽闭集。effect_bytes payload严格保持EffectBytes3的 {handleToken,encoding,byteLength}，encoding只有下列矩阵；byteLength/所选epoch/phase/pin全部绑定完整准确字节。源码图像沿原SourceImage2三臂和D7ProposedVersion1精确结构，只把bytes选此新effect_bytes；生产/拟议版本与subject/map和SourceRevisionPlan必须相等。

| rank / kind | 精确新payload与编码/phase |
| --- | --- |
| 0 source_change | 原kind/subject/before/after成员；已存在文档的完整前像和已提交后像只用exact_source_utf8，资源只用resource_bytes，批注只用typst_annotation_value1。预览拟议节点/批注可分别采用相应新构造编码；构造前像不成为已存在来源，不能以其字节冒充最终完整已提交正文。 |
| 1 conditional_source_change | 原完整before/result/versionBasis/forceAdmission结构；仅preview、existing Node、完整typst_document_construction1且实际有transfer引起条件源状态分支才适用。final两遍后真no-op省committed项，否则恰一真实source_change；不接受旧C/H。 |
| 2 entity_state_change | 原完整EntityImage与实际subject结构/lifecycle矩阵不变。 |
| 3 d3_plan / d3_receipt | 原phase singleton，request指identity_request envelope；receipt为该完整原request domain的真实byte-equivalent结果，不换回旧wire。 |
| 4 semantic_extension | 仅已提交阶段，格式与编码严格对应下述完整矩阵，次序为1/2/3。旧复杂事实关系复制分支在本格式退休，不以空的虚构效果补位；其他真实来源与定义传递仍完整披露其准确计划和结果。 |
| 5 period_scope_change；6 series_configuration_change | 原全量binding/config矩阵和排序不变；完整SeriesScope在身份物化后按原规范排序。 |
| 7 workspace_bootstrap | 预览仅typst_symbolic_bootstrap1，已提交仅typst_bootstrap_plan1；各自选择下述完整工作区初始化载体，不截断权限、注册表、配置或目标保管信息。 |
| 8 authority_change | 原完整AuthorityImage及preview pending/committed真实authority规则不变。 |
| 9 field_change | 原owner/field/beforeVersion/afterVersion/before/after结构；两个bytes仅typst_field_image1，精确解码下述完整TypstFieldImage1，不再读取Entry UUID。仅owner_fields。 |
| 10 conflict_branch_source；11 conflict_resolution_change；12 canonical_plan | 原全量heads/canonical-plan/元数据规则不变；每个真实source bytes及Node/Annotation结果采用rank0对应准确编码，plan中的原language payload pin选proposed_input envelope，绝不漏branch。 |

相同rank二级比较器逐字复用原effects owner §4；rank13 presentation_policy_change新目标退休，其余不重新编号。full和owner_fields互斥：owner_fields只含全部实际授权Field的rank9，必须事前证明覆盖全部作者效果，未selected source/其它Field不向caller披露；raw bytes/value/state真no-op时items=[]及原sourceVersions保持，external→managed同字节admission不是no-op。source_change.before为真实物理前像，不用selected历史head代替。preview/committed refine只有原条件分支许可，不能加未预览效果。

```text
format -> encoding (committed only; secondary rank)
typst_source_materialization_effects/1 -> typst_source_materialization_effects1 (1)
typst_definition_transfer_effects/1 -> typst_definition_transfer_effects1 (2)
typst_canonical_effects/1 -> typst_canonical_effects1 (3)
```

```text
TypstFieldImage/1 = {
  kind:"typst_field_image",version:1,profileCarrier:TypstProfileCarrier/1,
  ownerNodeRef:NodeRef,fieldId:FieldId,
  sourceVersion:SourceVersion/2|D7ProposedVersion/1,
  field:SOURCE.TypstSelectedField/1
}
TypstSourceMaterializationEffects/1 = {
  kind:"typst_source_materialization_effects",version:1,decisionKey:DecisionKey/2,
  owners:[{ownerNodeRef:NodeRef,before:SourceVersion/2,after:SourceVersion/2,
    beforePin:PinRef/2,afterPin:PinRef/2,
    fields:[{fieldId:FieldId,beforeImagePin:PinRef/2,afterImagePin:PinRef/2}...],
    transformEvidencePin:null|PinRef/2}...]
}
```

SelectedField是全量准确该Field的状态/native value/rawFieldSource，无绝对范围/ASTpath或其它字段位置；内部完整Projection仍受保护。图像FieldId/version各自与field_change及真实source对应，不截断array、重排同值项或重序列化raw。物化effects数组按完整owner/FieldId排序唯一，包含每个实际materializer owner（内部no-op owner可before=after且fields=[]，不伪造公共源效果）；计划/结果与原同决议receipt/CP4/afterPin一一匹配，完整source evidence只交付full授权。FEATURES §6是TypstDefinitionTransferEffects1唯一owner，rank4编码逐字采用其完整closed transfers数组，不另定义同名plan/results对象。typst_canonical_effects1精确是原D3CanonicalEffects1成员集的nominal新域，仅plan/source payload/effect bytes递归应用本§闭合映射；原requestFingerprint/receipt/source-version/full-origin规范保留，不扩旧decoder。

TypstSourceMaterializationEffects1仅适用真实existing源writer；fresh/copy/template的目标前像是absent，完整Field初值及constraint gate由TypstDocumentConstruction1和完整source_change的fresh版本依据覆盖，不生成该existing-only extension，也不伪造before SourceVersion/Observation。existing Field本身absent由SelectedField.absent图像表达，beforeImagePin仍绑定该准确图像；fresh Node无需beforeFieldImage。新annotation construction编码只适用protocolOwner=D3的同map identity construction；D6已有Annotation修改只用完整concrete Value1。

### 4.2 D10闭合祖先的实际选择

下列是§4角色表的具名完整祖先替代，不向原ControlDependencies3等闭合类型塞envelope：

```text
ControlDependencies/4 = ControlDependencies/3全部成员，version=4，
 workspaceReads:Option<FormatAdapterEnvelope/1(role=workspace_reads)>
ControlPrepareBinding/4 = ControlPrepareBinding/3全部成员，version=4，
 dependencyPins:ControlDependencies/4
D10ControlInput/3 = D10ControlInput/2全部成员，version=3，
 dependencies:ControlDependencies/4,
 effectPlan:FormatAdapterEnvelope/1(role=control_effect_plan)
```

旧outer只保留原decoder。新控制prepare的protected backing是Binding4、canonicalIntentBytes精确绑定Input3及其闭合dependencies/effectPlan；OwnerInputBinding2使用原D10 owner但ownerKind/intention为d10_control/3，canonicalDescriptorBytes精确完整Input3；originalCommitRequest仍原PreparedCommitRequest1，唯一commit/确认/stop/claims/Money/恢复算法不改。D10VersionedControlPrepareBinding的新nominal包装只允许 {schema:"d10_control_prepare_binding/4",value:ControlPrepareBinding4} 或原真实1/2/3 arm，不扩原Versioned1 decoder。

控制记录的语言相关祖先统一选下述具名新载体，不能虚构扁平成员。载体内的来源规范替代按**静态类型**定义，不按运行时成员名扫描；原配置、绑定、用量、范围、投影、归属、时间、停止和费用槽保持。没有语言输入的记录可以保留原真实版本。完整引用字节必须与该记录对应，未知角色或错误包装直接拒绝。

```text
D10FormatControlCarrier/1={kind:"d10_format_control_carrier",version:1,
 profileCarrier:TypstProfileCarrier/1,role,payload:S(Base(role))}
role -> Base(role)
image -> D10ControlRecordImage/1|/2
pin -> D10ControlRecordPin/1|/2
range -> D10ControlRange/2
claims -> D10ExecutionClaims/2
author_step -> D10AuthorStepResponsibility/2
subscription -> ScheduleSubscription/2
occurrence -> AutomationOccurrenceRecord/2
schedule_witness -> ScheduleContinuityWitness/2
schedule_step -> ScheduleContinuityStep/2
schedule_invalidation -> ScheduleContinuityInvalidation/2
S(RecordImage activation arm).supplement.registrySnapshot/evolution -> SOURCE new types
S(RecordImage run arm).supplement.authorSteps[] -> carrier(author_step)
S(RecordImage automation arm).supplement.subscription -> carrier(subscription)
S(RecordPin).image -> carrier(image)
S(ControlRange occurrences arm).records[] -> carrier(occurrence)
S(ControlRange records arm).members[] -> unchanged ControlRef (not record bytes)
S(ScheduleSourceBinding recurrence arm).initial/checkpoint -> recurrence_evidence envelope
S(ScheduleOccurrenceProof recurrence arm).evidence -> recurrence_evidence envelope
S(Witness/Step/Invalidation): every statically typed ScheduleRecurrenceEvidence/2 slot -> recurrence_evidence envelope
S(Claims).recordPins/prepareBindings/authorSteps/subscriptions/occurrenceRecords/ranges ->
 carrier(pin)/new VersionedBinding4/carrier(author_step)/carrier(subscription)/carrier(occurrence)/carrier(range)
S(AuthorStep interactive arm).authorRequest.request -> identity_request envelope|original commit2
S(AuthorStep interactive arm).preparedFormat -> "typst_prepared_action1"|"typst_prepared_edit1"
S(AuthorStep core_field_member arm).link -> nominal original Link2 with new prepared_action backing
```

该有限类型代换也作用于上表基型内的ScheduleSourceBinding、ScheduleOccurrenceProof，以及本节声明的记录包装祖先，表外成员和合法值完全不变。新ControlDependencies4.recordPins/controlRanges、ControlEffectPlan载体的changes/recordPins、ControlPrepareBinding4准确预览/声明引用、Inventory相同typed证据槽明确选择上述新载体或真实旧臂。新版包装schema精确为d10_format_control_<role>/1，value为对应完整carrier；不得增加到旧Versioned1闭合集。连续性工件使用D6-Typst-Schedule-<Witness|Step|Invalidation>/1加NUL和完整carrier字节，仍在原容量/生产者/永久gap与同P事务内；原工件不重编码。

该有限typed-schema替代在新carrier域内一次定义全部祖先，旧持久Record/Pin prefixes与signature bytes原样；新carrier完整canonical bytes包含profile/role，使用前缀 D10-Format-Control/1 + NUL，不能使用旧Record/1前缀冒充旧图像。除真正选择这些语言槽的caller外，不需更换Control storage；这里只解决输入类型封闭，不给包/插件control授权。

typst_symbolic_bootstrap1精确 `{kind:"typst_symbolic_bootstrap",version:1,template,slots}`，template为完整bootstrap_plan envelope，只有其新decoder真实typed fresh NodeRef位置允许 {kind:"symbolic_subject",subject:NodeSubject}，slots恰完整 {path,subject} 数组。原literal/Workspace/authority/handles/Policy不得替换。path使用原member-UTF8/Counter-index比较器，禁止duplicate/ancestor overlap/missing/extra；同map物化后仅initialSeriesConfigurations/periodScopeBindings按原实际数组比较器排序，再完整验证新Plan无initialPresentationPolicy。committed typst_bootstrap_plan1 bytes恰完整物化envelope。preview不成为mutation/evidence，不公开预分配ID。

typst_annotation_construction1精确解码 TypstAnnotationConstruction/1={kind:"typst_annotation_construction",version:1,profileCarrier,subject,template,refSlots,revisionSlots,bodyReferencePlanPin}。subject只Annotation existing/mapped/new；template为EDITOR完整TypstAnnotationValue1的nominal symbolic型：仅target/replyTo/locator中的真实typed Ref位置允许symbolic_subject，其余Value成员及actor/time冻结规则完全不变。refSlots恰每个替换的 {path,subject}；revisionSlots恰locator生产revisionToken位置的 {path,sourceSubject}，值是Core拟议token placeholder，不允许任意Literal改写。bodyReferencePlanPin可null，否则绑定同source typed inline link字符串的官方AST byte编辑计划：完整before body pin、每个准确string-token range/原decode address/targetSubject，禁止任意源码片段；Core同map编码整个安全原生string，保持其它body字节并完整inline重解析。typed path与subject entity kind必须匹配，sourceSubject有同plan SourceRevisionPlan/token，locator仍经SOURCE最终坐标验证。Core只从获权复制前像或已验证create/edit proposal与受信actor/time产生此对象；caller不能以模板赋予attribution。

该constructor在同stage12 map改写新AnnotationRef、target/reply及inline typed链接，同一拟议token/两遍最终坐标验证后产完整Value1，最后同P保存aggregate/sourceVersion/token/receipt；现有edit无需symbolic即用准确concreteValue1。它给copy包含fresh target/reply正向路径而不扩大旧Result9的Value4 decoder；旧Result9仅真实记录恢复。

### 4.3 当前调度证明的完整链（TR-08）

下式是recurrence_evidence完整新名义payload，不再按旧ScheduleRecurrenceEvidence2的Proof3槽解码。外层是§4完整FormatAdapterEnvelope1，role=recurrence_evidence；inner kind/version只保留结构标识，由新外层decoder选择新payload。实际成员名是dependencyProof，没有proof别名。

```text
Payload(recurrence_evidence) = {
 kind:"d10_schedule_recurrence_evidence",version:2,
 observation:SourceObservation/1,sourcePin:PinRef/2,metadataPin:PinRef/2,
 dependencyProof:DependencyProof/4,
 registrySnapshot:SOURCE.TypstSchemaSnapshot/1,
 registryEvolution:null|SOURCE.TypstSchemaEvolution/1,
 recurrenceContext:RecurrenceReadContext/1
}
CurrentRecurrenceEvidence = FormatAdapterEnvelope/1(role=recurrence_evidence)
S(ScheduleContinuityWitness/2).initial/checkpoint = CurrentRecurrenceEvidence
S(ScheduleContinuityStep/2).before/after = CurrentRecurrenceEvidence
S(ScheduleContinuityStep/2).dependencyBefore/dependencyAfter = DependencyProof/4
S(ScheduleContinuityInvalidation/2).evidencePins = [PinRef/2...]
```

Invalidation保持原完整成员集合，没有dependencyProof/before/after，不为整齐而补造。其evidencePins保存导致binding_changed/gap的真实证据；引用新证据/步骤时按新域解码。Witness/Step其他字段、Step.portableChanges真实ChangeRecord1/Notice3/CP4三pin项、retainedInputs及原状态机不变。旧2代Witness/Step的Proof3 decoder不扩。

| 当前实际producer/consumer槽 | 完整新类型与交叉验证 |
| --- | --- |
| 配置注册→ControlDependencies4.workspaceReads | workspace_reads的dependencyProof4与同次D10ControlInput3.dependencies、ControlPrepareBinding4.dependencyPins、InputDescriptor4/PreparedIntent4一致；实际源观察进入sourceInputs，proof条目/stamp与controlInputs对应。注册仍原配置事务，不凭文件缺失创建Witness。 |
| ScheduleSubscription.source的recurrence臂 | initial/checkpoint均CurrentRecurrenceEvidence，订阅由carrier(subscription)保存。initial属注册真实cut，checkpoint属最后连续验证cut，不把两个历史cut重写相等。 |
| carrier(schedule_witness).payload | initial/checkpoint选上述完整证据；首次revision1/transition0/fresh producerEpoch原样。只有原注册的source/control transition producer能更新checkpoint。 |
| carrier(schedule_step).payload | before/after为新完整证据；dependencyBefore逐字等于before.payload.dependencyProof，dependencyAfter等于after.payload.dependencyProof。两者各绑定真实前后cut、Workspace/CommitDomain、源观察、metadata、schema/演进、规则/权限及完整正负范围，不重采样证明。 |
| AutomationOccurrenceRecord.proof的recurrence臂 | evidence为CurrentRecurrenceEvidence；外层ScheduleOccurrenceProof不是DependencyProof。完整projection/originalStart/due/key保持原规范；carrier(occurrence)经§4.2包装进入ControlRange occurrences.records[]。 |
| carrier(claims).payload | subscriptions/occurrenceRecords/ranges选择相应当前载体；保留全部连续性artifact和实际recordPin。automation.supplement.subscription、run.supplement.authorSteps及Inventory准确证据按§4.2，不在claims假造扁平dependencyProof。 |
| D10ControlInput3→作者/控制prepare→唯一P | dependencies严格ControlDependencies4，effectPlan为本profile封装；descriptor、预览、protected backing、pinDirectory全链generic4。最终仍原commit2，注册、subscription/producer关联、retention和记录同一原事务，无调度专用ledger。 |

每份sourcePin绑定observation的完整准确源字节，metadataPin绑定同源版本的实际受保护元数据；schema/evolution与真实registry依赖及完整pins相同。Proof4覆盖实际source、document_format、registry、规则和authorization读取；仅实际消费World才含compile_world。不含该项不能借环境资源支持编译，包含该项不能降Proof3通过旧槽。空/未知schema、partial metadata、gap或无权不是连续step。

retainedInputs、invalidation evidencePins及consumer pinDirectory按完整PinRef排序去重收集：before/after sourcePin/metadataPin、Proof4全部evidencePin、schema及存在的evolution pin、实际规则/授权/格式/World证据、portableChanges三pin、原continuity/preview/proposal/recovery引用。去重不能丢artifact或只留摘要，更不能以current文件补原before。两个Proof4由准确完整canonical payload及protected pins保留，不另放自由对象。artifact按§4.2具名prefix+NUL+完整carrier核kind/role/profile/长度/摘要；假旧域或错误prefix拒绝。原4096pin/16MiB证据元数据、独立source PinBudget、checked推进和reference-safe保留不变；下一步无法保全时先同事务永久gap/invalidation，随后无关source工作才可继续。

真实旧Subscription/Witness/Step/Invalidation（原1/2域、Proof2/3）先按recorded decoder/pins恢复，不装入CurrentRecurrenceEvidence或重编码4。原D6 §7.2.1及D10-CONTROL §0.2限定的Subscription1→2显式continue与mixed retained chain只保留其原producer、initial/checkpoint、continuityPins和原artifact domain；原条款要求完整retained history且无格式/规则/业务断裂，不能凭本文扩成新的3→4迁移桥。Typst格式/binding改变属于真实continuity断裂，原control transition必须拒绝同generation继续，或经原显式replace/new-generation路径重新注册当前4代证据。版本升级本身不修改真实旧chain，缺旧历史也不视为空。旧saved/planned/unknown与已实际存在的旧继续关联按原操作履约；本候选不创建新桥、重生producer或复活失效generation。

TR-07批注完整保存资格由EDITOR §8.1唯一拥有。Core消费固定complete/strict，沿pinned intent→edit_input封装→InputDescriptor4.saveProfile、PreparedIntent4安装保护→原commit2重验。ordinary批注不进入新plan，Draft保留；Document普通保存保持原矩阵。

## 5. 准确 source、Frontier、map 和 pins

SourceVersion2 的 managed/external 两臂仍保持原字段：managed 的 producing commitDomain/observationEpoch/revision/changeId 与 external 的 producing commitDomain/observationEpoch/externalSequence 不混用。当前 SourceObservation1 独立保存 observerDomain、entityRef、完整生产 sourceVersion、当前 observationEpoch、FileObjectBinding1 和 evidencePins。observerDomain 必须是本次 operation CommitDomain，生产域可不同；watcher gap、ABA、external/object replacement、不可证明 rematerialization 使当前观察失效，即使 bytes/digest 相同。SourceVersionRef1 token 仍选择完整受保护 observation，不是版本号码。

Frontier2 只有 {kind,version,heads}；Workspace 逐项在 heads[i].commitDomain.workspaceRef 验证，**没有** frontier.workspaceRef 或 heads[i].workspaceRef。空 heads 也需外层 descriptor/proof 的 Workspace/CommitDomain、当前 cut 和连续已证明空历史。exact managed_atomic 与所有请求规定的完整 Frontier equality 保留；scope_dependencies 只接受完整连续、非回退且与原全部正负范围无关的 sealed 扩展，不能以 vector 数字或 index completeness替代。

普通/fresh SourceRevisionPlan1、D3 canonical-conflict-only2、D6 source-conflict3 的适用矩阵保留。H(D,E) 从同生产域已封存历史证明，缺历史不是H=0；真实 source change 使用 checked H+1、fresh complete-empty 为1、跨 observer epoch不重置。true raw no-op、source-unchanged control/structure、deletion 不造 after managed revision。external equal-byte admission 是真实 change。唯一 winning planning CAS 同时冻结 exact source afterPin、before observation/原 conflict wrapper、lastIssued/empty-history、map、stamp、token、format/schema/world/transfer和安装计划；seal 时才分配同一次 ChangeId，将 stamp 合为 SourceVersion 并签原 RevisionTokenSealArtifact1/SourceTransformSealArtifact1，保留原双 trust profile及真实 before whole-source摘要。prepared/token/span 不因此成为 current persistent地址。

## 6. 官方编译环境和 World 资源授权

```text
TypstProcessorEnvironment/1 = {
  kind:"weftext_typst_environment",version:1,
  profileCarrier:TypstProfileCarrier/1,
  compilerBuildSha256:Digest,standardLibrarySha256:Digest,
  syntaxBuildSha256:Digest,projectionBridgeBuildSha256:Digest,
  target:"html"|"paged",
  date:null|text,timezone:"UTC",
  packageLockPin:PinRef/2,fontCatalogPin:PinRef/2,
  resourcePolicyPin:PinRef/2,budget:TypstCompileBudget/1
}
TypstCompileBudget/1 = {
  sourceBytes:Counter,totalResourceBytes:Counter,resourceCount:Counter,
  importDepth:Counter,decodedPixels:Counter,fontBytes:Counter,
  pluginFuel:Counter,pluginMemoryBytes:Counter,
  workerMemoryBytes:Counter,wallTimeMillis:Counter,outputBytes:Counter
}
TypstWorldProof/1 = {
  kind:"weftext_typst_world_proof",version:1,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,frontier:Frontier/2,
  environmentPin:PinRef/2,authorizationProofPin:PinRef/2,
  reads:[TypstWorldRead/1...],
  complete: true
}
TypstWorldRead/1 =
 {kind:"workspace_source",request:text,outcome:"present"|"absent",
  observation:null|SourceObservation/1,bytesPin:null|PinRef/2,
  resolutionEvidencePin:PinRef/2}
 |{kind:"environment_asset",request:text,outcome:"present"|"absent",
   assetKind:"package"|"font"|"plugin",artifactPin:null|PinRef/2,
   manifestPin:PinRef/2,resolutionEvidencePin:PinRef/2}
```

Digest 是 sha256: 加64 lowercase hex。环境由获准精确 Rust compiler/syntax/stdlib build 和部署资源提供者生成，实际 digest 不能为空或猜测；语言 baseline 只是相同 profile 的最低精确语言版本，build/target/date/fonts/packages/资源政策都是真实环境依赖。date 为 null 禁止系统日期读取；非 null 是冻结 ISO Gregorian UTC 日期，由宿主在授权/clock cut 提供，不调用宿主当地时区。budget 每项 >0，具体数值由部署策略/调用预算的较严上界冻结；累计checked，无溢出。不能取消执行的 compiler/plugin 在具备硬 CPU/wall/memory kill 及无权限文件系统的 worker 中运行；超过预算时没有成功 projection/compile，终止 worker保留 Draft/源及诊断。包和字体锁完整 manifest bytes/digest 由各自 typed pin绑定，禁止latest、机器环境字体或下载结果自动加入。

Core 创建 snapshot World，而不是给编译器 Workspace mount。include/import/read/json/yaml/csv/xml/image/bibliography 以及任何格式的数据加载统一经过 World resolver；JSON/YAML 合法原生功能不等于路径授权。先主体与最低entity披露，再对应 source_read/resource_read、locator/envelope 等真正需要能力及 deny，再 canonical resolution 与获权 bytes；其它 Node主源要求其独立source_read，跨 ACL资源要求资源owner grant，read已公开主源不能顺带读取私有子源。不能证明必要权限时统一不泄密失败，不能返回空文件、缓存片段或不同“存在/不存在”错误。

reads 的 request 是 canonical logical请求（不披露宿主 absolute路径）。workspace_source present 必须有完整准确 observation+bytesPin，absent必须两者null并由resolutionEvidencePin证明获权目录/namespace完整负范围及trusted absent对象。environment_asset不伪造Node/Resource身份或SourceObservation：present的artifactPin为管理员获准lock中的准确bytes及digest，manifestPin完整证明namespace/version/字体face或plugin条目；absent的artifactPin为null，manifestPin和resolutionEvidencePin证明已获权lock完整负范围。每项manifest/evidence仍受独立资源准入与当前authorization门。日期只取冻结environment.date，不进入reads。仅文件不存在、placeholder、offline、metadata缺失或I miss不是absent。每个实际尝试，包括错误分支、候选package解析、字体fallback/缺glyph选择和负查找，都在受保护trace中记录，按(kind固定rank、request、assetKind) canonical bytes排序唯一；重复请求不同结果是conflict。未实际完整追踪不能 complete=true，也不能给窄写/strong Action证明。被拒绝的 trace保留仅作诊断/审计，不向无权caller暴露路径或资源计数。

绝对宿主路径、..逃逸、symlink/reparse/hardlink绕界、未获权unmanaged/ignored目录读取、环境变量、任意宿主fs/network一律无 World出口。普通编译不下载包；package resolver仅消费管理员已准入且digest固定的本地 lock。package字体/plugin间接读取仍过同 World。WASM plugin只有显式参数和受限memory/fuel，不能打开host fs/network、访问secret或发起Core动作；“WASM”不是Core capability。需要外部数据的connector仍由D10原独立认证/grant/费用/确认/stop/unknown责任管线获取并提交普通资源提案，再由本World显式读取。包可合法计算，但不能调用Query全库或write命令。

WorldProof作为§4新 compile_world key的精确evidence payload，并同时用原source/authorization/registry/format keys证明实际workspace资源；不塞原execution_resource decoder。producer是Core真实World observation manager：在同一 observerDomain、ownerNodeRef、target、principalAudience及cut完整构造environment/authorized source+asset正负trace，然后保存WorldProof pin及其protected stamp。epoch是此manager的真实观察incarnation，revision随此key绑定的任何正负资源、lock、font fallback、date/环境、source或authorization变化checked前进；gap/无法连续追踪使旧stamp不可用，不能在重建后补同stamp。stamp选择完整proof，不是caller摘要；evidencePins准确覆盖proof/environment/all reads/manifest/authorization证据。consumer在prepare、preview交付、render/export、最终read barrier/P及receiver本地新读取复验全proof与当前资源，变化则stale/reprepare；source/static未使用World的操作无需此key。saved/planned恢复留原proof与pins；失权暂停，缺资源/完整trace不重新采样替代。保存字面Field只需SOURCE静态证据，不为了compiler权限先读整篇hidden body。

窄Field写的额外披露能力是精确新 `{kind:"typst_dependency_state",fieldId:FieldId}`。Policy/5 精确继承D6-CONTROL §10/10.1 Policy3的全部成员/grants/deny语义，仅version=5、capability union增加此臂；不继承旧Policy4的AsciiDoc native_dependency_state作为Typst权限。此能力仅workspace scope、当前schema精确FieldId，默认不授予，不由Field read/write、source_envelope_state、source_read、admin或包能力隐含；必须经原policy_admin事务显式安装。其披露只有该Field聚合隔离资格，不允许正文、拓扑、计数、路径或其它值。无grant在读取native dependency拓扑前统一not_visible；有grant仍需原Field read/write/source-write deny/类型与静态完整约束和实际官方before/after use/正负World无外部影响证明，未知或多来源拒绝保留Draft。完整Source授权路径保留。

因旧Bundle2.policy闭合为Policy3/4，新 WorkspaceAuthorizationBundle/3 精确继承Bundle2成员，version=3，policy严格为Policy3或Policy5，其trustRoot/trustRevision/trustDeclarations及mixed Declaration1-prefix/Declaration2-suffix规则不改。新current policy component bytes用Bundle3，完整canonical pin/摘要包含version与Policy5；旧Bundle2 decoder不扩。精确consumer槽为bootstrap_plan.initialPolicy/target policy component、authorization dependency实际policy evidence、D10控制权限/issuer/bootstrap initial grant及trust管理/replica/conflict的selected/result policy bundle和它们的head/carry evidence pins。新bootstrap可Policy3且没有窄grant；Bundle3所有activation/head/carry证据明确bundleVersion=3而completion仍CP4/ChangeRecord1，用新nominal adapter域保存，不塞旧bundleVersion1/2 closed arm。声明/PoP/root签名仍按原Declaration2完整域，activation从同DecisionKey真实CP4 policy after-image重派生；换bundle version不重签真实旧声明或改历史cut。真实saved/planned的旧Policy/Bundle/证据按原记录恢复，不因缺此grant重新执行。

### 6.1 Bundle3的准确信任consumer

本节是有限的完整successor定义，取代上段“新nominal域”的概述；不新增adapter Role、不放宽旧head/hop。

```text
PolicyBundleHeadEvidence/4 = PolicyBundleHeadEvidence/3
 | {head:ChangeId/1,completionProofVersion:4,bundleVersion:3}
TrustConflictCarryValidationHop/3 = TrustConflictCarryValidationHop/2
 | {declarationVersion:2,declarationRevision:Counter,
    declarationDigest:Digest,activationChangeId:ChangeId/1,
    completionProofVersion:4,bundleVersion:3,
    changeRecordPin:PinRef/2,completionProofPin:PinRef/2,policyBundlePin:PinRef/2}
TrustConflictCarryValidationEvidence/3 = {
 head:ChangeId/1,factId:Digest,origin:TrustConflictCarryValidationHop/3,
 carriers:[TrustConflictCarryValidationHop/3...]
}
ConflictResolutionPolicyDerivedPlan/3 = Plan/2完整成员与原约束，
 headEvidence:[PolicyBundleHeadEvidence/4...],
 selectedBundleVersion:1|2|3,carryEvidence:[TrustConflictCarryValidationEvidence/3...],
 resultBundleVersion:3
ConflictResolutionInput/4 = Input/3完整成员与原约束，version=4，
 resolution.policy:Policy/3|Policy/5,derivedPlan:ConflictResolutionPolicyDerivedPlan/3
ConflictResolutionPreview/3 = Preview/2完整成员与原约束，version=3，
 resolution.policy:Policy/3|Policy/5,derivedPlan:ConflictResolutionPolicyDerivedPlan/3
```

Plan、Input、Preview的准确原schema是D3/D6-SCHEMAS §9、D6-CONTROL §0.1.4/§9.4及D6 §17.1，不借相同短名导入别的owner。导入old arm只走原exact decoder；新head/hop只接受CP4+ChangeRecord1+Bundle3、真实Declaration2/root/PoP签名及首次追加该声明的原policy after-image。完整直接origin与每个实际carrier的cut/pins保留，不能把origin换resolver时间或只保selected安全branch；完整fold/outcomes/compromise继承算法原样。

精确槽：独立Plan3.headEvidence/carryEvidence；Input4.derivedPlan与Preview3.derivedPlan（不假造outer carryEvidence）；每个origin/carriers[j]按该项真实completionProofVersion/bundleVersion逐项解码。branchEvidence.policyBundlePin仍PinRef2，但其相应headEvidence声明的owner版本选择完整Bundle1/2/3字节，不由pin递归搜索猜类型；selectedBundlePin/resultBundlePin分别对应Plan3显式version。OwnerInputBinding2.ownerKind=intentKind为 d6_conflict_resolution/4，canonicalDescriptorBytes恰D3-CJ/3(Input4)；PreviewBinding选完整Preview3及其pin，PreparedIntent4.pinDirectory收集所有branch/selected/result和逐origin/carrier真实pins，按原union规则，不删真实旧证据。source_merge/choose_source_head不使用这些policy-only新类型，保持原ownerInput而仅generic descriptor/proof/prepared4。

新bootstrap、trust add/rotate/revoke、replica registration/continuation及D10 custody读取真实policy组件按bundle.version精确选Bundle3；新trust声明对应的activation证据为上述4/3 arm，原Declaration2签名/双profile顺序不变。新权限管理入口精确为下述对象，不能冒充不存在的旧公开policy-set请求。它要求当前policy_admin与完整旧授权世代、预期revision/Frontier，ownerKind=intentKind为typst_policy/1，canonicalDescriptorBytes恰完整请求；sourceInputs=[]，原policy/authorization依赖、before/after Bundle3 pins和准确预览都由同一prepare冻结。新conflict policy prepare是D6-CONTROL §9.4原request的wireVersion3，仅policy槽换Policy3|5；完整原成员/OperationId/expectedKey保留，产生Input4而不改旧Input3。两者最终仍原commit wire2和单P。

```text
TypstPolicyPrepare/1 = {
 wireVersion:1,kind:"typst_policy_prepare",workspaceRef:WorkspaceRef,
 commitDomain:CommitDomain/2,operationId:Uuid,expectedFrontier:Frontier/2,
 expectedAuthorizationRevision:Counter,expectedPolicyRevision:Counter,
 policy:Policy/3|Policy/5,budget:BudgetBinding/1
}
```

新target始于Bundle3；若确有真实旧Bundle1/2 history要被新target采用，必须是已授权显式导入/格式转入intent并保留原chain，Bundle3安装是checked authorizationRevision后继，不默默把同值旧bundle升级。new result3来源须在该plan的完整after pin中冻结，只有实际追加声明才推进trustRevision。旧saved/planned/unknown以及旧before.policy4、CP3/4/5各自按真实arm保留；普通reader不把Policy4 grant转给Typst，也不创建迁移决议。

bootstrap seed的生产者是受权issuer配置准入，不从SOURCE的自然语言说明、metadata类型名或文件缺失生成Registry。它提供SOURCE完整TypstSchemaSnapshot1/Binding1/Qualification1及所有真实pins，creator grant按这个准确已准入seed的实际FieldId生成；Core在目标真实empty/custody证明与唯一bootstrap计划内安装准确seed bytes。预置contacts/tasks使用SOURCE具名完整seed constructor，用户可经同一schema-admin演进安装其它定义；缺实际seed/权限/完整evolution拒绝bootstrap，不把未知表当空。fork保留并验证完整source schema history，不能冒充create只装默认seed。

## 7. 编译投影、撤权、缓存与恢复

```text
TypstSourceEvaluation/1 = {
  kind:"weftext_typst_evaluation",version:1,
  qualification:TypstDocumentQualification/1,
  environment:TypstProcessorEnvironment/1,
  control:TypstRootDeclaration/1,fields:TypstFieldProjection/1,
  body:TypstBodyProjection/1
}
TypstSourceSnapshot/1 = {
  kind:"weftext_typst_source_snapshot",version:1,
  evaluation:TypstSourceEvaluation/1,exactSourcePin:PinRef/2,
  evaluationPin:PinRef/2
}
TypstBodyProjection/1 =
 {state:"available",target:"html"|"paged",worldProof:TypstWorldProof/1,
  compilationEvidencePin:PinRef/2,semanticProjectionPin:PinRef/2,
  originEvidencePin:PinRef/2,outputPin:PinRef/2}
 |{state:"unavailable",reason:"source_error"|"resource_unavailable"|
   "unsupported_projection"|"budget_exceeded",diagnosticPin:PinRef/2}
TypstCompilationEvidence/1 = {
  kind:"typst_compilation_evidence",version:1,
  sourcePin:PinRef/2,environmentPin:PinRef/2,worldProofPin:PinRef/2,
  semanticProjectionPin:PinRef/2,originEvidencePin:PinRef/2,
  outputPin:PinRef/2
}
```

官方编译器生成内容、元素语义、阅读顺序和来源位置；语义投影是EDITOR具名 TypstSemanticProjection/1 的完整规范工件。官方Content仅在受控进程内使用，不序列化成不存在的官方wire，也不把内存对象冒充PinRef字节。准确编译证据记录上述全部真实pins，证明它们来自同一次Core官方求值/realization和最终收敛introspector轮；projectionBridgeBuildSha256绑定EDITOR规定的官方hook桥实际代码，独立一致性场景验证show/循环/重绑定/表格/标题来源。flow-semantic是呈现后端，不是虚构官方Target：它使用官方Paged target；html-experimental用官方Html target。合法源但流式阅读不能准确呈现的内容仍允许源码/分页预览，并报告该呈现能力不可用；普通阅读/编辑范围仍需EDITOR验收。投影不是改造后的AST、同文字匹配或另一Typst解释器。正文不可用时不以旧HTML/PDF伪装当前，不阻断合法静态声明/字段或稿件留存；诊断仅披露获权范围。生成、导入、多来源或show改变输出不能自动变直接可写。

evaluationPin绑定完整 evaluation；exactSourcePin绑定qualification的准确source与whole bytes；body各pin及WorldProof属于同一environment/qualification/cut。编译结果不决定birth、Field写权限、保存或发布成功。Snapshot缓存key包括完整source production/current observation、profile binding/stamp、build/target/date、全package/font/resource policy、全部正负依赖和principal authorization generation；不允许以源hash作为缓存完整key。撤权/deny/ACL变更先在Core/Server线性化，随之使受影响snapshot、query/index/cache、editor map与下载资格失效。任何缓存取用、render输出/HTML/图片/缩略图/搜索片段/下载/prepare或final P之前都检查current主体和真实依赖；已缓存获权资源不让别的主体继承权限。撤权后的响应不能继续发旧正文或资源bytes；撤权先于seal则paused_authorization，seal先发生则原committed事实保留，交付仍按当前披露。

重建index/recompile重新取得当前观察、完整WorldProof和权限，不复活旧selector/Draft/map/preparation。包、字体、负范围或环境变化使真实consumer stale；不影响未消费这些资源的普通源保存。backup保留原字节/portable authority；restore必须按真实历史decoder与admission重新验证。同步仅运输原source/format/metadata/Notice/CP/ChangeRecord及原outbox，partial arrival不完成资格；receiver重新建立自己的观察与资源资格，不复制发送者token或编译授权。

新 Typst 的段内标题及作者呈现控件按 EDITOR 明确修改同 `.typ` 中 target-scoped 原生 show rule，走同一 Draft→准确 source patch→普通源 prepare/Notice3/CP4/ChangeRecord1→P→同步/恢复。设备阅读偏好仅布局，不成为同步作者 authority。新目标具名退休独立 presentation_policy_change current effect、presentation Set producer 和 bootstrap initialPresentationPolicy：不生产独立 sidecar/head/outbox，也不为新 Workspace 合成旧 presentation owner。effect_manifest 新 payload 的 EffectItem 集合删除这一 arm；其余 rank/phase/完整集合门原样，删除不会对原其它 effects 重新编号。新bootstrap仍把root/Registry/Policy/两trust keys/所有source-format/series/period及activation同一P完成，但不产生CP5 owner record。真正已有的原presentation policy saved/planned/unknown与Notice4/CP5/ChangeRecord2、head/receipt/outbox按原decoder恢复；这是历史履约，不是第二个新呈现写入口。

## 8. 真实历史与精确旧→新路径

恢复先通过原最低披露、原record适用的authorization/custody/domain/P连续性门；按原DecisionKey+完整canonical fingerprint+protocolOwner定位，然后saved交付/发布、planned恢复原冻结plan、unknown保留原责任；只有unseen选择新profile和adapter。当前source/新schema/新World未就绪不能让已saved变成重执行；same key不同fingerprint仍operation_id_conflict。不可从current bytes重造old r5、missing P、receipt、token、Money或external结果。真正CP2 committed/restored保持原12-kind component union和SourceVersionRef1，禁止document_format或升级重编码；缺Policy/trust head只按原恢复路径，不拿当前head补旧证据。

本文不宣称任一未部署设计版本已存在runtime资料，不自动建立永久old/new桥接或迁移数据库。有证明真实saved/planned/unknown记录的版本才承担其原decoder/bytes/TTL/last-reference与no-duplicate-effect责任；没有这种证据的旧候选留作只读设计输入。新设计可以不部署旧候选；真实旧数据导入必须是新explicit intent。

| 旧具体owner与section | 本候选 disposition/唯一新位置 |
| --- | --- |
| root AGENTS Storage；D1 §§3,5–7,9,12 | product `.adoc`仍原型；目标inventory用§1；单holder/五surface/Server fence/普通sync/non-disclosure/里程碑完整保留§§4–8，不扩大平台发布。 |
| AUTHORING-INTEGRATION §§5,6,6.1；D3-SCHEMAS/D6-SCHEMAS §5 | AsciiDoc Profile/Binding/Environment/H生产具名替换为§§2,3,6,7；旧类型不扩。 |
| D3 §§4.7,11.1,14–15,21.2；D3-SCHEMAS §§6–8 | 生产/current版本、候选map、exact Frontier、pins/transfer保持§§3–5；语言input/action/Result槽用§4 adapter；Annotation新类型由EDITOR定义并在proposed/effects/receiver选其decoder，旧Value4不改。 |
| D6-CONTROL §§0.1.1–3,3–9,15,17.8；D6 §§4–12 | generic schema/唯一事务与错误顺序保持§§4,5,8；format bytes consumer具名换§2 qualification。 |
| D6-CONTROL §0.1.4,10.2,20；D10-CONTROL §12；D3/D6-SCHEMAS §9 | 新presentation owner effect/Set/initialPresentationPolicy具名退休§7，旧真实记录仍恢复；bootstrap Registry/格式槽换§4；root/dual trust/PoP/activation/compromise/no partial prefix原样。 |
| D6-CONTROL §§18–19；D6 §§15–17；D3-SCHEMAS §8 | 原准确before摘要、total mapping、两profile签名/outbox保持§5；新AST/transfer来源换§3，不把旧坐标当新坐标。 |
| D10-CONTROL §§0.1–0.3,4,6–8,10–12,14,16；D3-SCHEMAS §10 | claims/Money/approval/external unknown/stop/custody保持原owner；真实workspaceReads/registryChange/activation/recurrence/author-step语言嵌套槽只按§4映射；编译包不授动作§6。 |

## 9. 原21影响和新验收

原21关闭只属于原输入，不能作为Typst通过。下列编号前缀均为 A2-GLOBAL-6B13-；本文为Core相关重新打开的设计义务，非作者自关闭。

| 原项 | Typst处置及新场景 |
| --- | --- |
| P1-01,P2-14 | 新Document控制/完整内容/typed transfer的同map物化，§3、TC-01/02；旧H/Q不移植。 |
| P1-02,P2-15 | 独立presentation current路径具名退休、原生show同source commit替代§7、TC-03；原其余混合effects rank规范排序及真实旧owner恢复保留。 |
| P2-01,P2-03,P2-12 | 新adapter exact定义传递、真实Frontier嵌套/source binding、§§4–5、TC-02/04。 |
| P2-02,P2-07 | 原真实历史/CP2两臂与故障责任保留§8、TC-05。 |
| P2-08,P2-09 | 双trust genesis、mixed声明签名域/完整activation保留§§5,7、TC-06。 |
| P2-10 | 上述逐owner/逐槽替代及未变义务，TC-07核完整source-map，不以一句retained替代。 |
| P1-03,P1-04,P2-05,P2-06 | SOURCE/EDITOR拥有原生heading/完整locator/float/table替代；Core只供相同profile/cut/环境，TC-08核交叉consumer。 |
| P1-05,P1-06,P2-11,P2-13 | FEATURES/EDITOR拥有Trash只读search/saved search binding/audio-video annotation/quote隐藏边界；Core保持lifecycle/授权/typed pin读取，TC-09核撤权与消费链。 |

| 新ID（全部产品UNRUN） | 正向与必须拒绝的反例 |
| --- | --- |
| TC-01 | create/copy/template含正文、重复联系方式、Node/Resource/Annotation及reference，唯一map/CAS/P，完整after；重复根声明、ID/map不等、缺birth/format、partial metadata、重抽UUID拒绝。 |
| TC-02 | control插入使全文offset改变，native definition/reference两遍与最终sourcePin一致；self-reference、重叠patch、混profile、旧H/单行Q猜测、漏typed transfer拒绝。 |
| TC-03 | run-in控件对准确target-scoped native show一次source commit/同步/恢复；新bootstrap无initialPresentationPolicy/CP5 owner，source/series/dualtrust/activation同P；真实旧独立owner零component及mixed历史恢复、P前及P内失败无committed态，P后只重发原bytes。 |
| TC-04 | exact各真实nested Frontier Workspace，proved-empty、不同producer/observer、external equal-byte admission、format-only stale；假顶层字段、H空历史猜测、scope extension hole/正负范围变化、ABA拒绝。 |
| TC-05 | 真实saved r5/current r6、planned、unknown、CP2 committed/restored、历史Policy缺head；只恢复原operation/decoder/pins，不重签/补head/收费；第三态和无P不能伪造terminal。 |
| TC-06 | 新target两个staged keys、两Declaration2/PoP/root签名、同ChangeId激活；losing compromise branch完整fold、continuation双profile；单prefix、缺key、旧签名域、重建root拒绝。 |
| TC-07 | producer→consumer固定槽逐项carrier/pin/schema一致；false RootSource扁平字段、错版本、漏完整binding/digest、混old/newholder拒绝。 |
| TC-08 | include/read/json/yaml/package/font/plugin、跨ACL及负依赖均有完整WorldProof；hidden存在探测、symlink/path逃逸、缓存另主体、包隐式下载、missing字体fallback、预算超限拒绝且Draft保留。 |
| TC-09 | revoke与prepare/render/download/P竞速线性化，缓存与derived index重建不复活旧proof；合法body error仍读root静态Field，不能用旧预览作当前；隐藏quote/Trash写/重复执行拒绝。 |
| TC-10（TR-08/PF-03） | 同次已获权配置注册的InputDescriptor4/Proof4/PreparedIntent4→subscription.initial/checkpoint→witness→准确step前后Proof4→occurrence.proof.evidence→claims→D10ControlInput3完整正例；缺source/schema/格式pin、envelope装Proof3、Step证明与before/after不等、遗漏continuity或portableChanges pin、错误artifact域、unknown/gap均拒绝。真实旧Proof3、原artifact及原已获证mixed链在saved/planned/unknown原样恢复；Typst格式断裂只走原控制transition拒绝continue或replace新generation，不发明3→4桥或洗成同generation。 |
| TC-11（TR-07/PF-04） | EDITOR complete+strict批注正文编辑/重新挂接/重确认及hidden-target reject沿真实封装→generic4→原commit2/P，建议apply仍原双entity原子链；ordinary或observed_only批注在prepare前拒绝留Draft，不给caller's annotation flag降级；Document普通save路径仍可用。 |

设计文档检查仅证明文档一致性；这些场景需要真实官方语言、条件安装、crash注入、两副本、Server ACL和硬资源隔离实现后才能产生产品证据。
