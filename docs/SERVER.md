---
source_language: zh-CN
translation_of: SERVER.zh-CN.md
translation_status: synced
---

[简体中文](SERVER.zh-CN.md)

# First bounded Server / WebUI slice

`0.6.0-alpha.1` adds a single-writer hosted source-editing entry for Windows/NTFS. It reuses Core file guards, save journals and recovery evidence, with actor-owned drafts and original operation records. This is the first v0.6 slice; the complete scope remains in the [roadmap](ROADMAP.md).

## Configuration and startup

Create a library and notes in local mode first, or select existing ordinary `.typ` files. Managed notes require their original `--data-dir` identity/history evidence. Stop the local process before starting Server. One writer owns the author tree; a second instance with a different private directory must not bypass ownership.

Create configuration outside the document root. This example exposes two existing notes. The helper is create-only, never overwrites configuration, and does not start or expose a service. It prints one-time random credentials and stores only their SHA-256 digests. Keep administrator and personal tokens separately, outside source repositories and synchronization roots.

```text
python scripts/server_config.py --root "D:\Notes" --out "D:\Weftext-Service\server.json" --document shared=Shared/Shared.typ --document other=Other/Other.typ --user alice --user bob
weftext.exe --server-config "D:\Weftext-Service\server.json" --data-dir "D:\Weftext-State" --no-browser
```

The default bind is `127.0.0.1:47821`. Open the service URL and enter a personal token; the administrator token cannot log into the editor. A trusted operator chooses `root`, resource IDs/relative paths/display titles/`editable`, explicit dependency `inputs`, and deployment ID. Clients supply resource IDs or opened sessions, never filesystem paths, dependency roots or asserted actor identities. Each actor/resource has separate `read`, `write`, `history`, and `download` grants. Missing grants deny access; a disabled user loses all access. The latter three require `read`. Resources are authorized separately; dependencies do not inherit main-source permission.

TLS, password accounts, SSO, service installation and high availability are not built in. For LAN deployment, run behind a trusted HTTPS reverse proxy with an exact `origin`, such as `https://notes.example.test`, preserve that Host, and send tokens only over encrypted connections. `bind` selects the actual listener. The app neither changes firewalls nor deploys a proxy. Non-loopback HTTP requires explicit `allowInsecureHttp: true` for controlled test networks; WebUI secure-context features still require HTTPS or loopback. Host and any Origin must match exactly; forwarded headers never grant authority. Local HTTP/browser tests do not establish cross-machine TLS deployment acceptance.

## Writing, conflicts and disconnection

The WebUI provides source editing, Typst preview within explicit authorized dependencies, title/source-text search within authorized configuration, exact historical source viewing, and saved-source download. Search explicitly covers configured readable sources, reporting incomplete results on budget exhaustion or read failure. It is not the multiuser implementation of CEL Query. Ordinary files without admitted note history report it unavailable.

Two users editing one document keep separate drafts. The first save commits; the other old-Base save is rejected, preserving the original proposal and current source. Reopen, inspect both, edit the merged proposal, confirm its new basis, then Save. Basis confirmation itself does not change the author file; Core retains the original conflicting draft. An unrelated document continues saving.

Input is immediately retained in the current tab's session storage, and asynchronously in the actor's server draft while connected and idle. Disconnection permits further typing without local commit authority or automatic save retry. Reloading while offline does not guarantee the app can load. After reconnecting, reopening compares a draft still present in that tab's storage. Closing the browser, private mode or clearing storage may lose input not yet sent to the service; storage write failure is reported. Different users, devices and browser tabs do not share this tab's recovery store.

Each save first retains its exact operation ID and submitted source. A lost response queries that same operation, including after server restart. Unknown/recovery-pending states do not become success or a new blind retry. A not-registered/not-committed result is durably settled before permitting a new explicit proposal, fencing any delayed original request. Receipt application preserves input typed after dispatch. Actor drafts and operation records reside under `actors/` in the existing private state directory, outside the document tree. A server backup must consistently capture the entire private state and document tree; the existing local library backup entry does not promise all actors' private drafts.

## Revocation and delivery

The administrator reads `GET /api/hosted/admin/policy` with its Bearer token, then posts the complete `{expectedGeneration, users}` policy. The update conditionally commits and persists against the current generation; stale updates conflict. Restart loads saved policy rather than restoring revoked bootstrap grants. This endpoint cannot change document paths. After first startup, initial configuration users no longer overwrite persisted policy. Change the administrator token by stopping the service and editing private configuration.

Authorization epochs are server-generated per actor. Reads, search, history, saves and every download chunk reauthorize. Preview freezes individually authorized explicit inputs before compilation and checks the actor epoch again before delivery. Frozen World never fills missing inputs from disk; compiler diagnostics containing absolute server paths are not returned. Revocation and save/chunk-admission decisions are ordered at the same short service boundary. Already admitted bytes cannot be recalled; further chunks stop. Old sessions/tickets cannot revive after regranting permission. Other actors whose authorization did not change retain valid save sessions.

Downloads freeze exact bytes, length and digest. Each request returns at most 64 KiB; later saves never replace the artifact. Tickets bind actor and authorization epoch, expire after 15 minutes or restart, and cannot be transferred to another user. The WebUI verifies the complete digest before delivering a file. Responses disable browser caching. An online idle UI checks authorization every five seconds, clearing old display, preview and tab recovery cache on detected changes or denial. Already received/exported content cannot be recalled, and offline browsers cannot immediately learn about revocation.

## Budgets, acceptance and remaining scope

Configuration permits at most 128 users and 256 files, with 32 open sessions per actor and 128 service-wide. Source is limited to 2 MiB. Preview permits 64 explicit inputs and 32 MiB of frozen files, reuses compiler process time/memory budgets, and allows one active preview. Search reads at most 16 MiB. Download staging is limited to 32 artifacts and 64 MiB. Exhaustion produces an error or incomplete result, never a subset posing as complete.

`scripts/test_hosted.py` is included in the existing `scripts/mvp.py check/package` entry. It exercises real-process concurrent conflicts, actor drafts, idempotence/restart, rejection of local-path APIs, hidden-object nondisclosure, history permissions, frozen compilation, exact resumed bytes and persisted revocation. Browser checks additionally cover login, lost responses, offline drafts, explicit merging and cache removal. Actual outcomes belong to the candidate record, not this document.

**Not yet connected:** same-owner manifest/blob synchronization, offline committing replicas and partial synchronization; hosted five-view rich editing, CEL Query/narrow-field permissions, annotation/proofreading/suggestion transactions, Office interchange/download, configuration/structure operations, media and revocation across all consumers. These remain v0.6 work; existing local features remain local. Their routes are absent from Server, preventing local APIs from bypassing hosted authorization. Realtime collaboration, other OS backends, Mobile/CLI whole-product acceptance and automation follow later milestones.
