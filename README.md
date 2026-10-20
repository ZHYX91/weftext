---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext

A file-based knowledge and writing tool built around native Typst. The **established v0.6 scope is complete for 0.6.0 on Windows/NTFS and has passed acceptance on the final optimized executable; it is not formally published**. Local and controlled Server / WebUI, multiuser authorization and same-owner synchronization include partial replicas, complex structural combinations, native semantic revalidation and resource-retirement preservation. Archived prototypes are not current build dependencies.

The same final optimized executable passed the complete HTTP matrix, affected browser suites and real NAS HTTPS acceptance, retaining original physical identities, personal Draft/Undo, separate authorization and original-operation recovery boundaries. See [MVP](docs/MVP.md) for supported scope and unverified platform conditions. Windows/NTFS and NAS client results do not establish other OS, real system IME, complete assistive-technology or actual power-loss acceptance.

The previous delivery is a **0.6.2 local delivery candidate with the initial Windows local/self-hosted recognition scope complete**: PDF text layers, PNG/JPEG OCR, initial mainland official-document and digital ordinary-invoice schemes connect original-page review, field confirmation and atomic Core import. Recognition requires an explicitly installed separate runtime. Uncertain image results and complex layouts retain partial status and losses; see [MVP](docs/MVP.md) for support and acceptance boundaries. The 0.6.1 final-candidate real system IME check and one earlier Undo anomaly retain their evidence limits. Obsidian/reference plugins serve only as independent interaction references, without a Weftext runtime dependency or vault integration.

The working tree is the **0.6.3-alpha.3 third-stage candidate**, retaining the first two stages of navigation/writing, properties/Query and ordinary root presets, and adding link diagnostics, personal ignore/expected-isolation rules and stable-fragment candidates; see [MVP](docs/MVP.md) for scope and limits. Two-dimensional graphs/heading structure remain later stages; whole-version 0.6.3 completion is not claimed.

The [hierarchical-tag target design](docs/REQUIREMENTS.md#511-hierarchical-tags) was added on 2026-10-20 after stage three and before graphs. The candidate still supports flat tags only; v2, descendant filters and move authorization are not implemented.

- [Local candidate use and boundaries](docs/MVP.md): writing, properties/Query, links/annotations, Office output/interchange and save/recovery, with build commands and actual limitations.
- [Server / WebUI guide](docs/SERVER.md): separate authorization for multiuser five-view writing, fields/Query, annotations, resources, structures and outputs, plus same-owner hosted synchronization; MVP records tested scope and remaining conditions.
- [Version roadmap](docs/ROADMAP.md): implementation scope and completion gates, including v0.6.1/v0.6.2 enhancements, v0.6.3 everyday usability/knowledge organization and subsequent platform acceptance.

- [Requirements](docs/REQUIREMENTS.md): the single current inventory of 27 groups, including navigation, rich types, library styles, five views and Chinese defaults.
- [Technical design](docs/design/README.md): authoring, Core, editor and features, with necessary schemas, examples and bounded experiments.
- [Implementation order](docs/design/INTEGRATION.md): vertical integration, current v0.6 progress and subsequent versions.
- [Final design acceptance](docs/design/REVIEW.md): fixed technical baseline, disposition and evidence limits.
- [Acceptance and release](docs/WORKFLOW.md): concise development entry points separating design checks from product tests.

## Local check

Python 3.11 or newer; no additional Python packages:

```text
python scripts/check.py --models
```

The default checks local file links, bilingual pairs, JSON and requirement inventory. `--models` runs bounded models in temporary copies without changing recorded evidence. Reproduce official Typst compilation and platform probes using their READMEs when relevant; do not rebuild them for every prose change.

Historical prototypes, designs and full reviews remain recoverable but are not build dependencies. Contract changes update affected consumers and both languages. Passing documentation checks does not establish product availability.
