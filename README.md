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

The working tree is being consolidated as the **0.6.3-rc.2 release candidate, with whole-version acceptance in progress**. It retains the first three stages of navigation/writing, properties/Query, ordinary root presets, link diagnostics and stable-fragment candidates, and connects hierarchical tags, a two-dimensional relationship graph, a separate read-only heading structure, authored sibling order and on-demand media thumbnails. See [MVP](docs/MVP.md) for scope and limits. Implementation and targeted checks establish neither whole-version 0.6.3 completion nor public release.

The [hierarchical-tag design](docs/REQUIREMENTS.md#511-hierarchical-tags), fixed on 2026-10-20, now connects Core v2 forests, exact/descendant Query, independent move authorization, common selectors and synchronization/native-package consumers. Alpha.3's flat-tag conclusion remains a fact of that earlier candidate. TAG01–10/UX01–12 still require actual acceptance on the same final candidate; implementation does not close real retained-data conversion or historical IME/Undo limits automatically.

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
