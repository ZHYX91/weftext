---
source_language: zh-CN
translation_of: 15-content-intake-foundation.zh-CN.md
translation_status: synced
---

[简体中文](15-content-intake-foundation.zh-CN.md)

# Content intake foundation

Every importer uses one Weftext-owned pipeline:

```text
source artifact -> probe -> plan -> bounded format worker -> Weftext Import IR
-> validate -> preview -> Core transaction -> receipt
```

Import creates a reviewable proposal for canonical documents/resources and never writes the workspace directly. Export and rendering are separate non-authority boundaries.

## Shared contracts

`SourceArtifact`, `FormatProbe`, `ImportPlan`, `ImportDocument`, `ImportProposal`, and `ImportReceipt` are versioned format-neutral contracts. The Import IR is an internal conversion contract, not a workspace format or normalized document database. Adapters map their parser output into Weftext-owned types; format-specific structures remain behind adapters.

## Safety and commit

Probe uses bounded evidence rather than filename trust. Encrypted, malformed, oversized, recursively nested, decompression-bomb, traversal, link/reparse, and active-content inputs fail closed or use a separately reviewed sandbox. Workers have cancellation, time, memory, entry/page, output, temporary-file, filesystem, and network limits and receive no writable workspace handle.

Validation reparses proposed AsciiDoc through the canonical profile, verifies UTF-8/resources/limits/reserved paths, and produces a complete preview. Commit is one recoverable Core transaction. Approval binds the shown proposal and never permits silent reconversion with different bytes.

## Local and enhanced processing

The baseline route is deterministic and offline-capable. Optional enhancement handles only selected uncertain evidence and returns a typed IR patch. It discloses provider, egress, scope, cost when available, retention, redaction, and uncertainty. Core validates the patch, regenerates the proposal, and follows the same preview/transaction route.

## Adapter quality

Structured parsers are preferred when they retain more meaning. Each adapter requires malicious/corrupt fixtures, deterministic validation where meaningful, cross-platform execution, cancellation/cleanup, path/resource safety, CJK/RTL evidence, accessible diagnostics, and an explicit loss report.
