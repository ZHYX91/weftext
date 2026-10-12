---
source_language: zh-CN
translation_status: source
---

[English](EDITOR.md)

# Typst 阅读、编辑与独立批注

状态：**协调设计候选，未实现，未独立接受**。固定输入 `af6737a057c7bb69763dfaad0a57da4652e085f3`，作者分支 `design/typst-native-20261012`。本文拥有阅读/编辑行为、编辑资格及批注交互；[SOURCE](SOURCE.zh-CN.md) 拥有官方语义与来源投影，[CORE](CORE.zh-CN.md) 拥有闭合持有者、授权、事务及恢复，[FEATURES](FEATURES.zh-CN.md) §§2–4 唯一拥有地址、链接转换与潜在匹配算法。下列版本是新的 Typst family，不扩大旧 D8/AsciiDoc decoder。

## 1. 作者源、工作面与状态

每个 Node 的 `.typ` exact bytes 是唯一正文作者权威。Source、富编辑、流式阅读、分页预览、属性面板共享同一 owner、Base 与 Draft；切换工作面不提交、不清空稿件、不重建身份。普通合法 Typst 的读/保存资格与控件能力分开，标题可以没有，文件名不补写 `title`。宏、导入、模板、复杂表格或未知可视控件不构成整文件拒读/拒存的理由。语言有效性、根控制声明/typed fields、编译/布局可用性和本次操作权限分别报告。

会话保持四条状态轴：完整 `SourceObservation/1` 与 exact Base；单调 `draftSerial` 与 exact Draft；IME/input generation、inputSerial 与 selection；当前读写资格、prepare、preview、提交/恢复。Draft 是内存中的非权威 proposal；本地恢复稿可以加密保管但不能作为 current source 或跨设备权限票据。Source、富编辑和属性面板共享一个 Undo/Redo 栈，外部同步/Agent/其他作者提交不混入栈。

成功 receipt 证明原操作的真实决议，不直接覆盖用户后继 Draft。重新授权读取 after 后才能更新 Base；当前 Draft 与 after 精确相等时才 clean。Undo 每次使用新 serial，生产版本、token、SourceMap、selector、prepared evidence 不因字节相等复活。撤权立即停止内容交付、装饰和写入；保留当前用户已输入的恢复稿，重新授权之前不能用缓存重新展示受保护原文。

## 2. 连续阅读的交付后端

默认生产目标选定 **`flow-semantic/1`**：SOURCE 的官方 Rust Typst **0.15.1** 求值/编译投影 → Weftext 语义阅读树 → 受控 DOM/原生可访问节点 + 阅读样式。它只呈现官方已经求值的元素、内容、顺序和来源，不重新解释 Typst 源码、执行函数或以 AST 文本代替 `show` 后结果。它是 Weftext 需要实现的 renderer，不能把“计划采用”写成已完成。编译器、标准库、字体、资源、包、投影与 renderer 的准确 pins 同 CORE carrier 冻结；升级重新做矩阵，不使用 `latest`。

生产最低矩阵必须交付：中文/英文/RTL 普通段落和软换行；title；heading 的实际层级（控件 1–9，不把更深合法级别判错）；strong/emph/strike；原生 inline/display math；项目/编号/术语列表；原生链接及文内引用；图片 alt/caption；简单和带 header/span 的表格；raw/code；不可流排内容的可读 island。普通文本自适应容器宽度，连续纵向滚动，不生成每页白纸或依赖 PDF 查看器。公式和复杂布局可以由同版本官方分页编译的 SVG frame 作为局部 island，保留官方内容边界、语义说明/可访问替代及准确来源入口；不把整篇截图当流式阅读。二维宽表/图可以局部横滚且提供同数据的可访问结构，页面正文仍能在 320 CSS px 重排。

`show`/`set` 的语义内容由官方求值决定，flow 的几何是阅读布局，print 的几何由官方分页布局决定。flow 不声称复刻任意页面坐标、页眉/页脚、浮动、绝对放置或分页计数器；这些元素显示局部版式预览和“分页专用”说明。若官方元素无法给出可信 reading order/替代内容，明确该 island `layout_unavailable` 并提供 source 与分页入口，不伪造普通段落。官方不透明内容只能进入有边界的 island，不能丢弃；基本矩阵缺项或普通内容全篇降级为 source/PDF 都阻止流式阅读里程碑通过。

**`html-experimental/1` 是可选实验预览，不能成为生产后端的唯一实现或通过条件。** 官方明确 HTML 仍不完整且不适用于生产，当前主要输出语义标记而非全部 CSS；实验路径固定同版本官方 `typst-html`，允许目标专用原生 show rules，但必须展示实验状态、能力/损失，不默认承担生产阅读。没有通过本站矩阵时不能因一个 HTML 样例成功宣称支持。任何实验 HTML 片段均先验证成阅读树，不能直接执行作者 HTML。[官方 HTML 状态](https://typst.app/docs/reference/html/)、[固定 Rust HTML crate](https://docs.rs/typst-html/0.15.1/typst_html/)。

两条后端均使用静态输出安全策略：拒绝 script/event handler、iframe/object/embed、表单提交、作者主动 CSS/外部字体/网络 fetch、`javascript:` 和未批准协议；SVG 同样去除脚本、foreignObject、外部引用。仅允许 renderer 固定标签/属性/样式集合，正文为 text 节点，资源通过 Core 授权 pin/受控交付地址；链接点击送给 FEATURES resolver。WebUI 使用 CSP、隔离无特权 origin 和无脚本 sandbox（交互在宿主覆盖层）；Desktop 原生/受控 webview 也不授文件/网络/进程能力。清洗损失显式报告，不能静默删合法作者内容并声称无损。

每次渲染绑定 carrier、source Observation、draftSerial（如为草稿）、projection、资源/授权 cut 与 `layoutEpoch`。旧预览可由用户显式查看且标“旧版本”，不能提供 current 编辑/批注目标或覆盖新稿。取消和 budget failure 保留稿件，错误、partial、stale、unavailable 与完整空文档分开。

### 2.1 可交付语义投影 artifact

`TypstSemanticProjection/1` 的producer是固定官方Rust build中的Core evaluation/realization bridge，consumer是flow renderer、body Query与EditMap资格检查。若官方公开API不足以获得post-show语义树/来源，实现必须在锁定的官方求值/realization边界加具名只观察hook（hook版本/digest进入build环境），与无hook官方编译结果逐项对照；不得自己求值源码、从PDF反抽文或把pre-show AST改叫Content。该hook与renderer是实施范围和验收门，不是上游已提供完整编辑器的声明。

```text
TypstSemanticProjection/1 = {
 kind:"typst_semantic_projection",version:1,profile:TypstProfileCarrier/1,
 ownerNodeRef:NodeRef,sourceObservation:SourceObservation/1,
 environmentPin:PinRef/2,worldProofPin:PinRef/2,
 originEvidencePin:PinRef/2,root:[SemanticNode/1...],
 introspection:[SemanticIntrospectionEntry/1...],navigation:[SemanticNavigationEntry/1...]
}
SemanticNode/1 = {key:Counter,originIds:[Counter...],direction:"ltr"|"rtl",
 presentation:"block"|"inline",element:SemanticElement/1}
SemanticElement/1 =
 {kind:"text",text:text}
 |{kind:"paragraph",children:[SemanticNode/1...]}
 |{kind:"title",children:[SemanticNode/1...]}
 |{kind:"heading",level:PositiveInt,outlined:boolean,numberingText:null|text,
   children:[SemanticNode/1...]}
 |{kind:"format",style:"strong"|"emph"|"strike",children:[SemanticNode/1...]}
 |{kind:"math",display:boolean,framePin:PinRef/2,semanticText:null|text}
 |{kind:"list",style:"bullet"|"numbered"|"terms",items:[SemanticListItem/1...]}
 |{kind:"table",columns:Counter,rows:[SemanticTableRow/1...]}
 |{kind:"link",destination:SemanticNativeDestination/1,children:[SemanticNode/1...]}
 |{kind:"resource",resourceRef:ResourceRef,bytesPin:PinRef/2,alt:null|text,
    caption:[SemanticNode/1...]}
 |{kind:"raw",text:text,language:null|text,block:boolean}
 |{kind:"island",nativeKind:text,framePin:null|PinRef/2,
    semanticText:null|text,reason:"print_specific"|"unsupported_element"|
    "reading_order_unavailable"}
SemanticListItem/1 = {key:Counter,originIds:[Counter...],
 marker:null|text,term:[SemanticNode/1...],body:[SemanticNode/1...]}
SemanticTableRow/1 = {key:Counter,originIds:[Counter...],
 section:"header"|"body"|"footer",cells:[SemanticTableCell/1...]}
SemanticTableCell/1 = {key:Counter,originIds:[Counter...],column:Counter,
 colspan:PositiveInt,rowspan:PositiveInt,children:[SemanticNode/1...]}
SemanticNativeDestination/1 = {kind:"uri",value:text}
 |{kind:"label",value:text,finalLocationKey:null|Counter}
 |{kind:"location",finalLocationKey:Counter}
 |{kind:"page_point",page:PositiveInt,xPointsBits:text,yPointsBits:text}
SemanticIntrospectionEntry/1 = {key:Counter,originIds:[Counter...],
 nativeKind:"heading"|"other",headingLevel:null|PositiveInt,
 bodyText:null|text,readingNodeKey:null|Counter}
SemanticNavigationEntry/1 = {key:Counter,readingNodeKey:null|Counter,
 page:PositiveInt,xPointsBits:text,yPointsBits:text}
```

所有对象闭合，key在整artifact唯一且按官方逻辑顺序生成，不作持久identity；root/children/items/rows/cells顺序正是官方reading order，不能由DOM/哈希/视觉x排序。PositiveInt是官方正整数无控件9级上限，canonical十进制字符串以免JSON精度损失。table columns>0、cell跨度不越界/不重叠且确实对应官方网格；不能证明的table整体为island而非伪造cells。originIds严格指同originEvidencePin内真实SOURCE来源，不能内含任意caller range；所有pins同owner/evaluation/environment/World cut。link destination是官方原生值，FEATURES再资格化应用地址，不由语义artifact授read/write。framePin是官方同build分页片段受控SVG输出，不是HTML script；semanticText若不可由官方内容证明为null且明确替代不可用，不能猜公式朗读。unknown element必须为带原因island；缺普通最低矩阵仍不通过。CORE冻结实际官方paged/html编译target，flow-semantic只选择presentation backend；跨target reuse必须证明同资源cut而非共享旧token。

具体捕获阶段为：官方evaluate产生Content → 官方realize的prepare/visit_show_rules入口记录原元素及effective style/scope → recipe/builtin执行得到returned/realized Content → visit_styled与官方Introspector/Frame记录actual text、reading occurrence、位置及来源。只发布最终收敛Introspector轮及成功compile的trace，不交付早轮临时值/empty。bridge关联同次trace的entry/return，不重执行规则；最终display payload只能来自return/Frame，语义role只能来自官方保留的semantic element或逐项证明的透明规则。默认heading/strong realization给heading实际level/outlined/numberingText及strong格式children；未改写普通正文保exact origin。`#show heading: it => [X #it.body]` 必须显示官方实际X+body，X是generated；若不能证明返回内容保留heading role/level，则为局部island并注明原heading的导航来源，不把原body/旧role伪装为post-show heading。`#show strong: it => it.body` 不继续假画strong。编号、counter、direction、inline/block placement从final trace和resolved style读取，numberingText不自行计数；不能完整归属的counter/state输出为island。

生产flow-semantic默认绑定官方 **Paged** target，`#context target()`与PDF同样返回官方paged语义；flow是呈现后端而不是新增Typst Target，不fork标准库target分派。`if target()=="html" { [网页] } else { [分页] }`在默认flow与print都显示“分页”，实验html才显示“网页”，各target的World/资源trace分别资格化。flow通过阅读布局重排Paged语义，不能偷换成Html再声称同内容。已资格化的native run-in规则必须证明语义entry→return对应、真实level/numbering、body顺序和inline placement，才生成heading与同行paragraph；任意custom show不承诺可逆或保role。若固定官方build无法提供上述trace，则bridge里程碑未完成，不能退回AST/实验HTML称完成。resolved方向和placement必需；font/color/page geometry依flow明确损失策略，不声称与print一致。

官方Content只在Core同次官方求值进程中存在，不作为PinRef/可移植JSON。CORE `TypstCompilationEvidence/1`冻结完整source/environment/world/semanticProjection/origin/output pins，retained artifact可恢复读取，但旧map/token不能复活。

`SemanticNativeDestination/1`保留官方string/label/location/page-coordinate不同类型，不把label或location强转URL。finalLocationKey只索引本次最终官方Introspector/Frame导航trace，缺目标时label保value且key=null，显示不可导航；location key不得指另一次compile。页面point使用官方resolved绝对pt的finite IEEE754 binary64小写16hex位型，page≥1，导航明确打开同次分页布局，不当源地址。FEATURES resolver再验应用URI/当前label与权限；语义目标和pin均不授权限。[官方链接目标类型](https://typst.app/docs/reference/model/link/)。

`introspection`与display root分域：final官方Introspector仍登记的HeadingElem即使被不透明show显示成island，仍给真实headingLevel/官方bodyText及origin，readingNodeKey指island或null；不能因此抹掉Query行。heading body无法准确投成文本时bodyText=null表示该列不可得，不伪空串；other的headingLevel必为null。Query heading来自introspection，正文可读文字来自root实际returned内容，二者不可混用。此处条目只是具名heading投影，不宣称完整官方任意字段API；其它Query列由其owner具名projection，缺投影报unavailable。Query key仅result epoch内有效，不授rich write或持久locator。

navigation数组是最终trace的闭合可恢复投影，key唯一，page及点值与同次官方Frame相等；readingNodeKey只在确切节点对应时非null。finalLocationKey必须引用其实际entry；scope外、不完整或无法投影时导航不可用，不能搜索同名label/显示文字。两个数组和阅读树同一编译证据固定，读取artifact仍重新授权/验环境，不复活运行时地址。

## 3. 官方来源到可编辑区域

SOURCE 产生官方 AST、BodyProjection 与 SourceMap；EDITOR 消费它们形成只在本 Draft 内存在的 EditMap。所有位置同时绑定 owner、Base Observation、carrier、draftSerial、exactSourceDigest；digest 是交叉检查，不替代 exact source/current Observation。字节、UTF-16、Unicode scalar、grapheme 和 visual glyph 坐标显式转换，源范围为 UTF-8 半开区间，CRLF 不可拆。导航 map 与写入 map 独立。

| SOURCE origin | 阅读/导航 | 可视写入资格 |
| --- | --- | --- |
| `rootLiteral` | 当前 root 的准确值/文本 | 唯一可写范围、边界及语义证明成立时可写 |
| `rootConstruct` | 当前 root 的原生 strong/heading/link/table 等构造 | 仅该构造具名 adapter 证明参数/body 的唯一范围及完整改写时可写 |
| `generated` | 官方计算/循环/show 生成结果和已知真实来源 | 默认只读，选择“编辑来源”；不以显示字符串反算 |
| `include` | 实际 included owner/source（须独立授权） | 本篇渲染只读；显式打开来源建立该 owner 新会话后按来源资格编辑 |
| `multiOrigin` | 多个真实来源及原因 | 只读；不默认选择一个来源 |

即使有 Span 或 rootConstruct，重绑定的标准函数、复用变量、同一来源多次输出、不可逆 escape/智能引号/文本变换、show 删除/重排/拼接或隐藏来源都不自动取得 scalar write。显示文字相等没有任何资格作用。与此同时，文档包含一个宏不取消独立普通区域：adapter 按官方数据流和当前 show/set scope 建依赖闭包；受影响/不可证明的区域只读，未受影响且证明完整的段落仍可编辑。若全局规则作用全部正文且证明不可得，全正文 rich-readonly 是有理由的结果，Source 编辑仍可用；不能静态见到 import 就拒绝全文件。

EditMap 中每个 flow 是一个逻辑 inline slot，有 `flowKey`（本次 projection 内唯一）、logicalText 和 source segments；segment 标记 `exactScalar|escaped|atom|generated`，只有 exactScalar 或具名可逆 codec 能直接写。每个 structural region 有 official AST path、准确范围、constructKind、operation capabilities 和原因；site 是 parser 证明的 sibling 插入边界。数组下标、table 行号、AST path、DOM key 都仅 revision/Draft scoped，不能持久作身份。reading selection 跨 island 时可读复制，但只有整段明确写资格才能转换成写选区。

## 4. 基本双向编辑与原字节保全

普通写作必须交付下表；高级合法源另有 source 路径，而不把以下基本控件都延期。UI 操作生成具名 intent；Core adapter 编码合法 native Typst，完整官方重解析后返回新 Draft、EditMap 和可证明 caret。禁止从 DOM `innerHTML` 序列化回 `.typ`，禁止全文件 pretty-print。

| 控件/操作 | 原生作者表示与正向范围 |
| --- | --- |
| 正文输入/删除、Enter/合并段落、空文档/删空再输入 | root-authored paragraph 文本；按 native codec 转义用户 literal，保留已存 EOL/空白，生成 EOL 用当前文档明确策略 |
| 粗/斜/删除线开关和内文编辑 | native strong/emph/strike 或未重绑定标准调用；无嵌套歧义的准确 inline 选区，保留外部 comment/trivia |
| title 与 heading 1–9，提升/降低 | native `title`/`heading` 或 `=` markup；title 与 section 分开，不按字号猜级别 |
| inline/display math 创建及修改 | 原生 math 源的局部公式编辑器和官方预览；公式视作选区 atom，内部由 math-source adapter 编辑，不用渲染 glyph 当数学源 |
| 项目/编号/术语列表，新项/拆项/合并/缩进 | native list/enum/terms；唯一 root list/item region，跨项操作预览准确范围，不把文本 `-` 粘贴执行成列表 |
| 表格 cell 编辑、增删/重排行、增删简单列 | 标准未重绑定 `table`，columns 可证明为固定正整数，完整显式 cells；矩形无 span/header/footer 的简单网格提供行列操作，具体写法依 native 参数/内容范围 |
| 表头/span/block cell | 按官方语义可读；每个具有唯一 origin 的 cell 可局部编辑；涉及 span/header/footer 拓扑的操作显示 source/高级入口，绝不拉平或自动修补 |
| 链接插入/转换 | FEATURES §§2–3 的 native `link` 编码与同目标验证；源可写、目标可读独立验证 |

每个 rich operation 都冻结 selection 的完整 before source slice、operation kind/参数及 map binding。text 输入按 literal 编码，格式/列表/公式/链接命令按各自构造 codec；两者不可混用以偷偷注入代码。所有成功写入证明：选区外字节逐一不变；完整官方 AST 重解析成功（或 §6 明确受限的错误修复分支）；目标结构/值精确符合 intent；非目标控制声明、业务字段及 AST 子树/来源无变化；新 scalar/结构的 provenance 来自这次操作；caret 可在新 map 唯一定位。若原本 active show 引起目标显示转化，必须重新官方求值确认资格，不能仅以 AST 相同通过。

跨粗/斜文本边界的选区在同一 paragraph 内通过 inline-tree split/merge 编码整段目标区域，并保留未选 leaves 的精确 bytes；跨 paragraph/list item 的删除、分段和 text/plain 粘贴通过结构 intent 一次处理完整目标。跨 title/heading/table/code/math/generated island 的选区不默认 flatten：复制逻辑文本可用，写入在缺少覆盖整段的具名操作时解释原因并提供准确 Source 选区。table Cell 在相同内容时以当前 AST cell path 定位，只修改选中的那格。

表格重排先依据官方 AST 参数序列建立 row trivia：cell内部comment随cell；两个row间在前row末cell之后、后row首cell之前的comment/空白作为后row的leading trivia；尾row之后到closing delimiter的trivia固定table尾；table参数/columns/header/footer不随row移动。逗号及必要分隔由native codec最小生成，原comment内容/EOL逐字保全。存在跨row表达式、spread/生成cells、无法证明的参数归属或重排改变header/span语义时只拒绝该reorder，不以“有comment”笼统拒表。SOURCE属性数组与Document table分域；六row domain继续区分native table、Field occurrence、NodeCollection、derived Query、external/import IR、Resource preview，不引入RecordRef/行UUID。通用structured/table budget按FEATURES的D5接续，不能借source intent拆批绕过。

矩形 text/plain TSV 粘贴只在简单网格、完整目标 cells 和全部 literal 可表示时提供显式“粘贴到表格”：一次解析为 cells，一次原子 Draft operation，预览维度/扩展，预算失败或任一 cell 无资格则整次不改，不静默只贴部分。普通粘贴仍是 literal 文本。列重排、公式列生成、自动 ragged repair、HTML/RTF 富粘贴不属基本矩阵；未来必须单独 mapping/loss 与验收。它们不是合法复杂表格读/保存的前提。

## 5. 输入、剪贴板、Undo 和双向定位

每个 controller 最多一个 transform 在途，冻结 owner/Base/carrier/source/selection、generation、inputSerial、draftSerial、nativeAfter 与后继输入日志。迟到回应只能完成原 generation，重放/重新资格化后继输入或保留未应用的 native draft，不能覆盖后来输入。Source 与富编辑互相定位用 map/官方 origin：layout 命中 → logical selection → source range；Source caret → 官方 origin → 当前 layout stop。不能映射时给出 Source 位置和原因，不搜索同样文本。布局重排仅更新 layoutEpoch。

中文 IME begin/update 是本地 preedit，不进行 prepare、链接扫描转换或结构命令；composition commit/end/final input 形成一个 Draft transaction 和一个 Undo group。active composition 保留焦点与 virtualization item，Enter/save/slash 不抢 composition；取消/失焦/后台切换不制造提交。宿主只保留一个 composition owner。Undo/Redo 重建新 projection 和 serial；批量粘贴、一次格式切换、表格操作、链接转换和属性操作各自一个可见 Undo 单元。

Copy Text 按逻辑阅读顺序；Copy Source Fragment 只复制当前获权的准确原始源，不拼接合成 joins。复制表格可显式 TSV，不授予资源下载能力。跨块/跨格式 Copy Text 包括明确的逻辑块分隔，记录 chosen flavor；不包含隐藏内容。默认 Paste 只读 text/plain 并 literal 编码，HTML/RTF/scripts/macros 不执行。无 plain flavor 显示 unavailable。Cut 必须先成功写入真实 OS clipboard 再修改 Draft；clipboard 失败完整保留选区和源。Source 模式粘贴是显式源码编辑，可输入合法函数/宏，仍经保存门；UI 标识该模式，不能把 source paste 当 rich literal paste。

## 6. 错误分区、源编辑与保存

SOURCE 官方 parser 的错误节点和作用域边界决定 error regions。控制声明、字段和正文各自有 validity；正文编译失败不抹掉完好 AST 根声明，也不把旧正文/派生值标 current。普通 Source Draft 总能在完整 source_read 后编辑 exact bytes；错误原文保留用于修复。malformed UTF-8 走 CORE physical repair 而非伪造 AST。

语法错误岛附近只给 Source 修复；其它段落仅在官方 AST 证明完整边界、无错误影响的作用域/依赖、目标与非目标不变量时可继续 rich 输入。该 Draft 仍标 syntax-invalid，不伪造整篇 valid。编译错误但语法完整时，AST 的 direct root literals 可独立编辑；需要求值/渲染证明的 slot 保持 unavailable，而有完备独立证明的普通区域不一律禁用。错误范围可能吞掉后文时不能冒称 safe partition。

whole-source 保存不要求全部 rich regions 可写、renderer 成功、完整全库 Query 或图形控件。它要求 CORE 当前格式/身份/适用局部 typed/授权/安装门及 SOURCE language/control 验证；有效语法的正文编译错误可作为有诊断的作者稿保存，不能称编译或导出成功。syntax-invalid 的普通保存不能冒充 valid；显式 repair/checkpoint 若 CORE 未提供相应资格则只保存恢复 Draft。结构控件不能借 whole-source 名义绕过 strong 门。ordinary/complete 与 strict/observed_only 是独立轴；observed_only 只能按 CORE 预先选择的受信人工单 existing Document 条件，不从 strict 失败自动回退。Server hosted 写入走 Server，离线本地 parser 不授 hosted commit 权限。

prepare 冻结 complete proposed source/pins、Observation、carrier、typed/环境/授权依赖和 operation；preview 显示全部影响与准确 bytes，分页不能当完整预览。confirm/commit 使用同一 OperationId、原 D6 `PreparedIntent/4`、`d6_commit_request/2` 和 CAS/decision，按CORE的format参数绑定，而非创建平行事务族。unknown 保留原请求与稿件，按记录版本恢复，不换 OperationId 重试。committed Undo 只反转自己的单 source decision：当前版本和 bytes 精确等于 after、fresh 授权/格式/Observation、before/after/pins完整、无控制或其它实体效果，产生新的 inverse prepare/preview/commit；Redo 同理，不能撤掉别人的变化。

## 7. 编辑接口的 producer / consumer

以下闭合形状由 CORE 注册；本文定义语义，SOURCE/CORE 类型逐字引用其 owner，不复制它们的 union。每个对象包含 `kind` 为表内固定小写 snake_case、`version:1`，拒绝 unknown/missing/duplicate member、非法 null、错 version；Counter/Token/PinRef/NodeRef/SourceObservation 复用原 owner。`binding` 恰为 `{ownerNodeRef,baseObservation,profile:TypstProfileCarrier/1,draftSerial,exactSourceDigest}`，并与请求 exact source交叉验证；不是 capability。

| holder 与完整附加成员 | producer → consumer |
| --- | --- |
| `TypstEditorRead/1` (`typst_editor_read`): workspaceRef, commitDomain, ownerNodeRef, budget | 界面或命令行向核心请求获权源码读取；返回来源快照和完整观察，最终交付门通过后才展示正文 |
| `TypstDraftProjection/1` (`typst_draft_projection`): workspaceRef, commitDomain, binding, source, budget | 编辑器请求官方解析与投影；返回准确源码、分区诊断、编辑地图与资格，不能产生持久定位器 |
| `TypstEditOperation/1` (`typst_edit_operation`): workspaceRef, commitDomain, binding, source, operation, budget | 输入控制器调用核心适配器；操作为§4闭合命令，返回新投影及光标，失败不修改草稿 |
| `TypstEditPrepare/1` (`typst_edit_prepare`): workspaceRef, commitDomain, profile, expectedSourceObservation, proposedSource, invocationClass, saveProfile, guarantee, writeProtection, budget | 编辑器消费原D6 `InputDescriptor/4`、`DependencyProof/4`、`PreparedIntent/4`，冻结完整前后像；CORE `TypstDocumentFormatBinding/1` 和固定的 `TypstDocumentQualification/1` 证明准确格式 |
| `TypstPreparedEditBinding/1` (`typst_prepared_edit_binding`): workspaceRef, commitDomain, profile, operationId, principalAudienceToken, inputDescriptor, sourceInputs, proposedInputs, dependencyProof, preparedIntent, preview, expiresAt | 核心交给预览、确认和恢复；预览为CORE `FormatAdapterEnvelope/1(role:effect_manifest)`，效果字节使用对应效果角色；具名替换原D7语言槽，准备记录绑定原 `d6_commit_request/2`，完整载体、固定像和切点逐项一致 |

`TypstEditInput/1` 恰为 `{kind:"typst_edit_input",version:1,invocationClass:"interactive_source_save"|"noninteractive",writeProtection:"strict"|"observed_only",intent:TypstPinnedEditIntent/1,origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request}}`。`TypstPinnedEditIntent/1`闭合为 `{kind:"document",target:{ref:NodeRef,expectedObservation:SourceObservation/1},proposedSource:PinRef/2}` 或 `{kind:"annotation",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken:AnnotationRevisionToken/1,proposedValue:PinRef/2,targetPolicy:"preserve"|"replace_current"}` 或 `{kind:"annotation_reconfirm_suggestion",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken:AnnotationRevisionToken/1,proposedValue:PinRef/2}`。annotation/reconfirm必须strict且origin=direct；Core操作gate展开complete Value后绑定proposedValue，caller不得提供它。Core生成/持久化的 `OwnerInputBinding/2.ownerKind=intentKind="typst_edit/1"`；canonicalDescriptorBytes是CORE定义的完整 `FormatAdapterEnvelope/1{kind:"weftext_format_adapter",version:1,role:"edit_input",profileCarrier,payload:TypstEditInput/1}`。profileCarrier相同、document pin是exact UTF-8完整源，annotation pin是完整D3-CJ/3(TypstAnnotationValue/1)，proposedInputs恰一相应entity项，sourceInputs与InputDescriptor逐字相同。generic PreviewBinding/SourceRevisionPlan/Frontier保持owner类型，InputDescriptor/DependencyProof/PreparedIntent统一CORE新增通用/4（含compile_world）而不是原/3扩union；Typst adapter只更换具名payload。

`operation` 的 closed arm 是 `replace_literal{flowKey,startScalar,endScalar,text}`、`splice_blocks{regionKeys,startScalar,endScalar,text}`、`split_paragraph{regionKey,offsetScalar}`、`merge_paragraphs{leftKey,rightKey}`、`set_inline_style{regionKey,startScalar,endScalar,style:strong|emph|strike,enabled}`、`set_title{regionKey|null,text}`、`set_heading{regionKey,level:1..9}`、`edit_math{regionKey,mathSource,display}`、`edit_list{regionKey,action:insert|remove|split|merge|indent|outdent,itemIndex,text:null|text}`、`edit_table{regionKey,action:set_cell|insert_row|remove_row|reorder_rows|insert_column|remove_column|paste_rect,arguments}`、`convert_link{regionKey,startScalar,endScalar,address,visibleText}`。`arguments` 按 action 闭合：set_cell={row,column,text}；insert_row={at,cells:[text...]}；remove_row={at}；reorder_rows={order:[Counter...]}；insert_column={at,cells:[text...]}；remove_column={at}；paste_rect={row,column,cells:[[text...]],extend:boolean}。不存在任意 JSON/DOM patch arm。null 只在以上显式槽合法；itemIndex 对本次 list、indices 对本次 table；level、style、layout 不改变语言上限。SOURCE whole edit走 prepare，不冒充局部 operation。控制/属性专用操作仍由其 owner，不借本 union 改 id/types。

SourceMap/结构 operation 是 proposal producer，不是保存认证。Core 再检 exact before、完整权限和所有 pins；Frontier/2 保留原因果含义，不是源范围完整证明。Agent 只能形成上述准确 proposal 或 Core 已有 Action，不能因会写 Typst 获取文件直写权限。print preview 和 flow 共享同一 EditMap资格；分页坐标另有布局 map，不签另一份作者身份。

run-in作者控件是**完整Source适配器路径**而不是上面局部operation union的新隐含arm：准确读取现有native show scope→提供native规则的完整proposedSource预览→官方全源重解析/求值与§2.1透明规则证明→`TypstEditPrepare/1`。只能编辑自己可无损表达的规则，未知custom show保留并解释source-required。插入新规则不覆盖既有规则或改变scope外字节；需影响多个heading时preview逐个列准确影响集合。取消保Draft，一次确认一个Undo。规则中的label/引号/括号按官方native编码重解析，不让用户手写内部wire；无法证明就保原源码，不以新界面自动规范化整篇。

## 8. 独立 JSON 批注与引用

继续使用 node-local `weftext.annotations.json`，annotation owner/identity、回复、审阅、建议、attribution、权限、物理 aggregate CAS 与 source版本按原 D3/D6 语义。新的 `TypstAnnotationInlineBody/1` 为 `{format:"typst-inline",version:1,source}`，使用固定官方 Typst parser/compiler、无文件/网络/包/插件、单个完整 paragraph 的 `typst-annotation-inline/1`：text、strong/emph/strike、native math、经 FEATURES 资格化的 link；完整消费、max source 64 KiB、render 256 KiB、4096 semantic nodes。多段、heading/list/table、import/include/任意计算/副作用拒绝为 `invalid_annotation_body`；exact body与diagnostics保留，不回退另一个parser/plain_text。

CORE 为 Typst 新持有者注册以下闭合对象（拒绝unknown/missing/duplicate/非法null），不拓宽原Value4：

```text
TypstAnnotationTarget/1 =
 {kind:"document",owner:NodeRef}
 | {kind:"document_element",locator:SOURCE.TypstElementLocator/1}
 | {kind:"document_range",locator:SOURCE.TypstRangeLocator/1}
 | {kind:"resource",resourceRef:ResourceRef}
 | {kind:"resource_region",locator:ResourceRegionLocator}
TypstAnnotationProposal/1 = {
 purpose:"comment"|"mark"|"suggestion",target:TypstAnnotationTarget/1,
 replyTo:AnnotationRef|null,body:TypstAnnotationInlineBody/1|null,
 appearance:AnnotationAppearance/1|null,labels:[text...],
 reviewState:"open"|"resolved"|"not_applicable",
 suggestion:SuggestionAuthorProposal/1|null
}
TypstAnnotationValue/1 = {
 kind:"typst_annotation_value",version:1,profile:TypstProfileCarrier/1,
 purpose,target,replyTo,body,appearance,labels,reviewState,
 suggestion:Suggestion/3|null,creator:AnnotationActorSnapshot/1,
 authoredAt:AnnotationTimeSnapshot/2,lastEditor:AnnotationActorSnapshot/1,
 editedAt:AnnotationTimeSnapshot/2
}
TypstPortableAnnotationRecord/1 = {
 kind:"typst_portable_annotation",version:1,annotationRef:AnnotationRef,
 annotationRevisionToken:AnnotationRevisionToken/1,value:TypstAnnotationValue/1
}
TypstAnnotationAggregate/1 = {
 format:"weftext.typst.annotations",version:1,ownerNodeRef:NodeRef,
 records:[TypstPortableAnnotationRecord/1...]
}
```

完整值的用途、目标、回复、正文、外观、标签与审阅状态类型和提案一致，建议除外；Suggestion/3、外观、作者与时间、同所有者无环回复、根及回复审阅约束、操作类别门逐字复用D3。规范字节为完整值及聚合的D3-CJ/3；记录非空、按完整引用排序唯一，空侧文件不存在；聚合观察和安装逐字复用D6物理CAS类型。TypstAnnotationRead/1返回 `{kind:"typst_annotation_read",version:1,workspaceRef,commitDomain,annotationRef,sourceObservation,annotationRevisionToken,value:TypstAnnotationValue/1,targetResolution:exact|mapped|candidate|ambiguous|orphaned|unavailable,body:{state:absent,exactSource:null,semanticText:""}|{state:valid,exactSource:text,semanticText:text}|{state:invalid,exactSource:text,semanticText:null,diagnostics:[...]}}`；草稿及读取使用新正文类型和配置。批注准备也通过CORE格式封装，携带提案、准确前像令牌及目标策略，通用D6事务不变；调用方不携完整值的归属或信任证据。

旧 Value4/asciidoc-inline 与其真实恢复记录保持原decoder，不在打开时换format。五个 target arm不混淆；Resource PDF/image/time region复用原D9精确坐标/timebase，不降wholemedia。允许已存批注的完整作者值读取，不意味着目标源/截图/媒体/context可读。

读/选区→批注草稿→Core target资格→单一annotation prepare/preview/commit→重新获权读取，连成完整路径。创建 range/element只能对已经提交且 fresh资格的准确source版本签发；未保存Draft选区先保存并重新定位，不能持久化draftSerial或DOM地址。generated/multiOrigin可整文批注或选择真实已获权源范围，不能伪造显示range为source-range。flow与print产生相同source target，只有显示布局不同。用户标记文字/写想法、回复、resolve/reopen与target exact/stale/orphaned/unavailable状态分离；失效批注留在未归组列表，不能丢JSON/讨论。

quote与context读取必须重新target-source-read/Resource-read；用户拥有批注读取权可读**已存**expectedText摘要、basis和attribution，即使目标隐藏，也不额外读取目标、不新算摘要、不把已存成员null掉。摘要/相似quote/path/hash不能恢复旧map/token。受保护 SourceTransform 链经 CORE 严格认证逐跳只映射未改范围；覆盖选区、gap或多义则candidate/orphaned。manual reattach显式选当前目标，preview准确变化，获得新token并使pending suggestion需要重确认；普通编辑不改目标。

插入引用消费已获权来源的准确quote/locator与可写**目的**Draft；来源只读完全可用，不要求来源write。目标/context之后失权时引用导航不展示隐藏内容。引用所存的实际文字是目的文档作者内容；UI说明其复制性质，不用它证明目标仍current。建议 apply要求当前pending+confirmed、精确annotation token、target read/write、当前source与range、完整依赖/preview，Core同一决议提交source mutation+accepted状态；reject只要求批注自身资格，不能借它读隐藏target。reconfirm为独立操作，caller不能伪造actor/time/trusted/confirmation/basis。并发accept/reject只有一个winner。

### 8.1 批注提交的闭合入口

`TypstAnnotationPrepare/1`恰为 `{kind:"typst_annotation_prepare",version:1,workspaceRef,commitDomain,profile:TypstProfileCarrier/1,saveProfile:"ordinary"|"complete",guarantee:"replica_local"|"managed_atomic",writeProtection:"strict",intent,budget}`。intent只有 `{kind:"annotation",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken,value:TypstAnnotationProposal/1,targetPolicy:"preserve"|"replace_current"}` 与 `{kind:"annotation_reconfirm_suggestion",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken}`。reconfirm不接受caller value；manual reattach是annotation+replace_current独立gate，不能同时更改purpose/suggestion类别。Core从before+proposal+operation-class展开完整Value，pin后构造§7 TypstPinnedEditIntent/1→TypstEditInput/1→同prepared_edit envelope与TypstPreparedEditBinding/1，不是把proposal当完整author value。

`TypstAnnotationAction/1`是closed union `{kind:"create_annotation",destinationOwnerRef:NodeRef,value:TypstAnnotationProposal/1}`、`{kind:"apply_suggestion",annotation:AnnotationRef,expectedAnnotationRevisionToken:AnnotationRevisionToken/1}`、`{kind:"reject_suggestion",annotation:AnnotationRef,expectedAnnotationRevisionToken:AnnotationRevisionToken/1}`。FEATURES `TypstActionSpec/1.intent={kind:"annotation_action",action:TypstAnnotationAction/1}`唯一封装此union，CORE action_input/prepared_action沿原D7流程冻结dependencyProof4/sourceInputs/proposedInputs/完整effects。create通过D3原same-candidate-map身份分配和target资格；apply生成目标source与accepted Value两项原子effects，strict单决议/CAS/P，不能用单entity prepared_edit；reject只有annotation after。apply/reject caller不携replacement或targetLocator，而只消费准确token已保存Suggestion/3。before/after aggregate和所有变动annotation/source pins进入原完整pinDirectory/preview；undo annotation多entity decision不混入单source committed Undo。

## 9. 阅读上下文、链接提示与可访问性

侧栏/分屏可从Query、反链、搜索和批注打开来源，返回恢复原列表位置、筛选、选区和Draft，再重新获权。大纲保留官方heading语义/实际level、过滤/折叠/当前位置/聚焦章节；聚焦只呈现，不删/重排source。run-in依SOURCE原生show/presentation在同行显示heading与正文，保留独立源范围、heading role/真实aria-level、键盘跳转与大纲；不能把H6–H9降成H5、把title当section。作者run-in控件只在可证明的heading/show scope插入或修改native show规则，预览完整准确source改动并经同一source commit；设备阅读偏好仅改布局。新设计取消旧portable shared presentation sidecar及bootstrap initialPresentationPolicy分支，不以缺sidecar阻断读取；真实旧policy/Notice4/CP5/ChangeRecord2按记录恢复，不假造component。高于HTML h6的深度用role=heading+准确aria-level及完整大纲替代。专注/打字机滚动独立开关，IME、AT焦点、用户主动滚动优先。

潜在链接完整算法与源编码由 FEATURES §§3–4 唯一拥有；此处规定必须可观察的交互：默认关闭设备开关，关闭不改源；aliases同范围多目标显式选，不默认首项；“白云山南路”中白云山 `[0,3)`、山南路 `[2,5)`、山 `[2,3)` 全保留；最多两条上下分离线，更多重叠在聚合入口完整可达，首20目标后稳定分页。keyboard/触屏/可访问列表与hover等价，Escape回原选区，不抢输入。真实链接、出链、入链、潜在提及分开；提示不是图边。部分 combining/ZWJ scalar命中只发现不转换，不扩大选区。转换再次检查源read/write、目标read、fresh range/text及官方native link完整重解析，显示文字/未选bytes不变且同target，一个Undo；不要求潜在目标write。

编辑 segmentation固定继承 Unicode 18.0.0、UAX #9 rev52、UAX #29 rev49，与 Query/CEL 自有版本分域。native authored direction优先，再scope显式设备/session preference，再first-strong auto与明确neutral fallback；locale不改source。caret是logical point+upstream/downstream affinity+layoutEpoch；Left/Right视觉、修饰词移动逻辑，Backspace/Delete逻辑grapheme、CRLF原子，copy始终逻辑顺序。Ref/path/email/code等LTR隔离但不改字节；RTL镜像不反转table列身份、math/媒体/数据方向。

控件提供真实role/name/state/selection/focus，AT按logical reading order，表格保留header/span语义；virtualized active/focused/composing item不被回收，live region不逐字朗读preedit。必须验keyboard-only、Arabic/Hebrew/CJK混排、emoji/combining、200/400% zoom、320px、高对比、减少动画，真实Desktop/WebUI/Mobile字体和IME/屏幕阅读器分别报告。缺某surface控件报capability unavailable，同Core语义、source/advanced路径与目标资格不变；CLI不宣称图形render。

设备可保存并选择具名“阅读/写作/复盘”布局，也可由用户命名；保存仅记录面板几何、选择的视图类型和可重新解析的导航目标，属于设备偏好，不新增业务作者格式。恢复布局先保留各已打开所有者的Draft、Undo及选区，再恢复面板，并通过当前Core/Server重新授权、解析和读取目标；不存在、范围失效或不可读的面板保留明确状态，不显示缓存原文。布局不保存结果集、运行时selector/map/prepared evidence、权限或替代Draft，不修改文档顺序、Query排序和共享策略。选择、另存、覆盖或删除布局均不提交正文，不随布局清理未保存稿；设备保存失败保留当前会话。共享/跨设备布局及多Query持久仪表盘仍需单独设计，不能由本地布局入口宣称支持。

## 10. 性能与完成证据

保留D8性能目标：input可见反馈p95≤50ms，layout反馈≤100ms；1000事件，cold/warm各3run；Desktop 10 MiB/100k lines、Mobile 1 MiB/10k lines、单行1MiB；steady editor memory Desktop≤256MiB/Mobile≤128MiB；10倍超限稳定拒绝/降级不丢Draft。CPU parse/compile/layout在worker，增量优化必须与完整官方重算tree/error/origin相等。每键不要求完整编译完成：native preedit/局部输入反馈先显示并标Draft，官方结果随后绑定generation；不能把异步未验证结果准备保存。虚拟化不截断源、不把一页当全集。

研究只证明有限官方AST局部回写与显示可行性：32个Rust unit checks（20+12）与六个whole-Text叶编辑后官方编译，不包含完整DOM选区映射、任意位置输入、IME、跨块剪贴、行列操作、完整Undo、真实Core权限/提交或跨设备重锚。本文所有产品验收为 **UNRUN**。流式后端、adapter和安全策略都必须实现并独立验收，不能借实验HTML、AX样例、unit检查或打印成功宣称完整编辑器。

## 11. 旧 owner 具名处置

| 固定输入实际owner/sections | 新section与处置 |
| --- | --- |
| A2 D2 §§1,5–7,9；§11中的§1.1–1.4；D2 §12 projection | SOURCE官方Typst替换语言/投影；EDITOR §§1–4替换消费者，保留exact-source/来源/独立标题原则；旧AsciiDoc枚举仅historical |
| A2 D3 §§4.7,5.3.1,6–6.2,14.1,16.4 | CORE/SOURCE新carrier与locator替换实际格式槽；EDITOR §§1,6–8保留授权/currentness/ABA/owner及独立JSON语义，不把Span当地址 |
| A2 D8 §§1–9,12–15；D8-INTERFACES §§1–9,11–12；D8-SCHEMAS §§1–9,11,13–14 | EDITOR §§1–10和CORE具名新持有者替换Typst读取、草稿、编辑、批注和渲染槽；旧wire1/2/3、Value4/R6保持真实记录版本恢复 |
| A2 D8-DIRECTION §§1–13 | EDITOR §§5,9–10保留逻辑坐标/四能力/真实AT和平台验收，不照搬AsciiDoc native方向语义 |
| A2 EXPERIENCE §§1–3,5–5.2,6表格段,7–8 | EDITOR §§1–10保留上下文、源/富编辑、批注、基本表格体验；FEATURES §§2–4接管链接算法/编码；属性与Query/输出流程消费各自新owner |
| A2 D8 §§10–11,11.1–11.2；D8-INTERFACES §13；D8-SCHEMAS §§12 | Query/Search/View原领域语义继续由FEATURES明确接续；不新增editor Query语言或View持久状态 |
| AsciiDoc Annotation SCHEMAS §6.3/6.3.1/6.4,§7/7.1/8；A2 D3-SCHEMAS相应§6.3/7/7.1/8 | CORE登记新Typst持有者；EDITOR §§7–8替换body及消费资格；旧inline语言/旧holder禁止混用，SourceTransform/trust保持原owner |

历史snapshots不修改；此前设计关闭/测试不能接受新格式。本文表只在列出的行为/槽位替换，不自动接纳其它旧业务复杂性或虚构迁移。

## 12. 编号正反验收

以下每项同时要求正向与反向结果，设计oracle不是产品已跑测试。

| ID | 正向 | 反向/拒绝与保全 |
| --- | --- | --- |
| ED-01 | 同Draft Source↔rich↔flow↔print保留选区/Undo；合法宏读存 | 控件缺失不拒整文；切换不提交/丢稿 |
| ED-02 | flow-semantic普通中文在320/960px连续重排且AT读取 | 仅PDF/整篇SVG/实验HTML成功不能通过；unsupported island明确 |
| ED-03 | body macro旁独立普通paragraph局部写入 | 重绑定/生成/同字串/多origin/unsafe show不反算；读源可修复 |
| ED-04 | titleless、heading1–9、run-in同段与outline准确 | filename不造title；深heading不降级；字号不造语义 |
| ED-05 | 中文/combining/emoji/RTL输入与一个composition Undo | 不拆grapheme/CRLF；迟到transform不覆盖后继input |
| ED-06 | Enter/合并/删空再输入、跨strong/emph选区保全外字节 | rich literal不注入函数/markup；无法映射不字符串搜索 |
| ED-07 | 原生math局部源码编辑及预览，列表拆/并/缩进 | glyph/计算结果不当source；失败留exact Draft |
| ED-08 | 两个等值table cell只改第二；简单增删行列/TSV矩形一次Undo | span/header拓扑不拉平；预算/部分不可写整次不改 |
| ED-09 | Copy Text逻辑顺序/Source准确、跨块plain paste一次操作 | HTML/RTF不执行；clipboard fail不cut；只读island不误删 |
| ED-10 | 完好root数据在body编译错误时可读/可修，合法source独立保存 | 错误影响区域不给rich资格；syntax-invalid不伪称valid保存 |
| ED-11 | 新鲜载体、完整观察与固定像→完整预览→同一操作提交 | ABA、撤权、观察缺口及旧地图拒绝；未知结果不换请求重试 |
| ED-12 | committed Undo新inverse plan、Draft Undo新serial | 他人after/控制effects不误撤；equal bytes不复活token |
| ED-13 | flow/print相同source selection签同资格target，JSON comment保留 | draft/DOM/Span不持久化；失效目标不丢讨论、不相似重锚 |
| ED-14 | 只读来源quote可插可写目的；annotation-only读已存摘要 | 源write不是引用前提；hidden target零额外读取/零新摘要 |
| ED-15 | 手动reattach/建议reconfirm/apply完整gate，同决议更新 | 普通edit不伪confirmation；reject不读target；accept/reject单winner |
| ED-16 | 白云山/山南路/山、多alias、两层overflow/20分页均可keyboard选 | 最长匹配不吞其它范围；partial ZWJ命中不可扩大转换 |
| ED-17 | 明确转native link完整reparse同target/text/外bytes，一个Undo | stale源/目标read丢失拒绝；点击建议不改源、不成图边 |
| ED-18 | run-in/deepheading、bidi caret、表格AT、focus/zoom/高对比 | DOM order/视觉x不作identity；虚拟化不回收composition |
| ED-19 | HTML/SVG安全清洗、资源当前授权、实验标记明确 | 作者JS/CSS/network/file URI拒绝；清洗损失不藏起来 |
| ED-20 | 性能分平台真实测量、budget失败留完整稿 | unit/AX/六编译不声称IME/编辑器/生产测试通过 |
| ED-21 | default heading/strong final realization准确role/numbering/style；owned run-in有heading语义和inline placement | `show heading=>[X body]`保实际X但不伪role，`show strong=>body`不假粗体；未收敛轮不交付 |
| ED-22 | 默认flow/print官方Paged target相同分支，实验Html不同分支各自资格 | 不新增semantic Typst Target，不以Html计算替换Paged后隐藏内容差异 |
| ED-23 | 设备保存并恢复具名布局、几何和可重新解析目标；重新授权打开，当前Draft/Undo/选区保留 | 布局不保存权限、结果集或运行时地图；失权面板不返缓存，选择/删除布局不提交或丢稿 |
