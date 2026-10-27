---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext

Weftext is a knowledge and writing tool built around **native Typst (`.typ`) author files**. It organizes notes, structured properties, queries and exports while preserving original files, note identities and save/recovery boundaries. The project **is still in development and has not been formally published**. Implemented source code is not proof of whole-version or cross-platform acceptance.

## What you can do

- Work with local notes and folders, navigation, ordinary editing, properties and Query while retaining the native `.typ` file as author content.
- Use controlled Server / WebUI paths with separate multiuser authorization and same-owner synchronization; indexes, caches and exports must not grant access to unauthorized content.
- Inspect links and structure, hierarchical tags, relationship graphs, read-only heading structure and media thumbnails within the separately verified candidate scope; newer capabilities still need whole-candidate checks.
- Preview and confirm save, structural and Office output operations with original-request recovery. Initial material recognition needs a separately installed local runtime, and uncertain results must not be presented as complete.

## Current scope and maturity

| Stage | Evidence and boundary |
| --- | --- |
| **0.6.0 Windows/NTFS** | The established v0.6 scope passed HTTP, affected browser and real NAS HTTPS acceptance on the same optimized executable, **but was not formally published**. That result does not qualify other platforms. |
| **0.6.2 local delivery candidate** | Initial Windows local/self-hosted recognition covers PDF text layers, PNG/JPEG OCR and selected official-document and ordinary digital-invoice schemes; the independent runtime, partial results and complex-layout limits still apply. |
| **Previous 0.6.3-rc.4 candidate** | Navigation, properties/Query, tags, relationship graph and read-only heading structure were integrated; **whole-version acceptance remains incomplete**. |
| **0.7.0-rc.3 development candidate** | Integrates GUI/CLI, Capture, reminders, language/host foundations, save recovery and the native author-close lifecycle, with controlled Linux Render/Notes service paths connected. Checks and actual-run evidence remain candidate-specific; full desktop, installation and cross-platform acceptance is incomplete. These are development slices, **not an accepted v0.7 product or installer**. |

See the [local-candidate scope](docs/MVP.md), [roadmap](docs/ROADMAP.md) and [acceptance workflow](docs/WORKFLOW.md) for actual evidence, supported environments and open gates. Passing checks on an earlier candidate does not qualify the current commit.

## Explore and check locally

1. Read the [local use and limitations](docs/MVP.md) and [Server / WebUI guide](docs/SERVER.md) before choosing a test environment or filesystem. Do not use an irreplaceable library as a test fixture.
2. For documentation, link and model contracts, run `python scripts/check.py` with Python 3.11 or newer. Add `--models` only when changing the bounded design models; **this is not product acceptance**.
3. On a development-ready Windows host, `python scripts/mvp.py check` builds and runs the existing local product checks without publishing software. The actual prerequisites, commands and evidence boundaries are in the [workflow](docs/WORKFLOW.md).
4. For the separate command-line entry, consult the [CLI and original-request recovery guide](docs/CLI.md). It shares the same Core as the desktop entry; uncertain effects must be investigated under the original request, not retried with a new ID.

## Still unqualified

- Native Windows window/installer delivery, real OS IME, full assistive-technology support, physical power-loss behavior and the original historical Undo anomaly retain separate acceptance boundaries.
- Complete Linux, Android and Apple products, file backends, input methods and performance are not qualified by Windows/browser checks or component simulations.
- Obsidian plugins are independent interaction references only: **Weftext neither depends on them at runtime nor integrates or synchronizes with an Obsidian Vault**.

## Technical reference

- [Requirements and scope](docs/REQUIREMENTS.md) · [technical design](docs/design/README.md) · [implementation order](docs/design/INTEGRATION.md)
- [Design review](docs/design/REVIEW.md) · [local candidate](docs/MVP.md) · [Server / WebUI](docs/SERVER.md)
- [CLI](docs/CLI.md) · [controlled Linux workers](docs/LINUX-WORKERS.md) · [roadmap](docs/ROADMAP.md) · [development, acceptance and release](docs/WORKFLOW.md)

Design checks, observed product acceptance and publication are separate conclusions. Passing the document check does not establish a releasable product.
