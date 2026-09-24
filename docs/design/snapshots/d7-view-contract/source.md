---
_weftext:
  id: "b0056f5d-51c1-4d32-a8be-bfe47e339229"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 View Contract — revision06-draft

## 1. 输入、身份与纯展示

ViewSpec exact `{format:"weftext.view",version:1,inputSchema,layout,bindings,options}`。inputSchema 是完整 Query TerminalSchema（Execution定义），columnId 是终端column name的稳定、大小写敏感标识，**不是D4 FieldId**。View按columnId及精确TypeSpec绑定；label可变不改columnId。删除/改名/改型导致schema_mismatch，不能按位置、相似label或首行推测重绑。所有bindings值都是columnId，特定literal variant仅在明确允许的阈值/固定格式中使用。未知layout/成员/Binding拒绝。

View没有 CEL、SQL、filter、sort、take、group aggregate、bin、quantile、fetch、脚本、任意renderer config、Action或source authority。分组线条、bar几何位置、面板分区和图形连线只消费显式数据键，不合并事实。renderer不能补title、name、avatar、resourcebytes、tooltip详情或hidden endpoint。打开实体与下载资源是独立当前授权操作，不能由render触发补读。Value§5.1允许显示的作者Ref/历史Locator只显示已投影原成员，不据此加上当前有效/可见标志或自动查询目标；点击另走完整当前门。

输入必须是同一已完成、当前可交付的ResultHandle；取完分页并验证完整typed envelope后才能绘制语义图表或发布导出。可预先显示与结果无关的框架/loading；不得显示前200行临时折线、抽样、估值或“其它”桶。查询成功但View验证失败只拒绝该View，不把成功Query改成半坏rows；同一query可被另一个合法View消费。

## 2. TerminalSchema 与通用选项

rows schema含ordered布尔和ordered columns `{columnId,type}`；scalar schema含type；graph schema含nodes/edges各自rows schema及键合同。labels只由View `options.labels` object给出：keys须为已存在columnId、values为plain text≤4096 Unicode scalars，未给时显示columnId原文。common options exact `{title,description,labels,textDirection,numberFormat,partition?}`，title/description plain text（可空），textDirection=ltr/rtl/auto只管数据label，不推断App语言；numberFormat=`canonical`，首版不支持本地浮点格式器/自定义模板/单位转换。整个View必须满足此common object，并按layout额外增加表列的选项。

partition只适用bar/line/scatter/pie/heatmap：exact `{columnId,scale:"shared"|"independent"}`，列必须nonoptional equatable。完整rows按该列相等分组，顺序为已排序输入的首次出现；每行恰属一panel，null/missing不可能；不创造空panel，授权过滤后不存在的组无痕。每个panel仍执行同一layout唯一键和预算；shared的轴domain在全部panels完整几何extent上统一取极值，independent逐panel，但显式标示刻度独立。partition称“分面板”/Panel Partition，不叫Facet以免与D4分类混淆；它不是dashboard，也没有跨Query筛选。

所有图表的row顺序必须由Query sort明确建立。category、series、legend、panel出现顺序取有序输入第一次出现，不能按本地locale、hashmap或renderer默认重排。line还验证每series的x严格递增；不能按x偷偷sort。table/list对unordered允许显示，但使用Core内部稳定运输序并显示“未指定语义顺序”；不能据此执行take/拖动重排。

## 3. Layout 闭集

表中列名为bindings成员，`?`可缺省；未列即禁止。共用options之外附加选项均显式列出，没有自由object。

| layout | result / bindings | 附加options与语义 |
|---|---|---|
| table | rows；`columns:[columnId...]`，非空唯一 | 无；显式顺序显示完整cells；可虚拟化，不把只加载部分行当complete |
| list | rows；`primary`, `secondary?`, `target?` | 无；primary/secondary text或Optional<text>；target EntityRef或Optional，不隐式读target |
| task_list | rows；`title`,`status`,`target` | 无；title text，status严格tasks/status code type，target NodeRef；是只读Task投影；勾选由独立D8 Action intent，Query仍须验证tasks/task |
| board | ordered rows；`key`,`lane`,`title`,`target?` | 无；key nonoptional equatable全局唯一；lane text/code，title text；lane按input首次出现；拖动不修改Query或直接写Field |
| calendar | ordered rows；`key`,`start`,`end`,`title`,`target?` | `timeZone`,`tzdbVersion`,`calendarVersion`必填text；date或instant端点同型；正时长/end-exclusive，不把period当event，不自动reminder |
| timeline | ordered rows；同calendar | 同calendar；仅布局不同，无自动推算duration/依赖调度 |
| gallery | rows；`title`,`resource`,`target?` | 无；resource为ResourceRef/Optional；只显示明确已投影元数据，封面字节由独立授权Resource consumer取得，失败不读其它资源/换owner |
| metric | scalar quantitative或Optional<quantitative>；bindings=`{}` | `unitLabel` plain text必填可空；不是Gauge；不从多行自动选第一行/求和 |
| bar | ordered rows；`category`,`value`,`series?` | `orientation:horizontal|vertical`,`mode:grouped|stacked`,`valueDomain:Domain`,`unitLabel`；value quantitative且nonoptional |
| line | ordered rows；`x`,`value`,`series?` | `xScale:linear|date|instant`,`xDomain:Domain`,`valueDomain:Domain`,`unitLabel`,`timeZone`,`tzdbVersion`,`calendarVersion`；missing value必须Optional且生成gap，不连跨gap |
| scatter | ordered rows；`x`,`y`,`key`,`series?`,`size?` | `xDomain:Domain`,`yDomain:Domain`,`unitLabelX`,`unitLabelY`；key全局唯一，x/y nonoptional quantitative；size nonnegative numeric，0为无面积但保留a11y数据点 |
| pie | ordered rows；`category`,`value` | `unitLabel`；value nonnegative quantitative且nonoptional；category唯一；0值列在table，0-total显示empty-total不除0 |
| heatmap | ordered rows；`x`,`y`,`value` | `valueDomain:Domain`,`unitLabel`；x/y text或semantic_code，组合唯一；value nonoptional quantitative；缺(x,y)是missing格，不补0 |
| network | graph；bindings=`{nodeLabel,edgeLabel,layerConstraint?,nodeDetails?}` | `placement:circle_v1|layered_v1`,`flow:up|down`；nodeLabel/edgeLabel各自table的text列；layered必有layerConstraint text列、circle禁止；node/edge keys与端点见Query graph result；不接受用户代码/physics |

numeric=int64/integer/decimal；quantitative=numeric或quantity。bar category和series为nonoptional text/code；line series为nonoptional text/code，x按xScale对应nonoptional quantitative/date/instant；scatter series同；pie category同。可选target绑定仅指向已存在cells的EntityRef，打开时fresh authorization，不从cell持有权利。calendar key全局唯一；周期先经Algebra§11在Query中生成显式日期，unavailable分支须完整状态表或明确筛选，View不能补算或丢行。相同终端的month/week viewport不重写period身份；本版不在ViewSpec添加未定义mode成员。milestone(end==start)本版calendar/timeline不接受，Gantt另行延期，不能以零duration假装事件。

Domain exact `{kind:"auto",zero:bool}` 或 `{kind:"fixed",minimum:TypedLiteral,maximum:TypedLiteral}`。fixed同axis类型且min<max，任意数据超界整个View `domain_excludes_data`，不clip；auto包含所有数据，zero只适用quantitative且将同basis的0并入domain。空输入auto域不产生数字刻度，仅empty状态；单值auto保留该value并显示单点/单值标注，无任意±1伪数据。line date/instant不得zero=true；quantity的zero是同已验证basis的零量；数值线/散点auto zero由作者明确给值。无reverse/log/symlog或renderer默认nice domain。轴tick文字取实际数据极值与显式0（若存在），数值canonical，无隐式时区；D8可增加不改变domain的视觉tick优化，须单独交互合同，不回写Query。

bar唯一键(category,series)，无series时固定single；不自动aggregate重复key。stacked要求非负value magnitude，每category各series缺项显示缺项位置，不补0、不连面积；完整stack高度是这些明确值的几何累加，不成为导出新事实，accessible table仍原值。line唯一(series,x)，x严格递增（可不同series穿插）且series内至少0/1/2点均合法；Optional.none使该点位置断线并有缺失标记，不跳过后连线。pie比例是value/完整sum的几何比例，不能替代Query提供业务百分比；精确整数/decimal先精确比值，renderer最后转换屏幕坐标，原值不经float回写。scatter大小不能表示负面积。heatmap sparse≠zero。

all-missing line保留x与缺失状态，无线；0rows是empty；scalar none是no-value；scalar0是0，三者不能相同。empty与权限过滤后的empty使用同一空态，不解释“被隐藏了多少行”。producing、error、cancelled、expired/reset与empty区分；旧结果可在已交付本地画面标stale，但不能新增交付、导出或动作，用户刷新生成新result。

## 4. 宽表、顺序、颜色、交互与无障碍

v1图表要求long-form。`month,revenue,cost`宽表可用原Query `derive points=[{series:'revenue',value:row.revenue},{series:'cost',value:row.cost}]→unnest→project→sort`，list内两个object同型；无需新unpivot语法，也不让renderer melt。如果不同measure型/单位，Query先显式转换到可证明一致的类型，否则分成独立Query/View；不会仅因单位标签相同就堆叠。

颜色：每个可见series/category key的Equality canonical bytes加UTF8 `D7-Palette/1\0`做SHA-256，前8bytes作为unsigned big-endian模12决定固定palette slot。12个slot为具名样式（不是任意CSS）；同key跨刷新/排序/设备稳定，冲突允许但必须结合明确label及不同可访问标记。无series为single固定slot0；不可用颜色无法通过改变query补救；D8确定满足对比度的实际色值/纹理。图例按输入顺序；用户临时隐藏series是明确的非权威本地展示状态，需可恢复且标示部分显示，任何导出必须明确“当前展示”或完整数据，不能声称完整原图且漏series。

tooltip/details只使用当前row已投影列；本版不接受自由tooltip表达式、阈值线、reference band、目标值overlay或点击触发新Query。可经独立交互发起新的显式Query/Action，但不是ViewSpec副作用。键盘可到达每个有意义的数据项及其原始精确cells；每个chart有同数据/顺序的accessible table和plain text title/description，不通过颜色、图形位置或动画单独传义。读屏顺序按Query数据序与输入panel序；CJK、Arabic/Hebrew与混合RTL内容原样保存，标识符和数轴语义不因AppRTL反转。

全部布局支持减弱动态效果、高对比、缩放、可读打印/导出替代。Mobile空间不足使用同完整数据的table/list fallback并明确layout unavailable；不能通过抽样或截断获得“支持”。CLI/Server输出typed数据及View验证状态，不谎称已渲染图形；Desktop/WebUI/Mobile renderer缺失按D1 capability错误。D8负责像素/键盘焦点/屏幕阅读器与RTL实际验收，本合同不宣称现成实现。

## 5. Graph 数据与确定性

network只消费完整Query graph result。节点包含孤立节点；edge必须有可见合法source/target，隐藏endpoint的edge在Query授权入口整体去除，不在View中制造匿名隐藏节点。允许parallel edges、self edge和cycles（若原D4relation允许）；各有唯一edgeKey和label，类型和asserted/derived不被View隐藏。首版circle_v1按Query已排序node序编号i=0..n-1，位置角度2πi/n（n=1居中），图形连线按edge输入序，self edge明确loop；parallel曲线按同端点pair内edge序分槽，D8实现需保留对应键和顺序。nodeDetails若存在，值为非空、唯一、有序的graph.nodes columnId数组，必须全部存在于nodes schema。它只渲染各节点当前row内已投影的完整typed cells：按声明列序、object成员schema序、list原序递归显示；保留重复、空text、空list、Optional.none和原精确值，不filter/flatten成另一关系、不产生新事实、排序或目标Ref。list里的每个已存在item可显示为附属于owner节点的只读终端明细卡片，嵌套层次保留；卡片没有graph vertex/edgeKey、导航或关系展开能力。Family文本亲属以其text和relation原值显示为这种不可展开末端卡片；没有Ref就没有点击解析目标。完整a11y table/export仍包含原nodes cells。此绑定不改变rank算法、Query graph端点或单一terminal合同。算法和seed固定（无random），无源坐标持久化；不同像素尺寸可缩放，语义/a11y/export数据相同。不以布局距离推断事实。

layered_v1的每条edge通过显式layerConstraint列给出`none|forward|reverse|same`，不读取FieldId猜布局。none没有层约束但仍显示；forward要求rank(target)=rank(source)+1；reverse要求rank(source)=rank(target)+1；same要求两端同rank。事实source/target和asserted/derived方向始终保持，布局约束不生成inverse作者事实。

算法固定：先以same edges求等价类；在等价类上将forward视为source→target、reverse视为target→source，只用于检查严格层级图。有自环或有向cycle即cycle_not_supported。再在完整非none约束的无向连通分量内，以Query nodes序最早节点赋potential=0，沿edge输入序传播上述exact差值方程；已有potential不等时layout_constraint_conflict，不能删边或移动一个配偶凑图。每个连通分量统一减最小potential得到非负rank；孤立或仅none连接节点rank0。这样不受DFS/BFS或物理分区变化影响。各rank内按Query nodes序排；flow=down时rank向下、up时向上。最小平移只作用几何，不成为Query作者值；同层相对左右位置不代表血缘。

Family Tree模板以D4 graphProjection=family的明确亲子事实(child→parent)输出reverse，以明确spouse/sibling事实输出same；多父母由同一child的差值方程共同约束到同层，旁系通过实际边连接，孤立节点保留。其它亲属literal没有NodeRef、不能生成entity endpoint；模板在同一graph.nodes的literal_relatives列明确投影完整关系标签与作者text，以network的nodeDetails显式绑定该列，按下面的终端明细卡片规则显示；不声称同一Query另有第二rows terminal，也不把文本亲属包装为隐藏实体占位。friends/professional不进入family专用模板；通用network可以包含它们并显式none。原D4拒绝的self事实不能由View放行；允许self的其它关系在circle呈loop，layered仅none/same自环可满足，strict自环拒绝。parallel facts及其各自label保留，重复相同约束不增加事实或改变层。

这套通用几何约束还适用组织层级或其它有向图，新增业务不新增layout枚举。矛盾家族图可用circle显示全部合法事实，但必须由用户显式选择另一View；layered失败不得自动隐藏矛盾或换图并称原View成功。tree/treemap和Gantt仍按下节延期，不把多父母Family Tree强制成forest。

## 6. 未选 layout 的明确处置

| 家族 | v1处置与条件 |
|---|---|
| table/list/task-list/board/calendar/timeline/gallery | 上表支持；form属于D8生成Action的输入交互，不是Query View layout |
| KPI/metric/bar/column/line/scatter/pie/donut/heatmap | 支持基础六类；column只是bar vertical；donut/area/bubble没有隐藏新统计，当前wire仍只接受原layout，alias字符串拒绝；bubble可显式scatter.size |
| histogram | 必须Query先算明确bin edges/count；本版无通用bin函数，已存bin数据可bar；自动分箱延期，不在View做 |
| boxplot/quantiles | 延期新的Query统计feature和分位算法；不能renderer算percentile或猜quartiles |
| error bars/confidence/range bands/targets/baselines | 延期closed绑定和区间有效性合同；已有interval数据可table/timeline（符合后者正区间）；不按label猜统计含义 |
| tree/treemap/sunburst | 延期；需明确parent missing/cycle/multi-parent、sibling order、leaf/self/sum语义与zero-total规则后新增layout；不得把D4亲属多parent图强转forest |
| Gantt | 延期；需冻结milestone与end-exclusive、FS/SS/FF/SF、lag单位、critical/working-calendar是否支持；普通timeline不能冒充调度计算 |
| sankey/chord | 延期；需流量守恒/负值/零总量/循环/边聚合合同；network只表达已有关系，不把edge count当flow |
| slope/bump/rank/window/pivot | rank/window为未来Query feature；long-form已存在排名可line；renderer不推导排名；pivot可多个显式aggregate列，无自动动态columns |
| stacked area/streamgraph/combo/radar/funnel/waterfall/bullet | 延期各自closed数据/轴/stack/基线规则；不透传底层图库configuration |
| density/violin/beeswarm/KDE | 延期统计或collision-layout合同；无默认带宽/随机抖动/采样 |
| parallel coordinates/geo/candlestick | 延期维度、地图/金融time domain provider合同；不默认网络获取地图或行情 |
| gauge/3D/wordcloud/custom script | gauge先用metric；其余本版拒绝，表达成本或安全/可比性不合需求 |

这些延期不改变原Field/Relation数据定义，不要求新业务扩展改变Query语法；它们是缺少已冻结通用运算/展示语义。不得宣传为已经覆盖全部DataviewJS任意程序。

## 7. View 完整校验、错误和资源

View验证顺序：closed decode→当前result授权/epoch→complete result→schema exact→layout/binding types→全数据结构和order→domain/数值→budget→当前交付门。错误固定 `invalid_view|unsupported_layout|schema_mismatch|invalid_binding|incomplete_result|duplicate_key|invalid_order|negative_value|invalid_interval|missing_endpoint|cycle_not_supported|layout_constraint_conflict|domain_excludes_data|budget_exceeded|renderer_unavailable`。circle_v1可显示上游合法cycle；layered_v1严格层级子图有cycle时实际发cycle_not_supported，差值方程矛盾发layout_constraint_conflict。两者都不自动补正数据。

当前权限错误使用Execution/D6的not_visible/reset顺序，不附隐藏rowId/endpoint/key或数量。View错误可指静态binding name，不能附数据值。points/series/categories/panels/nodes/edges/labels/outputbytes分别有有限request/policy/host最小预算，checked加法在分配前计费；任何一个超限全View失败。全部数据用于accessible table也计output，不能只给前N行来绕过。渲染或外部导出发布前再次检查result当前授权；D9worker只接有限完整输入，不授Core写权限。


## 8. 完整数量basis及几何轴域

View先验证整个轴分组全部present值的TypeSpec与动态basis。quantity要求dimensionId/unitId完全相同；date要求calendarId/calendarVersion/precision完全相同且当前ResultHandle保有对应verified comparator。fixed endpoint同样参与这项验证。shared panel合并验证，independent可逐panel具有不同basis，但每panel必须明确显示其完整basis，不以同一个unitLabel掩盖差异。line的series不能自成另一个未声明轴；同axis中任何不兼容值拒绝invalid_binding。numeric的unitLabel仅plain-text说明，quantity自动附完整unitId，date附明确calendar/version/precision；unitLabel永不证明单位已验证或触发换算。

axis extent由以下精确数学先计算，再一次应用Domain；quantity计算只对同basis magnitude进行，保留basis。不要先验证单条raw值就宣布整个layout在域内。

| layout/axis | 纳入domain的完整extent |
|---|---|
| bar grouped | 每个实际bar的0基线和value端点，负值沿负方向；zero=false也必须包含实际基线0 |
| bar stacked | 每(category, panel)按series显示序的全部精确非负前缀和及0；最大值是该category完整sum |
| line | x为全部x；value为全部present y，none形成gap不产生y；仅当domain.auto.zero=true时加入0 |
| scatter | 全部实际x/y；size只控制面积，不暗扩轴域，不把size当坐标 |
| heatmap | 所有实际value；missing cell无数值 |
| pie / metric | 无轴；pie精确sum只作几何分母，0-total独立空态，quantity仍全basis一致 |

每个panel完成geometry extent后，shared取全部panel extent并集，independent逐个处理。fixed必须覆盖全部extent，否则domain_excludes_data；不clip、不暗扩fixed。auto取exact min/max并加作者允许的zero；全空无刻度，单点仍保持单点，不能任意添加数据。stack `6+6`的extent为[0,12]，fixed [0,10]拒绝，auto上界12；这12是布局校验值，accessible table和导出typed数据仍恰是原两条6。grouped [-3,2]实际extent[-3,2]，fixed [1,3]因负值与0基线均拒绝。任何中间精确计算超有限资源预算整体budget_exceeded，不能用浮点溢出或截断得到可绘图的假域。
