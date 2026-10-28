---
source_language: zh-CN
translation_status: source
---

[English](SOURCE.md)

# Typst 作者源与集中 schema

状态：**design-refinement-not-implemented**；固定旧基线的独立接受保持为历史事实，当前修订及证据限制见 [REVIEW](REVIEW.zh-CN.md) §8。

## 1. 作者源与角色

一篇笔记有一份 `.typ` 作者源。稳定笔记 ID、集中 schema、独立批注、资源及真实目录的权威由协调 Core 设计确定。本文只定义源角色、值语义及消费边界；不复用旧 Pin、Proof、Entry、carrier 或 SourceRevisionPlan。

标准 `#metadata((id: "...", types: (...), ...)) <weftext>` 携带作者属性。`id` 是 Core 分配并在成功创建事务后承认的 lowercase UUIDv4；它不是权限。改名/移动保持 ID，复制/作为新笔记导入取得新 ID，外部改变 ID 是待处理身份冲突。主源路径来自 Core 目录权威，metadata 不重复存 path/parent/children。缺 ID/部分同步不自动造 ID。

官方 Rust `typst-syntax` 是唯一 parser。AST 检查应用角色；不做正则头部解析、第二 lexer、YAML/JSON 嵌入方言。保留准确 UTF-8 字节、BOM、注释、换行、字面量拼写及未知值。根源、依赖文件和编译输出有各自版本；输出不能成为新作者权威。

根角色按以下顺序识别：

1. 解析完整准确源。直接根 markup 层的裸 `metadata` 调用，恰一个 positional dictionary 参数，后跟精确 `<weftext>`；中间只允许空白或注释。不能套 context/function/loop/conditional/include/show。根声明不强制第一行。
2. 证明该位置裸名称仍是固定官方标准库 `metadata`。实现从根 AST 检查此前作用域：无前置 binding/assignment/import/include/show/context/任意函数调用；允许 trivia、无嵌入代码的纯 markup，以及明确静态的标准 title/set（参数树没有用户代码或 binding）。不确定前缀时根角色不可认证，保留合法源。默认创建把声明放在第一个代码表达式，以便简单而可证明。拓展安全前缀须另加 AST 反例和官方语义实验，不能执行前缀猜结果。
3. 完整根AST中满足上述direct根调用位置和精确label的应用控制候选恰一个；第二direct候选使声明歧义，候选binding/字面资格无法确定时控制认证不可判断，不取首项。函数/未执行分支、include或动态产生的同名metadata/Label不成为direct根控制权威；普通字符串/注释/raw也不是声明。后续遮蔽不改已识别位置binding。原生label冲突可导致独立编译错误，不能用runtime query决定笔记身份。
4. 根 dictionary 的直接 named/keyed 项，key 用官方 AST 解码且精确唯一。`id:` 与 `"id":` 重复也拒绝；不允许 spread/dynamic key。`id` 是 direct string；`types` 缺席表示空类型集合，存在则 direct string array，(), `("task",)` 分别为空和单元素。计算的 id/types 不能认证。
5. ID/types 是控制键；其余业务键由 schema 解释。大小写和 Unicode 规范化不自动合并；保留 `weftext-` 前缀。语法破损导致无法排除第二声明时结果为不可判断，不能“首个获胜”。

安全前缀另接受 §1.4 的规范受管编号原生头部：集中 generate／parse 完整逆认证、标准 binding 与精确范围均成立后，只将整个已证头部作为受信例外，使其后的既有根 metadata 保持身份认证。任意改动或未知函数／show 不获得此例外，不放开一般程序前缀，也不把标记注释变成第二权威。

合法 Typst 不符合受管根角色，仍可作为原文打开和保存；已有笔记进入保稿/明确修复状态，不被索引器重造身份。完整语言与可编辑字段子集分开。普通正文可以 import、函数、计算、复杂表格；源保存、控制认证、字段校验和编译分别报告结果。

文档 title/subtitle/heading 的角色由编辑器协调设计拥有。name、论文正式标题和文档 title 各有明确语义；文件名只是显示回退。若论文正式标题即用于文档 title，可从唯一 publication 字段生成 title，表单只修改这个字段，不要求双写。生成 title 没有伪造的源码可写地址。

### 1.1 保存 Query 的直接声明角色

保存 Query 使用同一官方 AST、标准metadata binding证明及静态原生值解码，角色为根markup层直接 `#metadata((kind: "weftext.query", key: "...", binding: ..., scope: ..., types: ..., where: ..., select: ..., order: ...)) <weftext-query-work-phones>`。FEATURES拥有scope/types/where/select/order及其他查询字段的闭合定义；SOURCE不重复存CEL/Query AST。kind和key必须direct Str，key为Core在明确新建定义事务中一次分配的lowercaseUUIDv4，完整身份是owner+key，不是另一笔记ID，也不授任何权限。已有裸外部key须经Core admission，不能仅看字符串接受出生。

直接metadata调用的形状、dict精确key唯一、无spread/dynamic key、完整源可判定以及binding证明与§1一致；查询数据树必须全部属于§2静态域。其前缀除了§1安全集，还允许已认证且所有参数值树均静态的标准metadata声明（包括根weftext声明及其它query），不能把computed根值执行后当安全前缀。参数树中任何用户调用/binding使本次定义资格不可用，保原文不执行查询。

定义必须接一个精确原生Label，标签保留`weftext-query-`前缀且在全根直接应用声明的Label中唯一；后缀可编辑，key保持不变。所有直接query角色的key在同owner准确根源中精确唯一；重复即歧义，不last-wins。nested/include/generated metadata、动态label、字符串中的伪声明不获得key身份。已识别的直接key也不能覆盖不完整源中尚未排除的第二声明。缺label/重复key/遮蔽/计算声明可保合法源码，但不成为应用可执行保存定义。复制按Core冻结owner/key映射分配新key，实际保存的定义引用同事务更新。读取current/pinned、参数/求值/导航语义由FEATURES拥有；任何历史定义都不复活writer。

### 1.2 原生引用的持久稳定关联

作者源可含一个owner唯一的直接声明：

```typst
#metadata((kind: "weftext.citations", entries: (
  (key: "chen2026", work: "weftext-node://WorkspaceId/NodeId"),
))) <weftext-citations>
正文保留原生 @chen2026 或 #cite(<chen2026>)。
#bibliography("weftext.citations.bib")
```

角色的字典闭合为kind/entries；CitationBinding/CitationEntry定义在集中schema中，entry恰key/work。key使用citation_key codec，work为稳定NodeRef且typed引用生成要求library/work（含paper）目标。采用§1/§1.1同一directmetadata/静态树/标准binding证明，完整根直接应用声明中固定`<weftext-citations>`恰一个；entries的key精确唯一，空数组可用。声明没有另一UUID/定义key，身份只是本owner的唯一角色，不是第二文献库。未知成员、重复key/label、动态或遮蔽声明保合法源，但应用关联不可用，不取首项；无此声明的普通native引用不按同名猜Work。

插入引用时先获权选稳定Work，将entry和原生citation写入同Draft/Plan；既有key→同Work可复用，另Work不得占同key。publication.citation_key只提供新entry建议，作品后来修改建议不改变现声明。外部改entry是显式改变稳定目标，admit核完整声明和目标；缺/撤权/dangling不能假作完整引用成功。复制由Core重映实际复制Work的NodeRef，保原生key；合并owner时冲突需在预览中显式分配新key并改所有已证明的native引用，不扫描raw/字符串猜。跨库缺目标/依赖保源并报不可用，不自动造作品。

每次compile从声明和获权当前Work publication快照生成一个冻结的只读虚拟`weftext.citations.bib`资源，按entry.key输出，保作者顺序和目标NodeRef依赖；重启可由两份真实作者输入重建，不保存手工可改Bib镜像。当前版不承诺逐entry pinned版本。真实作者资源同名、键重复或输出缺必要数据时明确失败/损失，不覆盖文件；独立native bibliography仍是普通资源，不自动合并。

识别应用citation须证明官方标准cite binding及静态Label key；普通`@key`是native ref，必须用准确获权编译语义确认它实际解析为citation，不能因为entry同名就把heading ref当作品引用。复杂/计算cite仍保合法源，仅在明确只读生产者给准确分类时消费。正文Label与Bib key冲突使官方编译报歧义，应用也不能猜。此角色只持久关联，不授读取Work或源码权限；生成器/导出/反向引用受FEATURES和Core快照授权。

### 1.3 脚注、尾注与题注的源接口

脚注正文使用官方 `#footnote[内容]`；重复引用给定义加唯一原生 Label，以 `#ref(<label>)` 引用同一注释。脚注／尾注正文及题注都是同一 `.typ` 中的作者内容，不进入审阅批注 JSON。图／表题注使用 `figure` 的 `caption`、`kind`、`numbering` 和位置；公式本体与描述性题注分别控制，默认不同时开启两条编号序列。

定版尾注模块是产品随附的 [weftext-endnotes.typ](../../crates/weftext-export/assets/weftext-endnotes.typ)，不依赖联网包或隐藏本机模块：

```typst
#import "weftext-endnotes.typ": endnote, endnote-ref, endnotes
正文#endnote(id: "stable-key")[中文尾注正文]
重复引用#endnote-ref("stable-key")
#endnotes(title: [尾注])
```

`endnote` 定义一份正文和首个标记，`endnote-ref` 只引用既有唯一非空 key，`endnotes` 输出一次文末列表；默认标题为“尾注”，`title: none` 隐去标题。原生 metadata/counter/query/link 依定义顺序独立连续编号，列表编号回跳首次标记，不合并同文独立注释。有界接口覆盖空正文、多段、强调、列表、链接、已支持公式和脚注共存；分节／分章重启与任意第三方尾注宏不因此获得支持。准确模块字节作为获权资源随独立 bundle 交付，文件名固定为 `weftext-endnotes.typ`，首次提供模块仍走同一 Core 创建／资源计划。

交换角色来自同次真实 Typst 生产者。脚注 reference/body 由真实声明 location 共享 `note-id`，Html 的 ARIA `doc-endnotes` 不能把脚注变成尾注。题注由实际 FigureKind、figure location、counter 和 position 取得 `target-id`，作者 `caption-body` 与生成 `caption-number`／`caption-prefix` 分开。尾注只在实际生产者源字节匹配随附模块、相关 metadata 来自该模块且 show／style 资格成立时认证；改变模块或同名遮蔽保合法源与显示，不授自动结构交换／回写资格。

`RenderOutput.sourceInfo.notes` 是获权冻结编译的来源投影，含准确主源范围和 raw，按源码权限交付。新 writer 以原生 `#box[#endnote(...)[...]]`／`#box[#endnote-ref(...)]` 的准确 AST recipe 保每处标记来源；旧未包装定义可凭真实 Content.span 编辑正文，旧引用缺准确来源时只读，不按顺序、邻近或同文配对。`Engine::edit_notes_frozen` 只消费同次获权主源／模块文件，修改后用同一文件集合重新真实编译。删除定义而仍有引用须同一操作迁移正文及目标，缺证据则保输入；source-only 候选 map 不能替代编译依据。

DOCX 输出真实 footnotes/endnotes 部件、各自关系、正文标记和重复 NOTEREF；导入按部件自己的关系读取正文，缺失／重复 ID 不猜归属，准确原件保留。`analyze_docx_import` 只列实际 SEQ 与真实对象候选；闭合 `DocxImportOptions` 为 `contentHash,captionPairings`，每个配对恰 `captionBlock,objectBlock,kind,captionText,position`。非空 hash 须匹配准确原件，一一配对随创建 Plan 冻结；未确认配对保文字与损失，不由 Caption 样式／邻近位置自动认定。确认的 captionText 是明确选择的纯文字，原富格式和复杂域选项另报损失。编号更新、分页、跨部件审阅与真实 Office 编辑保存重开分别验收；原件存在不代表正文／结构已进入新文档。

真实定义与重复标记还分别携带 `data-weftext-note-definition=true|false`，来自实际 Footnote.is_ref 和准确模块的真实 metadata 事件协议。前向重复引用只输出指向真实定义的 NOTEREF，不把首次显示位置变成新定义或改变独立注释的编号顺序；缺唯一真实定义／引用证据时明确未转换正文，不猜首项。

需要提交依赖证据的消费者使用 `edit_notes_frozen_with_dependencies → {edit,dependencies}`：合并当前编辑资格与修改后真实编译实际读取的 loaded／unavailable 依赖，不把未使用 snapshot 文件算入。合成入口按真实 LocalWorld compile FileId／override 分类为 derived-source／derived-file，普通 style 作者源仍是 source／file，并由现有 Core 当前性 guard 核验；这份可丢弃投影不成为新数据权威。

### 1.4 数学与受管编号的冻结来源

数学作者源仍是原生 Typst。已支持 OMML 导入生成新原生数学源与逐字符／结构范围，整体公式与符号／子式批注分别定位，重复 `x` 不按文字搜索合并。导出只消费同次 MathML 实际生产者范围，同源祖先链取覆盖完整对象的外层目标，重复生成或缺准确来源时保讨论并报告未进入目标 DOCX。合成多字符子式范围须同文件原生 span 严格有序、无重叠、间隙仅空白且存在完全一致的官方 AST 祖先。未知 OMML 结构保准确原件，不递归折为貌似可编辑的普通文字；布局空格、字体、特殊公式与审阅内容的损失和结构成功分别报告。

编号控件写原生 set/counter/show-set，`// weftext:heading-numbering` 包围的规范头部没有独立权威。只有完整逆解析再生成逐字节一致、binding／尾部规则资格成立，且本次真实 heading 的有效 numbering function span 属于主源规范头部时，才产生 `data-weftext-managed-numbering-pattern` 和同源 hash；最大层级外另发 `managed-numbering-visible=false`，排除标题保真实 `numbering: none`。DOCX 据此生成原生标题编号、实际起始覆盖及排除，中文“一／壹”分别使用 `chineseCounting`／`chineseLegalSimplified`；未知或相似自定义函数继续保源和明确损失，不从显示结果猜方案。个人辅助显示不写作者规则或改变输出。

本接口已用产品 Engine Html／Paged 和官方 Typst 0.15.1 独立 PDF／Html／PNG 编译及拒绝用例验证；DOCX 源／部件／域与数学范围有定向程序测试。这些结果不等于真实 Word、WPS 或 LibreOffice 双向编辑保存重开通过，不改变当前版本完成／发布状态。

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
| partial_date / temporal_range | 原生记录，显式 calendar/precision；年、年月、月日和起止范围不伪造缺失部分。birth事件未知年份用 `(month: 5, day: 12)`；端点含义由具体类型声明：During 缺端为未知，显式 nullable 的 none 端为无界；Range 缺端为无界。不得用统一空值规则抹平这一区别。区间开闭、granularity 由 schema 声明。 |
| zoned_civil / recurrence | 原生记录含 local_datetime、IANA zone、可选 disambiguation；时区数据库/版本、重复规则与 DST 歧义处理由时间服务提供。规则字典不是另一个可执行作者语言，不把月/年转换成整数秒。 |
| node_ref / target_or_text | schema 指定的原生字符串或记录；只在该位置解码引用，不扫描像 URI 的普通文本。见 §4。 |
| list / record / union | 原生数组和字典。union 必须原生域不相交，或有固定 discriminator 成员；禁止 first-match。未知字典成员保原文且单独标 unbound，不能假装已校验或使相邻已知成员消失。 |

absence、none、""、`()`, (:)、0 和 false 不同。必填约束检查 presence；nullable 仅控制出现后是否允许 none。默认值只在明确新建/模板时写成真实源，不在读取既有笔记时补值。来源、时段、状态历史是对应类型的可选嵌套业务记录；不统一强迫每个值附置信度或通用事实账本。

静态读取不需要编译全文：字段里的直接数组/字典完整递归解析，复杂/计算表达式标 computed。有一项无法投影时完整该 Field 不可作为截断的成功列表；相邻其他 Field 独立可用。未知成员保全，不授 typed action。计算表达式合法，不自动物化/覆盖；编译得到的语义可提供只读投影，绑定准确获权快照和全部实际依赖，不能直接作为作者字段写回。Query 是否纳入某个计算字段必须由明确的只读计算生产者声明，不能以 metadata query 最终值偷偷变成作者权威。

## 3. 集中 schema 的闭合模型

schema是Core管理的标准JSON非执行配置，不是.typ嵌入语言。准确配置版本/授权由Core管理，笔记不重复schema或内部FieldId。

实际作者配置见 [schema.authoring.json](examples/schema.authoring.json)：provider 仅选择 `{id,implementation,data,limits}`，validator 仅选择 `{id,provider,operation}`。固定 family、exports、inputs、parameters shape、result 均来自随 Core 交付的受信内建注册表，不能在作者配置中覆写。下文闭合形状和 [schema.json](examples/schema.json) 描述解析后的完整模型；该文件由 [resolve_builtin_schema.py](examples/resolve_builtin_schema.py) 生成，是可重建的测试/审阅投影，不再手工维护第二份签名。数据、限额、实际 constraint 参数及所有业务类型仍可配置。

本设计阶段 [builtin-abi.design.json](examples/builtin-abi.design.json) 是唯一内建签名输入，尚不是已编译实现。生产 Rust 落地时由同一受信注册源产生实际导出和检查用描述，替代手工设计镜像；未知操作/ABI、签名覆写或缺必需数据仍拒绝。Core 的 ConfigRevision 固定实际作者配置及其解析所依赖的注册版本/数据，历史基准不能在升级时偷偷换注册含义。

Schema顶层恰为 `version, definitions, fields, types, aliases, relations, validators, providers, series_policies`。version=1。除作者值Record的明确unknown策略，配置对象未知成员/版本一律拒绝；JSON对象重复键拒绝。所有ID/key/alias精确唯一，不做case/NFC规范化。Core配置SchemaRef/RevisionId/安装权限和字节摘要属于Core，不额外建立总账、签名树或自报出生。

Shapes保留 scalar/enum/list/record/union/use。Scalar闭合codec为 `text,nonempty_text,phone,email,identifier,doi,url,language_tag,citation_key,node_ref,resource_ref,tag_ref,schema_ref,config_revision,bool,int,float,decimal,exact_decimal,duration,exact_seconds,date,time,local_datetime,instant`。schema_ref/config_revision只用于保存应用定义的binding；Core解析准确身份，不把普通相似文本当引用。Record恰 `{kind,members,unknown,constraints}`；unknown为preserve或reject，preserve留未知作者成员为unbound，reject用于应用QueryBinding/validator参数。配置结构自身不会利用preserve放开。

成员恰 `{shape,required,nullable}`；Field恰 `{id,key,shape,nullable,default}`，default为absent或native source常量；Type恰 `{id,fields,requires,conflicts,constraints}`；fields项恰`{id,required}`。与原方案一样共用Field一份值、required OR、类型组合无优先级。Person/Org/Work业务字段全可选；Task.status必填；Period.period和Range.range必填；Event requires Range、不复制第二份时间字段；paper requires Work。普通项目仍是Task.project的普通笔记目标，没有新Project必填schema。

Union discriminator闭合为null、member(keys)、tag(key)、record_presence(key,present,absent)。null必须原生域不相交；member恰一个指定分支成员出现；tag精确命中唯一分支enum。record_presence仅选择记录分支，present/absent是variants的准确索引，其余非record分支先按原生域区分。EventTime是date、instant、CalendarDate、MonthDay：record有calendar选CalendarDate，否则选MonthDay，避免unknown-preserve令完整年月日同时匹配月日分支。选中分支仍完整校验，不first-match；非法额外calendar不能被当作普通MonthDay忽略。

按原生域选择分支时，零匹配报告值的类型不符合任何分支，多匹配报告分支不唯一；两者均为invalid，不能互相误报或降成unavailable。恰好一个匹配仍须完整校验所选分支。Task.due中的无时区`local_datetime`因此保原值并报告无匹配分支，不能补设备时区或扩成instant；显式偏移缺失的string则属于已选instant分支的值校验失败。

输入路径由官方已投影的值树定位，非代码：Path是有序 `{kind:"member",key}` 或 `{kind:"each"}` 数组。Constraint恰 `{validator,inputs,parameters}`；input selector恰 local(path)、field(id,path)或context(name)，分别表示当前record值、Type拥有的已知Field或Core固定owner。context只允许owner和resource_metadata；后者是Core对本次显式resource输入产生的获权资源元信息Cell快照，不是任意workspace读取。Validator恰 `{id,provider,operation,inputs,parameters,result}`：inputs逐slot有闭合签名，parameters是unknown=reject的record Shape，result固定valid-invalid-unavailable。安装时slot集合精确、selector路径类型吻合、参数精确匹配；unknown规则/operation不得运行。这里没有作者函数、表达式、分支脚本或任意读路径。

局部validator的读取闭包仅本record/Field；Type validator显式列出跨Field。读取/动作/窄写按闭包获权；不能只检查目标Field再偷偷读另一个隐藏Field。本次操作必要类型/shape/use/constraint依赖不完整就unavailable，不当空成功。字节保存与typed操作状态仍分离。

窄写通过同一静态闭包检查接入现有纯Field的`event`和`task-reminder` Type validator。编辑due或reminder必须在读取作者值前同时取得两个Field的授权，即使当前reminder缺失或其分支不用due。候选源码由共用Type validator重新校验；context selector和其他operation仍为unavailable。

创建模板在共享写入器、本地及托管入口全程只接收准确FieldId，native key仅为源码输出地址。合法的跨FieldId/key碰撞不能重定向、重复或代填另一个Field的值或授权。每个必填或有native default的Field仍须在默认值/字段值校验前独立获得原授权。仅用native key提交的创建输入被拒绝。

```text
Scalar {kind:"scalar", semantic:closed-codec-name}
Enum {kind:"enum", values:[string...], open:bool}
List {kind:"list", items:Shape, min:nonnegative-int, max:int|null}
Record {kind:"record", members:{key:{shape:Shape,required:bool,nullable:bool}},
        unknown:"preserve"|"reject", constraints:[Constraint...]}
Union {kind:"union", variants:[Shape...], discriminator:
       null|{kind:"member",keys:[string...]}|{kind:"tag",key:string}|
       {kind:"record_presence",key:string,present:variant-index,absent:variant-index}}
Use {kind:"use", name:definition-name}
Field {id:string,key:string,shape:Shape,nullable:bool,default:
       {kind:"absent"}|{kind:"native",source:native-constant-source}}
Type {id:string,fields:[{id:FieldId,required:bool}],requires:[TypeId...],
      conflicts:[TypeId...],constraints:[Constraint...]}
Constraint {validator:ValidatorId, inputs:{slot:Selector}, parameters:closed-record}
Selector {kind:"local",path:Path}|{kind:"field",id:FieldId,path:Path}|
         {kind:"context",name:"owner"|"resource_metadata"}
Path [{kind:"member",key:string}|{kind:"each"}...]
```

definitions、fields、types、aliases等顶层容器的准确array/object表示以 [完整规范 schema](examples/schema.json) 为准；全部结构成员必需，包括空constraints数组。definitions有限无循环，aliases对象准确映到TypeId；key、ID和短名精确比较。创建default的native source须经§2受限官方解码及同shape/validator检查，读取不补值。此版无UI标签/提示配置，显示key不能改变语义。

安装先检查所有闭合成员/codec/无循环use，重复ID/key/alias、控制键注册、未定义Field/Type、requires循环、非对称conflicts、union重叠/错误判别、default/enum不匹配、selector路径/slot/参数类型及预算均拒绝；provider ABI/Data也按§3.1核验，再验证每个关系/系列及Type的读取闭包，不依赖安装先后。Scalar是固定可信host实现，配置不能创造codec。citation_key是ASCII全文匹配`[A-Za-z][A-Za-z0-9_-]*`的原生非空string，以便确定生成Bib与原生Label；publication.citation_key仍是普通可选作者text建议，创建绑定时必须通过此codec。phone/email保非空拼写而不证明可拨/存在；DOI/URL/language_tag按固定语法校验、不访问网络。closed enum未知值invalid，open enum未知string有效并标unregistered，可字面筛选/分组，但动作不猜业务含义。

多个Type无优先级求requires闭包、合并Field，required取OR。同FieldId恰一份定义/作者key，不同ID不能争同key；冲突不last-wins/缩成隐式交集。未知Type保token，已知不相交Field仍能读，完整校验unavailable；alias和完整ID映同Type的重复声明诊断。短名来自集中aliases，没有schema时不自动下载seed。

### 3.1 闭合 validator/provider 与实际算法

Provider恰 `{id,family,implementation,data,exports,limits}`。implementation只允许 `{kind:"builtin",id,abi}`，abi必须准确匹配受信id的固定版本（本配置v1/abi1或v2/abi2），匹配Core已批准Rust实现目录与固定ABI，不接受作者代码/URL/module。exports项恰 `{name,inputs,result,effect:"pure",query_name:string|null}`；inputs/result是固定本稿接口标签，必须与实现导出签名逐项一致，不是作者可任选的自由类型字符串。validator inputs/parameters也与该operation的受信内建签名逐项核对，不能自报名为int-range却接受text。query_name全schema唯一；primitive calendar.periodBounds没有query_name，仅temporal/rules的路由函数暴露它，禁止“第一个provider胜出”。limits恰max_steps/max_bytes/max_expansion，均正整数；所有执行在预算内无IO/network/clock，输入仅已获权值和冻结配置数据。配置不能自授扩展权限。省略/缺实现/缺批准数据时不能激活完整schema；原 source仍可保，受影响typed服务不可用，不猜默认值。

所有本版引用的14 validator均已声明，不依赖以后“注册”：

| Validator | 固定输入/参数与行为 |
| --- | --- |
| core/int-range | int或absent value；min/max i64、exclude int数组。absent通过；有值精确检查域及排除项。 |
| core/decimal-range | 输入为十进制值或缺席值；上下界 min/max 使用精确十进制文本 decimal_text。置信度 confidence 限制在零到一之间，不转换成浮点数。 |
| core/custom-code | code文本、custom文本或absent；parameter code="custom"。选custom须非空custom值，其他code不得把custom当另一个service身份。 |
| core/owned-resources | owner NodeRef、resource或列表/absent，以及Core已获权resource_metadata；参数owner_required为bool。avatar必须本owner，Work资源允许显式共享资源且不得冒其owner；缺当前获权资源证据unavailable。provider仅检已给Cell，不调用Core/文件IO。 |
| temporal/month-day | MonthDay完整record；月1..12/真实月最大日，允许2月29日，无虚构年份。 |
| temporal/calendar-date | CalendarDate完整record；精确选择已安装calendar key/version，对年/月/日精度检查实际成员与provider范围/闰月。缺低精度成员不补值。 |
| temporal/during | During使用TemporalEndpoint；闭合同粒度≤与混精度证明、缺端未知/none无界按§3.1.1，调用固定v2算法。 |
| temporal/range | start/end_exclusive record；至少一端有界，两端同域且start<end，半开。 |
| temporal/period | period完整record；series_key命中冻结政策，calendar/zone版本、kind/rule与政策一致；provider解析key，合法性与界限由真实规则确认。 |
| temporal/recurrence | recurrence完整record；frequency允许daily/weekly/monthly/yearly，interval≥1；weekStart已列码域；byMonth合法、byMonthDay为±1..±31且非0，Weekday ordinal±1..±5且非0；count≥1与until至多一项；rDates/exDates/exception onset不混date/instant，calendar/zone版本存在、fold earlier/later明确、gap=skip。selectors组合只选合法civil日期，不存在日期skip。 |
| temporal/exception | onset/action/range/title/status/note record；cancel不能带replacement成员；replace必须有range或title/status/note实际变更；onset永远原发生起点，不是改期后起点。 |
| temporal/event | range+recurrence两个明确Field；无recurrence通过；有规则需bounded range，date全日与instant对应规则域；anchor**只从range.start经冻结calendar/zone转换派生**，不另存同一时间副本。until/rDates/exDates/exceptions同base；replacement同base；date长度为民用日，instant长度精确elapsed seconds。 |
| units/quantity | quantity record；exact value/precision、单位必须在冻结unit表，precision如有非负；无默认本币/单位，不换算。 |
| units/measurement | kind+quantity record；height unit dimension=length，weight=mass。观测时间独立event_time，多个观测/来源照存。 |

形状先静态解码，再validator调用；Validator签名验类型不递归重新调用自身record constraints。若validation输入unbound/invalid/unavailable不得输出valid；unknown作者成员保持unbound且不加入已知读取闭包。Provider纯函数只允许本版 compareTemporal、duringDuration、containsDate、nextCivilDay、rangeOverlap、rangeDuration、periodBounds、expandRecurrence及calendar/zone primitive exports，FEATURES绑定这些准确导出，不接受任意名称动态执行。rangeDuration/duringDuration保years/months/days或exact seconds，compareTemporal按§3.1.1；开放/未知时长unavailable，不补“现在”。FEATURES.resolvePeriod先调用纯periodBounds再做获权Query/创建决议，不属于pure provider函数。

`expandRecurrence` 的自产诊断使用与 UI 同源的结构化消息记录（语义 key、有类型的命名参数、原始诊断及有界因果链），不返回已渲染的产品说明字符串。显示语言只作用于呈现，不改变 CEL 返回值、发生坐标、停止位置、coverage 或预算判断。作者事件成员及真实第三方原始诊断保持原值；缺少或无效输入不因翻译获得业务资格。

Provider数据闭合为none、unit-table、gregorian、calendar-table、zone-table。unit-table项只有key/dimension（此轮不自动提供换算）；gregorian含key/version/min_year/max_year。calendar-table有key/version/epoch_date/连续years，每年有year/有序months，每月number/leap/days；检查正天数、唯一(number,leap)、连贯范围、checked累计。给定epoch后按实际表转ordinal；范围外unavailable。可以导入获权非Gregorian数据pack，但不是仅填calendar文字就证明已支持农历。

zone-table含key/version/from/until/initial_offset_seconds/严格递增transitions，每项at/offset_seconds；at是精确instant，offset为有界秒，校验覆盖和连续性。toCivil查实际区间；toInstant求所有落在区间内的候选，0为gap(skip)，2候选按earlier/later选择；范围外unavailable。真实IANA数据由获准loader从冻结tzdb产生该表并保来源/版本，不能把设备当前tzdb当固定来源。完整数据包在Core依赖快照中获权；synthetic测试provider只证明表机制，不冒真实历法/IANA支持。

Gregorian builtin提供0001..9999、leap校验、ISO week-year/quarter，UTC表提供显式范围；机器规范schema含这些真实算法descriptor、一个有限synthetic nonGregorian calendar测试pack、units表。产品激活仍须匹配已批准实际实现；本文不假称Rust已有代码。Period scope政策闭合为library或node_subtree(root NodeRef)，multiplicity unique/many；这些是可移植非执行数据，不新增查询语言。calendar key+version、zone key+version同family内唯一，不能重绑已有数据版本。

频率phase与默认选择也是固定temporal/recurrence及temporal/event算法，不是作者provider脚本：interval为1..65535，count为1..2147483647，until不能早于派生anchor。anchor永远为base首候选，即使显式selector不命中；之后仅候选≥anchor。daily按anchor的civil日差，weekly按含anchor的week bucket差，monthly按month ordinal差，yearly按year差取interval整倍数；weekStart缺席默认MO，仅影响weekly bucket。

weekly无byWeekday时选anchor weekday。monthly无byMonthDay/byWeekday时选anchor day。yearly三个selector全缺时选anchor month，无day/weekday时选anchor day；有day/weekday而无month时遍历全年月份。同维度取并集、不同维度取交集，显式空selector没有候选（首anchor仍保）；byMonthDay负数从月末倒数，不存在日期skip不clamp且不计count。可选weekday.ordinal是本轮明确month内第±n weekday细化，仅monthly/yearly允许，yearly在各候选month内算而非年内序号，不误报历史已选。

instant base候选经冻结zone解fold/gap后准确UTC去重升序再计count；civil候选顺序不能当UTC顺序。结束count/until前须用已验证offset界及完整zone coverage证明未来civil候选的最早UTC下界；不能遇到第一个超until就结束。证明预算/coverage不足按unavailable/partial返回，不以当前窗口未发现当全series不存在。规则与例外成员证明必须共享这同一算法。

发生键统一为date原值或instant准确UTC值，不按偏移拼写分出两键。先从range.start生成B并应用count/inclusive until（count先于任何排除/例外），再并入RDATE集合R；R不消耗base count且可越count/until。每个B∪R原键至多一个exception，onset须证明属于B∪R；cancel与replace同键是source invalid，不让cancel遮掉坏replace。EXDATE可不命中，保其作者意图。统一decisionFunction对每个原键依cancel > replace > EXDATE > template决定结果；窗内和窗外移入的replacement均使用它，不能先排EXDATE丢掉replace。校验/扩展按同冻结calendar/zone和预算证明membership，超预算/范围不完整为unavailable/partial，不冒invalid或完整零。所有规则再按FEATURES窗口算法筛选实际发生区间，重复/来源保持准确。

系列政策闭合对象恰 `{id,key,calendar,zone,kinds,rule,scope,multiplicity}`，calendar/zone是准确ProviderId；kinds是已支持kind非空去重数组，scope恰`{kind:"library"}`或`{kind:"node_subtree",root:NodeRef}`。安装校验provider family/版本/规则及root身份；query scope授权/unique负范围证明属于Core，provider不会读取目录。

已注册period规则恰Gregorian `iso8601`和有限calendar-table `table-ordinal-v1`。前者key分别为YYYY-MM-DD、ISO week-year YYYY-Www、YYYY-MM、YYYY-Qq、YYYY，真实范围和week53按Gregorian算法检查。table规则year key为Y<canonical-int>，month为Y<year>-M<正ordinal>，day追加-D<正day>，quarter为Y<year>-Q<正ordinal>（按该年有序months每三项分一组，末组可不足三项）；week为W<canonical-int>，0从epoch_date所在Gregorian Monday起、每七civil日一bucket。month ordinal含独立leap month，不能只按number合并；bound由已冻结表/epoch实算，跨pack边界不可用。该有限表算法是明确新设计，不冒完整特定历法pack实现。输出bound统一Gregorian date半开范围；当consumer需zone instant界限时显式经冻结zone转换，不默补本机时区。未支持rule拒安装，不能让任意text驱动脚本。

#### 3.1.1 期间端点保留原知识精度

`TemporalEndpoint` 是 date、instant、CalendarDate 三个原生域不相交的分支；During.start/end 和 Range.start/end_exclusive 实际引用它。CalendarDate 保 calendar/version/precision/year，以及精度要求的 month/day/leap_month；year 不添月日，month 不添日。Yearless MonthDay 不是可排序期间端点。date 为完整Gregorian民用日，instant为精确UTC瞬间，两域不隐式换算。Task.start/due同样使用TemporalEndpoint，承接已选date分支的年/月/日精度；completed_at仍精确instant。来源accessed保本候选原定DateOrInstant；测量observed_at等已有EventTime保精度，不从字段重名擅自扩域。

固定calendar `dateBounds(CalendarDate)` 返回只读计算cell `{calendar,version,precision,ordinal,min_day,max_day}`：ordinal是该历法/版本内年序、累计有序month序（含独立闰月）或日序，min_day/max_day是此知识bucket覆盖的Gregorian日ordinal闭区间。Gregorian按真实闰年/月长，有限表按实际months/days和epoch checked累计。native date在计算时等价于固定Gregorian版本precision=day，双方可比较/用于同一期间；作者表示不改写。bucket首末日仅用于证明，**绝不写回成真实开始/结束日或给用户声称原本知道该日**。

共享比较 `compareTemporal(a,b)` 返回value(-1|0|1)或invalid/unavailable。已获权合法同calendar/version/precision端点按对应ordinal比较；同精度年月因此正常可用。同日精度的不同历法可以在两个冻结provider都可得时按准确Gregorian日ordinal比较；非日精度混历法/版本不按数字年份猜次序，返回unavailable。缺批准provider/覆盖/获权输入也unavailable；已知不合法日、额外已知精度成员或date/instant混域为invalid。instant按准确UTC值比较。

同历法/版本混精度端点采用计算区间[Alo,Ahi]与[Blo,Bhi]：若Ahi<Blo则-1，若Alo>Bhi则1，否则unavailable。不得因两个bucket重叠就当相等。内部含端点比较≤在Ahi≤Blo时证明true，在Alo>Bhi时证明false，其余unavailable；严格<在Ahi<Blo时true，在Alo≥Bhi时false，其余unavailable。同精度直接用ordinal关系（这是所记录粒度的比较，不是声称具体边界日相同）。Query普通结构相等仍保calendar/version/precision及分支，不将year/month去重成某个day；需要时间顺序/跨表示日等价时显式用此函数，不给任意record新增默认order。

调用级共同基准资格先于无界替换与三值合取。Core先为整个调用所需Field/point及准确冻结provider数据完成授权，任一输入Cell不是value或依赖不可得就走原invalid/unavailable/EvaluationIssue，不读取未授权内容。固定temporal builtin收集**所有实参实际存在且非none的有限端点**（containsDate另含point），完整解码并验证每个端点/provider/coverage，不能只验证随后某条不等式恰会访问的端点。全部为instant才能在准确UTC域计算；civil与instant任何混用invalid，不补host zone。全部civil且calendar/version相同可共基准，**无需同precision，也无需任意两个端点可证明先后**；不同历法/版本只有全部为完整day且各冻结provider支持时才能共Gregorian日ordinal，任一粗精度跨基准unavailable。native date按上述固定Gregorian day规则参加。资格只决定能否比较；混精度比较本身仍按bucket证明返回true/false/unavailable。

每个Range必须至少有一个有限端，所以rangeOverlap可从双方全部有限端确定共同域，起点/终点在同侧也要检查。只有这个前置资格成立后，缺端才成为该共同域内的无界。During的缺端仍unknown；两端none时containsDate从已验证point取得域，不要求增加作者domain字段。单个Range的shape/local准入仅证明该项有效，**不证明两个range实参共域**。`schema.json`与ABI的`rangeOverlap.inputs=["range","range"]`签名保持；这个跨参数资格是受信固定builtin的必执行合同，不是配置自报已检查、额外作者属性或另一套schema。compareTemporal/内部关系、containsDate、duration及recurrence模板/窗口/替换使用同一资格步骤；各自原粒度、presence及精确要求不改变。

During有界端闭合：同粒度start≤end；混精度用上述≤证明，倒序invalid，无法证明有序unavailable且保原源/修复入口。端点缺席允许保稿但消费未知；两个成员都存在后，none分别为负/正无穷，绝不从缺端补none。containsDate先验证point、整个期间/端点存在及跨输入共同基准，再作 `start≤point && point≤end`；none的比较为true。混精度合法期间在远前/远后可确定false、明确内部可确定true，边界bucket无法判定就unavailable。已验证完整输入的两个比较用确定性三值合取：任一false给false，两true给true，其余unavailable；缺成员/非法/未获权输入不能靠false洗掉。

Range仍半开[start,end_exclusive)，缺端无界、none非法、至少一端有界；有两端须严格start<end。同粒度按ordinal准入；混精度依<证明，确知倒序/空范围invalid，不能证明非空unavailable。rangeOverlap先验证双方完整获权输入、各自Range有效性及上述跨参数共同基准，再判 `a.start<b.end_exclusive && b.start<a.end_exclusive`；**仅在已共同定域后**才将无界比较视为true并做三值合取。单侧开放civil/instant不能因两个无界分支而true；粗精度跨calendar/version不能被true或false洗掉。同基准的month/day两个only-start仍可true，不要求它们的起点可排先后。邻接不重叠，已合格基准上的边界不确定不当false。

duration先完整验证所有实际有限端/provider及共同基准，再处理无界/未知和单位选择；有界同calendar/version/precision的rangeDuration返回 `{value,unit,calendar,version}`，year/month/day的value为endOrdinal-startOrdinal，unit恰years/months/days；During的duringDuration为闭合粒度计数endOrdinal-startOrdinal+1。它们是所记录粒度的单位差/计数，**不是精确持续天数或秒数**。instant二者返回 `{value:exact_seconds,unit:"seconds"}` 的end-start（连续时间闭端不加epsilon）。native date与Gregorian CalendarDate(day)兼容；两个完整日的不同历法由此函数明确采用共同Gregorian日ordinal返回days（输出calendar/version为Gregorian），不是比较数字年份；原作者历法保留。开放/未知端、混精度、非日混历法或缺provider时长unavailable，不填now、不换算month/year成固定日数。

精确recurrence保持独立：模板、window及replacement在rangeOverlap前也需共同基准，不以单项ExactRange有效或开放window跳过；DateOrInstant不扩大，until/rDates/exDates/exception.onset仍只完整date或instant；Exception.range使用ExactRange，端点仍DateOrInstant并调用同半开range约束。temporal/event在recurrence存在时另要求原range两端都是完整date或同域instant、bounded且有序；CalendarDate即使day精度也须显式经获权provider转换/预览为精确模板值，不能自动物化。年/月模板、partial replacement或混域原发生坐标typed invalid，原合法Typst保稿，不产生虚构首日发生项。

temporal/rules与Gregorian/table calendar builtin选择明确v2/abi2，新增固定dateBounds primitive、compareTemporal及duringDuration签名在schema与ABI参考均已定义；其它本配置实现仍使用其准确v1/abi1；v2仅标本候选签名，不要求运行兼容未部署v1。配置不据数字选latest。保存定义/源原基准不能假作兼容升级，按§3.2–3.3显式核真实shape/函数合同，缺旧配置不重绑；不存在真实用户数据时不编造迁移已完成。

表单保精度选择和原成员，只为用户明确提升精度才要求补具体日；清空端恢复缺席，明确无界才写During.none。Query函数参数同TemporalEndpoint；例如March-start持续中对April完整日可true，对March中旬可unavailable，对March这个同精度point按月粒度true且明确标月粒度。导入保源calendar/version/precision，不能把导入年月强制写成Excel日期；DOCX显示“2020年3月（仅知年月）”，准确结构输出保原记录。XLSX有序附表以calendar/version/precision/year/month/day/leap_month分列，另保endpoint_presence与explicit_unbounded区别，缺成员不补0或1；只有完整Gregorian日且目标支持时可显式输出日期cell。Word/XLSX再导入需这些准确结构数据或用户确认映射，仅凭显示字符串不得推回更高精度。目标格式缺这些槽时附准确结构资料/原件并报告loss，不静默消除精度。

### 3.2 保存 Query 的持久 schema 基准

直接Query metadata新增必填 `binding: (schema: SchemaRef, revision: ConfigRevisionId, language: "weftext.query.cel.v1")`，其闭合shape为QueryBinding。它**就存在同一Query声明的原生dictionary里**，和kind/key/scope/types/where/select/order一起保存；没有另一份可编辑binding数据库/AST/map。SchemaRef用Core配置身份；ConfigRevisionId指其准确已安装不可变修订；SchemaRef字符串精确为weftext-config://WorkspaceId/schema。格式有效不证明存在或授权。

首次builder/new definition：Core一次分配key → 读取获权准确schema配置Revision → 使用固定CEL profile解析作者文本 → 从该baseline精确keys/aliases、relations/provider签名解出语义IDs及lexical局部变量 → 校验全部引用闭包/参数/类型 → 在同一源事务中写binding和文本、提交key出生。失败保Draft，unknown查原opId，不换key/基准。外部定义admit也须验证明确携带baseline及真实可得配置，不能仅key字面/当前同名alias接纳；缺baseline保原文但不可执行，提供明确rebind预览。

cold reopen/current definition：先获权读owner+key的当前声明（pinned定义则读指定源revision） → 获权取binding指向的准确schema revision → 固定parser/profile重建AST → 静态symbol绑定精确baseline语义ID。即使current alias已换指别的Type，也不重新按它解析。旧baseline不可得/失权/语言未知统一unavailable，不latest回退。AST、符号表、execution cache都是可丢弃投影。

执行读**当前获权数据**，按永久语义IDjoin其当前真实schema/源admission basis；名称/显示label可变而ID兼容，直接消费。Shape成员语义、target type/direction/cardinality/precision/codec变更不是仅label rename：若当前定义与baseline消费合同不兼容，明确definition-needs-migration/unavailable，不把旧表达式套到新同名含义。固定历史definition只固定作者文本+binding，不冻结数据、不授写。

修改Query文本默认保原binding，重新按它bind；添加baseline不存在的字段/函数需显式rebind到指定新schema revision，并预览每个语义ID/type/target差异。schema升级事务不得静默改Query binding；只有显式迁移计划一起改引用。复制产生新owner/key；若同schema域保binding，跨域必须携带准确schema revision或显式语义映射迁移，预览实际变更，缺定义依赖不能以同名alias猜。schema history GC必须检查当前/保留历史/回收站Query binding、模板和其他配置引用，仍可解析的引用不能删除baseline；若用户明确丢历史造成不可得，引用保留且报告unavailable，不重绑。导出可移植定义包含其准确配置依赖，未获准导出的依赖明确loss/不可执行，不补当前snapshot。

### 3.3 源值的持久 schema 基准

Core每个SourceRevision的CommitManifest/admission记录恰一个SourceSchemaBasis：SchemaRef及ConfigRevisionId。cold解析只据该源基准将作者key/type短名解为永久语义ID；schema变化不能令不变源字节变成另一含义。新建与外部admit先冻结获权基准并在同事务承认。body-only和既有字段更新默认保源基准；增加它没有的字段/Type或改名语义需显式schema/source迁移，预览重绑ID及原生名词改写。它不是第二份Fields/AST镜像，不是作者要每篇手填的schema。

消费当前源值须同时满足真实source basis及当前获权schema的显式保ID兼容映射；不能只凭同use名称或ID相同视为兼容。兼容检查展开definitions，并核对成员codec/precision、required/nullable、validator输入参数及ABI、关系target/direction/max、时间数据版本和series政策。仅显示名改动不变含义；新增optional成员可明确登记兼容。改变业务语义/读取闭包的迁移需新ID或准确获权预览，不静默复用废弃ID。source和Query的基准分别保留它们自己的事实：旧Query定义读当前真实获权值，绝非旧数据快照。Core readConfigProjection提供这些必要已获权定义；不可得依赖统一unavailable，不降级按当前同名key解码。

当前决定保留 SourceSchemaBasis 和保存 Query 的准确 binding：它们保护已有短名/别名可变规则下的冷读取与历史语义。内部配置版本共享去重，普通用户不逐篇填写基准。首代不增加任意复用已用技术 key/Type alias 的界面；显示标签可改。将来若采用不可复用技术词汇，可在证明不兼容变更、旧别名、历史 Query 与跨库映射后再缩减基准，不能只删版本而保留名称漂移。

### 3.4 WorkspaceSettings 的可选共享处理默认值

`.weftext-settings.json` 继续使用 `WorkspaceSettings` 角色、权限、修订及同步，不新增 locale 权威。外层 `version: 1` 和其它已有 JSON 成员保留；可选 `processing` 是闭合 `ProcessingDocument`：

```text
processing {schemaVersion:1,
  defaults:{version:"1",languages:[{language,script:null|string,region:null|string}],
    scheme,inputFormat,documentType,dependencies:[{id,version}]},
  rules:[{id,version,category:"typo"|"sensitive"|"character",text,message,
    replacement:null|string,scope?:ProcessingRuleScope}],
  scopedAllowlist:[{text,scope:ProcessingRuleScope}]}
ProcessingRuleScope {universal:false,languages:[],scripts:[],regions:[],
  inputFormats:[],documentTypes:[],dependencies:[{id,version}]}
```

defaults 版本是字符串 `"1"`，默认方案／输入／文类为 `general`／`typst`／`prose`，语言和依赖数组默认为空；rules、scopedAllowlist 可缺省为空。scope 数组及 universal 可缺省为上示值；缺 scope，或 universal=false 且语言／文字体系／地区全部为空，是待确认的 unknown，不静默全语言。显式 universal=true 不得同时限定语言／文字体系／地区，其它输入、文类和依赖条件仍准确匹配。缺 script／region 保未知，general 不绑定地区，依赖按准确 id/version 匹配，缺依赖单列不可用。未知成员／不支持版本不安装；`processing: null` 的显式更新移除此成员，保其它外层 JSON。

共享默认只在明确采用时成为本次处理设置。`ProofreadOptions.processingOrigin={source:"shared-library",baseId,revision?}` 随冻结源、规则／词典版本及 scope 绑定原获权配置；scan／未确认 prepare 继续核权限和版本，共享撤权或旧 basis 不得继续扫描／准备，已确认操作沿同一 Core 原操作恢复。单篇和局部排版语言仍只由原生 `text.lang`／`region` 决定，UI 语言独立；个人本地偏好不写共享配置。

初始化语言／地区也只改原生样式：本地创建界面从实际产品样式读取已认证的 `text.lang`／`text.region` 字面槽，预览完整源；地区未指定写为原生 `none`，不推断地区或简繁转换。准确 `initialStyle:{source,enabled}` 与路径、原 operationId 一起冻结，由 [CORE](CORE.zh-CN.md) §3 的同一创建 Plan 建立普通 Styles 节点、样式资源与 defaultStyle 选择器；不先建库再补第二次 locale／样式事务。后续单篇／局部覆盖继续遵循原生 Typst 作用域，处理默认值和个人界面语言不覆盖作者排版源。

存在 processing 时，schemaVersion／defaults 必填；defaults 的 version／scheme／inputFormat／documentType 必填。上述默认值用于明确新建，不表示读取缺项 JSON 时补齐这些必填成员。

### 3.5 识别导入的原件、来源证据与确认字段

v0.6.2 的 `ImportRecognized` 同时创建一个普通笔记、准确原件及 `recognition-evidence.json` 两个登记资源。原件字节不变；证据包含输入摘要、实际选择、后端／模型／规则与实现版本、逐页原始识别和来源几何、明确接受的部分状态及本次核对稿。证据只表示“导入时接受的历史事实”，不是第二份可编辑正文或当前字段值。原 ResourceRef 在 Core 分配最终身份后写入；目标 schema 修订与摘要也在原 Plan 内冻结。后续正文／字段修改不回写原件和历史识别记录。

确认字段可仅作为明确标识的可编辑正文，也可显式映射到已安装永久 FieldId 及适用 TypeId。无映射不得声称已有 typed 字段、Query 或字段级授权。首轮映射只开放可完整验证且不另需必填字段／跨字段约束的文字、准确十进制和日期标量；号码保文字前导零，金额不经浮点，日期不猜补。映射值仅存于原生 metadata；正文不再另写一份声称当前有效的相同字段。默认 schema 没有专用发票属性，不擅自创造业务字段或误用任务／文献字段。

方案字段必须来自本次适用方案的准确候选；缺失、歧义和解析失败不能自动确认。选择实际已解析候选与明确标记的人工修正分别保留；同一字段键或目标 FieldId 不接受互相覆盖。正文修改、顺序调整和表格确认仍是作者提案，必须能回到冻结块／表格来源；识别文字通过原生字符串构造写入，不执行其中的 Typst、脚本或链接。

## 4. 关系、文字回退与来源

`target_or_text` 是判别明确的原生 record union：`(ref: "weftext-node://WorkspaceId/NodeId")` 或 `(text: "云山公司")`。两者恰一分支；ref 分支不加文字 label 的同步副本。作者确有“当时名称/出版名称”需求时字段另命名 `credited_as` 或 `name_at_time`，语义是历史作者文字，不能被目标改名覆盖。显示 ref 时读取当前获权目标；目标不可读/失效时给通用状态，不借本地旧标题泄漏。text 分支保留作者文字且不按同名自动升级关系。显式选择目标才将 text 转 ref。

引用 URI 提供稳定笔记关系；不是任意磁盘权限，也不证明目标存在。target 的 type 要求是否强制是该关系 schema 的显式约束；不能所有 node_ref 都默认指向相同类型。删除/回收不自动抹关系，跨库复制显式重映实际复制的目标，外部目标保原引用/明确失效。

Relation恰 `{id,source_type,field,target_path,target_domain,target_types,direction,inverse,max_assertions,reject_self,graph_projection,cycle_policy,resolution,delete}`。目标域 target_domain 固定为 node_ref；目标类型 target_types 可为普通笔记 ordinary，或类型集合 any_of(ids)。方向 direction 可为有向 directed 或对称 symmetric；反向名称 inverse 为字符串或空值。关系数量上限 max_assertions 为非负整数或空值，自指拒绝开关 reject_self 为布尔值。图呈现 graph_projection 选择 general、ranked 或 family；循环政策 cycle_policy 选择 allow 或 reject。解析政策 resolution 固定为 live_on_typed_create_retain_dangling，删除政策 delete 固定为 retain_explicit_cleanup，分别保留悬挂引用并要求显式清理。

安装时路径必须能从Field shape到准确node_ref槽、source_type必须拥有Field、target Type均存在、max与实际shape一致、symmetric不可伪装有方向inverse。只有Person关系/任职及本轮Work未知作者使用Target的text分支；它没有ref路径，作为literal作者信息保全，不lookup/反向/图边。Org关系、Work venue/version_of、Event participant分别使用RefRelation/RefTarget，不允许普通文字绕过已选目标域；container文字仍独立且不自动生成venue。max_assertions先统计全部真实关系records（含允许的literal），再产生ref图边，不能因没有ref而忽略基数。用于typed新增关系的ref须获权且当前live并满足目标类型；仅保存未知/悬挂原文不冒typed成功。已经存在的失效目标保dangling，清理是新获权操作。

27条历史关系全部具体定义：Person十条（parent/guardian/manager/mentor四有向、spouse/sibling/family-related/social-related/professional-relation五对称、engagement有向普通目标）；Org十二条（allied-with/related两对称，其余十有向Org→Org）；Task dependency；Work creator/venue/version-of；Event participant。另有本轮已选Task.project有向普通目标。schema.json有每条路径/目标类型/反向/基数。

graph_projection只是呈现要求：parent/spouse/sibling为family，manager/mentor/engagement为ranked，guardian和其他为general。历史没有仅凭ranked强制禁止循环；完整已选配置cycle_policy均allow。图不能唯一rank时显示ambiguous/unranked，作者事实照存。若用户以后显式配置reject，安装仍使用同闭合槽且操作必须覆盖当前获权完整必要图scope；无法证明/隐藏依赖时统一unavailable，不以隐藏cycle决定公开错误。这不是此轮新强制业务规则。

每个对称assertion只在一个owner存一次，允许另一owner独立assertion保来源；graph可合边显示，但edit先选真实sourceOwner/Field/当前itemPath并重新获权，不能写合并边或代表行。复制先schema遍历全部这些准确ref路径，包含Work嵌套和Event记录，不扫描自由文字或CEL字符串。

路径walker先按Shape判别分支；Target.text产生literal assertion而非错误/整项缺席，ref+text是invalid，不能跳过。整体Field/member absent不造assertion。RefTarget不含text分支。list each保ordinal及sourceOwner/Field/itemPath，反向只有ref且需获权，不把partial当全量。

关系索引只是可重建语义投影。反向视图只列当前可读的来源记录及可披露目标，不泄漏隐藏来源的数量或名称；扫描不全为 partial。删除/复制影响只检查操作实际需获权的关系范围，缺完整范围不能声称无引用。历史任职时段属于源记录，反向 Query 保留同一记录的 role/期间关联。

`sources` 是可选有序原生记录数组：reference 是 SourceTarget（ref笔记/resource资源/url三分支），locator 为页/章节/原文位置，accessed 为 date/instant，note 为作者备注。来源可关联一个任职/历史事件/论文导入记录，不给所有普通电话强行新增来源。置信度/争议状态/测量精度如有实际业务需求，由该类型明确字段表达，不能重新引入一个必填通用 Entry。

没有跨版本逐值 UUID。表单数组行和引用观察依当前准确源版本+Field+AST 路径定位；删/重排/外部修改后失效，不能按同值重绑定。若将来确有独立分享一段任职历史、长寿命关系批注或外部系统键需求，优先独立笔记或显式业务键，并另行说明生命周期；本稿不暗藏 per-value identity。

标签的稳定身份来自 CORE 的集中 TagId 登记，原生字段只保存引用。新增闭合标量 codec `tag_ref`：作者字符串为 `weftext-tag://<WorkspaceId>/<TagId>`，两个标识均按 Core 的 UUID 规则解析；格式有效不证明存在或授权。可选通用 tags 字段采用有序数组，项是判别明确的 `(ref: "...")` 或 `(text: "研究")`；前者指已登记标签，后者是普通文字标签，不能按同名自动升级。ref 分支使用 tag_ref，text 分支使用 nonempty_text，二者恰一出现。表单选已有标签或明确创建时，可由同一 Plan 登记 TagId 并写入引用；标签显示名改动不改每篇源。精确 Query 以完整 TagRef 比较；显式后代筛选另消费 CORE 的冻结标签树，导航与快捷入口保存准确匹配模式，文字标签保原值。目录的 DirectoryRef 仅是 Core 的目标身份，不为每篇笔记新增目录属性权威。

多级标签不改 `tag_ref` 字符串、ref/text 判别、数组顺序或重复值规则，不在每篇 metadata 添加 parent/path/祖先槽，也不增加正文标签语法。父关系只由 CORE §2.3 的注册表保存；完整路径和匹配结果是派生值。跨库的父关系和所有真正 typed 槽使用同次 Core 映射，普通文字不按形似路径替换。后代谓词及其准确 profile／签名、冻结参数与输入依赖由 FEATURES §5.6 负责；可变 Tags 是执行数据，不能复制进固定 schema/provider 配置或定义基准冒充当前树。这是待实现合同，现有 schema 样例／设计模型不证明层级支持。

### 4.1 一般人物关系的分类权威

family_relations、social_relations、professional_relations分别用FamilyRelation、SocialRelation、ProfessionalRelation；不再把通用Relation.role/customLabel当分类。每个已填写项required target与relationship，relationship恰 `(preset:"code")` 或 `(custom:"作者称谓")`，两分支记录unknown=reject，preset closed enum：family仅extended-relative；social仅acquaintance/classmate/friend；professional仅colleague。码是本Field固定业务含义，显示可本地化；不需要全局关系登记。

custom是非空准确原文，大小写/Unicode/空白保留；`(custom:"friend")`不同于`(preset:"friend")`，不自动转码。跨域preset（如family.friend）、两个分支同时出现、空custom和未知preset typed invalid。三个一般关系不接受typed未分类项：UI新增时明确选预设或自定义，未选仍保Draft；raw源码可保存target-only或旧role/customLabel并标invalid/unbound、提供修复，不替作者发明分类。旧成员即便同名也不能隐式认成relationship；已选择的分类不可因换语言/导入/再打开丢失。

relationship分支与target.ref/text是两件独立事实。表单改称谓不改Field、target或方向；Query用`r.relationship.preset`或`r.relationship.custom`明确守卫分支，同名文字不混分组。导入无确证码时保custom原文/原件或未决稿，不按显示名猜preset；DOCX显示分类来源，准确结构输出/XLSX附表保Field、branch、preset码或custom原文以及顺序/期间/来源，重入据此还原，不仅存翻译标签。全部仍按Field声明为symmetric general、拒self-ref、单作者源、无语义inverse/代际rank；custom“manager/parent”不会改成有向Field。colleague明确断言与任职重叠派生来源分别保留，不自动双写。

## 5. 七类已选能力与原生代表源

完整规范配置为 [schema.json](examples/schema.json)，不是旧最小seed。原生 [native-cases](examples/native-cases/) 只是语法及设计oracle样例；不等于全部能力或产品已安装。

机器配置是准确字段/成员名、枚举、nullable/required、路径、默认值和关系定义的权威候选。下面不是增加占位名称，而是各组的实际shape及消费者：

| 类型 | 已定义Field与record成员 | Query/表单/导出消费 |
| --- | --- | --- |
| Person | 共用names/phones/emails/addresses/websites；accounts(service/serviceKey/identifier/usage/customLabel)；avatar资源；nationality_states/legal_sex_states/gender_identity_states(value)；professions(text/context)；engagements(employer/department/position/rank{system,level}/status)；life_events(kind/customKind/event_time/description/confidence)；measurements(kind/quantity{value,unit,precision}/observed_at)；九类人物关系 | 名称一份业务权威；所有有用期间/备注/来源保在同record，默认不必填。preferred/selected多项时显式歧义，不能最后一项胜出。生日从所选birth event或其MonthDay派生，不另双写birthday。任职相关条件同项绑定，测量单位量纲校验，人物文字target不造节点/反向。 |
| Org | 共用联系组；organization_states、identifiers、classifications、sites；十二类组织关系 | scheme/value独立，site不偷换website；parent最多一项，关系target使用RefTarget，不允许literal逃目标类型域；所有Org目标按Org类型校验；目录parent无互推，人员名册只派生engagement。 |
| Task | required status；priority/start/due/completed_at/reminder；dependencies数组record(target node_ref,during/note/sources/status)；project node_ref | status改动和完成时间是显式独立效果；dependencies目标Task，inverse dependent派生；project普通笔记，不强制Project类型。正文即详情，无镜像。单一提醒意图见§5.2。 |
| Work/Paper | 一份publication record：kind、identifiers、publication_history、resources、保序contributors(agent/credited_as/family/given/roles/affiliations)、venue、version_of、title/alternative_titles/issued/container/volume/issue/pages/abstract/citation_key/url/keywords/language/sources | formal数据一份；creator可ref Person/Org或显式literal，venue ref Work/Org、version_of ref Work，均最多1。角色/作者顺序完整，未知作者不强建人。完整引用输出操作才检查所需title/kind/authors，不反推所有Work必填。 |
| Period | required period含series_key/kind/calendar/calendar_version/rule/key/zone/zone_version | 集中series policy决定scope/multiplicity，不写第二份政策。period provider验证真实key→半开bound；week ISO week-year；先查已有，再明确新建。unique需Core完整current负范围和并发事务。 |
| Range | required range(start,end_exclusive)，TemporalEndpoint保年/月/日或instant、至少一端有界，两端同域且start<end | 开放端缺席，非none/现在/极值。半开overlap统一，duration派生。UI包含末日仅表示选择，保原exclusive端点精度。 |
| Event | requires Range；event_status/recurrence/participants/reminders可选 | recurrence、range相关Type validator明示读两Field；不用同一个status key与Task冲突。participants使用RefTarget，目标Person/Org、不允许literal逃域。reminder是意图，不实现送达。 |

每个记录仅按完整schema所列成员使用可选during/note/sources/selected，但不是另包Entry。avatar为可选Resource record(resource/role/note/sources/during)，Work publication.resources为同record有序数组，保逐资源来源与备注，不仅裸URI；资源元数据只由Core生产。Org sites为text记录，不偷换成address或website。During/Range端点、缺端、none、precision与所有消费者共用§3.1.1，不再限完整date/instant。CalendarDate有calendar/version/precision/year及按精度的month/day、可选leap_month；精度年/月不补日。非Gregorian也用此shape，由批准calendar provider验证范围；未安装provider/data不可用而不是Gregorian回退。精确金额/量值优先native decimal，超范围exact_decimal文本；量纲不隐式换算。

简洁生日面板读取明确所选life_events中kind=birth项，修改就写这一真实项的event_time，不写birthday镜像。无项可明确添一项MonthDay；多项冲突必须选具体项/明确追加，不last-wins。只有年/月精度时显示该精度，不为日历虚造月日；来源、冲突与selected选择在同事件保留。names只用preferred按角色/语言作名称选择，通用selected不成为第二个preferred权威。

### 5.1 七个已选类型的 source→target 能力协调清单

下表核对历史 `D4` §9 与其 历史机器目录 的已选业务能力；历史表示不成为新实现。本表和当前 schema 已完整列出已选业务能力，实施不需要读取旧目录；历史来源标签只用于溯源。Person、Organization、Work 的业务字段可选，Task status 必填；周期 period、区间 range 及 Event 复用的 range 是对应用途的必要字段。完整 Project 管理不在历史七类已选范围内；Task 到普通项目笔记的可选关系已选。

| 历史来源/已选能力 | 新作者源表示（SOURCE拥有形状） | 消费与约束owner |
| --- | --- | --- |
| D4§9.2：Person多名称，role/language/script/preferred、期间/备注/来源 | names有序records；单一姓名权威，不另复制name或core aliases | SOURCE schema；EDITOR按语境显示/修改明确选项 |
| Person phone/email/address/website多值，preset personal/work/other+customLabel、期间/备注/来源 | 四个独立有序records数组；label码与customLabel文字分开，重复值合法 | SOURCE；EDITOR常用行/高级折叠；FEATURES Query/导出 |
| 人员账号、头像 | 账号记录包含预设服务或自定义服务标识、账号、用途和自定义标签；头像Resource record引用归属本笔记的资源 | SOURCE 定义形状；Core 管理资源归属和复制；FEATURES 报告导出损失 |
| 人员国籍、法律性别、性别认同三种独立状态；职业与任职独立 | 三组独立状态记录，含值、有效期、备注和来源；职业记录可关联组织语境；任职记录包含目标、部门、职位、职级制度及级别、有效期、状态、备注和来源 | SOURCE 定义形状；FEATURES 定义时段选择与反向任职；不按末项猜当前，不跨职级制度排序 |
| Person life-events birth/death/employment-start/graduation/marriage/custom；measurement height/weight | life_events records显式event_time，保year/month/day/历法或instant及冲突来源；measurements含kind/exact quantity/unit/observed_at/precision/sources | SOURCE时间/量值形状；FEATURES历法/时间与量纲约束；confidence如确有来源用途可选，不统一必填 |
| Person parent/guardian/manager/mentor；spouse/sibling/family-related/social-related/professional-relation | 每含义独立关系数组：前4 child/ward/report/mentee→父母/监护人/上级/导师；后5对称。target ref或literal+期间/状态/备注/来源 | FEATURES方向、对称单份权威、反向与ranked图验证；派生colleague不生N²作者边；literal不生反向 |
| D4§9.3：Org names/status/identifier/classification/site/address/contact | 独立数组；identifier/classification scheme+value；site不是website偷换；每项期间/备注/来源 | SOURCE；FEATURES分组/时间过滤；外部编号不合并组织 |
| Org parent/governs/owns/allied-with/related/brand-of/business-guided-by/jointly-led-by/member-of/subsidiary-of/supervised-by/territorially-administered-by | 12个独立关系字段，Org→Org；只有allied-with/related对称，parent最多1，其余many；关系记录保validity/status/note/sources | FEATURES关系定义/反向/基数。历史多个parent的表达需显式重审max1范围，不能静默扩大；目录parent无互推；人员名册仅engagement派生 |
| D4§9.1：Task status/start/due/priority/completed-at/dependency；本轮project | status closed enum required，其余optional；completed_at exact instant；dependencies稳定Task引用；project普通Node引用、不强制Project type | SOURCE；FEATURES板/历/完成动作与inverse dependent；是否设置完成时刻进入动作预览；未选择完整status历史，不能虚构强制history |
| D4§9.5：Work kind/identifier/publication-state/resource/creator/venue/version-of；RQ10论文数据 | kind、identifiers、publication_history、resources、保序contributors、venue、version_of；论文publication结构补完整引用信息。出版状态有时间/来源。venue Work或Org最多1，≠publisher；version_of Work最多1 | SOURCE形状；FEATURES文献权威/引用/反向/导入导出。unknown作者可literal/结构化姓名，不强建Person；全部业务字段optional |
| D4§9.4：period-note period+series，scope unique/many | period record含kind day/week/month/quarter/year、calendar/version/rule/key、zone/tzdb，series_key；scope是集中系列政策，不在每篇源复制 | SOURCE形状；FEATURES周期解析/ISO week-year/unique并发与找已有；Core唯一创建事务 |
| range-note：精度日期或instant半开range，至少一端有界 | range record明确start/end_exclusive，same basis、start<end，开放端省略，不填当前时间/极值 | SOURCE形状；FEATURES时间服务/重叠/导出；UI可友好显示包含末日但源与Query仍半开 |
| event复用range；status/recurrence/participants/reminder-intent optional | 原range字段不复制；recurrence record含frequency/interval/weekStart/byMonth/byMonthDay/byWeekday/count或inclusive until/rDates/exDates/exceptions；participant target Person或Org+role；reminder意图 | FEATUREScivil/DST/fold/gap/规则/窗口/例外服务，SOURCE原生结构；单次cancel/replace保原起点、range/title/status/note；occurrences派生、显式promote才新建笔记；提醒送达后置 |

所有已选形状/关系/领域validator/provider/series均在完整schema给出，安装与调用按§3；.typ样例只检语法。原生event不另存anchor，唯一从range.start派生，规则/例外按§3.1。birthday是所选birth投影，完整Project管理仍候选。

历法/节假日pack是获权服务上下文，停用不删日期；不在源复制单isHoliday。ICS UID/RECURRENCE-ID/SEQUENCE是外部来源控制而非笔记身份；导入保原件/unknown property、附件与损失报告；subscribe/upsert仍需独立连接能力，普通导入不冒同步。完整HR/CRM、单位换算、国家职级pack、工商/文献connector、投稿/协同审稿、完整Projects/Assets/Places等仍候选或后置；不能从“所有类型”原则发明首发范围。Diary只是周期笔记/模板使用方式，不增平行身份。

publication.citation_key仅是新绑定建议，稳定引用声明见§1.2。DOI只存publication.identifiers的scheme/value项，不再另存doi镜像。Work与paper共享一份publication记录，paper requires work；kind、publication_history、resources、contributors、venue、version_of及正式引用数据都在该记录，不另建Library副本。DOI 是 external identifier，不自动成为笔记 ID，也不默默合并不同稿件/版本。导入按获权候选确认创建/匹配/更新；未知信息和源文件保留，未支持的引用字段列损失报告。已知值与导入原值冲突需要选择，不能覆盖。

### 5.2 单一任务提醒意图

可选 FieldId `tasks/reminder`、作者 key `reminder` 是一个 closed union；未填不提醒，`none` 不代替缺席。每个分支都有稳定小写 UUIDv4 `id`，这是提醒业务身份，不是所有原生值的通用行 ID。切换分支保留该 ID；复制任务仍须按新 NodeRef 在本设备明确启用。不得将推导触发时刻双写为另一可编辑字段。

| kind | 准确成员 | 语义 |
| --- | --- | --- |
| `absolute` | `id,kind,at` | `at` 为带显式偏移的 exact instant；可无 due，不随 due 改期。 |
| `due_before` | `id,kind,before` | `before` 为非负 exact_seconds 字符串；due 必须为 exact instant。用精确十进制相减，保留 due 和提前量的秒及更细精度。 |
| `due_day` | `id,kind,clock,zone,zone_version`，可选 `fold` | due 必须是完整 civil day；`clock` 为 `(time: native time, fraction?: exact_decimal)`，fraction 属 `[0,1)`；显式冻结 zone/key+version，fold 仅 `earlier/later`。 |

三个分支及 clock 均拒绝未知或重复成员。提前量上限为 128849018820 秒。年月精度、未知历法／时区版本、coverage 外时间、DST gap、歧义而未选 fold 和闰秒不能猜成可执行时刻。捆绑配置当前仅提供 `UTC/fixed-utc.v1` 时区；不存在宿主 IANA 数据回退。自定义冻结规则必须由同一批准 provider 资格化。唯一解析函数返回 ready/absent/invalid/unavailable、精确 UTC trigger、目标 civil minute 的 exclusive end 和实际 provider 指纹，不读取时钟或文件。

Type validator `temporal/task-reminder`（`temporal/rules` 的 `task-reminder` 操作）静态声明 reminder 与 due 两个 Field 输入；两者均至多一个值，不因分支绕过必要读资格。分支解析只将实际时间依赖计入定义摘要：absolute 不依赖 due；due_before 依赖准确 due 与提前量；due_day 依赖完整日、clock、所选 zone/version 与实际触发结果。无关正文、title、路径或整个 Schema 修订不成为执行键版本。Query、字段修改校验与表单共用这一语义；改 due 时会重新校验受影响的 reminder。

分钟辅助输入仅替换日期／小时／分钟，保留已有秒、小数和显式偏移；提前分钟使用精确整数运算并保留原秒余量。全天不变成宿主零点，语言与设备时区只影响显示。提醒随原作者字节走普通保存、历史与同步，设备启用、处理状态及 OS 提交结果不写入作者字段。Event 的既有 `reminders` 仍是事件意图，不据此触发任务投递。

## 6. 表单、Query、导出与安全编辑

字段面板来自集中 schema：常用组优先，可选组展开，数组添项/删项/重排；不使用的历史和来源不要求填写。作者字段、缺值、错误、unknown、只读计算分别显示。源码、表单与富编辑共享 Draft/Undo；保存前一份准确变更预览。UI简洁不裁掉嵌套数据。

Query 保留 scalar/record/ordered-list 类型，并支持同列表项的相关条件：例如 `exists engagement: employer.ref=O && during contains D` 必须对同一项成立；不能将两项组织与时间交叉配对。显式 flatten 产生 owner+当前 snapshot+数组 occurrence 的只读行，顺序列清楚；不是永久 row ID。any/all/count、空列表、缺值、unknown/invalid 均有明确语义，默认多值筛选不得悄悄只看第1项。复杂值默认无overall order；显式scalar仅同型可排序scalar，ref_identity比完整同型EntityRef；sequence_lexicographic仅同型可排序scalar数组，首差决定/同前缀短先。array<record>需显式已绑定scalar成员投影或min/max再排序；不可用值不当普通空。聚合/计算/反向关系结果只读；动作回到原笔记重新申请写入依据。

字段权限第一阶段以整个 top-level Field 为单位：嵌套行不是另一个默认 ACL 域。仅字段读取只返回该字段的语义和获准 raw 表示，不给全文 byte offset、旁邻字段、AST路径或正文。后续若需嵌套权限，要明确约束和unknown成员如何分权，不能仅隐藏控件。

结构化写入消费 Core 的一次性写入依据，冻结实际源快照/observer incarnation/file generation、schema 与当前权限。默认改完整 Field；精确子成员 patch 仅在当前快照 AST 的唯一静态范围及完整容器前像得到证明时使用。writer 按官方 parser/原生 constructor 回读拟议值，验证根角色、约束及非选字节一致；增删/重排的逗号和注释归属进入准确预览，不全文件 format。旧路径不跨 revision 重用，hash相同也不能复活依据。

完整源保存可以保合法计算 Field、未知 schema/成员和编译失败的正文，按 Core 记录相应投影状态。typed 操作不得把 known invalid/缺必填洗成成功；显式 raw/repair 保存保稿但不发布完整 typed 事实。编译失败不会冒旧 preview 为当前，也不阻止无正文依赖的静态字段查询。

导出/Query输入是只读语义投影，附当前性与完整性；不是写入依据。复开保存 Query/使用缓存/恢复布局都重新获权和求值，不复活旧写权限。导出 XLSX 可用主表+有序子表：owner列和显式序号维持 contributor/employment/contact关系，精确数字与标识按目标的明确编码输出，不能经Excel数值位数限制失真。

### 6.1 显式受控字段写入

完整源获权用户按上面的普通AST局部patch编辑，无固定容量/空白padding或静态正文要求。仅有Field权限的调用方还需Core显式受控字段写入和field_profile_use授权；资格及生命周期是可披露元状态，FieldRead不暗授观察隐藏正文变动。

管理员/完整源获权后台producer用官方AST认证指定direct根应用声明及其标准metadata/constructor binding、selected Field的唯一静态值范围和受影响schema约束输入闭包。它不要求整个body静态；声明后的import/function/query/动态内容仍可合法存在，runtime/include/generated metadata不成为控制或Field权威。参数解码只接受§2常量域，不能运行用户函数取得值。修改值必须独立回读为相同shape的完整静态表达式，不引入绑定/控制声明/Label；根控制id/types、角色唯一性与非选字节不变。

profile内部绑定准确sourceRevision/generation、SourceSchemaBasis/当前兼容schema、auth和唯一选中范围；公开只给批准Fields/受影响输入闭包/epoch，不给隐藏body、offset、大小或AST。闭包只包含被本次变更实际影响的具名schema约束：selected Field及其跨Field validator、关系target/资源元数据等显式输入必须全获权，不能为了窄改重新验证未受影响的隐藏Field或隐藏正文。未获得所需闭包资格时不给操作。服务不compile/eval隐藏body，不用隐藏值谓词决定公开validation。

最小受控实现为可信Server fenced writer：资格提前显式公布，窄请求仅校验获权新值及获权约束输入，消费opaque FieldBasis并用Core prepare/commit事务核实际sourceRevision与可信generation。内部作可变长度精确byte splice，非选字节逐字保全。正文若读取该Field，其隐藏编译派生值可能变化；这是既有源码对获权Field的依赖，并非对body作者字节的额外写入，窄调用也无权取得该派生结果。结构回读只核声明/patch不变性，不重新编译body后暴露诊断。

opaque FieldBasis的公开CAS比较获权Field前像、获权约束及已公开profile合同，不把内部完整sourceRevision变化直接报stale。Server遇到仅隐藏源改动时内部重新认证/重定位并以实际current revision事务fence；不能复用旧offset，且不能为了重基重新验证未受影响的隐藏值。selected Field或获权语义变化才给相应公开冲突。无法重新认证只按已授权可披露的资格生命周期/统一不可用处理，不成为“隐藏正文是否改过”的探测接口。

调用方不接收全文诊断、offset/具体进度/大小，使用Core统一异步Operation/Job与不可用结果；后台audit和重新公布资格是另一个明确获权操作。控制key新增/删除和缺唯一静态范围不可借窄writer完成。开放本地缺可信generation时，watcher未到/同fileID/hash相同不能证明隐藏body未变，需重新资格认证或完整源授权。无法满足获权约束闭包或可靠fence统一unavailable，不偷偷升级到整源权限。

本合同保证公开语义结果不取决于未授权内容，不宣称完美constant-time；内部拼接/IO残余时序需测量与部署评审。固定容量/padding和全body白名单不成为作者格式或默认协议。实际producer、并发/撤权/故障仍是产品验收门。

## 7. 验收与证据边界

| 设计检查 | 成功要求 | 失败/保全要求 |
| --- | --- | --- |
| 根角色 | BOM/注释/多行/direct metadata、静态安全前缀、后置shadow符合规则 | 第二direct控制候选/unknown前缀/重复key/control计算不认证；保原文 |
| 原生多值 | 相同电话两项标签/备注和顺序从源→表单→Query→XLSX一致 | 重排后旧第1项定位不可用；不按同值误改 |
| 嵌套任职 | 组织ref、职位、期间和来源留在同一项；反向组织视图派生 | 文本不造ref、partial不报全库零、撤权不泄来源数 |
| 论文 | 论文正式数据一份，作者顺序跨表单/查询/引用导出一致 | 未知作者文字保原值，导入冲突不覆盖，资源不双维护 |
| 类型组合 | 短名与全名同义、shared Field只一份，required OR | 同key不同含义/alias重绑/冲突不能 last-wins |
| 精确值 | int/decimal/partial date/offset instant编辑查询导出一致 | bool≠int，float不冒金额，时间不补本机时区，none≠缺席 |
| 安全写 | 当前一次性依据+唯一范围+回读+事务+同Draft/Undo | ABA/迟到/重排/撤权失效；不能隐藏body侧信道放行 |
| 完整源 | import/function/computed/复杂表格保源，可读复杂只读内容 | 控件不足不拒完整语法，资源失败不伪装语言非法 |

### 7.1 有效期间与可选成员设计验收

以下编号是设计要求，未运行 provider/CEL/UI 产品测试。D 表示合法完整date，I 表示精确instant；新增年/月期间验收见§7.2；除行内明确说明外，输入均已获权且所需 provider 存在。

| ID | 输入/动作 | 必须结果 |
| --- | --- | --- |
| DUR01 | 两个有界且有序 D 端点；point 在起点前/起点/内部/终点/终点后 | false/true/true/true/false，两端均包含。 |
| DUR02 | 仅 start、仅 end、空记录或整个 during 缺失 | 一律 unavailable，不推断当前有效或零匹配。 |
| DUR03 | start=D、end=none；point 在 D 前/等于 D/之后 | false/true/true；明确持续中，不是结束时间未知。 |
| DUR04 | start=none、end=D；point 在 D 前/等于 D/之后 | true/true/false。 |
| DUR05 | 两端都是 none | 任意合法 TemporalEndpoint 返回 true（包括年/月）；非法point为invalid，缺provider为unavailable。 |
| DUR06 | D/I 混域、端点倒序或 point 域不符 | invalid；两个有界端点相等时仅包含该点。 |
| DUR07 | 输入不可用/未获权或所需 provider 缺失 | unavailable，不变 false/true，不额外披露源。 |
| DUR08 | 清空表单输入与明确选择无起点/持续中 | 清空省略成员，明确无界写 none，往返保留两者差异。 |
| DUR09 | Range 省略一个端点；Range 为空或端点为 none | 前者是合法开放半区间；空记录/none 端点 invalid，不把 During 规则带入 Range。 |
| QOPT01 | 一项 work 电话及一项无 label 电话运行保存的 Query | where 选中笔记，带 guard 的 select 按原序只保 work 项，不产生缺 label 的 EvaluationIssue；全部无 label 或空电话数组不匹配。 |

原生 person/query 样例只验证相应源码写法。[cases.json](examples/cases.json) 记录语义 oracle，编译成功不证明这些语义已经实现。

### 7.2 期间精度与关系类别验收

以下是新增设计义务。有限 [模型检查](examples/precision-relation-checks.json) 和 [脚本](examples/precision_relation_model.py) 从实际Field/Shape路径检查它们；模型不是官方AST decoder、Rust provider/CEL、表单或Office。结构JSON往返只证明模型传输，不冒DOCX/XLSX再导入成功。

| ID | 输入/动作 | 必须结果 |
| --- | --- | --- |
| PREC01 | engagements.during.start=2020年3月、end=none | 保年月原值；月粒度March包含true，April完整日true，February完整日false，March中旬完整日unavailable；缺end仍unknown。 |
| PREC02 | 状态During 2019年到2021年；Range 2020年到2022年 | 闭合计数3years、半开差2years，非精确日数。 |
| PREC03 | Range March到May；During March到May | 半开2months、闭合3months；与May开始范围邻接不overlap。 |
| PREC04 | 月范围的完整日窗口在内部/远后/边界内 | true/false/unavailable，不补1日，不以unavailable当false。 |
| PREC05 | Mixed precision有序范围完全可证/边界重叠；混精度duration | 前者valid、后者unavailable；duration不补天数。倒序或同精度半开相等invalid。 |
| PREC06 | CalendarDate(day)与native Gregorian date；非Gregorian实际月表 | 前者可同日域比较/日数；后者保实际month ordinal与months单位。非日混历法unavailable；civil/instant混域invalid，缺provider unavailable。 |
| PREC07 | Task.start=year、due=month | 实际Field接受；显示/排序保精度，年月卡片不伪造day/event；completed_at精确要求不变。 |
| PREC08 | event recurrence template或replacement为year/month | typed invalid、保合法Typst原件；不因Range变宽而产生首日anchor。 |
| REL01 | social.preset=friend与social.custom=friend | 两项保独立分支/顺序，Query分组及导入导出不合并。 |
| REL02 | family.friend、social.colleague、professional.friend、unknown preset、两分支或空custom | typed invalid；正确Field预设与非空custom可用。 |
| REL03 | target-only或旧role/customLabel | raw保稿/显式修复；UI未选分类保Draft，不凭文字发明typed分类/方向。 |
| REL04 | custom大小写/空白/Unicode及年月期间跨表单/结构导入导出 | 保原成员与branch/精度；DOCX/XLSX真实往返仍UNRUN。 |

三份精度正例（含任职/状态、Range、Task）与一般关系正例、两份语义负例均在 [native-cases](examples/native-cases/)；官方编译只验证源码合法，负例不会因此取得typed资格。

### 7.3 跨参数共同时间基准验收

这些设计 oracle 针对实际 Range 字段解码后调用 provider 的共同基准，不以两个字段分别有效代替调用资格。完整获权输入及所需冻结 provider 先就绪；未知/不可得/invalid 不被无界或布尔短路洗掉。有限模型当前共96项（原69项加27项），不代表真实provider/CEL或权限验收。

| ID | 输入/动作 | 必须结果 |
| --- | --- | --- |
| RDOM01 | 两个分别有效的only-start date/instant Range；only-end及交换参数 | 四例均invalid，在无界快捷分支前拒绝混域。 |
| RDOM02 | 同历法/版本month的两条未来/过去开放范围；同月month/day两条同侧范围 | 均true；共同域成立不要求compareTemporal能对两有限端点排序。 |
| RDOM03 | 同域不同侧开放范围：有交集、分离、邻接 | 半开规则分别true/false/false。 |
| RDOM04 | native Gregorian date与CalendarDate(day)；受支持的跨历法/冻结版本完整日 | 同一civil日基准；同侧开放true，不同侧邻接false。 |
| RDOM05 | 粗精度跨历法/冻结版本的未来/过去开放范围 | unavailable，即使单个不等式或无界快捷分支本可给true/false。 |
| RDOM06 | 开放范围有限端缺provider或日期非法 | 分别unavailable/invalid，不能转换成布尔值。 |
| RDOM07 | containsDate粗精度跨历法；duration与recurrence模板混civil/instant | unavailable/invalid/invalid；During两端none仍由有效获权point给基准，不改变既有语义。 |

当前模型、原独立70组不改期望的作者复跑和额外8组同族复跑保留原作者证据身份；该发现随后在固定技术基线上独立设计CLOSED，最终处置与非作者检查见REVIEW §7。schema形状与builtin ABI不新增参数：共同基准预检是§3.1.1受信固定实现的必需算法，配置不能自行宣称满足它。[cases.json](examples/cases.json)保留编号及证据边界。

设计检查记录 [design-checks.json](examples/design-checks.json)，执行脚本 [validate_repair.py](examples/validate_repair.py) 只校验闭合/引用/路径/[builtin-abi.design.json](examples/builtin-abi.design.json)固定ABI设计参考（运行时必须核受信Rust registry，不能信配置自报）及有限业务oracle，不是Rust安装器。Query cold基准测试用人工symbol列表，不是CEL解析器；calendar测试仅Gregorian与有限synthetic pack。

本机官方 `typst 0.15.1 (9dfd3a08)` 复用现有工具，七类native及三个legal-but-schema-invalid源可编译，保存Query也可编译；记录 [syntax-experiments.json](examples/syntax-experiments.json)。[citation-experiments.json](examples/citation-experiments.json) 证明手供Bib下@/cite可编译、heading/Bib同键报歧义，未证明真实Work读取/资源生成或引用提取。CLI query不认证角色，JSON不是精确值权威。schema配置、实际Rustprovider、CEL冷解析、权限/事务/fieldprofile/DST完整规则、导出/Office仍待实现与验收；设计检查不关闭非作者问题。没有真实数据迁移证据。

官方依据：[v0.15.1 AST](https://github.com/typst/typst/blob/v0.15.1/crates/typst-syntax/src/ast.rs)、[metadata](https://typst.app/docs/reference/introspection/metadata/)、[decimal](https://typst.app/docs/reference/foundations/decimal/)、[datetime](https://typst.app/docs/reference/foundations/datetime/)、[cite](https://typst.app/docs/reference/model/cite/)、[ref](https://typst.app/docs/reference/model/ref/)。官方语言说明不能证明本文应用机制已实现。
