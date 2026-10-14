---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext

A file-based knowledge and writing tool built around native Typst. Current state: **a 0.6.0-alpha.1 Windows local and bounded hosted candidate; the complete product remains in progress**. Archived prototypes are not current build dependencies.

- [Local candidate use and boundaries](docs/MVP.md): writing, properties/Query, links/annotations, Office output/interchange and save/recovery, with build commands and actual limitations.
- [First Server / WebUI slice](docs/SERVER.md): multiuser five-view native writing, actor drafts, conflicts and revocation; same-owner hosted sync is connected; complete feature integration remains pending.
- [Version roadmap](docs/ROADMAP.md): seven milestones after the first version, covering current confirmed requirements.

- [Requirements](docs/REQUIREMENTS.md): the single current inventory of 27 groups, including navigation, rich types, library styles, five views and Chinese defaults.
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
