---
source_language: zh-CN
translation_of: LINKS.zh-CN.md
translation_status: synced
---

[简体中文](LINKS.zh-CN.md)

# Link addresses and form conversion

Status: unimplemented design candidate. This file solely owns the four address forms, Workspace-root codec and conversion workflow. D2 retains native syntax, D3 identity/lifecycle, D6 inventory/authorization/commits, D8 Draft and D9 exports. No new link parser, identity directory, authority or commit system is introduced; original global findings remain open.

## 1. Four forms and one default

| Form | Address basis | Guarantee and scope |
| --- | --- | --- |
| Stable ID | Original complete D3 NodeRef with existing n1.W.N encoding | Move/rename within one Workspace preserves identity; deletion never redirects to a new same-name note. |
| Source-relative path | Path base from actual native D2 evaluation | Usually the root note's containing directory; moving source or target may change resolution. |
| Workspace-root full path | Explicit WorkspaceId plus full path from its root | Moving the source or relocating the same whole Workspace preserves meaning; moving/renaming target still requires updates. |
| Local absolute path | Explicit device file URI | Only an actually accessible mapping on this device; no cross-device or Server-client portability promise. |

Library here means Workspace, not a same-name folder or new identity domain. Cross-library addresses identify the target WorkspaceId and never substitute a current-Workspace same-path target. New internal note links default to stable ID as a device setting; changes affect subsequent explicit creation without mass source rewrites. Titles/aliases/filenames are picker search inputs; addresses are constructed only after selecting the actual target. Display text stays independent.

Shortest valid path simplifies a relative path rather than adding a fifth addressing protocol: generate the shortest canonical relative path to the proven target under its actual base and verify unchanged resolution. Never search the whole Workspace by basename, title, alias or shortest unique suffix. Sharing short URLs requires service/share authorization and is outside this operation. Local absolute, Workspace-root and Web URL roots are distinct.

## 2. Native syntax and closed target encoding

Use native xref between AsciiDoc documents and native link for ordinary files/URIs. D2 parses the complete native grammar before target-codec validation. [Native inter-document references](https://docs.asciidoctor.org/asciidoc/latest/macros/inter-document-xref/) and [native file links](https://docs.asciidoctor.org/asciidoc/latest/macros/link-macro/) remain applicable. No double-bracket link syntax is introduced, and ordinary leading-slash paths are not reinterpreted as Workspace roots.

```adoc
xref:../资料/白云山.adoc[白云山]
link:weftext-workspace://<WorkspaceId>/<encoded-path>[白云山]
link:file:///D:/notes/report.pdf[报告]
```

The second line is illustrative; angle-bracket placeholders are not legal stored values. The sole Workspace-root URI codec has exact scheme weftext-workspace and authority equal to a D3 lowercase UUIDv4 WorkspaceId. Userinfo, port, query, missing authority and empty path are forbidden. One leading slash precedes nonempty components representing real Core inventory paths. Optional fragment identifies an anchor in that same target. This is Weftext addressing semantics, not a claim that ordinary AsciiDoc tools can resolve Workspace identity alone. Enable this new codec only under verified managed2 format qualification. Actual managed1/BaselineOnly retains ordinary native links/raw source without acquiring Workspace-root semantics on old-record reads. Activation requires explicit format conversion and full Source preview; saved/planned records recover their original profile. Cross-Workspace stable IDs still satisfy original D3 reference/copy constraints; URIs expand neither cross-library writes nor identityMap authority.

The generator preserves URI unreserved bytes of each exact UTF-8 path component and percent-encodes others with uppercase hexadecimal, retaining separator slashes. Decode only once at this URI layer; reject malformed escapes, non-UTF-8, NUL/control, empty components, dot/dot-dot, backslash and encoded separators. Do not decode resulting percent text again, normalize Unicode, fold case or interpret plus as space. Noncanonical stored spellings fail the codec while retaining repairable Source. Native parsing/substitution produces nativeTarget before this codec; never pre-process raw source with another percent/macro parser. Encode fragment separately using the same UTF-8/unreserved rules and decode once to the exact author-anchor string. Empty fragment differs from no fragment; generators emit no empty fragment. Path dot-component restrictions do not reinterpret legal anchors as file paths.

Paths cannot escape Workspace boundaries or bypass Core inventory through symlinks/aliases. Unsupported storage names, existing alias conflicts, unmaterialized placeholders or unprovable scope prevent usable conversion; string normalization does not prove the actual file. File URIs use supported platform URI/path libraries and validated round trips, not hand-built drive/UNC strings. Relative form is unavailable when source and target do not share a proven relative-path domain.

## 3. Parsing does not create identity facts

D2ProductInline3 already has link.nativeTarget, xref.nativeTarget/refid/path, label/nativeAttributes/slotOrigins. All four spellings retain those actual slots without widening D2IdentityAdapter1. Only original valid stable-ID targets produce its node_link arm. Workspace-root, relative and local paths keep adapter=null even when this navigation has authorized a Node target. Original Core inventory, protected FileObjectBinding, NodeRef, lifecycle, SourceObservation and authorization prove actual navigation/conversion targets. Filename, file-carried ID, hash or cached index never infers identity.

D7 core/document-link asserted edges, D3 durable inbound and identityMap rewriting still consume original recognized identity references only. Outgoing UI lists may show ordinary path links with their form indicated, but currently resolved paths are not stable identity edges or complete permanent backlinks. ID-to-path preview explicitly describes the change in stable association capability and retains original effects/invalidation; it is not claimed as a semantically identical abbreviation. Unknown/unauthorized paths remain literal ordinary links without creating notes.

Current D2IdentityAdapter1.node_link has only NodeRef and no author-anchor member. A fragment-bearing link can convert only when the existing target form fully represents it and the same target/anchor is provable. ID is explicitly unavailable without that capability; never delete the fragment or invent an n1 suffix. This file does not claim Node-plus-anchor support. Named adapter, D3 reference/copy and D7 edge extensions are a separate future increment. Native xref anchors remain fully available; generated heading anchors retain native rename fragility rather than becoming durable Locators.

## 4. Context-menu conversion and copying

Right-click, touch long-press and keyboard commands equally expose Convert link form, Copy as and Simplify relative path. Actions apply to the current occurrence and show its current form and unavailable reasons. Unknown/unauthorized targets retain original nondisclosure outcomes; disabled text cannot reveal hidden paths, existence or titles. Preserve authored display text, label formatting and applicable native attributes by default. If a macro change cannot preserve an attribute losslessly, show the authorized difference and reject quick conversion while retaining full Source editing.

The conversion algorithm binds all dependencies to the same actual source version/Draft serial and authorization cut:

1. After original disclosure gates, read the exact occurrence and D2 projection; require D8 full Source read/write qualification. Included, generated, multi-origin, non-uniquely invertible or partial-grapheme occurrences gain no quick-write qualification. Attribute-produced targets without a unique safe literal edit position reject quick conversion; changing a shared attribute is not disguised as changing this occurrence alone.
2. Resolve the original target under its real native evaluation base. A convertible managed target requires complete NodeRef/current binding; ordinary external targets require original local/resource resolution qualifications without inventing managed IDs. A fragment additionally requires the exact unique author anchor in that target source. Missing, duplicate or unverifiable anchors retain the link with lossless conversion unavailable. Equal strings or same-name anchors are insufficient.
3. Generate the chosen form, then inversely resolve through the same native parser and actual resolver to prove identical target/fragment. Future move behavior may differ and is explained in preview. Path generation/disclosure separately requires original location/structure disclosure: readable Node does not imply readable parent directory or local path. Only disclosable forms are available. Workspace-root cross-library reads retain original cross-Workspace gates; carrying W grants no authorization.
4. Construct complete proposed Source in the same Draft, preserving untouched bytes, display text, applicable attributes and anchor. Full reparse/original SourceTransform verifies effects before original D8→D6 prepare/save. Do not use narrow plain-text branches that prohibit new macros. One conversion is one Undo unit; cancel writes nothing.
5. At the final read barrier, revalidate source/target/path binding/anchor/authorization/format/registry and actual dependencies. Rename, same-path replacement, ABA, revocation, sync gaps or Draft changes invalidate preparation. Recalculation needs new preparation rather than silently retargeting under old evidence.

Copy as distinguishes address-only from complete AsciiDoc link; complete links use the same native encoding and label preservation. It requires no source write permission, Draft edit or receipt and neither publishes sharing nor auto-saves. Source/target/address disclosure remains required and is revalidated immediately before clipboard write. Relative copies explicitly label their base; pasting into another document does not inherit target semantics automatically. Weftext paste with real source context may offer explicit relocation preview; plain-text clipboard data never infers hidden identity. Copying already-authorized literal source unchanged is a separate ordinary copy action, available even when its target is disconnected, missing or unreadable. It asserts no successful resolution and neither queries nor adds hidden target information; conversion-copy target gates must not block it.

## 5. Workspace and device boundaries

Workspace paths come from Core's current controlled inventory, not a title-built logical tree or duplicate editable metadata. Relocating the same whole Workspace changes actual root binding while retaining URI WorkspaceId/path; fork/copy into a new Workspace is not the same library. Relative paths retain D2's actual native evaluation base; includes or configured prefixes are not reinterpreted by UI as an assumed root-document directory.

Local absolute form exists only with a verified file mapping on the caller's device and path-disclosure authority. WebUI/remote clients never receive Server disk paths as usable local addresses, nor pretend client file URIs read Server files; remote navigation/download retains original Server entry points. Workspace-root URIs resolve only against authorized connected local or Server Workspaces. Disconnected libraries are unavailable without automatic network connection, same-name guesses or sharing-link creation.

Copy/template/import cannot blindly copy relative target bytes while claiming references are preserved. Paths to retain are first resolved in their original authorized source context, then encoded and verified under the frozen new source location. Mapped targets come from the original identity map and their actual new paths. Cross-library URI authority changes only with an explicit Workspace mapping and an actually mapped target, never global UUID replacement. Unproved location/target, incompatible relative domains or unpreservable fragments follow original explicit loss/rejection paths. Changes share one complete preview, SourceTransform, C/Q and final seal, adding no background rewrite service or durable path identity map.

## 6. Export boundary and future routing

Existing native xref conversion remains. Exact-source exports retain exact bytes and generationPolicy none, without rewriting files from link preferences. Existing generation providers produce addresses only when declared capabilities, actually frozen inputs and controlled outputs prove target/fragment. Never infer output filenames from source names or rewrite original notes.

General stable-ID/Workspace-root conversion to offline document-set relative addresses needs frozen occurrence→authorized target/anchor→controlled output name/fragment routing evidence for the output set. Current D9DocumentRenderBinding1 and ExportPlan4's typed evidence union do not carry that complete table. This candidate adds no free evidence pin or widened closed Plan and does not claim general offline routing complete. Conversion unprovable under existing profiles is unavailable or enters original explicitly confirmed loss reporting, never a plausible link to the wrong object. A later D9 versioned render-binding/typed pin, consumer and confirmation design owns that addition. It does not block source-level four-form conversion or existing exact-source export.

## 7. Design acceptance

All cases are design requirements; product UNRUN.

| ID | Positive and counterexample coverage |
| --- | --- |
| LINK-01 | Convert the same target among available forms with label/applicable attributes retained; ordinary slash roots never become Workspace roots. |
| LINK-02 | Source moves affect relative form, whole-library relocation retains root form; target rename/replacement stales path preparation, ID never redirects by name. |
| LINK-03 | Same-name notes require explicit choice; shortest path never searches Workspace suffixes; no cross-Workspace authority rewrite without mappings. |
| LINK-04 | Percent/space/Unicode/case/plus preserve exact meaning; reject double decoding, encoded separators, dot traversal, userinfo/port/query and alias/symlink escape. |
| LINK-05 | Conversion preserves exact fragment; absent ID anchor slots, duplicate/missing anchors or no inverse mapping disable conversion without dropping anchors. |
| LINK-06 | Readable Node without location disclosure yields no path; Server disk paths stay private; disconnected libraries get no guessed substitutes. |
| LINK-07 | One conversion/Undo; cancellation/revocation/ABA/Draft changes do not commit; copy changes clipboard only, without write permission or sharing publication. Literal broken-link copying remains available; conversion-copy still requires target qualification. |
| LINK-08 | Shared-attribute/include/generated sources retain bytes with unavailable reasons; full Source conversion never injects macros through narrow text writers. |
| LINK-09 | ID-to-path explains stable-edge changes; D7 asserted graph/Ref inbound never promote transient path resolution to identity. |
| LINK-10 | Copy/template relocation, fresh maps and C/Q share one decision; unproved preservation follows original loss/rejection without silent retargeting. |
| LINK-11 | Exact-source keeps bytes; unsupported offline routing never claims success or adds free members/pins to old Plans. |
