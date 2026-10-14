---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Bounded origin/edit/save prototype

This tests one Typst source → actual origins in the same final Html tree → local edit → shared Draft/Undo → a dedicated experiment file. It is not a complete editor, production Core or five-view product acceptance. On 2026-10-13, [results.json](results.json) records **15 passing real compiler/Windows file cases**. [test_session.mjs](test_session.mjs) separately checks shared history, stale result/range rejection and synthetic composition grouping, not real OS IME.

## Scope and structure

- [03-source-origin.patch](03-source-origin.patch) follows native Spans on the same final Typst 0.15.1 Html text into official Source AST byte ranges, ancestors and whole-tree syntax kinds. It does not guess addresses by comparing DOM text. Span alone is not writer qualification.
- [server.py](server.py) operates only on a dedicated experiment directory's `note.typ`. Its writable profile requires unique literal body/heading/emphasis/list origins in a code-free document. Repeated includes, show rules, generated functions, nested code and tables retain source access. Even independent literal text beside code conservatively loses projection writers; this is a prototype limit.
- [session.mjs](session.mjs) holds one source, serial and Undo/Redo. Late compilation is accepted only with matching source and serial. [app.mjs](app.mjs) provides five bounded entry points: source, basic styling with syntax retained, click-to-open local source, literal text writeback and read-only reading. V03 needs an Apply button. V04 replaces only nonempty single-line text and escapes native punctuation; it is not structural editing.
- Windows save checks original file identity/exact Base under the same share-zero handle, records before/after, then writes/flushes and records commit. Response retries retain the operation. Reconstructing Probe after an injected exception preserves pending evidence; third-party content is not overwritten. Production ancestor/reparse/hardlink guarantees, directory durability, power loss and process termination were not tested, so this does not qualify a CORE backend.
- Browser checks covered V02 visible syntax, V04 Chinese writeback, V01 source verification, Undo/Redo, V03 local editing and save. Final-version read-only reading and rejected input checks appear in [ui-results.json](ui-results.json). The page uses host CJK fonts, not offline font-package evidence. Experiment data is outside this repository.

## Reproduce

Follow [html-flow](../html-flow/README.md) to apply patches 01/02 and its Cargo.lock to clean official v0.15.1 source. Apply this directory's patch 03 only to that isolated tree, using a separate target. Preserve the earlier evidence's source and binary.

```powershell
$editorProbe = (Get-Location).Path
$typstSource = 'C:\typst-editor-source'
$editorTarget = 'C:\typst-editor-target'
git -C $typstSource apply --check (Join-Path $editorProbe '03-source-origin.patch')
git -C $typstSource apply (Join-Path $editorProbe '03-source-origin.patch')
cargo build --manifest-path (Join-Path $typstSource 'Cargo.toml') --target-dir $editorTarget -p typst-cli --locked
$editorBinary = Join-Path $editorTarget 'debug\typst.exe'
python test_probe.py --binary $editorBinary --output C:\temp\editor-results-new.json
node test_session.mjs
python server.py --binary $editorBinary --data C:\temp\weftext-editor-demo --port 8791
```

`--data` must be an empty dedicated experiment directory or one marked by this experiment; never use a real library. Open `http://127.0.0.1:8791/`; Ctrl+C stops the service. Tests use temporary copies; the output parent must exist. Browser drafts live in this local origin's localStorage and retain original Base on restore. This is not encrypted, multiuser or cross-device storage.

## Unproven

Actual CJK IME/AT, cross-format selections, full caret/selection preservation, V02 Enter/structural syntax editing, complete inline V03 behavior, V04 empty-document/last-text deletion/structural insertion, tables and mixed-code rich editing, style visual fidelity, long-document performance, production Save As, managed Nodes/authorization/sync and Office output remain unimplemented here. Structural or unproved edits use source. The product should select a mature editor session component and incrementally implement official AST inverse editing, rather than treating this DOM prototype as a product backend.

This experiment supports the first vertical implementation slice. It proves neither universal writable origins nor complete five views. Fix actual failures at their responsible mechanism without automatically restarting global design review.
