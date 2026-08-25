---
source_language: zh-CN
translation_of: 11-ai-agent-integration.zh-CN.md
translation_status: synced
---
[简体中文](11-ai-agent-integration.zh-CN.md)

# AI agent integration

This specification defines the harness-neutral agent contract, host bridge, and scoped read-only MCP tools. Mutation tools, UI, packaging, and Server delegation may be exposed only when they preserve the authorization, approval, and transaction boundaries below.

## Support levels

Weftext may integrate several AI agent harnesses, but support claims are explicit:

- First-tier: Weftext maintains the adapter, compatibility matrix, UI path, documentation, security review, conformance suite, packaged acceptance, and supported-version diagnostics.
- Compatible: an adapter implements the public contract and passes core conformance, but Weftext does not promise packaged or cross-version acceptance.
- Experimental: no compatibility or release guarantee.

DeepSeek Harness (DSH) is the first harness designated first-tier. This prioritization does not make Weftext a DSH frontend, require one model provider, or prevent later first-tier harnesses.

## Architectural boundary

The stable Weftext side is harness-neutral and consists of:

- scoped context and document/search reads;
- structured tool/action descriptions and capability requirements;
- deterministic previews, base revisions, approvals, commit outcomes, and structured errors;
- streamed session events, status, cancellation, and reconnect/resume semantics where available;
- actor, delegated-client, adapter, and audit attribution.

Core does not depend on DSH, Node.js, a model SDK, prompt format, or agent transcript schema. No harness-specific field is added to the `weftext` envelope or an annotation sidecar. Agent sessions are not needed to rebuild or interpret a workspace.

## Import enhancement patches

An importer may request optional agent enhancement only after deterministic local extraction. The agent receives user-approved page/region evidence plus a bounded Weftext Import IR fragment and returns a typed patch against explicit target IDs and one base IR revision. Supported operations may correct OCR, reading order, heading classification, table structure, formulas, or figure descriptions. Whole-document source replacement, arbitrary AsciiDoc rewriting, direct workspace files, stale targets, and out-of-scope patches are rejected. The local validator applies an accepted patch to the IR, regenerates the exact proposal, and uses the ordinary preview and Core transaction. Provider, egress, cost, retention, redaction, confidence, and agent provenance remain visible in the import receipt.

## DSH adapter

The first-party DSH integration has two coordinated halves:

1. A host bridge lets Desktop or Server start or connect to a tested DSH runtime, map lifecycle/session events into the shared agent UI, request cancellation, and surface failure or resume state.
2. A Weftext tool/plugin package gives DSH scoped reads and structured Weftext actions. It never implements direct filesystem mutations.

The preview host transport uses a newline-delimited JSON-RPC server because it exposes durable `session.event` data and whole-agent `session.status`. Wire `0.0.1` has no per-prompt result, prompt-cancel, session-close, active approval request, or real protocol negotiation. The bridge enforces one successful initialization before prompts, validates the service name and supported version, and represents cancellation honestly as whole-runtime termination. Packaging may revise the launch mechanism without changing workspace persistence or Core action semantics.

DSH currently identifies itself as a developer preview and warns that APIs may change incompatibly. Every Weftext release claiming DSH support therefore publishes tested DSH versions and adapter versions. Startup performs a version/capability handshake; an unsupported combination fails closed and explains the supported path.

Official upstream references: [product overview](https://deepseek.com/harness/en/), [repository](https://github.com/deepseek-ai/deepseek-harness), [architecture](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/architecture.md), [SDK JSON-RPC server](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/sdk/server/README.md), and [ACP scope](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/acp/acp/README.md).

## Context and data handling

A session begins with an explicit workspace, subtree or node scope and a capability grant. Weftext supplies the minimum context needed for requested tools and filters inaccessible data before disclosure. Search, backlinks, aliases, diagnostics, counts, recent items, and errors obey the same scope.

The first-party integration does not provide DSH with a writable Weftext workspace mount. Local reads should use Weftext context tools or a deliberately scoped read-only representation. In hosted mode the adapter uses the authenticated Server API and never sees the hosted filesystem path.

Model credentials, DSH profiles, plugin configuration, session transcripts, checkpoints, caches, and approval logs are device or Server control-plane state. They are not written to portable workspace content by default and are excluded from portable workspace backups unless an explicit backup policy says otherwise. Exporting selected agent output creates or edits an ordinary node through a Core transaction and records visible provenance without embedding secrets.

## Actions and approvals

Read-only tools may run automatically only within the granted context policy. A mutation request contains the human actor, delegated client and agent origin, target, capability, base revision, proposed plan, affected nodes/resources, external-egress implications, and confirmation policy.

The user-facing state distinguishes:

1. generated text or a proposal;
2. a preview awaiting approval;
3. an approved action awaiting commit;
4. a committed Core transaction or a structured failure.

Bulk, destructive, cross-workspace, permission, secret-access, and external-egress operations require explicit policy and preview. Cancellation stops future work but does not pretend to undo a transaction that already committed. Undo or recovery uses ordinary Core semantics.

In Server mode effective permission is `human actor ∩ delegated session capability ∩ workspace policy`. Revocation takes effect before the next tool call or commit. Agent errors must not disclose whether an inaccessible node exists.

## Local and Server paths

Local Desktop integration owns adapter lifecycle, secure credential references, selected context, approvals, event display, cancellation, and diagnostics. Local mode still records an origin and session capability even though it requires no Server account.

Server integration may use a Server-managed adapter or accept an authorized remote adapter. Both are delegated clients. Server-managed execution additionally requires explicit sandbox, resource quota, network egress, secret, retention, upgrade, and operator-observability policies. Real-time collaboration does not allow an agent to bypass revision checks or structural transaction serialization.

## First-tier acceptance

DSH first-tier support is complete only when all of the following are evidenced:

- a published Weftext/DSH compatibility matrix and fail-closed version diagnostics;
- a maintained first-party bridge and Weftext DSH tool/plugin package;
- real session event/status streaming, cancellation, and documented reconnect/resume behavior;
- scoped read/search and propose/preview/approve/commit flows over real Core actions;
- no raw writable workspace access in the first-party integration;
- local packaged Desktop tests and, when Server support is claimed, role/ACL/non-disclosure/audit tests;
- stale-revision, denied-capability, adapter crash, Server restart, incompatible-version, and partial-session recovery tests;
- dependency license, supply-chain, SBOM, update, and rollback evidence for every bundled runtime component.

A mock conversation, direct source-file write, unpinned developer installation, or successful model response alone is not first-tier evidence.

## Current capability boundary

The active Rust workspace now contains `weftext-agent`, `weftext-agent-dsh`, and `weftext-agent-mcp`. The first crate defines capability intersection, action requests, previews, approval decisions, runtime capabilities, handshakes, and normalized events. The second owns DSH process launch, protocol framing, strict initialization/version checks, prompt receipts, notification mapping, stderr diagnostics, graceful shutdown, and termination-based cancellation. The third serves only `workspace_inventory` and `read_document` over MCP stdio. Its startup argument fixes one workspace, document selection uses a node UUID from a rebuilt valid inventory, results use relative paths, and its tool catalog contains no mutation, shell, arbitrary-path, or egress operation. The CLI exposes `weftext agent mcp serve`, `weftext agent dsh support`, and `weftext agent dsh probe`.

Protocol tests use a real child process that behaves as a deterministic fake DSH runtime. They do not require a model credential. MCP tests perform initialization, deterministic tool discovery, inventory, exact source read, pre-initialization refusal, and out-of-scope UUID refusal. An integration check with `@modelcontextprotocol/sdk` 1.12.0—the published DSH MCP client's dependency line—launches the native server and calls both tools successfully.

`integrations/dsh/weftext-readonly.cordis.yml` is the Weftext-owned DSH composition. It mounts the official `@deepseek-ai/dsh-mcp-client` and explicitly disables Bash, raw filesystem, workspace-context loading, skills, jobs, goals, and subagents. The official published npm JSON-RPC demo still requires deployment-owned Cordis configuration, and its independently published release-candidate packages currently do not form a compatible packaged dependency set for this new composition. Weftext therefore reports `read_only_tools` with `ready: false`: source-level configuration and MCP interoperability are evidenced, but packaged DSH acceptance, model-driven call evidence, UI, and all mutation flows remain open.
