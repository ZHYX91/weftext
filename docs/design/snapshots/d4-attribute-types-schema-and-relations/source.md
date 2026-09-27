---
_weftext:
  id: "6e66067a-08b5-4c21-9791-45aaa6a50323"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。

# D4 Attribute Types Schema and Relations



日期：2026-09-01。

## 1. 裁决范围与上游绑定

本候选只裁决 D4：作者属性事实、typed value、Facet schema、FieldId/namespace、可重复值、inline note、来源证据、关系语义，以及适用的 People、Organizations、Calendar、Library、Task 压力 schema。它严格绑定：

- D1 产品表面与能力边界；
- final D2 v2，SHA-256 `873D2487AC27C7C579D64C14CAB106215FBD462286416860225F2F404BC85137`；
- 同包D3完整replacement `wireVersion:11`、Result/9及其§4.1.3a；旧D3 v9是原历史基线，不是当前新请求边界；
- 同包D3 Lexicon replacement，当前共42个概念（39个继承及3个D3新增），保持各自归属并补充本次组合/观察消费映射；
- D2/D3 Implementation Impact；
- `D4-D10-A2 Mandatory Scenario Inputs 2026-08-28.md`，SHA-256 `47375A78115E905DD7B1EA620B8250674A08691E9B85D1795495618415F78D7B`。

上游不变量：Node/Document 单一作者权威、D2 namespace-scoped carrier outer grammar、Task=`ordinary Node + tasks/task Facet`、Template=Core meta-kind、Resource/Annotation owner-local、D3 typed refs/lifecycle、carrier/lexical entry 无 durable identity/locator/Annotation target。D4 不修改这些合同。

## 2. 顶层选择

### 2.1 选择摘要

1. Node 的可组合持续 schema 使用冻结术语 `Facet`，不再引入裸 `Profile`。D2 `Weftext AsciiDoc Profile` 与 CEL/Value Profile 必须始终限定。
2. declared Facet membership 的唯一作者源是 D2 exact Document source 中按 A+ grammar 解析出的 `wf-facets` token set；D4 不增加 membership sidecar、YAML、header alias 或第二读写路径。D4 conformance先验证controller固定的完整D2来源与摘要，candidate自报或同步刷新摘要不能授权修改上游；固定条款的B2/Facet/source-authority结构化解释由独立评审核对，随后可执行六格与membership案例验证该有界解释，不声称正则表达式或静态tuple能够判定任意自然语言变体的语义。
3. 领域事实的唯一作者源是 D2 Attribute Carrier Block 中的独立 D4 entry；普通 header attribute 不自动成为 D4 Field。
4. 一个 entry 只承载一个 canonical Field 的一个 value occurrence。namespace block 只是 source grouping，不是 whole-object value、Facet instance、权限边界、transaction aggregate 或 precedence unit。
5. Field repeatability 由重复 entry 表达；D4 v1 不用 generic array/map/`any` 把独立事实重新包成 blob。只有 schema 明确证明为一个不可分割值的内部成员（例如 recurrence 的有限 `RDATE` set）才可使用有界 typed collection。
6. optional note 内嵌于同一 entry。D4 v1 不新增 Annotation target，因而不触发 D2/D3 reopen。
7. `occurrenceKey` 是 owner Node + FieldId 内、revision-bound mutation 使用的 value-internal selector；它不是 EntityRef、Locator、RecordRef、OperationId、Annotation target 或独立 lifecycle identity。
8. Relation 是 schema-declared relation Field 的一种作者事实；不建立用户可任意扩张的全局 RelationType registry。每个 relation Field 自己声明方向、inverse、symmetry、cardinality、target domain、lifecycle resolution、删除与 graph projection。
9. unknown/uninstalled/incompatible provider 不使字段变成空值或被删除。D2 raw source 保持权威；typed capability 进入显式 unavailable，只有 byte-preserving、write-set 不相交的编辑可以继续。
10. Facet schema v1 只有依赖与互斥，没有继承或 override。组合冲突 fail closed；不存在 source order、安装顺序、UI 顺序或 last-wins。

### 2.2 被拒绝的顶层替代

| 替代 | disposition | 原因 |
| --- | --- | --- |
| 开放 Node type / Node kind 子类 | reject | 产生平行 identity/kind、破坏普通 Node + Facet composition。 |
| 单一 `people`/namespace object blob | reject | 扩大 diff/merge/权限/迁移冲突域，破坏独立 Field/occurrence。 |
| 插件私有作者数据库 | reject | 第二作者权威、卸载丢失、跨表面漂移。 |
| 普通 header attributes 与 carrier 双读 | reject | precedence/fallback 无法闭合。 |
| D6 sidecar 保存领域事实 | reject for D4 | D4作者事实不能进入第二权威；D6另拥有不可由作者源重建的policy、ledger、burn/custody及必要范围控制事实，派生cache可重建。 |
| 每个 phone/name/relation 建 Node | reject | identity 膨胀；与 D3/D5 分域冲突。 |
| 独立全局 RelationType registry | reject for v1 | namespace、inverse、治理、未知 provider 与用户理解成本高；schema-declared Field 足够。 |
| Annotation 作为每个 field note | reject for v1 | 需要 durable field target，正式重开 D2/D3；inline note 已完整闭合当前需求。 |
| 仅 `(field,value)` 或 source range 寻址 | reject | duplicate/reorder/external edit 时命中不确定。 |
| Facet schema inheritance/override | reject for v1 | composition precedence 与 diamond 冲突无必要；显式 requires 足够。 |

## 3. 共享 semantic namespace registry

### 3.1 一个 registry，三个不同名字域

D4 选择共享 namespace registry：D2 `namespaceToken`、FacetId 的 namespace 部分、FieldId 的 namespace 部分必须 exact-equal，并解析到同一个 `SemanticNamespaceId` owner。字符串相同只在 registry owner proof 成功后表示同一 semantic owner。`SemanticNamespaceId`只有一个decoder，逐字继承冻结D2 `namespace-token`：由一个或多个`.`分隔的lower segment组成，禁止空segment；每段首字符`a-z`，后续只允许`a-z0-9`或内部不连续`-`，禁止首尾/连续连字符；完整namespace为1..63 ASCII bytes。任何D4 identifier path都不得退化为仅接受单段KEBAB namespace。

三个完整 ID 仍分域：

- `FacetId = namespace "/" facet-name`，沿用 D2 grammar：`facet-name`独立为1..63 ASCII bytes，完整FacetId为1..127 ASCII bytes；
- `FieldId = namespace "/" local-field-path`；
- `SemanticCodeId = namespace "/" code-path`，只在跨 Field/跨 contribution 引用时必须全限定。

`local-field-path`/`code-path` 为 1..8 个 lowercase ASCII segment，以 `.` 分隔；每段 1..63 bytes、首字符 `a-z`、后续为 `a-z0-9` 或不连续的内部 `-`；同一segment可有多个内部连字符，但`-`不得位于首尾或连续出现。完整 FieldId/SemanticCodeId 最多 255 bytes。comparison 为 exact ASCII code point；禁止 trim、case-fold、Unicode normalization、percent decode 或 locale rewrite。同一个`SemanticNamespaceId` decoder必须用于FacetId、FieldId、SemanticCodeId、alias ID、inverse code、unit ID、calendar ID和external-identifier scheme；各ID再叠加自己独立的完整长度与local部分限制。

### 3.2 owner 与 anti-spoof

registry 的 semantic owner class 是 closed union：`core | first_party | publisher | workspace_user`。D10 后续冻结 package/signature/install mechanics；D4 只冻结可观察语义：

- `tasks`：Core exclusive，绑定 frozen `tasks/task`；third-party/user 不可 claim/shadow。
- `core`：Core usable，只放跨领域且确有 Core 语义的 Field/Code；provider 不可写私有状态。
- `wf`：永久不可分配 semantic namespace，保留给 source/control 语法，任何 claim 拒绝。
- `people`、`organizations`、`calendar`、`library`：first-party reserved semantic namespaces；是否交付 UI/module 不改变 owner。
- `user`：Core 介导的 Workspace custom field namespace；不得 shadow 其他 namespace。跨 Workspace copy 若目标没有相同 custom schema，raw source保留且 typed state unavailable，不能猜映射。
- 其他 publisher namespace 只有在 D10 可证明唯一 owner 时可用；安装顺序、当前 enablement、display name 或本地化 label 不构成 owner proof。

同一 namespace 不得出现两个 active owners。owner unknown/conflict 时，D2 raw source仍可读，但 D4 typed state 为 unavailable，任何触碰该 namespace 或依赖它的 Facet operation fail closed。

D4与D10的握手边界固定为一个不可变、generation-bound只读`RegistrySnapshot/1`。snapshot exact members为`kind,registryGeneration,snapshotDigest,predecessor,rows,calendarComparators,calendarPeriodRuleContributions,calendarSeriesScopePolicyContributions,tzdbContributions,unitContributions,semanticCodeContributions,externalSchemeContributions,fieldSemanticBindings,facetSemanticBindings,semanticTombstones,semanticMigrations`。`snapshotDigest`是D10认证后交给D4的完整canonical snapshot content digest，覆盖除自身外的全部members；`registryGeneration`仍是plan/retry绑定token，二者不能互代。每次decode、catalog load、validation或operation plan都必须携带exact `RegistryBinding/1={expectedRegistryGeneration,expectedSnapshotDigest}`，并逐字同时匹配snapshot的`registryGeneration,snapshotDigest`；同generation下替换为另一份自洽snapshot仍固定`incompatible_schema`，不能因新digest自洽而接受。

每个namespace row exact为`namespaceId,ownerClass,ownerId,aliasDefinitionsState,aliasSchemaDigestSet,facetDefinitionsState,facetSchemaDigestSet,fieldDefinitionsState,fieldSchemaDigestSet,registryGeneration,verificationState`；三个definitions state各自只允许`complete|unavailable`：`complete`下空digest set证明“已完整装载且确实没有定义”，`unavailable`下对应digest set必须为空且表示定义集合不可证明，两者不可互换。七类contribution的nested exact objects分别为：calendar comparator `{kind:"calendar_comparator_contribution",calendarId,calendarVersion,precision,comparatorId,orderedLexemes}`；calendar period rule `{kind:"calendar_period_rule_contribution",calendarId,calendarVersion,periodKind,periodRuleId,keyProfile}`；calendar series/scope policy `{kind:"calendar_series_scope_policy_contribution",policyId,policyVersion,policySchemaDigest}`（identity为`policyId,policyVersion`，payload digest为§9.4完整closed policy的D3-CJ/3 SHA-256，policyId须经同snapshot verified-owner preflight，版本不得为空）; tzdb `{kind:"tzdb_contribution",tzdbVersion,zoneIds}`；unit `{kind:"unit_contribution",unitId,dimensionId}`；semantic code `{kind:"semantic_code_contribution",codeId}`；external scheme `{kind:"external_scheme_contribution",schemeId}`。`dimensionId`是verified-owner SemanticCodeId；quantity Field可据此做closed dimension约束，unit ID本身不暗示维度。这里的dimensionId证明由当前`unitContributions[].dimensionId`及相应namespace的verified-owner检查共同完成，不要求它另列于`semanticCodeContributions`；measurement的byCode仍须匹配当前unit contribution ledger。所有collection必须先证明JSON array类型，空object/string、null、Boolean、number均不能冒充空array；malformed container/element必须返回closed incompatibility而非host exception。所有数组按对应identity tuple的D3-CJ/3 canonical bytes严格升序、identity唯一；ID、bounds、closed profile、zone/lexeme集合与canonical bytes逐项验证，unknown member、null、重复、乱序、超界或不属于verified owner的contribution使整个snapshot `incompatible_schema`。calendar period contribution还必须满足closed `periodKind↔keyProfile`映射`day→iso-date-v1|week→iso-week-v1|month→iso-month-v1|quarter→iso-quarter-v1|year→iso-year-v1`，不能把任一合法profile重新配给另一kind。D4只接受`verificationState=verified`且同namespace恰一owner的row。namespace owner lookup及RegistryBinding验证必须在读取、解析或解释inner Entry JSON之前完成；零个verified row固定`namespace_owner_unprovable`，多个可用owner row固定`namespace_owner_conflict`，generation与plan不等固定`registry_generation_changed`，snapshot digest或loaded definition digest不兼容固定`incompatible_schema`，且失败路径不得调用inner parser。签名、publisher身份、package安装/信任/撤销和row如何产生由D10冻结。D4不得读取package安装顺序或凭据自行补proof，D10也不得改变已验证schema的D4语义。

reserved owner tuple逐字冻结为`core→(core,weftext.core)`、`d4→(core,weftext.d4)`、`tasks→(core,weftext.tasks)`、`people→(first_party,weftext.people)`、`organizations→(first_party,weftext.organizations)`、`calendar→(first_party,weftext.calendar)`、`library→(first_party,weftext.library)`、`user→(workspace_user,<current WorkspaceId>)`；`user.ownerId`必须逐字等于D10已认证的当前D3 WorkspaceId，其他Workspace、publisher或其他owner class均不可claim。任何reserved tuple不匹配在Entry解析前为`namespace_owner_unprovable`。`wf`永久不可分配，snapshot中出现任何`wf` row都同样拒绝。每个loaded alias/FieldDefinition/FacetSchema必须完整展开alias、通过closed validation、按D3-CJ/3计算digest并命中其verified namespace row的对应digest set；标记`complete`的namespace必须同时证明反向覆盖：已装载alias/Field/Facet的各自digest集合与advertised集合逐项相同，漏载或多载均使context构造失败。只有证明完整的集合才可判断known/unknown定义；完整空集合可返回unknown，state=`unavailable`统一在内部Entry解析前返回`provider_or_schema_unavailable`。calendar comparator/period rule/series-scope policy、tzdb zone、unit、registry-scope semantic code和external scheme必须命中同一RegistryBinding绑定的contribution table，不能只证明namespace owner。

Workspace上下文绑定：成功的 `ValidatedCatalogContext` 必须不可变地绑定一个已认证的当前 WorkspaceId。该身份可来自D10提供的显式host Workspace，或同一已认证Registry中唯一verified `user` owner row；同时提供时二者必须逐字一致，否则构造失败。公共接受路径中的source containing owner必须与该context Workspace一致；在Entry内部解析、Facet解释、关系或Calendar/recurrence效果规划前，不匹配固定返回 `namespace_owner_unprovable`，失败无write set、无成功read set并保留完整原状态。Registry结构校验只确认 `user` 属于合法D3 WorkspaceId与 `workspace_user` class，不将conformance fixture中的Workspace常量当作产品身份。此约束针对消费context的source owner，不把普通value/provenance中的跨Workspace NodeRef改写或禁止；其解析可用性与目标约束仍按各自契约判断。

`fieldSemanticBindings`与`facetSemanticBindings`分别按`fieldId|facetId,semanticMajor` canonical排序；每项exact `{fieldId|facetId,semanticMajor,semanticDigest}`。Field digest覆盖完整展开`valueType`以及`shape,qualifierSetId,cardinality,occurrenceOrder,duplicatePolicy,constraints,relation`；Facet digest覆盖`requires,conflicts,fields,relations,constraints`。`predecessor`只有两种closed arm：受D10 trust-root认证、只允许一次的`{kind:"registry_bootstrap",bootstrapId:"weftext-d4-semantic-ledger-v1"}`，或后续每一代必需的`{kind:"registry_predecessor",registryGeneration,snapshotDigest}`。每个非bootstrap validation plan必须提供closed `RegistryEvolutionProof/1={previousSnapshot,currentSnapshot}`；current predecessor必须exact绑定previous的generation与完整digest，两个snapshot都先独立通过认证与closed validation，不能只把当前ledger与自己比较。

目录装载的唯一成功输出是不可变 `ValidatedCatalogContext`：先完成RegistryBinding、closed snapshot与必要的RegistryEvolutionProof比较，再完成同binding的catalog装载。非bootstrap缺proof不得暴露可用于Entry/操作接受的definitions；proof.currentSnapshot必须与当前完整snapshot相同，previous须命中predecessor。公共Entry、Facet和Calendar/Relation planner只消费该context及显式revision-bound输入；materialized inspection copies不可替代context，也不可回写改变它。低层schema/decoder函数只用于有界fixture谓词，不能单独出具完整acceptance结果。该conformance factory执行结构、digest与evolution检查；D10的真实签名和trust-root认证仍是明确的上游输入前提，不声称Python对象封装实现了真实provider信任。


单份 snapshot 在交给任何 catalog/context 或 transition 入口前也必须证明历史内部一致：active 与 tombstone identity 互斥；每个 tombstone 恰有一个 from identity/digest 匹配的 migration；每个 migration 的 target digest 命中当前 active binding 或后来退役的 tombstone，历史链无环且最终抵达 active identity。bootstrap 是没有退役与迁移历史的首次 ledger；不能把历史导入伪装成新的 bootstrap。认证前提不豁免这些结构与语义检查。

逐代比较同时覆盖Field和Facet：保留的同ID major-1 digest必须byte-equal；同ID升major、换digest或静默删除固定`incompatible_schema`。每个消失的active ID必须在current出现exact tombstone `{kind:"semantic_tombstone",semanticKind,semanticId,semanticMajor:1,semanticDigest,retiredInGeneration,reasonCode:"replaced_by_migration"}`，其中新墓碑的`retiredInGeneration`必须逐字等于首次记录该退役事实的current `registryGeneration`；后续代只能原样累计，不能改写或伪造退役来源。每个退役ID还必须有恰一个exact migration `{kind:"semantic_migration",migrationId,semanticKind,fromId,fromDigest,toId,toDigest}`指向fresh active ID；from/to digest必须分别命中previous/current ledger，fromId不得等于toId，orphan tombstone、orphan migration、删除无migration、replacement无tombstone、digest不符或退役代不符全部拒绝。migration记录identity固定为`(semanticKind,fromId,toId)`，与snapshot唯一性验证和排序使用同一identity；`migrationId`是记录中不可变的UUID payload，同一迁移批次可以在不同identity记录中复用，绝不能用它有损归并历史。`semanticTombstones`与`semanticMigrations`是跨全部后续generation累计且逐项byte-immutable的历史ledger：current必须原样包含previous全部记录，任何已tombstone ID永久不得再次active，不能靠隔一代删除历史后以新digest复活。七个contribution table也采用monotonic identity ledger：既有calendar comparator `(calendarId,calendarVersion,precision)`、period rule `(calendarId,calendarVersion,periodKind,periodRuleId)`、series/scope policy `(policyId,policyVersion)`、tzdb version、unitId、codeId、schemeId不得删除或在同identity下改payload；新语义必须使用fresh identity。与旧语义无替换关系的fresh Field/Facet ID可独立新增且不得伪造migration。corpus必须用完整snapshot覆盖bootstrap、wrong predecessor、同ID Field/Facet mutation、silent deletion、fresh replacement缺项、完整replacement、tombstone mismatch、retirement-generation mismatch、standalone addition、三generation复活攻击以及tzdb/unit同identity mutation；单份当前map或fixture自报“mutation accepted”不构成证据。

贡献预检与结构诊断共用声明槽位的constructor识别前提：先按Field/QualifierSet或D3引用位置确定允许的constructor/closed arm；缺失、未知或不匹配的constructor仅报告该处类型诊断，不解释其内部成员，也不解析内部贡献ID，独立兄弟成员仍继续。D3未标记的source span不虚构kind要求。识别成功后，所有contribution-backed identifier在其包含对象的结构/type被解释前都必须以同一snapshot递归完成availability preflight与namespace owner proof；这明确覆盖object/union/collection内值、range bounds、qualifier `validity|eventTime|observedAt|status`、external provenance的`scheme|observedAt`、registry-scope semantic code、calendar ID/version/comparator、tzdb version/zone、unit ID、external scheme、alias ID与每个directed relation inverse code。语法合法但owner或具体contribution未验证时绝不降级为普通unknown string，也不得由wrapper归一为`invalid_value|invalid_qualifier|invalid_provenance`；依§8保留raw并进入`retained_unavailable`或在触及operation中fail closed。一次decode只看已绑定snapshot，不可在中途刷新或混合generation。

## 4. canonical source：D4 Entry/1

### 4.1 outer binding

D2 outer bytes保持不变：

```adoc
[weftext-attributes]
....
namespace people
entry {"v":1,"field":"name","occurrenceKey":"5e6d8b56-8f22-4b97-a6e1-1aacbf2f9a78","value":{"kind":"object","members":{"role":{"kind":"semantic_code","code":"ordinary"},"text":{"kind":"text","text":"张三"}}}}
....
```

D4 只解释 `rawEntrySource`。payload 必须是严格 JSON 的一个 object，UTF-8 已由 D6/D2 前置保证。JSON duplicate member、trailing token、NaN/Infinity、浮点 number、unknown envelope key、missing required key、illegal null 全部拒绝。object member source order 不产生语义优先级。

### 4.2 closed entry envelope

字段恰为：

| key | required | contract |
| --- | --- | --- |
| `v` | yes | integer `1`；其他值 `unsupported_entry_version`。 |
| `field` | yes | block-local field path；与 block namespace 展开成唯一 FieldId。不得重复 namespace。 |
| `occurrenceKey` | yes | canonical lowercase RFC 4122 UUID text；variant必须RFC 4122且version允许`1..5`，不要求v4；只在同一 owner Node + FieldId 内唯一。 |
| `value` | yes | closed typed value，禁止 null/`any`/untyped JSON。 |
| `qualifiers` | no | 由 Field semantic shape 允许的 closed object。 |
| `note` | no | 非空 Unicode plain text；无 note 时必须省略，不存空占位。 |
| `provenance` | no | 1..16 个 D3-compatible provenance atoms；只解释来源证据，不授权、不拥有、不改变 identity。 |

`field` 展开规则：`namespaceToken + "/" + field`。entry 不得声明别的 namespace。多个同 namespace blocks 按 source order 形成一个 semantic stream；block 没有 identity/precedence。唯一性键固定为 `(owner NodeRef, expanded FieldId, occurrenceKey)`：只有同一owner、同一展开后FieldId的第二个相同key才是semantic duplicate，即使value相同也reject；不同FieldId可合法复用相同key bytes。检查必须跨该Field在全部同namespace blocks中的entries，不能按block重置，也没有last-wins。

### 4.3 occurrenceKey 的严格非身份边界

`occurrenceKey` 只解决同一 Node、同一 Field 中 duplicate values 的精确 patch/reorder/note 命中。mutation request 必须同时绑定 owner NodeRef、FieldId、occurrenceKey 和 exact expected source revision；缺任一项都不能定位。

本D4的规范性选择明确为非身份方案：occurrenceKey是value-internal、revision-scoped selector，不授予独立lifecycle或跨revision continuity。一个revision中取得的完整selector只对该revision有效；后续revision中的操作必须重新绑定当前作者源与其exact source revision，不能仅凭相同key bytes承接先前定位或授权。move/rename保留Entry、fresh-owner copy/import/template instance保留key bytes，仅表示作者源的字节保留；既不建立跨revision Entry identity，也不建立跨owner continuity。D4据此不新增D3 field target、EntityRef、Locator或AnnotationTarget，当前选择不要求D2/D3 reopen；未来若选择真正的跨revision durable field-target能力，必须另行正式reopen，不能把本selector静默升级。

- key 不进入 D3 EntityRef/Locator/AnnotationTarget decoder；
- key 不可单独 resolve、跨 owner 查询、授权或形成 link；
- delete 后不存在 tombstone/restore；重新加入相同 key 必须视为新 source fact且需防 ABA 的 D6 revision/CAS；
- identity-preserving Node move/rename保留 entry bytes；跨owner创建fresh Node的copy/import/template instance可以保留key bytes，因为scope中的owner NodeRef已改变且不表示identity continuity。只有在同一owner+Field内复制一个occurrence时必须生成fresh key；D3 identityMap从不包含这些keys；
- Workspace fork/continue只按 D3 mode 处理完整 source；key 不能证明两个 Workspace facts identity-equal；
- 如果未来需要 cross-revision durable field target、field link 或 Annotation target，必须提交 formal D2/D3 reopen，当前 key 不能升级。

### 4.4 flat/header 与 carrier 映射

普通 D2 header attribute 从不因名字像 `people.name` 就自动成为 D4 Field。D4 canonical source只有 carrier entry。

显式 Map Attribute Action 可以把一个合法 header attribute或外部 YAML/JSON 值预览为一个或多个 D4 entries；plan必须列出 exact source range、target FieldId、type conversion、new occurrence keys、loss、provider/schema依赖与删除/保留原 source 的选择。commit 后只保留用户选择的一个 canonical target；不得双写或设置 fallback。

plain external export 可以产生 full-FieldId flat artifact；re-import 仍必须 preview 后写回 carrier。flat artifact/cache 从不是 Weftext Document authority。

## 5. D4 typed-value algebra v1

### 5.1 closed type constructors

Facet/Field schema 只能组合以下 constructors：

1. `text`：Unicode string；normalization policy显式为 `exact | nfc-for-compare`，source bytes不被自动重写；schema可用exact `nonEmpty:true`拒绝空字符串，省略时允许空。
2. `boolean`：JSON true/false。
3. `integer`：canonical base-10 string，`0`或`-?[1-9][0-9]*`；实现先检查位数/范围再转换。 D4作者integer的有符号canonical string与D3Integer控制域不同；后者是0..2^63-1的非Boolean JSON integer，二者不得互换。
4. `decimal`：canonical finite base-10 string，exact grammar为`(?:0|-?[1-9][0-9]*|-?(?:0|[1-9][0-9]*)\.[0-9]*[1-9])`；因此`-0.5`与`0.5`合法，`-0|-0.0|0.0|1.20`非法。禁止 exponent、NaN、Infinity、negative zero、trailing fractional zero；numeric zero唯一写作`0`。measurement精度必须用显式precision qualifier/field，不靠lexical trailing zeros。
5. `semantic_code`：Field-local short code或完整 SemanticCodeId；schema必须声明哪种。
6. `calendar_date`：`calendarId`、`calendarVersion`、`precision=year|month|day`、provider-defined canonical `lexeme`。每个calendar/version/precision必须由当前verified registry snapshot提供closed canonical decoder与versioned deterministic comparator；只有该comparator能决定日期先后，Core不得以lexeme byte order代替。Core内建 `calendar/iso8601` v1，严格验证ASCII `[0-9]`组成且year统一在`0001..9999`内的`YYYY|YYYY-MM|YYYY-MM-DD`及真实Gregorian日期；year/month/day precision都拒绝`0000`，unknown/missing comparator保留 raw并 typed `retained_unavailable`，不按设备 locale猜。
7. `zoned_instant`：D4 v1采用严格RFC 3339 exact-instant profile：所有数字位置只允许ASCII `[0-9]`，四位非零Gregorian local year `0001..9999`、真实日期、`T`、24-hour `00..23:00..59:00..59`、可选任意精度fraction、以及`Z`或合法`±00..23:00..59` offset；`-00:00`与leap-second `:60`因不能在该profile下独立证明exact instant而拒绝。decoder必须用exact integer/proleptic Gregorian arithmetic得到任意fraction精度不丢失的唯一UTC instant；decoded UTC domain不再附加宿主timestamp year限制，所以合法local boundary经offset可落到proleptic year 0或10000一侧。禁止Unicode digit class、shape-only regex、binary float、宿主microsecond截断、offset normalization或host date-range conversion。RFC3339 offset只参与解码该唯一instant；`timeZone`是同一instant的显示/日历上下文，不要求等于源字符串offset（例如`00:00Z`配`Asia/Shanghai`合法并显示为当地`+08:00`）。`timeZone`必须是声明的verified `tzdbVersion`中的IANA zone ID，version/zone contribution缺失时`retained_unavailable`。创建时 gap/fold必须显式裁决并记录最终 instant，读取不重新猜设备默认 zone。
8. `date_range`：calendar_date bounds；`start`/`endExclusive`各为 typed bound或 `{kind:"unbounded"}`，至少一端有界；两端有界时同 calendar/version/precision且由该verified versioned comparator证明 start < endExclusive。
9. `instant_range`：zoned_instant bounds或 unbounded，至少一端有界；两端有界按decoded exact UTC instant证明 start < endExclusive，源字符串与offset表示不得参与先后比较。
10. `quantity`：canonical decimal + namespaced `unitId`；本阶段只冻结原值无损存储、unit identity与dimension compatibility，原值不被派生显示覆盖。当前exact snapshot没有换算factor/offset/algorithm/digest，单位换算保持不可用；D10须先冻结经过认证的versioned conversion contribution合同，D7/D8随后才能消费其明确绑定的派生结果，不可按unit名称或设备locale猜换算。
11. `node_ref`、`resource_ref`、`annotation_ref`：必须直接复用冻结D3的完整closed decoder，包括UUID v4、exact member set、Unicode-scalar token、`D3Integer=0..2^63-1`、source-span ordering及三种Locator；禁止D4近似decoder。每次Entry validation显式携带containing `ownerNodeRef`；所有direct authored ResourceRef（包括普通value和provenance）与direct authored AnnotationRef都必须以D3-CJ/3 canonical bytes证明其`owner == ownerNodeRef`。这是D4 authored-value locality规则，不修改D3 ref identity。裸 UUID/path/title/URL、ambient owner补全或member-order敏感比较拒绝。
12. `external_identifier`：closed `{kind:"external_identifier",scheme: SemanticCodeId,value: non-empty text}`；它不是 NodeRef。spec可进一步给出exact contribution-backed scheme set，不能因scheme在全局registry存在就跨Field混用。
13. `object`：schema列出 closed member set、required/optional、各自type；禁止 additionalProperties。
14. `union`：schema列出 2..8 个带唯一 `kind` discriminant的 closed variants；禁止 structural guessing。
15. `bounded_set` / `bounded_sequence`：只可作为closed object的直接或递归内部member，schema必须给item type、minimum、maximum `1..256`、nesting depth与set uniqueness/order；不能作为顶层Field valueType或Field repeatability的替代，不能承载独立note/permission/lifecycle。recurrence的`rDates/exDates/by*`是允许例，phones/names/engagements不是。

D4 v1 不提供generic author-facing array、open map、opaque object、binary或 executable value。独立事实repeatability用entry；Resource bytes用D2 Resource；大文本用Document/Resource，不塞进 field。

### 5.1.1 authored TypedValue wire

每个作者值必须有string `kind`，先确认discriminant再成功解码；缺kind、非string kind、未知kind或wrong constructor都不能以invalid_value同时表示成功。qualifier的enum也先检查scalar类型再查成员，不能把dict/list交给host hash set。每个作者值都是下列exact JSON之一；所有object都拒绝unknown/missing/illegal null，nested value递归使用同一表：

| constructor | exact authored wire |
| --- | --- |
| `text` | `{kind:"text",text:<Unicode string>}` |
| `boolean` | `{kind:"boolean",value:<JSON boolean>}` |
| `integer` / `decimal` | `{kind,value:<canonical string>}`；range由ValueTypeSpec检查。 |
| `semantic_code` | `{kind:"semantic_code",code:<declared short code or SemanticCodeId>}` |
| `calendar_date` | `{kind:"calendar_date",calendarId,calendarVersion,precision,lexeme}` |
| `zoned_instant` | `{kind:"zoned_instant",instant,timeZone,tzdbVersion}` |
| `date_range` / `instant_range` | `{kind,start,endExclusive}`；每个bound是对应typed value或exact `{kind:"unbounded"}`。 |
| `quantity` | `{kind:"quantity",decimal:<canonical decimal>,unitId:<SemanticCodeId>}` |
| `node_ref` / `resource_ref` / `annotation_ref` | `{kind,nodeRef|resourceRef|annotationRef:<frozen D3 typed ref>}` |
| `external_identifier` | `{kind:"external_identifier",scheme:<SemanticCodeId>,value:<non-empty text>}` |
| `object` | `{kind:"object",members:<object>}`；member names与required/optional完全由ObjectMemberSpec/1决定。 |
| `union` | `{kind:"union",variant:<declared token>,value:<variant TypedValue>}` |
| `bounded_set` / `bounded_sequence` | `{kind,items:[<TypedValue>...]}`；只在parent ObjectMemberSpec允许时合法。 |

set items必须按每个item的D3-CJ/3 canonical UTF-8 bytes unsigned byte-lexicographic升序、无重复；sequence保留作者顺序。浮点JSON number与`NaN|Infinity|-Infinity`在递归解析的任意深度都拒绝，不能等到类型转换后才发现。

### 5.1.2 schema meta-wire v1

限额必须在每个直接或组合消费入口共用：先保留source Workspace、Registry、namespace owner和schema availability的前置屏蔽，再对raw Entry作有界UTF-8计数，之后才解析、提取诊断span或构造该Entry的候选状态。Facet预处理的pre-state与initial Entries也适用；被Cleanup移除的Entry不例外。schema先以memoized、saturating计数证明完整展开大小（含JSON escaping、UTF-8、标点、重复alias及完整Field/Facet外层），再展开或生成完整canonical buffer；alias本身的限制对象仍是expanded root，不能因identity wrapper而缩小合法根类型。计数不替代depth/cycle/placement与原有语义校验，不增加数值或时间精度限制。仅用于比对的未解释source副本先作逐字比较；未修改的relation read引用须经过共享gate后才能复制成返回候选。所有拒绝保留原始source/order、空write set和null成功read set。

`ValueTypeSpec/1`是closed tagged object，而不是实现内部class。合法exact member sets为：

- scalar spec：`{kind}`，其中kind为`boolean|decimal|calendar_date|zoned_instant|date_range|instant_range|quantity|node_ref|resource_ref|annotation_ref|external_identifier`；
- text spec：`{kind:"text",normalization:"exact"|"nfc-for-compare"}`或增加唯一可选exact member `nonEmpty:true`；`false|null`非法；
- integer/decimal bounded spec：`{kind,minimum:<canonical string>,maximum:<canonical string>}`，integer另可使用exact `{kind,minimum,maximum,excludedValues:[1..64 sorted unique canonical integer strings]}`；也可使用前述无界scalar member set；minimum必须小于等于maximum，excluded value必须落在bounds内；
- semantic code spec：`{kind:"semantic_code",codeScope:<CodeScopeSpec/1>}`，其中CodeScope只有`{kind:"field_local",codes:[1..256 sorted unique short codes]}`、`{kind:"namespace",namespaceId:<SemanticNamespaceId>}`或`{kind:"contribution_set",codes:[1..256 sorted unique SemanticCodeIds]}`；field-local short code逐字使用1..63 ASCII bytes的lowercase kebab grammar，namespace code逐字使用完整SemanticCodeId grammar与255-byte上限；`contribution_set`中的每项还必须命中当前RegistryBinding的semantic-code contribution，运行值既要命中该set也要继续命中同一snapshot，三种scope不能互相接受；
- external identifier spec：无额外限制时为`{kind:"external_identifier"}`；Field-specific scheme domain使用`{kind:"external_identifier",schemeScope:{kind:"contribution_set",schemes:[1..256 sorted unique SemanticCodeIds]}}`，每项装载时与运行时都必须命中当前external-scheme contribution，值还必须属于该exact set；
- alias spec：`{kind:"alias_ref",aliasId:<FieldId-shaped registry-local ID>}`，只在schema registry中合法；alias必须存在、无环，展开后才计算depth和digest；
- object spec：`{kind:"object",members:[ObjectMemberSpec/1...]}`；member为exact `{name,required,valueType}`，name是lowerCamel ASCII token，数组按name的unsigned UTF-8 bytes升序、唯一，1..64项；
- union spec：`{kind:"union",variants:[UnionVariantSpec/1...]}`；variant为exact `{variant,valueType}`，variant是lowercase kebab token，数组按variant升序、唯一，2..8项；
- collection spec：`{kind:"bounded_set"|"bounded_sequence",minimum:<0..256>,maximum:<1..256>,itemType:<ValueTypeSpec/1>}`，`minimum<=maximum`；它只能出现在ObjectMemberSpec的`valueType`位置，不能是FieldDefinition顶层valueType、alias根或另一collection的direct itemType。

递归depth从FieldDefinition顶层valueType根记`1`；每次alias expansion本身加`1`，进入object member、union variant或collection item也各加`1`，全局maximum=`8`。每个alias expanded root、完整展开FieldDefinition和FacetSchema的D3-CJ/3 canonical UTF-8 maximum都为`65536` bytes；每个raw Entry JSON maximum=`65528` UTF-8 bytes，使D2物理行按exact 6-byte `entry `前缀加最多2-byte CRLF仍不超过冻结的`65536` bytes；object members maximum=`64`、union variants maximum=`8`、collection items maximum=`256`、provenance atoms maximum=`16`。超限在转换/分配前拒绝。schema canonical bytes与digest必须由完整展开且验证通过的closed object按D3-CJ/3产生；registry/provider不得用程序class name、map iteration order或package version参与digest。`mutually_exclusive_members`适用于展开后的root object，或root union的object variants。至少须有一个适用object；装载时每个名字必须命中每个适用object variant的direct member；运行时只对已通过typed validation的active object variant检查同时出现的成员数，非object variant不应用成员互斥。recurrence的date/instant两variant均必须解析count/until，不依赖root恰为object；relation `targetDomain:"node_ref_or_text"`的target union必须逐字是`node→node_ref`与`text→text`，不得接受kind等价的改名或交换。

所有schema meta-wire整数先按冻结D3 `D3Integer=0..2^63-1`验证，JSON Boolean绝不等于integer；再应用各member较窄范围。这覆盖catalog/Field/Facet `wireVersion`、Field/Facet `semanticMajor`、globalLimits、collection bounds、CardinalitySpec及provenance `inputIndex`；v1 header/semantic major必须是non-Boolean exact `1`。`CardinalitySpec/1` exact为`{minimum:0|1,maximum:<positive D3Integer|"many">}`，integer maximum时必须`minimum<=maximum`。所有v1 `FieldDefinition.cardinality.minimum`与directed `RelationDefinition.targetCardinality.minimum`规范值都固定为`0`，它们不表达business requiredness；`maximum`才是相应post-state limit。`FieldConstraintSpec/1` v1只有exact `{kind:"at_most_one_preferred"}`、`{kind:"mutually_exclusive_members",members:[2..64 sorted unique ObjectMember names]}`与`{kind:"measurement_unit_dimension",codeMember,quantityMember,byCode}`；最后一种要求两个名字分别命中root object的required semantic-code/quantity member，`byCode`按code排序且每个dimensionId命中当前unit contribution ledger，运行时只有完整value通过typed validation及相关contribution可用性验证后才执行FieldConstraint；qualifier/provenance的独立错误仍单独报告，不能修改source-start诊断排序。measurement code对应的expected dimension必须逐字等于quantity.unitId contribution的dimensionId，否则在unit token固定`invalid_value`。目录中的`people/height→core/length`、`people/weight→core/mass`，因此height+kg和weight+cm都拒绝。`FacetConstraintSpec/1` v1只有exact `{kind:"required_field",fieldId,when:"effective"}`与`{kind:"union_variant_equal",leftFieldId,rightFieldId,when:"both_present"}`；后者的量化规则为：同一owner的两个union-root Fields都至少有一个已通过typed validation的occurrence时，两侧全部occurrences必须共同使用唯一一个相同variant；允许任意多个同branch occurrence，数量和作者顺序不参与branch equality。任一侧缺席时本约束不失败，requiredness仍单独求值。两侧具有相同的混合variant集合也必须返回constraint_conflict，不能用集合相等、multiset相等或逐位置配对替代此规则。Create/Assign必须满足effective required Field与branch equality；Facet仍effective时删除最后一个required occurrence拒绝；移除Facet结束要求，保留Field进入`retained_without_membership`。未列出的constraint kind拒绝，不得由provider私增解释，也不得仅因global catalog存在FieldDefinition推断requiredness。

measurement 的 `byCode` 必须逐项覆盖 code member 在同一 RegistryBinding 下的完整合法 code 域，既不能缺项，也不能添加该域外的项。`contribution_set` 域是其 exact codes；`namespace` 域是当前已验证 contribution 中该 namespace 的全部 codes；field-local scope 与 namespaced mapping 不兼容，装载时拒绝。namespace 增加 code 后必须重新证明完整映射，不得让新 code 静默跳过维度约束。运行时缺少 expected dimension 仍明确拒绝，不能解释为无约束。

### 5.2 semantic shapes 与 qualifiers

每个 FieldDefinition 必须声明一个shape以及与之exact匹配的`qualifierSetId`：

| shape | allowed qualifiers | meaning |
| --- | --- | --- |
| `fact` | `validity?: date_range|instant_range`, `selection?: ordinary|preferred|deprecated` | 普通事实/状态；历史由多个 occurrences 表达。 |
| `event_assertion` | required `eventTime: calendar_date|zoned_instant`, `confidence?: decimal[0,1]`, `selection?: ordinary|preferred|deprecated` | 对一次事件可有多个相互冲突assertions；preferred只是显式作者选择。 |
| `observation` | required `observedAt: zoned_instant`, `confidence?: decimal[0,1]`, `selection?: ordinary|preferred|deprecated` | 带时间/单位的观测；latest/preferred是派生 projection。 |
| `relation` | `validity?: date_range|instant_range`, `status?: semantic_code` | target与relation schema共同决定edge语义。 |

四个QualifierSetSpec/1的ID固定为`d4/fact-qualifiers-v1|d4/event-assertion-qualifiers-v1|d4/observation-qualifiers-v1|d4/relation-qualifiers-v1`。每个spec exact为`{qualifierSetId,shape,members}`，member exact为`{name,required,wireType}`；四个完整objects由规范性catalog annex逐字给出，数组按name升序。wireType闭集为`typed_validity_interval|selection_enum|typed_event_time|typed_confidence_decimal|typed_zoned_instant|typed_semantic_code`，分别使用5.1.1的typed wire（selection例外为raw `ordinary|preferred|deprecated` string）。`typed_semantic_code`在v1明确采用registry-backed open namespace scope：值必须是语法合法且namespace owner proof可用的完整SemanticCodeId；共享relation qualifier set不另带Field-local `codeScope`，未知或owner不可证明的namespace仍按§3/§8 fail closed。

`qualifiers`是closed object：只能出现该QualifierSetSpec列出的members，missing required、unknown member、illegal null或wrong type拒绝。`confidence`比较canonical decimal后必须在`0..1`；schema的`at_most_one_preferred` constraint使同一Field两个active preferred entries冲突，不按source order选一个。权威划分唯一：validity/status/eventTime/observedAt/confidence/selection只能在qualifiers；role/position/rank/department及其他domain payload只能在value。相同概念不得同时出现在两处，也没有precedence/equality fallback。event payload的domain code/内容留在`value`，时间只在required `eventTime`。

### 5.3 provenance atoms

`provenance` 只允许以下exact atoms；validator必须调用冻结D3 decoder，不能只检查nested object类型：

- `{kind:"node",nodeRef,locator?}`，`nodeRef`必须通过exact NodeRef decoder；`locator`只允许exact frozen `DocumentElementLocator|DocumentRangeLocator`且locator.owner必须byte-equal该nodeRef；
- `{kind:"resource",resourceRef,regionLocator?}`，`resourceRef`必须通过exact ResourceRef decoder并owner-local；`regionLocator`只允许exact frozen ResourceRegionLocator且其resourceRef必须byte-equal atom.resourceRef；
- `{kind:"external",scheme,value,observedAt?}`，`scheme`是当前verified registry snapshot证明owner的contribution-backed external-scheme SemanticCodeId，`value`是non-empty Source描述文本；二者都不是 SourceBinding/OriginBinding；
- `{kind:"transform",inputIndex,operationId}`，`inputIndex`是zero-based D3Integer，必须小于该atom自身index并指向同一provenance array中确实存在的更早atom；`operationId`必须是D3 canonical lowercase OperationId（canonical lowercase RFC 4122 UUID），只解释生成步骤，不成为content ref。self/forward/out-of-range index、Boolean、`2^63`或任意字符串operationId拒绝。

credentials、provider account、sync token、etag、cursor、SourceBinding、ForeignIdentityKey与OriginBinding不进入普通 portable field entry。D3 Provenance继承成员各自lifetime，不授权、不建立ownership、不证明identity相等。

## 6. Field 与 Facet schema

### 6.1 FieldDefinition/1

每个 FieldDefinition 是 closed object：

```json
{
  "wireVersion": 1,
  "kind": "field_definition",
  "fieldId": "people/name",
  "semanticMajor": 1,
  "valueType": {"kind":"alias_ref","aliasId":"people/name-value"},
  "shape": "fact",
  "qualifierSetId": "d4/fact-qualifiers-v1",
  "cardinality": {"minimum":0,"maximum":"many"},
  "occurrenceOrder": "author_order",
  "duplicatePolicy": "key_unique_values_may_repeat",
  "constraints": []
}
```

非relation Field exact members就是示例中的十一个keys；relation Field额外required `relation`且shape必须`relation`，非relation Field出现`relation`（包括null）拒绝。unknown/extra/missing/illegal null拒绝。`semanticMajor`是non-Boolean exact `1`并服从§3永久binding ledger。`qualifierSetId`必须与shape固定映射匹配；valueType必须通过5.1.2并且顶层不能是collection。Cardinality跨同Node/Field所有blocks计算；symmetric relation例外采用§7.1的endpoint-total解释。value相同允许，只要同Field key不同；业务去重只能是闭集schema constraint并返回诊断，不能自动merge。

Field definition独立于 Facet membership而可解码；Facet引用FieldIds并增加 required/conditional constraints。Remove Facet 后field entries保持已知typed facts，状态为 `retained_without_membership`；它不是 invalid，也不自动cleanup。

`FieldDefinition/1.duplicatePolicy`的闭集只有`key_unique_values_may_repeat`一个值，其他值固定拒绝。v1允许同Field中value字节相同、occurrenceKey不同的多个出现项，但每项仍须满足type、cardinality及已声明constraint；v1没有`distinct_values`约束。新增duplicatePolicy值须显式升级wire版本并接受语义兼容性评审，不能静默扩宽v1。

### 6.2 FacetSchema/1

Facet schema 是 closed、versioned registry contract：

```json
{
  "wireVersion": 1,
  "kind": "facet_schema",
  "facetId": "people/person",
  "semanticMajor": 1,
  "requires": [],
  "conflicts": [],
  "fields": ["people/name"],
  "relations": ["people/parent", "people/sibling", "people/spouse"],
  "constraints": []
}
```

FacetId 是 semantic-major identity。`semanticMajor=1` 对已冻结 v1 ID 永久绑定；同 ID 后续 registry revision必须保持§3 semantic ledger中的完整Facet semantic digest不变，包括requires/conflicts/fields/relations/constraints；新增optional Field也会改变该payload，必须使用fresh FacetId。label/view/action等外部contribution可独立演化，但不能改变既有 Field type/cardinality/relation语义、使旧valid source invalid或改变write effect。任何breaking semantic change必须新FacetId并走显式migration；不靠provider package version改变作者含义。

FacetSchema exact members就是示例九个keys；`requires/conflicts/fields/relations`都按D3-CJ/3 bytes升序、唯一，fields与relations不重叠且都必须解析到同catalog FieldDefinition。v1只允许：

- `requires`：effective closure，派生且不回写 `wf-facets`；
- `conflicts`：无序对称冲突；
- exact FieldId集合和closed constraints；
- 无继承、override、priority、shadow或implicit membership。

declared set 由 D2 source order投影但语义无序。effective closure必须确定、无环；missing dependency、cycle、conflict、同Field不相容定义都使相关typed state unavailable/invalid并阻断触及操作。

catalog loader必须机械证明requires图无self-edge、无2-node或更长cycle；`conflicts`必须是无序对称关系且禁止self-conflict；任一Facet的effective closure若同时包含冲突对也拒绝。只检查引用存在不构成conformance。

同一catalog支持至多1024个Facet；此上限不收紧D2作者declared集合的32项上限，也不另加依赖深度上限。目录全图与节点相关依赖闭包共用迭代式图计算，每个已完成闭包只计算一次并供多个父节点复用；支持上限内的合法长链及汇合DAG不得依赖宿主递归栈、提高进程递归限制或被降格为无效schema。节点组合只遍历其声明可达的依赖，错误不得返回可用于accept的部分闭包。

Facet的`relations`只引用并暴露已定义的relation Field，不拥有或改写该Field的`subjectPredicate`。新的兼容Facet可经合法Registry演化引用既有relation Field，其引用不改变Field payload、semantic digest或主体域。Field域中的Facet仍须完整解析；实际每次关系接受仍以§7.1中Field自有的subject/target谓词逐端校验。仅列出relation引用而不具备所需effective membership的节点不能因此通过关系校验。

#### 6.2.1 Source-bound Node classification

保留D2的Task谓词：成功且绑定当前source revision的D2完整分类结果中，`coreKind="ordinary"`并且作者declared NodeFacetSet显式包含exact `tasks/task`，才是Task。typed Task Fields、Task UI启用状态以及requires派生的effective Facet都不能替代该声明。Template携带explicit tasks/task属于D2不可接受的来源；D4保留上游诊断，不能另造D2错误。若一个本身有效的D2声明经D4组合使effective closure含有Task而其源并不是Task，则为`facet_conflict`，即使完全没有关系Entry也不得产生成功typed node state。

通用requires图仍合法；extension直接、多层或diamond依赖Task均可装载，但Node必须同时显式声明Task才可使用这些扩展。Task与独立Project Facet可共存；Template可携带不依赖Task的合法Facet。`compose_facet_definitions`只是schema图组合；Node接受路径必须进一步使用共享`compose_node_facets`。公共Entry/有限case检查只证明对应作者值与约束，不单独证明D2 Node分类，不能代替Facet、关系或recurrence操作的节点检查。

传输复用现有owner/source revision：Facet及recurrence pre-state新增`coreKind`，其`declaredFacetIds`来自同一NodeRef、ownerRevision的D2完整解析；可信关系node state仅在§7.4 source-bearing variant中携带`coreKind,declaredFacetIds,effectiveFacetIds`，绑定该项`nodeRef,sourceRevision,stateToken`。coreKind只允许ordinary或template；declared是0..32项合法、排序、唯一的D2集合，effective必须等于从declared通过当前catalog组合出的闭包。缺失、错误、不可证明或与预期revision不符的分类零写失败，不能从effective补齐declared、采用caller isTask布尔值或把缺失kind默认为ordinary。trashed endpoint仍有真实Document，其分类绑定保留的实际source revision；tombstoned/not_found endpoint没有当前Document，必须用§7.4无源variant，禁止sourceRevision、coreKind或Facet字段。masked/unprovable只表达受限读取结果，不能携带伪源。没有分类与分类为空是不同事实；历史cache不能冒充当前D6读取。

Facet owner的pre-state（现存操作是真实前像；受信fresh Create是§6.3已物化拟议结果）必须与可信关系读取逐项匹配coreKind、declared、effective和source revision。拟议membership exact为`{nodeRef,declaredFacetIds}`，只适用于本次affected owner；共享关系检查沿用已证明coreKind并重新组合，不接受caller supplied effective closure。四种Facet动作不改变coreKind；其planned declared变化必须对应同一D6 source patch，并在提交前满足D2完整解析及CAS。source声明改变即使effective closure未变，也必须读取和绑定全部相关incidence scopes。coreKind或source声明在其他操作中变化时必须使owner revision失效，任何旧读取都不能继续提交。

Task关系的subject/target域逐端消费上述显式谓词，其他Facet域仍消费effective closure。共享检查同样用于混合Task/Calendar节点的recurrence edit与projection；关闭Task界面不改变Core分类。保留tasks/status等scalar但移除最后一个Task声明后，该Node不再是Task；字段保留不恢复分类。Python conformance只验证闭合输入、绑定一致性和语义，D2真实full parse、当前source真实性、权限和原子提交仍是D6的可信输入前提，不把自报分类字段当作认证证明。

### 6.3 Create、Assign、Remove、Cleanup

D4冻结纯语义 plan，不冻结D6 authz/CAS/receipt transport：

1. `CreateWithFacets`：显式declared set、initial entries、Template output（若有）一起validation；无默认隐写。`when:"effective"`要求在post-state同时满足，Facet-required initial entries由plan明确列出并获得fresh occurrence keys。
2. `AssignFacet`：preview effective closure、continuing requiredness、conflicts、initial fields、unknown providers与source patch；post-state必须满足所有effective Facet的required Field，保持NodeRef。
3. `RemoveFacet`：只移除declared membership；仍有dependent Facet时拒绝移除其依赖；当前closed action每次只移除一个Facet，必须先移除dependent，再移除依赖，不声称实现atomic multi-remove。字段默认保留为 `retained_without_membership`。
4. `CleanupFacetFields`：独立显式Action；只列出不再被任何declared/effective Facet使用的candidate entries，用户逐项选择。零silent delete。

四种动作共用closed wire。`FacetOperationRequest/2` exact为`{kind:"d4_facet_operation_request",wireVersion:2,operationId,operationKind,expectedOwnerRevision,expectedRegistryBinding,expectedRelationReadBinding,expectedRecurrenceReadBinding,declaredFacetIds,targetFacetId,initialEntries,cleanupSelectors}`；`operationKind`只允许`create_with_facets|assign_facet|remove_facet|cleanup_facet_fields`，不适用member用`null|[]`而非省略。`cleanupSelectors`每项exact `{fieldId,occurrenceKey}`，完整selector唯一；相同key在不同Field合法。preview、unused-field证明、删除与post-state对比逐字复用同一selector，重复selector或无法唯一匹配时零写失败，未选中的raw Entry必须逐字保持。每个entry reference exact `{fieldId,occurrenceKey,rawEntrySource}`；必须先证明namespace/Field定义可用性，再strict parse原始Entry并绑定同一field/key。不可用时零内部解析，不得用decoded object替代作者源。pre-state exact `{nodeRef,coreKind,declaredFacetIds,entries,ownerRevision,registryBinding}`；outcome exact `{kind:"d4_facet_operation_outcome",wireVersion:2,operationId,status,postState,writeSetOwners,readSet,recurrenceReadSet,rollbackByteEqual,intermediateStateObservable:false}`。现存源的成功实际修改只增加一次owner revision并保持NodeRef；受信fresh Create按下述分支保持D3初始结果revision=1；Remove只改declared membership且Entry raw bytes不变；Cleanup只删除request逐项列出且已证明不被任何effective Facet使用的occurrence；unknown Facet、RegistryBinding/CAS失配或任一constraint失败都返回byte-exact pre-state、空write set、不可观察中间态。

受信D3 fresh `create_with_facets`采用§7.4 prepared-origin分支：此处pre-state是已绑定完整拟议结果源的只读语义输入，不是现存Document前像；ownerRevision与expectedOwnerRevision均为该D3初始结果revision=1，分类/declared/entries必须逐项匹配同一结果payload和context。request声明集合及有序initialEntries必须正好等于该完整结果声明/作者Entry序列（包括显式Template output），不是在已物化结果上再追加一次。D4只验证其全部post-state约束，成功postState保持相同源/revision=1，writeSetOwners表示原D3 fresh source一次发布，不增为2，不构造revision=0的假源。失败保留该暂存输入且不发布任何源；原D3 allocation/burn规则不变。这个分支只能由已验证D3 fresh Node集合启用，普通现存Node不能借create绕过更新CAS，客户端不能自报prepared-origin。普通existing Assign/Remove/Cleanup仍从真实前像构造拟议源，实际源改变时恰增一次；完全不改源不制造source revision。新建前需要准备初始字段的UI/Template plan先形成完整D3 result payload，再调用本语义验证。

operation kind的成员适用表在任何作者Entry解析和post-state构造前验证：

| operationKind | declaredFacetIds | targetFacetId | initialEntries | cleanupSelectors |
|---|---|---|---|---|
| create_with_facets | 0..32 unique合法D2 FacetIds | null | 显式Entry references | [] |
| assign_facet | [] | 一个合法D2 FacetId | 显式Entry references | [] |
| remove_facet | [] | 一个合法D2 FacetId | [] | [] |
| cleanup_facet_fields | [] | null | [] | 完整且唯一的显式selectors |

Create请求的声明顺序不影响集合，重复声明必须拒绝而非去重。已有pre-state和Create/Assign后的declared集合均不得超过D2的32项上限；所有不适用参数必须精确为表中null或空数组，任何额外内容零写失败，不得静默忽略。


每个plan必须绑定 exact owner NodeRef、source revision、RegistryBinding、declared/effective before/after、entry-level write set与ordered diagnostics，并显式消费§7.4的可信关系读取上下文。四种动作必须把拟议的Entry及显式declared membership（fresh Create直接取已绑定完整结果，不二次append或increment）一起送入共用后状态校验并派生effective closure；局部Field/Facet校验通过不能单独产生accept。Remove若使仍保留的incoming/outgoing关系失去subject/target domain，返回`relation_target_invalid`并保留完整pre-state；若仍有另一effective Facet证明该域，则可保留。Cleanup必须把所选事实的删除纳入同一关系后状态。成功outcome携带完整`readSet`，失败为`null`。architecture corpus从显式request/pre-state/read-context执行四种动作、typed validation/duplicate/effective requiredness、完整依赖与绑定失配、Remove/Cleanup及零写回滚；案例清单以同代源语料为准。D6以后增加actor/authz/idempotency/receipt，不得改变D4 semantic effect。

Create 与 Assign 的 `initialEntries` 均保留作者输入顺序；可对语义无序的 Facet 声明规范排序，但不得按 occurrenceKey 或 canonical Entry 内容重排 author_order 的断言。

成功Assign的Entry后状态必须逐项等于原有Entry序列再接上本次显式initialEntries：原有reference、raw bytes及相对author order全部保持，新增项不得缺失、重复或出现未请求项。conformance必须断言这一完整可观察结果；仅执行命名案例或检查status/NodeRef不构成preservation证据，并须以删除原有Entry的结果变异确认该断言确实能检出退化。

## 7. relation model

关系更新的有序作者源变换：保留同一containing owner与Field的retarget必须在原位置替换选中occurrence；不得按canonical JSON、key或target排序整个作者stream。canonical owner relocation先从旧stream移除选中occurrence，再插入目标owner同Field stream的末尾；该Field尚无occurrence时附加到目标owner已有作者stream末尾，目标owner在投影inventory中尚无事实时附加其新stream。全部未选中occurrence的相对作者顺序及raw source逐字保持。公共relation-update模型的entries投影保留每个owner/Field的该顺序；跨owner inventory interleaving不声明物理Document位置，D6必须把此保序结果绑定到真实source patch与revision。无序read binding、derived projection与allocation仍可canonical排序；任何后状态或提交前条件失败均返回完整原状态和空write set。


### 7.1 RelationDefinition/1

literal target的空值规则由对应Field的展开TypeSpec唯一决定。当前People relation的text arm未声明nonEmpty，故允许exact空字符串；Entry、Facet/case和原子relation update必须一致接受，不得在操作入口暗加non-empty限制。空literal仍是显式text arm，不是缺失成员、null或未解析NodeRef，不生成target inverse、graph edge或自动Node；该规则不改变主体Facet域、source cardinality和完整后状态校验。未来某个Field显式声明nonEmpty时，所有路径都通过同一绑定schema执行该限制。

Relation 不是第三种ref，也不是Document link/Citation/structural parent/Annotation reply。subject永远是containing Node，不写入relation object。`RelationDefinition/1`只有两个exact member sets：

- directed：`{kind:"relation_definition",targetMember,targetDomain,direction:"directed",inverseCode,sourceCardinality,targetCardinality,subjectPredicate,targetPredicate,resolutionPolicy,deletePolicy,graphProjection}`；
- symmetric：`{kind:"relation_definition",targetMember,targetDomain,direction:"symmetric",selfEdgePolicy,subjectPredicate,targetPredicate,resolutionPolicy,deletePolicy,graphProjection,endpointPurgePolicy}`；它必须省略`sourceCardinality,targetCardinality`。

`targetMember`必须指向展开后object valueType的required member；`targetDomain`只允许`node_ref|node_ref_or_text`且与该member type逐字一致。`subjectPredicate,targetPredicate`各自是closed EndpointPredicate：exact `{kind:"ordinary_node"}`或exact `{kind:"facet_any",facetIds}`。后者facetIds是1..8 sorted unique、已装载FacetIds，要求该端effective Facet set至少交一项；其中`tasks/task`仍必须使用冻结D2的ordinary Node加declared Task判据，effective-only不成立。ordinary_node只要求同一受信读取中的coreKind严格为ordinary，不要求任何Facet，不自动Assign。两种条件都不能豁免same-Workspace、source/incidence revision、可见性、生命周期、授权或原子写入约束；未知kind、额外成员、空Facet列表均拒绝。条件不成立固定`relation_target_invalid`。text arm没有target Node，只检查subject条件。目录逐Field定义该domain：`people/engagement`为Person→ordinary Node，People family/social relations为Person→Person，Organization relations为Organization→Organization，`library/creator`为Work→Person|Organization，Task dependency为Task→Task。只有明确ordinary_node的端点允许无Facet普通Node，不能据此放宽其他Field。

`inverseCode`是directed required SemanticCodeId，必须同时命中同一RegistryBinding的`semanticCodeContributions`；symmetric时必须省略。`selfEdgePolicy=accept|reject`只在symmetric required。directed两个cardinality都用CardinalitySpec/1，target minimum在v1固定`0`；`sourceCardinality`必须逐字等于owning FieldDefinition cardinality，不能形成第二来源限制。symmetric的唯一cardinality authority是owning `FieldDefinition.cardinality`：对任一endpoint Node按FieldId聚合所有active incident NodeRef facts（无论该Node是canonical storage owner还是target）加该Node locally authored literal facts，形成endpoint-total；同一canonical fact在同一endpoint只计一次，超过maximum固定`field_cardinality_conflict`，结果不得随提交方向或storage role变化。`endpointPurgePolicy`在symmetric唯一为`explicit_remove_before_either_endpoint_purge`：任一endpoint仍有active incident fact时purge拒绝；显式cleanup与endpoint purge必须是同一D6原子事务，任一步失败完整回滚。`resolutionPolicy`唯一为`live_required_on_create_retain_suspended`，`deletePolicy`唯一为`retain_fact_explicit_cleanup`，`graphProjection=none|general|ranked|family`。relation object没有qualifier declaration；唯一Qualifier权威由Field的`d4/relation-qualifiers-v1`给出，role等domain成员只在value。

`targetDomain:"node_ref_or_text"`的union arm行为全局固定，无需另一个隐式policy：NodeRef arm服从targetCardinality、resolutionPolicy、target lifecycle、inverseCode与graphProjection；text arm是unresolved literal authored value，永不进入target resolution、target cardinality、inverse projection、graph projection、target-not-visible diagnostics或target lifecycle。`targetCardinality`在post-state按exact `(Relation FieldId,target NodeRef)`跨Workspace聚合active authored NodeRef-arm occurrences；任何仍保留的active source fact都计数，不因target为resolved、suspended、tombstoned、not_visible或unprovable而改变，且该计数本身不得泄露target state；text永不计数，超过maximum固定`relation_cardinality_conflict`。source cardinality、occurrence lifecycle、explicit cleanup、qualifiers与author order仍适用于两个arm。两个相同text strings不产生Node identity或共享relation target，same-title Node/provider不得将text自动coerce为NodeRef；text→NodeRef只能是显式atomic occurrence update。`node_ref`-only relation拒绝text arm。

v1不支持cascade delete、implicit target creation、title/path matching、cross-Workspace target或Resource跨owner relation。

### 7.2 一份事实与inverse

directed relation权威事实只在subject Node的Field entry。inverse是可重建projection，不双写。structural move不改relation。

symmetric relation的NodeRef arm只允许同Workspace NodeRefs。只有NodeRef arm做canonical endpoint ownership：canonical fact owner按两端`nodeId`的D3-CJ/3 canonical UTF-8 bytes做unsigned byte-lexicographic比较，较小者为owner；相等是self-edge并按Field schema显式accept/reject，不能进入比较tie fallback，entry target是另一端。`node_ref_or_text`的text arm始终由当前containing owner authored，不存在第二endpoint、canonical endpoint owner、inverse、edge、target cardinality或target lifecycle；因此spouse/sibling/family-related/social-related/professional-relation的literal值也是ordinary authored literal fact，不是假Node。任一端UI发起NodeRef arm都必须由D6未来事务对canonical owner授权并写同一份事实；无canonical owner写权限则原子失败。不得在两端各写一份spouse/friend。

architecture conformance必须在stateful one-fact store执行而非只复述此规则：store同时保存raw D4 Entry source bytes与一次解析所得typed value、qualifiers、note、provenance、occurrenceKey与resolution；每次载入都证明`strictParse(rawEntrySource)==entry`。从相同empty stores分别提交A→B与B→A的same occurrenceKey/same payload必须得到byte-equal完整canonical post-state；在同一shared store依次从两端提交same key/same payload是idempotent且仍只有一份source fact，same key/different payload固定`duplicate_occurrence_key`，不同keys依schema cardinality保留为不同facts；self-edge按policy accept/reject；对同一spouse fact trash/restore任一endpoint只切换`resolved→suspended→resolved`；删除inverse/graph index后从single source facts rebuild必须得到byte-equal projections，且canonical owner选择不依赖调用者或授权顺序。

任何text↔NodeRef或NodeRef endpoint变化都是一个atomic occurrence update，preserve同一occurrenceKey与其余Entry members。若更新使canonical owner改变（例如B上literal spouse显式解析为NodeRef A），plan必须显式携带`ownerBeforeNodeRef,ownerAfterNodeRef`、两个owner的source revisions、完整remove/add write set与两边CAS；旧owner删除与新owner新增不可分开可见。plan不得携带自报`maximum`：唯一maximum逐次从已绑定owning FieldDefinition.cardinality读取；先删除同一稳定source-fact identity的旧事实与旧projection/allocation，再加入拟议事实形成完整post-state。symmetric cardinality必须分别对拟议事实的subject与NodeRef target两个semantic endpoints遍历同Field全部active facts；storage owner不代替endpoint，NodeRef source fact按稳定事实identity去重，local literal只计其authoring subject，`many`视为unbounded且不做numeric compare。目标owner已有相同`(FieldId,occurrenceKey)`但payload不同固定`duplicate_occurrence_key`并零写；collision、任一owner未授权、revision失配、任一semantic endpoint-total cardinality冲突、target lifecycle失败或projection写入失败都必须返回pre-state raw source bytes、revisions、projections与allocations的byte-exact整体副本，空write set且不暴露intermediate state；解析后JSON canonical equality不够证明rollback。成功的NodeRef→NodeRef必须替换旧projection/allocation，NodeRef→text必须删除旧derived state；不得append stale projection/allocation。独立state-machine conformance必须从输入pre-state计算结果并对CAS/auth/collision/local与incoming cardinality、两端orientation、text→NodeRef、NodeRef→text、NodeRef→NodeRef、`many`、lifecycle/injected failure/raw-byte rollback逐项断言；不得把fixture `expected`复制成computed result，修改expected必须使测试失败。

target后来trashed时，relation fact保留且resolution=`suspended`；restore可重新resolved。tombstoned/not_visible/unprovable沿用D3不泄露结果，默认不自动删除source fact。purge/cascade policy由D6在本`retain_fact_explicit_cleanup`语义下实现；任何cleanup显式、可预览。



relation模型的source fact exact为`{ownerNodeRef,fieldId,entry,rawEntrySource}`，`entry`仅是同raw的一次解析缓存。没有独立持久`subjectNodeRef`；containing owner就是源subject，symmetric另一endpoint只由raw target恢复。operation中的subjectNodeRef仅表示当前提交方向，必须等于旧作者owner或旧symmetric NodeRef target，不可成为第二作者权威。owner改变时必须把raw target写成新owner的另一端，并用同一context在actual post-state owner下重新校验完整Entry；保留的ResourceRef/AnnotationRef若因此不再owner-local，v1原子失败，不得静默删除、reparent或改身份。若需copy/remap，须另行明确执行D3合规操作后重试。

source revision inventory只覆盖本次实际读取的source-bearing owners/endpoints；独立entity-state inventory覆盖全部必要完整NodeRefs，包括无源端点。两者与incidence范围共同形成§7.4 binding，不能把无源端点补成第0版Document。source-bearing node state将真实分类与sourceRevision、lifecycle/stateToken绑定，所有实际write owners都必须在唯一owner↔source revision映射中出现，逐项CAS匹配；缺项、错owner、重复owner或过期均零写。成功恰对每个实际改写source owner增加一次revision，未改写owner不增；projection/allocation从post-state raw source重建且定位含owner+FieldId+occurrenceKey。整个状态模型为D6提供可验证语义，D6仍负责认证read snapshot、权限和物理原子提交。

### 7.3 relation vocabulary governance

relation kind与固定direction由FieldId拥有，不另设开放registry。`people/parent`、`people/spouse`、`people/sibling`与`organizations/parent`是不同Field；一个Field永不声明`directed/symmetric`混合行为。provider只能贡献自己namespace的Field/Code；custom relation若没有自有closed FieldDefinition，只能写入该namespace已冻结的neutral symmetric `related` Field或text fact，不能冒充parent/manager/family rank。

### 7.4 共用后状态与完整读取依赖

关系更新必须对每个受影响的source owner检查本次变更适用的effective Facet约束，包括canonical relocation后不再有本地relation stream的原owner。required_field始终要求该owner实际作者源中的Field occurrence；incoming inverse/projection不能替代。关系操作的约束依赖范围由该owner发生变更的Field决定，membership变更时扩至其全部relation Fields。当前closed Facet constraint algebra仅允许required_field涉及relation；union_variant_equal的两端只能是非relation union Fields，因此不能把关系变更扩大成对无关非relation namespace的读取。所有适用Field的完整incidence scope、owner source revision、before/after membership与RegistryBinding须先证明，再执行与Facet操作共用的约束求值。删除或迁移最后一个仍被要求的occurrence必须返回required_field_missing，并保持raw source、revisions、projection、allocation逐字回滚与空write set；只有所有适用后状态条件通过后才分配成功版本和写入集合。

`CreateWithFacets`、`AssignFacet`、`RemoveFacet`、`CleanupFacetFields`与relation occurrence update使用同一`validate_proposed_relation_state`语义。输入是当前可信读取上下文、请求期望binding、拟议完整关系事实集合、实际受影响owners及拟议effective membership；结果只含`status,readSet`。只有`accept`允许后续增加作者版本、生成write set或提交投影。任何失败保留原始作者状态、零写集且无可观察中间态。

D6提供与同一不可变读取视图绑定的`RelationReadContext/2`，exact为`{kind:"d4_relation_read_context",wireVersion:2,registryBinding,nodeStates,entries,incidenceScopes}`，后三者为数组。此处为受信Core→D4语义接口，不是向principal披露完整上下文的API。认证、授权和完整性由D6证明，客户端自报complete/stateToken不能成为证明。

`nodeStates`是按完整NodeRef排序且每Ref恰一项的closed union：

| kind | exact members | 语义 |
| --- | --- | --- |
| `source_node_state` | `kind,nodeRef,lifecycle,stateToken,sourceRevision,coreKind,declaredFacetIds,effectiveFacetIds` | lifecycle仅live/trashed；真实完整Document存在，sourceRevision为D3Integer，分类按§6.2.1从该源和Registry产生 |
| `sourceless_node_state` | `kind,nodeRef,lifecycle,stateToken` | lifecycle仅tombstoned/not_found；前者是确定最小tombstone，后者是同cut完整inventory证明的不存在；均没有当前Document，严禁源revision/分类/Facet字段，包括null/0/空集占位 |
| `masked_node_state` | `kind,nodeRef` | 当前必要状态不可披露；不是实体生命周期，不提供状态token或作者事实 |
| `unprovable_node_state` | `kind,nodeRef` | 必要状态不可证明；不是不存在证明，不提供状态token或作者事实 |

nodeStates必须覆盖全部实际raw source owners及本次用到的旧/新NodeRef endpoints，不能省略删除目标。Facet及其他写入owner、entries中的任一当前作者owner必须为source_node_state；tombstoned/not_found owner携带当前Entry固定拒绝。source_node_state的declared是D2合法sorted unique集合，effective是同Registry完整组合闭包；缺源/坏源/缺分类不能通过sourceless variant规避，必须失败。not_found不是第五实体生命周期；此词仅为D3 resolver的确定读取结果。foreign Ref仍按D3和same-Workspace边界拒绝，不制造本地不存在证明。

stateToken是非空Unicode scalar string的D6受保护状态依赖引用，不是D3 identity、source revision、43字符能力Token或caller签名。D6在同cut绑定完整Ref、实际状态、store/authority连续性与独立lifecycle版本；source-bearing及tombstoned都必须提供，不能仅给tombstone加状态依赖。live→trashed→live即使sourceRevision不变也使旧stateToken失效。not_found绑定完整inventory负范围版本，后来分配该Ref或无法证明同一负范围即失效。D6可保守用其完整contentSequence/control依赖，不能用当前state文字相同当CAS通过。状态版本/受保护依赖记录是Core控制事实，不向D3 tombstone增加历史分类、payload、parent或旧ACL。

- `entries`每项仍为§7.2原始source fact，按`(ownerNodeRef,fieldId,occurrenceKey)`唯一。公共Entry decoder在真实source owner下验证Registry/strict JSON/typed Ref和locator形状/owner locality/raw与cache一致；它不要求旧NodeRef target现在live或仍有Document。目标域与生命周期由以下操作gate按旧/新事实角色判定，不能偷偷塞回旧Entry decoder。
- `incidenceScopes`每项仍exact `{fieldId,endpointNodeRef,revisionToken,factSelectors}`，selector仍exact `{ownerNodeRef,fieldId,occurrenceKey}`。Field/endpoint唯一且完整覆盖当前原始facts，包含空集合。无源endpoint的scope由仍存在的作者源和完整反向范围证明，不读取其不存在的Document，不因endpoint为tombstoned/not_found就把scope填空。保留事实的结构性计数继续按§7.1，不等于live navigation或披露授权。

membership变化读取该owner全部适用关系Field的incident scopes；关系变化读取旧/新owner和NodeRef target在对应Field的scopes。已读取但未改变的其他事实不递归扩张到其所有端点所有Field；actual dependency inventory仍完整，缺项或范围未知不能自动补空。

`RelationReadBinding/2` exact为`{kind:"d4_relation_read_binding",wireVersion:2,nodeRevisions,entityStates,incidenceRevisions}`，三数组按条目D3-CJ/3 bytes排序且按语义键唯一：nodeRevisions项仍exact `{nodeRef,sourceRevision}`，只且完整覆盖context中的source_node_state；entityStates项exact `{nodeRef,lifecycle,stateToken}`，只且完整覆盖全部已证明source_node_state/sourceless_node_state；incidenceRevisions项仍exact `{fieldId,endpointNodeRef,revisionToken}`，完整覆盖全部实际读取范围。masked/unprovable不能产生成功binding/readSet。expectedRelationReadBinding必须精确等于实际可信binding，不能缺源、缺状态、增伪源或借用另Ref token。D6原子提交同时CAS Registry、source、entity-state、incidence及适用授权依赖；state token不是免检凭证。成功只为实际source writes增加一次作者revision；无源B和其他只读端永不进入source write set。

共用gate的唯一先后为：外层D6先执行适用D3 disclosure/当前操作权限；随后D4严格解码新版context/request/binding并验证Ref coverage及source-bearing来源形状；masked/unprovable或无源source owner立即失败；同cut认证/expected binding精确相等；Registry及旧raw Entry/source selector/revision证明；完整旧/新incidence scopes；构建唯一proposed state；最后验证本操作实际需要的subject/target域、生命周期、基数、Facet requiredness。结构、版本、来源、绑定、范围或必要状态不可证明固定operation_precondition_failed（Registry变化沿registry_generation_changed）；拟议NodeRef目标非live或域不成立固定relation_target_invalid，所需域的Registry不可用沿relation_target_unavailable。已有ordered diagnostic及namespace masking仍适用；此顺序规定操作precondition与后状态阶段，不改Entry内部诊断排序。对principal的外层错误继续由D3/D6遮蔽，任何新variant不得暴露隐藏Ref状态。

旧/新事实的判定按真实before/proposed diff，不信任客户端标签：

1. 新增、owner迁移、target变化或copy产生的新NodeRef关系，必须证明新target live、完整域/Schema、权限和post-state基数。§7.5经D3证明的fresh fork初始化与合法trashed保留是原有窄例外，不因新版读取扩大。foreign/not_found/tombstoned目标不因external-preserve变合法。
2. 显式删除或retarget的旧事实，只证明它确实在source-bearing A的前像中、完整selector/revision、旧target确定状态及完整旧incidence；删除阶段不重新接受其旧target，不要求tombstoned/not_found B的源或分类。retarget后的live C单独完成第1项。A的当前subject域及最终requiredness仍须证明；失败完整零写。
3. 原样保留的事实须完整保持owner/Field/key/raw及端点。source-bearing目标需要本操作适用的真实分类证明；trashed按原policy suspended。directed tombstoned/not_found旧目标可按retain_fact_explicit_cleanup保留确定非live resolution，不能标为可恢复Trash，不伪造target域，也不从结构性计数删除该事实；需要新增/改写或对其当前域作新断言时不能使用此例外。masked/unprovable必要端点仍失败，cleanup不绕授权。symmetric的explicit_remove_before_either_endpoint_purge保持：任一端purge前全部active incident facts须显式清理，Trash/隐藏不等于事实已移除；不得用directed保留分支接受遗留symmetric tombstoned事实。

restore+cleanup由D3原restore plan绑定：context是不可变前像（A source_node_state/trashed，B sourceless/tombstoned）；D3已验证的原mode lifecycle effects向共用后状态求值器提供A→live的确定拟议状态，不能预先改context或由D4客户端自报restore权限。普通Facet/关系编辑不取得restore/purge能力；create/fork fresh初始化仍仅限各自已验证D3输入。源清理、A lifecycle变化、全部state/source/incidence CAS、derived invalidation和原D3 receipt+完整D6 effects在同一原decision原子保存。纯restore无源改变不增source revision；restore同时实际删除/retarget只增A一次，B不写源且tombstone事实不变。CleanupFacetFields仍只可选择已不被effective Facet使用的Field；一般显式关系删除不冒称该专门动作。

**D3 fresh拟议源的窄边界。** §7.5与§15的fresh result Node尚未发布，其source_node_state是同一已绑定D3 proposal中的完整暂存结果源，而非当前权威实体。只有受信D3适配器提供的exact fresh candidate map/closure可建立此项；stateToken的受保护记录必须标为prepared-origin并绑定mode、candidate map、完整result payload/prospective sourceRevision、拟议lifecycle及原allocation-history/authority读依赖。普通读取或客户端不能选择这种origin，也不能把tombstoned/not_found旧Ref变成prepared fresh。binding wire中的token不暴露内部origin，但Core逐项验证其与已验证D3模式/fresh集合一一相等。对这些fresh项，提交验证原candidate allocation/burn-history/authority CAS及同一planned payload/reservation，不能拿不存在的当前Document做source/lifecycle CAS；所有既有项仍完整验证当前source/state/incidence，禁止因此跳过旧endpoint依赖。source cut与target prepared视图及Registry按原copy/proposal边界分开；fresh-payload proof不冒充旧源证据。fresh Facet Create的request/pre-state/outcome严格按§6.3直接验证完整结果并保持revision=1；copy/fork同样不在结果revision上额外加版。该规则只承接原先的fresh初始化，不新增D4 allocation API或第二decision。

**版本策略与消费者。** RelationReadContext/1与RelationReadBinding/1只属于历史模型/已保存证据，不能进入新candidate准备、验证或提交。新版均显式wireVersion=2，旧版、混版、unknown member或缺版本在入口拒绝，不能给旧nodeStates补字段转换。FacetOperationRequest/2及其outcome、一般关系operation/pre-state/outcome模型的版本2、Create/Assign/Remove/Cleanup、D3 restore/purge物化桥和typed copy/fork都使用上述新binding/readSet。D3已保存decision按原协议重放不重新解释为新context；需要新decision时必须从当前真实cut重建。Entry/1、D2 wire2、D3 wire11、RecurrenceReadContext/1与RecurrenceReadBinding/1、D4 source/copy effects原shape保持；它们仅含真实source owner的revision，不能加入无源端点。历史机器运行报告不变，也不证明新版decoder已经实现。

模型入口`facet_operation_result(pre,request,relation_reads,context,recurrence_reads)`与`relation_update_result(pre,operation,context,relation_reads)`均显式消费该上下文。关系模型operation exact为`{wireVersion:2,fieldId,subjectNodeRef,fromOwnerNodeRef,toOwnerNodeRef,occurrenceKey,nextTarget,authorizedOwners,expectedSourceRevisions,expectedRegistryBinding,expectedRelationReadBinding,injectFailureAt}`，pre-state exact为`{wireVersion:2,entries,sourceRevisions,sourceRevisionOwners,nodeStates,projections,allocations}`，outcome exact为`{wireVersion:2,status,postState,writeSetOwners,readSet,rollbackByteEqual,intermediateStateObservable:false}`。`authorizedOwners`是D6认证后的模型输入，`injectFailureAt`仅为有界失败测试输入，不能成为作者Field或产品请求的自授权能力；sourceRevisionOwners只是测试状态中局部索引名到NodeRef的映射，不是身份。关系pre-state的entries/nodeStates必须与可信上下文精确一致；旧source revision检查继续保留，完整read binding不能省略。

`sourceRevisionOwners`必须只且完整映射source-bearing inventory，不能含无源端点；`sourceRevisions`及`expectedSourceRevisions`必须具有完全相同的局部索引键集，每个值先通过D3Integer检查，再比较确切值；宿主语言的`7.0 == 7`或`true == 1`不得使CAS接受。source/expected/read-set版本均遵守这一整数合同，任一版本类型错误返回operation_precondition_failed和完整零写回滚。

共同关系 gate 对每条必要读取事实的 source owner 都检查生命周期，而不仅检查写入 owner。`not_visible|unprovable` source 零写失败；`tombstoned` owner 的 Document source 已不存在，携带当前作者事实的组合拒绝。`trashed` source 可原样保留；结构性 incidence/projection 记录不代表 live resolution，消费者必须结合同 binding 的 lifecycle 解释为 suspended，不得因 endpoint 是 source 就忽略状态。

### 7.5 Entity Copy 与 Workspace fork 的关系效果

本节是D3 §8.3第8项要求的D4逐Field选择，适用于 `copy_node_subtree`、以fresh identity复制Node的transfer/import/template实例化，以及 `fork_workspace`；各D3 mode的closure、lifecycle、authority与原子边界仍分别适用，不能互换。identity-preserving move/rename/continue不执行本节fresh-owner重排，保留原关系source与occurrenceKey。standalone `copy_resource`/`copy_annotation`不自动复制或改写D4关系；只有D3明确允许的existing-container显式修改才可另行进入D4普通编辑校验。

下表对当前全部27个关系Field逐项适用；新增合法扩展Field同样继承所属direction规则，不允许由UI label、namespace、安装顺序或copy调用者选择另一隐式policy。此规则是D4操作契约，不向closed RelationDefinition/1增加未声明成员。

| FieldId | 复制／分叉策略 |
| --- | --- |
| `calendar/participant` | directed mapped-owner |
| `library/creator` | directed mapped-owner |
| `library/venue` | directed mapped-owner |
| `library/version-of` | directed mapped-owner |
| `organizations/allied-with` | symmetric canonical fresh-owner |
| `organizations/governs` | directed mapped-owner |
| `organizations/owns` | directed mapped-owner |
| `organizations/parent` | directed mapped-owner |
| `organizations/related` | symmetric canonical fresh-owner |
| `organizations/brand-of` | directed mapped-owner |
| `organizations/business-guided-by` | directed mapped-owner |
| `organizations/jointly-led-by` | directed mapped-owner |
| `organizations/member-of` | directed mapped-owner |
| `organizations/subsidiary-of` | directed mapped-owner |
| `organizations/supervised-by` | directed mapped-owner |
| `organizations/territorially-administered-by` | directed mapped-owner |
| `people/engagement` | directed mapped-owner |
| `people/family-related` | symmetric canonical fresh-owner |
| `people/guardian` | directed mapped-owner |
| `people/manager` | directed mapped-owner |
| `people/mentor` | directed mapped-owner |
| `people/parent` | directed mapped-owner |
| `people/professional-relation` | symmetric canonical fresh-owner |
| `people/sibling` | symmetric canonical fresh-owner |
| `people/social-related` | symmetric canonical fresh-owner |
| `people/spouse` | symmetric canonical fresh-owner |
| `tasks/dependency` | directed mapped-owner |

共同reference policy为D3 internal-rewrite/external-preserve，不选择“关系一律保留旧target”的override。source事实仅从被纳入Node的真实D2作者stream取得；inverse/graph/index中的incoming行不是额外source，不能因复制一个endpoint而制造原本不在closure中的事实副本。每条被复制的relation occurrence恰产生一条结果事实；不能静默omit、合并同值事实或生成反向第二副本。

1. **完整映射与保留。** 先由D3证明source cut、identityMap与各mode允许的结果closure。包含owner、relation target、qualifier/value/provenance内所有已知typed refs及locators都按其真实位置处理；target在map内则使用同kind mapped ref。same-Workspace copy的closure外合法live/trashed NodeRef按D3保留；foreign/not_found/tombstoned/owner-mismatch不因“external”而获得保留权。transfer/fork不得保留指回source Workspace或其他foreign/dangling target。literal target始终保持exact text，不做target lookup、不根据文字查找或新建Node。occurrenceKey字节保留但只在新的owner/Field/revision scope内使用；不进入D3 identityMap。
2. **结果owner。** directed与literal事实的结果owner是source containing owner的mapped Node。symmetric NodeRef事实先完成两个endpoint映射／合法external保留，再按§7.2从结果endpoint计算唯一canonical owner与另一端authored target。结果canonical owner若不同于mapped source owner，只允许迁移到本次操作identityMap产生的fresh Node，且该Node必须位于同一target Workspace与结果closure。已有endpoint不属于允许写入集；partial copy若要求在其上保存新canonical事实，整次copy失败。不得为使copy通过自动调整已经承诺的identityMap、凭空扩大closure、写入source Workspace或生成第二作者副本。
3. **保留成员与owner-local约束。** 对所有被复制的Entry保留未受显式ref/locator重写影响的作者字节、qualifiers、note、provenance、occurrenceKey及内部顺序。owner迁移本身不授权复制／移动Resource或Annotation，也不授权改写ResourceRef.owner。ResourceRef/AnnotationRef先按D3 owner-map规则重写；在actual result owner下仍非owner-local则整次失败，不得删除、转成text、丢弃provenance或伪造新Resource身份。node provenance与locator继续服从自身ref/owner/revision规则。transform inputIndex仍指向原provenance序列，不重新排序atoms。
4. **完整结果校验。** 所有关系重写／迁移汇总成一个拟议source状态后，统一验证actual containing owner与authored target域、生命周期／resolution、duplicate occurrenceKey、local及incoming cardinality、declared/effective Facet closure与requiredness。被移空的mapped owner也必须检查，incoming投影不能替代其required authored Field。异质subject/target域的symmetric Field仍按canonical source角色判断；fresh ID排序反转可能使该map不合法，这是明确的整次失败，不得悄悄交换Facet或收窄Schema。结果排序保留各mapped owner原作者stream内未迁移occurrence相对顺序；跨owner迁入的事实按完整source owner RefKey、FieldId、source occurrenceKey排序后追加到相应carrier，不重排原有未选中source。D6提供真实D2 carrier位置与完整source patch，不能把模型inventory顺序冒充Document author order。
5. **无隐式数据损失。** 本节关系policy不隐式删除事实、不自动选择value transformation。用户明确选择的删除／替换必须作为另外完整preview的D4作者source transformation，逐项显示损失与结果约束，并与D3允许的同一操作effect一起接受；普通copy/fork路径不能以“修复”为名代为选择。不存在这种显式plan时，任何无法保留并合法重建的被纳入关系导致whole operation失败；fork也不得omit source live/trashed成员。原source、target已有owner、revision与derived state保持原样；语义失败outcome无writes、无成功readSet、无部分结果。D3 fresh allocation/reservation的burn规则仍由D3决定，D4的零作者写入不声称取消其历史账本。

完整typed-copy接受需要可证明的source interpretation及必要target约束。§8允许的raw byte-preserving copy/export与typed-unavailable状态继续有效：缺失目标custom schema时保留作者raw source并明确不可作typed validation证据，不自动安装／映射schema。若具体D3 identity-changing操作需要重写其中无法证明的typed slot或证明关系后状态，则该次typed-copy plan不得从raw保留推断可提交，必须报告所缺证明；不能将opaque export冒充已完成的typed fork。此区分不赋予修改／丢弃unknown source的权限。

**D4自有闭合语义效果扩展。** `D4RelationCopyEffects/1`不是D3 wireVersion11 reference-slot union的新arm。它作为独立、versioned的D4 receipt extension由D6绑定到同一D3 OperationId、source cut与target proposal。语义wire exact为 `{kind:"d4_relation_copy_effects",wireVersion:1,operationId,sourceRegistryBinding,targetRegistryBinding,sourceOwners,resultOwners,facts}`。`sourceOwners`每项exact为 `{nodeRef,sourceRevision}`，覆盖全部被解释的source containers；`resultOwners`每项exact为 `{nodeRef,sourceRevision}`，覆盖全部结果source containers，包括因迁出事实而为空的mapped owner。数组按完整NodeRef canonical次序排序、无重复；source/result revisions分别属于各自D2 payload，不共享ambient revision。两RegistryBinding使用现有closed binding shape，不能以同generation名字替代精确绑定。

`facts`每项exact为 `{sourceOwnerNodeRef,resultOwnerNodeRef,fieldId,occurrenceKey,beforeRawEntrySource,afterRawEntrySource,referenceChanges}`，恰覆盖每条被纳入关系source一次，按source owner RefKey、FieldId、occurrenceKey排序。owner变化必须与已验证D3 Node map及上述canonical fresh-owner规则相符；raw前后必须分别匹配各自真实carrier及source revision，不能仅比较JSON语义。每个 `referenceChanges` item exact为 `{pointer,before,after}`：pointer是该Entry中真正typed ref或locator根的RFC6901语义路径（以`$`为根），before/after为完整D3 typed对象；locator作为整体重发，不与其内部ref再重复列项。每个实际变化的已知ref/locator根恰有一项，未改变的根不得伪造rewrite项，根之间不得重叠；按pointer的UTF-8字节顺序排序。owner迁移由owner字段表达，carrier/Entry range与occurrenceKey不是D3 ref slot，不能塞进D3 `rewrittenReferences`。该扩展不代替D3 identityMap、payload preimage/result、lifecycle、placement、authorization或receipt验证。

本地copy conformance适配器消费独立于待验effect的可信D3/D6投影，exact为 `{operationId,mode,identityMap,sourceOwners,resultOwners,sourceLifecycles,sourceFacts,locatorResults}`。这不是新增D3 wire；mode来自已验证的实际D3操作，不能由effect自报。`sourceFacts`是完整source cut中被纳入Node的关系作者事实投影，按每个owner的真实作者顺序排列，不从effects反推；`locatorResults`逐source fact对应真实D3 locator重发结果，以不重叠的Entry内完整locator根pointer为key。`sourceLifecycles`按NodeRef排序，each exact `{nodeRef,lifecycle}`，恰覆盖sourceOwners且state为live/trashed；result owner lifecycle必须逐Node map保持，同Workspace copy只允许live source与live结果。此投影所需的closure完整性、Node/Resource/Annotation及locator真实preimage/result与revision证明由D3/D6提供，不能用请求声明代替。

校验时sourceOwners/resultOwners分别恰覆盖Node identityMap的from/to；source owner与result owner的revision分别来自各自真实D2 payload proof。target relation读取中的fresh Node表示已暂存的结果payload，其sourceRevision必须等于resultOwners中同Node的result revision；不得以修改前的空container revision冒充结果证据，也不得在验证完成后对已绑定fresh结果再增一次revision。每个新owner还必须有全部关系Field的完整incidence scopes，并检查全部effective关系requiredness，不能只检查有新增local事实的Field。fork初始化可以保留已证明的trashed owner与suspended事实；该入口仅由复制校验提供经D3证明的fresh owner集合，普通Entry编辑、Facet操作与关系更新没有此初始化权限。完整effect检验产生纯语义结果，不执行物理commit；D6原子失败仍须保留其完整pre-state，D3 lifecycle/receipt证据不被本地模型输出替代。

D4负责上述effect的含义、完全覆盖与source/target校验；D6负责可信snapshot、身份／授权、source patch定位、完整preview、传输及physical CAS/commit。D4本地语义模型可验证闭合plan，但不声称已执行D6复制引擎或实际receipt提交。需要后续实现的运输层不免除当前D4关系决策。

## 8. availability、validation 与 commit composition

### 8.1 per-namespace state

每个 namespace typed state为：

- `available`：owner、schema、entry syntax/type/constraints全部可证明；
- `retained_unavailable`：unknown owner、schema definition unavailable/untrusted/uninstalled、incompatible schema/calendar/unit contribution；raw source保持，不投影为空。关闭UI/module/runtime本身不移除已安装的portable schema definition，因而不会单独改变typed meaning；
- `invalid`：在可证明的schema下entry malformed、type/cardinality/key/constraint/relation冲突；
- `not_present`：source确实无该namespace；必须与unavailable区分。

Node-level typed snapshot可以是 `complete | partial | unavailable`；partial必须携带每个namespace状态，调用方不得把 omitted namespace当empty。D2 exact source/projection不受D4 typed state改写。

### 8.2 operation matrix

| operation | retained_unavailable/invalid namespace |
| --- | --- |
| exact source read/repair view | available，显示raw source与non-disclosing status。 |
| byte-preserving copy/export | available，附typed-unavailable/loss status；不得声称validated。 |
| body或不相交namespace edit | 仅当old/new affected raw blocks byte-equal、Facet closure不依赖该namespace、write-set不相交且D6 CAS通过。 |
| typed query/view/action on affected Field | unavailable/reject；不得返回empty或stale cache。 |
| edit/add/remove affected entry或Facet | reject，除显式repair/remove/cleanup plan能证明完整effect。 |
| schema/provider reinstall | 从current exact source重建；不得使用旧projection。UI/runtime re-enable若schema一直可用，只恢复相应capability，不改变typed facts。 |

### 8.3 deterministic diagnostics

D4 diagnostics envelope为 `weftext.d4.diagnostic/1`，closed fields exact为 `code,namespace,fieldId?,occurrenceKey?,sourceRange?`；v1没有开放`details` map，新增机器成员必须升wire version。排序先source start，后rank，后stable code；同source point固定rank：

| rank | code family |
| ---: | --- |
| 10 | `namespace_owner_unprovable` / `namespace_owner_conflict` |
| 20 | `provider_or_schema_unavailable` / `incompatible_schema` |
| 30 | `invalid_entry_json` / `unsupported_entry_version` / `unknown_entry_member` |
| 40 | `invalid_field_id` / `unknown_field` |
| 50 | `invalid_occurrence_key` / `duplicate_occurrence_key` |
| 60 | `value_type_mismatch` / `unknown_value_kind` / `invalid_value` |
| 70 | `invalid_note` / `invalid_qualifier` / `invalid_provenance` |
| 80 | `constraint_conflict` / `field_cardinality_conflict` / `calendar_series_scope_conflict` / `required_field_missing` / `preferred_selection_conflict` |
| 90 | `facet_dependency_missing` / `facet_dependency_cycle` / `facet_conflict` / `field_definition_conflict` |
| 100 | `relation_target_invalid` / `relation_cardinality_conflict` / `relation_target_unavailable` |
| 110 | `operation_precondition_failed` / `registry_generation_changed` |

unknown provider是typed unavailable而不是D2 parse error。已知schema下unknown Field是invalid；schema本身不可得时不能泄露“field不存在”，统一返回provider/schema unavailable。

diagnostic遮蔽是有意的：namespace owner不可证明时，rank 10在读取或解释inner JSON之前胜出，即使payload随后看起来malformed也只返回non-disclosing owner-unprovable。只有owner/schema proof成功后才产生rank 30+ inner diagnostics；repair surface仍可显示授权的exact raw source。实现不得为了“更具体错误”跨越该遮蔽顺序。

owner/schema proof成功后，strict JSON parser必须在接受任意合法member whitespace、escaped key与UTF-8内容的同时保留每个object key token、value、array item和scalar的exact half-open UTF-8 byte span；path使用解析树位置而非substring查找，因此multibyte text、escaped key、任意whitespace与同名nested keys都不歧义。所有dynamic object-key path component逐字采用RFC 6901 escaping（`~→~0`、`/→~1`），所以顶层key `value/members/x`不与真实nested path `value→members→x`碰撞。validator从真实raw Entry运行同一strict parser、catalog resolver与递归typed validator，收集所有不跨authority mask且由当前bytes可确定的完整`Diagnostic/1` sequence，不能消费fixture自报的`faults[]`，也不能在第一个structural/type error处short-circuit；typed value、qualifier object与provenance atoms均聚合独立fault并定位最窄token。同一object中一个availability-failed child只遮蔽该child的进一步结构解释，已证明独立的siblings（例如非法decimal、missing unit、多个非法qualifier/provenance atom与非法note）仍分别报告。无法strict parse的lone surrogate等raw只产生`invalid_entry_json`，不得从不可解析树伪造nested faults。

每项诊断以其最窄fault key/scalar/value token的start byte作为`sourceStart`，再按完整exact `(sourceStart,rank,stableCode)`排序并只把head作为primary。object member出现顺序只决定sourceStart，control-flow early return、name排序或无空格substring search不得覆盖该顺序。Entry parser产出的local UTF-8 half-open span必须通过closed `D2SourceProjection/1={kind:"d2_source_projection",documentRevisionToken,entryStartUtf8}`投影到绑定该D2 source revision的absolute source range；revision token不得从当前文档隐式补取。corpus必须覆盖nested key/value/array/scalar spans以及multibyte/escaped/whitespace/structural混合，并分别证明只修改raw、只修改expected sequence/span都会失败。`constraint_conflict`逐字表示“一个已通过typed validation的object违反closed FieldConstraint”。所有authority/availability结果（`namespace_owner_unprovable|namespace_owner_conflict|registry_generation_changed|provider_or_schema_unavailable|incompatible_schema`）在相应branch的structural/type解释前递归preflight，穿过object/union/collection/range/qualifier/provenance wrapper时保持原code与state；只有已证明available的child structural/type failure可归一为`invalid_value|invalid_qualifier|invalid_provenance`。

missing nested member没有独立token，诊断保留缺失成员的语义pointer，并使用最近实际存在的包含object的完整half-open UTF-8 span；适用于递归object member、required qualifier及provenance成员。沿已转义JSON Pointer的component边界回溯，不按显示字符串或top-level member起点制造零宽span。已存在的key、scalar、array item与value仍使用自身最窄token span；仅缺失token使用包含object。

诊断遮蔽只改变可见诊断，不产生成功的TypedValue。公共Entry接受器必须在遮蔽前分别保存结构/type与contribution的完整验证结果；只有两者都成功才可提取relation target、解释CalendarPeriod/recurrence或执行依赖完整value的Field constraint。不可用分支仍保留其availability结果，独立note、qualifier、provenance与sibling错误继续按同一source顺序聚合，不能靠外层通用异常捕获替代封闭Entry结果。

strict JSON已解析为object但缺少required envelope member时，产生一项`invalid_entry_json`，定位到整个已解析envelope的UTF-8 half-open span；缺失多个成员也只产生这一项envelope错误，不为不存在的token制造span或重复的invalid-field/key错误。存在的version和Field足以解释相应分支时，继续检查独立value/qualifier；provenance由已知Entry版本、owner和Registry证明，不依赖Field/value存在；note等可独立判断的成员继续报告。未知或缺失版本不能解释依赖该版本的typed payload。排序仍是sourceStart、rank、stable code；未能strict-parse的raw保留单一parse failure，不伪造内部错误。

最终commit eligibility：`D2 eligible AND operation-applicable D4 semantic gate AND D6 gate AND D7 gate`。D4 reject不能制造partial/old D2 projection或第二作者source。

## 9. reference schema catalog v1

本节冻结 architecture-level semantic schema/Field ownership，用于证明通用模型；它不承诺模块已实现、发行或启用。规范性机器目录是同一候选代的 `D4 Reference Catalog Registry v1.json`，以提交包内完整文件及保留的本地副本绑定，不在正文复制包装字节数或哈希。该annex逐字给出4个QualifierSetSpec、22个无环ValueType aliases、61个完整FieldDefinition、7个完整FacetSchema与1个CalendarSeriesScopePolicy；这里的自然语言只能解释，不能覆盖annex。annex语义变化必须形成一致的新候选修订并重验；产品registry自身的语义digest合同仍适用。

目录装载必须先验证globalLimits、alias closure/depth、所有Field/Facet exact member sets、sorted unique arrays、Field/Relation一致性、Facet引用完整性与CJ/3 digest；任一失败使整个catalog unavailable，不能按“能读多少算多少”降级。所有业务Field是否required仅由完整FacetConstraint决定，不靠本节口语推断。

### 9.1 `tasks/task`（Core built-in）

- required on Assign/Create：`tasks/status`，Field-local code `open|done|cancelled`；没有read-time默认写入；
- optional：`tasks/start`与`tasks/due`是closed date-or-instant union，`tasks/completed-at`是zoned_instant，`tasks/priority`是`low|normal|high|urgent` closed code；
- relation：`tasks/dependency`，directed NodeRef，inverse `tasks/dependent`, graph general；
- checklist toggle不创建Task；promotion创建fresh ordinary Node、声明exact Facet、写显式initial facts并以普通Node link替换occurrence；无TaskRef/mirror。

### 9.2 `people/person`

全部可选，Node title独立且可在Create时由preferred name初始化一次，不持续同步：

- `people/name` repeatable object：text、required role code、language/script optional；validity只在fact qualifiers，不在value。alias/former/legal/transliteration都是name role，不另复制aliases；
- `people/email`、`people/phone`、`people/address`、`people/website`是repeatable labeled facts，其required text及present customLabel必须non-empty，label exact contribution set只允许`people/personal|people/work|people/other`；`people/name.text`同样non-empty。`people/account`专用exact `people/account-value={customLabel?:text(exact),identifier:AccountIdentifier,usage?:semantic_code(contributions=people/other|people/personal|people/work)}`。`AccountIdentifier`是closed TypedUnion：`preset`为external_identifier，scheme只允许people/facebook、people/qq、people/wechat、people/x；`custom`为closed TypedObject，required `serviceKey`和`identifier`均是non-empty exact TypedText。全局存在的`library/doi`不能作为account scheme；额外安装user/mastodon等scheme也不扩大preset集合，自定义服务应使用custom分支。custom serviceKey是作者在该断言内声明的服务文本，不是D3 identity、可信publisher或全局注册项；不得自动把文本“wechat”转换为preset。custom identifier是该服务内的账号文本，与serviceKey、usage、customLabel及Entry note独立；不得拼成一个字符串。所有作者文本保持大小写、Unicode与空白原文；仅preset显示标签可本地化，不能回写翻译或自动合并账号。

账号两个分支共享Field的many cardinality、occurrence identity、qualifiers和来源合同。相同服务及账号值可在不同occurrence中并存，显示标签、usage和备注互不覆盖；重复完整selector仍拒绝。服务键/账号值缺失、空字符串、错误类型、未知union分支或多余member均拒绝；自定义服务不要求外部scheme contribution，但People Field/alias自身不可用时仍按既有规则保留raw并阻断typed操作。D9须显式把已证明的preset映射为preset，否则保留custom服务键和账号原文并报告映射选择；不得通过显示标签、note或标题猜测服务。
- `people/life-event` event_assertion：`value`使用下述closed preset/custom union；required qualifier `eventTime`承载calendar_date或zoned_instant，可另带confidence/selection，provenance只在Entry外层；允许互相冲突assertions，不以一个birthday scalar或复制日期覆盖；
- `people/profession` temporal fact；
- `people/measurement` observation：measurement code、quantity、observedAt；
- `people/nationality-state`、`people/legal-sex-state`、`people/gender-identity-state`是三个独立fact Fields，归属`people/person`，各自使用non-empty exact TypedText作者值、many独立occurrences和fact qualifiers；它们分别表示作者记录的国籍／公民资格状态、法律或登记性别状态、自我认同状态，不能彼此推导或互相覆盖。每条可携带独立`qualifiers.validity`和外层provenance；同一Field不同或重叠期间的断言并存，省略validity只表示期间未知，不表示永久有效或当前唯一值。保留相互冲突的来源和原始文字，不设置全球国家／性别枚举，不依据姓名、外貌或其他字段推断这些状态；自由note不能替代这三个字段。D7查询必须按完整FieldId分域，再按显式期间／selection规则派生视图，不能last-write-wins回写；源状态本身不依赖D7实现。
- `people/engagement` directed relation，authoritative subject为Person，target任意ordinary NodeRef或text fallback（Organization Facet仅增强组织投影，见§13），value只含position/rank/department等domain payload，validity/status只在relation qualifiers；Organization roster为derived inverse，不双写；
- family方向拆分：`people/parent` directed、inverse `people/child`；`people/spouse`与`people/sibling`分别是symmetric；`people/family-related`是neutral symmetric custom family Field。这四个family Field、`people/guardian`、`people/social-related`、`people/professional-relation`、`people/manager`、`people/mentor`与`people/engagement`逐字使用`node_ref_or_text`；其他v1 relation Field均为NodeRef-only。不存在混合direction的`people/kinship`；
- `people/spouse`的作者事实基数为`minimum=0, maximum=many`：每段关系以独立occurrenceKey保存，可有各自validity、note与provenance。后续关系不得覆盖或删除先前关系；未填有效期不推断为当前关系，重叠或互相冲突的声明也不因全局单偶规则而拒绝。current/history为基于显式validity或独立裁决的投影，不按数组位置、最近写入或canonical owner选择。每条NodeRef事实仍只有一个canonical source owner；多条历史事实不改变一条事实的对称去重、权限与显式cleanup语义。
- `people/family-related`、`people/social-related`、`people/professional-relation`均为symmetric general；有方向的职业关系使用`people/manager`和`people/mentor`，均为directed ranked。colleague默认由overlapping engagement派生，不持久化O(n²) edges；只有明确作者断言才写入professional-relation的colleague preset，两种来源不自动双写。
- `people/avatar` owner-local ResourceRef。

同名Person合法，NodeRef区分；Search可从name fields派生terms但不能复制为另一份alias authority。People provider不可用时raw names保留；D7决定受权限约束的降级search，不得用stale index冒充current。

#### 9.2.0 人物关系的预设域、自定义标签与方向

三个一般关系Field的value均为closed TypedObject：required `target`沿用显式node/text union；required `relationship`为closed TypedUnion。`preset`分支是SemanticCode，family-related只允许`people/extended-relative`，social-related只允许`people/acquaintance|people/classmate|people/friend`，professional-relation只允许`people/colleague`，均须命中对应的已安装contribution；`people/birth`、其他类别代码及任意新安装代码不得扩大这些集合。`custom`分支为non-empty exact TypedText，作者在此保存关系称谓，大小写、Unicode、空白原样保留；标签恰为parent、mentor、manager或任一preset显示名也不自动转码。relationship不能省略，两个分支不混合；旧的顶层relationCode/customLabel均不是该新wire的成员。

category由Field选择决定；code是preset的稳定标识，custom文本是作者标签，target仍为独立的NodeRef/text；validity/status留在qualifiers（该closed relation QualifierSet不接受confidence），note/provenance留在Entry。该三个Field的NodeRef事实均明确声明对称存储、拒绝self-edge、canonical owner唯一、general graph；preset和custom均不产生语义inverse row、代际rank或Family Tree边。这里的对称仅表示作者选定的一般关联，不把自定义称谓解释为双向同义的角色，更不推断亲子、上下级或导师关系。literal target依旧没有任何graph/inverse，也不自动建人。

需要方向的事实必须显式选定对应Field：parent表示subject的父母为target，inverse为child；guardian表示subject的监护人为target，inverse为ward；manager表示subject的上级为target，inverse为direct-report；mentor表示subject的导师为target，inverse为mentee。后三者分别使用`people/guardian`、`people/manager`、`people/mentor`，均为Person→Person或literal fallback，沿用同一个target-value primitive和Field-owned relation contract。guardian使用general projection，不能由监护关系推断血缘或代际；manager/mentor使用ranked projection，仅表达各自明确的角色方向。spouse/sibling保留既有对称family语义。Field选择与受信schema是这些语义的唯一来源，custom label永远不是选择器。所有新Field均为many、独立occurrence、关系qualifiers和显式cleanup，NodeRef仍受相同权限、availability、target lifecycle及原子写入规则约束。

此处选择按固定方向分Field，并以三个小型类别词汇加custom承接自由称谓。相比在同一个Field内部由code切换方向，这避免在更改标签时改变canonical owner、基数计算和inverse权限；相比独立全局关系类型注册表，不增加另一套注册、身份或迁移权威。扩展可按既有Field贡献机制增加具有明确方向的领域关系；不得把每个自定义词自动注册。UI采用一个“人物关系”编辑区及family/social/professional分类，编辑器明确显示当前断言是中性关联还是具有方向的角色；分类不直接决定布局。

revision29尚未激活，本次仅重写研究语料，不是用户数据迁移。D9未来导入旧wire时必须先预览明确的Field和descriptor映射；缺失关系称谓不能静默发明，无法证明的代码保留raw并报告未解决项。把custom改成manager/mentor或反向操作属于显式语义变更，不能作为label重命名；尤其从旧directed professional-relation转为新的symmetric一般关联，必须由用户确认预览中的方向和唯一存储位置变化，不自动重解释既有事实。

#### 9.2.1 生命事件、重要日期与派生纪念日

`people/life-event-value`是closed TypedUnion。`preset`分支为TypedObject：required `eventCode`精确限定为`people/birth`、`people/death`、`people/employment-start`、`people/graduation`、`people/marriage`五项已注册contributions；optional `customLabel`为non-empty exact TypedText，optional `description`为exact TypedText。`custom`分支为TypedObject：required non-empty exact `customLabel`及optional exact `description`，不允许`eventCode`。无论其他code是否已注册，均不能跨入preset域；custom label恰为“birth”也仍是custom，不自动转码。预设名称可本地化，但code不变；作者label与description保持原文，Entry `note`仍为独立备注，不能替代eventTime或customLabel。两分支均以required `qualifiers.eventTime`表达事件时间，禁止在value中重复日期。

| 作者事件或用途 | 事实源与明确选择 | 派生边界 |
| --- | --- | --- |
| 出生、死亡 | 各自以birth、death的独立事件断言保存日期、来源和冲突历史 | 生日及逝世纪念日从选中的对应断言派生，不增加birthday/death-date标量权威 |
| 结婚、结婚纪念日 | 可保存独立marriage事件；仅当用户明确选择某条spouse关系的validity.start且确认它表示所需结婚日期时，才从该成员派生 | 登记、仪式或关系期间并非天然同一事件；关系无此日期时不能猜测，选择关系源不复制为life-event |
| 入职、入职纪念日 | 可保存独立employment-start事件；只有明确选择某条engagement的validity.start并确认其表示入职时才复用 | affiliation、参与或一般成员资格的开始不自动等于入职；事件本身也不创建组织关系或inverse |
| 毕业 | graduation事件保存每次毕业的独立断言，description可说明作者背景 | 同一Person可多次毕业，不以同code、同标签或同日合并 |
| 自定义重要日期 | custom事件保存逐字label、eventTime及来源；不依赖另一个预设code contribution | 同名或同日可为不同事件；纪念用途须显式选中来源，不由文字猜测语义 |

这些断言属于同一many Field，但不共享“最多一个preferred”约束。出生与死亡分别选择，preferred birth和preferred death可以同时存在；某类存在多个preferred或多个冲突来源时显示歧义并要求显式选择，不能按置信度最大、最后写入或数组位置静默胜出。毕业、结婚、入职及custom可包含多次独立事件，不能仅按code或label识别同一次事件。D7查询或用户显式选择承担投影选择，不新增全局事件identity；一条作者断言仍由owner NodeRef、完整FieldId和occurrenceKey定位。同一日期也不证明两条断言是同一事实。

普通编辑必须区分“修订所选断言”与“新增独立断言”：前者使用当前来源revision及完整selector，只修改用户选中的作者Entry；后者使用fresh occurrenceKey追加，原有冲突和来源保持。编辑derived重要日期必须返回当前选定的作者源；若选中的是关系validity.start，预览必须说明会改变关系期间，不能暗中创建第二个日期源。单纯改变显示或纪念规则不回写事实。选择关系源时不向relation qualifiers发明selection成员；该选择属于D7/D8调用上下文。来源不存在、歧义、权限或revision不可证明时停止派生/编辑，不能退回同名、最近项或stale cache。

日期保持calendarId、calendarVersion和precision；只有year或month时不能补造day，calendar_date不能默认变为本地午夜instant。当前calendar_date不支持yearless月日；D9导入该类材料必须明确报告不可无损表达并保留原始材料，不能捏造年份。纪念日的年度规则、闰日处理、非Gregorian换算、时区与观测horizon由D7/D10提供显式版本化输入；现有Gregorian recurrence profile不自动承诺所有历法或纪念规则。投影每次绑定一个当前作者来源及其精确temporal member和规则版本，只生成derived结果，不自动添加Calendar Event Facet、recurrence作者事实或Node。

关闭Calendar UI或投影runtime不删除这些portable作者事实；真正缺少Field/alias/code/calendar contribution时按现有availability规则保留raw并停止typed接受，不能把缺失解释为空日期。custom分支不要求为每个作者标签注册额外code；整个Field、alias及其声明的contribution依赖和实际使用的历法仍须可验证，不能绕过catalog装载前提。D4目录和公共输入验证证明两分支的作者表达；上述选择、普通编辑交互及版本化纪念日投影是明确的D6/D7/D8/D10后续验收义务，不声称现有Calendar recurrence模型已经实现这些投影。此更改仅修订未激活的bootstrap候选，不授权对已激活同ID schema或用户来源静默改写。

### 9.3 `organizations/organization`

新增组织关系的确切映射如下；FieldId独立保存edge kind，不由note、target分类或另一种关系推导：

| Field | 作者subject | target | 派生inverse |
| --- | --- | --- | --- |
| `organizations/business-guided-by` | 接受业务指导的组织 | 业务指导机关 | `organizations/business-guides` |
| `organizations/territorially-administered-by` | 受属地管理的机构 | 属地政府组织 | `organizations/territorially-administers` |
| `organizations/jointly-led-by` | 明确共同领导安排下的机构 | 一个参与领导的组织 | `organizations/jointly-leads` |
| `organizations/supervised-by` | 受监管组织 | 监管组织 | `organizations/supervises` |
| `organizations/subsidiary-of` | 子公司组织 | 母企业组织 | `organizations/has-subsidiary` |
| `organizations/brand-of` | 明确独立引用的品牌组织Node | 关联企业组织 | `organizations/has-brand` |
| `organizations/member-of` | 作为成员的组织 | 联盟或协会组织 | `organizations/has-member-organization` |

全部新增Field均为directed、NodeRef-only，两端必须满足`organizations/organization`，source/target cardinality均为0..many，使用现有relation qualifiers、author_order与key_unique_values_may_repeat。含义分别是业务指导、属地管理、共同领导、监管、子公司隶属、品牌关联和组织成员资格；不等同于既有generic governs、owns或symmetric allied-with。明确jointly-led-by不从同时存在其他边推导；同一机构可同时保留primary parent A、business-guided-by G、territorially-administered-by L及分别指向G/L的jointly-led-by事实。品牌只有明确独立Node才用brand-of；名称、商标文本本身不自动创建实体。Person任职仍只由people/engagement承载，member-of不代替人员名册。

新增Field由既有verified organizations namespace owner提供，不引入国家/职位/等级全局enum。每个Field的resolutionPolicy为live_required_on_create_retain_suspended、deletePolicy为retain_fact_explicit_cleanup、graphProjection为general；inverse仅派生。UI/runtime禁用与实际Schema贡献缺失分开，后者保留原文并停止不可证明的typed interpretation/action，不改成neutral related或解析note。复制/fork按7.5 directed mapped-owner统一规则处理全部新增Field；原FieldId/occurrenceKey及未变作者数据保持，允许的ref重写和完整后状态验证不能合并edge kind。


- `organizations/name` repeatable；
- `organizations/status` temporal fact；
- `organizations/parent` directed NodeRef relation，authoritative subject为child Organization，inverse `organizations/child`；structural parent/path永不改变它；
- `organizations/identifier` external_identifier repeatable；
- `organizations/classification` namespaced scheme/value repeatable；
- 关系按方向拆分：`organizations/governs` directed/inverse `organizations/governed-by`，`organizations/owns` directed/inverse `organizations/owned-by`，`organizations/allied-with` symmetric，`organizations/related` neutral symmetric；不存在混合direction的`organizations/relationship`；
- `organizations/site`、`organizations/address`、`organizations/contact` repeatable；
- Person membership/appointment唯一作者源是`people/engagement`，Organization-side member list只派生。

国家/行业枚举只能由Organizations-owned namespaced pack贡献，不进入global enum；中国公安/公共机构样本只作压力fixture。改名保持NodeRef；merge/split/successor必须显式Action和必要fresh NodeRef，identifier不等于identity。

### 9.4 Calendar

calendar/period-value的seriesKey是required的exact TypedText，空字符串是合法值，missing/null/非字符串均拒绝。Entry、Facet Create和series/scope unique/many规划必须消费同一作者值域；规划器不得另加truthiness/non-empty条件。规划与重试仍须绑定与raw Entry完全相同的seriesKey，不按标题、路径或locale补默认值；其他历法、周期规则和时区成员仍须匹配其已绑定contribution。

CalendarPeriod中以TypedText承载的calendarId/calendarVersion、periodRuleId及timeZone/tzdbVersion仍是有语义角色的依赖引用。它们必须在包含对象的完整结构接受前进行可用性检查：calendarId先证明namespace owner，再按calendarId、calendarVersion、periodKind、periodRuleId逐级匹配当前周期规则contribution；periodRuleId是该tuple内的local规则标识，不另造namespace身份。tzdbVersion和timeZone独立匹配时区contribution。诊断定位到第一个不能证明的实际标识符token（TypedText的text或periodKind的code），不能一律指向periodKey。calendar和timezone独立失败时两者都保留；只有匹配预期的已知object、TypedText或semantic_code构造器内才读取依赖；其额外member或其他独立结构错误不遮掉已识别依赖。缺失、未知或类型不匹配的kind在当前子树报告构造器错误并停止内部语义解释，不伪造已知wrapper收集其子成员。子树外的seriesKey、其他已知成员或note错误仍单独报告，按原始source起点排序；这些诊断均不能修复来源或使损坏值通过。periodKey解释必须依赖已证明的规则，其格式错误定位自身text token。

产品层可由单一 `Calendar / 日历` module呈现，但三个Facet分域：

- `calendar/period-note`以`required_field(fieldId="calendar/period",when="effective")`声明必填Field。`CalendarPeriod/1` exact members为`calendarId,calendarVersion,timeZone,tzdbVersion,periodKind,periodRuleId,periodKey`；它作为`calendar/period-value.period`，同object另有required `seriesKey`。`periodKind=day|week|month|quarter|year`，`periodRuleId,keyProfile`必须命中当前RegistryBinding的verified calendar period-rule contribution并满足§3的exact kind↔profile mapping，`timeZone,tzdbVersion`必须命中同一tzdb contribution。`periodKey`不是regex-only string：profile decoder必须得到唯一canonical semantic key；ISO date检查真实Gregorian date，ISO week用ISO week calendar验证year/week组合（包括只有确有week 53的year才接受`W53`），month/quarter/year验证真实bounds与非零四位year；ISO v1 lexical forms分别为`YYYY-MM-DD|YYYY-Www|YYYY-MM|YYYY-Qq|YYYY`。series identity逐字是`(calendarId,calendarVersion,timeZone,tzdbVersion,periodKind,periodRuleId,seriesKey)`，period identity再加periodKey；不用title/path/locale猜测。
- `calendar/range-note`以`required_field(fieldId="calendar/range",when="effective")`声明必填Field：date_range或instant_range。多篇政策不暗用Event；它由当前RegistryBinding中verified `CalendarSeriesScopePolicy/1` contribution显式提供给D6 semantic plan。
- `calendar/event`以`requires:["calendar/range-note"]`声明Facet依赖，使用同一`calendar/range`作者事实并施加相应required_field约束，再增加 `calendar/event-status`、`calendar/recurrence`、`calendar/participant`、`calendar/reminder-intent`；Assign Event不复制start/end。closed `union_variant_equal(calendar/range,calendar/recurrence,when=both_present)` constraint要求两者branch逐字相同。

RecurrenceRule/1的annex alias `calendar/recurrence-value`是closed `date|instant` union，采用下列profileVersion=1作者合同。expansion必须由caller给horizon和budget；derived occurrence/override默认无Node identity，只有D3明确promote/adopt才可创建fresh Node。Record的一般领域合同与替代比较归D5，本D4不冻结其身份或存储模型。

#### 作者源及版本

继续使用同一Event Node的独立`calendar/range`和单值`calendar/recurrence` Field。重复规则和所有单次例外均在该recurrence Entry中；例外没有独立Node、Record、D3 ref或Annotation target。外部ICS UID/RECURRENCE-ID、SEQUENCE仍归D3来源绑定与D9映射，不替代本地Node identity。

`calendar/recurrence-value`仍为`date|instant` closed union。每个object variant必需`profileVersion`（TypedInteger，exact value `"1"`）、`anchor`、`frequency`、`interval`；optional members为`weekStart,byMonth,byMonthDay,byWeekday,count,until,rDates,exDates,exceptions`。未列member拒绝；不存在从系统时钟、title、range另一端、locale、显示时区或expansion horizon补锚点的默认。公共Entry与操作接受路径必须在完整schema/context解码后调用同一源语义函数。

profile1采用Gregorian civil day/week/month/year规则。date arm的anchor和所有日期使用同一已验证的`calendar/iso8601` calendarVersion及day precision；其他Calendar/precision的作者值保留原始内容，但不能由这个profile猜测其重复含义。无可用规则为provider/schema unavailable，禁止隐式转公历。instant arm的anchor为确切zoned_instant，使用同一timeZone/tzdbVersion；rule不是每天加86400秒。

range与recurrence同时参与Event接受时，必须同branch、同完整temporal basis；range.start和endExclusive都必须有界、start<endExclusive，且start在同basis中精确等于anchor。无界/ongoing range单独仍合法；它不提供可重复的有界模板长度，不能和recurrence一起被接受。unbounded start且无anchor的来源因此拒绝。dates以calendar day数保留模板长度，instants以任意精度exact seconds保留模板长度；不经过float。

普通发生的endExclusive明确由`originalStart + templateDuration`计算：date是同calendarVersion的Gregorian day差，instant是模板两个确切UTC端点的elapsed seconds差。instant选择保留elapsed duration；保留civil结束钟点会在DST跳变处改变持续时长，本profile不采用这个隐式替代。例如civil 01:30开始、模板持续2小时，发生当日向前跳1小时后，结束显示为civil 04:30而不是03:30。开始仍按上面的civil重复规则解析，不能因此把每日开始改为固定加86400秒。replacement使用完整作者range，不再套用模板长度。需要输出但超出本profile可表示日期域的端点返回不可用，不能夹断范围或返回部分成功。

#### frequency、selector与默认含义

interval为1..65535，frequency只为daily/weekly/monthly/yearly。anchor永远作为base set首个候选，即使显式selector没有命中它，也不会被重新解释成其他日期；其后只考虑不早于anchor的候选，去重。

| frequency | interval的phase | 未提供selector时的默认 |
| --- | --- | --- |
| daily | candidate civil date与anchor的day差是interval整数倍 | 每个命中day |
| weekly | 从包含anchor的week bucket起算，bucket差为interval整数倍；weekStart默认monday | byWeekday默认anchor weekday |
| monthly | 与anchor的month ordinal差为interval整数倍 | byMonthDay和byWeekday均缺失时，month day默认anchor day |
| yearly | 与anchor的year差为interval整数倍 | byMonth、byMonthDay、byWeekday都缺失时，month默认anchor month；后两者均缺失时，day默认anchor day |

显式byMonth为1..12；byMonthDay为-31..-1或1..31，负数自月末倒数，0非法；byWeekday为七个closed weekday codes。不同维度取交集，同维度取并集。monthly若仅byWeekday则使用选定月份全部命中weekdays；yearly有byMonthDay或byWeekday而无byMonth时使用该年全部月份。weekStart仅决定weekly buckets，其他frequency中不改变结果。不存在按选中项数量重新计interval。

不存在的月日跳过，不向月末clamp；例如anchor Jan31的monthly默认不会生成Feb28。闰日yearly规则只在实际有该日期的年份命中。count计算真实生成的base候选，不计算被跳过的无效日期。

instant arm先以固定tzdb版本把anchor解析为civil date及wall time，包括完整小数秒。之后在命中的civil date保留相同wall time，以同一版本规则解析：DST gap跳过该候选，fold选择较早的UTC instant；显式anchor本身保持原确切instant，不因fold默认改写。RDATE及replacement range已经是明确instant，无需再做fold选择。规则缺失或离线无法获得所需版本不能换用OS当前timezone数据，也不能返回空集合代替失败。这些日期/时区解释属于语义依赖；下游负责提供对应版本的实际规则和有限展开实现。

#### count、until与集合优先级

count为1..2147483647，和until互斥。count从anchor起对按时间升序、去重后的base候选计数，发生在EXDATE和exception之前；删掉一项不会自动补下一项。until是inclusive的base起始时间上界，完整basis必须等于anchor且不能早于anchor。省略两者表示语义上可无限的base series，执行仍必须受有界horizon/work/output budget约束。

成员证明与投影必须共用这一base枚举含义，不能一处按civil遍历顺序计数、另一处按UTC排序。有限规则允许offset跳变，civil date的较后候选可能解析成较早instant；不得遇到第一个超until的civil候选就宣称其后均不命中。当前纯模型根据已验证的±86400秒offset界，为查询上界或第count项证明未来civil date的最早可能UTC下界，再结束枚举；所有候选按确切instant去重排序。预算或为此前瞻证明所需coverage不足时失败，不将未证明当不存在。

令B为上述base集合，R为显式RDATE集合，E为EXDATE集合。RDATE可以早于anchor或超出count/until，只要basis相同；它不消耗base count。先对`B ∪ R`以精确temporal起点去重，再处理每个原始起点：

1. 存在cancel exception：不投影该次发生。
2. 存在replace exception：使用replacement，优先于同起点EXDATE。
3. 无exception且属于E：不投影。
4. 否则继承series模板。

一个起点只能有一个exception，不能同时cancel和replace。EXDATE可以不命中当前集合，仍保留作者意图；exception的originalStart必须可证明属于`B ∪ R`，否则系列操作不接受。所有temporal集合除canonical排序/成员约束外，还须按精确时间意义查重；同一instant的不同offset写法不能绕过唯一性。

#### 单次取消与覆盖的完整wire

exceptions是optional bounded_set（1..256项；无例外时省略），每项为TypedObject，exact members为`originalStart,action`。originalStart使用所属date/instant arm，且basis与anchor逐字相同。action是closed TypedUnion：

- `cancel`：value为field-local semantic_code，exact code=`cancelled`。
- `replace`：value为TypedObject，required member=`range`，optional members=`title,eventStatus,note`。range直接使用该arm的date_range/instant_range，两个有界端点及ordering必须通过同basis验证；title为非空exact text，note为exact text且可空，eventStatus使用既有`cancelled|confirmed|tentative`域。

替换范围的start可以移到别日；originalStart保持原始值，不按新start、显示时区、ordinal、数组索引或horizon改写。不同原始起点可被显式移到相同新start，它们仍是两个不同的系列相对结果，不因显示重叠合并。缺省title/eventStatus/note继承当前series内容，显式空note表示清空该次投影说明；replacement不改写Node的D3 Document标题或其他发生。

单次覆盖v1的明确编辑面是时间范围、显示标题、事件状态和说明。它不创建另一套参与者关系或提醒作者权威；参与者/提醒继续属于series源。D9遇到本profile未表达的per-occurrence participants/reminders或其他ICS字段，必须保留来源并在preview标明精确loss/unsupported项，不能丢弃后宣称无损round-trip。新增覆盖维度须完整schema修订，不得塞进未知members。该边界不免除当前必须表达的改期、取消、例外来源保留及可重建性。

完整ValueTypeSpec使用现有18个constructors，不增加新TypedValue kind。Field alias→root union→rule object→exceptions set→exception object→action union→replacement object→range/text/code的最长schema路径为8；Entry整体仍受D2物理行与D4 UTF-8限制，不能通过256项上限规避总字节限制。

#### revision-bound selector、编辑与显式rebase

一个操作selector exact为`{ownerNodeRef,fieldId:"calendar/recurrence",occurrenceKey,originalStart}`。owner和occurrenceKey绑定series原始Entry；originalStart是该Entry内部的语义地址，不是可跨版本盲用的D3 durable locator。编辑请求必须绑定expectedOwnerRevision、expectedRegistryBinding、确切before recurrence/range raw sources，且在提交前验证同一读取视图与source CAS。仅携带一个旧derived-cache row或ordinal不得授权编辑。

`RecurrenceEditRequest/1`的D4语义请求exact为`{kind:"d4_recurrence_edit_request",operationId,editKind,ownerNodeRef,expectedOwnerRevision,expectedRegistryBinding,expectedRecurrenceReadBinding,beforeRecurrenceSource,beforeRangeSource,originalStart,afterRecurrenceSource,afterRangeSource,rebaseDecisions}`。editKind只允许`set_exception|remove_exception|edit_series`；前两者使用originalStart，edit_series时为null。range source不是单独recurrence值的副本，两个raw source各自必须绑定真实Field/occurrenceKey。该wire不包含自报授权成功或自报成员存在的Boolean。

set exception必须证明原始起点属于当前`B ∪ R`；remove exception必须精确命中现有作者例外，允许显式删除因外部编辑而孤立的旧项，不强求被删除项仍属于当前base。两者只能改变该selector对应例外，其他规则成员、例外及range源不变。edit_series对每个已有exception必须显式给一个rebase decision，恰好完整覆盖旧selector集合：`{originalStart,disposition,nextOriginalStart}`，disposition为`keep|remap|drop`。keep要求nextOriginalStart语义相同且属于新集合；remap要求显式新selector、证明新集合成员并保持或显式编辑相应payload；drop的nextOriginalStart为null，且新源中没有对应旧例外。多个decision不能映射到同一新selector。新加入的例外也独立验证，不能伪装成旧例外的自动重绑定。旧source的结构、来源与版本必须可证明；旧项是否仍属于base不是显式drop/remap的前置条件。完整after-state中的所有例外都必须证明新集合成员资格，不能用repair名义继续保留孤立项。

缺decision、孤儿selector、复用旧版本cache、rule/tzdb/basis变化后未明确rebase、重复目标或任一source revision失配均零写失败；保留全部before raw source与投影。规则修改不能静默丢例外、按当前顺序重绑或将相同显示时间自动视为同一发生。只有完整D4语义、D6认证/权限/原子CAS都通过才能提交；成功只增加实际series owner sourceRevision。

#### 系列编辑的完整纯语义接受器

公共`recurrence_edit_result(pre,request,context,recurrence_reads,authorized_owners,inject_failure_at=None)`消费ValidatedCatalogContext、D6可信source/authorization/temporal reads。它不把请求字段当作授权，也不实现D6物理事务或receipt。前置状态exact为`{nodeRef,coreKind,declaredFacetIds,entries,ownerRevision,registryBinding,lifecycle,recurrenceProjection}`；lifecycle必须live。entries逐项沿用Facet operation的完整source reference；这些引用来自绑定当前D2 source revision的真实投影，fieldId namespace必须对应实际D2 carrier，不能从请求或过期cache补齐。recurrenceProjection只是可为null的派生cache观察值，不参与作者校验或权限判定。

接受前依次证明：owner/非Boolean D3 revision及RegistryBinding一致；D6授权inventory确实包含实际owner；两个before raw strings逐字命中真实Field/OccurrenceKey；四个before/after Entry在actual owner下经过公共availability-first decoder；field/key、外层note/qualifiers/provenance保持；effective Facet closure可证明且包含Event；after模板/anchor、来源例外和完整rebase合法。`set_exception|remove_exception`要求rebaseDecisions为空，range raw必须逐字不变，除选中例外外的rule成员及其他例外不得改动。`edit_series`要求originalStart为null，并按上一节逐项完成旧例外去向。

完整post-state校验覆盖calendar和effective Facet closure依赖的全部namespace。对这个不修改membership或其他namespace的操作，已证明不在依赖集合中的namespace只保留source bytes；无关provider缺失不能逼迫清除作者内容。unknown effective Facet使依赖闭包不可证明，仍须拒绝。不能据此跳过实际calendar源、被引用Field或跨Field约束。成员/rebase与post-state复核共用一次总work budget，后阶段只可消费剩余额度，不能重新获得完整预算。

系列编辑接受前（包括无改动的成功结果），依赖范围内每个Entry reference都必须经公共Entry decoder验证，expanded FieldId与occurrenceKey逐项等于wrapper声明，且同owner内完整`(fieldId,occurrenceKey)` selector唯一；不同Field复用同一key仍合法。该绑定比较与Facet操作及投影的适用引用检查共用实现，不能只核验所选recurrence/range或丢弃其余wrapper后仅校验raw streams。内部矛盾或重复selector固定`operation_precondition_failed`，完整保留原revision、source和cache，空write owners与invalidations；不得静默修正wrapper。此规则不扩大纯投影入口的依赖范围，也不要求解析已证明无关且不可用的namespace。

outcome exact为`{kind:"d4_recurrence_edit_outcome",operationId,status,postState,writeSetOwners,readSet,projectionInvalidations,rollbackByteEqual,intermediateStateObservable:false}`。readSet成功时exact为`{nodeRevisions:[{nodeRef,sourceRevision}],recurrence:<RecurrenceReadBinding/1>}`，绑定修改前实际owner和实际时区读取。若两个after raw都与before逐字相同，可返回accept但不写source、不增revision、不清cache；它不替代D6跨请求receipt幂等。实际source变化只增加series owner revision一次，D3Integer上限时零写失败，其他raw references/order/member均保持。

成功修改使旧recurrenceProjection失效为null，并给出`{ownerNodeRef,fieldId,occurrenceKey,sourceRevision}`列表：recurrence总是列出；range source有改变时同时列出range，覆盖只改模板长度也影响重复投影的情形。所有invalidations按D3-CJ/3排序，sourceRevision为before版本。D6在一个原子提交中执行source修改与投影失效，D7按新绑定重建；不能让新source配旧cache可见。任一类型/域/权限/CAS/读取/预算/rebase错误或测试注入的提交前失败，都返回包含旧source、revision和cache的完整pre-state副本，空write owners、null readSet、空invalidations。测试注入参数仅属于conformance harness，不进入作者或产品请求wire。

#### horizon、cache、权限和证据边界

horizon是有界查询范围，永不参与rule phase、count起点或selector身份。投影按最终replacement/template range与horizon相交过滤；必须另外考虑从窗口外移入的至多256个exceptions，不能仅展开窗口内originalStarts。rule成员存在证明不准把“当前窗口没找到”当成全series不存在；工作budget不足时返回明确不完整/不可用结果，禁止将partial结果报成完整接受。

derived cache key包含owner、recurrence和range各自的Field/occurrenceKey、source revision、RegistryBinding、固定calendar/tzdb规则及horizon。删除cache后，同样作者源和依赖必须恢复相同originalStart与覆盖结果。切换显示时区只改变显示，不改selector。读取/编辑权限由series owner与实际D6规则决定；无授权不返回目标存在与否，cache不是额外授权。

公共纯投影入口`recurrence_projection_result(pre,request,context,recurrence_reads,authorized_owners)`消费与系列编辑相同shape的D6 pre-state。只依赖实际Event membership、唯一recurrence/range两个作者Entry；其余series展示数据仍由D7通过同owner版本的读取继承，不能将本接受结果当成所有无关Field、完整Event后状态或显示面的验收。expected owner/source revision、Registry和实际temporal read binding均必须相等；授权检查早于source解析、schema可用性和lifecycle信息。请求不能提供替代source、cache授权或临时时区规则。

请求exact为`{kind:"d4_recurrence_projection_request",ownerNodeRef,expectedOwnerRevision,expectedRegistryBinding,expectedRecurrenceReadBinding,recurrenceOccurrenceKey,rangeOccurrenceKey,horizon,outputLimit}`。两个OccurrenceKey分别匹配实际Field的ref和解码Entry；不同Field可复用同key，不能把Field范围扩大成namespace唯一。horizon为同完整basis的bounded date_range/instant_range，start<endExclusive；outputLimit为1..2^63-1的D3Integer，Boolean拒绝。workBudget来自可信读取context，一份budget覆盖所有exception成员证明、base枚举和结果行处理。资源限制不改变成功结果含义。

先证明所有exceptions属于B∪R，再枚举有界base前缀，并加入全部RDATE及已证明的窗口外exception selector；按cancel/replace/EXDATE/template优先级处理，最后按`finalRange.start < horizon.endExclusive && finalRange.endExclusive > horizon.start`过滤。普通发生起点可早于horizon.start；两个最终range相同但originalStart不同的发生都保留。结果按originalStart的确切时间升序，不因改期重新分配selector。

outcome exact为`{kind:"d4_recurrence_projection_outcome",status,complete,rows,projectionIdentity,readSet,writeSetOwners}`。接受时status=`accept`、complete=true、writeSetOwners=[]；rows每项exact `{originalStart,range,overrides}`，overrides只含replacement中实际提供的title/note/eventStatus，普通发生为{}。所有输出date使用canonical day lexeme，instant使用保留完整小数精度的UTC Z形式及原timeZone/tzdbVersion；这是派生表示，不重写作者Entry。投影instant的小数秒去除末尾零，零小数省略小数部分；精度保证是exact数值相等，不承诺复用作者的小数词法，作者Entry继续逐字保留。

projectionIdentity exact为`{ownerNodeRef,sourceRevision,sourceKeys,registryBinding,recurrenceReadBinding,horizon}`；sourceKeys为按FieldId排序的两个`{fieldId,occurrenceKey}`，horizon使用同样canonical派生表示。readSet exact为`{nodeRevisions:[{nodeRef,sourceRevision}],recurrence}`，recurrence为实际读取binding。D6负责sourceRevision实际对应源快照的真实性，不能由caller/cache自报替代。没有新的pack/row hash、occurrence Node、作者Entry或额外写集。发生任一来源、绑定、域、成员证明、预算或coverage失败时complete=false且rows/projectionIdentity/readSet全部null、writeSetOwners=[]；outputLimit耗尽固定operation_precondition_failed，禁止截取前N项再标为complete。


#### Recurrence读取上下文与公共接受路径

`RecurrenceReadContext/1`是D6可信读取输入，exact为`{kind:"d4_recurrence_read_context",registryBinding,workBudget,timezoneRuleSets}`。workBudget是1..2^63-1的D3Integer，Boolean拒绝；它限制本次成员证明工作，不参与作者含义。timezoneRuleSets按timeZone/tzdbVersion唯一，每项exact `{timeZone,tzdbVersion,revisionToken,segments}`；version/zone必须命中同一RegistryBinding的tzdb contribution，revisionToken为非空Unicode scalar string。D6负责真实版本数据认证，不能让普通请求以自报segment数据获得权威。date-only规则可使用空timezoneRuleSets，但仍须显式读取上下文和budget。

segments是按UTC升序、相接且无重叠的非空有限数组；每项exact `{start,endExclusive,offsetSeconds}`，两端为带Z的确切UTC instant，start<endExclusive；offsetSeconds为-86400..86400 canonical integer string。闭包验证拒绝缺段、重叠、未知字段和非canonical值。仅当请求civil time对应全部可能UTC范围都落在已知coverage内，空解才可解释为gap；coverage不足是`provider_or_schema_unavailable`。fold取已验证全部解中的较早instant，不能依赖segment枚举顺序。fraction保持任意精度，禁止调用OS当前timezone或浮点timestamp补数据。

`RecurrenceReadBinding/1` exact为`{registryBinding,timezoneRevisions}`；timezoneRevisions逐项exact `{timeZone,tzdbVersion,revisionToken}`，按D3-CJ/3 bytes排序、唯一并完整覆盖读取规则集合。Facet请求的`expectedRecurrenceReadBinding`必须与实际binding逐项相同。active Event含recurrence时，公共case与Facet操作都必须先证明模板/anchor一致、规则可用和所有例外属于B∪R，然后才可接受；budget耗尽固定`operation_precondition_failed`，缺规则/coverage固定不可用。非active Event recurrence操作使用null recurrence binding/context，不能隐含开启另一条接受路径。

成功Facet outcome的`recurrenceReadSet`由实际读取产生，供D6与source revisions、关系readSet一起进行提交校验；没有active recurrence时为null。失败两个read sets均为null，返回byte-exact pre-state和空write owners。绑定Registry版本但不绑定实际使用的timezone revisionToken不足以提交。系列编辑仍须同时执行上面的before raw source绑定、完整显式rebase与D6权限/CAS，不能以一个Entry成功解码代替完整操作接受。

`CalendarSeriesScopePolicy/1` exact为`{kind:"calendar_series_scope_policy",policyId,policyVersion,keyMembers,scopeKinds,nodeScopeMember,multiplicities,conflictCode,retryRevisionRequired}`；其canonical bytes的SHA-256是policy schema digest。RegistrySnapshot必须以`calendarSeriesScopePolicyContributions`保留exact `{kind:"calendar_series_scope_policy_contribution",policyId,policyVersion,policySchemaDigest}`，generated catalog必须保留同一policy definition，D6 semantic plan再以`RegistryBinding/1 + policyId + policyVersion + policySchemaDigest`三方相等解析，不得只凭对象成员名宣称可用。`key` exact为`{series,periodKey,scope}`：`series`逐字复用上文七成员series identity，且其calendar/version/periodKind/periodRuleId/timeZone/tzdbVersion都必须在同一绑定snapshot解析；`scope`只允许closed union `{kind:"workspace",workspaceId}`或`{kind:"node",scopeNodeRef}`；禁止title、path、folder、view index或ambient locale进入key。`multiplicity`只允许`unique|many`。`periodKey`必须由同snapshot的period rule验证为canonical，并与拟写入`calendar/period`原始Entry中的periodKey逐字一致；series七成员也必须从该已通过公共Entry校验的作者值完整还原，不能由caller另报与作者源不一致的series。已有/并发候选每项绑定NodeRef、完整key与其作者period Entry，先验证再按完整key比较；同series的不同period必须共存，同period不同series也不冲突。该policy不是Node Field或第二作者事实。D6 create plan必须携带完整typed key、policy binding、plan observed revision、current revision、已有与并发候选集合：revision不相等固定`operation_precondition_failed`且零写；`unique`在同key已有或并发出现第二个不同NodeRef时返回exact `calendar_series_scope_conflict`且零写，`many`允许不同NodeRef；retry必须逐字复用同一绑定key并使用current revision，任何key替换或stale revision都固定`operation_precondition_failed`，不能以path/index去重。D6只实现authz/CAS/linearization，不得改变这项D4 semantic result。

`DerivedDuration/1`只由先通过同一`calendar/range` RegistryBinding、provider与ordering gate的value计算，不是authorable Field、qualifier或独立authority。任何bounded endpoint的calendar/version/comparator或timeZone/tzdb不可解析时，必须原样返回`provider_or_schema_unavailable`，不得继续产生duration。两个有界`date_range`逐字复用绑定的verified versioned comparator：ISO day使用Gregorian ordinal差，ISO month使用month ordinal差，ISO year使用year ordinal差，非lexical calendar使用contribution的ordered-table index差，得到exact `{kind:"calendar_units",calendarId,calendarVersion,precision,units}`；`units`是正canonical D3Integer。两个有界`instant_range`先分别验证tzdb contribution，再由decoded exact UTC instants相减得到normalized arbitrary-precision `{kind:"exact_seconds",seconds}`，不得经过binary float或host timestamp truncation。任一open bound只有在其余bounded endpoint也通过同一range/provider gate后才返回exact `{kind:"unavailable",reason:"open_range"}`。任何请求把duration作为Entry成员、Field或写回值都固定`constraint_conflict`；projection/export只能消费该派生结果。

Calendar System与Calendar Holiday Schedule data pack是Workspace/View context contribution，不是每Node Facet或Field。派生输出必须带packId/version/source/applicability；多个pack并存，不写单一`isHoliday`。父module/package/capability lifecycle由D10，D4只冻结typed output namespace与不回写作者facts。

ICS `UID`/`RECURRENCE-ID`/`SEQUENCE`是D3 SourceBinding/Provenance/version材料，不是NodeRef。subscribe/sync、initial_import、adopt、managed_copy/promotion保持D3 intents。VFREEBUSY/VTIMEZONE零Node；VTODO→Task和VJOURNAL mapping只经显式preview。

### 9.5 `library/work`

- `library/work-kind` semantic code（article/book/report/standard/dataset等）；
- `library/creator` directed relation to Person/Organization；My Works由当前Person authorship派生，不是另一个Facet/collection；
- `library/identifier` external_identifier（DOI/ISBN/arXiv/provider key不等于NodeRef）；
- `library/publication-state` temporal fact（draft/accepted/published）；
- `library/venue` NodeRef relation；其subject必须具有`library/work`，target必须具有`library/work`或`organizations/organization`之一，保持单一有向作者事实。文章A的venue可以是期刊Work J；出版机构Organization O不能因出版J而自动替代J。Organization仅在作者确实指认该组织为发布/呈现场所或主办机构时作为venue，不从publisher/creator或名称猜测。venue不声明出版、作者或版本关系，也不自动派生这些事实；publisher不是本Field的同义词。Work/container使用普通Node与Work Facet，不新增Venue identity。
- `library/version-of` directed relation；若edition/release取得独立可引用身份，显式fresh NodeRef；
- `library/resource`只允许owner-local ResourceRef；
- Citation仍是D2 Document occurrence，target同Workspace Node；Relation、Citation、Reference、Node Link不得混同。

`Library / 文献库`与`Work`是D4 schema术语；用户可见module/package命名和delivery由D10/A2复核，不由存在这些Field自动承诺产品面。

## 10. mandatory scenario dispositions

完整逐项矩阵见同包 `D4 Mandatory Scenario Disposition Matrix v1.md`。本候选的总裁决：

矩阵的125行、57条A2和302个完整命题逐项进入`D4 Scenario Evidence Specification v2.json`并逐字进入corpus `scenarioProofBindings`。`proofId/proofKind`是既有artifact字段名，其中proofKind固定`scenario-contract`，不表示自动语义证明。每个clause固定绑定命题完整原文、`evidenceKind`、明确的`testScope`和命名case引用；`evidenceKind`区分`executable-model`、`contract-check`、`integrity-check`与`contract-review`。无论类别，`independentSemanticReviewRequired=true`，每条完整架构义务均须由独立评审核对。

本地validator验证完整行/列与命题覆盖、来源完整性、引用解析及指定模型案例的实际结果；完整原文直接比较只证明绑定，不计算行/命题或包装哈希。它不声称可判定任意自然语言改写、语义相关性、架构充分性或尚未实现的宿主行为。没有对应行为实现的下游/取舍/UX契约不得借相邻样本标成已执行；保留原始完整义务并交由完整独立评审，未来阶段仍必须补产品验收。builder只消费逐条人工审阅的静态spec，不从关键词自动推断语义。新候选的单点变更须更新本地输入绑定并重新取得与delta相称的独立结论，不能通过重写证据绑定自行转换fail为pass。

- People/Organizations/Calendar/Library/Task均由普通Node + Facet + independent fields表达；无新identity/kind/私有DB；
- extension taxonomy、package/permission/runtime、D6事务、D7 Query/View、D8 UI、D9 import/template、D10 registry mechanics分别defer-with-owner，但D4 semantic inputs已冻结；
- D9 Office template与D7/D8 chart sections只接收D4 type/null/ref/interval边界，不越权冻结其syntax/layout；
- D1 compatibility=`compatible`：这些是已冻结可选能力边界的实例，不新增surface/mode/release；
- D2/D3 compatibility=`compatible`：只解释D2 opaque payload，不改变outer grammar/exact source，且inline note/value-internal key不进入D3 ref/locator/Annotation target；
- formal upstream reopen required=`no`。本候选已在§4.3选择revision-scoped非身份selector，不把这一选择延后给Gate。独立Gate仍须核验实际合同是否遵守该边界；若证实当前操作授予跨revision durable field-target能力，必须先纠正违反项，或在真正选择该能力时提交impact-scoped D2/D3 reopen，不得在冲突未闭合时冻结或静默改名继续。

## 11. 实现与测试边界

本D4只冻结合同；本包的纯语义conformance模型不等于产品代码实现。未来实现至少需要：

1. independent strict D4 Entry/1 parser与closed JSON decoder；
2. namespace owner/anti-spoof/registry snapshot interface；
3. typed value/schema validator与per-namespace availability；
4. entry-level source patcher，保留unaffected bytes/line endings/order/comments；
5. Facet composition/Assign/Remove/Cleanup pure planner；
6. relation canonical-owner/inverse/lifecycle projection；
7. fixtures覆盖duplicate values/keys、多block、unknown provider、external edit、copy freshening、Task/Template、People/Organizations/Calendar/Library；
8. normal/optimized validation、five-surface semantic parity、controlled terminology gate；
9. D6/D7/D8/D9/D10 owner stages以后分别补transaction/query/UI/import/package mechanics；
10. A2前不得改public Supported声明、`repos/weftext`或brand。

## 12. 冻结门

D4只能在以下全部成立后从candidate提升：

- immutable package包含本候选、术语增补、scenario matrix、D1–D3/Impacts/mandatory input完整bytes；
-按显式选择的`gpt6-pro` profile，由唯一writer完成完整候选和必要检查，直接进入独立Chat GPT-6 Pro评审；DeepSeek不作为本D4的必过门槛，未完成的历史评审不计为接受；
-全部finding逐项adjudicated并落实成立的完整修正；有界修正须由独立Pro显式复核关闭，accepted foundational scope/domain/identity/authorization/transaction/query/action/public-wire change须fresh完整Pro review；
-真正全新的独立Chat conversation中核验`GPT-6 Pro`及选中Pro的实际UI文字；选项序号不充当独立reasoning effort。完成响应须明确accept/pass、P0=0/P1=0、terminology pass且无fallback；
-candidate/package/response完整本地副本与逐字比对、修订号及文件清单、session IDs/title/URL/model/tier/elapsed/fallback、invalid attempts、P0/P1/verdict证据完整；不要求新包装哈希或字节framing；
-D4 decision、D4 Lexicon Addendum、D4 Implementation Impact、control indexes、links、IDs、protected-path guard全部机械通过；
-本D4收尾未修改D1–D3、D3 Lexicon、`repos/weftext`、brand或公开产品文档；D5架构工作仅在D4正式验收后按其独立范围开始，不混入本D4接受结论。


## 13. People × Organizations intake revisions

本D4设计接受mandatory intake §13.1.3：“People 可以引用任意普通 NodeRef，不强制目标已安装或声明 Organization schema”。`people/engagement`的subjectPredicate为Person Facet条件，targetPredicate为ordinary_node。任职目标可以是没有Facet的普通Node，也可以具有Organization或其他Facet；保留真实NodeRef、Person侧唯一作者事实及可判定的通用inverse。该选择不授权自动Assign、创建目标、改写已有来源或双写membership。

基本任职引用与Organization增强投影分开：通用inverse从Person侧实际NodeRef事实派生，无Organization Facet也成立；Organization专属名册、组织图和其他领域视图可要求Organization条件，这只决定增强视图的适用性，不反向否定普通Node任职事实。literal fallback仍只保留文本，不等价于NodeRef，也不产生可靠inverse。关闭Organizations UI或目标没有Organization schema不阻断普通Node任职；实际Person/Field定义、必要读取或引用状态不可证明时，仍按共享availability及生命周期规则保留raw并阻断相关操作。

People九个非任职关系Field均接受精确NodeRef/text target union：`people/parent`、`people/spouse`、`people/sibling`、`people/guardian`、`people/family-related`、`people/social-related`、`people/professional-relation`、`people/manager`、`people/mentor`。三个通用类别Field的完整value仍包含§9.2.0规定的relationship descriptor；target arm不能替代完整value。text target分支逐字保留作者所知姓名，不自动建立Person Node；不执行target resolution、inverse、graph、target cardinality或target lifecycle。NodeRef分支仍按Field固定方向与目标域处理；family-related、social-related、professional-relation均为symmetric general，parent与guardian为明确directed，manager与mentor为directed ranked。text与NodeRef之间的显式替换必须使用§7.4完整原子更新，不修改occurrenceKey和其余作者成员。

职业与任职不合并：`people/profession`保留一般职业text及optional organization context，后者不建立任职relation/inverse；任职的position/department是text，rank是制度限定的作者值。`EngagementRankValue/1`使用既有TypedObject，唯一required members为`system`与`level`，两者均是nonEmpty exact TypedText；不得额外加入code、国家默认值、推断排名或把system放入note。外层`rank`仍optional，缺省表示未提供职级，不从position/department/organization补值。示例：`{"kind":"object","members":{"system":{"kind":"text","text":"A组织技术职级制度 2024"},"level":{"kind":"text","text":"资深四级"}}}`。两个组织或制度的同名level可以独立保存；原文的大小写、空格、语言和版本描述均不自动翻译或归一。该system是作者明确给出的制度上下文，不是全球制度ID；相同文字不证明两个制度相同，level不具备隐式数字序或跨制度换算。

Rank替代已比较：任意namespace code不能证明职级含义，拒绝；一份全球rank枚举不能覆盖跨制度差异，拒绝；独立Rank/RankSystem Node或新增全局Registry表并非保留任职事实所需，拒绝本轮引入。当前选择完整制度限定的自定义作者值，保留制度和级别两个正交来源，不伪装为已提供机器可解释的Rank preset。未来Organizations/schema-pack若贡献预设或可比较的Rank code，必须提供封闭码域、制度归属、原文/显示标签分离和显式迁移；不能把已本地化标签或任意semanticCode自动填入本对象，也不能同ID静默扩形。D9导入若原始记录只有代码而无法确定制度及级别含义，保留原始来源并明确mapping/loss，不猜测补齐。employment type、独立office/department实体与完整HR模型仍待后续各owner评审；这不推迟当前custom Rank作者值。canonical mapping与rejected aliases见D4 Lexicon增补§5。


### 未激活候选与既有ledger的边界

本候选是未激活的D4 bootstrap设计，不是对已激活Workspace ledger的迁移。当前完整目录及其Field/Facet定义绑定review Registry generation33。真实已激活ledger若采用同类语义变更，仍必须遵循§3的fresh identity／显式迁移规则，不能把未激活候选的重新绑定当作许可。

## 15. D3 v11 符号源组合的前验桥与真实效果

D3 v11十数组与Result/9 C分段是尚未分配Node/Resource/Annotation参与D4 initial entries、完整Template、SCC及copy内部typed refs的唯一前验桥。D4 Entry/1保持原closed decoder；FacetOperationRequest/2与RelationReadContext/2、RelationReadBinding/2采用§7.4明确的新decoder及状态依赖；其他实际值wire保持各自版本；不得把new_result_subject直接塞进它们的NodeRef成员。D6先按D3受绑定符号输入和stage12私有candidate map生成实际完整源与拟议revision，再从真实source提取D4实际输入、执行完整Node分类/Facet/requiredness/relation/recurrence和范围验证，全部在planning reservation以前。成功planned保存的就是该唯一已验证源与map。

对称关系source assembly复用§7的唯一事实/canonical owner规则。copy迁移只能到其fresh closure；禁止目标在本次mapping之外既有endpoint时仍为whole reject。create compound可在明确existingPayloadEdits中绑定旧endpoint的preimage/revision及完整result，但须当前权限且实际可能修改范围提前授权。D3为符号fresh组合选择保守授权全部潜在existing owner；这不修改普通D4关系编辑按actual owner的规则。不能因UUID导致owner不合法而重抽ID、改变Facet或删provenance。

`D4SourceMaterializationEffects/1`是D4自有的closed receipt extension，恰为`{kind:"d4_source_materialization_effects",wireVersion:1,operationId,registryBindings,owners,entries}`。registryBindings为按WorkspaceRef排序的`{workspaceRef,registryBinding}`完整实际binding，复用现有D4 decoder；owners为按完整NodeRef排序且无重复的`{nodeRef,before,after}`，before闭集`{kind:"absent"}`或`{kind:"source",sourceRevision,sourceBytes}`，after恰为`{kind:"source",sourceRevision,sourceBytes}`。sourceBytes为canonical无padding base64url exact UTF-8 bytes，不另造作者源；before absent只允许同D3 receipt fresh Node，actual no-op revision不变。owners恰覆盖使用C的全部result owners和全部实际被改写的existing source containers，包含因迁出而空的结果owner。

Document-wide源物化先按D3同包C规则在完整owner stream上保留/原位替换/迁出，再把迁入项追加到目标Field尾部；Field缺席时追加owner stream尾部并使用已绑定的合格carrier。必须跨全部同namespace blocks定位，不得用首/末namespace block代替Field位置。输入潜在carrier不是第二事实，最终空carrier整体消失；comments/EOL/unknown raw和非引用bytes逐项与完整before/after对照。source effects的owner及Entry顺序必须由这个真实source变换独立恢复，不能用effects排序冒充作者顺序。

entries按fact source subject键（无fact的literal用container subject键）、FieldId、OccurrenceKey排序且无重复。每项恰为`{sourceSubject,resultOwnerNodeRef,fieldId,occurrenceKey,beforeRawEntrySource,afterRawEntrySource,referenceChanges}`；beforeRawEntrySource为raw string或null，null仅explicit fresh initial Entry；其余必须与同cut真实source一致。afterRawEntrySource是唯一被emit结果raw Entry。referenceChanges复用§7.4完整typed根pointer/before/after union，新增初始fresh Entry的before只可null；每个受模板影响的根恰一次，exact保持根不伪造change。所有非引用bytes/迁移/源顺序由原request独立重建；不能从待验证effect本身生成期望source。samefact只一条entries，即使骨架含多个emit候选。sourceSubject是D3 plan地址，不形成Entry/Field EntityRef。

该扩展与D3 request、candidate map、receipt及D6 commitSequence在同decision原子保存；既有copy仍同时满足D4RelationCopyEffects/1，其facts投影必须与本扩展一致。D3 preserved/identityMap/resultAllocations/placements/D2 authored reference证据不被本扩展替代。D6 effectsToken只负责当前获权运输，完整raw source读权不足时不能向caller交付owners中的源值；保存完整内部证据不授用户读取权。

本节不改变Field值constructor、Registry、Task/Person/Calendar域、Relation/Locator含义或D2 carrier grammar。新wire名归D4 Field Value Occurrence的语义效果技术投影，sourceSubject归D3 Payload Subject内部计划地址；不新增公共实体概念。D3/D4 Lexicon中的既有ID、locale和用户术语继续拥有原名。

## 16. D6潜在观察范围与Calendar控制消费

D4完整依赖读集证明不等于调用者可观察constraint结果。D6接口§13在值/范围求值前证明保守ObservationScope，包括空范围；未获权不得由unique、基数、incoming或跨Field真假选出不同成功/拒绝。固定scope与同Registry定义/真实依赖在planning、commit、replay和delivery重验；不新增D4公开permission token或信任客户端完整性声明。局部Field路径只有完整约束代数的静态依赖上界和实际读取追踪均可证明独立时才适用，否则使用全Workspace观察。

calendar/period作者value仍只有period与seriesKey，不增加scope Field。完整key中的scope来自D6同cut CalendarPeriodScopeBinding；其建立、保留、显式迁移/删除、control inbound与版本CAS依D6接口§8.1。series/periodKey必须仍由真实Entry解析并按本稿policy验证。新period固定workspace scope且要求显式已有配置；managed copy按D3 map重绑scope；full fork同时重绑完整source配置和binding。首次create/fork的policy/Registry/Calendar配置按D6 §14的受权固定bootstrap plan同activation发布；其完整prepared结果必须逐key执行本稿unique/many语义，不能凭target空表省略校验。Registry trust root、workspace_user owner与source/target binding分离规则保持。

managed copy内部node scope若映射为本次fresh Node，其所需配置按D6接口§8.1从真实source配置确定派生、保留multiplicity并同copy提交；只建立该fresh scope配置，不改既有配置，不需先激活scope Node或第二次管理提交。完整prepared范围、target Registry和负inventory仍全部验证。

## D7联合修订的消费边界

本次仅同步D3 wire11/Result9消费：原D4 C carrier语义、RelationContext/Binding2、Entry/Facet/Registry/Recurrence/DerivedDuration均保持原形；D7 Q只处理D2保存定义payload，不能占C carrier或改变D4作者typed值。D4扩展仍由原同decision完整保存；新D7EffectBytes运输只是读出这些完整原格式，不是新D4效果。历史v10/Result8 bounded evidence保持原证据归属，不计作v11/Q实现测试。
