---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：0f987339-9151-4a03-bd35-d5b207a8540f。

候选状态：D7 文件权威协调后像，未独立接受、未激活、未实施。固定 S 中的阶段接受、D8/D9 未启动及模型计数仅为历史来源记录；本候选保留其真实语义和证据限度，不把历史标签当本轮状态。当前消费同包 D3 wire12/Result9、D4 当前消费者与 D6 Control 的完整生产版本、当前观察、依赖和单决议恢复合同。新增 D3 冲突准备与 D7 /3 绑定尚待全包独立联合审查；portable Locator 跨副本重资格仍待具名裁决，不宣称本候选已整体闭合或可激活。

# D7 窄Field准入与公开结果

本文件是D7规范的一部分，要求同包D6显式replacement共同接受。它给一个可构造的phone-only正向路径；不会由field_read/field_write自动增加元数据或其它作者内容权限。D6主稿及Control Interfaces必须同步采用本合同后才可激活，未完成此同步前本稿仍是未接受草稿。

## 1. 显式元数据权限

D6 当前 Policy/3 已定义两个 closed 无参数 capability：`{kind:"source_envelope_state"}`和`{kind:"commit_sequence_state"}`。它们不被source_read、field_read、field_write或policy_admin蕴含，也不蕴含任何这些能力。source_envelope_state只配workspace或ref_set scope，与entity_state一样先按请求完整Ref和当前policy验证，不用subtree/先查parent；commit_sequence_state只配workspace scope。deny覆盖allow；缺显式allow不合格。既有Policy/1不会自动升级或补grant；只有正常受权的当前 policy 修改才可安装 Policy/3；本文件不为旧 Policy/1 或 /2 自动补 grant。bootstrap仍按冻结的完整profile版本建立policy，不得回放时偷偷扩大旧grant。

source_envelope_state明确授权观察该完整源的revision及其变化、source字节长度、物理行/深度/数量容量边界、整个D2结构/容器是否有效、当前Registry下整个源的D4验证是否可用/有效，以及这些有限状态对源编辑成功/失败和CAS的影响。它不允许返回body、title、其它Fields、Facet名称、具体隐藏错误位置、值、ref或约束命中成员；公开精确schema如下。这个权限有意允许观察同owner源是否被其它编辑改变，不能宣传为隐藏源活动完全不可观察。

domainCommitSequence 是 D6 在完整 CommitDomain/2 中维护的成功 author decision 序号；它不是 Workspace 跨副本/authority 的全局计数。fresh domain 初始化0，同域 D3/D6 最终 committed decision（包括其合同允许的 control_only/no_op）在同一 P seal checked+1，重放/恢复尝试/prepare/rejection 不增；完整 DecisionKey 至多一次。MAX 不回绕。不同 ReplicaEpoch/authority 的计数不可相加、比较新旧或借来推导 source H；切换域必须当前 fence/连续性资格。本 capability 的 workspace scope 仅限定 Workspace，实际读取绑定请求指定 CommitDomain，不授其它域序号或任一 decision 内容。具此明确资格的 phone 编辑者可以观察该域提交活动，不能称其为严格秘密；源生产版本、Frontier 与内部 contentSequence 仍是不同事实。

current `d7_source_envelope_read` exact `{wireVersion:2,kind:"d7_source_envelope_read",workspaceRef,commitDomain:CommitDomain/2,expectedFrontier:Frontier/2,ownerNodeRef}`。先 closed decode、当前 Workspace/entity_state/source_envelope_state 及所需 Registry/Field 披露，再验证当前域/连续性与精确 cut，才读取 source/聚合验证状态。成功 valid exact `{wireVersion:2,kind:"d7_source_envelope",ownerNodeRef,sourceVersion:SourceVersion/2,sourceVersionRef:SourceVersionRef/1,sourceBytes,maximumPhysicalLineBytes,maximumDepth,entryCount,validation:"valid"}`。invalid/unavailable variant 同成员但 validation 分别为 invalid/unavailable，并禁止 sourceVersionRef、maximumDepth、entryCount；物理 sourceBytes/maximumPhysicalLineBytes 仍须真实完整读取证明，不以0填未知。源/域连续性或完整物理读取不可证则没有成功 envelope，依原 D7 query-error/2 diagnostic 映 source_unavailable/result_unavailable；不存在/不 live/无权依原当前 state 遮蔽为 not_visible，不泄漏隐藏原因。

sourceVersion 是真实完整生产地址，生产域/世代可以不同于当前请求域，不能用一个 sourceRevision 数值替代。valid 的 sourceVersionRef 只由真实 D6 SourceObservation/1 producer 产生：entityRef/sourceVersion 与此响应逐项相等、observerDomain 等于请求域，当前 FileObjectBinding、完整 pins、source/控制/Registry proof 同 cut且通过本文件静态资格。它不是 D7 自签 token，不授 body/其它 Field/public source bytes 或写权限；每个后续 consumer 仍重验实际授权/用途。invalid/unavailable 不产生可供 prepare 使用的该 token。这里只公开聚合状态，无 hidden 原因/位置/值/Ref；详细诊断仍归原 repair/audit 资格。FieldSelector/EntityTarget 的既有 Counter 只能与已被当前观察证明的 managed revision 比较，不能把 externalSequence 强塞其中；原 qualified ordinary external 读取/人工入管仍由其真实 owner 处理。

真实旧 wire1 source_envelope_read/source_envelope 保持原 decoder 和当时 sourceRevision/计数语义，不能给已存旧 handle 补域或把它升级为当前 Action 证据。新请求只用本 /2；没有部署/记录证据的旧草稿不虚构永久存储义务。


## 2. 从真实Registry构造潜在范围

输入只允许显式单owner的Field append/replace/remove/set_field_member或同型bulk分解；native、Facet、identity、Query membership、Calendar、relation/inbound/foreign-binding及任意raw变换不能利用此证明。先只读已认证的完整Registry定义图，不读取owner作者值或当前membership来决定资格。图枚举所有Field/Facet/alias及constraint constructor；未知、不可用、未覆盖新constructor直接不可证明。scope不是调用方传入的qualified布尔值。

对触及Field F计算如下闭包，所有条件来自已验证schema及closed意图形状：

1. F及其完整alias、qualifier、unit/code/external-scheme等贡献依赖进入范围。若任意可达值/qualifier/provenance可能引入typed Ref/Locator或relation，且当前adapter不能从意图证明这些成员逐字保持，则本版窄证明拒绝；需要更大真实范围。潜在输入provenance仍按D4完整验证，不能删掉后继续。
2. Field cardinality与at_most_one_preferred仅影响F，故读取F完整Entry集合。mutually_exclusive_members与measurement_unit_dimension读取该Field完整value及相关贡献，不扩大到其它作者Fields。
3. 所有可能含F的Facet constraints必须逐个检查。required_field在存在合法旧Entry的replace/set_member下由条目数量不变证明保持；append由数量不减保持；remove不能从未读membership猜required是否适用，本版窄路径拒绝。union_variant_equal：若closed mutation不能证明F原root union variant保持，则加入另一Field的完整数据且仍不能隐藏实际Facet适用性；本版最小路径保守拒绝此类可能变更。不得因当前owner碰巧未声明该Facet而跳过潜在约束。
4. Classification及其它Field必须逐字不变，源变换仅在已验证F的精确Entry区间；任何namespace/trivia/body/header/其它Field差异使证明失败。已有源的完整D2/D4有效性由source_envelope_state授权的真实验证状态决定，不由请求或cache自报。有效前像+保持不变量的局部变换，对未变Fields保持其所有原验证结果；涉及重新判断不可证明的跨Field/分类条件时不能走此路径。
5. 计划保存潜在Field上界、完整Registry定义路径与逐constraint保持证明、源Envelope状态及版本、F完整raw前像/拟议Entry、实际全源byte patch、所有当前授权与依赖。内部读取分成用于原字节保留的完整源、已授权Envelope聚合验证、以及进入业务结果的F事实；追踪必须能验证业务/公开效果从不读取上界外作者值。未知读取类别在求值前停止。

通过上述图证明后，才检查当前principal对F及闭包全部Fields的field_read、实际修改的field_write、对应源owner的完整entity_state和source_envelope_state（不是给所含作者Ref目标增授状态权）；需要提交/receipt的Action还须指定请求域的 workspace-scoped commit_sequence_state。不能证明则在作者读取前not_visible；不得试运行后看这次结果是否安全。source_write deny仍阻止实际修改。结果可见性与原source版本/CAS始终另验；不因为完整源被内部读取而输出整份source。

## 3. 正向phone构造

使用实际D4 catalog的people/phone，text成员在其已展开closed object内修改。选择两个完整合法、同值但不同occurrenceKey的phone Entries之一；set_field_member只改required text路径，保持同key、qualifiers、note、provenance和全部其它members。Registry全图证明它不是relation，不改变root variant和任何Facet membership，Field计数不变；at_most_one_preferred输入保持，requiredness保持，text的nonEmpty和code域仍按D4验证。本构造同时涵盖省略provenance、合法同/跨Workspace Node provenance、历史Locator和owner-local Resource provenance；资格不由当前值是否为空决定。Value§5.1的统一作者原值规则允许在完整Field读取权下逐字返回这些原作者成员，不查询其目标Workspace/state/content。closed text变换证明全部Ref/Locator/provenance保持，因此这些目标状态不是本窄操作的依赖，也不要求对任意foreign Workspace作静态授权。原D4仍完整检查形状、Locator owner和Resource locality；它明确允许普通foreign NodeRef，不得宣称全被拒绝。新建/改写Ref、依赖目标状态或relation不属本局部text证明；需要其真实更大范围。点击仍走独立当前resolver/content门。

主体显式获得：workspace的workspace_state/entity_state/locator_state、owner的source_envelope_state、workspace scope的people/phone field_read/field_write（原D6 Field capability不允许ref_set；本例不偷偷增加该scope），以及指定请求域的 workspace-scoped commit_sequence_state；没有source_read/body读取、其它Field读取或policy_admin。read/选择返回两个完整phone selectors；第二个selector的revision和raw匹配后prepare，preview只包含该Field两条Entry的完整before/after及owner SourceVersion；提交经原D6 CAS恰修改第二条，第一条、body和隐藏Field逐字保持；effects与原receipt可完整审阅。此构造必须由实际source/Registry/权限/提交模型执行，不能仅把本段expected写入报告当pass。

## 4. 全公开outcome比较

两有效世界在输入、F事实、已授权Envelope元数据、明确获权 domainCommitSequence及当前policy/Registry/authority上相同，仅隐藏body/其它Fields内容不同：读取、选择、preview、成功/no-op/拒绝、receipt.sourceVersions、effects、replay结果必须逐字相同（opaque token随机性按同一随机币耦合）。source前像和后像秘密区域保持各自原文，但不交付给此主体。

若两世界只因隐藏源变化导致sourceRevision、长度、容量、验证状态或指定域 domainCommitSequence不同，这些差异由上述显式元数据capability预先授权；其公开影响可不同，必须在证据中逐项指出原因。不能新增未列的隐藏success oracle。D2容器接近capacity、CRLF/Unicode字节差、其它Field并发引发source CAS、新Registry约束、source_validity改变、撤权、source A→B→A与lost receipt重放都必须覆盖。去掉source_envelope_state或commit_sequence_state的负例在读取作者源/ledger前统一not_visible；不能为了让窄路径成功而自动授予。

这是明确减少内容读取面的权限配置，保留必要的源元数据与提交活动观察成本。它不声称不同物理I/O/OS故障/墙钟耗时不可区分；这些已在D6范围外，但不能用该限制豁免本文件列出的业务输出、计数、CAS与preview/effects。

## 5. 当前完整证明与准备绑定

本窄路径先验选择 D6 owner_fields ObservationScope/2，完整 SourceObservation/1 和 source key 仍由 D6 生成；完整当前 Registry context、结构/state/authorization 及真实需要的其它 DependencyProof/2 正负key不能省略。source_envelope_state 只授权列出的聚合观察，不把 owner_fields 升格为 local_source 或全 Workspace 写域，不取消 source_write/Field deny。守恒证明覆盖实际 schema 的每个可能 constraint，遇未识别 constructor 在作者读取前停止；不可按这次 payload 内容碰巧简单才选窄 profile。

PreparedActionBinding/3 的 sourceInputs、D6 PreparedIntent/2、源前像/唯一拟议源、FieldEntryImage/2、preview header 的完整 DecisionKey 与原预算/guard 均逐项相等。source-state 未变且 raw 真正 no-op 才不列 field/source change；equal-byte external admission 若当前具体窄adapter不能由完整 schema/类型证明，则不能借本窄路径强行入管，应使用用户选择的真实 source owner 入口。已提交窄效果始终是全部真实作者修改的完整 Field 投影，原完整 source/pins 只在受保护记录中保留；不能在 commit 后把无法投影的 canonical/结构/其它Field改变隐藏掉。
