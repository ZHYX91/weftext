---
_weftext:
  id: "2d01ea44-1790-431d-986c-0214dbad544f"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 Scenario Dispositions — revision06-draft

规范fixture定义不等于已运行产品。局部模型实际覆盖以model-report逐项为准；其它行是独立评审与后续产品conformance义务。Mandatory完整原文随包提供，以下按原编号逐项处置，不以目录替代阅读。原文的旧Profile/Task/Record或reviewer名称须按当前D2/D4/D5及本次评审合同解释；未改写原历史文本。

## 1. D7正例与反例

| ID | 场景 | 规范位置 | 正例 / 应成立结果 | 反例 / 应拒绝结果 |
|---|---|---|---|---|
| EXT | 新增业务领域 | 主稿§2/Algebra§1–2 | equipment/device及course/course新增合法Facet/Field，原scan/read可类型检查 | scan_equipment或运行时Field名拒绝；未安装定义不是空字段 |
| BAG | selector与bag | Algebra§1/3 | exact refs重复去重；unnest [x,x]得到两行、count_rows=2 | 把两行按Ref提前合并会错误得到1 |
| DAG | 共享scalar与规范图 | 主稿§4–5/Algebra§5 | 两个derive共用global aggregate；ID改名及合法拓扑置换规范描述相同 | joint cycle、unreachable、scalar偷读row拒绝 |
| SCHEMA | flat schema | Algebra§3 | aggregate只含keys/measures；project同批读取input | group后引用旧row、derive覆盖原column拒绝 |
| OPTIONAL | 缺失/None/错误 | Value§1/4 | optional param默认none与显式some空字符串区分 | null、unbound required、value(none)不可吞成false |
| RELATED | 存在/不存在 | Algebra§4 | none仅在可见关系中无total predicate命中时保留 | left traverse后过滤不能替代none；may_error predicate静态拒绝 |
| REFS | 精确实体与生命周期 | 主稿§3/Algebra§1 | 同完整NodeRef source保持，隐藏和missing exact seed同零成员 | 裸UUID/TaskRef/Field key不能进入EntityRef decoder |
| LINEAGE | project与动作 | Algebra§3/Execution§6 | 一对一project仍可重签原唯一occurrence窄evidence | unnest/aggregate/queryRef后不能implicit evidence；不得挑第一目标 |
| QUERYREF | 调用边界 | Algebra§5 | 同值callee bag两个occurrences加caller namespace后sort/take稳定 | 继承callee权限/order/evidence、地址别名绕cycle拒绝 |
| BARRIER | edge/target权限 | Algebra§2/4/Execution§3 | 读edge和target均合格才输出关系及qualifier | 无权target时不能只清ref而保留edge数量/name |
| COMPLETE | 完整结果与分页 | Execution§1/3 | 200+1行全成功后才分页，空nonterminal后继续 | 第201行错误、取消/资源失败禁止先显示chart或成功EOF |
| AUTH | 撤权与令牌 | Execution§3/6/8 | 旧result在auth generation变化立即reset | ResultRowHandle/ByteHandle/preview/effects/cursor串用或旧epoch复活拒绝 |
| NUMERIC | 精确聚合 | Value§2–3 | [INT64_MAX,1,-1]各partition最终sum=MAX，avg只舍入一次 | 中间int64溢出、float舍入、先平均分区平均违反合同 |
| VALUE | D4全值桥 | Value§1–2 | arbitraryinteger/decimal、CamelCase对象、date scopes/instant任意fraction完整保存 | 34/18截断、九位时间限制、calendar/unit猜测拒绝 |
| CACHE | 负依赖与增量 | Execution§3 | 空related范围有incidence/query-scan版本；完整batch后原子delta | 缺index当空、gap补差、逐条混新旧rows拒绝 |
| FEATURE | 语义版本和能力 | 主稿§2 | 新Field无需feature；新通用op有稳定ID，Core推导依赖 | 作者requires、unknown feature fallback拒绝 |
| DYNAMIC | 可移植动态块 | Execution§9 | 恰一Query+一View，参数及now/this绑定显式 | results/cursor/UIstate/resolved this/actionQuery或address指错payload拒绝 |
| DOMAIN | 原生表格/Node | 主稿§3/Execution§5–7 | native row显式promotion新Node+原行link同D3decision | table外观变Record identity或checkbox镜像拒绝 |
| POST | 完整post-query | Execution§7 | new Node排top1首位才requireMembership成功 | 仅通过where但排第二时整个create拒绝 |
| BULK | 全部目标冻结 | Execution§5–7 | all_result 350个目标即使preview只200也全冻结 | 1001目标/commit重新Query扩大/同Ref重复写拒绝 |
| FIELD | 精确Field修改 | Execution§5–6 | 两个相同phone不同key/note只改被选项 | 旧revision/ABA/constraints=[]当完整窄资格/增长越D2容量拒绝 |
| PROMOTE | 创建组合与建议 | Execution§5–6 | checklist/native row fresh Node+oldsource同D3合法mode；suggestion精确range | 先旧source提交后创建、S reply证据替代target或stale range拒绝 |
| SEARCH | 搜索贡献 | Algebra§6 | name/alias/content描述生成通用Query，rank和dedupe固定 | hidden name进入score、provider缺失跳项、自动抄names到aliases拒绝 |
| COLLEAGUE | 按需任职交集 | Algebra§6 | 选person双traverse完整validity交集，结果derived且cache可重建 | 10k任职写50m关系、unknown validity当current或literal target当Node拒绝 |
| CALENDAR | 时间与recurrence | Algebra§6/§9/§11、Value§5 | 五种period经完整类型链到Calendar/Timeline；ISO跨年、闰日、显式unavailable；range/event/recurrence仍各保原义 | 仅标签无point投影、无声丢超域period、无限物化Node、exhaust当truncate、设备tzdb默认或假holiday拒绝 |
| LIBRARY | 作品生命周期 | Algebra§6 | draft→published同Node，MyWorks按显式creator/currentPerson查询 | 按title/PDF/DOI合并Node或把Work当Task拒绝 |
| VIEW | View闭合绑定 | View§1–3 | columnId+完整type对应；无source读取即可渲染 | label/position重绑、脚本/hidden fetch/renderer聚合拒绝 |
| WIDE | 宽表转long-form | View§4 | Query明确2-item list→unnest→project→sort | renderer自行melt revenue/cost或按label混单位拒绝 |
| ORDER | 折线和唯一键 | View§2–3 | 每series x严格递增且唯一，legend按输入首次出现 | 乱序x/重复bar key/heatmap(x,y)不能自动sort/aggregate |
| EMPTY | 空/零/缺失 | View§3 | scalar0/none/0rows不同；line none断线；pie0total有空总量态 | sparse heatmap补0、allmissing当成功曲线、hidden-count解释拒绝 |
| PANEL | 分面板与颜色 | View§2/4 | 一个完整Query各row恰一panel，稳定key颜色与input序 | 把panel叫Facet/生成hidden空panel/按设备sort/recolour拒绝 |
| GRAPH | 关系图完整结构 | Execution§2/View§5 | isolated/parallel/self/cycle显式；layered rank子图DAG可多parent | missing/hidden endpoint补匿名node、rankcycle/抹平edgekind拒绝 |
| ACCESSIBLE | 六caller和可访问 | View§4/Impact§3 | 精确cells+accessibletable、CJK/RTL不改语义、Mobile诚实fallback | 只图无table、颜色单独传义、CLI数字舍入或未有renderer声称支持拒绝 |
| EFFECTS | 预览/效果与重放 | Execution§6/8 | 完整source/receipt资格重验后分页，lostreceipt同request重放 | 换token域、rawredaction冒充exact、未授权commitSequence/effects拒绝 |
| BUDGET | 独立预算 | Execution§1/View§7 | rows/AST/work/deps/points/panels等逐维checked最小预算 | 0当无限、N+1 silentlysample或重启刷新额度拒绝 |
| TERMS | 术语与受控迁移 | Lexicon/Impact§2 | columnId≠FieldId、Facet≠CEL Profile、三类source/provenance分层 | schema或locale受控slot名称漂移失败，历史自然语言同词不误报 |

## 2. 原Query非回退17主题

每行正反例在§1，实施caller为Core/CLI/Desktop/Server/WebUI/Mobile；尚未逐caller运行的不得计已通过。

| 主题 | fixture |
|---|---|
| EntitySet与bag | BAG |
| 唯一规范DAG | DAG |
| Row schema | SCHEMA |
| CEL/optional/error | OPTIONAL |
| related any/none | RELATED |
| exact entities | REFS |
| 六类身份 | LINEAGE |
| QueryRef | QUERYREF |
| ActionEvidence重签 | LINEAGE |
| 数据安全屏障 | BARRIER |
| 原子结果与撤权 | COMPLETE/AUTH |
| 数值与聚合 | NUMERIC |
| TypedLiteral/Orderable | VALUE |
| read-set/cache/incremental | CACHE |
| 版本与feature | FEATURE |
| DynamicBlock | DYNAMIC |
| 原生表格/Record/Node | DOMAIN |

## 3. Mandatory §9 全部57项

| 原编号 | D7裁决与交叉owner |
|---|---|
| A2-01 | 独立Fields/Ref保持；read entries保留note，provider unavailable失败，不自动merge（VALUE/REFS/SEARCH）。 |
| A2-02 | hidden parent整个edge不可见、spouse独立可见；literal无endpoint（BARRIER/GRAPH）。 |
| A2-03 | structural、member、dependency、document-link各自relation ID；graph不合并（GRAPH）。 |
| A2-04 | period七成员与scope从作者/控制读取；rename/move不改语义；周/DST规则用D4版本（CALENDAR）。 |
| A2-05 | D6唯一/多篇configuration与负范围CAS；D7collection postquery不替代该门（POST）。 |
| A2-06 | date/instant end-exclusive；不同scope不比较；导入/导出格式由D9，Query值无隐式conversion（VALUE/CALENDAR）。 |
| A2-07 | D9输入适配+原D3create/copy owner合同；D7不授Resource跨owner或递归导入（PROMOTE）。 |
| A2-08 | D9/D10映射sync与D6事务；D7只消费已提交fields，不存credentials/cursor作者源（FIELD）。 |
| A2-09 | 所有caller同schema/errors；unknownprovider不当空（FEATURE/ACCESSIBLE）。 |
| A2-10 | 沿D4 Field-owned关系；无独立无限RelationType ontology，新业务不新增Query语法（EXT）。 |
| A2-11 | 旧specialization候选由D2/D4替换：ordinary+Facets；opaque源按上游保留，typedread未获定义不可用（VALUE）。 |
| A2-12 | D5非持久Entry selector+whole source版本；D6冲突/merge明确，不让UI list决定Record域（FIELD）。 |
| A2-13 | inverse按actualcanonicalowner，currentauth+D6完整incidence；失index完整scan或unavailable（BARRIER/CACHE）。 |
| A2-14 | 同5，title/path不参与CalendarScope唯一性；D7preview不借Query空结果通过（POST）。 |
| A2-15 | People/Calendar/Graph映射D1已有能力，无新正式surface；runtime安装D10、交互D8（EXT/ACCESSIBLE）。 |
| A2-16 | Organizations catalog用同read；未知scheme保留原始source、触及查询明确不可用（VALUE）。 |
| A2-17 | structural_parent与organizations/parent各自read/traverse，修改后者不移动Node（GRAPH/FIELD）。 |
| A2-18 | 多种组织关系各自FieldId及方向；graph labels保持种类（GRAPH）。 |
| A2-19 | 改名/合并/历史identifier不产生自动identitymerge；显式D3操作另验（REFS）。 |
| A2-20 | position/rank/department是现有typed members；新增职务值不新增Query字段/算子（EXT/COLLEAGUE）。 |
| A2-21 | schema/connector/state分域，Query不repair作者事实；D10缺失按能力定义停止（FEATURE）。 |
| A2-22 | D1既有官方可选模块实例，无新增surface；物理发布证明尚未完成（ACCESSIBLE）。 |
| A2-23 | 完整§2.6逐项映射下表；D4无序Facet集合，不引单Profile查询域（EXT）。 |
| A2-24 | D9模板完整输出+D3compound+D4组合门，不以安装或模板优先覆盖schema（PROMOTE/POST）。 |
| A2-25 | D2/D4保留opaque source，D7typedread不推断unknown Facet；无二次写路径（FEATURE/FIELD）。 |
| A2-26 | 旧Task specialization已由ordinary+tasks/task替换，NodeRef唯一；无tasks独立domain（DOMAIN）。 |
| A2-27 | range≠event；Calendar View对明确时间列展示，提醒/participant不由View制造（CALENDAR/VIEW）。 |
| A2-28 | 不同规则pack版本/来源不合并isHoliday，D10未闭合算法不可用；不复制作者事实（CALENDAR）。 |
| A2-29 | end-exclusive/offset/任意precision；unbounded需要明确bound，View正区间要求不满足时拒绝（VALUE/EMPTY）。 |
| A2-30 | D4recurrence原originalStart内部key，完整horizon/override绑定；旧evidence不复活（CALENDAR/AUTH）。 |
| A2-31 | 无限RRULE只有限horizon完整展开，RDATE/EXDATE/override原合同；无occurrence Nodes（CALENDAR）。 |
| A2-32 | source binding UID/RECURRENCE-ID由D3/D9/D10，Query不以title/path/UID作identity（REFS）。 |
| A2-33 | subscribe/copy/adopt为不同authority Action，D7result订阅不是ICS订阅（TERMS/PROMOTE）。 |
| A2-34 | D9各component mapping/loss+boundedrecurrence，不一component一Node；D7只接已接纳typed源（CALENDAR）。 |
| A2-35 | UI模块关闭不改变D2Task/Template解释；查询仍Node/Facet；无旧Task枚举fallback（DOMAIN）。 |
| A2-36 | D10逐contribution权限；D7只读schema/Query/View纯定义，拒绝wholepackage授权（EXT）。 |
| A2-37 | Settings/Marketplace等D8/D10；D7Lexicon映射且无Diary模块；不声称界面已验收（TERMS）。 |
| A2-38 | 不因领域名建可执行Query插件，Business等由既有schema/pack表达；无medical产品承诺（EXT）。 |
| A2-39 | D9生成/默认/持续schema分域，D7prepare完整结果冲突拒绝；模板不保留后续权威（PROMOTE）。 |
| A2-40 | 单Calendar三个Facet语义；旧Time/Chrono只是historical rejected/retirement，无wirealias（CALENDAR/TERMS）。 |
| A2-41 | Librarycreator筛选MyWorks，status更新sameNode；edition独立身份由显式D3create+relation（LIBRARY）。 |
| A2-42 | Library字段/作品Node/Resource/citation原owner，D7不按媒介增domain（LIBRARY）。 |
| A2-43 | Calendar/Library canonical names不随语言变化，D8/D10界面/manifest待实施（TERMS）。 |
| A2-44 | 沿D4 Library域命名：Work对象可自著外部、Bibliography派生书目，不新建References实体（LIBRARY/TERMS）。 |
| A2-45 | period/range/event各自Field/唯一范围/recurrence，不因同module混type（CALENDAR）。 |
| A2-46 | Mobile本地Core查询、通知/ICS/participant依赖独立；source不因拒绝网络删除（ACCESSIBLE）。 |
| A2-47 | 采用单Calendar产品分组但不合并ontology/capability；D7不重开D1模块导航（CALENDAR）。 |
| A2-48 | period套不同Template/Preset仍同Node/scope；无diary identity/membership（CALENDAR/DOMAIN）。 |
| A2-49 | D10verifiedcontributiongeneration进入Query闭包；未接纳provider不得注入schema，配置UI归D10（FEATURE）。 |
| A2-50 | 禁用规则使相关派生Query失效/reset，原source保持；未引用pack的查询无该依赖（CACHE/CALENDAR）。 |
| A2-51 | D2 AsciiDoc Profile/CEL Profile/Facet分别限定，无裸Profile三义（TERMS）。 |
| A2-52 | D3SourceBinding/Provenance/OriginBinding与D7ResultSubscription各自词义、owner及动作（TERMS/REFS）。 |
| A2-53 | D1–D6原Lexicon完整为输入，D7定义34个新概念和继承绑定；含条件源修改预览与效果交付世代，不重新定义Node（TERMS）。 |
| A2-54 | 受控locale/wire/type符号精确漂移失败；用户source/历史语词不误报（TERMS）。 |
| A2-55 | LibraryWork/task/project work，Reference/NodeRef/link分别限定，Query名字用稳定schema IDs（TERMS/LIBRARY）。 |
| A2-56 | D9 NodeTemplate/OfficeTemplate/Preset/default输入仅一次完整source，D7Action不共用自由模板blob（PROMOTE/TERMS）。 |
| A2-57 | 无兼容原型替换S1–S7，Lexicon记录deletion targets，历史不删（TERMS）。 |

## 4. 原Facet16 / People16 / ICS6

Facet16继承D4组合门并经D7通用Query/Action检查；不会因旧候选叫Profile而新增Query domain。

| 原矩阵 | 对应D7 fixture |
|---|---|
| §2.6-01 | EXT/PROMOTE |
| §2.6-02 | FIELD |
| §2.6-03 | EXT/CALENDAR |
| §2.6-04 | EXT |
| §2.6-05 | EXT/LIBRARY |
| §2.6-06 | DOMAIN |
| §2.6-07 | FEATURE |
| §2.6-08 | FEATURE |
| §2.6-09 | VALUE |
| §2.6-10 | FEATURE |
| §2.6-11 | VALUE |
| §2.6-12 | FIELD |
| §2.6-13 | FIELD |
| §2.6-14 | FIELD |
| §2.6-15 | DOMAIN |
| §2.6-16 | ACCESSIBLE |
| §13.5-01 | SEARCH/REFS |
| §13.5-02 | TERMS/VALUE |
| §13.5-03 | BARRIER/GRAPH |
| §13.5-04 | DOMAIN/FIELD |
| §13.5-05 | DOMAIN |
| §13.5-06 | PROMOTE |
| §13.5-07 | DOMAIN |
| §13.5-08 | VALUE/FIELD |
| §13.5-09 | CALENDAR |
| §13.5-10 | COLLEAGUE |
| §13.5-11 | FIELD |
| §13.5-12 | SEARCH |
| §13.5-13 | COLLEAGUE/GRAPH |
| §13.5-14 | EXT/CALENDAR |
| §13.5-15 | FIELD/DOMAIN |
| §13.5-16 | FIELD/BARRIER |

People周年日与holiday派生：本版只执行D4已完整冻结的recurrence；partial calendar precision不能补成年月日，生日规则/提醒派发需D10验证provider合同，当前明确不可用；依赖变更reset且不复制birth fact。

| ICS组 | D7边界 |
|---|---|
| master/occurrence identity | D3Node identity与原originalStart key分层，无materialization。 |
| subscribe/sync | 外部source与D3binding由D9/D10，不等于d7_subscribe。 |
| copy/import | fresh identity和ownerlocal资源原D3mode，Query无副作用。 |
| promote/adopt | 明确Action与完整source，不能将rowHandle变NodeRef。 |
| mixed VCALENDAR | D9分类mapping/loss，D7typed输入不容任意JSON。 |
| recurrence/timezone budget | D4完整horizon/moved exceptions/coverage/limit；超限失败，不截断或猜tzdb。 |

## 5. Chart §15全家族与横切检查

View§3列出第一批六图、七种非图展示与network；§6逐家族拒绝/延期及所需通用合同。tree/treemap/Gantt、统计密度/分位、地图/金融数据等不能从通用图库参数获得隐含支持。form是生成Action的D8交互。
§15.8逐项对应：宽表=WIDE；全部顺序=ORDER；空/零/缺失=EMPTY；PanelPartition=PANEL；tooltip/reference说明=VIEW（当前overlay不支持）；精确scale/domain/unit=VALUE/VIEW；graph/tree/Gantt=GRAPH及View§6；版本重绑=VIEW/FEATURE；稳定color/legend=PANEL；资源=BUDGET；a11y/CJK/RTL/print/六surface=ACCESSIBLE；多DynamicBlock=DYNAMIC。错误与late validation=COMPLETE。

## 6. 宏观替代、保留限制与真实运行边界

已比较的完整替代：每业务域专用查询（拒绝，扩展需改grammar且权限分叉）；任意JS/View脚本（拒绝，不能静态追踪/复现）；SQL+独立expression（拒绝第二语言与隐式row identity）；typed DAG+纯CEL（本候选，复杂度和完整结果延迟明确）；单pipeline（作为UI投影可用，不能替代共享scalar规范形态）；持久Record/Field库（D5已取消，D7不恢复）。这些判断是作者候选，独立审查可提出满足全部约束的更优完整替代。
Narrow Field合同明确Policy/2的metadata授权和commitSequence算法；真实Registry+D2 carrier+D4 typed输入的phone成功/拒绝/CAS/replay由当前源码及报告的22项有界模型检验。所有模型源码与实际依赖随原完整审查包提供，报告精确限定覆盖；它们不证明完整产品事务/host。历史D6 R5-P2-A、R5-P2-B仅是问题追溯标签，旧结论不作为当前接受证据。
D8 RTL intake完整随包，作为下游交接约束；本阶段未启动D8。Office §14十项由D9模板语法/column寻址负责，D7仅提供完整typed结果/版本/权限、无新增普通Node导出配置、无持久table/column ID。

## 7. revision03具体构造与边界

以下补充对上面的全部原编号继续生效，不用新增编号代替Mandatory覆盖：VALUE覆盖18种D4构造、全部qualifier/provenance与既有DerivedDuration；实际59条原Entry及7条Family Entry已由D4接纳。ORDER/VALUE覆盖NFC equality一致序、samebasis数量聚合、堆叠累计几何轴域。GRAPH的完整source/QuerySpec/TerminalSchema/ViewSpec见Family Witness，父母方向与显示层级分离，spouse/sibling同层、并行边/孤点/隐藏目标/literal relatives明确。

FACET投影明确要求read facets的scope=declared/effective，分别消费同源revision的D2直接声明与D4完整闭包。真实catalog的calendar/event依赖calendar/range-note：只声明event与显式同时声明两者有相同effective、不同declared，不能靠闭包反推作者选择；Task仍用ordinary+exact declared tasks/task。该区分适用于所有扩展，不引入领域专用算子。

FIELD/PROMOTE补充Execution§10–11：两个同值phone条目通过Field selection明确定位；toggle_checklist、AssignFacet/RemoveFacet/CleanupFacetFields已有closed输入并交原D4/D6；普通list_item或错native kind拒绝。POST/DYNAMIC补充源内SavedQueryDefinition.creationPolicy、唯一QueryCall和D3完整绑定，重启从源恢复，变化使旧prepare依赖冲突。

AUTH/POST/BULK覆盖PreparedActionBinding、D3 wire11 stage3/5/14、原planned与saved重放；完整预览和已提交效果采用Preview/Effects合同，15种D3模式全部列出。DYNAMIC/REFS的copy/fork typed references、DefinitionAddress与创建策略由Q合同处理，普通text保留，跨Workspace map外Ref明确整体拒绝。新作者源尚未分配fresh引用不伪造UUID；既有相互引用定义copy/fork完整支持，循环Query仍按执行definition_cycle拒绝。

保存定义slot路径仅采用D3逐segment typed comparator；Counter索引2先于10，不能混用CJ/3整路径字节序。恢复继续遵守原D6最终事务中当前授权/依赖再证明：相关变化不能提交旧计划；全局保守版本改变可再证明原固定范围未变；无法证明则保持planned，不靠新Query改选目标。新增SQLite有界模型明确区分这些分支，并检出只算preview前200条的错误；它不替代产品完整恢复实现。

本版未扩展Chart原延期边界；没有任意join、脚本renderer、用户函数、长期Record或新的领域专用Query语法。架构acceptance和真实产品conformance仍是两种不同证据。


### 周期与相邻时间消费链补充

A2-04/A2-40/A2-45：Algebra§11与Temporal Query View Witness给出day/week/month/quarter/year原Entry→D2 carrier→D4 gate→typed read→CEL解包→terminal→Calendar/Timeline正例；月/周viewport共享完整原Node与period身份。9999 exclusive end越域保留unavailable，明确限定该Calendar只画可编码区间，全状态table保留原period。A2-06通过date/instant range显式解包和正区间验证；A2-27仍需显式Facet Action，绘制不增加Event。DerivedDuration与recurrence保留原D4语义、provider/负范围和权限；Temporal报告分别限定运行与未运行范围，不能用一个总数替代这些链路。


revision06细化：GRAPH的Family Witness以同一个graph.nodes.literal_relatives及显式network.nodeDetails显示文本亲属不可展开末端明细卡片，保留关系标签/重复/空text而不生成NodeRef或第二terminal。REFS/LIFECYCLE按D3§7.3.1保持此前独立trash成员的restore membership，owner purge另取完整owned闭包。VALUE/ORDER使用明确constructor expected type与NFC Optional默认值；PREVIEW覆盖existing C的条件item/no-op/revision、恢复位置完整映射、全variant排序及不可变语义与delivery epoch。上述为架构合同，具体新用例与全消费者验证以本修订实际报告为准，不沿用旧pass数宣称通过。
