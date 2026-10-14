---
source_language: zh-CN
translation_of: SERVER.zh-CN.md
translation_status: synced
---

[简体中文](SERVER.zh-CN.md)

# First bounded Server / WebUI slice

`0.6.0-alpha.1` adds a single-writer hosted native-document editing entry for Windows/NTFS. It reuses Core file guards, save journals and recovery evidence, with actor-owned drafts and original operation records. This is the first v0.6 slice; the complete scope remains in the [roadmap](ROADMAP.md).

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

The WebUI provides source, styled source, on-demand syntax, visual writing and reading views, Typst preview within explicit authorized dependencies, title/source-text search within authorized configuration, exact historical source viewing, and saved-source download. Search explicitly covers configured readable sources, reporting incomplete results on budget exhaustion or read failure. It is not the multiuser implementation of CEL Query. Ordinary files without admitted note history report it unavailable.

Two users editing one document keep separate drafts. The first save commits; the other old-Base save is rejected, preserving the original proposal and current source. Reopen, inspect both, edit the merged proposal, confirm its new basis, then Save. Basis confirmation itself does not change the author file; Core retains the original conflicting draft. An unrelated document continues saving.

Input is immediately retained in the current tab's session storage, and asynchronously in the actor's server draft while connected and idle. Disconnection permits further typing without local commit authority or automatic save retry. Reloading while offline does not guarantee the app can load. After reconnecting, reopening compares a draft still present in that tab's storage. Closing the browser, private mode or clearing storage may lose input not yet sent to the service; storage write failure is reported. Different users, devices and browser tabs do not share this tab's recovery store.

Each save first retains its exact operation ID and submitted source. A lost response queries that same operation, including after server restart. Unknown/recovery-pending states do not become success or a new blind retry. A not-registered/not-committed result is durably settled before permitting a new explicit proposal, fencing any delayed original request. Receipt application preserves input typed after dispatch. Actor drafts and operation records reside under `actors/` in the existing private state directory, outside the document tree. A server backup must consistently capture the entire private state and document tree; the existing local library backup entry does not promise all actors' private drafts.

## Revocation and delivery

The administrator reads `GET /api/hosted/admin/policy` with its Bearer token, then posts the complete `{expectedGeneration, users}` policy. The update conditionally commits and persists against the current generation; stale updates conflict. Restart loads saved policy rather than restoring revoked bootstrap grants. This endpoint cannot change document paths. After first startup, initial configuration users no longer overwrite persisted policy. Change the administrator token by stopping the service and editing private configuration.

Authorization epochs are server-generated per actor. Reads, search, history, saves and every download chunk reauthorize. Preview freezes individually authorized explicit inputs before compilation and checks the actor epoch again before delivery. Frozen World never fills missing inputs from disk; compiler diagnostics containing absolute server paths are not returned. Revocation and save/chunk-admission decisions are ordered at the same short service boundary. Already admitted bytes cannot be recalled; further chunks stop. Old sessions/tickets cannot revive after regranting permission. Other actors whose authorization did not change retain valid save sessions.

Downloads freeze exact bytes, length and digest. Each request returns at most 64 KiB; later saves never replace the artifact. Tickets bind actor and authorization epoch, expire after 15 minutes or restart, and cannot be transferred to another user. The WebUI verifies the complete digest before delivering a file. Responses disable browser caching. An online idle UI checks authorization every five seconds, clearing old display, preview and tab recovery cache on detected changes or denial. Already received/exported content cannot be recalled, and offline browsers cannot immediately learn about revocation.

## Budgets, acceptance and remaining scope

Configuration permits at most 128 users and 256 files, with 32 open sessions per actor and 128 service-wide. Source is limited to 2 MiB. Preview permits 64 explicit inputs and 32 MiB of frozen files, reuses compiler process time/memory budgets, and allows one active preview. Search reads at most 16 MiB. Download staging is limited to 32 artifacts and 64 MiB. Exhaustion produces an error or incomplete result, never a subset posing as complete.

`scripts/test_hosted_editor.py` extends the full `test_hosted.py` suite and is included in the existing `scripts/mvp.py check/package` entry. It exercises real-process concurrent conflicts, actor drafts, idempotence/restart, rejection of local-path APIs, hidden-object nondisclosure, history permissions, frozen compilation, exact resumed bytes and persisted revocation. Browser checks additionally cover login, lost responses, offline drafts, explicit merging and cache removal. Actual outcomes belong to the candidate record, not this document.

**Not yet connected to Server:** library-style configuration, CEL Query/narrow-field permissions and embedded queries, stable Node-link navigation, annotation/proofreading/suggestion transactions, Office interchange/download, configuration/structure operations, media and revocation across all consumers. These remain v0.6 work; existing local features remain local. Their routes are absent from Server, preventing local APIs from bypassing hosted authorization. Realtime collaboration, other OS backends, Mobile/CLI whole-product acceptance and automation follow later milestones.


## Same-owner remotely hosted synchronization

Server can additionally host one explicitly paired manifest/blob history. The operator supplies the original replica's WorkspaceId and genesis digest through `syncIdentity`. `users.<id>.syncOwner: true` explicitly grants full-history transport for that pairing and defaults to false. It is independent of document read/write/history/download rights; document authors, field-only clients and the administrator login credential do not inherit it. Do not give full-sync credentials to users who should see only selected documents. This mode trusts the owner's devices and hosting account; digests establish integrity, not protection against a malicious server or a device with stolen owner credentials.

The provisioning helper accepts `--sync-workspace-id ID --sync-genesis DIGEST --sync-owner USER`; USER must also be explicitly created through `--user`. The document list may be empty, allowing an existing empty root folder to serve an independent relay. Received records never directly install into root. If root is already a paired replica owned by the same Core private state, its identity must match. Authorized hosted author saves enter that replica's original outbox and are published when full-sync history is read; private actor drafts and policy are excluded.

Persistent policy also binds `syncIdentity`: existing full-sync grants cannot silently move to another pairing through startup configuration. Adding sync to a deployment that previously had none preserves its user policy; new bootstrap syncOwner values do not override it. Grant explicitly through the existing administrator policy API. Revoking syncOwner changes that user's authorization epoch; regranting never revives old tickets. Unchanged users retain their tickets.

In the local Sync panel's remote transport section, enter the service root URL and full-sync credential. Upload selected local objects, or inspect the remote inventory, select objects and receive them. Original clone/apply/merge previews still control author-file changes after reception. Received and installed versions remain distinct. Non-loopback URLs require HTTPS; literal loopback IPs support HTTP acceptance. The client validates certificates, refuses redirects and ambient proxies, and never takes credentials from URL userinfo/query parameters. Credentials stay in the current UI and active process memory, not browser storage or the author tree. Closing the panel clears its input; an already started transfer can continue. Stop prevents subsequent requests after the current block.

Transfers are idempotent by complete record digest, with 64 KiB blocks. Downloads freeze an exact record set; inventory pages contain at most 128 records. Every block rechecks actor, epoch and current full-sync rights, and newer commits never replace the frozen download. Upload buffers resume within the same actor/epoch/record; identical repeated blocks do not rewrite content. Only complete records passing digest and Core manifest validation enter private history. Wrong bytes never become author versions. Restart drops partial in-memory records and tickets while retaining complete durable records; a new transfer skips verified complete records. Transfer completion means records were copied, not that every commit is admitted or the working tree is installed.

Server permits at most four download snapshots totalling 256 MiB, and 32 upload buffers/completed receipts with a total retained budget of 256 MiB. Clients close completed receipts. Records retain Core's 64 MiB blob, 4 MiB manifest and graph limits. Tickets expire after 15 minutes, and revocation clears caches for the revoked epoch. The local program allows one remote task at a time for up to 15 minutes, retaining up to 64 task statuses for one hour within that process. Browser storage keeps only the task ID; a lost response queries the original task first. After process restart/expiry, explicitly start a new transfer and reuse complete records. Installation remains a separate durable original operation, so a network restart does not repeat author effects.

`test_remote_sync.py` joins the existing check/package entry and covers three real programs, offline conflicts/unrelated progress, partial replicas, block retry/restart, per-block revocation/regrant, unaffected owners, persistent pairing, non-expansion of old grants, redirects and response budgets. `test_remote_sync_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` uses explicitly configured Playwright for actual UI acceptance. Loopback HTTP/headless browser results do not establish cross-machine HTTPS, long latency, real network failures, malicious-transport defenses or other OS acceptance.


## Hosted authorization for native document views

All five views share the native SourceEditor, WritingController, source buffer and undo history. V02 decorates original syntax/headings; V03 reveals the same editor at an exact current construct; V04 edits only text, empty slots and structures whose unique origin is certified by both the final tree and official AST; V05 has no document-editing entry. Title, subtitle, formatting, list, ordinary table, math, code and link controls propose native patches. These remain drafts until the existing Save transaction commits. View changes never write files. Reopening the same retained source preserves Undo; switching documents or logging out follows the existing draft lifecycle.

`editor/map` and `editor/render` register exact session source/serial. `editor/edit` consumes that already observed source/serial/hash. Older serials, different text at the same serial and revived old addresses after equal-byte Undo are refused. After asynchronous work, the service rechecks the actor, session epoch and exact draft; edit proposals additionally require document write. Reader maps/trees have no write flags and readers cannot call the edit endpoint. The client also checks its current editor, source and serial. Revocation destroys the editor, structure dialogs, preview and tab cache; late results cannot reinstall them.

Compilation uses only independently authorized explicit inputs and rechecks dependency bytes before delivery. Only main-document origins can become edit ranges, with resource IDs replacing filesystem paths. Include origins remain read-only with original paths/raw ranges removed; metadata does not flow to other consumers through presentation. Hosted and local views share DOM element, attribute, image and style allowlists instead of injecting author HTML. Bundled offline fonts load from the same origin; failures are visible and preserve input. Compilation retains its single-active-worker budget. Manual read-only preview may briefly wait for a busy compiler; saves never automatically repeat.

`test_hosted_editor.py` adds exact-draft/ABA, cross-actor and reader denial, hidden/missing dependencies, virtual main origins/read-only includes, and in-flight revocation/newer-draft checks. `test_hosted_browser.mjs` covers the five views, Chinese/non-BMP input and Undo, original save conflict/lost-response/disconnection, input fencing on reopen and late rich edits after revocation. Automated keyboard input is not real operating-system IME acceptance. This integration does not claim Query, annotations, library styles or other hosted consumers are already connected.
