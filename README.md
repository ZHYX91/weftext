---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext

A file-based knowledge and writing tool built around native Typst. Current state: **accepted design, product not implemented**. This is the refactoring baseline; archived prototypes are not an installable product release.

- [Requirements](docs/REQUIREMENTS.md): the single current inventory, including navigation, rich types, library styles, five views and Chinese defaults.
- [Technical design](docs/design/README.md): authoring, Core, editor and features, with necessary schemas, examples and bounded experiments.
- [Implementation order](docs/design/INTEGRATION.md): begin with a vertical local open/edit/save/recover slice.
- [Final design acceptance](docs/design/REVIEW.md): fixed technical baseline, disposition and evidence limits.
- [Acceptance and release](docs/WORKFLOW.md): concise development entry points separating design checks from product tests.

## Local check

Python 3.11 or newer; no additional Python packages:

```text
python scripts/check.py --models
```

The default checks local file links, bilingual pairs, JSON and requirement inventory. `--models` runs bounded models in temporary copies without changing recorded evidence. Reproduce official Typst compilation and platform probes using their READMEs when relevant; do not rebuild them for every prose change.

Historical prototypes, designs and full reviews remain recoverable but are not build dependencies. Contract changes update affected consumers and both languages. Passing documentation checks does not establish product availability.
