---
source_language: zh-CN
translation_status: source
---

[English](D8-IMPACT.md)
# A2 D8 Implementation Impact and Acceptance Overlay

状态：author-resolved-pending-independent。本文只描述实现/测试义务；除最终 handoff 明确列出的 repository checks 外，不声称产品行为已经执行。

## 1. Core implementation surface

实现必须保留一个 Core parser 与 current D2 Asciidoctor 2.0.26 product。Source read、Draft projection、edit map/origins、prepare、preview、commit/recovery 都消费同一真实 Source/Observation，而不是 UI parser/cache。核心模块至少需要：
- D2 Snapshot/3 / sourceOrigins adapter；
- D8 wire2 document/Draft entries；
- Draft Edit Map 与 UTF8/UTF16/scalar/EOL/grapheme converter；
- async input/IME controller；
- D8EditPrepareRequest/3 + PreparedEditBinding/3；
- D7/D6 EffectManifest/3 / EffectBytes/3 preview paging；
- committed single-source Undo/Redo adapter；
- Annotation Value4/R6 read/draft/edit/reconfirm/suggestion adapter；
- presentation-policy complex-owner graph/set/resolve；
- D7 Search interaction adapter；
- D7 View renderer/interaction/accessibility adapter。

无其中 adapter 时只关闭对应 rich/structured surface，不能禁合法 Source。

## 2. Exact-source and parser conformance

测试必须覆盖 current D2 的完整 legal AsciiDoc source，包括 titleless、subtitle 与 H1–H9。
还要覆盖 run-in/discrete/float、strong/emphasis/strike、links/citations、STEM/Mermaid。
还必须覆盖 attributes/includes/substitutions 与 unknown/inert。
native table 的 span、block、multiline 与 advanced style 也属于同一测试范围。
rich adapter 不支持必须返回 unavailable/unrepresentable，
而 Source roundtrip 仍 exact。

每个 Draft edit positive 必须比较完整 parse tree、diagnostics、sourceOrigins 和未选 bytes；不能只比 rendered text。incremental parser 只有与 full parse 完全等价才 PASS。

## 3. Draft Edit Map and coordinate tests

必须覆盖：
- UTF-8↔UTF-16↔scalar↔grapheme；
- CRLF/CR/LF、BOM、mixed EOL；
- emoji ZWJ、combining、CJK、RTL isolate；
- repeated equal cells/text；
- escape/join/atom read-only origin；
- plainRegion maximality；
- zero/nonzero EOF site；
- nine prefix/suffix EOL candidates；
- empty/blank/spaces/tabs；
- Enter→paragraph split、delete-last-text、merge、retype；
- exact preservation of every unselected byte。

fuzzy string search、DOM index、latest-source relocation 必须有负例。

## 4. Async/IME/Undo tests

至少构造：
response reorder、slow transform + successor typing、owner/session
switch、revocation mid-flight、IME begin/update/end/cancel、composition
blur/background、Enter/slash during composition、surrogate/emoji
mutation、selection reverse、system Undo integration、Source/Write/Read
cross-surface stack。

断言迟到 response 不覆盖 later input，不 drop successor log，preedit 不可 prepare，final composition 一个 Undo group，Undo/Redo serial 单调且 old map 不复活。

## 5. Clipboard and OS boundary

真实 OS clipboard 测试必须分平台证明 text/plain、exact Source copy、logical
copy、cut write-before-delete、clipboard failure
no delete、RTL logical order、TSV representability rejection、resource/download
permission。
HTML/RTF/code/script 不执行。
若平台 API 无法证明 clipboard success/atomicity，
cut 不得删除。

D9 rich clipboard 未实施则能力 truthful unavailable，不由 D8 测试伪造。

## 6. Prepare/preview/recovery

real Core decoder/conformance 要覆盖 PB3/PIntent3/Proof3/Effect3、TTL、budget 与 pins。
还必须覆盖 OperationId、planToken、preview cursor/epoch 与全部 pages。
确认前随机丢 page/byte slot 必须阻止 submit。

history corpus 要包含真实 wire1/PB1、wire2/PB2、current PB3；saved/planned/unknown 分别恢复原 bytes/request/pins/OperationId。不能通过 current reprepare“修复”unknown。

observed_only positive 仅 trusted interactive one-document whole-source；
Annotation/Undo/structured/bulk/automation negative。
合法 ordinary source path 在无 unrelated workspace proof 时仍能成功。

## 7. committed Undo/Redo

真实 sequence：
commit A → exact current after → Undo prepare/preview/confirm
→ inverse commit → Redo fresh prepare。
负例：
- intervening other commit；
- byte-equal but different production version ABA；
- missing before pin；
- additional control/Field/identity effect；
- lost permission；
- stale format/Observation；
- cross-Workspace；
- old Draft selector reuse。
每次 inverse/redo OperationId 必须新，旧 receipt 保留历史责任。

## 8. Annotation

测试 current Value4/R6：
- valid/invalid/absent body；
- Core attribution only；
- whole document/resource vs range/region stale；
- Trash suspend/restore fresh qualify；
- exact stable production address + receiver Observation；
- d9rg1 PDF/image region；
- manual reattach；
- reconfirm suggestion；
- apply/reject race；
- target hidden reject；
- apply real source mutation + accepted Annotation same seal；
- copy/import/fork identity mapping；
- historical Value3/plain_text read-only recovery。

invalid R6 不 fallback plain_text；old suggestion evidence不 revive。

## 9. Structured row/table/Field/Query

六 row domains分别测试。
native table fixture 包含 span/block/multiline/header/footer/ragged/trivia；
unsupported rich editing仍 Source-valid。
保留 set/insert/remove/reorder/checklist/promote 原 API，
禁止 native-column/TSV batch generator/auto ragged repair。

Field tests覆盖 equal-value duplicate occurrences、note/provenance/order、inverse
derived readonly、phone-only narrow current proof。
Query/collection覆盖 duplicate Node、page vs all、aggregate
readonly、explicit parent/no-root-fallback、requireMembership postquery
top/take、background commit stale Draft。

## 10. SEARCH-01–08 product tests

产品测试必须直接消费 current D7 Search fixture/QuerySpec/CanonicalGraph oracle，而不是重写一套预期：
- 三 preset scope/source/empty；
- title/subtitle/filename/path/resource distinctions；
- exact/NFC/CJK/RTL；
- shortcut quote/escape/keyword/precedence/incomplete/error span；
- visual/shortcut/plain roundtrip；
- opaque advanced conditions；
- permission/count/rank/snippet/index state；
- save/reopen/copy/import version dispatch；
- fresh hit resolution。
IME preedit=zero Query；UI error 不 fallback。mixed union duplicate occurrence 不合并。

## 11. View/renderer/AT

real renderer tests使用 D7 ViewSpec/1 fixture：
complete result before semantic draw；
wide long-form only；
line explicit sort/increasing x；
Panel Partition；
quantity/date basis；
network deterministic placement；
accessible table。
deferred layout 必须 unsupported_layout，
不通过底层 chart library 偷启用。

Desktop、WebUI 与 Mobile 分别测试 pixel/keyboard/focus/AT/RTL。
同时分别测试 high-contrast、zoom 与 reduced-motion。
Mobile fallback必须同完整 data。

## 12. Unicode/RTL/AT corpus

Unicode18/UAX9r52/UAX29r49 conformance plus fixed 98 RTL cases逐项执行，
不合并。
真实 NVDA/VoiceOver/TalkBack（按 claimed surface）测试 role/name/state/focus/logical
read/composition noise。
async font loading、wrap、dual affinity、hit-test
epoch、virtual focus pin 都需真实 host evidence。

## 13. Performance gates

保留：
- input p95 ≤50ms；
- layout p95 ≤100ms；
- ≥1000 events，cold/warm 各3；
- Desktop 10MiB/100000 lines；
- Mobile 1MiB/10000；
- one 1MiB line；
- 10× over-limit deterministic rejection/degradation；
- 100k navigation Nodes、10k rows、page≤200；
- steady editor memory Desktop≤256MiB、Mobile≤128MiB。

测量必须记录 hardware/OS/runtime/build/dataset/warmup/sample/raw distribution。
CPU-heavy parse/layout 不在 UI thread。
超 budget 保留 Draft/dirty，
不能 truncate/split。

## 14. D9/D10 direct integration

D9 integration tests只验证具名 D8 boundary：
rich clipboard unavailable、resource-region validator、template
dirty-Draft no overwrite、query_json result-only、export
complete input。
D10 integration只验证 explicit D8 proposal、no arbitrary patch/system clipboard/workspace
mount、SearchContribution still D7 Query、Mobile negative author-approval
capability。
D9/D10 full module acceptance 不由 D8 测试代替。

## 15. Evidence classes and current status

必须分别报告：
- bounded semantic/unit model；
- repository/static checks；
- real Core decoder/state-machine；
- real OS/clipboard/filesystem；
- GUI/IME/font/bidi；
- AT；
- multi-replica/race/crash；
- performance；
- migration/activation/deployment。

本作者批创建时这些产品类别全部 UNRUN。GitHub docs/source checks 只能证明 repository consistency，不能证明 D8 已完成语义接受。原 D8 source-map 的 FC 分类、historical→current disposition 与逐项义务语义核销缺口继续保持 OPEN，留给后续固定 SHA 的非作者完整 D8 复核。
