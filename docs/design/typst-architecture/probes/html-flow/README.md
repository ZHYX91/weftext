---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Bounded Typst 0.15.1 HTML style probe

This is a bounded technical experiment for an explicit Html flow / Paged print route, not product implementation, complete E1 closure or RQ03/RQ5.3 acceptance. Display occurrences are selected from the same final `HtmlDocument` children, not directly from stable-round realization traces. See the [evidence](evidence.md) for the support table and consumer contracts, [version.json](version.json) for exact versions and patch/lock pins, and [analysis.json](executed-summary/analysis.json) and [results.json](executed-summary/results.json) for the executed bounded results.

## Reproduction

Git, Rust/Cargo and standard-library Python are required. Download the [fixed v0.15.1 source](https://github.com/typst/typst/archive/refs/tags/v0.15.1.zip) and [official release CLI](https://github.com/typst/typst/releases/tag/v0.15.1), and verify the source archive SHA256 in `version.json`. Use a clean complete source directory; do not apply the patches again to an already patched tree.

Run the following PowerShell commands from this probe directory, replacing the three example paths with the actual source root, independent target directory and official CLI. Both `git apply` commands run in the source root with absolute patch paths from the probe directory; the supplied `Cargo.lock` replaces the source-root lock. The runner and analyzer are beside `fixtures/` in this directory. They locate fixtures relative to their scripts and generate `results/` here, with actual commands, exit codes, stdout/stderr, final native trees and on/off/release outputs.

```powershell
$probeDir = (Get-Location).Path
$sourceDir = (Resolve-Path 'C:\typst-0.15.1').Path
$targetDir = 'C:\typst-html-probe-target'
$officialCli = 'C:\typst-release\typst.exe'

Push-Location -LiteralPath $sourceDir
try {
  git apply --check (Join-Path $probeDir '01-inherited-observer.patch')
  git apply (Join-Path $probeDir '01-inherited-observer.patch')
  git apply --check (Join-Path $probeDir '02-html-style-adapter.patch')
  git apply (Join-Path $probeDir '02-html-style-adapter.patch')
  Copy-Item -LiteralPath (Join-Path $probeDir 'Cargo.lock') -Destination (Join-Path $sourceDir 'Cargo.lock')
} finally {
  Pop-Location
}

cargo build --manifest-path (Join-Path $sourceDir 'Cargo.toml') --target-dir $targetDir -p typst-cli --locked
python (Join-Path $probeDir 'run_style.py') --binary (Join-Path $targetDir 'debug\typst.exe') --official $officialCli
python (Join-Path $probeDir 'analyze_style.py')
```

On Windows, use short source and target paths to avoid MSVC long-path response-file errors; adjust the shell, paths and executable name for other platforms. This experiment used Cargo/Rust 1.98.0. The first build may download dependencies pinned by the supplied lock; add `--offline` only after those dependencies are cached. The inherited observer patch isolates unrelated test-resource dependencies and records World requests; it is not the display-consumption selection algorithm. Both patches passed check/apply on clean official source, and the six changed files equal the executed source. The same 16-case runner/analyzer passed from a separate fixtures path; after this oracle narrowing, analysis passed again on the existing final trees.

## Executed scope

For all 16 cases, adapter-off HTML equals official release bytes; adapter-on single/four-thread HTML and final native trees match; Paged off/on/release PDFs match byte for byte. The assert analyzer checks effective font/size/color, literal marker overrides and depth, simple cell stroke/align, body structure, measurement discard, actual reuse, table links, removed headings versus native introspection, and local native frames. The body-structure comparison ignores only newly produced markers and style carriers with explicit producer identities.

Ordered World source/file/font calls match in 15/16 cases: the unsupported-marker warning case reads the same source three times instead of twice, with 11 font calls in both modes. The extra read is consistent with diagnostics, but no call-stack trace was taken; identical dependencies are not claimed for every case.

What is proven is a bounded set of emitted native CSS values: font candidates, size, solid fill, effective strong/emph, literal Content markers and simple synthesized cell stroke/fill/align. Isolated SpaceElem, SmartQuote and Pre/raw paths retain official conversion and inherited container CSS; arbitrary local typography is not fully carried. The five levels are five source display/editing views, not font-size presets or new compilation targets; those UIs were not tested here.

Browser font loading, visual equality, viewport behavior, AT, complete table-edge conflicts/gutters/line rules and arbitrary free styles remain unproven. Native frames preserve complex dash/function-marker visual output, but complete frame reading/copy semantics are unavailable. Automatic fallback selection is not implemented.

The environment switch is fixed for each separate process and is not a memoization key. Production configuration must form part of library/routines/profile identity or a fixed worker configuration; do not toggle this research switch inside a shared cached process.

The prototype's `data-probe-text` / `data-probe-marker` attributes identify test carriers in controlled fixtures; they are not trusted production origins or writer qualifications. Authors can forge those names in raw HTML. Production needs internal provenance that authors cannot forge and must not infer origins from these attributes. The corrected analyzer folds only explicitly marked text-carrier spans and preserves other span structure and the projection's non-style attributes; styles are checked separately as CSS values. It also asserts that stock fixtures contain neither test attribute. This is an oracle repair: analysis was rerun on existing final trees, with no new compilation or fixtures.

## Consumer contract

The same final body child paths are the common consumer root for flow, Query display-body, default copy and RenderedLink. Native heading levels/counters come from the same document's introspector, never DOM h2 inferred as business level2. Copy requires explicit structural serialization of actual br/role boundaries, not TextItem concatenation. A frame's local target is Paged while its occurrence belongs to the outer Html compilation. Paged exports may branch differently; fragments/counters/sources are not assembled across compilations. Span is not writer proof; generated or ambiguous origins receive no writer.

This package contains only patches, lock, fixtures, runner/analyzer, compact machine results, versions and bilingual documents. It excludes official source trees, targets and download caches. See the [evidence](evidence.md) for complete consumer boundaries and unproven requirements.
