---
_weftext:
  id: "ebd24e10-6020-41e0-8a08-f494e46c21ec"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。


# D4 Implementation Impact and Test Outline



日期：2026-09-01。

## Revision37 preservation and source-copy comparison

- Relation-update and recurrence-edit outcomes retain detached pre-state without encoding or JSON round-trips. Ordered relation replacement uses the same lossless copying discipline. Raw strings in unrelated unavailable namespaces remain opaque; no whole-state or operation byte cap is added.
- Copy-effect verification first establishes the exact ordered row inventory and identities, then matches raw before/after sources against independently admitted expected sources, then compares the remaining typed JSON structure. Boolean/integer distinctions and array order remain exact; dictionary order is required for state preservation but not semantic object comparison.
- The shared typed comparator also checks decoded Entry cache copies in direct/shared relation validation and copy construction without encoding unverified cached values. Existing authority masking, complete post-state checks, diagnostics, valid precision and Registry generation33 remain unchanged.
- Consumer regressions instrument actual source-bearing encoders and state round-trip decoders, in addition to Entry admission/parsing. Small valid controls, early failures, opaque-source retention, claim mismatch, exact raw/member order and result detachment receive scoped evidence. These checks do not constitute independent semantic acceptance or downstream host proof.

## revision36 admission obligations

- All ordinary/composed raw Entry decoding uses shared pre-parse admission after existing Workspace/Registry/namespace/schema masking. Include Facet pre-state and initial Entries, including sources later removed by Cleanup, and diagnostic source-position consumers.
- Schema sizing is a memoized, saturating pre-materialization pass over actual canonical JSON byte costs. Bound each alias expanded root and each complete expanded Field/Facet independently. Preserve the existing depth/cycle/placement checks and valid numeric/temporal precision; integer counting is not a replacement semantic validator.
- Local source-copy matching compares admitted raw strings and exact JSON structure without first serializing an unchecked read copy. Relation-copy planning holds existing read references opaque until the common gate accepts; only then deep-copy the returned candidate.
- validate_d4_generation36_admission.py observes parser/span/materializer timing, byte boundaries, authenticated-context masking, zero-write rollback, independent canonical encoding costs, non-bootstrap complete-definition admission, alias reuse and unchanged placement/depth. Existing source/copy/recurrence suites remain required. Finite local results are not independent Pro acceptance or physical-host validation.

## revision35 integration obligations

- Ordered source transformation precedes common post-state admission. A same-owner/Field relation update replaces the selected occurrence in place. Relocation removes it from the old stream and appends to the destination Field stream (or destination owner stream if the Field is absent). Preserve every unaffected occurrence, its relative order and raw source. Cross-owner inventory interleaving is not a physical Document-position contract; D6 must bind the result to the actual patch/revision. Canonical ordering remains limited to unordered bindings and derived inventories.
- For union_variant_equal, when both operands have occurrences, all occurrences on both sides must use one common variant. Repetition and different occurrence counts remain legal; missing-side requiredness is separate. Equal mixed variant sets still fail. The loader retains many-valued union operands, and every applicable public operation uses the same quantified evaluator.
- A shared constructor-fault location rule gives object faults their /kind pointer: missing kind uses the containing-object span, while existing wrong/unknown kind uses its token. Non-object input points to itself. Keep constructor opacity and independent sibling/source-order diagnostics.
- Required supplementary evidence: validate_d4_generation35_relation_order.py (all24 many-valued relation Fields and76 order/orientation/relocation scenarios with refusal controls), validate_d4_generation35_constraints.py (196 public Create/Assign combinations in a valid non-bootstrap successor schema), and validate_d4_generation35_constructor_locations.py (21 public exact-location and composed rejection checks). These finite checks do not establish arbitrary schema completeness, physical permissions, transactions or host implementation.

## 1. implementation effect graph

```text
D2 exact Document source
  -> D2 header Facet memberships + raw carrier entries
  -> outer namespace owner + RegistryBinding/schema availability proof
  -> authenticated closed/evolution-validated registry + bound catalog context
  -> D4 strict Entry/1 parser
      -> FieldId expansion + OccurrenceKey uniqueness
      -> recursive typed value decode against closed ValueTypeSpec/1
      -> exact QualifierSet + FieldDefinition + FacetSchema validation
      -> per-namespace availability
      -> relation fact + inverse/graph projections
  -> proposed source under its actual owner + common complete post-state validation
  -> source-derived projections and exact revision/read/write coverage
  -> operation-applicable D4 semantic gate
  -> future D6 authz/CAS/transaction/receipt gate

unknown/unavailable provider
  -> raw source preserved
  -> typed capability unavailable, never empty
  -> only byte-equal nonintersecting edits

Facet change
  -> pure create/assign/remove/cleanup preview
  -> entry-level write set
  -> future D6 commit wrapper
```

## 2. component impacts

| area | frozen D4 effect | forbidden substitute authority |
| --- | --- | --- |
| Core D4 parser | strict closed JSON Entry/1 overD2 rawEntrySource; duplicate keys/unknown members/null/floats reject | provider parser、UI JSON、last-wins |
| semantic registry | one immutable exact `RegistryBinding/1=(expected generation,expected snapshot digest)` with closed nested contribution objects, alias/Facet/Field digest states, authenticated predecessor, cumulative immutable Field+Facet tombstone/migration ledgers and monotonic contribution identities；`user` owner locked to current Workspace | self-consistent same-generation substitution、current-map self-comparison、three-generation resurrection、same-ID tzdb/unit mutation、silent deletion、install order、display label、runtime consumer as owner、syntax-only namespaced value acceptance |
| Field model | full FieldId + exact semanticMajor、closed recursive ValueTypeSpec/QualifierSet/cardinality/constraints、measurement code↔unit dimension coupling、independent occurrences、raw Entry maximum 65528 bytes | whole-namespace blob、header auto-claim、height+kg/weight+cm、same FieldId semantic drift、array-of-all-facts、D2 physical-line overflow |
| occurrence mutation | owner+FieldId+OccurrenceKey+expected revision selector；唯一性exact为同owner+同Field；不同Field或跨owner可复用key bytes | `(key,value)` guess、namespace-wide bare-key uniqueness、FieldRef/RecordRef |
| notes/provenance | optional inline note; closed D3-compatibleprovenance atoms | Annotation target withoutreopen、credentials/token inauthor source |
| Facet composition | declared D2 set, derivedrequires closure, symmetric conflicts, effective required-field post-state invariants, no override | hidden effective membership writeback、global Field minimum requiredness、provider priority |
| availability | explicit per-namespace available/retained_unavailable/invalid/not_present | unknown-as-empty、stale typed cache、silent cleanup |
| relations | fixed-direction schema-declared Relation Field、explicit subject/target Facet predicates、one fact、derived inverse、canonical symmetric owner、endpoint-total symmetric cardinality、atomic cleanup-before-purge、literal text arm has zero target mechanics | arbitrary same-Workspace Node target、mixed-direction Field、orientation-dependent limits、global openRelationType、two-end writes、path/title matching、text-to-Node coercion |
| Task | built-in `tasks/task` fields; explicit status while Facet remains effective | TaskRef、Task entity/mirror、read-time default write |
| People | independent name/contact/account/event/observation/engagement/relation fields；non-empty name/contact text；field-specific contribution sets keep account service, usage and contact label domains separate | Person kind/privateDB/names-to-alias copy、DOI-as-account scheme、birth/child-as-usage-or-label、service-in-label |
| Organizations | semantic parent/identifiers/classifications/relations; member inverse fromPeopleengagement | structural parent authority、globalcountry enum、two-sided membership |
| Calendar | exact calendar/rule/tzdb-bound period identity；closed RegistryBinding-backed `CalendarSeriesScopePolicy/1` typed key with `unique|many` result；period/range/event typed separation；duration is exact derived output only；recurrence requires complete date `(calendarId,version,precision)` or instant `(zone,tzdb)` basis equality and matching range branch | path/index uniqueness、authored duration、IntervalNode、same-kind mixed calendar/version/precision/zone recurrence、cross-day autoEvent、derived occurrence Node |
| Library | Work fields/relations/identifiers; Citation remainsD2 occurrence | Reference entity、DOI asNodeRef、PDF path identity |

## 3. future implementation slices

1. strict parser/decoder + diagnostic rank table；
2. type constructors、recursive ValueTypeSpec/alias expansion、canonical numbers、arbitrary-precision exact instants、versioned calendar comparator、exact D3 ref/locator decoder；
3. immutable registry snapshot, generation binding, pre-parse owner proof and contribution lookup interface；
4. exact QualifierSet/Field/Facet/catalog validator、CJ/3 digest与composition planner；
5. source-preserving entry patch/fresh-key copy rewrite；
6. availability and byte-preserving nonintersecting edit gate；
7. relation/inverse/symmetric owner/lifecycle projection；
8. RegistryBinding-backed 61-Field/7-Facet reference catalog definitions and fixtures；
9. D6 wrapper、D7/D8/D9/D10 adapters only aftertheir own frozen stages；
10. public specs only withmatching implementation+tests+acceptance in later authorized work。

## 4. test outline

### 4.1 source and grammar

- LF/CRLF/CR raw entries, JSON duplicate keys, trailing tokens, unknown envelope member, illegal null, float/NaN/Infinity/-Infinity at every nesting depth, version 0/2；
- dotted D2 namespace + localField expansion；namespace 63/64 bytes、FacetId 127/128 bytes、empty dot segment与leading/trailing/consecutive hyphen；wrong namespace、multiple blocks same namespace、32/8192/64KiB D2 boundary preservation；
- identical values with distinct keys accepted；same key in different Fields accepted；duplicate key in the same expanded Field across any same-namespace blocks rejected；
- header attribute that resemblesField remainsordinary header; explicitMap producesone carrier target and loss report；
- unknown provider raw exact round-trip, external formatting damage, noold projection fallback。

### 4.2 types

- integer/decimal canonical boundaries beforehost conversion；`-0.5|0.5|-1|1.25`接受，decimal zero唯一`0`，`-0|0.0|-0.0|1.20|exponent`拒绝；Unicode exact/NFC comparison withoutsource rewrite；
- calendar_date ISO/non-ISO provider unavailable, year/month/day precision；ASCII digits only、三个precision都拒绝year `0000`；用lexeme顺序故意不同于chronology的registered comparator证明不按byte order；
- zoned instant严格RFC3339 ASCII digits/真实日期/clock/offset，Arabic-Indic digits、`+25:00|-00:00|:60`拒绝，sub-microsecond order不丢精度，IANA zone必须属于declared tzdbVersion，无device-default guess；
- date/instant ranges bounded/start-open/end-open, both-open reject, equal/reverse cross-offset reject, decoded exact-instant order、end-exclusive display；
- quantity units/precision/conversion provenance；
- NodeRef/ResourceRef/AnnotationRef exact frozen wire、wrong kind/extra/missing/non-v4/bare UUID/path/title/URL；DocumentElementLocator/DocumentRangeLocator/ResourceRegionLocator closed variants；direct value与provenance cross-owner ResourceRef reject，member source order不影响D3-CJ/3 owner equality；
- ObjectMemberSpec/UnionVariantSpec exact member sets、sorted uniqueness、alias cycle/missing/depth 8、schema/entry byte limits；
- catalog/Field/Facet wireVersion、Field/Facet semanticMajor、globalLimits、collection/cardinality/provenance index全部先验证non-Boolean D3Integer；directed target minimum固定0；symmetric relation禁止source/target cardinality members；MAX+1与Boolean拒绝；
- bounded_set/sequence only closed object internal members；top-level Field collection reject；item types recursively checked；set CJ/3 order/uniqueness、minimum/maximum、nested overflow逐项拒绝；generic array/map/any/opaque reject；
- qualifiers exact QualifierSet：fact/event_assertion/observation/relation的required/optional members、date/instant range、eventTime、observedAt、confidence、selection、status逐项wrong/missing/unknown/null拒绝；validity只在qualifier，role/position/rank/department只在value，双写和`qualifiers.role`拒绝。
- provenance transform必须引用同array更早atom，inputIndex为zero-based D3Integer且operationId为canonical lowercase UUID；self/forward/MAX+1/任意字符串逐项拒绝。

### 4.3 occurrence and note

- two identicalphones differentnotes/provenance; patch/reorder/delete exact key；
- expected revision missing/stale, ABA delete/re-add, duplicate key in same Field after external copy；same key/different Field acceptance；
- identity-preserving Node move retainsbytes；cross-owner managed copy/import/template may preservekey bytes becauseowner scope changes；复制到两个target后相同key互不影响且D3 resolver均拒绝；same-owner duplicate freshenskey；
- key sent toEntityRef/Locator/AnnotationTarget/RecordRef decoders rejected；
- absent note omitted; emptyplaceholder rejected; note neverparsed forbehavior。

### 4.4 Facet and availability

- 0..N declared set, source reorder semantic equality, requires closure/cycle/missing/conflict；
- same Field compatible shared definition accepted；完整snapshot chain同时逐项比较Field与Facet ledger，tombstone/migration历史累计immutable、retired ID跨三代不可复活，contribution identity monotonic；same-ID type/cardinality/constraint/relation/Facet closure/major/tzdb/unit drift拒绝；deletion必须matching tombstone，replacement必须fresh ID与exact old→new migration，wrong predecessor/orphan/missing/mismatch拒绝，standalone fresh ID不伪造migration；catalog aliases无环、alias digest membership且61 Field/7 Facet references complete；
- unknown/uninstalled/incompatible schema acrossread/body edit/typed edit/query/export/reinstall；UI/runtime disable withportable schema retained mustnotchange typed meaning；owner-unprovable/conflict/generation-changed必须遮蔽malformed inner JSON且parser call count为零；unverified relation status/calendar/unit/scheme按同一rank-10 proof fail closed；
- exact generation+digest pair、same-generation substituted snapshot、`user` wrong-Workspace owner、complete-empty definitions→unknown Field与unavailable definitions→provider unavailable逐项验证；20个directed inverse codes（完整Field→inverse对应见4.5清单）逐一命中semantic contribution table；
- contribution preflight先于wrapper structural/type validation，覆盖union extra member、missing calendar comparator、qualifier observedAt+bad confidence及external provenance observedAt+extra member；availability code不得被归一；
- strict parser在合法whitespace下保留所有nested object key/value、array item、scalar的half-open UTF-8 byte spans；真实raw+catalog validator聚合multibyte/escaped/whitespace/同名nested key和独立nested faults，不消费fixture `faults[]`；lone surrogate只报invalid JSON；local span携带D2 revision token投影absolute source range；只改raw或只改expected sequence/span必须失败；三模式byte-identical；
- Field minimum全部为0，maximum跨blocks；Assign/Create/continuing effective state/删除最后required Field/Remove保留Field逐格验证；Remove keepsfields；cleanup never deletesField still usedbyanotherFacet；
- requires两节点/长cycle、self/missing dependency、asymmetric conflict与closure conflict用catalog mutation fixtures在normal/`-O`/`-OO`均拒绝；
- Task+Project+Calendar; Template+nonTask Facet accepted, Template+`tasks/task` remainsD2reject。

### 4.5 relations

- recurrence series edit通过公共纯planner验证before raw source、实际owner与revision、D6 authorization/read绑定、稳定Field/OccurrenceKey及完整显式rebase；单次编辑不能夹带rule/range或其他例外修改。unknown非依赖namespace逐字保留，unknown effective Facet仍阻断；一份budget覆盖全部成员证明和post-state复核；
- series实际改写只增加owner revision一次；raw no-op零写、不增版本；range改变同时使range和recurrence投影失效。权限/CAS/原始字节失配、孤立after selector、缺rebase、revision overflow和提交前注入失败均完整保留旧source/revision/cache，零writes/invalidations；D6实际事务/receipt和D7宿主重建仍须后续验收；

- Facet和relation公共操作必须调用同一proposed-state gate；可信输入包含raw owner事实、revision-bound有效域/lifecycle、完整Field/endpoint incident scopes。expectedRelationReadBinding及成功readSet同时绑定Node sourceRevision与incidence revisionToken；缺失/重复/stale全部零写，目标sourceRevision未变也不能绕过入边集合并发检查；
- Remove Facet检查其他owner的incoming关系域，Cleanup显式删除进入完整后状态；文字目标无目标依赖，未改写的只读owner不增版本；不相关suspended事实保留。实际D6认证、完整读取和物理CAS是后续宿主义务，本模型不把客户端自报完整当作证明；
- `D4 Operation Cases v1.source.json`保存显式操作世界和独立期望；builder不调用接受函数生成expected，也不自动填补缺失依赖。隔离重建必须包含`d4_relation_state.py`及回归所需的`d4_relation_test_support.py`，缺生成输出时重建结果与当前文件逐字一致；

- directed `people/parent`/inverse child one fact；symmetric spouse/sibling/family-related分Field；structural move no change；
- symmetric NodeRef arm spouse/friend writtenonlycanonical owner fromeither UI side；text arm留在containing owner且零endpoint mechanics；same-Workspace D3-CJ/3 nodeId canonical UTF-8 unsigned byte comparison、self-edge policy、trash/restore suspension、inverse/graph rebuild与not-visible preflight；
- stateful store保存raw Entry source bytes及其一次解析值/qualifiers/note/provenance/key；A→B/B→A same-key same-payload一份事实，same-key different-payload collision，distinct keys按schema保留；text↔NodeRef owner move绑定before/after revisions、canonical-owner-changing时双owner write set、stable-owner时单owner write set、key preservation；任一source revision delta必须同时属于`authorizedOwners`与computed `writeSetOwners`，stable-owner `text→NodeRef`保持未写owner revision不变；独立state machine先构造完整拟议post-state，从FieldDefinition读取maximum，按稳定事实identity去重并分别计算两个semantic endpoints，不用storage owner代替endpoint；覆盖auth/CAS/collision/local+incoming endpoint-total cardinality、orientation reversals、text→NodeRef、NodeRef→text、NodeRef→NodeRef、`many` unbounded、stale projection/allocation替换、lifecycle/injected projection failure与raw-state byte rollback，collision保持`{a:7,b:11}`、一份source fact、零projection、空write set；
- target live→trashed→restore→tombstoned/not_visible/unprovable; factretain/no leakage/noauto delete；
- mixed-direction FieldDefinition拒绝；directed sourceCardinality与Field cardinality逐字一致；symmetric Field maximum对incoming/outgoing canonical NodeRef facts与local literal facts做endpoint-total，storage orientation不影响结果；任一endpoint purge前显式cleanup且两步原子；同一`people/engagement` NodeRef arm产生resolution/inverse/graph，text arm产生零resolution call/零inverse/零graph，same-title/provider不coerce；custom neutral relation no family rank；
- 27个relation Fields（20 directed、7 symmetric，完整集合见下表）逐一对allowed subject/target Facet cross-product接受，对arbitrary same-Workspace错误subject或target拒绝；所有authorization/CAS/collision/cardinality/lifecycle/domain/projection failure在构造post-state前或失败回滚后逐字保持raw Entry source bytes、解析值、qualifiers、note、provenance、source revisions、projection rows、allocations，write set为空；
- closed create/assign/remove/cleanup Facet request/outcome及显式relation read context由同代源语料中的stateful cases执行：Create/Assign保持NodeRef并CAS revision，Remove保留raw entries，Cleanup只删显式unused keys，unknown Facet/CAS失败byte-exact零写；125条scenario row、302个完整命题逐行绑定disposition、closure、明确testScope与实际case IDs；模型执行、结构检查、来源完整性与独立契约评审分列，所有命题仍须独立语义Gate；
- Resource cross-owner relation attempt reject; NodeRef-to-owner orcopy-to-fresh-owner only；
- graph/backlink index delete/rebuild produces sameprojection and neverauthoritativefact。

关系验收清单按规范性catalog逐项核对；symmetric项没有inverse code，不以历史计数代替完整集合。

| FieldId | direction | inverseCode |
| --- | --- | --- |
| calendar/participant | directed | calendar/participates-in |
| library/creator | directed | library/created |
| library/venue | directed | library/hosts-work |
| library/version-of | directed | library/has-version |
| organizations/allied-with | symmetric | — |
| organizations/brand-of | directed | organizations/has-brand |
| organizations/business-guided-by | directed | organizations/business-guides |
| organizations/governs | directed | organizations/governed-by |
| organizations/jointly-led-by | directed | organizations/jointly-leads |
| organizations/member-of | directed | organizations/has-member-organization |
| organizations/owns | directed | organizations/owned-by |
| organizations/parent | directed | organizations/child |
| organizations/related | symmetric | — |
| organizations/subsidiary-of | directed | organizations/has-subsidiary |
| organizations/supervised-by | directed | organizations/supervises |
| organizations/territorially-administered-by | directed | organizations/territorially-administers |
| people/engagement | directed | organizations/has-engagement |
| people/family-related | symmetric | — |
| people/guardian | directed | people/ward |
| people/manager | directed | people/direct-report |
| people/mentor | directed | people/mentee |
| people/parent | directed | people/child |
| people/professional-relation | symmetric | — |
| people/sibling | symmetric | — |
| people/social-related | symmetric | — |
| people/spouse | symmetric | — |
| tasks/dependency | directed | tasks/dependent |

### 4.6 domain fixtures

- full People fixtures：names、contacts、canonical-service accounts、birth/death/employment-start/graduation/marriage的preset事件与custom重要日期、冲突断言、measurements、engagements、parent/spouse/sibling/family-related/social-related/professional-relation、provider missing、same-name Nodes；
- social/professional分别验证精确literal来源、空target读取的公共后状态、零inverse/graph和显式text↔NodeRef原子替换；两者均保持symmetric general方向；明确有向的manager与mentor角色按其独立Field合同验证。
- Library venue验证Article Work→Journal Work及显式Organization场所、非法Person target、provider缺失、错误facet和目标删除；venue不复制或猜测publisher/creator，期刊不被强制标记为Organization。
- Organizations：semantic parent vs structural；governs/owns/allied-with/related fixed-direction Fields；identifier history、country pack absent、Person-side engagement inverse；
- Calendar：exact CalendarPeriod tuple/canonical periodKey/rule contribution、closed periodKind↔keyProfile mapping、real Gregorian/ISO-week semantic decoder（含valid/invalid W53与year bounds）、RegistryBinding-backed series+scope `unique|many` key与concurrent collision/accept/path-index rejection、closed/open date/instant duration derivation与authored-duration rejection、period/range/event、AssignEvent without duplicate range、all-day/DST/open、date/instant-discriminated RecurrenceRule with homogeneous until/rDates/exDates and range-branch equality、count-until conflict/set items/horizon/override/cache rebuild、packs multi-source；
- ICS：sameUID differentSourceBinding, series/override zero occurrenceNodes, VTODO→Task preview, VFREEBUSY/VTIMEZONE zeroNode；
- Library：draft→published sameWork, edition freshNode+relation, DOI notidentity, creator/venue/Citation/Resource separated。

### 4.7 terminology and non-regression

- D4 concept IDs/owned names unique anddisjoint fromD3 owned/retired names；
- controlled `Profile|attribute|field|property|fact|metadata` collision matrix；D4 surface裸`occurrence`/`key`只有qualified context可映射，正负例各一；
- relation/link/reference/citation and Source/Provenance/SourceBinding/OriginBinding negative cases；
- normal/optimized (`-O`,`-OO`) validators byte-identical；
- local links、unique `_weftext.id`、完整candidate/package/evidence副本与直接逐字比较；不新增包装或命题哈希；
- protected `repos/weftext` dirty-state content hash andbrand tree exactly unchanged。

## 5. acceptance boundary

D4冻结只证明architecture contract足够稳定。未来implementation必须等待A2与显式授权，并同步Core、all affected surfaces、fixtures、tests、public zh/en docs and `04 Acceptance` evidence。D5–D10不得从本Impact被视为已完成或已授权。

### 配偶历史与通用有限基数验证

- 实际spouse目录接受多条独立历史声明，验证不重叠、重叠、缺省validity、文字/NodeRef混合、显式更新后其他事实raw源码与provenance不变。
- 通用有限基数反例使用独立conformance catalog中的acme.tools/limited-relation及固定Fixture ID，经完整目录/Registry验证后供测试驱动选择；不得把maximum或fixture selector加入产品operation request。五项公共关系零写失败和Facet跨owner入边失败继续保留原诊断。

### People任职目标的D8/D9交接边界

传给D8编辑器与D9导入/connector的既定边界是：`people/engagement`的NodeRef arm允许任意普通Node，不要求Organization Facet；编辑器保留真实引用，不自动Assign、不强制转为literal。Organization专属名册/图按其领域条件增强，通用inverse仍来自Person侧事实。导入仍须核对same-Workspace、source/incidence绑定、生命周期及授权，不能借普通Node条件绕过共享约束。

### 制度限定的作者职级

- engagement.rank是closed TypedObject，required system和level均为nonEmpty exact TypedText；未给rank时不推断。验证不同制度同名级别、多组织/同组织兼任、原文保存、缺成员/错误类型/无关People code及额外推断成员拒绝。
- UI可渐进展开职级制度与级别，但提交必须保留两个明确来源。D9对无法解释的外部rank code必须明确mapping/loss并保留源；D10机器码域/制度preset的未来扩展必须显式演进，不将本对象当作已注册代码或全球等级顺序。

### 有界重复投影与系列编辑衔接

- 公共纯投影读取实际Event membership和两个Entry；先授权，再验证owner/source revision、Registry、Field/key及实际temporal读取绑定。projection identity同时含range和recurrence selector，删除或伪造cache不影响结果，无Node或作者写入。
- date保持模板calendar day数，instant保持确切elapsed秒数；开始采用civil重复而非每天加86400秒。覆盖DST gap/fold、显式late-fold anchor、30位小数和需要输出但越出日期域的失败。
- count/until成员判断和投影共用按确切时间排序的base枚举；有限timezone跳变导致civil遍历顺序与实际时间不同也不能误计数或提前结束。需要的前瞻coverage缺失、工作或output budget不足均返回无rows的失败。
- 普通范围与窗口相交、窗口外前后两个方向改期移入、改期移出、RDATE/EXDATE/取消/覆盖优先级、相同最终范围不同selector以及显式edit/rebase后新版本重投影均有公共纯路径检查。D6读取真实性/物理并发、D7展示时区渲染和真实cache更新、D9 ICS映射仍需对应阶段验收。

### 闭包类型与宿主数值边界

- 对18个schema识别的TypedValue构造器，覆盖代表性合法来源的每层member/container替换为其他JSON类型，不仅变更kind；覆盖四类Provenance的嵌套ref/locator、schema member trees、catalog必需header与全部alias body。
- 公共Facet/关系请求的嵌套类型替换必须明确拒绝并保留完整pre-state；关系expectedSourceRevisions额外验证值相等的浮点数不等同D3Integer。alias全部header先闭包验证，再建立引用表，不泄漏missing-valueType KeyError。
- 在Entry byte cap内，5000位合法fraction的公共Recurrence Entry、实际投影range两个端点及独立exact roundtrip必须保留完整精度；低Decimal context precision不改变结果。不得通过调高全局宿主integer digit limit或缩小作者精度合同绕过问题。


## revision28 People状态与参考算法验收义务

D4使用Person Facet下独立的nationality-state、legal-sex-state、gender-identity-state Fields；每个作者断言独立保留TypedText、validity与provenance，重叠／冲突不覆盖。D7必须按明确FieldId和期间派生视图，未知期间不得假定当前、永久或唯一；preferred选择不能合并法律登记与自我认同。实际目录和完整公共案例是D4输入，不能交给下游临时发明领域定义。

保留有界recurrence参考算法作为语义oracle。D6/D7后续性能验收覆盖早期daily anchor、多例外与窄horizon的长prefix情形；共享枚举、可验证prefix复用和版本绑定索引必须与cold-cache结果一致，不能改变count/until、原始selector、coverage或预算不足失败的含义。本P2不阻断D4，也不构成宿主延迟／吞吐承诺。

## revision29 People事件与重要日期验收义务

D4公共Entry、case及Facet接受路径必须覆盖五项preset code和custom label、required eventTime、date/instant与year/month/day精度、未知或跨域code、空/缺失label、错误branch及额外成员。原先birth object不得被当成新union的合法来源；这只修订未激活候选，不授权既有数据自动转换。合法相同label、相同日期、冲突birth/death与独立preferred断言可并存；所有拒绝保持原始输入和零写失败，缺contribution不得伪装为空值。

D6/D7/D8后续验收必须区分修订选定断言、追加新断言和仅修改投影规则；birth/death分域选择，多preferred不自动胜出。结婚或入职日期只能从显式选择且语义匹配的关系validity.start或独立事件断言产生，不能复制两个事实源或按一般任职/隶属期间猜测事件。D7/D10纪念日投影必须绑定当前完整selector、来源revision、temporal member与规则版本；验证来源变化、权限变化、Calendar UI关闭、实际contribution缺失、闰日、partial日期、非Gregorian规则缺失及不支持的yearless导入，分别保留作者事实和明确不可派生/有损状态。当前conformance不声称提供这套宿主交互或纪念日投影实现。

## revision29 Source Task分类验收义务

Node typed接受使用一套共享检查：ordinary加source-declared exact tasks/task才是Task；合法requires图保留，effective Task但没有显式Task声明的Node在无关系、关系两端、Create/Assign/Remove和Task/Calendar recurrence路径都必须拒绝。验证direct、多层、diamond扩展、合法Task+Project、独立Template Facet、非法Template Task、缺失/错误分类、source revision失配及同effective闭包但declared不同。Remove须先移除dependent再移除Task，当前不支持一次请求同时移除多个Facet；保留Task scalar不等于保留Task身份。

Facet与recurrence source pre-state新增coreKind，可信relation node state新增coreKind及declaredFacetIds，全部复用原有NodeRef/source revision绑定；effective仅从显式声明组合并精确核对。D6不能从old cache、UI状态或effective闭包补造这些字段；Node kind或source声明改变必须失效owner revision，同闭包声明变化也必须复核incidence读取。现有公共结果检查验证有限输入的语义和零写行为，不认证D2解析来源、权限、实际CAS或宿主编辑事务。

## Revision30 People account authoring contract

D4 owns the closed preset/custom AccountIdentifier union inside people/account. The custom arm has independent required nonempty exact serviceKey and identifier text; usage and customLabel remain outer value members and Entry.note remains separate. No global custom-service registry or automatic matching is introduced. D8 presents the branch explicitly, localizes only preset display, and preserves duplicate author occurrences and all exact custom text. D9 explicitly maps authenticated known presets or retains custom service/account text; it must never infer service from display label or note. D10 availability applies to actual Field/alias and preset contributions, not to arbitrary custom author text. Acceptance must cover Entry/case/Facet, source/author order, same-value distinct occurrences, missing/empty/wrong types, installed cross-domain schemes, and unknown provider behavior; no implementation or host acceptance is claimed here.

## Revision30 人物关系合同与验收义务（inactive）

三个一般People关系Field使用分域preset/custom descriptor与symmetric general语义。guardian、manager、mentor使用明确方向的独立Field；guardian不进入Family Tree或代际rank，manager/mentor仅按声明角色投影。D8在同一人物关系编辑区明确区分中性关联与方向角色，custom称谓不触发字段变换；D9预览导入与显式语义变更，不能静默把旧directed professional事实改为symmetric。D10继续按现有Field/contribution可用性证明，不引入全局关系类型注册表。独立复审前须证明各公开操作入口的代码分域、原文保留、对称唯一存储、角色inverse、字面target无图，以及拒绝时的完整零写入。


## Revision32 Workspace、关系复制与诊断验收义务（inactive）

公共D4接受context不可变绑定已认证Workspace；源owner不匹配时，在内部Entry解析、Facet解释与效果规划前返回namespace_owner_unprovable。D6/D10必须提供host Workspace与Registry的同一可信绑定；Registry结构验证不能固定使用conformance Workspace常量。验证普通／自定义user Field与Facet、合法后继Registry、Create/Assign/Remove/Cleanup、关系状态／编辑、Calendar与recurrence入口；负例断言零解析、原状态精确保留、空write set与null成功read set。合法跨Workspace NodeRef值或provenance并不因source owner绑定而改写或禁止。

对称关系的域验证使用actual canonical source owner与authored target。D8的提交方向不是另一个subject authority；合法扩展Schema可有异质subject/target域。检查两端提交、fresh NodeId两种排序、相同规范事实与效果、literal无target lookup、失败回滚，不以限制所有对称Field为相同端点域来规避语义。

D4 Candidate7.5逐项覆盖27个关系Field及同direction扩展Schema。D6复制／fork实现必须消费经D3验证的完整source cut、identityMap、source/result payload与revision、lifecycle和locator重发证明；D4Effect不能自证这些输入。所有被纳入关系source恰映射一次；保持occurrenceKey与未受ref修改影响的Entry字节，内部ref重写、D3允许的external保留、canonical fresh-owner迁移及所有新owner的完整后状态同时验证。需要写入已有endpoint、破坏owner-local provenance、遗漏／合并事实、错误revision、缺少空scope、requiredness/cardinality/domain失配均拒绝整次操作。fork必须保留live/trashed两态；初始化复制结果与普通编辑权限分开。结果原owner作者顺序保留，迁入事实按完整源selector追加。D4RelationCopyEffects/1是独立closed扩展，不向D3 wireVersion9塞入新reference-slot arm；source carrier/range与occurrenceKey不成为D3 typed identity。纯Python关系copy模型只证明所述语义，不认证D3投影、不实现D6物理commit或宿主source patch。

递归provenance诊断覆盖external/node/resource/transform及nested D3 refs/locators。missing成员保留exact pointer并使用真实最近object span；已有scalar/key用最窄UTF8 span，unknown key按RFC6901转义，多个独立fault分别报告且按source位置排序。observedAt/provider不可用不吞掉独立siblings；不得把整atom错误作为多个member错误的替代。测试必须断言完整code/pointer/token/span而非仅非零宽度或prefix。


## Organizations场景18补充验收

以同一机构保留primary affiliation、business guidance、territorial administration与显式joint leadership的并存事实；公共Entry、完整关系后状态及复制结果必须逐项保留FieldId、target、occurrenceKey与限定信息。另比较supervised-by/governs、subsidiary-of/owns、brand-of/identity及member-of/allied-with，禁止按note猜测typed edge kind。D7/D8只消费已明确的关系语义，D9映射不得创造额外D4含义。此处为待实现/验证验收义务，不声称Graph、connector或物理复制已运行。

## D6所需完整版本演进实施义务

当前目标为同包D3 v11/Result9和D4§15源物化桥。上列既有机制与场景继续非回退；其中旧wire/corpus计数仅说明历史覆盖，不能作为当前版本验收结果。实施必须统一十数组per-mode decoder、现有源revision/potentialChanges授权、artifact fresh/update/companion分区、candidate map与planning CAS、C carrier完整源物化、S fresh reply和回执/效果双向验证。原v9只保存决议重放，unseen零写拒绝；新版本不向旧请求补默认数组或悄悄转换已存decision。

逐实际输入验证row/checklist promotion、Resource创建+existing occurrence、existing Annotation reply→fresh同owner、pure existing upsert归D6、mixed D3批次、fresh Task SCC、完整Template、copy内部D4 typed refs、symmetric迁移/空carrier/禁写既有端、provenance/locator所有typed根、权限不足/撤权/ABA/collision/crash/replay/watermark。全量D2/D4语义和真实源解析必须使用独立真实输入，不能从待验effects反推期望值；本次有界模型不等于上述完整产品验收。本文件不授权产品实现或公开规范修改。


D6 revision03源assembly门：真实完整Document跨全部同namespace carrier定位Field尾；Field缺席落实owner尾及已绑定潜在carrier；先全量迁出/迁入再删除空carrier；同owner原位更新；多迁入规范排序；comments、CRLF、unknown namespace及所有未选中raw bytes保留；完整before/after与D4 effects必须由不同路径重算对照，不以inventory顺序冒充物理source位置。


关系读取版本2采用source_node_state、sourceless_node_state、masked_node_state、unprovable_node_state闭集。stateToken/entityStates是D4 RelationReadBinding对D6受保护DependencyProof的技术映射，不是作者内容、身份或SourceVersion；无新增manifest/CLI/UI术语，代码类型RelationReadContext/RelationReadBinding维持D4所有权，版本2成员按正文exact shape。源、实体状态、incidence分别完整绑定；所有source-bearing、sourceless、prepared-origin边界都要覆盖，不能给tombstone填空分类。新Facet request/outcome及关系模型版本2必须同步；Entry、Recurrence及source/copy effect原版本不变。旧模型运行只证明历史形状。

实施验收：directed旧target purge后，真实旧source selector+state+完整incidence允许显式删除/retarget与D3 restore合成；新target必须live/domain，owner实际requiredness及同源一次revision保持。symmetric purge须原子移除全incident；无源owner携带Entry拒绝。源不变lifecycle ABA、state token跨Ref/跨版本/缺失/伪造、masked状态、prepared-origin冒用、负范围分配、incidence并发、每个失败点零写回滚，均独立覆盖。分类投影、四种Facet动作、general update、C/fresh copy桥和完整effects只能使用新版读取，tombstoned端点不得进入source effects。

受信D3 fresh Create conformance须验证结果revision=1、expectedOwnerRevision=1、request声明/initialEntries与完整暂存结果精确一致，成功不二次append/increment；revision0假前像、结果2、缺失/重复initial Entry、现存owner冒用fresh origin都拒绝。普通existing实际修改仍加一次。

## 当前观察与初始化实施义务

接入D6接口§13固定潜在观察scope、D3 stage3与D6最小授权定位、空范围前置权限、当前policy/Registry CAS、replay/recovery/delivery；成对隐藏状态在无权时必须同一not_visible且零业务读取/decision，有权时仍完整检查真实约束。纯policy管理保留control_only恢复路径。实现§14 issuer首次信任与可撤销grant、family固定profile、独立target principal、完整Registry seed/source重绑、初始policy/Calendar配置和scope绑定一次activation；完整fork保留multiplicity且不复制ACL。实现§8.1 scope选择与迁移、空配置删除、控制inbound和ABA版本。模型不是完整decoder/host验收，需在对应实施入口冻结并实际验证所有运输、认证与故障前提。

## D7联合revision03消费补充（未接受）

当前协议基线为D3 wire11/Result9；D4 C及全部领域类型语义不变。新增受控名称和exact位置：PreparationBinding=D3.identity_operation_request.preparationBinding；DefinitionTransfer=D3.intent.plan.definitionTransfers[]；D3-Symbolic-Result/9.Q（定义结果分段）=D3-Symbolic-Result/9的Q；PreparedActionBinding=D7受管不可变准备输入；SourceEnvelopeStateCapability/CommitSequenceStateCapability=D6 Policy/2.capabilities；EffectManifest/EffectBytes=D7只读效果运输。它们均非内容实体或第二ledger。完整定义以同包D3主稿§21、D6 Control Interfaces §15–16及D7专项正文为准。

必须新增实际版本解码/legacy replay、fingerprint差异、planned恢复、保存定义typed引用转移/Locator、完整效果及窄Field正向outcome测试；历史wire10与Result8测试保留历史归属，不以数字替换声称新版本通过。其它原实现影响和必需检查不因本补充被删除。
