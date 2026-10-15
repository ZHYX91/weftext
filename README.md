---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext

A file-based knowledge and writing tool built around native Typst. Current state: **0.6.0-alpha.1 Windows local and bounded hosted development; complete v0.6 remains unfinished**. Hosted consumers, same-owner synchronization, partial replicas and bounded Compound creation of Node parents outside moved trees have been connected and checked in separate batches. The latest complete package remains the first Server / WebUI slice; later independently tested executables do not establish new whole-package acceptance. Archived prototypes are not current build dependencies.

- [Local candidate use and boundaries](docs/MVP.md): writing, properties/Query, links/annotations, Office output/interchange and save/recovery, with build commands and actual limitations.
- [Server / WebUI guide](docs/SERVER.md): separate authorization for multiuser five-view writing, fields/Query, annotations, resources, structures and outputs, plus same-owner hosted synchronization; MVP records tested scope and remaining conditions.
- [Version roadmap](docs/ROADMAP.md): implementation scope and completion gates, including v0.6.1/v0.6.2 enhancements and subsequent platform acceptance.

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
