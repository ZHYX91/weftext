---
source_language: zh-CN
translation_status: source
---

[English](ACCEPTANCE.md)

# 设计验收矩阵

状态：**design obligations only; not executed**。

- core design oracles: 438
- actual-owner coordination fixtures: 312
- total: 750

每项都是本候选的规范义务；PASS/FAIL文字描述的是未来验收条件，不表示本轮已经运行。逐项与 SPEC.zh-CN.md 的对应关系由 ID 前缀和下表“规范域”给出。

## Core — 438

| ID | 规范域 | 必须满足的条件 |
|---|---|---|
| AD2-01 | SPEC §§1–4 | 所有 core-language oracle固定 asciidoctor-ruby/2.0.26@0b99b39…；rolling docs/latest版本不得改变该 corpus 结果。 |
| AD2-02 | SPEC §§1–4 | 文字、escape、specialchars、quotes、attribute substitution、replacements、macros、post-replacements按2.0.26实际 substitution order相等。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-03 | SPEC §§1–4 | hard-set API attr可以改变 generic effective attributes；它不能成为 wf-kind/wf-facets authored control authority。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-04 | SPEC §§1–4 | source无 wf-kind、host只注入同名attr时，generic environment可有该值，但 Node仍 ordinary。 |
| AD2-05 | SPEC §§1–4 | soft-set与 hard-set必须可区分；source可覆盖 soft-set但不能覆盖 hard-set。 |
| AD2-06 | SPEC §§1–4 | 完整重放2.0.26 authored attribute events及其顺序/set/unset/substitution；RootAuthoredControlProjection必须消费同一 parser产生的 authored provenance，不能另写一个简化 assignment parser。 |
| AD2-07 | SPEC §§1–4 | body attr assignment只影响其后 generic AsciiDoc处理；不能反向接管 root Node classification。 |
| AD2-08 | SPEC §§1–4 | included source内 wf-kind/wf-facets可按普通 AsciiDoc参与 include环境，但不能改变 including root Node control。 |
| AD2-09 | SPEC §§1–4 | root control引用另一个 root-authored attribute时可生效；如果 control最终值依赖 host/include/builtin provenance，则 generic AsciiDoc仍可有效，但 Weftext Node-control必须 unavailable/invalid而非接管。 |
| AD2-10 | SPEC §§1–4 | link:n1.W.N[label]先由标准 link grammar处理，再由 Weftext adapter投影 NodeLink；不能存在第二 link parser。 |
| AD2-11 | SPEC §§1–4 | n1 target合法时不要求自造 ++ passthrough canonical syntax。 |
| AD2-12 | SPEC §§1–4 | link/citation label corpus覆盖 Doe, 2025、a=b、合法 ] escaping、single/double quote、comma、role attrs及2.0.26允许的 multiline attr-list情况；全部由 baseline attr-list parser处理。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-13 | SPEC §§1–4 | native bibliography anchor/xref与 Weftext citation role可同文档并存，不自动合并 provenance/identity。 |
| AD2-14 | SPEC §§1–4 | derived Weftext bibliography placement的删除只删除 placement，不删除 cited Node或引用事实。 |
| AD2-15 | SPEC §§1–4 | r1.UUID只能按 containing Node解析 owner-local Resource；另一个 owner的同 UUID leaf不是同 ResourceRef。 |
| AD2-16 | SPEC §§1–4 | image alt/width/height等使用 native image semantics；copy rewrite只重写 Resource identity，不损失 occurrence presentation。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-17 | SPEC §§1–4 | audio使用其实际 title/start/end/options等 native attrs，不凭 image模型添加字段。 |
| AD2-18 | SPEC §§1–4 | video poster/width/height/start/end/options按固定2.0.26语义。 |
| AD2-19 | SPEC §§1–4 | D4 carrier inner payload取 exact authored literal range；renderer escaping不得成为 typed input。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-20 | SPEC §§1–4 | payload包含默认 delimiter行时 emitter选择合法不碰撞 delimiter；parse→source round-trip payload byte-equal。 |
| AD2-21 | SPEC §§1–4 | included managed source中的 carrier/query/view仍归真实 source owner。 |
| AD2-22 | SPEC §§1–4 | local include、tags、lines、indent、leveloffset等获准时与 Ruby 2.0.26 semantic witness相等。 |
| AD2-23 | SPEC §§1–4 | include因 file permission被拒时 source syntax仍 valid；EvaluationStatus为 effect_denied/incomplete，directive bytes保留。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-24 | SPEC §§1–4 | URL include只能消费 exact authorized snapshot；相同URL后续新bytes不得继承旧 current proof。 |
| AD2-25 | SPEC §§1–4 | unmanaged snapshot可 one-shot read/preview/export，但不能充当长期 current Query/Action dependency。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-26 | SPEC §§1–4 | attribute/include/substitution出现多个 source origins时必须保留完整 origin graph；Write/Annotation不能自动挑最近 origin。 |
| AD2-27 | SPEC §§1–4 | unordered/ordered/description/callout/checklist/hybrid/mixed list以及 list continuation/attached blocks均完整。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-28 | SPEC §§1–4 | checklist/list continuation + attached table等历史 parser edge case不得 panic/silent flatten；不等价即 core Gate fail。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-29 | SPEC §§1–4 | PSV/CSV/DSV/TSV、cols、header/footer、span/dup/alignment/style、a AsciiDoc cell完整进入 semantic corpus。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AD2-30 | SPEC §§1–4 | Complex Source↔Write corpus必须同时含 deep level6–9 heading、run-in、table/list/STEM；只改一个普通 paragraph后，所有未触及 bytes（含 marker、role、trivia）逐字节保持；随后 HTML/PDF/DOCX仍分别满足 deep/run-in backend规则。 |
| AD2-31 | SPEC §§1–4 | rich editor没有某控件时，该合法 construct仍能 parse/read/source-edit/save；不能以控件缺失判 unsupported syntax。 |
| AD2-32 | SPEC §§1–4 | STEM provider缺失只影响 BackendStatus，不影响 core language validity。 |
| AD2-33 | SPEC §§1–4 | Mermaid fixed provider产生的 SVG必须通过 SvgOutputProfile。 |
| AD2-34 | SPEC §§1–4 | SVG含 script/event/external network URI/DTD/entity/未授权CSS/font时即使 provider成功也拒绝。 |
| AD2-35 | SPEC §§1–4 | provider/config/font/runtime profile变化必须失效相应 render cache。 |
| AD2-36 | SPEC §§1–4 | PDF unavailable时 exact .adoc export仍可成功。 |
| AD2-37 | SPEC §§1–4 | Oracle semantic witness与 Weftext provenance witness分别 deterministic；线程/map迭代顺序不能改变 canonical output。 |
| AD2-38 | SPEC §§1–4 | diagnostics比较 severity/semantic code/location；exact message仍保留以发现 normalization过度。 |
| AD2-39 | SPEC §§1–4 | 任何真实 core-language mismatch不得登记成 backend/environment/Weftext-domain exception。 |
| AD2-40 | SPEC §§1–4 | Asciidork补齐成本大小不改变“完整2.0.26”验收范围。 |
| AD2-41 | SPEC §§1–4 | embedded 模式下，API attributes 按 notitle="" 后 showtitle="" 与相反顺序输入时必须是两个不同环境：固定 Ruby 2.0.26 依据 ordered attr_overrides 中二者最后出现的 key 派生另一个 alias；canonical environment bytes 与最终 title/effective-attribute 观察都必须保留该 API 顺序，按名字排序合并为同一环境必须失败。 |
| AN2-01 | SPEC §§9–16 | 每个 root/reply均是独立 AnnotationRef；不存在 ThreadMessage identity。 |
| AN2-02 | SPEC §§9–16 | thread机械等于 D3 same-owner reply_closure，cycle必须拒绝。 |
| AN2-03 | SPEC §§9–16 | 两个 concurrent fresh replies可同时存在，不使用时间戳 LWW。 |
| AN2-04 | SPEC §§9–16 | 同一 reply concurrent edit必须通过 Annotation revision/CAS/ConflictRecord处理。 |
| AN2-05 | SPEC §§9–16 | reviewState与 target resolution状态正交。 |
| AN2-06 | SPEC §§9–16 | reopen只改 root reviewState，不自动修改 target/reanchor。 |
| AN2-07 | SPEC §§9–16 | highlight/underline/squiggle/strike × yellow/red/green/blue/purple/pink/gray portable round-trip。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AN2-08 | SPEC §§9–16 | Annotation inline body完整支持其 frozen doctype=inline profile中的 CJK/RTL/emoji/strong/emphasis/link/STEM等。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AN2-09 | SPEC §§9–16 | trusted insertion在 target前时 exact range按 transform机械移动，无需人工 reanchor。 |
| AN2-10 | SPEC §§9–16 | insertion在 range start时按 fixed affinity保留原选择文字。 |
| AN2-11 | SPEC §§9–16 | insertion在 range end时不自动扩大旧 selection。 |
| AN2-12 | SPEC §§9–16 | 真正 overlap时 trusted mapping停止；context候选不能变成 write authority。 |
| AN2-13 | SPEC §§9–16 | observed_only或只有 whole before/after、没有 Core edit provenance的 save不得产生 SourceTransform。 |
| AN2-14 | SPEC §§9–16 | external edit后唯一 quote/context只可生成 candidate。 |
| AN2-15 | SPEC §§9–16 | 重复文本多候选必须 ambiguous，不选第一个/最近。 |
| AN2-16 | SPEC §§9–16 | signed transform chain缺任意 transition即不能 mapped。 |
| AN2-17 | SPEC §§9–16 | source先到、artifact后到期间不能提前声称 mapped。 |
| AN2-18 | SPEC §§9–16 | 错误 signature/profile/trust cut/cross-field的 transform必须拒绝。 |
| AN2-19 | SPEC §§9–16 | mapped仅作为当前 fresh qualification输入，不能复活旧 PreparedIntent/ActionEvidence。 |
| AN2-20 | SPEC §§9–16 | replace accept需要 current fresh target、current expected bytes及新的 D7 prepare。 |
| AN2-21 | SPEC §§9–16 | geometry仍可映射但 expected bytes改变时 suggestion不能 accept。 |
| AN2-22 | SPEC §§9–16 | delete同样必须 current exact expected bytes。 |
| AN2-23 | SPEC §§9–16 | insert使用 zero-width target + pointAffinity + fresh current qualification。 |
| AN2-24 | SPEC §§9–16 | explicit reanchor令 pending suggestion进入 needs_reconfirmation。 |
| AN2-25 | SPEC §§9–16 | reconfirm必须在 current exact target上重建 targetBasis及 expected bytes/point。 |
| AN2-26 | SPEC §§9–16 | accept在一个 D6 seal同时写 Document after与 Annotation accepted。 |
| AN2-27 | SPEC §§9–16 | accept/reject race对同 Annotation current version只有一个 winner。 |
| AN2-28 | SPEC §§9–16 | 失去 target disclosure时流程必须在读取 transform/current geometry前停止。 |
| AN2-29 | SPEC §§9–16 | 只有 annotation/history source disclosure同时成立时才能读旧 quote/prefix/suffix。 |
| AN2-30 | SPEC §§9–16 | raw backup已交付 bytes不能被后续 revoke神奇收回；在线 API仍服从当前权限。 |
| AN2-31 | SPEC §§9–16 | Review Bundle无 history权限时实际不包含 historical excerpts。 |
| AN2-32 | SPEC §§9–16 | root-authored source span的 Annotation owner=root Node。 |
| AN2-33 | SPEC §§9–16 | managed included Node source span的 Annotation owner=被 include Node；root permission不能代替。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| AN2-34 | SPEC §§9–16 | unmanaged/network included bytes不能直接建立 durable exact target，可 annotate directive或先 adopt。 |
| AN2-35 | SPEC §§9–16 | synthetic multi-origin text不能自动选择一个 owner。 |
| AN2-36 | SPEC §§9–16 | PDF/image normalized rect绑定 exact ResourceVersion，zoom/shell变化不改变它。 |
| AN2-37 | SPEC §§9–16 | audio/video rational timebase/ticks及 [start,end) round-trip。 |
| AN2-38 | SPEC §§9–16 | media decoder/profile unavailable时已有 raw record保留，新 exact region creation unavailable。 |
| AN2-39 | SPEC §§9–16 | Node copy必须 fresh AnnotationRefs、完整 rewrite reply graph、保留 creator/authoredAt/lastEditor/editedAt attribution，并通过真实 source/resource identityMap重发 target；不能复制旧 locator或按文本猜位置。 |
| AN2-40 | SPEC §§9–16 | 单个 Annotation value损坏不能被解释为“该 owner没有批注”。 |
| AN2-41 | SPEC §§9–16 | unknown future Value version必须保留 raw bytes/identity并报告 unsupported。 |
| AN2-42 | SPEC §§9–16 | unresolved suggestion所需 history/transform/trust evidence受 last-reference retention保护。 |
| AN2-43 | SPEC §§9–16 | explicit history prune后历史查询明确 unavailable，不拿 current value冒充。 |
| AN2-44 | SPEC §§9–16 | resolved thread普通 reply不自动 reopen；Reply-and-reopen是显式 compound operation。 |
| AN2-45 | SPEC §§9–16 | Trash/restore/purge服从 reply closure；只有 purge终止该 identity恢复能力。 |
| WFX-L01 | SPEC §§1.4,7–8 | Node title变化不改变 stable n1 target。 |
| WFX-L02 | SPEC §§1.4,7–8 | path/move变化不改变 NodeRef link。 |
| WFX-L03 | SPEC §§1.4,7–8 | explicit authored label保持原文。 |
| WFX-L04 | SPEC §§1.4,7–8 | dynamic label仅在当前授权可读取 target title时显示；否则 neutral，不泄漏旧 title/path。 |
| WFX-L05 | SPEC §§1.4,7–8 | same-document anchor仍用 native xref。 |
| WFX-L06 | SPEC §§1.4,7–8 | 普通 inter-document xref保持标准语义。 |
| WFX-L07 | SPEC §§1.4,7–8 | 删除 Derived Index后 backlinks可从 actual source/references重建。 |
| WFX-L08 | SPEC §§1.4,7–8 | partial index不能证明 zero backlinks。 |
| WFX-L09 | SPEC §§1.4,7–8 | complete backlink结果必须具有 ref_inbound/source/lifecycle/auth completeness proof。 |
| WFX-L10 | SPEC §§1.4,7–8 | hidden source Node不能因引用 visible target而泄漏 title/path/excerpt。 |
| WFX-L11 | SPEC §§1.4,7–8 | disclosure不足时 hidden / absent / broken保持 non-disclosing。 |
| WFX-L12 | SPEC §§1.4,7–8 | subtree copy closure内 NodeLink按 identityMap rewrite。 |
| WFX-L13 | SPEC §§1.4,7–8 | closure外合法 same-Workspace target按 D3 preserve。 |
| WFX-L14 | SPEC §§1.4,7–8 | Workspace fork完整 rewrite closure-internal stable refs。 |
| WFX-L15 | SPEC §§1.4,7–8 | ordinary import无 identity manifest不能按 title/path自动重连。 |
| WFX-L16 | SPEC §§1.4,7–8 | Weftext citation进入同一 inbound graph但保留 citation semanticRole。 |
| WFX-L17 | SPEC §§1.4,7–8 | label strong/emphasis/escaping等由 baseline inline parser处理。 |
| WFX-L18 | SPEC §§1.4,7–8 | backlinks永远 derived，不写成 target Node的第二 author fact。 |
| WFX-H01 | SPEC §1.3 | BaselineOnly中 7+ equals完全遵循 Ruby 2.0.26 baseline。 |
| WFX-H02 | SPEC §1.3 | WeftextManaged 7 equals为 authored level6。 |
| WFX-H03 | SPEC §1.3 | 8/9/10 equals分别 authored level7/8/9。 |
| WFX-H04 | SPEC §1.3 | authored levels1–5完整保持 baseline意义。 |
| WFX-H05 | SPEC §1.3 | document title/book part level0语义不被 deep extension改写。 |
| WFX-H06 | SPEC §1.3 | 5→7等 skip只 warning + continue，不能 hard reject。 |
| WFX-H07 | SPEC §1.3 | deep section回祖先 level正常闭合 hierarchy。 |
| WFX-H08 | SPEC §1.3 | leveloffset参与真实2.0.26 state machine，effective可6–9乃至更深。 |
| WFX-H09 | SPEC §1.3 | offset-derived effective>9不得被 Weftext semantic cap拒绝。 |
| WFX-H10 | SPEC §1.3 | deep explicit anchor进入正常 catalog/xref。 |
| WFX-H11 | SPEC §1.3 | auto ID沿用同一 baseline generator。 |
| WFX-H12 | SPEC §1.3 | outline保存 exact effective level。 |
| WFX-H13 | SPEC §1.3 | folding/navigation完整保存 deep hierarchy。 |
| WFX-H14 | SPEC §1.3 | D7 heading query返回 exact level及 locator。 |
| WFX-H15 | SPEC §1.3 | heading level rich edit只修改 marker，然后完整 reparse。 |
| WFX-H16 | SPEC §1.3 | effective1–5 HTML映射 h2–h6。 |
| WFX-H17 | SPEC §1.3 | effective6–8使用 role=heading / aria-level7–9。 |
| WFX-H18 | SPEC §1.3 | effective9使用 aria-level10并保留完整 accessible outline。 |
| WFX-H19 | SPEC §1.3 | deep levels不得全部 flatten成同一 semantic level。 |
| WFX-H20 | SPEC §1.3 | DOCX provider至少对 effective1–9保持 Heading1–9结构。 |
| WFX-H21 | SPEC §1.3 | PDF/other backend表达不足时显式 degradation，不改 source/AST。 |
| WFX-H22 | SPEC §1.3 | exact .adoc export不重写 heading markers/leveloffset。 |
| WFX-R01 | SPEC §1.3 | 无 explicit role且 workspace default=separate时保持 Separate。 |
| WFX-R02 | SPEC §1.3 | .run-in + eligible first paragraph形成 RunIn。 |
| WFX-R03 | SPEC §1.3 | .separate强制 Separate，即使 workspace default=run_in。 |
| WFX-R04 | SPEC §1.3 | .run-in.separate仍是合法 AsciiDoc source，产生 typed role_conflict + Separate fallback。 |
| WFX-R05 | SPEC §1.3 | explicit run-in允许 blank source trivia。 |
| WFX-R06 | SPEC §1.3 | explicit run-in允许 comment trivia。 |
| WFX-R07 | SPEC §1.3 | 第一 paragraph合法 ID/role attrs不破坏 semantic adjacency。 |
| WFX-R08 | SPEC §1.3 | 真正 intervening semantic block阻断 run-in。 |
| WFX-R09 | SPEC §1.3 | list/table/admonition/source/image等 next block不得被强拼正文。 |
| WFX-R10 | SPEC §1.3 | no body时 Separate。 |
| WFX-R11 | SPEC §1.3 | 多个 paragraphs只有第一段参与 visual run-in。 |
| WFX-R12 | SPEC §1.3 | run-in不删除 heading anchor/xref/outline身份。 |
| WFX-R13 | SPEC §1.3 | heading与paragraph exact source ranges始终独立。 |
| WFX-R14 | SPEC §1.3 | heading/body inline AST分别解析，不把拼接文本重新 parse。 |
| WFX-R15 | SPEC §1.3 | RTL/CJK/emoji不插入硬编码 LTR punctuation。 |
| WFX-R16 | SPEC §1.3 | synthetic visual join没有 writable source scalar。 |
| WFX-R17 | SPEC §1.3 | Copy Text必须得到 rendered heading text + U+0020 + rendered first paragraph text；Copy Source Fragment必须保留 heading source、两者之间全部真实 blank/comment/attribute trivia及 paragraph source，不能把 synthetic join写回 source。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| WFX-R18 | SPEC §1.3 | Enable仅保留 .run-in；Disable仅保留 .separate；Use Default移除两者。 |
| WFX-R19 | SPEC §1.3 | body Enter形成第二 paragraph后只有第一段继续 run-in。 |
| WFX-R20 | SPEC §1.3 | body-start Backspace不能默默消灭 heading structure。 |
| WFX-R21 | SPEC §1.3 | authored deep heading 6–9同样支持 run-in。 |
| WFX-R22 | SPEC §1.3 | backend不支持时只允许 presentation degradation，heading/body semantics必须保留。 |
| T3-ENV-01 | SPEC §§1.2,4 | 同 source在 html5与docbook processor backend下，ifdef::backend-html5[]结果及 context digest不同。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-ENV-02 | SPEC §§1.2,4 | SERVER/SECURE safe mode对 docdir/docfile masking与 SAFE结果一致于2.0.26。 |
| T3-ENV-03 | SPEC §§1.2,4 | logical docfile/baseDir变化正确改变 {docfile} / relative include及 context digest。 |
| T3-ENV-04 | SPEC §§1.2,4 | SOURCE_DATE_EPOCH变化正确改变 local/doc intrinsic time attrs。 |
| T3-ENV-05 | SPEC §§1.2,4 | hard_set、soft_set、hard_unset、soft_unset四态逐项复现 Ruby value/value@/nil/false precedence。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-ENV-06 | SPEC §§1.2,4 | managed include SourceObservation变化使旧 evaluation dependency stale。 |
| T3-ENV-07 | SPEC §§1.2,4 | external immutable pin byte-equal时可 exact replay；不同 pin不能复用旧 evaluation。 |
| T3-ENV-08 | SPEC §§1.2,4 | safe<SERVER下两个 ambientUserHome值使 {user-home}结果/context不同。 |
| T3-ENV-09 | SPEC §§1.2,4 | SERVER/SECURE下 ambientUserHome变化不改变 processor default "."。 |
| T3-ENV-10 | SPEC §§1.2,4 | SOURCE_DATE_EPOCH存在时优先于 inputMtime；epoch缺失时 inputMtime优先于 clockNow用于 doc*。 |
| T3-ENV-11 | SPEC §§1.2,4 | outfile/outdir在 processor可见时变化必须进入 context。 |
| T3-ENV-12 | SPEC §§1.2,4 | Managed profile不得读取未进入 context的 cwd/home/clock/file metadata。 |
| T3-ORACLE-01 | SPEC §§2–3 | Ruby sourcemap无 inline range时 Oracle witness仍合法，且不伪造 SourceOrigin。 |
| T3-ORACLE-02 | SPEC §§2–3 | strong/emphasis/link/footnote由真实 Ruby semantic converter callback观察。 |
| T3-ORACLE-03 | SPEC §§2–3 | block metadata完全相同、inline semantics不同必须产生不同 witness。 |
| T3-ORACLE-04 | SPEC §§2–3 | Weftext provenance range错1 byte时 provenance Gate失败，即使 semantic witness相同。 |
| T3-ORACLE-05 | SPEC §§2–3 | 产品 HTML DOM与 Ruby DOM不同但 semantic events一致时 Core Gate可以通过；HTML Backend Gate独立判。 |
| T3-ORACLE-06 | SPEC §§2–3 | observer不得删除 target/type/role/terms/see等导致真实不同语义归一化。 |
| T3-ORACLE-07 | SPEC §§2–3 | diagnostics exactMessage保留，semanticCode映射错误必须 fail。 |
| T3-ORACLE-08 | SPEC §§2–3 | authored/included/substituted/synthetic provenance arms全部 decoder round-trip。 |
| T3-ORACLE-09 | SPEC §§2–3 | provenance graph cycle/forward/unknown reference拒绝。 |
| T3-ORACLE-10 | SPEC §§2–3 | run-in synthetic join只能引用真实 heading/body origins且不可写。 |
| T3-ORACLE-11 | SPEC §§2–3 | concealed indexterm: 即使 HTML输出为空，也必须产生 concealed indexterm semantic event。 |
| T3-ORACLE-12 | SPEC §§2–3 | visible/concealed indexterm的 text/terms/see/see-also均可区分。 |
| T3-ORACLE-13 | SPEC §§2–3 | toc::[]必须产生 context=toc/content_model=empty block event。 |
| T3-ORACLE-14 | SPEC §§2–3 | 固定2.0.26出现未登记 built-in converter transform必须 oracle_contract_incomplete，不能 ignore。 |
| T3-ORACLE-15 | SPEC §§2–3 | ListItem/Table::Cell由 parent callback机械枚举，不能因无独立 callback而丢失。 |
| T3-ORACLE-16 | SPEC §§2–3 | Alpha. 与 Beta. 在其它 metadata/catalog/diagnostics相同时必须产生不同 text fragments。 |
| T3-ORACLE-17 | SPEC §§2–3 | literal block仅普通 text不同必须产生不同 witness。 |
| T3-ORACLE-18 | SPEC §§2–3 | pass/raw block实际合法 substitutions的结果必须进入 text fragment，不能只比较 node.source。 |
| T3-ORACLE-19 | SPEC §§2–3 | mixed Alpha *{name}* Beta必须保留 ordered text → strong start → resolved text → strong end → text。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-ORACLE-20 | SPEC §§2–3 | 一个 {counter:x} 每次只自增一次；semantic probe不能因额外 content访问导致双增。 |
| T3-ORACLE-21 | SPEC §§2–3 | 连续 counter在 document traversal得到1、2、3等真实 stateful顺序。 |
| T3-ORACLE-22 | SPEC §§2–3 | footnote注册/编号只发生一次，fragment event与最终 catalog一致。 |
| T3-ORACLE-23 | SPEC §§2–3 | quoted node中后续 attribute/macro substitutions仍可运行；probe marker不得使 quote payload opaque。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-ORACLE-24 | SPEC §§2–3 | 同一 scope被 semantic pass求值第二次必须直接使 oracle adapter失败。 |
| T3-ORACLE-25 | SPEC §§2–3 | Backend Gate必须使用 fresh Ruby Document，不能复用已被 semantic pass counter/footnote mutation过的 Document。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-PROFILE-01 | SPEC §§5–8 | managed Node不能由 caller切成 BaselineOnly。 |
| T3-PROFILE-02 | SPEC §§5–8 | ordinary import source含 deep marker时 DeltaReport列 deep_section。 |
| T3-PROFILE-03 | SPEC §§5–8 | ordinary source含 .run-in 时列 explicit_run_in。 |
| T3-PROFILE-04 | SPEC §§5–8 | root wf-kind/facets产生明确 managed delta。 |
| T3-PROFILE-05 | SPEC §§5–8 | n1/r1/citation/carrier/query/view全部进入完整 delta inventory。 |
| T3-PROFILE-06 | SPEC §§5–8 | 无 semantic delta时可以直接建立 /1 managed binding，但仍必须真实安装 binding component。 |
| T3-PROFILE-07 | SPEC §§5–8 | 用户拒绝 managed activation时不得分配 managed Node。 |
| T3-PROFILE-08 | SPEC §§5–8 | trusted Weftext transfer保留 exact profile generation。 |
| T3-PROFILE-09 | SPEC §§5–8 | future /2存在时 old /1 Node仍按 /1解释。 |
| T3-PROFILE-10 | SPEC §§5–8 | source变化后旧 DeltaReport digest不能用于 confirmation。 |
| T3-PROFILE-11 | SPEC §§5–8 | DeltaReport ordering/digest跨实现 deterministic。 |
| T3-PROFILE-12 | SPEC §§5–8 | profile migration必须显式分析/确认，不能 open-time auto-upgrade。 |
| T3-PROFILE-13 | SPEC §§5–8 | fresh import Notice必须同时含 document 与 document_format component。 |
| T3-PROFILE-14 | SPEC §§5–8 | 只有 Document bytes、缺 format component时状态是 incomplete/proof_unavailable，不得 default任何 profile。 |
| T3-PROFILE-15 | SPEC §§5–8 | completion proof component set缺 document_format时 receiver不得加入 Frontier。 |
| T3-PROFILE-16 | SPEC §§5–8 | profile-only migration即使 source byte-equal仍是 portable metadata change，有 ChangeId。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-PROFILE-17 | SPEC §§5–8 | profile+source migration时两个 components同一 plan/CAS/seal。 |
| T3-PROFILE-18 | SPEC §§5–8 | same-Workspace copy fresh Node bindingRevision=1并保留 source profile generation。 |
| T3-PROFILE-19 | SPEC §§5–8 | fork不自动升级 profile generation。 |
| T3-PROFILE-20 | SPEC §§5–8 | formal restore恢复 exact historical binding；ordinary import fresh identity走 import gate。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-PROFILE-21 | SPEC §§5–8 | 每个 managed semantic read同时取得 exact SourceObservation与 DocumentFormatCurrentQualification。 |
| T3-PROFILE-22 | SPEC §§5–8 | strong DependencyProof同时含 source(owner) 与 document_format(owner)。 |
| T3-PROFILE-23 | SPEC §§5–8 | profile-only /1→/2、source unchanged时 source dependency不变但 format stamp改变；旧 D7 prepare必须 stale。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-PROFILE-24 | SPEC §§5–8 | scope_dependencies不得把已消费 document_format的 stamp变化判 unrelated。 |
| T3-PROFILE-25 | SPEC §§5–8 | D2 AST cache必须以 format stamp/profile为 key；metadata-only migration使旧 AST stale。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-PROFILE-26 | SPEC §§5–8 | D4 typed projection随 format stamp失效并重算。 |
| T3-PROFILE-27 | SPEC §§5–8 | D8 old Draft文本保留，但旧 projection不能直接 prepare，必须按 current profile reproject。 |
| T3-PROFILE-28 | SPEC §§5–8 | D9 frozen export绑定 exact format qualification；执行前变化需 reprepare。 |
| T3-PROFILE-29 | SPEC §§5–8 | Derived Index cache含 format stamp；migration必须失效 heading/ref/query候选。 |
| T3-PROFILE-30 | SPEC §§5–8 | Trash仅 lifecycle变化时 exact binding bytes/revision/profile/stamp保持。 |
| T3-PROFILE-31 | SPEC §§5–8 | restore使用 Trash保留的 exact binding，绝不使用 latest/default/BaselineOnly。 |
| T3-PROFILE-32 | SPEC §§5–8 | purge Notice/CP必须把 current document_format component从 present变 absent。 |
| T3-PROFILE-33 | SPEC §§5–8 | purge后 tombstone不得产生 current ManagedDocumentSemanticQualification。 |
| T3-PROFILE-34 | SPEC §§5–8 | purge后 retained historical format bytes只能服务历史/recovery，不能成为 current binding。 |
| T3-PROFILE-35 | SPEC §§5–8 | document_format stamp continuity gap创建新 epoch；equal bytes不能恢复旧 stamp。 |
| T3-H01 | SPEC §1.3 | article standard levels1–5与 Ruby 2.0.26一致。 |
| T3-H02 | SPEC §1.3 | Managed raw markers level6–9创建对应 section。 |
| T3-H03 | SPEC §1.3 | skip level产生 warning并继续。 |
| T3-H04 | SPEC §1.3 | fragment root可直接从非level1开始且保持2.0.26 root relaxation。 |
| T3-H05 | SPEC §1.3 | fragment nested skip仍产生真实 expected-level warning。 |
| T3-H06 | SPEC §1.3 | book document title后 level0可成为 part。 |
| T3-H07 | SPEC §1.3 | article不合法 level0保持 Ruby原 error/recovery语义。 |
| T3-H08 | SPEC §1.3 | [discrete] / [float] deep heading不进入 section hierarchy。 |
| T3-H09 | SPEC §1.3 | deep discrete auto ID正常进入 refs catalog。 |
| T3-H10 | SPEC §1.3 | positive leveloffset作用于 ordinary/discrete heading。 |
| T3-H11 | SPEC §1.3 | doctitle判断使用 rawLevel+offset==0。 |
| T3-H12 | SPEC §1.3 | 正式 section parse的 negative effective level按2.0.26 clamp0。 |
| T3-H13 | SPEC §1.3 | effective level10+因 leveloffset产生时不得被 Weftext判 invalid。 |
| T3-H14 | SPEC §1.3 | appendix/bibliography/glossary等 special semantics不被 deep extension破坏。 |
| T3-H15 | SPEC §1.3 | :toclevels: 4按 Ruby numeric comparison工作。 |
| T3-H16 | SPEC §1.3 | :toclevels: 9在 deep AST上自然显示到对应深度。 |
| T3-H17 | SPEC §1.3 | :sectnumlevels: 2只编号至2。 |
| T3-H18 | SPEC §1.3 | :sectnumlevels: 9允许 deep numbering至9。 |
| T3-H19 | SPEC §1.3 | 11+ equals不成为 Weftext deep-heading error，继续 baseline non-heading路径。 |
| T3-H20 | SPEC §1.3 | standard marker level5 + leveloffset+10可得到 effective15并保留结构。 |
| T3-H21 | SPEC §1.3 | HTML effective>9保留 exact aria/data level并只报告 backend/AT degradation。 |
| T3-H22 | SPEC §1.3 | DOCX/PDF对超能力 level使用 explicit degradation，不改 source。 |
| T3-TITLE-01 | SPEC §1.1 | = Main: Subtitle解析 main/subtitle。 |
| T3-TITLE-02 | SPEC §1.1 | = A: B: C按最后一个 : 切 main=A: B / subtitle=C。 |
| T3-TITLE-03 | SPEC §1.1 | [separator=::]正常使用 :: 。 |
| T3-TITLE-04 | SPEC §1.1 | :title-separator: ::正常生效。 |
| T3-TITLE-05 | SPEC §1.1 | WeftextManaged不隐式把默认 separator改成 ::。 |
| T3-TITLE-06 | SPEC §1.1 | D2/D8/D9读取完全相同 title/subtitle projection。 |
| T3-FACET-01 | SPEC §§5,7 | 32个 lexical-valid FacetId成功。 |
| T3-FACET-02 | SPEC §§5,7 | 第33个 Facet拒绝 managed-domain commit。 |
| T3-FACET-03 | SPEC §§5,7 | exact duplicate FacetId拒绝。 |
| T3-FACET-04 | SPEC §§5,7 | FacetId最大长度边界按 frozen decoder执行。 |
| T3-FACET-05 | SPEC §§5,7 | uppercase、非法 dot/hyphen/segment结构拒绝。 |
| T3-FACET-06 | SPEC §§5,7 | lexical-valid但 Registry未知 Facet保持 source-valid，并在 typed provider层 unavailable。 |
| T3-FACET-07 | SPEC §§5,7 | host/API wf-facets不得修改 root Node membership。 |
| T3-FACET-08 | SPEC §§5,7 | included source wf-facets不得接管 including root Node。 |
| T3-RUN-01 | SPEC §1.3 | workspace policy=run_in + physical adjacency可以 implicit RunIn。 |
| T3-RUN-02 | SPEC §1.3 | implicit default遇 blank line必须 Separate。 |
| T3-RUN-03 | SPEC §1.3 | implicit default遇 comment必须 Separate。 |
| T3-RUN-04 | SPEC §1.3 | explicit .run-in允许 blank/comment/paragraph metadata并按 semantic adjacency RunIn。 |
| T3-RUN-05 | SPEC §1.3 | .separate覆盖 workspace run_in default。 |
| T3-RUN-06 | SPEC §1.3 | .run-in.separate → warning + Separate，不使 source invalid。 |
| T3-RUN-07 | SPEC §1.3 | role conflict属于 presentation result而非 SyntaxStatus invalid。 |
| T3-RUN-08 | SPEC §1.3 | presentation policy revision变化使 D8 render cache失效。 |
| T3-RUN-09 | SPEC §1.3 | frozen D9 export继续使用冻结 policy，即使 workspace随后改 setting。 |
| T3-RUN-10 | SPEC §1.3 | Enable删除 separate并唯一保留 run-in。 |
| T3-RUN-11 | SPEC §1.3 | Disable删除 run-in并确保 separate，因此 default=run_in也不会重新生效。 |
| T3-RUN-12 | SPEC §1.3 | Use Default移除两 role并恢复当前 workspace policy。 |
| T3-XF-01 | SPEC §§11–13 | replacement [20,30)→3 bytes使后方 target左移7。 |
| T3-XF-02 | SPEC §§11–13 | delete strictly before target正确 delta shift。 |
| T3-XF-03 | SPEC §§11–13 | edit strictly after target unchanged。 |
| T3-XF-04 | SPEC §§11–13 | insertion at nonzero range start使 range整体后移以保留原文字。 |
| T3-XF-05 | SPEC §§11–13 | insertion at range end不扩大旧 selection。 |
| T3-XF-06 | SPEC §§11–13 | insertion strictly inside range停止 mapping。 |
| T3-XF-07 | SPEC §§11–13 | zero target + left affinity保持 insertion前。 |
| T3-XF-08 | SPEC §§11–13 | zero target + right affinity移动至 insertion后。 |
| T3-XF-09 | SPEC §§11–13 | 多个 disjoint original edits按 original coordinates累计 delta。 |
| T3-XF-10 | SPEC §§11–13 | target与真实 replacement overlap时 mapping停止。 |
| T3-XF-11 | SPEC §§11–13 | whole-source save只有 before/after且无 provenance → compilation unavailable / no artifact。 |
| T3-XF-12 | SPEC §§11–13 | generic post-hoc diff永远不得生成 trusted transform。 |
| T3-XF-13 | SPEC §§11–13 | revision-token-only trust authorization不能验证 transform profile。 |
| T3-XF-14 | SPEC §§11–13 | exact transform profile root授权后方可签。 |
| T3-XF-15 | SPEC §§11–13 | CAS loser产生的 staging signature不能成为 portable artifact。 |
| T3-XF-16 | SPEC §§11–13 | required plan signing failure不能 seal。 |
| T3-XF-17 | SPEC §§11–13 | profile prepare时 unavailable可冻结 disabled，ordinary save仍能成功。 |
| T3-XF-18 | SPEC §§11–13 | historical rotation后旧合法 artifact继续按历史 cut验证。 |
| T3-XF-19 | SPEC §§11–13 | transform chain gap不得通过 current diff重建。 |
| T3-XF-20 | SPEC §§11–13 | required plan recovery不能降级 disabled。 |
| T3-XF-21 | SPEC §§11–13 | disabled plan recovery不能升级 required。 |
| T3-XF-22 | SPEC §§11–13 | 同 point多个 independent inserts按真实 transaction order canonical合并。 |
| T3-XF-23 | SPEC §§11–13 | boundary insertion不能仅因 touching replacement而强制 merge。 |
| T3-XF-24 | SPEC §§11–13 | true overlapping nonzero replacements可机械 island化。 |
| T3-XF-25 | SPEC §§11–13 | event replay必须产生 exact admitted afterPin/afterSourceSha256。 |
| T3-XF-26 | SPEC §§11–13 | [5,10)→X + insert@10→Y + left point10必须得到 X\|Y。 |
| T3-XF-27 | SPEC §§11–13 | insert@5 + replace[5,10) after顺序唯一，point5因 replace starts-at-point按 conservative规则停止 mapping。 |
| T3-XF-28 | SPEC §§11–13 | required sealed decision恰有一个 outbox item keyed {changeId,ownerNodeRef}。 |
| T3-XF-29 | SPEC §§11–13 | 相同 outbox key不同 artifact/pin是 integrity conflict，不 LWW。 |
| T3-XF-30 | SPEC §§11–13 | publication recovery只能从 saved decision构造 exact outbox key。 |
| T3-XF-31 | SPEC §§11–13 | replace [5,10)→"XY" 后在 generated X/Y间输入 ! 必须 representable为同一 [5,10)→"X!Y" event。 |
| T3-XF-32 | SPEC §§11–13 | initial insert payload中继续正常打字/退格必须 fold进同一 insert event，不得无故 disabled。 |
| T3-XF-33 | SPEC §§11–13 | current edit完全落在同一 generated replacement payload内必须机械 fold。 |
| T3-XF-34 | SPEC §§11–13 | cross-anchor edit不能唯一保存 original anchor/boundary slot时必须 PortableTransformCompilation.unavailable。 |
| T3-XF-35 | SPEC §§11–13 | unavailable result必须在 planning前决定并冻结 disabled；普通 source save仍成功。 |
| T3-XF-36 | SPEC §§11–13 | recovery不得把 unavailable/disabled重新 diff为 representable。 |
| T3-XF-37 | SPEC §§11–13 | decoder-valid event的 generated payload必须按 deterministic after output span从 exact afterPin切片验证，不得内容搜索。 |
| T3-XF-38 | SPEC §§11–13 | after source中出现重复相同字节序列时 payload extraction仍由 offset唯一确定。 |
| T3-XF-39 | SPEC §§11–13 | event payload digest/length与 sliced afterPin不一致时 transform拒绝。 |
| T3-XF-40 | SPEC §§11–13 | compiler对每个输入 transaction sequence必须返回 representable或 closed unavailable reason，不能无返回/implementation自由分支。 |
| T3-SUG-01 | SPEC §§10,15 | pending+confirmed replace accept在同 ChangeId写 Document+accepted。 |
| T3-SUG-02 | SPEC §§10,15 | reject是 Annotation-only pending→rejected。 |
| T3-SUG-03 | SPEC §§10,15 | accept/reject并发只有一个 Annotation source-version winner。 |
| T3-SUG-04 | SPEC §§10,15 | accepted/rejected terminal，不允许再次 accept/reject。 |
| T3-SUG-05 | SPEC §§10,15 | explicit reanchor使 pending suggestion needs_reconfirmation。 |
| T3-SUG-06 | SPEC §§10,15 | reconfirm基于 current exact target更新 stored target basis。 |
| T3-SUG-07 | SPEC §§10,15 | targetBasis只 hash current Annotation Value中的 stored target。 |
| T3-SUG-08 | SPEC §§10,15 | insert的 zero-width stored target同样使用上述 basis规则。 |
| T3-SUG-09 | SPEC §§10,15 | 失去 target source disclosure时 Accept必须在读 transform前停止。 |
| T3-SUG-10 | SPEC §§10,15 | 无 Annotation read时 Reject不能泄漏 suggestion state。 |
| T3-SUG-11 | SPEC §§10,15 | Reject要求 annotation_read + annotation_write。 |
| T3-SUG-12 | SPEC §§10,15 | Reject不需要 Document target/source read。 |
| T3-SUG-13 | SPEC §§10,15 | V1 suggestion经 signed V1→V2无关 edit后，basis仍校验 stored V1 target、chain证明 V2 current target，允许继续 fresh accept。 |
| T3-SUG-14 | SPEC §§10,15 | mapped V2 locator hash与 stored basis不同不构成失败，因为两者不是 equality对象。 |
| T3-SUG-15 | SPEC §§10,15 | expected bytes/point测试针对 fresh current target，不针对 V1 old bytes。 |
| T3-ACTOR-01 | SPEC §§9,16 | authenticated actor snapshot由 trusted host/Core注入，caller不能填写认证 attribution。 |
| T3-ACTOR-02 | SPEC §§9,16 | 两设备同一真实人可得到不同 presentation，不声称 stable person ID。 |
| T3-ACTOR-03 | SPEC §§9,16 | same-Workspace copy保留 review原 attribution。 |
| T3-ACTOR-04 | SPEC §§9,16 | cross-Workspace trusted transfer保留 originWorkspace，不能冒充 target Workspace authenticated actor。 |
| T3-ACTOR-05 | SPEC §§9,16 | ordinary untrusted import标 imported_unverified。 |
| T3-ACTOR-06 | SPEC §§9,16 | formal same-Workspace restore exact保留 attribution。 |
| T3-ACTOR-07 | SPEC §§9,16 | 时间 snapshot永不决定 conflict winner。 |
| T3-ACTOR-08 | SPEC §§9,16 | creator/authoredAt后续 mutation必须 byte-equal immutable。 |
| T3-ACTOR-09 | SPEC §§9,16 | lastEditor/editedAt每次合法 mutation由 Core重写。 |
| T3-ACTOR-10 | SPEC §§9,16 | time producer明确叫 prepare_server/device_clock，不宣称 commit timestamp。 |
| T3-ACTOR-11 | SPEC §§9,16 | saved/replayed prepare继续使用 frozen time，不重新采样。 |
| T3-INLINE-01 | SPEC §§1–4,9 | 一个 paragraph中 strong/emphasis/link等合法 inline完整解析。 |
| T3-INLINE-02 | SPEC §§1–4,9 | soft line wraps仍属于一个 paragraph并允许。 |
| T3-INLINE-03 | SPEC §§1–4,9 | blank line产生第二 paragraph时 complete-consumption Gate拒绝整个 Annotation body。 |
| T3-INLINE-04 | SPEC §§1–4,9 | heading/list/table/delimited block等不能被 inline profile静默忽略，必须 invalid_annotation_body。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| T3-INLINE-05 | SPEC §§1–4,9 | inline STEM语法可有效；renderer缺失只影响 render。 |
| T3-INLINE-06 | SPEC §§1–4,9 | Annotation body中的 n1/r1只按普通 inline语法处理，不偷偷取得 Weftext stable reference adapter authority。 |
| T3-INLINE-07 | SPEC §§1–4,9 | source/body/render limits在 N/N+1边界 fail closed。 |
| O34-01 | SPEC §§2–3 | 同 source/environment 的未插桩与观察版运行，实际 selected-backend 最终 bytes、diagnostics、catalog、counter/attribute 结果相等。 |
| O34-02 | SPEC §§2–3 | observer 不替换 converter 返回对象、不改 bytes/encoding、不插 marker；每个实际返回值继续原样进入下一步骤。 |
| O34-03 | SPEC §§2–3 | https://pre**mid**post.example 的 quote 后字符串、真实 URL match/captures/target 与未插桩 Ruby 完全一致。不得按预想 URL 补全或清洗。 |
| O34-04 | SPEC §§2–3 | link:pre**mid**post[label] 的 target scalar 来自真实 capture；包含 converter 反馈字符时完整保留，且可追溯到 strong 返回值，不当作 label 的视觉嵌套。 |
| O34-05 | SPEC §§2–3 | Alpha *{name}* Beta 中 attribute 后续改写不被观察器遮蔽；旧 quote 参数、后续字符串版本、最终 text 的关联都存在。 |
| O34-06 | SPEC §§2–3 | 同一 scope 中重复三次相同文本或同标签链接，三个来源按 actual spans 区分，不能靠内容搜索选择某一次。 |
| O34-07 | SPEC §§2–3 | Alpha./Beta.、literal 的不同正文、pass 的不同正文，在 metadata 相同情况下仍产生不同语义观察。 |
| O34-08 | SPEC §§2–3 | concealed indexterm 虽返回空字符串仍有真实 Inline 事件、match 位置及 terms/see/see-also；不依赖 catalog 或 HTML恢复。 |
| O34-09 | SPEC §§2–3 | 显式 + 和 hardbreaks-option 两路径均保留 break 前完整文字与实际区间，最终投影不丢、不重复。 |
| O34-10 | SPEC §§2–3 | counter、counter2、set、footnote 编号/注册在观察开关前后次数和顺序相同；日志序列化不能触发第二次 content 求值。 |
| O34-11 | SPEC §§2–3 | list item、description term/body、普通 cell、a cell、title/cache-hit 的原生调用序列被保留；观察器不能多调用或擅自去重。 |
| O34-12 | SPEC §§2–3 | 原生 passthrough 提取/恢复、nested apply_subs、drop-line、特殊字符转义后的区间关联正确；observer 不增加自己的 sentinel。 |
| O34-13 | SPEC §§2–3 | target/refid/path、AttributeList 命名/位置属性的 slice、decode、normalize 来源可由真实操作图追溯；不额外调用 getter 或 scanner。 |
| O34-14 | SPEC §§2–3 | 观察日志缺失、未知必要变换或容量耗尽时不得输出“完整通过 witness”；不得修改原求值结果来补偿。 |
| O34-15 | SPEC §§2–3 | 产品 renderer 使用不同外层 DOM/CSS，不因这一点令 Core Gate失败；真正 target/text/substitution 不等仍必须失败。 |
| O34-16 | SPEC §§2–3 | 五个保留 closed types 的原正反 decoder 向量继续通过；新 Witness/6 无悬空类型，旧 marker/InlineDescriptor/Witness5 不再作为 current emitter。 |
| X34-01 | SPEC | insert@5→AA、insert@10→BB 后跨两个 generated anchors 的 replacement，即使 provenance 完整，仍先验证 boundary invariants；不满足则 unavailable，而非被旧 #11 强制 representable。 |
| X34-02 | SPEC | 同一 insertion 内继续输入/退格、同一 replacement 内修改 XY→X!Y、普通 disjoint edits 满足不变量时必须 representable；不能统一 disabled。 |
| X34-03 | SPEC | B.3 的 mixed events 将 [10,14) 映射为 [11,15)，point14 的 left/right 分别为15/17；分类只用 original coordinates。 |
| X34-04 | SPEC | [5,10)→X 加 insert@10→Y 的 left point 保持 X\|Y；replacement 从 point 开始则停止。两条 transition 连续映射时必须逐段验证。 |
| X34-05 | SPEC | CoreSourceEditPlan/2.edits 与 SourceTransformEvidence/2.edits 同为 canonical /3 events，字节不等即拒绝 seal；seal/recovery 不重新编译。 |
| X34-06 | SPEC | 当前 decoder 拒绝旧 candidate PortableEdit/2 与新 /3 混用；真实旧 saved/planned 数据仍按其原规则恢复。编译 unavailable 不阻止普通 source save，也不能在恢复时升级 emission。 |
| C34-01 | SPEC | fresh coordination 只有 FC-3.4 一份权威闭集。缺 D4 typed/control/carrier、D9 或 Index/cache 任一项，不能宣布协调完成；文末摘要只能引用该闭集。 |
| C34-02 | SPEC | v3.3 原 340 条编号与义务完整保留；本轮新增恰为 O34×16、X34×6、C34×2，共24条。所有案例仍标为设计 oracle，不伪称已运行产品测试。 |
| P35-01 | SPEC §§2–3 | 案例一严格得到 final Alice 与 strong [6,11)；Rust 保存 raw+final 或只保存 final 时投影相同。 |
| P35-02 | SPEC §§2–3 | 案例二严格保留含 <strong>mid</strong> 的 target；投影中没有中间 strong 或依赖边。 |
| P35-03 | SPEC §§2–3 | 案例三 label 的 strong 存活，而 target=dest；不得与案例二的 scalar consumption 混同。 |
| P35-04 | SPEC §§2–3 | 案例四 Alpha 只出现一次，hard_break只一次；既不丢 prefix，也不多出 soft_break。 |
| P35-05 | SPEC §§2–3 | 案例五 hidden term有确定 zero-width semantic site；没有可见 hidden text。 |
| P35-06 | SPEC §§2–3 | 案例六 footnote definition、ref atom、最终counter分别正确，正文不重复；同名 footnote 后续引用不创建第二份 definition。 |
| P35-07 | SPEC §§2–3 | 案例七相同文本的两个 strong 精确定位到不同范围，不依赖 substring search。 |
| P35-08 | SPEC §§2–3 | 案例八 counter2无 visible output但仍影响最终文本/状态；不能只比较最终 visible text而漏 finalState。 |
| P35-09 | SPEC §§2–3 | 自定义 substitutions 使 macro 先于 attribute 时，后续对 target/label 的实际改写反映在最终 scalar/flow；不能固定取最早 callback字段。 |
| P35-10 | SPEC §§2–3 | authored passthrough <strong>x</strong> 不被投影成普通 strong；normal specialchars运输层、真实 replacement、raw feedback三者不混淆。 |
| P35-11 | SPEC §§2–3 | 被后续消费/删除的纯 formatting 可以消去；实际已发生的 index/anchor/footnote/counter/catalog结果不能同时被误删。 |
| P35-12 | SPEC §§2–3 | 两个实现使用不同 raw IDs、缓存次数或 inline树形，只要最终 runs、marks、scalars、facts/state相同，canonical CSP相同。 |
| P35-13 | SPEC §§2–3 | CJK/emoji、soft break、literal换行以及 mark range按 Unicode scalar计数；不把 Ruby byte offset直接当 Core坐标。 |
| P35-14 | SPEC §§2–3 | 缺失必要观察或 Core final值不能通过投影器补跑第二 parser/HTML反推；相关完整兼容 case仍是未通过，而不是被移出范围。 |
| N36-01 | SPEC §§2–3 | Quote/verse 的 attribution/citetitle必须进入 canonical properties；正文相同、Alice→Bob或书名变化均使CSP不同。 |
| N36-02 | SPEC §§2–3 | [source,ruby] 与 [source,python] 的 language差异必被发现；实际 listing context/style/source-language继承与 fenced source路径均按同一槽位投影。 |
| N36-03 | SPEC §§2–3 | 数字位置键、命名rekey结果及 style/id/role/option物理别名只产生一个canonical槽位；原生优先级冲突使用真实结果，不在 projector改判。 |
| N36-04 | SPEC §§2–3 | converter未消费的真实 ticket=OPS-7、custom=007仍保留为 named属性，007仍是text；role与自定义roles不能碰撞。 |
| N36-05 | SPEC §§2–3 | 内部 cloaked-context、缓存、reader/column对象、临时root-option不进入CSP；真正作者同名属性不因名称前缀被删除；未知对象不得to_s。 |
| N36-06 | SPEC §§2–3 | 每个CoreBlockKind及mark/atom必须命中本profile唯一行；table列/跨度/对齐、section编号、image参数和callout关联各有正反向字段变更，不能以“未列公开字段”漏收。 |
| N36-07 | SPEC §§2–3 | 八个v3.5 expected projections原样通过；普通paragraph不因本profile多出默认subs/style/空options，footnote/link正文不重复。 |
| N36-08 | SPEC §§2–3 | Ruby无column、Rust有column但logicalFile/line/code相同时CSP诊断相等；code来自同一canonical namespace。 |
| N36-09 | SPEC §§2–3 | 无位置唯一为null；file-only/line-only保留；禁止猜位置或用第二parser补列号；同诊断两次不能去重。 |
| N36-10 | SPEC §§2–3 | links/images/includes分别在任意输入排列下规范化为相同多重集合；删除一个重复item必须使CSP不同。 |
| N36-11 | SPEC §§2–3 | Property、attributeChanges、counters按name排序唯一；冲突duplicate key拒绝而非LWW；roles、menu、列、cell、children等语言有序数组不得被全局排序。 |
| N36-12 | SPEC §§2–3 | footnote index2必须排在10前；CoreSite数值路径和index occurrence multiplicity正确；所有内层properties先规范化再生成外层CJ排序键。 |
| M37-01 | SPEC §§2–3 | Witness/7唯一current；缺新数组或用Witness6伪补空数组不得通过full-property Gate；旧nested types全部保留。 |
| M37-02 | SPEC §§2–3 | nil/absent/false/Integer/Symbol/Float/String精确区分；:chapter不得字符串化成true，Float保留原数值编码。 |
| M37-03 | SPEC §§2–3 | 普通book chapter与book special section的numbered分别记录实际true与:chapter；projector不重算初始化逻辑。 |
| M37-04 | SPEC §§2–3 | section numeral/caption必须取assign_numeral/assign_caption之后结果；constructor或initialize_section中间值不能冒充最终。 |
| M37-05 | SPEC §§2–3 | cols=1,2最终列宽为实际balance后值；C2第一次66.6666不能误选为最终66.6667。 |
| M37-06 | SPEC §§2–3 | 无显式cols的自动建列、absolute width、autowidth及最后列修正都具有真实typed evidence，不从source重算。 |
| M37-07 | SPEC §§2–3 | Header cell的@style与继承attributes style不混淆；最终halign/valign/span正确。 |
| M37-08 | SPEC §§2–3 | reinitialize返回new Cell时final row绑定new subject；旧临时Cell不重复进入CSP。 |
| M37-09 | SPEC §§2–3 | ListItem marker/checklist/id/roles/options完整；fold_first后被折叠paragraph不成为第二正文节点。 |
| M37-10 | SPEC §§2–3 | coids必须记录parse_callout_list实际赋值后的值；不能在较早parse_list_item返回处结束证据。 |
| M37-11 | SPEC §§2–3 | dlist多term、nil description、仅附属block的description都按实际tuple关系绑定，不按text猜对象。 |
| M37-12 | SPEC §§2–3 | named/positional/rekey/copy/inherit完整来源可追溯；同值真实覆盖也更新来源，不能凭值相等合并。 |
| M37-13 | SPEC §§2–3 | 显式foo-option=bar同时输出option membership与named值；改bar→baz必须改变CSP。 |
| M37-14 | SPEC §§2–3 | 显式foo-option空值仍输出named空值；%foo/options=foo/opts=foo只输出membership。 |
| M37-15 | SPEC §§2–3 | named assignment与option expansion相互覆盖时使用真实最终writer；已被原生覆盖的bar不得复活。 |
| M37-16 | SPEC §§2–3 | 作者root-option与converter temporary root-option按来源区分；临时写／删除不制造或抹除作者属性事实。 |
| M37-17 | SPEC §§2–3 | model snapshot不额外调用content/text/title/reftext/alt/parse/reinitialize/width/counter；原output与副作用次数符合已有neutrality Gate。 |
| M37-18 | SPEC §§2–3 | 同对象重复观察/缓存消费只形成一条subject链；相同内容的不同对象不合并；bind不能按字符串或位置搜索补造。 |
| M37-19 | SPEC §§2–3 | 第六节每一组PropertyProfile输入都有旧具体成员或新producer落点；删除任一required事实会令证据Gate失败，而非把合法语言移出支持范围。 |
| M37-20 | SPEC §§2–3 | missing/duplicate/conflict/非法来源链/未finalize中间值均受明确gate；完整合法证据仍产生原八个expected projections且不增加无关默认属性。 |
| M38-01 | SPEC §§2–3 | Observation ID 整个 modelPropertyObservations 中 observationId严格递增且唯一；duplicate或逆序拒绝。 |
| M38-02 | SPEC §§2–3 | Subject identity 同一实际 Ruby对象跨snapshot始终同subjectId；不同对象不得复用。old/new reinitialized Cell必须不同ID。 |
| M38-03 | SPEC §§2–3 | Snapshot identity snapshotObservationId=x 唯一解释为 snapshot.observationId=x；不存在隐式ordinal namespace。 |
| M38-04 | SPEC §§2–3 | Snapshot chain 每subject首snapshot previous=null；之后previous必须恰为该subject直接前一个snapshot，skip/branch/cross-subject/forward均拒绝。 |
| M38-05 | SPEC §§2–3 | Bind freshness bind必须引用存在、同subject、严格更早且恰为bind时刻latest snapshot；引用旧snapshot拒绝。 |
| M38-06 | SPEC §§2–3 | Cut head uniqueness heads按subjectId数值升序且每subject恰一项；duplicate head拒绝。 |
| M38-07 | SPEC §§2–3 | Root replacement treeprocessor返回新Document时，model_ready.documentSubjectId必须来自最新 document/0 bind；旧root不得被自动纳入head closure。 |
| M38-08 | SPEC §§2–3 | Exact reachable coverage header、child、List、ListItem、Column、final Cell、inner Document通过forward roles全部进入 closure；遗漏任一 live subject拒绝。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| M38-09 | SPEC §§2–3 | No stale expansion parent / cell_column backedge不得扩大closure；attribute_buffer、table_parser_context不得仅因存在snapshot进入heads。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| M38-10 | SPEC §§2–3 | Width balance Column第一次width snapshot与balance后snapshot同时存在时，cut只能引用后者。 |
| M38-11 | SPEC §§2–3 | New Cell Cell#reinitialize返回新对象且final row指向新Cell时，new Cell必须有head；old Cell不得是额外head。 |
| M38-12 | SPEC §§2–3 | Late facts late coids、assign_numeral/caption后的snapshot必须成为对应subject head；较早snapshot拒绝。 |
| M38-13 | SPEC §§2–3 | Model-ready vs evaluation-complete model_ready后发生合法semantic mutation时，evaluation_complete必须引用后续snapshot；不能复用model_ready旧head。 |
| M38-14 | SPEC §§2–3 | Temporary write temporary write后有真实restore时，evaluation_complete head指restore后的latest snapshot且作者winner保持；temporary未闭合时不得产生合法evaluation_complete cut。 |
| M38-15 | SPEC §§2–3 | Dangling / omission / extra dangling ref、forward ref、missing required live subject、extra unreachable semantic head、stale intermediate head均明确拒绝；不能降级成合法语法unsupported。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| M38-16 | SPEC §§2–3 | Supporting evidence closure head引用的previous/provenance/operation/string/inline evidence必须全部存在且类型正确；internal supporting evidence允许存在而不成为head。完整合法proof不调用parser或getter补证据。 |
| M39-01 | SPEC §§2–3 | 跨 namespace 大ID不得误拒 真实 evidence： operationId = 100 model observationId = 10 且 operation存在、类型正确、原 operation DAG合法。 ModelPropertyInput引用 operation100： PASS 不得因： 100 < 10 == false 拒绝 Witness。 |
| M39-02 | SPEC §§2–3 | 跨 namespace 小ID不得伪造时序 inlineEventId = 1 model observationId = 100 即使： 1 < 100 也不能单凭数值认为 inline event已合法先发生。 PASS条件： inline event真实存在 + 类型正确 + 原inline evidence reference contract成立 若 event1不存在或类型不对： FAIL 即使数值更小。 |
| M39-03 | SPEC §§2–3 | 同 namespace 仍严格 backward ModelPropertyInput.model_slot.observationId 必须引用同一 modelPropertyObservations namespace中的实际 snapshot，且： targetObservationId < containingModelObservationId forward、self、非snapshot target均拒绝。 本测试证明本轮没有误删 v3.8 已通过的 model reference时序规则。 |
| M39-04 | SPEC §§2–3 | Carrier entry 不是 event ID，也绝不与 model observationId 做数值比较。PASS 必须同时满足：最终 stream/index/type 静态有效；producer-conformance 证明真实 bind callback 当时 entry < targetStream.lengthAtBind；callback 当时持有 exact 同一 Ruby object。未来才补齐 carrier 不能使此前无效 bind 变为有效。 |
| M39-05 | SPEC §§2–3 | Root catalog parent top-level root： Document D1 实际执行： D1.register(:refs/footnotes/...) M37-16必须记录 catalog record： parent → D1 且在D1 live时允许该record进入 supplementary closure。 出现不存在的： owner → D1 必须 decoder FAIL。 |
| M39-06 | SPEC §§2–3 | Inner Document catalog parent 结构： root D1 → table → final Cell → cell_inner_document D2 实际 catalog register发生于： D2.register(...) 要求： catalogRecord.parent → D2 D2已经通过 cell_inner_document进入 closure，所以 record可live。 错误： catalogRecord.parent → D1 若实际receiver是D2，则： FAIL 不能因top-level root更方便而重写parent。 |
| M39-07 | SPEC §§2–3 | DocBook真实 cleanup delete 输入具有作者： root-option="bar" 固定DocBook流程真实得到： Sauthor  "bar" → Stemp    "" → Sdelete  absent/deleted 要求： evaluation_complete physical head = Sdelete 同时： CSP authored semantic property: named/root-option="bar" options=["root"] observer日志中不得出现伪造的： restore "bar" ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| M39-08 | SPEC §§2–3 | Temporary cleanup不得复活旧 authored 值 真实顺序： authored "A" → authored "B" → temporary "" → internal cleanup delete 要求： physical head = absent semantic author winner = "B" 不得得到： "A" 若 "B" 之后还有真正 authored delete： semantic winner = absent temporary cleanup不能改变真正 author write chronology。 |
| M39-09 | SPEC §§2–3 | Future carrier reject：T1 产生 snapshot S；T2 bind(stream=inlineObservations,entry=7)；T3 才 append inlineObservations[7]。即使最终数组存在 entry 7，producer-conformance 仍失败；后来补齐 carrier 不能把此前无效 run 重新判有效。 |
| M39-10 | SPEC §§2–3 | document/0 正例：top-level Document#parse 实际返回 Document D；document carrier 先发布 document/0=D；随后 bind callback 在仍持有 exact 同一 Ruby object D 时建立绑定。 |
| M39-11 | SPEC §§2–3 | document/0 反例：先 bind document/0、之后才发布 actual returned Document，或 bind 时按 title/source/text 重新寻找 D；即使最终 entry0 正确也必须失败。 |

## Actual-owner coordination — 312

| ID | 规范域 | 未来验收条件 |
|---|---|---|
| FC4R-PROD-01 | SPEC §3.4 | 两个固定2.0.26 source block 只在声明 language 上不同，例如 ruby 与 java，必须产生不同 D2DelimitedSemantics/1.language；D8/D9 无需重 parse exact source 即可区分。 |
| FC4R-PROD-02 | SPEC §3.4 | source/listing 中真实存在的 linenums/start/indent/tabsize/highlight/line-comment 必须完整进入 D2DocumentSnapshot/3；漏掉 present fact 或用 host default 补值都使 product projection 不合格。 |
| FC4R-PROD-03 | SPEC §3.4 | 正文相同的 [quote,Alice,Book] 与 [quote,Carol,Book] 必须保留不同 attribution 且 citetitle=Book；D9 renderer 只能消费产品 facts，不得回看 raw source 重新推导。 |
| FC4R-PROD-04 | SPEC §3.4 | fixed-Ruby appendix/special/numbered section 与可见标题相同的普通 section 仍必须不同；sectname/special/numbered 要保留并参与 outline/TOC/render semantics。 |
| FC4R-PROD-05 | SPEC §3.4 | terms 相同的 visible/concealed indexterm 必须可区分，see/see-also 精确保留；DocBook 类输出无需第二 parser 即可得到正确语义。 |
| FC4R-PROD-06 | SPEC §3.4 | quoted inline 的 #id、.role 或 #id.role 必须在产品 projection 中保留 id 和有序 roles；CSS/anchor/accessibility consumer 不得丢这些 fixed-Ruby facts。 |
| FC4R-PROD-07 | SPEC §3.4 | audio/video projection 必须保留真实 start/end/options 及适用 poster/width/height/preload/list/playlist/theme/lang/control facts；相同 target URL 不得把不同媒体语义合并。 |
| FC4R-PROD-08 | SPEC §3.4 | 合法 arbitrary native attribute name 可通过 D2NativeAttributeSet/1 表示，但 value 只能属于 closed D2NativeSemanticValue/1；陌生属性名本身不能使语法 invalid，自由 JSON value 必须拒绝。 |
| FC4R-PROD-09 | SPEC §3.4 | ordered/unordered/description/callout/checklist list 必须按实际 kind 保留 style/start/reversed/checklist/interactive/coids，同时保持 item order 与重复可见文本。 |
| FC4R-PROD-10 | SPEC §§3.4,8.3 | native table 的 format/grid/frame/stripes、physical columnStart/colspan/rowspan 及 TOC levels 必须供产品 consumer 直接读取；合法复杂表或 TOC 不能为了 render/export 再建 parser。 |
| FC4R-PROD-11 | SPEC §3.4 | fixed-Ruby block 的 observable style/caption/numeral/substitution list 或 positional value 只要不同，即使 body/title/named attribute map 其余相同，也必须产生不同 D2BlockCommonSemantics/1；consumer 不得回 parse source 找回这些 slot。 |
| FC4R-PROD-12 | SPEC §3.4 | native inline icon/image 与 ref/bibref anchor 必须保持不同 product semantics；icon 的 size/flip/rotate/title 与 bibliography-anchor 行为都通过 closed inline projection 保留，不允许第二 parser。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-PROD-13 | SPEC §3.4 | footnote type 只能是 null/ref/xref；callout guard 必须精确保留 scalar guard 或 two-part comment guard。unknown free-form subtype/guard structure 必须被 closed product decoder 拒绝。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-ORIGIN-01 | SPEC §3.4 | :who: Alice 后 Hello {who} 的最终 Alice 必须记录 authored definition、reference site、substitution provenance；只有全部 origin path 真正收敛同一 authored range 时才可导出 unique writable source。 |
| FC4R-ORIGIN-02 | SPEC §3.4 | generated/multi-origin semantic value 若没有唯一 authored writable range，仍可 read 与 exact-Source-save，但 structured write 必须 unavailable，不能猜一个 range。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-ORIGIN-03 | SPEC §§3.4,8.1–8.3 | Root R 含 include::child.adoc[]，child 从 V1=Alpha 变成 V2=Beta 时，即使 R bytes/document_format 不变，V1 的 D2 snapshot 对 D7/D8/D9 都必须 stale；managed include SourceObservation 必须相等。 |
| FC4R-ORIGIN-04 | SPEC §3.4 | artifact/network include 的 pin 或 network request digest 改变必须使旧 evaluation binding stale；相同 logicalPath 或最终文本不能证明 currentness。 |
| FC4R-ORIGIN-05 | SPEC §3.4 | ordered API attributes、locale/provider profile、sourceDateEpoch 或其它 processor-environment input 变化必须改变 environment digest 并失效旧 product evaluation，即使可见输出偶然相同。 |
| FC4R-ORIGIN-06 | SPEC §3.4 | 未进入该 product evaluation closed actual dependency set 的无关 global fact 不可阻塞 current consumption；scope_dependencies 只证明真实绑定的 dependencies。 |
| FC4R-LEVEL-01 | SPEC §§3.4,8.1 | fixed Ruby 合法 effective level 若大于 2^63-1，例如超大 leveloffset，必须以 D7 integer canonical decimal 表示，不能因 int64/Counter 截断或拒绝。 |
| FC4R-LEVEL-02 | SPEC §§3.4,8.1 | 普通 effective level 1–9 必须继续既有 heading/query/outline/render 正向路径，不能因为 current level type 改成 arbitrary precision 而出现兼容分支或显示变化。 |
| FC4R-LEVEL-03 | SPEC §3.4 | 处理极大但合法的 effective level 若耗尽 bounded work/memory，只能按资源合同返回 budget_exceeded/unavailable；不得改判 syntax-invalid 或 numeric_overflow。 |
| FC4R-RUN-01 | SPEC §8.2 | 显式 .separate 即使 Workspace presentation policy missing/corrupt/conflicted 也必须正常得到 Separate；该 decision 不读取或 pin policy。 |
| FC4R-RUN-02 | SPEC §8.2 | 显式 .run-in 且存在 eligible semantic-adjacent body 时，即使 Workspace policy 有多个 heads 也可 RunIn；explicit source role 不依赖选择 policy branch。 |
| FC4R-RUN-03 | SPEC §8.2 | 两个 role 都不存在且 eligible body 真正需要 implicit Workspace default 时，policy missing/corrupt/multiple heads 必须使 default-dependent presentation unavailable，不能用 ambient host/device default。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-RUN-04 | SPEC §8.2 | 没有 eligible body 时使用 closed no_eligible_body→Separate decision，不读取 Workspace policy，与 policy state 无关。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-RUN-05 | SPEC §8.2 | 两个 offline writer 都从 revision1 出发可形成不同 revision2 record；sync 必须把两个 record hash 都保留为 maximal heads，不能按 revision equality、arrival order 或 LWW 选 current。 |
| FC4R-RUN-06 | SPEC §8.2 | 对多个 presentation heads 的授权 conflict resolution 必须命名完整 expected head set，并在一个 D6 planning CAS/P decision 中产生 checked max(parent.revision)+1 的 multi-parent successor；partial head set 必须 stale/invalid。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-RUN-07 | SPEC §8.2 | saved/planned/unknown presentation-policy mutation 必须恢复 exact original head set、proposal、pins 与 P responsibility；禁止重新采样新 heads 或作为另一个 decision 重试。 |
| FC4R-RUN-08 | SPEC §8.2 | policy-head 变化只失效 D8PresentationDecision=workspace_default 的 render cache；explicit/no-body/conflict-fallback 不得获得从未消费的 policy dependency。 |
| FC4R-RUN-09 | SPEC §§8.2–8.3 | D9 plan 若用 workspace_default policy head P staging，后来 policy 变化也不能改原 plan bytes；explicit source-role plan 根本不含 policy binding，只有 fresh prepare 才消费新 policy。 |
| FC4R-RUN-10 | SPEC §8.2 | D8 presentation head-set 必须是带 epoch/revision stamp 的 owner-specific protected observation；policy mutation 在 OwnerInputBinding 中冻结这份 exact stamped head-set，任何已证明 head-set transition 都推进 stamp，不能用 numeric policy revision 选 branch。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-RUN-11 | SPEC §8.2 | committed presentation-policy record 的 activation ChangeId 必须与原 portable D6 P decision/ChangeRecord 相同，并且只有 exact receipt/EffectManifest presentation_policy_change association 验证后才可 admission。/1 current policy 只是 unique current /2 record 的机械 view，绝不单独持久化。 |
| FC4R-EXP-01 | SPEC §8.3 | asciidoc_source 在所有 renderer/template/provider registry unavailable 时仍必须有唯一 Plan3：generationPolicy=none，template/route/render bindings 全 null。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-EXP-02 | SPEC §8.3 | resource_exact 与 query_json 同样必须有唯一 policy-independent 正向编码，不得因 Office generation configuration unavailable 被禁用。 |
| FC4R-EXP-03 | SPEC §8.3 | missingPolicy=empty 只能用于 exact existing template path 且 authorized projection 确为 none；unknown path、unreadable data、type error、unavailable schema 都必须失败，不能变成 empty。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-EXP-04 | SPEC §8.3 | imageSizes 按 ResourceRef key 唯一、micrometre dimensions 为正；需要 layout choice 时必须绑定同一 authorized Resource，missing metadata/layout loss 服从 fixed Templates，不得按 filename/digest 猜。 |
| FC4R-EXP-05 | SPEC §8.3 | D9ExportTemplateBinding/1.inputIndex 必须选择恰一个 catalog template item，pin 与 item.pin byte-equal，profileId/profileVersion 必须是能解码这些 exact bytes 的 accepted decoder。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-EXP-06 | SPEC §8.3 | nonnull route 必须是连续 provider/profile chain 且 terminal profile 与 export target 匹配；target/terminal mismatch 必须失败，不能静默换 route。 |
| FC4R-EXP-07 | SPEC §8.3 | 同一 semantic Plan3 的 evidencePins 必须恰为 recursive typed PinRef union + recoveryPins；漏 template/style/route/snapshot/dependency/staged pin 或加入 unrelated pin 都失败，两个实现必须产生相同 Plan bytes。 |
| FC4R-EXP-08 | SPEC §8.3 | fresh Plan3/Receipt3 token 必须使用 d9_export_plan/3 与 d9_publication/3；inspect/confirmation/recovery 先按 token tag 再按 record version 分派；历史 Plan/Receipt1-/2 保留原 tags/bytes/pins，不能 repin 成 /3。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-EXP-09 | SPEC §8.3 | Plan3 一旦冻结 source/include/render/presentation/generation inputs 与 staged bytes，later source/include/policy/template/route change 不得让 saved/unknown publication rerender；需要 current reprepare 时必须生成 new plan 并服从原 authorization/error order。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-TABLE-01 | SPEC §8.3 | 只有一个 unique lowercase-ASCII leaf amount 的 native table 可直接暴露 COLUMN=amount、SET=native_table，不要求 ordinary Node export metadata。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-TABLE-02 | SPEC §8.3 | multi-row header 中两个 leaf 都是 Amount、上层分别 Plan/Actual 时，leaf alone ambiguous，必须用 shortest unique header suffix 确定 physical column。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-TABLE-03 | SPEC §8.3 | 两个 table 含相同 header suffix 时，下一层必须用 exact table title 消歧；不能合并 SET namespace，也不能取 first table。 |
| FC4R-TABLE-04 | SPEC §8.3 | table title 与 full header path 仍 byte-equal 时，必须使用 closed zero-based table/column occurrence qualifier；ordinal 来自 frozen product projection，不能来自 UI/memory order。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-TABLE-05 | SPEC §8.3 | CJK、RTL、combining-character 与 normalization-distinct header 按 exact Unicode scalar sequence、normalization=none、case-sensitive、保留 whitespace 比较；non-ASCII qualified name 使用固定 ASCII digest token。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-TABLE-06 | SPEC §8.3 | 空 header、重复 label、colspan/rowspan merged head 必须从 D2 table-grid facts 确定性构造 headerPath；rowspan 不得把同一 source cell 在后续 path level 重复计入。 |
| FC4R-TABLE-07 | SPEC §8.3 | later 新增同名 table/column 可以让原 short fresh selector 变 ambiguous 并要求 qualified template token；already prepared plan 继续冻结原 projection/selector/staged bytes。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-TABLE-08 | SPEC §8.3 | 一个 repeat SET 内全部 data.SET.COLUMN token 必须解析到同一 selected table/rowset；落到另一 SET/table、unknown column 或无法消歧时必须 template_invalid/ambiguous，禁止 implicit join。 |
| FC4R-WIRE-01 | SPEC §§7–8 | 合法 replica_local create_node/move_node/reorder_node/trash current wire13 request 省略 expectedAuthority、workspaceProposal、preparationBinding，并按 inherited mode matrix strict-decode 成功。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-WIRE-02 | SPEC §§7–8 | 同一 replica_local request 若出现 expectedAuthority、workspaceProposal、preparationBinding，或把这些 member 编成 JSON null，都必须 closed-decode/mode validation 失败。 |
| FC4R-WIRE-03 | SPEC §§7–8 | managed_atomic create/fork 要求 expectedAuthority=create 与 inherited required proposal；continue 要求 continue；其它 managed_atomic mode 要求 existing；descriptor/request 中存在的值必须 exact match。 |；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
| FC4R-WIRE-04 | SPEC §§7–8 | current D7/D8/D9 路径若生成 wire13 D3 request，必须选择实际可编码的 inherited optional-member shape；不能仅因 /13 schema 存在就强迫加入 mode 禁止的 member。 |
| FC4B-VAL-01 | SPEC §§9.6–9.9 | 当前交互式 Annotation 创建必须使用 D7CreateAnnotationIntent/2；调用方只能提供 destinationOwnerRef 与 AnnotationEditableProposal/1。经过正常 owner/create 授权后，Core 注入 creator/authoredAt/lastEditor/editedAt、构造 wire13 create_annotation graph，并保存一个 PortableAnnotationRecord/4。调用方提供 actor/time/trusted flag 或借 generic d3_operation 冒充可信创建都必须失败。 |
| FC4B-VAL-02 | SPEC §§9.6–9.7 | 仅把 reviewState 从 open 改为 resolved 也会改变完整 Value/4：必须只分配一个 fresh annotationRevisionToken，经既有 SourceRevisionPlan 路径推进 managed Annotation SourceVersion，byte-preserve creator/authoredAt，并由 Core 重写 lastEditor/editedAt；使用旧 token 并发 prepare 的另一编辑必须输掉 CAS。 |
| FC4B-VAL-03 | SPEC §§9.6–9.7 | 仅修改 appearance 或 labels 仍是实际 Value/4 mutation，必须使用同一个 one-token/one-managed-after 规则；不能因为 target/body/reply 未变化就复用旧 token。 |
| FC4B-VAL-04 | SPEC §9.6 | 完整 editable Annotation value 与 current state 相等时必须是真正 no-op：attribution、annotationRevisionToken、SourceVersion、H 都不变，不建立 SourceRevisionPlan，也不伪造 source_change。 |
| FC4B-VAL-05 | SPEC §§9.6–9.7 | 当前 annotation_value 的 payloadBindings、proposed pins、effects 与结果物化，都必须绑定完整 Value/4 的精确 D3-CJ/3 字节与 SHA-256；PortableAnnotationRecord envelope/ref/token 不进入 value hash，相同 digest 不能替代 identity、currentness 或 CAS。 |
| FC4B-VAL-06 | SPEC §9.7 | 同一 existing Annotation 同时修改 target@0 与 reply@1 时，target 必须保留完整 reference evidence；identity-preserving reply change 恰有一个 S/annotation_reply_change，禁止重复 reply reference result。target/reply toSource、receipt source version 与 toAnnotationRevisionToken 必须全部使用唯一 final Annotation revision。 |
| FC4B-VAL-07 | SPEC §9.7 | restore trashed Annotation A 并把 reply P→Q 时，target@0 与非空 reply@1 都以 non_live_source 为前态、resolved 为后态；旧 reply=null 时只有 reply 使用 absent，target 仍是 non_live_source。typed target/reply evidence 不能被 lifecycle-only 取代，也不能按 slot 分别增加 revision。 |
| FC4B-VAL-08 | SPEC §§9.6,16–17 | current Portable Annotation JSON corrupt/unknown 时不得产生 partial trusted Value/4 projection。有权限的 repair/backup 可以读取 exact raw portable bytes；真正 historical D2 Annotation-v2/Value3 record 只按 recorded decoder 恢复，禁止猜成或迁移成 Value/4。 |
| FC4B-D8-01 | SPEC §9.8 | 当前 d8_edit_prepare 的 Annotation intent 必须是 D8EditIntent/3，包含精确 expectedAnnotationRevisionToken、AnnotationEditableProposal/1 与 targetPolicy；PreparedEditBinding/3.intent 必须使用这个 closed type，proposedInputs 必须 pin Core 构造的完整 D3-CJ/3(Value/4)，不能只 pin editable subset。 |
| FC4B-D8-02 | SPEC §§9.6,9.8 | D8 Annotation prepare 必须拒绝 caller 提供的 actor/time/authentication/trusted-origin 字段。Core 在 current authorization/CAS 后构造 attribution 并冻结在同一 immutable plan；replay 不重新采样。 |
| FC4B-D8-03 | SPEC §§9.3,9.8 | Annotation inline Draft 若不满足唯一 R6 AnnotationInlineProfile，必须保留 exact source 与 diagnostics，同时 visual rendering 和 prepare unavailable；不得回退 plain_text，也不得调用第二 parser。 |
| FC4B-D8-04 | SPEC §9.8 | 只有 annotation_read 而没有 annotation_write 的 principal 可以取得已授权 current Value/4 的 read-only Annotation Draft/projection，但不能因为 render 或 target resolution 成功就 prepare write。 |
| FC4B-D8-05 | SPEC §§9.8,10 | pure synchronization 或 stable-locator 重新取得资格只要不改变 Value/4 bytes，就不得推进 annotationRevisionToken、SourceVersion 或 lastEditor/editedAt，也不能复活旧 PreparedEditBinding/PAB/ActionEvidence。 |
| FC4B-SUG-01 | SPEC §§9.9,15 | 当前 apply_suggestion 的 replace 分支必须重新读取 pending+confirmed Suggestion/3 与精确 target，验证 expectedText，以保存的 replacementSource 唯一生成一个 SourceTransform replace，并在同一 DecisionKey、planning CAS 与 P seal 中原子提交 target after-image 和 Annotation accepted/not_applicable state。 |
| FC4B-SUG-02 | SPEC §§9.9,15 | current apply_suggestion 的 delete 分支必须 fresh 验证 expectedText，并唯一映射到 replacement bytes 长度为0的 SourceTransform replace；target 删除与 Annotation accepted state 不能只提交一边。 |
| FC4B-SUG-03 | SPEC §§9.9,15 | 当前 apply_suggestion 的 insert 分支必须重新验证保存的零宽 point/basis 与 pointAffinity，并使用保存的 replacementSource 唯一生成一个 SourceTransform insert；调用方不能用 targetLocator 或 patch bytes 覆盖 stored suggestion。 |
| FC4B-SUG-04 | SPEC §§9.9,15 | reject_suggestion 只要 Annotation state disclosure、annotation_read/write 与 exact current Annotation token 成功，即使 target source hidden/unreadable 也必须可走正向路径；它不读写 target，只把 Value/4 改为 rejected/not_applicable，并遵守正常 revision/actor-time 规则。 |
| FC4B-SUG-05 | SPEC §§9.6,9.9,15 | accept、reject、正文编辑、reviewState 编辑、labels/appearance 编辑、reply 编辑或其它 suggestion edit 都竞争同一个 Annotation revision token；同一旧 token 的 prepare 最多一个胜者，失败方必须重新读取并 prepare。 |
| FC4B-SUG-06 | SPEC §§9.8–10,15 | mapped/candidate geometry 本身永远不授权 suggestion accept 或 edit。manual reattach 必须 fresh 选择一个 exact same-owner target，通过新的 Value/4 mutation 把 pending 改为 needs_reconfirmation，再由 reconfirmation/fresh prepare 重算 basis/expected bytes；旧 PAB/PreparedIntent 不得复活。 |
| FC4B-LIFE-01 | SPEC §§9.7,16 | fresh/mapped fresh Annotation 的初始 nonnull reply 继续用 inherited reference plan/result slot，绝不能用 structural S。只有 mode matrix 明确允许的 destination-owner existing Annotation 才可走 inherited existing-reply S；同时必须保留 target@0 evidence，reply 禁止双记。 |
| FC4B-LIFE-02 | SPEC §§9.7,16 | Node/copy_annotation copy 必须分配 fresh AnnotationRefs，通过真实 identityMap/candidate map 重写 target 和完整 same-owner reply graph，并为每个 result 物化唯一 final Value/4/revision；禁止复制旧 locator 或按文本/相同 hash 猜位置。 |
| FC4B-LIFE-03 | SPEC §§9.6–9.7,16 | ordinary import 只能按继承的 owner rules 创建 fresh Annotation identity；导入归因必须明确为 imported_unverified。D3 mode matrix 禁止时，fresh imported Annotation 不能成为 existing Annotation 的额外 same-owner structural reply mutation。 |
| FC4B-LIFE-04 | SPEC §§9.6–9.7,16 | 独立 Annotation Trash/restore 若 Value/4 byte-equal，只改变 lifecycle，不 mint Annotation revision。restore 同时发生实际 Value/reply change 时，必须走 typed preimage/S/non_live_source 规则并使用一个 final revision。永久 purge 继续 inherited D3 tombstone/no-reuse，不生成 replacement Value/token，也不能让 AnnotationRef 再次使用。 |
| FC4B-LIFE-05 | SPEC §§10,16 | portable backup/export 只能按相应披露规则保留 Annotation identity/value 与精确 resource-region/source-origin facts；它不授予当前 permission、SourceObservation、ActionEvidence 或执行权限。target SourceOrigin、Annotation identity 与 reply structure 必须始终作为三类不同事实处理。 |
| FC4B-ALIAS-01 | SPEC §9.3; SCHEMAS §7 | AnnotationInlineBody/1 是唯一 current canonical type；AsciiDocInlineBody/1 只能作为 canonicalOf=AnnotationInlineBody/1、replaces=null 的 alias。registry consumer 不得把它解释成 successor migration，也不得生成第二 wire/version。 |
| FC4A-PROD-01 | SPEC §§3.4,8 | current managed Document 含 authored level-6 section 时，必须 strict-decode 为 D2DocumentSnapshot/3，且 D2Heading/3 的 authoredLevel=6、effectiveLevel 来自 native 状态机；D8 read、D7 headings scan、D9 document rendering 必须消费同一个 occurrence，不能再由 historical document_snapshot wire2 拒绝。 |
| FC4A-PROD-02 | SPEC §§3.4,8 | 固定2.0.26合法的 open/example/sidebar/admonition/list/table/pass/STEM/native-inline 组合，即使 rich editor 没有结构控件，也必须可 parse/read 并经 exact Source 无损保存；任何 product consumer 都不得删除未知 UI 的合法 D2ProductBlock/3 或 D2ProductInline/3 arm。 |
| FC4A-PROD-03 | SPEC §3.4 | 已获授权但语法无效的 AsciiDoc 必须保留精确源文本并返回有序的 D2 诊断，同时把产品语义投影标记为不可用、提交资格标记为拒绝；不得返回不完整的语义树。若失败来自物理 source envelope，则继续使用原有 source_unavailable 错误类别，不能伪装成语法错误。 |
| FC4A-PROD-04 | SPEC §8.1 | D7 headings scan 消费 D2DocumentSnapshot/3，输出既有 owner/title/level object，其中 level 来自 D2Heading/3.effectiveLevel，并在内部保留 exact current locator；projection invalid 时必须让相关 complete scan 失败，不能静默跳过 heading。 |
| FC4A-PROD-05 | SPEC §8.1 | D7 body_text 必须递归消费完整 D2DocumentBody/3，包括 list/table、literal/source payload 与显式 inline label；它不是 exact source、不可写。若某个合法 arm 尚无定义好的 text mapping，该 adapter 必须 unavailable，不能因此缩小 D2 syntax。 |
| FC4A-PROD-06 | SPEC §§1.4,3.4 | native link/xref/image/citation 必须先经原生 grammar parse，再由 D2IdentityAdapter/1 附加 stable identity；managed include 保留自己的 source owner/range，因此 path/title/hash 不能推断 identity，include 也不能授予 root Document 对 included source 的 write authority。 |
| FC4A-D8-01 | SPEC §8.2 | current outer D8 wireVersion2 的 d8_document 必须在同一 SourceObservation 下返回 D2DocumentSnapshot/3；current valid Draft projection 保留每个合法 /3 body arm。真实 saved 的旧 D2 wire2 snapshot 只走 historical recovery。 |
| FC4A-RUN-01 | SPEC §§8.2,13.2 | 每个通过 current unseen fresh create_workspace/fork_workspace Plan4 成功 activation 的 Workspace，都必须在同一原子 bootstrap 中立即拥有恰一个 current D8WorkspacePresentationPolicy/1；它投影自 parents=[]、revision=1、defaultPresentation=separate 的 /2 record，并使用原 bootstrap ChangeId。activation 后 SetRequest 变更必须要求 policy_admin、exact expectedHeads/head-set stamp 以及 checked successor revision/conflict 规则；任何 presentation-policy transition 都不修改 Document source 或 SourceVersion。 |
| FC4A-RUN-02 | SPEC §8.2 | 显式 run-in/separate role 必须覆盖 Workspace policy；Enable/Disable/Use Default 只修改 source roles。Use Default 删除两者并恢复当前 policy；三种命令都不得写 D8WorkspacePresentationPolicy/1。 |
| FC4A-RUN-03 | SPEC §8.2 | 只改变 presentation-policy revision 时，使用该策略的 D8 渲染缓存绑定必须失效，但标题与正文的身份、源文件字节、SourceVersion 和作者写入的 roles 均保持不变；策略记录不可用时不得偷偷采用宿主环境的默认值。 |
| FC4A-RUN-04 | SPEC §8.3 | D9 plan 若在 presentation-policy revision P 下 prepare，则 Workspace 后续变成 P+1 时，旧 plan 仍使用 exact P binding 与相同 staged bytes；fresh plan 才消费 P+1。 |
| FC4A-EXP-01 | SPEC §8.3 | 已授权 target asciidoc_source 必须直接导出选中的 exact Document bytes，并要求 routeBinding/templateBinding/documentRenderBinding 都为 null；HTML/PDF/DOCX provider unavailable 不得使 source invalid，也不得阻断该 exact-source path。 |
| FC4A-EXP-02 | SPEC §8.3 | HTML 文档导出必须绑定精确的 D2DocumentSnapshot/3、格式资格、presentation-policy binding、已经接受的 route/profile 与 staged bytes；深层标题和 run-in 的呈现必须来自冻结的产品投影，不能为了导出再建立第二套 parser。 |
| FC4A-EXP-03 | SPEC §8.3 | selected accepted profile 支持时，DOCX/ODT export 必须保留 explicit effective Heading1–Heading9；更深或 target 不支持的结构只能产生显式 ExportLossReport item 或 target unavailable，不能成为 D2 source rejection。 |
| FC4A-EXP-04 | SPEC §8.3 | PDF route/provider unavailable 时必须返回 export unavailable，同时同一 current D2 snapshot 仍保持 valid 且 exact-source export 可用；accepted route 必须冻结自己的 layout/font/accessibility losses。 |
| FC4A-EXP-05 | SPEC §8.3 | 两个实现编码同一 ExportPlan/3 时，inputDomain/catalog/selection/projection、document render binding、template、route steps 与 profile versions、styles、generation policy、target、destination、proof/evidence pins、loss report、staged outputs 必须逐字节一致；任何自由结构或省略具名成员都 strict-decode 失败。 |
| FC4A-EXP-06 | SPEC §8.3 | D9ExportConfirmation/1 必须精确覆盖原计划中所有 requires_choice 与 blocking loss；确认动作不能改写 route、target、destination 或 staged bytes。inspect、confirm、publish、delivery 都继续按原顺序重新检查授权；外部发布与 unknown recovery 继续承担原有 create-only 和 saved-intent 责任。 |
| FC4A-ROUTE-01 | SPEC §8.4 | 新的 D7 Definition Transfer current submission 必须使用 D3IdentityOperationRequest/13，同时 definitionTransfers/Result9 完整 inner semantics 不变；真实 saved/planned wire12 transfer 继续使用原 decoder、fingerprint、effects 与 recovery。 |
| FC4A-ROUTE-02 | SPEC §8.4 | current D9 Import IR preparation 的新 author submission 必须绑定 D3IdentityOperationRequest/13，同时 ImportIR/Mapping/ConversionInput/file-safety/loss 规则不变；historical wire12 import job 不得原地迁移。 |
| FC4A-ROUTE-03 | SPEC §8.4 | 旧 D9 S12 中“D2 只有五级标题”的前提不再属于 current 规则：WeftextManaged 作者级 H6–H9 必须完整贯穿 D2、D7、D8、D9；某个目标格式的深度能力不足时，只能报告显式 loss/degradation 或 provider unavailable，不能强制拒绝源文档。 |
| FC4A-IMPACT-01 | SPEC §§3.4,8.4 | 已路由的 D2 implementation-impact companion 不再把开放的 native AsciiDoc、include、passthrough 或 fixed-baseline extension 一概判为不支持。current implementation 必须能解析、读取并通过 Source 无损保存每个合法 fixed-baseline construct，同时继续执行原有单一作者权威、精确源文本、授权和 invalid-repair 安全约束。 |
| FC34-FMT-01 | SPEC §§5–8 | fresh managed Document同一P安装 source + ManagedDocumentFormatBinding/1；bindingRevision=1，profile固定 exact Ruby SHA + weftext_managed/1。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34-FMT-02 | SPEC §§5–8 | 把 {kind:"document_format"} 放进 PortableComponentKey/1 必须旧decoder拒绝；不得认为这是 CP3 合法component。 |
| FC34-FMT-03 | SPEC §§5–8 | legacy/unbound source可按原授权 raw read/repair；不因 profile未绑定删除/隐藏原bytes。 |
| FC34-FMT-04 | SPEC §§5–8 | legacy/unbound source仅因包含 [weftext-attributes] 就自动切 WeftextManaged，拒绝。 |
| FC34-FMT-05 | SPEC §§5–8 | 已管理节点执行未来的 weftext_managed/1→/2 配置档迁移时，源内容保持不变；同一 Notice3/CP4 组件集合只含 document_format 迁移，sourceChanges=[]；不得生成 SourceRevisionPlan、SourceVersion 或 H(D,E)，也不得把 absent/Baseline→managed 误作迁移。 |
| FC34-FMT-06 | SPEC §§5–8 | managed profile-only migration不得增加 SourceVersion 或 H(D,E)。 |
| FC34-FMT-07 | SPEC §§5–8 | sourceVersion相同但 format stamp变化，旧 InputDescriptor/PAB/EditBinding/ExportPlan继续使用，拒绝。 |
| FC34-FMT-08 | SPEC §§5–8 | format变化后 dirty Draft bytes仍保存；projection/map/preview失效并重新qualification。 |
| FC34-FMT-09 | SPEC §§5–8 | format变化删除 dirty Draft，拒绝。 |
| FC34-FMT-10 | SPEC §§5–8 | exact source export只消费实际 source路径；不因无 semantic parse额外要求 complete Query。 |
| FC34-FMT-11 | SPEC §§5–8 | rendered export列 document_format key；format变化使未发布 ExportPlan3 reset。 |
| FC34-FMT-12 | SPEC §§5–8 | D7 narrow field为了读取 profile自动增加 full source_read，拒绝。 |
| FC34-FMT-13 | SPEC §§5–8 | narrow field用原 source_envelope/Field资格 + internal format dependency完成。 |
| FC34-FMT-14 | SPEC §§5–8 | Query的 query_scan 被当成 document_format证明，拒绝。 |
| FC34-FMT-15 | SPEC §§5–8 | Trash→restore保持 exact profile；purge同P物理删除 format component。 |
| FC34-FMT-16 | SPEC §§5–8 | derived index 没有 format row就宣称 component absent，拒绝。 |
| FC34-FMT-17 | SPEC §§5–8 | 在当前 D9 wire1 probe union里直接新增 adoc而不升 D9 owner版本，拒绝。 |
| FC34-ST-01 | SPEC §§11–13 | representable PortableTransformCompilation/1 + exact transform trust 时，planning 冻结 CoreSourceEditPlan/2，events=SourceTransformPortableEvent/3[]、emission=required；同P生成 Evidence/2、artifact及恰一个outbox item。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34-ST-02 | SPEC §§11–13 | transform profile unavailable时 TransformEmissionPlan/1=disabled{reason:"transform_profile_unavailable"}；ordinary合法source save仍可commit且无transform outbox item。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34-ST-03 | SPEC §§11–13 | source提交后再从before/after做diff并补签 artifact，拒绝。 |
| FC34-ST-04 | SPEC §§11–13 | frozen required{profile,expectedTrustRevision,expectedTrustKeyId} 在seal时 profile/revision/key不再匹配，则原 plan 不得降级为无transform save，必须按原pause/reprepare规则处理。 |
| FC34-ST-05 | SPEC §§11–13 | insertion恰在 point p，left/right affinity分别得到规范不同结果。 |
| FC34-ST-06 | SPEC §§11–13 | edit overlap target却仍声称 exact mapped，拒绝。 |
| FC34-ST-07 | SPEC §§11–13 | remote缺中间 transform artifact，用相同最终文本重建mapping，拒绝。 |
| FC34-ST-08 | SPEC §§11–13 | 只有 revision-token profile授权的同一public key验证transform artifact，拒绝。 |
| FC34-ST-09 | SPEC §§11–13 | raw public key即使后来被两个profile分别明确root-authorize，验证仍要求 exact profile+artifact domain。 |
| FC34-ST-10 | SPEC §§11–13 | CAS loser产生的staging signature进入portable outbox，拒绝。 |
| FC34-ST-11 | SPEC §§11–13 | SourceTransform 待签消息必须恰为 ASCII D6-Source-Transform-Seal/1 || NUL || D3-CJ/3(完整 artifact 只删除 signature)，中英文必须写出同一消息；seal 后 publication 失败只重发 exact canonical pinned artifact bytes，不重签。 |
| FC34-ST-12 | SPEC §§11–13 | SourceTransform artifact被写进 CP4 components，拒绝；它不是portable component。 |
| FC34-TR-01 | SPEC §13 | Bootstrap4 保留 D3 proposalId 的规范小写 UUID，并使用已闭合的 Profile4、creator、Registry、series、period 辅助类型；只有一个根，Genesis2 恰有两条声明：rev1 为 revision-token，rev2 为 transform，predecessor 链必须正确。 |
| FC34-TR-02 | SPEC §13 | genesis2只一 declaration或三 declaration，拒绝。 |
| FC34-TR-03 | SPEC §13 | 两 declaration profile正确但revision都=1，拒绝。 |
| FC34-TR-04 | SPEC §13 | 两 declaration同一个 bootstrap DecisionKey/activation ChangeId；history cut只能全进或全不进。 |
| FC34-TR-05 | SPEC §13 | bootstrap成功前先把其中一个 DomainSealKeyHandle变usable，拒绝。 |
| FC34-TR-06 | SPEC §13 | crash planned恢复同一 staged handle associations；不重生成。 |
| FC34-TR-07 | SPEC §13 | real Bundle1若以后首次授权 transform，产生 Bundle2 successor，历史 Declaration1 exact prefix保留。 |
| FC34-TR-08 | SPEC §13 | 把 old Declaration1改version2后继续称原签名有效，拒绝。 |
| FC34-TR-09 | SPEC §13 | policy conflict 同时含 CP3+Bundle1 与 CP4+Bundle2 head 时，必须按 proof/bundle version 严格分派每个 head，验证 exact address/root/history evidence 并折叠两个 profile state；合法 current policy_bundle_choice 必须有正向 resolution 路径，decoder fallback 或永久拒绝全部合法 mixed conflict 都失败。当前 source_merge/choose_source_head 保持 ownerKind/intentKind=d6_conflict_resolution/2 并使用 Input2/Plan1/Preview1，而 current policy_bundle_choice 使用 d6_conflict_resolution/3 与 Input3/Plan2/Preview2；三臂都使用 current outer InputDescriptor3/DependencyProof3/PreparedIntent3 family，任何 arm/version mismatch 都失败。 |
| FC34-TR-10 | SPEC §13 | 未选分支的 transform compromise 仍必须以原 activation cut 保留在 effective Carry1/Carry2 union；选择其他分支不能复活 compromised key。合法的受影响 transform pair 可通过 FreshDomainAuthorizationSpec2 产生 fresh transform Outcome2/PoP2。当前策略准备还必须将 OwnerInputBinding2.canonicalDescriptorBytes 精确绑定为 D3-CJ/3(Input3)；pinRefs 必须恰为各分支的 ChangeRecord/CP/Bundle pins、所选/结果 bundle pins，以及每个保留 Carry 的 origin/carrier ChangeRecord/CP/Bundle pin 的排序去重并集；Preview2 绑定完整 Plan2 与 branchEvidenceDigest，PreparedIntent3.pinDirectory 还必须保留 DependencyProof evidence、精确 preview 与 installation pins。遗漏任一 losing-branch Carry hop、result pin 或 preview pin 都使 planned/recovery closure 失败。 |
| FC34-TR-11 | SPEC §13 | current d6_replica_register_prepare 保持 wire2 request，mint 一个 ReplicaEpoch，并按 revision-token 后 source-transform 固定顺序恰生成两个 staged Handle2；同一 DecisionKey 下追加两条 Declaration2，且两把 handle 只有经同一 CP4/ChangeRecord 的一个 P seal 才能一起 usable。D6 Storage §9.1 current consumer 必须接收同一个 one-ReplicaEpoch/two-staged-Handle2/two-Declaration2 transition，在 receiver admission 验证两条 authorization，并同时 admission 两个 profile，不能部分 admission。 |
| FC34-TR-12 | SPEC §13 | generic dual-profile trust management 不能替代 replica_register authority，也不能产生只 admission 一个 profile 的 replica；planned replica recovery 必须恢复原 staged pair/declarations/component pins，绝不重新生成 key。historical single-profile registration 保持 recorded decoder/bytes；current retire 阻止两个 profile 的 future signing，但不能变成 execution takeover。 |
| FC34-TR-13 | SPEC §13 | D10 PublisherIdentity/package signature被拿作 SourceTransform trust，拒绝。 |
| FC34-H-01 | SPEC §17 | old D3 wire12 saved decision在新runtime重放原receipt，不要求 InputDescriptor3。 |
| FC34-H-02 | SPEC §17 | old PreparedActionBinding3 planned继续原恢复，不原地升级PAB4。 |
| FC34-H-03 | SPEC §17 | CP3带 document_format key，旧decoder必须拒绝。 |
| FC34-H-04 | SPEC §17 | CP4被改标为CP3以兼容旧consumer，拒绝。 |
| FC34-H-05 | SPEC §17 | D7 old effect transport继续解 Plan1/Plan3历史；current Plan4只走 EffectManifest3/EffectBytes3。 |
| FC34-H-06 | SPEC §17 | SourceTransform缺失时复活旧 Draft selector/PAB/ActionEvidence，拒绝。 |
| FC34-H-07 | SPEC §17 | current format或transform证据重新取得后，只能建立新的 current qualification；不修改旧 saved record。 |
| FC34-FMT-18 | SPEC §§5–8 | BaselineOnly ordinary分析成功但portable component不存在；不得生成 ManagedDocumentFormatBinding(profile=baseline)。 |
| FC34-FMT-19 | SPEC §§5–8 | DependencyKey/3 canonical rank必须 source,document_format,lifecycle,...,execution_resource；把document_format append至末尾必须失败。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34-FMT-20 | SPEC §§5–8 | PortableComponentKey/2必须 document,document_format,resource,...,conflict；把document_format排在conflict之后必须失败。 |
| FC34-ST-13 | SPEC §§11–13 | current plan/evidence出现 SourceTransformEdit/1 或 SourceTransformPortableEdit/2 必须失败。 |
| FC34-ST-14 | SPEC §§11–13 | required plan 的 D3-CJ/3(plan.edits) 与 D3-CJ/3(evidence.edits) 不相等时必须拒绝seal。 |
| FC34-ST-15 | SPEC §§11–13 | Event3 先验证 UTF-8 boundary、replace/insert range、removedByteLength=end-start 与 exact before slice 的 removedSha256；SourceTransformEvidence2.beforeSourceSha256 还必须独立等于 evidence.before 所选完整精确 before source 的 SHA-256。即使所有 Event slice 与 replay 都匹配，whole-before hash 错误也必须失败。 |
| FC34-ST-16 | SPEC §§11–13 | generatedOutputSpan 只能由 normative before/after replay cursor 与 delta(R)=replacementByteLength-(endByte-startByte) 派生；receiver 必须重新取得历史 exact-before bytes，不能用 current-file bytes、slice hash 或 SourceVersion 字段相等替代。 |
| FC34-ST-17 | SPEC §§11–13 | replace [5,10)->X + insert@10->Y 必须保留两个event及canonical boundary order，保持 X\|Y；合并为导致 XY\| 的单replacement失败。 |
| FC34-ST-18 | SPEC §§11–13 | required seal必须同时重验profile、expectedTrustRevision、expectedTrustKeyId与usable handle；只验当前存在某transform key失败。 |
| FC34-ST-19 | SPEC §§11–13 | 跨generated anchors无法保持boundary slot时返回 PortableTransformCompilation.unavailable；ordinary save仍可继续，但不得制造空events CoreSourceEditPlan或transform artifact。 |
| FC34B-D10-01 | SPEC §14 | 当前 unseen Workspace control prepare 使用 D10WorkspaceReadDependencies/2 + DependencyProof/3；解析 managed Document 时包含 document_format。真实 saved/planned 历史记录必须先按其 recorded owner version 恢复。 |
| FC34B-D10-02 | SPEC §14 | 用 D10WorkspaceReadDependencies/1 + proof2 建立同一个 current unseen prepare 必须在 current owner gate 失败；这不能反向否定真实历史 saved/planned dependency record。 |
| FC34B-D10-03 | SPEC §14 | 对当前尚未建立决议的 control operation，依赖链固定为 D10WorkspaceReadDependencies/2 → ControlDependencies/3 → D10ControlInput/2 → ControlPrepareBinding/3，并与 D6 Descriptor3/Proof3 inputs 同 cut 逐项一致；current external consent 在 Binding3 内保留原 ExternalConfirmationRequirement/Record 语义，已证明的 saved Binding2 继续精确历史恢复。 |
| FC34B-D10-04 | SPEC §14 | ControlDependencies3中workspaceReads被deployment-only body偷偷设some；FAIL。 |
| FC34B-D10-05 | SPEC §14 | recurrence source V 不变但 document_format M1→M2 时，current Storage producer 必须记录 ScheduleContinuityInvalidation/2 的 binding_changed；即使 recurrence output 恰好相等，也不能把 Witness2 连续推进。 |
| FC34B-D10-06 | SPEC §14 | source/profile bytes 相等但 document_format proof continuity 出现 gap 时，current producer 必须记录 ScheduleContinuityInvalidation/2 的 gap 并保留最后合法 checkpoint；不能伪造 Evidence2 或 continuous Step2。 |
| FC34B-D10-07 | SPEC §14 | 没有 format/rule discontinuity，且完整 current ChangeRecord1 + Notice3 + CP4 chain 证明正文变化真正无关时，ScheduleContinuityStep2 才可原子推进 Witness2；retained step pin 必须使用 artifact/recovery，并覆盖 UTF8 D6-Schedule-Step/2 + NUL + 完整 canonical Step2 bytes，更新后的 witness pin 使用 D6-Schedule-Continuity/2，byteLength/SHA-256 覆盖完整前缀 bytes。 |
| FC34B-D10-08 | SPEC §14 | current ScheduleContinuityStep2 若尝试把新的 current portable transition 解成 Notice2/CP3 必须失败。schedule continuity decoder 必须精确分派：Step2/Witness2 使用 /1 artifact domain 必须失败，historical Step1/Witness1 使用 /2 domain 也必须失败，unknown/mismatched domain 不得 fallback；retained historical transition 只能按其真实 /1 bytes 与 decoder 继续。 |
| FC34B-D10-09 | SPEC §14 | 旧 Subscription1 继续是合法 historical retention owner；只有完整 retained history 证明期间没有 format/rule/business discontinuity 并建立 current Evidence2/Proof3 cut，才可显式 continue 为同 generation 的 Subscription2。已有 Witness1/Step1/Invalidation1 pins 保持精确 /1 bytes，只有之后新产生的 Witness2/Step2/Invalidation2 使用 /2 domain；真实 bridge 可以保留 version-mixed exact typed chain，但绝不 repin。 |
| FC34B-D10-10 | SPEC §14 | old Subscription1 的 retained history 若出现 format/rule/business discontinuity，就不能继续原 generation，必须 explicit replace；后台 migration 或 reset invalid generation 均失败。 |
| FC34B-D10-11 | SPEC §14 | fresh automatic author preparation 必须在返回前原子保存 D10AuthorPreparationLink2、精确 PAB4 与所需 Effect3 evidence/pins，再从完整 current preparation 构造 ApprovalUse2；已证明的 saved/planned Link1/PAB3/Effect2/ApprovalUse1 responsibility 保留原 decoder、bytes、pins、request 与 OperationId。 |
| FC34B-D10-12 | SPEC §14 | fresh interactive author responsibility 必须声明 owner contract 选定的 exact current PAB4 或 EditBinding3；current type 不匹配则失败。真实 saved/planned PAB3/EditBinding2 属于历史恢复，不能被升级。 |
| FC34B-TR-01 | SPEC §13 | Declaration1用原 CP3 activation；PASS。 |
| FC34B-TR-02 | SPEC §13 | Declaration2用 exact CP4 + Bundle2 policy after-image首次追加；PASS。 |
| FC34B-TR-03 | SPEC §13 | Declaration2只找到同DecisionKey CP3，没有CP4；FAIL。 |
| FC34B-TR-04 | SPEC §13 | mixed /1,/2 history分别按CP3/CP4 activation fold；PASS。 |
| FC34B-TR-05 | SPEC §13 | Carry2 factId 使用 D6-Trust-Compromise-Fact/2 与精确九字段 body；direct revoke compromise 映射 trustKeyId，direct rotate compromise 映射 oldTrustKeyId，originDeclarationDigest 哈希完整 Declaration2，originActivationChangeId 经原 CP4/ChangeRecord 重派生，不能使用 resolver 时间。TrustConflictCarryValidationEvidence 还必须 pin 原 Declaration2 activation 的 ChangeRecord1/CP4/Bundle2 以及每个实际经过的 resolver carrier；resolver 时间不能替代 origin cut。 |
| FC34B-TR-06 | SPEC §13 | 当前 wireVersion3 的 transform profile 轮换必须使用 mode=ordinary，并验证 PoP、rotate、root 的精确签名域和签名体以及真实根授权 producer；K1 签署 transform 后普通轮换到 K2，远端仍以 producing CP4.frontierBefore 验证 K1。 |
| FC34B-TR-07 | SPEC §13 | transform KT1 分叉为 ordinary KT2 与 compromise KT3 时，选择 ordinary branch 仍必须在 canonical recursive union 中保留 losing branch 的原 KT1 compromise fact 及其精确逐 head origin/carrier validation evidence；同 factId 不同 bytes 为 integrity_conflict，安全 selected key 或合法 fresh transform recovery 均按完整 Outcome2 规则处理。planned recovery 必须恢复冻结的 Input3/Plan2/Preview2/pins，不能从 current history 重算。 |
| FC34B-TR-08 | SPEC §13 | 普通transform receiver使用current Bundle而非producing CP4.frontierBefore；FAIL。 |
| FC34B-CONT-01 | SPEC §13 | Bundle2中revision current K仍由Declaration1授权且Handle1 usable；revision-token new signing PASS，transform signing FAIL。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34B-CONT-02 | SPEC §13 | 当前未见过的 add/rotate/revoke 使用已闭合的 wireVersion3 双 profile prepare family，caller 不携带 key material；Declaration2 将 revision profile 普通轮换到 K2 后，只有 Handle2(K2) 可新签名，而已保存/已规划的 wireVersion2 record 继续原 recovery。 |
| FC34B-CONT-03 | SPEC §13 | old states (revision=current, transform=none) → revoke revision；authorize B2 revision；authorize B2 transform；同一CP4。PASS。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34B-CONT-04 | SPEC §13 | old states (revision=none, transform=current) → revoke transform；authorize两个B2 profiles。PASS。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34B-CONT-05 | SPEC §13 | 任何profile conflicted/gapped/unproved，却先让clean profile的B2 handle usable；FAIL。 |
| FC34B-CONT-06 | SPEC §13 | 完整continuation declarations不是固定rank顺序或存在可观察中间prefix；FAIL。 |
| FC34B-HOLDER-01 | SPEC §§7–8 | wire13 conflict resolution保存 D3ResolutionInputUse/2 + InputDescriptor3；PASS。 |
| FC34B-HOLDER-02 | SPEC §§7–8 | wire13仍保存InputDescriptor2 guard；FAIL。 |
| FC34B-HOLDER-03 | SPEC §§7–8 | D4 strong relation operation source不变而format stamp变；old proof不能继续，必须stale/reprepare。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34B-HOLDER-04 | SPEC §§7–8 | D5 native table edit同样消费source+format；format变而旧table locator/sourceVersion碰巧相同，仍stale。 |
| FC34B-HOLDER-05 | SPEC §§7–8 | D4/D5为了配 /3 而机械升级RelationReadContext、table locator、occurrenceKey或numeric sourceRevision；FAIL。 |
| FC34B-CR-01 | SPEC §6.3 | FC current portable decision 同 P 冻结 ChangeRecord1、Notice3、CP4；CP4 component keys 与 Notice3 canonical key 集合/顺序严格相等，每个 after 都是实际 installed/sealed owner-versioned image，且 DecisionKey/ChangeId/frontiers/digests 一致。 |
| FC34B-CR-02 | SPEC §6.3 | ChangeRecord1指向正确CP4但错误Notice3 digest；FAIL。 |
| FC34B-CR-03 | SPEC §6.3 | CP4 frontierBefore/After 与 ChangeRecord 不一致则 FAIL；frontierAfter 必须恰为 frontierBefore 增加本 ChangeId，其他 domain 不回退，scope_dependencies 必须有完整连续 ChangeRecord/completion chain 与 retained unrelatedness proof。 |
| FC34B-CR-04 | SPEC §6.3 | pre-FC历史bytes没有已证实decoder，却被运行时按ChangeRecord1解释；FAIL。 |
| FC34B-CR-05 | SPEC §6.3 | receiver admission 必须验证 exact Notice3/CP4 bytes、每个实际 component byte 与 owner version、完整 sourceChanges/production SourceVersions、restored-arm exclusions 与连续 chain；未知历史 decoder 或缺 proof 时 strong path 按 gap/proof_unavailable 停止，ordinary source read/save 继续原资格。 |
| FC34D-MIX-01 | SPEC §14 | ControlPrepareBinding/1(K1)+ControlPrepareBinding/3(K2)，K1!=K2，同时存在于同一 D10ExecutionClaims/2.prepareBindings 并进入 Inventory2/Record3；/1 与 /3 各按 exact decoder，按 StableControlKey canonical order，原 bytes/pins分别保留。 |
| FC34D-MIX-02 | SPEC §14 | ControlPrepareBinding/1(K)+ControlPrepareBinding/3(K) 出现在同一 Claims2；即使 canonicalIntentBytes/originalCommitRequest相同也必须作为重复 StableControlKey责任拒绝，不得LWW、优先新版本或静默丢旧版本。 |
| FC34C-MIX-01 | SPEC §14 | 一个 ControlDependencies3 同时包含 old stop Pin1 与 new automation Pin2；同cut完整匹配各自binding。PASS。 |
| FC34C-MIX-02 | SPEC §14 | external_effect current record仍为 Image1/Pin1，并可被 current Dependencies3/Inventory2完整携带。PASS。 ；其中英文名称均为本条引用的协议标识或固定字面量，不改变本条中文条件。 |
| FC34C-MIX-03 | SPEC §14 | 尝试用 Image2 表示 stop/external_effect/reservation；FAIL closed decode。 |
| FC34C-MIX-04 | SPEC §14 | 同一 EffectPlan2 可表示 Automation before=Image1+Subscription1、after=Image2+Subscription2，且不重编码 before；若 scheduling owner 的 continue 合法，旧 subscription 可以保持同一 generation，不能一律强制 replace。 |
| FC34C-MIX-05 | SPEC §14 | Image2 automation/run → Image1 downgrade；FAIL。 |
| FC34C-MIX-06 | SPEC §14 | Pin1 认证精确的 D10-Control-Record/1 前缀 Image1 payload，Pin2 认证 /2 前缀 Image2 payload，artifact byteLength/SHA-256 必须匹配；schema tag 与 pin domain 不一致或把 Image1 repin 成 Pin2 都失败。 |
| FC34C-MIX-07 | SPEC §14 | mixed range 的 rank 固定 records=0、cost_lineage=1、occurrences=2，并按跨版本 logical identity 唯一；Range2 occurrence array 可含不同 key 的旧 K1 与新 K2，内部按 AutomationOccurrenceKey 排序。 |
| FC34C-MIX-08 | SPEC §14 | Occurrence1(K) + Occurrence2(K) 同key；FAIL。 |
| FC34C-MIX-09 | SPEC §14 | record pin 的同一 cut identity 为 binding.ref、binding.revision、usageRevision，跨版本最多一项；byte-equal 重复项也拒绝，bytes 不同则为 integrity_conflict。 |
| FC34C-MIX-10 | SPEC §14 | ControlDependencies3 若 range/barrier 来自 cut B，而 record pin、binding、usage revision、stop pin 或 Workspace evidence 来自 cut A，则同 cut 校验失败。全部值必须来自同一个真实 Authority Store barrier。 |
| FC34C-EXEC-01 | SPEC §14 | Claims2 可同时包含 Binding1/2/3、Step1/2、Subscription1/2、Occurrence1/2、Pin1/2，只要 semantic key 不同；prepareBindings 在三代 Binding 间统一按 inner StableControlKey 全局唯一。 |
| FC34C-EXEC-02 | SPEC §14 | MoneyResponsibility2 可同时需要旧 Pin1、新 Pin2 与 Range1/Range2；reservation 按完整 Binding 唯一，layer 按 CostLayerKey，evidence pin 按 pinToken，mixed pin/range 使用各自跨版本 canonical order。 |
| FC34C-EXEC-03 | SPEC §14 | Inventory2 必须完整携带 mixed ApprovalUse、Claims2、Money2、ExternalResponsibility1、StopResponsibility1，并按各自跨版本 logical identity 排序；快照来自一个真实 barrier，stopCapacity 是同一 safety store 的实际 StopCapacity1。 |
| FC34C-EXEC-04 | SPEC §14 | ApprovalUse1 与 ApprovalUse2 若拥有同一 DecisionKey，就是重复责任并失败；schema version 不能拆分 logical identity。 |
| FC34C-EXEC-05 | SPEC §14 | old Subscription1 与 new Subscription2 可在同一 Automation 的不同 generation 共存；相同 Automation+generation 必须失败。合法 configure continue 可以保留旧 generation。 |
| FC34C-EXEC-06 | SPEC §14 | Inventory2 若遗漏 started/outcome_unknown external responsibility、已 stopped latch、仍被引用的 completed attempt、旧 recovery pin 或实际 StopCapacity，均不完整。 |
| FC34C-D6-01 | SPEC §14 | Record3 的 Proof2 inventoryPin 必须严格解码为 D6-Execution-Inventory/2。workspaceRef 以及 approvalUses、claims、moneyLineage、externalUnknowns、stopState 五类 payload 分别与 Inventory2 逐字节相等；Inventory2.storeIncarnation 等于 Proof2.storeIncarnation，stopCapacity 等于同一 barrier 下 safety store 的实际值。 |
| FC34C-D6-02 | SPEC §14 | execution custody 只有在一个完整 mixed Inventory2/barrier 与真实 old-holder fencing 下才能由 Record2 交给 Record3；保留全部旧责任，revision=old+1，executionDomainId 与 store continuity 不变，shared StopCapacity 不能复制到第二个 active store。 |
| FC34C-D6-03 | SPEC §14 | 没有 handoff、checkpoint 或真实 responsibility mutation 时，仅因支持 current FC schema 不得后台迁移 Record2/Proof1/Inventory1，也不得把历史 bytes repin 成 Record3/Proof2/Inventory2。 |
| FC34C-D6-04 | SPEC §14 | handoff 若拿不到任一旧 Pin1/PAB3/Binding1/Subscription1/stop/external responsibility 的原 decoder、bytes、pins，或无法证明 authoritative store/capacity barrier，必须暂停或 unavailable，不能构造部分 Inventory2；无关 ordinary source operation 继续原资格。 |

| FC4C-SUG-01 | SPEC §9.10 | pending+confirmed suggestion 通过普通 D8 ordinary_edit 直接提交 accepted/not_applicable；必须在 transition gate 拒绝，且不得产生 target write、Value4 revision 或 P。 |
| FC4C-SUG-02 | SPEC §9.10 | pending suggestion 通过普通 D8 ordinary_edit 直接提交 rejected/not_applicable；FAIL，只有 reject_suggestion 可产生 rejected。 |
| FC4C-SUG-03 | SPEC §9.10 | 已接受或已拒绝的终态 Suggestion 经普通编辑被改回 pending/confirmed；必须失败，即使 expected token 与 current Value4 都正确，终态也不得重开。 |
| FC4C-SUG-04 | SPEC §9.10 | interactive create_annotation 的首版 proposal 企图制造 accepted/rejected；FAIL；合法 suggestion 首版只能由 Core 构造 pending。 |
| FC4C-SUG-05 | SPEC §9.10 | caller proposal 夹带 confirmed、targetBasis、expectedText 或 pointAffinity 不能进入 current wire；合法首版 pending+confirmed 只有在 Core fresh exact target read 后重算全部 target evidence 才成立。 |
| FC4C-SUG-06 | SPEC §9.10 | manual reattach 选择新的 exact same-owner target 后，after 必须 pending+needs_reconfirmation；即使 caller/UI 声称 confirmed 也不能同次升级。 |
| FC4C-SUG-07 | SPEC §9.10 | 显式 annotation_reconfirm_suggestion 处理 pending+needs_reconfirmation 时，Core 必须重新读取当前目标并重算 basis、expected 与 point 后才可得到 pending+confirmed；若目标不可见，必须在泄露目标字节前返回 not_visible，且零变更。 |
| FC4C-SUG-08 | SPEC §9.10 | terminal Annotation 可修改 body/appearance/labels/review/reply 等仍合法字段，但 Suggestion/3 必须 byte-equal；改 kind/replacement/evidence 或 reopen terminal 均失败。 |
| FC4C-SUG-09 | SPEC §§9.9–9.10 | 处于 pending+needs_reconfirmation 的建议在目标隐藏或不可用时仍可拒绝：只核 Annotation 的披露、读取、写入与 token，得到 rejected/not_applicable；不得读取目标。 |
| FC4C-SUG-10 | SPEC §§9.7,9.10,9.12 | copy/import/backup 恢复的 terminal Suggestion 可保留历史展示状态与 attribution，但没有当前 Workspace apply receipt/target source change 就不证明目标曾被修改；fresh ordinary import 使用 imported_unverified。 |
| FC4C-CARRIER-01 | SPEC §9.11 | replace apply 的当前 Annotation 后像在 PAB4/D7ActionInput3 中必须使用 D7ProposedInput/3 与 d3_annotation_value4，并固定完整 Value4；若用 /2 与 d3_annotation_value3 承载相同字节必须失败。 |
| FC4C-CARRIER-02 | SPEC §9.11 | delete apply 的 Annotation accepted 后像同样必须使用当前 /3 的具体 Value4 carrier，并与目标的零字节 replacement source change 进入同一个 P；不得回退到历史 carrier。 |
| FC4C-CARRIER-03 | SPEC §9.11 | insert apply 的 Annotation accepted 后像同样必须使用当前 /3 的具体 Value4 carrier；保存 pointAffinity 的目标变换与 Value4 pin 必须属于同一次准备，不能分开形成决策。 |
| FC4C-CARRIER-04 | SPEC §9.11 | 真实 historical PAB3/Input2 的 d3_annotation_value3 按原 decoder/bytes/pins 恢复；把 old record 升成 /3 或把 current Value4 降装 /2 都失败。 |
| FC4C-CARRIER-05 | SPEC §9.11 | 合法的当前 D3 symbolic branch 继续使用 d3_symbolic_result9；OwnerInputBinding.pinRefs 必须精确覆盖 /3 的 proposed pins 与真实受保护证据，具体 Value4 与 symbolic Result9 不能互换标签。 |
| FC4C-READ-01 | SPEC §9.11 | Value4 body exact source 为 `*Alice*` 时，D8 read 返回 exactSource=`*Alice*` 且 semanticText=`Alice`；D7 annotation_body 唯一输出 `Alice`。 |
| FC4C-READ-02 | SPEC §9.11 | R6 正文无效时，D8 read 必须保留精确源文与 diagnostics，并令 semanticText=null；D7 annotation_body 在授权与当前性验证后返回 source_unavailable，禁止退回 plain_text 或自行去除标记。 |
| FC4C-READ-03 | SPEC §9.11 | d8_annotation_read 必须返回完整 Value4、annotationRevisionToken、SourceObservation 以及 creator/authoredAt/lastEditor/editedAt；若只返回 AnnotationEditableValue/1，则缺少当前记录与归因信息，属于不完整读取。 |
| FC4C-READ-04 | SPEC §9.11 | 只有 annotation_read 的 principal 可获得完整 readonly read + readonly Draft projection，但不能进入 D8EditPrepareRequest/3；render 成功不授予 write。 |
| FC4C-READ-05 | SPEC §9.11 | 具有 annotation_write 时，draft_open 的基础 token/Observation 必须与 read 精确相等；prepare 后若 aggregate、record、token 或 dependency barrier 任一变化，最终屏障必须判为 stale 并重新准备，即使 Value hash 相等。 |
| FC4C-READ-06 | SPEC §9.11 | Annotation 本体可读而 target hidden/unavailable 时仍可读完整 Annotation attribution/body；targetResolution=unavailable 且不泄露 target source bytes/expectedText。 |
| FC4C-READ-07 | SPEC §9.11 | body=null 的 annotation_body 唯一语义 text 为 empty string；不得把 absent body 解释成历史 D2 plain_text 或调用另一 parser。 |
| FC4C-STORE-01 | SPEC §9.12 | 同一 aggregate 有 A/B 两条记录，只改 A 时 after 必须完整保留 B 的 logical record；B token/Value4/managed SourceVersion/H 均不变。 |
| FC4C-STORE-02 | SPEC §9.12 | 两个并发 plan 分别改同 sidecar 的 A/B：最多一个 strict file CAS 先成功；失败者 fresh re-read 后重建包含胜者 change 的 aggregate，禁止 LWW 覆盖。 |
| FC4C-STORE-03 | SPEC §9.12 | aggregate records 出现重复 AnnotationRef，即使 bytes 相同也整文件 strict-decode FAIL；不同 bytes 是 integrity conflict，不能 last-one-wins。 |
| FC4C-STORE-04 | SPEC §9.12 | record owner 与 envelope owner 不同、replyTo 跨 owner 或完整 reply graph 成环，均整 aggregate FAIL，不能只丢坏记录继续。 |
| FC4C-STORE-05 | SPEC §9.12 | JSON 被截断、出现重复键、未知格式/版本或成员畸形时，整份 aggregate 都不得部分信任；正常当前读取必须返回 unavailable/conflict，获授权的原始修复或备份只能取得精确原始字节。 |
| FC4C-STORE-06 | SPEC §9.12 | Node/adoc 的协调重命名只更新 FileBinding 与物理 observation；ownerNodeRef、AnnotationRef、Value4 和 revision token 均保持不变，随后通过重新读取 aggregate 恢复当前观察。 |
| FC4C-STORE-07 | SPEC §9.12 | 显式备份保存精确 aggregate 字节与 Frontier，但不携带当前权限、PAB 或 ActionEvidence；恢复时仍须完整严格解码，并验证 owner、reply、history 以及原 D6 admission 规则。 |
| FC4C-STORE-08 | SPEC §9.12 | Node copy/import 用真实 identityMap/candidate-map 生成 fresh AnnotationRefs、改写 target/reply 并生成新 owner aggregate；terminal display 可保留但不能制造 destination apply receipt。 |
| FC4C-STORE-09 | SPEC §9.12 | 纯 Trash/restore 在 Value4 字节不变时保留 token，且不产生 source revision；purge 删除对应 record，最后一条记录被清除后，规范物理状态必须是 sidecar absent。 |
| FC4C-STORE-10 | SPEC §9.12 | 同一 DecisionKey 修改一个 Node 的两个 Annotation 时 Notice3/CP4 各有两个 logical annotation components/source changes，但 InstallationPlan 只有一个完整 sidecar after 与一个 physical install。 |
| FC4C-STORE-11 | SPEC §9.12 | A 改动导致 sidecar FileObjectBinding 更新后，B 的旧 aggregate-backed current observation stale；fresh read 可把未改 B 重新绑定到新物理 observation，不分配新 token/SourceVersion/H。 |
| FC4C-STORE-12 | SPEC §9.12 | 已验证的 ChangeRecord/Notice/CP 连续链可以接纳规范 aggregate；直接外部 JSON、mtime、provider 的 latest 声明或相同 hash 都不能当作可信转换，显式 import/admission 仍必须执行 imported_unverified 归因与生命周期 gate。 |

| FC4D-PROD-01 | SPEC §3.4 | 每个 D2NativeAttributeEntry/1 都携自己的 sourceOrigins。unique authored attribute 且当前 source-write 已授权时，只能用该 entry 的 exact gate；node-level sourceOrigins 不能替代。 |
| FC4D-PROD-02 | SPEC §3.4 | [source,ruby] 加正文时，block style=source、semantics.language=ruby 与 body text 必须分别绑定不同 exact authored span；修改一个 slot 不能落到另外两个。 |
| FC4D-PROD-03 | SPEC §3.4 | [#foo.red] 包住 quoted content 时，id=foo、每个 role element 与 child text 必须有不同且机械配对的 origin；从 coarse node range 或第二次 parse 重建任一映射都失败。 |
| FC4D-PROD-04 | SPEC §3.4 | 每个具名 common/per-kind/block/inline semantic member 都有 SCHEMAS 要求的 closed origins/slotOrigins；所有 sequence-origin array 与对应 value array 长度和顺序完全相同，缺失、多余、错位或跨 slot 复用 origin 都 strict-decode FAIL。 |
| FC4D-PROD-05 | SPEC §3.4 | structured slot 的 writableSource=unique、exact SourceOwner 可写且完整 evaluation binding current 时，必须存在合法 structured-edit 正向路径；managed-include slot 需要该 include owner 的写权限，不能继承 root 权限。 |
| FC4D-PROD-06 | SPEC §3.4 | generated、multi_origin、ambiguous、non_author_source 或其它不可写 provenance 仍可读但 structured-readonly；实现既不能猜 physical edit，也不能为省事把全部 unique authored 值一律 readonly。 |
| FC4D-PROD-07 | SPEC §3.4 | prepare 后 exact root/include SourceObservation、environment digest 或任一已消费 dependency 改变时，field-level write 在 final barrier 必须 stale，即使 slot text 与 root hash 未变。因此旧写入必须拒绝，不能沿用过期证据。 |
| FC4D-PROD-08 | SPEC §3.4 | D2DocumentMetadata/3 按 fixed-2.0.26 Document#authors 顺序保留每位 author 的 name/firstname/middlename/lastname/initials/email；D9/UI 禁止通过拆 fullname 或重 parse source 补组件。 |
| FC4D-PROD-09 | SPEC §3.4 | effectiveAttributes 是 fixed Document#attributes 最终完整 present map，保持实际 enumeration order 且 name 唯一；builtin/generated/environment-derived 值带真实非 authored origin，不能伪造 physical sourceRange；headerAttributes 只代表 lexical authored header entry。 |
| FC4D-PROD-10 | SPEC §3.4 | backend/basebackend/filetype/outfilesuffix、safeMode、doctype、完整 title/mainTitle/subtitle 与 revision 来自同一次 fixed evaluation 并有字段 provenance；D8/D9 不得用 ambient default、fullname 或 raw Source 补猜。不得补造缺失状态，也不能改用宿主默认值。 |
| FC4D-PROD-11 | SPEC §§3.4,17 | richer managed Product carrier 不得破坏 fixed-parent 对合法 external ordinary exact Source read、Draft 与 authorized whole-source save 的正向路径；该路径不能伪造 managed revision 或 Product slot provenance。外部文件的普通正向路径继续有效。 |
| FC4D-POL-01 | SPEC §8.2 | 改变型 D8PresentationPolicySetRequest/2 按 closed decode→presentation disclosure→policy_admin→frontier/domain→head set→exact expectedHeads→parent record/pin/ancestry→semantics/budget，并返回一个 D8PresentationPolicyPrepareResult/2，把同一 operation 的 Input/Proposal/headEvidence/PreparedIntent3/d6_commit_request2/preview 闭合。完整验证顺序不可跳过，任何早期失败都不能泄露隐藏状态。 |
| FC4D-POL-02 | SPEC §8.2 | policy headEvidence 与 frozen heads 一一同序并选择每个 retained parent record 的 exact bytes；Proposal 冻结 parents/revision/value 且没有 activationChangeId；PreparedIntent 固定 saveProfile=control_only、既有 control_only Workspace scope、sourceInputs=[]、managed_atomic/strict，controlInputs/DependencyProof 只含实际 authorization/control dependencies，owner pinRefs 精确覆盖这些 pins 与真实 evidence；不创建 SourceRevisionPlan 或 author-source mutation，唯一 ChangeId 仍是 final-P portable decision ChangeId。这条控制路径不读取作者正文。 |
| FC4D-POL-03 | SPEC §8.2 | 恰一个 current head 且值已经等于请求 defaultPresentation 时，prepare 返回 d8_presentation_policy_no_change，零 PreparedIntent/P/ChangeId/record/outbox；初始化和 multi-head conflict 永远不能当这个 no-op。 |
| FC4D-POL-04 | SPEC §8.2 | prepare/planning/staging/InstallationNotice/install/verify 只冻结 proposal/input/pins：不得构造/编码 committed /2 record、计算 recordSha256、创建其 portable_metadata pin/outbox item、预留 ChangeId 或新增 PortableComponentKey。这些阶段都没有当前成功变更号。 |
| FC4D-POL-05 | SPEC §8.2 | final P 在分配 ChangeId 前重验 policy_admin、domain/frontier、完整 frozen head-set stamp/heads 与 ancestry；并发 head 变化或 P 前 authorization revocation 必须失败/重准备且零新 ChangeId。 |
| FC4D-POL-06 | SPEC §8.2 | 全部 final checks 通过后，同一个 P transaction 才 checked-allocate ChangeId C，从 frozen Proposal+C 构造 canonical D8WorkspacePresentationPolicy/2，计算 address/hash/exact portable pin，提交 presentation_policy_change、receipt/effects/ChangeRecord association 与 D8PresentationPolicyOutboxItem/1，并原子推进 owner head graph。整个提交必须原子完成，不能留下半成品记录。 |
| FC4D-POL-07 | SPEC §8.2 | 两个 offline successor 从同一 parent 出发后，即使 revision/value 相同，sync 后也保留两个 maximal heads；explicit resolution 必须把完整 current head set 都列为 parents 并生成一个 multi-parent successor，禁止 arrival/hash/revision/LWW winner。 |
| FC4D-POL-08 | SPEC §8.2 | final P 已提交而 publication/transport 未完成时，retry 只从原 D8PresentationPolicyOutboxItem/1 解引用并发布 exact pinned record bytes；不得重编码、再分配 ChangeId、重算 Proposal 或改变 heads。 |
| FC4D-POL-09 | SPEC §8.2 | receiver admission 必须验证 strict canonical record/address/pin、activationChangeId、same-decision EffectManifest/receipt/ChangeRecord association、parent ancestry 与 head-set continuity；equal hash、trusted sender、Boolean verified、provider latest 或 revision equality 都不能接纳 head。上述证据必须来自同一原始决策并能连续验证，任何单一声明都不能替代完整证明。 |
| FC4D-POL-10 | SPEC §8.2 | saved/planned/unknown 恢复 original Input/head stamp/headEvidence/Proposal/PreparedIntent/request/pins 与 P responsibility，禁止重新采样；不新增第二 Policy store、ledger、CAS、migration layer 或 content authority。恢复只能使用原始冻结证据，不能重新取当前状态。 |
| FC4D-EXP-01 | SPEC §8.3 | routeBinding.steps 的 array position i 必须有 step=i，恰形成0..N-1；每步 evidencePins 按 pinToken sorted/unique，乱序或任一 duplicate token 都拒绝 frozen/received Plan。 |
| FC4D-EXP-02 | SPEC §8.3 | styleBundles 按 styleBundleId bytes sorted/unique；duplicate ID 拒绝，同 ID 不同 version/pin 是 integrity conflict，不能 first/last/LWW。规范顺序固定，重复键直接拒绝。 |
| FC4D-EXP-03 | SPEC §8.3 | stagedOutputs 与 PublicationReceipt/3.outputs 按 controlled relative name sorted/unique；duplicate output name 拒绝，同名不同 bytes/digest/pin 是显式 conflict。规范顺序固定，同名重复必须拒绝。 |
| FC4D-EXP-04 | SPEC §8.3 | generationPolicy.render.bindingChoices 与 missingPolicy 分别按 templatePath canonical sorted/unique；同 path 不能有两个 inputIndex/action，也不能同时落入两类 choice，duplicate key 不能静默覆盖。 |
| FC4D-EXP-05 | SPEC §8.3 | nativeTableBindings 按 exact controlled (setName,columnName) sorted/unique；同 key 不同 token/selector 失败；既有 ResourceRef imageSizes/layoutChoices、lossKey lossChoices、pinToken top-level evidencePins/recoveryPins 保持既定 comparator。所有比较键都使用既定规范顺序，重复键绝不能覆盖旧值。 |
| FC4D-EXP-06 | SPEC §8.3 | D9 prepare 在冻结 Plan bytes/token/pins/staged manifest/confirmation basis 前先检测 duplicate/conflict 并 canonical-sort 全部合法 set-like input permutation，使置换得到 byte-identical Plan；frozen/received/recovery record 若已乱序/重复则 fail，不能读时排序。 |
| FC4E-GATE-01 | SPEC §§9.8–9.10 | D7CreateAnnotationIntent/2 与 current D8 Annotation arm 只接收 AnnotationEditableProposal/1，绝不接收 AnnotationEditableValue/1；caller 提交 state/confirmation/basis/expectedText/point/actor/time/trusted 字段必须拒绝，完整 Value/4 只能由 Core 构造。 |
| FC4E-GATE-02 | SPEC §9.10 | interactive_create 必须区分 absent_annotation 与已有 no_suggestion value，并正向支持合法 root comment、root mark、reply 以 suggestion=null 创建；只有合法 root suggestion 才进入 pending，首版绝不能 accepted/rejected。 |
| FC4E-GATE-03 | SPEC §9.10 | ordinary_edit 必须正向支持 no_suggestion→no_suggestion，包括普通字段编辑和满足既有 Value/4 cross-field 的合法 purpose/root↔reply 变化；stored target content 不变时不得新增 target-source-read 权限要求，reply 改动继续使用继承的 structural S 规则。 |
| FC4E-GATE-04 | SPEC §9.10 | no_suggestion→pending 只能经 current D8 ordinary_edit，在 Annotation 授权/token current 且真实 stored target fresh qualification 后合法；Core 自行计算 target evidence，但强制 pending+needs_reconfirmation，因此 caller/类别转换不能制造 confirmed/accepted/rejected。 |
| FC4E-GATE-05 | SPEC §9.10 | pending suggestion 可由 ordinary_edit 清成满足 cross-field 的 no_suggestion root comment/mark/reply，且清除本身不要求 target-source read；accepted/rejected terminal 不能清成 null、换 purpose/reply、重开 pending 或互换 terminal。 |
| FC4E-GATE-06 | SPEC §9.10 | manual_reattach 对 no_suggestion→no_suggestion 与 pending→pending+needs_reconfirmation 都有 exact 合法同 owner target 的正向路径；reattach 不能同时做类别转换，terminal suggestion 禁止 reattach。 |
| FC4E-GATE-07 | SPEC §9.10 | caller 试图通过 Proposal、targetPolicy、generic d3_operation、stale Draft 或 historical recovery input 偷带 lifecycle/evidence 时，均不能制造 accepted/rejected/confirmed，也不能绕过具名 operation-class gate。 |
| FC4E-GATE-08 | SPEC §9.10 | 继承的披露/错误顺序按操作实际消费保持：不消费目标内容的 no-suggestion 普通编辑与 pending 清除不要求 target-source read；类别创建、reattach、reconfirm、apply 只在各自具名 qualification/read 阶段读取目标，隐藏目标字节不得泄露。 |
| FC4E-NOOP-01 | SPEC §§9.6,9.10 | Core 绝不把 caller Proposal bytes 直接与 Draft/current AnnotationEditableValue/1 比较；完成 decode/auth/currentness 和 operation-class gate 后，先展开一份保留 before attribution 的完整 candidate Value/4，再比较 canonical candidate Value/4 与 canonical current Value/4，只有相等才是真 no-op。 |
| FC4E-NOOP-02 | SPEC §§9.6,9.10 | 未改动的 pending+confirmed ordinary prepare 必须是真 no-op：confirmation、targetBasis/expectedText/pointAffinity、attribution、annotationRevisionToken、SourceVersion、H 全部逐字保留且无 SourceRevisionPlan；ordinary prepare 不能隐式 reconfirm。 |
| FC4E-NOOP-03 | SPEC §§9.6,9.10 | 未改动的 pending+needs_reconfirmation ordinary prepare 必须是真 no-op：needs/evidence/token/SourceVersion/H/attribution 保持不变，不建立 SourceRevisionPlan 或 source_change；只有显式 reconfirm 在 fresh target read 后才能转 confirmed。 |
| FC4E-NOOP-04 | SPEC §§9.6,9.10 | 未改动的 terminal suggestion 与未改动的普通 no-suggestion comment/mark/reply 都在 Core 展开后成为真 no-op；terminal evidence byte-equal，两个场景都不改 attribution、不分配 revision。 |
| FC4E-NOOP-05 | SPEC §9.6 | expanded candidate 真正不同时，Core 才写 fresh lastEditor/editedAt、分配从未使用的 final AnnotationRevisionToken 并冻结唯一 SourceRevisionPlan/value pin；即使 Value bytes 回到旧值也必须 fresh token，ABA 不能复用旧 revision identity。 |
| FC4E-NOOP-06 | SPEC §§9.6,9.10 | permission revocation、并发 token/Observation 变化、stale base Observation/Draft 或 final read barrier 失败时，必须保持继承的 failure/reprepare 顺序并零变更，即使原本 expanded candidate 会等于 current content；no-op 不授予额外 authority。 |
| FC4E-NOOP-07 | SPEC §§9.8–9.10 | 旧 selector、PAB/EditBinding/ActionEvidence、saved Draft 或 historical recovery record 继续绑定原 token/Observation/decoder，不能借 equal candidate bytes、no-op prepare、类别转换或新目标资格复活。 |
| FC4F-PROD-01 | SPEC §3.4 | D2EffectiveDocumentAttribute/1 的 value 必须使用 closed D2NativeSemanticValue/1。fixed core 的 safe-mode-level、max-include-depth、authorcount 必须编码为 canonical unbounded integer，不能变 text，也不能靠 to_s 或 attribute name 猜回类型。 |
| FC4F-PROD-02 | SPEC §3.4 | fixed manpage 的 mannames 必须保持 ordered textArray，元素边界与顺序精确保留给 HTML5 join 与 DocBook map consumer；Product state 绝不能先 join/stringify。 |
| FC4F-PROD-03 | SPEC §3.4 | effectiveAttributes 必须等于 final present Hash map：key absent 就没有 entry，present nil 为 null；Boolean/String/Integer/textArray 类型互不混淆；integer lexeme 必须 canonical，text-array 顺序、重复项和空串都保留。 |
| FC4F-PROD-04 | SPEC §§1.2,3.4 | ProcessorAttributeOverride 继续只允许 text-set/null-unset；已接受的扩展/API 路径不得把任意 Ruby 对象泄漏到 Product。若最终属性值是 Symbol、Float、Hash、嵌套数组、混合数组、含非 String 元素的数组或其它不透明值，rich projection 必须不可用，不能通过 to_s 或 free-JSON 强制转换。 |
| FC4F-PROD-05 | SPEC §§3.4,8 | 每个 typed effective value 与 sourceOrigins 必须来自同一次 fixed evaluation；generated value 不伪造 physical range，current D7/D8/D9 consumer 保留 typed value/evaluation barrier，禁止重 parse source 或猜类型。 |
| FC4F-POL-01 | SPEC §§8.2,13.2 | current unseen fresh create/fork 的 WorkspaceBootstrapPlan/4 必须包含 mandatory D8PresentationPolicyBootstrapInit/1；before 是受保护且由 custody 证明为空的 head set，proposal 精确指 target Workspace，parents=[]、revision=1、defaultPresentation=separate，且没有 activationChangeId。 |
| FC4F-POL-02 | SPEC §§8.2,13.2 | bootstrap 的准备、规划和暂存阶段必须在原 create/fork 的 OperationId、DecisionKey、planning CAS 与 P 之下冻结初始化 helper 和 committed=null 的 presentation_policy_change；不得另行调用 SetRequest/policy_admin，也不得提前创建 record/hash/address/pin/outbox/ChangeId。 |
| FC4F-POL-03 | SPEC §13.2 | 原 bootstrap 的唯一最终 P 分配 ChangeId C，并在同一个原子转换中构造 canonical /2 record、hash、精确的 portable pin/address、已提交的 owner effect、原有 receipt/effects/ChangeRecord 关联、outbox 和单 head 的 D8 转换，同时提交 target activation/custody 以及其余 bootstrap state；所有 decision/activation ID 必须一致。 |
| FC4F-POL-04 | SPEC §13.2 | planning loser、abort、authorization/custody/final-check 失败或 P commit 失败都必须保持 target 未 activation，且不留下 partial D8 record/pin/head/outbox；绝不存在 active Workspace 等待后来 SetRequest 初始化的窗口。 |
| FC4F-POL-05 | SPEC §§8.2,13.2 | fresh create 或 fork 成功 activation 后，真正依赖 default 的 eligible document 必须立即通过唯一 current revision1/separate policy 得到结果；显式 run-in/separate、role-conflict 与 no-eligible-body 继续完全不消费 policy。 |
| FC4F-POL-06 | SPEC §§8.2,13.2 | bootstrap P 已提交但 publication/transport unknown 时，retry 只能重发 retained D8PresentationPolicyOutboxItem/1 的 exact record bytes；receiver admission 必须证明原 bootstrap ChangeRecord/receipt/effect/activation association 与 ancestry，不能新分配 ChangeId 或重编码。 |
| FC4F-POL-07 | SPEC §13.2 | current Plan4 的 saved/planned/unknown recovery 必须恢复原 initialPresentationPolicy、OperationId/proposal/custody mapping、pins 与 decoder；真正历史 Plan1/Plan3、restore、continue、failover、普通 managed copy 都不能合成该字段或静默迁移 active Workspace。 |
| FC4F-POL-08 | SPEC §§8.2,13.2 | heads=[] 只有在完整 protected proved-uninitialized state 时才合法，包括仍未 activation 的 bootstrap before 或 SetRequest 明确证明的 initialization state；missing/corrupt/unproved metadata、unknown decoder、absent file 或 continuity gap 都必须 unavailable，绝不能当 default/separate 或伪造初始化。 |
| FC4F-POL-09 | SPEC §8.2 | bootstrap 完成后，既有 SetRequest、offline successor、多 head 显式冲突解决、outbox 与 retention 规则保持不变：相同 revision/value 绝不能产生 LWW winner，后续 policy change 也不能改写已经冻结的 D9 Plan/staged bytes。 |
| FC4F-EXP-01 | SPEC §8.3 | bindingChoices 与 missingPolicy 的 templatePath 按 exact Unicode scalar sequence lexicographic 比较，normalization=none、case-sensitive，精确 scalar prefix 较短者在前；合法输入置换因此冻结成唯一 byte-identical 顺序，不使用 locale/case-folding。 |
| FC4F-EXP-02 | SPEC §8.3 | 大小写变体或 precomposed/decomposed Unicode 等 scalar sequence 不同的 path 保持不同 comparator key，按 scalar order 而不是 normalization 排序；某 path 是另一路径的 exact scalar prefix 时必须排在较长项之前。 |
| FC4F-EXP-03 | SPEC §8.3 | 继承的 safe-relative、alias 与 conflict 验证通过后，stagedOutputs 和 PublicationReceipt/3.outputs 必须按协议名称的原始无符号 UTF-8 字节做字典序比较；若一项是另一项完全相同的字节前缀，则较短者在前。禁止 normalization、case folding、host path-library collation 或 separator rewriting。 |
| FC4F-EXP-04 | SPEC §8.3 | output alias/conflict validation 与排序彼此独立：即使 raw UTF-8 comparator 能给两个 protocol string 排序，只要继承的 target validator 判它们 alias/conflicting 就必须拒绝；排序不能修复 path-safety conflict。 |
| FC4F-EXP-05 | SPEC §8.3 | 合法 template/output set 的任意输入置换必须在 freeze 前按两个 exact comparator canonicalize 成相同 Plan/Receipt 顺序；已经 frozen/received/recovery 的 record 若乱序、duplicate 或跨集合 conflict 必须失败，禁止 read-time sorting 修复。 |

## PR4 schema repair coverage（不新增验收 ID）

此前 dangling-schema 补全本身没有新增、删除或重编号当时的554条义务。本次环境修订只新增 AD2-41，使累计清单变为555条；M39-04 是已接受 v3.10 对原 ID 的正确修订，不是新增场景。其它既有 row 义务全部累积保留。直接覆盖关系为： 第四批A只新增 FC4A-PROD-01..06、FC4A-D8-01、FC4A-RUN-01..04、FC4A-EXP-01..06、FC4A-ROUTE-01..03 与 FC4A-IMPACT-01，使累计清单变为576条；没有替换任何既有 ID 正文。 第四批B只新增 FC4B-VAL-01..08、FC4B-D8-01..05、FC4B-SUG-01..06、FC4B-LIFE-01..05 与 FC4B-ALIAS-01，使累计清单变为601条；此前全部 row 正文逐字节保留。 本次第四批A残余修复只新增 FC4R-PROD-01..13、FC4R-ORIGIN-01..06、FC4R-LEVEL-01..03、FC4R-RUN-01..11、FC4R-EXP-01..09、FC4R-TABLE-01..08 与 FC4R-WIRE-01..04，使累计清单变为655条；不创建新 finding ID，也不替换此前任何 row 正文。 本次批注残余集中修复只新增 FC4C-SUG-01..10、FC4C-CARRIER-01..05、FC4C-READ-01..07 与 FC4C-STORE-01..12，使累计清单变为689条；此前655条 row 正文逐字节保留。 本次三项A残余集中修复当时只新增 FC4D-PROD-01..11、FC4D-POL-01..10 与 FC4D-EXP-01..06，使停止 head 的累计清单变为716条。本次 B226 operation-gate 修复对旧 row 只做两条具名 current-contract 更正：FC4B-VAL-01 与 FC4B-D8-01 将 stale caller 类型 AnnotationEditableValue/1 改为 AnnotationEditableProposal/1，同时完整继承原 owner/create 授权、Core attribution、完整 Value4 pinning 与防伪造义务；除此以外既有 row 正文不变。随后只新增 FC4E-GATE-01..08 与 FC4E-NOOP-01..07，使累计清单变为731条；不新增 finding ledger/version store。 本次 fixed438 residual 修复当时只做一条具名 current-contract 更正：FC4A-RUN-01 将 stale active-Workspace/expected-revision 表述改成 fresh Plan4 原子 revision1/separate 初始化与 exact expectedHeads/head-set currentness，同时完整保留 policy_admin、checked successor/conflict 行为以及不改 Document source/SourceVersion 的义务。随后只新增 FC4F-PROD-01..05、FC4F-POL-01..09 与 FC4F-EXP-01..05，使累计清单变为750条。本次最后窄修另对既有 FC4E-NOOP-03 做一条具名 current-contract 更正：把过宽的 no-plan 表述收窄为不建立 SourceRevisionPlan 或 source_change，同时保持 needs/evidence/token/SourceVersion/H/attribution 以及“只有显式 reconfirm 加 fresh target read 才能转 confirmed”的原义务不变。本次还只把既有 FC4F-PROD-04、FC4F-POL-02、FC4F-POL-03、FC4F-POL-09、FC4F-EXP-03 五条中文验收正文和 SCHEMAS.zh-CN.md 的三段中文说明改写为自然中文；ID、义务、schema shape 与英文语义都不改变。累计仍为750条未运行设计义务，不新增 acceptance ID、ledger、CAS、migration mechanism 或 test ID。

- 保留的见证与观察证据类型，包括文档、块、集合、目录、诊断、字符串、调用、操作、行内及正文证据：AD2-37、AD2-38、O34-01–O34-16、P35-01–P35-14、N36-04–N36-09、M37-01–M37-20、M38-01–M38-16、M39-01–M39-11；
- M37生产点、快照与cut、命名空间以及绑定时生产者时序规则：M37-01、M37-17–M37-20、M38-01–M38-16、M39-01–M39-11；
- AnnotationInlineBody/1 / AsciiDocInlineBody/1 alias 与唯一 AnnotationInlineProfile/1：AN2-08，并继续受AD2完整2.0.26语言Gate约束。

这些条目仍是未运行设计要求；schema补全不把它们改写成已通过实现测试。

第二批限定 D10 修复不新增验收 ID；上述既有 FC34B/C/D 行同时覆盖 fresh/current owner routing、Binding1/2/3 历史分派、mixed canonical order、单一 barrier、Record3↔Inventory2 五类 payload equality、Proof2.storeIncarnation 与实际 StopCapacity。

## 未运行边界

本 PR 未执行上述 750 个设计场景，也未运行 Ruby/Rust parser、provider、SQLite、replica、crypto、crash/fault-injection、Automation scheduling 或 execution-custody handoff。作者只允许检查 ID 唯一性、计数、JSON schema、replacement source blob与文档引用一致性。