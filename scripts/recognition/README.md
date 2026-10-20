---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Independent recognition runtime

`install.py` independently installs the initial Windows x64 recognition backend. The [lock file](../../resources/recognition/runtime-lock.json) fixes the SHA-256 of every download and runtime file. It creates this product's own Python 3.12 virtual environment with pypdfium2 5.14.0 (PDFium 156.0.8076.0), Pillow 12.1.1, Tesseract 5.5.3.20260724 and the specified tessdata_best 4.1.0 commit. It does not search another workspace or PATH for a similarly named backend.

From the product repository, use an independent local **64-bit, non-venv CPython 3.12**:

```powershell
& 'C:\Path\Python312\python.exe' scripts/recognition/install.py --runtime-dir 'D:\Weftext\.workspace\tools\recognition' --python 'C:\Path\Python312\python.exe'
```

Both paths must be explicit and absolute. The entry point creates an ownership marker in a new directory; it rejects an existing nonempty directory without that marker. It neither deletes nor takes over existing files. Conflicts or hash mismatches stop installation and preserve the files. A completed directory can only be verified against its original lock. Use another explicit independent directory for upgrades so active jobs cannot mix runtime or model versions.

Only this explicit installation entry point downloads locked official assets. The official Tesseract NSIS installer writes registry entries and may act on an existing installation, so this entry point **executes neither the Tesseract nor the 7-Zip installer**. The locked official `7zr` extracts the 7-Zip console and DLL; that console statically extracts Tesseract. Only `tesseract.exe` and its 33 DLLs in the locked PE dependency closure are copied. System/user PATH, registry, file associations and Start menu entries remain untouched. Backends run by absolute path; subprocess PATH contains only Windows system directories and never searches for a global backend as a fallback.

A complete verified asset cache also supports offline installation. After installation, verify actual files and rerun startup probes:

```powershell
& 'C:\Path\Python312\python.exe' scripts/recognition/install.py --runtime-dir 'D:\Weftext\.workspace\tools\recognition' --python 'C:\Path\Python312\python.exe' --offline
& 'C:\Path\Python312\python.exe' scripts/recognition/install.py --runtime-dir 'D:\Weftext\.workspace\tools\recognition' --verify
```

`--offline` fails on a missing asset without requesting the network. Wheels also install only from the verified cache with `--no-index --no-deps --require-hashes`. Jobs use a completed directory and do not download missing models. Installation probes create and render a PDF page, run OCR on an English/digit sample with every model combination, check the model list and record actual versions. They establish backend startup and model loading; they do not establish Chinese accuracy or product import acceptance.

The resulting `installation.json`, version `1`, records absolute runtime paths, base Python, actual versions, lock digest, every Tesseract binary/model digest, a license index and probe results. Directory layout:

- `venv/Scripts/python.exe`: the product's independent Python; `venv/Lib/site-packages` contains the two locked recognition wheels and the virtual environment's bundled pip.
- `tesseract/tesseract.exe`: statically extracted backend and fixed DLLs; `tessdata` contains only the specified `chi_sim`, `chi_tra` and `eng` models.
- `downloads`: official original distributions, cache and extraction intermediates; `licenses`: original notices/licenses; `work`: startup probe outputs. These are owned tool installations outside automatic temp cleanup.

Explicit model combinations are `chi_sim`, `chi_tra`, `eng`, `chi_sim+eng` and `chi_tra+eng`, all using LSTM `--oem 1`. Recognition and output document language remain separate; there is no automatic translation or simplified/traditional conversion. Initial budgets are 16 MiB input, 100 pages, 300 DPI, 40 million pixels per page, 32 MiB total output and 60 seconds of OCR per page. Individual jobs must still report partial failures and unprocessed pages within these limits.

The directory preserves complete downloaded official distributions, Tesseract AUTHORS/LICENSE, model licenses, every PDFium/Pillow and bundled dependency license inside the wheels, the 7-Zip license and an exact DLL inventory. This entry point lets users obtain a private local runtime from upstream; it does not grant redistribution rights. The official Tesseract installer does not include separate notices/corresponding source materials for every DLL. Before redistributing a complete runtime, satisfy each applicable upstream obligation; this iteration does not claim completed runtime redistribution review. Installation acceptance on other operating systems, mobile or server execution hosts remains subject to the product's version scope.

DOCX structure and numbering-reader checks use only the Python standard library. The product check entry runs `python -S scripts/recognition/test_docx_reader.py` with site packages disabled so installed local dependencies cannot hide accidental coupling. `build_scheme_fixtures.py` requires the selected Typst CLI and Pillow only when generating PDF/image fixtures; its OOXML fixture needs neither.

## Offline worker contract

Run `worker.py --runtime <absolute installation directory>` with that private environment's Python. It reads one UTF-8 JSON request from stdin and returns one JSON result on stdout. The [request](../../resources/recognition/request.schema.json) and [result](../../resources/recognition/result.schema.json) versions are both `1`. `inputPath` is an operator-side path to an already-authorized exact original; never accept a browser-supplied local path. The caller must freeze original digest, selected pages/regions and processing settings, then revalidate original, authorization, rules and model versions before delivering a result or committing a plan. The product bridge connects embedded fixed scripts to HTTP, Core Plans and the product UI. The worker remains read-only: intermediate results confer no commit authority; the authorized host and Core determine persistence and delivery eligibility.

Requests select unique one-based page numbers; omitting `regions` selects a whole page. `rect` is `[left,top,right,bottom]` in visible-page coordinates with a top-left origin: PDF points, or EXIF-oriented image pixels. Explicit regions cannot overlap. `mode` is `auto`, `text` or `ocr`; `rotation` is a recognition-only clockwise 0/90/180/270 degree rotation. `segmentation` values `auto`/`sparse`/`block`/`line` fix Tesseract PSM 3/11/6/7 respectively; arbitrary configuration files or commands are not accepted.

A region's explicit `ocrScale` can be `1` (default), `0.75` or `0.5`, applying LANCZOS downsampling only to the derived OCR raster; original pixels, page coordinates and rendering DPI limits remain intact. `ocrGeometry` records scale and actual OCR dimensions; `ocrToVisible` compensates integer-size rounding, crop, rotation and scaling. Users can review retries for different font sizes/layouts; model scores and optional scaling do not replace fixed quality gates.

PNG transparency is composited onto white in the derived page before preview/OCR, preserving original alpha and file digest; dropping alpha would incorrectly turn a transparent black background into solid black.

`operation` defaults to `recognize`. An `inspect` request omits `pages` and returns the bounded original's `pageCount` and every page's dimensions, orientation, crop and coordinate units, with empty `regions` and no rendering/OCR. `preview` uses the page selection and returns selected whole pages as `preview: {mediaType: "image/png", dataBase64, width, height}` with the same geometry; region coordinates keep their whole-page context. These operations neither load nor verify OCR models, but verify the PDF/image runtime and original; page, pixel, memory, time and total-output budgets still apply. The `__weftext_auto_` region ID prefix is reserved for derived regions. Adjacent regions assign characters with half-open boundaries to avoid duplicate boundary-center glyphs.

Results omit original paths and raw backend diagnostics. They preserve original SHA, selection, exact tool/model digests, frozen implementation digest, page/region states, raw text, word boxes and recognition scores. Scores are not accuracy measurements. `renderToPdf` maps render pixels to original PDF user coordinates. Image `visibleToRaw` preserves EXIF mapping; `ocrToVisible` maps cropped/rotated recognition pixels back to the visible page. A six-value affine `[a,b,c,d,e,f]` means `(a*x+c*y+e,b*x+d*y+f)`.

Automatic routing prefers a usable text layer and recognizes only image rectangles completely disjoint from existing text. A text region's `excludedRects` explicitly excludes those OCR rectangles. Unresolved image/text overlays return `partial`, preserve text and require an explicit selection rather than duplicate both pipelines. Empty OCR is not a blank page: a nonwhite image with no text is partial. Reading order, headers/footers and ruled grids are confirmation candidates; suggestions never delete original blocks. A basic complete straight grid runs one OCR pass after removing detected rules from a derived raster, preserving original and geometry. Complex merged/cross-page tables, handwriting, formulas and complex layouts have no structure-recovery acceptance.

Identical text-layer glyphs at identical coordinates appear once; repeated words at different positions remain. Complete straight grids can also recover cells directly from text-layer glyphs without OCR. An absent grid candidate does not establish complex-table recovery; originals and raw text remain available for review.

Horizontal whitespace marked as generated by PDFium is not original PDF Unicode text and is omitted from extracted content; explicit source whitespace and generated line boundaries remain. Large same-row geometric gaps split text-layer fragments into separate blocks for reviewable simple-column order, avoiding concatenated left/right column rows.

PDFium runs in one isolated process per page. A Windows Job bounds recognition and descendants to 1 GiB and 60 seconds; closing/canceling kills OCR descendants. Output and diagnostic files are also bounded. Jobs verify locked wheel files, Tesseract/DLLs and selected models before and after execution. They neither connect to the network nor search PATH for backends. Failed/over-budget pages remain explicit alongside successful pages as `partial`; completing a selected range does not establish whole-original coverage.

Windows page processes start suspended and resume only after Job assignment; children receive an explicit environment allowlist. OCR rasters/TSV reside within that invocation's private job directory, cleaned by the parent on normal exit or page-budget failure. OS locks can retain owned temporary files; cleanup errors must not replace the exact recognition/budget outcome. Forcibly terminating the entire worker can also retain its private directory; follow existing ownership rules rather than treating a killed process as completed cleanup.

An internal host can call `execute(request, runtime, progress=None)` with an optional callback receiving an isolated snapshot after each accounted page; `inspect` calls it once after the complete inspection. Snapshots retain the full selection, completed pages and known original page count, with `partial` status while selected pages remain unaccounted. Consumer mutations cannot change in-flight results; callback exceptions propagate to the host. This module accepts no client progress path and maintains no second commit ledger. Progress is provisional recovery evidence before final runtime/model revalidation: even a complete last snapshot cannot replace normal return and host delivery checks. A host recovering from timeout/restart preserves completed pages, explicitly identifies unprocessed scope and revalidates provenance eligibility.

Recovered product checkpoints carry `workerFinalValidation: false` and an outer job status of `interrupted`. Completed pages remain available for read-only review, with explicit missing pages and interruption losses. Neither `acceptPartial` nor loss acceptance authorizes importing them: rerun with an explicit new jobId, then review again. Querying/retrying the original jobId never resumes or reruns processing automatically. Partial results that completed final validation retain their existing separate partial-acceptance and Core confirmation contract.

Confirmation eligibility lasts 15 minutes from job creation, checked separately from the 15-minute execution budget. After expiry, status and existing page text remain readable only while current authorization, original, execution implementation and runtime still match; the response carries `confirmationExpired: true`. Expiry blocks starting/retrying the original jobId, page previews, downloads and new confirmation plans; commit still checks its original plan expiry. Read-only recovery never bypasses revocation, original replacement or model changes. Further processing requires an explicit new job.

Backend checks use self-created TEST material and local fonts to rasterize samples without copying font files:

```powershell
& '<installation>\venv\Scripts\python.exe' scripts/recognition/test_worker.py --runtime '<installation>' --evidence '<owned evidence directory>'
& '<installation>\venv\Scripts\python.exe' -m unittest discover -s scripts/recognition -p test_worker_boundaries.py -v
& '<installation>\venv\Scripts\python.exe' scripts/recognition/test_worker_progress.py --runtime '<installation>' --evidence '<owned progress evidence directory>'
```

These runtime component tests do not replace IM01–09, LC09, real-client, authorization/sync/recovery, specialized-field or Office round-trip acceptance.
