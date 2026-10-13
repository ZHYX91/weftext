---
source_language: zh-CN
translation_status: source
---

[English](FEATURES.md)

# Features：链接、查询、导航、批注与输出

状态：**accepted-design-not-implemented**；固定技术基线的独立全局接受见 [REVIEW](REVIEW.zh-CN.md) §7。本文承接 [REQUIREMENTS](../REQUIREMENTS.zh-CN.md) 的链接、Query、导航、业务类型、批注和输出需求，并保留 Agent、自动化、外部效果与费用边界。共同快照、冻结分页、schema 基线、Calendar 优先级、批注投影、citation 和 Html/Paged 消费者与其它 owner 协调；修订与有限证据见 [REVIEW](REVIEW.zh-CN.md)。设计接受与产品验收分开，本文不宣称实现已完成。

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

## 4. 潜在链接的完整匹配与重叠算法

候选词来自当前可披露笔记标题及 schema 明确标记的 alias/name 搜索字段，排除当前笔记。人员的旧名等可由 name role 派生候选，不双写另一份别名事实。候选字典以 `(target identity, exact term)` 去重；相同标题指向多对象全保留。匹配默认精确 Unicode scalar 字面匹配，不自动拼音、fuzzy、casefold 或正规化。可扩展匹配方式必须显式选择且标明原因。

读取能力与写入能力分开：可准确提取的普通可见逻辑文字可以发现候选，只有唯一源范围且当前可写映射才提供快捷转换；合法 generated/include 内容可以只读发现并标只读来源。代码、公式、URL、真实链接、属性声明和批注正文不自动扫描为普通文字。逻辑片段之间不拼词，软视觉换行仍是同片段，source 换行是否属于同逻辑流由 EDITOR 明确映射，不靠屏幕文字猜测。

具体算法（n 为扫描文本长度，z 为命中数）：

1. 从一次获权词典快照建立 Aho–Corasick trie，终止节点保留所有完整目标，遍历每个逻辑片段，返回全部命中，复杂度 `O(n + z)`；预算不足按片段记录 partial，不保留最长匹配后删其他。
2. 命中为 `(flow, startScalar, endScalar, targets[])`；按完全相同范围分组，目标按完整身份稳定排序，不按标题合并。范围记录实际匹配词和来源。包含和交叉不是去重条件。
3. 另计算 [start,end) 间隔重叠图。`a.start < b.end && b.start < a.end` 才重叠；端点相接可共层。所有命中保留在语义列表中。
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

### 5.1 Core LogicalCut：结果不能由逐对象 current 读取拼成

反例：同一Core事务把 `A.amount=10,B.amount=0` 变成 `A.amount=0,B.amount=10`。先读A旧值再读B新值会算出sum20，这在任何已决状态都不存在；给两项各自ReadBasis或最后核“都曾读过”不能修复。因此Query不是循环调用 `readEntity(current)`。

`LogicalCut` 由CORE产生，包含同一authority的已决journal generation、完整causal heads、entity→准确revision/blob/lifecycle映射、冻结结构事实、schema/rules/settings以及授权audience。Core在一次短commit/read barrier中取得这一已决视图，并与cutHandle同一受保护注册步骤保留必要历史blob/视图。只在注册成功后释放barrier；不能捕获半安装事务。它是本次历史读取快照，不建立第二份可独立更新的current正文/目录权威。

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

Cell与CEL边界具体如下：字段root绑定携SOURCE T以及其Cell状态，state函数 `isValue/isAbsent/isNull/isInvalid/isUnavailable` 是此language profile的具名内建，直接检该Cell不先unwrap。普通值运算/record member/list comprehension只在value态取得T；在非value态使用产生明确 `cell_absent/null/invalid/unavailable` EvaluationIssue，绝不让状态tag作为CEL bool/string或自动false。可选record member保presence，标准 `has(p.label)` 检查对应已授权record键；`has`不读取无权字段。guard使用标准短路 `&&/||`，不会为了判断左侧false而访问右侧值。provider `bool-cell`在value态适配CEL bool，compareTemporal的order-cell在value态适配i64(-1/0/1)，duration-cell按固定record签名适配，其它态转EvaluationIssue，不把整个Cell结构放进 `&&`。例如containsDate未记期间→unavailable，不意味着该员工永久有效。

静态型检查记录declared T/成员presence/可能的read effect，禁止跨T coercion及非法member；运行时依准确Cell保持错误差别。filter产value(bool)才确定成员；unavailable/invalid依赖导致未决行和partial诊断，schema/表达式静态非法是整个query error。明确state test可选择“只看可得值”或诊断行，这是可见作者条件而非后台丢数据；数据库存未决仍不得complete。简单UI只为已知absent/可选member生成明确守卫，不默认把isValue=false的invalid/unavailable排除；用户明确“仅可得值”才生成并展示该状态条件。高级源码不替作者补false。select直接字段仍导出其原Cell状态，不先unwrap再丢状态；aggregate输入含非value必须有作者明确筛除/诊断策略，否则partial或error，不能补0。

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

新定义创建分配一次不随标签改名变化的应用 key，仍置于原生 metadata，label 可编辑；读取按 key 识别，不按同名新对象猜替代。复制产生新的 owner/key 映射，typed 保存定义引用随同一 copy map 更新；引用 current/pinned选择保持。SOURCE 必须将 key 声明、重复检查及字面投影资格与 root 一致设计。高级/计算声明可原生读取保存，但不能伪成应用可执行定义。

| 需求 | 候选交互与状态 |
| --- | --- |
| NAV01 | 导航/列表/正文三面板可收起调宽，窄屏逐层进入返回。会话记录 Query AST、范围、selected identity、scroll anchor + offset、焦点及按 owner 保留的 Draft/Undo。返回先获权重新求值；selected消失显示移出/删除/不可用，不复活旧行。后台预览不聚焦正文或提交 Draft。 |
| NAV02 | 列表可选当前获权摘要、图片/PDF缩略图、明确名称的日期、关键属性与密度。无作者标题以可披露 filename/path 作显示回退并标回退，不补写标题。预览状态是 absent/building/ready/failed/unsupported/unavailable，字段 absent 另标；首行可用即能浏览，不等全库 OCR/缩略图。 |
| NAV03 | 目录、标签、类型及属性值点击生成同一 Query 条件 chip，可逐项移除，包含子目录/全库/正文搜索明确。facet counts 对 current获权相同范围按 `count_distinct(self)`，多值笔记每值最多一次、缺值单独组，partial计数标已知范围，顺序选择不改日期条件。 |
| NAV04 | 快捷入口使用 NodeRef、DirectoryRef、TagRef 或保存 Query key，可重排移除，删除入口不删目标。Node 目录直接用 NodeRef；非 Node 受管目录由 CORE admitDirectory 登记身份后使用 DirectoryRef；标签由 CORE 登记，SOURCE 的 typed TagRef 与导航/Query 一致。显式接纳只影响选定目录，不自动改造全库或 unmanaged 边界。重复身份、观察缺口或目标失效准确提示，不猜同名。仅保留路径的外部书签明确不具改名稳定性。列表置顶仅改变当前合格结果的显示顺序，不越筛选/权限，也不等同快捷入口。 |
| NAV05 | 个人本地 ViewPreference 按目录/标签/Query身份记排序、分组、列、摘要/缩略图开关和密度，可恢复默认。失效字段/target提示，不替同名项；设置不含结果或writer，不改源、真实目录顺序或其他人的偏好。 |

设备偏好存应用配置，不进入业务 metadata。快捷方式、视图偏好和具名布局都是重新解析入口，不能持久化旧授权/结果/writer。列表 pin 可个人按 context 保存 NodeRef 集合，先 query 得当前成员后再稳定分区 pinned/unpinned，各区按显式 Query 排序；导出 Query 数据仍按 Query 原顺序，导出所见列表需显式含该个人排序选择并标其含义。

表/列表/任务清单/看板/日历/时间轴/画廊/指标和既已选图表布局只消费 typed terminal；无 renderer 明确 unavailable。视图不偷偷追加fetch/filter/聚合。AT 可访问完整表格作为等价入口；行虚拟化不冒全量。快速录入包含位置、类型/模板、可选标题、属性和正文；字段可分组/搜索/固定。成功 Receipt 打开真实对象，才清空“再记一条”。unknown 保原 opId/Draft。创建在当前筛选外不是创建失败，完整 current 才能断言不在结果，否则 membership unknown。

任务仍是普通 task 笔记：详情正文、status/due/project等字段共用。勾选、拖看板、改日历日期展示准确 field before/after，prepare重新获权，禁止写聚合/生成行。普通 checklist 是正文项，不自动生成独立 task。周期入口先按明确 series/period/scope 找已有笔记；不存在时只在用户明确创建才写，同 key 并发创建由 Core 决议唯一性。civil 日期、精度、历法和显式时区/规则依赖保持准确；提醒送达、稍后提醒、完成后重复和自然语言日期后置。

## 8. 丰富 schema 的业务保留与查询职责

所有类型支持实际需要的多值、嵌套、时间与来源；集中 schema 定义类型/约束/关系，短名供普通 UI/source使用。常用字段先显示，可选复杂信息按组展开。以下是历史已选能力的重新表示范围，不是继承旧每值 Entry/qualifier/Registry名义版本；SOURCE 拥有完整字段定义，Features拥有查询和视图语义。

| 类型 | 当前保留能力与表示职责 |
| --- | --- |
| Person | 有序 names（角色/语言/文字系统/期间）；重复 phone/email/address/website/account（preset/custom服务、usage、label、note）；profession与制度限定rank、任职（目标普通笔记或显式文本fallback、岗位/部门/期间/状态）；测量（量纲/时间）、国籍/法律性别/性别认同的独立期间断言；生命事件/重要日期与来源/冲突/显式选择；头像资源。parent/guardian/manager/mentor方向明确，spouse/sibling/一般family/social/professional关联保留独立历史和角色。不从姓名/自定义称谓推断方向、血缘或当前有效性。 |
| Organization | 多名称、标识、分类、状态历史、site/address/contact；业务指导/属地管理/共同领导/监管/母子公司/品牌/组织成员、parent/governs/owns及一般关联分别定义，不按目录推组织关系。人员名册派生任职，不双写。 |
| Project/Task | 项目是任务指向的普通笔记，完整Project领域schema仍属候选，不凭名称新增必填字段。task status为required，start/due/priority/project/completed-at/dependency保已选含义；详情正文，完成时间是明确动作产生，不因手改status虚构历史。完整任务状态历史尚非已选，不能自动加入。周期索引/已有对象导航继续保留，复杂HR/提醒不是自动前置。 |
| Work/Paper | 多种作品类型、标识、保序作者/机构与角色、publication状态历史、venue为Work/Org引用、container为独立作者文字；版本关系、引用键及owner-local附件保留。当前未定义独立publisher字段，venue不冒publisher；paper subtype有实际论文所需卷期页码/日期/摘要/主题等。版本取得独立可引用身份时显式新笔记，不按同DOI覆盖。 |
| Calendar | 已选事件区间、来源、参与关系、系列规则/例外/改期/取消和已有周期笔记定位的资料能力保留；日期/时刻、precision/calendar/timezone清楚。提醒执行服务后置不删除已保存作者提醒资料。完整规则schema与provider由SOURCE/协调 owner定义，本文件不把任务截止日自动变事件。 |

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

## 10. 模板、复制和文献导入

排版模板用标准 Typst module/import/functions/set/show；包只看到冻结获权inputs、字体/资源，不能从模板函数取得文件/网络、可信id或author write。应用创建模板定义参数类型/默认值和准确title/body/field槽，UI不要求用户手写recipe/wire。缺参数、null与未给参数不同，默认仅未给时生效。文字参数安全literal，不把`#metadata`输入当代码执行。预览包含全部新笔记、资源、typed引用、批注保留/省略以及具体loss；实际和预览绑定同一Plan。

复制算法：枚举完整已获权选择闭包 → 一次分配 fresh笔记/资源/批注/定义key及候选RevisionId映射 → 官方AST与schema遍历所有 typed ref/link/definition槽 → 在旧context解析路径，再以新source位置及同map编码 → 验证全部目标/fragment/owner/定义选择 → 冻结全部after和损失 → 原子提交。自由文字、未知字段、CEL string和看似UUID的号码不做字符串替换；未知应用定义的typed引用范围不能证明时完整复制拒绝或显式保留原件/loss方案。两份笔记互引、无Ref定义和同源多实例分别用完整source/result key关联，不能依数组位置join。资源re-own明确，不因路径相同误共用私有附件；批注目标可证明则同map更新，不能证明则保讨论并标stale/明确省略选择。

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

## 11. 可编辑输出与内网下载

中文默认、字体和语言交付遵循 [EDITOR](EDITOR.zh-CN.md) §8.6/CJK01–08：模板、转换和内网 worker 使用同一获权冻结环境，不依赖公共字体 CDN 或操作系统偶然回退。PDF、可编辑 DOCX 与独立 Typst 工程分别验证字体/语言映射、许可和目标端缺依赖；XLSX 中文字符串同样保原值，工作簿字体按目标能力披露。网页预览能显示中文不证明下载工件也正确。

准确source导出只交原 `.typ` bytes，不依赖排版/模板；准确结构结果导出交完整schema、Cell和值/order，不交runtime句柄或writer。PDF/SVG/PNG走真实可用Typst输出，实验能力标实验；DOCX/XLSX是Weftext独立转换器和真实验证路线。

DOCX具名默认profile消费一次冻结Html compile的final语义tree、同target产品及origin/source角色关联；不是从Paged realization推默认flow，也不把Html/Paged heading/link拼成一个“同一快照”。若用户明确选择另一个Paged语义转换profile，完整preview/输出注明target且仅用该次Paged最终数据，能力独立验收。native headings没有显示关联时不能默认插入DOCX幽灵段，需明确source结构输出路线或loss。HTML frame局部Paged内容保持同Html compile的局部来源边界；不能借它宣称整篇Paged语义一致。Html样式/桥的设计已接受；转换profile仍须通过实际产品producer验收门。

DOCX输出真实paragraph、run、style、table、image、关系和引用，不用整页图片冒可编辑。title/副标题保持SOURCE角色，无标题不添作者title。同段heading默认整段 `Heading N` 并参与编号，后部正文用模板正文character属性重置继承，同时保作者显式strong/emph等。DOCX Heading 1–9原生范围内保持层级；10+保源并报告目标不支持或明确loss，不压成9级。普通Word目录可收整段是明确默认；“只含标题文字”是单独可选路线。必须用真实Word/WPS/LibreOffice验证编号、目录和ref，并明确各应用实际支持，不以OOXML能打开代替交互验收。

XLSX输出真实workbook/sheets/typed cells及明确选择的table/query顺序。电话/标识作text；精度超过目标数值能力以准确text或显式loss选择，不能先舍入；civil date/time/local datetime不用主机zone，instant含明确offset或精确text；nested数组/记录默认以可读结构文本和准确附表/结构导出保留，若用户选unnest则保源ordinal和重复。字符串以字符串cell写入，`=...`不会当公式执行；原导入公式只有明确允许的公式slot输出。merge/header/空值/null与缺值区别不能被平铺吞掉。PDF表格不是工作簿。

跨文档离线链接按该输出明确target同次compile的每个RenderedLink occurrence作route：真实目标→唯一输出文件→renderer实际named destination。默认DOCX用其Html语义profile，打印/PDF用其Paged profile；不能拿Html链接catalog验证Paged输出或反之。不猜原label就是Word bookmark/HTML id，不合并重复include项；每项恰一个route或明确loss，未导出目标不能声称离线可跳。route只改工件不改作者源；原生外部URL依目标格式能力保留，不授网络/资源读取权限。

内网路径：选inputs/格式 → Job生成并验证准确工件 → inspect完整输出/loss → confirm同一冻结plan/bytes → download。无需外部发布目录或publish receipt。工件绑定inputs、环境、schema、当前可交付权限、digest/length与输出名；下载逐请求/块/resume重验资格，续下载只交同bytes不重新编译。撤权停止后续交付，过期明确需新生成/新确认，不自动换内容；授权检查先于存在性错误避免泄漏。文件名/ContentType来自受控输出，worker额外文件隔离。生成成功、已确认、已交付、打印成功、另存Resource、外部发布和unknown分别显示；download不写作者事实，Resource保存另走Core事务。

## 12. Agent、automation与外部效果

保留已选agent/automation/connector能力的必要授权和结果语义，重新采用当前Plan/Job/OperationJournal体系；不是因为需求表无新编号而删除D10。agent读取/作者写入走相同Core范围，模型、文档、网页、邮件与工具输出都是数据，不能自行产生批准。有限委托明确主体、范围、动作、次数/期限、budget；字段授权不升级整篇、包执行不获ambient workspace/network/secret。精确包/字体/provider版本准入支持内网离线，不静默联网。

自动作者写先fresh prepare完整效果，匹配用户已授权的有限规则或交互确认；实际效果超界则停等用户，不静默扩大。规则不是模型摘要，批准绑定准确Plan。scheduler有稳定事件/run key，重启不重复已处理发生项，改规则不替换旧unknown Run。stop/revocation在首次不可逆发送/提交前有当前检查与原子fence，先赢则阻止；已发/已提交历史保留，不伪退款或回滚。取消停止尚未开始工作，unknown仍对原请求只读reconcile。

外部mutation先冻结准确account/target/payload与idempotency key，展示完整真实请求并由获准用户确认；宽泛授权、读权限、export确认都不等于发送批准。发送状态 prepared/submitting/succeeded/failed-no-effect/unknown 明确；unknown只有可靠同请求幂等合同或无效果证据才自动重试，不换id再发。凭据轮换不能借新账户复做旧未知操作。外部补偿是新授权效果；Core作者事务与外部mutation分别报告，不能假综合原子receipt。

可能收费的model/tool/network调用开始前以实际账户、币种、pricing版本和有限上限原子预留费用，防并发超支。settled、released-no-charge和uncertain区分；unknown或stop不取消已经可能发生的费用。grant续期不清零历史消费。费用和author效果不是同一回滚状态，UI呈现两者。secret不进入public日志、Query、source或工件；worker只读限定输入/输出slot，无直接workspace挂载或任意命令权限。首代不自动增加白板、持久多Query仪表盘、提醒执行、复杂工作流或实时协作。

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
| F13 / RQ21–24/D10 | 授权与外部approve/费用/stop/unknown交叉测试；同文档冲突保双方稿、无关文档可继续，跨authority不假原子。 |

实现前重点原型：官方最终语义来源轨迹是否能覆盖全部links/headings且不伪writer；大词典Aho–Corasick与密集重叠可访问性；嵌套数组query同项/空值/完整性及窄字段授权；个人导航状态在撤权/Draft切换下保全；批注多文件原子恢复；可编辑Word混排与真实XLSX精度；撤权过程中chunk交付边界。源/元数据字面投影可不等待整文编译，但所有声明和快捷写必须用官方AST及准确binding证明。无法证明的输入保源/稿并明确能力状态，不能由Features另起字符串parser绕过。

需求分工：本文件直接承担 RQ08–20 与 NAV01–05 的消费者行为；RQ01–07 的语言/编辑/标题由SOURCE/EDITOR提供且输出由本文件接续；RQ21–25 的多surface/存储/授权/事务/上下文由CORE/EDITOR与本文件共同验收。目录/标签稳定快捷身份、唯一Type/Value、definition key出生、Calendar完整schema是明确协调接口，不可用旧协议家族名称代替交付。首代没有新建未部署兼容decoder；发现真实历史资料时另定义显式导入与保原件范围。
