---
_weftext:
  id: "3240b157-f174-4ab2-a3f2-d2b3be045388"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 Query、View、Action 与 Dynamic Block — revision06-draft

状态：candidate；architecture-only；尚未评审或激活。规范由本主稿、Value/CEL、Query Algebra、View、Execution/Action、Narrow Field Qualification、Prepared Action Binding、Definition Transfer、Preview and Effects Transport、场景、术语及 Impact 共同组成。四个专项合同是规范正文，不能仅读摘要。本文不证明任何产品、host、发布或完整 parser 已实现。

## 1. 问题、边界与输入

用户需要同一机制完成内容检索、可复用参数查询、节点集合、任务/人员/组织/日历/资料库投影、完整统计、图表和受控修改。Core 是解释和执行权威。Desktop/CLI/Mobile 的本地 Core 与 Server Core 执行相同合同；WebUI 只调用 Server。界面、renderer、插件和转换 worker 不补做领域计算或直接写源。

冻结基线为 D1、D2 Profile2/wire2、D3 wire10/Result8、D4 Context2/Binding2 与 Recurrence1、D5 无持久 Record、D6 coordinated-r05。本包明确提出完整D3 wire11/Result9和D6 Policy/2联合replacement，以及D4仅消费wire/Result版本的同步；必须连同完整替换稿独立审查后才可共同激活。D1/D2和D4类型/领域语义保持各自原合同。QuerySpec v5 仅为待复核输入，不是当前产品 wire。

Query 选择和变换数据；View 将已经完整成功的数据映射到展示通道；Action 准备明确修改并交给 D3/D6 原事务；DynamicBlock 保存一次 Query 与一次 View 调用及显式绑定。四者均不创建新的内容实体类别。

## 2. 扩展不等于新语法

Query 的域是上游通用对象/occurrence，操作是通用代数。FacetId、FieldId、关系 FieldId 是 Registry 数据，不是 Query grammar 的枚举。设备、课程、合同等新领域，使用 D4 已有类型和约束组合注册定义后，使用原 scan/read/filter/traverse/aggregate；不得增加 `scan_people`、`scan_organizations` 或每领域 CEL 函数。内建 namespace 不获得额外权限。

Registry 在同一 AuthorizedCut 内提供版本固定的静态 schema。每个字段引用须是编译期确定的完整 FieldId；CEL 不能用运行时字符串挑任意 Field 或绕过依赖分析。参数可控制值与 exact entity 集，不能动态改变 FieldId、FacetId、算子名或类型。扩展的物理安装、代码运行、签名、凭据、升级和外部数据获取归 D10；Query 只消费已经接纳的定义和已进入当前 authority 的作者事实。

已知定义不可用、未知定义、未知通用 feature 分别是 `definition_unavailable`、`unknown_definition`、`unsupported_feature`；均不作为无字段/无行继续。这里的诊断只在定义可披露后返回，否则 `not_visible`。移除已用定义会使依赖它的查询不可用并使旧结果 reset，不推测为 text 或忽略相应筛选。未使用的未知 opaque 作者条目按 D4 保留，不强制整个无关查询依赖它；但 D2 全源有效性及实际读取的资格仍须满足。

只有新增通用算子、类型或结果语义时，才扩展稳定 feature ID 合同。例如本版Algebra§11显式周期边界投影首次冻结共同时间语义；设备、课程等后续Field复用该投影不改变grammar。兼容新增不提升语义 major；改变旧输入的含义、默认值、类型/错误、权限、排序或 portable wire 必须新 major。Core 从完整调用闭包推导 feature 集；作者没有 `requires` 字段。新业务 catalog 条目不是 Query feature。

## 3. 无身份结果与六类边界

唯一 durable 内容引用为 D3 完整 NodeRef、ResourceRef、AnnotationRef。WorkspaceRef 是 namespace。Task=ordinary Node+exact tasks/task；Template=Core meta-kind；二者仍是 NodeRef。Document、heading、checklist、native row、Field occurrence、Query/View 定义、结果行、Calendar occurrence 均不成为第四类实体。不存在 RecordRef、TaskRef、ViewRef、CollectionRef、DocumentRef 或裸 UUID coercion。

| 类别 | 寿命和作用 | 不允许的用途 |
|---|---|---|
| EntityRef | D3 qualified stable identity；当前授权、生命周期另验 | ref 存在不授 read/write；标题、路径不替代 |
| Locator | D3 完整 revision-bound 位置；单独 resolver | 没有通用Locator TypeSpec或CEL resolver；D4作者provenance可按Value§5.1显式投影为普通结构值，不授予定位/写入能力，不变成durable ID |
| LogicalOccurrenceKey | Core 内部语义 invocation/operator/来源 occurrence 的规范键 | 不进 CEL、View、wire、诊断、日志给调用者 |
| ResultRowHandle | 某个完整 result epoch 内一次行选择；opaque、短期 | 不缓存成永久身份，不跨 result 续用，不直接写 |
| Provenance | Core 内部该行依赖和来源轨迹 | 不是 D4 作者 provenance 值，更不是授权 |
| ActionEvidence | 当前主体、动作、精确目标、版本、范围、期限的受管证明 | 不泛化为 permission/capability，不扩大目标 |

结果 cells 中的 D4 provenance 是作者提供、经原类型门及所属完整作者事实读取授权的普通值；名称必须限定为 `authoredProvenance`。其中作者已保存的Ref/Locator按Value§5.1完整投影原值，不查询或声称目标当前状态/位置，不与Core内部行lineage/Locator的禁止泄露混同。实际解析、关系endpoint和内容读取另走目标当前授权。D3 Query Provenance仍内部，不以同名字段输出；公共作者值不携带或恢复内部ActionEvidence。

## 4. 规范文档与版本

所有 D7 JSON 使用 strict UTF-8、单个 JSON object、重复 key/未知 member/未知 enum 拒绝、unpaired surrogate 拒绝。结构整数遵守 D6 Counter 的非 bool、无小数/指数/-0 词法及指定上界。所有可选 member 缺省规则逐一规定；未列可选即必填。不接受 JSON null。此限制仅作用 D7 自有 wire，不重写 D2 中明确必填 null 或 D3 原 wire。

QuerySpec 顶层 exact keys：`format:"weftext.query"`, `version:1`, `parameters`, `relations`, `scalars`, `result`。保存时由SavedQueryDefinition wrapper承载QuerySpec和可选CollectionCreationPolicy，纯QuerySpec不含创建策略。ViewSpec 为独立格式；QuerySpec 无 view、layout、pageSize、refresh、writes。查询 author JSON 不是结果 JSON，result 不是源。

`parameters` 为 ParameterSpec 数组，每项 `{name,type,required,default?}`。name 满足 `[a-z][a-z0-9_]{0,63}`，唯一；required=true 禁 default，required=false 必有 TypedLiteral default，Optional 默认 none 也须显式写。arguments 是同名 TypedLiteral object：未知参数拒绝，缺 required 为 unbound_parameter，缺 optional 使用定义默认，显式 none 仅对 Optional 合法。类型严格相等，不隐式 nullable/int/ref 转换。

每条 relation 有 `id` 与 `op` 及其所属算子成员；id 用同样 name grammar，relation/scalar ID 分命名空间。relations、scalars 可按任意合法拓扑顺序列出，引用必须解析；joint graph 含 relation→scalar、scalar→relation，cycle 拒绝。不存在隐式 main pipeline。result 为 `{kind:"rows",relation:id}`、`{kind:"scalar",scalar:id}` 或 Execution §2 的完整 graph variant。从 result 不可达的节点拒绝，防止隐藏表达式或错误分支。定义参数可未使用，但调用者不能借未使用参数影响结果 identity。

规范化保留操作与数组中的语义次序，例如 project 字段顺序、sort keys、literal list；不保留本地 ID 拼写或独立节点的拓扑排列。使用 §5 的 canonical graph 描述；不是对任意等价 SQL/CEL 做归一。产品缓存可使用 D3-CJ3 加 SHA-256 域分离摘要，但必须检查对应完整规范描述，不以 hash 碰撞取得错误 schema/权限。此为产品算法，与评审包装无关。

## 5. 规范 DAG、错误与优化

CanonicalGraph(Q)：先去除 id，并以引用指向图节点。以 result 为根，按成员名 UTF-8 顺序、数组 index、表达式 AST 子节点顺序遍历依赖边；首次访问某节点分配从0递增 canonicalOrdinal，再递归其依赖。重复引用重用 ordinal。规范记录逐 ordinal 保存 op、非引用参数、规范表达式 AST（其中scalar引用也替换为目标ordinal，普通string literal不替换）、引用 ordinal、声明字段顺序；ParameterSpec 按 name 字节排序。引用 key 明确由每个 operator schema 标识，不把普通字符串误识为边。定义闭包中的保存查询按 caller path 独立命名空间化。此算法与作者 ID、拓扑数组排列、物理执行和 index 顺序无关；不同共享结构可以不同 canonicalGraph，不声称公用子式等价。

表达式总性区分 `total` 与 `may_error`；静态 checker 按 Value/CEL 表推导。普通 filter 需要 bool，可为 may_error；related 的 predicate 必须 total bool。没有 error-as-none、null-as-false、跳过坏行或局部成功。错误发生在逻辑输入上的全部语义可达求值，而不是物理只取第一页。

基础失败顺序：strict decode→当前请求/定义可见性→版本/feature/Registry closure→type/graph/context→完整 authorized execution→完整 terminal type/ordering/size→publish。外层当前授权优先于保存的错误。执行内每个失败候选的内部排序键为 `(canonicalOrdinal, LogicalOccurrenceKey, expression AST path, fixed code rank)`。只报告规范最小者；互不依赖分支均须达到判定，不以先完成者决定错误。上游失败阻止其依赖节点，其余可判定分支仍按预算求值。预算/取消/环境不可用是独立终止结果，不伪造某个 CEL error；一旦完整逻辑判定无法在预算内完成，返回资源/取消类别，不承诺最小 CEL 错误。

公共 diagnostic exact `{code,operator? ,expressionPath?}`；operator 仅 canonicalOrdinal，expressionPath 为静态 AST index 数组。不含内部 occurrence key、entity、值、路径、隐藏候选数量或执行进度。固定 code rank 见 Execution。诊断只在产生它的输入可观察时交付；无资格时 `not_visible`。

优化必须保持类型、bag 次数、orderedness、内部 occurrence、可达错误、正负依赖、授权和预算合同。`may_error` 不跨 filter/take/security barrier；不能用 short-circuit 跳过需要报错的分支。related total predicate 可在已授权输入上短路存在性，但完整范围依赖仍须记录。batch 定义语义；并行/缓存/incremental 只能通过等价证明使用，否则执行 batch 或 reset。

## 6. 产品取舍与拒绝方案

选择 relation DAG+纯 CEL，而非 SQL/DQL/JS、可执行 View、每业务模块专用查询、持久结果记录库。DAG 让共享 scalar 和多层聚合明确；代价是作者 JSON 不适合手写，D8 可以提供结构编辑器，但输出只有同一 QuerySpec。Facet/Field 扩展不产生新语法。

v1 不支持任意 join、union、recursive QueryRef、用户函数、任意代码、外部 fetch、任意 JSON path、持久 Record 或 View 写入。非相关 scalar、one-hop relation traversal 和 related(any/none) 覆盖既定组合；需要完整 join/window/quantile 的用例在场景表明确延期，不能在 renderer 偷补。

本版明确承担完整结果先验证的内存/磁盘与首次延迟成本。任何预算不足都完整失败；用户可显式修改 Query 的 semantic take/范围后新执行，但不能自动截断原结果。只有Field权不足以获新窄编辑路径；还须明确授权源Envelope元数据和Workspace提交序号。窄合同逐项公开这些观察成本，并给出真实Registry/D2/D4成功构造；它不授予body或其它Field读取。

## 7. 接受边界

必须逐项覆盖原始 Mandatory、非回退矩阵、扩展不改语法的新增约束及 D6 保留限制；必须有独立 Chat GPT-6 Pro 对完整组合明确 accept/pass、零未决 P0/P1、术语通过和唯一总控验收，才可激活。任何本地模型仅证明其列出的子代数，不证明完整 D2/D3/D4 解码、真实权限、OS 持久化、renderer 或六个 conformance caller 已实现。
