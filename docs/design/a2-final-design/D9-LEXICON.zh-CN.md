---
source_language: zh-CN
translation_status: source
---

[English](D9-LEXICON.md)

# A2 D9 术语与命名词表

状态：**author-resolved-pending-independent-review**。D3–D8 owner 不变；D9 只命名 conversion-control concept。

| Concept | Owner 与含义 | 明确排除 |
| --- | --- | --- |
| Source Artifact | D9 input control；host-pinned external bytes | 非 Node、SourceBinding、path/hash identity ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Import IR | D9 conversion；closed typed intermediate evidence | 非第三方 AST 或 author source |
| Source Location / Import Observation | D9 input-version coordinate 与 extraction evidence | 非 D3 Locator/write authority/provenance grant ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Conversion Provider / Route | D9 host registry；reviewed adapter / finite ordered pipeline | 非 free command、fallback、Model Provider 或 worker authorization ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Import Mapping / Mapping Proposal | D9 Core conversion choice / immutable pre-prepare proposal | 非 Query operator、ActionSpec 或 generic patch ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Import Job | **D6-owned**，D9 操作 inherited record | 非第二 ledger 或 job-wide atomic transaction |
| Coupling Group / Import Batch | 不可拆 author dependency group / 一个原 atomic request | 非 UI/Query page 或 worker process ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Conversion Input | D9 immutable raw/IR/mapping/loss/route evidence | 非 identity、OriginBinding 或 author source ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Worker Invocation | D9 worker-control record | 无 path、command、permission 或 publication authority |
| Node Template / Template Recipe | D2 Template identity / D9 one-shot construction recipe | 无 persistent instance binding 或 script interpolation ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Office Template / Placeholder / Style Directive / Repeat Band | ordinary Office bytes 与 visible compiler syntax | 无 content-control/named-range/ExcelTable hidden layer ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Render Snapshot / D7 Result Pin | D9 finite projection / pin D7-owned complete result | 非 author snapshot 或 rowHandle identity ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Export Plan | D9 current immutable output preparation，current type `ExportPlan/3` | 非 D6 PreparedIntent 或 author ledger |
| Staged Output / Publication Receipt | complete unpublished bytes / external publication fact，current `PublicationReceipt/3` | 绝非 D3/D6 author receipt ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Import Loss / Export Loss | 分离的 D9 fixed-proposal loss domain | 非 safety approval 或 cross-domain address ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
| Image Physical Size / imageSizes | verifiable source physical fact / user output layout choice | 绝不猜 host DPI/viewport ；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。 |
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

**visible native selector** 是 Office template 中 exact `native.table[...]::column[...]` 文字。**compiled native key** 是 Plan3 内部 ASCII `nt_...` / `nc_...` identifier。二者刻意分开：visible bytes 是 authoring authority；compiled key 只服务 canonical Plan structure。完整 `D9NativeTableSelector/1` 保存 semantic selector。；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。

## Retirement names

fresh current path 拒绝未发布 legacy ImportIr/YAML proposal、attr/record alias、formula-reorder/broad-view template grammar、free provider command/fallback、free binding dictionary、rowHandle identity、generic author-receipt alias、D9-private Locator kind。historical research 可以在明确 non-authoritative provenance 下保留这些文字。；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。

## D10 direct boundary

D10 可以 catalog conversion contribution 并 gate 外部 capability，但不拥有 D9 wire、Provider/Route semantics、Worker sandbox、template grammar、Plan、PublicationReceipt 或 author commit。D9 Worker networking 继续默认 denied。Mobile conversion 继续 unavailable。完整 D10 不属于 D9 acceptance。；其中英文名称仅表示固定协议标识、字段名或固定字面量，均按本条中文条件解释。
