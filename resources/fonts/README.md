---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Offline CJK fonts

This directory contains unmodified official Noto CJK static OpenType fonts. Native compilation and the browser use the same files without installing system Chinese fonts or connecting to a public CDN. [manifest.json](manifest.json) records file sizes, SHA-256, official commits, versions, locales, weights, original download URLs and actual font-table inspection results.

| Role | Font | Shipped weights |
| --- | --- | --- |
| Han in body, titles, lists and tables | Noto Serif CJK SC / TC | Regular 400, Bold 700 |
| Sans-serif documents and UI | Noto Sans CJK SC / TC | Regular 400, Bold 700 |
| Han and monospace candidates in raw/code | Noto Sans Mono CJK SC / TC | Regular 400 |

SC supplies default China glyph forms; TC supplies default Taiwan forms. These full language fonts cover basic simplified/traditional Han, Chinese punctuation and mixed Chinese/Latin, rather than a subset containing only the test string. They do not cover all Unicode. Mono ASCII advances are 500; Han and the inspected full-width punctuation advances are 1000 (unitsPerEm 1000), preserving the half-width/full-width relationship. Actual tabs, indentation, copying and rich editing continue to consume original source; all characters must not be forced into one width.

Sans/Mono are pinned to [Sans2.004](https://github.com/notofonts/noto-cjk/tree/523d033d6cb47f4a80c58a35753646f5c3608a78); Serif to [Serif2.003](https://github.com/notofonts/noto-cjk/tree/9b0f1436e455d902de067a2501422e5dc71ad16b). They are redistributed under SIL Open Font License 1.1. Ship the [Sans/Mono license](LICENSE-NotoSansCJK.txt) and [Serif license](LICENSE-NotoSerifCJK.txt) with the product; original copyright metadata remains inside the unmodified fonts. The upstream [third-party note](NOTICE-NotoSansCJK.md) includes historical Apache wording; LICENSE at the pinned commits and font metadata establish OFL 1.1 for these versions.

Mathematical structure continues to use the valid OpenType MATH font New Computer Modern Math. [Original notices](NOTICE-TypstAssets.txt) for Typst's embedded fonts come from pinned `typst-assets 0.15.1` and include the actual licenses for New Computer Modern, DejaVu Sans Mono and Libertinus. The [embedded-font manifest](builtin-manifest.json) records the 17 files' actual font versions, weights, styles, coverage counts, mathematical tables, SHA-256 and official pinned sources. The engine combines it with the CJK manifest. Native compilation retains all original fonts; the browser shares their bytes except for the two explicit derivatives below. NewCM10-Regular uses GPL 3 or later with Font Exception / Distribution Exception; the other NewCM fonts use GUST 1.0. A shared family name does not establish a shared license. These Han fonts provide candidates for body, raw and Chinese mathematical strings; they must not replace the entire formula's mathematical font. Author-selected fonts, `fallback: false` and custom native rules retain Typst semantics. The default set must not conceal missing glyphs in an author's selection.

Chromium's OpenType Sanitizer rejects the original NewCM10 Regular/Italic CFF hint programs. The browser uses renamed `WeftextWebSerif-*.woff2` only for these two weight/style combinations. The [web manifest](web-manifest.json) records original inputs, actual derived bytes, transformations and complete geometry comparisons; the [change/license notice](NOTICE-WeftextWebFonts.txt) distinguishes their use. CFF desubroutinization, hint removal, renaming and WOFF2 encoding retain every glyph, cmap, outline, metric and the other 12 SFNT tables; removing hints may change individual rasterized pixels. The CSS family New Computer Modern is an explicit alias from native output to equal-geometry web fonts; their internal name is Weftext Web Serif. Complete original inputs are in `web-sources`, with terms in the [original notices](NOTICE-TypstAssets.txt) and [LPPL 1.3c](LICENSE-LPPL-1.3c.txt). Install the pinned [requirements-web.txt](requirements-web.txt) tools and run `python resources/fonts/derive_web.py --check` to reproduce and compare completely in memory; these extra generation tools are needed only for font maintenance.

The browser loads actual resources from authorized same-origin `/fonts/<filename>` routes. [fonts.css](fonts.css) declares only font files and real weights, without persisting document style facts. Body/raw/mathematical-text mappings belong to host adaptation of the final native projection. Failed font loading must produce accurate presentation status and preserve input and drafts; a CSS family name is not proof of successful delivery.

Verify assets with standard Python:

```text
python resources/fonts/verify.py
```

The default check compares pinned SHA-256 for CJK files/licenses, native name/cmap tables and Mono advances, and matches all 27 browser declarations to the actual font manifests. Optionally specify `--builtin-dir <typst-assets-0.15.1/files/fonts>` to verify actual bytes, versions and MATH tables of the pinned compiler's embedded fonts. It proves nonzero glyph IDs for sample characters, rather than actual layout of every glyph, browser loading, mathematical composition, PDF embedding or OS IME. Native compilation and platform acceptance run separately; unrun Desktop, WebUI, Server/CLI, Office and assistive-technology scenarios do not close because this check passes.
