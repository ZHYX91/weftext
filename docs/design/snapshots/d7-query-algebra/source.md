---
_weftext:
  id: "ae07f3ef-aee1-473b-a992-a17a530d0416"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 Query Algebra — revision06-draft

## 1. Source selectors 与 scan

Selector 是 closed object：`{kind:"workspace"}`；`{kind:"entities",value:S}`；`{kind:"subtree",root:S,includeRoot:bool}`。S 恰为 `{kind:"literal",literal:TypedLiteral}` 或 `{kind:"parameter",name}`，不得 CEL expression。entities 的值为同 domain ref 的 list；去重完整 Ref 后形成集合，输入重复不产生重复 rows。subtree 只用于 Node domain，root 为 NodeRef；通过先验 root state disclosure 后才找树，root隐藏/不可用统一 not_visible；root live且授权才完整遍历结构范围。workspace 绑定执行请求中的唯一 WorkspaceRef；跨 Workspace refs 拒绝。实体存在性仍由D3，不能先查path再授权。

`scan` exact `{id,op:"scan",domain,selector,as}`，domain=`nodes|resources|annotations|headings`。nodes 包含 ordinary与template，不隐式排除Task或Template；resources/annotations只产生owner-local完整Ref。headings selector仍选Nodes，逐其完整D2 section occurrence产生对象 `{owner:NodeRef,title:text,level:int64}`，内部保留Locator/revision，不公开其坐标；顶层文档title不是heading。domain不接受people/tasks/records或plugin name。

scan output schema恰有一个名为as的column，类型分别为NodeRef/ResourceRef/AnnotationRef/heading object。所有scan unordered，entity domains每个Ref最多一行，headings是occurrence bag。Internal key 按§8的完整K构造，scan特有payload包含domain、完整Ref及heading的D3 locator规范位置，位置仅对同源revision有效，不能用path/title识别。source-free entity scans只读D3/D6 inventory与state；headings须D2 full-source read资格、完整projection，invalid不得跳行。

实际集合为当前授权主体可观察的live实体。对完整Ref先判适用state disclosure，不获权则不查existence；隐去对象不计入count。exact entities中无权与missing均零成员，不返回逐Ref状态；公开输入本身不能授存在性。结构selector必须有完整目标结构观察资格，否则前置not_visible，不能沿隐藏parent推测可见后代。query-scan负范围依赖覆盖输入集合、隐藏策略generation和完整inventory cut。

## 2. read：明确的数据引入

`read` exact `{id,op:"read",input,from,bindings}`。from是静态类型EntityRef的CEL；bindings非空数组，每项 `{name,source}`。name不得覆盖输入column，同批绑定只读输入row，不能引用同批兄弟输出。输出=原schema+新增column，multiplicity/cardinality/orderedness/原lineage保持。任一from失败整个query失败。read不是隐式left join；from对应对象不live/不可读时not_visible，不输出none伪装无Field。调用者想沿可见关系取目标，应先traverse。

source variants及类型：

| source exact shape | 适用from | 输出型与语义 |
|---|---|---|
| `{kind:"title"}` | NodeRef | text，D2 title原文 |
| `{kind:"core_kind"}` | NodeRef | TypeSpec exact `{kind:"text"}`；cell为D2当前完整分类的JSON string `"ordinary"`或`"template"`，只此两值；不虚构D4 CodeScope或SemanticCodeId |
| `{kind:"facets",scope:"declared"}` | NodeRef | list<text,32>，同Node/sourceRevision的D2完整分类中作者直接声明的Facet集合，按UTF8字节排序、唯一；不从effective反推 |
| `{kind:"facets",scope:"effective"}` | NodeRef | list<text,1024>，同一声明及当前D4 Registry下完整有效依赖闭包，按UTF8字节排序、唯一；不以声明上限截断，未知provider不当空 |
| `{kind:"body_text"}` | NodeRef | text，按D2 tree递归连接text与显式换行，规则见下段 |
| `{kind:"field",fieldId,shape:"values"}` | NodeRef | list<T,8192>，完整Field author order；无Entry=空list |
| `{kind:"field",fieldId,shape:"entries"}` | NodeRef | list<object{value:T,qualifiers:Q,note:Optional<text>,authoredProvenance:list<P,16>},8192> |
| `{kind:"derived_duration",fieldId}` | NodeRef | list<DerivedDurationUnion,8192>，每个原Field Entry产生一个派生值，语义见§9 |
| `{kind:"derived_period_range",fieldId}` | NodeRef | list<DerivedPeriodRange<T>,8192>，每个原Field Entry保留完整value及独立周期边界结果，语义见§11 |
| `{kind:"structural_parent"}` | NodeRef | Optional<NodeRef>，root none；需parent及结构观察资格 |
| `{kind:"resource_descriptor"}` | ResourceRef | object{mediaType:Optional<text>,byteLength:integer}，完整已提交descriptor，不含ByteHandle/URL/路径；D2 derived.mediaType不可用为none，不能猜；mediaType是可重建元数据而非作者Field |
| `{kind:"annotation_body"}` | AnnotationRef | text，D2 plain_text body |

facets的scope是必填closed enum，不按UI或当前数据默认。两种投影都从同一当前完整D2分类、D4 Registry与source revision产生，采用相同完整source_read及分类可披露门；结果分别保存declared/source与effective/Registry依赖。D4有效性仍完整验证，声明投影不绕过未知Facet或facet_conflict。Task判断要求core_kind=ordinary且declared投影显式含exact tasks/task；不能把effective包含当作独立Task判据。一般Facet域按D4规则消费effective。相同effective而不同declared可通过两次显式read区分，不推断或删除原声明。

Q 是 Field所选D4 qualifier set展开的closed object，非必填member用Optional；P为Value§5.1的D4 authored provenance完整closed union映射，原Entry省略provenance时投影为[]；present空数组仍由原D4门拒绝。shape values仍执行全部Entry类型/基数/关系ref门，但不披露未请求note；shape entries不公开occurrenceKey或D5selector。FieldId字面常量按同cutRegistry类型检查，不依据第一条数据猜类型。Field与别名新定义仅改变Registry输入，read grammar不变。

body_text固定提取D2 body：textinline取原value，link/citation取显式label（无显式label的引用不隐式取title，使用空text），resource取显式caption否则空text，inline容器按sourceorder连接；每个block完成后追加LF，section先title+LF再children；table逐cell text用TAB连接、逐rowLF；protected literal/source取payload原文；saved Query/View definition、attribute carrier和comment不贡献。去除最终一个人为LF，保留原payload内换行与Unicode；输出不是exact author source，不能回写。D2未知/invalid不部分提取。

read先按完整Ref/state和每项source所需source_read/field_read/resource_read/annotation_read授权，再availability/parse。只有Field权限不授权whole source或其它Field；内部为保留/解析而读取源不产生对外权利。输出成功/失败、D2容器有效性、源版本、实际被观察的ref目标状态、Registry及控制依赖的资格必须先证明。普通作者Ref/Locator原值按Value§5.1完整保留而不解析；持有该值不授权read.from、traverse或renderer取得目标状态/内容。可执行baseline：完整source_read且无适用Field read deny可消费D2源；只有field_read的路径必须有静态完整输出独立性证明（涵盖D2 framing/容量与所有control依赖），没有证明时在业务读取前统一not_visible。本版窄Field成功路径以《D7 Narrow Field Qualification》的Registry先验保持证明及明确metadata权限为准；不自动加入policy_admin要求。不得“读完发现恰好无秘密”后才授权。

### 2.1. core_kind 完整类型与消费例

在当前源已通过 D2/D4 完整分类与读取资格后，普通 Node 与 Template 的投影都使用同一 text TypeSpec。执行 `read {name:"core_kind",source:{kind:"core_kind"}}` 后显式 `project` 仅保留该列，完整终端 schema 为：

```json
{"kind":"rows","ordered":false,"columns":[{"columnId":"core_kind","type":{"kind":"text"}}]}
```

普通 Node 的 cells 是 `["ordinary"]`，Template 的 cells 是 `["template"]`；这些是 cells 的全部内容，运输 row 仍按 Execution§1 带各自受管 rowHandle。CEL `row.core_kind == "ordinary"` 为合法 text 等值比较，Template 返回 false；`row.core_kind == 1` 为 type_mismatch。View 的完整示例：

```json
{"format":"weftext.view","version":1,"inputSchema":{"kind":"rows","ordered":false,"columns":[{"columnId":"core_kind","type":{"kind":"text"}}]},"layout":"table","bindings":{"columns":["core_kind"]},"options":{"title":"Core kind","description":"当前完整分类的只读投影","labels":{},"textDirection":"auto","numberFormat":"canonical"}}
```

把该 View inputSchema 换为 semantic_code/optional/不同 columnId，均 schema_mismatch；不得根据首个 cell 补类型。source `{kind:"core_kind",scope:"core"}` 违反 closed shape；Core 的 `person` 或 `core/ordinary` 不是合法分类值，必须在源分类门拒绝，不能变成新的 code 值。D2 的 Template/Core kind、Task 判定保持原义。

## 3. 基础关系算子

所有 `input` 为relation ID，expr为单一CEL string。

| op exact成员（均另有id/op） | schema / multiplicity / order / lineage |
|---|---|
| filter: input,predicate | predicate bool；保留为true的原行；0..N；保序/lineage；may_error完整失败 |
| derive: input,fields:[{name,expr}] | add-only；同批expr输入scope；一行一行；保持order/lineage |
| project: input,fields:[{name,expr}] | 替换为非空closed schema；expr同批输入scope；一行一行；保持order与内部唯一lineage |
| unnest: input,expr,as,mode | expr为list<T>；mode=inner/left；inner每item一行，left空list一行none；as在left为Optional<T>；原columns保留 |
| aggregate: input,keys:[{name,expr}],measures:[Measure] | 输出仅key+measure；keys可空，measures非空；group bag；unordered；擦除来源/action lineage |
| distinct: input | 按全部公开cells相等去重；规范raw代表；unordered；擦除来源/action lineage |
| sort: input,keys:[{expr,direction,none}] | keys1..16，direction=asc/desc；none=first/last，对非optional也必填但无效应；完整total order；lineage保持 |
| take: input,count | count 0..1,000,000 Counter；要求输入ordered；取规范前count；lineage保持 |

derive/project fields1..64唯一name，任一expr错误整query失败，禁止按列顺序赋值。unnest as不能覆盖输入名。inner保留每个重复item；left非空也包装some；childKey使用§8的operator+parent+itemOrdinal规范构造，空left为独立empty标记；left的T若为Optional在编译时type_mismatch，不能生成nested Optional或混淆原none与空list；输入有序则按parent序再ordinal有序。unnest产生多分支lineage，因此不再作为隐式唯一源动作依据；显式EntityRef可以后续fresh解析动作。

Measure exact `{name,kind,expr?,scale?,rounding?}`，kind=count_rows/count_values/count_distinct/sum/avg/min/max。count_rows禁止expr，其余必须expr；只有avg必须scale/rounding，其他禁止这两项。expr可Optional：count_values只数some，count_distinct只数present的不同值，sum/avg/min/max忽略none但无present输出none；count_rows数所有occurrences。sum支持同型numeric或quantity；avg同型numeric→Optional<decimal>、quantity→Optional<quantity>；quantity全present的完整basis必须一致，按Value§5.3保留unit。其它类型拒绝。keys每项必须equatable，同名/与measures撞名拒绝。

sort先按key逐一比较，完全相等时使用内部LogicalOccurrenceKey规范字节比较，asc内部tie固定不因最后key desc反转。keys表达式在全部输入上先完整求值；不能用top-k绕过后段错误。Orderedness是查询type/effect，不意味着原source有用户可见自然序。经过queryRef、aggregate、distinct、traverse必须新sort后take。sort不改变bag；take是明确语义截断，不等于分页或host预算。

## 4. 关系 traversal 与 related

`traverse` exact `{id,op:"traverse",input,from,relation,direction,mode,multiplicity,targetAs,edgeAs?}`。from为NodeRef CEL；relation为完整D4关系FieldId或封闭`core/structural-parent|core/document-link`，direction=outgoing/incoming，mode=inner/left，multiplicity=per_edge/distinct_target。targetAs新增NodeRef（left Optional）；edgeAs仅per_edge可给，为经过授权的object `{relation:text,direction:text,value:FieldValue,qualifiers:Q}`（core关系value为object{target:NodeRef}，Q为空object）；left包Optional。关系种类字段永不省略或改成generic parent。direction对symmetric只改变起点视角，不新增inverse事实。

每个输入row执行一个hop。D4 directed/symmetric按原actual owner、完整incidence；literal target不会匹配Node、产生inverse或graph endpoint。目标非live或任何edge/target/metadata资格不足时此edge对Query不可见，连edge数量/qualifier也不输出；来源owner本身不合格则not_visible。distinct_target对每个输入row按完整targetRef去重，禁止edgeAs（否则代表哪条edge未定义）；per_edge保留重复事实和并行document links。left无可见match产生唯一none行；这表示可见关系子图没有match，不宣称全Workspace无关系。输出unordered；key按§8完整K的per_edge/distinct_target/left variant，包含本调用和本算子，不能只用endpoint pair；left用empty标记。traverse擦除隐式单一源Action lineage，显式targetRef仍可fresh动作。

core/structural-parent由D3树控制state，outgoing子→parent，incoming parent→children；完整结构观察资格不可省。core/document-link只从D2明确NodeLink occurrence，outgoing作者owner→target，incoming需完整引用范围；不同source occurrence是不同edge。两者与organizations/parent、tasks/dependency、people/parent均不合并。

`related` exact `{id,op:"related",input,from,relation,direction,quantifier,predicate}`，quantifier=any/none。predicate scope只有 `target` NodeRef、`edge` 上述typededge；可比较两者已有值但不能read/dereference；必须静态total bool，禁止outer row/scalar相关变量（from可依row，predicate可用固定param）。如需目标title谓词，先把字段作为D4relationvalue或者使用有界traverse/read/filter组合；v1不在relatedpredicate隐式fetch。any在可见edge中存在true即保留input，none在无true时保留。保序/原lineage；全范围负依赖仍记录；不等同left-traverse后filter。若predicate只问targetRef相等，可直接表达“无指向指定节点的关系”。

## 5. Scalar 与 QueryRef

ScalarSpec exact二variant：`{id,kind:"aggregate_value",relation,column}`，relation必须keys=[]的aggregate，column为其measure；或`{id,kind:"query_ref",definition,arguments}`，callee必须scalar result。scalar不可关联调用者row或this，同一次invocation求值一次，可被多个derive/filter读取。联合图阻止relation→scalar→同relation cycle。global aggregate保证一行，不把任意零/多行relation隐式压成scalar。

QueryRef relation exact `{id,op:"query_ref",definition,arguments}`，无input，只有root。definition是下述DefinitionAddress；arguments为ValueSource object，值仅literal/parameter（caller scalar或row禁止）。callee只能访问传入值、同cut显式context及其自己的定义闭包。security-invoker：保存者/owner权限不传播。先current definition可见性，再编译closed payload，再数据权限；可读定义不等于可读数据。

DefinitionAddress exact `{owner:NodeRef,at:A}`，A=`{kind:"anchor",name:text}` 或 `{kind:"locator",locator:D3Locator}`。anchor解析使用D3唯一anchor规则，丢失/重复不回退名称/title；locator必须当前精确revision和saved-definition元素kind。无独立definitionId/ViewRef。D3 owner/locator披露和D2源读取授权在查找前。地址在一个调用cut解析一次，绑定完整owner source version、实际payload、anchor/locator解析和Registry闭包。保存源更改使下一次调用重新解析，旧result按D6cut保持但不能新动作；订阅重算新的定义版本，shape改变reset。

保存Query payload为Execution§7完整SavedQueryDefinition；调用只执行其中QuerySpec，不读取其creationPolicy作普通Query计算，也不引用this。query_ref depth≤16，包含重复调用在内最多64次invocation，所有保存定义解析出node+revision+occurrence再作call-stack cycle检查；地址拼写不同不能绕过同一occurrence cycle。每次调用只导入公共terminalschema/cells，擦除order、Provenance、ActionEvidence和implicit lineage；Core内部caller namespace加callee occurrenceKey，重复rows仍独立，不按cell去重。ResultRowHandle不被import。调用后显式sort可以依内部key稳定take，callee内部identity不暴露。

## 6. 通用领域投影与必要的有限专用语义

以下是**通用已有数据**的组合方案；领域名不是grammar。Facet值判断可用`read facets→derive/filter contains(list,...)`；因此CEL另允许 `contains(list<T>,T)->bool`，T equatable，total。字段读取和关系例子均必须通过同一Registry/权限路径。

Person/Organization/Task/Library均scan nodes→read facets/fields→filter→project。任职事实保留organization Node target或literal、position/rank/department、validity及notes；未知validity不当作当前。MyWorks通过library/creator与显式currentPersonRef条件，不因draft/published分域；ISBN/DOI等externalId不是Node identity。同名对象用完整Ref区别，显示名变化不改变集合。

Search使用通用数据管道：Query read title/body_text与**显式列出的**Field values，再CEL有界list.exists与contains/startsWith、derive显式rank、sort、project。无隐藏global aliases字段或自动transliteration。People name.role alias/former/transliteration等由people/name原值进入同一个read；Organization name也是Field；新领域可注册自己的nameField和下述纯数据contribution，grammar不变。默认产品搜索是可检查的Query模板生成器，不是第二搜索执行引擎。v1 substring exact或显式nfc字段；不声称自动大小写折叠、语言分词、拼音、音译、模糊或全文相关性。多字段匹配用bool OR，一Node一行，rank用明确int64 literals与conditional，不暴露隐藏字段分数；输出snippet由已读text取值，v1不自动高亮截取。插件移除使实际依赖其Field的模板definition_unavailable，不补空结果。

D7冻结纯数据 `SearchContribution`：exact `{contributionId,version,fieldId,textPath,role}`，contributionId为D4 namespace/local-id同词法，version为正Counter，fieldId必须归已验证namespace且当前available；textPath为0..8个静态object member名，指向Field value的text或Optional<text>；role=`name|alias|content`。不包含脚本、网络权限或全文index私有payload。D10负责接纳/安装/真实性并提供同cut完整Contribution集合和独立generation依赖；D7不在D4已closed RegistrySnapshot中塞新member，也不自建作者别名源。未实现该D10来源时，只能使用明确内置、版本固定并经相同字段验证的描述或显式Query，不声称插件动态注册已实现。

搜索模板输入为同一Workspace、needle text、明确选择的contributionId集合（或由用户选择all且绑定当时完整集合）、title/body两个bool。builder按contributionId字节排序产生read column，先按完整D4→D7 bridge展开每个textPath：required exact text直接使用path，Optional<exact text>使用path.orValue('')；required NFC text直接使用path，Optional<NFC text>使用path.orValue(nfc(''))。exact路径的比较needle保持exact，NFC路径显式使用nfc(needle)；所用contains/等值两侧完整TypeSpec必须一致。对每个贡献以values.exists(v,上述同型text表达式的匹配)生成total布尔，不能用may_error的.value()及非literal条件分支假定流分析消除错误。title exact equality优先rank0，name exact1，alias exact2，任一substring3，body/content-only4，未命中不输出；多个命中取最小rank。每个rank均仅从已授权read值求得，之后sort(rank,title,内部tie)，project NodeRef/title/rank。完全空needle拒绝invalid_request，不能变成未声明全域枚举。read任何必需contribution不可用按既定整query规则失败；不得隐式跳过无权字段后仍宣称执行同一完整搜索。规模超过Query列数/AST/预算则明确budget_exceeded，不按插件顺序丢contributions。第三方扩展只增加descriptor和Registry事实，不改CEL/Query grammar或授整个package read权。

Link display Query显式投影label：source显式label优先，否则经授权read target title；Person preferred name只有显式选择people/name及qualifier时参与。无target读取资格显示调用处已知的静态“不可用链接”界面状态，不fetchhiddentitle；完整Ref不变。View仅用投影label。

Graph的Query输出可用traverse得到edge表；每row必须保留relation种类、方向、asserted/derived标记及显式endpoint labels。FamilyTree仅D4graphProjection=family允许的亲属边；friends/professional关系不当家谱代际。ranked为布局建议而非永久rank或亲属证明；layered_v1按View规定的显式forward/reverse/same/none约束求几何层级。10k engagements按选定person→outgoing engagement组织→incoming engagement其它person两次通用traverse；保存第一条edge的qualifiers，再用两条明确validity比较求交集，过滤self，distinct到对方Ref后得到按需colleague；不写约50m作者事实。无任职时间不当当前：先要求两条validity存在、union variant同为date或instant、每端完整有界，再比较 `startA < endB && startB < endA`（end-exclusive）；不满足已知时间条件不宣称重叠。使用unionValue+Optional显式guard，filter允许may_error但lazyguard阻止实际none读取。无界有效期只有在作者显式unbounded而非缺validity时才按相应无穷端点处理；此时比较公式对缺start/end分别取true，不将整个缺validity当全时段。日期历法/version/precision属于动态basis；先以dateBasis(a)==dateBasis(b)作显式lazy guard，只有同basis分支才比较，跨basis不会由相同TypeSpec误认为可比较。缺comparator在原read阶段不可用。输出标derived、依赖两条源事实及完整负范围；asserted professional-relation与该推导并存但不自动合并。全pair join仍不在v1，选中一个person的两hop查询不需要专用colleague算子。

`recurrence`是此版独立时间展开operator，调用D4已冻结算法；§9/§11的派生投影仍属于通用read。三者由共同时间语义定义，不按业务namespace增加grammar。exact `{id,op:"recurrence",input,from,horizon,limit,as}`；from NodeRef；horizon为下面明确的TypedLiteral range bridge，limit1..65535，as新增object `{originalStart:point,range:range,title:Optional<text>,eventStatus:Optional<code>,note:Optional<text>}`。输入Node必须有相应range+recurrenceField及完整D4RecurrenceReadContext/1和显式Calendar/tzdb版本；非recurring Node不得自动造一条（静态调用者先筛选）；每个输入expand受D4完整horizon/limit契约，超过limit整体失败而非take。outputbag unordered，内部key为§8规定的invocation/operator/父occurrence+原series+canonical originalStart，而非最终显示时间；重复父行各自展开仍保留不同key；moved-in exception必须包含，final相同而original不同不合并。无Node materialization/提醒/外部sync，ruleprovider缺失明确unavailable。

recurrence的horizon仍使用Value§1已有TypeSpec，不存在kind=date_range或instant_range的Query primitive。其type必须恰为object，members按UTF8顺序为end和start，各为Optional<calendar_date>或各为Optional<zoned_instant>；两端value均必须some。Core先按TypedLiteral完整验证，再把start.value与end.value映回D4原start/endExclusive并加对应date_range/instant_range kind，交给原D4 horizon gate验证同basis与正区间；不能把Query object wire原样送入D4或让实现任选两种编码。源recurrence/range的date/instant variant必须匹配该静态point类型，否则type_mismatch。输出originalStart使用该point类型；range使用同一Value§1 Optional端点bridge；title/note为Optional<text>，eventStatus为Optional<semantic_code>，scope恰为当前calendar/event-status的完整ResolvedCodeScope。三个Optional presentation值仅来自D4 occurrence的明确overrides，未给为none；需要普通Node title/Field fallback时由Query另一次显式read并用CEL选择，不由recurrence隐式扩大读取。这些派生值均无作者写回或Event创建副作用。

Calendar period/range/event分别读取其已定义Facet和Field；period经§11显式派生civil日期边界后可进入Calendar/Timeline，range经原Field bridge解包端点，event沿其明确range读取；period identity七成员以及scope/config版本不由显示日历猜测。D4已冻结的DerivedDuration由下面通用read adapter承接；holiday/workday、周年日、节气等未冻结算法不得擅自新增；D4规则包提供者在D10接纳前相应派生功能不可用。输入已包含明确算出的数据仍可呈现，但不能自动写回或冒充本版计算。

## 7. 完整示例（新领域无语法改动）

假设当前Registry合法定义 `equipment/device` 以及 `equipment/serial-number:text`。以下Query只有通用operators；更换为people/person及people/name时grammar不变，返回type由Registry改变。

```json
{"format":"weftext.query","version":1,"parameters":[],"relations":[
 {"id":"n","op":"scan","domain":"nodes","selector":{"kind":"workspace"},"as":"node"},
 {"id":"f","op":"read","input":"n","from":"row.node","bindings":[{"name":"facets","source":{"kind":"facets","scope":"effective"}}]},
 {"id":"d","op":"filter","input":"f","predicate":"contains(row.facets, 'equipment/device')"},
 {"id":"v","op":"read","input":"d","from":"row.node","bindings":[{"name":"serials","source":{"kind":"field","fieldId":"equipment/serial-number","shape":"values"}}]},
 {"id":"p","op":"project","input":"v","fields":[{"name":"device","expr":"row.node"},{"name":"serials","expr":"row.serials"}]}
],"scalars":[],"result":{"kind":"rows","relation":"p"}}
```

这是合法Registry前提下的Query示例，不是已安装equipment插件或真实source fixture。设备没有新的domain、EntityRef或权限接口。需要单serial时用at+Optional，不能因首条数据只有一个值就推导scalar schema。

## 8. 每算子的LogicalOccurrenceKey

K是内部closed tagged tree，只有如下构造；所有输出行都由其产生，禁止host枚举序、分区号、自增计数、显示值散列或结果页位置充当key。编码为D3-CJ/3 canonical UTF8 bytes，比较为unsigned byte lexicographic；规范非负ordinal使用D3整数，Ref用其完整wire，eqKey用Value§2带TypeSpec的Equality canonical tree。使用完整树比较，不用碰撞概率证明唯一。

`I`为调用路径：root=[]；每次QueryRef（relation或scalar）追加caller的canonicalOrdinal，callee重复调用不共享I；同一scalar引用重复使用仍是同一次invocation。`o`是当前Query内canonicalOrdinal。`base={invocation:I,operator:o}`是下表每个object的必有成员。嵌入parent/callee key按完整树，不先转text。多根graph使用主稿唯一CanonicalGraph分配ordinal。

| op | K在base之外的exact members |
|---|---|
| scan entity | kind=entity, domain, ref（selector已按Ref去重） |
| scan headings | kind=source_occurrence, owner, revision, locator（真实D3完整位置） |
| read/filter/related/derive/project/sort/take | kind=preserve, parent:K；这些op的0/1输出对应输入唯一key，不因投影删column丢key |
| unnest inner或left非空 | kind=unnest_item,parent:K,ordinal（原list零起index；即使item相等也不同） |
| unnest left空 | kind=unnest_empty,parent:K；与ordinal=0互异 |
| traverse per_edge | kind=traverse_edge,parent:K,edge（exact below）,direction |
| traverse distinct_target | kind=traverse_target,parent:K,target:NodeRef |
| traverse left无匹配 | kind=traverse_empty,parent:K |
| aggregate | kind=group,keys（完整按声明顺序的typed eqKey数组）；global group的keys=[]，不用任一输入代表的key |
| distinct | kind=distinct,cells（按完整schema列序的typed eqKey数组）；公开raw代表另按Value规则 |
| query_ref | kind=query_import,callee:K；base已位于caller，callee包含独立调用I |
| recurrence | kind=recurrence,parent:K,series:NodeRef,originalStart（完整typed Equality canonical tree） |

edge closed三variant：Field事实`{kind:"field",owner,sourceRevision,fieldId,occurrenceKey}`；D2 NodeLink `{kind:"document_link",owner,sourceRevision,locator}`；结构 `{kind:"structural",child:NodeRef,parent:NodeRef,placementRevision}`。均取实际受权事实，symmetric incidence两端使用同canonical owner/source occurrence；同一from输入只输出同事实一次，反向视角由direction区分。不把literal target造edge。不存在cross-QueryRef复用ActionEvidence；key只是唯一bag occurrence构造，单一来源/可写性另由Provenance判定。

每个relation内K唯一的归纳：scan源唯一；1:1保父唯一；unnest父+ordinal；traverse父+真实edge/去重target；group/distinct按eq类各输出一次；import调用路径+callee唯一；recurrence在每个父内D4 originalStart唯一。各tag不交叉。输出unordered时仅以K字节序运输，不赋予作者自然序；sort用户key全部相等时以K升序决胜；take随后截取。error排序引用失败op输入K并追加宏ordinal路径，输入枚举/worker分区改变不改变所选错误。

## 9. D4 DerivedDuration只读adapter

`read source={kind:"derived_duration",fieldId}`要求Field展开后的valueType为date_range、instant_range，或其closed union（每个variant都必须是range）；否则编译type_mismatch。fieldId仍是Registry常量，无calendar业务专用算子。先执行普通Field完整授权、可用性、Entry及qualifier/provenance/ref门，再逐Entry调用D4 DerivedDuration/1，输出Value§5.3完整union，保持author order与次数。空Field输出[]。本adapter不会按preferred/first/latest自动挑选，不写Field。calendar/range只是在现有catalog上的常用输入。

date有界：ISO day用Gregorian ordinal差，month用12*(year-1)+month-1差，year用year差；其它已verified calendar用该calendar/version/precision comparator orderedLexemes下标差。exact instant用无界整数UTC秒加精确fraction相减、canonical decimal输出，不用host时间范围。open bound在所有已存在bound通过原provider/ordering gate后才输出unavailable/open_range。Registry comparator/unit/code、tzdb contribution/实际所用规则版本、Field/source revision与负范围均纳入实际dependency；duration本身不新增Calendar rule或调用设备默认。

已通过D4独立有效值检查的zoned_instant用其明确offset确定UTC，不为求差隐式做当地civil-time求解；若组合读取需要D4 RecurrenceReadContext规则coverage则完整绑定和证明，否则不得虚构“已读取timezone segments”。相同range的版本/provider变化使旧cache失效；open_range不得掩盖已知bound不可解析。


## 10. asserted graph edge的值来源证据

Core为已验证D4 relation Entry产生内部FactOrigin={actualOwner,sourceRevision,fieldId,occurrenceKey,完整原value/qualifiers及当前可见endpoint}，不是公开值或ActionEvidence。read entries/values的每个list item只带其自身FactOrigin，不将整个Field的所有origins折叠到每个item。CEL成员选择、Optional/union显式解包、object/list构造保留逐member/item来源；literal/param/row entity本身不制造relation FactOrigin；list.filter保留被选item，map每次仅联合当前表达式实际使用的输入item/member origins，nested list保留各子item树。数值/文本转换可以携带来源但不证明结果还是原关系；未知来源传播必须标为无法证明，不以“看起来相同”恢复来源。unnest选择该精确item的来源树，仍擦除implicit Action lineage；二者是不同用途。

在graph终端，对每条declared asserted row，联合实际sourceRef/targetRef/relationColumn三cell的FactOrigins，必须恰有一个真实完整origin；并逐字证明source/target等于该事实按D4方向允许的端点、relationColumn等于该FieldId，当前state/subject/target资格已通过。literal target、混合两事实、无origin或计算后不再等于原端点全部invalid_provenance。edgeLabel/layout constraint来自明示常量或已读值，不改变事实方向或凭空证明亲子。traverse edgeAs由同一来源规则生成，core结构/NodeLink用各自完整原发生证据并作同样exact match；query_ref擦除这些内部origins。这样多个关系Field先map成同型对象、两次unnest合并为edge表时可保留每条实际asserted事实，同时不会给这些行隐式Field修改权。


## 11. DerivedPeriodRange：通用周期值的只读边界投影

`read source={kind:"derived_period_range",fieldId}` 是 closed shape，from只接NodeRef。fieldId是同cut Registry静态常量，必须通过普通Field定义可披露、type、source/Field权限和全部Entry有效性门。Core推导feature `weftext.query.derived-period-range/1`；不支持即unsupported_feature，不能回退字符串日期或renderer算法。它是本版首次冻结的共同周期语义；新增业务Field复用此shape不新增feature。没有新作者Field、内容identity、事务或D4 Registry member。

### 11.1. 完整输入与输出型

展开全部alias后，输入D4 valueType必须逐成员等于当前 `calendar/period-value` 的完整结构：required object `{period:P,seriesKey:exact text}`；P为七个required members，calendarId/calendarVersion/periodKey/periodRuleId/timeZone/tzdbVersion皆exact text，periodKind为field_local semantic_code，codes恰按D4顺序`day,month,quarter,week,year`。不接受缺省、额外member、optional、normalized text、仅相似名称或其它code scope。这里冻结的是该结构及period语义，不要求FieldId为calendar/period或Facet为calendar/period-note；其它namespace的合法Field可引用同alias或声明完全相同结构。静态结构不合为type_mismatch。编译时还须解析原alias及本adapter依赖的Calendar定义，不能在无数据时猜型。

显式选择这个adapter意味着把每个输入值作为CalendarPeriod完整验证：按D4§9.4在同cut verified Registry证明calendar/version/kind/rule tuple、kind↔keyProfile、canonical真实periodKey以及独立tzdbVersion/timeZone；该检查对任何FieldId均执行，不能依赖D4只为内建calendar/period触发的快捷分支。保持原Field的完整qualifier、note、provenance、ref、基数和约束校验；投影不输出这些未请求的附加值。失败按既定D7映射：权限先not_visible；可披露定义缺失按unknown_definition/definition_unavailable；已授权源值/动态provider无效或不可用为source_unavailable，不包装成下面的unavailable业务值。

设T为普通Field values read在当前完整FieldId下的D7 TypeSpec（包含field_local的完整FieldId）。`DerivedPeriodRange<T>`不是新增primitive；是以下结构记法：

- object成员按UTF8顺序为 `range:R`、`source:T`，二者必填；source逐字保留普通Field bridge的完整value，含七成员period及seriesKey。
- R是有序closed union：先 `bounded`，其type为object成员 `end:calendar_date,start:calendar_date`；后 `unavailable`，其type为object成员 `reason:exact text`。bounded两端nonoptional、严格start<end，end为exclusive。unavailable.reason仅`endpoint_out_of_domain|unsupported_calendar`，这些是D7固定投影状态text，不虚构D4 semantic code scope。
- source数组保持原Field author order和每个Entry次数，每Entry恰一结果；空Field=[]，不会因投影不可用丢Entry。同一个结果item包含其原值和边界，不能分别读取两个列表再猜序号拼接。整体输出list maximum8192；适用原maxRows/value/work/dependency预算，先checked计费，无隐式limit。

wire使用既有Value§1编码。例如range cell为`{"variant":"bounded","value":{"end":{"kind":"calendar_date","calendarId":"calendar/iso8601","calendarVersion":"1","precision":"day","lexeme":"2026-10-01"},"start":{"kind":"calendar_date","calendarId":"calendar/iso8601","calendarVersion":"1","precision":"day","lexeme":"2026-07-01"}}}`；或者`{"variant":"unavailable","value":{"reason":"endpoint_out_of_domain"}}`。外层source仍必填，不受variant影响。

### 11.2. 五种ISO profile与边界

本feature只为已验证的`calendarId=calendar/iso8601,calendarVersion=1`计算Gregorian civil day边界；periodRuleId可为同Registry中任一匹配tuple的合法local ID，算法由其已验证keyProfile确定，不按ID拼写推断。其它已完整有效的calendar/version输出unavailable/unsupported_calendar；不擅自套ISO算法、修改作者identity或装载D10代码。缺规则/provider仍失败，不属于unsupported_calendar成功分支。

| periodKind / 已验证keyProfile | start（含） | end（不含） |
|---|---|---|
| day / iso-date-v1 | periodKey所指Gregorian日期 | 下一civil day |
| week / iso-week-v1 | 指定ISO week-year/week的星期一 | 七civil days后；week 1为包含该year一月四日的星期，星期一为第一天 |
| month / iso-month-v1 | 指定year/month第一日 | 下月第一日 |
| quarter / iso-quarter-v1 | 指定year的`1+3*(quarter-1)`月第一日 | 三个月后第一日 |
| year / iso-year-v1 | 指定year一月一日 | 下一year一月一日 |

算法用proleptic Gregorian精确整数civil运算，四百年闰年规则，不用设备locale/时钟/tzdb算法。每个输出point固定原ISO calendarId/version和precision=day，须通过D4 CalendarDate gate及同basis order；输出需要的day comparator可用性先证明。day/date arithmetic在此adapter内完整冻结，不因此开放CEL日期构造器或任意日期加法。

先在数学整数calendar域计算真实两端，再检查D4公开CalendarDate的0001..9999域。任一端超域时返回unavailable/endpoint_out_of_domain，保留原合法period；禁止clip、宿主exception、变为open bound、加写伪range或把作者值标invalid。例9999-12-31、9999-W52、9999-12、9999-Q4、9999年都可为有效身份，但其exclusive end不可编码；0001-W01从0001-01-01开始可编码。该限制必须作为完整输出状态可见。需要让超域端点成为普通Calendar cell须另行协调D4值域，非本版暗改。

timeZone/tzdbVersion保持七成员identity且完整验证/绑定；civil日期计算不将当地零点转instant、不读取timezone segments，不受DST 23/25小时或设备默认时区影响。同一civil day在不同时区拥有不同原period identity，不能因派生日期相同合并。这里不计算真实经过秒数；需instant range或duration时另用明确已有作者值/规则。周年日、工作日、holiday、节气和外部规则接纳仍属原延期边界。

### 11.3. 依赖、行身份与消费

依赖覆盖完整源revision、所读Field和递归alias/schema/code scope、Registry generation及实际calendar rule/comparator/tzdb contributions、适用authorization/control、scan与Field空范围。unsupported_calendar也绑定已验证但尚不支持的实际calendar/version。source、规则、provider可用性或授权变化使cache/订阅失效或reset；Action prepare/commit仍重证这些依赖，不能因日期恰同省略。期望值全部由同cut解析，不能把caller报告的“已验证”当proof。

read本身仍1:1保持输入LogicalOccurrenceKey与原Node lineage；一个list item不创建新Node/Occurrence identity。derived range没有可修改的FieldSelector，也不是关系FactOrigin。unnest继承原§3 parent+ordinal key并擦除implicit Action lineage；只有显式target NodeRef经fresh Action解析可打开/修改原Node。Node move/rename/template变更不改变原period identity；新source revision仍使旧结果的动作证据失效。

完整正例、逐stage的TypeSpec/行数及完整终端cells在`D7 Temporal Query View Witness v6.json`：scan显式Nodes→read title和derived_period_range→inner unnest→显式filter bounded→derive解包→project完整key/source/start/end/title/target→sort→Calendar或Timeline同一terminal。key用`{target:NodeRef,source:T}`，calendar/period本身基数0..1使其在该例全局唯一；一般可重复Field须作者Query给合法唯一键或接受View duplicate_key，不能以内部ordinal冒充公开identity。月/周是同一日期数据的不同viewport，View不重新解释periodKey；任何viewport裁显是D8明确交互，不改变Query结果或原period identity。

对unavailable，完整all-items Query+table View保留每个source及原因；另一个显式bounded-only Query才筛出可画区间。Calendar结果不得声称包含全部周期：调用产品须让用户看见明确选择的bounded-only条件及可用的完整状态查询，不能静默丢不可画项。D7不是多终端dashboard，因此不承诺一次Query同时返回两个独立View；两个Query各自完整成功和当前授权，不能把两次cut混成一个快照。canonical导出完整all-items数据时保留unavailable。

Calendar/Timeline仅消费nonoptional day points，不读取source.period、不猜时区，不创造Event或reminder。相同period可在month/week视口显示，原NodeRef、七成员identity、seriesKey始终保留；CalendarPeriodScopeBinding仍是D6控制输入，adapter不新增Node作者scope字段，也不重新决定unique/many。
