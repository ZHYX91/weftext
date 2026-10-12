---
source_language: zh-CN
translation_status: source
---

[English](SOURCE.md)

# Typst 作者源与普通业务属性

状态：**协调设计候选，未实现、未独立接受**。固定输入 `af6737a057c7bb69763dfaad0a57da4652e085f3`；作者分支 `design/typst-native-20261012`。本文拥有 `typst_source/1` 的根声明、原生值、schema 及来源投影。存储/事务由 [CORE](CORE.zh-CN.md) 拥有；显示/编辑由 [EDITOR](EDITOR.zh-CN.md) 拥有；链接、Query、模板、批注与输出由 [FEATURES](FEATURES.zh-CN.md) 拥有。它们是同一候选的具名接口，不拼接旧 AsciiDoc decoder。以下验收均为设计 oracle，不是产品测试通过。

## 1. 一份原生源

根 `.weftext-format` 精确字节为 `weftext.typst.v1\n`，受管形态是 `X/X.typ`；一个 Node 恰有一份主源。`TypstProfileCarrier/1` 的 `profileId=weftext.typst.v1`、`documentLanguage=typst`、`languageBaseline=0.15.1`、`sourceContract=typst_source/1` 与 CORE 的 format binding 同次绑定。标题、路径、目录树与文件名不是声明 ID。普通正文使用完整固定官方 Typst 语言；正文不要求标题，`title` 与 `heading` 各自保留原生语义。

```typst
#metadata((
  id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
  types: (),
)) <weftext>

#title[项目记录]

= 调研
正文、公式 $x^2$、列表、表格与 import/set/show 均使用原生 Typst。
```

所有原字节，包括 UTF-8 BOM、注释、空白、LF/CRLF/CR、字面量拼写与未知内容，均保留。D6 strict UTF-8 是物理入门，官方 `typst-syntax` 是唯一语法 parser；AST 检查器只校验应用角色并调用原生 token/AST 的解码和范围接口，不重写 lexer、字典 parser 或解释器。不剥离头部后把正文 offset 归零。

新笔记不生成 YAML front matter、JSON Entry 块或另一套双链语法。正文合法的 `json`/`yaml` 数据加载、资源、独立配置、批注和内部协议仍按自己的权限/格式使用。源中不存在 native/JSON 双业务载体或自动 fallback。

## 2. 唯一实际根声明

<weftext> 是大小写敏感、精确原生标签的保留应用角色。认证的是根源中的 AST 声明，不是渲染元素。识别规则按顺序执行：

1. 对准确完整根文件进行官方解析。声明是原生 `#metadata((...))` 的直接调用，随后在同一根 markup 层是其 `<weftext>` 标签，两者之间仅有空白/注释。调用目标是裸标识符 `metadata`，恰一个 positional dictionary AST 参数，没有 named 参数、spread、content block 或额外参数。外层调用不能套 `context`、条件、代码块、循环、函数、show rule 或 `include`。位置不要求第一表达式：声明可在标题前或后、普通静态正文后，不能由渲染顺序选择它。
2. 固定标准库在该位置提供真正的 `metadata`。官方 AST 对声明前的根词法前缀证明没有名称binding改变：允许BOM/trivia、纯markup文本/格式/heading/raw、无嵌套代码的标准 `#title[...]`，以及全部参数为静态字面量/标准值、无嵌套调用或binding的标准set rule（如 `#set text(size: 12pt)`）。这类标准调用/标准值由固定library确认；长度等普通native值只用于该前缀，非Field类型扩展。其它前置 let、import/include、任意调用、show/context、赋值或未知binder使本版控制binding证明 unavailable，不能执行它们后猜。宿主不能替换内置binding。后文合法的 `let metadata = ...` 不改变已认证声明，其输出不控制Node。前缀允许集合只是控制证明支持范围，其余合法源仍可保存，绝非第二语言parser。
3. 遍历完整根 AST 中实际 Label 节点，精确 `<weftext>` 必须恰一个。第二个同标签即使字典相等、位于尚未执行的分支或函数体内，也使控制声明歧义；不能“第一个/最后一个获胜”。字符串、注释和 raw code 中相同文字不是 Label 节点。其它标签和 FEATURES 保留的局部角色不计为根声明。
4. 根 dictionary 使用直接 named/keyed 条目；静态原生字符串 key 可用。解码后的 key 精确唯一，无 spread、动态 key 或重复（包括 `id:` 与 `"id":`）。必须有 `id`。只有 `id`、`types` 是根控制键；其余为业务键。`weftext-` 前缀保留给未来具名角色，当前禁止业务 schema 使用；不按显示标签、大小写或 Unicode 规范化合并键。
5. `id` 必须是直接原生 Str AST，解码后精确 lowercase UUIDv4；无补空格、大小写修复、插值、变量或函数求值。`types` 缺席等义于空声明集合；存在时必须直接原生 Array AST，元素均为直接 Str，空数组是 ()，单项是 `("person",)`。字符串、`none`、拼接、spread 或重复展开类型不是空集合。

根声明可以跨多行，含原生注释和尾逗号；这些规则没有定义新的语言语法。普通合法 `.typ` 若不符合这个应用角色，仍能打开、读写和编译；它没有据此获得受管 Node admission。已有受管文件的坏控制声明进入 CORE 原文/Draft/显式 repair 路径，不自造身份、不删除源。

导入/包含文件中的标签、函数动态返回的 metadata、`label("weftext")` 和编译 introspection 的结果均不是实际 root-authored 声明。渲染中相同标签可能产生编译错误或普通诊断；它不改根控制数据。应用绝不通过 `query(<weftext>)`、PDF/HTML、隐藏元素、编译成功或 show 的最后结果认证身份。全根 AST 无法完整判定重复（例如损坏语法使剩余结构不确定）时报告 `control_unavailable`，不猜唯一。正文未知函数等只影响 compilation 的错误不妨碍完整 AST 上的直接字段读取。

根接口称 `TypstRootDeclaration/1`：有效输出冻结 root source pin/完整 SourceObservation、声明/字典/ID/types 的 UTF-8 byte ranges、ID、原 types 顺序和根结构资格；无效输出冻结同源诊断，且不发布有效控制资格。它是 CORE evaluation 内的派生结果，不是永久身份证明、另一份可编辑元数据或新的签名 token。ID 与分类各自的语法诊断可展示，不能以可读 ID 跳过坏控制的 admission 门。

## 3. 稳定身份与类型

NodeRef 仍为 WorkspaceId + NodeId。文件 ID 只表达作者携带的声明；birth、file binding、tombstone、不复用、ACL、执行责任与格式生效证明来自原 Core/D3/D6 权威。源 ID、受保护绑定和 birth 不一致是 conflict/incomplete，不选一边覆盖另一边。缺 metadata、partial sync 或裸 `.typ` 不允许索引器重新分配 ID。

创建、模板实例、作为新笔记导入、copy/fork 的 fresh Node 使用原唯一私有 candidate map 分配的 UUID；move/rename 保留 ID，restore 使用实际历史绑定。`TypstControlConstruction/1`/`TypstDocumentConstruction/1` 由 CORE 从同次官方 AST 和准确 before pin 构造；已有 ID 只改其 Str value span，types 只改其 array span。没有声明时只在明确的新建/导入/修复完整预览中插入，不盲补第二声明。模板不能预填最终 ID。源、format binding、birth、引用映射和 SourceRevisionPlan 同一 CAS/P 安装；retry/unknown 使用原 frozen map、UUID、bytes/pins，不能重新随机。

`types` 是无优先级的集合，作者顺序仅用于无损编辑。中央 schema 的精确 alias 扩展到 `namespace/type-name`；全名始终可写，显示中文名不是 alias。alias 与全名同列而展开为同一类型时拒绝重复。unknown 合法 token 保留，相关 schema 状态 unavailable；不删除、降为空或凭安装顺序重绑定。非法 token 是分类错误。namespace/type/FieldId 的 ASCII 语义标识解码、owner 认证和 ledger 演进保留 D4 §2；源码键是另一种原生 dictionary key 域。

`template` 是 Core meta-kind 保留 token，不能由插件重绑定，不能与业务类型同列；实例目标类型由获权 Recipe 指定。Task 仍是 ordinary Node 的明确任务类型，不设 TaskRef、镜像 checklist 数据库或另一身份。ID 和 types 不作为业务 Field，不开放给属性写入器、bulk 字段、自动化 Field action 或 import 的普通字段映射。类型变更须验证完整有效闭包和实际 Source/control 写权限；界面分组/前缀不添加类型。

## 4. 原生值的准确集合

`TypstNativeValue/1` 是内部原生值投影域，不是要求作者手写的 tagged JSON。它恰支持下表递归集合；它不缩小完整 Typst 语言。

| 原生作者值 | 投影与精确边界 | 默认写入表示 |
| --- | --- | --- |
| string | 官方 Str 解码的 Unicode 字符串，空串与缺席不同；不 trim、NFC 改写或把电话转数字 | 原生引号字符串，按官方语法转义 |
| bool | `true`/`false`；不接受 0/1、字符串或大小写别名 | 同值 keyword |
| int | 官方 Int 的 signed 64-bit 域；支持实际native Int literal/一元符号及受控标准 `int("-9223372036854775808")`，不把bool/float转整数 | 优先十进制Int literal；同类型回读失败则使用合格int(canonical string) |
| float | 官方 Float 的有限 binary64 值，直接 float literal 与一元正/负号；内部冻结IEEE-754 64位pattern（含负零），不经JSON number丢bit；NaN/Infinity无Field投影 | 能原生回读到相同位型的 round-trip literal，必须显式保持 float 类型 |
| decimal | 受控标准 `decimal("3.14")` 或 `decimal(59)`；完整保留原生精确值，拒float隐式转换，超官方可表示范围报错 | 原生 `decimal` 构造器 |
| duration | 受控标准 `duration`，named参数限weeks/days/hours/minutes/seconds及直接Int；由官方构造器冻结实际native duration，不把calendar月/年当秒数 | 原生 duration(...)，必须精确回读同值 |
| none | 原生 `none` 是显式空值，仅 nullable schema 合法；不是 missing 或 default | `none` |
| datetime | §5 的标准构造调用，保留 date/time/local_datetime 三种形态 | 同形 datetime(...) |
| array | 原生 ()、`(value,)`、`(value, value, ...)`；有序、保留重复，元素递归属于此域 | 原生 array |
| dictionary | 原生 (:) 或静态 key/value entries；精确 string keys 唯一，values 递归属于此域；作者顺序保留，语义查找按key | 原生 dictionary |

官方 parser 与 native value API 解码字面量；直接一元符号仅作用于一个数值 literal，溢出报诊断，不自行计算二元表达式。标准int(Str)仅接单个direct Str，解码为canonical signed decimal且在i64域内，经官方int constructor验证；不接float或其它表达式。`-9223372036854775808`在官方语义实际是float，不能因字面像整数自判int。writer必须官方回读验证类型和值；i64MIN生成`int("-9223372036854775808")`。duration参数允许此同一受控Int常量表示。无引号标识符不当 string。(value) 是 grouping，不是 singleton array。字典/数组可含原生注释和 trailing comma；动态 key、spread、变量、用户函数、算术、访问表达式、content、length、angle、color、label、regex、module、bytes、auto 和非表内值没有本版作者业务投影。int(Str)/datetime/decimal/duration仅由受证明的标准binding和const参数经官方构造器解码，不递归执行任意函数。完整语言中的其它构造保持原生行为和源码入口。

按顶层业务 Field 独立投影。直接字面 Field 是 `available`；违反已知 schema 是 `invalid`；缺 schema/owner 是 `schema_unavailable`；合法但超出上述静态投影或计算式是 `computed_unavailable`。同一数组或字典中一个元素不可投影，使整个 Field computed/unavailable，不产生截断列表。一个计算 Field 不使其相邻合法电话/status 字段不可读。无法完成预算内验证时是 unavailable，不能返回部分值为成功。

编译器可另给计算内容的当前 `derived_readonly` 展示，绑定完整 environment/dependencies/origins。这个展示不是 authored Field、不能写回、不能作为缺失字段默认值，不能让 Query/Action 把 computed_unavailable 当已验证业务事实；缺编译结果保持 unavailable。若以后要将计算 Field 纳入全库 typed Query，须增加具名生产/当前性合同，不能靠最终 metadata query 偷渡。本候选保留 D5/D7 派生计算列/公式及原生正文计算，见 FEATURES。

缺失 key 表示 absence；`none`、""、`()`、(:)、0 与 false 都是各自真实值。空数组表示零个联系方式，不能变成“未知”或满足要求非空的约束。必填成员要求实际 presence，nullable 只决定 presence 后是否允许 none。非法值不转 absence，不用默认值遮住错误。schema default 仅用于新建/模板录入生成实际作者值；已有源读取不会自动补值。UI 的未保存建议必须标为 suggestion，不能进入 Query、排序、关系或导出事实。

## 5. 日期、时间、精度与引用

原生标准 `datetime` 有 date、time、date+time 三种内容，构造器没有作者时区成员。[官方 datetime](https://typst.app/docs/reference/foundations/datetime/)

- `date`：恰 year/month/day 三个直接 Int 构造参数，真实 Gregorian 日期，schema 本版范围 0001..9999，精度 day；不附时区。
- `time`：恰 hour/minute/second，范围 0..23/0..59/0..59，精度 second，无日期或时区。
- `local_datetime`：恰六个参数，日期/时间同上，精度 second；表示民用本地时间，绝不等于 instant。

参数可按原生任意顺序书写，不能重复，不能 positional、spread、表达式或省略 precision 所需成员。标准绑定由§2词法前缀和固定库证明；不调用用户同名函数。`datetime.today()`、日期算术等是合法 Typst，属于 computed_unavailable 作者 Field。构造器的真实有效性由固定官方实现验证；AST shape 检查不另造日期 parser。`due: datetime(year: 2026, month: 10, day: 20)` 是日期，不从上海时区补成午夜 UTC。

生日未知年份使用原生 `birthday: (month: 5, day: 12)`，schema `month_day` 是对应两个必填Int成员的dictionary语义，范围month 1..12/day按月合法最大日（允许2月29日，不猜年份）。它与完整date构成不相交union。不能填虚构年份、epoch或设备今年；按年显示是否遇到2月29日由获权Calendar视图策略决定，不改作者值。

`instant` 使用原生 string，schema 明确 RFC3339 instant 语义：要求日期、时间和 `Z` 或显式偏移，拒绝 `-00:00`、闰秒及无偏移；精确小数秒不经 float 截断。采用 D4 §4.1 精确 RFC3339/UTC 比较规则，保留原始偏移字节。需要 IANA zone/tzdb 的 recurring civil event 另外使用 schema 的原生 dictionary 明确 zone 与冻结版本，由 FEATURES temporal 服务处理；不能从 `datetime`、机器 locale 或 Workspace 显示偏好猜。非 Gregorian 历法、月/年精度与 ranges 同样需专门 schema 的普通字典，不能把 day datetime 假装等价。本版普通 task due 用 date 或 instant 的不相交原生域，不丢任意小数 instant 精度。

精确十进制（如普通金额）优先使用原生 `decimal("3.14")`，其固定精度/范围由官方0.15.1实现定义；标准constructor接单一direct string或int，拒float转换/计算参数。数值比较使用native精确值；来源/编辑同时保留scale和raw，写出同值native decimal，不悄悄四舍五入。超出native可表示精度的原D4任意精度需求以明确schema `decimal_text` 使用canonical string，其原精确比较规则保留，导入不能近似成native decimal/float。[官方 Decimal](https://typst.app/docs/reference/foundations/decimal/)

原生 `duration(weeks: 1, days: 2)` 等构造器是elapsed duration，不是民用月/年周期；named参数唯一、Int常量且经官方constructor成功；先checked验证每单位乘法与官方seconds→minutes→hours→days→weeks累加顺序的每个中间值，可抵消但中间超界亦拒静态Field，不声称官方提供checked constructor。默认writer使用duration(seconds:完整Int常量)且重读相等。本版受控constructor仅接Int，native duration按官方time::Duration保存精确整秒，内部用canonical signed i64 seconds；schema需要精确任意小数秒时用明确 `exact_seconds_text` 而非舍入。不从duration推导Calendar月/年。量值、范围、外部 identifier 等实际保留功能使用其具名schema dictionary普通原生值；没有通用 `any`、执行对象或JSON逃生口。[官方 Duration](https://typst.app/docs/reference/foundations/duration/)

跨笔记业务引用使用 schema `node_ref` 的原生字符串 `weftext-node://<WorkspaceId>/<NodeId>`，精确 URI 与共享 Ref 解码见 FEATURES；业务关系引用不带 fragment。真正正文链接可带其合法 fragment。普通 `company: "云山公司"` 是作者文字，绝不自动 resolve 成组织；若 schema 选用 `company_ref`，必须显式写稳定 URI。解析、ACL、unavailable、Trash、跨库 copy/fork 和 inbound 证明仍由 Core/FEATURES 处理，名字相同不造关系。电话/email/URL 格式验证不默认拨号或发消息。

## 6. 一个中央 schema 与多类型合并

schema 是独立的可移植获权配置，不重复存进每篇笔记；其持久承载、摘要、版本、认证和演进进入 CORE 的同次 `registry` dependency。名称为 `TypstSchemaSnapshot/1`，不能伪装旧 RegistrySnapshot/1 或 /2。固定成员为 kind:"weftext.typst.schema", version:1, generation, digest, predecessor, fields, types, aliases；digest 覆盖除自身外完整 canonical 对象，generation+digest 同时冻结；predecessor 是经认证 bootstrap 或前代完整 binding。内部配置可使用 JSON，作者 `.typ` 不需要。

`fields` 按 FieldId 排序唯一，每项固定 fieldId, key, labels, valueType, nullable, enum, default。labels 是显式语言标识→显示文字的表，不作语义键；enum 为完整允许原生值数组或 null（无枚举）；default 为 `absent` 或已验证 literal default 的配置记录。`types` 按 TypeId 排序唯一，每项固定 typeId, labels, fields, requires, conflicts；fields 是 {fieldId,required} 列表，requires/conflicts 是精确 TypeId 集合。`aliases` 为按name排序的 {name,typeId}，全快照 name/key/ID 绑定唯一。未知成员、错误容器、重复、引用缺失、循环、不对称 conflicts、同key映到不同FieldId或 ID digest 偷换都拒绝 schema 安装；不是空配置。

valueType 恰为 §4/§5 的标量语义名 string|bool|int|float|decimal|duration|date|time|local_datetime|instant|decimal_text|exact_seconds_text|node_ref，或以下递归结构：array(items,minItems,maxItems)、dictionary(members,additional)、union(variants)。这些是配置代数的描述名，**不是笔记中的新函数语法**。members 每项固定 key,valueType,required,nullable；additional 固定 `preserve`，未知字典成员保留并标为 unbound，不参加已知约束或 writable member action。union 必须有至少两条可唯一判定的分支（例如 datetime date 与 instant string）；多分支都匹配是 schema 冲突，不能 first-match。范围、enum、非空等成员限制只在中央已验证定义声明，不从示例 label 猜。

支持任意合法业务 key 字符串，不把中文显示标签转换成字段。默认生成器选可直接写的原生 identifier key，如 `phones`；其它 key 用原生字符串，例如 `"office phone"`。精确大小写/NFC 不相等；`phones` 与 Phones 不是别名。保留控制key不注册Field。未绑定顶层key保留为 `unbound` 原生数据，可在全源中修改，不误认其它namespace，也不自动删掉。既有非 canonical 原生拼写只有在被实际编辑的 value token 内才可改变。

多类型按已验证 requires 的完整 DAG 求 effective closure；不是 source 顺序 override。取各类型 fields 的并集；同 FieldId 只有一份中央定义和一个源 key；required 用逻辑 OR。该 Field 的 nullable/enum/default/valueType 全来自同一中央定义，因此不能按类型各自重定义。安装时发现两个 type contribution 想给同 key 不同 FieldId、不同类型/默认值或不兼容语义即拒绝，不能收窄交集、最后覆盖或复制两份源值。有效闭包 conflict 时 classification invalid；unknown declared type 时相关验证 unavailable，不能从已知其它类型推断完整成功。无类型的普通 note 可保留已绑定字段，但不凭字段存在自动具有 Person/Task membership。

界面可按“联系人/任务”等分组显示，同 Field 在多个组仍是同一个字段；折叠、隐藏、语言与安装顺序不改变值、权限或类型。更改key/alias/定义必须预览实际受影响源、冻结 before/after schema 与原权限，显式演进。同语义 ID 的不可变摘要、owner tuples、retirement/tombstone 和历史原 bytes 义务保留；简化新字段使用 fresh ID，不修改旧61 Field/7 Facet目录的原字节或假装已有迁移。

### 6.1 闭合配置、原生值和资格编码

本节固定真正内部编码，取代§6描述代数时使用的括号简记。所有对象闭合，未知成员/版本拒绝；Counter、PinRef/2、NodeRef、SourceObservation/1、SourceVersion/2、DependencyProof/4直接复用CORE引入的原精确类型。text不trim，Digest是64个lowercase hex SHA-256，ID使用§3精确域；集合排序按unsigned UTF-8，所有计数/容器在分配前检查溢出/预算。

```text
TypstNativeValue/1 =
  {kind:"string",value:text}|{kind:"bool",value:Boolean}
 |{kind:"int",value:canonical-signed-decimal-i64}
 |{kind:"float",bits:lowercase-16-hex-IEEE754-binary64}
 |{kind:"none"}
 |{kind:"decimal",negative:Boolean,coefficient:canonical-unsigned-decimal-u96,
    scale:integer(0..28)}
 |{kind:"duration",seconds:canonical-signed-decimal-i64}
 |{kind:"date",year:integer(1..9999),month:integer(1..12),day:integer(1..31)}
 |{kind:"time",hour:integer(0..23),minute:integer(0..59),second:integer(0..59)}
 |{kind:"local_datetime",year:integer(1..9999),month:integer(1..12),
    day:integer(1..31),hour:integer(0..23),minute:integer(0..59),second:integer(0..59)}
 |{kind:"array",values:[TypstNativeValue/1...]}
 |{kind:"dictionary",entries:[{key:text,value:TypstNativeValue/1}...]}

TypstValueType/1 =
  {kind:"scalar",name:"string"|"bool"|"int"|"float"|"decimal"|"duration"|
    "date"|"time"|"local_datetime"|"instant"|"decimal_text"|
    "exact_seconds_text"|"node_ref",nonEmpty:Boolean}
 |{kind:"array",items:TypstValueType/1,minItems:Counter,maxItems:Counter|null}
 |{kind:"dictionary",members:[{key:text,valueType:TypstValueType/1,
    required:Boolean,nullable:Boolean}...],additional:"preserve",semantic:"object"|"month_day"}
 |{kind:"union",variants:[TypstValueType/1...]}

TypstSchemaBinding/1={version:1,generation:Counter,digest:Digest}
TypstSchemaSnapshot/1={kind:"weftext.typst.schema",version:1,generation:Counter,
  digest:Digest,predecessor:{kind:"bootstrap"}|{kind:"previous",binding:TypstSchemaBinding/1},
  fields:[{fieldId:FieldId,key:text,labels:[{language:text,text:text}...],
    valueType:TypstValueType/1,nullable:Boolean,enum:null|[TypstNativeValue/1...],
    default:{kind:"absent"}|{kind:"literal",value:TypstNativeValue/1}}...],
  types:[{typeId:TypeId,labels:[{language:text,text:text}...],
    fields:[{fieldId:FieldId,required:Boolean}...],requires:[TypeId...],conflicts:[TypeId...]}...],
  aliases:[{name:text,typeId:TypeId}...]}
TypstSchemaEvolution/1={version:1,previousSnapshot:TypstSchemaSnapshot/1,
  currentSnapshot:TypstSchemaSnapshot/1}
TypstSchemaQualification/1={version:1,snapshot:TypstSchemaSnapshot/1,
  binding:TypstSchemaBinding/1,evolution:TypstSchemaEvolution/1|null,
  dependencyProof:DependencyProof/4}
```

float bits按sign/exponent/fraction的64位整数网络次序hex编码（高位在前），有限值才合法；正负零不合并来源值，数字比较仍按对应native语义。int字符串只能`0`或非零无前导零signed decimal并在i64域内。decimal coefficient范围0..79228162514264337593543950335，保留native负号/scale，不能用JSON number或float承载；原生构造器必须准确得到相同值/表示，不可表示即失败；duration仅本文Int参数构造的whole-second子域，按官方checked representability验证总和，不以`.seconds()`浮点输出证明相等。date/time额外验证真实日期与官方constructor，不因wire day上界31允许2月31日。dictionary entries按作者顺序且key唯一，不排序值；array保重复。此内部tagged表示不写回`.typ`，不是重新引入作者Entry。

nonEmpty仅string/instant/decimal_text/exact_seconds_text/node_ref可true，其它scalar固定false；它不执行trim。instant/decimal_text/node_ref等在native层仍是string，只有typed schema位置生产对应业务语义。month_day dictionary须恰month/day两个required且nonnullable int成员、additional=preserve；已知month/day按§5检验，未知成员仍unbound。array最大null仅表示没有schema数量上限，实际明确的Core预算仍有限；min≤max。类型树无执行式/alias递归，深度/字节受本次固定预算，超预算整项unavailable而非空。

labels按language排序唯一，language必须显式语言tag，不将显示文字变alias；fields/types/aliases和嵌套成员按相应ID/key排序唯一；requires/conflicts为排序唯一集合。enum为null或非空、唯一且全部匹配完整Field语义(valueType+nullable)的原生值集合；nullable=true时none可作为enum/default值；dictionary成员同理按member valueType+nullable验证。default literal必须通过同一schema全验证。nullable Field的none只有在nullable=true且enum为null或含none时允许。union各分支不能重叠，unknown schema不先试某个分支。

qualification绑定snapshot两项generation/digest；bootstrap generation=1且evolution=null，非bootstrap要求完整evolution的previousSnapshot精确等于predecessor.binding、currentSnapshot等于snapshot、generation checked +1。digest取D3-CJ/3对象除digest自身的SHA-256；旧snapshot原bytes不补字段。安装者须先有CORE真实registry配置/owner认证与管理授权，同cut dependencyProof的registry项必须完整覆盖已安装schema及绑定/evolution的真实pins，authorization项覆盖当前policy；proof不是客户端自报true，不能单凭一份自洽snapshot授予其定义权限。本文不扩D10签名或旧Registry decoder。

同一新目标schema域保留ID定义不可变：Field的key/type/nullable/enum/default和Type的fields/requires/conflicts变更须新ID；显示labels可在明确配置演进预览中变，不改语义。删除可用定义在本版拒绝schema安装，先通过另行具名退役/迁移控制设计再做，不隐含缩短旧ledger或复活同ID。本版支持新增无冲突定义/alias，已有alias不改绑；新schema域初次建立是目标设计bootstrap，不把旧候选称为真实运行迁移。

### 6.2 各操作的不可用结果

| 输入状态 | raw/Source与不相交普通保存 | 新建/触及Field的typed保存 | complete Query/Field Action/typed export |
| --- | --- | --- | --- |
| unknown type/schema owner或schema_unavailable Field | 获权可读；未触及bytes保全，不相交正文修改可保存并标typed partial/unavailable | 不得宣称完整typed新建；明确raw/import repair路径独立，触及普通Field写入不放行 | 实际消费处unavailable，不能空行/漏字段宣称complete |
| computed_unavailable合法Field | 普通全源可读写并保留表达式；非计算邻居可独立available | 完整Source保存保合法表达式并标computed；普通Field action不可写该表达式结果 | authored Field消费者unavailable；派生列/正文计算按FEATURES独立合同 |
| 已知本地invalid/requiredness失败 | Draft/准确repair可读；未触及的原错误不洗成complete | typed普通保存/新建拒绝，保留Draft；显式repair不是typed成功 | 拒绝或invalid诊断，零业务写 |
| body compile/resource失败，control/literal Field完整 | Source与独立Field仍可读/保存，编译结果标failed | 完整有效本地字段可按Core保存，缺显示不伪造typed失败 | 不消费body的Query可正常；render出口unavailable |

任何结果仍先核实际授权、真实版本和操作所需证明；这张表不授权隐藏源读取或绕过已知deny。新建原文导入可保unknown内容但不能声称typed copy/fork的完整语义成功。

## 7. 普通通讯录与任务

联系人只保留常用信息。本文在一个新的目标 `TypstSchemaSnapshot/1` 域定义内部类型 `people/contact`、`organizations/contact`、`tasks/task`，alias 分别为 `person`、`organization`、`task`；新schema域的完整身份为 `(schema binding, semantic ID)`，不得把它当旧活跃catalog中同字节ID的摘要修改。普通字段是中央定义，用户不必写这些内部ID。共用联系方式由同一 `people/contact-*` Field定义复用；组织独有字段用 `organizations/contact-*`，任务用 `tasks/*`。旧catalog owner/ledger/原字节按其原域保持；新目标定义不虚构已部署迁移。

```typst
#metadata((
  id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
  types: ("person",),
  name: "张三",
  company: "云山公司",
  department: "研发",
  position: "工程师",
  birthday: datetime(year: 1990, month: 5, day: 12),
  phones: (
    (number: "010-12345678", label: "工作"),
    (number: "010-12345678", label: "私人", note: "只在晚间联系"),
  ),
  emails: ((address: "zhang@example.org", label: "工作"),),
  addresses: ((address: "北京市海淀区", label: "工作"),),
  website: "https://example.org",
)) <weftext>

#title[张三]
= 联系记录
谈话摘要、历史和细节写正文。
```

Person：`name` string 必填非空；`company/department/position/website` optional string，`birthday` optional date或month_day；`phones` array of dictionary，number string 必填非空，label/note optional string；emails 对应 address string 必填非空；addresses 对应 address string 必填非空。联系数组允许缺席或空，可保留相同值；number 不要求 E.164、不去空白改写，电话号码的前导零和 `+` 都是文本。label 是普通可编辑文字，不默认变为 semantic code；note 是普通备注，不解析成 provenance/assertion。`name` 联系数据和文档 title 相互独立，UI 创建时可建议复制，后续不强制双写。

```typst
#metadata((
  id: "0efb7d4e-6248-451c-b747-692f1ae9b1cb",
  types: ("organization",),
  name: "云山公司",
  website: "https://example.org",
  phones: ((number: "010-12345678", label: "总机"),),
  emails: ((address: "info@example.org", label: "公开"),),
  addresses: ((address: "北京市海淀区", label: "办公"),),
)) <weftext>

#title[云山公司]
业务介绍、联系人与组织说明写正文。
```

Organization：name required 非空 string，website optional string，phones/emails/addresses 与 Person 共用相同 Field 定义，不因类型改写数组结构。组织笔记的文件树 parent 是 D3 结构；业务 `parent_ref` 若显式安装专用 schema 则为独立业务关系，二者不互相推导。任职详情、组织层级或图谱不是普通 company 文本的隐式约束。

```typst
#metadata((
  id: "e8ec9521-901f-4e5f-b4a3-151302967ac8",
  types: ("task",),
  status: "open",
  priority: "normal",
  due: datetime(year: 2026, month: 10, day: 20),
)) <weftext>

#title[准备项目报告]
整理调研结果，形成项目报告。
```

Task：project为可选node_ref稳定字符串，指向ordinary项目笔记，不要求目标Project类型；aliases为可选非空字符串数组，提供显式笔记别名（例如白云山2026的aliases=("白云山",)），不由标题/文件名回写生成。Task：status required string enum open|done|cancelled；priority optional string enum low|normal|high|urgent；due/start optional date 或显式 instant string，原始 status/priority 值域保留。录入新建默认 status="open" 可由模板明确生成，不是读时默认。types+status 给任务视图实际生产者，正文 native checklist/list 不自动生第二个 durable Task。复杂任务依赖、关系及 Calendar event 消费须按 FEATURES 具名 schema/证明，不能把普通 Task due 宣称为完整 recurrence event。

### 7.1 基础目标schema的具体绑定

下表固定本版基础配置而非从示例猜类型。全部Field nullable=false；除status外default为absent；除任务枚举外enum=null。`contact-string`表示scalar string/nonEmpty=false，`contact-required-string`仅nonEmpty=true；这些只是本表缩写，不是源函数或额外TypeSpec alias。

| key | 新目标schema的FieldId | valueType与实际类型使用 |
| --- | --- | --- |
| name | people/contact-name | contact-required-string；Person/Organization required |
| company | people/contact-company | contact-string；Person optional |
| department | people/contact-department | contact-string；Person optional |
| position | people/contact-position | contact-string；Person optional |
| website | people/contact-website | contact-string；Person/Organization optional |
| birthday | people/contact-birthday | date或month_day不相交union；Person optional |
| phones | people/contact-phones | 数组至少0项且maxItems=null；字典的number必填非空，label/note为可选字符串，未知成员保留；人员/组织类型可选 |
| emails | people/contact-emails | 同phones，number改address；Person/Organization optional |
| addresses | people/contact-addresses | 同emails；Person/Organization optional |
| status | tasks/status | 字符串标量且nonEmpty=true；枚举open/done/cancelled，默认字面字符串为open；任务类型必填 |
| priority | tasks/priority | 字符串标量且nonEmpty=true；枚举low/normal/high/urgent；任务类型可选 |
| due | tasks/due | date或instant不相交union；Task optional |
| start | tasks/start | 同due；Task optional |
| project | tasks/project | 标量node_ref且nonEmpty=true；任务所属ordinary项目笔记，Task可选 |
| aliases | core/aliases | 数组元素为非空字符串，至少0项且maxItems=null；所有基础类型可选，无类型普通笔记亦可使用 |

aliases保留原生顺序与重复字符串，名字/别名候选索引的去重与ACL由FEATURES负责，不改变作者值或赋予别名身份；project读取/复制/删除依赖按typed NodeRef及实际inbound门。系统别名语义仅认同一已安装schema资格中Core保留的core/aliases FieldId及其准确绑定；同名业务键或插件字符串数组不能认领它。项目语义仅认tasks/project，要求主体显式Task类型、实际Field/目标披露与scope权限及同cut引用证明，不将company文字或其它NodeRef字段当项目；无默认inverse/复杂事实网络。三个基础类型的requires/conflicts均空；它们可显式组合，同key/Field复用按§6合并，Person+Organization仍只有一份name/phones。template为Core meta-kind不放入这三个类型的可组合域。schema bootstrap的具体language labels由安装的完整配置明确给出并签入digest，不能读宿主locale自动填。UI分组按使用这些Field的type生成，不增task/contact嵌套Field。需要公司稳定引用、组织parent、专业关系或其它值时显式安装新的Field/type贡献，不默默把基础text升级成relation。

### 7.2 唯一基础seed生产与安装

Core的内置seed工厂根据本节确定性构造完整Snapshot，而不是从示例/旧catalog推断。generation=1，predecessor={kind:bootstrap}；fields/type/aliases按§6.1排序；nullable均false；所有labels精确为空数组（尚未提供显示翻译时UI可显示key的非权威标签）。后续管理员可明确安装带语言labels的配置后继，不凭设备locale回填。

工厂逐项展开§7.1：contact-string为字符串标量/nonEmpty=false，contact-required-string为true；date/instant为标量并分别nonEmpty=false/true。month_day为semantic=month_day的字典，成员day/month均required=true、nullable=false、整数标量/nonEmpty=false，additional=preserve。birthday的联合体分支顺序固定date后month_day；due/start固定date后instant。phones为数组（元素为字典，最少0项且maxItems=null），字典semantic=object，三个成员label/note/number（按key排序），label/note为可选成员、number为必填成员且nonEmpty=true，均nullable=false；emails/addresses只将number换address并按key重排。所有默认值为{kind:absent}，status例外为{kind:literal,value:{kind:string,value:open}}；status/priority 枚举分别按ASCII排序cancelled/done/open与high/low/normal/urgent的原生字符串值数组，其它enum=null。project为scalar node_ref/nonEmpty=true；aliases为array(minItems0,maxItemsnull)，元素scalar string/nonEmpty=true；两者default=absent、nullable=false。

types恰people/contact、organizations/contact、tasks/task三项，requires/conflicts均空：Person使用name(required)、company/department/position/website/birthday/phones/emails/addresses/aliases(optional)；Organization使用name(required)、website/phones/emails/addresses/aliases(optional)；Task使用status(required)、priority/due/start/project/aliases(optional)。type fields按FieldId排序。aliases恰organization→organizations/contact、person→people/contact、task→tasks/task并按name排序。每个对象均按§6.1准确成员名编码，本文简记里的字符串需真实为JSON字符串，不把none/null、Boolean与integer混用；digest按§6.1从完整构造对象计算，不使用占位摘要。

工厂输出只是不可变配置候选，不是自动激活或读时默认。唯一安装入口是CORE获权的Workspace bootstrap/registry配置事务：管理员选择完整seed，冻结准确snapshot bytes/pin、TypstSchemaBinding、必要的Evolution和format registrySeedBinding，验证真实owner/管理Policy，再用同一原planning CAS/P建立schema配置。只有已安装且取得TypstSchemaQualification的seed能被新建Task/Person表单、模板、Field producer与Query消费。source.type alias不触发seed下载/安装，不把缺registry当空seed；已计划操作冻结原seed不因软件更新重生。普通task创建由该已安装status default经获权模板/表单显式写入source，最后重验同cutschema，不从本段散文临时补默认值。

## 8. 精确来源与编辑资格

`TypstFieldProjection/1` 每个业务key冻结：owner、完整 source observation/version+pin、schema generation+digest、解码key、FieldId（未绑定时无ID）、availability、typed value（仅available时）、原表达式字节与AST path、完整origin及validation dependencies。origin 类别统一为 rootLiteral|rootConstruct|generated|include|multiOrigin；path/range 都只是此次来源，不是持久身份。根声明内直接 literals 是 rootLiteral，标准 datetime 是 rootConstruct（含int(Str)/datetime/decimal/duration）；其它表达式保留原 origin，不靠渲染同文本猜 rootLiteral。

默认写入粒度是整个 Field value；联系方式数组里没有每值 UUID、assertion ID 或隐藏稳定值身份。子成员控件可对当前 AST path（如 phones 第1项 number）做 revision-local patch，但 selector 必须冻结完整数组前像、准确源 pin/version、当前 observation、schema、授权及选中 token 原字节。删/重排/外部改值、ABA、gap 或另一次 revision 后旧 path 必须失效，重新读取选择；相同值不重绑定。批注也不能把数组下标当跨版本永久对象，见 FEATURES。

窄写资格是独立授权结果：唯一 rootLiteral/rootConstruct 可写span，完整 current source+schema+format 与 Field read/write、entity/state/commit 资格；静态保守 dependency 上界以及实际 origin/use footprint 都可证明；完整 before/after local constraints、requiredness/closure 不变性；不披露未授权正文/其它字段/negative range。原 narrow-field disclosure/静态和实际双证明保留，CORE的Policy/5仅新增默认deny的Workspace能力 `typst_dependency_state(fieldId)`，能力先于依赖检查；旧Policy4的AsciiDoc `native_dependency_state` 不自动授权新的Typst内容。

由于根 metadata 不绑定一个可供后文引用的变量，普通直接 status 的独立改值有可实现正例；但任意正文可通过 introspection 读它，不能仅 grep `status` 宣称无依赖。没有完整官方求值来源/依赖证明时，结构化计划采用已获权的 whole-source scope；Field-only 客户得到聚合 unavailable，不看隐藏正文后泄露原因，也不临时授予全源权限。复杂合法表达式不能因缺窄证据而拒绝独立 Source 保存。

修改既有 string/number/none/bool 仅splice选中value token，int(Str)/datetime/decimal/duration这类rootConstruct仅splice完整标准构造表达式或具有完整schema解码/同类型回读资格的参数token；新建/移除key、新增/删除/重排array元素须完整Field容器和trivia边界的来源/授权计划，不能用不存在selector或猜逗号。保持未选字节；被删除项绑定的注释处理进入准确预览，不顺手格式化其它项。完整官方重解析后证明唯一声明、ID/types、拟议值、schema、非目标AST结构与byte equality；字段改值不能挟带第二Label、修改key或控制声明。

属性面板、源码编辑与普通富编辑共享一个 Draft/Undo 和同一 Core prepare/commit。属性面板保留按schema分组、搜索、固定常用字段、作者值/只读计算/不可用状态的区分；显示标签不替代FieldId。设备最近使用仅在真实提交成功后更新，取消、预览或hover不计；该设备偏好不成为业务源或权限。Source editor 可保存完整合法 Typst（含import、functions、计算值、生成内容），与属性/富编辑的可写子集分离；保存不要求每个投影都available，不把缺renderer/资源或正文compile失败写成语言非法。坏语法/坏控制/已知局部schema错误不能洗成typed成功，保留Draft与CORE明确repair/原文状态。保存/安装/seal/预览编译是不同结果；旧preview不显示为当前。已知源变换的批注坐标平移只消费同一真实 plan，不能靠文本匹配复活旧地址。

### 8.1 闭合根/字段输出与地址

`TypstByteRange/1={start:Counter,end:Counter}` 是准确UTF-8半开byte范围；start≤end且边界必须为合法scalar boundary。声明和值均正宽，只有CORE明确的新建插入点可零宽。内部AST path数组由官方根children索引构成，不能从DOM/text或文件路径计算。

```text
TypstSourceDiagnostic/1={code:text,range:TypstByteRange/1|null,message:text}
TypstRootDeclaration/1 =
 {state:"authenticated_structure",version:1,observation:SourceObservation/1,
  sourcePin:PinRef/2,declarationRange:TypstByteRange/1,dictionaryRange:TypstByteRange/1,
  idRange:TypstByteRange/1,id:NodeId,typesRange:TypstByteRange/1|null,
  declaredTypes:[text...],diagnostics:[TypstSourceDiagnostic/1...]}
 |{state:"invalid_control",version:1,observation:SourceObservation/1,
   sourcePin:PinRef/2,diagnostics:[TypstSourceDiagnostic/1...]}
 |{state:"incomplete_source",version:1,observation:SourceObservation/1|null,
   sourcePin:PinRef/2|null,diagnostics:[TypstSourceDiagnostic/1...]}
TypstFieldProjection/1={version:1,owner:NodeRef,observation:SourceObservation/1,
  sourcePin:PinRef/2,schema:TypstSchemaQualification/1|null,
  fields:[{key:text,fieldId:FieldId|null,
    state:"available"|"absent"|"unbound"|"invalid"|"schema_unavailable"|"computed_unavailable",
    value:TypstNativeValue/1|null,expressionRange:TypstByteRange/1|null,
    astPath:[Counter...],rawExpression:text|null,
    origin:"rootLiteral"|"rootConstruct"|"generated"|"include"|"multiOrigin"|null,
    diagnostics:[TypstSourceDiagnostic/1...]}...]}
TypstElementLocator/1={version:1,owner:NodeRef,revisionToken:Token,
  path:[Counter...],range:TypstByteRange/1}
TypstRangeLocator/1={version:1,owner:NodeRef,revisionToken:Token,
  range:TypstByteRange/1}
TypstSelectedField/1={version:1,fieldId:FieldId,key:text,
  state:"available"|"absent"|"invalid"|"schema_unavailable"|"computed_unavailable",
  value:TypstNativeValue/1|null,rawFieldSource:text|null}
TypstOriginEvidence/1={version:1,evaluationDigest:Digest,
  records:[{id:Counter,kind:"rootLiteral"|"rootConstruct"|"generated"|"include"|"multiOrigin",
    sources:[{owner:NodeRef|null,pin:PinRef/2,path:[Counter...],range:TypstByteRange/1}...],
    inputs:[Counter...]}...]}
```

authenticated_structure仅指§2准确唯一AST结构、direct ID/types和binding成立，不表示出生/权限认证。缺可解析根声明是invalid_control；字节/完整源资格缺失，或语法损坏导致不能排除第二声明是incomplete_source。`control_unavailable`是diagnostic code，不是第四state。diagnostic code/message是显示信息，不作为安全分派；安全只按闭合state。无权限先返回Core不可披露结果，不能使用诊断泄漏存在。

Field fields按key排序唯一，是完整被获权Field范围而非全库完整声明；若调用只获权一个Field，仅输出该Field。available时value必须非null，`none`用`{kind:"none"}`；absent时value/range/raw/origin为null且path=[]；其它state的value=null，存在expression时range/raw/path三者精确同源，不虚构missing token。unbound可在独立全源/原生数据展示中保留raw，但不发布typed value。schema=null只表示无法取得完整资格，不是空schema。sourcePin/observation/owner/schema与CORE evaluation同cut逐项一致。

`TypstFieldProjection/1`是内部完整源投影，绝对range/path不得直接给Field-only调用方。CORE `TypstFieldImage/1`的projection采用`TypstSelectedField/1`；仅传所选完整Field的原生key/value与该项owned comma/trivia的rawFieldSource，无绝对offset、邻接字段、根dictionary或正文。value/state同上；absent时raw/value=null。opaque pin绑定准确Field载体bytes不是合成JSON；旧FieldEntryImage无此decoder。原生字符串很像Ref也只有已绑定typed位置解码，不扫描raw。

TypstOriginEvidence/1由CORE同次官方求值/AST产生，record.id恰连续0..n-1；inputs引用更早id且无重复，sources按(owner,pin,path,range)规范排序唯一，不能从HTML文本匹配回填。literal/construct恰一个真实源、无合成依赖；include指实际其他unit，multiOrigin必须完整多源，generated列完整实际inputs且允许无作者源。evaluationDigest绑定同次源/World/environment/target，证据pin覆盖完整records；记录存在只证明origin，不授读写/稳定身份。静态Field读取不需要编译证据；EDITOR/CORE compiled semantic originIds必须逐项命中此record，缺证据只读/unavailable。

新地址使用revisionToken而非嵌入future SourceVersion.changeId；同决议构造先由原SourceRevisionPlan冻结token，两遍物化稳定，seal后受保护binding关联实际完整version/ChangeId，不改已freeze after源，不自指。地址尚未seal时不可对外宣称可读。

这两个新地址不扩大旧l1/l2或D2 locator decoder。Token是43字canonical base64url、tag精确d6_source_revision/2，沿原Artifact/受保护RevisionTokenBinding/2解析真实已seal生产SourceVersion/2；cold read先获权、验证原production binding，再取得新的current SourceObservation与该版本精确相等，最后用固定官方AST验证range/path。external未admission的选择只留Draft/runtime basis，不发持久locator；相同字节或相同path不续接跨版本身份。generated/include/multiOrigin需实际唯一owner来源才有准确源地址，不把渲染元素序号强变地址。批注/建议使用FEATURES/EDITOR相应新format adapter，历史版本按原decoder。

## 9. 明确简化及旧条款处置

本表对输入版本的 actual current 文件/section 指出具名替换；历史文件与已记录decoders/bytes不修改。没有真实部署证据的设计前稿不产生runtime迁移责任。正文和资源转换须另行import/loss预览，不宣称已转库。

| 旧current来源与section | 本候选具名处置 | 新owner/section与验收 |
| --- | --- | --- |
| `a2-final-design/AUTHORING` §§1–2 | `typst-root-authoring` supersede：AsciiDoc属性名、连续header、逗号types改为原生metadata/dict/array | SOURCE §§1–3；S-01–06 |
| `AUTHORING` §3 | retain稳定Node/birth/ACL/partial-sync原则；载体与construction supersede | SOURCE §3、CORE；S-07 |
| `AUTHORING` §§4–5、`D4` §§2.2、3、16.1、16.3 的新作者载体 | `typst-schema-native-values` supersede native binding/codec/header-entry UUIDv5/Entry封装；中央schema原owner/绑定/ledger义务retain | SOURCE §§4–6、8；S-08–14 |
| `AUTHORING` §6、`D4` §§4.2、16.2 | `ordinary-business-model` supersede根collection禁令、逐值Entry/qualifier/provenance及JSON复杂fallback；用原生array/dict，不默认每值UUID/置信度 | SOURCE §§4、7–8；S-09、15–17 |
| `D4` §§10.1–10.2、17.1、`D4-IMPACT` §9 People/Organizations、`snapshots/d4-reference-catalog-registry/source.json` people/organization definitions | `contact-simplification` supersede默认事实断言、状态/测量/职业分类、rank制度、亲属/社交/职业/27关系与自动inverse/network义务；常用姓名/联系/单位/职位/备注retain为普通字段/正文。专门关系功能须FEATURES具名安装，不默认导回通讯录 | SOURCE §7、FEATURES；S-15–18；旧catalog原字节retain只作原对象 |
| `D4` §§4.1、9、10.3、16.6、17.5 | retain精确decimal/instant、日历/时区/recurrence服务语义和完整证明；`native-temporal-values` supersede作者JSON constructors为schema原生string/datetime/dictionary | SOURCE §5、FEATURES；S-12、19 |
| `AUTHORING` §7、`AUTHORING-INTEGRATION` §§2–3.1 | `typst-field-origin` supersede header/Entry selector、rawAttributeSources和旧native适配器的本载体应用；retainCurrent cut/disclosure/窄静态+实际证明和单Draft | SOURCE §8、EDITOR/CORE；S-20–23 |
| `AUTHORING-INTEGRATION` §§4–6.1 | `typst-format-integration` supersede本候选profile/registry/header-H构造分派；禁止把Typst塞旧Result10 H或Environment4 | CORE、SOURCE §§1、3、6；S-07、24 |
| `D2` §§1、5、8–9、§12中的SourceRange/Origin/Snapshot来源合同 | retain唯一源、严格UTF-8、真实owner/origin、availability/currentness、domain隔离；AsciiDoc专用projection closed arms由新官方Typst CORE生产者supersede | SOURCE §§1、8、CORE/EDITOR；S-22–24 |
| `D2` §§2、6–7、11–12的fixedRuby语言/语义oracle/旧property profile | `official-typst-language` supersede语言引擎及原oracle义务，不重新实现Typst或把staticField集合当语言上限 | CORE/EDITOR、SOURCE §§1、4；S-25 |
| `AUTHORING` §8、`AUTHORING-INTEGRATION` §7 | 本profile验收由下表替换；原例对原版本仍retain，不能把旧关闭证据当本候选通过 | SOURCE §10、统一ACCEPTANCE/INTEGRATION |

取消默认的复杂联系人事实系统是用户确认的产品简化，不是缺失实现的借口。历史note/provenance/precision等真实资料不静默丢失：转换报告保留原件，并按明确选择映为普通note/正文/独立资料或拒绝有损转换；不从note自然语言反解析成旧事实。专业知识图谱、Library、Calendar与业务Action的保留需求按FEATURES具名schema处理，不以保留复杂能力为由迫使所有联系人使用它们。

## 10. 正反验收oracle

每行要求正常、失败/保全和实际producer→consumer；尚未运行产品实现/权限/同步测试。

| ID | 正例 | 反例/应保全结果 |
| --- | --- | --- |
| S-01 | BOM/comment/multiline dictionary+label，空types，无title正常读取 | title/纯markup/静态set text在metadata前有正向root资格；let/import前缀未证明binding则普通源可保存但不认证root |
| S-02 | id direct lowercase UUIDv4，types singleton comma | id计算/大写/其它UUID版本、types string/none/spread：控制错误，不自动修复 |
| S-03 | raw/string/comment中的`<weftext>`不计Label | 两个实际root Label同值或inactive branch重复：歧义，无last-wins |
| S-04 | 声明后shadow metadata，合法include/body仍能读root literals | import/function/dynamic label产生metadata不能当root；不能靠compile query认证 |
| S-05 | dict key原生identifier与quoted string准确解码 | id重复别名、dynamic key/spread：没有可用控制资格 |
| S-06 | unknown合法type保留且局部schema unavailable | alias+fullname同义重复、类型冲突：不当空types |
| S-07 | create/template/copy fresh ID，与birth/format同plan；move/restore保原身份 | ID外改、同库碰撞、partial metadata、unknown seal：不合并、不重签、不重抽UUID |
| S-08 | string/bool/int/finitefloat/none/array/dict逐字段准确类型 | bool↔int、float↔decimal、超域/nonfinite/错schema不强转 |
| S-09 | `()`空array、`(:)`空dict、("a",)单array、重复phones保持顺序 | `(value)`不当array；空/none/missing不混；不去重同值 |
| S-10 | 未绑定topkey、未知nestedkey字节保留，未选Field正常读取 | 不删未知字段、不认相似key、unknown schema不空成功 |
| S-11 | 好phones邻接formula computed字段可独立AST读 | formula/spread内数组整体computed_unavailable，不截断、不写派生值 |
| S-12 | date/time/localdatetime标准constructor与instant/decimal文本按schema | today/日期算术无静态Field；datetime不猜时区，不float截小数 |
| S-13 | 两type共用一个Field；required OR，统一definition | 同key不同Field/type/default冲突在schema安装拒绝，无source-order override |
| S-14 | schema default仅新建时写实际value | missing/invalid旧source不自动default，generation相同digest不同拒绝 |
| S-15 | 人员与组织的姓名、单位、职位平铺，标签与备注使用普通字符串 | 不自动生成事实断言、逐值UUID、置信度或语义代码标签 |
| S-16 | phones第1项同值仅改准确number span，全文其它字节不变 | 第0项删/重排后旧第1path拒绝，不跨rev重绑定 |
| S-17 | 新增/删除/reorder按完整容器前像与准确trivia预览 | 逗号/注释归属不明确拒计划，不能全文件format |
| S-18 | company文字无target lookup；explicit company_ref读稳定NodeRef | 组织名/结构parent不猜业务relation；ACL/target未知不空成功 |
| S-19 | Calendar/decimal业务服务原生字典/string保精度与proof | 缺rule/tzdb/范围不猜local defaults或声称complete |
| S-20 | current rootLiteral status有获权static+actual无外部依赖证明可窄写 | introspection/body read/未知use证据则窄不可用，不能grep放行 |
| S-21 | Field-only输出仅获权字段+聚合资格 | 隐藏正文/negative范围差异不得泄露path/offset/原因；不得隐授Source权 |
| S-22 | 单Draft Undo、源码完整合法import/functions保存，与rich资格分离 | 宏/生成/多源island只读不拒普通合法source；compile失败不冒旧preview |
| S-23 | 当前唯一token splice，重解析值+非目标bytes/AST不变 | 注入控制/第二label/改变key或stale observation：原稿保留零安装 |
| S-24 | 新Typst carrier/profile pin，真实旧records按原decoder恢复 | 不复用AsciiDoc H/parser、不读时升级、不虚构迁移 |
| S-25 | 任意官方合法正文与native计算保持Source入口；日期float补齐产品案例 | 研究静态allowlist不升级成语言限制；原32研究检查不算本产品验收 |
| S-26 | §7.2完整seed确定性生成、管理员获权安装后task/default/project/aliases才供表单与Query；白云山2026的显式白云山别名 | alias不自动安装seed，缺schema不默认open，旧catalog同ID不改摘要 |
| S-27 | int(Str)准确i64MIN；两个相邻大于2^53整秒duration保持不等；decimal精确构造 | 负数字面实际float不能当Int，.seconds()f64相等不证明duration相等；单位/中间累加超界拒静态Field |

## 11. 官方依据与证据边界

语法/数据引用以固定 [官方实现 v0.15.1](https://github.com/typst/typst/tree/v0.15.1)、[AST](https://github.com/typst/typst/blob/v0.15.1/crates/typst-syntax/src/ast.rs)、[Source](https://github.com/typst/typst/blob/v0.15.1/crates/typst-syntax/src/source.rs) 为准。[Metadata](https://typst.app/docs/reference/introspection/metadata/)、[Dictionary](https://typst.app/docs/reference/foundations/dictionary/)、[Array](https://typst.app/docs/reference/foundations/array/)、[Datetime](https://typst.app/docs/reference/foundations/datetime/) 说明原生机制；本文件的root角色、schema、ACL和应用身份不是Typst自带能力。

本轮作者以隔离官方0.15.1 CLI实际编译本文四个中文示例，并query核对原生值；另编译title/set在metadata之前、int(Str)的i64MIN、decimal("0.30")、大于2^53整秒duration和month_day组合样例，五项compile/query均exit0。query仅验证native示例语义，不用于本协议identity认证；未做PDF视觉验收。

先前研究已实测官方AST直接元数据、同值第二电话token改写、非选字节/逆splice、正文compile错误分层及有限正文读写；这些有限证据没有验证本文新增float/datetime/decimal/duration/schema合并/权限/同步/新carrier。本文不宣称真实用户资料迁移、完整rich editor或事务/身份认证已实现。
