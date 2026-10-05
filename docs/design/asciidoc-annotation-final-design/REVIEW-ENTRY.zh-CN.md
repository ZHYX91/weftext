---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)

# 独立设计复核入口

## 固定对象
- 仓库：`ZHYX91/weftext`
- parent：`e8aa0b341630a57c786c0891d4bbd1620247441d`
- parent branch：`fix/pl-ir-01-portable-locator-requalification`
- 候选 branch：`docs/asciidoc-annotation-final-design`
- fixed S：`7e18168dad3e6d120fce0dd607dc10fa7894e252`

reviewer 开始时记录实际 Draft PR head SHA，并验证它以 parent e8aa 为祖先。作者交接后 branch 必须停止写入；head移动即重新冻结审查对象。

## 角色
这是非作者独立设计审查，不是实现验收、A2/global review或发布审查。PR3/PL-IR-01既有 bounded 结果不重开，除非本候选直接破坏其合同。

## 必读
1. 根 `AGENTS.md`、`docs/design/AGENTS.md`、`docs/design/README.md`、`docs/design/inputs.json`。
2. 本目录 README、SPEC、SCHEMAS、ACCEPTANCE、terminology registry、replacements。SPEC管行为/算法，SCHEMAS管closed shapes与历史分派，二者均为规范正文。
3. replacements列出的 fixed-e8aa actual-owner规范，包括主owner以及具名 lexicon/impact/acceptance/machine-registry direct holders；不得只读主正文后忽略旧current-version引用。
4. D10 historical prepare binding需核 `d99f053b9386c9c9e1664251fdec9f00e33fac2c` 的真实 `ControlPrepareBinding/1` decoder；该事实只证明decoder来源，不证明部署。

## 必核 Gate
- 完整 Asciidoctor Ruby 2.0.26目标未被Rust候选缩小；Ruby只作oracle。
- Witness7/PropertyProfile/CSP/producer-conformance无第二parser、observer语义写或跨namespace伪时间。
- stable Node双链、authored 6–9、标准leveloffset无effective硬上限、run-in source分离、native title separator保留。
- managed format只有一个portable binding；BaselineOnly不是binding arm；DependencyKey3/PortableComponentKey2 rank准确。
- SourceTransform使用Event3/CoreSourceEditPlan2/Emission1/Evidence2，按original-before映射且seal不重编译。
- Annotation candidate/fuzzy永不授source write；suggestion accept取得fresh target/bytes和新D7 prepare。
- CP4/ChangeRecord1/Declaration2/Carry2/historical transform cut/Handle1/dual-profile continuation闭合。
- D3/D4/D5/PAB4/Edit3/ExportPlan3/D10 mixed-holder closure完整；历史bytes不重编码。
- D10 VersionedControlPrepareBinding恰含Binding1/2/3并按StableControlKey跨版本唯一。
- provider可用性不混为语言有效性；本PR没有provider runtime通过证据。
- 双语 ACCEPTANCE 恰含438 core +217 coordination=655条唯一义务，全部未运行。

## 裁决
给P0/P1/P2、具名finding、逐项disposition、阅读范围、未验证实现范围，以及是否允许进入随后A2/global阶段。文档一致性不能代替语言/provider/replica/crash实现证据。
