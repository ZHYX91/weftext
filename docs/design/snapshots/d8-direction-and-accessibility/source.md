---
_weftext:
  id: "3e8f9777-29c5-4ee7-9098-b924becd255c"
---

当前权威状态：D8 D8-r03-p2-2026-09-24，仅由总控验收（外部控制记录未随本输入发布）和committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。下面送审正文中的candidate/未激活为历史标签，不覆盖本段。架构接受不代表产品实现；D9未启动。

# D8 Direction、双向文本与可访问性

revision: D8-r03-p2-2026-09-24；candidate，属于D8规范。此文件逐项回答2026-09-01 RTL必审输入。内容保存、bidi交互、RTL shell和UI翻译分别验收，互不推导。

## 1. 标准、版本与产品选择

本代的字素/默认单词分段及bidi测试固定Unicode18.0.0：使用[UAX #9 revision52](https://www.unicode.org/reports/tr9/tr9-52.html)与[UAX #29 revision49](https://www.unicode.org/reports/tr29/tr29-49.html)。版本已在2026-09-23核对官方发布页；实现须固定相同数据表和测试语料，不能依每台设备的未声明ICU版本改变语义。UBA处理逻辑字符顺序的显示，分段处理移动/删除边界；视觉shape、字体与折行仍需要平台实测。

[Input Events Level 2](https://www.w3.org/TR/input-events-2/)用于识别浏览器输入/组合事件差异；其文本不等于浏览器行为已被证明，D8按实际事件归一而非假定全部beforeinput可取消。[WCAG 2.2](https://www.w3.org/TR/WCAG22/) AA作为WebUI验收目标；原生表面增加对应平台辅助技术实测。以下键位、默认方向、镜像和存储选择是Weftext产品合同，不声称标准替本产品决定。

## 2. 方向权威、继承与持久化

| 层 | 选择、缺省及override | 保存方式/边界 |
|---|---|---|
| shell | user setting=`ltr|rtl|auto`；auto只按所选UI locale的locale descriptor确定；无已安装descriptor则ltr | 当前用户设备偏好；不读正文第一个字符决定shell；修改不写作者source |
| 文档Read/Write | document display override=`ltr|rtl|auto`；默认auto；explicit作为paragraph base direction，auto用该paragraph首强字符，无强字符继承本次document fallback | 按principal+authority+NodeRef保存可丢弃的个人阅读偏好；fallback为本次shell dir；不复制为其他owner的作者事实 |
| block/paragraph/cell | 显式session override优先，再document explicit，再auto；auto独立判断每个text flow，neutral继承document fallback | override只绑定当前author revision或draftSerial的source range；revision变更丢弃/要求重选，不按内容猜跨revision延续 |
| inline/name/label | 普通用户内容作为独立bidi isolate，默认auto；D7 View数据label按下述已有textDirection，技术token按专用规则 | 不新增inline方向作者语法；不能让名称控制相邻菜单或label |
| 原生table、Node grid、board | 容器方向决定列的视觉排列；每个cell文字独立auto或session override | columnId和逻辑列序不变；UI偏好不存D7 ViewSpec未知成员 |
| Query/View | 容器布局方向可用当前个人偏好；数据label段基向逐字消费已验证ViewSpec.options.textDirection；CEL、ordering、时间zone/context、row handle及数据完全不变 | 任何可丢弃设置不得进QuerySpec、DynamicBlock、结果schema或author事实 |
| Source/代码/技术诊断 | 默认ltr skeleton、每logical line/技术片段隔离；原始文字允许其bidi runs，提供显式logical-codepoint inspection | 不用CSS override重排作者字符，不改源内Bidi_Control；inspection不能回存UI插入标记 |

auto使用UBA的首强字符规则（包括isolates的规定），不把European/Arabic-Indic数字当locale探测器。空文本/纯数字/标点没有强字符时采用表中fallback且显示可更改状态。显式ltr/rtl只设paragraph base level，不倒排字符串，也不覆盖源中合法的Unicode方向控制。

D7 View的数据label方向与容器布局分开计算。每个View内的普通text数据值、作者给出的列label/图例label及展示标题/说明作为各自独立text flow；已有`options.textDirection=ltr|rtl`明确决定这些flow的paragraph base，`auto`按各flow首强字符决定，neutral fallback为该View当前容器方向。D8普通Document的display override及block/session override不跨入View内部，不覆盖这个已有作者选项；个人方向设置只改变View容器及auto的neutral fallback。typed Ref/UUID/FieldId、source坐标和canonical数字等专用技术字段仍按§3独立ltr显示，它们不被当成普通plain-text label推断。未知textDirection仍按D7 closed decode拒绝。更改保存View的textDirection属于明确作者定义编辑，经D8/D6准备与提交；临时切shell/container方向不修改ViewSpec、Query或Action。

具体组合：shell=rtl、document override=rtl、View.textDirection=ltr时，View普通label的base仍ltr，容器可以RTL；View.textDirection=auto、label=`שלום`时首强字符决定rtl，label=`123`时使用容器fallback。以上只改变显示，不倒排scalar，也不改变columnId、row handle、Query顺序或提交request。

方向组合的可执行判定表（单个View普通label flow；技术typed值另走§3）：

| shell/container | Document/session override | saved textDirection | label | 结果base |
|---|---|---|---|---|
| rtl/rtl | ltr或rtl | ltr | שלום或123 | ltr |
| ltr/ltr | ltr或rtl | rtl | ABC或123 | rtl |
| 任意/ltr | 任意 | auto | שלום | rtl |
| 任意/rtl | 任意 | auto | ABC | ltr |
| 任意/ltr | 任意 | auto | 123、标点或空串 | ltr |
| 任意/rtl | 任意 | auto | 123、标点或空串 | rtl |

reload保留已保存View的textDirection；个人容器偏好若保留则同fallback，若被清除则按当前shell重新推导auto-neutral fallback。两种情况都不改saved View原值。临时Document/block/session override从不越过View边界。

caret边界验收另列：空flow只有一个stop，两向均clamp；单字素在左右文档端首步clamp后反向可移动，不要求回原点；bidi双affinity及soft-wrap两视觉stop不得按logical点去重。条件满足的成功相邻步才做往返恒等断言，layout epoch变化需重新hit-test。

app `lang`表示实际UI语言；content语言只由已知作者事实或用户明确的阅读语言选择标注，未知时不把阿拉伯字形猜成确定语言。direction和language分开。界面可手工切rtl而保持中文/英文UI；这不是阿拉伯本地化通过。

本代不增加D2 portable block/inline direction属性，也不解析普通header `dir`为隐藏控制语义。需要跨设备/导出保留的显式作者方向语法归后续有明确D2/D9修订的工作；现有Unicode作者字符保持原值。设备偏好可删除且不改变document meaning、身份或Core输出。

## 3. 混合文本与隔离

Arabic、Hebrew、CJK、Latin、European digits、Arabic-Indic digits、emoji、组合音标、标点、括号、URL、path、email、Ref、citation、formula/code必须同时进入语料。普通用户文字分别隔离，避免名称或clipboard内容影响相邻button/路径/错误代码。DOM/原生布局加入的隔离机制仅为presentation，不插回source或clipboard。

完整Node/Resource/Annotation引用、UUID、FieldId、operationId、source坐标、version、URL/path/email技术字段作为独立ltr区域显示；链接label可以auto，两者提供明确区分。URL仅显示不fetch；用户点击才产生经产品能力与安全规则处理的navigation intent。文件路径不是Ref。

代码、CEL、Query JSON、公式inert文本不镜像运算符/语法次序。Core已有token/语法映射可供技术显示分段；没有准确映射时显示ltr source及方向控制字符的可见检查视图，不能靠UI另造lexer改变解释。控制字符和confusable文字提示只说明显示风险，不删除或normalize作者源；用户可显式审查码点/原始文本。

语料基线示例（作为原文，不预先倒排）：`مرحبا 123 / ١٢٣ ABC 中文 שלום`、`A (שלום) 12:34`、`مرحبا https://example.test/a?x=1`、`x + שלום == 1`、`e\u0301`（实际组合音标另见fixture）、emoji家庭ZWJ序列。平台截图必须与同源logical文本及caret位置一起采集，单张RTL截图不能证明copy/delete正确。

## 4. 键盘、选区和视觉顺序

每个text flow及普通输入region在一个layout epoch中产生字素边界caret stops，包含logical source点及`upstream|downstream` affinity。视觉左右移动选择当前visual line上相邻caret stop；同一个logical点可能因bidi boundary有两个视觉位置，必须保留affinity，不能去重成一个随机位置。换行端按相邻visual line对应边沿继续，文档端clamp保持不动。仅在折叠selection、同一layout epoch、首步确实移动到相邻stop、且期间没有输入/目标/布局改变时，Left后Right（或相反）必须回到同一stop，包含原affinity；首步在端点clamp则不要求往返还原。非折叠selection按明确collapse规则先收至对应视觉端，不把collapse当相邻移动逆运算。

逻辑Previous/Next Grapheme命令按源顺序移动；Word Previous/Next使用固定Unicode18默认word boundaries并跳过非word间隔，不读设备locale。CJK无字典分词增强承诺。Source码点模式是明确选项而非默认。两个选择端点由logical位置表示，Shift只改focus；绘制可以是多个visual rectangles，操作仍只作用一个明确logical range。

Home/End为当前visual line最左/最右caret stop；Document Start/End为logical source首/末，使用平台常见修饰键并在命令目录中明确命名。Paragraph Start/End为logical段首/末，单独可调用，不用同一名称随rtl换含义。Up/Down保持visual x目标、到相邻visual line最近合法stop；并列距离以当前affinity再logical较小点裁决。鼠标点击/拖动使用该layout epoch hit-test；epoch已变则重新hit-test，不能沿旧像素落入新对象。

Backspace/Delete遵守主稿logical preceding/following字素，Direction不翻转它们。选择/删除不得切开emoji、combining cluster或CRLF；字符级Source检查有明确单独命令。复制按logical selection顺序。边界处工具栏、context menu、IME候选锚定focus caret的visual rect，但其动作仍绑定同一logical target。

表格方向键在navigation mode移到视觉相邻cell，以当前columnId映射logical列；进入edit mode后方向键交文本编辑器，Escape离开edit后回同cell。Tab/Shift+Tab按逻辑column order进入下一/前一个可操作cell；列镜像只改变视觉位置，顺序仍是阅读顺序的inline-start到inline-end。Enter打开当前cell编辑或明确的preview命令，不在IME中提交。tree向inline-end展开/进子项，inline-start折叠/到父项（RTL反向物理左右）；Up/Down按logical tree序。每项操作均有不依赖左右记忆的可访问命令。

## 5. 镜像边界

| 随shell/container镜像 | 必须保持语义方向 |
|---|---|
| navigation位于inline-start、inspector在inline-end；pane resize handle、menu submenu展开边、popover首选对齐、边框/间距、start/end圆角 | 作者source字符顺序、Node/Field身份、child ordinal、Query sort、表格源列顺序 |
| breadcrumbs分隔/下一层chevron、可展开tree chevron、纯UI返回/前进箭头 | media播放、品牌/Logo、下载/上传、数学运算符、代码、时钟图形 |
| table/board的视觉列布局及scroll affordance；键盘hit-test映射到同columnId | Calendar/Timeline的早→晚轴、数轴、chart数据轴/图例映射；本代早/小在左；D7没有reverse axis配置，RTL不新增该选项 |
| 纯shell导航和布局装饰 | D7关系edge箭头表达真实from→to；不能因rtl反转边或owner；地理/流程语义箭头按数据 |

菜单会因viewport避让而放到另一侧，但不改变target/命令含义。逻辑CSS属性只实现已决定的布局，不成为产品语义。scroll起点/scrollLeft的浏览器差异由adapter归一；焦点、可访问顺序和真实column顺序独立记录，不能依row-reverse后错误tab顺序。

## 6. 无障碍合同

全部创建、编辑、目标选择、预览、确认、冲突、恢复、reanchor、board拖动均有键盘和辅助技术可完成路径；拖动提供“选择目标列/位置”替代。给控件稳定的role/name/state、readonly/disabled原因，input错误与对应字段关联；焦点可见且不被固定工具栏完全遮挡。状态变更用适量live announcements，不能每个compositionupdate打断读屏。

屏幕阅读器按logical内容与层级朗读；bidi视觉重排不重排accessible字符串，不读DOM制造的方向控制/装饰glyph。selection方向/范围、source行列、cell header/行列位置、stale Annotation、未保存/结果未知应有可读状态。source与Write双pane不重复宣读同一隐藏副本；在模式切换后还焦点至对应目标，普通region使用exact raw scalar坐标；readonly元素/原子/escape/join使用Core origins选择其准确source范围，并公告不是逐字符可写映射。不得在合法投影已承诺来源的情形退回Source开头，或用字符串搜索代替来源。

200%文字缩放、400%页面缩放与320 CSS px重排、系统高对比、prefers-reduced-motion为必须后续测试的配置。正文单列重排不能隐藏内容/操作；二维数据表可在有标签region中保留二维滚动并提供逐行详情。触摸命中目标按平台可访问基线（Web至少满足WCAG2.2 AA目标大小规则），颜色不能是权限/错误/选中的唯一编码。RTL和LTR都执行同一组检查。

虚拟列表必须提供真实row/column counts或明确未知，不用已渲染20行冒充总数；焦点对象保持挂载或使用经实际AT验证的active-descendant方案。屏幕阅读器请求未渲染项时，先定位真实logical目标并加载，再聚焦/公告。异步排序/结果reset后旧focused row标记失效并回稳定容器，不能把其DOM位置交给另一行。composition宿主禁止卸载。

## 7. 跨表面矩阵

以下均是目标合同；当前无一行因本候选自动成为Supported。

| 能力/情景 | Desktop本地/远端 | Server WebUI | Mobile本地/远端 |
|---|---|---|---|
| canonical读取、投影、准备、提交 | 本机Core/Server | 仅Server | 包内Core/Server，合格backend前提 |
| Source/Write/Read、Field/table/collection | 共享状态，键鼠及可访问命令 | 同状态，browser事件adapter | 同状态，触摸sheet/键盘/读屏，不减语义 |
| native IME | OS/WebView真实IME | 浏览器真实IME | 软键盘、手写/语音和外接键盘 |
| LTR/RTL与mixed script | 逐OS/WebView/font证据 | 逐browser/font/AT证据 | iOS/Android布局、输入与AT证据 |
| offline local commit | 本地可；远端只Draft | 只Draft | 本地可；远端只Draft |
| clipboard受限 | 不可用时显式说明；cut不得先删 | 受浏览器user gesture/权限约束 | OS权限/后台限制；不后台读取剪贴板 |
| Undo/reanchor/冲突 | 同Core计划、完整preview | 同 | 同，不按触摸便利自动fuzzy重定位 |
| Agent/转换/模板能力 | 按D1/D9/D10明确可用性 | 仅Server能力 | D1当前排除转换执行/委托/批准和Agent入口；只读已提交结果 |
| 阿拉伯UI翻译 | 当前未交付 | 当前未交付 | 当前未交付 |

标准键位允许平台修饰键不同，但命令语义/目标/错误不变；任何缺少安全输入、全效果展示、AT导航或资源容量的具体host应准确声明对应能力不可用，不能把整个Mobile从正式方向中删除，也不能声称未测试组合已受支持。

## 8. RTL必审输入逐项处置

九问依次落点：能力分离§1/7；方向权威§2；混合内容§3；编辑interaction§4及主稿5–9；镜像§5；a11y§6；跨表面§7；localization§1/2/7；规模§6及Impact。验收矩阵提供Arabic-only/Hebrew-only及混合场景的正反case，覆盖save/reload/offline/conflict/recovery与Core请求同一性。不能以dir=auto、citation locale或字符串储存通过替代这些场景。

普通Write区域的源EOL与段间空行分别可见且可被辅助技术描述；源EOL为区域编辑坐标，不改变Read的D2 paragraph语义。空region/space/tab caret、Enter后逐EOL Backspace/Delete、重新打开与undo/redo均按Interfaces §3，不依赖旧DOM或旧revision。origin的relation不影响字素边界或赋予写权限。
