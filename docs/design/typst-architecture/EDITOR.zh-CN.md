---
source_language: zh-CN
translation_status: source
---

[English](EDITOR.md)

# Typst 编辑、阅读、标题与 Word 的新项目方案

状态：设计候选，未接受、未实现。本文定义编辑、流式阅读、标题与 Word 输出的共同规则；实验与实际支持范围见 [REVIEW](REVIEW.zh-CN.md)。

[SOURCE](SOURCE.zh-CN.md) 拥有作者源与类型/来源解释，[CORE](CORE.zh-CN.md) 拥有身份、依据、授权与统一事务，[FEATURES](FEATURES.zh-CN.md) 拥有链接、Query、模板及转换。当前结果要求来自 [REQUIREMENTS](../typst-native/REQUIREMENTS.zh-CN.md)，这些 owner 的新候选与本文一起协调，前轮技术不成为新的规范依赖。

## 1. 选择与产品承诺

选择官方 Typst 语法作为唯一正文源。普通写作、连续阅读、源码编辑、分页预览和属性编辑共享一个文档会话。编辑器不以 DOM 或自行定义的富文档 JSON 替代 `.typ`。合法复杂源保全；局部不可逆构造在该处提供明确源码入口和官方版式预览，不拉平、不删除，也不把普通文档全部退为源码与 PDF。

新的候选路线为：官方 Rust parser/compiler → 官方 HTML target 的最终消费树 → 有界样式/角色适配及来源资格；官方 Paged target 承担分页与打印。阅读和打印是明确的不同原生 target，不把“必须从同一 Paged 结果重建流式树”当用户需求。官方 HTML 尚属实验功能，当前工作是验证受控集成的可行性，不宣称 stock HTML 或产品阅读器已经就绪；验收范围见§8及 REVIEW。

Core 继续唯一拥有结构、身份、读写授权、提交与恢复。本文只消费 Core 的获权源快照、ReadBasis、统一 Plan/Journal/Receipt 与 Job 抽象，不重新定义它们的持久格式。Core 对象变化必须协调本文的实际消费，不能只修接口名字。

## 2. 一个会话、一个当前编辑依据

获权 `readEntity(ref,current,purpose)` / `readScope(scope,purpose,completeness)` 返回对象身份、完整原源 bytes 和 ReadBasis。ReadBasis 包含真实版本、观察者 incarnation、文件 generation、schema/auth 与该操作实际依赖；相同 bytes 不意味着同一读写依据。编辑会话持有：Base 快照及 ReadBasis、可变 Draft bytes、单调 draftSerial、当前 selection、input generation 和一份 DraftUndo。编辑器局部 basis 为 `(session, owner, ReadBasis, draftSerial, exact Draft bytes, Environment)`；digest 只便于检查，不替代 exact bytes 或新鲜权限。

源码、富编辑、属性面板的成功本地操作都改变同一 Draft、递增 serial、加入同一撤销栈。一个批量粘贴、一次格式命令、一次表格操作、一次属性变更、一个 IME 组合、一次同行切换各为一个可见撤销单元；模式切换、布局、读取与编译不是撤销单元。Undo/Redo 得到新的 serial 和新地图；不复活旧写地址。并排另一篇笔记有另一份会话，返回恢复原稿和位置。

源修改产物始终是提案。保存向 Core `prepare(operationId,intent,draftBase,budget)` 交完整拟议源，消费实际计划检查后 `commit(operationId,planHandle)`；结果未知时保留稿件并查 `operationStatus(operationId)`。不换操作 ID 猜重试；终态冲突后新拟议内容重新准备是关联原 Draft 的新操作。外部修改/同步使用 Core 的 observeExternal/admitExternal/merge；编辑器显示当前冲突并保双方稿。当前版本变化不默默覆盖本地 Draft。

提交成功只更新该操作的决议。重新获权读取 after 之后更新 Base；用户已继续键入时 Draft 不被 after 替换，只有 Draft 与 after 精确相等才 clean。已提交撤销是新事务：仅在当前版本及 bytes 仍精确等于自己的 after、仍获权且反转不包含他人/控制/多实体效果时反转自己的源变化。否则给出冲突和保留稿，不误撤他人。崩溃恢复稿由 Core 的恢复能力保护，明确不是已保存正文或跨设备权限。

### 2.1 五档视图的共享实现边界

按 REQUIREMENTS §5.4 提供 V01 纯源码、V02 渲染且显示语法、V03 按需显示语法、V04 隐藏语法的界面编辑、V05 阅读。它们共享会话、Draft、serial、selection 和 DraftUndo，视图仅是个人界面状态。V01–V04 的成功操作写同一 buffer；V05 禁用正文编辑命令路由，独立批注按自身权限执行。连续/分页是独立布局与 target 选择，不替代其中一档。

V01 使用准确 source buffer 与官方 parser 的高亮。V02 仍按完整 source buffer/顺序绘制，保每个作者语法字符及准确选择位置，为已识别标题、强调、列表等应用可辨认样式，语法标记独立装饰且始终可见。编译语义样式须绑定当前 serial/来源；无效源仍可按 parser 显示并明确预览未就绪。不从 HTML 反序列化或重造 Typst 标记来拼 V02；生成结果可作明确辅助预览，不能替代、重复计入或删除作者表达式。

V03/V04/V05 的渲染部分消费§8同一当前最终投影。V03 在§3已证明唯一来源的活动构造处显示关联源码编辑区，直接编辑同一 Draft 的准确范围；显示语法本身不改源。键盘命令和触屏按钮可显式进入/退出。生成、多源或 include 出现项没有本篇 writer 时保只读并提供真实来源导航，不用相同字符串猜源。投影失败/待定时保 Draft，给明确 V01/V02 入口，不把旧画面当当前可写。

V04 使用相同写资格和§4具名操作，文档区不显示源码标记，也不因聚焦自动退为 V03。特殊构造由明确专用控件处理；缺控件则保留只读出现项，让用户主动切换可显示语法的视图，不能把隐藏语法解释成任意逆向编辑。V05 选区用于阅读/复制/批注，键入和粘贴不得开启正文写入。

切换先按§5完成或保留 IME，再依当前源/投影关联转移选区；无唯一显示位置时保原源选区、恢复最近可证明容器/阅读位置，不改变源点来假装映射成功。同会话进入 V05 默认显示明确标记的当前 Draft 预览；用户显式选择已提交版本才另看它，不能替换未保存稿。返回编辑继续原 DraftUndo。个人视图偏好不写作者源；V02/V03 语法装饰不进入正文 Query、逻辑复制、批注可见摘录或导出，显式复制源码仍返回获权原字节。

## 3. 可写区域的实际产生方式

当前完整官方 AST 与精确源字节产生 revision scoped 编辑地图。每个可写区域须证明：当前 root 的唯一构造/内容槽、准确 UTF-8 半开范围、标准绑定未被重绑定、当前 show/set 作用域、显示与源的可逆对应、操作允许改动的完整范围。Span、AST 路径、label、数组位置、相同文字、渲染坐标都不单独授写。官方 AST 给范围与语法，数据流和求值观察给对应与依赖；两者都不能省。

| 来源 | 阅读与查询 | 当前富编辑 |
| --- | --- | --- |
| 本篇唯一 literal 或适配过的标准 construct | 显示官方实际结果 | 证明成立的文字槽或具名结构命令可写 |
| 变量、函数、loop、show 生成内容 | 保实际出现项、真实来源与含义 | 无默认写地址；明确“编辑来源” |
| include/import 的内容 | 另行获权来源后呈现 | 本篇只读；打开该来源建立新会话 |
| 同一源重复输出或多来源拼接 | 每个实际出现项可读 | 无唯一逆映射则只读，不任选一个来源 |
| 不透明或缺可靠阅读顺序的布局 | 有界版式岛及准确状态 | 源码入口；不根据图中文字反写 |

依赖闭包按区域计算；旁边有 macro 不取消独立普通段落。全局自定义 show 真正影响全部正文且无法证明时，可以整篇 rich-readonly，但须解释实际原因、保 Draft 和 Source 路径；基本文档出现这种结果仍是基本能力失败。生成标题保真实 level 及查询出现项，无写地址不等于标题不存在。

空文档、只有控制数据/注释的文档和删除最后一段后，官方 AST 仍须产生正文 sibling 插入边界；空 list item/content slot 产生该槽的零宽插入点。注释、label、metadata 或表达式里的空白不冒正文插入点。Core/共享 Rust adapter 重算当前边界与能力；过期点整次拒绝，不能因 byte offset 一样继续。字节、UTF-16、scalar、grapheme、视觉 stop 显式换算，CRLF、combining 和 ZWJ 不拆。不同宿主使用同一版本化 Unicode 实现并跑对应矩阵，不继承旧文档写下但未证明的版本号。

## 4. 最低编辑矩阵与保源算法

所有富操作以具名 intent 输入共享 Rust 适配器，返回完整新 Draft 与可证明的新选区。只做目标范围内最小 splice，保范围外每个 byte；保注释、空白、EOL、作者函数和参数写法。不用 innerHTML 反序列化，也不 whole-file pretty-print。

| 必须交付的操作 | 默认原生表示与边界 |
| --- | --- |
| 任意普通文本位置输入/删、Enter、合并、删空再输入 | 当前正文槽；文字按 native literal codec 转义，不把 `#`/`=`/`-` 粘贴成代码或结构 |
| 粗/斜/删除线与跨格式文字选区 | 标准 strong/emph/strike；同段 inline tree split/merge，未选 leaves 与 trivia 原样保留 |
| 文档 title、副标题、section heading 与改级 | §6 的角色和唯一源值；实际 level 为正整数，控件常用 1–9，更多深度有数字入口 |
| inline/display 公式创建与修改 | 原生 math 源局部编辑器及官方预览；可视公式是 atom，不把 glyph 逆推源；定界符逃逸不能附加代码 |
| bullet/enum/terms 的创建、拆并项、缩进 | 原生 list 构造；空正文可创建实际空项继续键入，裸 marker 不冒成功 |
| 链接插入、地址转换 | 消费链接 owner 的 encoder/resolver；源写与目标读分别验证，保文字、fragment 与同一目标 |
| 普通网格 cell、增删行列、行重排、矩形 TSV | 仅官方可证明固定 columns、显式 cells、无 span 的完整简单网格；失败整次不改 |
| header/span/block cell 的局部内容 | 保完整拓扑；唯一源槽可单格改，复杂拓扑命令进入 Source/高级入口 |
| Source 编辑任意合法 Typst | exact source，不受控件矩阵限制；不同有效性与保存结果分别说明 |

格式跨界、跨普通段落/列表的删除和 plain paste 使用覆盖完整范围的结构命令，一次撤销。跨 heading、title、table、code、math 或 generated island 的写选区缺具名算法时拒绝该操作并给准确 Source 选区；阅读复制仍可用。Copy Text 按逻辑顺序含明确块分隔；Copy Source Fragment 交获权的准确原始片段，不拼接合成源。Cut 先确认系统剪贴板写成功再改 Draft。默认 paste 只收 text/plain 并编码；无 plain flavor 显示不可用。Source paste 明确处于源码模式。

表格的生成 cells、spread、ragged、span、header/footer、自定义 show 均原样保留，缺控件不变成矩形网格。两个相同 cell 依当前 AST 槽区分，只改用户选中者。行重排只对简单网格证明 comment/trivia 归属：cell 内随 cell，行间 leading trivia 随后行，closing 前尾 trivia 固定；不明归属拒该 reorder。矩形 TSV 显式预览尺寸/扩展，一次完整预算/权限验证，不能部分贴入或拆批绕预算。

每次成功适配须官方完整重解析、证明 intent 的结构和值、范围外 bytes 不变、非目标控制/字段/构造不变、caret 能唯一定位；依赖 active show 的可视操作须重新求值检验，不能 AST 相同就称显示正确。错误岛只允许有完整边界及依赖证明的其它区域富编辑；错误吞后文不能猜安全分区。

## 5. 输入与迟到投影

原生输入控件即时显示尚未验证的本地输入，不逐键等待整文编译；共享 source buffer 是唯一 Draft。parser/compile/layout 在 worker。结构变换每个 controller 最多一个在途，冻结旧 basis 与目标，后续输入有序保管。回应仅在对应 serial/generation 仍适用时安装；后继输入存在时，只在可证明可重放的队列上重算，否则保新稿及未应用文字供源码修复。旧响应不得替换新 source、选区、焦点或清 clean；没有“最后返回者获胜”。

IME begin/update 仅 native preedit；composition commit 是一个 Draft 操作与一个 Undo。宿主 final-input 的重复事件去重，cancel 保未提交文字恢复机会。组合中 save/Enter/slash/格式、潜在链接转换、窗口切换和虚拟化不得抢组合 owner 或提前提交；失焦行为按真实宿主验证。active/focused/composing 节点不被回收。权限变化或 adapter 失败保用户新输入的恢复稿，但不再从缓存回显失权原文。

layoutEpoch 只描述当前几何，不改源 serial。当前草稿语义未验证时明确 pending，旧预览可显式看且标版本，不给 current 编辑/批注目标。语法、类型/应用控制有效性、编译、layout、预算、权限、保存与投影新鲜性分别报告。合法语法但编译失败的正文可在 Core 对当前格式/控制/字段校验通过时保存为有诊断的稿；syntax-invalid 若 Core 尚无正式 checkpoint 能力只进恢复稿，不能虚报普通保存成功。源码导出保准确 bytes，可独立于编译。

## 6. 标题与副标题的唯一含义

文档标题使用官方 title，章节使用官方 heading；无标题合法，文件名/路径/摘要只是 UI 回退，不补作者 title。默认新建带标题笔记用一个 document.title 值和 `#title()` 呈现；属性面板改同一个源值，导出读取同值。直接 `#title[Title]` 是合法原生作者形式：无 document.title 时可把唯一直接 title.body 作为标题来源，不强迫双写；两者显式不同则分别显示“元数据标题/显示标题”，不给无损统一按钮，源码保留。复杂生成 title 可排版与读取实际值，只有能证明唯一源地址时面板可写。重复 title 或混乱 role 是应用诊断，不能据此删除合法 Typst 源。

```typst
#set document(title: [研究设计])
#title()
#block[一项定性与定量研究] <weftext-subtitle>

= 背景 <background>
普通正文。
```

副标题采用原生 block 与唯一 label `<weftext-subtitle>`。应用约定它是唯一文档 title 的附属 frontmatter，独立于 title.body、heading counter、outline、编号与章节级别；不用冒号、字号、位置邻近来猜。面板创建副标题需有唯一标题；删除标题时明确预览同时删除关联副标题或将它保留为普通 block 并移除角色 label，一次操作。无 title 的同名 labelled block 保源并报关联缺失，不自动变章节标题。多个同名 label、非 block 或不可确定关联时不提供副标题 writer。作者可改排版，控件不要求字号/间距参数。template 可复用 document.title/title()/该 block 而不维护另一份业务 title。源码可用标签自行引用副标题，应用不把它当 section ref。

上面是建议采用的轻量应用约定，需要 SOURCE/schema owner 同步；label 只说明角色，不是持久身份或权限。官方说明 title 与一级章节的区别及 frontmatter 应在 title.body 外；本方案在此基础上定义应用副标题关联。[Typst title](https://typst.app/docs/reference/model/title/)

## 7. 同级逐标题混排与完整层级

每个 section heading 保 native heading 身份、真实 level、原 counter、outline 与 label 引用。普通 1–9 控件以外的合法 10+ 可读、可查询、可导航、可保源；深度输入用正整数，不压成 H5/H6/H9。渲染使用 role=heading 与准确 aria-level，不强迫有限 HTML h1–h6 代表完整模型。字号独立于角色。[Typst heading](https://typst.app/docs/reference/model/heading/)

单个 heading 的“单段/接正文”选择写原生 labelled show 规则到作者源，默认选 label；已有唯一作者 label 可复用，没有则产生不冲突的普通 label 并显示在 Source 预览。它只选择这一个 heading，不使用 heading.where(level: N) 强制整级。独立样式是默认规则；同行规则输出原计数器与 heading body 后接普通空格，紧随同一原生首段。无需第二计数器、隐藏 block ID、间距尺寸或共享 presentation sidecar。

```typst
#set heading(numbering: "1.1")
#show <methods>: it => context {
  strong[
    #if it.numbering != none {
      counter(heading).display(it.numbering)
      text(" ")
    }
    #it.body
  ]
  text(" ")
}

== 研究背景 <background>
独立标题的正文。

== 访谈方法 <methods>
同行正文并保留 *作者强调*。

== 问卷方法 <survey>
独立标题的正文。编号引用 @methods。
```

应用 writer 对这项受控 native 规则提供唯一识别和反转。开启先证明 heading 后有可接的普通首段及准确 scope；有空段时预览移除恰该段落边界，有列表/表格/另一个 heading 或不明构造时不吞它，解释并让用户选择合格正文。未知 show、counter reset/自定义 numbering 或模板接管同目标时先保原源，不能覆盖；可证明的原生规则才给控件。关闭只去本规则并恢复必要段落边界，新增 label 保留以免破坏作者引用。两次切换不重复注入规则。保存/同步/恢复携带同一 `.typ` 源，所以跨设备呈现不靠本机状态。

增删/移动/改级保持选中 label 和原生 counter；不写固定数字。outline 用 heading body，不带同行正文；原生编号引用与链接分别处理。无编号 heading 的默认 @ref 不承诺自动可用：有限实验已见官方报错，提供明确 label #link 形式；不暗改作者引用。单段和同行混用统一编号是已执行有限证据；深层、长段落、孤行、非拉丁、分页及交互仍需验收。

## 8. 原生双目标：最终消费树、样式与来源

Core `compile(sourceSnapshot,environment,target,budget)` 供应固定官方 Rust build、fonts/packages/assets 及已获权只读 World，返回该次 CompileSnapshot。普通连续阅读选择官方 Html，分页预览/PDF/打印选择官方 Paged，不新增作者语言或伪 target。每份产物绑定准确 source、环境、target、依赖及权限；同源不意味着两个 target 有相同正文、计数器或读取集合。切换 target 是新编译，不拿另一份结果替换当前投影或恢复旧写资格。[官方 target](https://typst.app/docs/reference/foundations/target/)

**最终消费是唯一入口。** 从成功编译返回的官方 `HtmlDocument` 根开始，只遍历其实际 `HtmlNode` 子边，按原序给最终出现项分配本投影内的 key。不能遍历所有 evaluate/realization 事件、Tag 原始 body 或 Introspector 元素并假称这些都显示了。尺寸测量后丢弃的 link 不进入 RenderedLink；表格中只显示一次的 cell 不因测量产生多条；同源真正输出两次仍保两次。原生最终节点是阅读结构输入，序列化 HTML 是交付格式，两者都不是新的作者源。

三个产品分域：AuthoredLink 来自可识别作者构造；RenderedLink、正文检索、逻辑复制、可见批注摘录和离线链接 route 来自所选 target 的最终消费投影；heading 产品来自该次最终 native Introspector，保真实 level/label/counter 及其显示对应或无显示状态。show 删除 heading 仍可能保 introspection，不能把它补成可见文字。原生 HTML 的 h2 等标签不能反推业务 heading level；由原元素的真实语义和实际消费关联取得。任意 show 输出只保实际 return，不把入口 strong/heading 的角色强加给返回的普通文字。

**有界原生样式适配。** stock HTML 主要输出语义结构，不能据它成功就声称原生字体、字号、列表 marker 或表格边框已保留。适配器在官方 HTML 元素实际生产点消费已有 effective StyleChain/已求值参数，把具名可表示属性转为受控 CSS/角色/来源旁信息，随所产生的节点进入最终树。不能扫描作者 set 文本重造优先级，不能为样式再执行 show/query/context，不以所有稳定日志做文字匹配回填。基础正文/列表/表格的库默认、note 和局部覆盖须实测；支持表随实现版本发布，未知单位/paint/自定义排版不静默忽略。此适配是内部目标转换，不要求作者写 CSS 或另一套样式配置。[官方 HTML 范围](https://typst.app/docs/reference/html/)

最低 renderer 仍须让普通中英/RTL段落、title/subtitle、heading、strong/emph/strike、列表、raw/code、链接/引用、图像 alt/caption、普通及 header/span 表格连续滚动并随 320px 重排，宽二维内容仅局部横滚。基本公式优先消费官方 MathML，再验证实际 AT；必要原生 frame/SVG 带准确数学或文字替代，不以图片冒无障碍通过。HTML 标签/属性/样式/资源均经安全的 typed 转换；脚本、任意宿主文件/网络和不受控事件处理器不执行，受影响的合法原源保留并明确报告显示限制。只有 Core 获权资源句柄可被 renderer 读取，HTML URL 不能绕过 World/资源交付权限。

**局部排版与 target 边界。** 页面计数、绝对定位、浮动和复杂图不保证与可重排流式布局等价。已明确的原生 `html.frame` 在同次 Html 编译内调用 Paged，其内部 `target()` 实际为 paged；记录这个边界并消费官方返回的 frame，不从另一份整篇 PDF 按坐标截出片段。能够在当前官方调用链中安全形成局部 frame 的内容可显示为带替代的版式岛；没有合法局部替代时明确该段或该文的 Flow 不可用，保 Source 和独立 Paged 预览。不得把一般段落、列表、表格都降成图片或整篇 PDF 来声称满足普通阅读。任意自由排版保源与可用的官方输出，并不等于所有原生效果都有无损重排。

**来源与验证。** 最终节点携带可验证的 native source/construct 关联时，再与§3当前官方 AST、唯一作者槽和实际 show 作用域证明结合，产生富编辑能力。只有 span、相同字串、DOM位置或一次出现仍不够。计算字符串、多次输出、动态替换和 include 均保实际出现项，缺唯一逆映射只给来源入口；不从 PDF/截图反推源。透明库 entry 中的真实 note 与普通用户 include 区分，不能因宿主 wrapper 存在而取消所有普通正文编辑，也不能扩大 imported source 写权。

旧 [11 例观察实验](probes/semantic-bridge/README.zh-CN.md) 仍证明其有限无副作用/缓存重放事实，但独立 measure/table 反例已否定“稳定事件合集就是阅读树”。后续 final PDF consumer 改善了选择，却仍缺可靠的布局前段落分隔关联，因此本候选撤回从 Paged trace 构造默认流式树的选择。新路线须对最终 HTML tree、同源重复、测量丢弃、表格、库样式覆盖、target/frame、counter/依赖及只读来源逐项做有界证据；不以 experimental 标签直接否决，也不以几例编译宣称全 E1。固定 compiler/适配版本，升级重跑必要回归，维护范围不得扩为另一套 Typst 求值和排版器。当前实验与独立裁决见 REVIEW。

### 8.1. 一个普通模块与一个自动编译入口

选定方案：库级默认样式存于普通 `.typ` 模块，Core 库设置仅选择该获权 module EntityRef 与启用状态；模块 bytes 是样式权威，设置只负责选择，不另存一份字体/边框事实表或新 style DSL。设置界面编辑模块中的可识别 native set/show-set 参数，通过模块自身的 Core Plan 保存。高级源码可编辑完整合法样式；未知规则保源并标明该控件不可无损编辑，不全文件规范化。

每次笔记编译由 Core 自动构造可查看的临时 entry，**用户不需每篇手工 import**。entry 导入选定 module，执行它的标准内容包装函数，再 include 获权 realnote。entry 是派生编译输入，不写进笔记、没有笔记身份，不成为第二正文权威。禁默认入口和笔记隐式重复应用同一 module；笔记显式引用相同 module 时说明其额外 native scope，不能猜测去重删除作者源码。

```typst
// library-style.typ：普通库样式源
#let library-style(body) = {
  set text(font: "New Computer Modern", size: 11pt)
  set list(marker: [•])
  set table(stroke: 0.5pt)
  show list: set text(size: 10pt)
  body
}
```

```typst
// entry.typ：Core 自动生成且可查看
#import "library-style.typ": library-style
#show: library-style
#include "notes/realnote.typ"
```

模块入口函数名/调用 profile 是应用集成约定，源码仍是普通 Typst，不是新作者语言。初始化库可供应默认模块，用户只在库设置修改一次；禁每篇偷偷补写 import。用户明确选择不采用库样式则该 compile profile 无 wrapper，界面说明实际环境。当前被选样式损坏/失权时报告准确 layout unavailable，保笔记 Source/Draft；不能偷偷回退另一版/default 模块并冒同一输出。

### 8.2. 原生覆盖语义，不编造通用优先级

库默认尽量用 set 和 selector show-set 组合，字体、边框等属性不要放进重建元素的 transformational show 内部。单篇用 set/show-set，内容块内的规则只作用该块；单个 table 的 stroke 等显式参数可覆盖该默认。作用域、选择器、规则顺序、显式元素参数和 transformation 都遵循官方求值，不定义“单篇永远胜过库、局部永远胜过一切”的 CSS 式规则。单篇一般 text set 也不承诺覆盖所有库级 selector show-set；需要对应 native 选择器或明确局部元素属性。

官方特别建议将可组合的 show-set 与 transformational show 分开，因 transformation 内部 set 不再可由后续 show-set 覆盖。本轮最小实验证实这个反例，库 UI 应提示不透明/固定变换对覆盖的影响。用户仍可用高级原生 transformation；不能为了控件能力删掉它。[Typst Styling](https://typst.app/docs/reference/styling/)

列表的 marker、字体与正文等可分别设置，表格的 text 与 stroke/fill/inset 等分别使用标准 text/table/selector 参数。只向 UI 暴露有清楚 native AST 及 source writer 的属性：改默认只写 module，改单篇只写 note，改局部只写该 scope；每次一个明确 DraftUndo 与 Core 事务，不把三种改动混成一键不明影响。预览列受影响 scope，尊重已有原生显式参数。[Typst Table](https://typst.app/docs/reference/model/table/)

### 8.3. 源身份、相对路径、编辑与阅读

SourceSnapshot 和应用身份/字段资格始终来自真实 note 原 bytes/官方 AST，控制声明仅由该 owner 的源判读。wrapper/module 中出现 metadata、id/title 相似值不能出生笔记、覆盖控制字段或授权限。应用不从 synthetic entry 的 AST 重扫身份，也不因统一 wrapper 使所有笔记成为新源对象。

entry 的 source map 声明 designated authorSource=realnote；只有 Core 生成、完整证明恰一次包含当前 note 的入口才能让 bridge 把这层系统 include 认作该会话的作者源。仍须以 realnote 当前 AST、ReadBasis/Draft serial、实际 show scope 和唯一可逆来源检验富编辑。笔记自己写的 include/import 仍是独立来源，重复/include/generated/multi-origin 不授当前笔记 writer。不得把 realnote 因 wrapper include 一律变只读，也不能用此例外将任意 included 文件变可写。此桥消费与区分系统 entry 的实现仍须实测。

relative read/image/import/include/bibliography 等按照真实源 FileId 与所在 source 的位置解析：note 的 read("asset.txt") 找 note 旁资产，module 的相对路径找 module 旁资产，不能改为 wrapper 所在位置。Core World 用受控原路径映射提供获权 exact bytes；移动 note 的 native 相对路径含义与既有链接/资源规则一致，不偷偷补全或改写源码。包括绝对/包路径都受 World policy，无 ambient 读写/网络。样式控制呈现；dark/zoom/high contrast 是个人阅读层，默认不进入作者样式 module/官方导出环境；不可访问颜色/字体可在阅读层调整并明确与分页输出的差别。

### 8.4. 冻结依赖与样式改版

CompileSnapshot/Environment 与 DependencySet 记录实际库设置选择、style module RevisionId/bytes、entry profile/digest、真实 note/Draft、实际 module import/package/font/asset World 读及 target/budget。无权 style 不用已缓存内容续编译；包不静默取 latest。文件源有效性和源码保存独立于默认样式能否编译，样式错误不允许毁坏 note。

样式模块保存得到新 RevisionId，但不生成每篇笔记的假修订或修改 note AST/ID。模块/库选择改变后，依赖该样式的 preview、layout map、可视编辑资格、编译派生Query以及待交输出标 stale；新读/编译重建实际影响，未消费 style 的静态字段查询不受无关失效。已保存旧产物可显式看旧版，不冒当前。迟到 v1 preview 不可安装成 v2，不能改 Draft/Undo/选区；外部字体/资产/模块当前性变化同理。

Core 产品验收应实际冻结 v1 compile→提交 v2 module→交迟到v1→确认拒绝current安装→新compile采用v2，同时 note revision/bytes不变；ACL撤权、缺module、wrapper错误与换target分别测试。本文 CLI 只证明两次真实编译的差异，没有运行 Core cache invalidation 或当前授权门。

### 8.5. 准确源码导出与独立使用

准确 note `.typ` bytes 导出仍交原件，明确它不单独携带库样式。另提供“含库样式的独立 Typst 工程”完整 bundle/entry：获权 note 原 bytes、固定 style module/实际import与assets、可查看 entry、所需环境/包/字体清单和许可/loss，保持相对 source 路径或完整可验证路径映射。编译默认入口为 entry，绝不将 wrapper伪作原 note。只能交获权完整依赖；不可分发字体/包或闭包不完整时具名 loss/route unavailable，不声称独立无损可复现。不能仅导单个note再承诺同样样式。

PDF/可编辑 DOCX 和其他呈现输出消费同一冻结 style dependency；DOCX 必须另行验证 effective styles 映射，普通 native defaults 可转成 template/paragraph/run/table 样式，任意 show 不能当成全可表达。Job 固定 exact staged bytes 与 loss；style改版使未冻结预览/待准备内容失效，已确认文件下载续交仍是原 exact bytes并检查权限，不因全库样式变动重编译换内容。此处不扩为独立 Word 样式优先级协议。

## 9. 渲染与权限安全

编译 worker 无 ambient workspace、任意路径、网络、凭据、Action 或作者写能力。Core 只供应实际获权依赖与冻结环境；未获权 include/package/resource 不返回内容，错误不泄隐藏路径/数量。编译受时间、内存、递归、输出和进程树预算；失败保稿，worker exit0 不等于输出可交。

阅读树生成宿主固定节点与样式，文字是 text 节点；作者 HTML/CSS/JS 不直接进入特权 DOM。SVG/实验 HTML 经 allowlist 验证，禁 script/event handlers、foreignObject/主动嵌入、表单、外部 font/fetch 与未准协议；资源使用 Core 受控地址，链接交 resolver。WebUI CSP/隔离无特权 origin/sandbox，Desktop webview 同样无文件/进程桥。清洗影响合法内容时明确 loss/local unavailable，不能默默删除称保真。

渲染/缓存/搜索摘要/thumbnail 都绑定实际消费 scope 及当前 auth。撤权停止新交付、异步安装和装饰，隐藏受保护缓存；已由用户键入的恢复稿保留。只字段权限不产生正文摘要、title 投影以外的整源或缩略图。不同 source/annotation/resource 各自授权，源读取不顺带授下载/发布。环境 cache key 包括实际依赖版本、schema 和 target；旧成功产物仅显式旧版本阅读，不提供 current 操作资格。

## 10. 可访问性与上下文

键盘、触屏与 AT 有等价的标题操作、来源入口、表格操作、冲突/错误入口、返回和潜在链接候选选择。logical order 来自 bridge，不按视觉 x/y 或 DOM 临时次序拼。caret 维护逻辑点及视觉 affinity；Left/Right 依宿主视觉方向，词移动/删按共享逻辑字素，copy 按逻辑顺序。RTL 隔离 path/email/code 不改 bytes；表格列 identity 不镜像颠倒。

大纲保持实际深度、过滤/折叠/当前节/聚焦；聚焦和专注不删源。导航→列表→正文及窄屏返回保 Draft、Undo、筛选和可恢复位置。local 命名布局与 NAV01–05 列表偏好仅记个人呈现及可重新解析目标，不存结果、权限或旧地图；恢复重新授权，目标失效不猜同名。迟到缩略图/preview 不抢输入焦点。潜在链接只是提示；多个同范围/交叉范围及溢出都可显式选，不能最长匹配吞掉其它候选，转换继续走链接 owner。

每个平台分别测真实 CJK IME、Arabic/Hebrew 混排、emoji/combining、键盘/触屏、screen reader、200/400% zoom、320px、高对比、减少动画。live region 不逐字报 preedit；focus/composition 节点不因虚拟化消失。缺某 surface 能力准确标 unavailable；CLI 不宣称图形验收。性能保留待实测目标：输入反馈 p95≤50ms、layout≤100ms，1000事件 cold/warm 各3run；Desktop 10MiB/100k lines，Mobile 1MiB/10k lines、单行1MiB，steady memory 256/128MiB；十倍超限不丢稿。目标未跑不得冒通过。

## 11. 独立批注的协调边界

独立 JSON 存批注、回复、解决状态和目标历史；不把它混为正文或呈现权威。persisted target 由 Core 在已提交的获权源版本上建立精确文档/元素/范围或 PDF/image/audio/video 区域，Draft/DOM/projection key 不持久化。正文变更仅通过 Core 可证明 unchanged-range 映射延续；覆盖、gap、歧义则 orphaned/candidate，保讨论，不相似猜重锚。生成/多源 display 可整文批注或显式选真实来源，不能造源 range。

annotation 读取只交自己的已存摘要/正文/历史，不自动读取已隐藏目标、截图或重算 context。manual reattach 明确选新目标并使建议需重新确认。建议 apply 通过统一多实体 Core 事务同时修改准确当前源与 accepted 状态；其中一个失败整次未成功。reject 只需批注资格，零额外 target read。只读来源可引用到可写目的，引用复制的文字是目的作者内容，不证明来源仍 current。批注 source mapping、媒体精度与多实体提交仍是 Core/annotation owner 的实际实施门；本文不复活旧 locator/transform wire。

## 12. 可编辑 Word 默认与已知边界

DOCX 是 Weftext 自有转换器，不是 Typst 官方内建输出。默认语义转换 profile 消费一次冻结 Html 编译的最终语义树、同 target 的标题/引用产品及准确来源关联、获权资源；不自己重跑 Query、猜第一表或混入另一份 Paged 输出。自定义 show 后未显示的 native heading 不能凭 introspection 补成幽灵段落。明确选择 Paged 语义转换时是另一个具名 profile，预览与输出均注明 target，支持范围单独验证。局部 html.frame 保同次 Html 编译的来源边界。转换保真实段落、run、table、bookmarks 与可编辑资源；无法表达的复杂布局明确 loss/unavailable。标题映 Title、关联副标题映 Subtitle；不进 section counter。此路线与 FEATURES §11 一致，最终语义生产者仍受 E1 验证门约束。

1–9 级 section 映真实 Heading N、outlineLvl=N−1 与同一多级编号。独立 heading 和同行 heading 共用编号定义。同行默认是一整个 Heading N paragraph，前部 title runs，后部 body runs：这正是已认可方案。正文字符格式来自模板正文占位符/Normal 的**有效字符属性**，解析 docDefaults、theme、basedOn、字符样式与直接属性继承；显式复位 Heading 继承的 bold/italic 等，再叠加作者强调。不能 copy 空 raw rPr 就称复制了有效样式，也不硬编码字体/字号。段落级缩进、行距、keepNext 等仍是 Heading 段属性，预览须如实展示。[Microsoft 段落样式](https://learn.microsoft.com/en-us/office/open-xml/word/how-to-create-and-add-a-paragraph-style-to-a-word-processing-document)

默认 Word TOC/导航按整段 Heading/outline 处理，可能把同行后部正文收进去；正文 run 换字体不会改变段落范围。输出预览/说明必须写明这一点，不能冒称默认目录只含标题。编号引用用准确 heading bookmark 与原生字段，不固定数字。[Microsoft 标题多级编号](https://support.microsoft.com/en-gb/word/number-your-headings)

“只标题目录”是可选细化：style separator 或显式 title bookmark/目录生成均须具名 route、真实应用验证及 loss 说明，不是默认完成的条件。默认保原生 Heading 和多级编号，不以写死编号或整篇图片掩盖问题。9级以上的 Word 标准 outline 无同等级别可映射：保源真实深度，明确该导出不具备原生深层大纲/编号等价；用户不接受此损失则 DOCX route unavailable。可选择保实际层级信息的自定义样式/可编辑文本，但不能命名 Heading9 冒等价，也不声称动态编号全保。[Microsoft outlineLvl 的 0–8 层与 9 无大纲语义](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.outlinelevel?view=openxml-3.0.1)

Job 完整生成并验证准确文件与 loss，再显示结果；内网下载不以外部发布为前置。确认固定源、模板、依赖和 staged bytes；resume 只交同 bytes，每 chunk 复验当前 auth，过期再生成是新计划。保存、打印、下载、保存 Resource 与外部发布各报告自己实际结果，不以参考渲染图冒 Office 成功。

## 13. 当前证据、决定与实施门

有限实验：Typst 0.15.1 官方 compile/query 已验证单个 labelled 同行 heading 与同级独立 heading 共享 counter/outline/ref；增删、移动、提升/降低自动更新，size-free 样例无必填尺寸；subtitle block+label 可查且不入 heading。空段打断同行、无编号默认 ref 报错也有反例。独立 DOCX 有 10 项 OOXML 检查，保 Heading2、真实九层编号、正文字符复位/强调及 TOC/REF 字段，但只是受控样例。32 个旧有限 Rust 检查及少数源 splice 不覆盖完整 editor。新方案没有重跑这些实验，更没有生产通过。

本轮另执行七个官方 CLI 样例与独立官方 AST 枚举：default、自定义 heading show、删除 heading show、剥离 strong 的 show、重复 include、loop 生成 heading、Paged/Html 分支。自定义 show 的 query.body 只有原标题，而 PDF 实际有 PREFIX/body/SUFFIX；删除 show 后 query 仍有 GoneHeading，PDF 无该标题；重复 include 有两个实际 heading/counter[1]/[2]，root AST 无 included text 槽。两次 query 相等只是有界可重复性，不证明内部收敛轮；剥离 strong 样例未测字体/role。CLI/AST 不产生流式桥通过。这些反例促成 §8 的最终消费树路线，当前有限证据与未跑范围见 REVIEW。

本轮新增 [Html 最终树与样式探针](probes/html-flow/README.zh-CN.md)：16 例关闭适配与官方 Html 字节一致，开启适配的串行/并行最终树一致，三种 Paged PDF 字节一致；消除 discarded measure，保重复 include 和不同 show 的真实出现项。库→笔记→局部字号、字体候选、强调/斜体及简单单元格边框和对齐有实际值证据。它没有实现浏览器 renderer、逆向编辑、复杂表格拓扑或全部样式；动态 marker 反例增加一次诊断源读取，World 顺序只在 15/16 例一致。局部 html.frame 的原生 Paged 内容可见，不证明其流式复制或无障碍树。产品适配开关必须属于固定 worker/environment/cache profile，不能在同一缓存中改变进程环境开关；探针标记不是可信来源或写资格。

规范 DOCX renderer 未找到 PATH 上 soffice，真实安装探测见 Office/LibreOffice；因此是“规范渲染不可用且实机未跑”，不是“机器无 Office”。ReportLab reference 图不是 Office renderer。真实 Word/WPS/LibreOffice 的字段更新、TOC 范围、导航、字体继承、style separator、保存再开和深级行为全部 UNRUN。

| 门 / 对应需求 | 必须做的真实验证 | 通过前阻止什么 |
| --- | --- | --- |
| E1 / RQ01–03、06、§5.3 | 已有 16 例官方 Html 最终树及有界样式原型；独立核最终消费选择、原生语义、target 边界、保源降级与不可伪造来源方案 | 独立接受前不关闭桥设计问题；完整 renderer、编辑逆映射和样式支持仍须实现验收 |
| E2 / RQ02、24、§5.4 | 五档之间共享源/Draft/Undo 与语法显示；真输入 surface：空→输入→删空、任意 caret、跨格式/块、math/list、等值 table cell、IME/Undo/迟到响应及权限/ABA | 阻止普通编辑器完成声明 |
| E3 / RQ03、23 | 真 Desktop/WebUI/目标 Mobile 的 AT/RTL/zoom/320px、sanitizer/隔离/资源撤权、预算和性能 | 阻止该 surface 阅读与无障碍通过 |
| E4 / RQ04–07 | 深度1–9与10+、titleless、副标题角色、逐标题同行、长段落/孤行/分页、counter/custom scope | 阻止全标题矩阵通过，有限原生证据仍成立 |
| E5 / RQ07、20 | Word/WPS/LO 实机打开/更新字段→增删/移/改级→目录/导航/引用→再存/再开；不同模板、CJK/RTL与正文有效字符格式 | 阻止 DOCX 互操作支持声明；style separator 单独验 |
| E6 / RQ18、23–24 | 精确批注映射、媒体区间、隐藏目标零额外读、建议多实体事务与崩溃/并发 | 阻止批注/建议应用完成声明 |

可直接进入协调候选的决定：唯一 Typst 源；共享会话与一个 DraftUndo；basis 新鲜性与统一 Core 提交；局部保源/不捏造生成写地址；title 与 heading 分域、轻量副标题 label；逐标题原生 show 而非整级设置；普通用户无间距参数；Word 默认整段 Heading N 与有效正文字符格式，并明示目录范围。E1 已有可运行的有限桥样例，仍需独立确认可实施边界；E5 是输出关键实施门，不能拿现有 OOXML 图替代。具体性能参数、样式分隔符可选路线与 hook 维护成本可随原型修订，需求本身不因此消失。

## 14. 需求覆盖与替换范围

本提案直接承担 RQ01–07、18、25 和 NAV01–02 的编辑/阅读交互；RQ08–11 属性、RQ12–14 链接、RQ15–17 Query/任务、RQ19 模板、RQ20–24 Core/转换/权限由各新 owner 供数据与动作，本文明确其会话、源、焦点、依据、loss 和权限消费。NAV03–05 统一 Query 与 local 偏好由导航 owner 协调，不建编辑器专用事实或查询语言。

对旧 EDITOR §§1–12 全部用本文新会话/地图/桥/输入/批注交互/门替换，不保其 wire、closed union、history decoder 或版本 pin 义务；旧正文仍保留为固定输入记录。FEATURES §§7–8 的用户输出/导入/下载结果保留，新 conversion/Core owner 另定机制，本文替换标题与 Word/editor 消费。SOURCE 的旧 title 单一 writer 限制、旧 carrier/locator 与强制格式资格不自动进入新机制。没有真实兼容对象就不虚构解码器；后来发现真实用户数据则保原件，单独确定导入与恢复范围。作者方案必须再做需求覆盖与非作者复核，不能把原 CLOSED 复制为本轮通过。
