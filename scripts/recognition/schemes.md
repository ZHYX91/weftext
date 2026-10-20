---
source_language: zh-CN
translation_of: schemes.zh-CN.md
translation_status: synced
---

[简体中文](schemes.zh-CN.md)

# Initial recognition schemes

`schemes.py` converts genuine recognition / DOCX evidence into unconfirmed proposals. It does not run OCR, persist business fields or execute recognized text. It is a v0.6.2 component; unit checks, real offline worker checks and product HTTP / UI acceptance remain separate evidence.

## Fixed scope

- `invoice-cn-mainland-digital-ordinary-single-page@1`: mainland China digital ordinary invoice, standard single page without a specific-business label; PDF, PNG and JPEG. The actual required title must match. Multiple pages or invoice titles are not combined. The invoice number is exactly 20 ASCII digits retained as text with leading zeroes. Dates require a complete valid Gregorian `YYYY年M月D日` or `YYYY-MM-DD`. Supported proposals cover buyer/seller names and tax IDs, amount/tax/grand totals and an explicitly stated currency. Amounts use exact decimal strings with two fractional places and optional valid English thousands separators; decimal commas are not guessed. `¥` / `￥` alone do not establish a currency. This extracts syntax and does not verify tax authenticity, business validity or structured line items.
- `official-document-cn-mainland-hans@1`: mainland China Simplified Chinese official-document proposals from DOCX structure or genuine PDF / PNG / JPEG recognition. Explicit agency, document-number, date and recipient labels produce field candidates; visible titles / numbering remain candidates for review. DOCX titles, levels and native numbering require the real DOCX adapter's evidence. An ordinary numeric paragraph prefix does not thereby become a proven list.

Both schemes require explicit operation settings `language: "zh"`, `script: "Hans"`, `region: "CN"`. Missing settings return unknown, without inference from UI, Han characters or the executing machine. A different scope is not applicable and does not silently select another scheme.

The invoice scope follows the invoice type, basic fields and 20-digit numbering in the State Taxation Administration's [Announcement No. 11 of 2024](https://fgk.chinatax.gov.cn/zcfgk/c100012/c5236067/content.html), with its attachment 1 ordinary-invoice layout inspected. The public repository contains only self-authored TEST examples, without real invoices. Inspecting the official attachment does not certify a fixture or tax result.

## Internal input and result

Call `propose(scheme_id, recognized, options)`. `recognized` directly consumes the offline worker's `input`, `status`, `pages -> regions -> blocks`. Blocks preserve exact `id/text/sourceMethod`, original-page `bbox` and available words. The DOCX adapter supplies `docxBlocks` with original `blockIndex`; styles, levels and numbering come from the real package, not plain-text claims. The integration invokes it after Core authorization and exact-original freezing; client assertions cannot substitute for an authorized original.

Fields are unconfirmed candidates with distinct `status`: `present`, `missing`, `duplicate`, `ambiguous`, `parse-failure`. A unique parsed `value` is a string; amounts, identifiers and dates never become floats. Each candidate preserves `raw`, `parsed` and exact `sources`: quote, block identity, original page/region or DOCX block index, with explicit Unicode code-point `start/end`. Boxes use word geometry only when strict sequential alignment with original text is proven; otherwise they retain block precision or unavailable coordinates.

Buyer/seller roles require explicit labels and provable standard left/right column geometry. Missing geometry, two unresolved roles in one block or text spanning both columns remain ambiguous. Role words within company names are not role labels. Two different valid invoice numbers without grouping reject pooled fields and retain only unassigned-number evidence. An unlabeled summary row requires explicit amount/tax columns and same-row geometry. Duplicates are not silently collapsed. Arithmetic differences only create review issues. Partial recognition retains its coverage warning.

Official-document numbering has explicit `keep` / `remove` / `rebuild` choices, default keep. Only this operation's `selectedStructureIds` receive modification proposals; stale/unknown selections are rejected. Rebuild removes a selected plain-text candidate's visible prefix and requires the authoring adapter to apply native numbering once, without inserting a duplicate numeric prefix first. Native DOCX counters are separate from paragraph text: structural proof does not establish that a literal body prefix is redundant, so the literal body is retained. Missing official-document markers are not applicable; numbered text alone leaves the document type unknown. An invoice is not processed as an official document. The module mutates neither source evidence nor a document. Persistent provenance and confirmed field mappings follow SOURCE / CORE; this JSON is not another editable author format or fact store.

`fields` is a dictionary keyed by the declared field name. Every field has `valueKind`: `text`, `decimal` or `date`. Invoice and tax identifiers are always text; only amount/tax/total use decimal, and issueDate uses date. Confirmation consumers match this semantic kind to an existing scalar schema field, without converting leading-zero identifiers into numbers.

`docx_reader.read_docx(original_bytes, expected_sha256)` reads real bounded OOXML paragraph styles, inherited outline levels and resolved numbering definitions. `blockIndex` is the zero-based paragraph ordinal in `word/document.xml`, including table paragraphs; `part` and `paragraphIndex` accompany quotes. It does not claim rendered page coordinates, evaluated fields or complete DOCX interchange. Revisions, active/non-text content and unresolved numbering disclose partial evidence. Encrypted, malformed, duplicate-part, excessive and DTD/entity packages are rejected; no active content or external relationship executes.

Structures include exact `contentSource` ranges alongside their full original source. Proven DOCX numbering carries definition evidence and a selected `nativeNumberingAction` (`preserve`, `omit`, `replace`), distinguishing native counter removal from literal text changes.

## Reproduce

```text
python -m unittest discover -s scripts/recognition -p test_schemes.py
python -m unittest discover -s scripts/recognition -p test_docx_reader.py
python scripts/recognition/build_scheme_fixtures.py --typst ABSOLUTE_TYPST --output-dir OWNED_OUTPUT
python scripts/recognition/test_scheme_runtime.py --runtime INDEPENDENT_RUNTIME --fixtures-dir OWNED_OUTPUT --evidence-dir OWNED_EVIDENCE
```

The unit commands check proposals and actual DOCX structure. The generator uses the repository's licensed Noto fonts and explicitly selected official Typst to create self-authored TEST invoice/official-document PDF, PNG, JPEG and genuine styled/numbered DOCX; Pillow only encodes fixture JPEGs. The runtime checker invokes the independently installed worker and then the scheme, retaining raw results/proposals and comparing every declared field with the manifest. This does not establish product permissions, durable commits, UI or real-invoice acceptance. Missing dependencies, partial outcomes and failures remain distinct.

The generator retains the unchanged original positive raster samples and adds eight negative invoice variants in every format: wrong title, absent/ambiguous/invalid date, inconsistent arithmetic, duplicate/competing numbers and multiple titles. The runtime checker verifies declared outcomes, exact source quotes, original digests, unconfirmed state and genuine DOCX numbering choices. Optional `--selections FILE` records explicit original-page regions keyed by fixture filename, with worker-supported segmentation and OCR scale. Selected-region runs remain separate from automatic whole-page recognition; new fixtures or routes do not retroactively qualify earlier failed inputs or routes.
