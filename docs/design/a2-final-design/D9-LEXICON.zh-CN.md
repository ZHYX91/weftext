---
source_language: zh-CN
translation_status: source
---

[English](D9-LEXICON.md)

# A2 D9 术语与命名词表

状态：**author-resolved-pending-independent-review**。D3–D8 owner 不变；D9 只命名 conversion-control concept。

| Concept | Owner 与含义 | 明确排除 |
| --- | --- | --- |
| Source Artifact | D9 输入控制项；由 host 固定的外部字节 | 不是 Node、SourceBinding，也不以 path/hash 作为 identity |
| Import IR | D9 conversion；closed typed intermediate evidence | 非第三方 AST 或 author source |
| Source Location / Import Observation | D9 输入版本坐标与提取证据 | 不是 D3 Locator、写入权威或 provenance grant |
| Conversion Provider / Route | D9 host registry；经过评审的 adapter / 有限有序 pipeline | 不是自由 command、fallback、Model Provider 或 worker authorization |
| Import Mapping / Mapping Proposal | D9 Core 的转换选择 / prepare 前不可变 proposal | 不是 Query operator、ActionSpec 或通用 patch |
| Import Job | **D6-owned**，D9 操作 inherited record | 非第二 ledger 或 job-wide atomic transaction |
| Coupling Group / Import Batch | 不可拆分的 author dependency group / 一个原子 request | 不是 UI/Query page 或 worker process |
| Conversion Input | D9 不可变的 raw/IR/mapping/loss/route evidence | 不是 identity、OriginBinding 或 author source |
| Worker Invocation | D9 worker-control record | 无 path、command、permission 或 publication authority |
| Node Template / Template Recipe | D2 Template 身份 / D9 一次性构造配方 | 不具有持久实例绑定，也不执行脚本插值 |
| Office Template / Placeholder / Style Directive / Repeat Band | 普通 Office 字节与可见编译器语法 | 不存在 content-control/named-range/ExcelTable 隐藏层 |
| Render Snapshot / D7 Result Pin | D9 有限投影 / 固定由 D7 拥有的完整结果 | 不是作者快照或 rowHandle 身份 |
| Export Plan | D9 current immutable output preparation，current type `ExportPlan/3` | 非 D6 PreparedIntent 或 author ledger |
| Staged Output / Publication Receipt | 完整未发布字节 / 外部发布事实，现任为 `PublicationReceipt/3` | 绝不是 D3/D6 author receipt |
| Import Loss / Export Loss | 相互分离的 D9 fixed-proposal loss domain | 不是 safety approval 或跨域 address |
| Image Physical Size / imageSizes | 可验证的源物理事实 / 用户 output layout choice | 绝不猜测 host DPI 或 viewport |
| Region Body / d9rg1 | D9 non-identity geometry | 非 complete Locator；outer l1 仍归 D3 |

## Public technical interfaces

唯一 technical owner：
- Probe Interface：`d9_probe`、`d9_probe_result`。
- Conversion Start Interface：`d9_convert`、`d9_conversion_started`。
- Conversion Job Interface：`d9_conversion_state`、`d9_conversion_state_result`、`d9_conversion_cancel`。
- Import Analysis Interface：`d9_import_analyze`、`d9_import_analysis`、`d9_import_choose`。
- Import Preparation Interface：`d9_import_prepare`、`d9_import_prepared`、`d9_import_next`。
- Import Job State Interface：`d9_import_state`、`d9_import_state_result`；操作 D6 ImportJob。
- Node Template Analysis Interface：`d9_template_analyze`。
- Error Interface：`d9_error`。

这些 label 不增加 domain concept 或 submit path。

## Current-version names

final current D9 协调 D7 `PreparedActionBinding/4`、D3 `D3IdentityOperationRequest/13`、D6 `DependencyProof/3` 与 Effect3，以及 D9 `ExportPlan/3` / `PublicationReceipt/3`。D10 较早 naming companion 中 PAB3/Plan2/Receipt2 只表示 intermediate owner provenance；final FC 适用处由本文具名 supersede。

`D7ResultPin` 不拥有 D7 schema/value semantics；nested TerminalSchema/V 仍归 D7。`TemplateRecipe/2` 不拥有 D2 Template identity。`RegionBody/d9rg1` 不拥有 D3 Resource region identity。

## Office native selector terminology

**visible native selector** 指 Office template 中精确可见的 `native.table[...]::column[...]` 文本。**compiled native key** 指 Plan3 内部 ASCII `nt_...` / `nc_...` identifier。二者刻意分离：visible bytes 才是 authoring authority；compiled key 只服务于 canonical Plan structure。完整 `D9NativeTableSelector/1` 保存 semantic selector。

## Retirement names

fresh current 路径拒绝未发布的 legacy ImportIr/YAML proposal、attr/record alias、formula-reorder/broad-view template grammar、自由 provider 命令/回退、自由 binding dictionary、rowHandle 身份、通用 author-receipt alias 与 D9-private Locator kind。historical research file 只能在明确标注为非权威来源证据的前提下保留这些文字。

## D10 direct boundary

D10 可以登记 conversion contribution 并控制外部 capability，但不拥有 D9 wire、Provider/Route semantics、Worker sandbox、template grammar、Plan、PublicationReceipt 或 author commit。D9 Worker networking 默认仍为 denied；Mobile conversion 继续 unavailable。完整 D10 不属于 D9 acceptance。
