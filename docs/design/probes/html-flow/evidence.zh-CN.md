---
source_language: zh-CN
translation_status: source
---

[English](evidence.md)

[复现说明](README.zh-CN.md)

# Typst HTML 样式与最终消费有限证据

状态：2026-10-12 实际技术实验，不是独立审查、产品实现、完整 E1 关闭或 RQ03/RQ5.3 验收。范围为本包 16 个 fixtures；不推导任意 Typst 文档、完整 renderer 或 rich writer 通过。

## 结论与对照

普通笔记可优先使用官方 Html target 的同次最终 `HtmlDocument` 作为真实流式消费入口，Paged print/export 是明确的另一 target。有限测量/复用反例表明，最终 children 比稳定 realization trace 更适合作为实际消费依据，普通正文、列表、表格、链接、硬换行无需从 Frame TextItem 恢复。原生 HTML 未自动满足用户原生样式要求；小适配可以在实际生产点读取有效 `StyleChain` 和已合成 `CellGrid`，携带有限关键样式，无需重新求值用户 show、按文字/Span/坐标去重或重写 Typst 排版器。

实际结果见 [analysis.json](executed-summary/analysis.json) 与 [results.json](executed-summary/results.json)：

| 对照 | 实际结果 |
|---|---|
| adapter off HTML 与官方 release 字节 | 16/16 相同 |
| adapter on 单/四线程 HTML 和最终原生树 | 16/16 相同 |
| Paged off/on/release PDF 字节 | 16/16 相同 |
| 有限正文结构投影：文字/角色/链接/br/frame | 16/16 相同；仅忽略有明确 producer 身份的新增 marker 与样式 carrier |
| World source/file/font 调用全序列 | 15/16 相同 |
| 最终树及 native introspection 的 JSON 截断检查 | 全部递归遍历 dict/list，无截断 |

unsupported 动态 marker 警告例：off source2/font11，on source3/font11。额外同 source 读取与新增 warning/诊断路径一致，未做调用栈追踪，因此“诊断读取”是有依据的解释，不是所有 engine 依赖顺序相同的证明。对照 retained native Frame 内容并未变化。

## 官方版本与生成入口

固定 [Typst v0.15.1](https://github.com/typst/typst/releases/tag/v0.15.1)，官方 CLI 为 `typst 0.15.1 (9dfd3a08)`。源码 archive SHA256、精确 `Cargo.lock` 与 binary/patch hashes 在 [version.json](version.json)。Cargo/Rust 1.98.0；HTML/Paged 均传 `--features html`、`--creation-timestamp 0`、`--ignore-system-fonts`，用 jobs1 并对 HTML 对照 jobs4。运行步骤见 [README](README.zh-CN.md)。

官方 `convert_to_nodes` 遍历 post-realization 有序 Content/Style pairs。`handle_html_elem` 在实际 children 生成后建立 HtmlElement；TextElem 沿官方 handle_text/Converter/空白保护生成文字。适配读取当前 `text::families`、`text::variant`、resolved TextElem.size 和纯色 fill。strong/emph native rule 使用官方 Paged 相同的 WeightDelta/ItalicToggle Content.set 运算，保留 delta:0 和嵌套 emph 的有效结果；用户 show-strip strong 不被套回默认粗体。文字 carrier 有明确身份，不改变官方 block/box 对“单元素”结果的分类。没有二次执行用户 show。

table rule读取已经 synthesize 的 CellGrid/Cell stroke/fill 与 TableCell.align，不重跑 cell 样式回调。literal list marker 使用原生 ListMarker::Content 的深度选择，按 Content 路径生成一次；body 的 Depth fold 使用官方 list 相同的 Depth(1)。Func marker在 flow 不执行、不猜值并发出明确 warning；frame 的 Paged 原生规则正常执行它。

CLI 在 `typst::compile::<HtmlDocument>` 返回 Ok 后直接递归快照 `doc.root_node()`。显示选择来自最终产品的 child paths，没有用 stable Sink 当 display，没有过滤 measure 栈，也没有 Span/字串/坐标去重。Tag保留原生 introspection 边，Frame保留原生产品边；head中的生成 CSS不是正文。最终 HtmlDocument是官方 memoization/并行/多轮处理后选择的返回值；真实复用产生多个 child paths 就保留多个出现项，观察随最终产品被消费，不靠重放 show 日志补全。

`TYPST_HTML_STYLE_PROBE` 仅在各独立 on/off 进程生命周期内固定，未被 comemo 跟踪。产品应使 adapter profile 成为 library/routines 配置身份的一部分，或采用固定 worker 配置；共享缓存进程内切换配置未测，不能由此宣称安全。没有禁 cache。

## 有限样式支持表

以下“携带”仅证明最终 native CSS 值，不证明浏览器字形或几何等值。

| 项目 | 已执行结果 | 资格与限制 |
|---|---|---|
| 正文的库级→笔记→局部字号 | 16pt→12pt→18pt；退出局部样式后恢复12pt | 有效字号携带；浏览器字形/分页未证明 |
| list 字号、marker | library9pt/LIB-V2→note15pt/NOTE-M；ONE/TWO 嵌套 | literal Content marker；新增 marker 不授正文 writer；固定间距0.5em，完整 indent/body-indent/marker-align 未实现 |
| 字体覆盖 | Libertinus Serif14pt/list New Computer Modern9pt→note NCM12pt→局部Libertinus18pt | 原生字体候选与 CSS 名称转义；未导出/加载字体文件，实际 browser font/glyph/fallback 未证明 |
| 纯色 | 蓝色 #0074d9；标题红色 #ff4136；表格黄色 #ffdc00 | 渐变和拼贴尚未映射，不能冒充纯色 |
| 粗体与斜体 | strong700；delta:0=400；emph 为斜体；嵌套 emph 恢复正常；show-strip strong400 | 使用原生样式运算，不凭入口角色套用粗体；字体拉伸、特性、字距与字偶距尚未实现 |
| heading/custom show | 20pt/red覆盖 PREFIX/HOne/SUFFIX；native默认15.4pt/700 | native defaults，不猜 DOM UA样式；DOM h2 与业务 level1分开 |
| 简单 table border | library1.3pt/red→note explicit2pt/green 四边CSS | 原生四边值携带；CSS双边、border-collapse冲突、gutter/跨格/hline/vline/厚度拓扑视觉未证明 |
| 表格单元格填充与对齐 | 黄色；水平和垂直居中→局部右对齐和底部对齐 | 纯色与明确的原生对齐；内边距、列轨道和基线尚未覆盖 |
| authored HTML CSS | 原生css位于携带值之后，authored style按官方export追加 | 任意 raw HTML/custom CSS selector交互未覆盖 |
| dynamic marker / dashed border | flow没有FuncMarker，dash四边无CSS | 必须能力标记及native frame或明确局部缺口；自动fallback未实现 |
| 非零 h /自由分页几何 | 官方HTML警告并忽略非零h；真实br保留 | 不猜间距；需明确flow支持、原生frame或Paged入口 |

普通 inline TextElem携带局部样式；孤立 SpaceElem、SmartQuote、Pre/raw路径未逐项加局部carrier，仍走官方转换并继承容器CSS。任意局部空白/引号字体字号不能声称完整覆盖；逻辑空白/换行保存不是字形度量等值。五档指五种源码显示/编辑视图，不是字号 presets 或新的编译 target；本轮未验证这些 UI。

浏览器流式布局与 Typst自由分页布局存在几何语义差异。可变viewport、browser shaping、page-relative placement、跨页counter/layout context、固定列/浮动、字形细节不能由少数CSS值证明等值。普通正文/列表/简单表格应保持真实flow；依赖整页关系的构造应有明确Paged查看/导出。局部自足构造可用原生 `html.frame`，不从另一份PDF截片段，也不承诺保留page-relative全文关系。

## 最终消费案例

- `measure-discard`：最终正文只有 VisibleOnly，没有 MeasuredOnly/link；没有按measure调用栈筛选。
- `standard-matrix`：正文一个link，2×2 table一个link，最终 `<a>` child各一次；原生table/ul/li/br角色保留，没有按Span去重。
- `measured-par-used`：measure(body);body 的最终树只有一个p和一个br，没有把两个producer记录当成两次最终出现。
- `repeat-include`：相同文件heading两次，两个h2/native Locations与number1/2保留；没有root writer，不按同Span合并。
- `same-source-different-show`：FirstShow/SecondShow两个原生block div，保实际返回结果，不据同来源推相同内容。
- `show-context`：PREFIX/HOne/SUFFIX为最终display；StrongBody无假粗；GoneHeading仍在同次native heading introspection，display没有它，AfterGone=2。
- `runin-list-table-math`：native heading level2/counter存在，display是段落内strong及相邻正文；query.body不冒充完整post-show display，Span不授heading display角色。
- `whitespace-break` 与 `whitespace-strong`：soft source空白遵循官方HTML规范化，explicitLinebreak保 `<br>`；p/list/cell边界存在。直接拼最终Text会丢失br/段落边界；非零h未支持。
- `frame-target`：外部Outer=html、After=1；一个最终 `HtmlNode::Frame` 内部Inner=paged、FrameLink，有真实native Link destination。`unsupported` 的局部frame原生文本含FuncMarker0，Shapes有真实dash pattern；没有跨编译补内容。

Frame是局部视觉保真blob。内部TextItem只用作原生Frame检查，不是flow/copy reading tree。完整语义、AT文本替代和可编辑writer未证明，不能把FrameItem字串拼成可写正文。

本原型的 `data-probe-text` / `data-probe-marker` 是受控 fixtures 的测试识别标记，不是生产可信来源或 writer 资格。作者可以在 raw HTML 中伪造同名属性；产品需要不可由作者伪造的内部 provenance，不可据这些属性判断来源。修正后的 analyzer 仅折叠明确标记的 text-carrier span，并保留其它 span 结构及投影中的非样式属性；样式另由 CSS值检查。它还断言 stock fixtures 不含这些测试属性。此项是 oracle 修复：读取既有最终树重跑分析，未新增编译或 fixtures。

## EDITOR 消费合同建议

1. 每次普通笔记编译形成明确 Html flow结果；最终 HtmlDocument body child paths 是 flow display、Query display-body、默认复制和 RenderedLink的共同消费根。同次Tag/introspector可提供关联，不能加入未被最终tree消费的realization记录。Frame的localTarget为Paged，但编译归属、source snapshot与occurrence路径仍归outer Html结果。
2. native heading query/业务level/counter是另一明确语义域，来自同次HtmlDocument introspector；显示角色/文字/链接来自实际child。DOM h2不等于业务level2，业务level按native Heading.level。删除heading可保留业务query记录，但不加入display/copy。
3. copy对真实text/br/p/li/td结构使用明确序列化策略；空格/段落/表格分隔属于公开copy策略，不按坐标猜。head/style与Tag元数据不是正文。frame缺已证明reading text时，保局部保真/替代状态，不冒充可复制flow text。
4. RenderedLink枚举同次最终 `<a href>` 出现项。native frame内部链接可在明确frame域读取实际native destination或官方SVG发出的链接，不以source query link或Span去重；产品frame link dispatch/AT未实现。
5. Paged print/export是独立target结果；`target()` 分支随各target正常编译，不跨Html/Paged结果拼正文/counter/link/origin。Html frame内部由官方设置Paged，需显式localTarget。
6. literal writer需另验source槽、source snapshot、唯一可逆字面来源和当前出现资格；Span只是一项证据。generated/context、拼接、多输入、多出现、无法完整逆映射的custom show不给writer并保源入口。原型未实现任意值流或writer判定。
7. 每项样式资格应区分Supported/Partial/NativeFrameRequired/PagedRequired，不将partial伪装成完整覆盖。普通笔记不能整篇fallback成source/PDF。官方HTML experimental状态是版本维护/兼容风险，不是拒绝flow路线的唯一理由，也不意味用户样式要求已全部实现。

## 复核边界

有限关键样式、最终消费入口和measure/reuse/custom-show/硬换行/table-link反例已有可执行证据；没有扩展为完整renderer或Paged生产消费DAG。剩余缺口包括动态marker、自由几何、浏览器字体资源/视觉、表格边界拓扑、frame reading语义、自动能力/fallback与writer资格。没有真实浏览器/320px/字体加载/AT试验。

本包的两顺序patch、精确lock、fixtures、runner/analyzer、机器摘要与版本均可复核。两个patch在干净官方源码上check/apply成功，6个改动文件与执行源码逐文件相等；相同16例也从独立fixtures路径再次运行assert通过；收窄oracle后又读取既有最终树通过分析。公开摘要不替代独立复核；结论不超出这些有限样例。

发布前，协调者另从实际 Git 暂存区提取 LF 文件，在独立目录重跑同一 16 例及修正后的 analyzer；全部上述有限断言通过。公开机器摘要取自这次准确发布输入的实测结果。

官方背景：[HTML文档](https://typst.app/docs/reference/html/)说明语义结构与当前样式缺口，[target文档](https://typst.app/docs/reference/foundations/target/)说明target选择；具体接口以固定v0.15.1源码为准。路线可行性是有限实验推断，不是完整功能验收。
