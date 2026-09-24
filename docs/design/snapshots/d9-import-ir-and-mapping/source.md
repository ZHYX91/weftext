---
_weftext:
  id: "1f779371-93a8-45b4-9b6a-c7304fbcdd35"
---

# D9 Import IR 与映射

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


revision: D9-r04-2026-09-24；candidate。所有类型只用于输入证据和转换提案，不是受管内容代数。本文件的版本 `1` 是新目标 profile；旧实现 `weftext.import-ir.v1` 不取得兼容身份，实施时成套替换。

## 1. 冻结与可扩展边界

冻结共同 envelope、来源坐标、分型内容、损失/未支持项、完整性及 Core 映射义务；不冻结第三方 AST，不宣称三种分支穷尽所有未来格式。新增不能表示的格式结构需要新 profile 和 admission，未知 kind/version 拒绝，不能塞一个 `any` 字段。IR 允许中间结构比 D2 丰富，转换到 D2 必须显式损失或拒绝。

`ImportIR/1` exact `{format:"weftext.conversion-ir",version:1,sources,profileId,documents,resources,issues}`。sources 为已绑定 SourceArtifact 目录，顺序与 probe 相同；documents 是非空有限数组；resources 是提取工件目录；issues 是完整检测到的问题。key 均为本 IR 的 Counter ordinal，连续从0、无重复/洞；它只是本次表示内地址，不进入 D3 identityMap。数组顺序表达 source/profile 定义的顺序，不用 object iteration 或摘要排序重排阅读顺序。

`document` exact `{key,sourceIndex,title,content}`。title 为原观察文本或 null，缺标题必须在 mapping 明确补入，不从文件名无声生成。content 三 variant：

- page exact `{kind:"pages",pages:[Page...]}`。
- flow exact `{kind:"flow",blocks:[FlowBlock...]}`。
- grid exact `{kind:"workbook",dateSystem,sheets:[Sheet...]}`。dateSystem=`none|excel1900|excel1904|odf`；不根据 host 默认猜。

一个输入可产生多个 document（如 package），每个都必须在 mapping 中选择 disposition；不自动把每 page/sheet 物化为 Node。PDF 默认一个 document，XLSX 默认一个 workbook；显示 sheet 不等于持久实体。

资源 exact `{key,sourceIndex,mediaType,byteLength,sha256,outputSlot,origins}`。outputSlot 是 worker 隔离输出中的 opaque Counter，host 验证实际 regular file bytes 后签发；不收任意路径、URL 或 base64 内嵌巨型数据。descriptor 与实际长度/摘要必须相同。相同 bytes 的两个语义资源可保持两项，是否显式复用由 mapping 决定；不能按摘要合并 provenance。

IR issue exact `{feature,locations,message}`，feature用§7最低闭集，locations为非空SourceLocation数组，message为非空纯文本。worker只报告观察，不能指定用户选择/授权/接受严重程度；Core独立coverage和mapping产生LossReport并合并所有worker issues，不允许吞掉。输入整体issue使用覆盖完整source的bytes location。零内容但合法的CSV空文件可形成空workbook；无法解析的空结果不能冒用这一规则。

## 2. 来源、置信度与完整性

`SourceLocation` closed union：

- `{kind:"bytes",sourceIndex,start,end}`，半开 bytes，0≤start≤end≤原 source byteLength；不声称文字 scalar 坐标。
- `{kind:"page",sourceIndex,page,rect}`，page 为零起真实 page index；rect=`{x,y,width,height}`，各为整数百万分比，0≤x,y<1,000,000，width,height>0，和≤1,000,000。基准为已定义的、正向阅读页面归一化空间。bbox 不等于文本选区。
- `{kind:"cell",sourceIndex,sheet,row,column}`，sheet/row/column 零起；来自真实逻辑 worksheet，不能按 filtered 可见位置编号。
- `{kind:"part",sourceIndex,part,elementPath}`，part 为经 canonical OPC/ODF part-name validator 的包内名称，elementPath 为从该 XML part 根出发的非负子元素 index 数组；命名空间已由格式 profile 验证。仅证据定位，不读取工作区路径，也不跨输入版本延续。

`Observation` exact `{origins,confidence,method}`；origins 为非空 SourceLocation 数组，confidence 为 null 或 Counter 0..10000（basis points），method=`extracted|ocr|inferred|user_supplied`。null 表示未提供可信置信度，不能变成100%。method 由运行 route/真实观察确定，worker 不可把 OCR 或推断标成用户原文。同一文字可以来自多个区域；文字相等不合并 observation。

IR validator检查全图无循环、完整索引、来源范围/owner，issue 引用、未知成员、实际资源、格式 profile 及预算；不保证 parser 没漏读 source。格式 coverage validator 还必须独立核对 page/sheet counts、全部选定包 parts、selected cell 范围/内容、读取顺序与 unsupported features catalog。所有已发现而无法表示的对象进入 issues，source 暂存保留供用户查看；不能把空 IR 当成功。无法确定覆盖或格式不识别为 invalid_output/unsupported_profile。生产 profile 验收需要真实语料和独立解析/渲染对照；当前有限 fixture 只提供反例和表示可行性。

## 3. Page 分支

`Page` exact `{index,widthMicrometres,heightMicrometres,rotation,items}`，index 连续；物理尺寸为正 Counter，rotation=`0|90|180|270`。profile 的区域基准已应用原件旋转，items 中 rect 是显示正向页面；rotation 记录转换元数据而不要求消费者再次旋转。

`PageItem` exact `{key,observation,rect,content}`。key 在所属 document 所有 items 内连续。content closed：`{kind:"text",text}`；`{kind:"image",resourceKey,alt}`（alt text|null）；`{kind:"table",rows}`（rows 为矩形 CellValue 数组）；`{kind:"unrepresented",feature}`。text 是逻辑阅读文本、保留 scalar，不按 glyph 视觉顺序逆排 RTL。items 顺序是 route 声明的阅读顺序，推断/多栏歧义产生 issue，不把几何 y/x 排序称为可靠语义。

page table 是提取观察，不自带 Field schema 或 header 权威；merged cells/嵌套结构若此分支不能表达必须 issue，并可选原图。未检测/OCR置信度不足不能补造内容；有 OCR 输出也必须预览。截图/扫描件的图像资源和 OCR 文本是两个有来源的结果，不宣称文本为 exact 原文。

### 图片物理尺寸 profile（image_physical_size/1）

这里的物理尺寸是源内显式声明的显示尺寸，不证明被摄物体、扫描纸张或相机真实尺度；元数据没有记录的值不能用格式/宿主默认补成观测。image_page_v1 的物理尺寸只可来自准确bytes中经格式decoder验证的PNG pHYs（unit=metre）或JPEG JFIF密度／EXIF XResolution、YResolution、ResolutionUnit。像素尺寸不是物理尺寸。PNG unit=unknown、JPEG仅aspect ratio/缺density、EXIF无实际记录的可用绝对单位均记为绝对尺寸缺失，不取72/96 DPI、操作系统默认或预览窗口尺寸。存在非法、零/负密度、无效有理分母或相互冲突的绝对密度声明时为invalid_output；不能挑一个来源或靠显式尺寸掩盖格式歧义。每个具名生产decoder必须完整检查这些元数据，未支持的密度变体不可猜测。

各轴先用exact有理数化为pixels per metre：pixels/inch乘5000/127，pixels/cm乘100；PNG整数ppm原样。多个绝对声明只有各轴化简后严格相等才一致。物理微米=像素数×1000000/ppm；用exact arithmetic舍入至最近整数，恰半取偶数。结果须为正Counter且在route预算内，舍入为0或溢出返回unrepresentable。存在非整数舍入时以precision issue/loss记录真实量化，不宣称无损。先计算原像素轴及物理轴，再按已验证EXIF orientation正向变换；5–8交换宽高，其余不交换。无合法方向声明时适用格式规范的无旋转默认；非法/冲突方向仍拒绝。

相对密度（PNG unit=0、JFIF unit=0）仅固定显示宽高比 W×dy/(H×dx)，不得当绝对单位；相对与绝对声明的比例须一致，否则invalid_output。所有密度/比例均无声明时，本profile显式采用square-pixel显示比例W/H，这只是布局基准，不补物理尺寸。正向orientation 5–8对该比值取倒数。Office显式尺寸需按这个显示比例而非总像素数猜保真；绝对尺寸存在时量化产生的微小比值变化按precision loss，不重复当独立aspect损失。

合法但缺少绝对密度的图片不能构造本代Page物理尺寸，image_page_v1转换返回unrepresentable；可作为不执行原件Resource保存。Office图片asset若提供冻结的显式尺寸仍可布局，见Templates，不能把该布局选择倒写成IR源观察。本代没有图片Page的默认DPI或用户补物理尺寸入口。

归一化Resource Region只依准确解码像素/方向与原Resource revision，不依物理尺寸。缺少DPI不妨碍已经满足几何profile的region签发；物理尺寸政策不得改变d9rg1或外层l1。



## 4. Flow 分支

`FlowBlock` exact `{key,observation,content}`；key 在 document 的全树 pre-order 连续。content：

| kind | exact 其他成员 |
|---|---|
| paragraph | text |
| heading | level,text；level 1..9 是源格式观察，D2 目标仍最多5 |
| list | ordered:Boolean,items:[{text,children:[FlowBlock...]}...] |
| table | rows:[[CellValue...]],merges:[{row,column,rowSpan,columnSpan}...] |
| literal | text,language:text或null |
| quote | text |
| image | resourceKey,alt:text或null,caption:text或null |
| break | 无 |
| unrepresented | feature:text |

Flow 输入不用 source_text 自带已受信 AsciiDoc。复杂 inline 样式、footnote、math、tracked changes、embedded object 等第一代 Flow 没有专用结构，必须 issues 和明确 flatten/保留原件/拒绝；不得把表面可读就称无损。页眉页脚/批注/脚注文本若导出器无法独立映射，不混入 body。后续 richer flow profile 可替换本 profile，但不能靠未知属性潜入。

## 5. Workbook 分支及 CellValue

`Sheet` exact `{index,name,visibility,rows,columns,hiddenRows,hiddenColumns,cells,merges}`；index 按 workbook 顺序连续，name exact 不 trim，visibility=`visible|hidden|very_hidden`。rows/columns 为该 profile 选定有限逻辑矩形大小，不信任 dimension 元数据直接分配；实际 sparse cells 按(row,column)排序且唯一。`cells` 元素 exact `{row,column,value,formula,styleClass,observation}`。不存在的坐标为 **absent**，现存空 cell 是 blank，不相等。formula 为 null 或 exact `{text,cached}`；cached 为 CellValue 或 null（无缓存），value 为公式缓存或 blank，但 formula 存在时两者必须一致，不得只读 `data_only` 丢掉公式事实。styleClass=`general|text|date|time|datetime|other` 仅源显示提示，不等于 typed value。

`CellValue` exact variants：`{kind:"blank"}`、`{kind:"text",text}`、`{kind:"boolean",value}`、`{kind:"number",lexeme}`、`{kind:"error",code}`。number lexeme 是源格式允许的有限十进制/指数 ASCII 词法，保留原文字、不经 binary float；格式 decoder 先限定长度/指数预算，Core mapping 再显式转 D4 canonical decimal/integer。它是 source evidence，不是 D4 值，也不允许 NaN/Infinity。error code 为格式 profile 的原合法 error 文本；不会变空值。XML shared/inline rich text 按逻辑 run顺序连接且记录 formatting loss，不能更改已有文字空白；无文本值和空字符串区分。

hiddenRows/hiddenColumns各为exact {start,end} 的半开source ordinal区间数组；0≤start<end≤相应rows/columns，递增、无交叠且相邻区间必须合并。无隐藏为[]；visible/hidden/very_hidden sheet状态独立。format coverage必须从实际row/column声明（含压缩/repeated范围）独立证明两个数组完整，不能从message推断；CSV/TSV为[]。有限逻辑范围包括所有实际cell/merge/明确hidden范围，不因dimension值遗漏隐藏内容；超过预算拒绝。

merges矩形须在sheet范围、正span、无重叠，anchor保留真实value，覆盖格不自动复制。公式/merge不在读取时展开。hidden sheet/row/column事实全部以typed字段保留，并列具有准确SourceLocation的hidden_content issue；其它rich text/comments/drawings/validation/filter/external links/pivot等仍完整issues。Worker不能决定include/exclude；由下面唯一mapping规则决定。

CSV/TSV 是单 sheet 的 text cells；所有输入字符保留，不先推断数字或日期。CSV profile 是 UTF-8 严格解码、可选 UTF-8 BOM 明确记录、用户选择 CRLF/LF、RFC4180 quote/double-quote规则；分隔符和 header 明确，不嗅探替换。非法 quote、ragged 行、NUL 拒绝；空文件是0行0列，单条空 record 是1个空 text cell。TSV profile 以 tab 分字段、所选 CRLF/LF 分行，第一代不支持引号转义、字段内 tab/换行；含这些字节不伪称无损 TSV。末尾单个 terminator 不生成额外行，空白和连续空字段保留。列由 ordinal 选定，重复/空 header 不作唯一 key。

## 6. ImportMapping/1

mapping exact `{version:1,documents:[DocumentMapping...]}`；每个 IR document 恰一项，按 key 排序，无遗漏/重复。DocumentMapping closed union：

- `{documentKey,kind:"omit",reason}`，reason 为用户明确非空说明；对应 omission loss。
- `{documentKey,kind:"document_node",destinationParent:NodeRef,destinationOrdinal,title,bodyPolicy,retainOriginal}`。title 非空且可编译到 D2 title；bodyPolicy=`strict|plain_text|page_images`。retainOriginal:Boolean。strict只接受能完整映射到 D2 的观察；plain_text必须逐 loss确认；page_images只接受有完整页面图像证据的 pages，逐页图片作为本 Node Resources，不能把未渲染的页面当空图。
- `{documentKey,kind:"rows_to_nodes",destinationParent:NodeRef,destinationOrdinal,sheet,range,header,titleColumn,facets,columns,hiddenPolicy,retainOriginal}`。range exact `{startRow,endRow,startColumn,endColumn}` 半开；所有选定 source rows 明确，header Boolean，titleColumn 为绝对源 column ordinal。facets 是 D2 FacetId sorted unique set；columns 元素 exact `{column,fieldId,conversion,emptyPolicy,formulaPolicy}`；FieldId 必须 Registry 已定义，不接受显示 label。conversion=`text|integer|decimal|iso_date|boolean`；emptyPolicy=`reject|omit|empty_text`；formulaPolicy=`reject|cached|literal`。同 source column 可显式映射多个不同 Field，但 duplicate target Field 必须由 schema 可重复和完整 entry plan证明，不能 last-win。
- `{documentKey,kind:"table_document",destinationParent:NodeRef,destinationOrdinal,title,sheet,range,header,formulaPolicy,hiddenPolicy,retainOriginal}`；新 Node body 内产生 D2 原生 table，全部 cells须证明 inert 且 D2可表示，不建立 row identity。

只有workbook可走rows_to_nodes/table_document/omit，不能走document_node；flow/pages只走document_node/omit。多文件目标 ordinal 由 Core 在同 batch按 D3 forest规则计算，冲突提案拒绝，不凭UI顺序占两个相同位置。titleColumn为空/blank/公式未裁决/非法 title 时失败，不临时用“未命名行N”。rows_to_nodes每数据行 fresh Node，重复行保留为不同对象。retainOriginal在 rows_to_nodes 时需明确额外 wrapper Node承载原件，第一代该 variant 固定 false；true 返回 mapping_required 要求单独附件意图，不能隐藏创建 wrapper。其他两 variant原件 Resource归新 Node。

两个workbook mapping必填 hiddenPolicy exact {sheet:"exclude"|"include",rows:"exclude"|"include",columns:"exclude"|"include"}。UI初值三者exclude，实际请求必须显式保存全部三项；矩形选区本身不等于include。权限检查先于显示隐藏范围。

1. sheet.visibility不是visible且sheet=exclude：选定结果为空，不产生任何Node/Resource；retainOriginal=true在此组合返回mapping_required，不能隐含附件Node。完整分析列明整sheet排除。include才继续行列计算。
2. 在原range半开行列中，按各axis的exclude删除与typed隐藏区间相交的indices；include保持。得到严格递增的source row list与source column list；绝不把source ordinal重编号后再应用binding。rows_to_nodes的titleColumn及每个columns.column须都在保留column list，否则mapping_required，不能挪到下一个可见列或默删Field。
3. header=true指原range.startRow，而不是“第一条留下来的行”；此行被隐藏排除时mapping_required。header本身不产生Node。header=false不读header语义。rows_to_nodes对剩余每条source数据行按原序生成一个fresh Node；同值行仍分别生成。
4. table_document输出为保留row list × column list按原序压紧的矩形，所有cell仍保留original cell origins。merge与选定矩形/排除范围有交叠且不能完整保留时unrepresentable；完整保留的merge按既定flattened table输出，merge布局loss必须明确接受，不复制anchor值。任一axis为空则没有table/Node/Resource，retainOriginal=true同样mapping_required。不能生成假header或空“未命名”Node。
5. 被排除的实际内容形成hidden_content requires_choice，effect精确列sheet/row/column和本次投影；纳入仅产生说明visibility未成为D2作者隐藏状态的notice（若目标无法表达该presentation则另列formatting loss）。切换hiddenPolicy必须重新d9_import_analyze，完整重新计算对象目录、ArtifactObjectKeys、组/批次和preview；旧analysis/loss接受不继承。相同range配不同hiddenPolicy是不同输入，即使本次恰好没有hidden对象。

转换规则：text保持exact；integer只接受ASCII canonical整数（不含千分符、locale数字、小数或 exponent）；decimal用exact arithmetic形成D4规范字符串，任何词法形式变化进入loss report；iso_date只接显式ISO完整日期文本；输出必须为D4 calendar_date且calendarId=calendar/iso8601、calendarVersion="1"、precision=day及经原decoder验证的lexeme，不生成kind=iso_date/date，不从styleClass猜。boolean只接源boolean或明确文本`true|false`，不把1/0当布尔。empty_text仅目标text允许，blank/absent/empty text的原区别进入loss；omit是零Entry，不造null。error cell在本代不能进入Field值，只能literal text经过明确转换/损失，未提供该专门转换则拒绝。

第一代 rows_to_nodes 的iso_date对numeric date serial不可用；先在显式映射预览中选择独立 `date_serial_to_iso` profile才可转换，该 profile 尚未开放。导出方向的日期策略见 Workers and Export。Excel1900 serial60、时区/非Gregorian/精度不能暗猜。公式cached路径必须有缓存并说明 freshness=unverified，不运行重算；literal仅导入为text原公式，cached缺失拒绝，公式从不转CEL。

复杂/多值D4 object、relation、quantity、People names等不能硬塞上述scalar mapping；本代选择保留原件或独立完整Core typed提案，未提供closed mapping的字段明确unsupported_profile。此限制不是把D4字段降成字符串，也不阻止人工D7 typed Action。

D2 table生成必须逐cell证明最终inline解析为请求的inert text；正确处理pipe/backslash，拒绝CR/LF、宏形状、attribute形状或其他不能无损表示的值。拒绝不会截断/自动拆行/ragged修复。本文只授权新Document中的明确转换，不开启D8现有table批量粘贴。

所有IR的text、标题、list item、quote/literal文字及模板replacement均为文字数据，不能串接后仅检查D2-valid。Core先由明确mapping形成预期D2语义AST，安全编码成完整source，再独立完整parse并逐项比较结构、文字、metadata、typed refs及其位置；除显式loss变换外必须相等。输入“== text”、宏或carrier形状不能变成意外结构，无法编码为所需inert语义就unrepresentable。未知kind/无法穷尽转换的block是阻断，不通过忽略字段获得valid source。

## 7. LossReport/1

本节只用于文件导入与Node Template构造；导出使用Workers and Export §3a的独立ExportLossReport/1，不扩大此处locations union、TemplateConstructionInput或PreparedActionBinding。

exact `{version:1,items}`；item exact `{lossKey,feature,locations,effect,severity,allowedChoices}`。lossKey是本次完整提案内连续Counter，与输入/映射一起绑定；文件导入locations为非空SourceLocation数组；Node Template构造则用Templates的TemplateLossLocation数组，两域不能混用。不能用任意字符串指向另一个文件；effect为非空可读说明；severity=`notice|requires_choice|blocking`；allowedChoices为闭集值数组，来自 `accept_loss|reject`。

第一代choice只裁决已完整生成的固定提案，不执行转换：blocking的allowedChoices固定[reject]且不能prepare；notice固定[]；requires_choice固定[accept_loss,reject]。accept_loss保留真实effect并在新的analysis记录已接受notice；reject返回cancelled且无新author request。转换本身必须在ImportMapping的bodyPolicy/retainOriginal/formulaPolicy/hiddenPolicy/omit，或TemplateConstruct的明确选择中表达，改变它们重新analyze。没有通用retain_original/flatten_text/omit choice；尤其rows_to_nodes不能靠loss choice绕过retainOriginal=false。完整选择一次覆盖全部requires_choice和blocking，blocking只能reject并使整个选择cancelled；旧analysis不可变，新的损失不继承旧接受。

最低feature词表：`formatting|layout|reading_order|ocr_uncertain|heading_depth|merged_cells|formula|formula_cache|precision|date_semantics|hidden_content|unsupported_object|metadata|control|field_semantics|references|annotations|signature|active_content|omission|direction`。未知feature需要新profile，不以free-text code隐藏未闭合语义。报告包括所有检测到的实际loss，不把假定未发现的内容描述为已保留。原件完整性与可证明的目标语义分别报告。

准备证据保留选择前初始LossReport和完整lossChoices；当前analysis显示已接受notice不删除原报告。每次Core复验按相同输入、profile与固定映射确认全部effect相等，才能消费原lossKey选择；任何新增或变化的损失要求重新analyze及确认，不能按序号套用旧接受。

各feature按固定目标效果分类：安全/格式不明/coverage不确定/无权/不合法D2-D4/活动模板为blocking，accept_loss不能覆盖；明确未选择内容或固定可表示投影造成的formatting/layout/reading_order/OCR/heading_depth/merge/formula-cache/precision/date/metadata/control/field/reference/annotation/signature/omission等损失为requires_choice；没有信息损失的纯来源/方向说明为notice。必须有实际来源与具体输出差异才能列为loss，无法定义目标效果的feature是blocking。原件保留是mapping选择的独立Resource，不能把其它loss清成无损。此分类与上面唯一allowedChoices矩阵合用；禁止UI或自由message定义转换程序。

## 8. Resource Region Profile/1

仅冻结 `pdf_display_rect/1`、`image_display_rect/1`。`regionToken` 是 `d9rg1.` + 对 `ASCII "D9-Region/1" + NUL + D3-CJ/3(RegionBody)` 的canonical无padding base64url编码；不存在host私有registry handle。RegionBody exact `{version:1,profile,page,rect}`，只持非身份几何事实。准确ResourceRef/revision由外层原D3 ResourceRegionLocator完整绑定，随后按原`l1`编码为D2 regionLocatorToken；禁止把d9rg1单独当成完整Locator或在RegionBody重复嵌入尚未分配的fresh Ref。Core按准确资源bytes验证几何，再结合原D3 fresh subject materialization签发外层owner/revision。page正整数，与本IR零起page必须显式+1转换。image只有page=1；多帧图像本profile拒绝，不能只取首帧。rect使用§2百万分比正面积矩形。

PDF基准：准确Resource bytes中该页CropBox裁到MediaBox后的有效范围，应用/UserUnit及/Rotate到正向显示，左上原点、x向右y向下；拒绝非法box、非正尺寸、非90倍旋转或无法确定的page。image基准：实际解码后按EXIF orientation变换的正向图，左上原点；方向标签非法/歧义拒绝。normalization由Core针对已支持的解码器profile验证，UI只提供选区意图；对外签发将x/y向下取整、right/bottom向上取整到百万分比并裁到[0,1000000]，零面积拒绝。相同源/profile/矩形输入输出唯一token。

d9rg1不含zoom、DPI、viewport、glyph索引或文件名。图片/页方向变化如果bytes变化就是新revision，旧完整l1 Locator stale；视图zoom/RTL shell不改Locator。copy/fork/owner变化必须按D3重新签发完整外层owner/revision，d9rg1几何仅在同一准确内容profile可验证时保留。PDF→PNG/OFD→PDF是不同Resource，不自动转移批注；用户可明确创建新target，旧target原样。

原D3 resolver的词法与失败顺序不变：非空regionToken仍是opaque string，不能在stage1/未授权时先运行几何decoder。先原owner/entity与mayDiscloseLocatorState资格，再准确revision和本profile几何验证；same-revision region无法验证按原resolver stale、操作stale_locator。只有D9新签发业务入口可使用unsupported_profile/invalid_output，不能把它们塞进既有D3 resolver。d9rg1区别于D2 ResourceRef的r1外层token，二者不能互用。

区域可访问替代：已有Annotation plain body照常显示，UI朗读页码与百分比矩形，并可键盘选择whole Resource；不声称从rect能生成精确可读文本。OCR生成的替代文本单独标明推断，并受Resource读取权限。region不授编辑、跨版本定位或正文source范围。所有解析、签发和真实屏幕阅读器仍待实现验收。

## 9. 第二结构验证的判定

PDF页面有几何/阅读顺序/OCR不确定性；XLSX工作簿有sheet次序、typed cell、absent/blank、formula/cache、merge与date system，不能经paragraph/table string模型无损表示。实际fixture从PDF bytes和XLSX ZIP/XML bytes分别提取再进入上述分支，验证schema/映射/损失拒绝而不依赖第三方IR标识。另用CSV quoted newline/ragged/重复header样例检查区别。该证据足以检验共同边界不强迫两种结构同构；不能证明所有PDF/XLSX、官方全量conformance、Office应用互通或生产隔离。新格式仍逐profile准入。
