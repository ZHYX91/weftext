---
_weftext:
  id: "a26b84bd-10b4-448c-9037-d5cb0bd65a4b"
---

# D9 接受矩阵与场景裁决

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


revision: D9-r04-2026-09-24；candidate。D9-01至05均在本包裁决，没有用未来D10替代转换事务或模板语义。架构门禁尚待实际独立终审。本矩阵的“有限验证”指validate_d9_r02.py、validate_r02_contracts.py及validation-r02对应结果；“规范检查”不是产品测试。

| 原问题 | 本代结论 | 证据及限度 |
|---|---|---|
| D9-01 | 自有封闭ImportIR，page/flow/workbook分支；PDF与真实XLSX源验证第二种不同结构；共同来源、资源、loss和Core映射 | 两类源bytes及提取IR、独立pypdf/openpyxl读取、负向schema/CSV；Flow仍待真实DOCX/ODT语料。 |
| D9-02 | Provider/Route固定版本、严格sandbox；PDF现有adapter未生产就绪；XPS候选MuPDF；OFD候选OFDRW；CAJ/HN无执行profile，保留惰性附件 | 一手来源调查和本地inventory；未安装运行这些候选、不作许可批准，不声称Windows/Linux覆盖。 |
| D9-03 | 普通文本token、精确ASCII空格、单次转义、不重扫值、closed namespaces、none policy、单row/column band、↓/→逻辑正向、格式一致继承 | 有限token/run/repeat反例通过；完整Office parser和互通待实施。X-001由此替换为明确合同，非历史candidate自动转freeze。 |
| D9-04 | H1–H5唯一映射、显式outline、可见style样板、body/bibliography唯一放置，不按localized style名猜 | DOCX/ODT XML片段结构验证；未声称在Office/WPS/LibreOffice渲染通过。 |
| D9-05 | document/native_table/node_collection/query_rows/query_json显式域；typed snapshot、projection/loss、精确数字/日期策略；外部输出与Resource提交分开 | 规范矩阵及边界分析；CSV有限解析、typed grid证据；真实导出器/发布故障/权限竞争待实施。 |

## 完整压力场景

| ID | 输入与反例 | 必需结果/权限与事务不变量 | 本轮证据 |
|---|---|---|---|
| S01 | 双页PDF有表格，用户普通导入 | 默认一个document Node，不自动每页Node；表格文字与几何观察不自动Field | 实际PDF、两parser、图像目视 |
| S02 | XLSX numeric16位、空格、blank/absent/empty、formula有/无cache、hidden、merge | lexeme/坐标/源类型保留；无重算/复制merge值；映射显式loss/拒绝 | 实际XLSX及独立openpyxl |
| S03 | CSV重复header、quoted newline、ragged、空文件 | ordinal mapping、保留文字；ragged拒绝；空文件不伪造一行 | 有限实际CSV |
| S04 | 扩展名PDF但bytes不是PDF；含DTD、zip traversal、zip bomb | probe识别不授执行；安全parser/隔离失败，无author写 | 规范；攻击语料待生产 |
| S05 | worker挂起/子进程继续写/退出0但IR不完整 | kill全树后才清理；unverified输出不可pin/commit；无工作区挂载 | 规范；OS隔离待实机 |
| S06 | CAJ-HN、缺MuPDF许可或OFDRW依赖 | 明确unavailable和D1规范reason；可明确存不执行原件；不换后缀fallback | 一手调查/规范 |
| S07 | Office中macro、公式、OLE、external rel、签名模板 | 本代模板整体拒绝，不执行/去除后伪称成功 | 规范；恶意模板套件待实施 |
| S08 | 同token跨同style和mixed style runs | 前者逻辑识别，后者template_invalid；不first-run取样 | 有限run模型 |
| S09 | `{{meta.title}}`、NBSP、attr/record旧alias、值含模板语法 | 无效语法拒绝；显式四brace转义；替换值不重扫 | 有限scanner |
| S10 | 未知path、none、empty text、多occurrence Field | unknown/ambiguous阻断；none需精确missing policy；empty可为空；导出报告绑定实际binding与模板来源 | scanner与r03有限导出模型 |
| S11 | ↓/→、0/1/N、RTL、cross merge、公式、超行列上限 | 逻辑索引一致，零删除，预检footprint，跨band/公式拒绝 | 有限repeat模型 |
| S12 | H1–H5、source H6、localized Heading label伪标题 | D2仅5级；显式outline映射；source H6需loss/拒绝 | XML片段+规范 |
| S13 | 同时body内部bibliography和独立slot | 唯一完整渲染，明确relocation；多个placement拒绝 | 规范；实际citation renderer待实施 |
| S14 | 模板插入1001个Nodes，或Node+资源被分到不同批 | 不可拆组整体拒绝；不先落前1000 | 有限partition模型 |
| S15 | 10,000独立Node，第六批commit前失败/commit后失去response | 精确前缀5/6；原OperationId重放，无重复Node | 10批模型、隔离SQLite；非真实D6 crash |
| S16 | 用户把同文件导入两次 | fresh两组identity；digest/row coordinate不是upsert key | 规范及D3原合同 |
| S17 | 原件Resource保存到既有Node | 原D3 create_resource，owner和完整写权；不能ordinary import借existing owner | 原D3矩阵规范核对 |
| S18 | proposal前后source/Registry/schema/route/模板/权限变化 | 旧准备失败或reset；完整新preview，不能沿旧确认换输入 | D6/D7/D8规范核对 |
| S19 | Server只许读Field但导出包含完整body或隐藏Query依赖 | prepare前整体拒绝；不能先提取后遮蔽；不把unreadable变none | 原授权规范核对 |
| S20 | Query第一页空nonterminal或权限换代 | 继续直到真正terminal；换代整个export reset，不混前后epochs | D7规范核对 |
| S21 | 同值bag行、无序rows、rowHandle变化 | bag保留；显式稳定sort/tie keys，否则拒绝；不导出rowHandle | D5/D7规范核对 |
| S22 | CSV危险公式头、none和empty、复杂值、长数字 | 危险text拒绝；ExportLossReport完整绑定固定结果/投影/文件与确认；typed sheet/JSON可替代 | 82项中有限报告/确认片段；目标应用安全待实施 |
| S23 | 外部发布rename后crash、目标被别人创建/移动 | create-only；原intent恢复可证才published，无法证unknown，不换名字重复写 | 规范；真实文件系统fault tests待实施 |
| S24 | 已发布外部文件但create_resource失败 | 两个独立真实结果，不生成综合原子receipt或删除用户文件 | 规范 |
| S25 | Template的tasks/task事实复制到target普通Node | 移除wf-kind，targetFacets+facts完整D4校验；Template源不改，参数不exec | D2/D4规范核对 |
| S26 | Node Template复杂closure来自集合入口且membership不成立 | 本代复杂集合实例化unsupported；简单collection_create完整requireMembership | D7规范核对 |
| S27 | 模板/转换proposal遇到D8 dirty Draft | 保持generated来源；明确接收/冲突，原full scope与workspace_constraints，不无声覆盖 | D8规范核对 |
| S28 | PDF旋转/crop、image EXIF、旧revision区域、copy/fork | d9rg1纯几何＋原l1准确Ref/revision；不同内容stale，fresh外层重签，不存在第二身份 | D3/D9语义核对；实际解码待实施 |
| S29 | Mobile上传触发转换、委托/批准Server conversion | D1 unsupported_surface；Mobile仅普通附件和已commit结果 | D1规范核对 |
| S30 | 全选Native grid批贴、rich HTML、existing generic patch | D8禁用项保持；本代仅显式新Node文件转换/plain粘贴，不拆为七actions规避 | D5/D8规范核对 |
| S31 | custom people/account service与identifier相同/未知预设 | 不从label/note猜服务；首代scalar mapping对此unsupported，保留原件；未来typed mapping须D4 preset/custom精确分支 | D4来源与限制核对 |
| S32 | ICS UID/RECURRENCE-ID、自带Node UUID、自动upsert | ICS生产profile未开放；本代不把外部stable id当D3 identity，不创建OriginBinding或sync写权 | D3/D4边界核对 |
| S33 | 只拿IR schema绿灯却提取漏掉sheet/隐藏对象 | coverage是独立门；无法证则invalid_output，所有已发现未支持内容列issue/loss | 真实fixture有限对照；生产覆盖待实施 |
| S34 | mapping/construction改变导致新损失、分析过期、准备只给计数 | 重新完整分析并重置旧loss接受，完整新choice必须用户裁决；完整源/资源/分组可inspect，原D7全效果仍必需 | 规范 |

## 证据归属

当前validation-r02/results.json为63/63，contract-results.json为59/59；两组是不同有界子集，不合并声称产品场景通过。旧validation/results.json的57项作为历史研究保留。PDF两页PNG已由总控目视：内容完整、表格与正文无裁切；Poppler提示缺Symbol/ArialUnicode display font，但本fixture只用Helvetica，实际两页渲染未见缺字。记录该warning，不扩展为多语言字体通过。实际XLSX由原始ZIP/XML创建以保留特殊单元格案例，由openpyxl独立读取几个结构事实；没有打开Office应用。source tokens是fixture占位D6词法，不是生产host签发证据。

validator仅实现本fixture所需子集，尤其不实现全套IR closed decoder、格式coverage、CSV全部词法、BCP47、D2/D4/Result9、authorization或OS隔离。这些计数不是产品scenario通过数。S01–S34全部有裁决，但只有表中明确写有限验证的片段实际执行；其余须后续实施门。

## r02 必须联合核对的反例与证据边界

模板：重复source拒绝；self/cross-template唯一fresh subject；Resource owner严格重写；parent import与简单collection create逐字段合法；同输出但不同template/recipe revision使旧准备失效；完整membership/top-take与lost receipt原请求恢复。PreparedActionBinding/2四处镜像/消费和历史/1恢复逐项检查。

隐藏内容：同range的exclude/include、隐藏中间行、隐藏列绑定不移位、被隐藏header拒绝、隐藏sheet、空axis、merge相交、manifest/groups重算，全部按typed字段和显式mapping，不能用message或choice猜。

导出与模板：NodeRef/复杂object/quantity/scalar none/graph保留原D7型；RenderSnapshot拒绝未投影复杂值；auth generation reset；普通data token只在同SET item scope，0/1/N和跨SET都不隐式first。

总控补充：IR与预期D2 AST一致、D4真实calendar_date/zoned_instant、固定style资产版本、d9rg1与原D3授权/stale顺序。新增有限模型证据单独记录；原57项只是原有研究子集，任何新用例不能被计作实际D3/D6产品、OS隔离、Office互通或完整格式conformance通过。

## r03 导出组合与单位检查

逐生产者/消费者构造完整ExportInputCatalog、ExportProjection、ExportLossReport、确认和staged bundle；覆盖Optional<Text>→CSV、Document/Field/Annotation省略、Office缺值、重复同值行、错误pin/列/来源域、权限换代、确认后bytes/报告/模板变化、空结果/graph/scalar原V。图片覆盖密度有/无/非法/冲突、方向交换、exact有理换算、半偶量化、显式尺寸有无及物理事实与布局policy分离。有限模型源码/实际输出独立留存；真实decoder、当前权限、Office renderer与OS发布仍是未完成实施门。

新增validate_r03_export.py实际为82/82，独立保留validation-r03/export-results.json、export-witnesses.json和consumer-matrix.json。实际最小DOCX只验证ZIP/XML中的位置，未运行Office；两份PNG验证尺寸/密度元数据，JPEG仅为单位算术模型。native-table与bibliography案例只验证来源位置/块索引，不冒称完整D2 table parser或citation renderer。与原63/63、59/59分别报告，不合并为产品scenario通过数。

## r04 消费适用条件与完整有限链

正文/书目由ExportContentSelection/1明确选择，不能由Document pin存在或body slot缺失推导。纯Query、窄Field/集合、native_table不添加正文读取或虚构省略；请求正文却无权则失败。body、bibliography、样式回退、已知placement省略和报告位置采用同一选择条件。新验证必须从实际fixture和明确选择生成全部必需报告及准确data/report/manifest bytes，检验确认后不变及三个出口的当前资格；不可只分别证明几个地址存在。图片模型先校验绝对/相对声明一致，再进入有/无显式尺寸分支。

r03的82个断言和历史源码继续原样保留，但它们不验证完整Office消费义务、真实值派生或完整初始Plan与报告bytes的一致性。r04的新有限组合另行执行和记账；仍不声称完整D2/Office decoder、citation renderer、真实授权/CAS、OS隔离或publication故障门通过。

当前r04有限证据：validation-r04/export-results.json为90/90，validation-r04-consumers/results.json为130/130，后者包含12个连贯场景和独立结构反例；均与63/63、59/59及历史82/82分别记账。实际最小DOCX只验证协议样本的ZIP/XML和输出字节，未完成Office互通；非空书目/style只作结构模型。
