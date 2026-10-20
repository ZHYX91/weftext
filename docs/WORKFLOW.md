---
source_language: zh-CN
translation_of: WORKFLOW.zh-CN.md
translation_status: synced
---

[简体中文](WORKFLOW.zh-CN.md)

# Development, acceptance and release

## Current stage

The earlier fixed design baseline was accepted; [REVIEW](design/REVIEW.md) §8 records bounded refinements and actual evidence limits. The **established 0.6.0 Windows/NTFS scope is complete**: local, [controlled Server / WebUI](SERVER.md) and v0.6 synchronization passed the complete HTTP matrix, affected browsers and real NAS HTTPS on the same final optimized executable. It is not formally published. See [MVP](MVP.md) for use/limits and the [roadmap](ROADMAP.md) for later scope. Real OS IME, complete assistive-technology and power-loss acceptance remain incomplete. The v0.7 multiple-client milestone and isolated execution routes below are future work, not implemented product capabilities. Archived prototype dependencies do not make current documentation checks depend on the old UI or npm audit.

The new `0.6.3-alpha.2` second-stage candidate covers the navigation/ordinary-writing, property/multi-Query and ordinary root-preset slices described in MVP and reuses the same product entry points and single candidate record. Actual clients must verify returned Draft/selection/Undo, cancellation/stale maps and revocation cleanup. Recent visits, navigation and retained editors are page-local personal state, not author data or new authority. Original 0.6.1 IME/Undo evidence limits remain.

The current candidate has advanced to stage three, `0.6.3-alpha.3`, adding link diagnostics and stable-fragment potential mentions; MVP records actual support and original acceptance limits. Integrating the merged Windows guard, product CI and hosted-session cleanup fixes requires product checks on the integrated code; earlier candidates' passing records do not automatically establish this integration's acceptance. Hierarchical tags remain a target design, with graphs/heading structure still deferred.

## Routine checks

The working tree has completed the initial `0.6.2` Windows local/self-hosted material-import and recognition scope, providing a local delivery candidate qualified by complete checks, actual browsers and real Word chains. The prior 0.6.1 local delivery is preserved; final-candidate real OS IME and the original Undo anomaly retain their original evidence limits. A later version does not rewrite them. New recognition requires original preservation, offline models, quality, partial coverage, field mapping, Core operations and authorization/sync/revocation checks on the same optimized artifact. Simulated input, reference software and component passes do not replace the actual product chain. See MVP for support and outstanding items.

The second stage additionally verifies preset cancellation/original-operation recovery/no reset, independent occurrences, exact arrangement/removal and one-step Undo, retained typed errors, shared current/pinned calls, separate direct-field write grants/revocation and a two-program ordinary-root sync roundtrip. `scripts/test_query_organization.py` joins the existing complete check entry; `scripts/test_query_organization_browser.mjs` explicitly receives the same candidate executable and an owned evidence directory. After source freeze, HTTP, actual browser tasks and package audit consume the same optimized executable.

For prose changes run `python scripts/check.py`. For schema, type/time algorithms or model changes add `--models`. Implementation slices add their own formatting, static analysis and meaningful tests. Choose platform, filesystem-failure, authorization, editing and Office scenarios based on actual impact. Record failures honestly rather than weakening assertions or expectations.

Pull requests and updates to main also run [Product checks](../.github/workflows/product.yml). The Linux job installs locked frontend dependencies, then runs TypeScript tests, type checking and the Vite build. The Windows job uses pinned Node.js 24.19.0, Rust 1.99.1 and Python 3.11 with the existing `python scripts/mvp.py check` entry point. It builds once, then checks fonts, Rust formatting/Clippy/tests and actual-program local, hosted and synchronization HTTP scenarios. Design checks remain independent, and product jobs have no allow-failure or assertion-skipping fallback. CI does not publish or replace product acceptance for real desktop input, system IME, assistive technology, NAS or other operating systems.

## Choose environments by impact

Routine changes favor engine tests, HTTP and available headless/background browsers to avoid taking host focus. Select the smallest relevant scope below from the change and actual consumers. A narrow viewport does not establish Mobile support; HTTP or DOM value assignment does not establish real IME, touch or assistive-technology interaction.

| Impact | Preferred execution environment | Evidence boundary |
| --- | --- | --- |
| Algorithms, Core, Query, save protocols and APIs | Existing repository engine tests, fault injection and actual-program HTTP scenarios; start the local program with `--no-browser` | Establish the exercised algorithms, APIs and fault models; process interruption is not real power loss, and Windows/NTFS results do not extend to other filesystems. |
| Browser layout, five views, drafts, navigation and resource presentation | A headless or separate background browser exercising actual rendering and browser interaction against the same candidate | Record the actual browser and cases; frontend unit tests are not browser acceptance, and simulated browser input does not establish OS IME or complete AT acceptance. |
| Windows installation/update, system file integration, real IME and assistive technology | Isolated Windows Sandbox or a separate Windows VM with actual input, capture and normal-close evidence | Observe and act inside the guest, checking artifact retrieval and host focus isolation. A running Sandbox does not establish minimized, viewer-disconnected or unattended operation; use a capable VM for reboot or persistent system-state cases. |
| Linux/macOS desktops and their file backends | A separate test machine or suitable VM running the corresponding real OS | Separately verify installation, file semantics, keyboard/IME/AT, fonts and close/recovery. Windows, container or WSL command-line results do not establish that desktop's acceptance. |
| Android/iOS Mobile | The corresponding Android emulator/iOS Simulator, with necessary physical devices for cases the simulated environment cannot prove | Separately verify system file access, real soft keyboard/touch, lifecycle, offline use, fonts and platform AT. A browser viewport, simulator or the other mobile OS cannot substitute for required physical-device cases. |

Distinguish three states before execution: **declared** means a tool or document lists a capability; **available** means this environment, session, input and capture actually work; **observed** means a specified case has produced an observed result on the specified candidate. On first integration or changed environment capability, use a small case to confirm isolated input, observation, normal closure and result retrieval, recording any effect on host focus, mouse or clipboard. Later runs reuse valid environment evidence while checking the current identity, session and affected capabilities. This qualifies only the execution environment, not the product. The repository currently records no Weftext product evidence for these Sandbox, VM or mobile environments.

Stop the case on startup failure, a locked guest, unavailable input/capture or a stale session. Record unverified or tool error explicitly, without silently falling back to the host foreground. If startup/reconnection or a manual case needs foreground interaction, tell the user the required action and expected duration in advance and schedule a short window; sustained acceptance input remains bound to the target environment. Rerun only affected scope after failure, without changing product expectations to preserve background execution.

## Reproduction, regression and candidates

Issue reproduction retains the original candidate, environment and minimal trigger steps, recording actual observations; reproducing one defect does not pass a complete case. After a fix, use a new candidate for the counterexample and directly affected normal paths. Reuse passed results when implementation, dependencies and environment are unchanged and their identities are confirmed. Release acceptance separately checks promised platform scope, outstanding unverified cases and candidate integrity; reproduction or environment qualification cannot establish a release pass.

Reuse repository build/check/package entry points. Build each platform's candidate once per attempt, then use the same artifact for background, guest and necessary manual cases. Current `python scripts/mvp.py check` and `package` both build and check first; choose one according to whether packaging is needed, rather than chaining `build`, `check` and `package` into repeated builds. Complete supplementary acceptance on that packaged candidate after `package`. Source, dependency or package-content changes create a new candidate with affected checks; do not transfer the old artifact's pass. Other platforms do not yet have delivered candidate entry points.

Keep only one short acceptance record per deliverable slice: commit/candidate identity (artifact hash when needed), target OS/browser/guest or device versions, change impact and cases, actual commands/manual steps, passed/failed/unverified/tool-error results, limitations and necessary evidence locations. Separate environment qualification from product results. Retain necessary screenshots/excerpts for failures or key actions and summarize routine logs, without another acceptance fact store. Keep real inputs separate from generated output and never overwrite originals. Link checks and bounded models are not product acceptance.

## Release

`python scripts/mvp.py check` checks the current local implementation; `package` generates a local candidate with source and notices without uploading or publishing it. Release the same accepted artifact without rebuilding different bytes. Establish version, platforms, failures and user authorization before publication, then verify the download and a minimal runtime scenario.

The repository owns independent product build/check/package entry points. The personal workspace only supplies paths and local environment, without another release platform. Hash artifacts and material handoffs, not the entire repository for every discussion. Tags and releases correspond to actual deliveries; do not create an empty release now.
