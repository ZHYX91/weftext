---
source_language: zh-CN
translation_status: source
---

[English](FEATURES.md)

# Features 全新候选：链接、查询、导航、批注与输出

状态：协调后的设计候选，未接受、未实现。本文承接 [REQUIREMENTS](../typst-native/REQUIREMENTS.zh-CN.md) 的链接、Query、导航、业务类型、批注和输出结果，并保留 Agent、自动化、外部效果与费用的必要边界。验证状态见 [REVIEW](REVIEW.zh-CN.md)。

## 1. 少数共用对象与分工

作者事实仍是一份 `.typ` 源和独立 JSON 批注；链接索引、Query 结果、缩略图、反向关系、呈现偏好和导出工件是投影。SOURCE 定义 native 值与集中 schema；EDITOR 定义 Draft、源映射及可安全编辑构造；CORE 定义身份、读资格、逻辑事务与恢复。Features 不建立另一个字段数据库、写入口或权限判定器。

采用共用 `ReadBasis`、`RevisionId`、`Plan`、`OperationJournal`、`Receipt` 和 `CommitManifest`。RevisionId 是 Core 预分配的私有候选名，提交后才有效；字符串或相同字节不证明权限。ReadBasis 绑定真实读取所用 observer incarnation、文件 generation、schema、授权及实际依赖；新一次相同字节读取不复活旧写资格。`SourceSnapshot` 表示已获权的准确源快照，`FieldProjection` 表示只获准披露字段的投影，二者不是互相升级的对象。字段写入可以由 Core 私下定位源并作窄修改，不能为构造完整补丁向字段权限的调用方交付整篇源。

共用调用形状如下；本节消费 [CORE](CORE.zh-CN.md) 的逻辑接口，不是新 wire 版本族。值与schema由[SOURCE](SOURCE.zh-CN.md)、Draft与原生回写由[EDITOR](EDITOR.zh-CN.md)拥有；证据范围由[REVIEW](REVIEW.zh-CN.md)说明：

```text
readEntity(ref, current|revision, purpose) / readScope(scope, purpose, completeness)
  -> SourceSnapshot | FieldProjection | unavailable
compile(sourceSnapshot, environment, target, budget) -> CompileSnapshot
evaluateQuery(definitionSelection, parameters, scope, budget) -> QueryResult
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
RenderedLink = {projectionGeneration, occurrenceOrdinal,
                fragments[], destination, origin: unique_source | generated | multiple}
LinkResolution = resolved(target, fragment?) | missing | fragment_missing
                 | ambiguous | unavailable
```

AuthoredLink 由官方 AST 按源序枚举。RenderedLink 在最终语义投影中每个逻辑 link 一项，换行或分页碎片共一项；同一 include 运行两次即两项，即使 span 和文字相同。两者的关联必须由来源轨迹证明，不能用 span/字符串相等配对。生成项可以导航与导出，无唯一源不能写回或冒稳定作者边。

出链显示真实源与编译链接；入链提供 ID 指向和“当前路径解析”两个明确分组，后者同样可导航。typed 属性引用另列“属性引用”，不用正文假链接表达。路径索引是 current 范围内派生，源或目标移动、对象替换、权限/目录变化即失效并重算。潜在提及独立展示，不形成真链或引用完整性边。partial 不宣称全库没有入链；只有当前获权完整范围才能给完整零结果，不泄漏隐藏标题、路径或数量。没有自动孤立笔记删除，也没有默认全库改写路径链接。

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

基础节点为 `scan(scope,lifecycle)`、`read(fields/products)`、`filter(expr)`、`project(columns)`、`unnest(array,binding)`、`sort(keys)`、`take(limit)`、`union_all`、`distinct(keys)`、`group(keys,aggregates)`、`related(relation)` 与保存 Query 调用。正文/标题/表格/所有实际 heading（含生成 heading、任意深度）由同一官方语义投影读入；生成出现项以 run 内 occurrence 区分，无唯一源则纯只读。基础全文搜索生成显式 `textMatch` 节点，UI 显示是否查正文、目录/子目录/库范围及所有组合条件；本轮不另造查询 DSL。

Query 用 SOURCE 的唯一 Type/Value：text、bool、i64 int、exact decimal、显式 float、civil date/time/local datetime、带 offset instant、ref、array、record 和 schema判别union。money+currency、quantity+unit与精度日期都是该shape系统定义的业务record，不增加未注册scalar codec。数组保序及重复；record 成员按 schema 解释，未知原生成员仍保留为只读值，不猜类型。无隐式 float↔decimal、date↔instant、币种↔币种或单位换算。显式换算需指定准确 provider 与版本，并纳入依赖。电话始终 text。schema 冲突/缺失不能从第一行数据推断 type。

```text
Cell<T> = value(T) | absent | null | invalid(diagnostics) | unavailable(reason)
QueryResult = {handle, runId, definitionIdentity, inputBases, schema,
            coverage: complete | partial(reasons),
            freshness: current | stale(reasons), terminalKind, total?}
```

absent 是作者没提供；null 是 schema 允许的显式 none；invalid 是已读作者值不合定义；unavailable 是 schema、产品或所需依赖不可得。公开 unavailable 原因不能泄漏隐藏对象存在；不可披露对象直接不入公开范围。空字符串、空数组、zero 不等于上述状态。谓词显式测试 `isAbsent/isNull/isInvalid/isUnavailable`，取非 value 的成员报类型/值错误；未知/不可得不能被当 false 偷偷筛掉。若因部分字段不可得不能判断成员资格，run 标 partial 并给已确定行及未决原因；整体语义不合法则返回错误，绝不包装成 complete empty。FieldProjection 只消费可静态隔离的获准声明与 schema，不编译隐藏正文；隐藏值不得影响公开结果的成功/失败/分支/计时诊断。需要隐藏动态计算才能确定字段时给统一不可用，不执行后再以秘密条件决定公开状态。

coverage 与 freshness 是正交的：完整旧投影也可 stale；partial current 可浏览已知行。截断页不是 partial 的代名词，page 带续 cursor；`take(200)` 是作者显式查询限制而非全库统计。complete + current + 无行才显示该范围完整零结果；partial 的 count 是已知下界，aggregate 若全输入未决标 partial/estimated，不冒精确全量。禁止取当前页画全库比例。权限变化使 run/cursor失效，清理缓存的受保护 cells/snippets/counts，再获权重跑；其他依赖变化可显示已标 stale 的仍可读结果，但它们不能写。

结果行的 lineage 仅是来源说明，不是写权。直接 field 行可提出精确 intent，prepare 重新获当前 ReadBasis 和窄写资格；group/distinct/computed 值不继承代表行的 writer。跨已连接库可以 union 获权结果；定义与数据不跨授权域隐式读取；动作不因此获得跨库原子保证。回收站是显式 lifecycle 范围，获权只读，保 tombstone/不可用诊断；结果不授 restore/purge，任何经 trash 调用链输出的 view 动作禁写。

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

建议保存定义是直接应用 metadata 的原生结构，下例是设计 schema 示例，未声称已由实现执行；字段名由 schema 安装后的准确绑定解释，用户可以使用短名，内部不能按显示标签找同名字段。

```typst
#metadata((id: "f8597b15-6b7f-4a6a-9b52-c2732730fe41", types: ())) <weftext>
#metadata((
  kind: "weftext.query", key: "6b2b9e19-5599-48c3-b27b-6b006ec9c397",
  scope: (library: "current", lifecycle: "live"),
  types: ("person",),
  where: "phones.exists(p, p.label == 'work' && p.number.startsWith('138'))",
  select: (
    (name: "person", expr: "self"),
    (name: "contacts", expr: "phones.filter(p, p.label == 'work')"),
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
| 未完成任务 | `types has task && status == 'open' && project == param.project`；civil due 排序不补本机时区。 |
| 日历任务 | due is date 时显式投影 `[due,nextCivilDay(due))` 表示截止日卡片；不是新增事件事实或提醒。instant due 不猜结束时间。 |
| 稿件阶段历史 | `publication.publication_history.filter(s,containsDate(s.during,param.day))` 可返回多个冲突断言；不默认最后一条胜出。 |
| 深标题 | `read(headings); filter(level >= 6); project(level,text,occurrence)` 包含生成项；10+ 级仍按真实级别读。 |
| 金额汇总 | `group(currency,sum(amount))` 每币种独立，不混币；无有效金额且有invalid者不能报零总额。 |

相等用唯一递归 Type/Value 规则：text 精确；exact number 按数学值（保作者表示另存于源）；float 有限数且 signed zero 数值相等/原位值保全；instant 按精确 UTC 时刻；ref 完整身份；数组顺序敏感；record 键顺序无意义、数组内顺序有意义；tagged union 按分支+值。group/distinct 比完整相等键，hash 碰撞再比较完整值；选代表时用稳定完整值序，不按当前页第一行。它们产生新 group 行而不继承来源 writer。排序对复杂值必须显式投影 scalar，空值位置明确。

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
| Work/Paper | 多种作品类型、标识、保序作者/机构与角色、publication状态历史、venue/publisher分别表达、版本关系、引用键及owner-local附件；paper subtype有实际论文所需卷期页码/日期/摘要/主题等。版本取得独立可引用身份时显式新笔记，不按同DOI覆盖。 |
| Calendar | 已选事件区间、来源、参与关系、系列规则/例外/改期/取消和已有周期笔记定位的资料能力保留；日期/时刻、precision/calendar/timezone清楚。提醒执行服务后置不删除已保存作者提醒资料。完整规则schema与provider由SOURCE/协调 owner定义，本文件不把任务截止日自动变事件。 |

每条历史记录可选 validity、status/selection、note 和 source 信息，按领域放在同一原生记录；不另包一份通用事实信封。来源既可为原始附件定位也可为引用，不授其读取权。重复相同值/重叠期间允许保留，查询的“当前”必须显式期间/选择规则，不默认为最新写入。多个 preferred 造成歧义，界面显示并要求选择，不最大置信度猜胜出。

关系是 schema明确 typed ref槽与方向，事实只在一个声明所有者存一次，反向视图派生。symmetric关联也从一个明确作者所有者存储，不自动改另一个端点；从任一端打开编辑先获权定位该作者源，不持有源写权则只读。反向行保准确 `(sourceOwner,field,itemPath,ReadBasis)`，准备时重验，不直接写显示端点。已有另一独立断言保留其来源，不静默归并成同一事实；graph可合为一条显示边但展示多个assertion来源，不能拿边去写未知来源。人物text fallback无真实ref则不lookup/inverse/自动建人；目标删除后保作者事实并标suspended/dangling，cleanup须显式获权Plan，不自动跨源删关系。人员任职允许普通目标，不强迫先升级组织；Org其余typed关系按schema验证域。parent→child、guardian→ward、manager→direct-report、mentor→mentee和engagement→has-engagement逆向明确；general family/social/professional关系的preset码域及custom原文保持，custom不猜方向。colleague只按需从重叠engagement派生，不存O(n²)副本。业务图的cycle/rank约束仅在schema确有规定时验证，不为自由关联统一造家谱/组织树。

### 8.1 周期、区间与事件的真实消费者算法

Calendar有三个已选资料类型：period-note、range-note、event；day/week/month/quarter/year是period种类，不是另五种身份。period字段明确series、calendar/rule、period key、zone及unique/many scope；路径/标题不定义key。period导航先以同一Query查准确 `(series,key,scope)`，partial不能宣布不存在；unique创建需Core同Plan验证完整负范围及并发约束，many允许多个独立笔记。

range字段为 `(start,end_exclusive)`，date或instant半开区间，至少一端有界，开放端用明确缺端不填现在/最大日期。同型有界端点严格start<end；SOURCE的 `bounded_half_open_range` 具名约束负责本range准入。窗口重叠用 `start < window.end_exclusive && window.start < end_exclusive`，开放端按无界处理。`during(start,end)`是独立有效期间，不混作半开事件range；containsDate使用其明确闭合区间规则和未知状态。duration只派生，未定provider不伪造秒数；date单位/precision与instant精确elapsed seconds不同。event复用range，增加status、参与者及作者reminder-intent，不复制另一份时间事实；参与者Person/Org关系可反向派生，提醒意图资料保留而送达后置。

recurrence按冻结calendar/zone/provider及source anchor的civil phase生成daily/weekly/monthly/yearly候选，应用interval与显式byMonth/byMonthDay/byWeekday/weekStart，count或inclusive until限制；不存在民用日期跳过，DST gap/fold按明确规则处理，不能host默认。合并rDates后按原发生键去重，排除exDates，应用同原发生键的cancel/replace；replace可改range/title/status/note，不把改期终点变原发生身份。计算窗口结果同时枚举正常窗口候选和全部可能从窗口外移入的有限explicit replacements，再按实际range过滤；预算不足标partial，不漏移入却说complete。没有默认每次发生建Node；显式promote/adopt才创建。改series时预览已有exceptions的keep/remap/drop，不能静默丢旧例外。per-occurrence参与者/提醒覆盖若schema未选，保导入原件并报告unsupported，不冒全RFC兼容。

pack提供历法/节假日上下文，结果保pack/version/source/applicability，多pack并存，不写一个isHoliday权威。ICS UID/RECURRENCE-ID/SEQUENCE是来源标识不等EntityRef；VFREEBUSY/VTIMEZONE不自动Node，VTODO/VJOURNAL显式preview映射。Period、Range、Event完整native schema归SOURCE，以上算法是Features消费者实际义务，不能用“保留旧服务”替代。

这里没有要求常用信息填所有复杂字段，也没有禁止确需的逐项身份。schema与关系迁移必须显式 preview 原件、受影响字段和未知值；未知字段保源并只读展现，不能先删除后称精简。

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

读批注只披露已存内容（包括有权限的原 visibleExcerpt 摘录/归属）；不顺带读隐藏目标、新截屏或重新算context。当前目标context需独立 source/resource-read。引用只需来源可读和目的Draft可写，不要求来源可写；粘入引文是目的作者内容，不证明原目标仍current。

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
    issued: (calendar: "gregorian", precision: "month", year: 2026, month: 9),
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

## 11. 可编辑输出与内网下载

准确source导出只交原 `.typ` bytes，不依赖排版/模板；准确结构结果导出交完整schema、Cell和值/order，不交runtime句柄或writer。PDF/SVG/PNG走真实可用Typst输出，实验能力标实验；DOCX/XLSX是Weftext独立转换器和真实验证路线。

DOCX输出真实paragraph、run、style、table、image、关系和引用，不用整页图片冒可编辑。title/副标题保持SOURCE角色，无标题不添作者title。同段heading默认整段 `Heading N` 并参与编号，后部正文用模板正文character属性重置继承，同时保作者显式strong/emph等。DOCX Heading 1–9原生范围内保持层级；10+保源并报告目标不支持或明确loss，不压成9级。普通Word目录可收整段是明确默认；“只含标题文字”是单独可选路线。必须用真实Word/WPS/LibreOffice验证编号、目录和ref，并明确各应用实际支持，不以OOXML能打开代替交互验收。

XLSX输出真实workbook/sheets/typed cells及明确选择的table/query顺序。电话/标识作text；精度超过目标数值能力以准确text或显式loss选择，不能先舍入；civil date/time/local datetime不用主机zone，instant含明确offset或精确text；nested数组/记录默认以可读结构文本和准确附表/结构导出保留，若用户选unnest则保源ordinal和重复。字符串以字符串cell写入，`=...`不会当公式执行；原导入公式只有明确允许的公式slot输出。merge/header/空值/null与缺值区别不能被平铺吞掉。PDF表格不是工作簿。

跨文档离线链接按冻结语义投影每个RenderedLink occurrence作route：真实目标→唯一输出文件→renderer实际named destination。不猜原label就是Word bookmark/HTML id，不合并重复include项；每项恰一个route或明确loss，未导出目标不能声称离线可跳。route只改工件不改作者源；原生外部URL依目标格式能力保留，不授网络/资源读取权限。

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
