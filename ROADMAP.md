---
source_language: zh-CN
translation_of: ROADMAP.zh-CN.md
translation_status: synced
---

[简体中文](ROADMAP.zh-CN.md)

# Weftext roadmap

## Current status

The repository implementation is an unreleased early prototype. D1–D9 target designs are accepted and D10 is being prepared. The [design entrypoint](docs/design/README.md) distinguishes targets, candidates and implementation. Public design material is not a runnable release.

## Design gate

D10 converges Agent, automation and external capabilities. A2 then reviews the whole system from first principles, compares better complete alternatives, closes P0/P1 findings and freezes a design baseline. Design and necessary review material may be published earlier through GitHub branches and PRs. Unaccepted candidates do not replace upstream designs.

## First usable release

G1 focuses on Windows Desktop plus CLI: a usable loop through creation, editing, querying, preview, confirmed commit and restart recovery, with permissions, security and backup recovery from the first vertical slice. Keep slices buildable and testable; validate actual installation packages and devices before release. G1 does not wait for all conversion, Server, Agent or Mobile capabilities.

## Later capabilities

Linux CLI can expand with independent evidence. Server plus WebUI depends on accepted permissions, transactions, auditing and backup recovery, without automatically including real-time collaboration. Conversion closes implementation gates per format and platform; Agent, automation and connectors depend on D10 and the original transaction boundaries. Local Mobile development and conformance testing can progress independently, with no conversion or Agent execution initially.

GitHub is the current distribution channel; production signing, notarization and store distribution are not completion conditions. Development signing does not establish distribution support. Each platform and capability enters the supported list only when actual evidence is complete.

## Collaboration and evidence

Ordinary Chat at 极高 handles design and bounded implementation, independent Chat Pro handles counterexamples, alternatives and acceptance, GitHub holds fixed commits and PRs, and CI performs automatable checks. Codex validates when local environments, operating systems, devices, installation packages or real recovery are required. Report design acceptance, CI success, device validation and release separately.
