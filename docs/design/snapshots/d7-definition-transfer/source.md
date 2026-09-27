---
_weftext:
  id: "4d94859f-3586-4b26-bf82-79db2e1eecf6"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 保存定义的typed transfer — revision06-draft

此合同属于D7 portable payload的原子源变换，须由同包D3 wire11/Result9显式承接。D2仍将保存块视为inert；D3仍拥有identity mapping、Result materialization、原decision和receipt；D7只证明自身closed payload中哪些成员是typed引用及其唯一变换。不能把body中看起来像UUID的文字替换，也不新增Query实体。

## 1. 唯一typed引用遍历

按完整D7 schema解码SavedQueryDefinition、ViewSpec、DynamicBlock后，以静态schema路径遍历：DefinitionAddress.owner及其已识别D3 Locator；TypedLiteral.type指明的node/resource/annotation_ref值（递归Optional/object/list/union）；CollectionCreationPolicy.parent；保存View/Query调用地址；Query Selector literals、QueryRef arguments、parameter defaults、固定View Domain TypedLiteral及DynamicBlock literal/context bindings。普通CEL expression字符串、labels、任意普通text和lexical this bindings不在遍历内；即使其字符串含UUID也逐字保持。不能递归扫描未知JSON并猜ref。

每个typed Ref都用原D3完整decoder，且按原slot/owner/locality门判断。来自identity map内的Ref必须完整改为对应fresh Ref；同Workspace map外的完整Ref按显式preserve策略保持；跨Workspace map外Ref不能继续作为当前Workspace typed Ref，整操作拒绝cross_workspace_identity_preservation，不静默删默认值、替换none或把它改成text。source map遗漏不靠owner/path猜。Resource/Annotation映射包含其完整owner；new owner不相容拒绝，不能只换leaf UUID。this.node仍lexical，在目标Node实际执行时解析，不在copy中固化旧NodeRef。

TypedLiteral中的普通object仅由其TypeSpec声明的Ref成员受此规则；一个像Locator的普通object不会仅凭成员名被升级为可解析Locator。D7真实DefinitionAddress.at.locator才接受下段Locator重发。D4 authored provenance的复制继续由D4/D3原typed carrier合同处理；不能由D7的普通object去补签其ActionEvidence或重新解释历史原值。

## 2. 源、结果与显式slot清单

D3 wire11的plan在原九数组之外增加第十数组`definitionTransfers`，完整元素exact：`{kind:"d7_definition_transfer",sourceContainer,inputOccurrence,resultContainer,resultOccurrence,payloadFormat,slots}`。

sourceContainer为existing_entity_subject(NodeRef)或artifact_part_subject；inputOccurrence为非负Counter，指完整已绑定D2 source/artifact part中按source order的SavedQueryViewDefinition ordinal；resultContainer为existing/mapped/new Node subject；resultOccurrence为结果完整D2源同类ordinal。payloadFormat只能weftext.saved-query/weftext.view/weftext.dynamic-block；未知format不能成为本adapter输入。一般copy/fork不得借用ordinary导入转换旧QuerySpec：其源须是当前完整已验证payload。

slots按唯一typed-path comparator排序且路径唯一：逐segment比较，member text的tag rank=0并按原UTF8 bytes比较，Counter index的tag rank=1并按D3Integer数值比较；共同prefix后短路径在前。禁止用整条path的D3-CJ/3序列化字节排序；索引2必须先于10，元素exact `{path,before,after}`。path是`member name text|Counter index`数组，仅在完整schema遍历找到的typed Ref/DefinitionAddress Locator根合法，禁止祖先/子孙重叠；before为原D3 Ref或原完整DefinitionAddress Locator（原decoder确认variant）；after为`{kind:"map",subject:PayloadSubjectKey}`、`{kind:"preserve"}`或`{kind:"reissue_locator",ownerSubject:PayloadSubjectKey,targetOccurrence:Counter}`。map只能mapped/new同kind subject；preserve必须满足上段同Workspace外部保留；reissue_locator只接原DefinitionAddress DocumentElementLocator，并绑定其当前精确目标SavedQueryViewDefinition occurrence，owner映射机械确定。全payload全部typed Ref/Locator必须恰有一项，不能只列需要改变的引用。locator root已包含owner，禁止另列其owner嵌套路径。anchor地址的owner是普通Ref slot，anchor name原样保留并在目标验证唯一解析。

definitionTransfers按`subjectCanonicalKey(resultContainer),resultOccurrence`排序，result pair唯一，source pair可在显式独立copy映射中多次出现但仍逐一绑定各result。其全部成员进入原D3 canonical request/fingerprint。完整source payload不是slots声称的对象：Core从原preimage/source_artifact payloadBinding对应的完整source pin按inputOccurrence独立恢复，再由D7 schema重新枚举并与slots精确比对；result同样由完整结果独立恢复，缺/多slot、before不等、wrong type、owner互换均stage14 identity_map_incomplete。无Ref的合法saved payload也须有一条slots=[] transfer，证明已读其完整format而非当opaque跳过。

### 新作者定义与转移范围

definitionTransfers只描述已有payload的copy/fork/identity-bearing import；新增Node里完全新写的SavedQueryDefinition/View/DynamicBlock不是“从不存在的源转移”。此类新作者payload若只含当前已存在的合法Ref/DefinitionAddress可留B，但仍在stage14完整D7解码、读权限、所有引用/Locator及全D2/D4门验证；它不能宣称获某个旧定义的transfer效果。D9 ordinary import必须先产出当前完整D7 wrapper，不能由本合同隐式猜旧format。

新作者payload直接互引本次尚未分配的fresh Node不是本版输入形状：TypedLiteral与DefinitionAddress没有symbolic public variant，整请求拒绝unsupported authored payload，映原stage14 identity_map_incomplete；不能伪造UUID。copy/fork中已有定义相互引用则由下面Q+已有source mapping完整支持，不能以这项新写入限制拒绝它们。新建多个定义互引的作者工具可在D9提出另一显式版本，但本版不会拆成两个提交冒称原子。

## 3. Result/9的Q segment

Result/9完整继承Result/8的B/M/N/E/S/C分区、长度、显式slot、C carrier与原decoder限制，新增唯一Q tag；magic改为ASCII `D3-Symbolic-Result/9`+NUL，后续header及partition算法不变。Q只允许exact_source_document的D2 SavedQueryViewDefinition **payload span**，不含外层attributes/opener/closer或anchor；不能用于Resource、Annotation、Field carrier、普通literal/body。与B/M/N/E/S/C spans不重叠。

Q body是D3-CJ/3 exact `{transfer:<完整definitionTransfers元素>,inputPayload:<原inert UTF8 payload文本>}`；start/end绑定结果base中的完整payload span，bodyLength按实际Q body字节计。inputPayload必须与sourceContainer/inputOccurrence的真实原bytes相等，且完整解码format等于payloadFormat。Q与definitionTransfers一一对应；copy/fork/identity-bearing import中的每个已识别保存payload都必须Q承载，不能留在B规避转移；payloadBindings.result按完整Result/9 bytes绑定。语义结果为下面唯一materialize函数输出的UTF8 JSON，替换该payload span；外层D2 raw、普通CEL/text值逐字保持，JSON格式化可变仅限已明确列出的inert payload整体，preview须显示这一格式化变化。

算法materialize(inputPayload, slots, original cut, original D3 candidate map)：先完整解码和枚举，按map/preserve机械代换typed Refs；对reissue_locator使用同一原D3完整source与目标occurrence映射；以D3-CJ/3 canonical JSON写回这个portable payload（UTF8原text值及换行字符保留，JSON源码空白在该已选择payload内规范化）。完整最终Query/View/DynamicBlock解码、Workspace/ref/Locator、参数型、D2全source/载体及D4验证全部通过才可接受；不能把JSON合法当Query合法。

## 4. revision与位置闭环

内部Ref映射先由原D3 stage12的ephemeral candidate map固定。fresh源revision=1，existing改变源revision=原值+1；原D3/D6为每个拟议源固定其将要使用的revision token，token由受管revision身份生成，不取尚未确定的payload内容散列。原token域/真实性仍由D3/D6，不允许D7调用方指定。只有完整原planning CAS及最终author commit才发布这些tokens，不提前授予Locator能力。

reissue_locator目标必须是原已验证的SavedQueryViewDefinition occurrence，在本次完整变换中有唯一对应的结果occurrence；deleted/ambiguous/非当前preimage Locator拒绝，不最近似定位。D7只重发指向SavedQueryViewDefinition的DocumentElementLocator。D2规定保存块的opener与closing delimiter独占physical line；Q按D3-CJ/3把完整JSON payload写为一行，string内部换行均escaped。因此坐标数字的宽度不会改变保存块边界的line/column，采用确定的两遍物化：

1. 固定完整Ref/revision token及其它原D3/C非D7-position变换。Q中的待重发位置先用内部零坐标模板，整体canonical JSON仍一行；拼成完整source，用D2 parser取得每个映射后的SavedQueryViewDefinition的完整真实SourceSpan。内部模板不得交付或当合法Locator解析。
2. 将这些真实span同时代入相应Locator，再次序列化Q并拼成完整source。重新用D2 parser读取每个目标element span，要求与第一遍逐字相等；随后每个最终Locator用原D3 decoder/resolver证明same owner/revision/elementKind/exact target occurrence。任何不等、目标消失/多义或预算溢出整操作拒绝，不迭代猜测或交付中间值。

该算法的适用边界是完整saved-definition element位置：不是payload某byte或任意正文range。普通preserve的外部Locator逐字保持并验证原当前指向，不更新latest。D3/C处理的Annotation或D4 provenance坐标继续各自原接受器；它们须绑定同一最终source，不能分两次提交各自正确，也不能修改Q canonical JSON一行的前提。若一个同plan变换不能证明与这个两遍边界兼容，整计划在stage14失败；没有自由脚本参与重排源。

## 5. 完整效果与未知payload

原D3 receipt及D6效果同decision附带D7自己的`D7DefinitionTransferEffects/1`，exact `{kind:"d7_definition_transfer_effects",version:1,operationId,transfers}`。transfers逐项exact `{sourceContainer,inputOccurrence,resultOwner:NodeRef,resultOccurrence,beforePayload:text,afterPayload:text,slots}`；slots逐项`{path,before,after}`，after是实际完整Ref或重发Locator，不再是symbolic subject。数组顺序与request definitionTransfers一致；每项由实际完整before/after源独立恢复并逐项验证，不能仅复述request。该扩展不进入D3 authored node_link/citation slot union，不造D3 referenceLifecycleTransitions或持久definition身份。

copy/fork/import的包含Node若有未知或不可解码D7 payload，原raw可被普通D2只读保留，但**typed transfer整体拒绝**，不能假称已保留可执行引用含义；full fork不省略该Node或删block。用户可以在另一个明确author编辑中修复/改为普通inert literal后重新提交新operation，这里不自动做该转换。已识别payload没有Ref时slots=[]仍通过完整schema验证并保留text值；普通body/source/literal中的同UUID字符串完全不参与。旧result handles、ActionEvidence、preview/effects tokens本来不是合法portable schema成员，遇到它们拒绝unknown member而非搬到新Workspace。
