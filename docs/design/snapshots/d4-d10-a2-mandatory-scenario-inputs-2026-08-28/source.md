# Weftext D1–D10 / A2 强制场景输入

日期：2026-08-28。


## 1. 状态与使用合同

本文是用户已经讨论并同意纳入后续架构评审的**强制场景输入**，不是冻结决议、规范或产品权威。它保存在 `reviews/`，不属于 `knowledge/Weftext-Control` 的权威控制文档。

对应阶段必须：

1. 在真正全新的阶段任务提示中显式读取本文，不依赖聊天、聊天标题、近期会话列表或上一阶段模型会话；
2. 把与该阶段有关的场景、反例和替代方案完整放入 `$council` 的自包含候选与盲审包；
3. 让独立 OpenCode 评审、逐项裁决与全新 GPT-5.6 Sol / Pro Gate 判断候选是否成立，不得把本文的候选方向预写成结论；
4. 对每个适用项给出 `accept | revise | reject | defer-with-owner` disposition、理由、关闭证据、wire/source/状态/错误、实现影响和测试轮廓；
5. 只有相应阶段 Gate pass、P0/P1 全关闭并提升到 `knowledge/Weftext-Control` 后，结论才成为持久权威；
6. 若阶段结论与已冻结上游合同冲突，只能提出显式上游反例和重开请求，不能静默回退 D1/D2 或届时已冻结的后续上游合同。
7. 创建 D4、D5、D6、D7、D8、D9、D10 时，必须把本文路由给该阶段的部分逐字带入自包含 prompt，并把对应场景、反例、open questions 与替代放入自包含候选/盲审包；A2 必须读取本文完整 bytes 并做全链复核。
8. §8.5 是 D1–D10/A2 的横切 terminology gate：D1/D2 作只读非回退审计，当前 D3 仅接收与其 identity/reference/ownership/lifecycle 直接相关的自包含术语切片，D4–D10 各自维护权威 Lexicon 后再进入盲审；任何阶段都不得把聊天口语或历史代码名直接升级成 canonical term。

本文不得由讨论任务直接同步到 `knowledge/Weftext-Control`。同一时刻仍只允许当前设计阶段任务写控制区。

## 2. Profile、插件与作者内容边界

必须评审以下候选方向及完整替代：

- 不采用开放的通用 Node type。领域能力使用普通 Node 加可选、版本化 Profile；People、Temporal、未来 Project 不成为新的 Node identity 或 kind。
- 插件业务字段默认全部可选；使用稳定 canonical key 和 UI 本地化 label。插件关闭后，作者内容必须保留、可读；插件不得拥有私有作者数据库，也不得绕过 Core 写入。
- YAML/系统 envelope 只承载系统控制事实。邮箱、电话、时间范围、组织等对用户有意义且可查询、可导出的事实进入规范 AsciiDoc header attributes 或关系语义；正文保留叙述性备注。
- Node 的 schema-membership placement 必须由 §12 的 A/B/C 顶层替代比较决定。此前“另一个规范 AsciiDoc header attribute、不得复用 `wf-specialization`”现仅保留为方案 A；用户当前偏好是方案 B 的 closed `_weftext` Node control envelope，但尚未通过 review/Gate。
- 插件的安装/启用/可用状态属于系统、部署或 Workspace 配置/控制状态，不是 Profile membership 或作者内容。membership 不得因插件未安装、停用、升级失败或当前表面不可用而被删除、清空或改写。
- D4 必须冻结 typed value、列表编码、validation、Profile identity/version/membership 以及精确 wire/source。需验证 Profile 关闭、未知 Profile、版本升级、部分字段无效和跨表面一致性。
- D4 open questions 必须包括：是否允许一个 Node 同时拥有多个 Profile；profile canonical namespace、identity 与 version 如何编码；namespace collision 如何拒绝；未知/停用/版本不兼容 Profile 的 commit、read、edit、query、export 和 repair 行为；membership 与插件 enablement 如何保持正交。
- 必须比较至少这些替代：开放 Node type、Node kind 子类、普通命名空间属性而无 Profile、独立插件数据库、全部正文约定，以及候选的普通 Node + Profile。不得预设候选必胜。

### 2.1 D1/D2 非回退检查

- People、Organizations、Calendar（合并此前 Time Notes/Time/Calendar 候选）、Graph 若被接受为官方可禁用模块，必须显式判断它们只是 D1 已冻结插件/可选能力边界内的实例，还是构成新的正式产品表面、运行模式、能力所有者或发布承诺而必须提出 D1 重开。
- 不得为了接受模块而悄悄重开或改写 D1/D2。若成立的最小反例要求重开，阶段只能记录具体冲突、影响面和重开请求，并停留在候选状态。
- D2 的普通 Node、作者权威、header grammar、`wf-specialization=task|template`、Resource owner-local 和无第二内容权威都是当前冻结输入。若 §12 方案 B 胜出，它不是 D4 可局部添加的字段，而必须形成显式 D2 reopen request，并重证依赖这些边界的 D3；方案 A/C 也必须证明 single-authority 与 exact-source 闭合。
- 每个 D4–D10 owner 阶段都要列出 `D1/D2 compatible | requires explicit reopen` 的可审计 disposition；A2 再从完整产品面与依赖图复核一次。

### 2.2 可组合 namespaced Profile 候选

用户授权把下列产品意图作为 D4 正式候选和压力案例，但它仍须与单 Profile、无 Profile 机制、Node kind、自由属性和其他完整替代一起接受 `$council` 挑战：

- Profile 是 Node 声明遵循的领域语义/schema，不是新的 Node identity/kind、插件所有权、权限边界或插件私有数据区；Core 始终是 Node 与所有写入的唯一权威。
- 一个 Node 可声明 `0..N` 个 Profile。候选语义是无顺序集合、无覆盖优先级、相同 Profile ID 重复非法；实现不得按 source order、插件安装顺序或 UI 顺序解决冲突。
- 一个插件可以提供 `0..N` 个 Profile，也可以消费其他插件提供的 `0..N` 个 Profile。插件仅仅读取、渲染或提供 View，不得因此自动向 Node 添加 membership。
- Profile 定义至少要机械闭合：canonical namespaced ID、version/schema identity、依赖、继承或有效闭包、互斥、字段/关系冲突、缺失依赖、unknown/uninstalled provider、卸载保留、schema 演进、Query projection、Action 写入、权限与 diagnostics。
- 必须明确 declared membership 与 effective closure 的区别：只有 declared membership 是作者源；依赖/继承导出的 effective profiles、validation、能力、索引和 View 都是可重建派生状态，不能回写成第二份 membership。
- “组合”不等于 override。两个 Profile 对同一 canonical 字段声明不同类型、不同 cardinality、不同 relation direction 或不相容 constraint 时，必须得到唯一冲突/拒绝/不可用语义；不能 last-wins、安装优先或让某一插件私下接管。

### 2.3 作者权威、registry 与 Task/Template 非回退

- 此前候选把 declared membership 唯一写入 AsciiDoc Document header，示意 `:wf-profiles: ...`；该方案现命名为 §12.A，并已被用户新的 §12.B closed `_weftext` envelope 偏好 supersede 为正式对照，而不是默认胜者。两种示意 wire 都未冻结。
- 无论 A/B/C 何者胜出，都不得同时接受 YAML 与 header 两个作者入口、不得设置 precedence/fallback/双写。外部 YAML/header 可作为 D9 import source，但 commit 后只能有一个 canonical target；准确目标由上游 reopen/对应 Gate 决定。
- 插件 manifest/schema registry 承载 Profile 定义、依赖、继承、互斥和版本。插件安装/启用状态在控制面；registry、effective closure、validation result、索引和 capability projection 都可重建，不得成为作者数据。
- 必须逐条核对 A/B/C 是否保持 portable single authority、external edit、atomic move/copy/export、plugin-missing degradation 与 no-second-authority。B 触及 D2 Document grammar/exact-source/metadata projection，C 触及 D6 control-plane/sidecar；都不得把“历史已投入”当否决理由，也不得绕过显式上游重开。
- Task/Template 不回退：冻结的单值 `wf-specialization: task|template` 继续独立存在，不重复写 `profile=task`。候选允许 Task specialization 机械激活内建 Task schema，但该隐式 schema、其 effective closure 和 diagnostic 必须被评审，不能反向把 `wf-specialization` 泛化为 Profile。
- Task Node 仍可另加 `project/work-item` 等领域 Profile；Template Node 是否可加 Profile、模板实例如何继承/选择 Profile 及其 fresh identity 行为必须由 D4/D9 明确，而不是从模板本身持续推断。

### 2.4 创建、Assign 与 Remove Profile Action 候选

1. 新建普通 Node 时可选择一个或多个 schema memberships；Core 在一个原子计划中创建 Node、按 §12 最终选定 canonical source 写入 membership、应用默认值和必要关系。provider 可以提供专用创建表单，但表单不拥有第二套 validation 或提交路径。
2. 已有 Node 只能通过显式 `Assign/Add Profile` Action 添加 membership。Action 必须 preview schema 变化、默认值、关系、冲突、依赖、未知 provider、权限和预期 revision，再由 Core 原子提交；不得按既有字段或标题启发式静默“认领”，不得由 UI/插件直接写文件。
3. 分配 `people/person` 后，People 插件可提供人员专用属性编辑器、重复姓名提示、联系人列表、关系/组织 View 与导入导出；原 Document body、title、Resource 和 Annotation 仍是普通 Node 内容。
4. Remove Profile 必须独立定义。默认候选只撤销 membership 的持续语义与专用 UI，不静默删除作者字段；字段清理必须是另一个显式、可预览、可审计、revision-bound Action，并说明仍被其他 Profile 使用的字段如何处理。
5. Profile 组合仍须用第三方 hypothetical fixture 验证，例如同一 Calendar period note 组合一个 `example.custom/entry` Profile；这只测试通用组合能力，不构成 Journal/Diary 产品或 `diary/*` Profile 承诺。所有 Action 都由 Core 重新验证和提交。
6. D4/D6/D7 必须冻结 create/assign/remove/cleanup 的 request、plan、state machine、error priority、expected revisions、权限遮蔽、并发 CAS、receipt 与 retry；五个正式表面只能提供不同交互，不能产生不同领域效果。

### 2.5 Template 边界

- Profile 可以带来类似“加强版模板”的体验，但二者不是同一机制。Template 负责一次性实例化初始 title、body、attributes、children 和 Resources；Profile 是 Node 生命周期内持续生效、可验证、可查询、可组合的 schema 与行为合同。
- Create Action 可以同时选择 Template 与 Profiles：Template 产生初始内容和 fresh objects，Profiles 决定长期 membership、validation 与可用 Action。必须冻结两者冲突时的 preview/reject 规则。
- Template 不能成为实例后的隐藏持续权威；Profile 也不能覆盖或重新生成作者正文。后续编辑只服从实例自身 exact source、Profile 合同和显式 Action。
- D9 必须比较 Template 直接声明默认 Profiles、创建时调用方另选 Profiles、两者合并或禁止组合等替代，并服从 D3 fresh identity 与 D4 membership canonicalization。

### 2.6 Profile 强制 fixtures 与 open questions

对应阶段和 A2 至少逐项重放：

1. 新建 Person；
2. 给普通旧 Node 显式分配 Person；
3. 同一 Calendar period note 加一个第三方 hypothetical custom Profile；必须证明该 fixture 不会被误记为 Journal/Diary 产品承诺；
4. `Asset + Equipment`；
5. `Library Work + scholarly journal-article` schema/profile 组合；此处 journal 指学术期刊载体，不是 Diary/日记；
6. `Task specialization + Project work item Profile`；
7. unknown Profile；
8. provider 插件卸载后保留，再重装；
9. 两个 Profile 对同一字段声明不同类型；
10. 依赖缺失；
11. 互斥组合；
12. 外部编辑损坏 membership、分隔或字段；
13. 两个 writer 并发 Assign Profile；
14. Remove Profile 后默认保留字段与另行显式清理；
15. 外部 YAML/header 在 A/B/C 各方案下 preview 后只映射到一个 canonical target；不得形成双权威；
16. Desktop、CLI、Server/WebUI、Mobile 对同一 fixture 的 UI/command、wire、plan、diagnostic、receipt 一致。

仍须 council 裁决的问题包括：Profile ID/version 是否同一 token；依赖与继承是否都需要；effective closure 是否暴露 wire；unknown Profile 时 exact source 是否仍可普通编辑、哪些 typed commit/query 必须不可用；字段是 Profile-owned、共享 canonical field 还是 namespaced field；Remove 后 orphan fields 如何标记；manifest/schema 真实性与版本选择；Profile set canonical source order是否固定及重排是否是可观察编辑；五表面 capability 不可用如何 fail closed。

## 3. People Profile 完整场景

### 3.1 V1 候选字段目录

People V1 必须把下列目录逐项评审和裁决，不得因为初始候选只展开了多值字段而漏项：

- Node title：当前首选显示名；
- `names[]`；
- `emails[]`；
- `phones[]`；
- `accounts[]`；
- `organization`；
- `job-title`；
- `birthday`；
- `address`；
- `website`；
- 可选 `avatar`，候选为 owning Person Node 的 owner-local Resource；
- Document body：自由备注和叙述性内容。

除 Node title 与 Profile membership 契约外，People 业务字段候选均为可选。目标是知识工作区的最小联系人语义交集，不扩张成 CRM。准确 canonical 字段名、scalar/object/list 形状、结构化地址粒度、生日的 year/month/day 可缺失规则、website/account 规范化、avatar 的 ResourceRef 与 Document occurrence/presentation 关系仍须由 D4/D8/D9 逐项评审，不得从本目录推断为已冻结 schema。

### 3.2 身份、标题与消歧

- Node title 是当前首选显示名；Workspace 内允许重名，以 NodeRef 区分，不添加随机后缀。
- UI 可用组织、职位、脱敏联系方式、路径或短 ID 消歧；只提示可能重复，绝不自动合并。
- 必须评审标题与 `names[]` 的单一权威或机械派生规则，以及重命名、重复提示、显式合并/不合并、删除和恢复行为。

### 3.3 多名称

- 覆盖中文名、英文名、现用名、曾用名、法定名、别名、昵称、转写等。
- 候选条目至少考虑：`value`、可选稳定 `label/usage`、`language/script`、`note`。
- 必须关闭空值、重复、排序、本地化、未知 usage、大小写/Unicode、title 同步和导入映射语义。

### 3.4 联系方式与账号

- phone/email/IM 等候选采用结构化、多值 contact entries，不为 `workPhone/personalPhone/...` 无限平铺字段。每项至少有 `value`、可选 preset label code、可选 free-text `note`；technical kind 与 usage 是否分开由 D4/D5 按真实查询/validation 需求裁决。
- 常用 preset 示例 `mobile | home | work | work-mobile | other` 必须是 namespaced stable semantic IDs，由 People module 提供 zh-CN/en 等本地化 label；切换语言只翻译 preset display。用户 custom label/note 原样保存、不翻译，且不能把已本地化字符串写回 semantic code。
- preset 组合不得为设备类型×usage×组织上下文生成无限枚举；必须比较两个正交 code、一个窄 preset code、custom label 和更小替代的整体成本。
- 即时通讯 `accounts[]` 还需 canonical service，例如 `wechat | qq | x | facebook | custom`，以及 `identifier/value`、可选 label/note；service、technical kind、usage 与 display label 不得混成一个字符串。
- 同一 localizable-code 模式必须压力测试 phone/email/IM/address/organization role/relationship labels，并进入 §8.5 Lexicon；准确字段/wire 仍未冻结。
- 必须避免把 People schema 扩张成完整 CRM；比较最低稳定语义集、provider 特有字段、自由属性与 D5 Record 替代。

### 3.5 亲属关系

- relatives 是一个亲属关系 collection/editor 候选，不为 father/mother/son/daughter 等无限固定单字段。每项必须保存 stable directional relation code，例如 `parent | child | spouse | sibling | guardian | custom`，而不是只存当前语言的显示 label。
- target 必须是显式联合：`{kind: node, ref: NodeRef}` 或 `{kind: text, text: ...}`。不得按标题或字符串猜测，也不得自动创建节点。
- NodeRef target 可跳转、反向查询和派生；文本 target 只是作者记录，不产生 inverse edge，可在显式链接/创建后通过原子动作替换。
- `parent` 有方向，`child` 可由 inverse 派生而不双写；`spouse` 对称且不双写。每个 code 必须定义 direction、inverse display label、对称性、time/status/qualifier 与删除语义，才能安全生成 Family Tree。
- UI collection 可显示“亲属”，relation codes 本地化为父母/子女/配偶等。若需要父亲/母亲等更细显示，只能来自明确 schema fact、target fact 或用户选择，不能凭姓名、title、称谓或性别猜测。
- `custom` relation 可保留 custom label/note，但不得冒充可推导 parent/child graph、获得未声明 inverse 或进入代际 rank。
- 必须冻结事实唯一存储位置、跨权限 inverse edit、删除/Trash/恢复、target 不可见或被清理、schema/provider 停用和导入行为；准确 relation wire、collection identity/value/Record 边界由 D4/D5 裁决。

## 4. Organizations 官方插件候选

Organizations 是必须进入 D4–D10/A2 评审的官方可禁用插件候选，不是已批准实现、冻结字段目录、国家枚举或新的领域身份。当前阶段只需证明目标架构可承载该插件且不产生第二权威；具体国家枚举、完整表单、connector 和产品实现必须等 A2 通过后由明确实现任务决定。

### 4.1 获授权现实证据及其边界

- 用户授权只读使用 `D:\OneDrive\Note\实体\组织` 作为现实样本；不得修改、迁移、补字段或把 Vault 当成 Weftext 权威。
- 2026-08-28 只读快照确认该目录恰有 114 个 Markdown 文件。113 个文件出现 `上级组织`，112 个出现 `类别`；样本高度集中于中国公共机构/公安体系。公安层级只能作为高压 fixture，不能成为全球 Organization 设计中心。
- `D:\OneDrive\Note\实体\组织\组织.md` SHA-256 `CCFF6A8C3713E75ED731CEB8FAE81BC0040541D963B92FC5993AF60946D79939` 明确要求目录 Folder Note 父级与 `上级组织` 一致；`D:\OneDrive\Note\实体\组织\公共机构\公共机构.md` SHA-256 `D470C401AD2591F977064CFBABB43BB293FE22DFA38A163F8AB60F8C8691EB05` 进一步区分主隶属、业务机关与属地政府。
- 实际视图位于 Vault 根 `D:\OneDrive\Note\_views\组织关系.js`，不是组织子目录；SHA-256 `9745A136A63ADE92A32D810410E9A3979D38F6D032316AB63EC3069D622EAD08`。其 `childrenOf` 按 `file.folder`/parent folder 生成下级树，证明旧 Vault 存在“显式语义父级 + 目录派生父级”的双权威压力场景。
- 上述数量、字段出现与脚本只是一次有界证据快照；不得据此推断全球字段频率、用户总体分布、国家枚举或实现优先级。

### 4.2 产品分层替代

必须从零比较至少这些完整方案：

- 一个通用 Organizations 插件提供 Organization 基础语义；
- `Public Organization`、`Business Organization`、`Nonprofit/International Organization`、`Organizational Unit` 等作为可组合 Profile 扩展或预设；
- 中国公共机构等国家/行业特定能力作为可选 namespaced schema pack；
- 单一超大 Organization schema、每类组织独立插件、自由属性、独立 Record/数据库等替代。

企业属于 Organization 领域。深层 CRM、工商数据 connector、股权/财务分析可以成为依赖 Organizations 的未来独立插件，不能反向把通用 Organizations 扩张成 CRM。不得创建 `EnterpriseId`、`PublicBodyId` 或其他平行内容身份；Person、Organization 和 Organizational Unit 都仍是普通 Node/Profile。插件不得拥有私有作者数据库。

### 4.3 D4 强制问题与最小字段候选

- Organization schema membership 的 canonical source 由 §12 A/B/C 与术语 Gate 决定。基础/扩展 schema 是否允许组合、继承或二者都不允许，组合冲突、顺序、版本与 unknown/disabled pack 的 read/commit/query/export 行为必须机械闭合。
- 最小通用业务字段候选必须逐项比较更小替代：`names/aliases`、`status` 与有效期、`parentOrganization`、`identifiers[]`、`classifications[]`、`relationships[]`、`sites/addresses/contacts`、`memberships/roles`、`sources/verifiedAt`。全部业务字段候选均可选；目录不是冻结 schema。
- 国家或行业特有值不得硬编码为全球字段/enum。候选使用 namespaced scheme，例如 identifier `{scheme: cn.uscc | cn.police-organization-code | lei, value}` 与 classification `{scheme, value}`。中国 `机构系统/性质/类别/行政层级/规格/公益类别/管理体制` 是 schema-pack 候选，不是全球 enum。
- `管财领导`、`行政内勤`、`董事`、`经理` 等不得每个职位扩张成一个字段。优先比较 membership `{personRef, role scheme/value, validity/qualifiers}`、Profile 引用字段、普通关系与更小替代；人员和组织仍是普通 Node/Profile。
- 组织主隶属、业务指导、属地、监管、所有权、子公司、品牌、联盟成员等必须保留不同语义。设计要避免无限全局 ontology，同时证明 Profile 引用字段、namespaced relation scheme 和 qualifier 足以支持查询、inverse、权限、专用 View 和 connector。
- `Folder/Node structural parent` 与 `parentOrganization` 不能成为双权威。必须评审以显式语义 NodeRef 关系作为组织隶属唯一作者权威，Organization Chart/children 由此派生；物理路径或普通 Node structural move 不得暗改组织事实。

### 4.4 生命周期与识别边界

- 组织改名不改变 NodeRef；撤销、合并、拆分、继承、品牌与法定实体变化必须从零区分 status/validity、relationship、显式 Action 和 fresh identity，不能按名称或 identifier 自动合并。
- `identifiers[]` 是外部 scheme/value 事实，不是 Weftext identity。重复、复用、历史 identifier、同一组织多个 scheme、一个 scheme 多值和来源验证必须进入 D4/D6/D9 测试。
- Organizational Unit 是否可独立引用、拥有 Document/Resource/关系，只能通过普通 Node + Profile 取得；不得因为嵌套单位而发明插件局部实体 ID。

## 5. 关系与 Graph

### 5.1 历史证据边界

- 当前旧实现只有出链、反链、潜在提及和旧 Graph 产品意图，没有完成的 Obsidian 等价全局/局部图 UI。
- 历史实现只作待核证证据，不得反向决定从零架构。

### 5.2 投影与视图

- 通用 `GraphProjection` 必须保留 edge kind、方向/对称性和权限过滤。
- `GraphView` 至少评审 `force`、`radial`、`hierarchical/ranked` 布局；ranked 布局包含上/下或左/右呈现，但方向不等于上下。
- 家族树需要 parent 的代际 rank、spouse 同层；文本亲属只作不可展开末端卡片。
- 布局、折叠、坐标、选中和镜头状态不是作者内容或关系权威。

### 5.3 命名空间关系与 ontology 边界

- 未来 Project Profile/插件的 `contains | depends-on | blocks | inherits-from | derived-from` 等可选择进入图谱，但关系必须命名空间化、可筛选。
- structural parent、项目成员、正文链接和领域 `contains` 不得混成同一种边。
- 用户担心 `isChildOf/isPartOf` 无限扩张。优先评审“Profile 的引用字段携带关系语义并声明 `graphProjection`”，例如 `people.parents`、`people.spouses`、`project.dependencies`；Core 由字段 schema 派生有类型边和 inverse display。
- 不建立用户可任意管理的全局 ontology 或无限 `RelationType` registry，除非独立替代证明其必要性与闭合治理。
- 只有需要方向、inverse、约束、查询、专用视图或自动化时才建立领域字段；其余使用普通 link 或窄的 `core.related`。
- 独立 `RelationType` registry 必须作为正式对照替代被评审，不得预设 Profile-field 候选必胜。

## 6. 时间笔记 / Temporal 场景

用户确认：`Chrono` 名称来自其 Obsidian 插件 Chrono Notes，不要求 Weftext 保留；原插件还包含区间笔记。

### 6.1 模块与 Core 边界

- 建议评审一个可禁用的官方时间/日历能力；最新产品命名候选是单一用户可见 bundled module `Calendar / 日历`，不另设 `Time` 或 `Diary` 基础模块。内部 temporal Core types 与 Profile namespace 仍须由 §8.4 和 D4/D10 裁决；`Time Notes`/`Chrono` 只可作为历史候选、项目名或迁移 alias。该模块不是 Core `ChronoNode` 或新的 Node type。
- Core 只提供普通 Node、日期/时间/区间 typed values，以及 Query/View/Action/Transaction；模块提供创建、导航、模板、Calendar/Timeline/Interval UI。
- 重复、定时或后台自动创建属于 D10 自动化，不得由打开/读取或派生 View 隐式触发写入。
- 必须比较：保留内置 Chrono 特例、官方可禁用 Profile/模块、普通属性+保存查询、完全外部插件等替代。

### 6.2 TemporalScope

从零比较并冻结候选：

- `CalendarPeriod`：`day | week | month | quarter | year`，带 canonical semantic key、calendar 与 timezone/周规则；
- 任意有界 `Interval`：`start`、`endExclusive` 以及必要的 zone/offset/all-day 语义；
- `year/month/weekday/quarter/duration/overlap` 等尽可能机械派生，避免冗余 attributes 产生双权威。

必须覆盖 DST、时区改变、ISO week-year、周首日、闰日、跨日/月/年、零长度/反向/开放区间、全日与精确时间、calendar 版本和排序/相等语义。

### 6.3 语义键、标题与结构

- 固定的是 temporal scope 的规范语义键，不是 Node 标题、物理名、路径或 structural parent。
- 日/月/周/季/年与任意区间都是普通时间笔记；固定默认标题只可作为建议且允许重命名。
- 同一天可同时出现在月、ISO 周等多个派生 View，不用唯一 Node parent 表达多重日历归属。
- 年/月/日强制物理树不作为唯一权威；Calendar 层级应为派生 View。
- 可保留普通“时间笔记系列/根”概念，但必须评审 series 的 identity/relationship、同一 `series+scope` 是否唯一、是否允许同周期多篇、模板与默认目标位置。
- 旧 Chrono 的 canonical basename/path/层级只作历史替代。旧 Weftext `03-chrono` 实际把 quarter/month/ISO week/day 直接放在 year 下，并不等于 `day → month → year`；两者都不得未经评审继承。

### 6.4 Calendar 模块内 period note、range note 与 Event 的分域候选

D4/D7/D8/D9/A2 必须从零区分以下三个概念；不得因为都能出现在 Calendar 或 Timeline 中就合并成一种实体、Profile 或 Action：

1. **固定周期笔记**：calendar-defined `day | week | month | quarter | year`，使用规范 `CalendarPeriod` 语义；
2. **任意日期范围笔记**：普通 Node 加 temporal range/extent 语义，可被 Calendar/Timeline/overlap Query 呈现，但不因此自动成为日程；
3. **Calendar Event**：在 interval 之外具有显式调度语义，例如 `timed | all-day`、timezone、recurrence、reminder、participants、status 及相应 Action/权限/通知行为。

- 跨日不是 Event 的本质；Event 本来就以 start/end interval 表达。任意范围笔记只有经显式 `Assign/Add Profile` 或等价、可预览的 Core Action 应用 calendar/event Profile 后才取得 Event 语义，View 命中或字段相似不得静默认领。
- 示例 fixture：旅行 Node 可组合 `time/range + travel/trip`；仅当需要邀请、提醒、参与者或日程状态时再显式添加 `calendar/event`。添加 Event Profile 不得改换 Node identity、复制 temporal extent 或生成第二份 start/end 权威。
- 不得发明独立“区间笔记实体类型”、`IntervalNode` 或插件局部 identity。优先比较普通 Node 上的 typed temporal extent、可组合 Profile、无 Profile 的普通属性，以及 Event 专用 Profile 等替代；当前候选由单一 `Calendar / 日历` 模块在产品层贡献 period/range/event 能力，但三类 schema、Query、Action 与 capability 仍分域，所有写入均经 Core。
- 必须裁决一个 Node 是否可同时具有固定 `CalendarPeriod` 与任意 range、一个 Event 是否允许无 Profile 的只读投影、从 range 升级/移除 Event Profile 时哪些字段保留，以及 recurrence occurrence 是派生 occurrence、Record 还是 Node；不得用 UI 卡片形态预判 D5 结论。

### 6.5 Calendar System 与 Holiday Schedule 扩展包候选

- 领域和用户产品 UI 必须称为 **Calendar extensions/packs**，不是平铺的 “Weftext extensions”。`Calendar System pack` 可提供公历、农历、宗教历、财政历等表示、换算与规则；`Calendar Holiday Schedule data pack` 可提供国家、地区、组织、宗教的节假日、工作日调整、来源、适用区间和版本。此前模糊的 “Weftext 节假日扩展包” 表述不得继续作为正式候选。
- 历法算法与节假日数据优先作为依赖 `Calendar / 日历` 模块中基础 temporal contract 的受限、可版本化扩展包，而不是每个时间 Node 的 Profile 或作者字段。Calendar 模块定义并消费 `calendar-system`、`holiday-schedule` 等 extension points；Weftext Core/Extension Manager 只负责通用 package 安装、签名/信任、版本、依赖解析、权限和生命周期，不拥有历法/节假日领域语义。
- manifest 候选必须表达父模块依赖，例如示意 `extends/requires: weftext.calendar`；准确 wire、package kind、extension-point ID、version range、optional/required dependency 和错误优先级由 D4/D10 评审，示意语法不得直接冻结。
- Calendar 未安装、未启用或版本不兼容时，Calendar packs 不得单独生效、向 Node 注入 Profile/attribute/derived semantic 或注册孤立 View；必须进入确定的 `disabled | dependency_missing | incompatible` 等待裁决状态。用户 pack 配置、选择和来源信息不得因父模块暂时不可用而丢失。
- UI 优先归入 `Calendar → 扩展 → 历法 / 节假日`；全局扩展中心可展示 package，但必须标明“用于 Calendar”、父模块状态和可用 surface。不得把 Calendar data pack 与任意代码 Extension 放在同一无区别权限提示下。
- 纯规则/数据必须优先使用 declarative、受限 pack，不取得任意代码执行、Core 写入或私有作者存储能力；只有确需外部同步、凭据、网络访问或复杂运行行为时，才作为依赖相应 pack/Calendar capability 的 dependent extension，并进入 D10 capability/connector/control-state 评审。
- pack 属于 Workspace/View/Query context。普通日、周期或区间 Node 不得因为启用了某个 pack 就为每个 pack 写 declared Profile；星期、年月、季度、农历日期、节假日/工作日状态等通常由 canonical date/range 加选定 pack/version 机械派生，不重复写入 attribute。
- 多个 Calendar/Holiday pack 可同时启用。输出、Query 字段、diagnostic 和 View label 必须 namespaced 且携带 pack ID、版本、数据来源与适用区间；不得用一个无来源的 `isHoliday`、单一 calendar 或最后加载者覆盖其他结果。
- pack 卸载/禁用只移除对应派生显示、规则和可重建 cache，不改写 Calendar period/range/event Node 或删除作者 facts。Calendar 禁用时依赖 pack 的配置和 package lifecycle 仍由 Extension Manager 可见，但领域 contribution 不激活。
- 必须比较：受限 Calendar pack、Calendar dependent executable extension、内建固定规则、每 Node Profile/attribute、外部 connector 与完全不内建等替代；并冻结父 extension point、依赖解析、版本固定/升级、规则冲突、未知 pack、Mobile availability、权限、离线可用性、缓存可重建性、来源许可与确定性测试边界。

### 6.6 Temporal range wire、状态与错误语义

- D4 必须分别冻结 **all-day date range** 与 **zoned instant range** 的 typed wire。候选均使用 end-exclusive `[start, end)` 语义，但 date boundary 与 instant boundary 不得互换；UI 可用 inclusive 人类显示（例如 `[2026-03-01, 2026-03-04)` 显示“3 月 1–3 日”），且 wire/CLI/API/诊断必须保持 end-exclusive 明示。
- 必须裁决 open/ongoing range 的合法形状、无界端、未知结束与非法零长度/反向区间；不能用魔法最大日期、空字符串或当前时间冒充作者事实。
- zoned instant range 必须覆盖 IANA timezone、offset、DST gap/fold、跨区显示、timezone 规则版本与移动端离线；all-day range 必须说明 calendar/zone context 如何影响投影而不把本地午夜伪装成作者 instant。
- recurrence 必须冻结 rule/source、exception、expansion horizon、occurrence identity/reference、编辑 series 与单次 occurrence、权限、取消、离线缓存和冲突；expansion 结果是可重建派生状态，不能成为第二作者权威。
- overlap/contains/before/after Query、排序、Calendar/Timeline 投影、导入导出和 round-trip 必须在两类 range、open range、recurrence 与 pack context 下有一致、可测试的边界；错误优先级需区分 malformed source、invalid interval、unknown timezone/calendar/pack、unsupported recurrence、permission masking 与 stale revision。

## 7. 导入、模板、连接器与自动化场景

### 7.1 owner-local Resource 作为导入输入

- 产品动作可称“从附件创建节点…”。
- 默认读取源 Resource，不移动、不复制、不改变 owner；创建新的 Node、Document 和新输出 Resources。
- 只有显式选项才把原附件复制到新节点；该复制必须创建目标 owner 的 fresh Resource。
- 禁止导入结果自动再次触发导入；不得形成隐式递归。
- 相同 operation 的精确 retry 幂等；显式新的 operation 可以再次创建。
- D9 必须冻结 UI、source revision binding、probe/plan/worker/IR/preview/commit/receipt wire、错误、取消/崩溃清理、重复检测、实现影响和测试。

### 7.2 外部联系人导入与同步

- 使用最小语义交集加 provider adapter 和 loss report。
- provider ID、etag、changeKey、credentials、sync cursor 属 connector/control state，不是 Person identity 或作者内容。
- connector 只可提出或 patch 明确映射字段，经过 Core 校验与提交，并保留未映射的远端字段或在 loss report 中显式说明；不得覆盖本地未映射作者字段。
- 必须覆盖首次导入、重复导入、远端删除、本地删除、双向冲突、权限撤销、离线、provider 字段扩展、未知 service、插件关闭和凭据清理。

### 7.3 iCalendar / ICS 逻辑事件、recurrence 与导入意图

本节是一个基于 RFC 5545 领域模型的自包含压力案例，不把任何推荐映射预写成冻结结论。当前 D3 只需读取本节及与 D3 foreign identity/provenance 直接相关的验收要求；不得因此读取或处理本文其他 People、Organizations、Profile 或后续产品场景。D4/D5/D6/D7/D9/A2 必须把本节完整纳入各自 owner/cross-check：

1. **逻辑组件与身份候选**
   - `VEVENT` 是逻辑事件组件；`UID` 是外部持久标识/来源键候选，不是 Weftext `NodeRef`，不得占用或替换 Node identity。
   - recurring master 的 `DTSTART + RRULE/RDATE/EXDATE` 定义 occurrence set；同 `UID + RECURRENCE-ID` 的组件表达特定 occurrence override。`SEQUENCE`、时间戳及 source binding 参与同步/冲突判断，但都不是 NodeRef。
   - 无限或长期 recurrence series 绝不能展开成无限 Node。occurrence 默认是可重建派生 occurrence，不具 Node identity；exception 候选为 series-owned override record/structured value，只有显式 promote/adopt 且满足 Node 生命周期需求时才创建 fresh NodeRef。
2. **必须显式区分的导入意图**
   - `subscribe/sync`：外部 calendar 保持 authority；候选使用 `CalendarSource + event/series records` 或受管集合。provider sync token、etag、cursor、credentials 和抓取状态在控制面，不进入作者内容或 Person/Event identity。
   - `copy/import`：Weftext 成为 authority；可以按一个非重复 logical event 或一个 recurring series 创建 Calendar Event Node，但必须 preview 映射、loss、附件、未知 property 和 recurrence 边界。
   - `promote/adopt selected event or occurrence`：用户显式把选中的外部 logical event/occurrence 提升为 Node，保留 provenance/origin binding，但分配独立 fresh NodeRef；不得让源 UID 成为 NodeRef 或按标题/路径猜已有节点。
3. **默认物化候选与替代**
   - 一个非重复 logical `VEVENT` 可先成为 Event record；只有需要正文、附件、Annotation、独立关系/引用/权限/生命周期时才创建或提升为 Node。
   - recurring `UID` 在选择 Node materialization 时最多对应一个 series Node；occurrences 派生，`RECURRENCE-ID` exception 默认不成为 Node。D5 必须从零比较 record、structured value、Document occurrence、Node 和无独立对象替代，不得预设本候选必胜。
   - 批量“每个 `VEVENT` 建 Node”只能是有界、可预览、显式选择且带数量/资源上限的 import 策略，不能成为 subscribe/sync 默认；“每个展开 occurrence 建 Node”不得作为无界策略。
4. **确定 upsert、冲突与完整 wire**
   - 同源重导入必须以 source binding、`UID`、`RECURRENCE-ID`、`SEQUENCE` 及明确版本/修改事实作确定 upsert/conflict；Node title/path 改变不得造成重复创建，跨源 UID 相同不得未经 source scope 合并。
   - 必须覆盖取消、删除、源解绑、单向/双向编辑、authority 转换、离线冲突、timezone/DST、floating time、all-day end-exclusive、recurrence override、附件 URI/binary、安全获取、未知 `X-` property、恶意或超限 recurrence、解析资源限制及可重放 receipt。
   - recurring expansion 必须有 horizon/limit/cancellation 与确定 error priority；未知/恶意规则不得造成无限 CPU、内存、网络、Node 或 Resource 物化。
5. **非 VEVENT 组件不得一律映射 Event Node**
   - `VCALENDAR` 可含 `VTODO`、`VJOURNAL`、`VFREEBUSY`、`VTIMEZONE`。`VTODO → Task`、`VJOURNAL → journal/time note` 都只能是显式 mapping preview，并保持冻结的 Task/checklist/Node identity 与 author-source 边界；`VFREEBUSY`/`VTIMEZONE` 默认是 availability/timezone 语义输入，不是 Event Node。
6. **能力所有者候选**
   - Calendar/Event plugin 负责 ICS 领域映射、recurrence 与 event semantics；通用 Import worker 只负责 `probe/plan/preview/commit/receipt`、资源限制和原子提交；Time plugin 只提供 temporal extent 与派生 calendar views。三者不得各自建立不同写入权威或让 Import worker 硬编码插件领域。

正式替代至少包括：外部受管 records、全部复制成 Nodes、series-only Node、按需 promote、纯只读 View、不保留 sync binding，以及满足 D1–D6 合同的混合方案。结论必须同时与 D3 foreign identity/provenance、D4 typed wire/Profile、D5 Record domain、D6 authority/sync、D7 Query/View/Action 和 D9 import contract 闭合。

## 8. 阶段路由

每个 D4–D10 阶段都必须读取本文；owner 阶段作完整裁决，其他阶段只做不回退的交叉检查。

### 8.1 D5 必须执行的域比较

- 对 `names[]`、`phones[]`、`emails[]`、`accounts[]` 等 list-of-object，必须从零比较：D4 typed attribute value、Document occurrence、独立 Record domain 或满足全部上游约束的更优替代。
- 不得因为联系人列表的 UI 像表格，就暗中为条目生成 Record identity、独立 CRUD、owner、生命周期或第二作者权威。
- 同样不得把“候选偏向 typed attribute”写成 D5 的既定结论；D5 保留从零比较 Record/RecordCollection 边界的权力，但任何替代必须证明如何与 D2 Document/Node authority、D4 Profile source 和导入/导出保持单一权威。
- 若条目需要稳定定位、局部修改、并发合并或引用，必须明确它是 value-internal key、Document locator、RecordRef 还是拒绝该能力，并给出 wire、生命周期、权限和冲突反例。
- 对 Organizations 的 `identifiers[]`、`classifications[]`、`relationships[]`、`sites/addresses/contacts`、`memberships/roles` 重复同一比较：列表或表格式 UI 不得暗生 Record identity、独立 CRUD、插件局部 ID 或第二作者权威；D5 仍可提出满足全部上游合同的独立 Record 替代并承担证明责任。
- Profile schema 自身若含 list-of-object、内建 key 或 relationship collection，也必须进入相同 D5 比较；Profile composition 不能成为绕过 Record/occurrence 边界的通用实体工厂。
- 对 ICS recurring master、derived occurrence 与 `RECURRENCE-ID` override，必须比较 series-owned structured value/Record、Document occurrence、独立 Node 及无独立 identity；稳定定位、局部 override、引用和同步需求不得自动推出 Node materialization。

### 8.2 D6 必须冻结的持久化与事务门

- 冻结 Profile membership、Profile 字段、关系事实和 TemporalScope 的物理持久化、revision/read-set、validation、preview、commit、recovery 与 repair gate；不得建立插件私有作者存储。
- 冻结插件 schema 不可用、未知或版本不兼容时的 read/edit/save/export/repair 行为：保存不得静默删除、重写、默认化或丢弃未知作者字段，repair 不得用当前插件状态冒充作者意图。
- 冻结关系事实的唯一写入位置；从 inverse 或 symmetric 端编辑必须经过授权重验和同一原子事务，不能双写两份关系事实或让不同端局部成功。
- graph、backlink、Calendar、Timeline、Family Tree 等派生 index/view 必须可删除重建，不得成为关系或 temporal 权威。
- 覆盖本地/托管、离线、同步、并发冲突、权限变化、target 隐藏/Trash/purge、插件启停和 schema 升级。
- 若最终采用 `(series, temporal scope)` 唯一性，唯一约束与并发 CAS 必须绑定规范 typed identity/value，不能依赖标题、建议 basename、路径、structural parent 或索引碰撞实现。
- 对 Organizations 另须冻结：组织关系事实唯一 owner/source；从 parent/child、ownership/subsidiary、membership 任一 inverse/symmetric UI 端编辑时的授权与原子事务；改名、撤销、合并、拆分、历史与 undo/redo；identifier/classification 冲突；组织图索引重建；离线/同步/并发冲突；structural move 不改变 `parentOrganization`。
- 对 Profile 另须冻结：declared set 的 exact-source persistence、effective closure 重建、manifest/schema unavailable、create/assign/remove/cleanup 原子 plan、并发 Assign、external edit damage、unknown字段保留、卸载/重装、schema migration 和五表面一致恢复；默认值绝不能在 read/validation 时隐式写入。
- 对 ICS 另须冻结 subscribe/sync、copy/import、promote/adopt 的 authority 与状态转移；source binding/UID/RECURRENCE-ID/SEQUENCE upsert、解绑/删除/取消、单向/双向编辑、离线冲突、receipt/retry 和 control state 必须与 NodeRef、作者内容及派生 occurrence 分域。

### 8.3 统一扩展模型与产品术语候选

当前讨论已经证明 `plugin` 同时指代核心语义、内置产品功能、第三方可执行代码、纯声明 schema、数据集、connector、Profile 和 Template 会造成权限、卸载、移动端与用户预期混乱。D4/D7/D8/D9/D10/A2 必须从零比较并裁决统一分类；以下是候选 inventory，不是已冻结产品命名或实现承诺：

1. **Core contracts（非插件）**
   - `Node/Document/Resource/Annotation`、Task/Template specialization 与 checklist 边界、Node Profile/schema 机制、Core Action/transaction、Query/View 基础，以及通用 Import `probe/plan/preview/commit/receipt` 属 Core 合同；它们不能因某个 UI 模块或 provider 停用而变成不可解析作者内容。
   - generic relationship graph projection 更接近 Query/View 基础或 bundled generic view；不得先假设它是某个 People/Organizations/Projects 领域插件，更不得因此让领域模块拥有通用关系事实。
2. **Bundled system modules（用户可见内置模块）**
   - 候选 inventory 包括 Tasks UI、Templates UI、People、Organizations、Calendar、Projects、Assets、Places、Library 等；其中单 Calendar 与 Time+Calendar 对照、Library 的正式中英文名和 IDs 仍按 §8.4 比较。当前 inventory 不含 Diary 模块。模块可贡献 Profiles/schema、editor、View、Action、import/export 与诊断，但不能拥有 Node/Document/Resource、建立私有作者数据库或绕过 Core transaction。
   - Task/Template 语义是已冻结 Core specialization；即使 Tasks/Templates UI 模块停用、卸载或当前 surface 不可用，source 仍须可解析、保留、查询和安全编辑。
3. **Extensions（第三方或可选行为扩展）**
   - 未来 Finance、Personal Health、Equipment & Maintenance、Engineering BOM、Taxonomy 等专业领域只作为架构压力案例。它们可以贡献受 capability/permission/sandbox 约束的行为，但不自动成为当前产品承诺，也不得引入新的 Node identity/kind 或第二写路径。Journal/Diary 不进入当前第一方或第三方 Extension 规划清单。
4. **Packs（无代码或受限代码的声明/数据包）**
   - 包括 Profile/schema pack、country organization pack、calendar-system rule pack、holiday schedule data pack、localization/view preset。pack 默认只能通过受限声明、规则或数据贡献能力，不取得任意代码执行、网络、凭据或直接写文件权限；确需这些能力时必须升级为显式 Extension/Connector 并重新授权。
   - pack 必须归属提供其 extension point 的父模块/领域，而不是一律平铺成 “Weftext pack”：例如 Calendar System/Holiday Schedule 是 Calendar packs，country organization schema 是 Organizations pack。Core Extension Manager 仅治理 package 生命周期，不接管父模块领域 schema；父模块缺失时 contribution fail closed 且配置可恢复。
5. **Connectors / Adapters**
   - 包括 Zotero/Crossref、ICS/外部 calendar/provider、Microsoft/Google contacts 等。connector/adapter 负责外部协议、映射、source binding 与同步；凭据、sync token、etag/cursor 在控制面，作者内容与外部 identity/provenance 仍经 Core/领域合同提交。
6. **Template、Profile 与 Preset 不得混同**
   - Template 保持冻结的 Core Template Node specialization；Templates 模块只提供 inventory/editor/instantiate UX。Extension 可以分发 Template 或 target-profile preset，但 Template 本体不是随 provider 卸载的隐藏语义。
   - Profile 是生命周期内持续、可验证、可查询、可组合的 schema；Template 是一次性实例化内容；Preset 只是创建时的默认选择/参数。Preset 不得在实例创建后继续成为权威，Template 不得冒充 Profile，Profile 不得覆盖作者正文。

产品命名候选：用户层以“扩展（Extensions）”作为总称，设置中明确区分“内置模块 / 扩展 / 扩展包 / 连接器”；`plugin` 仅在确有必要时保留为开发者 manifest/runtime 术语，不作为所有贡献的用户可见统称，也不得暗示所有 pack 都运行任意代码。一个 `Extension Package` 可声明 profiles、views、actions、templates、presets、packs、connectors 等 contributions；每项 contribution 必须单独声明 capability、permission、sandbox/runtime、依赖、版本、移动端可用性、卸载/未知-provider degradation 与 author-data retention，不能以 package 安装权限笼统授权全部贡献。

必须使用现有候选作 inventory 非回退与防膨胀测试：

- Business 仍属于 Organizations 领域，不另建 Business 插件；Public Organization 与国家特有字段优先是 Organizations Profile/schema pack；
- Living Things 优先作为 Profile/schema pack 候选；Medical Knowledge/clinical workflow 只作未来边界压力案例，不构成当前实现承诺；
- Journal/Diary 只保留为“Calendar period note + Template/Preset 已足够，不应新增扩展/Profile”的防膨胀反例；历法/节假日优先是 pack；ICS 是 Calendar importer/connector，而不是每个 ICS component 一个插件/Node；
- People、Organizations、Projects、Assets、Places、Library/References 候选是否应为 bundled module、Extension 或仅 Profile/pack 必须按 D1 产品面、代码/权限需求、离线/移动端和用户价值逐项裁决，不能因领域名存在就无限增生模块；
- 最终术语和分类必须在 wire type/kind、manifest contribution、设置 UI、Marketplace、权限提示、安装/启停/卸载、Mobile capability matrix、错误诊断和用户/开发者文档中一一对应；兼容旧 `plugin` 词汇时必须有单向、无歧义的迁移映射。

### 8.4 `Calendar / Library` 命名、反膨胀边界与 `Work` 领域模型候选

D4/D7/D8/D9/D10/A2 必须把产品名、package boundary、领域语义、capability、canonical package/Profile ID、UI label 与 wire type 分开评审，再给出稳定映射；不得从用户可见短名直接推导 identity/kind，也不得让一个 package 强迫 ontology 合并。上一版 `Time + Calendar` 及随后 `Diary Extension/Profile` 假设均已被用户修订。当前产品收敛选择是单一用户可见内置模块 `Calendar / 日历`；Diary/Journal 只作为“Calendar + Template/Preset 已足够”的防 Profile/Extension 膨胀反例，不进入第一方或第三方规划清单。

1. **单一 `Calendar / 日历` bundled module 候选（当前偏好）**
   - 在同一用户可见 UI/package 中统一提供：Periodic Notes（日/周/月/季/年等 calendar-defined period notes）、Range Notes（任意日期/时间范围笔记）、Events（recurrence/reminder/participants/status）、calendar/timeline views、ICS import/sync、Calendar System packs 与 Holiday Schedule packs。
   - 产品打包统一不等于数据语义合并。period note、range note、event 必须是不同 Profile/structured semantics；一个 range note 只有通过显式、可预览的 Assign Event Profile Action 才取得调度语义。Calendar View 命中、跨日或 title 模式都不得静默变成 Event。
   - 基础 period/range notes 不需要网络、通知、联系人或 provider 凭据权限。ICS sync、外部 provider connector、reminder/notification、participants integration 必须各自 capability-gated、按需授权并可独立不可用；单一模块不得扩大默认权限或让 Mobile/离线因为高权限子能力不可用而失去基础时间笔记。
   - Core 仍可拥有 generic temporal value、interval、timezone、calendar arithmetic 与 Query primitives，但用户不需要看到名为 `Time` 的模块；Core temporal types 也不得因此变成 Calendar 模块私有作者权威。
   - Profile namespace 必须比较统一的 `calendar/period-note`、`calendar/range-note`、`calendar/event`，与保留 internal `temporal/* + calendar/event` 等替代。优先保证产品清晰和稳定迁移，不得让 package boundary 自动决定 schema/ontology boundary。
2. **正式对照与 Diary 非规划边界**
   - `Time + Calendar` 双模块：分别承载 temporal notes/views 与 scheduled events/ICS；必须证明两个高度重叠入口、导航、权限和移动端 capability 的用户成本低于单模块。
   - `Diary / 日记` 不作为当前基础模块、第三方 Extension 或必需 Profile 方案。Calendar period note 本身拥有可写 Document，已覆盖日记、周记、月记、季记、年记、计划与回顾；“日记”是使用方式，不是当前成立的独立领域边界。
   - “每日记录”“季度回顾”“年度总结”等正文结构由普通 Template/Preset 提供；心情、习惯、天气等若只是可选字段，可由用户属性、schema preset 或轻量 declarative pack 提供，不因此建立可执行 Diary Extension，也不要求 `diary/*` Profile。
   - 同一 Calendar series + canonical period scope 只保留一个 period note identity；切换 Template/Preset、日记写法或回顾结构不创建第二 Diary Node，不产生镜像、同步或另一份 temporal extent。准确 series/scope 唯一键、并发和允许多系列边界由 D4/D6 冻结，不能靠标题/路径实现。
   - 只有未来出现 Calendar 无法合理承载的稳定独立行为，例如专门隐私域、跨周期 journal workflow、独立互操作格式且确需不同权限/生命周期，并经过新的完整架构评审后，才允许提出 Diary Extension；当前 intake 不预留、命名或承诺该 Extension。
3. **`Library / 文献库` 候选**
   - 覆盖 bibliographic Works、作者/机构/venue、PDF/Resource、citation、DOI/ISBN/arXiv、BibTeX/RIS/CSL、阅读工作流与作者作品视图。
   - 避免把正式模块命名为 `References`：reference/citation 是 `Work` 在某个 citation occurrence 中扮演的关系或角色，不是实体本体。用户自己的论文、草稿、已接收/已发表版本与外部文献由同一 Library 领域管理，不分成“我的论文插件”和“外部参考文献插件”。

Library 的底层通用实体/根 Profile 候选为 `Work`，示意 ID `library/work` 仅用于评审，准确 namespace、package ID、Profile ID、version 与迁移 alias 由 D4/D10 冻结：

- `My Works` 是由 authorship 关系指向当前用户对应的 Person/Profile，再结合 publication lifecycle 机械派生的 Query/View；它不是另一个 Node kind、Profile、复制集合或私有数据库。
- `draft | accepted | published` 以及 `article | book | report | standard | dataset` 等维度必须按 schema/行为差异从零比较普通字段、子 Profile、relationship 或其他替代；不得为每个状态、载体或学科制造插件/type 爆炸。
- 一篇自著论文从草稿持续演进到发表，若仍是同一作品语义，可保持同一 Work Node/NodeRef 并更新明确 lifecycle；若出版版本、edition 或 release 取得独立可引用身份，则通过显式 `version/edition/publication` relation、各自 identifiers/provenance 和必要 fresh NodeRef 建模，不得靠含糊复制或文件名区分。
- DOI/ISBN/arXiv/provider key 是外部 identifiers，不是 NodeRef；作者 Person、Organization、venue、citation occurrence、PDF Resource 与 Work 的 owner/identity/relationship 必须服从 D3/D4/D5 边界。

正式命名替代必须至少比较：`Library / 文献库`、`Works / 作品`、`Literature / 文献`、`References / 参考文献`、`Bibliography / 书目`。裁决重点包括普通用户理解、是否自然覆盖自著与外部作品、论文/书籍/标准/报告/数据集、阅读与引用工作流，以及是否会与 Weftext `Workspace`、文件库或普通附件管理混淆。当前产品选择偏向 `Library / 文献库`、底层实体 `Work`，但仍须 council/Gate 逐项 disposition。

中英文与 ID 一致性要求：

- 用户可见 label 可本地化，canonical package/Profile/wire IDs 不可按语言变化；当前候选的单一 `Calendar/日历` bundled module 与 `Library/文献库` 必须各有唯一 package contribution 映射。当前不得为 Diary 预留 package/Profile ID，Template/Preset 名称也不能冒充新的领域 identity。
- D4/D10 必须裁决诸如 `calendar/period-note`、`calendar/range-note`、`calendar/event`、`temporal/*`、`library/work` 的 namespace/grammar/version/alias/collision，不能把示意 ID 当冻结结果；旧 `time`、`time-notes`、`temporal`、`chrono`、`references` 等若存在兼容需求，只能有明确、单向、可诊断迁移映射，不能形成双写或两个有效 identity。
- D7 Query/View/Action、D8 导航/设置/创建器、D9 import/export/loss report、manifest/Marketplace/Mobile/CLI/API 与用户/开发者文档必须共享同一术语表；wire 使用 canonical ID，UI 显示本地化 label，diagnostic 同时给稳定 ID 与可读名称。

### 8.5 D1–D10 / A2 横切 `Terminology and Naming` Gate

术语与命名是正式架构对象，不是实现末期文案润色。内部技术文档、领域模型、wire/API、manifest、schema/Profile IDs、代码函数/类型/变量名、CLI、UI 文案及中英文本地化必须清晰、准确、无矛盾和歧义。历史实现、本讨论和模型输出都只是候选证据；已冻结 D1/D2 术语是不可静默回退的上游合同，新冲突只能提出显式重开请求。

#### 8.5.1 权威 Lexicon 产物与不变量

- D3 起由当前唯一阶段 writer 建立并逐阶段维护权威 `Terminology and Naming Lexicon`；准确文件路径、分类与索引位置必须按当时有效的 Design Review Execution Protocol、Handoff Convention 和 Weftext-Control 目录规范决定，不得由 intake 讨论任务擅自创建第二权威。D1/D2 已冻结术语先作为 imported/frozen entries，不在下游阶段暗改。
- 每个公共概念只能有一个 canonical term；同一裸 term 不得未经限定同时表示多个概念。不同层可有不同 display label、wire identifier 与 code symbol，但必须通过稳定 concept/term ID 做显式一对一映射，不能依赖上下文猜测。
- Lexicon 每个条目至少记录：
  1. stable concept/term ID；
  2. 中文正式名；
  3. 英文正式名；
  4. 精确定义；
  5. 所属层级与 owner；
  6. 与相邻概念的排除边界；
  7. canonical wire/API/manifest/schema 名；
  8. code symbol convention（type/function/variable/namespace）；
  9. CLI/UI label 与 locale key 映射；
  10. 允许简称；
  11. 禁止、退役或仅历史可见的别名；
  12. 例子与反例；
  13. 首次冻结决议、版本/状态及迁移/删除目标。
- Lexicon 不能只列 glossary 文案；必须可机械追踪到冻结决议、wire/schema、代码命名规范、本地化表、rejected alternatives、迁移计划、负向门禁和受影响文档索引。所有链接、`_weftext.id`、term IDs 与证据哈希必须可验证。

#### 8.5.2 当前强制碰撞矩阵

对应 owner 阶段必须逐项给出 `keep-qualified | rename | split | merge | retire | requires-upstream-reopen` disposition，不能把用户口语类比（例如 class/instance）直接固化成 ontology：

1. **`Profile` 三重碰撞**：D2 已冻结的 `Weftext AsciiDoc Profile`（语法子集）、正在讨论的 Node 可组合 schema 机制，以及 CEL/Value Profile。必须决定 Node 机制是否改名为 `Facet/Schema/Model/Kind` 等，或以严格限定名消歧；wire/code/UI 中不得继续裸用一个 `Profile` 指代多个概念。
2. **`type/class/schema/profile/facet/specialization/role`**：Person 等可组合 schema、Task/Template specialization、关系 role/qualifier、编程语言 type/class 必须互斥定义；用户 label 与 code term 允许不同但映射必须显式。
3. **`plugin/extension/module/pack/connector/adapter/provider/contribution`**：区分 package、用户可见 bundled module、第三方可执行 runtime、父模块-owned declarative/data pack、外部 connector/adapter、schema provider 与 manifest contribution。
4. **`Template/Preset/export template/profile defaults`**：区分一次性 Node Template 实例化、创建默认/Preset、Office/文件输出模板和持续 schema/default；不得共享同一 wire kind 或 CLI verb。
5. **`Node/note/document/item/record/entity/occurrence`**：D2 Node/Document、D5 Record 候选、Document occurrence、ICS derived occurrence 与 UI item/note 必须区分 identity、owner 与生命周期。
6. **`attribute/property/field/fact/metadata`**：作者事实、AsciiDoc header attribute、schema field、UI property、系统/控制 metadata 的层级与 source authority 必须明确。
7. **`relation/link/reference/citation`**：结构/领域 relation、Document link、NodeRef/ResourceRef、Library citation occurrence 与泛称 reference 不得混为一类边。
8. **`Resource/attachment/file`**：owner-local Resource、Document attachment/occurrence、物理 file/path/blob 与外部 URI 必须区分。
9. **`Time/Calendar/period note/range note/event/diary/journal`**：当前单 Calendar product、Core temporal values、period/range/event semantics、Diary/Journal anti-expansion 使用方式，以及 scholarly journal 载体必须互斥。
10. **`calendar system/calendar view/calendar source`**：历法规则 pack、派生 View、ICS/provider authority/source binding 必须有不同 concept/IDs。
11. **`Library/Work/Reference/Citation/Bibliography/Publication/Literature`**：特别检查 `Work` 与普通“工作/项目”的英文碰撞；Library 自著/外部作品、publication lifecycle/version、citation role 与 bibliography output 必须分域。
12. **`import/copy/adopt/promote/subscribe/sync/connect`**：操作意图、authority 转移、是否 fresh identity、是否保留 source binding 与 retry/upsert 语义必须在 verb 层清楚区分。
13. **`owner/authority/source/provenance/origin`**：对象 owner、当前作者权威、输入/外部 source、provenance evidence 与 origin binding 必须有精确边界，不能用 `source` 覆盖全部含义。

#### 8.5.3 阶段流程与评审门

1. 每个 D1–D10 阶段只要新增、重命名、拆分、合并或退役公共概念，必须先更新 Lexicon candidate 与 collision/rejected-alternative ledger，再形成候选 bytes 和 blind-review package；不得在 Gate 后补写术语。
2. 每位独立 OpenCode 评审者与最终 GPT-5.6 Sol / Pro Gate 都必须明确检查：一词一义、相邻概念排除边界、跨层一对一映射、中英文本地化、普通用户理解、wire/manifest/code/CLI/UI 一致性、退役 alias 与迁移可行性。
3. D1/D2 只做已冻结术语的只读非回退 audit；发现碰撞时记录最小反例和显式 reopen request，不由 D3–D10 静默改名。D3–D10 的新术语按各阶段 owner 边界冻结；同一时刻仍只有当前阶段 writer 修改 Lexicon 与控制索引。
4. A2 必须制作自包含概念图和双语 term matrix，从零检查是否能用更少、更统一的概念体系满足所有场景；跨阶段重复、隐式同义词、循环定义、过度限定和领域膨胀都必须进入 P0/P1 closure。顶层实质改名按协议使用另一真正全新的 Sol / Pro 对话完成非回退 Gate。
5. 公共 wire/API/manifest/Profile/schema IDs 一旦随阶段冻结即稳定；display label/localization 可按受控兼容规则演进，但不得借 UI 改名改变 wire identity。尚未冻结的聊天用词不能因为先出现就获得兼容承诺。

#### 8.5.4 自动负向门禁候选

- 冻结后建立受控 `rg`/lint/codegen/schema/locale tests，检测：退役 identifier、wire field 漂移、manifest contribution 名漂移、代码 type/function/variable 与 convention 不一致、CLI verb/flag 与 wire action 不一致、locale key 缺失、中文/英文含义漂移、Lexicon/决议/文档链接断裂。
- 规则必须绑定受控目录、identifier/token、schema field、manifest key、symbol AST 或 locale key；不得粗暴禁止自然语言正文、历史证据、迁移说明、rejected-alternative ledger 或用户内容中的普通词。
- 负向门禁需给 allowlist/历史引用机制、误报测试、迁移截止版本和 deletion target；禁止词必须能追溯到 Lexicon entry 与冻结决议。
- 最终交付必须包含 canonical glossary/Lexicon、本地化 term matrix、rejected alternatives、代码命名规范、自动非回退测试、迁移/删除清单及受影响文档索引，并完成链接、IDs、hash 与会话证据验证。

| 阶段 | 强制 owner / cross-check |
| --- | --- |
| D1/D2 | §8.5 只读非回退 audit；若后续候选已经形成成立的顶层反例，则按 Current Roadmap 先做正式 replacement review。当前 Node control placement、Task ontology 与 persisted scoped attribute syntax 已路由到 D2 formal reopen；replacement pass 前原 frozen entries 继续生效。 |
| D3 | 仅接收 §7.3 identity/provenance 与 §8.5 identity-terminology 自包含切片：在当前冻结前裁决 ICS `UID`/source binding 与 NodeRef 分域、derived occurrence 无 Node identity、promote/adopt fresh NodeRef/provenance；建立 Lexicon candidate 并裁决 `Node/note/document/item/entity/occurrence`、`relation/link/reference`、`Resource/attachment/file`、`owner/authority/source/provenance/origin`。不得读取或处理本文其他后续产品场景。 |
| D4 | owner：在 replacement D2/D3 边界上完整裁决 schema/People/Organizations/关系/Calendar/Library 与 terminology。按 §13 冻结 `people` 仅是 namespace/group、每个 `people.xxx` 是独立 canonical field/occurrence；排除单一 `people` atomic attribute/blob，但不再由 D4 选择 D2 persisted source grammar。 |
| D5 | owner/cross-check：比较 typed value/Document occurrence/Record/Node、occurrence ID/note/Annotation，并证明 names/phones/addresses/relationships 等独立 fields 与重复 entries 可逐项寻址、diff/merge/query/partial update，不因 source block 暗建 Record identity或整体原子值。 |
| D6 | owner：冻结持久化、Action、authority、权限、历史、索引、同步/冲突。按 §13 保证每个 namespaced field/occurrence 可部分更新与授权；scoped-block 仅作可逆 source grouping，不把整个 People 数据锁成单事务 blob或扩大冲突域。 |
| D7 | owner：冻结 Query/View/Action/Graph/Calendar/Library/Search projection。按 §13 验证 Family Tree 只消费 directed kinship、一般图消费 social symmetric edges、层级图消费 professional directed edges、colleague 从 engagement 派生且可重建，并闭合 search contributions。 |
| D8 | owner：冻结五表面交互与本地化。UI 按姓名/联系方式/关系/任职等语义 section 展示而非机械等同 source block；需要时可揭示 canonical field ID/owner。第三方 contribution 显示在 People 表单仍保持自身 namespace。 |
| D9 | owner：Template/schema/Preset 与 imports。冻结 legacy single-`people` blob → independent fields 的 preview/migration/loss、flat/scoped-block 等价转换、block copy/template与 unknown-field export；禁止整包覆盖或按 label 猜映射。 |
| D10 | owner：冻结 Extension Package/manifest/contribution/runtime、namespace registration、official/publisher/user ownership、anti-spoof、schema/version compatibility 与 unknown-provider lifecycle。provider 不得写私有 control blobs或伪装官方 namespace。 |
| A2 | 从零重审完整组合，重放 §12/§13、People facts/relationships/search、field namespace A/B/C 真实 source、unknown provider/external AsciiDoc/diff/merge/copy/query、恶意 YAML、Mobile/import/concurrency；验证全链非回退。 |

## 9. A2 必须重放的端到端反例

1. 同名联系人、多个名称、两个手机号、一个纯文本 parent 和一个 NodeRef spouse；插件关闭、重开、导出、导入后作者事实与关系不丢失、不自动合并。
2. parent Node 对调用者不可见但 spouse 可见；Family Tree、Graph、反向查询和卡片均不泄露隐藏身份，也不把文本 target 猜成 Node。
3. Project Profile 同时有 structural child、member、dependency 和正文 link；Graph 可筛选且不会把四类边压成 `contains`。
4. 同一 Calendar day period note 同时出现在月视图与 ISO 周视图，Node 被移动、重命名或切换“每日记录”Template/Preset 后 temporal scope 与 period identity 不变；DST/周跨年仍得到唯一机械结果。
5. 同一 series+scope 在唯一与允许多篇两种政策下的并发创建；离线/重试/冲突不能静默复制或错误合并。
6. 任意跨时区 interval 与全日区间在 Calendar、Timeline、overlap Query、导出和重新导入后保持一致；派生 duration 不成为第二作者权威。
7. 从 Person 节点的 PDF Resource 创建新节点：源附件留在原 owner；显式复制才在目标产生 fresh Resource；retry 幂等，新 operation 可再次创建，结果不会自动递归导入。
8. 外部联系人首次导入后本地补充 note，随后 provider 改邮箱并删除远端自定义字段；同步只更新映射字段，保留本地 note，凭据/etag/cursor 不进入工作区作者内容。
9. 插件/Profile/connector 不可用时，同一工作区在 Desktop、CLI、Server/WebUI、Mobile 上保持相同内容语义；不可用不得伪装成空值、删除字段或产生第二写路径。
10. 比较普通 Node+Profile-field relations 与独立 RelationType registry 的完整系统成本；若后者胜出，必须证明如何避免无限 ontology、用户不可理解的类型爆炸和跨插件冲突。
11. 一个 Node 同时声明 People 与 temporal/calendar Profile，People provider 未安装、Calendar 模块停用；membership 与未知作者字段仍可读、保存和导出，`wf-specialization` 仍为 null/task/template 且绝不被 Profile 占用。
12. 两个设备并发编辑 `phones[]` 的不同条目，再由 provider connector 更新 email；分别以 typed value、Document occurrence 和独立 Record 替代复算身份、冲突、权限、receipt 与作者权威，禁止 UI 列表形态决定领域。
13. 从 spouse 的 inverse UI 编辑关系而另一端权限被撤销，同时 graph/backlink index 丢失；提交必须原子授权失败或成功，重建索引后只出现一份权威关系事实。
14. 两个 writer 并发创建同一 series+scope 的时间笔记且标题/路径不同；若政策唯一，只能由 typed scope gate 决出一个结果；若政策允许多篇，不得被路径或索引错误合并。
15. 对 People、Calendar（合并此前 Time Notes/Time/Calendar 候选）、Graph 分别裁决其是否只是 D1 可选能力实例；任何成立的产品表面变化必须形成显式 D1 重开请求，不能由模块安装事实暗改 D1。
16. 用企业、大学/学院、政府/内部单位、非营利、国际组织和临时委员会同时实例化最小 Organizations base；国家 pack 停用后，通用字段仍可读写，未知 namespaced scheme 原样保留且不被误判为全球 enum。
17. 移动 Organization Node 的 structural parent，而 `parentOrganization` 不变；Organization Chart 与下级查询必须保持语义关系。显式改变 `parentOrganization` 时不得暗移 Node 或物理目录。
18. 同一公共机构同时存在主隶属、上级业务指导、属地政府和双重领导；Graph/Organization Chart/filter 必须保留 edge kind，不能压成一个 parent/contains，也不能因为中国公安 fixture 扩张全球 ontology。
19. 企业改名后被合并、旧品牌保留、identifier 历史复用；NodeRef、status/validity、merge Action、relationship 与 source evidence 必须区分，不按名称或外部 identifier 自动合并。
20. `管财领导`、`行政内勤`、`董事`、`经理` 以 membership role scheme/value 表达并与 Person Node 权限交叉；验证职位新增不需要新增字段，inverse edit 同一事务，Person/Organization 都无插件私有身份。
21. Organizations 插件、国家 schema pack 与工商 connector 分别未安装、停用、版本不兼容或权限撤销；作者字段不丢失，connector state/credentials 不入内容，repair 不以当前 provider 状态覆盖作者事实。
22. 对 Organizations 官方可禁用模块单独作 D1 非回退裁决；若其只是既有插件能力实例则列出映射，若改变正式产品面则提出显式重开请求，当前 intake 不自行批准。
23. 完整重放 §2.6 Profile fixture matrix；特别比较 `0..N` 无序无优先级集合与单 Profile/Node kind/自由属性替代在 source bytes、冲突、权限、诊断、实现成本和用户理解上的全局表现。
24. Create Node 同时选择一个 Template、People Profile 与 hypothetical `example.custom/entry` Profile；Template 初始字段与 Profile 类型冲突时只能 preview 后明确拒绝/修订，不能让模板或安装顺序成为隐藏优先级。该 generic composition fixture 不得被解释为 Journal/Diary 产品承诺。
25. unknown Profile 的 Node 在未安装 provider 的 CLI/Mobile 上编辑普通正文，再在已安装 provider 的 Desktop 上 Assign 另一个 Profile；declared membership 与未知字段必须 byte-preserving，effective closure、capability 与 typed diagnostics 可重建且不回写。
26. Task Node 同时添加 Project work-item Profile；source 只含一个 `wf-specialization: task` 和独立 Profile membership，Task schema 与 Profile schema 冲突时 fail closed，不生成重复 `profile=task`。
27. 一个普通旅行 Node 先只声明 `time/range + travel/trip`，在 Calendar/Timeline 中可见但没有邀请、提醒、参与者或 status；随后显式 Assign `calendar/event` 才取得调度语义，移除 Event Profile 后保留作者时间范围且不改变 Node identity。
28. 同一日期同时启用公历、农历和两个地区 Holiday Schedule pack；派生结果按 pack/version/source namespaced 共存，停用或升级一个 pack 不改写 Node Profile/attribute，也不让单一 `isHoliday` 覆盖冲突结果。
29. all-day `[2026-03-01, 2026-03-04)` 在 UI 显示 3 月 1–3 日，zoned instant interval 跨 DST gap/fold，另有一个 open/ongoing range；五表面、overlap Query、导出/导入与移动端离线均保持同一边界和诊断。
30. recurrence series 含 exception、取消单次 occurrence 和跨时区查看；expansion horizon/cache 被删除重建后引用与权限仍一致，离线编辑与并发更新不得把派生 occurrence 升格成第二作者权威或隐式新 Node。
31. 导入一个无限 `RRULE` recurring master、多个 `RDATE/EXDATE` 和两个 `RECURRENCE-ID` override；subscribe 默认不创建 occurrence Nodes，展开受 horizon/limit 约束，删除派生 cache 后 occurrence set 和 override 仍可确定重建。
32. 同一 ICS source 重导入时 master `SEQUENCE` 增长、一个 occurrence 被取消且 Node title/path 已改；以 source binding+UID+RECURRENCE-ID 确定 upsert/conflict，不重复建 Node。另一 source 使用相同 UID 时不得跨源自动合并。
33. 对同一 `VEVENT` 分别执行 subscribe/sync、copy/import 和 promote/adopt selected occurrence；三者的 authority、NodeRef、provenance、解绑/删除、双向编辑和 receipt 必须可区分，ICS UID 任何时候都不替代 NodeRef。
34. 一个 `VCALENDAR` 同时含 `VEVENT`、`VTODO`、`VJOURNAL`、`VFREEBUSY`、`VTIMEZONE`、binary/URI attachment 与未知 `X-` property；preview 分类型展示映射/loss/安全策略，不能“一组件一 Event Node”，恶意超限 recurrence 必须 fail bounded。
35. 停用 Tasks UI 与 Templates UI bundled modules 后，Task/Template source、checklist、query 和 Core instantiate contract 仍可解析保留；重新启用只恢复 UX，不迁移 identity 或改写作者 bytes。
36. 一个 Extension Package 同时分发 Equipment Profile、View、Action、Template、Preset、Calendar holiday data pack 和外部 connector；每项 contribution 显示不同 capability/permission/runtime，拒绝 connector 网络权限不得阻止纯数据 pack/Template 被安全读取，也不得笼统授予 package 全部权限。
37. 在 Settings、Marketplace、CLI、Mobile 与开发者 manifest 中安装/停用 People bundled module、Equipment Maintenance Extension、China Organizations schema pack、Calendar Holiday Schedule data pack 和 ICS Connector；名称、父模块归属、wire kind、权限、降级、卸载保留及 unknown-provider diagnostic 必须一一对应，当前清单不出现 Diary Extension。
38. 对 Business、Public Organization、Living Things、Medical Knowledge、Journal/Diary、历法、节假日和 ICS 做 inventory：不得因每个领域名建立一个可执行插件；Business 留在 Organizations，国家字段/Living Things 优先比较 pack，Journal/Diary 由 Calendar+Template/Preset 覆盖，Medical/clinical 不产生当前承诺，历法/节假日归 Calendar packs，ICS 归 Calendar importer/connector。
39. 一个 Template、一个 Profile 与一个 Preset 都建议相同字段但值冲突；preview 明确一次性生成、持续 schema 和创建默认的来源与优先/拒绝规则，实例完成后 Preset/Template 不成为隐藏权威，卸载分发它们的 Extension 不删除作者内容。
40. 一个普通 range Node 与 period note 都由单一 Calendar/日历模块呈现；range Node 只有显式添加 Event Profile 后才出现提醒/参与者/status。相同 package 中三类 semantics、Query 与 Action 保持分域，旧 `Time`/`Chrono`/`Time Notes` 只作为可诊断迁移 alias。
41. 用户自己的论文从 draft 进入 accepted 再 published，authors 指向当前 Person；`My Works` View 机械更新但 Work NodeRef 不变。随后出版 edition 获得独立 DOI 和可引用分页时，通过显式 version/edition/publication relation 与 fresh NodeRef 表达，不靠复制标题或 PDF 路径猜身份。
42. 同一 Library 同时包含外部 article、用户草稿、book、standard、report 和 dataset，并通过 BibTeX/RIS/CSL/DOI/arXiv 与 PDF Resource 导入；`Work`、citation occurrence、author Person/Organization、venue、identifier 和 Resource 不混为一个 entity/type，也不为每种载体生成插件。
43. 在中文/英文 Desktop、Mobile、CLI/API、Settings、Marketplace、manifest 和导出诊断中重放单一 `Calendar / 日历` 与 `Library / 文献库`；用户看不到独立 Time 基础模块，canonical IDs 不随语言变化，Library 不与 Workspace/普通文件库混淆，旧 `time/references/temporal/chrono` alias 不形成第二 identity 或双写。
44. 对 `Library/文献库`、`Works/作品`、`Literature/文献`、`References/参考文献`、`Bibliography/书目` 做盲测式产品命名比较，必须覆盖自著与外部作品、标准/报告/数据集、阅读和 citation 角色；偏好 Library 仍须用明确 disposition 与反例证明。
45. 同一 Calendar module 中分别创建 day/week/month/quarter/year period notes、财政周期 range note 和 scheduled Event；固定周期唯一性、任意 range 多篇政策、Event recurrence 各自适用，不因共同 package 使用同一 type、唯一键或默认权限。
46. 基础 Calendar notes 在无网络、无通知权限的 Mobile 上正常创建/查询；随后仅授权 ICS connector、拒绝 reminder notification、撤销 participants integration。各 capability 独立降级，停用高权限贡献不删除 period/range/event 作者内容或让整个 Calendar module 不可用。
47. 在单 Calendar 与 Time+Calendar 双模块间重放导航、创建、查询、卸载、Mobile、ICS、Calendar/Holiday packs 与权限提示；Diary 基础模块仅作为应被现有能力覆盖的负面反例。当前单 Calendar 选择只有在 ontology/capability 未被错误合并且全链无 P0/P1 时才能冻结。
48. 一个 day period note 套用“每日记录”Template/Preset，一个 quarter period note 套用“季度回顾”，并加入可选心情/天气属性；切换模板或停用提供这些声明的轻量 pack 后仍是同一 Calendar period Node，不产生 `diary/*` membership、第二 Node、镜像或同步。
49. 安装两个 Calendar System packs 和一个 Calendar Holiday Schedule data pack，manifest 分别声明父 Calendar extension point 与兼容版本；Calendar 未安装、被停用、版本过低和 Mobile 不支持其中规则运行时，package 分别进入确定状态，不注册领域贡献、不注入 Node 语义，用户配置与来源仍可在全局扩展中心恢复查看。
50. 从 `Calendar → 扩展 → 历法/节假日` 禁用或卸载一个 Holiday Schedule data pack，再禁用整个 Calendar module；派生节假日/工作日显示与 cache 消失但 period/range/event 作者内容不改写。全局扩展中心始终标明包“用于 Calendar”，Core Extension Manager 不解释 holiday 领域值。
51. 在同一候选包中同时出现 D2 `Weftext AsciiDoc Profile`、Node 可组合 schema 候选和 CEL/Value Profile；评审必须禁止裸 `Profile` 继续三义，比较 Facet/Schema/Model/Kind 与限定名后为 wire/code/UI/中文建立一对一映射，且不靠用户口语 class/instance 决定 ontology。
52. 一个 ICS occurrence 被 `subscribe` 后再 `promote/adopt`，另一个 PDF Resource 被 `copy/import`；逐层检查 owner、authority、input source、source binding、provenance、origin、fresh identity、sync/upsert 和 CLI verb，任何一个裸 `source` 或 `import` 同时承载多义都必须 fail Gate。
53. 对 Node/note/document/item/record/entity/occurrence、Resource/attachment/file、relation/link/reference/citation、attribute/property/field/fact/metadata 生成双语 term matrix 和概念图；每对相邻概念至少有一个正例和反例，并可追踪到 D1–D7 冻结 owner。
54. 同一 canonical action/field/contribution 在 schema、OpenAPI/wire、TypeScript/Rust 等代码 symbol、CLI、Desktop/Mobile locale key 和文档中故意制造一处命名漂移；受控 lint/schema/codegen/locale test 必须精准失败，同时历史 evidence 和用户正文中的自然语言同词不得误报。
55. Library `Work` 与普通 project work/task、`Reference` 与 NodeRef/正文 link、`Publication` 与 published lifecycle/edition 同时出现；中英文 UI、code types、wire IDs 和 Query names 必须无需语境猜测即可区分，或触发 rename/qualification disposition。
56. 一个 Node Template、创建 Preset、Office export template 和 schema defaults 同时参与 preview；每个概念的 CLI verb、wire kind、code symbol、UI label 与生命周期必须不同且可映射，不能都叫 template/profile defaults。
57. 退役一个早期 wire alias 与一个 UI 旧名：Lexicon 记录首次冻结、兼容范围、warning、迁移截止与 deletion target；负向门禁只扫描受控 identifiers/locales，不禁止 migration note、rejected ledger 或历史 evidence 中的旧词。

## 10. 冻结验收门

对应阶段不得声称本文已关闭，除非：

- 适用条目均有逐项 disposition 与可定位关闭证据；
- People V1 字段目录、Profile membership/wf-specialization 分域、插件 enablement 配置、D5 域比较、D6 transaction gate 与 D1/D2 compatibility 均有显式 disposition；
- 可组合 Profile 候选的 `0..N` set、ID/version、declared/effective closure、依赖/继承/互斥/冲突、unknown/uninstalled、create/assign/remove/cleanup、Task/Template 边界、YAML import 和 §2.6 全部 fixtures 均有逐项 disposition；不得把用户示意语法直接当作冻结 wire；
- 单一 Calendar 模块内的 period note、任意 range note 与 Event 已分域裁决；all-day date range、zoned instant range、`[start,end)`/inclusive display、open/ongoing、DST/timezone、recurrence、overlap 与 Mobile offline 均有精确 wire/state/error/test closure，且未发明独立区间笔记 identity；
- Calendar System/Holiday Schedule 的受限 Calendar pack、dependent extension、Workspace/View context、多 pack namespaced/provenanced 输出、版本/来源/适用区间和不重复存派生 attribute 均有逐项 disposition；
- Calendar System/Holiday Schedule 已明确作为 Calendar-owned extensions/packs；父模块 extension point、manifest extends/requires、依赖/版本/Mobile/权限状态、Calendar 未启用时 fail-closed、配置保留、Calendar 内 UI 与全局中心归属、卸载不改作者内容及 Core Extension Manager 的通用治理边界均有 closure；正式文案使用 “Calendar Holiday Schedule data pack”，不再使用模糊的 “Weftext 节假日扩展包”；
- ICS 的 logical component、UID/source binding/NodeRef 分域、master/derived occurrence/override、subscribe/copy/promote authority、确定 upsert、非 VEVENT 映射、资源上限、附件与未知 property 均有逐项 disposition；默认和无界策略不得把 recurrence occurrence 物化为 Node；
- §8.3 的 Core contract、bundled module、Extension、pack、Connector/Adapter、Profile、Template 与 Preset 已逐项分域；用户命名、wire/manifest contribution、Settings/Marketplace、capability/permission/sandbox、卸载/unknown-provider、Mobile 与文档术语一致，并通过防领域数量膨胀 inventory；
- §8.4 的单 Calendar 与 Time+Calendar 对照、Diary/Journal anti-expansion 反例、Calendar+Template/Preset/可选声明字段覆盖、同 series+scope 单一 period identity 及未来重新提案门槛已逐项裁决；Calendar package 内 period/range/event 语义、基础 notes 与 ICS/reminder/participants capability、canonical package/Profile/wire IDs、旧 alias 迁移和全表面术语均有 closure；Library 的 Work、authorship/My Works、publication lifecycle、version/edition/publication relation、external identifiers、citation occurrence与自著/外部作品统一模型亦已闭合；
- §8.5 的权威 Terminology and Naming Lexicon 已按控制规范落盘并进入控制索引；所有 required fields、D1/D2 frozen imports、D3–D10 term changes、13 组高风险碰撞、双语 matrix/概念图、rejected aliases、迁移/删除目标、代码命名规范、本地化映射、受控负向 gates 与受影响文档索引均有逐项 closure；每个公共概念一词一义、跨层显式映射，且评审/Gate 证据明确包含 terminology disposition；
- §12 的 A reserved AsciiDoc header、B closed `_weftext` Node envelope、C D6 sidecar/control-plane 与更优替代均已在 pre-D4 formal D2 replacement review 中逐项 disposition；最终 canonical source、specialization placement、single-authority、portable copy/export、external edit、exact-source/hash、provider-missing、parser/repair/security/performance/Mobile/interop 已闭合。若 replacement 影响 D3，impact-scoped D3 amendment 也必须先完成；不得以 pending request 启动或声称 D4 pass；
- §13 的 People facts/search/relationships/optional note 均有 disposition；`people` 已明确只作 namespace/group，不是单一 Profile-data attribute。names/phones/addresses/relationships 等为独立 canonical fields，重复 entries/occurrences 可逐项寻址；flat source 与 scoped readable block 解析语义等价，整个 People atomic blob 被明确拒绝或有完整反证。partial update/permission、annotation/note、diff/merge/query/migration/removal 均闭合，领域 facts 不进入 `_weftext`；
- Organizations 的产品分层、最小字段缩减、namespaced scheme、role membership、组织关系分域、semantic-parent 单一权威、D5/D6 路由、D1 非回退和 114-note 压力证据均有显式 disposition；具体国家枚举、完整表单、connector 与实现仍留在 A2 后任务；
- 候选及盲审包包含本文相关场景、反例和正式替代；
- wire/source/状态/错误、实现影响和测试轮廓机械闭合；
- 独立 OpenCode panel 达到项目阈值，失败/空输出不计；
- 全新 GPT-5.6 Sol / Pro Gate pass，且没有未关闭 P0/P1；
- 若发生顶层实质修改，已在另一全新 Sol / Pro 对话完成非回退 Gate；
- 冻结文件、控制索引、链接、`_weftext.id`、候选/包哈希和会话 URL/model/tier 证据均通过验收。

## 11. Supersession、排队与交接控制

本节是此前多轮用户产品讨论的单一汇总控制指令。它不构成第二设计 writer、冻结决议或对当前 D3 的范围扩张；若本文早期候选与本节冲突，按下列后项 supersession 解释并在未来 D4 自包含 intake 中只保留最新语义，旧表述仅作为 rejected/history evidence：

1. 产品候选不再并列 `Time + Calendar`，收敛为单一 `Calendar / 日历` bundled module；内部严格区分 period note、range note、event。当前不规划独立 Diary/Journal module、Extension 或 Profile；日记、周记、月记、季记、年记、计划和回顾由 Calendar period note + Template/Preset/属性或轻量声明 pack 覆盖。
2. Calendar System/Holiday Schedule 是 Calendar child extension points/packs，不是平铺的 Weftext domain extensions；Weftext Core/Extension Manager 只提供通用 package 安装、签名/信任、版本、依赖、权限与生命周期框架。
3. `Library / 文献库` 是当前显示名候选，底层概念候选为 bibliographic `Work`；外部文献与作者自著作品统一管理。`Reference` 只表示被引用角色/occurrence 候选，不预设为实体名。
4. Template 数据模型与 instantiation 是否继续是 Core meta-object specialization 是待复核候选；Templates UI 可是 bundled module。Task 则必须按 §13 比较当前 Core specialization B1 与 bundled Tasks schema B2，不能与 Template 结论捆绑。持续 schema、一次性 Template、创建 Preset 与 export template 必须分域。
5. Node 可组合领域 schema 机制必须评审，但 `Node Profile`、`wf-profiles`、`people/person` 等都未冻结。必须先消解它与 D2 `Weftext AsciiDoc Profile`、CEL/Value Profile 的术语碰撞；membership placement 由 §12 A/B/C 比较，当前用户偏好 closed `_weftext` Node envelope，任何结果都不得形成 YAML/header 双权威。
6. People 等 module/extension 是 schema/provider，不拥有 Node；具体 Person Node 是 Workspace 表示实例。`class/instance` 仅是用户口语类比，不得未经术语与 ontology 评审进入 wire/code/UI。
7. Extension taxonomy 候选为 Core contracts、bundled modules、Extensions、parent-owned child Packs、Connectors/Adapters 与 contributions；`plugin` 是否只保留开发者 runtime 术语由 §8.5/D10/A2 裁决。
8. ICS 必须区分 subscribe/sync、copy/import 与显式 promote/adopt；recurring UID/series 不按 occurrence 无界建 Node，UID/RECURRENCE-ID/source binding 与 NodeRef 分域。
9. D1–D10/A2 必须维护权威双语 Terminology and Naming Lexicon 与横切术语 Gate，覆盖 docs、domain、wire/API、manifest、code、CLI/UI/localization、退役 alias 与受控自动负向检查。
10. 当前停止扩张更多领域清单。People、Organizations、Calendar、Projects、Assets、Places、Library、Tasks，以及 Finance/Health/Equipment/BOM/Taxonomy 等架构压力案例已经足够；后续评审优先消除概念重复、第二权威和 module/Extension/Profile 膨胀，不因新领域名增加产品承诺。

### 11.1 D3 未完成期间

- D3 任务 `01a04350-8f30-7da2-b68e-411db695f0c5` 保持唯一 `knowledge/Weftext-Control` writer。当前 controller 不为本 bundle 写任何控制文件、不启动 D4、不创建第二 D3，也不让其他阶段并发写控制区。
- D3 只处理已定向送达的 §7.3 ICS identity/provenance 与 §8.5 D3 terminology 切片；本 bundle 其余内容排队，不反向扩张 D3。
- 本 `reviews/` 文件只是 staging intake 和 supersession evidence；没有 `_weftext.id` 不会被误报为未来正式 D4 cross-domain intake，也不得被阶段候选引用为冻结权威。
- 按 `$council` 事件驱动监控 D3 targeted reviews、closure、后续 Sol / Pro Gate 与 evidence；失败/空输出不计，当前新的 P1 必须由 D3 自主裁决、修订和复审。

### 11.2 D3 完成后的严格顺序

状态：本小节原来的“先创建 D4 intake/任务，再由 D4 请求上游重开”顺序已由 §11.4 supersede；保留以下文字仅作形成分诊结论的历史证据，不再作为当前 dispatch 指令。

1. controller 先独立验收 D3：冻结文件/边界、全部有效与失败 OpenCode session 归属、panel sufficiency、Sol / Pro model+tier+URL、candidate/package/corpus hashes、P0/P1 closure、链接、`_weftext.id`、实现影响和 protected-path 边界全部机械通过；未通过则退回同一 D3，不启动 D4。
2. 在没有阶段 writer 并发的窗口，按持久权威规则只读查明并原子修复控制状态不一致：`Weftext-Control.md` 当前仍称 R3 profile-aware editor/migration active，而 `01 Dashboard` 与 `Current Roadmap` 当前仍称 D3 是 next。不得采信任一旧 Dashboard 自报；以冻结决议、有效 Gate evidence、Architecture Decision Ledger 与协议优先级决定唯一当前状态，并同步所有受影响索引。
3. 路径按届时有效控制规范决定，创建一份有唯一 `_weftext.id` 的**单一、非决议 D4 cross-domain intake**。它从本 staging file 的最新 supersession 生成自包含 bytes，列出来源/hash、候选/已选择/已拒绝/未决边界，不散落为聊天摘录或多份二级权威。
4. 为 D4 创建真正全新的 Codex project-local task，显式使用 `$council`。D4 必须把该单一 intake、冻结 D1–D3 必要合同、Terminology Lexicon candidate 与 acceptance rubric 组成 immutable blind package，首先完整比较 §12 A/B/C 与 upstream impact，再处理其余 schema/domain 候选。
5. 若 §12.A 或另一不重开上游的方案胜出且 §13.B1 保持，D4 可继续闭合；若 §12.B 或 §13.B2 任一胜出，当前 D4 只提交 formal D2/D3 reopen request，随后串行执行全新 D2-reopen council、受影响 D3 amendment/revalidation council，再创建全新 D4；若 §12.C 胜出并要求 D6 前置，按获批 roadmap reorder 创建独立 D6 boundary task，再创建全新 D4。Task B2 与 YAML B 互不蕴含，必须重放 `(A|B|C) × (B1|B2)`。任何 invalidated D4 package/session 不复用，也不计最终 Gate。
6. D4 acceptance 后，按 Current Roadmap 串行创建 D5–D10；每阶段只读取必要冻结上游合同和路由给自己的 intake/lexicon 条目，不继承模型会话。A2 读取完整自包含 bundle 做全局从零概念图、双语 term matrix、整体最优和统一重构可行性审查。
7. 每个阶段 final acceptance 都验证：candidate/package/corpus hashes、精确 OpenCode session IDs/有效性、Sol / Pro model+tier+URL、P0/P1 closure、冻结文件/控制索引、链接、`_weftext.id`、protected paths 与 supersession 无回退；通过后才自动推进。

### 11.3 用户对完整 People/Profile/Attributes bundle 的正式处理确认

用户已明确确认：本对话逐条形成的 People/Profile/Attributes 讨论不是可丢弃的聊天建议，必须统一进入下一适用设计阶段的自包含 intake、candidate、alternatives、wire/state/error、closure ledger、blind-review package 与最终 Gate。当前 `reviews/` 文件是总控在 D3 active/quota-paused 期间维持单写者约束的 staging carrier，不是冻结权威；D3 不追加处理。

D3 独立验收并完成控制状态一致化后，总控创建的单一、带 `_weftext.id`、非决议 D4 cross-domain intake 必须至少完整覆盖：

1. §12 closed `_weftext` Node-control envelope 与 memberships/profiles placement A/B/C；
2. 可组合 schema/Profile 机制及其与 Weftext AsciiDoc Profile、CEL/Value Profile、Task/Template specialization 的术语和 ontology 冲突；
3. People/Tasks/Calendar/Organizations 的 bundled-module/extension/schema provider 边界、停用/未知 provider 保存与 Core 单一写入权威；
4. People names/contact phones-emails-IM/addresses/important dates/profession-engagement/relationships 的完整场景；
5. event assertions、temporal state、measurements、repeatable entries、relations 的时间、精度、历法、单位、有效期、来源/provenance、confidence 与 current/preferred projection；
6. 每条 attribute value occurrence 的 optional note、stable target、inline note vs Annotation、重排/复制/删除/同步/merge/orphan；
7. namespaced canonical semantic field IDs、namespace owner/anti-spoof/version/custom mapping；
8. 用户最终澄清：禁止单一 `people` attribute/profile blob；`people` 只作 namespace/source grouping，解析语义必须是多条独立 `people.xxx` canonical fields 与逐项可寻址 occurrences。flat source 与 scoped readable grouping 可比较，但 block 只是可逆语法组织，不是整体 value/authority/transaction aggregate。

D4 prompt 和 immutable package 必须携带上述完整 bytes、alternatives、反例与 stage-routing，不依赖聊天、标题、近期会话或总控摘要。D5–D10 按本文 owner table 接收相应切片，A2 读取完整 bytes 做全链从零复核。若任何胜出方案改变冻结 D2/D3 顶层合同，当前下游任务只产出 formal impact/reopen request；按 §11.2/§12/§13 串行完成全新上游 council、独立 OpenCode 和 fresh Sol / Pro Gate，再以 replacement frozen contracts 重建全新下游任务。用户无需搬运交接消息。

### 11.4 2026-08-30 pre-D4 upstream supersession

用户已授权在 D4 前正式处理成立的 D2 顶层反例。本小节 supersede §11.2、§12.4 与 §13.4 中所有“先启动 D4，再请求 D2 reopen”的旧路由；候选内容、反例和未决问题本身不变。

本小节也 supersede 本文全部历史或未明确档位的 `Sol / Pro Gate`、`Sol Gate`、`Pro Gate` 等评审路由措辞。唯一有效档位由当前 Design Review Execution Protocol 的 profile 矩阵决定：D4、D5、D8、D9 使用 fresh GPT-5.6 Sol Standard；D6、D7、D10 使用 scheduled fresh GPT-5.6 Sol Pro macro Gate 并替代同阶段 Standard；正式重开 D1–D3 与 A2 最终 Gate 使用 fresh GPT-5.6 Sol Pro。本段只澄清 reviewer/model tier，不改变任何场景、反例、fixture、stage owner 或候选。

当前严格顺序：

1. 原 D2/D3 继续是唯一权威；D4 保持显式暂停。
2. 创建真正全新的 formal D2 replacement task，联合但独立比较 §12 A/B/C、§13 B1/B2，以及 flat legal header 与 persisted namespace-scoped block；不得预选用户偏好的 B 或 Task B2。
3. D2 replacement 使用一份 fresh DeepSeek V4 Pro + 一份 fresh GPT-5.6 Sol Pro replacement Gate，关闭全部 P0/P1 后才可原子替换 D2 权威。
4. 若 replacement 实质影响 D3，只创建 impact-scoped D3 amendment/revalidation task，重证 exact-source/locator/span、copy/fork/import/export、Task/checklist/VTODO 与相关 diagnostic；没有新反例的 NodeRef/lifecycle 主体合同不整体重做。
5. D2 与必要 D3 amendment 完成后停止。D4 不得自动创建，等待用户以后明确恢复。
6. 用户恢复后，才生成带唯一 `_weftext.id` 的 D4 cross-domain intake，并用 replacement frozen contracts 创建真正全新的 D4；不得复用 D2/D3 的模型会话、package 或 review tooling。

本 supersession 只改变阶段路由，不把 A/B/C、B1/B2、Node schema membership、People 字段或 scoped syntax 描述为已接受实现。

## 12. Node control envelope 与 schema-membership placement 顶层替代

本节 supersede 此前“declared memberships 唯一写在 AsciiDoc header attribute”的默认候选。用户当前偏好是文件顶端 closed `_weftext` YAML Node control envelope，但该偏好仍必须经过 `$council` 的 A/B/C 完整比较、上游影响裁决和 Sol / Pro Gate；不得由 staging intake 直接冻结或修改当前 D2/D3。

### 12.1 用户偏好与设计理由

- Node schema membership 决定随后哪些 attribute schema、validation、editor/View/Action contributions 生效，逻辑层级高于普通领域 attributes；它是 owning Node 的控制语义，不是某个 attribute schema 内的一项用户事实。
- membership ID 是 Core/Extension registry 中的 namespaced reserved identifier。正常用户路径是显式 `Assign/Add/Remove` Core Action 的 preview + atomic commit，不把它当普通属性编辑；Source/repair 模式仍可见、可修复，文件并非不可编辑。
- 一个 Node 可声明 `0..N` memberships。typed semantics 是 unordered set、无覆盖优先级；source 可保留稳定顺序，但 duplicate ID、非法 ID、互斥、依赖缺失和 schema field type conflict 必须产生确定诊断。
- Node control layer 与 Document domain facts 应视觉、语义分区：`_weftext` 只承载受限 Node-control facts；AsciiDoc title/header attributes/body 承载用户文档与领域 facts。provider 缺失或卸载时 envelope 与 raw attributes 都保留，Node/Document 不丢失。

非冻结示意：

```adoc
---
_weftext:
  profiles:
    - people/person
    - calendar/period-note
---

= 张三
:person-phone: ...
```

`profiles`、`people/person`、`calendar/period-note` 和 YAML shape 都只是输入示意；§8.5 可能要求把 Node mechanism 与 key 改名，不能由例子取得兼容承诺。

### 12.2 必须比较的完整替代

| 方案 | canonical source 候选 | 必须证明/反驳 |
| --- | --- | --- |
| A | 当前冻结 D2 的 `:wf-specialization: task\|template`，加未来 reserved AsciiDoc header membership attribute | 单一 AsciiDoc grammar、external tooling、exact source 和较低 reopen 成本是否胜过 Node-control/领域 facts 混层、list/set encoding 与用户把 reserved key 当普通 attribute 的风险。 |
| B | 文件顶端 closed `_weftext` Node envelope；`specialization` 与 memberships/profiles 同层，AsciiDoc attributes 只存领域 facts | 层级清晰、native list、Action-owned control 与一文件可移植性是否值得正式重开 D2 grammar/exact-source/specialization，并重证受影响 D3；能否实现 deterministic/lossless-enough external editing、五表面一致和 AsciiDoc interoperability。 |
| C | D6 control-plane/sidecar 保存 Node classification/membership | 是否能在不形成第二权威、不破坏 portable one-file copy/export/external edit 的前提下成立；owner、atomicity、missing sidecar、backup/restore、sync、move/fork/import 和 plugin-missing degradation 是否比 A/B 更简单。若需要 roadmap reorder，必须显式提出。 |

reviewers 必须允许满足全部冻结约束的更优 D 方案，但不能只比较 YAML 美观或沿用现有投入。Weftext 当前零用户、无兼容义务，已有实现成本不能单独否决 B；同样，已冻结上游合同和复审成本不能被忽略或由 D4 绕过。

### 12.3 若采用 B，closed `_weftext` envelope 的最低候选约束

- 文件只允许一个 reserved root `_weftext`，其 schema 为 closed keys。未知 root/control keys 的 parse、commit、repair、export 行为必须封闭；不得演化成通用 YAML frontmatter。
- 禁止 YAML anchors、aliases、tags、merge keys、custom types、arbitrary provider blobs、environment substitution、executable constructs 和隐式类型惊喜；解析必须有 byte/depth/item/string/resource limits。
- memberships/profiles 是 namespaced stable IDs 的 list source、unordered set semantics；source order 稳定但无优先级。duplicate、malformed ID、namespace collision、unknown provider、missing dependency、mutual exclusion 和 conflicting field schema 都有确定 error priority。
- provider 不得在 `_weftext` 写私有状态。provider/plugin version、installed/enabled state、permissions、credentials、sync token/etag、cache/index、derived closure 和 validation result 属 control plane/可重建状态，不进入 envelope。
- `specialization` 是否与 memberships 同层是 B 的强制比较点；是否加入 Node identity、file/control format version 或其他 Core facts，分别由 D2/D3/D6 owner 裁决。不得因为 Weftext-Control 历史 `_weftext.id` envelope 存在就类推产品 Document wire。
- Core 必须定义 deterministic、lossless-enough editing contract，避免保存时无关重排、换行/BOM/Unicode normalization/quote style/comment damage；准确保留与 canonicalization 边界、hash 语义、repair rewrite 和 diagnostic spans 必须可测试。
- rich editor 把 envelope 当系统管理区，普通操作通过 Core Actions；Source/repair 模式可见且手改会重新通过同一 parser/validation/commit gate。“不期望手改”不等于不可编辑或绕过 external-editor 事实。
- one file / one authority：YAML control 与 AsciiDoc header 若出现重叠 field 必须拒绝，不设 precedence、fallback、compat dual-read 或双写。importer 可从外部 YAML/attributes 映射，但 commit 后只有最终 canonical target。

### 12.4 D2/D3/D6 上游影响与严格序列

- B 会触及 D2 Document grammar、exact-source boundary、`wf-specialization` placement、metadata/control projection、duplicate/unknown diagnostics、source spans、plain AsciiDoc interoperability 与 five-surface parser contract。D4 不能把 YAML 当普通新属性局部冻结；若 B 胜出，必须生成 formal D2 reopen request。
- D3 当前仍按冻结 D2 完成，不得被本节打断或改写安全检查点。未来若 D2 重开并改变 source/specialization，必须逐项审计 D3 的 Node/Document identity、Task/Template lifecycle、exact-source payload/hash、copy/move/fork/import/export、fresh identity、artifact binding、resolver/diagnostic 和 wire corpus；只重证实际受影响合同，但不得用“identity 没变”跳过 source-bound evidence。
- C 若胜出，D4 必须提出明确 D6 owner/roadmap-reorder precondition，不能先冻结一个抽象 membership 再让 D6 任意选择第二权威。需要 D6 前置时，由 controller 串行创建独立 boundary task，完成后重新启动全新 D4。
- 严格处理顺序现由 §11.4 控制：在 D4 前完成 formal D2 replacement review；若 replacement 影响 D3，再做 impact-scoped D3 amendment/revalidation。C 的 sidecar/control-plane mechanics 可作为 D2 replacement 的依赖裁决或 D6 前置要求，但不得因此先冻结第二权威。完成后仍停在用户设置的 D4 pause；任何未来 D4 必须从 replacement frozen contracts 新建。
- D2/D3 reopen/amendment 期间仍只允许一个 control writer。所有旧 frozen decision 保持权威，直到 replacement Gate pass 并原子更新 ledger/index；失败或 pending Gate 不能让候选 YAML 成为兼容读路径。

### 12.5 必须重放的 wire、交互和 conformance fixtures

1. ordinary Node、Task、Template、single/multi-membership Node；specialization 与 memberships 是否同层且无重复 authority。
2. duplicate/malformed membership ID、unknown provider、missing dependency、mutual exclusion、两个 schemas 对同一 field 声明不同 type。
3. provider uninstall/reinstall、membership upgrade/remove、显式 field cleanup；raw facts 与 control source 保留。
4. external editor corrupt/truncate envelope、重复 `_weftext`、unknown key、YAML separator 与 AsciiDoc title ambiguity、Unicode/BOM/CRLF/LF、comments/order/quotes。
5. move/copy/clone/fork/import/export、exact-source hash、artifact round-trip、plain AsciiDoc export strip/retain policy与 loss report。
6. concurrent Assign/Remove membership 与 attribute/body edit；expected revision、read-set、atomic commit、conflict、retry/receipt。
7. 100k Node 只扫描 control/header 的索引性能、resource limits、incremental parse 与 cache rebuild；性能不能通过第二 authoritative index 实现。
8. Desktop、CLI、Server/WebUI、Mobile offline 的相同 parse/validation/diagnostic/repair；provider 缺失和 schema version 不兼容。
9. malicious anchors/aliases/tags/merge/custom type/deep nesting/alias bomb/oversized scalar，以及 truncated YAML 导致的 bounded fail-closed。
10. AsciiDoc external-tool 打开、编辑和导出：工具忽略/保留/损坏 envelope 时的 observable status、repair 和 loss，不用 silent fallback 掩盖。

### 12.6 阶段 owner 与最终 Gate 问题

- D4 owns A/B/C 的语义层级、membership/specialization/schema source、typed value、validation、术语和 upstream-reopen disposition；不得冻结 D6 sidecar mechanics。
- D6 owns 最终选择下的 persistence、parser/commit/repair、revision/transaction、sidecar/control-plane 替代、concurrency、sync/cache/index 与 source-of-truth enforcement。
- D7 owns membership-driven Query/View/Action projection；D8 owns rich/source/repair UI 与 five-surface diagnostics；D9 owns import/export/plain AsciiDoc/loss/worker boundary；D10 owns registry/provider state 与 envelope 禁止私有 blobs；A2 全链从零复核。
- 最终 Sol / Pro Gate 必须明确回答：closed Node control envelope 的层级清晰、native list 与 Action ownership 是否值得正式重开 D2 并重证 D3，还是 reserved AsciiDoc header 的单语法优势或 C/其他方案整体成本更优。不得只用已有投入/兼容成本否决 B，也不得忽略冻结上游、复审证据和长期 parser/interop 成本。

## 13. People localizable semantic codes 与 Task ontology 顶层替代

本节与 §12 属同一轮 D4 cross-domain intake。People 的稳定语义码/本地化显示和 Task B1/B2 都是候选，不是冻结结论；当前 active D3 不读取、不处理、不因本节改变 checkpoint。Task ontology 与 membership 存储位置正交，必须比较 `(§12 A|B|C) × (§13 B1|B2)`，不得把 Task B2 偷绑到 YAML envelope。

### 13.1 People labeled entries 与稳定语义码

- 持久化值必须使用 namespaced stable semantic code；People 在 zh-CN/en 等 locale 提供显示标签。用户自定义 label、note 与原始文本逐字保存、不自动翻译。不得把已本地化字符串写入语义字段，也不得用 `work-mobile-personal-former` 一类组合枚举无限膨胀。
- phone/email/IM 使用可重复、结构化 contact entries，而非 `workPhone`、`personalPhone` 等平铺字段。每项至少比较 `value + optional preset label code + optional custom label/note`；technical kind 与 usage 是否拆开、是否允许 preset 与 custom 同时存在、wire error priority 和 list ordering 由 D4/D5 裁决。
- `addresses[]` 候选同样为多值 labeled entries：地址值、可选 stable preset（示意 home/work/mailing/former/other）、可选 custom label/note。自由文本/格式化地址是跨国家保底，结构化组件是可选增强；不得把任一国家的行政区/邮编/街道结构固化进 Core。
- `names[]` 候选为多值条目，stable role/status code 可比较 preferred/common/legal/birth/former/alias/transliteration；locale/language/region/script 与 role/status 正交，禁止把中文名/英文名/罗马化/曾用等笛卡尔积塞进单一 label enum。可选 validity/note 支持曾用名。准确 code、字段、唯一性和 ordering 未冻结。
- Node title 与姓名条目、identity 分离：标题可由 preferred/common name 在创建时初始化或作为显示建议，但不是 names[] 的隐式第二权威，不承担全局唯一性；同名 Node 合法，不自动加随机后缀，identity 依赖 NodeRef/冻结 D3 identity contract。
- relatives 是关系集合/编辑器，不是 father/mother/son/daughter 固定字段。每项 target 是 Person NodeRef 或 literal text fallback，并保存 stable directional relation code（示意 parent/child/spouse/sibling/guardian/custom）、inverse display、direction、必要 validity/status/note 与删除语义。literal target 不产生 inverse edge；custom relation 不得冒充可推导的 parent graph；父亲/母亲等更细显示只能来自明确事实或用户选择，不按姓名猜。
- D4/D5 必须比较共享 `labeled entry` UI/SDK/schema primitive 是否减少重复，但不能把 phone、email、IM、address、name、organization role 与 kinship 强压成一个无语义字段。尤其 kinship relation code 影响方向、inverse、约束与 GraphProjection，不是普通 label。

#### 13.1.1 People fact-shape：不得简化为 singleton/list 二分

- `identity/control`：`_weftext.id`/冻结 D3 Node identity 是 Node 的唯一控制身份，不是现实人物属性，也不因同名、生日或外部 identifier 相同而合并 Person。
- `event assertion`：birth/death 等现实事件通常各发生一次，但工作区可保存多个互相冲突的 assertions；候选 assertion 至少比较 value、precision、calendar、provenance/source、confidence 与 preferred/current projection。生日应是 birth event 的首选断言/简化投影，不得宣称底层永远只能存一个 birthday value。
- `temporal state`：姓名、国籍、婚姻状态及需要明确区分的 sex/legal sex/gender identity 等概念可有 validity/history。不得把不同概念压成一个裸 `gender` 字段，也不得把“当前值”覆盖历史事实。
- `measurement observation`：身高、体重等是带 observed-at/effective-at 与 unit 的 measurements；默认 UI 可显示最新/首选值，但底层不得反复覆盖静态 scalar。单位换算、精度、来源和异常值诊断需可扩展。
- `repeatable entry`：phone、email、IM、address 等天然多值，使用 §13.1 的 labeled entries 候选；多值并不自动赋予每项独立 Node identity。
- `relation`：亲属、任职、membership 等有 direction/role/validity/qualifiers，按领域关系语义与 GraphProjection 处理，不是普通 labeled scalar。
- D4/D5 必须允许更少、更统一的通用 fact/typed-value 形态，但不得只以 `singleton` 与 `list` 两类 multiplicity 代替事件、时间状态、观测和关系语义。是否抽取 assertion/observation primitive、其 wire/source 与是否需要 Record domain仍待从零比较。
- 简单 UX 是硬约束：普通用户默认只看到并编辑一个 current/preferred value；只有在需要时才展开历史、来源、冲突、精度、历法、有效期或单位。不得为了理论完备性强迫每次录入复杂对象；同时 simple edit 必须有确定规则决定更新现有 assertion、追加新 assertion/observation或请求用户选择，不能静默丢历史。

#### 13.1.2 People × Calendar：important dates UX 与事实/周期投影分域

- People 可提供 repeatable `important dates`/milestones 集合式 UX，统一展示 birth、death、marriage anniversary、employment start、graduation 与 custom date；preset semantic code 稳定且本地化，custom label 原样保留。该 UI 聚合不意味着所有条目共享同一领域语义或存储形态。
- `birth` 是保留的 event semantic code，对应一次出生事实的一个或多个 date assertions，不是任意自由 label。默认显示 preferred assertion；精度、calendar、timezone/zone applicability、provenance/source、confidence、note 与 validity/recorded distinction 需按事件语义裁决。
- Calendar 可从 birth fact 派生每年 birthday anniversary/reminder/occurrence；农历、其他历法、自定义庆祝规则和通知由 Calendar recurrence/calendar contribution 表达。派生 occurrence 没有独立作者事实或 Node identity，除非用户执行另一个明确、有 receipts 的创建/提升 Action。
- 同一事实只保存一处：禁止同时维护 `birthDate` 与复制的 birthday recurrence 作为双作者权威。跨 module/extension 通过 NodeRef、typed contribution、derived occurrence 与可重建 projection 关联；Calendar/provider 停用只移除派生显示/提醒，不删除或改写 People birth assertion。
- date entry 必须比较 event date、precision、calendar、timezone、recurrence/celebration rule、source/provenance、confidence 与 note 等正交维度。普通录入仍可只填一个日期和 preset；高级维度按需展开，不用一个 label 枚举承载历法、精度和 recurrence 组合。
- D4 owns fact/projection/source-of-truth；D7 owns Query/View/derived occurrence；D8 owns unified important-dates UX；D9 owns联系人/Calendar import mapping 与 duplicate prevention；D10 owns Calendar rule/reminder/provider availability；A2 用停用、重导入、历法切换和权限做全链复核。

#### 13.1.3 People × Organizations：职业、任职、职务与职级

- `profession/occupation` 表示一般职业或专业身份（如律师、工程师、教师），可多值、可有 validity/history，但不必隶属某个组织。它不得与具体 employer/appointment 混成同一字段；profession code/preset、自定义值、来源与本地化仍待 D4 裁决。
- `engagement/appointment/affiliation` 表示 Person 在具体组织中的任职、隶属或参与关系，候选为可重复、带时间的 relation/structured fact。候选维度包括 organization target（优先 NodeRef、允许 literal text fallback）、department/unit、position/office title、rank/grade、validFrom/validTo/current、note/source；employment type 等只作为可选扩展，不得把完整 HR/CRM 模型带入 Core。
- organization、department/unit、position/office title 与 rank/grade 必须正交。相同 position 在不同 rank system 中不等价；rank/grade 通常依赖组织、国家或行业制度，不得把中国或其他单一制度固化成全球 enum。Organizations module/child schema pack 可贡献 namespaced stable codes/presets 和本地化显示，custom value 原样保留。
- 一人可同时在多个组织任职，也可在同一组织兼任多个 position；不得用 Person 上的单个 employer/title/rank scalar 覆盖其他任职或历史。current 是 validity-derived/explicitly adjudicated projection，不能通过数组顺序或最后写入猜测。
- organization target 为 NodeRef 时可形成可查询的反向隶属/任职 edge；literal text 只是作者 fallback，不产生可靠 inverse、Organization Chart member 或自动建 Node。People 可以引用任意普通 NodeRef，不强制目标已安装或声明 Organization schema；Organizations provider 可用时仅增强 validation/editor/query/chart，不接管或复制关系事实。
- 必须比较关系事实由 Person-side engagement、Organization-side membership 或独立 Record/Node 存储的替代，并冻结唯一 authority、inverse edit、权限、删除、合并、validity、provider missing 与同步语义；禁止 Person/Organization 两端双写。
- §8.5 Lexicon 必须明确中文“职业/任职/隶属/职务/职级/部门/单位”与英文 `profession/occupation/engagement/appointment/affiliation/position/office/rank/grade/department/unit` 的 canonical mapping、排除边界、wire/code/UI 名和 rejected aliases，不能在 API/UI 中互换。
- D4 owns semantic/wire alternatives；D5 owns typed relation/value/Record/Node comparison；D6 owns authority/transaction/history；D7 owns inverse query/organization chart；D8 owns渐进编辑与本地化；D9 owns联系人/组织导入 mapping/loss；D10 owns Organizations provider/schema-pack availability；A2 以跨组织兼任、国家职级体系、provider absent 和 text→NodeRef replacement 做全链复核。

#### 13.1.4 Attribute value occurrence 的 optional note 与稳定寻址

- 原则候选：每个 attribute value occurrence/entry **能够**附加 optional note，而不是每项强制填写、默认生成空 note 或在简单 UI 中始终展开。note 是用户自由文本；无内容时 canonical source 不存占位，默认只显示 `key = value`，有备注或用户展开时才显示。
- 同一 key 的多个 values 分别拥有自己的 note，例如每个 phone、address、name、appointment、measurement 或 event assertion 可单独注释；note 不得因为值相同、排序变化或 label 改变而串到另一项。
- label、validFrom/validTo、source/provenance、confidence、unit、precision/calendar、privacy/access qualifier 等只要需要 Query、validation、localization 或驱动行为，就必须是独立 typed fields；不得塞入 note 后再用自然语言解析恢复语义。
- 该能力要求 attribute occurrence/entry 有稳定、可确定的 target semantics。D4/D5 必须比较 explicit occurrence ID、container-scoped key、source-range identity、content-derived key 或更优方案，并验证 insert/reorder/copy/clone/delete、duplicate values、external edit、sync/merge、link target、Annotation target 和 ID collision；禁止仅以 `(key,value)` 猜目标。
- 必须从零比较两种主要 wire：A) note 是 entry/assertion 内嵌字段；B) 复用通用 Annotation，以稳定 occurrence ID 为 target。允许更优混合/第三方案，但不能双写。比较 portable text wire、AsciiDoc/external-tool compatibility、source diff 可读性、局部编辑、权限、Annotation lifecycle、孤儿清理、plain export/loss 和 Mobile offline。
- 共享 labeled-entry/fact-row editor 或 SDK primitive 可以统一 add/remove/reorder/note interaction，但 domain schema 仍保留 typed fields 与关系/事件/测量约束；UI primitive 不得成为无类型事实模型。
- 若 note 使用 Annotation，必须明确 owning Node/Document、target ref、copy/fork/import ref rewrite、target deletion cascade/orphan state、hidden target 权限和 provider missing；若内嵌，必须明确稳定 entry identity 是否写入作者 source、何时生成、外部删 ID 的 repair、以及注释链接能否跨重排保持。
- D4 owns semantic requirement/source alternatives；D5 owns occurrence/Record/Annotation domain boundary；D6 owns ID/lifecycle/transaction/sync；D7 owns query/link projection；D8 owns collapsed/expanded UX；D9 owns import/export/loss/ref rewrite；A2 以重复值、重排、外部编辑和跨设备 merge 做全链选择。

#### 13.1.5 Identity × People × Search：ID、locator、title、names 与 aliases

- `_weftext.id`/最终冻结 D3 Node identity 是系统生成、稳定、不透明的控制身份；用户不需要把它用作标题、文件 basename 或可见编号。按 ID/NodeRef 的引用连续性不依赖路径、标题、Person name 或 aliases。
- filesystem/node locator/name 只受通用合法性、局部冲突与平台边界约束，可使用人名、编号或其他用户选择；它是可变 locator，不承担现实人物身份、display authority 或稳定引用。准确的 locator/path/basename 术语必须服从最终 D3。
- Node display title 可自由编辑、不保证唯一；People create/editor 可用 current preferred name 初始化或建议 title，但 title 不是 Person name 的唯一权威来源，也不得与 `names[]` 隐式双写。Workspace 内同名合法，消歧使用路径/组织/职位/脱敏联系方式/短 ID 等明确上下文。
- `names[]` 是 Person 领域姓名集合，role 可含 preferred/common/legal/birth/former/alias/transliteration，并可带 locale/script/validity/note/source；People search、selector 与 link-display contribution 应索引这些 names，不复制为另一份 Person aliases authority。

必须从零比较：

| 方案 | alias/search 模型 | 必须证明/反驳 |
| --- | --- | --- |
| A（当前倾向） | 不设 Person `aliases` 字段；人物别名只是 `names[].role=alias`，People 自动贡献 search terms。 | 是否足以覆盖 former/legal/transliteration 等姓名搜索、provider missing 降级与普通 Node 非姓名同义词，而不产生第二权威。 |
| B | 保留通用 Core `nodeAliases`，只表示跨领域搜索同义词/旧标题；Person names 仍只在 `names[]`，两者独立贡献 search terms、绝不自动同步复制。 | `nodeAliases` 是否有跨领域稳定需求；能否明确排除现实姓名、identity/redirect/引用连续性，处理重复 term/ranking/rename migration 而不让用户困惑。 |
| C | 取消通用 aliases；各领域 schema/provider 贡献 search terms，无 schema Node 只索引 title/content。 | 无 Profile/schema provider 时的旧标题、拼写同义词、外部编辑与通用搜索能力是否仍可接受；provider missing 是否导致可观察搜索骤失。 |

reviewers 可提出更优 D，但不能通过把 names 复制进 aliases、把 alias 当 redirect/NodeRef、或把旧 title 自动永久累积为隐藏 identity 来闭合搜索。

- 必须对照最终冻结 D3 的 authoritative identity、locator、path、display label、rename/move、resolver 与 reference display semantics。若 D4 方案改变任一上游合同，只能提出 impact-scoped D3 reopen/amendment，经独立 council 与 fresh Sol / Pro Gate 后重启全新 D4。
- Search 必须裁决 title/content/names/nodeAliases 的 indexing、normalization、locale/script/transliteration、ranking、dedupe、exact-ID query 与 privacy filtering；同一字符串来自多个 contributions 时不复制命中或改变 identity。
- Link storage 仍是 NodeRef；显示可选择 explicit authored label、current title、People preferred name 或 fallback，但必须有机械优先/refresh policy，不能把 display label 反写成引用目标。路径移动、重命名和 preferred name 变化不破坏链接。
- People provider 缺失时 raw `names[]` 必须保留。是否由 Core 按已知 portable schema 继续索引 names、保留上次派生 index、或仅 title/content 降级，需明确状态/diagnostic/security；不得把 stale index 当作者权威或静默删除 names。
- §8.5 Lexicon 必须禁止把 `_weftext.id`、NodeRef、locator/path/basename、Node title/display label、Person name、alias、search term、redirect 混称“节点名/name/alias”；中英文 UI、wire、code、CLI 和 docs 要有一对一映射。
- D4 owns domain/source alternatives 与 D3 compatibility；D6 owns rename/move/index lifecycle；D7 owns Search/query/link-display projection；D8 owns同名消歧/selector/UI；D9 owns import old-title/aliases mapping与 loss；D10 owns provider/search contribution availability；A2 全链复核。

#### 13.1.6 People relationships：亲属、社交、职业及 asserted/derived

- friend/colleague 不属于 kinship/relative。上位产品与模型候选名优先比较“人物关系 / People relationships”，避免“相关人员 / related people”退化为没有 edge semantics 的任意 NodeRef 列表。亲属只是人物关系的一类，不应把所有关系塞进 `relatives[]`。
- 可共享 relation-entry primitive，但以小型可扩展 category/semantic-code 分域：`kinship/family`（parent-child、spouse、sibling、guardian 等）、`social`（friend、acquaintance、classmate 等）、`professional`（manager-report、mentor-mentee 等）。准确 namespace/code/分类未冻结；Core 不硬编码完整词汇或全球封闭 ontology。
- People、Organizations 或其他 module/extension 可贡献 namespaced stable relation codes、localized labels、direction、inverse、symmetry、allowed target/qualifier 与 graphProjection metadata。用户 custom label 原样保留；若 custom relation 未声明方向/inverse/symmetry，只能作为 neutral asserted edge，不臆造 parent/manager/Family Tree 语义。
- Family Tree 只消费具有明确 kinship direction/rank semantics 的 edges；friend 等 symmetric social edge 进入一般 People/relationship graph；manager-report、mentor-mentee 等 directed professional edge 可进入层级/ranked view。category 本身不等于布局，View 必须按 edge metadata 与权限过滤。
- colleague 通常可由共享 Organization engagement/overlapping validity 派生。不得因为同一组织有 N 人就持久化 O(n²) colleague edges；derived colleague 无独立作者权威、可由 engagement 重建。用户仍可显式记录特殊 colleague relation，但 wire/query/UI 必须区分 `asserted` 与 `derived`，不得在两者间自动双写。
- target 优先 Person NodeRef，允许 literal text fallback；NodeRef 支持可靠 inverse/query/Graph，text 不生成 inverse 或自动建 Person。条目可有 validity、note/source 与 qualifiers；relation fact 的唯一写入位置、symmetric normalization、inverse editing、删除、权限和 text→NodeRef 原子替换由 D4/D6 裁决。
- 必须比较 IA：A) 单一“人物关系”主区 + kinship/social/professional filters；B) 分区编辑器但共享底层 relation primitive；C) 更优渐进方案。无论选择，不得让“相关人员”成为无类型 catch-all，也不得让每个自定义词自动注册全局 RelationType。
- 本例是关系类型膨胀门禁：优先比较 extension-contributed 小型 preset lexicons + custom neutral relations 与 profile-field-declared graph metadata；独立全局 RelationType registry 保留为对照替代，若胜出必须证明治理、namespace、inverse、迁移、权限、未知 provider 和用户理解不会失控。
- §8.5 Lexicon 必须区分人物关系/相关人员/亲属/家庭/朋友/同事/任职/上下级/导师，以及 relationship/relation/related people/relative/kinship/family/social/professional/colleague/engagement；wire/code/UI 不得把 category、code、label 和 edge kind 混用。
- D4 owns relation semantics/registry alternative；D5 owns value/occurrence/Record identity；D6 owns asserted authority/derived rebuild/transaction；D7 owns graph projections；D8 owns IA/editor；D9 owns import mapping；D10 owns contribution/unknown-provider lifecycle；A2 重放规模、权限、历史和停用。

#### 13.1.7 Extensions × Attributes：canonical semantic field namespace

- 强候选：所有扩展/领域 schema 的 canonical semantic field ID 必须命名空间化；UI 默认只显示本地化 label，不显示实现 namespace。Profile/schema IDs 可示意为 `people/person`，字段可示意为 `people.names`、`people.contacts.phone`、`tasks.status`、`calendar.start`，但 separator、grammar、wire shape 与术语均未冻结。
- 一个 Node 可组合多个 schema/profile；裸 `name/status/type/date` 会发生 owner、type、validation、migration 与 locale 冲突。Query/API/import/sync/migration 使用 stable canonical ID；中文/英文 display label 改名、缩写或本地化不改变 data key。
- 必须比较至少三类 storage wire：A) 完整 namespaced key；B) profile/schema-scoped block；C) 可逆紧凑前缀。允许更优 D，但在 provider 未运行、未安装或已卸载时仍必须从 source bytes 判明 semantic owner、保留 unknown fields 并无损 round-trip；不得依赖 runtime registry 才能解码作者内容。
- domain fields 仍属于 AsciiDoc Document facts，不进入 closed `_weftext` control envelope；后者若胜出只保存 memberships/profiles 等 Core control semantics。禁止为方便 namespace 把 provider-private blobs 或领域属性搬进 control YAML。
- namespace 按 semantic owner，不按消费者：birth assertion 由 People 拥有；Calendar 只能贡献 recurrence/index/reminder/derived occurrence，不得另存 `calendar.birthDate`。同一 owner rule 适用于 Organizations engagement、Search terms、Graph edges 与 connectors；consumer cache/index 可重建。
- 必须冻结 official namespace、third-party package/publisher namespace 与 user custom field namespace/registration/collision/claim rules。第三方不得声明、shadow 或 masquerade 为官方 `people.*`；签名/安装顺序不改变 field owner。schema version 与 field ID 分离，升级通过 versioned schema/migration，不随意改 key 制造新事实。
- enum/label codes 若只在单一 field context 内解释，可以使用短 stable values；跨 field/extension 可引用的 relation kind、Action、Profile/schema type、classification scheme 等必须使用 namespaced stable IDs。reviewers 必须列出何时必须全限定、何时上下文足够，避免过度冗长与歧义并存。
- 用户仍可创建普通 custom `phone`/`status` 等属性；People/Tasks 等不得仅因同名就静默认领、迁移或验证。提供显式 map/promote Action：preview target canonical ID、类型转换、冲突、source loss、provider dependency 与 receipt，再由 Core 原子提交；取消不改原字段。
- 比较必须包含人类可读性、AsciiDoc attribute 合法字符/case folding、header/list/object encoding、diff、手工编辑、copy/paste、external tools、CLI/SDK ergonomics、codegen、locale mapping 与 100k-node header scanning，不得只按数据库 column 或语言 identifier 便利选择 syntax。
- §8.5 Lexicon 必须区分 Profile/schema ID、field ID、wire key、display label、locale key、enum code、relation kind、Action ID、package/publisher namespace 与 user custom key；跨层映射显式一对一，禁止裸 `type/name/status` 在 wire/code 多义。
- D4 owns semantic namespace/wire alternatives；D6 owns persistence/migration/round-trip；D7 owns Query/API identifiers；D8 owns labels/advanced ID reveal；D9 owns import/map/export；D10 owns registry/anti-spoof/version；A2 全链复核。

#### 13.1.8 Attributes wire：flat keys、nested object 与 namespace-scoped content block

本条 supersede 对用户问题的早期理解。用户实际比较的是：**一个 `people` attribute 内嵌所有 People 值**，还是**多条独立 `people.xxx` canonical fields/occurrences**。明确强候选为后者：`people` 只表示 namespace/group，不承载整个 Profile 数据；names、phones、addresses、relationships、engagements 等分别拥有独立 canonical field，重复 entries 也逐项可寻址。source group/block 只可减少视觉前缀噪音，解析语义必须等价于独立 fields，不能形成原子 blob。

当前 wire 倾向因此收敛为：**canonical semantic field ID 全限定 + 独立 field/occurrence 语义 + 可选的内容区 namespace-scoped readable grouping syntax + UI 领域分组**。最终 syntax 仍未冻结；D4/D5 必须给实际 source bytes、parsed wire、partial edit、round-trip 与 diagnostics 后裁决。

| 方案 | source shape 候选 | 优点 | 强制反例/风险 |
| --- | --- | --- | --- |
| A flat | `people.names`、`people.phones` 等全限定键平铺 | grep、单字段寻址、API/codegen 直接，provider 缺失仍显式 owner | 重复前缀、source 冗长；repeatable object、entry ID、note/validity/source、顺序和局部 diff 可能笨重或依赖另一个编码层。 |
| B single nested attribute/blob（强反例） | 一个 `people` attribute/object 内含全部 People values | 表面聚合、单次读取容易 | 把不同 fields/entries变成整体原子值，扩大 diff/merge/conflict/permission/migration；逐项 query/update/annotation/note困难；若在 `_weftext` 更违反 control envelope，若是 JSON/YAML string 则是 opaque provider blob。 |
| C scoped readable grouping（当前倾向） | Document content/header 的 `people` namespace block，块内短字段与 repeatable entries；parse 后仍分别映射 `people.names`、`people.phones` 等独立 IDs/occurrences | 减少视觉前缀且保留逐项语义，支持多 namespace；UI 可另按姓名/联系方式/关系/任职组织 | 必须证明 grammar、delimiter、duplicate/order/entry ID、external tools与性能；block 不能拥有整体 value identity、整体 precedence 或整体更新语义。 |

- `_weftext` control envelope 只承载 memberships/profiles 等 Core control facts；People、Tasks、Calendar 等 domain fields 无论 A/B/C 都不得嵌入该 YAML，也不得以任意 provider blob 绕过 D4 schema/D6 commit gate。
- C 的 group/block 只是 source organization/syntax sugar，不是 attribute value、Profile instance、transaction aggregate 或 UI section。多个 namespace 可各有 block；同一 namespace 多 blocks 若允许，解析后按 canonical field/occurrence 合并并机械诊断 duplicates，不得以 block precedence 覆盖。UI 可拆分 sections，第三方字段仍写自身 namespace。
- Core Node identity/title/locator 等 Core facts不重复嵌入 `people` block。共享/跨域 fact 按 semantic owner 归组：birth 在 People，Calendar 只派生；organization engagement 的 owner按最终裁决，消费者不得因 UI 位置复制 field。
- extension/provider 缺失时，block 和 entries 仍须以通用 Source/repair 模式可读、可编辑、复制并无损 round-trip；不得把 structured entries 编成单一不可解释 JSON/YAML string。unknown field/namespace 的 validation 与 safe preservation 状态必须显式。
- 每案必须覆盖：AsciiDoc legality、placement/delimiters、duplicates、entry order/ID、note/source/validity、external formatter、field-level permission、partial patch、independent annotation/link、merge conflict domain、copy/template、plain export、100k scan、Query/API canonical ID 与 SDK ergonomics。B 必须明确展示为何整体 blob 不会或会放大冲突、迁移与 extension-removal blast radius。
- Query/API 始终使用 full canonical semantic ID，并对独立 field/occurrence 寻址；source block 内 short name经 namespace可逆展开。不存在依赖 active Profile 顺序的 unqualified lookup，不允许 block precedence，也不提供一个隐式 `people` whole-object authoritative field。
- D4 必须输出最少三个实际源文本 fixtures：simple Person、multi-profile Node、unknown third-party namespace；D5 为 repeatable names/phones/appointments/relations + note/validity/source/entry ID 给出 value/occurrence/Record 替代；D6/D9 重放 external edit、migration、copy/template、merge与 loss。

### 13.2 Task B1/B2 顶层替代

| 方案 | ontology/package 候选 | 必须证明/反驳 |
| --- | --- | --- |
| B1（当前冻结） | Task 与 Template 是 Core mutually-exclusive specialization，source 为 `wf-specialization: task|template`；Tasks UI 只是 bundled module。 | Task 是否具有不可由普通 Document-bearing Node + schema 表达的 Core 本体差异；Core Task algebra/query/promotion 是否为 portable semantics 所必需，还是造成与统一 composition 机制重复。 |
| B2（新替代） | Core 只保留 ordinary Node/Document/Action、AsciiDoc checklist occurrence parser/toggle；bundled Tasks module 定义 namespaced `tasks/task` schema、状态/日期/依赖/View/提醒与 checklist→Task promotion。Task Node 仍有普通 NodeRef/Resource/Annotation/relations。 | provider 缺失/禁用时 source 与 portable semantics 如何解析、验证、查询、导出；built-in schema 是否随 Core 分发而 UI module 可停用；Task+Project/Calendar composition 是否更简单；promotion 是否能避免 checklist/Task mirror 与第二身份。 |

`tasks/task`、Tasks module/Extension、Profile/schema/membership 等名称都受 §8.5 Lexicon Gate 约束；表中仅是定位示意。B2 不意味着 arbitrary third-party provider 可以接管 Core 写路径或 Node identity。

### 13.3 Checklist、Task、Template 与 provider 边界

- checklist 是 Document occurrence，无 Node identity；parser/toggle 可作为 Core Document 能力。promotion plan、Task facts、Task views/actions/reminders 是 Tasks contribution 候选。无论 B1/B2，都禁止 checkbox/Task 双写镜像。
- reviewers 必须回答：Task 有何不可约 Core 差异；provider missing 时 portable semantics/validation/query/export 如何降级；schema 是否必须 built-in 但 UI 可禁用；Task+Project、Task+Calendar 等组合是否需要重复 specialization。
- Template 可能仍需 Core specialization，因为它是生成其他 Node、分配 fresh identity、重写 refs/slots 的 meta-object；Task 可能只是普通 Node 上的领域 schema。该区分是待证候选，不是结论。Task Template 候选只声明 target Task schema/membership，不把 Template Node 本身变成 Task。
- 若 B2 成立而 Template 保留 Core specialization，必须正式裁决 `wf-specialization` 是否缩减为 template-only、legacy Task source 如何迁移、wire/CLI/query/checklist promotion 如何非回退；不得由 D4 加一个兼容 dual-read 偷渡。

### 13.4 阶段 routing 与 upstream reopen

- Formal D2 replacement owns B1/B2 ontology、Template/Core distinction、与 §12 A/B/C/persisted syntax 的完整交叉比较。D4 later owns People entry/code/schema and only consumes the frozen source/Task boundary. D5 owns list-of-object/labeled-entry/occurrence/Record/Node 替代，不因 UI 列表暗建 identity。D7 owns kinship inverse/Graph 与 Task Query/View/Action composition；D8 owns preset localization、custom text、同名消歧和 provider-disabled UI；D9 owns contact import/loss、Task Template 与 checklist promotion；D10 owns provider/schema/locale-registry availability；A2 全链复核。
- B2 若胜出，会实质改变冻结 D2 Task algebra、wire、query domain、`wf-specialization` 和 checklist promotion，并影响 D3 Task lifecycle、D7 Query/View/Action、D9 Template/import/promotion。因此它只能在 pre-D4 formal D2 replacement Gate 冻结；随后按 impact map 做 D3 amendment，再由未来全新 D4 消费 replacement contracts。
- 旧 D2/D3 权威直到 replacement/amendment Gate pass 并原子更新 ledger/index 前持续有效。D4 继续暂停，不因本 routing update 自动恢复。

### 13.5 必须重放的 fixtures 与最终 Gate 问题

1. 两个同名 Person Nodes，各含 preferred/common/former/transliteration names、home/work/mailing/former addresses、多个 phone/email/IM；切换 zh-CN/en 后 preset 翻译而 custom label/note 不变，title/path 改名不改变 identity。
2. 同一 preset code 用于 phone/address/name/organization-role 时，Lexicon、wire、locale owner 与 UI 均有明确 namespace；未知 locale/unknown preset 有确定 fallback/diagnostic，不把显示文本回写成 semantic code。
3. parent/spouse/sibling/custom 与 literal target 混合；inverse edit、删除、权限、GraphProjection 和 provider unavailable 时只保留一份关系事实，custom/literal 不产生伪 parent graph。
4. 分别以 domain-specific typed values、共享 labeled-entry value、Document occurrence、独立 Record 和 Node 建模 names/addresses/contacts/relatives；比较 identity、局部编辑、并发、query、import/export、Mobile 与实现成本。
5. B1 Task、B2 Task、ordinary Node、Template、Task Template、Task+Project、Task+Calendar；Tasks UI/provider disabled/reinstalled、schema version mismatch、CLI/Mobile external edit 后内容与 identity 不丢失。
6. checklist toggle 与 concurrent promote；retry、expected revision、fresh Task identity、source occurrence disposition、undo/receipt 明确，绝不生成 checkbox/Task mirror。
7. 逐项重放 `(§12 A|B|C) × (§13 B1|B2)`；不得因为偏好 YAML 就默认 B2，也不得因为 Template 仍 Core 就默认 Task 必须 Core。
8. 同一 Person 有两个相互冲突的 birth assertions（一个仅年份、一个完整日期且使用不同 calendar/source）、多个历史姓名和国籍状态、legal sex 与 gender identity 分域、三次带单位/时间的身高体重 observations；默认卡片只显示 current/preferred 摘要，展开后证据不丢失，修改摘要不静默覆盖历史或把来源冲突伪装成单值。
9. 一个 Person 的 preferred birth assertion 为农历/低精度来源，Calendar 以明确规则投影今年 birthday anniversary 并设置 reminder；切换历法 pack、禁用 Calendar、修改 preferred assertion、导入重复联系人后不得产生第二 birth fact、漂移 recurrence 或 orphan reminder，重新启用后派生结果可确定重建。
10. 一名 Person 同时是无组织限定的“律师”，在 Organization A 任两个 position 且 rank 不同，在 Organization B 有历史 appointment，在 Organization C 只有 text fallback；禁用 Organizations、移动/重命名目标 Node、从 inverse UI 编辑、把 text 显式链接为 NodeRef 后，职业、任职、职务、职级、历史与唯一关系事实均不丢失或双写，只有真实 NodeRef 产生可靠反向查询。
11. 同一 Person 有两个相同 phone values、各自不同 note/source/label；并发设备分别重排、修改一项 note、删除另一项，外部编辑器复制一行。分别用 inline-note 与 Annotation-target wire 重放，必须确定命中正确 occurrence、检测 ID collision、处理 orphan，不按 `(key,value)` 串注释；无 note 的普通字段仍保持简洁 source/UI。
12. Person Node 的 basename 是编号、title 为中文常用名，`names[]` 同时含法定名、英文名、曾用名、alias 与转写；另一个 Person 同名。重命名 title、移动路径、修改 preferred name、禁用 People provider 后，NodeRef 链接仍稳定，search ranking/dedupe 与 selector 消歧确定；A/B/C 分别重放且不得复制 names→aliases、把 alias 当 identity，或以 stale index 泄露不可见姓名。
13. 同一组织有 10,000 名 overlapping engagements，colleague 查询按需派生而不创建约 5,000 万条 edges；其中两人另有显式 colleague、friend 与 mentor relations，一个 custom neutral relation、一个 literal relative target。Family Tree、一般关系图、职业层级图和权限过滤只消费匹配 semantics；provider 停用/索引重建后 asserted facts 不丢、derived edges 可重建且不变成第二权威。
14. 同一 Node 组合 People、Tasks、Calendar schema，三者都显示“状态/日期”；source 分别使用候选 full key/scoped block/compact prefix。卸载 provider、第三方尝试声明官方 `people.*`、用户已有裸 `phone`、schema 升级、复制到无 provider Workspace、CLI/API round-trip 后，owner/type/key 均不漂移；显式 map custom `phone` 前有 preview/receipt，Calendar 只派生 birthday 而不复制 People birth fact。
15. 分别以 A flat、B nested、C scoped readable block 表达同一 multi-profile Node：People 含两个 names、两个 phones（各自 note/source/entry ID）、一个 appointment，第三方 namespace 在 People UI 贡献字段，Tasks/Calendar 各有同名 `status/date`。删除 providers、用普通 AsciiDoc 工具编辑/格式化、复制为 Template、并发重排、查询 full IDs 后，source 可读、diff 可审、canonical wire 等价且无 `_weftext` domain blob或 namespace 抢占。
16. 对同一 Person，设备 A 修改 `people.names` 一项 note，设备 B 新增 `people.phones`，另一个角色只有 phones 写权限；分别以 single `people` blob 与 independent fields/scoped grouping 执行 partial patch、merge、annotation、permission denial、extension removal 和 migration。强候选必须让不相关字段不因整体 blob 产生冲突/越权/全量重写，且每个 occurrence target 稳定。

最终 Sol / Pro Gate 必须明确回答：People 的 stable-code/localized-preset/custom-text、labeled-entry 与 fact-shape 抽象是否在不抹平 event/state/measurement/relation 语义的前提下闭合，并兼顾默认简单 UX；Task 是否存在不可由 ordinary Node + schema 表达的不可约 Core差异。若没有，必须解释继续硬编码 Task subset/query domain 的理由；若 B2 更优，必须按上游 reopen 序列完成非回退证据后才能冻结。

## 14. D9 Office 模板内自包含绑定与普通 Node 零导出配置

本节是用户明确要求进入 D9“导入、导出与模板”的强制产品约束，并由 D9 `$council`、独立 DeepSeek V4 Pro 盲审与真正全新的 GPT-5.6 Sol / Pro Gate 收敛后写入权威规范。它不是当前冻结 wire；下列占位符、分隔符与路径仅为候选示意，不得从聊天直接固化。当前 D3 不读取、不处理本节；在 D3 完成独立验收前不得据此修改 `knowledge/Weftext-Control`、`repos/weftext` 或 `brand/`。

### 14.1 用户权威与非目标

- DOCX/XLSX 模板文件本身必须是格式、排版与数据绑定的用户可见权威。实际 Office 格式要求千差万别；必要绑定不能迁移到模板外的“导出方案”、隐藏 mapping、第二模板设计器或其他隐式控制面。
- 普通 Document-bearing Node 为导出保持零额外配置：不得为了 Office 导出要求作者增加 set/field、字段 UUID、隐藏列 ID、位置锚点或复杂 export metadata。普通节点只保存其正常标题、表格标题、多级表头与数据。
- Weftext 可以提供“复制此列占位符”等便利 Action，但用户仍在 Word/Excel 中直接粘贴占位符、排版和设置格式；该便利不产生第二绑定权威，也不把 Office 模板编辑搬入 Weftext 私有设计器。
- Record/RecordCollection 若 D5 最终建立，以及 Node collection 等已有 schema-bearing 集合，可以由导出引擎内部编译成冻结的 typed set/field；这属于既有领域/内部执行模型，不得反向要求普通 Node 保存额外导出 schema 或映射。

### 14.2 普通文档表格的列解析与模板内逐级消歧

- 普通文档表格必须支持模板内自包含列引用。无歧义时允许最短列名形式，示意 `{{ ↓ 金额 }}`；最终语法、箭头或方向标记、转义、解析 scope、case/Unicode normalization、全半角、空白与标点规则必须由 D9 冻结。
- 解析基数必须 fail-closed：0 个匹配明确报告不存在；恰 1 个匹配成功；多个匹配明确报告歧义。禁止 first-wins、last-wins、静默按旧位置/缓存继续绑定、自动追加不可见后缀或依据编辑器布局猜测。
- 发生重复时，模板必须能够逐级增加限定：完整多级表头路径、表格标题、同名表及同路径列的 occurrence ordinal。示意 `计划 / 金额`、`年度预算 :: 计划 / 金额` 或完全限定的结构化表达都只是候选；最终 wire 必须覆盖同一 Node 多表同名列、同一表多级表头重复、表格标题重复，以及名称含空格、分隔符、标点、CJK、RTL 与 Unicode 组合字符。
- 同一 DOCX/XLSX 重复样板带、循环区或一行输出 group 中的全部列必须解析到同一张源表及同一 row set。不得把各占位符独立全局查找后跨表拼行，造成数量不同、排序不同或行错位；group 的 source-table unification、row cardinality、filter/order 与 empty-set 行为必须进入 wire/error/test。
- 后来新增同名列或同名表导致原简写不再唯一时，既有模板必须转为确定歧义并阻断，要求用户显式更新模板。不得因模板曾经成功而用历史位置、内部 ID 或旧缓存悄悄保持绑定；成功解析结果可作为可重建编译缓存，但不是作者权威。

### 14.3 与现有 Office export 规范的关系

- 当前公开 `23-office-export-template-profile` 的 `data.<set>.<field>`、封闭点路径、禁止括号等规则可能不足以表达普通文档表格的 table/column selection 与重复消歧。D9 必须把它作为待验证历史候选，不能因已存在而默认保留，也不能把本节示意 syntax 直接替换进去。
- D9 必须明确区分并分别冻结：A) 普通 Document 表格的模板内 table/column binding；B) Record/Node collection 等 schema-bearing set/field binding；C) 普通无模板 XLSX 导出。三类可共享 parser/typed compiled plan，但不得共享到让普通 Node 被迫拥有 export-only schema 或让同一 token 在不同 context 下靠猜测解释。
- 若裁决修改规范 23，必须同步检查并原子更新受影响的规范 15、20、24、Architecture Decision Ledger、实现影响、测试轮廓、兼容/迁移或明确“无历史兼容负担”的规则，以及 Terminology and Naming Lexicon。模板/profile/preset/export plan、table/column/header path/set/field/occurrence 等术语必须一词一义。
- D9 输出必须包含候选、完整 source/template 示例、解析 grammar、typed compiled wire、preview/commit/receipt、错误优先级、实现影响、测试轮廓、P0/P1 closure ledger、最终 Gate 证据，以及对 existing template profile 的保留/修订/退役裁决。

### 14.4 强制 fixtures 与非回退门禁

1. 一个普通 Node 中有两张表，均含“金额”；最短列名产生歧义，加入表格标题后唯一。
2. 同一表有多级表头 `计划/金额` 与 `实际/金额`；裸“金额”歧义，完整 header path 唯一。
3. 同一 Node 有两张同标题表且 header path 相同；必须用确定 occurrence ordinal 或更明确的模板内限定，且插入新同名表后的 observable 行为被冻结。
4. 表格标题、各级表头包含空格、斜线、双冒号、引号、括号、emoji、CJK、组合字符与 RTL；escaping、normalization 与 exact display/canonical comparison 不靠实现语言默认值。
5. 一个重复样板带同时引用三列；其中一列在另一表也唯一命中，而另两列命中第一表。整个 group 必须因 source table 不统一而拒绝，绝不能跨表拼接。
6. 模板最初只有一个“金额”列并成功；Node 后来新增另一个同名列。下一次导出必须报告歧义并要求更新模板，不得沿用旧位置、内部缓存或自动后缀。
7. 同一 DOCX/XLSX 使用普通 Node 表格、Record set 与 Node collection；每一 binding domain 显式可判定，普通 Node 没有任何 export-only source metadata。
8. 用户在 Word/Excel 中复制、移动、重复样板、调整样式、重命名表格/表头；preview 必须精确列出解析目标、row set、歧义与 loss，commit/receipt 绑定模板 bytes/hash 与输入 revision/cut。
9. 普通无模板 XLSX 导出不需要 Office template placeholder，也不因规范 23 的 syntax 变化而被迫生成隐藏 mapping；列名冲突和 sheet naming 另有确定规则。
10. provider/module unavailable、Mobile 无 Office 编辑器、模板损坏、未知 token、恶意深层 header/超大表、外部编辑破坏 placeholder、并发修改 source Node 后的 stale preview 均有 fail-closed error 与可恢复路径。

D9 最终 Gate 不得遗漏以下五条 acceptance constraints：**模板自包含；普通 Node 零导出配置；最短唯一列名；歧义时只在模板内逐级消歧；严格阻断且绝不猜测**。A2 必须把 D9 结果与 D2 exact-source/Template、D5 Record domain、D6 revision/transaction、D7 collection/query、D8 UI/diagnostic 和 D10 automation/worker capability 做全链复核。

## 15. Query 结果可视化与版本化图表 ViewSpec

本节是用户授权保存并进入后续正式收敛流程的强制评审输入，不是已接受的产品目录、实现承诺或冻结 wire。所有 layout 名、binding 名、diagnostic、JSON token 与 version 拼写均为可评审候选；D7/D8 必须在各自全新、自包含的 `$council` prompt、盲审包和 Sol / Pro Gate 中完整带入并从零比较，D9/D10 按导出与 capability owner 交叉复核，A2 做全链全局最优审查。当前 D3 不读取、不处理本节；不得据此修改 `knowledge/Weftext-Control`、`repos/weftext` 或 `brand/`。

### 15.1 Query、View、Action 与交互状态的封闭边界

- Query 只负责 `source/scope/filter/derive/traverse/project/aggregate/sort/take`，产出已经权限过滤、类型化、语义完整且有界的 rows 或 scalar result。QuerySpec 不承载图表颜色、坐标轴、图例、布局或 renderer 配置。
- ViewSpec 只按 Query terminal public result schema 的稳定 field ID 做呈现绑定。它不得包含 filter、sort、take、aggregate、bin、quantile、权限/source、Action 或任意表达式，也不得通过 EntityRef enrich、隐式 join、客户端查询扩大 read-set。
- 图表不是任意 ECharts/Vega/JS/CSS 配置。ViewSpec 必须是 closed、versioned、deny-unknown-fields 的 discriminated union；每个 layout 的 bindings/options 集合封闭，不适用字段省略，不能用 `null`、unknown object 或 provider-private evaluator 扩展，插件不得注入脚本。
- 图表消费完整 semantic result，不能拿当前 page/window 偷画。超预算必须阻断或要求作者在 Query 中显式 aggregate/take；禁止静默 sampling、truncate、Top-N、自动“其他”、丢 null、自动 bin、自动 quantile 或偷偷排序。
- V1 图表全部只读。点击最多通过既有 ActionEvidence 打开已在结果中的来源对象；拖拽甘特、刷选回写 Query、图上编辑等写操作必须另行评审和授权，不能因可视化获得写权限。
- Portable Query/View authority 与 device-only interaction state 分离：hover、selection、zoom、pan、fold、临时 legend toggle 属设备状态，不改变作者事实。改变成员集、顺序、聚合、分箱或权限只能修改 Query/权威配置。
- Dashboard 不得成为一个暗中运行多个 Query 的超级 ViewSpec。当前优先用多个各自拥有 Query+View 的 DynamicBlock 组合；未来 dashboard container 若成立，另行冻结 layout、refresh、authz、subscription、错误隔离与 export。

### 15.2 候选实施目录与优先级

现有非图表布局候选继续保留并需非回退比较：`table/list/task-list/board/calendar/timeline/gallery/form`。新增 chart layouts 不得吞并这些语义或让 chart 成为默认 fallback。

第一批通用图表候选，优先覆盖知识工作常见结果：

1. `metric`：单值/KPI 卡，优先于仪表盘指针图。
2. `bar`：一个 layout 覆盖水平条形、垂直柱状、grouped 与 stacked；不为 bar/column 建两个领域类型。
3. `line`：折线；closed option 可启用 point 或 area，面积不是独立数据语义。
4. `scatter`：散点；可选 `size` 后得到 bubble 表现。
5. `pie`：`shape=pie|donut`；严格限制扇区数量和非负值，不自动生成“其他”。
6. `heatmap`：两个离散/有序轴加 numeric color；可覆盖普通热力图和 calendar heatmap 的受限表现，但不改变数据域。

第二批 Weftext 高价值结构/计划图候选：

7. `tree`：真正父子层级，不等于普通 Graph/network。
8. `treemap`：复用显式 `id/parent` hierarchy，另绑定非负 `size`。
9. `gantt`：与现有 timeline 明确分域；读取 item/task id、label、start/end，可选 progress/lane/parent/dependencies；V1 只读。

第三批只在 D7 数据语义或更重验证闭合后进入实现候选：

10. `histogram`：只消费 Query 显式输出的 `binStart/binEnd/count`，View 不自行选箱。
11. `boxplot`：只消费 Query 输出的 `minimum/q1/median/q3/maximum` 与可选 outliers。当前 aggregate 若无 quantile，View 禁止私算；D7 如引入 quantile，须冻结概率参数、插值法、exact numeric、null/empty 和并行确定性。
12. `network` / `sankey`：必须先冻结 node/edge 或 source/target/weight 契约、重复边、环、隐藏端点和 non-disclosure；tree 不能冒充通用 graph。
13. `sunburst`：hierarchy 的径向表现，只在 tree/treemap 契约稳定后评审。
14. `waterfall`、`funnel`、`radar`、`combo`、`bullet`：作为后置 closed layouts/options；combo 不默认开放双 Y 轴。
15. `geo`/地图、`candlestick`/K线：需要专门地理/金融 type、schema/profile/provider，不能进入无类型 Core V1。

明确反例：Core 首发不含 3D 图、词云、任意插件脚本图表；`gauge` 默认由更可读的 `metric`/`bullet` 替代。目录最终可删减、改名或分期，但 reviewers 必须说明覆盖能力、概念数量、无障碍和长期维护成本，不能按图表库组件清单照搬。

#### 15.2.1 常见图表家族 coverage checklist（不改变分期）

本清单用于检查上述目录是否覆盖常见数据语义，防止把 layout inventory 误读成图表库组件清单。它不把下列家族预先批准为 Core V1 独立 layout；reviewers 必须优先证明能否由既有 layout 的 closed bindings/options 表达，只在数据契约或验证语义确实无法封闭时提出后置独立候选。第一批仍固定为 `metric/bar/line/scatter/pie/heatmap`，第二批仍为 `tree/treemap/gantt`。

1. **error bar / confidence band**：只消费 Query 显式输出的 `low/high/center`；View 不推断误差、置信区间或样本分布。优先比较作为 bar/line 的 closed interval bindings/options；若两种 layout 的尺度、key、null、series 或区间语义无法共享，才提出后置 `range` 候选。
2. **range/floating bar 与 interval plot**：消费显式 `start/end` 或 `low/high`，并冻结同型、ordering、闭开边界、negative/zero span 与 overlap 表现。不得与可写 gantt 或普通 baseline bar 混义，也不得从单值和视觉位置反推区间。
3. **density/violin/beeswarm**：需要 Query 冻结 density、quantile 或 raw-observation budget、sampling prohibition、kernel/bandwidth（若适用）与 deterministic output。View 不私算 KDE、quantile、jitter 或 observation thinning；作为统计后置候选，不进入 Core V1。
4. **slope/bump/rank-change**：优先证明可由 line 加明确 categorical/order/rank result 表达。rank 与 tie policy 必须来自 Query；View 不重排、不自行排名或补 missing period。只有 line 的 closed contract 无法表达时才后置新增 layout。
5. **stacked area/streamgraph**：优先作为 line 的 closed area/stack options；进入候选前必须冻结正负值、baseline、series completeness、missing/null 与 deterministic layer order。View 禁止静默补零、center baseline、重排 series 或改变 aggregate。
6. **parallel coordinates**：依赖多维 typed numeric/ordinal axes、per-axis scale、normalization、null、brush 与 cardinality contract，验证和可访问性成本高；后置评审，不进入 Core V1。
7. **chord diagram**：作为 network/sankey 同一显式 edge/weight 数据族的后置表现；不得绕过 hidden endpoint/non-disclosure、duplicate edge、self-edge、cycle、direction、weight 与 authz-filtered completeness 规则。
8. **既有受限表现/别名**：`calendar heatmap` 属 heatmap 的受限变体，`donut` 属 pie shape，`bubble` 属 scatter+size，`area` 属 line option，`column` 属 vertical bar。它们不是新数据语义或独立 authority。`gauge` 继续优先由 metric/bullet 替代；word cloud、3D 与任意脚本图表继续明确不建议。

D7/D8 Gate 必须对每项给出 `reuse existing closed layout | defer as specialized candidate | reject` 的显式 disposition，并说明 binding/result schema、null/completeness、预算、无障碍和 renderer 成本；不得仅因为底层 chart library 有对应组件就增加 public layout，也不得以复用为名让一个 layout 接受开放 option blob。

### 15.3 Portable ViewSpec 候选形态

沿 DynamicBlock 候选保持 Query 与 View 两个权威对象；inline/saved 均可。下例只用于评审对象定位，不冻结 token：

```json
{
  "format": "weftext.dynamic-block",
  "version": 1,
  "query": { "kind": "inline", "spec": { "...": "QuerySpec" } },
  "view": {
    "kind": "inline",
    "spec": {
      "format": "weftext.view",
      "version": 1,
      "layout": "bar",
      "title": "各状态任务数",
      "bindings": {
        "category": "state",
        "value": "task_count",
        "series": "project"
      },
      "options": {
        "orientation": "vertical",
        "mode": "grouped",
        "labels": "value"
      }
    }
  }
}
```

`bindings` 的 value 必须是 Query terminal public schema 的 field ID，不是 display label、source path、CEL、JS 或 expression。准确 envelope、saved/inline reference、hash/version 与 existing `.weftext-query view=...` 的保留/替换均待 D7/D8 裁决。

候选 layout bindings 与 closed constraints：

- `metric`: required `value`，optional `label/comparison/target`；优先 `result.kind=scalar`。
- `bar`: `category,value`，optional `series/color/label`；orientation `horizontal|vertical`，mode `grouped|stacked`。
- `line`: `x,y`，optional `series,label`；x 是 orderable numeric/date/instant；option 只允许 `linear|step`、points、area、显式 `missing=reject|gap`，不做 smoothing/trendline。
- `scatter`: `x,y`，optional `series,size,label`；x/y numeric，size 非负 numeric。
- `pie`: `category,value`，optional `label`；value 非负 numeric、category 唯一；shape `pie|donut`。
- `heatmap`: `x,y,value`，optional `label`；`(x,y)` 唯一，value numeric。
- `tree`: `id,parent,label`，optional `color`；id 唯一，parent 同型 optional，parent 必须存在或是 root，cycle/self-parent 拒绝。
- `treemap`: tree bindings + required `size`，optional `color`；size 非负。
- `gantt`: `id,label,start,end`，optional `progress,lane,parent,dependencies`；start/end 同为 date 或同为 instant，且 `end>=start`；progress exact decimal `0..1`；parent/dependency 使用同一 id domain，unknown/self/cycle 规则封闭。
- `histogram`: `bin_start,bin_end,value`，optional `series`；bins 不重叠，boundary/open-closed 规则属于 Query output contract。
- `boxplot`: `category,minimum,q1,median,q3,maximum`，optional `series,outliers`；同一 numeric type 且 `minimum<=q1<=median<=q3<=maximum`。
- `network/sankey`: 至少 `source_id,target_id`，optional source/target label、weight、kind；同 id label 冲突、duplicate edge、cycle、hidden endpoint 与权限语义必须封闭。

### 15.4 完整结果、验证和稳定错误语义

- 执行 View 前必须检查 result kind、field existence、field type/optional、required null、row cardinality、semantic key uniqueness、完整结果标志与预算；validation/error priority 必须机械稳定。
- 稳定 diagnostics 至少比较并冻结：`unknown_layout`、`unknown_binding`、`result_kind_mismatch`、`binding_type_mismatch`、`missing_required_value`、`duplicate_chart_key`、`incomplete_result`、`view_budget_exceeded`、`negative_measure`、`invalid_interval`、`hierarchy_missing_parent`、`hierarchy_cycle`、`dependency_cycle`、`unsupported_renderer`。准确名字可变，但语义不可遗漏或靠 renderer exception 代替。
- required null 默认拒绝；只有 layout 明确允许且 ViewSpec 显式声明（如 line `missing=gap`）才表现为空隙。禁止默默跳行。
- bar 的 `(category,series)`、line 的 `(series,x)`、heatmap 的 `(x,y)`、pie category 必须唯一；重复表示 Query 未完成所需 aggregation，View 报错，不私下 sum、last-wins、dedupe 或排序。
- pie/treemap/bar 的比例与 stack 只做给定 numeric values 的几何归一化；percent label 可作为表现，但不产生新数据。group/aggregate/Other/bin/quantile 仍由 Query 负责。
- tree/treemap 的 forest policy、multiple roots、parent equality 与 deterministic order 必须显式；gantt dependency/parent 可见性、hidden predecessor 和 cycle 需在 authz-filtered result 上具有 non-disclosure error。
- Renderer/capability 不能改变 ViewSpec 含义。SVG/Canvas/第三方库只是实现细节；不支持时明确失败或显示规范 data-table fallback，不能换成语义不同布局。

### 15.5 可访问性、跨端一致性与导出证据

- 每种图都必须提供同数据的 accessible table、文本摘要、键盘遍历和 screen-reader name；颜色不得是唯一编码。高对比、zoom、reduced motion、CJK/RTL、色觉差异必须作为验收门禁。
- Desktop/WebUI/Server/Mobile/print/export 只要求语义等价，不要求 pixel-identical；locale/number/date formatting 不能改变 field identity、sort 或 aggregate。
- export/print/snapshot 必须绑定 Query semantic hash、ExecutionEnvelope/authz generation、ViewSpec hash、renderer/profile version 与完整-result evidence；不得从未经授权、过期或 partial cache 导出。
- provider unavailable、renderer crash、Server headless 缺失字体、Mobile 不支持布局时，规范 fallback/diagnostic 不泄露 rows，也不把旧图标为 current。

### 15.6 阶段 owner 与路由

- D4：chart binding 可接受的类型、optional/null，以及 numeric/date/instant/enum/EntityRef/list 等 typed-value 边界。
- D5：Record/Node collection 域、relation/hierarchy input 与 row authority，禁止 View 混用不同 authority 或暗建 Record identity。
- D6：SavedView/DynamicBlock authority、revision/cache/subscription、完整结果 delivery、portable/device state、receipt/export snapshot。
- D7：terminal result schema/field IDs、aggregate、quantile/bin 候选、hierarchy/dependency/edge result、完整且 bounded result；必须完整带入本节。
- D8：ViewSpec closed union、builder、renderer、interaction、accessibility、five-surface parity，以及 existing `.weftext-query view=...` 与 DynamicBlock 候选的保留/替换；必须完整带入本节。
- D9：DOCX/XLSX/PDF/图片/SVG/print 中的 chart export、字体/色彩/分页/alt text 与 loss，并与 §14 Office template authority 交叉复核。
- D10：renderer/provider capability、Server headless rendering、sandbox 与 extension boundary；禁止任意脚本图表。
- A2：Query/View/Action/permission/export/Mobile/Graph/Calendar/Organizations 全链复核，并从零判断更少、更统一的 View 概念体系。

### 15.7 强制 fixtures 与 Gate 问题

1. `task_count by state` grouped aggregate → bar；duplicate `(state,series)` 必须拒绝，不能私下合计。
2. date+value multi-series → line；覆盖乱序、同 x duplicate、optional.none、explicit gap 与 page incomplete。
3. pie 含负值、0、duplicate category、过多扇区；不得自动 Other、drop 或 top-N。
4. hierarchy 有 two roots、missing parent、self-parent、cycle、同 ID 不同 label；tree/treemap forest policy 明确。
5. gantt 混用 date/instant、`end<start`、progress 越界、unknown/self dependency、dependency cycle、hidden predecessor；只读与未来拖动 Action 分界清楚。
6. boxplot 的 q1/median/q3 次序错误；raw observations 禁止由 View 私算 quantile。
7. chart delivery 只有第一页而 semantic result 更大；必须 `incomplete_result`，不能渲染 partial chart。
8. ACL generation 变化、subscription reset/invalidate、offline stale、provider unavailable、renderer crash；旧图不得继续标 current。
9. Desktop/WebUI/Server/Mobile/print/export 对同一 Query+View 语义等价，均有 data-table fallback、CJK/RTL/色觉/键盘证据。
10. Dashboard 组合多个独立 blocks，其中一个权限失败；失败块不泄露、不污染或阻断无关块，精确隔离等待单独 container contract。

D7/D8 最终 Gate 必须明确回答：Query/View 分工是否封闭、图表是否严格依赖完整 semantic result、ViewSpec 是否保持版本化 closed union、第一/二/三批目录是否以最少概念覆盖真实用途、Graph/tree/timeline/gantt/calendar 是否不混义、以及无障碍/跨端/导出是否在不扩大权限和 read-set 的前提下可实现。任何 accepted 顶层变更必须按协议重新生成完整包并做 DeepSeek 与 Sol / Pro 非回退审查。

### 15.8 跨图表合同补充与强制 fixtures

状态：用户授权进入 D7/D8/D9/D10/A2 的非冻结强制评审输入。它不改变第一批 `metric/bar/line/scatter/pie/heatmap`、第二批 `tree/treemap/gantt` 的分期，不批准任何新 layout，也不把以下示例 token、字段或方案升级为 wire。D7/D8 必须从零裁决；D9/D10/A2 只按各自 owner 交叉复核。

#### 15.8.1 多 measure、多 series 与 result shape

当前 chart bindings 多为单 `value/y` 加 optional `series`，而 Query v1 result 只有 `rows|scalar`，且 v1 不含 dynamic pivot、collect、window/rank。D7/D8 必须比较并显式 disposition：

1. ViewSpec 使用 closed `values/measures` list；
2. Query 增加规范 unpivot/long-form 投影；
3. V1 只接受 long-form，并明确拒绝 wide form；
4. 满足同一权限、类型、确定性和可移植约束的更优封闭替代。

禁止 renderer/client 私自把宽表 melt/unpivot、用 display label 生成 series key、改变 read-set，或把缺失 measure 猜成 zero/null。强制 fixture：`month,revenue,cost` 三列绘制双系列 line/bar；字段显示名改变但 stable field ID 不变；同名 display label 不冲突；missing measure、optional measure、mixed numeric type、series 数超预算均机械失败。

#### 15.8.2 顺序语义

§15 禁止 View 私自 sort，但现有 line fixture 又要求覆盖乱序输入。D7/D8 必须冻结且只能选择一个封闭合同：Query 明确 `orderBy`、View 验证 monotonic 并拒绝、View 使用显式 order binding，或证明更优替代；不能由 renderer 猜。

同一裁决必须覆盖 bar category、line x、series/legend、heatmap axes、tree sibling、facet panel、network node/edge 的 deterministic order 和 tie-breaker。禁止 renderer 默认排序、locale collation、object/map iteration 或 chart-library insertion order 改变语义。强制 fixture：同一 bag 的不同交付顺序、duplicate x、equal sort keys、CJK/RTL/Unicode、subscription move/reset；Desktop/WebUI/Server/Mobile/export 的顺序语义必须等价。

#### 15.8.3 Axis、scale、unit 与 time contract

View 虽拥有坐标轴、图例和格式，但必须使用 closed、versioned contract。D7/D8 至少裁决 categorical、linear、date/instant scale；`log/symlog` 若后置必须返回 stable unsupported。还必须冻结或明确拒绝 domain、zero baseline、clamp、reversal、tick/label、shared scale、正负 stack 与 100% stack。

unit/currency/percent/display scale 不得改变 Query 数值事实；例如 `0.42` 显示为 `42%` 只能来自封闭、版本化呈现语义。不同单位不得只靠 label 叠在同一 axis。date/instant 必须绑定显式 timeZone/calendar/DST 语义，禁止使用设备默认值；几何位置、bucket 边界和排序不得随 locale 改变。强制 fixture：negative bar、mixed positive/negative stack、all-positive/all-negative、log 含 zero/negative、decimal scale、currency mismatch、date+instant 混用、DST fold/gap，以及不同 locale/theme/renderer。

#### 15.8.4 Empty、zero-total、sparse 与 missing state

必须冻结稳定语义、no-data state 与 diagnostic，覆盖：0 rows、scalar `optional.none`、single point、all required values missing、pie/treemap total=0、heatmap missing cell 与 explicit zero、stack series 缺 category、histogram gaps/empty bins。

不得把 empty result 冒充 unavailable/unauthorized，不得静默补 zero/category、沿用旧图或显示 NaN。`loading|empty|error|stale|incomplete` 必须可区分、无泄露且跨端等价。强制 fixture 还要证明 ACL-filtered empty 与真正 empty 在外部不可区分，同时内部 evidence/diagnostic 不泄露隐藏计数。

#### 15.8.5 Facet / small multiples

Facet 是同一个完整 Query result 的呈现分区候选，不等于 Dashboard，不是多 Query container，也不能产生隐藏 aggregate/filter。D7/D8 必须比较 cross-layout closed facet binding/modifier 与独立 layout，冻结 panel key/order/budget、`shared|independent` scale、empty panel、禁止分页、Mobile collapse、accessible traversal 与 export。

强制 fixture：使用一个 `project` field 把 line 分成多个 panels；facet 数超预算；某 panel 无值；shared 与 independent scale；一个被权限隐藏的 panel 不得泄露其存在。

#### 15.8.6 Tooltip/details、reference/target overlay 与 author description

增加 closed tooltip/details field-ID bindings 候选；hover、focus、keyboard、accessible table 必须使用同一组已投影值，禁止 EntityRef enrich 或 tooltip 时补查。reference line、threshold、target、confidence band 只能消费 Query 显式 field 或封闭 presentation literal；领域事实不得藏在 device-only state。`metric target` 与 bar/line target 必须一词一义。

View 可有 portable author description/alt-summary 候选；自动摘要不得虚构趋势、因果或统计结论，并须具有 screen-reader/print/export 等价。强制 fixture：tooltip field removed/retyped、hidden EntityRef、keyboard without hover、author description CJK/RTL、threshold outside domain、target unit mismatch。

#### 15.8.7 Hierarchy、graph 与 gantt 未闭细节

- **treemap/sunburst**：必须裁决 parent size 是 self value、children sum、leaf-only，还是要求相等；禁止 parent+children double count，并冻结 zero-size、forest、mixed internal/leaf 和 color aggregation。
- **network/sankey**：必须裁决 rows edge-list、node+edge multi-result 或更优 result kind；冻结 node attribute duplicate conflict、parallel/self edge、isolated node、direction/weight、authz-hidden endpoint。renderer layout seed/positions/device state/export reproducibility 必须确定，随机漂移不得冒充语义变化。
- **gantt**：必须明确 milestone、all-day/date interval inclusivity、dependency kind（FS/SS/FF/SF）、lag、critical path、working calendar 是否支持；未支持项返回 stable unsupported，不能让 renderer 自行解释 `dependencies` list。

强制 fixture：parent value 等于/大于/小于 children sum、isolated node、同一 graph 不同行序、random layout reproduction、milestone、`end==start`、不同 dependency kind/lag、DST/all-day。

#### 15.8.8 Color/legend stability、schema drift 与 ViewSpec evolution

color binding 的允许类型必须封闭；raw CSS/color string 不得成为脚本、样式或主题逃逸。冻结 category/series stable key 到 palette slot 的确定映射，refresh/reorder 后不得漂移；同时定义 theme/high-contrast/export fallback、legend label/order 与 device-local hidden state。

SavedQuery/ViewSpec 的 stable IDs、revision/hash、canonical serialization、unknown version/feature、field removed/renamed/retyped、saved-query definition change、copy/export/import、renderer-profile upgrade 都必须具有 fail-closed 或显式 migration contract；禁止按 display label 重新绑定。layout-specific budget 至少覆盖 points、series、categories、panels、nodes、edges、labels、output bytes；超限只返回稳定 non-disclosing error，不自动 sampling。

强制 fixture：same label/different stable key、palette exhausted、theme switch、legend local hide、SavedQuery field retyped/removed、old ViewSpec、新 Core/旧 Core、renderer unavailable、subscription 期间 schema change。

#### 15.8.9 当前必须显式回答的两个硬问题

1. 在 Query result 只有 `rows|scalar`，View binding 只有 single value/series，且没有 unpivot/collect 时，常见 wide multi-measure chart 无法机械表达。D7/D8 必须选择 closed measures、规范 long-form/unpivot、V1 拒绝 wide form或更优封闭方案；renderer/client 私自转换不算答案。
2. “View 禁止 sort”与“乱序 line fixture”存在未闭合同。D7/D8 必须冻结 Query order、View monotonic reject、显式 order binding 或更优替代；renderer 默认排序、locale 排序和 insertion order 均不得作为隐式语义。

本节最终只能产生 reviewers 的 alternatives、disposition、wire/error/test 结论；在 D7/D8 对应 `$council`、fresh DeepSeek 与全新 Sol / Pro Gate 通过并落入 Weftext-Control 前，全部内容仍是强制评审输入而非已接受实现。
