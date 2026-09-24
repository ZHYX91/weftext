---
_weftext:
  id: "5bb6d1c7-f8f4-4a29-ade3-c7f12dc837c6"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 Value 与 CEL Profile — revision06-draft

## 1. 类型与 wire

TypeSpec 是 closed tagged object。基本型 `{kind:K}` 的 K 为 `bool|text|int64|integer|decimal|node_ref|resource_ref|annotation_ref`。text 为 exact Unicode scalar 序列；需要 D4 nfc-for-compare 时用 `{kind:"text",comparison:"nfc"}`，exact 型省略 comparison，拒绝显式其它值。`{kind:"semantic_code",scope:S}` 的 S 为完整 D4 resolved CodeScope（field_local还绑定完整FieldId，防止跨Field相同code混同）；`{kind:"calendar_date"}`；`{kind:"zoned_instant"}`；`{kind:"quantity"}`；`{kind:"optional",item:T}`；`{kind:"list",item:T,maximum:N}`，N=0..65535；`{kind:"object",members:[{name,type}]}`，成员名使用本节object规则、唯一且按UTF8名排序，至多64；`{kind:"union",variants:[{name,type}]}`，1..8 唯一有序 variants。不得嵌套 optional(optional(T))；object 所有成员必填，D4 非必填 member 转成 Optional；无开放 map/any/dyn/null。calendar_date和quantity的calendar/version/precision、dimension/unit来自D4每个value，不能从无此限制的FieldDefinition或首值虚构静态约束；比较前完整检查其兼容性，不兼容为type_mismatch。

D4 date_range/instant_range 映射 closed object `{start:Optional<point>,end:Optional<point>}`，原endExclusive映为end，仍执行 D4 至少一端存在及端点合法性；unbounded映none，缺整个validity映外层none，二者不混同。D4 qualifier中date_range|instant_range转为union variants date/instant，calendar_date|zoned_instant同样date/instant。D4 bounded_set/sequence 映射 list；set保留D4原canonical authored typed-value顺序，不用Query Equality再去重（NFC等价但raw不同的合法作者项仍保留）；sequence保留author order。只有显式Query distinct/aggregate才采用Query等价类。D4 alias_ref 在 Registry closure 展开为结构 TypeSpec，保留 Field/alias 定义版本依赖；recursive alias 拒绝。D4 external_identifier 映射 object `{scheme:text,value:text}`，保留其 qualified scheme 合法性。Calendar/quantity的动态scope不能只靠显示单位或标签比较。D4 source 值的其他有效性约束仍由 D4，D7 不放宽。

`TypedLiteral={type:T,value:V}`。V 的编码：bool 原生 boolean；text 原生 JSON string；int64/integer/decimal 使用规范十进制 string；ref 直接嵌 D3 对应完整 canonical wire，不包裸 ID；semantic_code 为 D4 canonical code string；calendar_date/zoned_instant 用 D4 原 closed value wire；quantity 为 D4完整quantity wire，unitId须命中同cut Registry，由其取得dimension；list 为 V 数组；object 为按 schema names 的 exact object；union 为 `{variant:name,value:V}`；optional 为 `{state:"none"}` 或 `{state:"some",value:V}`。结果 schema 已给 T，cells 只编码 V，不每格重复 TypeSpec。任何 unknown/null/缺 member 拒绝。空 text、空 list、optional.none 和未绑定参数互异。

数值 lexical：integer/int64 为 `0|-?[1-9][0-9]*`；decimal 为规范 D4 exact decimal（无指数、+、前导零、-0、无意义末尾0；非整数末位非0），整数值可用整数词法。int64 范围 [-2^63,2^63-1]；integer、decimal 为数学任意精度，受统一资源预算限制，不限制为 host double、34digits/18scale。D4 合法但大于 host 可处理预算的值返回 budget_exceeded，绝不截断/coerce。协议 Counter 仍用其原数字词法和界限，不与任意精度作者数混同。

object member名逐字保留D4 lowerCamel字段，Query构造值还允许`[a-zA-Z_][a-zA-Z0-9_]*`；不套终端column/参数的lower_snake grammar。单个name和全部type编码仍受value/schema字节预算。union variant逐字保留D4声明token，大小写敏感；不能按展示label重命名。object成员比较和规范化按schema成员名UTF8顺序，schema列表也须此顺序；终端columns独立保留用户声明顺序。

## 2. Equality、canonical representation 与 Order

相等要求完全同型。bool/text(exact)/integer/int64/decimal 按数学或字面值；nfc text 比较先 NFC 但源值保留；semantic_code 按 scope+code；ref 按完整 D3 tuple；date 比较完整calendar/version/precision与规范日期；instant 按精确 UTC 时刻（偏移和展示 zone 不改变相等）；quantity 比较完整dimension/unit与exact magnitude。date/quantity不同动态scope相等为false，group/hash保留scope；有序比较要求兼容scope，否则type_mismatch，绝不自动兑换。

Optional none=none，some 递归；list 顺序敏感；object 逐 schema member；union 先 variant 再 value。全部合法公共类型可相等、hash、group、distinct。Equality canonical key 是带类型的规范值树：nfc text 用 NFC；instant 用 exact UTC civil ordinal+second+fraction（允许 D4 UTC year0/10000，保留任意精度），quantity 保留 unit；其余递归。使用 D3-CJ3 canonical UTF-8 编码，且类型描述包含其中。不同 raw 值相等时，distinct/group 的公开代表取 `(完整 raw value 的 CJ3 bytes)` 字典序最小者，使输入枚举和分区不影响显示值；不是“第一行获胜”。组合 key 的代表按每个等价类中完整 key tuple 最小 raw bytes 取，不能拼出从未出现的组合。

作者 Orderable 仅 bool(false<true)、text(Unicode scalar code point lexicographic，nfc 型只比较 normalized key，等价 raw 表示不参与值序)、int64/integer/decimal、同 calendar/version/precision date、instant、同 dimension/unit quantity，以及这些的 Optional。Optional sort 必须显式 none first/last；some 使用 item order。semantic_code、ref、list、object、union 没有作者自然序，sort/min/max 直接拒绝；需要排序时 Query 显式投影业务排序字段。内部 canonical value/occurrence tie key 不授作者对这些类型的排序功能。

## 3. 数学与聚合

int64 的单个 CEL +,-,* 一次计算后检查范围；不得把 may_error 表达式重排成不同溢出。integer/decimal +,-,* 采用精确数学值；无隐式 mixed arithmetic，int64→integer→decimal 必须显式函数。decimal 不使用 binary float。除法只用 `divide(a,b,scale,rounding)`，a,b 同为 decimal，scale 为 int64 literal 0..65535，rounding 为 literal `toward_zero|floor|ceiling|half_even`；b=0 错误。令 q=a/b，乘10^scale后按明确模式取整再除10^scale，规范化输出；half_even 正负半值均向最近偶数。普通 `/` 和 `%` 不在本 profile；整除可未来 feature，不能默默截断。

aggregate sum/avg 先以无界精确中间值汇总所有 present 值。int64 sum 最后一次检查 int64 范围，例 `[MAX,1,-1]` 得 MAX，分区不预溢出。integer/decimal sum 最终受编码/预算约束；avg 输出 decimal，需显式 scale/rounding，sum/count 只舍入一次，不先求分区平均再平均。count 输出 integer。资源限额与数学错误分离，不把中间值预算失败宣称为 numeric_overflow。min/max 只接受 Orderable present item；先验证全部present的动态basis兼容，相同值按规范 raw representative 选。NFC等价的两种raw文本满足x==y、!(x<y)、!(y<x)，min/max均取该等价类CJ3最小raw代表。sort先全部用户值序key，再内部occurrence key；raw表示只决定distinct/group/min/max代表，不成为CEL值序的第二层。空 global aggregate 一行：count_rows/count_values/count_distinct=0，sum/avg/min/max=none。空 grouped aggregate 零行。

## 4. Weftext CEL 子集

CEL 是唯一标量表达式语法，采用固定 profile `weftext.cel/1`。本 profile 只接纳 CEL AST 的 literal、identifier、static member/index、list/object literal、unary !/-、binary +,-,*,==,!=,<,<=,>,>=,&&,||、conditional、下表函数及本节显式有界list macros。禁 dyn、null、double、uint、bytes、timestamp/duration 的 host 默认 overload、任意comprehension、map值类型、reflection、regex、I/O、random、now()、user function。不使用 JS/SQL 解析器补充。native CEL 整数 literal 是 int64；大 integer/decimal 用显式构造器。受支持 CEL engine 必须为 exact integer/decimal 注册此处 overload，不改变 native int64 含义。开发期选 parser/library 属实施，不影响本表。

绑定根：`row.<columnName>` 为当前输入 schema；`param.<name>`；`scalar.<id>`；`context.now`、`context.timeZone`、`context.calendarVersion`、`context.tzdbVersion` 仅在推导依赖声明且调用提供时存在。`this` 不进入 SavedQuery；DynamicBlock lexical 引用只用于调用参数绑定，在 Core 中先解析为 TypedLiteral。CEL 不执行 Field 查找、ref dereference、查询调用或授权检查；这些都由 Query operators 明示。

object literal 必须有静态 string keys，类型由下述唯一双向规则检查/推导；list literal 同型，空 list 要 `emptyList(T)` 且 T 为编译期 TypeSpec intrinsic，不用字符串解释类型。static member 可用 `.name` 或 `['literal_name']`；运行时索引用 at，不允许动态 object member。表达式中的 T 是 checker AST type argument，不是第二语言文本；portable AST 输入仍是 CEL string 和所属 Query/Registry 类型环境。

| 表达式/函数 | 输入→输出 | 总性 |
|---|---|---|
| true/false、text literal、int64 literal | 固定型 | total；literal越界编译失败 |
| integer(text literal)、decimal(text literal) | 规范字面量→相应 exact 型 | total；不规范编译失败 |
| integer(int64)、decimal(int64/integer) | 显式精确 widening | total，预算仍可失败 |
| parseInteger(text)、parseDecimal(text) | →Optional<exact型> | total；不合法词法 none；不吞预算 |
| some(x)、none(T) | →Optional<T> | total，T 静态型 intrinsic |
| hasValue(optional) | →bool | total |
| value(optional) | →T | may_error (none_value) |
| valueOr(optional,default T) | →T | total iff两参数 total；普通 eager 参数求值 |
| at(list,int64) | →Optional<T> | total；负数/越界 none |
| size(text/list) | →integer | total；text 按 Unicode scalar |
| contains/startsWith/endsWith(text,text) | →bool | total，exact scalar，不读locale |
| nfc(text) | →nfc text | total，Unicode NFC；保留原value供展示，匹配/比较用normalized key |
| contains(list<T>,T) | →bool | total，T equatable |
| codeText(semantic_code) | →text | total，原规范code，不做本地化 |
| quantityMagnitude(quantity) | →decimal | total；显式丢弃单位，结果是纯数值，不携带已验证单位的承诺 |
| quantityBasis(quantity) | →object{dimensionId:text,unitId:text} | total；来自该值已验证的同cut unit contribution，不换算 |
| dateBasis(calendar_date) | →object{calendarId:text,calendarVersion:text,precision:text} | total；完整动态比较basis |
| unionIs(union,variant text literal) | →bool | total，variant必须在静态type中 |
| unionValue(union,variant text literal) | →Optional<variantType> | total，非该variant为none |
| +,-,* | 同 int64/integer/decimal | int64 may_error；exact型 total数学、预算独立 |
| divide(decimal,decimal,int64 literal,text literal) | →decimal | may_error；零除及规定rounding |
| ==,!= | 同型 equatable | total iff操作数total |
| <,<=,>,>= | 同型 nonoptional Orderable | date/quantity因动态scope检查为may_error；其余total iff操作数total |
| !,&&,|| | bool→bool | 下述确定 lazy 语义 |
| condition ? a : b | bool与同型分支 | 下述确定 lazy 语义 |

`none(T)` 和 `emptyList(T)` 的 portable CEL spelling 固定为 `optional.none()`、`[]` 配合 expected TypeSpec；只有 checker 可从所处表达式 expected type 唯一推导时才合法。上表 none(T)/emptyList(T) 是数学记法，不允许将 TypeSpec 当 CEL 对象参数。

**constructor与expected type的唯一规则。** checker区分合成类型与在已知完整TypeSpec下检查，两者使用同一CEL AST，不引入运行时coercion：
1. 无expected type的非空list literal，逐元素合成并要求相同完整item TypeSpec，其maximum恰为语法元素数；超过65535拒绝。它不从运行时值或外层消费预算推断更大maximum。
2. expected为list<T,N>时，list literal仅在元素数≤N且每个元素按expected T递归检查时取得该完整list<T,N>；空[]必须走此规则。已有类型的非literal list表达式必须与expected完整相等，不能将list<T,3>隐式widen为list<T,8192>。
3. object literal无expected时按各成员合成完整schema并按UTF8名规范排序；有expected object时必须成员集合完全相同，并按对应member TypeSpec递归检查。不存在隐式缺成员、extra member、optional member或开放map。
4. conditional有expected时两分支均按同一expected检查；无expected时两个可合成分支须完整同型。仅当一分支是需要context的[]或optional.none()（或只由这些及context-only constructors组成）而另一分支可独立合成时，以后者完整类型检查前者；两边都无法独立合成则拒绝。不以最大list bound合并两个已合成分支，不改变原lazy求值/总性规则。
5. optional.none()只在expected Optional<T>中成立；optional.of(x)要求x为非Optional，输出Optional<T>。at接收的item和unionValue选择的variant不得已是Optional，否则该调用在编译期拒绝；不创造被TypeSpec禁止的Optional<Optional<T>>，不隐式flatten。其余原函数/宏签名保持，map输出maximum等于receiver，filter保持receiver完整type。
6. expected type只能来自本profile已明确的函数实参签名/receiver推导（例如Optional<T>.orValue要求T）、已声明的参数/TypedLiteral或上述constructor/conditional父节点；普通project/derive不会凭终端label猜expected type，也不新增type-cast语法。普通已类型化表达式均作完整类型相等检查。int64字面量不因expected integer/decimal而隐式转换，text literal不因expected NFC text而隐式nfc。checker先验证整个AST与总性，实际样本、取值范围或未执行分支不能替代这些规则。some 的实际函数名 `optional.of`；hasValue/value/valueOr 的实际 member 名分别 `.hasValue()`、`.value()`、`.orValue(default)`；其它表列函数使用其表中精确名字。禁止表中数学简称作为额外alias。

`&&` 左false不求右；`||` 左true不求右；conditional 只求选中分支。左侧 error 则整个表达式 error，无 CEL 实现的 commutative error suppression；该差异是固定 Weftext profile，engine必须显式遵守。其他参数按 AST 从左到右 eager。checker 总性不靠样本，默认合并所有潜在分支；允许仅由 literal constant 折叠去除不可达分支，不能因为实际数据未遇到none就称total。related predicate 不允许 value(optional)、divide、int64算术等may_error；可用 `.orValue` 提供显式默认。

text匹配函数接受同comparison policy的两输入，exact按raw，nfc按双方normalized key。不隐式改变needle型；Search builder在nfc Field路径显式应用nfc(needle)，在exact路径保留原needle。Unicode normalization版本固定为Core `weftext.cel/1`采用Unicode15.1数据，升级改变匹配结果须新profile feature/语义审查，不使用设备Unicode版本；这是一项实施依赖与conformance义务，不是已运行library证据。

有界list宏仅CEL原spelling `.exists(x,p)`、`.all(x,p)`、`.filter(x,p)`、`.map(x,e)`，一变量、无index变体。变量不能shadow row/param/scalar/context或外层macro变量；静态绑定itemType，list最大值由TypeSpec给出，输出map保同maximum，filter保原type/序。p必须total bool；exists空false/all空true，因p total可短路，不吞预算。map的e可may_error，但所有被求值items按原ordinal执行，任一错误使整个表达式失败；不得跳item，错误候选含macro AST path+item ordinal的内部路径，公共diagnostic只报静态macro AST位置。禁止把用户序列当无限iterator，所有嵌套展开先checked计费。宏只操作已读取值，不引入新source权限或Query子调用。

## 5. 上游值边界

Calendar 日期不能字典序跨历法比较；instant不能限制九位小数或宿主日期范围；date/instant/range不能隐式互转。Quantity conversion、日期加法、周年日规则、holiday等依赖尚未由D10接纳的算法时，本版无对应 CEL 函数。Recurrence 通过固定 Query operator 调用 D4，不能在 CEL 里展开。Algebra§11 DerivedPeriodRange以既有object/union/calendar_date类型输出明确周期边界；只在该read adapter内冻结ISO civil运算，不增加CEL构造器、隐式object→calendar_date转换或日期加法。用户可查询已保存的规则计算事实，必须保留其版本/来源；Query不自动补写。

D4定义的actual relation endpoint及其关系读取仍执行完整target read/state gate：未获资格的关系occurrence在入口整体不可见，不能仅把target清空而保留role/计数。普通非relation typed Ref及作者provenance按§5.1的作者原值披露规则；已读取Ref不授权renderer或其它算子隐式读取目标title/resourcebytes。

### 5.1 D4作者provenance及其位置数据

先以实际containing owner、Registry和原D3 decoder完整验证D4 atoms，再作下述结构映射；不能在映射时修复非法ref/Locator、删除atom或猜其owner/revision。输出使用既有object/union/Optional/Ref类型，不新增Locator primitive、通用target值或CEL resolver。位置数据只是作者已经保存的值；Core内部派生的行Locator和Provenance不走此出口。

P是固定union，variant顺序为external、node、resource、transform；每个variant value是下表closed object。表中的名字是schema成员，实际TypeSpec members统一按UTF8名排序。所有text为exact；D3非负整数转换为int64的规范十进制string，原值须先满足D3域，不经宿主浮点。

| D4 atom kind → P variant | object members |
|---|---|
| external | scheme:text、value:text、observedAt:Optional<zoned_instant>；scheme依旧在输入时通过D4贡献真实性/availability门，不因投影为text而免验 |
| node | nodeRef:NodeRef、locator:Optional<union{element:L_element,range:L_range}>；element/range依原D3 kind选择，不能交换 |
| resource | resourceRef:ResourceRef、regionLocator:Optional<L_region> |
| transform | inputIndex:int64、operationId:text；原provenance数组顺序、backward index与合法OperationId全部保留，不变成content ref |

三个位置投影仅保留下列原成员值，不在投影object重复原kind（variant/type已确定它）：L_element={owner:NodeRef,documentRevisionToken:text,elementKind:text,sourceSpan:Span}；L_range={owner:NodeRef,documentRevisionToken:text,sourceSpan:Span}；L_region={resourceRef:ResourceRef,resourceRevisionToken:text,regionToken:text}。Span={startLine:int64,startColumn:int64,endLine:int64,endColumn:int64}。elementKind、sourceSpan顺序/坐标、revision/region token以及atom与Locator的owner/ref相等关系均由输入的D3/D4原门完整验证；不把展示相似当位置相同。缺失可选成员转none，存在转some；provenance数组上界16且保留每个原ordinal，原Entry省略provenance映空数组；present provenance必须先通过D4的1..16 atoms门，不能接纳空数组。

P、L_*只是规范中的结构型缩写，不是新的TypeSpec kind、schema identity或D3 alias。Equality/hash/group/distinct按本Profile的已有递归结构规则；没有自然排序。Query可以显式读取这些结构成员，View只呈现值；点击定位或将值重新构造成动作Locator必须走原D3 resolver及D7当前授权、确切owner/kind/revision检查，投影本身不证明位置仍有效。原作者可保存历史位置，Query不得将其重绑latest或重新签发为当前ActionEvidence。

已获权作者内容中的普通typed Ref/Locator原值与其指向对象的当前状态是不同读取对象。对当前主体完整获权读取的Field/source，只按原D3/D4 closed decoder验证并保留其中完整作者Ref、历史Locator、revision token及坐标原值；此投影不查询目标Workspace、existence、lifecycle、title、content或当前位置，也不声称引用当前有效。授权来自该完整作者事实/源的读取权；Field/source读取不蕴含目标state/content资格。此规则适用于普通非relation Ref值及D4现有provenance atoms，包括跨Workspace Node provenance和合法owner-local Resource provenance；AnnotationRef仅在原schema已有的普通值位置适用，不新增Annotation provenance atom。D4原domain/locality不变。

relation实际endpoint的domain/state、关系可见性、incidence、graph/遍历/join及点击解析仍先通过目标当前state/content和适用locator授权，再读取目标。D4语义角色区分actual relation endpoint与附属作者provenance，不能仅因JSON内出现node_ref就混同。renderer不得附加自动resolved标志、目标当前标题/缩略图/状态或位置；显示作者原值不授遍历权。依赖目标状态的Query/Action显式列入原潜在观察范围、真实依赖与CAS，不能使用只保留作者bytes的窄证明。

同一规则贯穿Query read、Field selection/evidence、preview、effects和重放。完整Field/source失权按原当前门阻止披露；只有被引用目标失权不改变仍获权的作者原值，但禁止该目标后续解析。任何阶段不得删Locator、改none、丢atom或改transform inputIndex。此处不会开放Query内部occurrence/source-lineage。

### 5.2 完整D4 constructor与qualifier桥

桥接输入是D4验证成功的完整Entry/Registry binding，包含source owner和原raw bytes；禁止直接将任意JSON按下表剥壳。D4 availability、类型、range、cardinality、关系、locality、非空、边界和贡献验证先行；映射只决定Query类型和编码，不复制或取消这些输入约束。每个mapped value继承所消费的Field/alias/QualifierSet、namespace owner、RegistryBinding、calendar comparator、tzdb、unit、code、external scheme和所有递归ref/Locator的真实正负依赖。source/权限依赖保持到发布、cache、Action复验。

| D4 constructor/schema | Query TypeSpec与V的唯一映射 |
|---|---|
| text | exact→text；nfc-for-compare→text comparison=nfc；V为text原文，nonEmpty仅输入验证 |
| boolean | bool；V为原Boolean |
| integer | integer；V为原canonical string；包括有界/排除值schema，不降成int64 |
| decimal | decimal；V为原canonical string，包括有界schema |
| semantic_code | semantic_code，scope为下段ResolvedCodeScope；V为原code |
| calendar_date / zoned_instant / quantity | 同名Query primitive；V逐字保留D4完整closed value wire，Registry信息只作绑定证据不偷偷增wire成员 |
| date_range / instant_range | object{end:optional(point),start:optional(point)}；逐bound对应，unbounded→none；不是缺整个range |
| node_ref / resource_ref / annotation_ref | 同名Query primitive；V恰为原nodeRef/resourceRef/annotationRef成员的完整D3 Ref |
| external_identifier | object{scheme:text,value:text}；V为两个原成员，不增加内部SourceBinding |
| object | 每个ObjectMember递归bridge；required原型，optional包装一次；V取members对象，缺optional填none，存在填some |
| union | variants按原variant token和顺序递归bridge；V为{variant,value:bridge(original.value)} |
| bounded_set / bounded_sequence | list，maximum取原schema maximum；逐item bridge，保持原items序与全部次数；不能按Query NFC equality再删项 |
| alias_ref（schema-only） | 先依D4有界无环展开，再递归bridge；alias身份/完整定义版本保留在依赖，不创造公开值kind |

ResolvedCodeScope是D7用于类型相等的closed描述，明确区分以下四种：`{kind:"field_local",fieldId,codes}`、`{kind:"namespace",namespaceId}`、`{kind:"contribution_set",codes}`逐字携带原CodeScope的完整数据；以及仅用于D4 relation qualifier status的`{kind:"registry"}`。codes保留原sorted unique域。registry表示当前Registry所有已verified的完整SemanticCodeId，绝非任意text；status的owner/contribution preflight按D4执行。类型本身不授任何贡献可用性；Registry binding由结果依赖保存。原文所称“D4 resolved CodeScope”在Query wire唯一指本段，不让实现自行添加scope成员。

Qualifier bridge按完整QualifierSet逐member构造closed Q，所有TypeSpec members按UTF8名排序：fact={selection:Optional<text>,validity:Optional<union{date:dateRange,instant:instantRange}>}；event_assertion={confidence:Optional<decimal>,eventTime:union{date:calendar_date,instant:zoned_instant},selection:Optional<text>}；observation={confidence:Optional<decimal>,observedAt:zoned_instant,selection:Optional<text>}；relation={status:Optional<semantic_code scope registry>,validity:Optional<union{date:dateRange,instant:instantRange}>}。selection只接受原ordinary/preferred/deprecated，confidence原0..1，时间和范围必须先通过对应D4 wire gate。qualifier里的raw selection投影为text而非虚构Field-local code。Entry.note缺省→Optional.none，合法present非空text→some原text；present空串仍按D4拒绝，recurrence replacement内可空note是另一层wire；provenance按§5.1完整映射。

### 5.3 数量与日期basis的显式承接

quantity的basis恰为当前已验证unit contribution的完整dimensionId和unitId；same dimension不同unit仍不兼容。`quantityBasis`与`dateBasis`只访问输入decoded value及其已经绑定的验证描述，不发起source、网络或新Registry读取。quantityBasis相等可分组，dateBasis相等是动态日期比较的完整前提；当前调用仅一个RegistryBinding，因此同basis对应一个已验证comparator。跨binding结果不能直接混进一次执行。

aggregate sum/avg扩充为同型quantity→Optional<quantity>：先扫描全部present值证明完整basis相等；否则type_mismatch，不按first value猜通用合法性。sum对magnitude做精确总和，avg按原scale/rounding一次舍入；输出原unitId，dimension来自同绑定。空present输出none。min/max同样先验证basis后按magnitude取值。显式quantityMagnitude→numeric后求和仍合法，但其结果是有意去单位的纯数值，unitLabel只是一段说明文字，不能将其升级成单位已验证的quantity。无需换算规则或新作者Field。

DerivedDuration桥仅输出以下closed union，variants按UTF8顺序：calendar_units→object{calendarId:text,calendarVersion:text,precision:text,units:integer}（D3Integer转canonical string）；exact_seconds→object{seconds:decimal}；unavailable→object{reason:text}且reason唯一open_range。派生结果保持D4原算法、精度与不可用边界，不是可写作者TypedValue；其它provider/schema失败为Query definition_unavailable而非unavailable/open_range。合法D4缺省与Query Optional的意义仍分开。
