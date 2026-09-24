---
_weftext:
  id: "a93f99cf-717a-4da0-953f-1e1680c2d039"
---

当前权威状态：D8 D8-r03-p2-2026-09-24，仅由总控验收（外部控制记录未随本输入发布）和committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。下面送审正文中的candidate/未激活为历史标签，不覆盖本段。架构接受不代表产品实现；D9未启动。

# D8 Editor Interfaces

revision: D8-r03-p2-2026-09-24；candidate。所有新接口由本机或Server内同一Core实现；客户端不拥有canonical parser。新接口只增加D8准备/草稿读取入口，提交仍是原D6；D3新身份/生命周期仍走D7/D3。禁止把本文件新kind注入D7 ActionSpec/1或D3 mode闭集。

## 1. 基础对象与解码

D8 JSON严格UTF-8，单个object，重复key、未知/缺失成员、未知kind/version、非scalar字符串及不合法null拒绝。表中未标optional的成员必填。D8结构数字逐字复用D6 Counter，拒绝bool/小数/指数/-0/溢出；嵌入D2/D3/D4原值按原协议的null和数字规则解码，不能用D8外层规则改写原对象。

`WorkspaceRef`、`NodeRef`、`AnnotationRef`、`SourceVersion`、`D3-Annotation-Value/3`、`D3-Annotation-Target-Projection/1`、D2 source_range、D6 BudgetBinding/Token各逐字复用原closed类型。D8 EntityTarget恰为D7已有`{ref:EntityRef,expectedRevision:Counter}`。同一请求的Workspace、owner、target refs必须一致；跨Workspace请求不能猜外部authority。

`draftSerial`只是本次客户端proposal的回传标记，不是服务器存储对象、revision或权限。`documentRevisionToken`只有Core对已提交源按D6生成；不得给未提交Draft伪造d6d token、l1 Locator或DocumentRef。D8返回的sourceRange只定位返回时绑定的完整Draft字符串，不能流入D3 AnnotationTarget。

## 2. 已提交文档读取

请求exact `{wireVersion:1,kind:"d8_document_read",workspaceRef,ownerNodeRef,budget}`。Core按closed decode → 当前workspace/entity-state、完整source_read及源Envelope状态资格 → authority/cut → exact source及D2解析 → 当前交付gate执行。budget为D6 BudgetBinding，客户端不能增大policy资源。

成功exact `{wireVersion:1,kind:"d8_document",workspaceRef,ownerNodeRef,sourceVersion,documentRevisionToken,authorityToken,snapshot}`。sourceVersion.entityRef=ownerNodeRef，revision为该cut当前版本；authorityToken为原D3/D6 opaque authority generation域；snapshot为完整D2 `document_snapshot`，exact `{wireVersion:2,kind:"document_snapshot",ownerNodeRef,document:<D2 document_payload>}`。外层ownerNodeRef、snapshot.ownerNodeRef及sourceVersion.entityRef必须逐字相同，snapshot及全部元数据来自同一cut。document_payload只出现在原D2合法的snapshot.document位置；D8不新增D2嵌入例外，也不返回额外NodeSnapshot。invalid D2仍通过snapshot.document的完整invalid branch给已获权source，不能输出partial body。physical decode失败不给D2 payload，使用source_unavailable；专门D6物理repair另行授权。

当前主体无完整source read时整个入口not_visible，不能返回删去Field的伪exact源。类型/locator/Ref存在性仍按上游当前授权顺序；读取source原值不自动解析所有作者Ref获取目标信息。D2必须验证的引用门使用Core当前cut，不能网络fetch。

正向形状：D8 metadata envelope内的snapshot是可独立通过D2 document_snapshot decoder的完整对象。反向形状：把document_payload直接置于d8_document.document、删去snapshot的wireVersion/kind/ownerNodeRef、任何owner不一致，均违反closed响应合同，不得作为成功被接受。

## 3. Draft投影、编辑映射与普通正文命令

所有入口先closed decode，再执行§2完整读取、sourceEnvelope及适用D6 workspace_constraints潜在观察资格、authority/current live owner和expectedSourceRevision相等检查，随后用同一Core的D2 parser完整解析source。没有author写、OperationId、PreparedIntent或receipt。客户端提供的source不是读取隐藏内容的资格。sourceRange只指本次完整proposal，永不发成D3 Locator。

### 3.1 投影及准确绑定

请求exact `{wireVersion:1,kind:"d8_draft_project",workspaceRef,ownerNodeRef,expectedSourceRevision,draftSerial,source,budget}`。

成功closed union：

- valid exact `{wireVersion:1,kind:"d8_draft_projection",ownerNodeRef,baseRevision,draftSerial,source,status:"valid",metadata,attributeCarrierBlocks,body,editMap,origins}`。
- invalid exact `{wireVersion:1,kind:"d8_draft_projection",ownerNodeRef,baseRevision,draftSerial,source,status:"invalid",diagnostics}`。

source逐字回显完整proposal，baseRevision恰为expectedSourceRevision。绑定为完整owner、baseRevision、draftSerial、source四者；客户端必须四者逐字匹配才消费，不能仅按serial或内容相似判断。Core无跨调用Draft对象，也不声称serial是持久身份。重复source的运输/内存计入预算，可在本机共享不可变存储，不能删减wire绑定。

metadata及attributeCarrierBlocks为D2原closed投影。body为D2 DocumentBody按schema递归仅删除addressable element的locatorToken；其余kind/成员/null/顺序/raw原值保持。不遍历用户payload的同名key。该body不是D2 wire payload。invalid只给原D2唯一primary diagnostics，禁止metadata/carriers/body/editMap/origins，不交付partial。D4 admission和最终提交资格不从此推导。

### 3.2 编辑资格、普通文本区域与只读来源

`editMap` exact `{flows,plainRegions,sites}`。所有数组按source/schema顺序；坐标只在§3.1完整owner/baseRevision/draftSerial/source绑定内有效，不是Ref、Locator或跨revision地址。

**BodyPath**从body根出发，字符串步闭集为`children|heading|items|nestedLists|rows|cells|inlines`，数字步为数组合法index；仅依D2 schema行走，不能进入metadata、anchor、ref、payload或任意用户key。槽路径与元素路径由终点类型区分。例如paragraph的路径是`["children",0]`，其Inline槽为`["children",0,"inlines"]`；相同文字的第二个cell使用自己的rows/cells路径，不按文字查找。

**flow** exact `{path,text,segments}`。每个D2 Inline[]槽恰一个flow。text按Inline顺序连接：Inline.text用原text；每个非text Inline用一个U+FFFC原子坐标占位。UI按Inline渲染真实label，不把占位写入source或复制文本；source原有U+FFFC仍可为普通text。**segment** exact `{start,end,sourceRange}`，连续无重叠覆盖非空flow；空flow为[]。nonnull仅用于单logical line内、Inline.text与raw substring逐scalar相等、Core词法证明未经过escape/join/macro加工的最大连续片段。null用于generated join、escape加工与原子，相邻null可合并。

nonnull片段的display offset d映射到source `(startLine,startColumn+d-start)`，两端包括边界。请求显式选择flowPath/segmentIndex，接缝不靠UI猜。反向selection保留anchor/focus及affinity，执行时才取logical min/max。flow是D2 readable投影；它不承诺对null片段提供逐scalar可写映射。只读导航另用下面的origins，不以null等于没有来源。

**plainRegion** exact `{sourceRange,text,parentPath,childStart,childEnd}`。sourceRange是完整源中的连续范围，text必须逐scalar等于该raw slice（保留space/tab、LF/CRLF/CR）；parentPath终点为body或section的children槽，半开ordinal区间[childStart,childEnd)只含普通paragraph。区域可没有paragraph，text可为空或只有blank/EOL；它不是D2空paragraph或持久对象。

Core按已完成D2解析的词法行和block stack构造最大连续区域：只包含同一parent下无anchor的普通paragraph行和其间/前后的blank trivia；paragraph的每个非blank行都由未加工的Inline.text贡献，显示只允许D2规定的行间join space。blank严格仅ASCII space/tab或空串，NBSP/NEL/LS/PS不能当blank或EOL。paragraph行、blank content和完整terminator都可在区域内；一条CRLF永不分开。区域不得包含header（包括其terminating blank）、实际carrier frame、comment、任何anchor附着区、heading/list/table/protected/atomic/escape语法，也不能跨parent。最后carrier之前的discovery trivia不暴露；最后carrier之后的普通blank trivia可暴露。用D2词法证据生成，UI不得扫描正文猜区域。

区域边界由上述完整行确定；相邻合格行在同parent合并成一个region，数组按source顺序且不重叠。不把paragraph之间的空白吞成隐藏结构：全部raw space/tab/EOL都在region.text中，末尾EOL也保留。空区域只在已物理结束header/carrier/comment之后、column0的EOF普通body起点提供，且该点前的必要separator已完整存在；不增加实体。非0列EOF、header terminating blank还没有EOL、紧邻非plain block/anchor的插入点只提供site，由insert_at_site生成分隔，不能直接以zero region splice接到结构中。相同zero-width point不重复。一个region包含多行paragraph时，D2 Read投影仍按原规则用space连接，Write的普通文本输入宿主则显示实际行结束和空白行，可用可访问的“源换行/段间空行”提示区分。普通内容编辑采用这个Core给出的文字区域，而不是让客户端从rich DOM重建源。

这是本代Write的明确呈现代价：普通区域中可看见作者实际换行，单EOL与空行分段的区别不会被隐藏；metadata、carrier和其他结构仍不作为普通文本编辑。宿主不得把CRLF/CR全部归一成LF后回传整块。可用通用scalar/UTF16/line-ending位置适配保持native输入，仅把实际发生的replacement送Core；这不是D2 parser。Read和“复制文本”仍消费D2 logical readable text；另有“复制源片段”才复制region raw。

**site** exact `{sourceRange,parentPath,childIndex}`，range为zero-width。Core枚举合法body sibling插入边界：header与最后carrier结束后的body起点、每个body/section child及其附着anchor/trivia整体之前、普通blank line起点、section实际stack确定的末尾，以及完整source的实际EOF。一个point只取其词法block stack确定的parent/ordinal。禁止carrier frame/anchor附着区/非paragraph结构内部；不得因有后续carrier而在其前面插body。实际EOF可为非0 scalar column；空body的EOF为body.children ordinal0，即使header或closer没有EOL。site不表示可直接向title/header/closer接字符，插入须经过§3.4。

region边界处的zero-width位置可同时是site，但caret优先采用region坐标；site提供删空后即使紧接protected block仍能重新输入的插入边界。每个成功普通编辑的caret必须在返回region或site中，不能依赖旧Draft历史才能找回。Source全文替换、草稿重新打开后从新parse即可重建同样资格。

**导航来源**为独立必填的`origins`，exact `{elements,flows}`，与editMap共享§3.1完整绑定，仅用于定位，不授编辑权。

- `elements`按schema顺序，给每个D2可地址body occurrence恰一个`{path,sourceRange}`。范围是Core解析该occurrence得到的完整lexical source span；section包括其范围，protected block包括实际frame/payload，不解析payload内部同名key。heading/paragraph/list/item/table/row/cell等分别有自己的范围；没有locatorToken也必须能生成。
- `flows`给每个editMap flow恰一个`{path,segments}`；导航segment exact `{start,end,sourceRange,relation}`，完整覆盖flow.text，空flow为[]。relation闭集为`scalar|escape|join|atom`，sourceRange永不null。scalar按上面的差值映射；escape覆盖产生该显示字符的完整raw escape；join覆盖该显示空格所替代的完整原EOL；atom覆盖该Inline宏的完整原文，不把label字符误认为原文字符。Core在生成IR时保留来源事件，不能事后搜索相同text/宏字面值。
- 除相邻scalar且raw连续外，不合并不同来源事件。重复原子各占一段，两个相同escaped cell各用自己的path/range。origin coverage可以有nested元素重叠，不能以重叠证明编辑权限。
- “定位到Source”：元素选中其element range；flow非空selection选择所有相交origin片段对应的source范围，scalar片段按选中部分裁剪、其他relation保留完整range，然后取最小包含范围并明确显示所包含原始标记；collapsed点用已选segment及offset/affinity，scalar给精确point，escape/join/atom选中完整origin range。空flow使用containing element range。不假称非scalar relation有字符一一对应。
- Origin只针对返回的Draft source，不能生成D3 AnnotationTarget、延续旧revision、越过当前读取授权，或替代编辑资格。invalid投影没有部分origins。来源存在不使readonly内容可写。

`mapBinding` exact `{ownerNodeRef,baseRevision,draftSerial,source}`，从消费的projection逐字复制。Core核对与请求的四项一致，再对本次source重建全部map/origins；旧binding配新source/serial为invalid_request，当前author revision不符仍按原顺序stale_target。UI不能改标签延续旧map。冗余source计入运输/内存预算。

### 3.3 安全片段替换

请求exact `{wireVersion:1,kind:"d8_draft_text_replace",workspaceRef,ownerNodeRef,expectedSourceRevision,draftSerial,source,mapBinding,flowPath,segmentIndex,start,end,replacementText,budget}`。Core重建当前editMap，验证该flow/segment存在、nonnull、start≤end且含于该segment。两端必须为当前flow Unicode18 extended grapheme边界，不能从segment内独自分段绕过组合字符。replacementText可空，不含CR/LF。按映射的唯一source range替换。

新source须完整D2-valid，去除locator/坐标后新树恰为原树该text片段替换；比较只合并相邻Inline.text，不删除paragraph或默改结构。metadata/carriers/raw及未触及字节完全保持。新macro/结构marker/delimiter或不能证明对应，一律unrepresentable_text。删空会去掉paragraph时使用§3.4 splice_plain，不能把不满足树等式的结果当本入口成功。

成功exact `{wireVersion:1,kind:"d8_draft_text_replaced",ownerNodeRef,baseRevision,inputSerial,projection}`。inputSerial回显请求draftSerial，projection是新source的完整valid投影，draftSerial严格为inputSerial+1，拒绝溢出。controller只在原owner/base/serial/source仍当前且该输入事务待处理时接受并采用这个新serial；不能给返回projection另贴一个不一致serial。

### 3.4 完整普通文本状态的最小命令

请求exact `{wireVersion:1,kind:"d8_draft_write",workspaceRef,ownerNodeRef,expectedSourceRevision,draftSerial,source,mapBinding,command,budget}`。command闭集如下；一次命令只形成非author提案，最终prepare独立执行§4。

| command.kind | exact其他成员 | 操作 |
|---|---|---|
| splice_plain | regionIndex,start,end,text | 在region.text的scalar半开区间[start,end)替换为text；text可空、可只有space/tab、可含完整CR/LF/CRLF；不另加/删空白 |
| break_plain | regionIndex,offset | 在区域合法caret处插入两个按下述政策选择的EOL，即普通Enter；选区非空时先用同一splice_plain事务替换为这两个EOL，不分成两个undo group |
| insert_at_site | siteIndex,text | 无region的插入点或显式新增正文时，为text构造最少必要的prefix/suffix EOL并检验；text非空，可为blank或含EOL，不要求必定产生paragraph |

region的start/end/offset必须位于该完整region.text的Unicode18 extended grapheme边界，CRLF视为不可分EOL。选择反向时只交换range端点，不倒排text。Source码点工具不绕进这些正文命令。所有普通文字、space/tab、paste、IME最终输入、删除最后非blank、跨空行选区删除均用splice_plain；Core只接受满足以下完整区域等式的结果。

**完整区域等式**：只替换指定source slice，原slice之外每个byte保持；完整D2重新parse必须valid；metadata、carriers及所有原区域之外的body结构/anchor/Inline/raw值在仅去除locator/坐标后相等；原parent的[childStart,childEnd)只可替换为0或多个普通paragraph。新区域的每个非blank行仍须是未加工text，D2只可按原规则连接同paragraph内的logical lines。不能新增macro、comment、heading、list、table、protected或anchor，不能改变其他parent。纯ASCII blank/EOL不产生paragraph，但仍保留为确切Draft source；NBSP等依D2正常text处理。Core必须证明新输入的全部scalar来自这次raw splice及原有region，不能用“解析valid”替代词法/树差异证明。

splice可以让一个paragraph变多个、多个变一个、或全部成为blank；也可以把两EOL删成一个，产生D2多行paragraph，其新region仍可继续编辑。这保证普通输入的后继状态闭合，不依赖单行paragraph索引。拒绝范围仅为确实跨受保护结构、产生非inert语法、字素非法、权限/版本/预算失败；空白本身不能被标为unrepresentable。旧paragraph流path可变，必须用返回map重新绑定。

**EOL政策**：在插入点所在logical line有原terminator时用它；否则取之前最近完整terminator；否则其后第一个；全source没有则LF。仅新生成的EOL用此政策，原有EOL和明确paste文本不normalize。break_plain插入两个EOL；Backspace/Delete按logical preceding/following一个字素（完整CRLF为一个）生成splice_plain，包括空行/区域末尾的EOL；因此一次Backspace不保证撤销整个Enter，撤销整个输入组是Draft undo。删除一个EOL后可能仍有源换行，Write显示并允许继续删除。段落“合并”显式选择Core给出的两个段落间raw EOL范围形成一次splice，不丢任何未选space/tab。没有输入日志的重新打开草稿执行相同规则。

**插入点构造**：insert_at_site用同一EOL政策，prefix/suffix各从空串、一个EOL、两个EOL选择，共9个候选；插入`prefix+text+suffix`，其余raw保持。完整parse后只可在指定parent/ordinal新增0..N普通paragraph，其他语义不变；text的整个插入span必须属于新plainRegion（包括空白和EOL），不能属于header terminator、comment、carrier或其他结构。新caret位于text末尾，必须落在新region/site。选择新增prefix/suffix EOL总数最少，再prefix数最少的候选，无解才unrepresentable_text。这也闭合`= T`、header attribute/comment EOF和carrier closer EOF，不把首字或space接入它们。普通blank line已有region时优先splice，保留其全部未选空白。没有region的site处Enter以insert_at_site(text=两个生成EOL)执行。

成功exact `{wireVersion:1,kind:"d8_draft_written",ownerNodeRef,baseRevision,inputSerial,projection,caret}`。projection是完整valid新投影，draftSerial=inputSerial+1，溢出拒绝。caret闭集：`{kind:"plain",regionIndex,offset}`或`{kind:"site",siteIndex}`。首选caret是本次替换/插入text末尾的绝对source点；先映射到新region（多个边界候选取source较前region，zero-width重复已禁止），没有region则需精确site；若新组合字素包含首选点，移至该完整字素logical末端，不能切cluster或改source。Core无法证明结果caret则不成功。no-op source可返回同内容的下一个serial及caret，但controller不创建内容undo group；计数仍不回绕。

一次实际source变更是一个Draft undo group，保存完整before/after source和selection。undo/redo恢复时分配新的单调serial并重新请求projection，不能恢复旧serial或重用旧map；原selection按exact恢复source重新绑定。它可以恢复纯空白或无paragraph状态，重新输入仍可用。所有返回仅完成当前匹配的owner/base/source/inputSerial事务，不能给projection另贴serial。

### 3.5 输入排队与迟到响应

每个controller最多一个变换请求在途。native事件按稳定host输入generation和原selection形成有序本地事务队列；在途期间后续文字/preedit可显示为明确未验证输入，不能标成当前Core投影，也不能prepare。后续事件保存完整native前后文本及selection，不能仅存旧Core scalar offset再盲目重用。先接受前一结果，再以其caret及该host输入日志的精确差异定位下一事务并请求新map；无法证明衔接时停止队列，保留全部输入并显式处理，不静默丢弃或猜目标。

迟到结果只完成其所属事务，不能把旧rendered text回写覆盖native host中更晚输入。composition的最终buffer形成一个事务，composition期间不运行结构命令或提交；队列排空、当前source/serial投影一致且idle才可prepare。取消、撤权、切owner、重连后旧generation的结果无权改变新会话。Source键入仍可直接修改exact scalar buffer，其变更让旧map失效；客户端不因此取得parser权威。远端离线只允许Source或保存未验证native输入，不产新Write projection。

普通space/tab必须成为已接受的blank Draft状态，不以长期pending-prefix代替。只有真实Core等待、composition或确实不可表示输入才保存在未验证队列；native文字的“当前显示”与已接受exact Draft source分别标明，不能误报保存。

## 4. 编辑准备

请求exact `{wireVersion:1,kind:"d8_edit_prepare",workspaceRef,intent,budget}`。intent闭集：

| kind | exact额外字段 | 允许的author变化 |
|---|---|---|
| document | `target:EntityTarget, source:text` | target.ref必须NodeRef；替换这一既有Document的完整源；不改变Ref/parent/ordinal/lifecycle/Resource/Annotation身份 |
| annotation | `target:EntityTarget, value:D3-Annotation-Value/3, targetPolicy` | target.ref必须AnnotationRef；显式annotation_read/write、目标state/locator及source_envelope_state、Workspace commit_sequence_state资格仍分别验证；修改这一既有Annotation值；targetPolicy=`preserve|replace_current`；同owner，不创建/删除对象 |

document使用真实旧源和expectedRevision；Core独立计算完整diff/MutationFootprint，按D6能力矩阵判断实际Field/body/title/Node声明权限及deny，不以UI模式或caller说“只改正文”替代。全源读取和full preview资格必须成立；所有将披露版本/源状态的完整目标须显式source_envelope_state，正常提交回执须Workspace的commit_sequence_state；它们不由source_read/write隐含。workspace_constraints潜在观察范围在作者求值之前选定并授权。本入口**不支持owner_fields**，phone-only主体继续D7窄路径。全源source_write不豁免node_control或细项deny。

document执行D2完整profile、D4完整operation-applicable gate（含raw保持的unavailable namespace例外）、D3原typed引用/owner验证，以及实际保存定义的D7解析/版本/闭包/类型门。D7保存定义门在这里明确为：按实际format完整closed decode、全部静态类型/引用地址/Registry及定义闭包检查，保存参数可保留声明未绑定的调用参数；不执行Query terminal或DynamicBlock。新增/修改或原本存在的不可解码D7 payload使本版document prepare整体拒绝，原raw仍可读取并留Draft；用户可在明确Source提案中修复或转为普通literal block后再准备。这是本适配器保守准入限制，不把D2 inert payload改成D2-invalid，也不声称已经支持无关body修改同时保留不可解码D7定义。

普通Source编辑是作者对完整源的显式提案，Core不能从结果字节证明其由人手工输入。D8 v1明确关闭全部UI生成的native列/多row/ragged批量Source路线，包括菜单、快捷键、拖动、TSV粘贴、表单或把批量操作拆成多个D8 prepare；这些命令不得调用document入口。保留D7既有set_native_cell、insert_native_row、remove_native_row、reorder_native_rows、toggle_checklist、promote_native_row、promote_checklist及真正通用Source编辑；逐项使用其原完整形状与门，尤其插行不是单个NativeSelector，既有重排不因涉及多行而关闭。1000-row结构操作上限仅由有完整结构意图的原D7/D5适配器执行；本通用Source入口不从diff反推意图，不按修改行数执行该结构限额，仍完整执行D6 source/write/decoded/work预算及实际footprint权限。没有尚缺的native-column/多row批量生成适配器，就不能声称D5全部结构功能已实现；这不删除现有reorder_native_rows。此为产品能力边界，不是防止任意外部程序生成Source的安全承诺；本接口无“人工来源证明”字段或伪造的Core校验。

annotation先取得annotation_read/write、entity/locator-state和full workspace_constraints资格，再读取当前完整Value/3。targetPolicy=preserve必须target逐字等于旧target；replace_current必须明确提供同owner新的合法target，不搜索或自动选择候选。两者的proposed target均必须在当前author revision精确resolve；旧stale target不因此自动变新。reply同owner且acyclic；purpose/body/suggestion沿D2原closed语义。suggestion的range与expectedDocumentRevisionToken须绑定同一当前Document，新target可显式保留原replacement作为新提案，但须完整显示其上下文及效果；不会自动接受建议或改Document。Field/carrier target拒绝，即使客户端把它编码成普通document range。

准备顺序固定：closed decode及静态Workspace/Ref关系 → 当前principal和先验scope/全preview资格 → authority/cut → current exact target/revision及proposed输入 → 独立实际footprint与权限 → 全部适用语义门/依赖/预算 → 完整效果与immutable PreparedEditBinding → 返回prepared。失败没有ledger或作者写。D6 commit在原步骤6/7/8检查同一计划与依赖；准备成功不保证提交。

`PreparedEditBinding/1`是Core内部不可变记录，exact语义成员：`kind:"d8_prepared_edit_binding",version:1,workspaceRef,operationId,principalAudienceToken,intent,origin,sourceInputs,proposedInputs,registryInputs,dependencyProof,observationProof,budgetBinding,expiresAt,request,preview`。origin为closed `{kind:"direct"}`或`{kind:"undo",originalRequest:<原D6请求>}`；后者只由§7的Core适配器产生并保存其完整历史证明。source/proposed/registry/dependency/observation/budget/expiry各使用D6 PreparedIntent原完整类型；request为原d6_commit_request，preview为完整EffectManifest语义清单及pins/交付epoch。它不是portable作者对象，不向client接受自报proof，不另建ledger。

生成顺序：Core生成新UUIDv4 OperationId与tag=plan的D6 planToken，构造完整D6 request，保存PreparedIntent及嵌入的完整binding/最小授权定位记录和pins，生成并固定完整preview投影，全部成功后才返回。这里不使用D7 bindingToken，也不伪造ActionSpec或D3 preparationBinding。

成功exact `{wireVersion:1,kind:"d8_edit_prepared",request,previewToken,previewCursorToken,previewManifest}`。request严格为D6原commit request；previewManifest严格为D7 EffectManifest去items的完整header，protocolOwner=D6、phase=preview、profile=full。tokens/cursors/bytes及完整当前授权、TTL/epoch、读取与恢复完全复用同包修订的D7运输，无D8第二效果系统。

## 5. 提交、效果与独立上游修订

确认只发送原d6_commit_request完整对象，所有client serial/模式/方向/快捷键都不进入此request。任何source、target、permission、base或意图修订都要重新prepare、新OperationId；原已submitted_unknown/planned先恢复决议。相同canonical request重放只读原decision或恢复其原plan，不能重做。prepared未提交时原TTL到期禁止新decision，已有saved/planned按D6恢复规则不受preview TTL删除。

唯一上游语义组合修订为D7 Preview and Effects Transport的producer范围：D7 PreparedActionBinding与D8 PreparedEditBinding均可作为preview的不可变输入。D8只允许D6/full，既有EffectManifest、EffectItem、byte encoding、tokens、epoch、错误与committed receipt运输全部不变。D6已允许显式closed编辑adapter创建PreparedIntent，本文件补齐该缺口，不改D6 wire/Policy/ledger；D3不变。

完整效果由真实before/proposed独立恢复，document只有该owner source_change及原适用D4控制effects；annotation只为该Annotation source_change。不允许source intent任意写control；如D4/D6原规则要求派生period scope更新，其明确效果及授权必须完整显示并同事务绑定，不能隐藏在companion。无实际raw变化时source_change为空，D6可能保存一个raw no-op committed decision并按原规则增加commitSequence；UI必须显示“未改变内容”，不能假称revision增加。按完整source字节确认，不按渲染结果判断no-op。

效果页200行仅运输，确认前必须提供总作用范围及可完整获取的源/效果，UI不能把已看到一页等同读过全部。状态`preview complete`只在manifest所有页及所需bytes完整decode后成立；人可以在清楚总范围后明确确认，软件不声称已逐字人工审阅。缺页/不可取/epoch混用时确认禁用；不自动重新prepare新效果并沿用旧点击。

## 6. 错误与例子

D8未提交入口错误exact `{wireVersion:1,kind:"d8_editor_error",code}`，code闭集：`invalid_request|not_visible|authority_unavailable|source_unavailable|stale_target|ambiguous_selection|unrepresentable_text|semantic_rejected|budget_exceeded`。无隐藏target、source、counts、locator或自由details。顺序为closed decode→当前授权与scope→authority→target/source可用性→已获locator/sourceEnvelope资格后的revision/selection→语义→预算/发布；budget耗尽可在任何需要资源的阶段终止，不能用未完成计算猜semantic错误。full source draft parse invalid是§3带D2 diagnostics的成功投影结果，prepare invalid是semantic_rejected；它们不混称commit成功。D6/D7/D3原错误不包装成D8新disposition。

以下是真实可序列化的D8输入示例，BudgetBinding由已受管资源描述提供，例中只展示intent以避免伪造预算对象：

```json
{"kind":"document","target":{"ref":{"kind":"node_ref","workspaceId":"9dfadab9-5f7a-4ba9-a6d4-681b29645283","nodeId":"c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"},"expectedRevision":7},"source":"= العربية\r\n\r\nمرحبا 123 שלום 中文\r\n"}
```

成功prepare返回唯一planToken对应的原D6请求；切换shell RTL或选择另一个pane不会修改该request。若当前源从revision7到8，即使新body仍是同一句，原intent prepare返回stale_target；若当前主体先失去资格则not_visible而不是透露8。

负向wire：`expectedRevision:true`、`7.0`、`7e0`、`-0`、unknown intent member、Node target放annotation意图、跨Workspace owner、malformed Unicode、旧D3 locator放draft range、以未知code继续、把D8 intent塞进D7 ActionSpec，一律不得成功。D8测试只证明列出的边界，不能以样例数量声称完整D2/D4/D7引擎实现。

## 7. 有界已提交撤销入口

`d8_undo_prepare` exact `{wireVersion:1,kind:"d8_undo_prepare",workspaceRef,originalRequest,budget}`，originalRequest必须是本Workspace中原完整D6 commit request。它是独立准备读取，不执行/重放原提交；结果仍为§4的d8_edit_prepared，最终提交使用新OperationId。

Core先closed decode，按原D6最小授权定位及当前workspace_constraints/full preview/历史读取/metadata资格，再authority/custody、原key与canonical request比对。只接受已committed、由D8 direct或本undo适配器产生、且恰一个实际source_change的document/annotation编辑；D7其他动作、D6控制意图、raw no-op、非committed、缺完整before/after pins或不能证明原request相等均semantic_rejected（更早权限/authority门仍优先）。普通入口不回显另一请求/历史值。

必须证明该entity当前sourceVersion与原receipt记录的after版本相等、当前值逐字等于原after、当前仍live，且没有需逆转的identity/lifecycle/placement/period scope/series配置或其他control effect。否则拒绝本次Undo并保留当前内容；即使同owner后来只改无关字段，也不自动rebase。此保守v1范围公开显示为“该操作已被后续编辑改变，不能直接撤销”；可另行完整当前Source提案，但不能叫自动Undo。

Core以当前版本及原before构造新document/annotation intent，origin保存原request和内部已核验历史绑定。Annotation拟议值仍须其target在当前Document/Resource revision精确成立，stale旧target不能因Undo复活。按§4完整重新执行当前权限、footprint、D2/D4/D7语义/依赖与preview；历史不是写授权，且旧原source版本从不恢复。新committed Undo增加新source版本；Redo只对这次Undo的committed request运行同一入口，因此仍是新计划、新preview、新OperationId。

本版不声称提供任意D7字段/批量/lifecycle/跨Workspace历史的统一Undo API。D7 Field操作可在明确当前选择后提出相反的普通Action，但不以此绕过其selector、权限或历史冲突；D3 Trash恢复使用本来就明确的restore intent。设备Draft撤销仍不调用此入口。

## 8. 具体边界构造与实现验收

这些构造是规范义务；附包有限模型只验证明示的小范围，未运行完整Core。完整source、typed upstream decoder、当前authority/permission/cut始终是前提，不能用抽象token fixture宣称D6实现已通过。

### 8.1 完整snapshot正反序列化

下面两个JSON是完整D2 snapshot，N为示例NodeRef；使用真实授权cut的W、SourceVersion(N,r)、D6 document token DT和authority token AT构造唯一D8 envelope：`{wireVersion:1,kind:"d8_document",workspaceRef:W,ownerNodeRef:N,sourceVersion:{entityRef:N,revision:r},documentRevisionToken:DT,authorityToken:AT,snapshot:<以下完整对象>}`。DT/AT必须由真实Core提供，不能拿示例占位字串作为有效token。所选snapshot与metadata必须是同cut的实际源。

```json
{"wireVersion":2,"kind":"document_snapshot","ownerNodeRef":{"kind":"node_ref","workspaceId":"9dfadab9-5f7a-4ba9-a6d4-681b29645283","nodeId":"c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"},"document":{"source":"= T\n\n","parse":{"status":"valid","diagnostics":[]},"projection":{"state":"available","metadata":{"title":{"value":"T","sourceRange":{"startLine":0,"startColumn":2,"endLine":0,"endColumn":3}},"subtitle":null,"headerAttributes":[]},"attributeCarrierBlocks":[],"body":{"kind":"document_body","children":[]}},"d2CommitEligibility":"eligible","repairVisibility":"not_required"}}
```

```json
{"wireVersion":2,"kind":"document_snapshot","ownerNodeRef":{"kind":"node_ref","workspaceId":"9dfadab9-5f7a-4ba9-a6d4-681b29645283","nodeId":"c8a4f2d3-3d0e-4e6c-a9a5-e2cf61ba467f"},"document":{"source":"broken","parse":{"status":"invalid","diagnostics":[{"family":"missing_document_title","orderRank":10,"sourceRange":{"startLine":0,"startColumn":0,"endLine":0,"endColumn":0}}]},"projection":{"state":"unavailable"},"d2CommitEligibility":"reject","repairVisibility":"exact_source_only"}}
```

负向逐项：D8直接含document而无snapshot；snapshot少wireVersion/kind/owner；仅改变snapshot.owner或SourceVersion.entityRef；D8 wireVersion改2或内层改1；snapshot/document/必填Ref置null；invalid同时带body或两项diagnostics；源physical bytes=`FF`未能strict UTF-8 decode。前七类不接受为成功响应，最后一种连D2 payload都不存在，返回source_unavailable。合法D2内部subtitle=null等仍允许，不从D8外层反推全局禁null。

### 8.2 显示/源映射构造

source=`= T\n\n甲\n乙\n\n甲 乙\n\n甲 乙\n`。flow paths依次children[0/1/2].inlines，三个text都为`甲 乙`；第一个segments显示[0,1)→source line2[0,1)、[1,2)→null、[2,3)→line3[0,1)。第二和第三各单segment [0,3)→line5及line7[0,3)。不能按相同text查找，第一个generated space不可写。

table source line `|甲\|乙|甲\|乙`的两个cell均显示`甲|乙`。两个flow分别rows[0].cells[0/1].inlines；第一个显示甲→raw列1、乙→列4，第二个甲→列6、乙→列9，两个显示pipe各为null。相邻Inline.text按拼接text累加offset，不因lexer切成两个text节点产生另一内容身份。非text Inline在flow占一个readonly U+FFFC，label可读但不能从glyph反推可写source。

反向selection anchor=段末/focus=段首保留方向；删除使用min/max logical range，replacement从不视觉倒排。旧owner/base/serial/source任一变化拒绝复用mapBinding。same-title/same-value/同source ABA不能代替版本证明。

### 8.3 普通写作闭环

E分别取LF/CRLF/CR，文字分别取Arabic、Hebrew和mixed。完整闭环须包括：

1. title/header/carrier EOF无body时insert_at_site首字或ASCII space/tab，完整保留前缀；已有blank region直接splice。空白必须立即进入已接受source并有plain caret，不等待以后出现非blank。
2. `= T\n\nم `中删除م，结果严格为`= T\n\n `，body没有paragraph，caret在blank region的offset0，space不丢；继续输入ש得到`ש `。
3. Enter插入两个EOL；在新的空行继续Enter、输入space/tab、Backspace/Delete逐完整EOL删除，可经过一个多行paragraph及重新成为单行的状态；重新打开当前Draft后同样能操作，不用历史Undo冒充删除命令。
4. 前后空白split、选择反向、跨空行删除和合段保留未选space/tab及mixed EOL；清空region后返回plain/site，继续首字；每一步undo/redo重新分配serial并投影。
5. 队列仅接受本事务的exact绑定及下一个serial；A结果迟到不能覆盖native后续B；composition期间不能prepare。最后才由真实D8 prepare、完整D7效果、明确确认及原D6 receipt证明作者提交。

负向包括macro/结构/comment输入、越过header/carrier/protected/anchor、split或splice使inert text变语法、旧map、跨owner/base/serial/source以及字素/CRLF内端点；完整拒绝并保留用户待处理输入。有限模型不实现完整D2/D6或真实host。

### 8.4 Native批量边界

1000与1001行的列插入/删除、多row、ragged/missing-cell、重复值、混有trivia、旧revision、额外未选变化，全部没有D8 UI生成Source的入口，故不发对应document prepare；不得自动拆成1000+1或多次单row提交。D7既有set_native_cell、insert_native_row、remove_native_row、reorder_native_rows、toggle_checklist、两种promotion和bulk_field各按原合同单独验收。有效两行表的完整无重复重排在具备原trivia证明时进入原D7 prepare；无证明仍按原错误拒绝。1000/1001测试只针对未闭合批量生成入口，不将普通行重排当作该禁用入口。普通Source显式更改1001行没有这个结构计数限制，仍受完整实际footprint、权限、source/decoded/work预算及语义门；不得把本条解释成证明来源为人工或放宽原D6授权。

### 8.5 EOF、空白与来源验收族

合法空body的EOF族：title-only、一个EOL、已终结空header、ordinary attribute EOF、header comment EOF、whitespace tail、discovery comment EOF、title前comment、最后carrier closer EOF、carrier后blank；跨LF/CRLF/CR、BOM有无、Arabic/Hebrew/mixed。`= T`的EOF是(0,3)，insert_at_site(م)得到`= T\n\nم`；`= T\n`须再新增一个LF建立separator；实际header/closer原值不可变。既有blank tail用region splice保留space/tab，不再强迫追加到新paragraph。

来源族：重复paragraph、两个同label原子cell、相同escaped cell、仅escape的flow、generated join、每种protected block、与正文相同token藏在protected payload。对Source全替换/恢复dirty Draft且无可用Base locator的输入，Core仍提供准确elements及flow origins。导航选择原macro/escape/EOL或element范围，绝不把来源范围偷换成写资格或D3 annotation locator。
