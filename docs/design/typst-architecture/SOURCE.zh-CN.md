---
source_language: zh-CN
translation_status: source
---

[English](SOURCE.md)

# Typst 作者源与集中 schema：重设计候选

状态：设计候选，未实现、未接受。需求依据为 [REQUIREMENTS](../typst-native/REQUIREMENTS.zh-CN.md)，验证状态见 [REVIEW](REVIEW.zh-CN.md)。本文定义作者源、应用角色与唯一字段类型体系；代表 schema 和样例不是全部业务能力的上限。

## 1. 作者源与角色

一篇笔记有一份 `.typ` 作者源。稳定笔记 ID、集中 schema、独立批注、资源及真实目录的权威由协调 Core 设计确定。本文只定义源角色、值语义及消费边界；不复用旧 Pin、Proof、Entry、carrier 或 SourceRevisionPlan。

标准 `#metadata((id: "...", types: (...), ...)) <weftext>` 携带作者属性。`id` 是 Core 分配并在成功创建事务后承认的 lowercase UUIDv4；它不是权限。改名/移动保持 ID，复制/作为新笔记导入取得新 ID，外部改变 ID 是待处理身份冲突。主源路径来自 Core 目录权威，metadata 不重复存 path/parent/children。缺 ID/部分同步不自动造 ID。

官方 Rust `typst-syntax` 是唯一 parser。AST 检查应用角色；不做正则头部解析、第二 lexer、YAML/JSON 嵌入方言。保留准确 UTF-8 字节、BOM、注释、换行、字面量拼写及未知值。根源、依赖文件和编译输出有各自版本；输出不能成为新作者权威。

根角色按以下顺序识别：

1. 解析完整准确源。直接根 markup 层的裸 metadata 调用，恰一个 positional dictionary 参数，后跟精确 `<weftext>`；中间只允许空白或注释。不能套 context/function/loop/conditional/include/show。根声明不强制第一行。
2. 证明该位置裸名称仍是固定官方标准库 `metadata`。实现从根 AST 检查此前作用域：无前置 binding/assignment/import/include/show/context/任意函数调用；允许 trivia、无嵌入代码的纯 markup，以及明确静态的标准 title/set（参数树没有用户代码或 binding）。不确定前缀时根角色不可认证，保留合法源。默认创建把声明放在第一个代码表达式，以便简单而可证明。拓展安全前缀须另加 AST 反例和官方语义实验，不能执行前缀猜结果。
3. 完整根 AST 的实际 Label 节点中 `<weftext>` 恰一个。未执行分支或函数体里的第二标签同样是歧义。字符串、注释、raw 中相同文本不是 Label。后续遮蔽 `metadata` 不改已识别声明；include/动态产生的同名元素不是根声明，也不以编译 query 认证身份。
4. 根 dictionary 的直接 named/keyed 项，key 用官方 AST 解码且精确唯一。`id:` 与 `"id":` 重复也拒绝；不允许 spread/dynamic key。`id` 是 direct string；`types` 缺席表示空类型集合，存在则 direct string array，(), `("task",)` 分别为空和单元素。计算的 id/types 不能认证。
5. ID/types 是控制键；其余业务键由 schema 解释。大小写和 Unicode 规范化不自动合并；保留 `weftext-` 前缀。语法破损导致无法排除第二声明时结果为不可判断，不能“首个获胜”。

合法 Typst 不符合受管根角色，仍可作为原文打开和保存；已有笔记进入保稿/明确修复状态，不被索引器重造身份。完整语言与可编辑字段子集分开。普通正文可以 import、函数、计算、复杂表格；源保存、控制认证、字段校验和编译分别报告结果。

文档 title/subtitle/heading 的角色由编辑器协调设计拥有。name、论文正式标题和文档 title 各有明确语义；文件名只是显示回退。若论文正式标题即用于文档 title，可从唯一 publication 字段生成 title，表单只修改这个字段，不要求双写。生成 title 没有伪造的源码可写地址。

### 1.1 保存 Query 的直接声明角色

保存 Query 使用同一官方 AST、标准metadata binding证明及静态原生值解码，角色为根markup层直接 `#metadata((kind: "weftext.query", key: "...", scope: ..., types: ..., where: ..., select: ..., order: ...)) <weftext-query-work-phones>`。FEATURES拥有scope/types/where/select/order及其他查询字段的闭合定义；SOURCE不重复存CEL/Query AST。kind和key必须direct Str，key为Core在明确新建定义事务中一次分配的lowercaseUUIDv4，完整身份是owner+key，不是另一笔记ID，也不授任何权限。已有裸外部key须经Core admission，不能仅看字符串接受出生。

直接metadata调用的形状、dict精确key唯一、无spread/dynamic key、完整源可判定以及binding证明与§1一致；查询数据树必须全部属于§2静态域。其前缀除了§1安全集，还允许已认证且所有参数值树均静态的标准metadata声明（包括根weftext声明及其它query），不能把computed根值执行后当安全前缀。参数树中任何用户调用/binding使本次定义资格不可用，保原文不执行查询。

定义必须接一个精确原生Label，标签保留`weftext-query-`前缀且完整值在全根实际Label AST中唯一；后缀可编辑，key保持不变。所有直接query角色的key在同owner准确根源中精确唯一；重复即歧义，不last-wins。nested/include/generated metadata、动态label、字符串中的伪声明不获得key身份。已识别的直接key也不能覆盖不完整源中尚未排除的第二声明。缺label/重复key/遮蔽/计算声明可保合法源码，但不成为应用可执行保存定义。复制按Core冻结owner/key映射分配新key，实际保存的定义引用同事务更新。读取current/pinned、参数/求值/导航语义由FEATURES拥有；任何历史定义都不复活writer。

## 2. 原生值和准确语义

可静态编辑的值递归包含 string、bool、none、官方 i64 int、有限 binary64 float、官方 decimal、标准 datetime、整数秒 duration、array 和 dictionary。字典 key 精确唯一、作者顺序保全；数组有序、保重复，单值没有 UUID 封装。`(value)` 不当单元素 array。标准构造器只接受已证明的标准 binding 和常量参数，不运行用户函数。

| schema 语义 | 作者表示和消费规则 |
| --- | --- |
| text / enum / phone / email / identifier / URL | 原生 string；语义 codec 明确声明，不 trim/NFC/大小写改写电话与标识，不把电话转数。enum 未知值是错误或具名开放枚举状态，不能悄悄删除。 |
| int / float | int 为官方 i64；i64MIN writer 用 `int("-9223372036854775808")` 并官方回读；bool、int、float 不强转。float 保留准确位型及负零，禁止非有限值。不用于金额。 |
| decimal / exact_decimal | 优先 `decimal("0.30")`，保值、scale 与原字节；超过官方范围时 schema 明确用原生 canonical decimal string，比较以精确十进制系数/指数实现，不经 float。表示分支写在 schema，不读值自动猜。 |
| quantity / money / range | 普通字典：精确 amount/value，显式 unit/currency 与可选精度/来源；不把无单位数字自动赋成本币。比较/换算须有具名单位服务，不隐式换算。 |
| date / time / local_datetime | 官方 `datetime(...)` 对应日、时、民用本地日期时间；缺时区不补设备时区，不当 instant。实际日期经官方 constructor 校验。 |
| instant | schema 声明 RFC3339 string，必须显式偏移或 Z，精确小数秒；禁止 unknown offset `-00:00`、闰秒和无偏移。保原偏移表达，排序归一到精确 UTC 值。 |
| partial_date / temporal_range | 原生记录，显式 calendar/precision；年、年月、月日和起止范围不伪造缺失部分。生日未知年份用 `(month: 5, day: 12)`；范围省略端点表示未记载，none 只在显式 nullable 时代表未知。区间开闭、granularity 由 schema 声明。 |
| zoned_civil / recurrence | 原生记录含 local_datetime、IANA zone、可选 disambiguation；时区数据库/版本、重复规则与 DST 歧义处理由时间服务提供。规则字典不是另一个可执行作者语言，不把月/年转换成整数秒。 |
| node_ref / target_or_text | schema 指定的原生字符串或记录；只在该位置解码引用，不扫描像 URI 的普通文本。见 §4。 |
| list / record / union | 原生数组和字典。union 必须原生域不相交，或有固定 discriminator 成员；禁止 first-match。未知字典成员保原文且单独标 unbound，不能假装已校验或使相邻已知成员消失。 |

absence、none、""、`()`, (:)、0 和 false 不同。必填约束检查 presence；nullable 仅控制出现后是否允许 none。默认值只在明确新建/模板时写成真实源，不在读取既有笔记时补值。来源、时段、状态历史是对应类型的可选嵌套业务记录；不统一强迫每个值附置信度或通用事实账本。

静态读取不需要编译全文：字段里的直接数组/字典完整递归解析，复杂/计算表达式标 computed。有一项无法投影时完整该 Field 不可作为截断的成功列表；相邻其他 Field 独立可用。未知成员保全，不授 typed action。计算表达式合法，不自动物化/覆盖；编译得到的语义可提供只读投影，绑定准确获权快照和全部实际依赖，不能直接作为作者字段写回。Query 是否纳入某个计算字段必须由明确的只读计算生产者声明，不能以 metadata query 最终值偷偷变成作者权威。

## 3. 集中 schema 的最小可实施模型

schema 是 Core 管理的可移植非执行配置；采用标准 JSON 配置数据是实现选择，不是 `.typ` 新作者语言。Schema revision 由 Core 的配置版本/事务管理；定义内容指纹、实际安装来源与授权随读取冻结，不重造 predecessor-proof 树。源码不写内部 FieldId，也不在每篇笔记复制 schema。

闭合 Schema 对象包含 `version`, `definitions`, `fields`, `types`, `aliases`；这个最小schema不含标签/UI提示成员；UI显示key，额外显示配置须有独立闭合定义且不能影响业务匹配。未知 schema 结构/版本拒安装；未知**作者数据**保全。`definitions` 是有限无循环命名值形状；named refs 仅配置复用，不执行源码。

Shape 是如下闭合配置代数：

```text
Scalar {kind:"scalar", semantic:明确codec名}
Enum {kind:"enum", values:[string...], open:false|true}
List {kind:"list", items:Shape, min:非负整数, max:整数|null}
Record {kind:"record", members:{key:{shape:Shape, required:bool, nullable:bool}}, unknown:"preserve", constraints?:[Constraint...]}
Union {kind:"union", variants:[Shape...], discriminator:null|{kind:"member",keys:[string...]}|{kind:"tag",key:string}}
Use {kind:"use", name:definitions中已定义名称}
```

每个 Field 有永久语义标识 `id`、精确作者 `key`、shape、nullable，以及显式 creation default（恰 `{kind:"absent"}` 或 `{kind:"native",source:native-constant-expression-string}`；安装时经官方受限解码/同schema校验）。每个 Type 有 `id`、`fields:[{id,required}]`、`requires`、`conflicts`。Aliases 精确映到 TypeId。member discriminator要求所列分支成员恰一个出现，所以ref/text同时出现不能利用unknown preserve穿过两个分支；tag discriminator要求一个固定枚举标签与对应分支精确一致。无discriminator时各分支必须原生域不相交。跨笔记 target type/唯一键/环检测由具名关系或导入操作验证，不藏进 arbitrary expression。

本版闭合 builtin codec 名单为 `text, nonempty_text, phone, email, identifier, doi, url, language_tag, node_ref, resource_ref, tag_ref, bool, int, float, decimal, exact_decimal, duration, exact_seconds, date, time, local_datetime, instant`。文本codec按指定业务格式处理；phone/email保存非空作者文本、保原拼写，格式建议不作为“号码可拨/邮箱存在”证明；DOI/URL/语言tag的语法校验使用固定公开标准，不发网络请求。node/resource ref由Core解析。其余原生值/精确字符串规则见§2。unknown codec拒安装，不临时把它当text。closed enum的非枚举值invalid；open enum的未知string保为有效扩展值，并标unregistered，Query可按其字面过滤/分组，专门状态动作不得猜其含义。none只受nullable允许。

所有形状对象的成员恰为上表；仅Record可省略constraints，省略等于[]。Constraint闭合为 `{name,reads}`；本最小版恰 `valid_gregorian_partial_date`（reads恰precision/year/month，Gregorian year范围1..9999，year精度不出现month，month精度必填1..12）、`valid_month_day`（reads恰month/day，允许2月29日）、`same_temporal_domain`（reads恰start/end）、`start_not_after_end`（reads恰start/end）、`bounded_half_open_range`（reads恰start/end_exclusive）。有效期during(start,end)使用前两种期间约束且边界为闭合，缺端允许、有端点依shape验证、两个端点必须同date或同instant且start≤end；calendar range(start,end_exclusive)只使用最后一种约束，恰date或instant同域、至少一端有界、有两端则start<end，语义半开。fixture含各自独立Record，不套错成员名。每种constraint的成员类型在安装时验证，未知name/额外成员拒绝。list的min/max直接提供长度约束；scalar语义域由codec提供。其他范围、互斥、计量或时间约束须另注册完整具名codec/constraint后才能供schema安装，不用散文扩开闭合对象。

安装检查：重复 ID/key/alias、控制键注册、未定义 use/Field/Type、shape 循环、union 重叠、类型 requires 循环、非对称 conflicts、enum/default不匹配、约束读取不存在成员和预算超限均拒绝；无“安装顺序优先”。没有安装 schema，`task` 字符串不会自动下载或创建 seed。

多个 Type 无优先级地求 requires 闭包、合并 Field 集合，required 取 OR。同一 FieldId 只对应一份定义和一个作者 key；不同 FieldId 不能争同一 key。类型组合冲突明确诊断，不能覆盖、缩窄成交集或把类型顺序当优先级。unknown Type 保其 token，已知不相交字段仍可读；完整类型校验为 unavailable。unknown 不是空类型。alias 与全名展开到同 Type 的重复声明应诊断。task/person/organization/paper/project 等短名来自集中 aliases，不让用户手写内部命名空间。

保存的 Query 绑定语义 Field/Type 标识；显示名称或短名变化不重新解析为另一个对象。schema 后继按获权显式配置事务保存旧定义版本；兼容增加可选字段/record成员可以保 ID，影响既有源的 required/类型/含义/key变化必须有准确影响预览和明确迁移，或新语义 ID。删除仍有消费者的定义、alias重绑和复用已废弃 ID 不静默完成。这里不要求“所有 type字段集永远不可变”才能增加功能，也不据未部署旧设计虚构 ledger/decoder 兼容义务。

## 4. 关系、文字回退与来源

`target_or_text` 是判别明确的原生 record union：`(ref: "weftext-node://WorkspaceId/NodeId")` 或 `(text: "云山公司")`。两者恰一分支；ref 分支不加文字 label 的同步副本。作者确有“当时名称/出版名称”需求时字段另命名 credited_as 或 `name_at_time`，语义是历史作者文字，不能被目标改名覆盖。显示 ref 时读取当前获权目标；目标不可读/失效时给通用状态，不借本地旧标题泄漏。text 分支保留作者文字且不按同名自动升级关系。显式选择目标才将 text 转 ref。

引用 URI 提供稳定笔记关系；不是任意磁盘权限，也不证明目标存在。target 的 type 要求是否强制是该关系 schema 的显式约束；不能所有 node_ref 都默认指向相同类型。删除/回收不自动抹关系，跨库复制显式重映实际复制的目标，外部目标保原引用/明确失效。

关系定义最少包含：稳定关系 ID、源 Type/Field、嵌套 target 路径、目标类型约束、direction（directed/symmetric）、可选派生反向显示名、完整性约束。任职源在 person.engagements，组织“任职人员”是获权 Query 的派生结果，不在 organization 再存 members。组织 parent 在 organization.parent；部门 child 和递归层级派生，不能等同目录父级。任务 dependencies 和论文 contributors 同样只有一个作者方向。对称关系每个assertion只在一个明确作者owner中保存，不自动双写另端；允许不同作者独立assertions保各自来源，不将它们当同一事实身份。图可按endpoints+relation kind去重显示，但保全部assertion来源。从任一端编辑先选明确sourceOwner/Field/当前itemPath并重新获权，不能对合并边或代表行直接写入；互为关系的显示名不等于反向写入。

关系索引只是可重建语义投影。反向视图只列当前可读的来源记录及可披露目标，不泄漏隐藏来源的数量或名称；扫描不全为 partial。删除/复制影响只检查操作实际需获权的关系范围，缺完整范围不能声称无引用。历史任职时段属于源记录，反向 Query 保留同一记录的 role/期间关联。

`sources` 是可选有序原生记录数组：reference 可为稳定笔记引用或明确 URL，locator 为页/章节/原文位置，accessed 为 date/instant，note 为作者备注。来源可关联一个任职/历史事件/论文导入记录，不给所有普通电话强行新增来源。置信度/争议状态/测量精度如有实际业务需求，由该类型明确字段表达，不能重新引入一个必填通用 Entry。

没有跨版本逐值 UUID。表单数组行和引用观察依当前准确源版本+Field+AST 路径定位；删/重排/外部修改后失效，不能按同值重绑定。若将来确有独立分享一段任职历史、长寿命关系批注或外部系统键需求，优先独立笔记或显式业务键，并另行说明生命周期；本稿不暗藏 per-value identity。

标签的稳定身份来自 CORE 的集中 TagId 登记，原生字段只保存引用。新增闭合标量 codec `tag_ref`：作者字符串为 `weftext-tag://<WorkspaceId>/<TagId>`，两个标识均按 Core 的 UUID 规则解析；格式有效不证明存在或授权。可选通用 tags 字段采用有序数组，项是判别明确的 `(ref: "...")` 或 `(text: "研究")`；前者指已登记标签，后者是普通文字标签，不能按同名自动升级。ref 分支使用 tag_ref，text 分支使用 nonempty_text，二者恰一出现。表单选已有标签或明确创建时，可由同一 Plan 登记 TagId 并写入引用；标签显示名改动不改每篇源。Query、导航和快捷入口以完整 TagRef 比较，文字标签保原值。目录的 DirectoryRef 仅是 Core 的目标身份，不为每篇笔记新增目录属性权威。

## 5. 最小 schema 与代表源

随稿 `examples/schema.example.json` 是完整的最小代表 schema（不含可选 UI 标签），`examples/*.typ` 给人员、组织、论文、任务和数值边界样例。它验证能力形状，不冻结所有类型字段集。

| Type alias | Field 的形状与必填策略 | 使用链 |
| --- | --- | --- |
| person | names、phones/emails、生日、任职数组、历史/事件、关系及来源均按已选需求可选。任职记录显式 employer、position、department、期间和来源。 | 快速记录可只填一个名称；按组增第二同值电话或历史任职；按同一任职的组织+期间筛选；组织反向人员派生。 |
| organization | names、联系方式、parent/相关组织、成立/解散时间、外部标识、事件/来源可选。 | 组织关系与目录分开；结构化层级保真实 target；反向成员不双写。 |
| paper | publication record及其字段可先不完整保存；正式title、kind、保序contributors是完整引用输出的操作所需数据，不能反推为所有论文笔记必填；其余DOI/URL、期刊/容器、卷期页、日期、摘要、关键词、语言、标识/来源可选。 | 一份论文笔记同时是文献库可查询资料；表单重排作者→Query序列→导出引用保持相同顺序。 |
| task | status 必填；start/due、priority、project、assignees、dependencies、完成时间和历史可选。 | 正文即任务详情；列表/板/历只消费字段；改状态明确指向同笔记 status，不生成清单镜像任务。 |
| project | names、status、成员、起止范围等按需可选，不强制完整项目管理表单。 | 与 person/task 组合仍同一笔记/Field；项目任务来自 task.project 派生。 |

名称只有names数组一份业务权威，不另存name/preferred_name镜像。姓名记录的role/language/script、期间及明确preferred支持不同语境；快速记录添加一项即可。展示优先显式preferred、没有则按当前显示策略从现有名字取回退，不补写作者值；domain names不自动复制core aliases。文档title独立。

论文 contributor 是 `(agent: (text: "张三"), family: "张", given: "三", roles: ("author",))`，或 ref agent 加 `credited_as`/name部件；corporate contributor 用明确 literal name，不能强行拆 family/given。数组顺序就是署名顺序，role 与 affiliation 属同记录，不靠扁平多值列交叉组合。无法拆分的导入姓名保 literal。kind 为 article/book/chapter/thesis/report/web 等显式 schema 枚举或受控扩展；完整字段可由类型审计后扩展。

citation key 是文献输出的显式可选字段，稳定引用权威为笔记 ID。DOI只存publication.identifiers的scheme/value项，不再另存doi镜像。Work与paper共享一份publication记录，paper requires work；kind、publication_history、resources、contributors、venue、version_of及正式引用数据都在该记录，不另建Library副本。DOI 是 external identifier，不自动成为笔记 ID，也不默默合并不同稿件/版本。导入按获权候选确认创建/匹配/更新；未知信息和源文件保留，未支持的引用字段列损失报告。已知值与导入原值冲突需要选择，不能覆盖。

原生 Typst bibliography 需要的 BibLaTeX/Hayagriva 文件是从选定获权论文数据快照生成的只读编译资源；不是另一份手工维护文献库。正文原生 citation 与 bibliography 保官方语法；Core 的编译资源映射将本次使用的稳定笔记引用映到确定 citation key/文件，并检查键冲突。原生独立 bibliography 文件仍是普通作者资源，不自动与论文库合并。DOCX/XLSX/BibLaTeX/CSL类导出只消费同一字段语义，按目标格式列出不能表示的来源、精度、角色或多值，不能把未知值变空成功。此接口仍需官方 bibliography 原型与真实 Office 验收。

### 5.1 七个已选类型的 source→target 能力协调清单

下表核对历史 [D4](../snapshots/d4-attribute-types-schema-and-relations/source.md) §9 与其 [机器目录](../snapshots/d4-reference-catalog-registry/source.json) 的已选业务能力；历史表示不成为新实现。Person、Organization、Work 的业务字段可选，Task status 必填；周期 period、区间 range 及 Event 复用的 range 是对应用途的必要字段。完整 Project 管理不在历史七类已选范围内；Task 到普通项目笔记的可选关系已选。

| 历史来源/已选能力 | 新作者源表示（SOURCE拥有形状） | 消费与约束owner |
| --- | --- | --- |
| D4§9.2：Person多名称，role/language/script/preferred、期间/备注/来源 | names有序records；单一姓名权威，不另复制name或core aliases | SOURCE schema；EDITOR按语境显示/修改明确选项 |
| Person phone/email/address/website多值，preset personal/work/other+customLabel、期间/备注/来源 | 四个独立有序records数组；label码与customLabel文字分开，重复值合法 | SOURCE；EDITOR常用行/高级折叠；FEATURES Query/导出 |
| 人员账号、头像 | 账号记录包含预设服务或自定义服务标识、账号、用途和自定义标签；头像引用归属本笔记的资源 | SOURCE 定义形状；Core 管理资源归属和复制；FEATURES 报告导出损失 |
| 人员国籍、法律性别、性别认同三种独立状态；职业与任职独立 | 三组独立状态记录，含值、有效期、备注和来源；职业记录可关联组织语境；任职记录包含目标、部门、职位、职级制度及级别、有效期、状态、备注和来源 | SOURCE 定义形状；FEATURES 定义时段选择与反向任职；不按末项猜当前，不跨职级制度排序 |
| Person life-events birth/death/employment-start/graduation/marriage/custom；measurement height/weight | life_events records显式eventTime，保year/month/day/历法或instant及冲突来源；measurements含kind/exact quantity/unit/observedAt/precision/sources | SOURCE时间/量值形状；FEATURES历法/时间与量纲约束；confidence如确有来源用途可选，不统一必填 |
| Person parent/guardian/manager/mentor；spouse/sibling/family-related/social-related/professional-relation | 每含义独立关系数组：前4 child/ward/report/mentee→父母/监护人/上级/导师；后5对称。target ref或literal+期间/状态/备注/来源 | FEATURES方向、对称单份权威、反向与ranked图验证；派生colleague不生N²作者边；literal不生反向 |
| D4§9.3：Org names/status/identifier/classification/site/address/contact | 独立数组；identifier/classification scheme+value；site不是website偷换；每项期间/备注/来源 | SOURCE；FEATURES分组/时间过滤；外部编号不合并组织 |
| Org parent/governs/owns/allied-with/related/brand-of/business-guided-by/jointly-led-by/member-of/subsidiary-of/supervised-by/territorially-administered-by | 12个独立关系字段，Org→Org；只有allied-with/related对称，parent最多1，其余many；关系记录保validity/status/note/sources | FEATURES关系定义/反向/基数。历史多个parent的表达需显式重审max1范围，不能静默扩大；目录parent无互推；人员名册仅engagement派生 |
| D4§9.1：Task status/start/due/priority/completed-at/dependency；本轮project | status closed enum required，其余optional；completed_at exact instant；dependencies稳定Task引用；project普通Node引用、不强制Project type | SOURCE；FEATURES板/历/完成动作与inverse dependent；是否设置完成时刻进入动作预览；未选择完整status历史，不能虚构强制history |
| D4§9.5：Work kind/identifier/publication-state/resource/creator/venue/version-of；RQ10论文数据 | kind、identifiers、publication_history、resources、保序contributors、venue、version_of；论文publication结构补完整引用信息。出版状态有时间/来源。venue Work或Org最多1，≠publisher；version_of Work最多1 | SOURCE形状；FEATURES文献权威/引用/反向/导入导出。unknown作者可literal/结构化姓名，不强建Person；全部业务字段optional |
| D4§9.4：period-note period+series，scope unique/many | period record含kind day/week/month/quarter/year、calendar/version/rule/key、zone/tzdb，series_key；scope是集中系列政策，不在每篇源复制 | SOURCE形状；FEATURES周期解析/ISO week-year/unique并发与找已有；Core唯一创建事务 |
| range-note：date或instant半开range，至少一端有界 | range record明确start/end_exclusive，same basis、start<end，开放端省略，不填当前时间/极值 | SOURCE形状；FEATURES时间服务/重叠/导出；UI可友好显示包含末日但源与Query仍半开 |
| event复用range；status/recurrence/participants/reminder-intent optional | 原range字段不复制；recurrence record含anchor/frequency/interval/weekStart/byMonth/byMonthDay/byWeekday/count或inclusive until/rDates/exDates/exceptions；participant target Person或Org+role；reminder意图 | FEATUREScivil/DST/fold/gap/规则/窗口/例外服务，SOURCE原生结构；单次cancel/replace保原起点、range/title/status/note；occurrences派生、显式promote才新建笔记；提醒送达后置 |

现阶段fixture是代表性最小schema，证明了作者源能表达的形状；上表已选字段的集中定义必须在实施前完整生成并通过类型/Query/表单/导出共用的验收。复杂calendar、quantity、relationship约束由其明确owner注册具名codec/constraint，不用默认text/unknown吞掉未完成的语义。未知provider原文保全，不据此声称业务已校验。

历法/节假日pack是获权服务上下文，停用不删日期；不在源复制单isHoliday。ICS UID/RECURRENCE-ID/SEQUENCE是外部来源控制而非笔记身份；导入保原件/unknown property、附件与损失报告；subscribe/upsert仍需独立连接能力，普通导入不冒同步。完整HR/CRM、单位换算、国家职级pack、工商/文献connector、投稿/协同审稿、完整Projects/Assets/Places等仍候选或后置；不能从“所有类型”原则发明首发范围。Diary只是周期笔记/模板使用方式，不增平行身份。

## 6. 表单、Query、导出与安全编辑

字段面板来自集中 schema：常用组优先，可选组展开，数组添项/删项/重排；不使用的历史和来源不要求填写。作者字段、缺值、错误、unknown、只读计算分别显示。源码、表单与富编辑共享 Draft/Undo；保存前一份准确变更预览。UI简洁不裁掉嵌套数据。

Query 保留 scalar/record/ordered-list 类型，并支持同列表项的相关条件：例如 `exists engagement: employer.ref=O && during contains D` 必须对同一项成立；不能将两项组织与时间交叉配对。显式 flatten 产生 owner+当前 snapshot+数组 occurrence 的只读行，顺序列清楚；不是永久 row ID。any/all/count、空列表、缺值、unknown/invalid 均有明确语义，默认多值筛选不得悄悄只看第1项。排序列表字段要选规则（如 min/max/lexicographic），不可用值不当普通空。聚合/计算/反向关系结果只读；动作回到原笔记重新申请写入依据。

字段权限第一阶段以整个 top-level Field 为单位：嵌套行不是另一个默认 ACL 域。仅字段读取只返回该字段的语义和获准 raw 表示，不给全文 byte offset、旁邻字段、AST路径或正文。后续若需嵌套权限，要明确约束和unknown成员如何分权，不能仅隐藏控件。

结构化写入消费 Core 的一次性写入依据，冻结实际源快照/observer incarnation/file generation、schema 与当前权限。默认改完整 Field；精确子成员 patch 仅在当前快照 AST 的唯一静态范围及完整容器前像得到证明时使用。writer 按官方 parser/原生 constructor 回读拟议值，验证根角色、约束及非选字节一致；增删/重排的逗号和注释归属进入准确预览，不全文件 format。旧路径不跨 revision 重用，hash相同也不能复活依据。

窄权限写入须证明全部被影响约束/依赖均在获准范围，且不以隐藏正文决定可见错误、成功或输出。Core可在隔离内部处理完整源，但须使用管理员可审查和发布的静态隔离合同（代码/结构依赖，不按秘密值决定资格）；表单结果只受获权value影响，错误与时间也不泄漏秘密分支。不能靠 grep、一次编译、或服务偷偷读隐藏 body 然后暴露分支结果放行。在该证明尚不可实现时，Field-only写明确不可用；完整源获权用户仍可用同一源事务编辑。此界限保留字段权限隔离需求，并列为原型门，不能声称窄写已实现。

完整源保存可以保合法计算 Field、未知 schema/成员和编译失败的正文，按 Core 记录相应投影状态。typed 操作不得把 known invalid/缺必填洗成成功；显式 raw/repair 保存保稿但不发布完整 typed 事实。编译失败不会冒旧 preview 为当前，也不阻止无正文依赖的静态字段查询。

导出/Query输入是只读语义投影，附当前性与完整性；不是写入依据。复开保存 Query/使用缓存/恢复布局都重新获权和求值，不复活旧写权限。导出 XLSX 可用主表+有序子表：owner列和显式序号维持 contributor/employment/contact关系，精确数字与标识按目标的明确编码输出，不能经Excel数值位数限制失真。

## 7. 验收与证据边界

| 设计检查 | 成功要求 | 失败/保全要求 |
| --- | --- | --- |
| 根角色 | BOM/注释/多行/direct metadata、静态安全前缀、后置shadow符合规则 | 真第二Label/unknown前缀/重复key/control计算不认证；保原文 |
| 原生多值 | 相同电话两项标签/备注和顺序从源→表单→Query→XLSX一致 | 重排后旧第1项定位不可用；不按同值误改 |
| 嵌套任职 | 组织ref、职位、期间和来源留在同一项；反向组织视图派生 | 文本不造ref、partial不报全库零、撤权不泄来源数 |
| 论文 | 论文正式数据一份，作者顺序跨表单/查询/引用导出一致 | 未知作者文字保原值，导入冲突不覆盖，资源不双维护 |
| 类型组合 | 短名与全名同义、shared Field只一份，required OR | 同key不同含义/alias重绑/冲突不能 last-wins |
| 精确值 | int/decimal/partial date/offset instant编辑查询导出一致 | bool≠int，float不冒金额，时间不补本机时区，none≠缺席 |
| 安全写 | 当前一次性依据+唯一范围+回读+事务+同Draft/Undo | ABA/迟到/重排/撤权失效；不能隐藏body侧信道放行 |
| 完整源 | import/function/computed/复杂表格保源，可读复杂只读内容 | 控件不足不拒完整语法，资源失败不伪装语言非法 |

本机官方 `typst 0.15.1 (9dfd3a08)` 仅复用既有工具；样例 compile/query结果记录在 `examples/syntax-experiments.json`，只证明官方语法可编译及原生 metadata 的实验值。CLI query 未用于角色身份认证，JSON输出未用于精确数值权威。schema配置、权限、Query、事务、引用资源生成、DOCX/XLSX与视觉排版尚未产品验证。没有真实数据迁移证据，不编造旧decoder或迁移完成。

官方依据：[v0.15.1 AST](https://github.com/typst/typst/blob/v0.15.1/crates/typst-syntax/src/ast.rs)、[metadata](https://typst.app/docs/reference/introspection/metadata/)、[decimal](https://typst.app/docs/reference/foundations/decimal/)、[datetime](https://typst.app/docs/reference/foundations/datetime/)。其中官方原生语法与值的说明不证明本文应用角色、schema、权限或生命周期已实现。
