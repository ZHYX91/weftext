---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：6ac539c2-40f1-46e6-92ac-e24bfaf8b113。

候选状态：D9 文件权威协调的 PL-IR-01 修复 consumer，未独立接受、未激活、未实施。本文仍保留固定 S 的能力/限制/历史证据边界，并消费当前 D3/D6/D7 单决议与稳定生产地址/current Observation 作者修复；该新 exact candidate 仍待独立复核，不自行关闭 P1，其它未接受 cross-owner producer 仍各自门控。

# D9 模板

固定 S 的范围与证据说明：区分产生 fresh Node 的 Node Template 与产生外部文件的 Office 模板。二者均不持续控制已产生结果，不执行Query/CEL/脚本，不创建模板专用内容身份。

## 1. 普通文本占位符的唯一语法

Office profile=`weftext.office-template/1`。输入 DOCX、ODT、XLSX、ODS；具体格式/操作 capability 分开，未验收不得宣布可用。以下语法在所有格式含义相同，不依赖内容控件、书签、命名区域、Excel Table、Office数据源、宏或插件。

```text
{{ meta.title }}
{{ header.description }}
{{ field.user/report-code }}
{{ content.body }}
{{ view.bibliography }}
{{ ↓ data.orders.product }}
{{ → data.months.amount }}
```

普通token精确为 `{{ ` + path + ` }}`，内侧各一个ASCII space，不允许tab/NBSP/额外空格/换行。重复token精确为 `{{ ↓ `或`{{ → ` + data-path + ` }}`。path内无空格/运算符/表达式/默认值/函数。每token UTF-8≤1024 bytes，每模板≤10000 tokens，全部结构和生成输出另受统一budget。

扫描按逻辑文本scalar从左到右一次：`{{{{`生成literal `{{`；`}}}}`生成literal `}}`；其余`{{`必须开始完整合法token，其余`}}`是语法错误；单个brace普通字符。例 `{{{{ meta.title }}}}` 输出字面 `{{ meta.title }}`。替换值绝不再次扫描；值内出现模板语法只是文字，不能注入循环或其他字段。转义在token识别之前消费，不能跨容器组成escape。全模板无效token即失败，不留下未替换标记仍报告成功；转义产生的字面标记是明确例外。

path first segment闭集：`meta|header|node|field|content|view|asset|data`。旧裸名称、`attr`、`record`、大小写别名或宽松空格不接受。UI提供插入/检查命令；语言和软件版本不改变token语义。用户可将token本身设为字体/颜色等样式。

## 2. 绑定域、值与缺失

所有绑定只能来自本次显式选择且已获原读取资格的输入；模板文本、拥有某个Document pin或缺少某个slot都不能增加选择范围。正文和书目按Workers and Export的ExportContentSelection/1判断，不能以“当前Node”或模板没有content.body推导隐式Document。query_rows、node_collection、native_table和query_json首代均不选择整份body或bibliography；document分支允许明确选择正文、书目、两者或都不选，但必须显示本次范围。用户明确要求的输入无资格则失败，不能将其改为未请求后继续。

field.FULL_FIELD_ID由此次显式选择的完整Field pin唯一确定owner；若相同path有多个owner候选，报ambiguous_binding，不从Query首行或模板位置推定。node.id同样须显式选定且有原Envelope读取资格的NodeRef。meta/header路径需要实际D2 Document来源及相应原读取资格；窄Field或Query结果不隐含这些路径。content.body和view.bibliography只在各自selection非null时存在，未选择却出现对应token即mapping_required，不读取额外输入，也不当成none。asset和data只消费已选Resource/dataset。

| 路径 | 精确绑定 |
|---|---|
| meta.title / meta.subtitle | D2完整DocumentMetadata title及显式subtitle；冒号不拆副标题。title必有，subtitle可缺。 |
| header.NAME | NAME符合D2 `[a-z][a-z0-9-]{0,63}`且不是wf-*或subtitle；本次source中明确存在的raw header text。未知名称是错误，值为空字符串合法。不展开属性/环境/URI。 |
| node.id | 获准读取的完整NodeRef的nodeId显示文本，不作输出identity或重新导入授权。 |
| field.FULL_FIELD_ID | `field.`之后全部字符为D4完整FieldId（namespace/path），只绑定该当前Node的完整已授权Field；不按label、中文名称或普通header猜。 |
| content.body | D2实际body投影，不含title/subtitle/control/carrier。只编译D2现有种类；正文heading H1–H5。 |
| view.bibliography | Core对准确citation集合、权限和固定bibliography渲染profile生成的块序列；未编译/不可用/引用目标不可读则失败，不给空列表。 |
| asset.NAME | NAME为ASCII `[a-z][a-z0-9_]{0,63}`，来自本次显式选择并冻结的Resource输入；不接path或URL。 |
| data.SET.COLUMN | SET/COLUMN各为ASCII `[a-z][a-z0-9_]{0,63}`，指本次准确typed dataset schema中的显式名称；无ambient dataset。 |

meta只开放title/subtitle。旧原型meta作者/修订日期/关键词等无D2固定解析意义时不作为自动字段；可以直接header路径，或显式Core派生dataset字段。node.name/path不开放，因为D6布局与显示label不能被当作稳定内容字段。view.toc/index/list_of_figures/list_of_tables/endnotes本代未冻结渲染profile，遇到即unsupported_profile，不能留空冒充成功。

Field只有零或一occurrence且为可渲染scalar时可在标量位置消费：零条目为明确none，按下述missing policy；一条value按D4型渲染；多条报ambiguous_binding，不取first/preferred/concat，也不丢qualifier/note而声称导出完整Field。复杂object/union/refs/quantity/bounded values只能显式Core投影为typed dataset；默认无字符串化fallback。D7查询负责具名的计算/选择，不由模板执行。

RenderSnapshot中每个binding是 `{kind:"none"}`、`{kind:"text",text}`、`{kind:"boolean",value}`、`{kind:"integer"|"decimal",value:<D4 canonical string>}`、`{kind:"date",value:<ISO完整日期>}`、`{kind:"instant",value:<D4完整zoned_instant>}`、`{kind:"blocks",blocks:<已编译ExportBlocks>}`或`{kind:"resource",resource:<已授权Resource pin>}`。这些是导出值，不是新D4/null值；unknown binding、unreadable、unavailable schema、invalid source与none完全分开。metadata/Field/resource首先完整授权，失败不以none遮蔽后继续导出。snapshot类型由Core推导，不接受worker猜测。

none在普通标量token默认为阻断；用户可在导出计划为该精确模板path明确 `missingPolicy=empty`，输出零字符并在ExportLossReport记录数据缺失表示损失，准确定位该binding、实际模板位置及其原始输入。未知path始终错误；empty policy不能消除类型/权限/字段缺失错误。重复dataset的零行按§5，不由missing policy控制。空text也是零字符但完整projection与ExportLossReport中与none区分；PublicationReceipt关联原报告，不向作者receipt添加字段。导出值含CR/LF：DOCX/ODT text位置转明确line-break对象、保持logical字符；XLSX/ODS内text保留换行；非法XML scalar（如NUL）整体拒绝，不替换。整数/decimal渲染精确canonical文本，不用host locale；布尔固定true/false；日期/instant文本采用其规范lexeme，目标数字型选项另按Export合同。

dataset来源显式为 `native_table`、`node_collection`或`query_rows`：native table按原column ordinal投影无FieldId，并在snapshot schema显式命名；Node Collection来自完整去重NodeRef结果；Query rows保留bag、列型和明确order。模板中的data.SET.COLUMN保存放置/column binding，完整FieldId直接写在field token，普通Node不增加export-only schema。运行时SET→已完成输入snapshot的选择属于本次导出计划，不持久写回Node；不能按集合标题/路径重新寻找对象。无序rows必须由用户在Core显式形成稳定sort或拒绝重复导出，不能用rowHandle/内存顺序排序。rowHandle不写入文件，重复同值行不得dedupe。

普通 data.SET.COLUMN 只允许出现在由同一 SET 的方向 token 建立的 repeat band 内，并读取该复制 item；方向 token 本身也读取该 item。repeat band 外的普通 data token 一律 template_invalid，即使 dataset 恰好一行也不特判。band 内另一个 SET、未知 column、blocks/resource 类型一律 template_invalid；缺读权先 not_visible；存在列中的 none 才适用明确 missingPolicy。静态 scalar token 不产生 item scope；零行仍删除整个 band，不执行不存在 item 的 scalar 缺值替换。例如单独段落“联系人：{{ data.contacts.name }}”拒绝；contacts 的同一重复行中方向 token 与普通 data.contacts.email 合法，data.other.email 拒绝。禁止 first/concat/隐式跨 SET join。

date binding 仅从 D4 calendar_date 的 calendarId=calendar/iso8601、calendarVersion="1"、precision=day 精确投影；其它日历或精度必须显式转换 profile，首代未开放，不能补日。instant binding保留完整 zoned_instant 的 instant/timeZone/tzdbVersion。将其渲染为 instant 原词法时，省略时区上下文属于 date_semantics loss；不得把显示时区误当原 offset，或按宿主精度截断。

## 3. 逻辑容器与格式继承

DOCX普通paragraph（body、table cell、header/footer）内相邻文本run可重建一个逻辑文本序列；ODT对应`text:p`/`text:h`、span、显式space/tab/line-break；XLSX/ODS每一个cell为容器。不能跨paragraph/cell/不同story拼token。tracked insertion/deletion、field codes、hyperlink boundary、drawing/textbox、脚注和未知inline element默认不纳入可模板区，命中或结构干涉即template_invalid；不能只漏扫然后称没有token。XML entity只按标准安全parser解码一次。

一个token可跨run/span，但token范围必须具有完全相同的有效字符格式。有效格式由本profile支持的style/default/inheritance/direct properties解析；无法证明等同、继承循环或不支持的format属性影响token则拒绝。禁止“取第一个run样式”吞掉混合样式。合法token替换在首text span写入结果，删除其余覆盖字符，保留token之外所有文字与格式。scalar继承token自身字符格式与所在paragraph/cell格式；多个同binding出现分别继承。

一个asset token必须独占普通paragraph（DOCX/ODT）或单独cell（XLSX/ODS）；第一代仅DOCX/ODT支持图片asset，sheets遇到resource是unsupported_profile。动态图片尺寸由显式导出plan给定且冻结；本代没有可见尺寸directive或隐藏控件语法；无显式尺寸仅可用经Import IR的image_physical_size/1验证的源声明显示尺寸；缺密度则mapping_required，不能用72/96 DPI。所有尺寸受输出边界，不能溢出版面后静默裁剪。alt/caption来自显式snapshot，缺alt在预览中提示。

ExportPlan 的 imageSizes 是按 Resource RefKey 排序唯一的 {resource:EntityTarget,widthMicrometres,heightMicrometres} 数组；两尺寸为正Counter，resource必须准确匹配此次已授权resource pin，不能依文件名/label/摘要匹配。每个所选Resource的动态放置统一使用其明确尺寸；需要同一Resource多种尺寸的本代模板返回unsupported_profile，不按位置猜。空数组表示逐Resource要求可验证物理尺寸。该规则同样覆盖生成的正文图片和asset；template本身已有的静态图片仅在其具名decoder能验证既有尺寸/布局并保持时允许，不能丢掉布局后靠密度重猜。

元数据缺失且有显式尺寸时可以布局，但它是用户layout policy，不是源物理事实；密度/方向非法或互相冲突仍拒绝。有源声明绝对尺寸时，明确尺寸与其按profile量化的宽高不同则列layout requires_choice；相同不重复列layout（非exact量化仍列precision）。缺绝对密度时，显式宽高比与image_physical_size/1确定的正向显示比例不同才列layout requires_choice；同显示比例的纯尺寸选择为notice。不能忽略unit=0的非square比例。物理量化非exact产生precision loss。全部报告使用ExportLossReport的resource input与实际binding/block位置，并进入固定Plan/bytes确认。region几何仍与尺寸政策无关。


blocks token必须独占DOCX/ODT主文档body普通paragraph，token之外允许零字符，不允许空白、书签、section properties、列表编号或其他inline对象；不放header/footer/table cell、text box或tracked change中。占位段落在输出副本替换为完整块序列，无块则移除该paragraph。模板原件不改。

## 4. body、bibliography 与标题样式

`ExportBlocks`只含从已valid D2明确编译的paragraph/heading/list/table/literal/quote/break/resource和bibliography；saved query/view定义不当作可执行模板，处理模式由Export合同明确；引用目标必须读权。没有D2 bold/math/footnote结构就不承诺将其作为富格式导出。

body与bibliography的slot语法始终检查：content.body和view.bibliography在全模板各最多一个；是否需要内容、是否发生省略，另由明确selection及真实源决定，不能由slot缺席推断。

- bodyInput=null：本次没有请求整份正文，scope明示“正文未请求”；不能产生body omitted、正文为空或已经完整枚举的事实。native_table即使因原读取合同持有完整Document pin，也只消费所选table，不因此选择其余body。没有body slot是合法的；有body token则mapping_required。
- bodyInput非null：Core先以完整资格校验并编译该准确Document body。有body slot时输出全部已选择body，书目placement依下一条单独处理；没有slot时，实际非空body的省略必须有source_range/blocks等可复算来源及requires_choice。已证明为空的body无丢失内容，可记notice，不能以未读取/未编译代替空。选择的body被省略但书目另行输出时，报告明确书目已保留及其余实际省略范围，不声称整份正文语义都消失。
- bibliographyInput=null：不运行书目view、不读取仅为书目计算而新增的目标或声称条目为空。若本次已选择并实际输出的body中存在已知bibliography_placement，该已知placement未生成view的变化须以原source_range报告omission requires_choice；这不要求枚举未请求的条目。没有选择body则不为其它输入虚构placement或书目省略。
- bibliographyInput非null：从明确选定Document的完整citation集合按固定profile编译并验证全部目标资格；与bodyInput均非null时必须指向同一Document。实际非空view必须恰有一个生效位置：独立view.bibliography slot，或实际输出body中的唯一原placement。独立slot优先，取代原placement时以真实来源和该slot记录relocation；body没有输出时，其内部placement不是可用位置。非空view无生效位置即mapping_required，不默默省略。已支持且证明实际零条目的view可以零blocks并移除已存在slot；无slot时为已知空结果的notice，不伪造丢失条目。未编译/无资格不等于零。

选定正文/书目所依赖的Document中，本profile至多支持一个bibliography_placement，多个则template_invalid，不任取一个；未请求该Document时不为此另行读取。title/subtitle只有其显式binding和token才输出；body不再重复它们。所有已选择输入的实际转变、缺值和已知省略完整进入同一ExportLossReport；未请求类别仅属于scope说明。

本代标题：AsciiDoc `==`→H1、`===`→H2、`====`→H3、`=====`→H4、`======`→H5；D2没有H6–H9，导入源level6–9必须heading_depth损失/flatten或拒绝，不能让renderer新增D2语法。

模板可用普通可见文字标记样式样板paragraph：`{{ style.heading.1 }}`…`{{ style.heading.5 }}`、`{{ style.paragraph }}`、`{{ style.literal }}`、`{{ style.quote }}`。它们是独立的封闭directive，不是§2值namespace，必须独占主body普通paragraph、每role至多一个、全部在输出中移除。样板paragraph提供实际style ID引用，不能按本地化显示名寻找“标题1”。样式规则只应用于本次实际生成的blocks，不要求纯scalar/dataset模板具有body slot。实际使用的role缺样板时采用固定版本默认style bundle；普通paragraph是明确例外：先用style.paragraph样板，否则用该批blocks实际插入slot的paragraph style（正文用body slot，独立书目用bibliography slot），该slot也无style才用固定默认。没有生成blocks就不为body/bibliography选择回退style。只向输出副本注入实际使用的缺失定义；既有同ID而定义不同就失败，不覆盖、不猜。

DOCX：每Hn输出paragraph的`w:pStyle`引用选定style，显式`w:outlineLvl=n-1`，n=1..5；要求paragraph style类型正确、其有效outline不与n冲突。仅有名字“Heading1”而语义不符不被当标题。ODT：输出`text:h text:outline-level=n`并引用选定paragraph style；目标style不得含冲突默认outline。正文`text:p`不因styled name被误升heading。样板style链循环/无法解析/role冲突拒绝。输出body普通paragraph继承选定paragraph style；literal/quote用各role；lists/table/image使用同一固定bundle定义的结构格式，后续自定义role只能新profile，不做隐式猜测。默认bundle版本、注入/复用style ID、实际level映射进入receipt。

这套可见style样板仍是普通文字和普通样式，Word/WPS/LibreOffice可编辑。所有实际interop、分页、字体fallback、RTL/shaping仍需逐版本渲染验收；样板语法稳定不等于各软件视觉完全相同。

默认 style bundle 是 renderer route 的必需不可变资产，注册时记录 bundleId、version、每个实际 style ID、完整定义 bytes、所用字体与依赖闭包；这些进入 routeRevision、ExportPlan、预览和 PublicationReceipt。相同 routeRevision 不得换 bundle、字体版本或定义，宿主 Office 默认值不是 fallback。最低语义具备 paragraph/literal/quote 与 H1–H5 的实际 style IDs，标题 outline 按本节强制。别名或本地化名称相同不能证明定义相等。具体视觉值由已验收资产固定，未提供完整资产或不能验证有效样式的 route unavailable。

## 5. 重复区、方向与碰撞

重复token只允许`data.SET.COLUMN`，并占满cell的逻辑文本；普通scalar可以在同一prototype band其他cell，但不可在重复cell与周边text混写。本profile只重复 **一个完整表格行或一个完整列**，不是矩形中的半条band。DOCX/ODT以所在table为容器，XLSX/ODS以整个sheet已占用矩形为容器。一个容器最多一个重复band及一个SET；↓选择包含重复token的唯一row，→选择唯一column，同一band全部重复token方向/SET必须相同。不同table/sheet可各有一个。嵌套表格重复、交叉band、同容器多band直接拒绝，避免不确定先后位移。

↓按snapshot顺序为每数据项复制prototype row，column不变；→复制prototype column，row不变。方向指逻辑row/column index递增，RTL视图不把→改成反向数据。除重复cell读取对应item列外，band内静态cell和普通scalar按其原值复制；数据0项删除band。DOCX/ODT table若删后0row或0column则移除整个空table；XLSX/ODS保持sheet（可以为空）。这是明确empty-region行为，预览列出。

坐标变换：repeat band index b、items n；axis index i<b不变，i=b复制为b..b+n-1，i>b移为i+n-1；n=0时删除b且后续-1。另一axis保持。全部对象先分类再计算目标footprint；预先验证row/column limits、累计cells、输出大小以及碰撞，之后生成副本，不边写边发现越界。

复制cell格式、border、行高/列宽及全部已支持presentation properties；merges只有完全位于band内才允许复制；跨band、covered cell含绑定、输出merge重叠、template本身ragged DOCX table、unmodelled gridSpan/vertical merge组合都拒绝。第一代 **存在任何公式的重复容器整体拒绝**，不平移公式引用，不依赖Office打开重算；模板中的其他公式也按下一段规则。图片、图表、drawing anchors、named ranges、Excel Tables、validation/conditional format/print range等在重复容器出现且涉及坐标时本profile拒绝，不声称自动移动它们。选择较小能力并非静默删对象。

第一代整个Office模板禁止宏、可执行field、公式、外部关系/数据源、embedded OLE、script、远程图片和数字签名。普通静态超链接也必须经过显式安全export link profile；本代templates不开放外部超链接，需用户另存安全模板。禁止项返回unsafe_input/template_invalid，不静默删除后生成另一含义文件。普通输入文件转换和已签名原件作为不执行附件保留仍独立可用；输出不继承原件签名有效性。

## 6. Node Template 的一次性构造

D2 Template Node 是 `:wf-kind: template`，不能同时有`tasks/task`。targetCoreKind固定ordinary，targetFacets可以含tasks/task并须D4全部admission。Template分类不从文件名、子目录或Office Resource推断。

本代Node Template profile=`weftext.node-template/2`。Template的合法Document是默认content source，不在其中执行Office占位符（D2不允许任意attribute interpolation）。参数与target plan作为该Template显式选择的一份普通JSON Resource；它是Resource bytes权威，不是hidden metadata或依名字自动发现的文件。请求必须指定 Template Node 与 recipe Resource 的真实 SourceVersionRef/1，二者 owner 一致，当前观察、完整生产版本和读取资格均精确有效。recipe本身的解析不授write。

首代仅允许 Template、recipe、source closure 与 destination 在同一 bound Workspace。外部 Template 先经原 D3 明确 copy/import。Node Template 走本节独立 construction adapter，不伪装成 page/flow/workbook IR；与文件导入共享完整分析、loss确认、D7准备和原D3提交。入口 d9_template_analyze exact {wireVersion:2,kind:"d9_template_analyze",workspaceRef,commitDomain,expectedFrontier,construction,budget}，commitDomain、expectedFrontier、budget 分别使用 D6 CommitDomain/2、Frontier/2、BudgetBinding/1，construction 为下文 TemplateConstruct/2；成功为主文 wire2 的 d9_import_analysis，但 analysis 的已存 source kind 明确是 node_template，不允许把它当 irToken。后续 choose/prepare/next/state 使用主文原形状及完整约束。

`TemplateRecipe/2` exact `{format:"weftext.node-template",version:2,parameters,nodes}`。

- parameter exact `{name,type,required}`；name为§2 NAME；type=`text|boolean|integer|decimal|iso_date`；required Boolean，无隐式默认/表达式。未提供optional参数只保持默认source，无none字段写入。
- node exact {source:TemplateSourceAddress/2,parentIndex,targetFacets,bindings}；TemplateSourceAddress/2 exact {ref:NodeRef,sourceVersion:<managed SourceVersion/2>}，ref 与 sourceVersion.entityRef 逐字相等。source 必须是所选 Template 的 live 子树内合法 Template Node 的准确 Document。nodes[0] 必须恰与请求 template 所选完整生产版本/NodeRef 相等且 parentIndex=null；其余 parentIndex 是更早 index。每个完整 source NodeRef 只能出现一次，不因 revision 不同允许重复；完整 live placement 范围与选择资格进入依赖。禁止 cycle、多个 root、重复源或引用未声明 parent。recipe index 固定输出位置，不作身份。
- binding exact `{parameter,slot}`。slot closed `{kind:"title"}`、`{kind:"body_text",locator:<当前D3 DocumentRangeLocator>,expectedText}`、`{kind:"field_append",fieldId}`。title仅text、每node至多一个；body_text要求范围只包含同paragraph的inert文本且准确expectedText，不跨结构/header/carrier/宏，replacement仍须inert且完整D2-valid；field_append用已定义scalar FieldId与参数型精确匹配，每追加一项由Core产生同owner/Field内fresh occurrenceKey，保留所有其他Entries。

recipe参数名唯一、binding目标不得重叠；同Field多个append需schema repeatability/cardinality准许。请求参数exact `{name,value:<相应D4 TypedValue>}`数组，未知/重复/缺必需均拒绝。不按字符串进行全source replace，不允许CEL/JS/环境/URI/循环。source copy中的wf-kind被Core显式移除，targetFacets与默认/参数facts作为完整proposed-state统一检查，冲突失败，无last-win。

所选 source NodeRef 到本次 fresh Node subject 一一对应；所选 ResourceRef 到其所选 owner 的 fresh Resource subject 一一对应，不能按相同 bytes 合并。遍历每一已知真实 typed slot：指向所选 Node（含 self/cross-template）的引用固定映到唯一 fresh subject；指向未选 Node 的引用仅在 externalNodePolicy=preserve_current 且同 Workspace 当前 live/read 合法时保持 exact，否则拒绝；owner-local Resource slot 必须映到其本次 fresh owner 下明确选中的对应 Resource，缺资源选择即 mapping_required，不能保留旧 owner。原 D3 Result/9 的 N/E/C 与逐 slot plan完整承载，不在 ordinary import 伪造 typed preimage。指向会改变的模板 Document/Resource 的 Locator 若不能按已有 D3 规则唯一验证最终位置则 unsupported_profile，不靠旧坐标或相似文字 reanchor。未知 schema 无法枚举全部 typed 根同样拒绝。第一代不复制 Annotation，检测到即显式 annotations omission loss；不复制 recipe Resource，除非用户明确选它作为普通附件。SavedQuery/View/DynamicBlock 定义均 unsupported_profile，不能当无引用 opaque payload 搬运。全部 D2/D4 admission、fresh occurrence/fact 与 reciprocal companion 仍按原 C/Result9 规则，不由 recipe 自造身份。

完整模板实例为一个 atomic coupling group，含全部 descendants，最多1000新Nodes并受更窄预算。普通 parent 入口唯一采用 D7 d3_operation + D3 import_new/ordinary_format；不用 create compound 替代。Core 建立 ConversionInput 容器，part0 保存完整 construction request、实际来源版本/bytes目录、recipe、参数、资源选择、loss、source→symbolic subject对应与分组；完整拟议源另由原D3 result payload/Result9绑定，原 bytes 各有真实 part 绑定；不伪造 import identityMap。collection 入口仅支持单Node、无新增Resource、无任何需映到本次 fresh subject 的 source/body/carrier引用；否则 unsupported_profile。该分支产生一份完整具体 source，走原 D7 collection_create/D3 create_node，保留原完整 CollectionCall、parentPolicy、requireMembership、sort/take 与正负依赖；其 D3 plan只有合法 create freshSubjects/result payload，不带 source_artifact。两分支的完整构造来源额外按下文受管 PreparedActionBinding/3 固定，而非扩展 ActionSpec 或 D3 request。

模板输出交D8时不自动写dirty Draft，不伪造human来源；用户接收后需原full源资格/预览/确认。模板更新不会自动改实例或复制后的Office输出；重新实例化是新的明确创建。

## 6a. 精确构造请求、准备绑定与原回执

TemplateConstruct/2 exact {version:2,template,recipe,parameters,resources,destination,externalNodePolicy}。template/recipe 直接使用 D6 SourceVersionRef/1（分别 Node、由 template 拥有的 Resource），不另包同义 target；parameters 是前文 {name,value} 数组，按 name 排序且唯一；resources 是所选 Resource SourceVersionRef/1 按完整 D3 RefKey 排序唯一数组，owner必须在recipe nodes选择内；externalNodePolicy仅 reject/preserve_current。destination恰为 {kind:"parent",parent:NodeRef,ordinal:Counter} 或 {kind:"collection",collection:CollectionCall,parentPolicy:ParentPolicy,ordinal:Counter,requireMembership:Boolean}，全部复用原D7类型。目标解析与资格不fallback。loss接受保存在analysis中，不以用户参数假装授权。

参数名 iso_date 只是配方选项，实际值必须采用完整 D4 日历日期型 calendar_date(calendar/iso8601,"1",day,lexeme)，并由原解码器验证真实日期；它没有建立一个新的日期种类。整数和小数同样必须精确匹配 D4 类型，不能从 Office 文字推断。正文和标题的插入都按预期 D2 语法树构造；独立完整解析后，结构、惰性文字和类型化槽必须逐项相等，仅能通过 D2 语法检查还不够。无法表达时返回 unrepresentable，不能生成意外标题、宏、载体或引用。

本候选使用 D7《准备动作绑定》正文 §5 定义的 PreparedActionBinding/3。配套修订篇只说明 D9 如何消费该记录，不另建一套结构。constructionInput 在非模板路径为空，在模板路径为下述完整受管对象；调用方不能追加任意 JSON，也不能借此建立新的提交入口。

TemplateConstructionInput/2 的精确结构为 `{kind:"node_template_construction",version:2,construction,inputPins,omittedAnnotations,lossReport,lossChoices,sourceSubjectBindings}`。

- construction 是上述完整 TemplateConstruct/2；recipe从准确 recipe pin 独立strict解析，不能接受客户端自报已授权recipe AST。
- inputPins exact 元素 {sourceVersion:SourceVersion/2,observation:SourceObservation/1,payloadKind,pin:PinRef/2}。payloadKind 仅 exact_source_document/resource_bytes，并与 pin.payloadKind、实体 kind 精确相符。sourceVersion 逐字等于 observation.sourceVersion，完整 Ref 相等，observation.observerDomain 等于原请求 CommitDomain。pin 选择实际不可变完整字节；受保护记录的完整生产版本、实体、长度、摘要及当前读取绑定均与本项相等，不把相同摘要当同来源。Document bytes 必须 strict UTF-8 且等于准确源，不是重排 header 后的等价文本；Resource bytes 原样。该内部耐久 pin 不是 ByteHandle/TTL 运输句柄或公共 base64。数组按完整 RefKey 排序唯一，恰覆盖实际读取的 recipe、所有 node 源和所有选中 Resource；不得缺少、增入未用作者源或只存摘要。Template 无内容变化也保存真实生产版本及当前观察。
- omittedAnnotations 是 Core 在同 cut 从所选 Template owners 完整枚举的 Annotation D9EntityVersionAddress/2 数组，按原RefKey排序唯一；本profile全部omit，没有用户自报遗漏清单。当前state/annotation读取资格及完整正负owner-annotation范围依赖必须先证明，无法证明整体not_visible/unavailable，不把不可见当空。它只固定明确不复制的对象及版本，不复制其body，不产生fresh subject；loss覆盖全部遗漏。
- lossReport保存选择前完整固定提案的初始LossReport（可含requires_choice，不含blocking），lossChoices逐项覆盖其全部requires_choice且值都是accept_loss；Core重算效果/目录与最初报告一致才可prepare。对外analysis可将已接受项显示为notice，但耐久构造证据不能删去初始severity/allowedChoices。Template loss的locations使用下文TemplateLossLocation，不把作者Ref伪装成外部IR sourceIndex。sourceSubjectBindings元素 exact {sourceRef,subject}，sourceRef为所选NodeRef或ResourceRef，subject为原D3 new_result_subject，同kind、Resource owner机械对应。按RefKey排序，源与目标各唯一且与recipe/资源选择、request freshSubjects、proposedInputs精确闭合；recipe文件若未被复制不进入本数组。

TemplateLossLocation是本构造域的只读证据地址，恰为 {kind:"template_source",sourceVersion:SourceVersion/2}（Node/Resource，必须在inputPins中）或 {kind:"template_annotation",target:D9EntityVersionAddress/2}（Annotation，必须在omittedAnnotations中）。它不是D3 Locator、author slot或写目标；模板损失只用本union，文件/worker IR不得提交这些作者地址。其余LossReport成员及accept/reject语义复用Import IR and Mapping，locations非空；每项实际内容转变须能从相应完整pin或已授权omission目录复算。

Core在分析和准备时从上述完整pins重做deterministic recipe验证与编译，证明结果等于原Action的source（collection分支）或原完整D3 Result9及artifact manifest（parent分支）。每一来源revision、live placement、recipe bytes、参数、外部引用、资源选择、Registry、loss和全部目标/Query资格进入完整正负依赖，unseen请求在同一原planning CAS复验。模板或recipe改版而生成文字恰好相同仍使旧准备依赖失败，不把输出摘要相同当来源相同。改参数、选择或loss产生新analysis/token/请求，不能修改旧record；unknown/planned先恢复原请求。

构造记录只解释准备依据，没有独立身份结果。两个首代分支的原D3 receipt identityMap都必须为空；完整 resultAllocations、initialPlacements、created reference证据按原D3返回。需要显示“模板S产生实例S′”时，只对已保存sourceSubjectBindings与原receipt.resultAllocations按subject作完整唯一join，得到可重算视图；不得创建第二权威映射、补receipt字段或用allocated数组位置猜。receipt缺subject/错owner/重复mapping拒绝整份派生显示。真正copy/fork/identity-bearing import仍只按其原identityMap，不受此解释变更。

集合反例：排序take1只留下旧Node时，requireMembership=true拒绝整个create，不公布fresh identity；false只承诺创建。编译成功、analysis成功、原D7 prepared均不是创建成功。lost receipt恢复同OperationId/request，重复不会重新编译或再创建。

## 6b. 固定生产来源、当前准入与原配方恢复

Recipe/2 的 source 是 exact 固定 managed production version，永不跟随 latest；只保存完整 sealed SourceVersion/2，不保存 current sourceToken。纯同步可以保留该生产地址。每次新实例化在先验 disclosure/exact-cut 与显式 Template/recipe/resource SourceVersionRef 检查后，Core 为每个 recipe source 建立 current Observation 并要求 Observation.sourceVersion 与 recipe 固定版本逐字相等。每个 `body_text` DocumentRangeLocator 随后运行 D3 稳定地址算法：验证 canonical revision binding 与 original sealed-outbox 关联，解析到同一生产版本，验证接收端 current Observation、原 exact range 以及 expectedText/inert-paragraph 约束，最后重验 cut。recipe/Locator bytes 不改写、不重签。生产版本改变/不可证明时使用既有 dependency_conflict/stale/availability；没有该 Locator 的模板不因本规则被全局禁用。

实例化顺序为 closed decode→当前入口/完整潜在 workspace_constraints 披露及读取资格→请求 CommitDomain/原 authority 与 P 连续性→验证显式 template/recipe/resources 的 SourceVersionRef→读取完整 recipe→在原潜在授权范围内取得每个精确生产地址的真实当前 Observation→复核完整源、live 子树、typed slots、Registry/规则和目标→完整分析/准备。recipe 内容不能扩大原潜在范围；实际新增读取资格也在读取前逐项证明。每个 source 的当前 observation.sourceVersion 必须逐字等于 recipe 中固定的完整生产版本，绝不只比 Counter、文本或摘要。显式选择的 Template 及 Resource token 必须来自真正 current-source/选择 producer，服务器不从旧 Counter 读取 latest 后补签。

当前来源已经改版时为 dependency_conflict，即使旧字节仍在历史 pin 或输出恰好相同也不得以旧版本冒充当前模板。需要旧内容时，用户先走原有明确恢复/复制操作，或明确选择新来源并更新配方；本适配器不直接实例化无当前资格的历史冲突头。当前元数据/源/连续性不可证明按实际 owner 的 source_unavailable/proof_unavailable/domain_unavailable 分支停止；无权限先 not_visible；可达矛盾证据才 integrity_conflict。外部 SourceVersion 不含 managed revision，不能填 Recipe/2 managed 地址或内层 revision；确实需要该来源时先显式合法 admission，再凭真实新版本准备。无条件自动写入和把 externalSequence 当 revision 都禁止。

所有实际消费 source bytes 的观察进入最终原 InputDescriptor/2.sourceInputs，使用其完整 {entityRef,observation,role} 项及原排序规则；普通来源只用 before/dependency，不放 artifact 或 conflict installation wrapper。真实 source pins、完整正负目录、placement、Registry、授权及所有 pre/post Query 进入同一 DependencyProof/2。D7 /3 constructionInput 是本篇 /2，resolutionInput 与最小映射 resolutionAccess 为 null；constructionReadRefs 恰覆盖实际输入和完整省略读取，按每个 Ref 的真实用途先验证源读取或 state/annotation 目录披露资格，再读大记录；仅省略目录项不升级为 Annotation body 读取，也不伪造该 body 的 sourceInputs。D9EntityVersionAddress/2 的完整定义及非 current-token 边界见 Worker 与导出 §3b。parent/collection 两分支都保持原 D3 wire12、d3_identity_operation/12、strict、managed_atomic、exact Frontier 及其单 planning CAS/P seal；没有 D9 提交入口或 owner descriptor 替身。

PL-IR-01 现已有上文消费的 D3/D6 修复：production address 加已认证 canonical Locator binding 与接收端自己 exact current Observation 可以给**新读取**资格而不改写 recipe。它仍是本 exact candidate 的独立复核门，不再是未定义算法，也不代表 P1 已关闭。相同 span/text/hash/version Counter、取 latest 重签或 DefinitionTransfer 均不能替代。缺资格只阻止受影响 construction，其它 template 能力继续自身门禁。

Recipe/1 和 node-template/1 的真实已存 analysis/preparation/decision 使用原 profile decoder。已经 planned/saved/unknown 的原 D7 /1、/2 和 D3 原 wire 请求先按原最小授权映射、custody、pins、原请求与字节恢复，不补 /2 地址、不换 token、不重新编译或创建第二实例。未成决议的旧准备只按其真实 TTL/过渡规则失效；不是把所有历史原型宣称已经部署。

尚未提交的旧配方保留可读、可显式升级的正向路径。Host 提供具名“按明确来源另建配方”准备动作：用户在真实当前读取中逐项选择原 recipe、每个 source Node 和资源，Core 完整解码 Recipe/1，并先显示每个旧 {ref,expectedRevision} 到新完整 managed SourceVersion/2 的映射。相同 Counter 不证明原生产版本；若原来精确版本的受保护历史证据可得，展示其与所选当前版本的完整比较；否则明确标为新的用户来源选择，不声称无损自动迁移。参数、树、targetFacets、bindings、资源和损失规则逐项保留；body_text Locator 仍须原 owner 当前验证，不猜迁移。

用户明确接受完整新来源映射后，Core 生成 exact Recipe/2 JSON 提案，经既有 D7 d3_operation + D3 create_resource 的完整 prepare/preview/独立确认另存一份普通 recipe Resource（用户明确给 owner），原 Resource 不自动覆盖。此动作使用原 Resource 创建协议与完整实际读依赖，不扩展 ActionSpec、public commit 或 constructionInput 的两个实例化 mode；只有该原 D3 receipt 才证明新配方已保存。新的模板实例化再显式选择这个新 recipe 的真实 SourceVersionRef。无当前合法来源/权限/Locator 时保持旧配方可读并拒绝该具体升级，不能将未知历史自动补签为当前或永久取消其它合法模板能力。

## 7. 首代限制与实施门

公开原型曾允许H1–H9、中文attr、自动公式调整与广泛view。基于当前D2/D4闭集、D5无Record和公式坐标修改缺完整安全语义，本代明确收窄/替换这些候选：H1–H5、header/完整FieldId、有限view、无公式模板、单band。没有修改产品实现，旧文本不作为D9能力证据。

后续实施必须实际证明多run一致格式、mixed拒绝、token转义/injection、零/单/多项重复、双方向/RTL、0row/column删除、样式inheritance/冲突、body/bibliography不重复、复杂值/权限/none、恶意包以及Word/WPS/LibreOffice各具名版本打开保存渲染。不以ZIP/XML样例通过声称真实Office互通。

## 8. 最小完整Office例子

假设当前Registry明确将`user/report-code`定义为不可重复text，当前Node恰一Entry，且Core已完成名为contacts的dataset，列name/phone均为已授权显式text投影。people/name和people/phone本身仍是D4复杂、多值facts，不因列名投影变为scalar Field。

DOCX逻辑段落依次为`{{ meta.title }}`、`{{ field.user/report-code }}`、一个使用实际H1 paragraph style的`{{ style.heading.1 }}`样板、`{{ content.body }}`、`{{ view.bibliography }}`。最后一个普通二列表格含header行Name/Phone及prototype行`{{ ↓ data.contacts.name }}` / `{{ ↓ data.contacts.phone }}`。所有token均是普通文字，style样板和body/view各独占paragraph；没有content control。subtitle只有用户明确选择且有值才加token。此例假定本次bibliography已编译且可读。

contacts有两项时结果保留header并有两行；零项时仅保留header。body原bibliography placement被明确移到独立slot且只渲染一次。样板paragraph移除；AsciiDoc `==`标题取得该style并显式outline0；缺其余role时按固定bundle策略处理。把prototype行改成整列、token改为→则沿columns递增重复，不因RTL反序。任何name投影不明确、多值未裁决、隐藏依赖、混合token样式或未支持对象都会阻断本次prepare，而非生成部分报表。
