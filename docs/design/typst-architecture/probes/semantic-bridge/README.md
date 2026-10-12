---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Limited Typst 0.15.1 semantic bridge probe

Status: executed research on 2026-10-12, not product implementation, complete E1 closure, or editor/renderer acceptance. Weftext input: `d167ae8c1d0346faf62e8e0887cf789de0bf0b89`. No product repository changes. The observer targets [official v0.15.1](https://github.com/typst/typst/tree/v0.15.1).

Eleven controlled fixtures were compiled with observer on, observer off (World-request/final-Frame comparison only), and official release CLI `typst 0.15.1 (9dfd3a08)`. All three PDFs were byte-identical for every fixture. On/off World source/file/font request sequences and final native Frame trees, including tags and glyph spans, were identical. See `executed-summary.json`. Timestamp 0 and embedded fonts were used with system fonts ignored; arbitrary projects/fonts are outside this proof.

Direct logging in visit_show_rules/realize misses cached work in the final convergence round. In the default fixture, stable round 3 physically executed only one realization. The patch adds an independent push-only tracked `Sink.probe(EcoString)` slot, merged through existing Sink.extend/parallelize order. Comemo replays these observer effects when cached results are reused. The stable subsink contains four realizations, including three replayed inner results and the strong output. Repeated include yields seven; the run-in/list/table/math fixture yields 42. Caches were retained and show was not re-executed. Content, styles and counter algorithms are unmodified. Observer CPU/memory overhead is unmeasured.

Observed boundaries: evaluated Content; prepared elements with native Location and effective level/outlined/numbering/direction; recipe/builtin returned Content and recipe span; ordered realization Content/Style arrays; native start/end Tags; iteration subsinks; successful final Introspector and Frame. Source ranges come from already returned Source and official parser, with no additional World reads. The runner extracts only successful stable rounds; earlier rounds remain diagnostic logs.

Fixtures cover default heading/paragraph/strong, heading prefix/suffix generation, removed heading, stripped strong, repeated include, loop-generated headings, a label-selected run-in heading with list/table/math, repeated variable content, Paged target, SVG resource, and computed string concatenation. Removed headings remain in final introspection while absent from display; generated prefix/suffix text is preserved. Repeated include retains the same physical span but distinct Locations and final counters [1]/[2], without a root writer. Builtin headings/strong can be associated with native tags, returned content and effective styles. Arbitrary recipes retain actual output without claiming the default semantic role survived.

Concrete origin limit: `left="FromA"; right="FromB"; text(left + right)` produces `FromAFromB`, but its Content/Frame span only identifies the call `text(left + right)`. These observation boundaries cannot reconstruct the two input strings' value-flow dependencies. Conservatively classify it generated with no default writer. Complete origin dependencies require an official eval construction/value-flow observer whose records also survive memoization. A unique span, equal string, or single occurrence never grants write authority.

This proves bounded observation of post-show execution events, styles, tags and physical origins while preserving official output and caching; it does not establish that these events form the finally consumed reading tree. Subsequent independent experiments on fixed Weftext `b895b35` found discarded text and links from `measure(link(...))` in the stable trace, and repeated observations of one basic-table link due to measurement/layout. Deduplicating by span, text or destination would discard genuine repetition. A final stable round is not final consumption selection and cannot directly supply RenderedLink, copied excerpts or Query body text.

The original eleven PDF/World/Frame equality results remain valid without establishing a complete flow route. This probe does not establish final consumption edges, value-flow/multiple-origin closure, authorization, table/math accessibility, inverse rich editing, HTML/AT/320px behavior, IME/Undo, performance or upgrade maintenance. See the [current REVIEW](../../REVIEW.md) for subsequent route decisions and verdicts.

Independent review also found that the old runner visited only objects with an `elem` key and missed depth-truncation sentinels without that key. The current runner traverses all JSON containers and includes a nested-sentinel self-check. Rechecking the eleven retained stable traces from the exact LF run found no truncation. This repairs the completeness oracle, without claiming a new compilation or final-consumption proof. A diagnostic depth limit may remain, but truncated results must be classified incomplete.

This small package contains only the patch for 12 official files/new observer module, actual dependency lock, fixtures, standard-library Python runner, version pins and compact executed summary. It excludes official source, targets, download caches and large traces.

Reproduce: download the fixed source archive and verify its SHA256 in `version.json`; apply `git apply /path/to/observer.patch` in a clean source directory; copy this `Cargo.lock` over the official lock. The patch excludes only docs/tests workspace members and three test-only typst-dev-assets dependencies to avoid unrelated Git test-assets downloads. Runtime dependencies remain locked.

```text
cargo build --locked -p typst-cli --no-default-features --features embedded-fonts --jobs 6 --config 'profile.dev.package."*".opt-level=0' --config 'profile.dev.debug=0'
python run_probe.py --instrumented-cli /path/to/source/target/debug/typst --official-cli /path/to/official-0.15.1/typst --output /path/to/probe-results
```

Adjust quoting for the shell; Windows uses typst.exe. The portable runner was executed again against the built binary from a separate fixtures path: all 11 passed. The patch passed git apply --check and git apply on clean original files extracted from the official archive; patched files equal the executed source. See version.json for archive/patch/lock pins.
