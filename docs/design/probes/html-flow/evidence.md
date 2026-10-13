---
source_language: zh-CN
translation_of: evidence.zh-CN.md
translation_status: synced
---

[简体中文](evidence.zh-CN.md)

[Reproduction](README.md)

# Bounded evidence for Typst HTML styles and final consumption

Status: executed technical experiment on 2026-10-12, not independent review, product implementation, complete E1 closure or RQ03/RQ5.3 acceptance. Its scope is the 16 fixtures in this package; it does not establish arbitrary Typst documents, a complete renderer or rich writers.

## Conclusion and comparisons

Ordinary notes can prioritize the same final `HtmlDocument` from the official Html target as a real flow consumer entry. Paged print/export is an explicitly separate target. The bounded measurement/reuse counterexamples show why final children are a better consumption authority than stable realization traces: ordinary body text, lists, tables, links and hard breaks need not be reconstructed from Frame TextItems. Native HTML does not automatically meet the native-style requirements. A small adapter can read effective `StyleChain` values and synthesized `CellGrid` data at actual production points, carrying bounded key styles without re-evaluating user show, deduplicating by text/Span/coordinates or rewriting the Typst layout engine.

Executed results are in [analysis.json](executed-summary/analysis.json) and [results.json](executed-summary/results.json):

| Comparison | Executed result |
|---|---|
| Adapter-off HTML versus official release bytes | 16/16 equal |
| Adapter-on single/four-thread HTML and final native tree | 16/16 equal |
| Paged off/on/release PDF bytes | 16/16 equal |
| Bounded body-structure projection: text/roles/links/br/frame | 16/16 equal; ignores only added markers and style carriers with explicit producer identities |
| Ordered World source/file/font calls | 15/16 equal |
| JSON truncation check of final trees and native introspection | Recursively visits every dict/list; no truncation |

For the unsupported dynamic-marker warning case, off has source2/font11 and on source3/font11. The additional read of the same source is consistent with the new warning/diagnostic path, but no call-stack trace was taken. Calling it a diagnostic read is therefore an evidence-based explanation, not proof that every engine dependency order is unchanged. The retained native Frame content did not change.

## Official version and production entry

The engine is fixed to [Typst v0.15.1](https://github.com/typst/typst/releases/tag/v0.15.1), with official CLI `typst 0.15.1 (9dfd3a08)`. Source archive SHA256, exact `Cargo.lock` and binary/patch hashes are in [version.json](version.json). Cargo/Rust 1.98.0 was used. Both HTML and Paged receive `--features html`, `--creation-timestamp 0` and `--ignore-system-fonts`, with jobs1 and an HTML jobs4 comparison. See the [README](README.md) for execution steps.

Official `convert_to_nodes` traverses ordered post-realization Content/Style pairs. `handle_html_elem` creates HtmlElement after producing its actual children; TextElem follows official handle_text/Converter/whitespace protection. The adapter reads current `text::families`, `text::variant`, resolved TextElem.size and solid fill. Strong/emph native rules use the same WeightDelta/ItalicToggle Content.set operations as official Paged, preserving effective delta:0 and nested-emph results. User show-strip strong is not wrapped back into default bold. Text carriers have explicit identities and do not change the official block/box classification of a lone element. User show is not evaluated twice.

The table rule reads already synthesized CellGrid/Cell stroke/fill and TableCell.align without rerunning cell-style callbacks. Literal list markers use native ListMarker::Content depth selection and are produced once through Content paths. Body Depth folding uses the same Depth(1) operation as official lists. Func markers are neither executed nor guessed in flow and produce an explicit warning; the frame's native Paged rules execute them normally.

After `typst::compile::<HtmlDocument>` returns Ok, the CLI directly snapshots `doc.root_node()` recursively. Display selection follows the final product's child paths: it does not use stable Sink as display, filter measurement stacks or deduplicate by Span/string/coordinates. Tags retain native introspection edges and Frames retain native product edges; generated head CSS is not body text. The final HtmlDocument is the official selected return value after memoization, parallel work and convergence rounds. Genuine reuse into multiple child paths retains every occurrence. Observation follows the consumed final product rather than replaying show logs to fill missing entries.

`TYPST_HTML_STYLE_PROBE` is fixed for the lifetime of each separate on/off process and is not tracked by comemo. Production should include the adapter profile in library/routines configuration identity or use fixed worker configuration. Switching configuration inside a shared cached process was not tested and cannot be called safe. Caches were not disabled.

## Bounded style support table

“Carried” below proves only final native CSS values, not browser glyph or geometric equivalence.

| Item | Executed result | Qualification and limit |
|---|---|---|
| Body library→note→local size | 16pt→12pt→18pt; AfterLocal returns to12pt | Effective size carried; browser glyph/pagination unproven |
| List size and marker | Library9pt/LIB-V2→note15pt/NOTE-M; nested ONE/TWO | Literal Content markers; added markers receive no body writer; fixed0.5em gap, complete indent/body-indent/marker-align unimplemented |
| Font overrides | Libertinus Serif14pt/list New Computer Modern9pt→note NCM12pt→local Libertinus18pt | Native font candidates and CSS name escaping; no font files exported/loaded; actual browser font/glyph/fallback unproven |
| Solid colors | Blue #0074d9; heading red #ff4136; table yellow #ffdc00 | Gradient/tiling not mapped and must not be presented as solid fill |
| Strong/emph | Strong700; delta:0=400; emph italic; nested emph normal; show-strip strong400 | Native style operations, no assumed bold from entry role; stretch/features/tracking/kerning unimplemented |
| Heading/custom show | 20pt/red on PREFIX/HOne/SUFFIX; native default15.4pt/700 | Native defaults, no guessed DOM UA styles; DOM h2 separate from business level1 |
| Simple table border | Library1.3pt/red→note explicit2pt/green CSS on four sides | Native side values carried; CSS double edges, border-collapse conflicts, gutters/spans/hline/vline/thickness-topology visuals unproven |
| Table cell fill/align | Yellow; center/middle→local right/bottom | Solid fill and explicit native alignment; inset/column tracks/baseline not covered |
| Authored HTML CSS | Native CSS follows carried values; authored style appended by official export | Arbitrary raw HTML/custom CSS selector interactions not covered |
| Dynamic marker / dashed border | Flow lacks FuncMarker and CSS on dashed sides | Requires capability status and native frame or an explicit local gap; automatic fallback unimplemented |
| Nonzero h / free page geometry | Official HTML warns and ignores nonzero h; actual br retained | No guessed spacing; explicit flow support, native frame or Paged entry required |

Ordinary inline TextElems carry local styles. Isolated SpaceElem, SmartQuote and Pre/raw paths do not each receive local carriers; they retain official conversion and inherited container CSS. Complete arbitrary local whitespace/quote typography is not claimed; logical whitespace/break preservation is not glyph-metric equivalence. The five levels mean five source display/editing views, not font-size presets or new compilation targets; those UIs were not tested.

Browser flow and Typst free-page layout differ in geometric semantics. A few CSS values cannot prove equivalence for varying viewports, browser shaping, page-relative placement, cross-page counter/layout context, fixed columns/floats or glyph details. Ordinary text/lists/simple tables should remain actual flow. Constructs depending on whole-page relationships need an explicit Paged viewing/export entry. Self-contained local constructs can use native `html.frame`, without cutting fragments from a second PDF or claiming to preserve page-relative whole-document relationships.

## Final-consumption cases

- `measure-discard`: final body contains only VisibleOnly, without MeasuredOnly/link; selection does not filter measurement call stacks.
- `standard-matrix`: one body link and one 2×2 table link each have one final `<a>` child. Native table/ul/li/br roles remain; there is no Span deduplication.
- `measured-par-used`: measure(body);body produces one final p and one br, not two final occurrences from two producer records.
- `repeat-include`: two headings from the same file retain two h2/native Locations and numbers1/2. There is no root writer or same-Span merge.
- `same-source-different-show`: two native block divs contain FirstShow/SecondShow, preserving actual returned results rather than assuming equal content from equal origins.
- `show-context`: PREFIX/HOne/SUFFIX are final display; StrongBody is not falsely bold. GoneHeading remains in same-compilation native heading introspection but is absent from display; AfterGone=2.
- `runin-list-table-math`: native heading level2/counter exists, while display is inline strong plus adjacent body in a paragraph. Query.body is not presented as complete post-show display and Span does not grant heading display roles.
- `whitespace-break` and `whitespace-strong`: soft source whitespace follows official HTML normalization; explicitLinebreak retains `<br>`, and p/list/cell boundaries exist. Concatenating final Text loses br/paragraph boundaries. Nonzero h is unsupported.
- `frame-target`: outside the frame, Outer=html and After=1. One final `HtmlNode::Frame` contains Inner=paged and FrameLink with an actual native Link destination. The local frame in `unsupported` contains FuncMarker0 in native text and actual dash patterns in Shapes; no content is supplied across compilations.

A Frame is a local visual-fidelity blob. Internal TextItems are evidence for native Frame inspection, not a flow/copy reading tree. Complete semantics, AT text alternatives and editable writers remain unproven; FrameItem strings must not be concatenated into writable body text.

The prototype's `data-probe-text` / `data-probe-marker` attributes identify test carriers in controlled fixtures; they are not trusted production origins or writer qualifications. Authors can forge those names in raw HTML. Production needs internal provenance that authors cannot forge and must not infer origins from these attributes. The corrected analyzer folds only explicitly marked text-carrier spans and preserves other span structure and the projection's non-style attributes; styles are checked separately as CSS values. It also asserts that stock fixtures contain neither test attribute. This is an oracle repair: analysis was rerun on existing final trees, with no new compilation or fixtures.

## Proposed EDITOR consumer contract

1. Each ordinary-note compilation produces an explicit Html flow result. Final HtmlDocument body child paths are the shared root for flow display, Query display-body, default copy and RenderedLink. Same-compilation Tags/introspection can associate them, but unconsumed realization records must not be added. A Frame has localTarget=Paged while compilation ownership, source snapshot and occurrence path belong to the outer Html result.
2. Native heading query/business level/counter form a separate semantic domain from the same HtmlDocument introspector. Display roles/text/links come from actual children. DOM h2 is not business level2; business levels use native Heading.level. A removed heading may remain a business query record without entering display/copy.
3. Copy applies an explicit serialization policy to actual text/br/p/li/td structure. Spaces, paragraph boundaries and table separators are public copy policy, not coordinate guesses. Head/style and Tag metadata are not body text. Frames lacking proven reading text retain a local fidelity/alternative state without posing as copyable flow text.
4. RenderedLink enumerates final same-compilation `<a href>` occurrences. Native frame links can use actual native destinations or links emitted by official SVG in an explicit frame domain, without source-query-link or Span deduplication. Product frame link dispatch/AT is not implemented.
5. Paged print/export is an independent target result. `target()` branches compile normally for each target; body/counter/link/origin are not assembled across Html/Paged results. Official Html frames internally select Paged and require explicit localTarget.
6. A literal writer separately requires a source slot, source snapshot, uniquely reversible literal origin and current occurrence qualification. Span is only evidence. Generated/context results, concatenation, multiple inputs/occurrences and custom shows without complete inverse mapping get no writer and retain a source entry. Arbitrary value flow and writer eligibility are not implemented by this prototype.
7. Style qualifications should distinguish Supported/Partial/NativeFrameRequired/PagedRequired rather than present partial mapping as complete coverage. Ordinary notes cannot fall back wholesale to source/PDF. Official HTML's experimental status is a version-maintenance/compatibility risk, not the sole reason to reject flow, and does not mean all native-style requirements are met.

## Review boundary

Bounded key styles, the final consumer entry and measurement/reuse/custom-show/hard-break/table-link counterexamples have executable evidence. The experiment did not expand into a complete renderer or Paged producer-consumer DAG. Remaining gaps include dynamic markers, free geometry, browser font resources/visuals, table-edge topology, frame reading semantics, automatic capability/fallback selection and writer eligibility. There was no actual browser/320px/font-loading/AT test.

The two sequential patches, exact lock, fixtures, runner/analyzer, machine summaries and version pins are reproducible. Both patches passed check/apply on clean official source; six changed files equal the executed source file for file. The same 16 cases passed assertions from a separate fixtures path; the narrowed oracle also passed on the existing final trees. Published summaries do not replace independent review, and conclusions stay within these bounded cases.

Before publication, the coordinator separately extracted actual staged LF files into an independent directory and reran the same 16 cases and corrected analyzer. All bounded assertions above passed. Public machine summaries record that exact publication-input run.

Official background: the [HTML documentation](https://typst.app/docs/reference/html/) describes semantic structure and current style gaps, and the [target documentation](https://typst.app/docs/reference/foundations/target/) describes target selection. Exact APIs are those of fixed v0.15.1 source. Route feasibility is an inference from this bounded experiment, not complete feature acceptance.
