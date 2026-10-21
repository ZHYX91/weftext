---
source_language: zh-CN
translation_status: source
---

[English](FEATURES.md)

# Features：链接、查询、导航、批注与输出

状态：**design-refinement-not-implemented**；固定旧基线的独立接受保持为历史事实，当前修订及证据限制见 [REVIEW](REVIEW.zh-CN.md) §8。

## 1. 少数共用对象与分工

作者事实仍是一份 `.typ` 源和独立 JSON 批注；链接索引、Query 结果、缩略图、反向关系、呈现偏好和导出工件是投影。SOURCE 定义 native 值与集中 schema；EDITOR 定义 Draft、源映射及可安全编辑构造；CORE 定义身份、读资格、逻辑事务与恢复。Features 不建立另一个字段数据库、写入口或权限判定器。

采用共用 `ReadBasis`、`RevisionId`、`Plan`、`OperationJournal`、`Receipt` 和 `CommitManifest`。RevisionId 是 Core 预分配的私有候选名，提交后才有效；字符串或相同字节不证明权限。ReadBasis 绑定真实读取所用 observer incarnation、文件 generation、schema、授权及实际依赖；新一次相同字节读取不复活旧写资格。`SourceSnapshot` 表示已获权的准确源快照，`FieldProjection` 表示只获准披露字段的投影，二者不是互相升级的对象。字段写入可以由 Core 私下定位源并作窄修改，不能为构造完整补丁向字段权限的调用方交付整篇源。

共用调用形状如下；本节消费 [CORE](CORE.zh-CN.md) 的逻辑接口，不是新 wire 版本族。值与schema由[SOURCE](SOURCE.zh-CN.md)、Draft与原生回写由[EDITOR](EDITOR.zh-CN.md)拥有；证据范围由[REVIEW](REVIEW.zh-CN.md)说明：

```text
readEntity(ref, current|revision, purpose) / readScope(scope, purpose, completeness)
  -> SourceSnapshot | FieldProjection | unavailable
compile(sourceSnapshot, environment, target, budget) -> CompileSnapshot
evaluateQuery(definitionSelection, parameters, scope, budget, context) -> QueryResult
readResultPage(handle, cursor) -> Page
prepare(opId, intent, draftBase, budget) -> Plan{preview, changes, losses, dependencies}
commit(opId, planHandle) -> Receipt | conflict | rejected | unknown
operationStatus(opId) -> unknown | in-progress | pending_reconciliation
                         | committed | conflict | failed
createJob / jobStatus / stopJob / inspect / confirm / download
```

prepare 固定真实前像和全部变动，commit 重验当前资格并作一次逻辑事务。一个事务可以改源、资源及批注，但只覆盖一个有相应保证的提交 authority；跨独立服务器不伪称原子。崩溃与文件安装由 Core 日志恢复，投影只看到已决议的逻辑状态。unknown 保留原 opId、Draft 与 plan，通过 operationStatus 查询原结果，不另起创建或外部请求。无关文档不因某一操作等候用户而长期全库锁定。

## 2. 原生链接与四个定位域

正文使用标准 Typst `link`、label 与 ref。建议保留清晰的应用 URI 名称，语法不要求复用旧 decoder：

```typst
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#plan")[项目计划]
#link("../资料/白云山/白云山.typ#route")[白云山]
#link("weftext-workspace://58c47256-bfce-4d29-a037-b83a3a644fb0/资料/白云山/白云山.typ#route")[白云山]
#link("file:///D:/notes/report.pdf")[报告]
```

上例的中文 URI 是阅读形式；生成器交付的 URI 规范字节使用 UTF-8 百分号编码。文内 `#link(<plan>)[View]` 和 `@plan` 保留官方语义，后者仍受可引用对象及编号条件约束，不冒跨笔记身份。

| 形式 | 基准与保证 |
| --- | --- |
| ID（新建默认） | 完整库 ID + 笔记 ID，改名移动仍同对象，删除不转向同名新对象。 |
| 源相对路径 | 基准是该链接真实作者源文件所在目录；include 中的链接以 include 文件为基准。 |
| 库内路径 | 指明库及根相对路径，源移动不变、目标移动可失效。 |
| 本机绝对路径 | 已验证平台 file URI，仅有当前本机保证；Server 不据它开放任意磁盘。 |

“最短有效相对路径”是源相对域里的计算操作，不是第五种协议或按标题查找。以源目录 S 和目标 T 的库存分量求最长公共前缀，输出 `../` × 未共享源目录层数，再连接剩余目标分量，保留 fragment；目标与源同一文件时用该文件准确 basename。对 `.`、..、符号链接或大小写规则存在多种解释的库存，只交付可由当前 Core 解析器往返到准确目标的规范形式。不能用“末尾两层当前唯一”代替路径运算。异卷、未连接库、路径不可披露或边界越界时提供可用的 ID 等形式与准确原因。

URI 分量按一次 UTF-8 编解码：斜杠是分隔符，`+` 是字面加号，`%252F` 解一次为字面 `%2F`；不双解码、不 Unicode 正规化、不按标题折叠大小写。拒绝畸形转义、控制字符、编码分隔符、越库 `..` 和未知 authority。file URI 使用平台库生成并回读；不手拼 UNC/盘符。字符串值再由官方 Typst string 构造器编码，显示 content AST 原样保存。损坏的应用地址仍保留合法源和修复入口。

## 3. 转换、复制与链接索引

对一个当前 occurrence，快捷操作有“转换形式”“复制目标地址”“复制完整链接”“最短有效相对路径”。原样复制源文字不依赖目标存在；解析后复制需要获准披露目标地址，但不需要源写权。写入转换要求源可写，目标只需相关读取/定位权限。

转换算法：取得当前 Draft serial 和 ReadBasis → 由官方 AST/EDITOR map 找到唯一作者 occurrence → 当前解析准确目标与 fragment → 生成新形式 → 用官方 parser 及相同 Core resolver 回读 → 比较目标身份、fragment、显示 content/原生参数及所有未选字节 → 一个 Draft 操作/Undo → prepare/commit。fragment 必须在准确目标语义中唯一存在，不能因为 label 不可读或重复而默默去掉；本机/路径不可披露时不退成包含隐私的位置文本。改名、替换、撤权、环境变化或后续 Draft 编辑导致本次资格失效，保留稿并重新准备。include/generated/shared-variable 链接若无唯一可写参数则给 Source 入口，不从相同文本反算地址。

链接枚举同时保留两种来源，避免重复编译出现项被合并：

```text
AuthoredLink = {sourceOwner, ReadBasis, astPath, byteRange, destinationSlot?}
RenderedLink = {compileIdentity, target, projectionGeneration, occurrenceOrdinal,
                fragments[], destination, origin: unique_source | generated | multiple}
LinkResolution = resolved(target, fragment?) | missing | fragment_missing
                 | ambiguous | unavailable
```

AuthoredLink由官方AST按源序枚举，与展示target无关。默认流式阅读的RenderedLink必须来自同一次Html compile的final tree，每个逻辑link一项，碎片共一项；同一include运行两次即两项，即使span和文字相同。打印/PDF/显式Paged输出另用该次Paged compile的最终消费数据，不能以相同source RevisionId跨Html/Paged拼它们。关联必须由各自同compile origin producer证明；仅有Introspector/AST/span/string不能证明显示出现项。生成项可导航/导出，无唯一源不能写回或冒稳定作者边。Html/Paged真实producer仍待EDITOR独立验收，不以旧realization trace推断宣称成功。

出链显示真实源与编译链接；入链提供 ID 指向和“当前路径解析”两个明确分组，后者同样可导航。typed 属性引用另列“属性引用”，不用正文假链接表达。路径索引是 current 范围内派生，源或目标移动、对象替换、权限/目录变化即失效并重算。潜在提及独立展示，不形成真链或引用完整性边。partial 不宣称全库没有入链；只有当前获权完整范围才能给完整零结果，不泄漏隐藏标题、路径或数量。没有自动孤立笔记删除，也没有默认全库改写路径链接。

### 3.1 编译target的统一消费边界

`CompileSnapshot`必须明确Html或Paged target、environment、准确source/cut/World依赖、final tree/实际输出以及该次origin map。默认flow正文/表格/文本copy/visibleExcerpt/潜在链接扫描只取同次Html final tree；用户显式print view选区/摘录取其同次Paged最终输出，两者都不能用另一次compile的map回写。原生 `target()` 可让正文、数字、heading或link不同，source版本相同不能消除此差异。

官方Introspector的native heading产品单独存在：Query可读取该target同次compile中的全部native headings（含生成/深heading）；每行带 `displayAssociation: displayed(current final-tree occurrence)|not_displayed|unavailable`。native heading没有显示关联仍可只读查询，但不得被当作Html已显示正文或writer；Html final tree里的显示heading也不能靠字符串等同猜其native项。标准的“正文搜索”明确读displayed Html body，不偷偷换成Paged或把未显示introspection内容混入。

Html frame内官方局部Paged renderer只是在该Html compile内的局部边界，frame的origin/输出仍属于这一compile，不是另开全篇Paged编译再将其任意内容冒Html tree。无合格映射的frame只读/明确能力状态，不能从其坐标造rootwriter。

准确源copy/fork的身份/typed链接改写仍基于官方authored AST/schema，独立于展示target；若预览/复制显示文字或导出图像则声明消费target并锁同次结果。Html style/桥的选型与设计合同已接受（REVIEW §7），产品producer与renderer验收仍未完成；消费者应报准确可用性，不把“同一快照”代替producer证据。

### 3.2 显示文字、稳定片段与复制目的

普通链接的显示 content 是作者内容，与对象当前标题分开；插入时可取获权标题作为文字快照，目标改名／移动不改写它。显式更新标题须满足 §3 的唯一作者槽及当前依据，并保留身份、fragment 和其它参数；不能安全改写的表达式保源。链接文字与 SOURCE §1.2 引用键已满足首轮别名需要，不增加全局别名注册表或自定义正文语法。

稳定 URI 的对象部分与可选 fragment 分别解析，NodeId 校验不得吞入 `#label`。先按 Core 稳定身份及独立权限找到对象，再在该目标当前编译／输出中解析准确唯一标签并定位；不得按标题、相似文字或旧坐标猜测。URI fragment 的编码／解码由同一解析器往返验证，不能误把编码后的分隔符当路径或重复解码。缺失／歧义片段不等于成功定位；获权时可另给“仅打开笔记”明确选择。未获权目标沿用一致不可用响应，不泄漏其标签存在性。导航重验当前依据并保护源 Draft；标签是目标内部定位，不提升为跨改名的全局身份。当前 hosted 解析器将带片段后缀作为 UUID 校验的缺口按 v0.6 修复，见 SERVER 与 CR14。

复制选项区分应用稳定 URI、部署 Web 深链、完整原生 `#link(...)`；原样复制仍按 §3 保源，生成／转换形式则校验准确身份、fragment 和合法原生字符串转义。Web 地址只来自可信部署配置及已实现路由，不猜公网域名、文件路径或当前页面地址，不含会话凭据。登录返回、重新打开、无权／撤权、目标移动及片段定位必须实测；尚无该路由的部署禁用并解释，应用 URI 可解析不代表 HTTP 深链已实现。输出按 §10.1.1 分项说明可用目标、应用依赖、Word 域、固定文字和未转换结果；不将转换损失隐藏在一个总成功状态中。新增复制／编辑体验在 v0.6.1 验证，v0.7 补平台唤起及交互。

## 4. 潜在链接的完整匹配与重叠算法

候选词来自当前可披露笔记标题及 schema 明确标记的 alias/name 搜索字段，排除当前笔记。人员的旧名等可由 name role 派生候选，不双写另一份别名事实。候选字典以 `(target identity, exact term)` 去重；相同标题指向多对象全保留。匹配默认精确 Unicode scalar 字面匹配，不自动拼音、fuzzy、casefold 或正规化。可扩展匹配方式必须显式选择且标明原因。

读取能力与写入能力分开：可准确提取的普通可见逻辑文字可以发现候选，只有唯一源范围且当前可写映射才提供快捷转换；合法 generated/include 内容可以只读发现并标只读来源。代码、公式、URL、真实链接、属性声明和批注正文不自动扫描为普通文字。逻辑片段之间不拼词，软视觉换行仍是同片段，source 换行是否属于同逻辑流由 EDITOR 明确映射，不靠屏幕文字猜测。

具体算法（n 为扫描文本长度，z 为命中数）：

1. 从一次获权词典快照建立 Aho–Corasick trie，终止节点保留所有完整目标，遍历每个逻辑片段，返回全部命中，复杂度 `O(n + z)`；预算不足按片段记录 partial，不保留最长匹配后删其他。
2. 命中为 `(flow, startScalar, endScalar, targets[])`；按完全相同范围分组，目标按完整身份稳定排序，不按标题合并。范围记录实际匹配词和来源。包含和交叉不是去重条件。
3. 不枚举两两重叠边。按 `(start asc,end desc)` 排序，以当前连通分量最大 end 扫描：下一 start 小于它则加入，否则开始新分量。`a.start < b.end && b.start < a.end` 才重叠；端点相接可共层。所有命中保留在语义列表中。
4. 候选视觉方案为最多两层上下分离下划线，按 `(start asc,end desc)` 贪心分配最小空闲层；无层者进入该重叠连通分量的“更多匹配”入口。两层、默认关闭和每页 20 项均是可替换设计选择，用户需求是完整可达。键盘/触屏/辅助技术列表直接列全部范围，与视觉层无关，稳定 cursor 分页不得静默截断。
5. hover/click/键盘/长按打开同一候选列表；Escape 回原选区，后台扫描不抢 IME/正文焦点。明确选择目标并按“转换”才写入，不默认第一项。
6. 转换重新获权、重验词典项和 current Draft，scalar→准确源 byte 映射的两端须为完整 grapheme 边界，不拆 combining/ZWJ/CRLF。半字素只读发现不可转换；不扩大选区。使用 §3 生成原生 ID link，比较目标、显示文字及外部字节，一个 Undo。

```text
文本：白云山南路
[0,3) 白云山 -> A、B（同范围两目标）
[2,3) 山     -> C（包含）
[2,5) 山南路 -> D（交叉）
视觉贪心：白云山 lane0，山南路 lane1，山 overflow。
全部列表：三个范围、四个目标；每个均可独立选择。
```

RTL 用逻辑范围及当前排版映射绘线，不按 x 坐标排序语义。用户可关提示而不改源，允许可见区优先及取消。目标撤权后清理其候选及已显示上下文，完整性只说明当前获权扫描程度，不暴露隐藏词数量。全库未扫描完不阻止一个已独立验证的局部转换。

区间排序 `O(z log z)`、分组/两条显示 lane 分配与存储 `O(z)`；每条 lane 只记 end，保全部语义命中。密集重叠不生成平方级图边；分页不删除候选。

## 5. 一个 Query 语义与清楚的状态

Weftext Query 查询获权的库数据，Typst 官方 `query` 仍只做当前编译文档 introspection。普通搜索、导航条件、可视化筛选、保存 Query 和 API 均产生同一个 typed Query AST，交给同一执行器。结构用原生 dictionary/array 储存，表达式选择 CEL 语法的静态受限、确定、无副作用 profile，使用标准 `exists/all/filter/map` comprehension；不继承旧 DAG wire/八必填成员/贡献版本树。CEL parser 只处理表达式字符串，官方 Typst parser 处理原生承载，两者职责分开；无 eval/import/任意回调/网络。高级表达式须静态型检查并保留源，builder 不理解时提供只读高级条件/Source，不替作者删条件。

基础节点为 `scan(scope,lifecycle)`、`read(fields/products)`、`filter(expr)`、`project(columns)`、`unnest(array,binding)`、`sort(keys)`、`take(limit)`、`union_all`、`distinct(keys)`、`group(keys,aggregates)`、`related(relation)` 与保存 Query 调用。默认displayed body/title/table来自该次Html final tree，native heading产品及显示关联按§3.1单列；生成/任意深heading只读存在不等于已显示或可写。显式Paged产品读另一个具名target，同run中每个product带compileIdentity/target且不跨目标拼值或occurrence。基础全文搜索生成显式 `textMatch` 节点，UI 显示是否查正文、目录/子目录/库范围及所有组合条件；本轮不另造查询 DSL。

Query 用 SOURCE 的唯一 Type/Value：text、bool、i64 int、exact decimal、显式 float、civil date/time/local datetime、带 offset instant、ref、array、record 和 schema判别union。money+currency、quantity+unit与精度日期都是该shape系统定义的业务record，不增加未注册scalar codec。数组保序及重复；record 成员按 schema 解释，未知原生成员仍保留为只读值，不猜类型。无隐式 float↔decimal、date↔instant、币种↔币种或单位换算。显式换算需指定准确 provider 与版本，并纳入依赖。电话始终 text。schema 冲突/缺失不能从第一行数据推断 type。

```text
Cell<T> = value(T) | absent | null | invalid(diagnostics) | unavailable(reason)
```

QueryResult只有§5.2那一份定义；本节不另定义handle/schema的平行版本。total/aggregate属于该冻结terminal，不在分页transport中重算。

absent 是作者没提供；null 是 schema 允许的显式 none；invalid 是已读作者值不合定义；unavailable 是 schema、产品或所需依赖不可得。公开 unavailable 原因不能泄漏隐藏对象存在；不可披露对象直接不入公开范围。空字符串、空数组、zero 不等于上述状态。谓词显式测试 `isAbsent/isNull/isInvalid/isUnavailable`，取非 value 的成员报类型/值错误；未知/不可得不能被当 false 偷偷筛掉。若因部分字段不可得不能判断成员资格，run 标 partial 并给已确定行及未决原因；整体语义不合法则返回错误，绝不包装成 complete empty。FieldProjection只消费SOURCE可验证的独立profile中获准声明/schema，不编译隐藏正文、不执行隐藏条件、不披露隐藏诊断；需要隐藏动态计算才能确定字段时在调用前给统一不可用，不执行后再以秘密条件决定公开状态。具体部署能力由SOURCE/Core profile决定，不由控件或reader自证。

coverage 与 freshness 是正交的：完整旧投影也可 stale；partial current 可浏览已知行。截断页不是 partial 的代名词，page 带续 cursor；`take(200)` 是作者显式查询限制而非全库统计。complete + current + 无行才显示该范围完整零结果；partial 的 count 是已知下界，aggregate 若全输入未决标 partial/estimated，不冒精确全量。禁止取当前页画全库比例。权限变化使 run/cursor失效，清理缓存的受保护 cells/snippets/counts，再获权重跑；其他依赖变化可显示已标 stale 的仍可读结果，但它们不能写。

结果行的 lineage 仅是来源说明，不是写权。直接 field 行可提出精确 intent，prepare 重新获当前 ReadBasis 和窄写资格；group/distinct/computed 值不继承代表行的 writer。跨已连接库可以 union 获权结果；定义与数据不跨授权域隐式读取；动作不因此获得跨库原子保证。回收站是显式 lifecycle 范围，获权只读，保 tombstone/不可用诊断；结果不授 restore/purge，任何经 trash 调用链输出的 view 动作禁写。

本地查询面板只合并同一库、查询／定义选择、typed 参数及同一环境中的在途读取，并最多保留一个最新等待意图。schema 和目录的异步预读前固定输入；新输入或环境失效后不安装旧结果。实际保存、配置／结构写入、导入、恢复及同步确认成功即使后续打开或交付失败，也使旧环境失效；未保存 Draft 和不影响查询的密度、缩略图等个人呈现不产生这种持久变化。这里的环境序号只是页面内的退休标记，不是 Core 修订、权限、结果缓存或持久 Job。完成后的明确刷新重新取 Core cut，保存定义的 current／pinned 规则不变；服务器中已开始的隔离编译仍受原预算约束，不把丢弃 UI 结果称为服务端取消。准确冻结结果也不因此被提升为持续 current。

读取状态与结果更新不得卸载、重挂 Query 输入控件；导航（Nav）与周期（Period）入口须在同一 root 身份内稳定挂载，不因 loading、结果状态或 schema 反序列化产生的新对象身份重建。尚未应用的筛选、聚合与 raw JSON 草稿、DOM 选区和 composition 属于当前 root／owner 的控件临时状态，须在这类更新中保留，不写入 Query 作者计划或形成新权威。真实 schema 变化须按原 `FieldId` 重新验证；失效的 raw 输入保留并明确标为 unavailable，不按同名字段替换，保留输入也不授予写权。root／owner 切换后，旧读取及其完成响应不得改动新控件、回填草稿或重置选区／composition。

### 5.1 Core LogicalCut：结果不能由逐对象 current 读取拼成

反例：同一Core事务把 `A.amount=10,B.amount=0` 变成 `A.amount=0,B.amount=10`。先读A旧值再读B新值会算出sum20，这在任何已决状态都不存在；给两项各自ReadBasis或最后核“都曾读过”不能修复。因此Query不是循环调用 `readEntity(current)`。

`LogicalCut` 由CORE产生，包含同一authority的已决journal generation、完整causal heads、entity→准确revision/blob/lifecycle映射、冻结结构事实、schema/rules/settings、按声明消费的 Tags 版本／原登记资格以及授权audience。Core在一次短commit/read barrier中取得这一已决视图，并与cutHandle同一受保护注册步骤保留必要历史blob/视图。只在注册成功后释放barrier；不能捕获半安装事务。它是本次历史读取快照，不建立第二份可独立更新的current正文/目录权威。

执行算法：

1. 对当前调用方获权，解析本次scope和定义选择，申请CORE cut；定义current版本、当前schema及实例scope在同cut冻结。pinned定义的历史版本单独固定，数据仍取本次cut。整个嵌套saved-query调用闭包共用适用authority的同cut；callee current定义也从该cut读取，不能递归各取新cut。静态检测定义调用环，冻结参数/默认值及now/timeZone等明确context一次；所需跨authority调用扩为显式federated向量，并受调用/展开预算。
2. `scan` 只枚举cut的准确库存/结构/lifecycle及其完整正负范围。使用索引必须证明其每条结果属于此cut；缺构建范围时用cut blob重算或标partial，不拿旧索引负范围证明不存在。
3. `readEntity(ref,at=cut,purpose)` 只取保留的cut blob，不重新读最新工作文件；字段/body/官方编译依赖均在同cut解析。World的受管include/resource/style/config读取亦取该cut；原生文内函数结果不能来自之后的latest附件。
4. 对外部包、固定provider数据和其他非工作区inputs锁准确不可变版本；负依赖（未找到文件/成员）由cut库存或该固定输入集合证明，不能在求值中逐个查ambient文件系统。
5. 计算到终点，封存结果/顺序及完整性说明，然后调用CORE `publishCurrentResult`，而非后台worker直接宣布current。publication barrier内同一受保护注册事务原子校验实际consumed正负依赖、完整scope、schema/settings、当前auth、observer连续性，并注册原run/retention和从捕获cut连续接续的change subscription。只有相关依赖仍与cut一致才标 `current_at(publicationCut)`；无关全库generation变化不必拒绝。验证与注册之间无空窗，不能先validate后另一次写handle。已经变更但未撤权可交封存complete/stale，或有限次新cut/newrun重算；不是“评估结束”本身授current。授权变化不发布原结果；watcher gap/raw external未知不得current。after-publication相关变化经同subscription标stale，授权变化整体invalidate，不能修改冻结行值。任意basis无法再证明不能把same bytes重装为新cut。

上述例只可能得到旧10+0或新0+10，sum均10。读cut不会长期持有全库write lock；历史blob保留支撑执行，结构新事务仍可提交。Core未能保留所需cut视图时返回cut_unavailable或明确partial，绝不以current逐项fallback补齐后声称coherent complete。

current的线性化点是publication事务的校验/注册决议，准确含义是“在本模式可证明的Core admitted事实下，该结果相关事实此刻仍适用”。它不宣称任意raw外部工具全库瞬时静止；external_pending/逐文件观察边界另显。出版过程中预算/取消/IO不足不给complete；如果已封存合法partial，可用同publication门发布partial/current或partial/stale，不能把混合txn读取的结果包装成partial补救。publication冲突只保原封存结果并报stale或重算，绝不修改其中一行后继续旧run。

context闭合为明确now(冻结Instant或null)、timeZone(明确zone及规则版本或null)。选择workspace默认zone时Core从本cut settings解析并冻结真实zone/rules；选择“当前时间”须显式请求Core/Server clock provider一次生成now，整个callee闭包共享该值，不反复取host clock。client显式now可作为查询语义，但不是授权expiry/批准/audit时间证明。输入context及实际生成选择纳入结果；不静默用设备时区解释civil date。

多个独立authority的union携 `logicalCuts:[...]`，一致性标为 `federated`（每authority内共同cut，authority间无共同瞬时承诺）。全局聚合可以显式表达这组固定cuts的结果，UI/导出说明其非单一全局时点；要求single_authority保证的query或跨域写入条件拒绝，不由“同一runId”伪造跨服务器原子快照。

外部编辑边界：未admitted的普通文件观察不插入business LogicalCut。Core记录external_pending/不完整/冲突并使相关结果stale；文件仍是实际current源，保留旧cut只为显式历史结果，不声称它是当前工作文件。`observed_only` 浏览可以显示各对象准确external observations及coverage，UI明确“逐文件观察”；它没有全库瞬时cut，不给强aggregate、negative-range/absence、全集批量或查询后条件资格。若用户需要强查询，先通过admitExternal/merge把所需观察纳入正常决议；不得悄悄丢外部稿或按最后到达覆盖。响应未决恢复同cut/run或明确过期，不生成名字相同的新cut冒原请求。

### 5.2 冻结结果与分页，不是回读 latest 的cursor

`FrozenResult` 是一次求值的不可变terminal schema+数据+行顺序/occurrence keys+coverage+cut/定义/参数/provider依赖。可在受控内存或只读spool/chunks保存；这些是可清理投影，不是业务事实。只有完整终点评估完成并封存索引/顺序后才发布complete handle；partial也先封存一个明确的已确定subset及缺口，不把后续完成行暗加同handle。

```text
QueryResult = {
  audienceHandle, runId, definitionSelection, schemaBinding, logicalCuts,
  inputDependencies, invocationContext, terminalSchema, frozenResultRef, publicationBasis,
  coverage: complete | partial(reasons), consistency: single_authority | federated,
  freshness: current_at(publicationCut) | stale(reasons), expiresAt
}
Page = {runId, frozenResultIdentity, startOrdinal, rows, nextCursor?}
Cursor = opaque Core record bound to
         {runId,logicalCuts,frozenResultIdentity,orderDigest,nextOrdinal,audience,expiry}
```

以上是准确语义对象，不冻结transport wire。Core按自己的受保护记录签发handle/cursor，不接受caller自报ordinal/run作为权限。页算法：先核current audience及整个实际消费的授权依赖，再核原run/cut/结果保留及cursor，最后读取冻结序列的 `[nextOrdinal,nextOrdinal+pageSize)`。再次请求同cursor得到相同行、序号和nextCursor；页大小不能改变语义membership、排序或统计。不能为分页重新运行query/read latest；相同ID仍指原结果，而非一个新结果的页。

显式sort用同SOURCE Type/Value比较器与声明missing/null顺序，tie用完整run内source/occurrence key稳定破同分；无sort保producer顺序，scan按cut库存完整实体key，unnest保parent order+item ordinal，union_all保声明输入序，group/distinct以规范完整typed equality key给新行，不借代表输入writer。相等hash碰撞比较完整值。全局sort/aggregate必须遍历完整逻辑输入并计预算；spool失败/预算失败不能截出第一页假complete。partial aggregate的已知subtotal不是完整值，signed金额subtotal也不能冒下界；不给未定义的“估计”算法以estimated标签。图表与导出引用同FrozenResult而非浏览页。列表晚到摘要/缩略图若附在原result row，必须从该row同cut source和独立获权资源生成；当前打开笔记可以重新read current，但展示新revision与原列表stale状态，不能把新缩略图伪成旧row同basis。

相关授权变化整体invalidate原handle/cursor并停止交付，不能删去隐藏行后继续旧ordinal，泄漏原行数/空洞。重跑是新run/cut/result，UI明确替换，不能拼接旧第一页和新第二页。异步索引补全、source修改或subscription更新也只发布replacement handle；原partial仍保持原subset，不把coverage改成complete而不重封存结果。expired/cut unavailable返回状态，用户明确refresh创建新run，不透明换内容。引用结果作action只提供目标说明；prepare重新获当前basis/真实依赖，冻结目标集，commit不再跑query扩大它。

### 5.3 保存定义的持久schema解析基线

单有 `phones`、`person` 等短名和CEL源码无法持久绑定Field/Type ID。保存定义新增必填原生成员：

```typst
binding: (
  schema: "weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema",
  revision: "8a81e35f-ad48-4c12-af4f-3d2607102fa9",
  language: "weftext.query.cel.v1",
)
```

示例ref/revision只说明形状，实际positive fixture必须由Core完成对应workspace初始化并取得准确已提交schema revision。SchemaRef是集中唯一schema的roleRef，内部为 `{workspaceId,kind:"schema"}`，作者字符串准确 `weftext-config://<lowercase-WorkspaceId>/schema`；不分配另一随机schema身份，不因字符串出现就创建role。schema baseline包含aliases、FieldId→作者key/shape、Type定义、Relation定义及provider/函数签名的完整配置；不是把这些定义再抄入Query。Query只保存表达式作者文本/原生结构和一个准确baseline选择，AST、name→ID map、typed plan均可重建，不持久另一份独立AST事实。

生产：Query builder先在获权current schema下官方投影声明，再CEL parse/type-check。静态symbol table区分保留产品（self/title/body/headings等）、parameter、comprehension局部变量与字段；局部变量按lexical scope绑定，`p.number`从p的已绑定record shape解析，不按全库同名Field匹配。unqualified field key必须在baseline唯一且不冲突保留名；消歧入口为 `field("<permanent FieldId>")`，参数必须literal且存在于baseline，解析阶段即成为typed field-reference节点，不执行动态name lookup；普通UI可保短名，无需手写永久ID。禁止动态拼FieldId或动态函数名。types短名、related RelationId与所有provider函数一并从baseline解析。Core检查本次引用闭包与安装schema一致，保存binding同metadata入一个source Plan；schema在prepare/commit间变动则冲突重准备，不保存未安装caller schema字符串。

冷重开：获权读定义及准确baseline配置revision → 选准确CEL语言profile/parser → 重新解析全部结构/表达式 → 由baseline生成唯一语义ID引用及Type/Value签名 → 验全部symbols/参数/函数/关系 → 取得本次LogicalCut的current schema → 按永久ID join当前定义，并由SOURCE的兼容规则验证每个真实消费定义。绑定闭包涵盖where/select/order、type/field读、parameter type/default、view列/表达式、export typed输入、related RelationId及definition-call参数/常量；callee定义消费自己的baseline，并共享适用data cut，不能以caller schema重新解释callee。copy/template/import必须枚举这些真实typed槽及其SchemaRef/revision依赖。绝不拿当前alias/key重解旧源码。baseline不可得/撤权、语言profile不可用、ID消失、语义/shape/direction变为不兼容时definition_unavailable，保源与迁移入口，不换最新schema补救。

兼容增可选record member不使旧已绑定成员失效；field key改名、nested member含义变更、target方向/域、nullable/required/enum意义、provider函数语义变动不能只因为“同ID”就自动通过。SOURCE确定兼容性，若需要迁移，preview完整oldID/oldshape→newID/newshape/表达变化并由明确Plan更新binding。display alias重命名可以保持同ID，旧baseline仍认旧别名；新UI生成当前别名要做显式重bind/文本变更，但不能漂到新同名ID。

普通编辑保存保原baseline；用户明确“重新绑定schema”才预览真正身份变化并更新它。current定义引用每次选当前source revision，该revision内自带自己的binding；pinned定义永用其原baseline，但数据取新cut。inline未保存查询在本次入口冻结current schema/profile，提升成saved时把该准确选择写入声明。保存scope里的 `library:"current"`准确指definition owner的library，不跟调用方ambient库漂移；改变scope必须明确改定义或已声明typed scope参数。复制/export声明保引用baseline配置revision依赖，或显式同map映射/迁移；丢baseline而仍称Query可运行不允许。Core `readConfigProjection(schemaRef,revision,authorized names/IDs,purpose)`只给最小获权引用定义投影，受保护服务可保完整baseline；不把全schema条目/隐字段名作为错误或预览交付。

### 5.4 表达profile、schema与provider的闭合职责

本候选选择单一 `weftext.query.cel.v1`：CEL语法/标准bool/string/list/map/comprehension、SOURCE Type/Value及具名纯函数签名，静态拒未知name、dynamic invocation、任意eval/IO。int严格i64 checked，不以CEL自动float提升金额；exact decimal/date/instant/ref/Cell通过明确typed adapter，只有本profile列明的同型比较/函数能使用。literal常量按SOURCE合法constructor/准确字符串规则生产，不能expected type偷偷把float变decimal。

CEL AST只来自作者表达式parser；官方Typst AST只解metadata与native值，CLI metadata JSON不作精确值权威。builder、text search和导航产生同一normalized Query plan。自定义纯函数、relation scan和recurrence expansion来自SOURCE闭合安装descriptor，不能用“已注册”散文去解释unknown codec/constraint。执行前所有实际函数/provider选择纳入baseline、LogicalCut依赖及预算；具体业务接口见§8.2–8.3。

Cell与CEL边界具体如下：字段root绑定携SOURCE T以及其Cell状态，state函数 `isValue/isAbsent/isNull/isInvalid/isUnavailable` 是此language profile的具名内建，直接检该Cell不先unwrap。普通值运算/record member/list comprehension只在value态取得T；在非value态使用产生明确 `cell_absent/null/invalid/unavailable` EvaluationIssue，绝不让状态tag作为CEL bool/string或自动false。可选record member保presence，标准 `has(p.label)` 检查对应已授权record键；`has`不读取无权字段。`&&/||` 使用标准 CEL 的可交换布尔语义，不承诺左到右求值；确需跳过分支的 guard 使用 `has(p.label) ? p.label == "work" : false` 等条件表达式。guard 不是授权门，实际输入权限/读取闭包在求值前核定。provider `bool-cell`在value态适配CEL bool，compareTemporal的order-cell在value态适配i64(-1/0/1)，duration-cell按固定record签名适配，其它态转EvaluationIssue，不把整个Cell结构放进 `&&`。例如containsDate未记期间→unavailable，不意味着该员工永久有效。

静态型检查记录declared T/成员presence/可能的read effect，禁止跨T coercion及非法member；运行时依准确Cell保持错误差别。最终 filter 表达式产 value(bool) 才确定成员；未被布尔结果吸收的 EvaluationIssue 导致未决行和 partial。中间曾求得 issue 不自动使已可确定的成员关系变 partial；诊断和字段 Cell 质量另记。schema/表达式静态非法是整个 query error。明确state test可选择“只看可得值”或诊断行，这是可见作者条件而非后台丢数据；数据库存未决仍不得complete。简单UI只为已知absent/可选member生成明确守卫，不默认把isValue=false的invalid/unavailable排除；用户明确“仅可得值”才生成并展示该状态条件。高级源码不替作者补false。select直接字段仍导出其原Cell状态，不先unwrap再丢状态；aggregate输入含非value必须有作者明确筛除/诊断策略，否则partial或error，不能补0。

#### CEL 组合与完整性

保持 [CEL 标准逻辑运算与宏](https://github.com/cel-expr/cel-spec/blob/master/doc/langdef.md#logical-operators)，不增加另一套求值器。以下 `issue` 是已获权业务值运算产生的可吸收 EvaluationIssue；权限不足、无完整读取范围、撤权、预算耗尽、取消与宿主失败不是该值，必须由宿主维持相应失败/partial，不能被 `true` 掩盖。

| 表达式或宏 | 成员资格结果 |
| --- | --- |
| `false && issue`、`issue && false` | false；该条件可确定排除 |
| `true || issue`、`issue || true` | true；该条件可确定包含 |
| `true && issue`、`false || issue` | issue；未决/partial |
| `exists` 至少一项 true，其它项 issue | true；不宣称其它项可用 |
| `all` 至少一项 false，其它项 issue | false；不宣称其它项可用 |
| `filter`/`map` 的必需元素产生 issue | 宏结果 issue；不静默删掉该元素 |
| 条件表达式未选择的分支 | 不消费该分支值；其静态类型/权限规则仍有效 |

SOURCE 时间函数仍须在调用内部完成共同时间基准检查，再计算自己的结果；CEL 吸收整个调用的 issue 不改变 provider 算法。最小真实 runtime 对照覆盖换序、exists/all、filter、条件 guard 和宿主预算中止；旧 Python 模型不作为此项运行证据。

### 5.5 正文内嵌 Query

正文内嵌只改变查询的呈现与编辑入口。定义继续使用 §§5.3、7 的原生承载、schema 基线和定义/实例规则，构建器与源码使用同一 Query AST、CEL 校验和执行器；结果容器消费 §§5.1–5.2 的同一封存 QueryResult，不另建编辑器查询引擎或可独立更新的缓存权威。[EDITOR §2.3](EDITOR.zh-CN.md#23-正文内嵌-query-的五档交互) 规定 V01 直接编辑查询源、V02 完整源始终可见且同时显示结果、V03 主动展开对应源、V04 构建器加明确独立源码入口、V05 只读且无定义编辑入口。

每个正文出现项的会话关联须记录它实际使用的定义选择、参数及结果依据，并关联当前 Draft serial 与准确源范围；这是现有来源和结果合同的消费，不另持久化定义。未保存的本篇定义按 §5.3 冻结当前 schema/profile，并明确显示草稿预览。调用保存 Query 时，修改本篇调用/参数与修改定义所属笔记分开；后者进入已获权 owner 会话，不从结果位置反写另一篇定义或 pinned 历史。构建器只对能证明完整往返的范围提出共享 Draft 修改；不能安全表达的原生查询保原字节、说明控件限制，V01–V04 提供明确源码入口，V05 不提供定义编辑。运行、展开/收起、切档和结果刷新均不隐式保存。

来源依据过期、当前定义/参数与旧结果不一致、数据 freshness=stale 是不同说明；可以并存，不能用“还有结果”掩盖当前定义未运行或错误。沿用既有权限、预算、错误、partial、stale 和撤权处理；完整空结果仅适用于其已说明且合格的定义/数据范围。结果行 lineage 不授源 writer，构建器可用也不授结果行写权。行上的勾选、改字段、拖移等既有操作仍按 §5、§7 独立 prepare 和重验目标，不能改查询定义来冒行写入；正文 V05 禁用这些写动作，保整体只读。

本地候选接通正文出现项与同一 Query 执行器：用当前准确 Draft 求值新预览，current 调用可使用同 owner 的准确草稿，pinned 调用仍读取真实保留版本，不借草稿冒充原 Revision。结果带原覆盖、错误与过期状态；迟到响应按来源与 serial 拒绝，刷新不保存定义。构建器只修改已证目标，复杂原生条件保持；其他出现项/源码字节保留。已有结果可经原冻结输出入口输出，不另查一次。DOCX/PDF 随文输出绑定准确 session/source hash/serial/occurrence 的已显示冻结结果，不另执行 Query，不写回作者源；缺失结果报告 `embedded_query_results_not_materialized`。DOCX 使用同一冻结 Html 编译的出现项证明，PDF 使用自己的原生 Paged 编译证明，不能借 Html 位置。分页结果只插入临时编译输入，冻结依赖不回落本机文件；保留 partial/stale/空范围状态及原输入版本。正文模板复用同一物化路径；未绑定正文报告不在输出范围。复杂行内位置、重复或生成来源、自定义绑定无法证明安全时拒绝猜测。原生分页预览仍不含 Query，界面引导到明确 PDF 输出入口。交付验收共用 [ROADMAP IQV01–07](../ROADMAP.zh-CN.md)，不另建通过记录。

保留两个最小技术反例：定义 Q1 改为 Q2 后，Q1 的响应迟到，不能回写新稿或标成 Q2 的当前结果；构建器不能往返某个原生条件时，不能在 V04 聚焦后自动露语法或删除该条件，进入 V05 后也不能沿用先前定义/行写命令。完整交互与状态矩阵按路线图验收，此处不另记已通过事实。

### 5.6 标签结构读取与后代筛选

2026-10-20 定版并接入 0.6.3-rc.1 的共用 Query／CEL profile，整版验收中。具名结构谓词 `tagInSubtree(candidate, root)` 的两个参数均为 typed `tag_ref`，返回含自身的子树成员判断。根只取字面 TagRef 或本次运行冻结一次的 typed 参数，首版不支持逐行动态根或跨工作区根触发额外读取。格式、工作区、根的原登记资格与存在性在准备阶段校验，即使没有候选行也不能把无效根报成完整空结果。对合格树中的同库候选使用结构判断；已证外库候选不在此树，同库未登记／缺依据候选产生 unavailable，而非 false。遵循 §5.4 的 EvaluationIssue、布尔组合和 partial 规则；例如在已可读取的 tags 上：

```text
tags.exists(t, has(t.ref) ? tagInSubtree(t.ref, root) : false)
```

这是 Query profile 的受信内建，不是从可变 schema/provider 配置安装作者代码或当前标签表。准确名称／签名及实现依既有 profile、定义 binding 与依赖规则绑定，禁止同名 provider 遮蔽或静默改变真实旧定义的绑定；若盘点发现冲突，显式迁移／重绑定，不保两套猜测分派。纯求值的完整输入是“本次冻结树、candidate、root”；树可作为隐含运行上下文，但不能从授权和依赖模型消失。

必须统一重构本地／hosted、保存定义／嵌套调用、内嵌 Query 及冻结输出共用的输入通路：先预读所选定义／callee 的准确版本，解析完整调用闭包、类型和根参数并声明一项标签结构读取需求；预读仅用于求需求，Core 在 capture barrier 内重验这些选择／版本仍一致，变化则重新解析并申请新 cut，不混入后来定义。随后在与笔记同一 cut 的 barrier 中取得、登记并保留准确 Tags blob、原身份资格及当前 audience；将不可变树交给现有执行上下文，纯谓词只读该输入。结果发布、分页、缓存和变更订阅消费同一证据。不得在旧快照后另查一次 latest 注册表、让 UI 展开 ID 冒统一查询、接受客户端自报版本作为资格，或另建标签执行器／通用 effect 框架。CORE 的现有 capture／witness、应用冻结输入和静态读取分析应一起调整，不在各消费者外挂独立树缓存。

只要已检查的表达式／callee 闭包含结构读取，就绑定整份 Tags 版本及其合格性，与本次是否真的调用函数或命中无关。零候选、空标签数组、全 literal、原空子树和否定条件都须覆盖；新增后代、分支移入／移出后，原结果按 §5.1 发布为 stale 或重跑，新页不能换树。整表 witness 已覆盖结构负依赖，不增加逐标签“尚不存在关系”协议，也不替代 source／scope 的完整正负范围。无法保留合格树、缺父／环／观察缺口或权限变化时沿既有 unavailable／partial／撤权处理，不以未知补 false、不宣称完整零结果。

纯 TagRef 精确相等沿原有 typed 值规则，不新增标签结构读取或 `tags.read` 前置；显示名称／路径是单独声明的注册表读取。current 普通运行与 pinned 定义重跑都使用本次 cut 的树；pinned 只固定定义。准确历史结果重放才消费对应历史树，缺历史不可补 latest；所有历史／缓存仍重验当前授权。缓存键保留定义、参数、源、范围、schema/profile、授权及真正使用的 Tags 证据，不能只用根 ID 或 Query key。

每份冻结森林生成 TagId 查找表、children 邻接表和迭代 DFS 区间 `[tin, tout)`；有效同库成员满足 `tin[root] <= tin[candidate] < tout[root]`。遍历及索引数据规模为 O(N)，区间比较为 O(1)，标识查找按所选容器计成本；不预计算所有节点的 N² 闭包。序号不写注册表或作者源；缓存只省计算，不替代资格／授权核验。接受 Tags 任意变化引起保守失效和全表重建，先不增加逐节点版本、语义摘要或子树订阅。按同一获权笔记范围 `count_distinct(self)`，多个直接标签命中父组也只计该笔记一次；子组计数不可直接相加冒父组总数。

## 6. 嵌套集合：同项绑定、保序与代表查询

数组值作为一个结构读取，数组每项在当前 ReadBasis 内用 `(owner,field,valuePath,ordinal)` 区分。ordinal 是当前定位，不是永久身份。`any(A,x=>P(x))` 在同一个 x 上算整谓词，`all` 空数组为 true，`any` 空数组为 false；缺 array 不自动变空数组，必须显式 `orEmpty`。filter/map 按输入顺序输出，重复完整项仍重复。unnest 按父行当前顺序、item ordinal 输出，保每项的同一 item binding；多数组展开只有显式选择 Cartesian/product 或按实际稳定键关联，不按索引猜配。

```text
phones = [
 {number:"13800138000",label:"personal",note:"晚间"},
 {number:"010-12345678",label:"work",note:"白天"},
 {number:"13800138000",label:"work",note:"值班"}
]
any(phones,p => p.number == "13800138000" && p.label == "work")
  -> true，只匹配第3项。
删除第3项后 -> false；不能将第1项号码与第2项label跨项拼成true。
filter(phones,p => p.number == "13800138000")
  -> 按作者顺序第1、第3项，各自label/note完整。
```

建议保存定义是直接应用 metadata 的原生结构，下例是设计 schema 示例，未声称已由实现执行；binding中的schema revision必须换为实际Core已提交版本。字段名由§5.3准确baseline解释，用户可以使用短名，内部不能按显示标签或current新同名字段替换。

```typst
#metadata((id: "f8597b15-6b7f-4a6a-9b52-c2732730fe41", types: ())) <weftext>
#metadata((
  kind: "weftext.query", key: "6b2b9e19-5599-48c3-b27b-6b006ec9c397",
  binding: (schema: "weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema",
    revision: "8a81e35f-ad48-4c12-af4f-3d2607102fa9", language: "weftext.query.cel.v1"),
  scope: (library: "current", lifecycle: "live"),
  types: ("person",),
  where: "isAbsent(phones) ? false : phones.exists(p, has(p.label) && p.label == 'work' && p.number.startsWith('138'))",
  select: (
    (name: "person", expr: "self"),
    (name: "contacts", expr: "phones.filter(p, has(p.label) && p.label == 'work')"),
  ),
  order: ((expr: "title", direction: "asc", missing: "last"),),
)) <weftext-query-work-phones>
```

本节前面的 `any(A,x=>P)` 是代数说明，实际保存文本用 CEL `A.exists(x,P)`；all/filter/map 分别用 `A.all(x,P)`、`A.filter(x,P)`、`A.map(x,E)`。只有一个表达式parser/evaluator；文本字符串是作者表示，AST是其可重建投影，不同时保存独立可能漂移的AST事实。自定义 containsDate 等函数有唯一静态签名，由schema/provider注册；未知函数整体拒绝而不执行动态插件。调用参数、默认值、view列与export全部使用同一SOURCE Type/Value，不再套另一TypedLiteral族。

| 用途 | 代表 Query 及结果含义 |
| --- | --- |
| 人员当前任职 | `engagements.exists(e, has(e.employer.ref) && e.employer.ref == param.org && containsDate(e.during,param.day))`，employer 与 during 来自同项；text分支不lookup，未知期间不视为永久当前。 |
| 组织名册 | `related(inverseOf(person.engagements.employer.ref),this.node)`，反向行来自获权人员作者项；组织不用另写 roster。 |
| 职级筛选 | 同一 engagement 的 `rank.system == param.system && rank.level == param.level`；不跨制度排序职级文字。 |
| 有序作者 | `publication.contributors.map(a,a)` 保 agent(ref或text)、family/given/credited_as、roles/affiliations各项完整和顺序，text分支不自动建人。 |
| 联系方式表 | `unnest(phones,p)` 投影 `self,p.number,p.label,p.note,p.ordinal`，重复号码仍各一行。 |
| 未完成任务 | `types has task && status == 'open' && project == param.project`；TemporalEndpoint due比较保精度，不补本机时区。 |
| 日历任务 | due为完整native date时才投影 `[due,nextCivilDay(due))` 截止日卡片；CalendarDate(day)需明确provider日域转换。year/month due显示对应年/月精度标记或时间带，明确具体日未知，不放到虚构首日，不造event/reminder。比较/过滤用compareTemporal，同精度年月可用，跨精度边界未决则partial。instant due不猜结束。 |
| 稿件阶段历史 | `publication.publication_history.filter(s,containsDate(s.during,param.day))` 可返回多个冲突断言；不默认最后一条胜出。 |
| 深标题 | `read(headings); filter(level >= 6); project(level,text,occurrence)` 包含生成项；10+ 级仍按真实级别读。 |
| 金额汇总 | `group(currency,sum(amount))` 每币种独立，不混币；无有效金额且有invalid者不能报零总额。 |

相等用唯一递归 Type/Value 规则：text 精确；exact number 按数学值（保作者表示另存于源）；float 有限数且 signed zero 数值相等/原位值保全；instant 按精确 UTC 时刻；ref 完整身份；数组顺序敏感；record 键顺序无意义、数组内顺序有意义；tagged union 按分支+值。group/distinct 比完整相等键，hash 碰撞再比较完整值；选代表时用稳定完整值序，不按当前页第一行。它们产生新 group 行而不继承来源 writer。复杂排序仅按下面明确的SOURCE具名比较器/投影规则，空值位置明确。

排序统一消费SOURCE比较器，不另开Value体系。默认record/union/list没有整体order。具名 `scalar` 比较器仅接受同型可排序标量：text精确Unicode scalar顺序（不locale折叠）、i64/finite float/exact decimal数学序、明确同域civil日期/时刻与精确UTC instant；`ref_identity`仅比较同种完整EntityRef的规范身份tuple，不按标题补序。显式 `sequence_lexicographic`仅接同型且items可用scalar比较器的数组：第一不等项决定，等前缀较短者先；值不被重排/去重。array<record>须先map已绑定标量member或明选min/max等标量投影，record/union不能默认lexicographic；quantity/money先声明单位/币种分组和准确magnitude，不跨域默比。missing/null顺序显式，invalid/unavailable走原状态处理，不按诊断文字排序。SOURCE和Query/参数/view/export均用这些同一规则。

修改第二个重复电话：UI 发出当前 basis 和 item path，Core 在准确前像官方 AST 中重新定位第二项，仅修改目标 member，完整重解析及 schema 检查，一次 Undo/事务。若期间数组重排或源变化则 conflict，重新显示当前各项供选择，不按相同号码猜另一个项。所有保序操作和导出都不因为去重显示、分组或索引排序而回写数组。复杂关系或历史若真实需要跨版本项引用，可由 schema 显式引入该项 id；这属于需要论证的领域能力，不为每个电话默认分配 UUID。

## 7. 保存 Query 与导航列表（NAV01–05）

保存 Query 的定义定位采用作者 note + 唯一应用 definition key，label是显示/原生定位名称。direct metadata用 `kind:"weftext.query"`、Core分配lowercase UUIDv4 key，身份是owner+key，不是新NodeID；定义其余scope/types/where/select/order成员由本文件拥有，SOURCE按同root的官方AST/binding规则识别并检查key与label全根唯一，generated/include/nested/计算声明不给定义资格。正常引用默认 **current**：每次打开当前获权定义，界面显示本次用的 RevisionId，定义变化说明“定义已更新”并重新求值。显式 **pinned** 固定定义 RevisionId，界面标“固定定义”；历史不可得或当前不允许读取时明确 unavailable，绝不偷偷换 latest。pinned 固定定义，不固定数据：每次仍以当前获权数据求值。需要记录一批准确旧结果时使用显式结果导出/历史快照入口，不用 pinned 假扮数据冻结。旧结果、cursor 或 pinned 定义均不给 current writer。

新定义创建分配一次不随标签改名变化的应用 key，仍置于原生 metadata，label 可编辑；读取按 key 识别，不按同名新对象猜替代。复制产生新的 owner/key 映射；current 定义引用随 copy map 更新。pinned 定义引用默认保留原 owner/key/RevisionId，预览明确其外部历史依赖，不把原版本号拼到新 owner，也不换成副本当前版。首代要求完全独立而仍含此依赖时明确拒绝该模式；用户可单独创建并核对旧定义内容的新副本，再显式改引用，不把该操作冒充保 pin 的普通复制。SOURCE 必须将 key 声明、重复检查及字面投影资格与 root 一致设计。高级/计算声明可原生读取保存，但不能伪成应用可执行定义。

| 需求 | 候选交互与状态 |
| --- | --- |
| NAV01 | 导航/列表/正文三面板可收起调宽，窄屏逐层进入返回。会话记录 Query AST、范围、selected identity、scroll anchor + offset、焦点及按 owner 保留的 Draft/Undo。返回先获权重新求值；selected消失显示移出/删除/不可用，不复活旧行。后台预览不聚焦正文或提交 Draft。 |
| NAV02 | 列表可选当前获权摘要、图片/PDF缩略图、明确名称的日期、关键属性与密度。无作者标题以可披露 filename/path 作显示回退并标回退，不补写标题。预览状态是 absent/building/ready/failed/unsupported/unavailable，字段 absent 另标；首行可用即能浏览，不等全库 OCR/缩略图。 |
| NAV03 | 目录、标签、类型及属性值点击生成同一 Query 条件 chip，可逐项移除，包含子目录/全库/正文搜索明确。facet counts 对 current获权相同范围按 `count_distinct(self)`，多值笔记每值最多一次、缺值单独组，partial计数标已知范围，顺序选择不改日期条件。 |
| NAV04 | 快捷入口使用 NodeRef、DirectoryRef、TagRef 或保存 Query key，可重排移除，删除入口不删目标。Node 目录直接用 NodeRef；非 Node 目录经用户明确授权，由 CORE admitDirectory 只登记导航身份后使用 DirectoryRef，不升级为 Node 或改变发现边界；标签由 CORE 登记，SOURCE 的 typed TagRef 与导航/Query 一致。显式接纳只影响选定目录，不自动改造全库或 unmanaged 边界。重复身份、观察缺口或目标失效准确提示，不猜同名。仅保留路径的外部书签明确不具改名稳定性。列表置顶仅改变当前合格结果的显示顺序，不越筛选/权限，也不等同快捷入口。 |
| NAV05 | 个人本地 ViewPreference 按目录/标签/Query身份记排序、分组、列、摘要/缩略图开关和密度，可恢复默认。失效字段/target提示，不替同名项；设置不含结果或writer，不改源、真实目录顺序或其他人的偏好。 |

设备偏好存应用配置，不进入业务 metadata。快捷方式、视图偏好和具名布局都是重新解析入口，不能持久化旧授权/结果/writer。列表 pin 可个人按 context 保存 NodeRef 集合，先 query 得当前成员后再稳定分区 pinned/unpinned，各区按显式 Query 排序；导出 Query 数据仍按 Query 原顺序，导出所见列表需显式含该个人排序选择并标其含义。

表/列表/任务清单/看板/日历/时间轴/画廊/指标和既已选图表布局只消费 typed terminal；无 renderer 明确 unavailable。视图不偷偷追加fetch/filter/聚合。AT 可访问完整表格作为等价入口；行虚拟化不冒全量。快速录入包含位置、类型/模板、可选标题、属性和正文；字段可分组/搜索/固定。成功 Receipt 提供真实对象入口，才可清空“再记一条”；§7.1 的独立窗口可按用户选择关闭或继续记录，不强制打开完整库。unknown 保原 opId/Draft。创建在当前筛选外不是创建失败，完整 current 才能断言不在结果，否则 membership unknown。

任务仍是普通 task 笔记：详情正文、status/due/project等字段共用。勾选、拖看板、改日历日期展示准确 field before/after，prepare重新获权，禁止写聚合/生成行。普通 checklist 是正文项，不自动生成独立 task。周期入口先按明确 series/period/scope 找已有笔记；不存在时只在用户明确创建才写，同 key 并发创建由 Core 决议唯一性。civil 日期、精度、历法和显式时区/规则依赖保持准确；v0.7 的本机任务提醒见 §7.1，稍后提醒、完成后重复和自然语言日期仍后置。

### 7.1 v0.7 桌面入口与分钟级任务提醒

本节承接 [REQUIREMENTS](../REQUIREMENTS.zh-CN.md) §5.9，是待实现方案。Windows 首轮采用 Rust 原生桌面桥接托盘和系统通知，独立碎片窗口优先验证 WebView2 复用现有 TypeScript 表单及 Draft；具体绑定库在小原型中按 IME、辅助技术、离线依赖和 MSIX 结果确定，不先替换整个前端。完整库可继续使用已有浏览器入口；CLI、无桌面的 Server 不启动托盘。平台适配层不持有另一套笔记存储或绕过 Core 的写入口。

宿主基础已从控制台入口拆出：`weftext-app` 库的 `LocalService::bind` 组装原本机 HTTP 路由、会话边界和唯一 Core，`browser_url` 仅向所选界面交付本机会话凭据，消费服务的 `serve_until` 由宿主提供停止信号并在结束时释放租约。控制台参数分发、持久目录选择和默认浏览器交接分别负责自身错误；内部编译／尾注 worker 在解析用户数据目录之前分发，仍沿原独立进程和预算。未来 GUI 可以保留 STA 主线程与消息泵，在后台 Tokio 运行同一服务；本轮没有引入 GUI、托盘、通知或第二写入口。当前数据位置及未验平台见 MVP，MSIX 的包外持久状态和卸载保全仍须按下文另验。

**进程与激活。** 同一用户和数据配置只允许一个桌面常驻实例拥有调度权，第二次启动、托盘菜单和通知回跳通过受当前用户保护的本机通道转交明确动作；拒绝任意命令、任意路径和自报权限，不能同时为 MSIX／便携实例重复注册同一调度。登录自启动仅进入托盘，是否自动打开库另设；系统或用户禁用自启动后不自行恢复。Explorer 重启可恢复图标。关闭 UI 不清空 Draft；退出先可靠保留未保存稿，持久化失败明确提示。重开恢复同一捕获草稿，确认 Receipt 前不清稿，未知创建查原操作；目标库离线、改名或权限失效时保稿并重新选择，不默默写入替代位置。

**任务意图与时间。** SOURCE 在实现切片定版一个可选的任务提醒字段，沿用原生记录、现有时间类型和统一 schema；未设置不触发提醒，旧事件 reminder 意图继续保留，不据此宣称所有事件提醒可执行。区分独立绝对提醒、相对精确 due 的提前量，以及全天 due 对应的明确民用时间／时区。只存一份作者意图，不双写可编辑的推导时刻；相对 due 的改期重算，绝对提醒不跟随 due 悄悄改期。日期字段保持全天，具体触发转 instant 须有明确时区、规则版本及 DST gap/fold 决议；不借宿主默认补全。界面到分钟，已有秒及更细精度保留。任务完成、取消、移入回收站或提醒关闭即停止其待执行项；恢复或重新打开任务后重新核验，不重放已决送达。

**调度与送达。** 以获权的已保存任务及提醒定义构建可重建队列，Draft 不提前触发。设备选择的库范围、启用开关和送达账本保存在 Core 管理的本机应用状态，不成为第二任务库，也不把设备送达状态冒作同步完成事实。同一任务／提醒身份／定义版本／触发时刻／设备形成稳定执行键，重启、重试和时钟回拨不重复提交已知已送项；相对时间变更使旧键失效。分别记录待执行、已提交系统、取消、失败及结果未知；API 成功不能标“用户已读”，崩溃发生在外部通知与本地记账之间时查询可查的原通知，无法确认则显示未知，不盲目重发或承诺恰好一次。多设备分别启用、分别提醒，首轮不做跨设备全局去重。

执行键中的定义版本只随提醒语义及其实际时间依赖改变，不能直接用整篇笔记 RevisionId；无关正文修改仍指向同一送达项。任务身份与提醒身份经改名／移动保持，复制任务获得独立身份，待用户启用其提醒后才进入本机队列。

**暂停与恢复。** 首轮由常驻进程本地调度，不把长时间有效的敏感通知预先交给系统，也不新增云推送或唤醒服务。睡眠／退出期间不承诺送达；恢复、重启及系统时间／时区变更后重新核验队列与任务状态，将错过且未处理的有效项汇总一次供查看，不逐条弹出。通知被系统禁用或勿扰时保留应用内提醒状态，不能强行绕过。应用内“已处理提醒”不等于将任务标完成。

**权限与隐私。** 投递前和点击回跳都重新取得当前所需权限，已有缓存或通知参数不授予读取／写入。默认系统通知仅显示通用提醒文案、不含笔记正文或敏感标题，点击后在应用内获权查看；改期、删除、取消、撤权后尽力撤回本应用已发布项，并阻止后续旧激活读取，不能声称收回用户已经看到的信息。断网仅允许继续执行现有离线授权范围；权限过期、范围不完整或库不可访问时暂停对应项并标受限。托管服务不得在服务器桌面代用户弹窗；WebUI 不假设关闭标签后仍有可靠后台任务。

**安装边界。** MSIX 使用 Full Trust 桌面包，资源只读、可写状态在包外；用户笔记库及需在卸载后保留的恢复稿放到明确的持久位置，避免依赖会随卸载清除的虚拟化 AppData。升级协调常驻实例退出／恢复及版本兼容，保留草稿、原操作身份与送达账本，不默默以不兼容旧版打开已升级数据。验证便携版与安装版的数据归属及实例协调。签名、包清单／图标、版本映射、离线 Runtime、应用激活和更新入口归产品打包脚本；外部 Office 仍按已安装能力探测。正常卸载撤销自身自启动和系统通知注册，保留用户库。具体完成门见 [ROADMAP](../ROADMAP.zh-CN.md)。

平台能力依据：[通知区 API](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area)、[系统通知](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/)、[登录启动扩展](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/desktop-to-uwp-extensions)、[WebView2 Runtime 交付](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)、[MSIX 桌面包](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion)、[包内外文件与卸载](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes)。这些资料证明可用平台机制，不代表 Weftext 已实现或验收。

### 7.2 日常使用入口与普通节点组合

REQUIREMENTS §5.15 在 v0.6.3 完善现有入口，不创建工作台类型、独立仪表盘或第二套查询模型。快捷打开、命令搜索、最近访问和返回位置复用现有对象身份与导航状态；节点内容／媒体浏览、图标及同级排序沿原导航、资源与结构能力实现。个人显示排序与作者顺序不同，后者只有明确命令才能通过 Core 修改；隐藏不等于撤权。

建库提供空白和可选根节点初始内容。选中预设后，用 §10 的创建模板及原建库／创建计划生成普通根笔记、解释文字和用户选择的 Query；预览实际新增文件、类型／字段依赖、查询范围与定义调用。实施前核定根笔记与现有 RootDirectory／库配置角色的准确关系，不能把目录书签冒充笔记，也不能为预设再建一个特殊根身份。初始查询只依赖已安装且可用的类型／字段；缺项明确解释，不静默安装未选配置或制造业务记录。新建结果与原操作可查询，取消、失败及重试沿原事务恢复。

预设只产生初始作者源，不保留一个会在升级时重新强制应用的布局权威。用户之后按普通编辑操作增删、改写、重排及清空，根节点的标题和用途都可改变。多个 Query 仍按 §5.5 分别关联出现项、参数、结果和失效状态，复用保存定义时不复制结果当正文。删除调用不删除被查询的笔记；删除被其它调用使用的定义须保留正常的引用失效说明，不能级联删除调用者或目标。根节点正文遵循普通共享权限；个人结果、可见性与显示偏好不反写共享内容。

属性就地编辑、关联选择、公式／汇总构建器和视图切换使用同一 schema、CEL、Query 与窄写资格。派生格、聚合格、来源不唯一的行只读；拖动看板卡片或批量修改须显式说明实际目标／字段，并通过既有 prepare／commit，不从展示分组推导写权限。先定义可支持的作者槽和操作范围，再开放控件；保存查询定义、修改调用参数与修改结果对象是三个可辨操作。

链接完整性入口在已有索引上提供获权问题分组和准确定位。孤立计算只针对声明范围内可证明的有效连接；无入链不等于孤立，partial 不得标全库孤立。预期孤立和忽略规则只改变诊断呈现，不删除真实边。片段候选扩展 §4 的词典，但每项携准确目标及片段来源；显示标题不是身份，新增大小写／部分词规则需显式选择并声明支持，不改变默认精确匹配。标题变化、重复标签或无法定位时标不可用，不能自动转为整篇笔记链接。

关系图只消费 §8.2 的获权关系断言及原链接索引，区分正文／文献／属性边、派生反向与目录结构，不把潜在提及当真实边。全局／局部及邻域展开明确范围、方向、关系类型和覆盖；相同端点的多条来源可展开，合并显示不能合并作者事实。只读图中的拖动默认调整布局，不移动文件或改关系。创建／删除关系须显式选择原字段／来源并走原写入。先交二维一般／层级／家族布局与可访问列表；不能布局的循环保留事实并说明，不泄漏无权节点或边的数量。三维相机／星系效果后置。

UX01–12 在 ROADMAP 统一验收；涉及新增持久偏好、预设来源或具体接口时，实施前更新实际 owner 合同，不在此预造文件格式或另一套通过台账。

2026-10-19 第二阶段实现：可选根预设只创建普通正文和最多三个原生查询，原创建计划一次接纳，后续不重置。多个出现项的结果／运行／个人视图按准确 key 独立；表格／列表／看板／日历保留缺值与无日期桶。作者筛选／排序／投影／分组进入原 QueryPlan 和执行器；复杂 same-item CEL 保源。直接字段 writer 须证明真实 NodeRef／FieldId、非派生当前来源并重新取得写资格；历史／汇总／不明确的行不写。共享 current／pinned 调用只改 caller，不从 Query 读取权推导 callee 写权。实际控件、个人偏好生命周期与限制见 MVP。

### 7.3 节点、诊断、潜在链接与图谱的操作流程

本节细化 v0.6.3 的 UX02、UX08–10；参照证据见 REQUIREMENTS §5.15.2，共用入口、焦点和取消规则见 EDITOR §2.8。操作对象仍由原 Core／Query／关系合同确定，不使用参照插件的路径身份或私有格式。

#### 7.3.1 节点与内容浏览

文件树的展开箭头只展开，点击节点名称打开正文；节点内容视图的单击负责选择，双击／Enter 打开，预览用独立入口。面包屑可返回父级并保留该处筛选、选项与滚动位置。无标题、缺主文件、不管理及不可用资源使用文字和图标区分；状态说明可从更多菜单或键盘到达，不靠颜色推断。缩略图按需加载，失败显示文件类型与原因，不阻塞列表。

右键、更多和 Shift+F10 打开同一对象菜单。对已选项打开菜单保多选范围，对未选项则明确改为该项；菜单标题显示单个对象或所选数量。文件动作与“整个节点”结构动作分别命名，重命名、移动、复制、移入回收站显示实际目标和包含范围。节点内拖放显示“移入”的目标高亮；同级重排显示前／后插入线，并且只有作者手动顺序模式提供该写入。个人名称／日期排序不因拖动转成作者顺序；切换手动顺序是明确动作。移动对话框和上移／下移提供键盘、触屏等效入口；重名、循环或撤权时保选择并报告原因，取消不改变结构。

图标选择显示当前有效值与继承来源，预览后应用；恢复继承、清除当前覆盖与隐藏节点是不同动作。模板、根节点预设及普通节点使用同一图标／内容／Query 入口。个人筛选、布局与展开状态按 NAV05 保存，只有显式作者操作改变结构、属性或正文。

作者顺序消费 CORE §2.4 的同一设置投影。节点内容显式选择“作者手动顺序”后，已接纳直接子容器才提供前／后拖放和上移／下移；个人名称／路径排序没有这些 writer。准备前可取消提案，准备后核对原父身份、完整前后顺序及实际设置文件影响，再确认原计划；关闭对话框保留原编号，刷新或重启可查回，同一原计划可明确取消。个人排序、摘要／缩略图开关和密度按稳定父引用保存在本机，可恢复默认，不保存结果或作者权限。

本地节点内容与 Query 列表共用按可见行启动的缩略图组件。节点列表先核对其获权扫描源的准确 hash 和原 NodeRef，冻结 Core 源读取 Base；Query 沿原行的准确源依据。二者均用同一当前获权资源清单和准确 ResourceRef／revision 交付图片或 PDF 首页，不建立编辑会话、不提交正文。异步状态分别显示无图、等待／生成中、就绪、解码失败、格式不支持和当前不可用；撤权／源或资源依据变化不回退最新资源。晚到结果不抢焦点，切换范围或关闭显示清除像素与 Blob URL，列表与打开笔记不等待其他缩略图。

#### 7.3.2 链接诊断

“关系”入口提供“链接问题”和“孤立节点”两个可键盘切换的结果页。问题默认按目标分组，可按来源或逐处引用查看；每处来源独立定位，同一缺失目标的多处引用不能被折叠成无法到达的唯一一行。孤立结果可切换列表／目录树；无入链是明确标注的另一筛选，预期孤立单独标识。节点存在但内部片段失效时，节点间连接与片段诊断分别呈现，不以缺片段推断目标节点不存在。

首次打开利用现有索引；未就绪时展示建立中、范围和可取消的观察／任务状态，不要求用户先找隐藏设置。更新失败保留仍获权的最近结果并标过期，重试不清空搜索。分页／虚拟化必须能到达完整获权结果，计数区分当前可见、检查范围和部分覆盖；不复制插件的固定每页条数。输入法组合、查询更新和增量索引均保搜索框焦点；清空搜索只清关键词，筛选和展开状态保留。空库、搜索无匹配、筛选无结果、尚未扫描和本次范围未发现问题分别说明。

点击问题或 Enter 定位准确来源；只能打开笔记时明确说明没有精确位置，失效片段可另选“仅打开目标笔记”。正文提示默认关闭，开启后不截走链接的原导航；hover／focus 可看原因，Esc 回到原链接，键盘／触屏可从当前笔记诊断列表获得同样信息。行菜单的忽略或预期孤立操作先显示对象／路径范围及匹配预览，保存后可撤销偏好修改；取消、Esc 不提交规则。忽略诊断与排除候选不改变真实关系，个人规则不写入共享笔记。设置保存失败与扫描失败分别提示并保各自输入。

#### 7.3.3 潜在链接

沿 §4 的完整匹配、重叠列表及默认关闭方案，工具栏／命令提供显示开关并说明只改变个人呈现。候选标记与真实链接有非颜色差别；hover 或键盘聚焦提供预览，点击／长按打开固定位置的候选列表。列表显示命中文字范围、笔记与内部片段、匹配来源及多个目标，不能靠悬停时在正文插入编号而反复挤动文字。进入候选菜单后保持列表展开，Esc 关最内层并恢复原选区；点击“打开”只导航，点击“转换为链接”才写一次 Draft。

候选菜单提供忽略该词、排除此目标和打开匹配设置，预览明确只影响此篇、所选范围还是个人全库偏好；不借排除操作改笔记标签。首个版本保默认精确匹配与全部重叠候选，提供范围／排除控制；不照搬启发式大小写、只留首个重复词或词内匹配。新增匹配算法须另行定版支持与验证，界面不能显示未实现选项为可用。真实链接、代码、公式等排除沿 §4，不由控件改变扫描资格。

输入法组合和正在编辑的文本范围不因新提示重排；组合完成后再安装适用装饰。切换关闭清除所有支持表面的提示及弹层，不留下仍可点击的过期标记。标题／片段无法稳定寻址时仍可说明只读候选，不能偷偷新增目标标签或退成整篇链接；需要作者改动的稳定引用按 EDITOR §7.1.1 明确处理。撤权及内容变化关闭失效候选并保自己的草稿。

#### 7.3.4 二维关系图与结构图

首次从“关系”打开当前节点局部图，范围选择器可切到全库；目录层级和标题结构是独立视图，不将它们的父子关系混入业务边。采用以下 Weftext 默认行为；参照 Obsidian 的直接打开和 Galaxy View 的选择聚焦后打开，在需要查看多种边来源时选择后者。

| 操作 | 明确行为 |
| --- | --- |
| 选择／打开 | 单击选中节点、突出获权邻域并显示详情卡；双击／Enter 或卡片“打开”进入笔记。空白单击／Esc 清选择，不改筛选。键盘结果列表可完成全部选择、打开及关系检查，不依赖 hover。 |
| 查看边 | 边可选中，详情列出关系类型、方向、原始字段／正文来源和是否派生反向；箭头与文字“来源→目标”共同说明方向，对称边不伪造箭头。同端点多条关系可逐项展开，不合并作者事实。 |
| 筛选／范围 | 类型、入向／出向／双向、邻域深度与目录／标签条件是可见且可清除的控件。搜索选择只定位图中对象，改变范围须明确操作；图例可点击筛选，不能只有悬停才可使用。显示当前范围、已展示数量和截断状态，不以全部不可见对象计算公开总数。 |
| 平移／缩放 | 空白拖动平移，滚轮缩放，按钮提供放大／缩小／100%／适应视口；节点拖动只改布局。输入框内的方向键和快捷键不控制相机。搜索、筛选与增量更新尽量保持视口，只有显式定位／Fit 才重新居中；缩减动态效果时不自动飞行或巡游。 |
| 关系编辑 | 详情提供“打开准确作者来源”，新增／移除在原正文或字段入口明确选择实际来源，预览后走原写入权限；派生反向指向真实作者位置，不重复写另一条边。只读、循环约束、来源不唯一或旧状态均给原因，布局操作没有写入副作用。 |

颜色、标签密度和布局属于个人显示；命名颜色组可复用已有 Query 条件，不新建事实库。普通／层级／家族二维布局提供同一选择与导航，无法层级布局的循环仍可在一般图或列表检查。大图按已声明预算加载并显示范围，使用列表替代不冒图已完整渲染。三维、漫游、最短路导览、力学／视觉预设和标签枢纽留作后续范围，不照搬参照产品的帧率或节点数承诺。

rc.1 工作树已接入同次知识读取的二维图谱和完整获权列表，图谱本身只读，编辑回到准确作者入口。当前本地预算为 5,000 节点／20,000 边，托管为 512 节点／4,096 边并保原 8 MiB 回执预算；SVG 绘制上限 200 节点／1,000 边，下方列表保本次已取得范围，部分覆盖与绘制截断分别报告。标题结构独立消费同次原生编译，预算为 5,000 标题。实现／组件通过不替代最终候选实际任务，整版验收状态见 MVP。

### 7.4 多级标签的浏览、选择与保存范围

新增的共享标签树采用 CORE §2.3，父子只表示可调整归组。点击树项默认“此标签及子级”，条件 chip 和结果页明确模式，可切到“仅此标签”；展开／收起只改显示，不赋值或换父。旧精确查询／TagRef 快捷入口保持精确。统一快捷入口模型的标签项必须显式携 `matchMode: exact | subtree`，保存当前筛选按原条件往返；只在存在真实旧入口时做有限转换，原仅 TagRef 项转换为 exact。新项缺模式属于非法输入，不保长期的缺省／双格式分派。已有保存 Query 也可表达完整条件。首版不做隐式每标签模式记忆，某处切换不改其它入口。不能仅重画旧入口就暗改其含义。

创建、改名、换父／移至根通过同一服务端 Core 操作，界面与键盘共用准确选择、预览、取消及原操作恢复。赋值／选父／树项显示分段面包屑；名称中 `/` 保完整片段。同名或同显示路径须附足以区分的 ID 后缀，碰撞则加长；创建先展示同名候选并允许明确仍创建，不按同名替换身份。父选择器排除可证明的自身及后代只是辅助，最终仍由 Core 全森林校验。树／选择器按声明预算分段或虚拟化，祖先路径按需生成，不为所有项缓存完整路径；显示截断须可辨并提供查看入口，不改变身份或 Query 完整性。

表单只写直接 TagRef，改名或换父不增祖先；显示路径与原字段身份分开。所有名称／路径／数量及结果都遵守独立获权范围，树可读不授笔记读权。完整标签对象、笔记库存完整性与界面仅展开部分节点分别显示含义；移动不要求全量笔记库存。二维图谱现复用同一选择器与显式条件，布局不定义标签 parent，也不引入继承。

硬删除、合并、批量拖拽、手工排序和多父不在首版。可逆隐藏另据真实需求决定，不预置共享状态机；若将来选择“从新标签选择中隐藏”，须单独操作及授权，保留现有引用和匹配，不级联后代，不等于禁止赋值或释放 5,000 条额度。局部引用为零不足以证明可安全删除。层级已接入 rc.1 工作树；原 alpha.3 仍按其扁平候选范围解释，新增实现及定向回归不替代 MVP／ROADMAP 的最终整版完成门。

## 8. 丰富 schema 的业务保留与查询职责

所有类型支持实际需要的多值、嵌套、时间与来源；集中 schema 定义类型/约束/关系，短名供普通 UI/source使用。常用字段先显示，可选复杂信息按组展开。以下是历史已选能力的重新表示范围，不是继承旧每值 Entry/qualifier/Registry名义版本；SOURCE 拥有完整字段定义，Features拥有查询和视图语义。

| 类型 | 当前保留能力与表示职责 |
| --- | --- |
| Person | 有序 names（角色/语言/文字系统/期间）；重复 phone/email/address/website/account（preset/custom服务、usage、label、note）；profession与制度限定rank、任职（目标普通笔记或显式文本fallback、岗位/部门/期间/状态）；测量（量纲/时间）、国籍/法律性别/性别认同的独立期间断言；生命事件/重要日期与来源/冲突/显式选择；头像资源。parent/guardian/manager/mentor方向明确，spouse/sibling/一般family/social/professional关联保留独立历史和角色。不从姓名/自定义称谓推断方向、血缘或当前有效性。 |
| Organization | 多名称、标识、分类、状态历史、site/address/contact；业务指导/属地管理/共同领导/监管/母子公司/品牌/组织成员、parent/governs/owns及一般关联分别定义，不按目录推组织关系。人员名册派生任职，不双写。 |
| Project/Task | 项目是任务指向的普通笔记，完整Project领域schema仍属候选，不凭名称新增必填字段。task status为required，start/due/priority/project/completed-at/dependency保已选含义；详情正文，完成时间是明确动作产生，不因手改status虚构历史。完整任务状态历史尚非已选，不能自动加入。周期索引/已有对象导航保留；§7.1 明确选择 v0.7 基础任务提醒，复杂HR／高级提醒仍后置。 |
| Work/Paper | 多种作品类型、标识、保序作者/机构与角色、publication状态历史、venue为Work/Org引用、container为独立作者文字；版本关系、引用键及owner-local附件保留。当前未定义独立publisher字段，venue不冒publisher；paper subtype有实际论文所需卷期页码/日期/摘要/主题等。版本取得独立可引用身份时显式新笔记，不按同DOI覆盖。 |
| Calendar | 已选事件区间、来源、参与关系、系列规则/例外/改期/取消和已有周期笔记定位的资料能力保留；日期/时刻、precision/calendar/timezone清楚。事件提醒送达仍后置但保已存意图，§7.1 的任务提醒不自动扩大为全部事件提醒。完整规则schema与provider由SOURCE/协调 owner定义，本文件不把任务截止日自动变事件。 |

每条历史记录可选 validity、status/selection、note 和 source 信息，按领域放在同一原生记录；不另包一份通用事实信封。来源既可为原始附件定位也可为引用，不授其读取权。重复相同值/重叠期间允许保留，查询的“当前”必须显式期间/选择规则，不默认为最新写入。多个 preferred 造成歧义，界面显示并要求选择，不最大置信度猜胜出。

关系是 schema明确 typed ref槽与方向，事实只在一个声明所有者存一次，反向视图派生。symmetric关联也从一个明确作者所有者存储，不自动改另一个端点；从任一端打开编辑先获权定位该作者源，不持有源写权则只读。反向行保准确 `(sourceOwner,field,itemPath,ReadBasis)`，准备时重验，不直接写显示端点。已有另一独立断言保留其来源，不静默归并成同一事实；graph可合为一条显示边但展示多个assertion来源，不能拿边去写未知来源。人物text fallback无真实ref则不lookup/inverse/自动建人；目标删除后保作者事实并标suspended/dangling，cleanup须显式获权Plan，不自动跨源删关系。人员任职允许普通目标，不强迫先升级组织；Org其余typed关系按schema验证域。parent→child、guardian→ward、manager→direct-report、mentor→mentee和engagement→has-engagement逆向明确；general family/social/professional关系按SOURCE §4.1真实Family/Social/ProfessionalRelation读取required relationship.preset或relationship.custom；码域分别extended-relative / acquaintance,classmate,friend / colleague，同名custom保独立分支；跨域/双分支/未分类invalid，raw保稿，custom不猜方向。由任职重叠计算的colleague视图按需派生，不批量存O(n²)副本；显式professional_relations断言独立保留，无engagement也可按其真实Field规则记录，不自动双写或合并两种来源。业务图的cycle/rank约束仅在schema确有规定时验证，不为自由关联统一造家谱/组织树。

### 8.1 周期、区间与事件的真实消费者算法

Calendar有三个已选资料类型：period-note、range-note、event；day/week/month/quarter/year是period种类，不是另五种身份。period字段明确series、calendar/rule、period key、zone及unique/many scope；路径/标题不定义key。period导航先以同一Query查准确 `(series,key,scope)`，partial不能宣布不存在；unique创建需Core同Plan验证完整负范围及并发约束，many允许多个独立笔记。

Range用SOURCE.TemporalEndpoint保年/月/完整日或instant；半开缺端无界/none非法/至少一端有界仍保留。During独立闭合，缺端未知/none无界。准入、同粒度/混精度包含、重叠及粒度单位时长统一调用SOURCE §3.1.1 fixed provider，**先完成整个调用的获权输入/provider和跨实参共同时间域资格，再处理无界和三值合取**；单项Range有效不等于两个实参共域，不补具体日/host zone。near-boundary unavailable不是false；明确远前/远后或内部证据可确定。Event复用Range，但present recurrence要求原精确DateOrInstant bounded模板及ExactRange替换，不将部分精度直接转成发生锚点。participants/reminders等不变。

recurrence按冻结calendar/zone/provider及source anchor的civil phase生成daily/weekly/monthly/yearly候选，应用interval与显式byMonth/byMonthDay/byWeekday/weekStart，count或inclusive until限制；不存在民用日期跳过，DST gap/fold按明确规则处理，不能host默认。定义B为合法base集合、R为rDates、E为exDates；先按精确原发生键对B∪R去重，然后对每键应用 **cancel > replace > EXDATE > template** 决策，绝不能先EXDATE删候选再试replace。replace可改range/title/status/note，不把改期时间变原发生身份。常规窗口与窗口外移入replacement使用下面同一决策函数，再按实际range过滤/排序；预算不足标partial，不漏移入却说complete。没有默认每次发生建Node；显式promote/adopt才创建。改series时预览已有exceptions的keep/remap/drop，不能静默丢旧例外。per-occurrence参与者/提醒覆盖若schema未选，保导入原件并报告unsupported，不冒全RFC兼容。

#### 8.1.1 原发生键的唯一决策与窗口流程

原发生key取date完整民用起点或instant精确UTC起点，offset不同但同UTC为同键，不用replacement起点或数组ordinal。base count在EXDATE/exception前对准确base起点排序/去重后计，删除不补下一项；RDATE不消耗count且可早于anchor/超出count或until，只要同域。until是inclusive的base起点限制，不限制已合法明确RDATE。EXDATE没命中B∪R仍保作者意图，不产生发生项；每exception的onset必须可证明属于B∪R，缺membership证明不能把它当幽灵发生。

在SOURCE完整typed准入已经证明每个原键最多一个exception之后，`decide(o)` 只用同cut的规则/集合：

| 原键o的状态 | 唯一结果 |
| --- | --- |
| 有cancel exception（是否同时EXDATE不限） | 不产生发生项。 |
| 有replace exception（是否同时EXDATE不限） | 产生完整replacement；未覆盖字段继承该次template，EXDATE不删除它。 |
| 无exception且o∈E | 不产生发生项。 |
| 无exception且o∉E | 产生该次template。 |
| 同键有多个exception，包括cancel+replace或不同replacement | typed series invalid，保原件并诊断；不能利用cancel分支洗掉冲突。 |

窗口算法：从B/R枚举可能与window重叠的原始template候选，同时枚举所有有限replacement onsets，逐一验证其B∪R membership；合并原keys并去重 → 每键只运行一次decide → 取实际range与window重叠者 → 按actual start、actual end、original key稳定排序。不能普通候选“先EXDATE”、移入候选“先replace”两套路径。有限替换枚举需完整coverage；不可完整证明时partial且不能给完整零结果/negative range。只改title/status/note的replace仍继承原range；移动到window外则不返回，原键仍保。

| 设计oracle（产品未运行） | 期望结果 |
| --- | --- |
| O=2026-10-15，O∈E，replace仍在同窗口 | 返回replacement，不能因EXDATE消失。 |
| O∈E，replace从window外移入window | 用同decide返回一次replacement，原key仍O，按actual时间重排。 |
| O在window，replace移动到window外 | 原template与替换都不在该window结果中；不保留原位置的幽灵卡片。 |
| O∈E且唯一cancel | 不返回；cancel胜EXDATE/template。 |
| 同O的cancel和另一个冲突replace | SOURCE typed准入拒绝而保source；不是两个卡片，也不是silent cancel胜出假valid。 |
| EXDATE不命中B∪R | 不新建/不删别的key，保EXDATE作者值；合法series其余发生不变。 |
| 同UTC不同offset的RDATE/EXDATE/exception key | 按唯一精确key计；不能用raw拼法绕过重复例外检查。 |

该决策保留历史已选改期结果，SOURCE/provider新schema与实现仍是本代的独立合同，不继承旧wire或版本包装。

pack提供历法/节假日上下文，结果保pack/version/source/applicability，多pack并存，不写一个isHoliday权威。ICS UID/RECURRENCE-ID/SEQUENCE是来源标识不等EntityRef；VFREEBUSY/VTIMEZONE不自动Node，VTODO/VJOURNAL显式preview映射。Period、Range、Event完整native schema归SOURCE，以上算法是Features消费者实际义务，不能用“保留旧服务”替代。

这里没有要求常用信息填所有复杂字段，也没有禁止确需的逐项身份。schema与关系迁移必须显式 preview 原件、受影响字段和未知值；未知字段保源并只读展现，不能先删除后称精简。

### 8.2 可安装关系定义及唯一source的真实producer

SOURCE修订的闭合schema增加 `relations`，不是第二关系数据库。每个Relation定义必须落到当前schema真实FieldId和展开后的准确typed target path，并声明source/target type域、方向、literal分支策略、来源基数、self规则及graph projection；inverse名只供派生呈现，不创建作者字段。安装器从完整schema展开验证path里的每个member/array-each与目标ref shape、Type membership、max/null等约束，不接受自由运行expression当target extractor。本节描述语义接口，准确JSON成员由SOURCE的 `schema.json` 唯一拥有。

```text
extractAssertions(sourceSnapshot, installedRelation, cut)
  -> ordered [{sourceOwner,fieldId,itemPath,sourceBasis,
               relationId,targetRef|literal,domainPayload}]
validateRelationChange(relation,before,proposed,cut,authorizedScopes,budget)
  -> complete dependency set + accepted semantic after | rejection/unavailable
readRelations(relation,scope,cut,budget)
  -> frozen source-assertion rows + coverage
```

extractor只读官方native字段投影，array ordinal仅在当前sourceBasis有效。沿Shape判别的ref/text union读取，ref路径在text分支缺席不是整个item absent：只在该关系明确允许literal时输出独立literal assertion，保文本但不lookup、无inverse/target-domain/graph、不会因同名目标存在而改变结果。ref+text双出现invalid不跳过；optional wholeField/member absent不造assertion。source基数统计真正作者records，包括许可literal，不先过滤ref漏计。ref-only关系若作者放text则保raw并typed invalid，不统一给literal豁免。People10与本轮unknown Work contributor可literal；Org12、Work venue/version_of、Task dependency、Event participant仍ref-only。

ref arm由Core解析准确EntityRef/lifecycle、SOURCE验证目标类型：engagement→ordinary Node；Person人物关系→Person；Org12关系→Org；Task dependencies→显式Task；Work contributors→Person|Org；venue→Work|Org且不等publisher；version_of→Work；Event participants→Person|Org。安装path实际来源是SOURCE成员 `target_path` 的member/each steps：engagement经 `engagements[].employer.ref`，contributors经 `publication.contributors[].agent.ref`，venue/version_of经 `publication.venue.target.ref` / `publication.version_of.target.ref`，task.project是scalar ref的空path。field-only规则只读取声明范围及实际端点资格，不回读隐藏body。

27种历史已选关系语义有可安装来源，而非散文枚举：Person10、Org12、Work3（contributors/venue/version_of）、Task dependency、Event participant。本轮task.project另装为第28项关系，不能将数量当已选证明。新文件必须把每种路径、域与方向实际装入schema；安装了其中一个不能宣称其它已有typed inverse。SOURCE Relation成员闭合集合为 `id/source_type/field/target_path/target_domain/target_types/direction/inverse/max_assertions/reject_self/graph_projection/cycle_policy/resolution/delete`，exact typed shape和constraints以其文件为准。Org parent、Work venue/version_of最大1按真实owning字段约束检查，其余按已选many；不能从显示图层合并边再计算作者基数。

create/update先校验完整当前作者container及适用type，再提取拟议after assertions，检查存在/类型/lifecycle/self/基数等真实需要的范围，给准确窄变动和全部read依赖入Core Plan。新增ref需要可读live准确端点；既有目标回收/失效保事实但派生suspended，不自动删。移除/清理针对作者source，不为删除inverse写另端。membership改变必须重验其真正受影响relations，不能只改types而让旧typed边伪合法。

每个symmetric assertion只一个作者owner，允许独立作者assertions各保其来源，graph按 `(fullEndpointPair,relationId)` 合并显示保完整assertion列表；写入必须选择准确assertion重新获权source。两条独立事实不是双写同一事实。完整列表不含隐藏source标题/数量；partial不说没有关系。copy同map重写真实target槽，并对新source after重新校验，不字符串扫描；外部target保留或显式损失，不伪造身份。

`graphProjection=general/ranked/family` 是布局含义，不自动等于acyclic写约束。guardian是general，manager/mentor是ranked，parent/spouse/sibling带family角色；custom一般family/social/professional称谓不生成代际方向。循环事实未经明确业务禁止仍保留，rank布局歧义可改general/标ambiguous，不能删边或借布局新增规则。若明确安装的规则确实要求acyclic，用同cut完整可达闭包的DFS/SCC验证proposed graph，并把遍历范围/负范围入Plan；完整性不足不可声称accept。需要全局incidence/unique/cycle范围的写入要求预先声明且获准完整constraint scope，不按“隐藏边恰导致cycle”动态返回不同公开错误；缺完整scope授权统一unavailable。普通source-only关系不因此被迫读取全库。

### 8.3 Calendar、量值与provider的闭合安装和调用

SOURCE修订schema包含七个已选base类型与paper扩展所需真实shape、constraints、`providers` 和 `series_policies`。provider是固定准入的实现/数据/精度/时区规则选择与静态导出函数签名，不是schema里的可执行代码。配置恰使用SOURCE的 `id/family/implementation/data/exports/limits`，implementation恰builtin id+abi，exports恰name/inputs/result/effect:pure/query_name（唯一公开CEL名称或null）；不向这个closed对象散文加digest字段。Core把该配置revision、真正准入builtin ABI/build digest、完整数据内容与runtime profile绑定成受保护实际依赖。过期/缺版本不自动latest或转text。Query解析基线固定其语义函数签名，LogicalCut依赖固定本次执行真正的实现/数据；静态shape有效不证明provider存在，状态明确provider_unavailable。只有query_name非null的白名单服务export进入CEL，primitive periodBounds等内部export不因此公开；同名query_name拒安装，内部调用按provider id+export精确选择，不按provider数组第一项胜出。SOURCE `builtin-abi.design.json` 是设计签名证据，产品安装器必须与受信Rust builtin registry逐项核implementation/abi/signature，不能以配置自授函数能力。

最小消费者签名与语义如下（使用SOURCE Type/Value，不另建日期/量值V）：

| 函数/操作 | 必须完整实现的语义 |
| --- | --- |
| `containsDate(during,point)` | 共用 SOURCE §5：during 两个成员都须存在，原生 none 明示该端无界，省略任一端/整个 during/空记录均为 unavailable。两界含端点且 start≤end，point为TemporalEndpoint；同精度年/月可用，混精度按SOURCE §3.1.1证明。先对point及全部有限端执行SOURCE调用级共同基准资格，两端none也验证point；混civil/instant或已证倒序invalid，不可判断/缺provider为unavailable，不可用值转 EvaluationIssue。instant 按精确 UTC，date 按明确 calendar/precision，不补 host zone；不借用 Range 的缺端规则。 |
| `compareTemporal(a,b)` / `duringDuration(during)` | 固定v2导出与SOURCE §3.1.1相同：前者给有据时间顺序或unavailable，后者给闭合粒度计数/精确instant差。Query显式调用，不给所有record默认order。 |
| `nextCivilDay(date)` | 指定calendar/provider的下一个民用日，溢出错误；只处理声明的完整day date，不将year/month/月日当已知年日。 |
| `rangeOverlap(a,b)` / `rangeDuration(range)` | `range(start,end_exclusive)`半开且同域、至少一端有界；overlap先完成SOURCE §3.1.1的全部有限端/provider/共同civil或UTC基准资格，随后才用§8.1无界/三值公式；同civil calendar/version可混precision而不需先后证明，跨calendar/version仅全day可用，否则unavailable；civil/instant混域invalid，不允许true/false洗掉资格失败。duration也先资格再处理无界/单位；year/month/day同精度duration是years/months/days单位差，不冒精确天数；instant用exact elapsed seconds。开放duration不可得，不填现在；缺calendar/provider停止，不产假值。 |
| `periodBounds(period)` / `resolvePeriod(period,seriesPolicy,cut)` | 前者是installed calendar/temporal纯函数，检查kind/calendar/version/rule/key/zone/tzdb、ISO week-year，返回确定range。后者是Features/Core orchestration，用已获权slots的period结果+同cut Query查已有/negative range，不冒可以直接读库的pure provider。series/period/scope是实际唯一键，标题/目录不参与。 |
| `expandRecurrence(event,window,providers,budget)` | §8.1完整候选/selector/count或until/rDates/exDates/cancel/replace及窗口外移入；原发生坐标和replacement时间分开。partial包含停止位置与缺口，不提供完整负范围。 |
| `validateQuantity(kind,value,unit,precision)` | 此逻辑wrapper消费 `units/rules` 的 `measurement/quantity` 实际exports，不新增未安装同名export。height/weight分别length/mass、精确decimal，unit与kind/维度同provider；unknown unit保原件但typed业务不可用。不内建默许所有单位，也不自动跨币/跨制度换算。 |

逐类归属：Period、Range、Event的记录shape和local约束由SOURCE安装；Field/Type IDs、relations和policies是真实config；Features提供以上具名纯validator/函数或有限只读expander；Core冻结provider与完整scope依赖并在prepare/commit重验。provider host只给已获权输入slots和budget，不能直接读库/文件/网络、写作者源或签授权。scalar provider失败报明确typed错误；需要全局事件窗口的expander可给partial；二者不洗成absent。

recurrence的准入不得只检查字段是dictionary：source anchor由event唯一完整date/instant `range.start` 与provider派生（CalendarDate转换是显式预览，年月不能补首日），作者recurrence不重复存另一anchor；其域与range一致、frequency/interval/selector域有效、count与until互斥、until域准确、rDates/exDates原发生域、exceptions key唯一与replacement range有效、zone/tzdb/规则完整，均要由SOURCE闭合具名constraint准确列reads，并调用对应固定profile。daily/weekly/monthly/yearly是civil phase；date range长度按民用日，instant模板长度按exact elapsed；不存在日期跳过，DST gap/fold策略由已安装profile明确给定（例如gap=skip、fold=earlier），不能host默认。profile换义需显式schema/规则迁移与exceptions keep/remap/drop preview。仍不宣称全部RFC recurrence/所有历法，不能表达的信息保原件和loss。

period `unique/many`策略实际装在series_policies，明确series/scope及key producer；unique检查必须在授权完整scope同cut读取已有+负范围，prepare/commit把该约束保护到Core单决议。隐藏成员导致未获完整scope时操作统一unavailable，不依据它是否恰冲突泄漏存在；readable已有对象返回准确选择/打开入口，partial只报membership unknown。多个series同period、many同key各个笔记均可共存；policy变动与旧key迁移是显式config Plan，不靠文件名去重。

本节的安装要求必须在SOURCE修订完整schema中实际闭合，未知约束名/函数/shape拒安装；缺少完整七类definitions时标具体不可用，不以“实施前再生成”宣称设计已可执行。当前为协调后的设计候选，安装器/validator/provider/Query/权限尚无产品实现证据；非作者报告与新准确候选复核前不自报关闭。

## 9. 独立 JSON 批注和原子建议

采用 node-owned `weftext.annotations.json`，每条有批注 id、回复关系、作者归属与时间、目的（comment/mark/suggestion）、正文、标签、外观和open/resolved状态。正文可用受限 Typst inline，允许文本/强调/公式/获准link，无 import/file/network/执行；格式错误保原字符串与诊断，不空掉讨论。unknown数据成员保原件，只有已理解且可安全patch的记录可编辑；整文件损坏时不覆盖空JSON，提供当前原件、可靠备份/恢复入口和无法读取的说明，尽量读取的讨论须明确 recovered/unverified，不能给它建议执行资格。

```text
AnnotationTarget = whole_document(owner)
 | source_element(owner, revision, sourceRange, elementRole)
 | source_range(owner, revision, byteRange, sourcePreimage, visibleExcerpt)
 | whole_resource(resource)
 | pdf_region(resourceRevision, pageIndex, pageBox, rectOrTextRange)
 | image_region(resourceRevision, pixelSize, rect)
 | media_interval(resourceRevision, timebase, startTick, endTick, optionalRegion)
```

所有坐标有闭合单位、原点/旋转/页尺寸或timebase，范围有界、端点顺序准确，不把微秒/帧号/裁切后坐标混用。PDF文字range须该revision的准确提取产品，图片归一坐标须可往返到固定尺寸，视频区域同时有时间和帧内区域；不能因 UI 不支持降成whole attachment。flow/print同一真实源选区签同target。generated/multiOrigin可整文批注或明确真实源选区，DOM坐标不作为永久源地址。新 range 先提交其源 Draft，再获current源重定位；源保存失败不丢批注草稿。

targetResolution 与讨论状态分开：exact/mapped/stale/ambiguous/orphaned/unavailable。新源版本只在 Core 保存了连续、准确编辑映射且范围未被改变时自动迁移；外部变化无连续证明就 stale/orphaned，不能相似quote搜索自动重锚。人工重附显式选current目标，预览改动；已有建议必须重新确认。range/element失效仍出现在未定位讨论列表，可读回复、resolve/reopen；不能删除JSON、用新摘要替代原摘要或擅自重新挂到同名文件。

读批注只披露已存内容（包括有权限的原 visibleExcerpt 摘录/归属）；不顺带读隐藏目标、新截屏或重新算context。内部sourcePreimage是定位/验证数据，不能随批注读取整体交付；annotation-read必须构造获权projection，不能序列化整个内部target/preimage记录。交付raw fragment另需独立的准确源范围披露资格；同样约束JSON导出/调试/批量/下载，不借批注权限绕过。当前目标context需独立 source/resource-read。引用只需来源可读和目的Draft可写，不要求来源可写；粘入引文是目的作者内容，不证明原目标仍current。

建议有显式 replacement 与 target basis、pending/accepted/rejected状态。apply输入只指批注 id/current批注basis，不接受调用方偷偷替换已审建议。Core重新读取pending建议、目标当前basis及源读写资格，检查未改范围、准确 sourcePreimage 与已批准的文字变换，生成完整 source after + annotation accepted after 的同一 Plan。用户看准确双效果预览；commit一个opId，在一次逻辑决议中同时改变源与接受状态。任何冲突/撤权/故障未决保pending和稿；恢复依据OperationJournal，不能只成功一个文件就报accepted。并发apply/reject通过同一annotation before generation只有一个赢家；reject只改批注自身，不读隐藏目标。reconfirm重新检查/预览独立操作，模型不能伪造确认或作者时间。undo accepted建议是新的完整反向事务，当前源或批注已变则conflict，不单独回滚状态。

### 9.1. JSON 存讨论；准确源版本与范围定位目标

批注继续在 node-owned `weftext.annotations.json` 中独立保存。正文、回复、作者归属、解决状态和目标历史与 `.typ` 正文分开；目标丢失、目标不可读或目标文件损坏，不是删除想法与讨论的理由。损坏侧文件保留准确原件与诊断，不覆盖空数组；恢复内容未验证时不给建议执行资格。

普通正文批注的身份基础是完整 NodeRef（库 ID + 笔记 ID）、Core 有效 RevisionId、该源版本的准确 UTF-8 半开字节范围。RevisionId 指向准确历史源版本，不授当前读写权。Core 在已提交且获权的 SourceSnapshot 上用官方 AST、最终来源映射及当前选区建立目标；Draft序号、DOM路径、屏幕坐标、排版页号和相同文字都不是源地址。整文、元素、源范围及媒体范围是不同目标类别。

原生 label 对段落导航有用，但不是每条批注必需的块 ID。批注不要求给全库每段自动插入 label。也不能用“label 还在”证明旧源选区/版本没有变过。

### 9.2. 源码字节与可见摘录必须分开

源 preimage 是准确源范围内的原字节，用于验证地址和编辑前像。visible excerpt 是当时用户看到并选择的逻辑文字，用于讨论及静态引用。二者可以不同，不能拿纯文本摘录和源码 slice 作同一个相等检查。

例如长源文件的 `[128,139)` 恰为以下 11 个 UTF-8 字节：

```typst
甲*乙*丙
```

用户看到的是“甲乙丙”，乙带强调，纯文字是 9 个 UTF-8 字节、3 个 Unicode scalar。准确源范围包含两个 `*`，visible excerpt不包含它们。若仅选乙，其唯一普通文本叶可以是 `[132,135)`；若选择完整三个字，source range可包含其间强调语法，必须由当前 AST/map证明该跨度和操作完整性，不将9字节摘录长度当source end。

下面是说明信息分工的 JSON 示意，不冻结新的 wire/持久 schema。UUID与revision只作示意，实际值由Core签发/验证；相同原文与摘要不能制造它们。

```json
{
  "annotationId": "82b289ba-01ae-4c7b-8e51-779442316658",
  "target": {
    "kind": "source_range",
    "node": {
      "workspaceId": "58c47256-bfce-4d29-a037-b83a3a644fb0",
      "nodeId": "12d7c4f3-9ee6-4c36-920d-ec6af27bc654"
    },
    "revisionId": "202173ac-0e45-45cb-8998-7289b1688b0c",
    "sourceRange": {"unit": "utf8-byte", "start": 128, "endExclusive": 139},
    "sourcePreimage": {"utf8": "甲*乙*丙"},
    "visibleExcerpt": {"text": "甲乙丙", "kind": "copied-logical-text"}
  },
  "body": "请解释乙与前后句的关系。",
  "reviewState": "open"
}
```

sourcePreimage可采用Core保留的准确历史slice或足以准确读取它的历史版本引用；若采用摘要，只作为辅助校验，不能独立查找范围、授权或恢复写资格。visibleExcerpt保存在批注中后属于批注已有内容，是否可读由批注权限决定；读取它不重新读取目标正文，不新生成隐藏target的context。完整 raw source若包含不属于摘录的私有内容，也不能因“保存批注”随意扩大给调用方的披露范围。由Core在已获权的目标范围中构造，并遵守SOURCE字段/body隔离。

上述JSON展示的是内部保存/验证数据分工，不是annotation-read响应模板。默认visibleExcerpt从同次Html final tree选区的逻辑文字取；显式print摘录取其Paged最终消费并记录该target/compile来源，原生source范围仍由各自真实map证明。保原摘录不等于缓存完整target原文或私有参数。

私有URI反例（设计交付oracle，未执行产品）：目标source为 `#link("https://private.example/path?token=private")[公开文字]`；内部sourcePreimage包含这段raw，用户保存visibleExcerpt="公开文字"。调用方仅获annotation-read/已存excerpt，无target source或URI披露权。正确response只交获权body、visibleExcerpt及允许的归属/目标状态，不包含sourcePreimage、私有URL、token、raw源码长度/新context；不得把内部target对象整体JSON序列化出去。若用户另有准确raw范围读权，才可通过独立raw fragment操作取得原source，不能因已有excerpt而推该权限。

### 9.3. 跨格式、跨段与生成内容的定位边界

跨strong/emph等原生语法时可以有准确同源范围，但source range可能包含格式定界符，富编辑建议必须由EDITOR验证可安全写入的操作；不能把visibleExcerpt当原生源码替换全部source range。应用建议分别校验准确源preimage和当前实际消费的语义/依赖，保外部字节、原生容器与目标文字含义，完整重解析；source变动和accepted状态仍进入同一个Plan。

选区跨两个段落时，只有两段及中间分隔都属于同一准确作者源、范围连续且AST/map可证，才可作为一个包含段落分隔的源range；visible excerpt独立记录当时逻辑换行。两个不连续源片段不能用其外接矩形range吞掉未选文字。首代不支持复合多range目标时保留批注草稿，提供整文批注或让用户分别选择准确范围，并明确差异；不伪造一个rootrange。

include内容只能指向其真实源所有者与revision，不能拿root的坐标冒充。重复include/函数/循环产生的显示出现项须先区分真实occurrence；即使它们共一个源span，也不能从等span推一个根writer。没有唯一真实来源的generated/multiOrigin文字可整文批注，或由用户明确选择可读真实源；不能将可见生成结果硬塞为根源slice。历史摘录照存，但source-target关系明确 unavailable/ambiguous。需要注释某一生成出现项的新持久目标类型须另设计，本补充不偷偷增加。

字符坐标须明确UTF-8 byte、UTF-16、scalar、grapheme与layout geometry的换算。UI选区不拆combining/ZWJ/CRLF；复制始终逻辑阅读顺序。PDF/image/audio/video批注继续使用准确资源revision、页/像素/区域/timebase；它们不被转成 `.typ` 假文字range。

### 9.4. 重定位依赖真实编辑映射

批注保留原始revision/range/摘录，当前解析另行报告exact/mapped/stale/ambiguous/orphaned/unavailable。Core保存的连续准确编辑map可把未变范围向后映射；在范围前插入文字只移动位置，不需要给文字附永久ID。若覆盖目标、分裂出现歧义、watcher gap、外部编辑无可证连续链、替换文件或历史不可得，保讨论并标待定位，不按quote/hash/同名label搜索猜重锚。

同字节删除后再建不复活旧ReadBasis。当前阅读/建议应用重新获权；已有精准历史版本可读不等于当前可写。人工重新定位显式选current目标、预览before/after，由Core持久化新目标历史；pending建议必须重新确认。普通改批注正文、reply或resolve不偷偷改变target。

### 9.5. 原生段落标签与“复制段落链接”

Typst不自动给每个块永久ID；可以给一个真实block加原生label，然后使用原生link跳转：

```typst
#block[
  这里是一段需要直接引用的说明。
] <para-method>

见 #link(<para-method>)[这段说明]。
```

该label是作者可以改名或删除的导航名称。`@para-method`还受对象可引用/编号规则影响，普通段落链接使用 `#link(<para-method>)[...]`，不承诺自动段落编号。跨笔记使用同一完整ID地址+准确fragment：

```typst
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#para-method")[方法说明]
```

“复制段落链接”操作：

1. 当前获权定位真实段落/block及其来源。若已有准确唯一label且绑定该目标，直接复用，不重命名作者label；只需来源相关读取与地址披露权限，不需源写权。
2. 若没有label，清楚显示该操作将添加原生目标标签。只有源可写，EDITOR能证明该段落可安全wrap为block或可安全标记其既有element时，才可准备准确源修改；必要wrap也属于用户应看到的变化，不能假称只写无影响ID。
3. 选择不占用保留应用角色前缀的普通label，例如 `para-<random-suffix>`；检查准确当前文档及实际依赖的全label集合，唯一性无法证明/生成冲突/源码绑定不明就不给shortcut。它不是新NodeID或全库块注册表。
4. 官方AST/sourcewriter构造、完整重解析与语义比较，证明目标真实、未选字节保全，按一个Draft操作/Undo进入统一prepare/commit。取消不加label；冲突保Draft重新准备。原生wrapper若改变布局/计数器等无法保原段落语义时明确不可用，给源码修复或选择已有label入口。
5. 本次新建label的link只有源已提交并获准解析其current目标后才作为已保存链接交付；未保存Draft链接明确标未生效，可保Draft但不假承诺他人可跳。unknown先查原opId，不能重抽label再提交同一操作。保存成功而clipboard失败分别报告，保留已建label及“重试复制”入口，不为剪贴板错误伪回滚已提交事实。

这项入口只为用户明确选择的段落创建/复用label，不自动给整篇或全库所有段落加ID。只读来源无label时，可以复制静态摘录+整篇来源链接，或用已有可读label；不能为了引用而要求源写权，也不能假造尚未存入源的fragment。

### 9.6. 当前导航、固定历史引用与静态引文

| 形式 | 含义与权限 |
| --- | --- |
| 当前段落link | NodeRef+label解析当前获权对象，改名移动同Node仍可用；label改名/删除、重复或生成绑定变化可失效，不猜近似标签。它不保证原文字没改。 |
| 批注历史target | NodeRef+RevisionId+准确源range表达原批注对象；current mapping是另外经证明的解析结果，原始历史不被覆盖。 |
| 显式固定引用 | 指定准确历史revision/范围，以Core历史读取/引用能力打开；历史不可得/失权明确不可用，不换current。具体外部序列化/可复制历史URI须另定，不能把普通NodeID#label说成已经固定。 |
| 静态引文 | 当前获权来源的可见文字复制到可写目的正文，保来源link和可选已存历史说明。文字是目的作者内容，不随源变化；当前来源link与复制文字可能不同，界面说明复制时间/历史选择。 |

插入静态引文只要求来源可读、目的Draft可写，不要求来源可写。如果引用选择新段落label，则添加label是独立的来源写动作，必须获准；读取或复制不能顺带授予该动作。source与destination不在同一authority时分别展示来源label保存与目的编辑结果，不虚构跨authority事务。

此轮支持设计范围只包含静态摘录、来源导航与已明确的历史批注。实时嵌入/live transclusion需要另论当前读取、缓存/撤权、循环/依赖、来源边界及导出，是尚未定义的能力；本补充不承诺它已完成，也不新增liveembed功能范围。

### 9.7. 本地校对、批量批注与基础文字修订

对应 REQUIREMENTS §5.7；alpha.4 接入下述本地有界实现，限制见 MVP。沿用当前批注、源映射和 Core 写入口，不另建可编辑校对正文。四类规则分别开关：可疑字符／符号修正、词典错别字、成对符号、用户敏感词；规则包含稳定标识、版本、说明和可选准确替换。空词典显示未配置，不声称已检查全部语言；默认规则不把所有不可见字符当错误。符号配对考虑类型、嵌套与语境，不能只比较左右数量。规则来自产品自有配置／用户显式导入，不读取另一项目的文件、进程或服务。

校对读取一次准确正文投影及其源依据，保留可见字符到 UTF-8 原源范围的映射。跨强调的词可以检测，但修改必须使用官方 AST 和可证明的文字写入；跨逻辑段／单元格不拼词，代码、公式、URL、metadata 与批注正文默认排除。generated、多来源和不可回写范围只给只读结果及原因，不伪造 writer。结果绑定文档／草稿版本、规则集和实际覆盖；取消／预算限制为 partial，迟到结果为 stale，不冒无问题的完整检查。

结果面板支持筛选、选择、忽略和一键加入批注。批量请求只选择本次已验证结果，由后端重验源、规则和批注依据并生成同一有界 Plan；unknown 查询原 opId。保存批注前先按 §9 的现有规则明确保存对应源，失败保结果和草稿。相同规则及经真实编辑映射证明的同一出现项复用既有批注，不用摘录文本去重而吞掉不同位置。重复检查／添加不改人工正文、回复或已处理状态；规则变化可提示重新核验，不能自动重开。忽略单个出现项和用户词表白名单分别处理。

校对来源保规则标识／版本、类别、准确检查依据；Word 来源保原件引用、部件及原批注／修订标识、原作者和时间。来源字段不授账号、权限或执行资格。人工／导入／校对记录仍在同一 `weftext.annotations.json`，未知信息保留；持久字段、读取投影、复制、原生包和同步消费者在实现切片内同步验证，不能把原始内部定位数据整体交给批注只读用户。

有明确 replacement 的发现可成为 pending suggestion；敏感词和纯配对异常只有审阅提示。基础文字修订分别表示插入、删除和替换，插入点须独立验证源版本、字符边界及当前编辑映射，不借邻近非空范围伪装。首轮只给可证明的普通文字编辑执行资格；结构／格式／移动、重叠或多个依赖提案保原始信息并标不能自动应用。接受经同一 Core Plan 改源和状态，拒绝只改状态；不会因作者归属、文件中的 accepted 字样或模型输出绕过核验。

语言／文字体系／地区适用条件、词典范围及处理设置版本的 v0.6.1 增强见 §10.3 和 REQUIREMENTS §5.12；现有四类规则可运行不代表这些选择机制已经完成，当前边界仍见 MVP。

### 9.8. Word 修订的正文基线

首轮导入以修订前正文为基线，将尚未接受的基础文字变更保存为独立待审建议；界面同时展示原文和拟议结果，不能静默选择修订后文本并丢掉待审语义。没有修订的正文直接转换。删除使用原文范围和空 replacement，插入使用准确插入点和拟插入文字，替换只在原文件明确关系或转换器证明同一修改时成组，不按相邻／同作者猜配对。

批注位于待插入文字或复杂修订中时保留其与原修订的关系、原件位置和讨论；若尚不能建立准确当前源目标则显示无法定位，不挂到附近文字。修订应用后只有真实转换／编辑映射才能建立新目标。原始文件和未支持内容可恢复，不将此保全结果称为完整修订往返。

### 9.9 对象内批注、Word 映射与退化结果

本节细化 REQUIREMENTS §5.7.1；§9.1–9.4 仍拥有持久目标／重定位规则，EDITOR §2.4 拥有选区交互。当前能力见 MVP；新增公式导入和映射在 v0.6.1，不能因本节改写旧通过记录。普通字符范围、多条独立批注、回复关系与可执行修订分别处理：批注范围相同／包含／交叉不是去重键或回复证据；只有真实 replyTo 形成讨论关系。

**导入。** 从原 DOCX 部件中的准确范围标记、段落／单元格／公式树和文字位置建立到新 Typst 源的转换映射；保存新源的准确版本／UTF-8 范围，保留原件与原标识作为来源。格式 run 拆分不得拆掉同一批注。验证范围内全部对象均转换、顺序一致、边界合法；不能只映射两端而吞掉中间未支持公式／图片／单元格。合并或嵌套表格、编号变化等结构损失与文字批注定位损失分别记录；不确定仍留原件／JSON，不能找相同文字补锚。

**公式。** 有界 OMML 导入将支持的结构转换为可编辑 Typst，并同步产生原数学对象／字符到新源的精确关联；不能先变成普通字符串再凭显示字符重建结构。输出沿实际 Typst／MathML 到 OMML 的转换建立映射，覆盖数学 run、整体数学对象及支持的符号／子式，不能只沿普通 Word 文字 run 查找。数学对象可以有覆盖相同范围的独立批注；同形重复符号靠结构与发生位置区分。对整体公式和声明支持的细选逐方向验收；不支持项保原件及讨论并列原因，不自动声称图片回退保留了可编辑性或字符定位。普通文字建议的执行器不能拿公式源码批注直接当数学修改建议。

**导出。** 冻结正文、批注与本次转换映射，按最终目标准确边界拆分文字 run 并保字符格式；源字节范围、单元格／列表容器、数学元素各自校验。同源多次出现必须区分对应发生项；不能把一个源范围复制成所有显示位置的批注。独立批注、回复及模板既有批注保持各自身份关系，包内重编号不能改线程含义。自动编号符号与列表正文、整格元素与格内字符不得互相替代。

| 结果 | 用户可见含义与处理 |
| --- | --- |
| 准确映射 | 在目标文档中保相应文字／元素范围与讨论关系；字符级与整体元素分别标明，不用“存在一条批注”代替范围核对。 |
| 范围改变 | 细选不能映射而可附到整体时，先显示原／目标范围及原因，用户明确选择才执行；原精细目标与历史仍保留。 |
| 仅普通批注／文字 | 不合格的待审修订可在准确目标上成为普通批注；批注正文富格式未支持时保可读文字并明确格式损失，原始信息仍保留。 |
| 未定位／未输出 | 导入讨论留原件资料／JSON；导出明确该条和受影响回复未进入 DOCX，给出原因及可保留的原生记录／损失报告。不能写成“目标文档已保留全部批注”。 |
| 不支持／需人工处理 | 媒体区域／时间段、未知结构、无来源的生成内容及复杂修订保原件，逐项说明；不自动缩成整附件、整篇或附近文字。 |

基础修订继续按 §9.8：插入为准确点＋待插入文字，删除保修订前文字＋空替换；无关系证明的替换保独立插入／删除。嵌套或结构修订未获资格不自动接受。接受与拒绝后再次导出不能复活旧 pending；普通重叠批注可并存，重叠可执行修订单独判冲突／降级。批注指向待插入或待删除文字时保其原修订关系，接受／拒绝后仅凭真实变换映射重定位，否则保未定位讨论。

§10.2 导入和 §11.3 输出共同消费这些规则。每次交换逐项统计准确、改变范围、普通批注、未定位和未输出，而非只比数量；Word 编辑／保存／重开后的范围和接受／拒绝结果才构成实际宿主证据。WPS、LibreOffice 及外部 DOC／WPS 转换分别验证，不用一个宿主的通过代替另一宿主。

## 10. 模板、复制和文献导入

排版模板用标准 Typst module/import/functions/set/show；包只看到冻结获权inputs、字体/资源，不能从模板函数取得文件/网络、可信id或author write。应用创建模板定义参数类型/默认值和准确title/body/field槽，UI不要求用户手写recipe/wire。缺参数、null与未给参数不同，默认仅未给时生效。文字参数安全literal，不把`#metadata`输入当代码执行。预览包含全部新笔记、资源、typed引用、批注保留/省略以及具体loss；实际和预览绑定同一Plan。

复制算法：枚举完整已获权选择闭包 → 一次分配 fresh笔记/资源/批注/定义key及候选RevisionId映射 → 官方AST与schema遍历所有 typed ref/link/definition槽 → 在旧context解析路径，再以新source位置及同map编码（§7 的 pinned 定义引用保原历史三元组并预览外部依赖，不套 owner/key 新映射） → 验证全部目标/fragment/owner/定义选择 → 冻结全部after和损失 → 原子提交。自由文字、未知字段、CEL string和看似UUID的号码不做字符串替换；未知应用定义的typed引用范围不能证明时完整复制拒绝或显式保留原件/loss方案。两份笔记互引、无Ref定义和同源多实例分别用完整source/result key关联，不能依数组位置join。资源re-own明确，不因路径相同误共用私有附件；批注目标可证明则同map更新，不能证明则保讨论并标stale/明确省略选择。

标签交换沿 CORE §2.3：同库普通笔记复制保原 TagRef；跨库先由目标 Core 分配／选择合格标签，完整源 TagRef 映射统一改写父关系、原生字段及保存 Query 真正绑定的 typed 常量／参数默认值，不做 CEL 字符串替换。预览必要祖先或显式重新挂接、目标既有父关系、匹配模式与预算；不能按显示路径合并或借导入恢复资格。

导入用Weftext自有Rust基础设施：先保原件；安全解析完整parts/sheets/资源及未知数据；形成准确native值和语义结构；对每个不能表达的信息给原始位置、影响、选项；预览；通过同一Core创建。PDF编译成功不证明业务保真，换扩展名不算导入。工作簿公式/hidden sheet/缓存结果分开保留，宏、恶意ZIP/XML和worker额外输出不会执行。大导入可分明确有限组，已提交前缀可查，cancel只停止后续组；unknown查原组opId，不重新导入既有前缀。

论文笔记的 `publication` 记录是唯一可编辑正式文献权威，不另维护一份可独立修改的Bib数据库。导入 BibTeX/CSL/RIS/DOI/provider记录映射为该记录，并保原记录附件和每条字段source mapping；引用用该note identity，Bib/CSL等供Typst引用的数据库是从当前获权文献快照派生的冻结输出。publication.title是正式作品标题，文档title独立；若文档title与作品标题相同，从publication.title生成，不双写。刷新导入只给变化preview，作者改过的字段保留冲突待选，不按provider最新静默覆盖；DOI/ISBN/arXiv是标识不是Node身份，重复导入以明确匹配建议+用户选择处理，不盲upsert。

```typst
#metadata((
  id: "8aa10c08-3e7d-4d10-9488-92bc7f393050", types: ("paper",),
  publication: (
    kind: "article", title: "原生笔记中的结构化数据", citation_key: "chen2026native",
    contributors: (
      (agent: (ref: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654"), credited_as: "陈明", family: "陈", given: "明", roles: ("author",)),
      (agent: (text: "李华"), family: "李", given: "华", roles: ("author",)),
      (agent: (text: "原生数据研究组"), roles: ("author",)),
    ),
    identifiers: ((scheme: "doi", value: "10.0000/example"),),
    issued: (calendar: "gregorian", version: "proleptic-gregorian.v1", precision: "month", year: 2026, month: 9),
    publication_history: (
      (state: "accepted", during: (start: datetime(year: 2026, month: 7, day: 20))),
      (state: "published", during: (start: datetime(year: 2026, month: 9, day: 1))),
    ),
    container: "知识工具研究", volume: "12", issue: "3", pages: "101–118",
    abstract: "一篇用于核对完整链的示例论文。",
    keywords: ("Typst", "结构化笔记"),
  ),
)) <weftext>
#context {
  let record = query(<weftext>).first().value
  title(record.publication.title)
}

= 阅读记录
正文记录问题与讨论；PDF由本笔记资源引用，不造第二个文献对象。
```

这是虚构例，DOI不是有效外部记录，真实positive fixture须换为获权可解析的Person、可选venue(Work|Org)和PDF ResourceRef及已安装paper schema。container文字不是venue ref，不自动建期刊。正式资料使用SOURCE唯一publication形状，正文生成title不可伪造writer；表单只改publication.title一次，EDITOR对generated title给真实字段编辑入口。agent恰(ref)或(text)分支，角色roles保数组；未知作者和组织作者不强建Person。示例作者顺序→表单同顺序→Query同项保序→CSL/Bib作者顺序→引用编号同一冻结数据库→DOCX bibliography各步一致。unknown日/月保precision，不能补1日；未知作者角色/provider字段保原件与诊断，不转缺值。

### 10.1 原生citation key到稳定Work的作者绑定

SOURCE拥有owner唯一的direct原生声明，与root相同官方AST/standard-binding资格，不新增独立citation key身份或另一套Bib事实：

```typst
#metadata((
  kind: "weftext.citations",
  entries: ((key: "chen2026", work: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/8aa10c08-3e7d-4d10-9488-92bc7f393050"),),
)) <weftext-citations>

见 @chen2026；也可 #cite(<chen2026>)。
#bibliography("weftext.citations.bib")
```

key→完整稳定Work NodeRef（含paper）的entries是作者源权威；每key唯一、固定role label在owner准确源中唯一，重复/计算/include/generated或不确定binding不拿首项。publication.citation_key只是新建entry时建议，不反查既有cite key，也不在publication字段变化后自动重绑另一Work。本轮原生citation采用current Work数据，在该次LogicalCut/compile snapshot冻结；不承诺pinned citation，普通key不代表固定历史数据。

生产路径：官方AST资格读取declared entries → Core独立获权解析真实Work及所需publication FieldProjection → 锁同cut SOURCE Type/Value/schema与publication快照 → 校验引用所需title/kind/保序contributors等数据或给精确unknown/loss → 生成只读虚拟资源 `weftext.citations.bib` → 由当前target的官方bibliography/cite求值得到实际citation产品。虚拟资源属于本次World输入slots、无新ResourceRef/birth/作者write；真实同名作者资源与虚拟名冲突就明确报错/换明确选择，不覆盖它。生成物和input key/Work mapping纳入CompileSnapshot依赖/retention，不落第二手工Bib库。读取Work出版字段不要求读取其私人正文或write Work。

原生 `@key` 可能是heading/其他ref，不能因为entries同名就当Paper citation。应用catalog只收已证明standard `cite` 的准确静态key，或官方ref→citation分类及同snapshot输出真正证明的 `@key`；普通@heading只保文内引用。heading label与bibliography key同名冲突按官方诊断不可误关联Work。生成citation可有同compile occurrence导航，但无唯一作者槽不伪writer/稳定author edge。缺/不可读声明或字段保合法native源与应用citation unavailable，不按DOI、标题、publication.citation_key或缓存猜目标。

copy/fork枚举entries.work为typed ref槽，通过同一identity map改到真正复制Work或明确保外部Work；key在其owner内保留，若合并owner引发重复key，须显式新key及所有已证明native cite/ref槽的完整改写Plan，不全源字符串replace。未知生成/变量引用无法证明改写覆盖时明确拒绝/损失，不能漏转。export DOCX/CSL/Bib都消费同一Work快照/有序contributors及该compile真实citation-key mapping；精确源export不暗插Bib，独立Typst bundle需明确包含该次生成Bib/全部依赖或报告不能离线引用。

SOURCE记录的有限0.15.1实验仅证明direct binding样例、`@key`/`#cite`和真实Bib可编译，以及heading/Bib同key官方报错；手供Bib不证明上述Core生成/权限/copy/target输出实现。新增设计oracle：两个Work具有同citation_key也不自动同一对象；只有声明entry选择目标；冷重建仍同Work，改论文citation_key不漂移；撤权不续交生成引用信息；有条件Html/Paged citation差异各target自证；未知@heading/冲突key不得冒Work边。

citation绑定key按SOURCE明确ASCII全文 `[A-Za-z][A-Za-z0-9_-]*` 校验，不从publication.citation_key建议text静默删字/正规化；不合此profile的原生键保原source，可修绑定或明确不用应用桥，不能借清洗碰巧合并两个Work。

期间表单、Query参数、导入、DOCX/XLSX共用SOURCE §3.1.1的原生precision记录和presence/none差别；附表必须保原成员，年/月不自动变日期cell。关系分类同SOURCE §4.1保Field+branch+code/custom原文；换显示语言不改code，导入无码证明不按同名文字猜，不能把preset/custom两项合并。准确结构输出与有序附表支撑往返，只有显示文字的外部文件需用户确认映射并报loss。

### 10.1.1 引用形式、关系分类与输出验收

REQUIREMENTS §5.10 的增强继续使用 SOURCE §1.2 的声明及 publication 唯一数据，不新增持久引用语言、可独立编辑的 Bib 或反向正文。EDITOR §2.5 负责控件，以下定义其语义及消费者边界；当前已有引用错误按原完成门修复，新增体验在 v0.6.1 实现验收；CR01–16 中 CR14 的既有片段解析缺陷保留 v0.6 完成门。

| 用户操作 | 原生表达及边界 |
| --- | --- |
| 普通引用 | `@chen2026` 或 `#cite(<chen2026>)`；既有 @ 不能在未分类时改成文献 cite。 |
| 叙述式、仅作者、仅年份、完整条目 | `#cite(<chen2026>, form: "prose")`；其他分别为 `"author"`、`"year"`、`"full"`。具体显示由实际样式与数据决定。 |
| 页码／章节补充 | 原生 `supplement`，例如 `#cite(<chen2026>, supplement: [第 12 页])`；保内容 AST，不把任意作者 content 拉平成数字。 |
| 仅加入文献表 | `#cite(<chen2026>, form: none)`；与声明中有 key 却未引用、`bibliography(full: true)` 包含未引条目分别处理。 |
| 引用样式 | 原生 bibliography/cite 的 `style` 及 set/show；界面可选编号、作者—年份等实际可用内置样式或获权 CSL。上标效果由样式决定，不直接改源数字或套第二套编号器。 |
| 文内交叉引用 | 原生 label 与 `@label`／`#ref(<label>)`；页码为 `#ref(<label>, form: "page")`，自定义文字为 `#link(<label>)[文字]`。依官方可引用对象、编号及目标输出条件，不据最终字符串猜类型。 |

语法依据为官方 [cite](https://typst.app/docs/reference/model/cite/)、[ref](https://typst.app/docs/reference/model/ref/) 与 [bibliography](https://typst.app/docs/reference/model/bibliography/)。用户手写的合法复杂形式继续保留；应用静态识别范围不是 Typst 语言上限。控件写入须通过当前官方 parser，实际排版和关系识别各按该次编译证明，不能把“语法有效”标成全部产品能力已通过。

关系视图按普通笔记链接、文献引用、文内交叉引用、属性关系筛选，保留 §3 的 ID／当前路径解析区别及独立潜在提及。绑定记录只证明 key→Work 选择，未使用的 entry 只显示“已绑定”；不能计为被正文引用。已证明的作者 cite 与实际编译出现项分层：条件未执行的源码引用不算可见引用；form:none 明示仅文献表；full:true 未引用条目不制造正文边。文献反向引用按稳定来源笔记分组，展开源引用位置及同次显示出现项；重复 include 可产生多个显示位置但不能增加虚假的作者槽或论文数。文内 @heading 留在文内关系，不产生跨笔记边。反向视图可重建，不能反写另一篇正文来维持双链。

引用键与正文标签冲突按官方诊断显示，插入时提前检查可证明的冲突；两个 Work 的相同建议键不合并对象，显式分配可用绑定键。原有 ASCII 绑定 profile 不扩大：其他合法原生键、变量／函数调用、被遮蔽的 cite 及动态 bibliography 不静默正规化。自备 Bib／多个 bibliography 的相同键只有该次实际 bibliography 归属和关联获证才接入对应 Work；缺证保源及 unavailable，不单靠 entries 同名猜边。无法确认完整性的索引显示 partial，旧编译／目标撤权不提供旧标题、数量或跳转；读取目标、定位源、编辑源分别检查权限。

导出固定同次正文、Work 出版字段、引用形式／supplement、style、bibliography 归属及输出目标，顺序和呈现由该次官方结果产生。精确源下载保持原字节；独立 Typst bundle 按该次依赖包含生成 Bib、实际 CSL／资源并验证离开 Weftext 后可编译，缺依赖明确不能独立使用。Weftext URI 仍是合法 link，但外部打开能力单独说明，不冒充普通 Typst 自带笔记库解析；不默改源把应用地址换成猜测文件路径。

DOCX 按实际支持逐项报告可更新的 CITATION／REF／PAGEREF 等域、超链接、固定文字或未转换结果，使用域只在目标数据／书签及语义能可靠表达时启用；此处不承诺所有 CSL／引用形式都成为 Word 原生域。固定文字保当前引用样式与定位补充，同时明确不能随 Word 中修改自动更新；无法保留的形式、页码、编号或文献数据给出损失。不能填假域或仅缓存显示值冒充可更新。流式／Paged／Word 各自分页及编号变化分别说明，Word 页码不偷用 Typst 页码。真实宿主验收必须检查编辑、适用域更新、保存与重开；Word、WPS、LibreOffice 能力分别记录，已有输出／导入范围不得因本节静默缩减。

引用键管理只作用于当前作者／bibliography 归属。可建议 `zhang2026` 一类键，沿现有 ASCII profile 校验，并检查当前可证明的键／标签冲突；同一 Work 有一个合格绑定时复用，有多个时明确选择，不能据建议键合并不同 Work 或静默新增别名。显式改键须先证明完整受影响绑定和原生引用范围、唯一可写槽及权限，再作为一个可撤销编辑与同一 Core 保存提交；动态引用、include／外部 Bib 等不能证明完整范围时，不提供安全改名承诺，保源并说明未覆盖项。禁止全局字符串替换，不改 NodeId 或其它笔记中的独立键。publication 的建议键变化不反写已有绑定；恢复、并发修改和失败沿用同一操作语义。

### 10.1.2 脚注、尾注、题注及编号的原生映射

REQUIREMENTS §5.11 在 v0.6.1 增加有界结构往返，使用同一作者源、Core 与输出计划。脚注／尾注、题注属于作者文档结构，其正文不进入审阅批注 JSON；作用于这些内容的审阅讨论另走 §9.9，不把两种注释混为一类。编号、目录及反向引用是派生结果，不另存一份可编辑权威。

| 结构 | 原生表达与首轮选择 |
| --- | --- |
| 脚注 | 官方 `#footnote[内容]`，需要重复引用时以唯一 label 配合原生引用；由真实 footnote 对象取得正文和关系，不靠上标形状识别。 |
| 文末尾注 | 采用随产品交付、固定版本的原生 Typst 辅助模块，以原生函数、metadata、counter、query/link 组织一份注释正文及文末列表；不宣称标准库内置 endnote，也不全局把 footnote 改成尾注。支持同篇两类注释及各自连续编号。模块的实际调用／内容来源接口在首个切片以 Html、Paged 和独立编译实测后定版，记录到 SOURCE；这是实现准入工作，不是当前已有模块声明。 |
| 图／表题注 | 原生 `figure` 的 `caption`／`kind`／`numbering`；表格由 figure 包含，按真实 kind 分别计数。题注文字、对象本体和目标 label 分开映射，保留题注上下位置等实际支持设置。 |
| 公式编号／描述性题注 | 本体编号沿用 `math.equation(numbering: "(1)")` 与 label/ref。需要描述性题注时可用原生 figure 包装；默认保公式编号而关闭包装的第二编号。用户显式选择题注编号模式时反向关闭本体编号，并明确 ref 指向哪一对象，不从“式(1)”文字猜计数器。 |

依据为官方 [footnote](https://typst.app/docs/reference/model/footnote/)、[figure](https://typst.app/docs/reference/model/figure/) 和 [equation](https://typst.app/docs/reference/math/equation/)；尾注模块使用合法 Typst 语言，不增加独立语法解析器。它的源码／版本及实际依赖随独立 bundle 一并交付，不以联网取包或本机隐藏模块为前置。任意第三方尾注宏、被遮蔽调用和动态 show 仍保合法源，但只有已证明真实绑定、内容来源及本次实际输出的构造进入自动交换；同名函数不算支持证据。首次实现同时证明空注释、多段正文、重复引用、两类注释共存和中文在 Html／Paged 中的真实表现。

内部映射依据是本次获权源与编译中的注释对象／引用位置、figure 类别／题注槽及 equation 编号目标。相同文字、序号和邻近位置不产生对象身份；重复引用保一份注释正文，独立同文注释不合并。label 重命名与复制只修改全部已证明的相应引用槽，缺完整覆盖则保源并明确限制。跨笔记稳定身份仍只用于笔记对象，Word 的局部注释 ID 不冒充 NodeId。

**DOCX 导入。** 解析真实 footnotes/endnotes 关系及部件、正文引用和各部件自己的资源关系，原件完整保留；分隔符／延续分隔符等特殊项不当普通注释。先建局部 ID→注释正文映射，再将引用及内容映射成新原生源，保类型和顺序；缺失、重复 ID、断链及不支持的子内容逐项报告，不能只导入标记而称正文已保留。重复引用按实际引用／域证据重建，不凭相同数字合并。题注读取实际 SEQ 等域、书签和引用关系；仅 Caption 样式、缓存显示值或紧邻图表不足以证明对象绑定，需要准确结构证据或创建预览中用户确认的配对。不求值任意外部／活动域，不把原文件陈旧缓存当新计数结果。

**DOCX 导出。** 在同一固定输出中生成真正的脚注／尾注部件、标记、内容与关系；引用顺序和连续编号可由 Word 正常处理，重复指向使用有实际支持的原生引用／域映射。图表题注保正文、类别及可更新 SEQ，目标书签与 REF／PAGEREF 等引用按支持语义映射；公式本体用 REQUIREMENTS §5.8 的原生数学转换，编号和描述性题注另按准确目标处理。不能只输出预先拼接的“图 1”或静态上标而通过结构验收；支持范围外退为文字／链接时明确结构和自动更新丢失，未转换的注释正文明确列出。分章重启、复杂自定义计数／符号、多级题注、分节尾注及复杂布局分别声明支持，不能从简单连续编号通过推及全部模式。

内容范围先覆盖普通段落、强调、列表、链接及已支持的公式，复用通用转换器；图片／表格等其它子内容只有实际映射后才计为支持，不支持时保原件及损失。注释中的文献引用复用 §10.1 的相同冻结 Work／bibliography 依据。作用于注释／题注的批注和修订若没有跨部件准确映射，按 §9.9／§10.2 保原信息并报告，不挂回主文档附近同文；不能因为脚注正文转换成功而声称其审阅信息也往返成功。

结构成功、内容成功、编号／交叉引用可更新、分页位置与审阅往返分别验收。Word 与 Typst 的分页可不同，移动对象后引用目标不得漂移；两方向测试需在真实 Word 编辑、更新、保存及重开。WPS、LibreOffice 与 DOC／WPS 外部后端按方向和版本单列，不绕过中间格式损失。NC01–10 是专项完成门，结果仍记入现有候选记录；此补充不改写过去验收。

### 10.2. DOCX 正文、批注与修订导入

单元格／列表局部、多条及公式批注的范围完整性、原件保全和退化分类统一遵循 §9.9；有界公式反向转换属于 v0.6.1 待实现增强，不因本节而成为当前能力声明。

脚注、尾注、图表题注及公式编号的 v0.6.1 扩展按 §10.1.2 读取独立部件、内容与引用，不由普通段落／域缓存导入冒充结构支持。

使用产品自有有界 ZIP/XML 读取器，拒绝越界路径、炸弹和外部实体；不执行宏、活动内容或外部链接。保准确上传原件，解析主文档、样式／编号、关系及资源，在新节点生成合法原生 Typst；不能把作者字符串当 Typst 代码执行。首轮可编辑范围以普通段落、标题、强调、列表、普通表格、图片及可证明文字修订为目标；复杂域／布局／对象逐项说明实际处理与损失。

转换同时建立 Word 部件及文字出现项到新源的映射，原始 Word ID 只作为来源。批注保作者／时间／正文／回复与可读取的解决状态；缺失、重复或非法标识保诊断，不凭显示文字合并。新 Node、资源、批注及目标依据由 Core 在同一导入 Plan 分配／绑定；未提交源的预测地址不能当已保存目标。预览显示正文、批注／修订数量、无法定位和不能处理的内容，用户确认准确计划后创建，不覆盖现有笔记，重试查询原操作。

基础修订按 §9.7–9.8 导入；复杂修订的原件保全与支持范围明确区分。带有不支持内容的文件可以显式接受具名损失或拒绝导入，不能把成功解包当无损。来源文档未声明或无从证明的信息显示未知，不补造作者、时间、回复关系和替换关系。

### 10.3. 语言地区、导入优化与校对的适用机制

REQUIREMENTS §5.12 与 EDITOR §8.7 共享以下分工；v0.6.1 实现现有 DOCX／校对链的设置与规则匹配，v0.7 接界面语言及跨端体验。这里不宣称 OCR 或公文／发票专用解析器已实现，也不改变 §10.2 的有界结构支持。

| 对象 | 唯一权威与作用 |
| --- | --- |
| 界面语言 | 个人客户端偏好，仅影响产品文本和明确的显示格式；Server 不替所有成员选语言，不修改作者内容或共享规则。 |
| 文档排版语言／地区 | 库原生样式和单篇／局部作者规则；Engine 提供可证明的有效值及来源。禁用、混合或计算值保持真实状态，不加平行 locale metadata。 |
| 导入／校对默认方案 | 产品内置方案或用户明确保存的个人／库默认。个人选择不暗中写共享库；库级默认／规则经 Core 权限、历史与同步，只描述处理，不重述文档排版权威。 |
| 本次操作的处理设置 | 本次选择、原文标记和已采用默认形成冻结输入，连同方案／词典版本、执行端及损失进入已有 Plan／结果；它是操作依据，不是可独立编辑的正文副本。 |

**选择和匹配。** 本次明确选择优先于可证明原文标记；无选择／标记时，仅可把可解析文档／库默认作为注明来源的回退建议。检测结果独立记录建议与不确定性，不能覆盖显式选择。原文有不同语言区段时保留其范围，处理覆盖按片段判断；统一选项若要覆盖原文标记必须在预览中明确。缺语言／文字体系／地区保持未知，不从脚本、界面、操作系统或 IP 猜国别；不从地区推导任务时区、历法、币种或管辖范围。

每个方案与规则声明稳定标识、版本、说明、语言／文字体系／地区范围、文档类型、输入格式及依赖。匹配区分明确支持、不适用、未知和缺依赖；无地区限制的通用规则可运行，地区专用规则在未知或不匹配时不运行，也不静默换包。用户自有词典／白名单可以明确声明通用或限定范围；旧的未声明规则保留原字节，升级提供范围确认，不能猜归某地区或突然当成全语言。敏感词是用户选定词表的匹配结果，不等于地区法律判定；标点规则须考虑真实语言及上下文，未覆盖片段不能标已通过。

**转换与修改。** 通用结构恢复处理标题／列表／表格等已有能力；地区专用字段提取是单独选择的方案。业务字段保原文和来源，含糊日期、数字、单位或称谓保未解析状态，不因 locale 直接规范化。纠错、简繁转换与翻译不隐含在结构导入中。已知 Word 语言标记按实际范围映射到可表示的原生规则；导出读取冻结编译中有效值，各 Word 语言槽和混合范围分别验证。无法安全表达的标记保原件并报告损失，不因某一个语言代码相同就宣称无损往返。v0.6.2 OCR 的识别语言／模型组合与输出文档语言独立，多语模型、缺模型和检测失败各有明确结果。

**执行和恢复。** prepare 固定原件／源版本、处理设置及来源、规则／词典／模型实际版本和已获权依赖；无相应消费者时不要求虚构模型。改变这些输入使未确认结果失效，确认后的恢复沿用原操作和准确工件，不能用最新词典重跑后冒充原结果。用户可关闭优化，仅按现有能力保原件／资源或作基础导入。原始识别／提取、处理结果与损失可区分；正文编辑与批注映射同一 Plan，处理记录沿用既有结果与历史，不另建日志事实库。

界面偏好不授予数据权限。共享默认、词表、原件、处理结果和工件各按既有 Core 权限／同步／撤权规则处理；执行端只读本次获权输入，词表不能因名称或地区相同替换成他人私有词表。规则不执行作者代码或主动联网；若将来采用云 OCR／翻译，另走既有外发授权，不能由“自动识别”选项取得权限。实现前明确实际客户端偏好与 Core 配置存储合同，并同步 SOURCE／CORE 及受影响消费者；本设计不先虚构持久字段或新协议。

产品文案以 `assets/ui-messages.json` 为唯一目录，当前基础提供 `zh-CN`／`en-US` 的稳定消息键及具名参数。共享消息文本参数限 Unicode scalar 字符串，整数限 JavaScript 可精确表示的安全整数；作者原始 UTF-16 输入、精确业务数字和第三方原始诊断独立保全，不经消息参数规范化。`weftext-messages` 从目录生成原生宿主可用的类型和格式化；`node scripts/generate_ui_messages.mjs` 生成前端类型，现有前端检查／构建拒绝过期生成文件。运行期标签绑定只改产品文字和辅助属性，组件销毁或授权退休时解除绑定；控件身份不使用译文。界面偏好持久化、全部产品消费者和 LC10 各平台场景仍须同批接通验证，目录和组件测试本身不表示界面语言功能或整版验收完成。

PDF／图片文本层提取、OCR、阅读顺序、基础表格和页面／区域来源按 §10.4 在 v0.6.2 实施并验证 LC09，避免已有文本与 OCR 重复写入；OFD／XPS 等格式仍后置。当前资源预览不等于可编辑重建。设置机制先在 v0.6.1 完成 LC01–08，LC10 在 v0.7 验界面和目标平台；文档检查不代表上述产品场景已通过。

### 10.4. v0.6.2 PDF／图片识别与专用导入

REQUIREMENTS §5.13 的本轮实现沿用 §10.3 的语言／地区机制及 Core 原件、资源、Job、Plan，不引入第二作者格式或工作区外的处理服务依赖。按下表划分消费者；原件只读，提取／识别结果是带来源的中间结果，最终可编辑内容为原生 Typst。

| 步骤 | 实际处理及结果 |
| --- | --- |
| 原件与范围 | 获权读取并固定准确文件、所选页／区域和输入格式；报告加密、损坏、超限或不支持项。原件页数与实际处理覆盖分别记录，不能用图片数量充当 PDF 页数。 |
| 文本层／OCR 路由 | PDF 逐页／区域判断可用文本、扫描图及重叠层，生成明确选择；默认优先可用文本层，损坏或可疑层显示原因并允许明确 OCR。混合页面只对所选缺失区域识别，同一区域不叠加重复正文。PNG／JPEG 进入图片识别。 |
| 几何与阅读顺序 | 记录原页坐标、旋转／裁切和识别用图变换；结果必须能准确回到原页，无法证明映射就标 unavailable。单栏及简单双栏保块顺序，页眉页脚过滤只产生可预览建议；复杂布局不按坐标排序后冒充语义完整。 |
| 结构与候选 | 段落、标题和有明确边界的基础规则表格转为候选；表格须核行列和文字覆盖，复杂合并／跨页结构回退为文字或图片并报损失。导入文字经安全转义，不执行识别出的 Typst、脚本或外部链接。 |
| 专用方案 | 使用下述公文／发票适用表处理准确中间结果，每项输出保原始摘录、页／区域或 DOCX 结构来源及规则版本；字段确认后映射到已有适用 schema，不创造未定义字段或一套平行业务数据库。 |
| 确认与提交 | 统一预览来源覆盖、候选变更与损失，冻结用户已采用结果，经 Core 创建新笔记、资源和明确字段。提取文本模式输出可下载文本及来源／损失说明，不伪装已完成结构恢复。部分成功需明确接受；失败／取消不覆盖原件。 |

**识别依赖。** 首个切片评估并固定独立安装的 PDF 读取／渲染、OCR 和可选基础表格后端；记录版本、许可证、模型校验、实际语言组合及执行位置。先交付简体、繁体、英文和中英混合；不把界面语言列表当模型能力。设置页面可检查依赖，离线包或用户明确安装提供模型，识别作业不临时联网找模型。Desktop／自托管服务各用自身已声明环境，纯浏览器仅调用获权执行端，不假设它可调用客户端 Office 或另一工作区。页数、像素、内存、时间与输出预算在支持表中明确，预算耗尽返回准确部分范围。v0.7 验对应平台依赖交付，v0.6.2 已须证明选定初始环境的离线全链。

**公文方案。** 首批作用于中国大陆简体中文，输入是已支持的 DOCX 或经本节提取的 PDF／PNG／JPEG。标题／编号恢复优先用真实 DOCX 样式和编号关系，纯文字模式只给有来源的候选；提供保留、去编号、按所选层级重新编号的独立选择，避免把可见数字与原生列表标记重复输出。机关、文号、日期、主送及其他已声明字段逐项确认，缺失不补；专用方案不更改原文用词或声明满足全部公文规范。不同地区、未知地区及不符合类型的材料按 §10.3 拒绝套用。

**发票方案。** 首个切片冻结首批中国大陆中文电子发票的票种／版式名称、输入和字段清单，并用独立样例证明至少一种票种的 PDF／PNG／JPEG；不能留下仅有方案名的空实现。支持号码、日期、购销方、金额、税额、合计的实际适用项，原文与准确解析值分别保留；号码按文本保前导零，金额用准确十进制，缺值不同于零，日期／币种不能仅凭地区补齐。算术核对只产候选问题，不自动修票。多票同文件必须明确分组供确认；无法分组则报告不支持，不能跨票拼接购销方与金额。没有支持的字段／明细行不承诺结构化，不能由识别结果推定真实性或业务有效性。

**来源和恢复。** 冻结原件摘要／资源身份、页／区域变换、后端／模型／规则版本、实际选项及每页状态。原始识别是可保存的只读证据，用户修正只改变候选／最终作者源；不得把模型重跑结果盖到已编辑笔记。原页媒体批注与新笔记文字批注分别定位，已有批注不猜测跨格式重锚。关联证据与原件读权限同样复核，历史、搜索、导出及缓存撤权复用 Core；日志不泄漏票据正文或私有词表。取消／进程中断保可恢复状态，重试提交沿原操作，重新识别则明确新输入依据和成本。引擎不执行主动内容，外发仍按既有授权边界。

实现前由 SOURCE／CORE 明确实际来源记录、Job 结果、配置与字段映射的存储合同及保留／清理关系，相关消费者同步更新；本次设计不先编造文件格式。产品自有测试样例和检查入口复用 WORKFLOW，不读取外部工作区脚本、配置、服务、模型目录或测试数据。IM01–10 与 LC09 验所声明输入／语言／方案，复杂表格、手写／公式 OCR、OFD／XPS 及更多地区／票种后置，不能把普通资源可打开记为识别通过。

## 11. 可编辑输出与内网下载

### 11.1. Office 模板与样式选择

Office 模板和 Typst 样式模块分别负责各自格式。DOCX 使用标准内容控件的 `w:tag` 定位正文、标题、字段或 Query 结果；XLSX 使用标准命名区域与表格定位字段和结果区域。用户在界面选择绑定，不在模板里另写一套表达式语言。检查模板中的真实位置、容量和支持状态后再生成；缺失或不支持的槽位明确报错，不凭同名文字猜位置。

默认使用模板的段落、字体、边框和表格样式，转换正文仍保留明确的强调等语义。用户可以选择保留可映射的来源样式；无法表示的样式进入损失预览。两种选择都只改变输出工件，不修改作者源。模板的已有内容和未使用部件保留；不能以重新建一份空文件冒充套用原模板。

Query 绑定使用用户实际选中的冻结结果，保持原行序、列序、schema 与完整性状态。容量不足、合并单元格冲突、公式或其他被保护的目标不能静默截断或覆盖。来源字符串不会变成公式，模板中的宏、外部链接与活动内容不会被执行；不支持的模板明确拒绝。生成后的预览、损失确认和下载使用同一份字节。

### 11.2. 转换和交付合同

脚注、尾注、题注及公式编号的结构输出遵循 §10.1.2；普通段落／域可见结果不能代替 NC01–10 的真实结构及更新验收。

中文默认、字体和语言交付遵循 [EDITOR](EDITOR.zh-CN.md) §8.6/CJK01–08：模板、转换和内网 worker 使用同一获权冻结环境，不依赖公共字体 CDN 或操作系统偶然回退。PDF、可编辑 DOCX 与独立 Typst 工程分别验证字体/语言映射、许可和目标端缺依赖；XLSX 中文字符串同样保原值，工作簿字体按目标能力披露。网页预览能显示中文不证明下载工件也正确。

准确source导出只交原 `.typ` bytes，不依赖排版/模板；准确结构结果导出交完整schema、Cell和值/order，不交runtime句柄或writer。PDF/SVG/PNG走真实可用Typst输出，实验能力标实验；DOCX/XLSX是Weftext独立转换器和真实验证路线。

DOCX具名默认profile消费一次冻结Html compile的final语义tree、同target产品及origin/source角色关联；不是从Paged realization推默认flow，也不把Html/Paged heading/link拼成一个“同一快照”。若用户明确选择另一个Paged语义转换profile，完整preview/输出注明target且仅用该次Paged最终数据，能力独立验收。native headings没有显示关联时不能默认插入DOCX幽灵段，需明确source结构输出路线或loss。HTML frame局部Paged内容保持同Html compile的局部来源边界；不能借它宣称整篇Paged语义一致。Html样式/桥的设计已接受；转换profile仍须通过实际产品producer验收门。

DOCX输出真实paragraph、run、style、table、image、关系和引用，不用整页图片冒可编辑。title/副标题保持SOURCE角色，无标题不添作者title。同段heading默认整段 `Heading N` 并参与编号，后部正文用模板正文character属性重置继承，同时保作者显式strong/emph等。DOCX Heading 1–9原生范围内保持层级；10+保源并报告目标不支持或明确loss，不压成9级。普通Word目录可收整段是明确默认；“只含标题文字”是单独可选路线。必须用真实Word/WPS/LibreOffice验证编号、目录和ref，并明确各应用实际支持，不以OOXML能打开代替交互验收。

XLSX输出真实workbook/sheets/typed cells及明确选择的table/query顺序。电话/标识作text；精度超过目标数值能力以准确text或显式loss选择，不能先舍入；civil date/time/local datetime不用主机zone，instant含明确offset或精确text；nested数组/记录默认以可读结构文本和准确附表/结构导出保留，若用户选unnest则保源ordinal和重复。字符串以字符串cell写入，`=...`不会当公式执行；原导入公式只有明确允许的公式slot输出。merge/header/空值/null与缺值区别不能被平铺吞掉。PDF表格不是工作簿。

跨文档离线链接按该输出明确target同次compile的每个RenderedLink occurrence作route：真实目标→唯一输出文件→renderer实际named destination。默认DOCX用其Html语义profile，打印/PDF用其Paged profile；不能拿Html链接catalog验证Paged输出或反之。不猜原label就是Word bookmark/HTML id，不合并重复include项；每项恰一个route或明确loss，未导出目标不能声称离线可跳。route只改工件不改作者源；原生外部URL依目标格式能力保留，不授网络/资源读取权限。

内网路径：选inputs/格式 → Job生成并验证准确工件 → inspect完整输出/loss → confirm同一冻结plan/bytes → download。无需外部发布目录或publish receipt。工件绑定inputs、环境、schema、当前可交付权限、digest/length与输出名；下载逐请求/块/resume重验资格，续下载只交同bytes不重新编译。撤权停止后续交付，过期明确需新生成/新确认，不自动换内容；授权检查先于存在性错误避免泄漏。文件名/ContentType来自受控输出，worker额外文件隔离。生成成功、已确认、已交付、打印成功、另存Resource、外部发布和unknown分别显示；download不写作者事实，Resource保存另走Core事务。

### 11.3. Word 批注与修订输出

单元格／列表文字、结构元素和数学对象分别按 §9.9 的最终映射核验；范围改变或未进入 DOCX 必须逐条披露，普通批注重叠不套用重叠修订的降级条件。

正文版只交所选正文；带批注审阅版按明确筛选输出批注，默认未解决项，已解决／历史项可显式选择。待审建议仍是批注或独立选中的 Word 修订，界面说明两者区别。输出固定同次源／编译、批注版本、所选记录和已存校对结果；不重新跑校对、接受建议、保存草稿或改变作者状态。

通过源范围到最终 DOCX 段落／run 的真实转换映射写入 `word/comments.xml`、关系及范围标记，必要时按准确文字边界拆 run 并保原格式。保作者／时间／正文，回复和解决状态按目标支持写入扩展或给具名损失；原 Word id 不直接复用为当前包唯一 id，避免与模板批注冲突。同源重复输出分别记录发生位置，不按文字搜索重新定位。无法映射的、目标未导出的或已失效批注在损失预览逐项列明，可保附加报告或由用户明确省略，不能悄悄丢失或挂到附近。

选择修订输出时仅将仍 pending、与导出基线准确相符且互不冲突的基础文字建议变成真正的插入／删除标记；接受过的修改进入正文，拒绝过的修改不再标为待审。复杂修订和未定位建议明确报告，不能转成普通正文后声称已保修订。预览说明 Word 接受全部／拒绝全部后的预期文字；在真实宿主验证这两个结果以及批注落点、回复、编辑保存和重开。OOXML 包检查与实际 Word／WPS／LibreOffice 结果分别记录。

### 11.4 导出、导入与校对的用户流程

UX11 在 v0.6.1／v0.6.2 采用同一任务入口：文档菜单、文件／结果上下文菜单和命令搜索先显示目标对象与实际支持格式；不支持的动作说明缺转换器、模板或资格。选择格式后只显示该路线可用的模板、来源样式／编号、语言地区与处理选项，预览正文／资源及逐项损失；默认输出遵循文档规则，不取个人虚拟编号。用户明确选择本地输出位置或获权下载，显示产物名称与资源范围；同名冲突不静默覆盖。普通导出不改原笔记，也不要求用户理解内部能力键或协议。

准备、运行、取消中、已取消、失败、部分成功和状态未确认分别显示。没有可靠百分比时显示当前阶段，不制造进度。每个任务提供自己的取消入口；取消关闭所属参数选择窗并使旧回调失效，不影响其它笔记任务。只关闭任务面板不取消后台作业，重开可按原操作查询状态；不能重复点击生成第二份任务，也不能在状态未知时盲重试。改变参数需要明确的新任务，保留上一任务输入供修改。

完成页列出主产物、关联资源、实际保存／下载状态和损失，提供打开、定位输出、下载及查看详情中该环境真正可用的动作。结果已生成但下载或清理失败时保留成功产物并指出后续失败，重试交付不重跑转换；取消不承诺回滚已经发布的结果。预览与交付来自同一工件，权限变化遵守原交付规则。运行时依赖缺失提示修复路径，具体错误详情按现有脱敏规则提供，不用内部堆栈作普通提示。

反向导入先选真实文件，展示原件与新内容、结构／编号／引用／审阅损失及目标位置，确认后复用现有创建／导入计划；同名时显式选择既有支持的冲突处理，不凭同名覆盖。OCR 加逐页状态、低置信度字段和核对入口，普通 DOCX 导入不为不需要的 OCR 等待模型。校对在现有面板区分扫描状态、问题类别和可应用建议，点击／Enter 定位精确来源；来源变化提示刷新而不套旧位置，阅读与复制不自动应用。整个流程共用 EDITOR §2.8 的键盘、焦点和取消规则，真实 Word／WPS／LibreOffice 结果仍分别验收。

## 12. Agent、automation与外部效果

RQ27 的交付链明确为：获权读取→提出修改→人工审阅/应用；随后支持有限规则对指定库内容自动写入；具体连接器可读取获准来源或生成待确认的准确外部请求。首代外部效果逐请求确认，无人值守外发后置。普通本地保存、编译、DOCX/XLSX 生成和内网下载不要求先实现模型平台、scheduler 或费用服务；实际启用收费/外部适配器时才启用相应 Job 责任。选型不由旧模块名称决定。

Agent 读取与作者写入走同一 Core；模型、文档、网页、邮件及工具输出是数据，不能产生批准。有限委托限定主体、范围、动作、次数/期限与预算；字段授权不升级整篇，包执行无 ambient workspace/network/secret。精确包/字体/provider 准入支持内网离线，不静默联网。

自动作者写先fresh prepare完整效果，匹配用户已授权的有限规则或交互确认；实际效果超界则停等用户，不静默扩大。规则不是模型摘要，批准绑定准确Plan。scheduler有稳定事件/run key，重启不重复已处理发生项，改规则不替换旧unknown Run。stop/revocation在首次不可逆发送/提交前有当前检查与原子fence，先赢则阻止；已发/已提交历史保留，不伪退款或回滚。取消停止尚未开始工作，unknown仍对原请求只读reconcile。

外部mutation先冻结准确account/target/payload与idempotency key，展示完整真实请求并由获准用户确认；宽泛授权、读权限、export确认都不等于发送批准。发送状态 prepared/submitting/succeeded/failed-no-effect/unknown 明确；unknown只有可靠同请求幂等合同或无效果证据才自动重试，不换id再发。凭据轮换不能借新账户复做旧未知操作。外部补偿是新授权效果；Core作者事务与外部mutation分别报告，不能假综合原子receipt。

任何携带库内容、查询片段或敏感派生值的外部请求，包括“只读”的模型/检索请求，都需要独立内容出站授权，限定目的地/账户、数据范围、用途与有效期，发送前重验。具体有限授权可在其范围内复用，不强迫逐次弹窗；读取权限和费用额度不能替代出站授权。外部变更另按上一段逐请求确认，二者不混用。

可能收费的model/tool/network调用开始前以实际账户、币种、pricing版本和有限上限原子预留费用，防并发超支。settled、released-no-charge和uncertain区分；unknown或stop不取消已经可能发生的费用。grant续期不清零历史消费。费用和author效果不是同一回滚状态，UI呈现两者。secret不进入public日志、Query、source或工件；worker只读限定输入/输出slot，无直接workspace挂载或任意命令权限。首代不自动增加白板、独立仪表盘、提醒执行、复杂工作流或实时协作；普通节点组合多个 Query 仍按 §7.2。

## 13. 覆盖、原型风险与设计验收

以下均为未运行的设计oracle，作者草稿不借旧CLOSED、AST实验或编译成功宣称实现接受。

| ID | 正向与关键反例 |
| --- | --- |
| F01 / RQ12–13 | 四形式互转同target/fragment/content、最短路径往返、原样copy断链；重复label/路径越界/撤权/Shared AST不快捷写，保源。 |
| F02 / RQ14 | 白云山/山/山南路、同范围A/B、RTL、overflow和分页全可达；半grapheme只读发现，partial不报全集，不默认目标。 |
| F03 / RQ08–09/15 | 第1号码+第2label不能跨项匹配，重复/顺序/notes保持；numeric/日期/币种不隐式coerce；absent/null/invalid/unavailable与complete empty不同。 |
| F04 / RQ06/15 | 生成深heading可查且按真实level，重复include分不同occurrence；group不借writer，partial汇总不假精确全量。 |
| F05 / RQ16 | current重开变化明确、pinned固定定义+current数据；历史不可得不latest fallback、trash只读、旧row无writer。 |
| F06 / NAV01–05/RQ11/25 | 宽窄屏返回保Draft/筛选/列表anchor；异步预览无焦点抢占；快捷身份/失效、pin筛选和个人偏好符合各边界；字段读不偷正文/count。 |
| F07 / RQ17 | task正文/字段/板历一致，civil due不补zone，周期先查对象并发唯一；generated/aggregate row拒动作。 |
| F08 / RQ18 | PDF/图像/音视频准确坐标、损坏JSON不覆盖、orphan保讨论、hidden目标不新读；建议source+accepted一次逻辑决议，故障和accept/reject竞争保单赢家。 |
| F09 / RQ19/24 | 两笔记互引、同源多实例、无Ref定义、资源/批注同map；未知typed槽/loss漂移不继续；unknown只查原opId。 |
| F10 / RQ10 | 论文作者表单/Query/CSL/引用/Word同顺序，刷新冲突保作者改动，标识不当身份，未知precision/字段保原件。 |
| F11 / RQ04–07/20 | DOCX实际可编辑、run-in正文格式/显式强调正确、Heading1–9与10+loss；真实XLSX不误公式/舍入。Office真实应用验收。 |
| F12 / RQ20/23 | 内网无需publish下载同确认bytes、resume不换内容、撤权停止；离线链接按occurrence完整覆盖，worker多余输出不交。 |
| F13 / RQ21–24/27 | 授权与外部approve/费用/stop/unknown交叉测试；同文档冲突保双方稿、无关文档可继续，跨authority不假原子。 |

实现前重点原型：官方最终语义来源轨迹是否能覆盖全部links/headings且不伪writer；大词典Aho–Corasick与密集重叠可访问性；嵌套数组query同项/空值/完整性及窄字段授权；个人导航状态在撤权/Draft切换下保全；批注多文件原子恢复；可编辑Word混排与真实XLSX精度；撤权过程中chunk交付边界。源/元数据字面投影可不等待整文编译，但所有声明和快捷写必须用官方AST及准确binding证明。无法证明的输入保源/稿并明确能力状态，不能由Features另起字符串parser绕过。

需求分工：本文件直接承担 RQ08–20 与 NAV01–05 的消费者行为；RQ01–07 的语言/编辑/标题由SOURCE/EDITOR提供且输出由本文件接续；RQ21–25 的多surface/存储/授权/事务/上下文由CORE/EDITOR与本文件共同验收。目录/标签稳定快捷身份、唯一Type/Value、definition key出生、Calendar完整schema是明确协调接口，不可用旧协议家族名称代替交付。首代没有新建未部署兼容decoder；发现真实历史资料时另定义显式导入与保原件范围。
