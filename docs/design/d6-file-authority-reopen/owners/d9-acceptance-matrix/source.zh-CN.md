---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：a26b84bd-10b4-448c-9037-d5cb0bd65a4b。

候选状态：D9 文件权威 PL-IR-01 修复消费者；未独立接受、未激活、未实施。本矩阵保留固定 S 的证据边界，并已消费稳定生产地址/current Observation 作者修复。PL-IR-01 各行只是设计义务，尚未作为产品测试执行；本 exact 修复仍待独立复核，不自行关闭 P1。其它 D3/D6/D7 整合门保持独立。

# D9 接受矩阵与场景裁决

固定 S 的范围与证据说明：D9-01至05均在本包裁决，没有用未来D10替代转换事务或模板语义。架构门禁尚待实际独立终审。本矩阵的“有限验证”指validate_d9_r02.py、validate_r02_contracts.py及validation-r02对应结果；“规范检查”不是产品测试。

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

## 固定 S 的历史证据归属

当前validation-r02/results.json为63/63，contract-results.json为59/59；两组是不同有界子集，不合并声称产品场景通过。旧validation/results.json的57项作为历史研究保留。PDF两页PNG已由总控目视：内容完整、表格与正文无裁切；Poppler提示缺Symbol/ArialUnicode display font，但本fixture只用Helvetica，实际两页渲染未见缺字。记录该warning，不扩展为多语言字体通过。实际XLSX由原始ZIP/XML创建以保留特殊单元格案例，由openpyxl独立读取几个结构事实；没有打开Office应用。source tokens是fixture占位D6词法，不是生产host签发证据。

validator仅实现本fixture所需子集，尤其不实现全套IR closed decoder、格式coverage、CSV全部词法、BCP47、D2/D4/Result9、authorization或OS隔离。这些计数不是产品scenario通过数。S01–S34全部有裁决，但只有表中明确写有限验证的片段实际执行；其余须后续实施门。

## r02 必须联合核对的反例与证据边界

模板：重复source拒绝；self/cross-template唯一fresh subject；Resource owner严格重写；parent import与简单collection create逐字段合法；同输出但不同template/recipe revision使旧准备失效；完整membership/top-take与lost receipt原请求恢复。固定 S 的 PreparedActionBinding/2 四处镜像/消费与真实历史 /1 恢复证据逐项保留；当前 /3 则以 D7 唯一 owner 和本候选消费关系重新检查，不能把旧镜像当当前结构。

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

## 当前协调的追加场景（设计 oracle，未执行）

原 S01–S34 的输入、裁决和有限证据完整保留；其中旧 wire/绑定和原有限验证均按历史版本解释，当前新执行同时适用以下条件。这些场景不是本轮产品测试 PASS。

| ID | 输入与反例 | 必需结果与边界 | 证据状态 |
|---|---|---|---|
| FA01 | 同 Node 在生产域 A 与 B 的 revision 都为 1，导入/模板 token 指 A 的观察 | 完整生产 SourceVersion 与当前 Observation 逐项匹配；不把 B:1 当 A:1，不由 old Counter 读 latest 补签 | 设计 oracle，未执行 |
| FA02 | 纯同步保留 A 的生产版本，接收域 B 当前观察有效 | 正向路径：认证 A 的 canonical RevisionTokenBinding/2 + original sealed-outbox 关联，证明 B 当前 Observation.sourceVersion 与 A 完整版本逐字相等，再验证原 Locator coordinate/profile 与最终 same-cut 门；payload/token 不改，旧 selector/PAB/Draft/ActionEvidence 不复活。真实 production ABA、binding/outbox 缺失、rematerialization 不可证明或 external 仅 bytes 相同仍 stale/unavailable | 设计 oracle，未执行 |
| FA03 | Recipe/2 固定源改版，但旧 bytes 仍 pin 或生成结果相同 | 旧构造 dependency_conflict，不跟随 latest；保留原配方可读及明确新来源/新配方路径 | 设计 oracle，未执行 |
| FA04 | Recipe/1 只剩裸 Counter，用户明确升级 | 显示真实原版本可证性与用户新选择；不猜生产域；保留全部 slots，原 create_resource 完整确认另存，不覆盖原配方 | 设计 oracle，未执行 |
| FA05 | 模板仅枚举 omittedAnnotations，未授权其 body | 先完整 state/annotation 目录资格及正负证明，仅保存版本地址；不读 body，不把隐藏项当零或默认全量省略 | 设计 oracle，未执行 |
| FA06 | 同 Workspace/OperationId 的两域请求、Job 第六批 unknown、设备换代 | key 含完整 CommitDomain；原请求恢复精确前缀，不跨域认领、重做或清空预算 | 设计 oracle，未执行 |
| FA07 | wire12 D3 prepared batch 被要求 D6 planToken，或把 preview header 当完整后像 | 只接受实际原生形状和完整 /3、manifest/pages/bytes producer；拒绝缺失完整证据，不开第二提交 | 设计 oracle，未执行 |
| FA08 | externalSequence=5，模板/Field inner revision 要求 5 | 不能当 managed revision；需要时先显式合法 admission，新版 H 按真正生产域分配，无自动源写入 | 设计 oracle，未执行 |
| FA09 | Job 已 saved/planned，而当前 D7 producer 未接受、preview TTL 过期或源 r6 已改变 | 原记录授权/custody/连续性先行，saved 原 bytes/planned 原计划恢复；新 profile 门只作用 unseen，不取消历史责任 | 设计 oracle，未执行 |
| FA10 | 无关完整 sealed Frontier 扩展；或 source/route/Query 负范围实际变化 | 前者仅按真实 scope_dependencies 证明保持；后者旧准备失败/reset，绝不重签旧 Observation 或拼权限 epochs | 设计 oracle，未执行 |
| FA11 | output pin 丢失后重渲染同摘要；rename unknown 后源/权限/route 改变 | 不重建原输出事实或换名发布；只按原 publication intent/占有/连续性恢复，不能证明则 unknown | 设计 oracle，未执行 |
| FA12 | 转换 proposal 自称 human，或 strict 能力失败后选 observed_only | 导入、构造、Query 导出、Resource 创建均拒绝弱化；D8 明确接收 generated proposal 并保留 dirty Draft 冲突 | 设计 oracle，未执行 |
| FA13 | table_document 输入 header=true，但目标生成展示 header option | 输入 header 与目标普通首行 cell 区分；任何 D2 table header/attribute option 拒绝，无新增语法 | 设计 oracle，未执行 |
| FA14 | 已发布外部 bundle 被当作 CP3/Resource author receipt；或 P 丢失后凭文件匹配恢复成功 | 两类结果保持各自原 authority；原作者单 P seal/charge 不重建，未知责任保留 | 设计 oracle，未执行 |

PL-IR-01 现已有作者修复候选；它保持为**针对本新 SHA 的独立复核门**，不再是算法未定义。必须把 D6 PL01–PL18 与 D9 的 body_text/region 专项行对照实际 D3/D6/D7/D9 正文复核。D3/D6 IR-06/07 与 D7 当前完整 consumer/producer 仍是独立整合门；任何设计 oracle 或文档检查都不能自行关闭 P1 或原 I01–I12。
