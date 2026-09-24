---
_weftext:
  id: "c99e6e2b-dc0c-4a3b-ab35-75a85d12ef99"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。

# D6 Storage Transactions Permissions and Sync

状态：设计候选，revision05-observation-bootstrap；尚未独立终审，不是已实现合同。

## 1. 选择与边界

选择一个 Core、一个工作区提交权威和一个事务性存储根。D2 exact Document source、Resource bytes、Annotation完整值是唯一作者数据；D3身份/生命周期/位置与OperationId、D4 Registry演进、D6权限/来源绑定是有明确用途的控制事实。字段、Facet分类、inverse、Query、图与渲染均从源和控制cut派生。没有Record域、持久Field occurrence identity或视图membership存储。

D6定义存储/事务/权限/完整读取与交付、恢复、离线和同步的共同机制。D7定义Query/View/Action表达式与运行算法；D8定义交互；D9定义格式与映射；D10定义认证适配、贡献真实性和外部执行。不将这些未来实现当本次完成，也不把本阶段机制留给下游临时补齐。

物理基线选择**事务存储中的唯一完整源字节**：一个受管事务存储域内一份SQLite authority store，按完整WorkspaceRef隔离多个Workspace namespace及bootstrap控制记录。Document保存完整UTF-8 byte sequence，Resource保存完整bytes，Annotation保存冻结的closed值。不是每Field一行的第二份作者事实，也不把解析后的JSON重编码当Document。单Workspace portable bundle是明确的一致snapshot输出，不等于一个正在写入的物理数据库。独立可浏览的 `.adoc` 编辑checkout是绑定既有实体的D6 source-edit proposal呈现，须由Core重新验证/提交；普通导出后再作ordinary import则由D3分配fresh身份。D2 §3不冻结 `X/X.adoc` 物理布局，parent/ordinal明确属于Node control。当前原型文件布局不是新架构的兼容约束；此选择必须由独立评审核对可移植性和用户可用性。

可移植bundle包含完整逻辑源、受管控制事实及恢复必要账本；它不等于复制一个仍运行的数据库文件。公开读取/导出工具必须无需特定UI即可完整取出原始源与owner/结构信息。SQLite物理页、rowid、payload storage key、路径均不是D3 identity。原始文件的便利性损失明确存在，不能称与普通文件夹完全等价。

### 完整替代

| 方案 | 原子性/读取/外部编辑 | 裁决 |
|---|---|---|
| 受管普通文件树 + undo/redo日志 + 全局恢复屏障 | 能保持人直接编辑文件；多对象改写需阻断全部Core读取直至恢复，外部读者仍可能看到中间文件，外部写者与日志恢复存在竞态；跨平台目录替换/持久化前提多 | 可行的另一个产品取舍，但本代不选择作为默认author backend；不能用若干rename假装通用事务 |
| 不可变源文件 + 原子manifest根 | 可保留独立源文件与一致cut；需要证明manifest和所有外部payload耐久顺序、GC/故障原子性，引用完整性和control ledger仍需事务 | 有力替代；本代统一进一个存储提交域，避免额外跨介质commit协议。未来blob backend必须保持本合同，不是自动获准 |
| 单一事务存储保存exact源 + 单写Core | 原子发布源/控制/receipt，MVCC稳定读取，大Resource可分块；代价是文件夹外部编辑改为checkout协议，数据库损坏需要完整备份 | 选择；数据可读性由完整源导出保证，不能声称数据库永不损坏 |
| Field/关系数据库和文档双写 | 模型易查询，但卸载、recovery、partial权限和源码同步需第二语义权威 | 拒绝 |
| 多主CRDT/LWW复制整个工作区 | 文本合并不能单独解决owner、purge、关系基数、Registry、绑定和权限 | 拒绝作为本代author commit；以后协作只提交Core验证后的源计划 |

## 2. 存储布局与唯一性

事务存储域根由Core显式open选择，内部marker声明 `weftext-storage/1`。权威文件为 `authority.sqlite3`；活动WAL/SHM属于该存储的一部分，不是可单独同步、删除或备份的cache。临时/锁文件只由backend管理，不能通过缺文件自动重建WorkspaceId。物理存储域不是Workspace identity，也不授予跨Workspace访问权；所有query、ledger与control索引以完整所属namespace隔离。逻辑表族如下，名称为内部存储角色，不扩张D3 wire：

| 表族 | 持久含义 | 不得存储 |
|---|---|---|
| workspace/authority/custody | WorkspaceRef、root、AuthorityInstanceId、generation、fence、D3 custody与连续性 | display/path充当identity |
| entities/placement/lifecycle | 完整typed key、owner、live/trashed/tombstone、连续ordinal及Trash placement | D4 membership、字段值或推断title |
| payload-version/chunks | Document完整原始bytes、Resource bytes、Annotation完整原值；current指针与source版本 | 同时可编辑的decoded cache |
| operation/allocation/burn/proposal | D3原状态机、canonical请求/计划/cut、saved bytes、reservation及family | UI重试次数替代幂等键 |
| registry/control-policy | 认证后完整Registry快照与历史、issuer/Workspace权限policy、固定bootstrap profile、Calendar配置和period scope绑定 | portable Entry内凭据或cursor |
| foreign-binding/import-job | SourceBinding、精确ForeignIdentityKey、OriginBinding历史、远端版本、本地映射base、批次决议/输入绑定 | 按title/row text猜身份 |
| dependency-revisions/outbox | 事务序号、各变更范围版本、派生失效事件与交付撤销代 | 第二份关系作者事实 |

所有完整Ref复合键使用D3闭合decoder和canonical equality；禁止按leaf UUID join。每个live/trashed Node恰一current Document，owner-local Resource/Annotation键与owner相同；live结构无环无孤儿且sibling ordinal完整。tombstone仅保留D3最小identity/kind；burn是不可再分配控制历史，不伪装tombstone。完整原始payload可在外部recent history/backup中保留，不能从tombstone读取title/path或旧ACL。

大payload以固定上限chunks存储，但只有一个完整payload descriptor的有序chunk集合是源；单chunk不是Resource/Document。staging为内部、不可枚举内容入口，绑定creator/principal、input和期限；stageInput是Core内部受限接口；外部上传/续传运输由D9/D10另行冻结，不作为当前D6已提供的opaque upload handle接口，任何后续上传标识不得读取他人staging或复用Resource ByteHandle。staging不分配NodeRef，不进入工作区查询；内容身份只在D3允许的planning boundary reserve。最终事务把已经耐久、完整、核验过的staged payload指针发布为current，不复制整份Resource进入内存。未完成upload不能提交；未引用staging可清理，但planned、snapshot或backup pin持有的chunks不能清理。

所有author pointer、控制事实、必须同步维护的版本/失效标记、operation decision与receipt存于**同一数据库事务**。不得依赖多个ATTACH数据库的WAL获得跨库原子性。索引/缩略图放设备派生存储，可随时重建；recent history与backup各有独立目录和权限，不是索引，不能位于作者存储域内部或经junction/symlink回入。受管存储域外普通文件不是自动新Node。

### 2.1 Bootstrap与portable边界

bootstrap issuer authority、allocation families、所有successor/predecessor的target ledger namespaces、custody与burn记录实际共置于该issuer所属存储域；此issuer可以签发多family并激活多个Workspace。fork的source可以在另一个域，输入是源authority授权并pin的有限immutable cut；fork不修改source，其新target和issuer仍共置。不是把既有source Workspace的数据库误当成target commit参与者。

issue在一个事务内建立family/proposal、reserved target IDs与issuer-custodied空ledger namespace；replacement在一个事务内burn predecessor、retire其custody并建立successor。正式recorded rejection/planning分别把family winner与target ledger及reservation/burn同事务写入；activation在一个事务内发布全部源、确定的初始policy/target principal映射/Registry/Calendar配置与scope绑定、target authority、committed receipt和custody移交。首次issuer信任起点、当前allocate_workspace准入及family固定bootstrap profile完整由控制接口§14定义；身份认证不自动授予这些能力。这里移交是同一物理域内的逻辑authority/custodian控制变更，不是先移动数据库再补写。claimed后无replacement。不同Workspace可独立复用OperationId UUID，物理共库不产生全局OperationId registry。

portable snapshot按一个已授权Workspace导出其完整作者源与**可重放控制闭包**：该Workspace的identity/lifecycle、ledger/完整saved bytes、allocation/burn、custody/continuity、相关bootstrap family及其predecessor历史、binding、policy和planned pins。闭包包括相关family控制证据，但不得混入同域其他Workspace的payload、ledger或密钥。若issuer/authority的部署方式不能提供独立可验证的该闭包，必须报该种可接续导出不可用，仍可提供普通源artifact；不宣称一个payload ZIP能够continue。

导出artifact默认只读，没有写custody。导入后如仅用于查看/ordinary import，保持D3既有身份规则。**首版continue支持范围限定为同一连续物理存储域内的逻辑authority接续或已证明连续的同一backend接管**；其custody/ledger/authority激活仍在同一数据库事务。复制snapshot到另一物理存储域的保留identity接续本版未交付，能力探测按D1声明not_in_release，提供只读artifact或显式D3 fork；不以“未来迁移日志”声称已有跨域原子handoff。已支持的同域请求如暂时不能证明continuity则进入D3原identity_authority_unavailable，不能把未交付能力和临时资格失败混同。D3本身允许在其全部证明成立后作跨域continue的语义不被修改，未来交付该能力须另行设计验证完整handoff机制。完整原始源的可移植性保留，跨设备保留identity迁移的当前限制明确交独立评审评估；不宣称与普通可搬动文件夹具有相同体验。普通跨Workspace copy+Trash仍为两个明确提交。

## 3. 后端资格、栅栏与物理前提

本地与本代Server使用同host的可靠本地文件系统及同一Core存储实现。SQLite WAL要求同host，不选NFS/SMB/File Provider上的活动WAL；Mobile先复制/建立应用管理的合格本地backend再使用同一Core。系统文档提供者只作为显式输入/输出；它不满足backend契约时能力unavailable，不降低事务语义。本代Server可有多个前端，但同一authority store只有一个实际提交持有者，不提供多主数据库集群。

取得写资格的顺序为：解析物理store身份（防路径别名）、持有跨进程OS exclusive ownership lock、完成打开数据库所必需的WAL物理恢复、核对当前authority/custody与账本连续性，然后在存储事务中提升单调fence generation并登记唯一recovery holder。在该fence下核对应用journal的决议状态与pins，恢复屏障通过后开放合格普通及管理操作。屏障要求控制状态可证明且payload pins完整，不要求所有planned都变terminal；因暂时撤权、额度或依赖证明不足而暂停的个别plan保持原状态，不能阻断为其恢复权限/资源的管理操作或无冲突的其他Workspace。每个planning/commit/abort/ACL mutation事务在持有数据库write lock时比较该fence与当前holder；旧进程即使恢复、lease时钟过期或持有旧连接也不能提交。OS lock只作活性/占用，持久fence才参与每次decision。generation上限拒绝，不wrap或重置。

Server failover只在证明旧writer已fenced且完整store/ledger连续后接管；丢失机器而仅有旧备份不能取得同一Workspace继续写资格。D3 continue/fork保持原矩阵。简单复制bundle到另一设备只读/reconciliation，不能根据新路径自动签发AuthorityInstanceId。

外部直接篡改/覆盖authority store、WAL或marker不是受支持的编辑协议。检测到文件身份替换、外部变化或完整性异常即废弃旧资格，停止新读取交付及写入，保存证据进入协调。该保证从实际发现起生效，不声称监视器能在任意恶意OS写入发生前发现它；普通用户内容外部编辑使用§10 checkout，因此不与active store竞写。威胁模型覆盖不可信客户端/worker/provider和并发Core，不能保证被本机管理员任意篡改的存储仍正确；此时只允许受权repair。

产品backend验收必须验证所用SQLite/VFS、持久化barrier、OS lock、文件系统、设备与故障模型；`synchronous=FULL`及实际成功设置是本地耐久基线，不能靠默认值。library承诺不是本项目掉电/损坏/实机验收。WAL checkpoint、磁盘满和长snapshot的资源开销必须有预算；不能以性能理由改为ack后可丢的耐久级别。

## 4. snapshot、revision与可信依赖

每次Core语义读取在一个不可变MVCC cut上进行；cut保存Workspace/authority generation、commit sequence、当前权限generation、RegistryBinding、完整source/control引用及必要contribution版本。一个cut的payload pointers与control必须来自同一read transaction，不分别“读最新”。pin保证读取期间payload不被GC；大结果pin按期限/磁盘预算限制，过期只使handle失效，不丢作者数据。

`sourceRevision`逐字使用D4要求的non-Boolean D3Integer；同owner的实际source改变一次commit恰加一，raw no-op不增，MAX时拒绝。Document token采用 `d6d:<storeIncarnationUuid>:<sourceRevision十进制>`，Resource token为 `d6r:...`，Annotation为 `d6a:...`；token的比较仅exact equality，类别/owner仍由其envelope绑定，不能单token resolve。incarnation是Core控制事实，恢复连续store保留；fork按D3得到新identity与新incarnation。source A→B→A的revision不同；不能只用内容摘要防ABA。lifecycle/placement各有独立版本，Trash/restore无source改变时不强增Document revision。Annotation body/target/reply任何committed变化均用新token。

面向principal的操作先授权，再由受信Core在最小适用范围内部读取完整源并strict UTF-8 decode。Core内部解析权限不授予principal/worker完整source读取权；还必须先按控制接口§13证明操作结果的潜在观察资格，不能只凭Field write在隐藏约束命中后选择成功/拒绝；内部原文、隐藏解析信息和D4 context不得直接交付，输出仍经§5字段与错误遮蔽。解码失败保留byte envelope，不造D2payload/diagnostic；解码后D2完整profile valid才可产生classification/attestation，保留其冻结五字段形状。D6额外依赖放外层cut，不修改D2 wire。title/coreKind/declared/effective从该完整source及同Registry解析；不从请求、header cache或UI状态补齐。

可信dependency的封闭语义种类：`source`、`lifecycle`、`placement-range`、`ref-inbound`、`relation-incidence`、`calendar-scope`、`registry`、`temporal-rules`、`authorization`、`foreign-binding`、`query-scan`。它们由Core读取追踪器产生，客户端不可提交“完整”证书。空集合同样绑定完整范围版本；range key由真实schema/ref/selector定义，不由label/path猜测。

基线完整性算法：从同cut中的完整entity inventory读取所有适用源/控制，通过D2/D4解析得到完整相关range，记录全局 `contentSequence` 与相关control generations作为保守负依赖。任何相关作者提交至少改变contentSequence；在commit重新比较，故可在无索引时完整扫描证明空scope。性能可用经证实涵盖此cut的派生索引替代扫描：它必须有完整构建inventory、parser/Registry版本、消费到该cut的连续失效序列和无缺口证明，范围facts逐项绑定真实source版本。只有部分/旧索引时继续完整扫描或明确不可用；不能把缺索引当空。全局sequence是安全的粗粒度退路，不把它包装成低冲突/高性能保证。

D4 RelationReadContext/2、RelationReadBinding/2由真实source/lifecycle/负范围读取投影；source-bearing和sourceless/受限状态分域，严格保持新版exact shape。RecurrenceReadContext/1保持原shape。请求expected bindings必须逐项等于实际完整binding，失败不返回成功readSet。membership变更读取全部相关incidence scopes；relation retarget读取old/new owner与endpoints；未改写只读owner不增source版本。trusted temporal rule set需D10认证来源和固定version，coverage不足不是gap。CalendarScope键的series和periodKey逐字由actual Entry还原，scope由同cut受管CalendarPeriodScopeBinding提供并绑定D4 policy；保存范围revision并在同事务CAS，同key不同Node的unique冲突、many接受与retry同键规则不变。具体series/scope的multiplicity从受管SeriesScopeConfiguration读取，包含完整typed series/scope、unique或many与configurationRevision；普通create/update不能通过自报many覆盖既有unique。普通已激活Workspace缺配置不猜默认值，须按控制接口§8由当前policy_admin且完整观察者建立；首份create/fork初始化使用§14的固定bootstrap计划，同一activation发布，不能先激活再补配置。period scope选择、显式移动、空配置删除及控制inbound清理由§8.1闭合；many→unique先证明全范围无重复，配置和范围版本同事务CAS。policy/configuration变更使依赖旧配置的新提交失效。

## 5. 权限模型与遮蔽

本地/Server共同使用认证的 `PrincipalContext`：authority内principal、session、delegation chain和当前policy generation。认证来源由host/D10提供，普通JSON无权自报认证。完全离线本地使用当前OS用户被授予的local principal；Server客户端必须经Server，断网草稿不获得本地托管解释权。凭据保存在设备/Server secret store，工作区仅保存引用和权限policy，不输出到Document或评审/错误文本。

policy为explicit allow grants与explicit deny的集合，deny优先，默认拒绝。role仅展开为相同grants，不是另一套判断器。grants有Workspace作用域、可选完整NodeRef subtree、可选完整FieldId集合和封闭能力：workspace/entity/locator state disclosure、document source read、field read/write、body write、Node control write/create、Resource read/write、Annotation read/write、lifecycle、Registry administration、foreign-binding administration、export、repair、audit。无通配用户可写代码；每次授权按当前parent/control计算，move可能改变继承权限，preview/commit必须同时验证old/new域并原子改变auth generation。

Field权限不得退化为namespace权限；phone write不授name write，inverse写必须授权actual canonical owner；新D3 fresh compound在采样前还保守要求全部声明的potential existing owner写权（D3 §4.1.3a），故未被选中的潜在端缺权也会拒绝；此明确能力成本不套用既有D4纯编辑，也不为未修改owner增版。partial Field读取不授完整raw Document：raw source read/export需完整作者内容授权，含隐藏Field则拒绝整份raw源，不输出所谓“exact但已删字段”的源。Field API仅交付授权字段，并明确它不是全源；禁止用隐藏字段作filter/count/order/graph的侧信道。跨Field/unique/incoming等潜在约束必须先按控制接口§13证明结果可观察；未获权的两种隐藏状态均在数据求值前返回同一not_visible，不能一边成功一边拒绝来暴露隐藏事实。Core内部完整读取与对外结果观察各自需要证明；不得返回隐藏D4 context。首版D3全模式采用保守Workspace全域观察（bootstrap target按原阶段两段绑定），因此局部写权可能不足以创建或修改identity；D6可证明独立的局部Field路径保留。纯policy/执行资源管理的control_only路径不依赖作者读取，保留当前受权管理恢复。

entity-state、locator-state、content-read不是彼此蕴含的能力。D3 resolver在任何existence/coordinate/index查找之前分别执行其disclosure谓词；未授权时四生命周期态及never-known同not_visible。D3 operation依原§13 stage3遮蔽后续，saved receipt也先检查当前授权。字段/源读取先授权，再availability与源解析；详细repair/audit另行授权，不能通过普通错误附带内部cause。

state-disclosure grant支持workspace域，以及用于entity/locator的有限完整Ref集合；只按请求完整Ref作先验相等匹配，不允许subtree。这样同Workspace可用exact-ref deny遮蔽parent、保留spouse可见，且相同Ref在live/Trash/tombstoned/never-known之间不改变授权规则。最小tombstone不因此增存parent或旧ACL。subtree仅用于通过适用disclosure闸门后的内容/Field等权限，不反过来决定对象是否存在。具体decoder和能力蕴含/deny矩阵见控制接口§4。

所有权限变化与auth generation增加在同一authority事务。每次plan、commit、page、download、export publication、subscription事件交付都重新检查当前principal/delegation和generation。撤权事务与交付的线性化点串行：若撤权先赢且当前所需授权不再成立则该新交付拒绝；已在线性化并交付给用户的字节不能追回，不宣称远程擦除或阻止其保存。长流每个chunk重新过交付闸门；开始下载时的一次授权不覆盖未来全部chunks。未交付授权缓存在generation变化后失效，不能继续使用旧授权结论。Query/plan/effects等句柄仍按各自依赖失效规则处理；D2 Resource ByteHandle的immutable source binding采用控制接口§11的专门规则：auth generation变化触发当前完整resource_read重验，仍获权且原pin/TTL/lifecycle/continuity有效时可继续读取同一旧版本；不得将无关权限变更等同lifecycle/continuity不可逆reset。

## 6. 事务、D3账本与源patch

D3 identity operation采用同包wire11正式修订request、fingerprint、A1–A11、stage1–15、P1/P2/TL1/TL2及错误wire。stage6仍空；D6不在其中塞私有验证或把所有失败改为同一种transaction error。特别是ordinary旧generation仅可在连续性证明后重放已存same-fingerprint决议，unseen在stage5拒绝。D3 preflight rejection不写ledger；stage7–15拒绝经其decision CAS；planned以后不再重新跑D3内容门禁并改判成recorded rejection。

规划前的preview是非提交结果，不能消费content reservation或修改binding。实际提交请求通过D3阶段和适用D2/D4/D7检查后，planning decision在一个database write transaction内CAS：ledger unseen、fence、权限、same current cut的全部read dependencies，以及proposal family/custody/continue candidate（适用时）。随后同时保存canonical plan/原cut、reservations和planned，并pin完整before/after payload。竞争失败遵循D3唯一restart点（create/fork从P2，ordinary从TL1）。不能另发未落盘的私有胜负结果。

非identity源编辑也使用同一Workspace-local OperationId ledger控制命名空间，ledger key恰为WorkspaceId+OperationId，记录protocolOwner=D3或D6。新D3 owner决议遵循同包wire11 canonical request、fingerprint、阶段、receipt/errors；已保存v9/v10决议仅按其原decoder/fingerprint/bytes重放或恢复，不产生新旧版decision；D6 owner使用独立闭合request，不向D3 mode闭集加值。同key不同protocolOwner在本协议已授权的ledger gate返回operation_id_conflict，不泄露另一请求。D6只对自己的envelope定义decode→current authorization→authority/custody→saved decision→source/dependencies→semantic validation→planning的顺序；不能用于包装D3请求以绕过其更精确顺序。纯D6编辑只产生D6 receipt。D3 receipt采用同包正式修订wireVersion11；需要额外D6效果证据时保存独立companion，来源只能是D3请求/plan已经绑定的完整pre/post源和可信cut，和原receipt同decision原子保存，不能借companion加入未绑定的额外作者变更。

持久planned可能与其他计划并存，但提交时必须在数据库write transaction重新验证原cut的全部业务source/control/negative dependencies、当前授权和当前物理fence。**物理fence、D3逻辑authority generation、业务依赖版本三个域独立**：新holder必须使用当前fence；原request的expectedAuthority和原plan/cut保持byte-equal，合法continue/failover的custody迁移通过完整continuity relation满足，不因旧fence/authority token与当前不等就判业务冲突。权限generation改变先重新授权，不把一次撤权当成永久冲突；未授权依D3 stage3遮蔽，保留planned，恢复授权后仍可恢复原计划。

planned以后不能重新解读新源替换原计划。仅当当前授权成功、authority/cut连续可证明，而原source/结构/语义依赖已经发生不能满足原计划的确定冲突时，Core可作出该计划永不提交的单一authoritative abort，写terminal_failed并burn原reservations；D3对外仍是identity_commit_aborted。保守全局范围版本改变只触发对原绑定范围的再证明，不在不相关写入后自动burn；再证明只能证明原plan仍适用，不生成新plan或重新跑D3 stages。无法完成该证明保持planned与authority_unavailable，不因超时或读权限暂失猜abort。新意图/重规划必须新的OperationId；其他计划可继续，除非authority整体被隔离。

唯一**author commit point**是最终database事务耐久提交：current payload pointers、Node/lifecycle/placement/control变更、SourceBinding/OriginBinding/cursor（适用）、D4 effects、source/range/auth generations、invalidation outbox、committed ledger与canonical receipt bytes同时发布。失败之前旧root完整，失败之后新root完整。不可在同一OperationId内出现“source写完receipt随后补写”。receipt传输失败不改变commit；重试仅在当前授权与连续性通过后返回保存bytes。

源patch由完整old source与current revision构造，采用严格不重叠UTF-8范围或明确的D4选择器变换；先核对expected raw span，再构造完整proposed source并执行D2/D4 gates。字节拼接保持未触及的顺序、comments、EOL与unknown namespace。只改note/Entry不重编码整个namespace；同Field相同value不同key不合并。relation same-owner原位替换，canonical迁移移除old并追加new，两个owner同事务且各增一次；D4 copy effect独立验证actual源与D3 cut，不从effect自身反推证明。

Core从完整before/proposed source计算实际MutationFootprint，逐项覆盖改变的Field/occurrence、body、title/coreKind/declared Facets、Resource/Annotation值和适用control。授权按真实footprint逐项判定，并验证它属于所声明意图允许的效果；phone意图夹带name或wf-facets修改，即使语法与schema都合法也拒绝。无法可靠分类的源差异要求完整source写权限，仍必须通过适用typed语义门禁；不能用raw/body patch逃逸relation、schema或Calendar约束。footprint、可信依赖、完整proposed bytes与plan一同保存，最终commit只发布这份已绑定源，不重新从客户端提取patch。footprint对外仍受披露限制。

旧revision的non-overlapping proposal可以产生新preview：保留base/current/proposed、真实源差异及所有权限/语义依赖，新请求显式绑定新revision。不能以key字节相同证明跨revision连续，也不能自动执行旧意图。外部删除再加同key总是冲突；无法证明连续变化时保留draft。整表/整源替换需相应完整写权限，不能绕过partial Field限制。

## 7. journal与恢复

应用journal就是持久operation/proposal/allocation/control记录及pin的canonical cut/plan；SQLite WAL提供单个存储事务的物理原子基础，两者不能混称。恢复进程必须先取得backend lock/fence并证明custody/ledger连续，才开放普通操作。

| 故障点/状态 | 恢复与可观察结论 |
|---|---|
| staging未完整，尚无planned | author root不变；可受权续传或清理orphan staging，不能报告Node创建 |
| planning事务未提交 | unseen或原saved decision；没有一半reservation；重新按原preflight处理 |
| planned已耐久，author commit尚未开始 | 复用原plan/cut/reservations；依赖相等可恢复，不重新采样ID |
| 最终事务写入中崩溃 | store恢复后只能判为旧planned或完整committed，凭实际ledger判定；模糊I/O结果不由客户端推断 |
| committed但响应丢失 | 当前授权后重放原receipt；不创建第二Node、不更新第二次cursor |
| authority/cut/账本不连续或损坏 | quarantine；保留planned/原bytes，authority unavailable或受权repair；不能从当前源猜历史决议 |
| authoritative abort事务中故障 | 原planned或完整terminal_failed+全部burn，不能部分burn |
| commit后派生index更新失败 | author commit保持成功；outbox未消费，相关结果unavailable/rebuild，不能邀请重试已完成写入 |

每个saved rejection/receipt/terminal error immutable。恢复可重复运行，不改变已完成decision；恢复中的第二writer仍受fence。检测到author数据库内部不一致时不能“修复索引后继续”掩盖identity/ordinal/binding碰撞。普通repair计划也需完整preview；不得使普通trash/restore绕过D3 stage4 integrity。

## 8. 索引、Query完整交付与reset

正确性必需的范围版本及失效事件是control，不是可丢的index。派生index有building/ready/unavailable，ready绑定完整cut、parser/profile/Registry与消费序列；更新可异步，但旧ready不得标为current。需要最新结果时等待catch-up、完整扫描或明确unavailable。source commit原子使受影响index generation失效/记录outbox；rebuild在旁路生成、完整验证后以指针切换发布，不阻塞已有合法snapshot读取。重建失败不回写作者源。

D7提供有限、确定且完整的Query evaluation plan；D6保存 `ResultHandle` 控制记录：principal/audience、Workspace/cut、auth generation、Query/参数/definition source绑定、Registry/规则、结果schema与完整性、有效期、资源预算。handle不等于内容身份或写授权。执行状态为producing→complete或failed/cancelled；仅complete可发布可分页消费的结果。有限输入snapshot不等于已证明全Query不会在末尾失败。完整语义求值、全部错误与输出schema/顺序验证完成前，只能提供进度，不交付可用于chart/export/requireMembership的rows。预算/取消/源不可用不能把部分rows发布成完整结果。运算可spool至受保护磁盘，不要求全部内存；complete后分页是已验证结果的运输，页读取失败不改写为成功EOF。

cursor绑定同handle和确切位置，不能只传row ordinal；每页验证audience、当前授权、handle未过期且snapshot仍pin。page可空但nonterminal；EOF必须来自terminal标记，不来自 `rows.length==0`。D5 grid page最多200，preview展示页200不表示只确认200effects；bulk plan固定全部target及revision，不能commit时重新Query扩大集合。

create-through-collection的requireMembership和“移出集合”共用完整proposed post-query证明：前者确认目标属于完整语义结果，后者确认选定目标不属于完整语义结果；Query定义、参数、schema/Registry、授权及正/负范围依赖与该证明一起进入planning/commit验证。语义top/limit属于结果，运输页不属于membership定义。仅当前page找不到目标不能证明移出；不能证明时只能按另一个明确意图报告普通事实编辑，不能把本次集合操作降格后仍报成功。

语义状态cache保存可重建的已解析typed状态及正/负依赖；snapshot结果cache保存某完整Query/cut的结果。两者均不能持有独立作者事实或授权。cache key同时含实际semantic依赖、principal policy视图与generation；缺Field/schema不是空值，缓存absence也要可证明范围版本。增量/批处理必须与同cut cold完整计算类型、bag/cardinality、顺序、错误、evidence一致；未证明支持的算子reset后重算，不能近似。

订阅stream有epoch、连续sequence与source-cut binding。撤权、Registry/规则变化、缺失事件、无法证明增量等价时发non-disclosing `reset_required` 并使旧epoch无效；客户端必须清除未交付旧rows/ActionEvidence并重新请求，不对旧结果局部拼接成新完整结果。无法交付reset时下一调用也拒绝旧epoch。D7冻结具体delta wire，D6冻结上述交付规则。view/compiler/renderer版本变化独立失效导出cache。

导出任务绑定完整源cut、模板原始revision、Query结果与规则、loss选择、renderer及权限；worker无可写workspace挂载，只得到已授权snapshot输入。输出先staged，发布/下载仍当前授权；撤权后可销毁未发布staging，不得为“任务已经开始”绕过。Query结果空仍须有明确schema，模板歧义按当前原文重新编译，不能旧位置/列名cache授权。

## 9. 导入、同步与控制耦合

SourceBinding定义比较域，OriginBinding只连接精确ForeignIdentityKey到NodeRef；UID、path、title、etag不等于Node identity。D3 initial_import/adopt/upsert/retired/non-live完整矩阵原样执行。一个key的active唯一性、retire历史、source版本与mapped作者变更在同一事务验证/提交；retire先胜后新explicit adopt可fresh，旧adopt观察active的尝试不能自动改判fresh。

每个connector定义版本比较器和明确mapping ownership，D9/D10提供真实source proof；D6只接受固定输入artifact/版本与授权。内容merge使用上一次已接受的mapping base、当前本地source和新的remote输入，按完整Field/当前selector执行三方比较。未映射Field、local notes与历史保持；同一映射事实双方都改则conflict，远端版本/时钟较大不自动胜。binding记录映射来源对应关系是控制证据，不能作为跨revision永久Entry身份；映射目标重新按当前源证明，ABA或不唯一拒绝。无明确版本排序的provider使用exact-version equality与冲突，不能比较etag字典序。

外部cancel/delete、binding detached/retired、Node lifecycle与authority/conflict为独立状态轴。remote delete不自动purge/trash本地Node；retire不删Node；adopt新Node由Weftext管理但保留Provenance。双向sync只在显式authority/merge决议后支持：外发写有同job idempotency与provider条件更新前提，远端调用不能与本地数据库组成原子事务。外发结果不确定时保持outbox pending/unknown，先查询provider结果，不能伪造exactly-once或盲重发不可幂等操作。未能证明provider幂等/条件写能力则双向写不可用，单向或显式人工冲突仍可用。

长import先创建有限job计划：完整immutable输入、mapping版本、有限atomic coupling groups、按D5限额显式batches及稳定batch OperationIds。先按完整poststate必须成立的约束建依赖图：已先行提交的target允许后续source引用并绑定其当前revision，不能仅因输入文件中target排后面就强行把整条无环引用链合并。不可拆的source↔binding、Node+Annotation创建意图、promotion源替换、完整template实例等仍在同group；真正相互需要fresh目标的强连通分量也必须合并，不能跨batch等待补边。跨group按拓扑顺序执行；合并后超过1000新Nodes或任何资源预算则在执行前拒绝/重新设计显式映射，不先半导入。全job可部分完成，不承诺一次全局事务。

每batch事务同时提交源、fresh identity/OriginBinding、mapping结果、receipt、job batch状态和可推进的sync watermark。watermark只有其覆盖的所有输入均已在该事务或已有可验证committed前缀内处理后才前进；gap/待冲突项阻止前进，不能因后面batch成功越过。job不按相同row text去重重试，重取已commit batch receipt后继续未提交部分。完整输入超过working memory时计划/映射/receipts可磁盘化，仍有总input、磁盘、时间和dependency预算。

10000 Nodes明确10批时，第6批commit前失败保留1–5，commit后receipt丢失保持1–6；返回真实partial状态及已知receipts，不能报告0或10000。模板每行生2 Nodes时1000输入行是2000Nodes，必须事先显式重划。ICS master与override若原子耦合且超过限额，拒绝该group或保留opaque输入，不截断无限recurrence。VFREEBUSY/VTIMEZONE不生成Node。

## 10. 离线、外部checkout和副本协调

本地Workspace离线使用本地权威，外部能力不可用不影响已证明的本地source操作。Server Workspace断网仅存草稿，草稿标明base cut/intent/source proposal；本地Core不解释托管Query或提交托管状态。重连须由Server重新授权/规划，旧row与receipt不能绕过。

外部编辑checkout含完整可读 `.adoc`/Resource文件及独立manifest绑定export cut、owner与原revision；manifest只是本次D6 source-edit proposal输入证据，不是活动Workspace第二权威，也不是写授权。导回只允许对显式绑定的既有live实体重新授权并提出新源版本，不能因同名、路径或文件内UUID选择目标；不完整manifest/缺文件不得解释为删除。显式删除意图使用D3 lifecycle请求，修改title/path不改NodeRef或parent。离开该显式编辑proposal作用域的普通artifact重新导入仍严格遵循D3 import_new fresh identity，不能凭manifest变continue/restore。用户可修改文本而不懂内部数据库，但提交仍是Core的preview/commit。原文件和冲突base/current/proposed均保留至用户裁决或明确draft清理。共享文件夹仅运输这种有限artifact或完整停机/backup snapshot，不同步运行中的SQLite/WAL。

两副本都发生离线author提交且无法证明唯一authority连续时，禁止静默continue、按mtime/LWW合并或丢弃一方burn/receipt。进入只读reconciliation：可将一方保留为完整artifact，再按D3显式fork获得新Workspace；想保留原Workspace继续，必须取得完整共同账本/排除双writer的权威证明。此限制明确承认本代不是离线多主协作系统，不把普通云盘当Server。未来实时共同编辑必须以Server cut+Core语义接受形成规范checkpoint，短暂操作序列/CRDT不成为作者权威。

## 11. 历史、备份、清理与修复

最近历史保存exact before/after source和操作来源，按显式保留策略管理；history key与NodeRef分域，Undo是对当前cut的新计划，不是回滚全库或复用旧receipt。不同字段和后来提交必须保留，发生权限/依赖变化则冲突。历史内容读取也检查当前授权；已purge对象不能通过普通resolver或receipt接口返回旧payload。

完整backup通过受pin的数据库一致snapshot/官方backup机制或已验证停机checkpoint导出，含current源、live/Trash/tombstone、operation/allocation/burn/custody、Registry与binding控制。不能仅复制 `.sqlite3` 丢WAL，也不能漏掉staged planned依赖。backup外部加密/目标认证由部署契约确定；恢复到新位置默认只读，D3 continue所需cut之后无遗漏提交和旧authority停用证明不可用时只能fork/repair。

最小tombstone、burn历史、用于重放的saved decision和retired binding的防复活信息，在相应Workspace authority存续期间不做普通cache GC。完整receipt若含payload/敏感数据，保存于受限控制存储；公开重放仍按当前授权。容量问题必须显示管理限制，不能静默删除幂等历史后把重试当新请求。snapshot/result/draft可以按已声明TTL和预算清理，失效返回明确结果而非重新绑定最新cut。

authority store损坏时保留原bundle及日志；仅在独立repair staging重建、核验完整身份与账本后显式切换，无法恢复control连续性不激活为同authority。直接从全源重建只可重建派生index，不能恢复不存在的burn、receipt或权限历史。整个Workspace销毁是独立管理操作，需实际授权；结束authority后D3 refs为workspace_unavailable，不把全部对象伪造为tombstoned。本架构工作不执行任何销毁。

## 12. 资源预算与下游接口

统一BudgetBinding按控制接口§6声明最大input/source/write/decoded bytes、work units、dependency entries、new entities、临时磁盘、elapsed及result/output限额；数值使用non-Boolean D3Integer并明确单位，未知字段拒绝。取消是Core受管Boolean状态，不伪装D3Integer；deadline、clock epoch、已耗counter与恢复规则见接口§6。可信backend按表面可用容量提供较窄预算，不能由请求增大。每次完整plan与其所有D4成员证明共享同一消耗账户，不能每个子函数重置预算。资源拒绝不放宽D2/D4精度，也不把合法source变语法invalid。

D5 hard limits保留：mutation1000显式Node targets、native edit1000row targets、import1000新Nodes含descendants；grid200行，preview200展示行与完整可取effects。一般D7 Query和Workspace总量不受这几个数字限制。超限不自动截断/拆批；新增显式batch计划才可继续。checkpoint/GC/long snapshot有独立可观测预算，UI线程不得承担全扫描/数据库维护；具体延迟目标由产品host验收定义。

对D7必须提供 `openAuthorizedCut`、`readSource`、`readCompleteScope`、`validateDependencies`、`publishResult`、`readResultPage`、`subscribeCut`、`commitBoundPlan`；对D9/D10提供 `stageInput`、`planAtomicGroups`、`commitImportBatch`、`replayOperation`、`compareForeignVersion`、`retireBinding`；这些是Core内部语义接口名称，不能用开放任意JSON callback暴露。[D6控制接口候选](../d6-control-interfaces/source.md)补充本阶段closed commit/receipt/error、policy、result handle分页及BudgetBinding，并明确PreparedIntent与D3请求的边界。未冻结的D7/D9/D10语义输入不因此获得通用写入口。

## 13. 验证范围与接受边界

本版是revision05-observation-bootstrap架构候选，不是已激活决议。其完整组成是本主稿、控制接口、术语增补、实施影响与场景处置；共同定义合同，不用其中一份摘要替代另一份。接受还需独立语义评审和控制落盘，局部文件检查或有界SQLite模型都不能代替。未实施的Core、全量D3/D4 conformance、客户端、性能及host验收依实施影响保留为后续实际证据义务。

物理参考：SQLite [WAL](https://www.sqlite.org/wal.html)说明同host限制、单writer/MVCC、FULL同步及ATTACH非跨库原子；[Atomic Commit](https://www.sqlite.org/atomiccommit.html)说明日志与底层存储假设。这里从这些机制推导候选实施路径，不把library文档当Weftext验证。

## 14. D3/D4正式组合修订的适用边界

同包D3 v11与D4符号源桥是本候选成立的必要组成；旧v9未因此获得新语义。D2 checklist promotion与D5 table-row promotion由一个D3创建decision同时绑定fresh Node完整源和既有Document的revision/preimage/完整result；成功后原row/checkbox变为普通Node Link。纯row copy仍可只创建fresh Node。Resource创建加既有owner source occurrence、创建Annotation加显式既有源修改、含fresh目标的完整Template/SCC group及mixed import使用同一闭合组合，不分拆提交。

pure existing upsert归D6；任何含fresh identity的atomic group归D3且全部existing edits必须进入同包wire11规范输入。D6 companion只增加从该同一已绑定plan派生的控制证据。D4 typed leaf与对称关系source assembly必须在planning reservation以前物化并验证；D4 copy已有endpoint迁移拒绝边界继续有效。原始D3 v9 conformance证据保留为历史基线，新符号桥的本地检查只证明其具名范围，不冒充全产品conformance或host验收。

D2 Resource snapshot的d6_byte_handle由控制接口§11的ByteHandle及closed byte-read协议完整承接：固定已提交ResourceRef/版本/cut/descriptor/audience/pin，不随source更新重绑latest；逐chunk当前授权、明确范围/EOF、有限共享预算及失效优先级。§12列D1–D5继承token/context映射及尚未开放的上传、repair和effects运输边界。


## 15. D4 关系读取中的无源状态依赖

D4新版实体状态证明和作者源证明必须来自同一cut。Core在独立lifecycle控制记录中维护每个已知完整Ref的单调版本，分配及每次live/Trash/tombstoned变化与原author decision同事务保存，checked-add/MAX拒绝；这不是D3 tombstone的作者字段。stateToken通过受保护依赖记录绑定完整Ref、该状态版本、实际状态、store incarnation及可证authority连续性。token不可由客户端、旧history/classification或只相同状态文字合成。source bytes ABA依sourceRevision，lifecycle ABA独立依此状态版本，两个维度不可互代。not_found使用完整inventory负范围证明及版本，后来分配或连续性不可证明时旧证明失效；保守全局sequence允许额外冲突，不允许遗漏分配。当前授权在读前及提交/交付重新验证；masked/unprovable不得变成tombstoned/not_found。

purge仍按D3只保留最小tombstone，不复制源版本、Facet、parent或旧ACL来满足D4。原source不再存在时没有SourceVersion记录可交付，D6只在独立读依赖保存状态证明。restore A并显式删/改A→已purge B的directed旧关系时，A源/状态、B状态、完整incidence及新target C的真实源/状态分别CAS；同一事务只对实际变更的A源增版，保存原D3 receipt和D4完整效果/失效。失败及并发失配无作者写入。该路径不放宽新目标live/domain或symmetric purge清理，不新增公开任意状态token查询API。

D3 fresh候选Node的D4 source/state binding由同一受保护prepared-origin依赖承接，绑定已验证mode、candidate map、完整暂存payload/拟议revision/lifecycle及allocation-history/authority版本。Core在planning/commit验证这些真实准备事实与原reservation，不伪造已存在的Document/state记录；既有Ref仍用正常source/lifecycle CAS。客户端不能自选origin，not_found/tombstoned旧Ref不能借此复活。

## 16. 约束结果观察与初始化消费合同

控制接口§13、§14及§8.1是本主稿的一部分：静态潜在范围在数据/ledger前授权，包括空范围；scope、真实依赖、当前policy/Registry与固定plan共同CAS。准备、业务拒绝、重放、恢复与交付同样受约束。issuer policy不蕴含任何既有Workspace内容权限；首次target policy/Registry/Calendar控制与原D3全部源只在同一author commit生效。普通管理、continue、failover及旧receipt绝不重新赋予creator权限。

managed copy内部node scope若映射为本次fresh Node，其所需配置按D6接口§8.1从真实source配置确定派生、保留multiplicity并同copy提交；只建立该fresh scope配置，不改既有配置，不需先激活scope Node或第二次管理提交。完整prepared范围、target Registry和负inventory仍全部验证。

## 17. D7联合replacement的明确增量

当前消费D3 wire11/Result9，新增显式准备绑定和保存定义Q转移，保留原ledger/stages/identity。D6 Control Interfaces §4、§13–16给出Policy/2、source_envelope_state、commit_sequence_state、profile/2 bootstrap、新窄Field正向证明、共享commitSequence算法、完整D7准备/效果同decision持久性。metadata与capacity/CAS的现行资格由这些具体条款定义，不能把“已有phone权限”解释为自动具有资格。D6 R5-P2-A仅作历史问题追溯标签，其旧裁决不作为当前接受证据。旧policy/profile/decision保留原版本，不回放升级。D7专项合同是当前共同规范输入；本次仅架构候选，没有产品API已经实现的声明。


D7联合修订消费D3§7.3.1的独立Trash恢复membership：owner边与恢复membership分别持久保存并进入同一状态/范围依赖及CAS；不能按当前owner join自动合并此前独立Trash项。owner restore、显式owner-local restore和owner purge各按原D3唯一闭包推导实际状态/receipt/effects，exact fork保留其规范连接。此处不新增作者源或第二生命周期权威。
