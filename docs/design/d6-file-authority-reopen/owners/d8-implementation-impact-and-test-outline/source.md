---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: a370f683-ad9d-468d-8a92-eb38b0fe872d.

# D8 Implementation Impact and Test Outline

Candidate status: D8-FA-r01 PL-IR-01 repair consumer; complete owner afterimage, not independently accepted, activated, or implemented. Fixed source S is 7e18168dad3e6d120fce0dd607dc10fa7894e252. Original evidence retains its original scope. This outline now consumes the authored stable-production-address/current-Observation repair but still requires independent review and does not close P1.

Revision: D8-FA-r01; candidate. These are subsequent implementation obligations. Bounded models from the original source remain historical evidence; this authoring batch performs only the document checks identified in its report. It does not claim an implemented Core, IME, renderer, assistive technology, or measured performance.

## 1. Implementation map and removal targets

```text
Desktop / WebUI / Mobile host input & accessibility adapter
              ↓ normalized edit / composition / selection events
Shared EditSession controller + private device draft journal
              ↓ explicit intent / draft projection request
Local Core or Server Core
  D8 document / annotation adapter → immutable D6 PreparedIntent
  D7 Field / native / collection / lifecycle Action adapter
              ↓ shared complete preview and effects transport
Explicit user confirmation → original D3 or D6 request
              ↓ original ledger / CAS / durable commit / replay
Committed source + receipt → invalidate / reopen projections

PL-IR-01 tests consume D6 PL01–PL18. D8-specific assertions include: same managed production version on another replica returns the same canonical documentRevisionToken with a different valid current Observation/sourceToken; watcher-gap requalification never mutates an old Draft/map; Annotation document_range can be freshly resolved and preserved under new authorization while Trash/live remains a separate lifecycle result; real production ABA or unproved rematerialization stays stale.
```

Core adds closed D8 read/project/text replacement/plain writing/prepare interfaces and reuses D2 parser spans, D3 Ref/Locator/Annotation values, complete D4 typed validation, D6 scope/budget/footprint/ledger, and D7 definitions/effects. It must transport the complete Draft Edit Map and implement grammar-complete EOF/sites, blank and multiline successors of raw regions, independent read-only origins, asynchronous input queues, and exact serial rules. Generated native-column/multirow Source batches remain disabled; this does not claim full D5 implementation. DraftProjection is a separate type: neither Draft ranges nor objects with Locators removed enter D2 wire.

The shared UI library implements the state machine, command routing, serial control, undo groups, target binding, selection-coordinate conversion, direction/layout epochs, and log recovery. Host adapters handle only OS/browser/IME/clipboard/AT differences. Core/UI boundary adapters must be headlessly testable.

Existing prototype Source/Write/Read, local IME guards, dir=auto, and physical CSS are migration/removal targets. Old source autosave, direct-API semantics, Task/Record/file authority, and parser branches cannot remain unchanged. Implementation updates consumers and bilingual public specifications together, without parallel new/old editor state machines. RTL never mirrors brand graphics or the logo.

## 2. Latency, budgets, and scale acceptance

These are product targets. Every claimed release host row records CPU, memory, OS, browser, font, AT, build, input corpus, cold/warm conditions, and raw distributions. No measurements currently establish these targets.

| Scenario | Target and measurement |
|---|---|
| Input, arrows, selection feedback | On the reference corpus, p95 from host event delivery to visible feedback ≤50 ms; continuous input loses or duplicates no characters; at least 1,000 events per group, three cold and three warm runs |
| Commands, prepare, network wait | Busy/unavailable feedback within 100 ms of command receipt; complete Core work is asynchronous and unknown submission is explicit; service time is not represented as a fixed success deadline |
| Cancelling recomputation/scrolling | UI updates within 100 ms of receiving cancellation; Core stops at declared work-unit checkpoints; durable commit is not cancelled; record actual Core stop latency |
| Direction/zoom/layout reconstruction | When idle, current viewport usable again at p95≤100 ms; defer reconstruction and retain the host during composition; animation is not completion |
| Large documents | Desktop/WebUI reference: 10 MiB and 100,000 logical lines; local Mobile: 1 MiB and 10,000 lines; additionally a single 1 MiB logical line, depth/capacity boundaries, and input ten times over budget |
| Large collections/navigation | 100,000 Nodes in navigation and a 10,000-row result; D5 fetched page≤200; selections across pages remain exact; rendered DOM rows are not the whole set; complete Query obeys D7 budgets |
| Memory/disk | Additional peak reference-editor memory: Desktop/WebUI≤256 MiB, Mobile≤128 MiB, including Draft/Base/Undo/viewport maps; account Core result spool separately under D6 budgets and include both in total application limits |

The reference corpus includes Arabic/Hebrew/CJK/Latin/emoji/CRLF and long valid D2 sources, not only ASCII whitespace. Scale never changes identity or grammar. Reject over-budget work completely before allocation/parsing or at cumulative accounting boundaries, or explicitly enter a declared Source/Read capability mode. Half-parsing cannot claim canonical validity. A release host missing its promised reference target cannot label that editor capability Supported. A lower-resource policy may declare a separate verifiable capacity row, without disguising failure.

Parsing/projection may be incremental only when its semantics, primary diagnostics, ranges, and rejection equal complete Core parsing. Otherwise run batch or withhold current Write. Viewport virtualization is separate from complete Core projection. Mount only visible rows plus bounded overscan, additionally pinning actual focus and the composition host. Explicit policy bounds the journal; memory pressure cannot silently discard unsubmitted Drafts or automatically author-commit them to shorten the journal.

## 3. Required testing layers

1. **Bounded models in the source:** scalar/UTF-8/UTF-16 and D2 newline mapping; exactly one local transaction for fixed IME traces; serial/preview/unknown-submit/replay state exploration; direction changes preserving typed intent; correct selection/targets under a fixed revision. These expose concrete errors in named classes, not complete Unicode, real UI, or Core permission correctness.
2. **Core implementation conformance:** actual D8 closed decoder, real D2/D4/D7 sources and Registry, complete D6 potential scope/footprint; two hidden worlds, error ordering, CAS/ABA/no-op; real byte pins, every EffectManifest page/epoch, lost receipts, crash recovery, and D3 creation/lifecycle paths.
3. **Unicode/layout implementation:** complete pinned Unicode18 BidiTest/BidiCharacterTest/GraphemeBreakTest/WordBreakTest data; platform shaping/font-fallback/dual-caret/wrapping fixtures; bidirectional correspondence between DOM UTF-16/native offsets and logical source; Bidi_Control inspection never contaminating clipboard/source.
4. **Real input and assistive technology:** Windows IME and Arabic/Hebrew layouts, macOS and every claimed platform, actual browser composition sequences; iOS/Android soft keyboards, dead keys, speech, handwriting, hardware keyboards; NVDA/VoiceOver/TalkBack with selected browsers. Synthetic events do not replace these tests.
5. **End-to-end scenarios:** every positive and negative Acceptance Matrix case on each claimed host, LTR/RTL, and applicable local/remote mode. Missing hardware/service means unaccepted, never a fabricated pass.

## 4. Subsequent stages and completion gates

D9 determines resource regions, rich clipboard conversion, TSV/CSV/Office mappings, and template outputs. D10 determines external capabilities, authentication contributions, and automation outside this generation. A2 reviews the complete D8-adapter/D7-effects combination and the product cost of explicit submission and the Write subset. This document does not start those stages.

D8 acceptance covers complete architecture and explicit later implementation gates. It requires real independent complete review and controller dispositions. The original source's Chat GPT-6 Pro designation is historical execution provenance; current review follows the human-authorized workflow. Local models and static checks do not turn implementation-pending into implemented. Any open architecture P0/P1 prevents freeze.

Additional method: a bounded oracle generates cases from D2 grammar input families and operation successors, not only prebuilt single-line paragraphs. Enumerate operations over blank text, EOLs, deletion of the last nonblank text, reopening without history, and selection direction. Source coverage separately tests navigation availability and editing eligibility for every body kind/Inline relation and repeated occurrence; null edit maps do not prove navigation. Real Core still requires complete D2 lexical/IR/source-origin, nested-parent, and protected-boundary tests. Every class outside the bounded oracle remains an implementation gate.

## 5. Implementation obligations for production-domain coordination

The wire2 paths for read, project, TextReplace, DraftWrite, prepare, and Undo bind Base to the same complete Observation. Updating document_read while leaving bare Counters in maps/queues is insufficient. Response, host-generation, source, and serial equality checks cover observer domain, complete production version, and evidence continuity. D8SourceTarget/2 is not an implicit upgrade of old D7 EntityTarget. Establish Registry/potential-scope/full-preview and actual metadata qualification before reading protected data and evaluating content. Source read/write does not implicitly grant domain commit_sequence_state.

Required real conformance includes: r1 from different production domains; managed/external transitions and external ABA; equal bytes with changed production history; observer-epoch reconstruction; late projection/queues never changing Base; log rebinding after TextReplace without a caret, grapheme merging, and input retention when proof fails; ordinary+strict success; all three trusted observed_only timing windows; rejection of weakening for noninteractive/Annotation/Undo; invalid never hidden by pending; preservation of the complete D7-definition restriction; true raw no-op/H versus equal-byte external admission; receiver-created CP3 observations; all production-version/current-observation/byte checks for Undo; recovery of old saved/planned decisions before new consumer gates; native D3 wire12 requiring no D6 fields; conflict-purpose inputs never becoming current D8 sources. Every scenario needs positive and negative cases, not token mocks in place of real owners.

Before implementation, complete actual end-to-end D7 PreparedEditBinding/2 D6/full production, current manifest/bytes, every page, and epochs. Missing producers cannot yield successful previews. D3 conflict lifecycle/physical installation-before revisions and current cross-replica Locator qualification remain named joint questions. The UI cannot add a compatibility ledger, arbitrary JSON, or guessed repair. These obligations do not remove any of the seven complete D8 groups or original 160 acceptance cases.
