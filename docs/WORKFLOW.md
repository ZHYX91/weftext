---
source_language: zh-CN
translation_of: WORKFLOW.zh-CN.md
translation_status: synced
---

[简体中文](WORKFLOW.zh-CN.md)

# Development, acceptance and release

## Current stage

The final requirements and technical design are accepted. Refactor in the order of [INTEGRATION](design/INTEGRATION.md) §4: deliver a runnable local slice before expanding. Archived prototype dependencies do not make current documentation checks depend on the old UI or npm audit.

## Routine checks

For prose changes run `python scripts/check.py`. For schema, type/time algorithms or model changes add `--models`. Implementation slices add their own formatting, static analysis and meaningful tests. Choose platform, filesystem-failure, authorization, editing and Office scenarios based on actual impact. Record failures honestly rather than weakening assertions or expectations.

Keep one short acceptance record per deliverable slice: commit/artifact identity, environment, scope, commands/manual cases, results, limitations and necessary evidence locations. Summarize routine logs. Keep real inputs separate from generated output and never overwrite originals. Link checks and bounded models are not product acceptance.

## Release

Add versioning, packaging and upload automation when a runnable product exists. Build one candidate and accept that candidate; release the same artifact rather than rebuilding different bytes. Establish version, platforms, failures and user authorization before publication, then verify the download and a minimal runtime scenario.

The repository owns independent product build/check/package entry points. The personal workspace only supplies paths and local environment, without another release platform. Hash artifacts and material handoffs, not the entire repository for every discussion. Tags and releases correspond to actual deliveries; do not create an empty release now.
