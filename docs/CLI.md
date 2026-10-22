---
source_language: zh-CN
translation_of: CLI.zh-CN.md
translation_status: synced
---

[简体中文](CLI.zh-CN.md)

# Formal command line

Owner connections first retain the process identity obtained from the original OS pipe and a protected handle to its original executable. The identification handshake carries only version, physical-profile digest and random challenge; business bodies, credentials and shutdown commands require complete executable qualification on that same file handle. Large-image qualification has its own deadline and tracked background work, outside the peer identification deadline. Failure never reconnects or resends automatically.

Normal shutdown waits for actual HTTP service and request-future destruction after the listener's completion signal, then joins accepted workers and releases the original Core. Local and hosted services use the same lifetime barrier; a completed response alone cannot prove final authority release.

`weftext-cli` is the business console entry. It shares one Core and local owner with the `weftext` desktop entry. It does not directly modify authored library files or start another Core. Implementation and usage documentation do not establish platform acceptance; [MVP](MVP.md) records actual artifacts, platforms and unverified conditions.

The default target is the current user's local profile; `--data-dir PATH` selects another profile explicitly. On Windows, the protected local channel qualifies the current user, physical directory and owner incarnation. An original request binds the physical profile identity; replacing a directory at the same path does not send an old request to a new empty journal. The lifecycle manages shared-owner exit; client exit never force-kills the shared process.

An on-demand headless owner keeps running so separate read, external-edit and save commands retain the same real session. It shows no tray and runs no desktop reminder scheduler. `capabilities --output owner.json` includes an OS-qualified `ownerIdentity`. To end that owner, use the same `--data-dir PATH owner-stop --owner-identity owner.json`. This connects only to the existing exact process, drains accepted work and waits for its exit, without starting an owner or terminating processes by name. An owner restart invalidates old basis sessions; the CLI rejects them explicitly rather than reopening a replacement.

`--data-dir PATH owner-status` reads only an existing owner's identity, including its local/hosted role. It starts no Core and calls no author API. Its JSON is also valid exact-identity input for owner-stop.

Owner status writes `{"schemaVersion":1,"call":"ownerStatus","ownerIdentity":{...}}`. The strict identity has `pid`, `created` (raw Windows FILETIME, not Unix time), `incarnation`, `profileIdentity` and `role` (`local` or `hosted`). Owner stop accepts this envelope, a capabilities envelope or the raw identity object from a UTF-8 JSON file of at most 16 KiB. Successful stop writes `{"schemaVersion":1,"call":"ownerStop","result":{"owner":{...},"exitCode":0}}` and exits 0 only after the exact original process handle signals exit. A closing acknowledgement alone is insufficient. A nonzero owner exit returns 3; a submitted shutdown with an uncertain result returns 6 and is not automatically retried.

`--endpoint https://server.example/ --credential-env WEFTEXT_CREDENTIAL` selects Server mode. The environment argument is a name; its secret value never enters argv, request files or output. Use an HTTPS root without user information, query or fragment. Controlled loopback tests may explicitly use `--allow-loopback-http` with a literal loopback IP. The client ignores ambient proxies, follows no redirects and never automatically resubmits effects. The Server receives authorized resource selectors; local absolute paths do not become Server capabilities.

## Input, output and language

The default `--format json` writes one UTF-8 JSON envelope and one LF to stdout. It contains `schemaVersion`, the exact target, fixed `call`, actual `httpStatus`, original service `result`, and an original request-file locator and digest for effects. Service error responses keep the same envelope and a nonzero exit. Local input, transport or delivery failures write no replacement stdout result: stderr receives a JSON line `{"schemaVersion":1,"error":{"code":"...","exitCode":6}}`, with `operationId` and `requestFile` when known. Retaining an input emits a separate stderr JSON line with `event:"requestRetained"` and the same locators. Thus stderr is JSON Lines in JSON mode. `--format human --locale zh-CN` or `en-US` uses the shared message catalog. Machine keys, error codes, source and authored values remain unchanged.

`--output PATH`, `note read --source-output PATH` and `export download --destination PATH` create new files only. They do not overwrite originals; parents must already exist. Windows device names, alternate streams and trailing-dot/space aliases are rejected. Request, result, source-copy and download paths cannot alias each other or enter known author libraries or Core-private directories. Download never regenerates an artifact: it checks the original request ID, chunks, length and SHA-256 before delivery. Failed writes retain their evidence and never delete existing originals.

Input files and stdin selected by `--source -` or `--input -` must contain complete UTF-8. Each process consumes at most one stdin stream. Source bytes pass through no shell, newline normalization or encoding repair. Source is limited to 2 MiB; remote uploads to 16 MiB; local interchange uploads to 64 MiB, with further actual format budgets. Duplicate JSON keys, invalid Unicode, truncation and over-budget input fail before submission. Native integer and decimal values retain their string representations instead of passing through floating point.

Shells can transform redirected or piped bytes. Windows PowerShell 5.1 redirection in particular does not promise native UTF-8 preservation. For exact files, use the CLI's `--output`, `--source-output`, `--source FILE` and download parameters. BrokenPipe is a result-delivery failure, not cancellation of an author operation.

## Common commands

`weftext-cli --data-dir PATH serve` runs the explicit HTTP host through the shared owner lifecycle. `serve --port PORT` selects the local port (0 selects an available port); `--open PATH --root PATH` supplies the initial local document. `serve --server-config FILE` selects the hosted Server configuration and conflicts with local port/open/root options. Only this subcommand owns those arguments; `--data-dir` remains the shared global option. A successful start writes one Ready JSON line `{"schemaVersion":1,"kind":"ready","url":"...","ownerIdentity":{...}}`, then keeps running until graceful stop. Ready is startup evidence; `owner-stop` and the original process exit establish completion. This command rejects client `--endpoint`/credentials and business `--output`, `--format human`, `--allow-partial` options.

A failed start writes no Ready. Its approved static diagnostic is a stderr JSON line `{"schemaVersion":1,"diagnostic":{"code":"...","message":"..."}}`, followed by the standard error line and exit 3. Configuration contents, arbitrary runtime error strings and credentials are not echoed.

In these examples, `C:\Notes` is the author library, the current directory is external working storage, and output paths do not yet exist.

```text
weftext-cli capabilities
weftext-cli library create --path C:\Notes --request-file create.weftext-request.json
weftext-cli library inspect --path C:\Notes
weftext-cli note create --root C:\Notes --parent C:\Notes --name 开始 --request-file note.weftext-request.json
weftext-cli note read --path C:\Notes\开始\开始.typ --root C:\Notes --output basis.json --source-output draft.typ
weftext-cli note write --basis basis.json --source draft.typ --request-file save.weftext-request.json --output saved.json
weftext-cli operation inspect --request save.weftext-request.json
```

`basis.json` must be a JSON envelope produced by this CLI's read command. Save uses its exact session/base and target; it never implicitly opens another session to overwrite concurrent changes. Obtain a fresh read result before another edit. `note save-as` additionally takes `--path`; the real Core still handles the destination.

| Family | Entry and input |
| --- | --- |
| Library | Local `library inspect/create`; Server `library list` reads authorized documents. Creation accepts actual options through `--options FILE`. |
| Note | Local `note read --path`, Server `--resource`; local `note create --root --parent --name` uses a real transaction. `note write/save-as` takes a read envelope and source file. |
| Query | Local `query catalog` requires `--root`; Server forbids root. `query run --input FILE` consumes the actual Query DTO, retaining coverage, limits and empty results. |
| Structure | `operation prepare/commit --input FILE --request-file PATH` consumes current Plan DTOs. Commit requires the original operationId and planId. Server operations use actor-authorized resources without exposing arbitrary HTTP routes. |
| Original result | `operation inspect --request FILE` queries only the original ID. `operation settle --request FILE --request-file NEW` queries first, then uses the original service's termination boundary. Committed facts win; retained recovery Plans are not treated as proof of no effect. |
| Local recovery | `operation recoveries`, `recovery --operation-id ID`, and `resolve --operation-id ID --action ACTION --request-file PATH` use existing recovery entry points without editing journals. |
| History | `history list/read/compare --basis FILE`; read/compare also take `--version`. `history restore --input FILE --request-file PATH` consumes the complete current restore DTO and preserves drafts and original-operation boundaries. |
| Export | `export prepare --basis FILE --kind docx/pdf/doc/wps`; local also supports source. Then `export status/confirm/download --request FILE` retains the original request. Confirm explicitly supplies `--digest` and, when needed, `--accept-losses`. |
| Full export options | `export prepare --input FILE --request-file PATH` consumes the fixed ExportPrepare DTO for real Query/field XLSX, native, bibliography, templates and embedded Query selections. Local mode also requires `--basis FILE` to qualify session/base and the author-tree boundary. |
| Import | `import inspect/prepare --file FILE --kind FORMAT`; local prepare requires `--basis FILE --name NAME` and optional `--parent`. Remote native/bibliography uses `--slot`, Office uses `--resource`. Commit/status/settle accepts only the corresponding import family's request file. |
| Sync | Local `sync binding/enable/publish` selects a root. `status/receive/export/history/adoption-cut/revalidation-cut/revalidation-review` takes each actual sync DTO. `sync operation` reads the original Plan; `sync cancel` cancels a prepared Plan by original operationId/planId. Semantic review does not write author facts. |

Use `weftext-cli <command> --help` for complete flags. Structure, Query, history restore and advanced export JSON is typed production API input. Do not omit the original session, base, Plan, authorized selector or loss confirmation, or replace the actual DTO with a guessed simplified body.

`sync check/push/pull` reuses the local owner's existing remote transfer jobs, taking `--server URL --server-credential-env NAME --identity FILE`; explicit loopback testing also takes `--allow-server-loopback-http`. Identity accepts exact SyncIdentity JSON or a CLI `sync binding` envelope. Push also requires `--root`; pull can select `--objects FILE`. Both retain the request file before submission and wait up to 900 seconds by default; `--wait-seconds 0` returns the current job state. `sync job --request FILE` reads the same job; `sync stop --request FILE --request-file NEW` requests that job's stop. The credential travels only in a separate protected-channel memory segment and never enters frozen wire. Transfer copies content-addressed records only; receiving still requires an explicit real Plan to preview and apply author changes. Expired jobs or owner restarts retain unknown without automatically starting another transfer.

Remote PDF and embedded Query export first observes the exact source/serial/hash through the existing editor map, then submits the complete snapshot. Map does not save authored files. Simple `--basis` input uses serial 0 from a fresh CLI session. A higher observed version or different source rejects a stale snapshot; the client does not reset the original session to bypass conflict. Advanced remote native and selectedWorks export requires no additional readable document; its resource/source fields and independent permissions differ from ordinary document export.

## Original requests and exit states

Every author, export and sync effect command requires a new `*.weftext-request.json`. Before sending, the CLI encodes actual wire once, creates or verifies the original ID, binds the real target, writes pure input using create-new, and calls `sync_all`. The file contains input and locators, with no credential, success receipt or second fact ledger. Responses never update it. Owner control instead uses exact process identity and the real shutdown receipt; it is not an author operation.

After a lost reply or uncertain effect, the client only queries the same original ID. It never repeats save, import, export or sync under a new session or ID. An explicit operationConflict cannot be overwritten by an old success for the same ID. `operation inspect` uses the original input file and does not interpret notRegistered as proof of no effect. Explicit settle uses the original service's termination/recovery boundary. A different actor, deployment or physical local profile rejects the old input.

| Exit | Meaning |
| --- | --- |
| 0 | This command completed fully. Explicit `--allow-partial` can also allow exit 0 for partial results without changing partial/coverage. |
| 2 | Invalid arguments, input, budget or input-record location; this effect has not been submitted. |
| 3 | Unsupported, unavailable dependency/connection or explicit service rejection. |
| 4 | Authentication/authorization failed; the original request remains retained. |
| 5 | Conflict, failure, expiry or an effect that did not fulfill the command. |
| 6 | Unknown, pending or recovery-pending original result; retain its ID and exact input. |
| 7 | Result/file/stdout delivery failed. Committed facts remain; this is not permission to repeat author effects. |
| 8 | Partial scope, limits or a recovery issue; original coverage remains explicit. |

The CLI reports only original facts supplied by the real service. If an API lacks an original-ID result, the CLI retains unknown; a collection status or directory existence cannot become a single-attempt receipt. Actual product acceptance must separately cover shell waiting, stdout/stderr, exit status, background owners and child workers. Private mechanism probes cannot establish product acceptance.
