---
source_language: zh-CN
translation_of: EDITOR.zh-CN.md
translation_status: synced
---

[简体中文](EDITOR.zh-CN.md)

# A new-project design for Typst editing, reading, titles and Word

Status: design candidate, neither accepted nor implemented. This file defines shared editing, flow reading, title and Word-output rules. [REVIEW](REVIEW.md) records experiments and actual evidence limits.

[SOURCE](SOURCE.md) owns source/type/origin interpretation, [CORE](CORE.md) identity/basis/authorization/unified transactions, and [FEATURES](FEATURES.md) links/Query/templates/conversion. Current outcome requirements come from [REQUIREMENTS](../typst-native/REQUIREMENTS.md). These new owners coordinate with this proposal; prior-round mechanisms are not normative dependencies.

## 1. Choices and product commitments

Official Typst syntax is the sole body source. Ordinary writing, continuous reading, source editing, paged preview and field editing share one document session. DOM or a separate rich-document JSON never replaces `.typ`. Preserve complex legal source. A locally irreversible construct gets a clear source entry and official layout preview; it is never flattened or discarded. Ordinary documents must support writing and flowing reading.

The revised candidate uses the official Rust parser/compiler, the official HTML target's finally consumed tree, and bounded style/role adaptation plus origin qualification. The official Paged target supplies pagination and printing. Reading and printing use explicit different native targets; deriving flow from the same Paged result is not a user requirement. Official HTML remains experimental. This work validates a controlled integration, without claiming stock HTML or a product reader is ready; see §8 and REVIEW for acceptance scope.

Core remains the sole owner of structure, identity, authorization, commit and recovery. This document consumes authorized SourceSnapshot, ReadBasis, unified Plan/Journal/Receipt and Job abstractions without redefining their persistent formats. A Core change requires coordination of actual consumption here, beyond renaming an interface.

## 2. One session and one current editing basis

Authorized `readEntity(ref,current,purpose)` / `readScope(scope,purpose,completeness)` return identity, exact original bytes and ReadBasis. The latter includes actual revision, observer incarnation, object generation, schema/auth and the operation's actual dependencies. Equal bytes do not imply equal qualification. A session holds Base and ReadBasis, mutable Draft bytes, monotonic draftSerial, selection, input generation and one DraftUndo. Local edit basis is `(session, owner, ReadBasis, draftSerial, exact Draft bytes, Environment)`; a digest is a convenient check, not a substitute for exact bytes or fresh authorization.

Successful local source, rich and field-panel operations change the same Draft, advance serial and enter the same undo stack. A batch paste, format command, table operation, field change, IME composition or run-in toggle is one visible undo unit. Mode changes, layout, reads and compilation are not undo units. Undo/Redo obtain a new serial and map, without reviving old write addresses. Another document in a split pane has its own session; returning restores the original draft and position.

Source changes remain proposals. Save passes the complete proposed source to Core `prepare(operationId,intent,draftBase,budget)`, inspects the actual plan and calls `commit(operationId,planHandle)`. Unknown results retain the draft and use `operationStatus(operationId)`, never a guessed retry with a new ID. Repreparing changed content after a terminal conflict creates a new operation linked to the original Draft. External changes and sync use observeExternal/admitExternal/merge; the editor exposes conflicts and retains both drafts. A changed current revision never silently replaces a local Draft.

Commit success decides only that operation. After a fresh authorized read of after, Base may advance; continuing user input prevents after from replacing Draft. Only exact Draft/after equality makes it clean. Undo of committed content is a new transaction, allowed only when current revision and bytes still equal one's own after, authorization is fresh and the inverse excludes another author's change, control changes and multiple-entity effects. Otherwise preserve the draft and show the conflict. Core recovery protects crash drafts; these are neither saved bodies nor cross-device permissions.

### 2.1 Shared implementation boundaries for five views

Expose REQUIREMENTS §5.4's V01 Source, V02 rendered with syntax, V03 syntax on demand, V04 syntax-hidden UI editing and V05 Read. They share session, Draft, serial, selection and DraftUndo; view choice is personal UI state. Successful V01–V04 edits update the same buffer. V05 disables body-edit command routing while independent annotations retain their own authorization. Continuous/paged layout and target selection are separate and do not replace a view.

V01 uses the exact source buffer with official-parser highlighting. V02 still draws the complete source buffer in source order, retaining every author-syntax character and exact selection positions while styling recognized headings, emphasis, lists and similar constructs. Markers are separate, always-visible decorations. Compiled semantic styling binds current serial/origins; invalid source remains editable with parser styling and explicit unavailable-preview status. Never deserialize HTML or reconstruct Typst markers to build V02. Generated results may have an explicit auxiliary preview but cannot replace, duplicate as body content or remove their author expression.

Rendered portions of V03/V04/V05 consume §8's same current final projection. V03 reveals a linked source editor at an active construct with §3's unique-origin proof and directly edits the same Draft's exact range; revealing syntax changes no source. Keyboard commands and touch controls enter/leave explicitly. Generated, multi-origin or included occurrences without a local writer stay read-only with real-source navigation; matching strings cannot select origins. Failed/pending projections retain Draft and expose explicit V01/V02 entry, never showing stale output as currently writable.

V04 uses the same writer qualification and §4 named operations, without syntax markers or automatic switching to V03 on focus. Specialized controls handle supported constructs; otherwise retain occurrences read-only and let users explicitly switch to a syntax-visible view. Hidden syntax does not imply arbitrary inverse editing. V05 selections support reading/copying/annotations; typing or pasting cannot activate body editing.

Switching first completes or retains IME under §5, then transfers selection through current source/projection associations. Without a unique display position, retain the original source selection and restore the closest proven container/reading position rather than change the source point to fake a mapping. Entering V05 in a session defaults to an explicitly labelled preview of the current Draft. Committed content requires explicit selection and cannot replace an unsaved Draft. Returning to editing resumes the same DraftUndo. Personal view preferences do not modify author source. V02/V03 syntax decorations are excluded from body Query, logical copying, visible annotation excerpts and export; explicit source copying still returns authorized original bytes.

## 3. Producing writable regions

The current complete official AST and exact bytes produce a revision-scoped edit map. Each writable region must establish a unique root construct/content slot, exact UTF-8 half-open range, unshadowed standard binding, current show/set scope, reversible display/source association and the complete range the operation may change. A Span, AST path, label, array position, equal string or rendered coordinate alone never grants write qualification. Official AST supplies syntax and ranges; data flow and evaluation observations supply association and dependencies. Both are required.

| Origin | Reading and queries | Current rich editing |
| --- | --- | --- |
| Unique root literal or adapted standard construct | Official actual output | Proven text slot or named structural command |
| Variable/function/loop/show output | Actual occurrences and truthful origin | No default address; explicit Edit Source |
| Included/imported content | Authorized source consumption | Read-only here; open the source in a new session |
| Repeated use of one origin or combined origins | Every actual occurrence remains readable | No inverse map means read-only; never choose an arbitrary origin |
| Opaque layout or uncertain reading order | Bounded layout island and truthful state | Source entry; never reverse-write text from an image |

Dependency closure is regional: a nearby macro does not disable an independent ordinary paragraph. An unprovable global show that affects all body content can make rich editing read-only, with the actual reason, preserved Draft and Source entry. This outcome on a basic document is still a basic-capability failure. Generated headings retain actual level and query occurrences; lacking a write address does not erase them.

The official AST must produce a body sibling insertion boundary for an empty document, control/comment-only document or deletion of the last paragraph, and a zero-width insertion site for an empty list item/content slot. Whitespace in comments, labels, metadata or expressions never becomes a body site. Core/shared Rust adapters recompute current boundaries and capabilities; a stale site fails as a whole even at the same byte offset. Convert bytes, UTF-16, scalars, graphemes and visual stops explicitly; do not split CRLF, combining sequences or ZWJ. Hosts use the same versioned Unicode implementation and its tested matrix, rather than inheriting an unproved version number from old documents.

## 4. Minimum editing matrix and source-preserving algorithms

Named intents enter shared Rust adapters, returning complete new Draft and a proved selection. Apply minimal splices within the operation's target; preserve every outside byte, comments, whitespace, EOL and authored calls/arguments. No innerHTML serialization or whole-file formatting.

| Required operation | Native representation and boundary |
| --- | --- |
| Ordinary typing/deletion at any caret, Enter, join, empty→type again | Current body slot with native literal escaping; pasted `#`/`=`/`-` do not execute code or structures |
| Strong/emphasis/strike and selections across formats | Standard constructs; split/merge the paragraph's inline tree, preserving unselected leaves and trivia |
| Document title/subtitle, section heading, level change | §6 roles and a single source value; positive actual levels, ordinary 1–9 controls and deeper numeric entry |
| Create/edit inline/display math | Local native math-source editor and official preview; visual math is an atom, never reverse-inferred from glyphs; delimiter escape cannot add code |
| Create/split/join/indent bullet/enum/terms items | Native list constructs, including a real empty item at an empty body; a bare marker is not success |
| Insert/convert links | Link owner's encoder/resolver; source write and target read separately qualified, preserving label, fragment and target |
| Ordinary grid cells, add/remove rows/columns, row reorder, rectangular TSV | Proven fixed columns and explicit complete span-free cells; any failure leaves the entire operation unapplied |
| Local content in headers/spans/block cells | Preserve topology; unique slots may be edited, complex topology commands use Source/advanced entry |
| Source editing of arbitrary legal Typst | Exact source independent of controls; report separate validity and save outcomes |

Deletion and plain paste across formats and ordinary paragraphs/list items use one structural command covering the whole range and one undo unit. Cross-heading/title/table/code/math/generated-island writes without a named algorithm fail with an exact Source selection; reading copy remains available. Copy Text follows logical order with explicit block separators. Copy Source Fragment returns authorized exact original slices, never fabricated concatenated source. Cut modifies Draft only after successful OS clipboard write. Default paste reads and escapes text/plain; absence of that flavor is unavailable. Source paste is explicitly a source-mode operation.

Generated/spread/ragged/span/header/footer/custom-show tables remain intact, without flattening into rectangles. Equal cells are distinguished by current AST slots. Simple-grid row reorder must prove trivia ownership: cell-internal comments follow the cell, inter-row leading trivia follows the next row, trailing trivia remains before the closing delimiter. Ambiguity refuses that reorder. Rectangular TSV paste explicitly previews dimensions/extension and validates its entire budget and permission set once; no partial paste or split to bypass limits.

A successful adapter fully reparses through the official parser, proves the requested structure/value, unchanged outside bytes and non-target controls/fields/constructs, and uniquely locates the caret. Visually qualified operations affected by active show re-evaluate rather than treating AST equality as correct display. Rich editing outside an error island requires full boundary/dependency proof; an error swallowing later source does not justify guessed safe partitions.

## 5. Input and late projections

Native controls immediately display pending local input without waiting for full compilation on every keystroke; the shared source buffer remains the only Draft. Parser/compile/layout run in workers. Each controller has at most one structural transform in flight, freezes its old basis and target and retains subsequent input in order. A response installs only at its applicable serial/generation. If newer input exists, recompute only a provably replayable queue; otherwise retain the new draft and unapplied text for source repair. Old responses cannot replace newer source, selection or focus, or mark it clean. Returning last does not confer authority.

IME begin/update are native preedit; composition commit creates one Draft operation and Undo unit. Deduplicate host final-input events and retain recovery opportunities on cancel. Save/Enter/slash/format/link conversion, background switches and virtualization must not seize the composition owner or prematurely commit it; test focus-loss behavior in actual hosts. Never recycle active/focused/composing nodes. Authorization changes or adapter failures preserve user-entered recovery text while preventing protected original content from reappearing through caches.

layoutEpoch describes geometry, not source serial. Unverified current semantics are pending. Explicit old previews carry version labels and no current edit/annotation target. Report syntax, application-control/type validity, compile, layout, budget, permission, save and projection freshness independently. Syntactically legal compilation failures may be saved with diagnostics once Core validates current format/control/fields. Syntax-invalid content goes to recovery Draft if Core has no formal checkpoint capability; it is never falsely reported as normally saved. Exact source export is independent of compilation.

## 6. Unambiguous title and subtitle roles

Use official title for the document and official heading for sections. Titleless documents are legal; file names, paths and snippets are UI fallbacks that never add author title. Default new titled documents keep one document.title value and render with `#title()`; the field panel edits that source value and export reads it. Direct `#title[Title]` remains legal: without document.title, the unique direct title.body can be the title source, without mandatory duplication. If both differ, expose metadata/display titles separately with no claimed lossless unify button, preserving source. Generated titles can render and expose actual values; panel writes require a unique proven source address. Duplicate title/ambiguous role is an application diagnostic, never grounds to delete legal source.

```typst
#set document(title: [Research Design])
#title()
#block[A qualitative and quantitative study] <weftext-subtitle>

= Background <background>
Ordinary body text.
```

The subtitle is a native block with the unique `<weftext-subtitle>` label. The application associates it with the unique document title as separate frontmatter, outside title.body, heading counters, outline, numbering and section depth. No inference from colon, size or proximity. Panel creation requires a unique title. Removing title previews either removal of the associated subtitle or retention as an ordinary block without its role label, in one operation. A labelled block without a title stays in source with a missing-association diagnostic; it never becomes a section. Duplicate labels, a non-block or uncertain association produce no subtitle writer. Authors control styling; no mandatory size or spacing parameters. Templates consume the same title/title()/block rather than another business title. Native subtitle links remain possible without making it a section reference.

This is a proposed light application convention requiring SOURCE/schema coordination; the label denotes role, not persistent identity or permission. Official title guidance distinguishes title from top-level sections and keeps frontmatter outside title.body. Subtitle association is this application's addition. [Typst title](https://typst.app/docs/reference/model/title/)

## 7. Per-heading mixed layout and full depth

Each section preserves its native heading, actual level, original counter, outline and label references. Beyond ordinary 1–9 controls, legal 10+ levels remain readable/queryable/navigable and source-preserved. Numeric depth is a positive integer, never capped to H5/H6/H9. Rendering uses heading role and exact aria-level rather than a limited HTML tag model. Size is separate from role. [Typst heading](https://typst.app/docs/reference/model/heading/)

Standalone/run-in choice writes an authored native labelled show rule per heading. Reuse a unique author label; otherwise add a nonconflicting ordinary label visible in the Source preview. It targets that heading, without forcing an entire level through heading.where. Default rules provide standalone layout; run-in emits the original counter and body followed by an ordinary space, joined to the immediately following native first paragraph. No second counter, hidden block ID, spacing dimension or shared presentation sidecar.

```typst
#set heading(numbering: "1.1")
#show <methods>: it => context {
  strong[
    #if it.numbering != none {
      counter(heading).display(it.numbering)
      text(" ")
    }
    #it.body
  ]
  text(" ")
}

== Background <background>
Body under a standalone heading.

== Interview Methods <methods>
Run-in body with *authored emphasis*.

== Survey Methods <survey>
Standalone body. Numbered reference @methods.
```

The writer uniquely recognizes and reverses its controlled native rule. Enabling proves an eligible following ordinary first paragraph and exact scope. If a blank paragraph intervenes, preview removal of precisely that boundary. Never swallow a list/table/another heading or unknown construct; explain and let the user choose eligible body. Preserve unknown show rules, resets/custom numbering or template-owned targets, rather than overwrite them. Only proven native rules get controls. Disabling removes only this rule and restores required paragraph separation, retaining any added label so author references remain valid. Repeated toggles do not duplicate rules. Saving/sync/recovery transport `.typ`, giving cross-device authored presentation.

Insert/delete/move/level changes retain the selected label and native counter, without literal numbers. Outline consumes heading body without run-in body. Numbered references and label links are different. The experiment found default @ref failure for an unnumbered heading: offer explicit native label links rather than claiming an automatic fallback or silently rewriting authored references. Mixed numbering has executed limited evidence; deep levels, long text, orphan lines, non-Latin scripts, pagination and interaction still require acceptance.

## 8. Native dual targets: final consumption, styles and origins

Core `compile(sourceSnapshot,environment,target,budget)` supplies a fixed official Rust build, fonts/packages/assets and an authorized read-only World, returning that CompileSnapshot. Ordinary continuous reading selects official Html; paged preview/PDF/printing select official Paged, without a new author language or invented target. Every artifact binds its actual source, environment, target, dependencies and authorization. Identical source does not make two targets' text, counters or read sets identical. Switching target is a new compilation, without substituting another result or reviving old write qualification. [Official target](https://typst.app/docs/reference/foundations/target/)

**Final consumption is the entrypoint.** Starting at the official `HtmlDocument` root returned by successful compilation, traverse only actual `HtmlNode` child edges in order and assign projection-local occurrence keys. Do not enumerate every evaluation/realization event, a Tag's original body or Introspector elements as displayed content. Discarded measurement links do not become RenderedLink; one displayed table cell is not multiplied by measurements; genuinely repeated source output remains repeated. Native final nodes are reading-structure inputs and serialized HTML is a delivery format; neither is another authored source.

Three products remain distinct. AuthoredLink comes from recognizable authored constructs. RenderedLink, body search, logical copying, visible annotation excerpts and offline link routes consume the selected target's final projection. The heading product consumes that compilation's final native Introspector with actual level/label/counter and a display association or explicit absence. A show-deleted heading may remain in introspection without becoming visible text. Native HTML h2 and similar tags cannot determine business heading levels: use actual element semantics and consumption association. Arbitrary show output preserves the actual return without imposing the entry strong/heading role on returned plain text.

**Bounded native-style adaptation.** Stock HTML primarily supplies semantic structure; successful export does not establish preserved native fonts, sizes, list markers or table borders. At actual official HTML element production points, the adapter consumes the existing effective StyleChain/evaluated parameters and converts named representable properties into controlled CSS, roles and origin side information carried by the resulting nodes into the final tree. It must not scan authored set text to reinvent precedence, rerun show/query/context for styles, or match text against all stable logs. Basic body/list/table library defaults, note rules and local overrides require experiments. A versioned support table states the limits; unknown units, paints and custom layouts are not silently ignored. This is internal target conversion and requires neither authored CSS nor a second style configuration. [Official HTML scope](https://typst.app/docs/reference/html/)

Minimum rendering still supports continuously scrolling and 320px-reflowing ordinary Chinese/English/RTL paragraphs, title/subtitle, headings, strong/emphasis/strike, lists, raw/code, links/references, image alt/caption and simple/header/span tables. Only wide two-dimensional content scrolls locally. Basic math preferentially consumes official MathML with real AT validation; necessary native frames/SVG retain accurate mathematical or textual alternatives, without claiming image-only accessibility. HTML tags/attributes/styles/resources pass safe typed conversion. Scripts, ambient host files/network and uncontrolled event handlers do not execute; affected legal source is preserved with explicit display limitations. Only Core-authorized resource handles are available to the renderer; HTML URLs cannot bypass World/resource delivery authorization.

**Local layout and target boundaries.** Page counters, absolute placement, floats and complex figures are not generally equivalent to reflow. Explicit native `html.frame` invokes Paged within the same Html compilation and its internal `target()` returns paged. Record that boundary and consume the official returned frame instead of cropping coordinates from another whole-document PDF. Content safely representable by a local frame in the current official call chain may become a bounded layout island with alternatives. Without a legal local alternative, report Flow unavailable for that region or document while preserving Source and an independent Paged preview. Ordinary paragraphs, lists and tables cannot all become images or a whole PDF and still satisfy ordinary reading. Preserving arbitrary legal layout source and available official output does not imply lossless reflow for every native effect.

**Origins and validation.** Where final nodes carry verified native source/construct associations, combine them with §3's current official AST, unique authored slot and actual show-scope proof to grant rich-edit operations. A span, matching string, DOM position or single occurrence is still insufficient. Computed strings, repeated output, dynamic replacement and includes retain real occurrences but only source navigation when no unique inverse exists; never infer origins from PDF/screenshots. Distinguish the real note within the transparent library entry from user-authored includes. The host wrapper must neither disable ordinary body editing nor enlarge imported-source write authority.

The original [eleven observer cases](probes/semantic-bridge/README.md) retain their bounded no-side-effect/cache-replay results, but independent measure/table counterexamples disprove that stable event unions form the reading tree. Later final PDF-consumer work improves selection but still lacks reliable association with pre-layout paragraph separators, so this candidate withdraws Paged-trace reconstruction as the default flow route. The new route requires bounded evidence for the final HTML tree, genuine repetition, discarded measurements, tables, library-style overrides, target/frame, counters/dependencies and read-only origins. An experimental label alone neither disqualifies it nor makes a few successful compilations complete E1. Pin compiler/adapter versions and rerun relevant upgrade regressions; maintenance must not expand into another Typst evaluator/layout engine. REVIEW records current experiments and independent verdicts.

### 8.1. One ordinary module and an automatic compilation entry

Selected route: ordinary `.typ` module bytes own library style; Core library settings select only the authorized module EntityRef and enable state. No duplicate font/border fact table or style DSL. Settings controls edit recognized native set/show-set arguments through the module's own Core Plan. Advanced source retains full legal styling; unknown rules remain intact and disable only controls that cannot edit them losslessly. No whole-file normalization.

Core automatically constructs a viewable temporary entry for every compile, **without requiring imports in every note**. It imports the selected module, applies its standard content-wrapping function and includes authorized realnote. Entry is derived input, never written into the note, with no note identity or second body authority. Prevent implicit duplicate module application. Explicit authored imports retain their additional native scope with explanation; never guess deduplication by deleting author source.

```typst
// library-style.typ: module shape; distribution defaults also implement §8.6 fonts/language
#let library-style(body) = {
  set text(font: "New Computer Modern", size: 11pt)
  set list(marker: [•])
  set table(stroke: 0.5pt)
  show list: set text(size: 10pt)
  body
}
```

```typst
// entry.typ: generated by Core and viewable
#import "library-style.typ": library-style
#show: library-style
#include "notes/realnote.typ"
```

The function name/call profile is application integration, still ordinary Typst rather than a new language. Library initialization can provide a default module; users configure it once. Do not silently add imports to each note. Explicitly opting out selects a compile profile without the wrapper, with the actual environment shown. A broken/unauthorized selected style reports layout unavailable while preserving Source/Draft, without silently substituting a different version/default.

### 8.2. Native semantics without a universal cascade

Prefer composable set/selector show-set defaults. Avoid putting default fonts/borders into transformations that reconstruct elements. Notes use set/show-set; block-local rules end with that block; explicit table stroke parameters can override defaults. Scope, selectors, order, explicit element parameters and transformations follow official evaluation. There is no invented CSS rule that all note rules defeat library rules or all local rules defeat everything. A general text set is not promised to override every library selector show-set; corresponding selectors or explicit local element properties may be needed.

Official guidance separates composable show-set from transformations because set inside a transformational show is no longer overridable by later show-set. This experiment reproduced that counterexample. Controls disclose opaque/fixed transformations and their impact; advanced native transformations remain legal and source-preserved. [Typst Styling](https://typst.app/docs/reference/styling/)

List marker/font/body and table text/stroke/fill/inset use their standard native properties and selectors. Expose only controls with clear native AST/source writers. Default changes write module, per-note changes write note, local changes write that scope, each with one explicit DraftUndo and Core transaction. Preview affected scopes and honor authored explicit arguments. [Typst Table](https://typst.app/docs/reference/model/table/)

### 8.3. Source identity, relative paths, editing and reading

SourceSnapshot and application identity/field qualification always come from realnote's original bytes/official AST. Only that owner's source defines controls. Metadata/id/title-like values in wrapper/module cannot create notes, override control fields or grant permission. Do not rescan synthetic-entry AST for identity or turn notes into new source objects.

The entry source map declares designated authorSource=realnote. Only a Core-generated entry proving exactly one inclusion of the current note lets the bridge treat this system include as the session's authored source. Rich editing still validates realnote's current AST, ReadBasis/Draft serial, actual show scope and unique reversible origin. Authored include/import remains another source; repeats/generated/multi-origin content grant no note writer. Neither make every realnote read-only due to the wrapper nor use this exception to make arbitrary includes writable. This distinction remains an implementation gate.

Relative read/image/import/include/bibliography paths resolve from the actual source FileId/location: note read("asset.txt") uses the note directory, module-relative assets use the module directory, never the wrapper location. Core World provides authorized exact bytes through controlled original-path mapping. Native relative meanings after a note move follow existing link/resource semantics without silent rewriting. Absolute/package paths also obey World policy, without ambient access/network. Style controls presentation. Personal dark/zoom/high-contrast belongs to reading preferences, outside authored module/default export environment; accessible reading adjustments disclose their difference from paged output.

### 8.4. Frozen dependencies and style revisions

CompileSnapshot/Environment/DependencySet record actual library selection, style RevisionId/bytes, entry profile/digest, realnote/Draft, actual module imports/packages/fonts/assets and target/budget. Unauthorized styles cannot continue through cached bytes; packages never silently use latest. Source validity/save remain independent of style compilation and errors preserve notes.

A style save creates its RevisionId without fake per-note revisions or changed note AST/ID. Module/selection changes make dependent previews/layout maps/visual editing qualifications/compiled Query/output preparation stale. Rebuild actual effects with a new read/compile; unrelated static fields that did not consume style need no invalidation. Explicit old artifacts remain old, never current. Late v1 output cannot install as v2 or replace Draft/Undo/selection. Font/asset/module currentness changes behave likewise.

Core acceptance must actually freeze v1 compile, commit v2 style, deliver late v1, reject current installation and compile v2, with note revision/bytes unchanged. Test revocation/missing modules/wrapper errors/target changes separately. CLI proves changed real outputs, not Core invalidation or current authorization.

### 8.5. Exact source export and independent use

Exact note `.typ` export returns originals and discloses absence of library style. A separate standalone Typst project bundle/entry includes authorized original notes, fixed style/imports/assets, viewable entry and required environment/package/font/license/loss information. Preserve source-relative paths or a fully validated mapping. Compile entry, never falsely present wrapper as original note. Deliver only authorized complete dependencies; unavailable distributable fonts/packages/incomplete closure need named loss/unavailable rather than claimed lossless independent reproducibility. A lone note cannot promise identical style.

PDF/editable DOCX/other presentation outputs consume the same frozen style dependency. DOCX separately validates effective-style mapping: native defaults may map to template/paragraph/run/table properties, but arbitrary show is not universally representable. Job freezes staged bytes and loss. Style revision invalidates unfrozen preparation, whereas confirmed file resume still delivers original exact bytes under current authorization, without recompiling into different content. This creates no separate Word cascade protocol.

### 8.6. Chinese defaults, fonts and language delivery

The normal path for [REQUIREMENTS](../typst-native/REQUIREMENTS.md) §5.5 needs no per-note Chinese activation. Library initialization enables §8.1's ordinary default module, using native text/set/show-set for fonts, language and region; new notes need only content. Settings edit that same module without a Chinese switch, second configuration authority or new syntax. Explicit style opt-out follows §8.1 without silently re-enabling the module; Chinese source remains editable/saveable. Source controls also need Chinese fonts and IME, but their personal display font does not change author styles.

Initialization uses the selected library language/region, or the UI language at initialization if not separately chosen, recording it in the module. Later UI-language changes do not silently change documents. Simplified Chinese for China uses `text(lang: "zh", region: "CN")`; other regions follow the actual choice rather than inferred nationality. Han display does not require lang=zh; language additionally affects language-aware glyphs, quotes and AT speech. Native note/local overrides remain available; undeclared passages inherit document language, without claiming automatic per-passage detection. [Typst text](https://typst.app/docs/reference/text/text/)

| Content | Default font responsibility |
| --- | --- |
| Body, titles, lists, tables | Latin fonts plus explicit simplified/traditional CJK candidates. Initial Chinese styling uses regional Noto Serif CJK, with Noto Sans CJK available for sans-serif appearance; library settings can replace them. |
| raw/code | Monospace code fonts plus CJK monospace candidates, such as DejaVu Sans Mono and Noto Sans Mono CJK. Verify Han/ASCII/punctuation/tab alignment without forcing every character to identical width or changing code strings/indentation. |
| Equations | Retain a valid OpenType MATH font for mathematical structures, initially New Computer Modern Math. Chinese strings/text use Chinese-capable text fonts/fallback rather than replacing all math with an ordinary Chinese font. |

`raw.lang` selects highlighting and is separate from `text.lang`; Chinese comments/strings need no extra tag. Equation text uses native expressions such as `$ "总价" = "单价" times "数量" $` or text content. Any equation-text control emits the same native expressions, not Chinese-specific commands. Verify raw/math glyph selection on the selected compiler; successful body font selection does not prove inheritance for both. [raw](https://typst.app/docs/reference/text/raw/), [math](https://typst.app/docs/reference/math/)

**Font delivery.** Products/offline installers supply default fonts; distribution inventories identify actual files, versions, regional variants/weights, coverage and redistribution licenses. No user installation, public CDN or incidental host font is required. Core supplies a frozen font collection through the existing Environment. Browsers load actual authorized same-origin resources and separately map raw and MathML/math text; CSS family names alone do not establish delivery. Pending/failed font loading discloses presentation state while preserving editing/Draft; author font URLs grant no arbitrary network access.

Cover basic simplified/traditional Han characters, common punctuation and mixed Chinese/Latin text. Explicit fonts, fallback:false and custom modules retain native behavior. Missing fonts, detected missing glyphs and out-of-coverage characters report their actual location/character and a repair entry, without replacing characters, losing source or silently overriding author choices. Diagnostics consume actual font/glyph results: exit0 or extractable text does not establish absence of tofu. Targets may have different layout metrics, without losing text, language or declared style mappings.

PDF verifies actual fonts and text extraction. DOCX retains editable text/equations and expressible East Asian fonts, language and effective character styling. Embedding requires licensing and target support; otherwise disclose target-side dependencies or substitution/loss, without promising identical layout in every Office client. Standalone Typst projects follow §8.5 with redistributable fonts and exact entry/font-path instructions; note-only export does not automatically include library defaults. Font collection/version changes use existing Environment invalidation without changing note bytes, identities or annotation ranges.

These E2/E3/E5 product cases are **unexecuted**; existing bounded style experiments do not close them:

| ID | Scenario | Required observation |
| --- | --- | --- |
| CJK01 | Clean environment without system fonts/network; default library; simplified/traditional titles, body, lists and tables without Chinese declarations/imports | Delivered fonts render without missing-glyph boxes; selection/copy and save/reopen preserve source characters. |
| CJK02 | Inline code and untagged/python/rust blocks; Chinese comments/strings, ASCII and tabs | Highlighting and Chinese support remain independent; actual alignment is usable and copied bytes/indentation stay unchanged. |
| CJK03 | Inline/display equations with Chinese text, fractions, roots, scripts and extensible brackets | Readable Chinese with correct math structure and intact source, without reducing the whole equation to plain text or screenshots. |
| CJK04 | English passages in Chinese libraries, Han in other-language libraries; note/local overrides followed by a UI-language-only change | Glyphs display and language attributes/AT follow actual inheritance/overrides; UI language does not change existing documents. |
| CJK05 | Desktop, offline intranet WebUI/Server and CLI without system CJK fonts; delayed/failed browser font loading | Default delivery needs no network font fetch; disclose presentation state without losing input/Draft or installing falsely successful previews. |
| CJK06 | Missing custom fonts, out-of-coverage characters and fallback:false; install admitted fonts/repair configuration | Diagnose actual gaps, preserve source, recompile on recovery and reject stale results; no universal Unicode claim. |
| CJK07 | PDF, DOCX and font-bearing standalone projects opened/copied/edited in target environments | Verify actual fonts, punctuation, math text, language and editability; disclose dependency/embedding limitations and record Office testing separately. |
| CJK08 | Chinese IME/unsaved Draft/equation editing across five views, plus library-font changes | Shared Draft/Undo and source ranges do not drift; invalidate actual consumers without minting note revisions. |

## 9. Rendering and authorization safety

Compile workers have no ambient workspace, arbitrary path, network, credential, Action or author-write capability. Core supplies only actual authorized dependencies and fixed Environment. Unauthorized includes/packages/resources return no content, and diagnostics reveal no hidden paths/counts. Time, memory, recursion, output and process-tree budgets apply. Failure retains Draft; exit0 does not authorize output delivery.

Reading nodes use host-fixed elements/styles with text nodes. Author HTML/CSS/JS never directly enters a privileged DOM. SVG/experimental HTML require allowlist validation against scripts/handlers, foreignObject/active embeds, forms, external fonts/fetch and unapproved protocols. Resources use Core delivery addresses and links use the resolver. WebUI CSP/isolated unprivileged origin/sandbox and Desktop webview with no file/process bridge apply. Sanitization affecting legal content reports loss/local unavailable rather than silently discarding it.

Rendering/caches/search snippets/thumbnails bind actual consumed scope and current auth. Revocation stops new delivery, asynchronous installation and decorations and hides protected caches, retaining user-entered recovery text. Field-only permissions do not create body snippets, whole source beyond title projection or thumbnails. Source/annotation/resource authorize separately; source read does not grant download/publication. Cache keys cover dependency revisions, schema and target. Old output may be explicitly read as historical, never as current action qualification.

## 10. Accessibility and context

Keyboard, touch and AT have equivalent heading operations, source entries, table commands, conflict/error access, return and potential-link choices. Logical order comes from the bridge, not visual coordinates or temporary DOM order. Carets retain logical points and visual affinity; Left/Right follows host visual direction, word movement/deletion uses shared logical graphemes and copy follows logical order. Isolating RTL path/email/code changes no bytes; table column identity is never reversed.

Outline retains actual depth, filters/folding/current section/focus. Focus mode never removes source. Navigation/list/body and narrow-screen return preserve Draft, Undo, filters and recoverable position. Local named layouts and NAV01–05 list preferences retain only personal presentation and re-resolvable targets, not results, permissions or old maps. Restoration reauthorizes; invalid targets never resolve by similar name. Late thumbnails/previews never steal input focus. Potential links are hints: multiple equal/crossing ranges and overflow stay explicitly selectable. Longest matching cannot swallow alternatives; conversion uses the link owner.

Test actual per-platform CJK IME, Arabic/Hebrew mixtures, emoji/combining, keyboard/touch, screen reader, 200/400% zoom, 320px, high contrast and reduced motion. Live regions do not announce every preedit character; virtualization retains focus/composition. Missing surface capabilities report unavailable; CLI has no claimed graphical acceptance. Performance targets await measurement: feedback p95≤50ms, layout≤100ms; 1000 events, three cold/warm runs; Desktop 10MiB/100k lines, Mobile 1MiB/10k lines and 1MiB single line; steady memory 256/128MiB; tenfold over-limit retains Draft. Unrun targets are not passed.

## 11. Independent annotations

Independent JSON stores annotations, replies, resolution and target history without becoming body/presentation authority. Core establishes exact committed authorized document/element/range or PDF/image/audio/video targets; Draft/DOM/projection keys are not persisted. Only proven unchanged-range Core mappings carry targets across edits. Overlap/gap/ambiguity yields orphaned/candidate with discussion retained, never guessed similar anchoring. Generated/multi-origin display supports whole-document comments or explicitly selected true source, without fabricated ranges.

Annotation reads deliver their stored excerpts/body/history without reading hidden targets, screenshots or recomputing context. Manual reattach explicitly chooses a new target and requires suggestion reconfirmation. Apply uses one unified multi-entity Core transaction for current exact source and accepted state; one-side success is not completion. Reject needs annotation qualification and no additional target read. Read-only sources may be quoted into writable destinations; copied text belongs to destination author content and does not prove source currentness. Source mapping, media precision and multi-entity commit remain actual Core/annotation implementation gates, without old locator/transform wire dependencies.

## 12. Editable Word default and its boundaries

DOCX uses Weftext's converter, not an official Typst builtin. The default semantic conversion profile consumes one frozen Html compile's final semantic tree, same-target heading/citation products, exact origin associations and authorized resources. It never reruns Query, guesses the first table or mixes in a separate Paged output. A native heading hidden by custom show cannot become a ghost paragraph merely because introspection retains it. Explicit Paged semantic conversion is a separate named profile whose preview/output identifies the target and whose supported scope is independently verified. Local html.frame content retains its same-Html-compile origin boundary. Preserve actual editable paragraphs/runs/tables/bookmarks/resources; unrepresentable layouts report loss/unavailable. Map title/subtitle to Title/Subtitle outside section counters. This agrees with FEATURES §11; final semantic producers remain subject to E1.

Levels 1–9 map to actual Heading N, outlineLvl=N−1 and one multilevel numbering definition shared by standalone/run-in headings. The accepted default is one entire Heading N paragraph with heading runs followed by body runs. Body character formatting resolves the template body placeholder/Normal's effective properties through docDefaults, theme, basedOn, character style and direct formatting. Explicitly reset heading-inherited bold/italic etc. before applying author emphasis. Copying empty raw rPr is not resolved style copying; do not hardcode fonts/sizes. Heading paragraph indentation, spacing and keepNext still apply, as preview must show. [Microsoft paragraph styles](https://learn.microsoft.com/en-us/office/open-xml/word/how-to-create-and-add-a-paragraph-style-to-a-word-processing-document)

Default Word TOC/navigation sees the whole Heading/outline paragraph and may include the run-in body; resetting run fonts does not narrow its paragraph scope. Output preview must disclose this. Number references use exact heading bookmarks and native fields, not fixed numbers. [Microsoft multilevel heading numbering](https://support.microsoft.com/en-gb/word/number-your-headings)

Title-only TOC is an optional refinement: style separators or explicit title-bookmark/generated directories need named routes, real-host validation and loss reporting. They are not a new default completion condition. Preserve native headings/numbering rather than hide the issue in literal numbers or document images. Word standard outline has no equivalent for 10+ depth. Retain real source depth and disclose absence of equivalent deep native outline/numbering; refusing the loss makes DOCX unavailable. Custom styles/editable text may retain level information, but Heading9 is not equivalent and full dynamic numbering is not claimed. [Microsoft outlineLvl: levels 0–8 and 9 meaning no outline](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.outlinelevel?view=openxml-3.0.1)

Job generates and validates exact files/loss before reporting outputs. Intranet download requires no external publication. Confirmation freezes source/template/dependencies/staged bytes. Resume delivers identical bytes with current authorization at every chunk; expiration followed by regeneration is a new plan. Save/print/download/save-resource/external-publication each report actual outcomes. Reference diagrams do not establish Office success.

## 13. Evidence, decisions and implementation gates

Limited Typst 0.15.1 official compile/query runs proved a labelled run-in heading sharing counter/outline/ref with same-level standalone headings, with automatic insert/delete/move/promotion/demotion updates, no mandatory dimensions in a size-free sample, and a labelled subtitle block outside heading queries. Blank boundaries interrupt run-in and default unnumbered refs fail. A separate controlled DOCX passed ten OOXML checks for Heading2, real nine-level numbering, body formatting reset/emphasis and TOC/REF fields. The earlier 32 limited Rust checks and a few source splices do not cover a full editor. This proposal reran none of these experiments and claims no product pass.

This round separately ran seven official CLI samples and official AST enumeration: default/custom heading show/removed heading/stripped strong/repeated include/generated headings/Paged-Html branches. Custom show's query.body contains only the original title while actual PDF contains PREFIX/body/SUFFIX. Removed show leaves GoneHeading in query but no PDF heading. Repeated include produces two headings/counters[1]/[2] without included root text slots. Equal repeated queries establish bounded repeatability, not internal convergence capture; stripped-strong fonts/roles were not checked. CLI/AST do not pass the flow bridge. These counterexamples led to the final-consumer-tree route in §8; REVIEW records current bounded evidence and unrun scope.

This round adds the [Html final-tree/style probe](probes/html-flow/README.md): adapter-off Html bytes match the official release for 16 cases, adapter-on serial/parallel final trees match, and three Paged PDF outputs match byte-for-byte. It excludes discarded measure while retaining repeated includes and distinct show occurrences. Actual values cover library→note→local sizes, font candidates, emphasis/italic and simple cell borders/alignment. It implements no browser renderer, inverse editing, complex table topology or complete style support. A dynamic-marker counterexample adds one diagnostic source read, so World ordering matches only 15/16 cases. Native Paged content inside local html.frame is visible, without proving flow copying/accessibility. The product adapter switch must belong to a fixed worker/environment/cache profile, never change as a process environment flag within one cache. Probe attributes establish no trusted origin or writer.

The prescribed DOCX renderer found no soffice on PATH; read-only probes did find installed Office/LibreOffice. The correct state is unavailable prescribed rendering and unrun real-host testing. ReportLab reference rendering is not Office. Field updates, TOC scope, navigation, font inheritance, style separators, save/reopen and depth behavior in actual Word/WPS/LibreOffice are all UNRUN.

| Gate / requirements | Required actual validation | What remains blocked |
| --- | --- | --- |
| E1 / RQ01–03,06,§5.3 | A 16-case official Html final-tree/bounded-style prototype exists; independently review final consumption, native semantics, target boundaries, source-preserving fallback and unforgeable origins | Bridge design remains open until independent acceptance; full renderer, inverse editing and styles still require implementation acceptance |
| E2 / RQ02,24,§5.4 | Shared source/Draft/Undo and syntax visibility across five views; actual input surface: empty/type/delete-empty, arbitrary caret, cross-format/block,math/list,equal table cells,IME/Undo/late responses,auth/ABA | Ordinary-editor completion |
| E3 / RQ03,23 | Real Desktop/WebUI/target Mobile AT/RTL/zoom/320px,sanitizer/isolation/revocation,budget/performance | Reading/accessibility acceptance for that surface |
| E4 / RQ04–07 | Depth1–9/10+,titleless,subtitle roles,per-heading run-in,long text/orphans/pagination,counter/custom scope | Full heading matrix; limited native evidence remains valid |
| E5 / RQ07,20 | Actual Word/WPS/LO open/update→insert/delete/move/change level→TOC/navigation/refs→save/reopen; templates,CJK/RTL,effective body format | DOCX interoperability claims; style separator tested separately |
| E6 / RQ18,23–24 | Exact annotation mapping,media intervals,no extra hidden-target reads,multi-entity suggestion transactions,crash/concurrency | Annotation/suggestion completion |

Decisions ready for a coordinated candidate: one Typst source; shared session and DraftUndo; fresh basis and unified Core commit; local source preservation without invented generated addresses; separate title/heading and a light subtitle label; native per-heading show; no user spacing parameters; whole-paragraph Word Heading N with resolved body character formatting and explicit TOC scope. E1 now has a runnable bounded bridge; its feasibility boundaries still need independent confirmation. E5 is the output gate that OOXML diagrams cannot close. Specific performance thresholds, optional style-separator routes and hook maintenance costs can be revised from prototypes without dropping requirements.

## 14. Coverage and replacement scope

This proposal directly owns RQ01–07,18,25 and NAV01–02 editing/reading interactions. New owners supply RQ08–11 fields, RQ12–14 links, RQ15–17 Query/tasks, RQ19 templates and RQ20–24 Core/conversion/authorization. This document states their session/source/focus/basis/loss/permission consumption. Navigation owners coordinate NAV03–05 Query/local preferences without an editor-specific fact/query model.

Replace old EDITOR §§1–12 with the sessions/maps/bridge/input/annotation interactions/gates here, without retaining its wire unions/history decoders/version-pin obligations. Keep old text as fixed input history. Preserve FEATURES §§7–8 user conversion/import/download outcomes while new conversion/Core owners select mechanisms; replace title/Word/editor consumption here. Old SOURCE single-title writer restrictions/carriers/locators/mandatory format qualifications do not become new mechanisms automatically. Without actual compatibility objects, invent no decoder. If actual user data is later discovered, retain originals and define import/recovery explicitly. Coverage and non-author review remain required; old CLOSED findings do not pass this proposal.
