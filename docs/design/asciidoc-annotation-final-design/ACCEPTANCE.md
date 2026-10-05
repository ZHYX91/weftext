---
source_language: zh-CN
translation_of: ACCEPTANCE.zh-CN.md
translation_status: synced
---

[简体中文](ACCEPTANCE.zh-CN.md)

# Design acceptance matrix

Status: **design obligations only; not executed**.

- core design oracles: 438
- actual-owner coordination fixtures: 212
- total: 650

Each row is a normative obligation of this candidate. Words such as PASS/FAIL describe the future acceptance condition and do not claim execution in this PR.

## Core — 438

| ID | Spec area | Requirement |
|---|---|---|
| AD2-01 | SPEC §§1–4 | All core-language oracles are pinned to asciidoctor-ruby/2.0.26@0b99b39…; rolling documentation or later releases must not change this corpus result. |
| AD2-02 | SPEC §§1–4 | Text, escaping, special characters, quotes, attribute substitution, replacements, macros, and post-replacements must match the actual 2.0.26 substitution order. |
| AD2-03 | SPEC §§1–4 | A hard-set API attribute may affect generic effective attributes, but it cannot become authored authority for wf-kind/wf-facets controls. |
| AD2-04 | SPEC §§1–4 | If source does not author wf-kind and the host only injects an attribute with that name, the generic environment may expose the value but the Node remains ordinary. |
| AD2-05 | SPEC §§1–4 | Soft-set and hard-set attributes must remain distinguishable; source may override a soft-set value but not a hard-set value. |
| AD2-06 | SPEC §§1–4 | Replay the complete ordered 2.0.26 authored attribute event stream, including set/unset/substitution; RootAuthoredControlProjection must consume provenance from the same parser and may not implement a simplified assignment parser. |
| AD2-07 | SPEC §§1–4 | A body attribute assignment affects only subsequent generic AsciiDoc processing and cannot retroactively take over root Node classification. |
| AD2-08 | SPEC §§1–4 | wf-kind/wf-facets inside included source may participate in the include environment as ordinary AsciiDoc attributes but cannot change the including root Node control state. |
| AD2-09 | SPEC §§1–4 | A root control may reference another root-authored attribute; if its final value depends on host/include/builtin provenance, generic AsciiDoc may remain valid but Weftext Node control must be unavailable/invalid rather than taking authority. |
| AD2-10 | SPEC §§1–4 | link:n1.W.N[label] is parsed by the native link grammar first and only then projected by the Weftext adapter into a NodeLink; no second link parser is allowed. |
| AD2-11 | SPEC §§1–4 | A valid n1 target does not require an invented ++ passthrough canonical syntax. |
| AD2-12 | SPEC §§1–4 | The link/citation label corpus covers Doe, 2025, a=b, legal ] escaping, single/double quotes, commas, role attributes, and multiline attribute-list cases allowed by 2.0.26; all are handled by the baseline attribute-list parser. |
| AD2-13 | SPEC §§1–4 | Native bibliography anchors/xrefs and Weftext citation roles may coexist in one document without automatically merging provenance or identity. |
| AD2-14 | SPEC §§1–4 | Deleting a derived Weftext bibliography placement deletes only the placement, not the cited Node or reference fact. |
| AD2-15 | SPEC §§1–4 | r1.UUID resolves to an owner-local Resource under the containing Node; the same UUID leaf under another owner is not the same ResourceRef. |
| AD2-16 | SPEC §§1–4 | Image alt/width/height and related fields use native image semantics; copy rewrite changes only Resource identity and preserves occurrence presentation. |
| AD2-17 | SPEC §§1–4 | Audio uses its actual native title/start/end/options attributes and does not inherit invented fields from the image model. |
| AD2-18 | SPEC §§1–4 | Video poster/width/height/start/end/options follow fixed 2.0.26 semantics. |
| AD2-19 | SPEC §§1–4 | A D4 carrier inner payload uses the exact authored literal range; renderer escaping cannot become typed input. |
| AD2-20 | SPEC §§1–4 | If a payload contains the default delimiter line, the emitter selects a legal non-colliding delimiter; parse-to-source round-trip preserves payload bytes exactly. |
| AD2-21 | SPEC §§1–4 | A carrier/query/view in managed included source remains owned by the real included source owner. |
| AD2-22 | SPEC §§1–4 | Authorized local includes, tags, lines, indent, leveloffset, and related behavior must equal the Ruby 2.0.26 semantic witness. |
| AD2-23 | SPEC §§1–4 | If an include is denied by file permission, the source syntax remains valid; EvaluationStatus is effect_denied/incomplete and directive bytes are preserved. |
| AD2-24 | SPEC §§1–4 | A URL include may consume only the exact authorized snapshot; later bytes at the same URL do not inherit old current proof. |
| AD2-25 | SPEC §§1–4 | An unmanaged snapshot may be used for one-shot read/preview/export, but cannot serve as a long-lived current Query/Action dependency. |
| AD2-26 | SPEC §§1–4 | When attribute/include/substitution output has multiple source origins, preserve the full origin graph; Write/Annotation may not automatically choose the nearest origin. |
| AD2-27 | SPEC §§1–4 | Unordered, ordered, description, callout, checklist, hybrid, and mixed lists, including list continuation and attached blocks, remain complete. |
| AD2-28 | SPEC §§1–4 | Historical parser edge cases such as checklist/list continuation plus attached tables must not panic or silently flatten; semantic inequality fails the Core Gate. |
| AD2-29 | SPEC §§1–4 | PSV/CSV/DSV/TSV tables, cols, header/footer, spans, duplication, alignment, style, and a AsciiDoc cells all participate in the semantic corpus. |
| AD2-30 | SPEC §§1–4 | The complex Source↔Write corpus includes deep authored levels 6-9, run-in headings, tables/lists/STEM; after editing one ordinary paragraph, every untouched byte including markers, roles, and trivia remains byte-identical, and HTML/PDF/DOCX each still satisfy deep/run-in backend rules. |
| AD2-31 | SPEC §§1–4 | If the rich editor lacks a control for a legal construct, the construct must still parse, read, source-edit, and save; missing UI is not unsupported syntax. |
| AD2-32 | SPEC §§1–4 | A missing STEM provider affects BackendStatus only, not core-language validity. |
| AD2-33 | SPEC §§1–4 | SVG emitted by the fixed Mermaid provider must pass SvgOutputProfile validation. |
| AD2-34 | SPEC §§1–4 | SVG containing script, event handlers, external-network URIs, DTD/entities, or unauthorized CSS/fonts is rejected even if the provider succeeded. |
| AD2-35 | SPEC §§1–4 | Changing provider/config/font/runtime profile invalidates the corresponding render cache. |
| AD2-36 | SPEC §§1–4 | Exact .adoc export can still succeed when PDF rendering is unavailable. |
| AD2-37 | SPEC §§1–4 | The Oracle semantic witness and Weftext provenance witness are each deterministic; thread or map iteration order cannot change canonical output. |
| AD2-38 | SPEC §§1–4 | Diagnostics compare severity, semantic code, and location; the exact message is retained to detect over-normalization. |
| AD2-39 | SPEC §§1–4 | A real core-language mismatch cannot be registered as a backend, environment, or Weftext-domain exception. |
| AD2-40 | SPEC §§1–4 | The cost of completing Asciidork support does not change the full 2.0.26 acceptance scope. |
| AD2-41 | SPEC §§1–4 | In embedded mode, API attributes ordered as notitle="" then showtitle="" and the reverse order remain distinct environments: fixed Ruby 2.0.26 derives the opposite alias from whichever key is last in ordered attr_overrides, so canonical environment bytes and observed title/effective-attribute state must preserve that API order; name-sorting them into one environment fails. |
| AN2-01 | SPEC §§9–16 | Every root and reply has its own AnnotationRef; there is no separate ThreadMessage identity. |
| AN2-02 | SPEC §§9–16 | A thread is mechanically the D3 same-owner reply_closure; cycles must be rejected. |
| AN2-03 | SPEC §§9–16 | Two concurrent fresh replies may coexist; timestamps are not used for LWW. |
| AN2-04 | SPEC §§9–16 | Concurrent edits to the same reply are resolved through Annotation revision/CAS/ConflictRecord. |
| AN2-05 | SPEC §§9–16 | reviewState and target-resolution state are orthogonal. |
| AN2-06 | SPEC §§9–16 | Reopen changes only the root reviewState and does not automatically change the target or reanchor it. |
| AN2-07 | SPEC §§9–16 | highlight/underline/squiggle/strike × yellow/red/green/blue/purple/pink/gray round-trip portably. |
| AN2-08 | SPEC §§9–16 | The frozen doctype=inline Annotation body profile fully supports its allowed CJK/RTL/emoji/strong/emphasis/link/STEM semantics. |
| AN2-09 | SPEC §§9–16 | A trusted insertion before the target mechanically moves an exact range through the transform without manual reanchor. |
| AN2-10 | SPEC §§9–16 | An insertion at range start preserves the originally selected text according to the fixed affinity. |
| AN2-11 | SPEC §§9–16 | An insertion at range end does not automatically expand the old selection. |
| AN2-12 | SPEC §§9–16 | A true overlap stops trusted mapping; a context candidate cannot become source-write authority. |
| AN2-13 | SPEC §§9–16 | An observed_only save, or a save with only whole before/after images and no Core edit provenance, cannot produce SourceTransform evidence. |
| AN2-14 | SPEC §§9–16 | After an external edit, even a unique quote/context match can produce only a candidate. |
| AN2-15 | SPEC §§9–16 | Repeated text with multiple candidates is ambiguous; never choose first/nearest. |
| AN2-16 | SPEC §§9–16 | A signed transform chain missing any transition cannot yield mapped resolution. |
| AN2-17 | SPEC §§9–16 | If source bytes arrive before the artifact, the target cannot be claimed mapped during that interval. |
| AN2-18 | SPEC §§9–16 | A transform with an invalid signature/profile/trust cut/cross-field relation is rejected. |
| AN2-19 | SPEC §§9–16 | Mapped resolution is only input to fresh current qualification and cannot revive an old PreparedIntent or ActionEvidence. |
| AN2-20 | SPEC §§9–16 | Replace acceptance requires a fresh current target, current expected bytes, and a new D7 prepare. |
| AN2-21 | SPEC §§9–16 | A suggestion cannot be accepted when geometry still maps but expected bytes have changed. |
| AN2-22 | SPEC §§9–16 | Delete likewise requires current exact expected bytes. |
| AN2-23 | SPEC §§9–16 | Insert uses a zero-width target, pointAffinity, and fresh current qualification. |
| AN2-24 | SPEC §§9–16 | Explicit reanchor moves a pending suggestion to needs_reconfirmation. |
| AN2-25 | SPEC §§9–16 | Reconfirmation rebuilds targetBasis and expected bytes/point against the current exact target. |
| AN2-26 | SPEC §§9–16 | Accept writes the Document after-image and Annotation accepted state in one D6 seal. |
| AN2-27 | SPEC §§9–16 | An accept/reject race on one current Annotation revision has exactly one winner. |
| AN2-28 | SPEC §§9–16 | Loss of target disclosure stops processing before transform/current geometry is read. |
| AN2-29 | SPEC §§9–16 | Historical quote/prefix/suffix can be read only when both annotation and historical-source disclosure are authorized. |
| AN2-30 | SPEC §§9–16 | Raw backup bytes already delivered cannot be magically revoked later; online APIs still obey current permissions. |
| AN2-31 | SPEC §§9–16 | A Review Bundle without history permission contains no historical excerpts. |
| AN2-32 | SPEC §§9–16 | The Annotation owner for a root-authored source span is the root Node. |
| AN2-33 | SPEC §§9–16 | The Annotation owner for a managed included-Node source span is the included Node; root permission is not a substitute. |
| AN2-34 | SPEC §§9–16 | Unmanaged/network included bytes cannot directly establish a durable exact target; annotate the directive or adopt the source first. |
| AN2-35 | SPEC §§9–16 | Synthetic multi-origin text cannot automatically choose one owner. |
| AN2-36 | SPEC §§9–16 | A PDF/image normalized rectangle binds an exact ResourceVersion; zoom or shell changes do not alter it. |
| AN2-37 | SPEC §§9–16 | Audio/video rational timebase/ticks and [start,end) round-trip exactly. |
| AN2-38 | SPEC §§9–16 | If a media decoder/profile is unavailable, existing raw records are preserved and creation of a new exact region is unavailable. |
| AN2-39 | SPEC §§9–16 | Node copy creates fresh AnnotationRefs, rewrites the full reply graph, preserves creator/authoredAt/lastEditor/editedAt attribution, and reissues targets through the real source/resource identityMap; it never copies an old locator or guesses by text. |
| AN2-40 | SPEC §§9–16 | Corruption of one Annotation value cannot be interpreted as “this owner has no annotations.” |
| AN2-41 | SPEC §§9–16 | An unknown future Value version preserves raw bytes and identity and reports unsupported. |
| AN2-42 | SPEC §§9–16 | History/transform/trust evidence required by an unresolved suggestion is protected by last-reference retention. |
| AN2-43 | SPEC §§9–16 | After explicit history pruning, historical lookup is explicitly unavailable; the current value cannot impersonate history. |
| AN2-44 | SPEC §§9–16 | An ordinary reply to a resolved thread does not automatically reopen it; Reply-and-reopen is an explicit compound operation. |
| AN2-45 | SPEC §§9–16 | Trash/restore/purge follows reply closure; only purge ends the ability to restore that identity. |
| WFX-L01 | SPEC §§1.4,7–8 | Changing a Node title does not change its stable n1 target. |
| WFX-L02 | SPEC §§1.4,7–8 | Path or move changes do not change NodeRef links. |
| WFX-L03 | SPEC §§1.4,7–8 | An explicit authored label is preserved verbatim. |
| WFX-L04 | SPEC §§1.4,7–8 | A dynamic label shows target title only when currently authorized to read it; otherwise it is neutral and leaks no old title/path. |
| WFX-L05 | SPEC §§1.4,7–8 | Same-document anchors continue to use native xref semantics. |
| WFX-L06 | SPEC §§1.4,7–8 | Ordinary inter-document xref keeps standard semantics. |
| WFX-L07 | SPEC §§1.4,7–8 | After deleting Derived Index, backlinks can be rebuilt from actual source/references. |
| WFX-L08 | SPEC §§1.4,7–8 | A partial index cannot prove zero backlinks. |
| WFX-L09 | SPEC §§1.4,7–8 | A complete backlink result requires completeness proof for ref_inbound/source/lifecycle/authorization. |
| WFX-L10 | SPEC §§1.4,7–8 | A hidden source Node cannot leak title/path/excerpt merely because it references a visible target. |
| WFX-L11 | SPEC §§1.4,7–8 | With insufficient disclosure, hidden/absent/broken remain non-disclosing. |
| WFX-L12 | SPEC §§1.4,7–8 | NodeLinks inside a subtree-copy closure are rewritten through the identityMap. |
| WFX-L13 | SPEC §§1.4,7–8 | A legal same-Workspace target outside the closure is preserved according to D3. |
| WFX-L14 | SPEC §§1.4,7–8 | Workspace fork rewrites every stable reference inside the complete closure. |
| WFX-L15 | SPEC §§1.4,7–8 | Ordinary import without an identity manifest cannot reconnect identities by title/path. |
| WFX-L16 | SPEC §§1.4,7–8 | A Weftext citation enters the same inbound graph while retaining citation semanticRole. |
| WFX-L17 | SPEC §§1.4,7–8 | Strong/emphasis/escaping and related label syntax are parsed by the baseline inline parser. |
| WFX-L18 | SPEC §§1.4,7–8 | Backlinks are always derived and never written as a second author fact on the target Node. |
| WFX-H01 | SPEC §1.3 | In BaselineOnly, 7+ equals characters follow Ruby 2.0.26 baseline behavior exactly. |
| WFX-H02 | SPEC §1.3 | In WeftextManaged, 7 equals signs represent authored level 6. |
| WFX-H03 | SPEC §1.3 | 8, 9, and 10 equals signs represent authored levels 7, 8, and 9 respectively. |
| WFX-H04 | SPEC §1.3 | Authored levels 1-5 retain baseline meaning. |
| WFX-H05 | SPEC §1.3 | Document-title/book-part level-0 semantics are not changed by the deep-heading extension. |
| WFX-H06 | SPEC §1.3 | A 5→7 level skip emits the native warning and continues; it is not a hard reject. |
| WFX-H07 | SPEC §1.3 | Returning from a deep section to an ancestor level closes hierarchy normally. |
| WFX-H08 | SPEC §1.3 | leveloffset participates in the actual 2.0.26 state machine and can produce effective levels 6-9 or deeper. |
| WFX-H09 | SPEC §1.3 | An offset-derived effective level above 9 is not rejected by a Weftext semantic cap. |
| WFX-H10 | SPEC §1.3 | A deep explicit anchor participates in the normal catalog/xref model. |
| WFX-H11 | SPEC §1.3 | Auto IDs use the same baseline generator. |
| WFX-H12 | SPEC §1.3 | Outline preserves the exact effective level. |
| WFX-H13 | SPEC §1.3 | Folding/navigation preserves the complete deep hierarchy. |
| WFX-H14 | SPEC §1.3 | A D7 heading query returns the exact level and locator. |
| WFX-H15 | SPEC §1.3 | A rich edit that changes heading level edits only the marker and then performs a complete reparse. |
| WFX-H16 | SPEC §1.3 | Effective levels 1-5 map to HTML h2-h6. |
| WFX-H17 | SPEC §1.3 | Effective levels 6-8 use role=heading with aria-level 7-9. |
| WFX-H18 | SPEC §1.3 | Effective level 9 uses aria-level 10 and preserves the complete accessible outline. |
| WFX-H19 | SPEC §1.3 | Deep levels must not all be flattened to one semantic level. |
| WFX-H20 | SPEC §1.3 | The DOCX provider preserves at least Heading1-Heading9 structure for effective levels 1-9. |
| WFX-H21 | SPEC §1.3 | If PDF/another backend cannot express a level, degradation is explicit and does not mutate source or AST. |
| WFX-H22 | SPEC §1.3 | Exact .adoc export does not rewrite heading markers or leveloffset. |
| WFX-R01 | SPEC §1.3 | With no explicit role and workspace default=separate, presentation remains Separate. |
| WFX-R02 | SPEC §1.3 | .run-in plus an eligible first paragraph produces RunIn presentation. |
| WFX-R03 | SPEC §1.3 | .separate forces Separate even when workspace default is run_in. |
| WFX-R04 | SPEC §1.3 | .run-in.separate remains legal AsciiDoc source and yields a typed role_conflict plus Separate fallback. |
| WFX-R05 | SPEC §1.3 | Explicit run-in permits blank source trivia between heading and body. |
| WFX-R06 | SPEC §1.3 | Explicit run-in permits comment trivia between heading and body. |
| WFX-R07 | SPEC §1.3 | Legal ID/role attributes on the first paragraph do not break semantic adjacency. |
| WFX-R08 | SPEC §1.3 | A real intervening semantic block prevents run-in. |
| WFX-R09 | SPEC §1.3 | A following list/table/admonition/source/image block is never forcibly concatenated as run-in body text. |
| WFX-R10 | SPEC §1.3 | With no eligible body, presentation is Separate. |
| WFX-R11 | SPEC §1.3 | With multiple paragraphs, only the first paragraph participates in visual run-in. |
| WFX-R12 | SPEC §1.3 | Run-in does not remove heading anchor/xref/outline identity. |
| WFX-R13 | SPEC §1.3 | Heading and paragraph exact source ranges always remain separate. |
| WFX-R14 | SPEC §1.3 | Heading and body inline ASTs are parsed separately; concatenated display text is never reparsed as source. |
| WFX-R15 | SPEC §1.3 | RTL/CJK/emoji presentation does not insert hard-coded LTR punctuation. |
| WFX-R16 | SPEC §1.3 | A synthetic visual join has no writable source scalar. |
| WFX-R17 | SPEC §1.3 | Copy Text yields rendered heading text + U+0020 + rendered first-paragraph text; Copy Source Fragment preserves heading source, all real blank/comment/attribute trivia between them, and paragraph source, and never writes the synthetic join back to source. |
| WFX-R18 | SPEC §1.3 | Enable retains only .run-in; Disable retains only .separate; Use Default removes both. |
| WFX-R19 | SPEC §1.3 | After Enter creates a second body paragraph, only the first paragraph remains run-in. |
| WFX-R20 | SPEC §1.3 | Backspace at body start must not silently destroy heading structure. |
| WFX-R21 | SPEC §1.3 | Authored deep headings 6-9 support run-in under the same rules. |
| WFX-R22 | SPEC §1.3 | If a backend cannot present run-in, only presentation may degrade; heading/body semantics remain intact. |
| T3-ENV-01 | SPEC §§1.2,4 | For the same source under html5 vs docbook processor backends, ifdef::backend-html5[] results and context digests differ as expected. |
| T3-ENV-02 | SPEC §§1.2,4 | SERVER/SECURE safe-mode docdir/docfile masking and SAFE results match 2.0.26. |
| T3-ENV-03 | SPEC §§1.2,4 | Changing logical docfile/baseDir correctly changes {docfile}/relative includes and the context digest. |
| T3-ENV-04 | SPEC §§1.2,4 | Changing SOURCE_DATE_EPOCH correctly changes local/doc intrinsic time attributes. |
| T3-ENV-05 | SPEC §§1.2,4 | The four hard_set, soft_set, hard_unset, and soft_unset states reproduce Ruby value/value@/nil/false precedence exactly. |
| T3-ENV-06 | SPEC §§1.2,4 | A changed managed-include SourceObservation makes the old evaluation dependency stale. |
| T3-ENV-07 | SPEC §§1.2,4 | An external immutable pin can be exactly replayed when bytes match; a different pin cannot reuse the old evaluation. |
| T3-ENV-08 | SPEC §§1.2,4 | Below SERVER safe mode, two ambientUserHome values produce different {user-home} results/context. |
| T3-ENV-09 | SPEC §§1.2,4 | At SERVER/SECURE, changing ambientUserHome does not change the processor default .. |
| T3-ENV-10 | SPEC §§1.2,4 | SOURCE_DATE_EPOCH overrides inputMtime; without it, inputMtime overrides clockNow for doc* attributes. |
| T3-ENV-11 | SPEC §§1.2,4 | When outfile/outdir are processor-visible, changing them must change the frozen context. |
| T3-ENV-12 | SPEC §§1.2,4 | Managed evaluation may not read cwd/home/clock/file metadata that is absent from the frozen context. |
| T3-ORACLE-01 | SPEC §§2–3 | When Ruby sourcemap lacks inline ranges, the Oracle witness remains valid and does not fabricate SourceOrigin. |
| T3-ORACLE-02 | SPEC §§2–3 | strong/emphasis/link/footnote are observed through actual Ruby semantic converter callbacks. |
| T3-ORACLE-03 | SPEC §§2–3 | Blocks with identical metadata but different inline semantics produce different witnesses. |
| T3-ORACLE-04 | SPEC §§2–3 | A Weftext provenance range off by one byte fails the provenance Gate even when semantic witnesses match. |
| T3-ORACLE-05 | SPEC §§2–3 | Core Gate may pass when product HTML DOM differs from Ruby DOM but semantic events match; HTML Backend Gate is independent. |
| T3-ORACLE-06 | SPEC §§2–3 | The observer may not normalize away target/type/role/terms/see fields such that genuinely different semantics collapse. |
| T3-ORACLE-07 | SPEC §§2–3 | Diagnostics retain exactMessage, and an incorrect semanticCode mapping must fail. |
| T3-ORACLE-08 | SPEC §§2–3 | Authored/included/substituted/synthetic provenance arms all round-trip through the decoder. |
| T3-ORACLE-09 | SPEC §§2–3 | A provenance graph cycle, forward reference, or unknown reference is rejected. |
| T3-ORACLE-10 | SPEC §§2–3 | A run-in synthetic join can reference only real heading/body origins and is not writable. |
| T3-ORACLE-11 | SPEC §§2–3 | A concealed indexterm produces a concealed semantic event even though HTML output is empty. |
| T3-ORACLE-12 | SPEC §§2–3 | Visible and concealed indexterms distinguish text/terms/see/see-also. |
| T3-ORACLE-13 | SPEC §§2–3 | toc::[] produces a block event with context=toc and content_model=empty. |
| T3-ORACLE-14 | SPEC §§2–3 | An unregistered fixed-2.0.26 built-in converter transform yields oracle_contract_incomplete and cannot be ignored. |
| T3-ORACLE-15 | SPEC §§2–3 | ListItem/Table::Cell facts are mechanically enumerated by the parent callback when no standalone callback exists; they cannot disappear. |
| T3-ORACLE-16 | SPEC §§2–3 | Alpha. and Beta. produce different text fragments even when all other metadata/catalog/diagnostics match. |
| T3-ORACLE-17 | SPEC §§2–3 | Literal blocks differing only in ordinary text must produce different witnesses. |
| T3-ORACLE-18 | SPEC §§2–3 | The result of legitimate substitutions in pass/raw blocks enters the text fragment; comparing node.source alone is insufficient. |
| T3-ORACLE-19 | SPEC §§2–3 | Alpha *{name}* Beta preserves ordered text -> strong start -> resolved text -> strong end -> text semantics. |
| T3-ORACLE-20 | SPEC §§2–3 | One {counter:x} increments exactly once; semantic observation must not double-increment via extra content access. |
| T3-ORACLE-21 | SPEC §§2–3 | Consecutive counters yield the actual stateful document order 1,2,3,... |
| T3-ORACLE-22 | SPEC §§2–3 | Footnote registration/numbering occurs once; fragment evidence agrees with the final catalog. |
| T3-ORACLE-23 | SPEC §§2–3 | Later attribute/macro substitutions still run inside quoted nodes; probe machinery cannot make quote payload opaque. |
| T3-ORACLE-24 | SPEC §§2–3 | A semantic pass attempting to evaluate the same scope a second time fails the oracle adapter directly. |
| T3-ORACLE-25 | SPEC §§2–3 | Backend Gate uses a fresh Ruby Document, not a Document already mutated by counter/footnote activity in the semantic pass. |
| T3-PROFILE-01 | SPEC §§5–8 | A managed Node cannot be switched to BaselineOnly by a caller. |
| T3-PROFILE-02 | SPEC §§5–8 | An ordinary import containing a deep marker lists deep_section in its DeltaReport. |
| T3-PROFILE-03 | SPEC §§5–8 | Ordinary source containing .run-in lists explicit_run_in in its DeltaReport. |
| T3-PROFILE-04 | SPEC §§5–8 | Root wf-kind/facets produce an explicit managed delta. |
| T3-PROFILE-05 | SPEC §§5–8 | n1/r1/citation/carrier/query/view all appear in the complete delta inventory. |
| T3-PROFILE-06 | SPEC §§5–8 | When there is no semantic delta, a /1 managed binding may be established directly, but the binding component must still be genuinely installed. |
| T3-PROFILE-07 | SPEC §§5–8 | If the user rejects managed activation, no managed Node is allocated. |
| T3-PROFILE-08 | SPEC §§5–8 | A trusted Weftext transfer preserves the exact profile generation. |
| T3-PROFILE-09 | SPEC §§5–8 | If a future /2 exists, an old /1 Node is still interpreted under /1. |
| T3-PROFILE-10 | SPEC §§5–8 | After source changes, an old DeltaReport digest cannot confirm a new activation. |
| T3-PROFILE-11 | SPEC §§5–8 | DeltaReport ordering/digest is deterministic across implementations. |
| T3-PROFILE-12 | SPEC §§5–8 | Profile migration requires explicit analysis/confirmation and never auto-upgrades on open. |
| T3-PROFILE-13 | SPEC §§5–8 | A fresh import Notice includes both document and document_format components. |
| T3-PROFILE-14 | SPEC §§5–8 | Document bytes without the format component are incomplete/proof_unavailable and never default to a profile. |
| T3-PROFILE-15 | SPEC §§5–8 | A completion proof missing document_format from its component set cannot be admitted into Frontier by a receiver. |
| T3-PROFILE-16 | SPEC §§5–8 | A profile-only migration is a real portable metadata change and gets a ChangeId even when source bytes are identical. |
| T3-PROFILE-17 | SPEC §§5–8 | A combined profile+source migration changes both components in one plan/CAS/seal. |
| T3-PROFILE-18 | SPEC §§5–8 | A same-Workspace copy creates a fresh Node with bindingRevision=1 while preserving the source profile generation. |
| T3-PROFILE-19 | SPEC §§5–8 | Fork does not automatically upgrade profile generation. |
| T3-PROFILE-20 | SPEC §§5–8 | Formal restore restores the exact historical binding; ordinary fresh-identity import follows the import gate. |
| T3-PROFILE-21 | SPEC §§5–8 | Every managed semantic read obtains both the exact SourceObservation and DocumentFormatCurrentQualification. |
| T3-PROFILE-22 | SPEC §§5–8 | A strong DependencyProof includes both source(owner) and document_format(owner). |
| T3-PROFILE-23 | SPEC §§5–8 | For a source-unchanged /1 -> /2 profile-only migration, the source dependency stays the same but the format stamp changes and the old D7 prepare becomes stale. |
| T3-PROFILE-24 | SPEC §§5–8 | scope_dependencies cannot classify a consumed document_format stamp change as unrelated. |
| T3-PROFILE-25 | SPEC §§5–8 | The D2 AST cache keys on format stamp/profile; metadata-only migration makes the old AST stale. |
| T3-PROFILE-26 | SPEC §§5–8 | D4 typed projections are invalidated and recomputed when the format stamp changes. |
| T3-PROFILE-27 | SPEC §§5–8 | D8 retains old Draft text, but the old projection cannot prepare directly; it must reproject using the current profile. |
| T3-PROFILE-28 | SPEC §§5–8 | A D9 frozen export binds the exact format qualification; a pre-execution change requires reprepare. |
| T3-PROFILE-29 | SPEC §§5–8 | Derived Index caches include the format stamp; migration invalidates heading/reference/query candidates. |
| T3-PROFILE-30 | SPEC §§5–8 | Trash-only lifecycle change preserves exact binding bytes/revision/profile/stamp. |
| T3-PROFILE-31 | SPEC §§5–8 | Restore uses the exact binding retained in Trash, never latest/default/BaselineOnly. |
| T3-PROFILE-32 | SPEC §§5–8 | Purge Notice/CP changes the current document_format component from present to absent. |
| T3-PROFILE-33 | SPEC §§5–8 | After purge, a tombstone cannot produce current ManagedDocumentSemanticQualification. |
| T3-PROFILE-34 | SPEC §§5–8 | Historical format bytes retained after purge serve only history/recovery and cannot become current binding. |
| T3-PROFILE-35 | SPEC §§5–8 | A document_format continuity gap creates a new stamp epoch; equal bytes cannot restore the old stamp. |
| T3-H01 | SPEC §1.3 | Article standard levels 1-5 match Ruby 2.0.26. |
| T3-H02 | SPEC §1.3 | Managed raw markers for levels 6-9 create the corresponding sections. |
| T3-H03 | SPEC §1.3 | A skipped level emits a warning and continues. |
| T3-H04 | SPEC §1.3 | A fragment root may start at a non-level-1 section while preserving the 2.0.26 root relaxation. |
| T3-H05 | SPEC §1.3 | A nested skip inside a fragment still emits the real expected-level warning. |
| T3-H06 | SPEC §1.3 | After a book document title, level 0 can become a part. |
| T3-H07 | SPEC §1.3 | An illegal article level 0 retains Ruby's original error/recovery semantics. |
| T3-H08 | SPEC §1.3 | Deep [discrete] / [float] headings do not enter section hierarchy. |
| T3-H09 | SPEC §1.3 | A deep discrete heading auto-ID enters the refs catalog normally. |
| T3-H10 | SPEC §1.3 | Positive leveloffset applies to ordinary and discrete headings. |
| T3-H11 | SPEC §1.3 | Doctitle recognition uses rawLevel+offset==0. |
| T3-H12 | SPEC §1.3 | Formal section parsing clamps a negative effective level to 0 as 2.0.26 does. |
| T3-H13 | SPEC §1.3 | An effective level 10+ produced by leveloffset is not invalid merely because Weftext authored levels stop at 9. |
| T3-H14 | SPEC §1.3 | Special appendix/bibliography/glossary semantics are not broken by the deep extension. |
| T3-H15 | SPEC §1.3 | :toclevels: 4 follows Ruby numeric comparison. |
| T3-H16 | SPEC §1.3 | :toclevels: 9 naturally includes the corresponding depth in the deep AST. |
| T3-H17 | SPEC §1.3 | :sectnumlevels: 2 numbers only through level 2. |
| T3-H18 | SPEC §1.3 | :sectnumlevels: 9 permits deep numbering through level 9. |
| T3-H19 | SPEC §1.3 | 11+ equals signs are not a Weftext deep-heading error; they continue through the baseline non-heading path. |
| T3-H20 | SPEC §1.3 | A standard level-5 marker plus leveloffset+10 can yield effective level 15 while preserving structure. |
| T3-H21 | SPEC §1.3 | HTML for effective levels above 9 preserves the exact aria/data level and reports only backend/AT degradation. |
| T3-H22 | SPEC §1.3 | DOCX/PDF uses explicit degradation for levels beyond backend capability and never rewrites source. |
| T3-TITLE-01 | SPEC §1.1 | = Main: Subtitle parses main=Main and subtitle=Subtitle. |
| T3-TITLE-02 | SPEC §1.1 | = A: B: C splits on the last :, yielding main=A: B and subtitle=C. |
| T3-TITLE-03 | SPEC §1.1 | [separator=::] uses :: correctly. |
| T3-TITLE-04 | SPEC §1.1 | :title-separator: :: takes effect correctly. |
| T3-TITLE-05 | SPEC §1.1 | WeftextManaged does not implicitly change the default separator to ::. |
| T3-TITLE-06 | SPEC §1.1 | D2, D8, and D9 read the exact same title/subtitle projection. |
| T3-FACET-01 | SPEC §§5,7 | Thirty-two lexically valid FacetIds succeed. |
| T3-FACET-02 | SPEC §§5,7 | The 33rd Facet is rejected for managed-domain commit. |
| T3-FACET-03 | SPEC §§5,7 | An exact duplicate FacetId is rejected. |
| T3-FACET-04 | SPEC §§5,7 | FacetId maximum-length boundaries follow the frozen decoder. |
| T3-FACET-05 | SPEC §§5,7 | Uppercase and illegal dot/hyphen/segment structures are rejected. |
| T3-FACET-06 | SPEC §§5,7 | A lexically valid but Registry-unknown Facet keeps the source valid while the typed provider layer reports unavailable. |
| T3-FACET-07 | SPEC §§5,7 | Host/API wf-facets cannot change root Node membership. |
| T3-FACET-08 | SPEC §§5,7 | wf-facets in included source cannot take over the including root Node. |
| T3-RUN-01 | SPEC §1.3 | With workspace policy=run_in and physical adjacency, implicit RunIn is allowed. |
| T3-RUN-02 | SPEC §1.3 | An implicit-default blank line forces Separate. |
| T3-RUN-03 | SPEC §1.3 | An implicit-default comment forces Separate. |
| T3-RUN-04 | SPEC §1.3 | Explicit .run-in allows blank/comment/paragraph metadata trivia and uses semantic adjacency for RunIn. |
| T3-RUN-05 | SPEC §1.3 | .separate overrides a workspace run_in default. |
| T3-RUN-06 | SPEC §1.3 | .run-in.separate yields a warning plus Separate without making source invalid. |
| T3-RUN-07 | SPEC §1.3 | A role conflict is a presentation result, not SyntaxStatus invalid. |
| T3-RUN-08 | SPEC §1.3 | Changing presentation-policy revision invalidates the D8 render cache. |
| T3-RUN-09 | SPEC §1.3 | A frozen D9 export continues using its frozen policy even if the workspace setting changes later. |
| T3-RUN-10 | SPEC §1.3 | Enable removes separate and leaves exactly run-in. |
| T3-RUN-11 | SPEC §1.3 | Disable removes run-in and ensures separate, so a run_in default cannot reapply. |
| T3-RUN-12 | SPEC §1.3 | Use Default removes both roles and resumes the current workspace policy. |
| T3-XF-01 | SPEC §§11–13 | Replacing [20,30) with 3 bytes shifts a later target left by 7. |
| T3-XF-02 | SPEC §§11–13 | A deletion strictly before a target applies the correct delta shift. |
| T3-XF-03 | SPEC §§11–13 | An edit strictly after a target leaves it unchanged. |
| T3-XF-04 | SPEC §§11–13 | An insertion at the start of a nonzero range moves the whole range right to preserve the original selected text. |
| T3-XF-05 | SPEC §§11–13 | An insertion at range end does not enlarge the old selection. |
| T3-XF-06 | SPEC §§11–13 | An insertion strictly inside the range stops exact mapping. |
| T3-XF-07 | SPEC §§11–13 | A zero-width target with left affinity stays before an insertion at the point. |
| T3-XF-08 | SPEC §§11–13 | A zero-width target with right affinity moves after an insertion at the point. |
| T3-XF-09 | SPEC §§11–13 | Multiple disjoint original edits accumulate deltas in original coordinates. |
| T3-XF-10 | SPEC §§11–13 | Mapping stops when the target truly overlaps a replacement. |
| T3-XF-11 | SPEC §§11–13 | A whole-source save with only before/after and no provenance yields compilation unavailable and no artifact. |
| T3-XF-12 | SPEC §§11–13 | A generic post-hoc diff can never create a trusted transform. |
| T3-XF-13 | SPEC §§11–13 | Revision-token-only trust authorization cannot validate the transform profile. |
| T3-XF-14 | SPEC §§11–13 | Signing is possible only after explicit root authorization of the exact transform profile. |
| T3-XF-15 | SPEC §§11–13 | A CAS loser's staging signature cannot become a portable artifact. |
| T3-XF-16 | SPEC §§11–13 | A required-plan signing failure prevents seal. |
| T3-XF-17 | SPEC §§11–13 | If the profile is unavailable at prepare time, emission can freeze disabled while the ordinary save still succeeds. |
| T3-XF-18 | SPEC §§11–13 | After historical key rotation, an older valid artifact continues to validate at its historical cut. |
| T3-XF-19 | SPEC §§11–13 | A transform-chain gap cannot be reconstructed from a current diff. |
| T3-XF-20 | SPEC §§11–13 | Recovery of a required plan cannot downgrade it to disabled. |
| T3-XF-21 | SPEC §§11–13 | Recovery of a disabled plan cannot upgrade it to required. |
| T3-XF-22 | SPEC §§11–13 | Multiple independent inserts at one point are canonically merged in real transaction order. |
| T3-XF-23 | SPEC §§11–13 | A boundary insertion is not forcibly merged merely because it touches a replacement. |
| T3-XF-24 | SPEC §§11–13 | Truly overlapping nonzero replacements can be mechanically islanded. |
| T3-XF-25 | SPEC §§11–13 | Event replay must produce the exact admitted afterPin and afterSourceSha256. |
| T3-XF-26 | SPEC §§11–13 | [5,10)->X plus insert@10->Y with left point 10 must yield X\|Y. |
| T3-XF-27 | SPEC §§11–13 | For insert@5 plus replace[5,10), after-order is unique and point5 mapping stops conservatively because the replacement starts at the point. |
| T3-XF-28 | SPEC §§11–13 | A required sealed decision has exactly one outbox item keyed by {changeId,ownerNodeRef}. |
| T3-XF-29 | SPEC §§11–13 | Different artifacts/pins under the same outbox key are an integrity conflict, not LWW. |
| T3-XF-30 | SPEC §§11–13 | Publication recovery constructs the exact outbox key only from the saved decision. |
| T3-XF-31 | SPEC §§11–13 | After [5,10)->"XY", typing ! between generated X/Y must remain representable as one [5,10)->"X!Y" event. |
| T3-XF-32 | SPEC §§11–13 | Continued typing/backspace inside an initial insertion folds into that insertion event rather than becoming arbitrarily disabled. |
| T3-XF-33 | SPEC §§11–13 | An edit wholly inside one generated replacement payload folds mechanically into that event. |
| T3-XF-34 | SPEC §§11–13 | A cross-anchor edit that cannot uniquely preserve original anchor/boundary slots returns PortableTransformCompilation.unavailable. |
| T3-XF-35 | SPEC §§11–13 | An unavailable result is decided before planning and freezes disabled; the ordinary source save still succeeds. |
| T3-XF-36 | SPEC §§11–13 | Recovery cannot re-diff an unavailable/disabled transform into a representable one. |
| T3-XF-37 | SPEC §§11–13 | For decoder-valid events, generated payload is sliced from exact afterPin using deterministic output spans, never content search. |
| T3-XF-38 | SPEC §§11–13 | Repeated identical byte sequences in after source do not make payload extraction ambiguous because offsets determine it. |
| T3-XF-39 | SPEC §§11–13 | A mismatch between event payload digest/length and the sliced afterPin rejects the transform. |
| T3-XF-40 | SPEC §§11–13 | For every input transaction sequence, the compiler returns either representable or one closed unavailable reason; there is no missing return or implementation-defined branch. |
| T3-SUG-01 | SPEC §§10,15 | A confirmed pending replace accept writes Document+accepted under the same ChangeId. |
| T3-SUG-02 | SPEC §§10,15 | Reject is Annotation-only pending -> rejected. |
| T3-SUG-03 | SPEC §§10,15 | Concurrent accept/reject has exactly one Annotation source-version winner. |
| T3-SUG-04 | SPEC §§10,15 | accepted/rejected are terminal and cannot be accepted/rejected again. |
| T3-SUG-05 | SPEC §§10,15 | Explicit reanchor moves a pending suggestion to needs_reconfirmation. |
| T3-SUG-06 | SPEC §§10,15 | Reconfirmation updates the stored target basis from the current exact target. |
| T3-SUG-07 | SPEC §§10,15 | targetBasis hashes only the stored target in the current Annotation Value. |
| T3-SUG-08 | SPEC §§10,15 | A zero-width stored insert target uses the same basis rule. |
| T3-SUG-09 | SPEC §§10,15 | If target-source disclosure is lost, Accept stops before reading transform evidence. |
| T3-SUG-10 | SPEC §§10,15 | Without Annotation read permission, Reject must not disclose suggestion state. |
| T3-SUG-11 | SPEC §§10,15 | Reject requires annotation_read plus annotation_write. |
| T3-SUG-12 | SPEC §§10,15 | Reject does not require Document target/source read permission. |
| T3-SUG-13 | SPEC §§10,15 | A V1 suggestion carried through a signed unrelated V1->V2 edit still validates basis against the stored V1 target while the chain proves the V2 current target, allowing a fresh accept. |
| T3-SUG-14 | SPEC §§10,15 | A mapped V2 locator hash differing from the stored basis is not itself failure because they are not equality operands. |
| T3-SUG-15 | SPEC §§10,15 | Expected bytes/point are tested against the fresh current target, not old V1 bytes. |
| T3-ACTOR-01 | SPEC §§9,16 | The trusted host/Core supplies the authenticated actor snapshot; callers cannot author authenticated attribution. |
| T3-ACTOR-02 | SPEC §§9,16 | The same real person on two devices may have different presentation and is not claimed to have a stable person ID. |
| T3-ACTOR-03 | SPEC §§9,16 | Same-Workspace copy preserves review attribution. |
| T3-ACTOR-04 | SPEC §§9,16 | Cross-Workspace trusted transfer preserves originWorkspace and cannot impersonate a target-Workspace authenticated actor. |
| T3-ACTOR-05 | SPEC §§9,16 | Ordinary untrusted import is marked imported_unverified. |
| T3-ACTOR-06 | SPEC §§9,16 | Formal same-Workspace restore preserves attribution exactly. |
| T3-ACTOR-07 | SPEC §§9,16 | A time snapshot never decides a conflict winner. |
| T3-ACTOR-08 | SPEC §§9,16 | creator/authoredAt remain byte-identical and immutable across later mutations. |
| T3-ACTOR-09 | SPEC §§9,16 | Core rewrites lastEditor/editedAt on every legal mutation. |
| T3-ACTOR-10 | SPEC §§9,16 | The time producer is explicitly prepare_server/device_clock and is not claimed to be a commit timestamp. |
| T3-ACTOR-11 | SPEC §§9,16 | Saved/replayed preparation uses the frozen time and does not resample. |
| T3-INLINE-01 | SPEC §§1–4,9 | One paragraph fully parses legal inline strong/emphasis/link and related semantics. |
| T3-INLINE-02 | SPEC §§1–4,9 | Soft line wraps remain one paragraph and are permitted. |
| T3-INLINE-03 | SPEC §§1–4,9 | A blank line creating a second paragraph causes the complete-consumption Gate to reject the Annotation body. |
| T3-INLINE-04 | SPEC §§1–4,9 | Heading/list/table/delimited-block syntax cannot be silently ignored by the inline profile; it produces invalid_annotation_body. |
| T3-INLINE-05 | SPEC §§1–4,9 | Inline STEM syntax can be valid; missing renderer affects rendering only. |
| T3-INLINE-06 | SPEC §§1–4,9 | n1/r1 text inside Annotation body is handled by ordinary inline syntax and does not secretly acquire the Weftext stable-reference adapter authority. |
| T3-INLINE-07 | SPEC §§1–4,9 | Source/body/render limits fail closed at the N/N+1 boundary. |
| O34-01 | SPEC §§2–3 | For the same source/environment, uninstrumented and observed selected-backend runs have identical final bytes, diagnostics, catalog, counters, and attributes. |
| O34-02 | SPEC §§2–3 | The observer does not replace converter return objects, mutate bytes/encoding, or insert markers; each actual return continues unchanged into subsequent processing. |
| O34-03 | SPEC §§2–3 | For https://pre**mid**post.example, the post-quote string and actual URL match/captures/target equal uninstrumented Ruby; the observer does not repair or clean an expected URL. |
| O34-04 | SPEC §§2–3 | For link:pre**mid**post[label], target scalar comes from the actual capture and preserves converter-feedback characters, traceable to the strong return but not mis-modeled as visual nesting in the label. |
| O34-05 | SPEC §§2–3 | In Alpha *{name}* Beta, later attribute rewriting remains visible; the old quote argument, later string versions, and final text relationship are all evidenced. |
| O34-06 | SPEC §§2–3 | Three repeated identical texts or labels in one scope are distinguished by actual spans, never by content search. |
| O34-07 | SPEC §§2–3 | Alpha. vs Beta., differing literal bodies, and differing pass bodies produce different semantic observations even when metadata matches. |
| O34-08 | SPEC §§2–3 | A concealed indexterm has a real Inline event, match position, terms/see/see-also even though it returns an empty string; no catalog/HTML reconstruction is used. |
| O34-09 | SPEC §§2–3 | Both explicit + and hardbreaks-option paths preserve complete pre-break text and real ranges without loss or duplication in projection. |
| O34-10 | SPEC §§2–3 | Counter/counter2/set/footnote numbering-registration counts and order are unchanged by observation; log serialization cannot trigger a second content evaluation. |
| O34-11 | SPEC §§2–3 | Native call sequences for list items, description terms/bodies, ordinary cells, a cells, and title/cache hits are preserved; the observer neither adds calls nor deduplicates them. |
| O34-12 | SPEC §§2–3 | Native passthrough extraction/restoration, nested apply_subs, drop-line, and post-escape interval relationships remain correct with no observer sentinel. |
| O34-13 | SPEC §§2–3 | Target/refid/path and AttributeList named/positional slice/decode/normalize origins are traceable through actual operations without extra getters or scanners. |
| O34-14 | SPEC §§2–3 | Missing observation, an unknown required transform, or capacity exhaustion cannot emit a “complete passing witness”; the original evaluation result is never changed to fill the gap. |
| O34-15 | SPEC §§2–3 | A product renderer may use different outer DOM/CSS without failing Core Gate; real target/text/substitution inequality still fails. |
| O34-16 | SPEC §§2–3 | The five retained closed-type positive/negative decoder vectors still pass; Witness/6 has no dangling types and the old marker/InlineDescriptor/Witness5 are not current emitters. |
| X34-01 | SPEC §§2,11–13 | A replacement spanning two generated anchors after insert@5->AA and insert@10->BB still validates boundary invariants; if they cannot be preserved it is unavailable rather than forced representable. |
| X34-02 | SPEC §§2,11–13 | Continued editing inside one insertion/replacement and ordinary disjoint edits remain representable when invariants hold; the compiler cannot disable them wholesale. |
| X34-03 | SPEC §§2,11–13 | The mixed-event fixture maps [10,14) to [11,15) and point14 left/right to 15/17 using original-coordinate classification only. |
| X34-04 | SPEC §§2,11–13 | [5,10)->X plus insert@10->Y preserves left point as X\|Y; a replacement starting at the point stops mapping. Multi-transition chains validate each segment. |
| X34-05 | SPEC §§2,11–13 | CoreSourceEditPlan/2.edits and SourceTransformEvidence/2.edits are the same canonical /3 events; byte inequality rejects seal, and seal/recovery never recompiles them. |
| X34-06 | SPEC §§2,11–13 | The current decoder rejects mixing retired PortableEdit/2 with current /3; genuinely historical saved/planned data keeps its original rules. Compilation unavailable does not block ordinary source save and recovery cannot upgrade emission. |
| C34-01 | SPEC §§2,11–13 | Fresh coordination has one authoritative FC-3.4 closed list; missing D4 typed/control/carrier, D9, or Index/cache means coordination is incomplete, and summaries may only reference this closed list. |
| C34-02 | SPEC §§2,11–13 | All 340 v3.3 IDs and obligations remain; this round adds exactly O34×16, X34×6, C34×2 =24. They remain design oracles and are not reported as executed tests. |
| P35-01 | SPEC §§2–3 | Case 1 yields final Alice with strong range [6,11); Rust internal storage as raw+final or final-only projects identically. |
| P35-02 | SPEC §§2–3 | Case 2 preserves a target containing <strong>mid</strong> exactly; the projection contains no intermediate strong mark or dependency edge. |
| P35-03 | SPEC §§2–3 | Case 3 preserves strong formatting in the label while target=dest; it is not confused with Case 2 scalar consumption. |
| P35-04 | SPEC §§2–3 | Case 4 contains Alpha exactly once and one hard_break, with neither lost prefix nor extra soft_break. |
| P35-05 | SPEC §§2–3 | Case 5 gives the hidden term a deterministic zero-width semantic site and no visible hidden text. |
| P35-06 | SPEC §§2–3 | Case 6 has one footnote definition, a ref atom, and correct final counter with no duplicate body; a later same-id reference does not create a second definition. |
| P35-07 | SPEC §§2–3 | Case 7 locates two strong ranges over identical text distinctly without substring search. |
| P35-08 | SPEC §§2–3 | Case 8 counter2 has no visible output but still affects later text/final state; comparing visible text alone is insufficient. |
| P35-09 | SPEC §§2–3 | When custom substitutions run macro before attribute, later real target/label rewrites appear in final scalar/flow; projection cannot freeze the earliest callback field. |
| P35-10 | SPEC §§2–3 | Authored passthrough <strong>x</strong> does not become a normal strong mark; transport specialchars, real replacements, and raw backend feedback remain distinct. |
| P35-11 | SPEC §§2–3 | Pure formatting later consumed/deleted may disappear, but real index/anchor/footnote/counter/catalog effects cannot disappear with it. |
| P35-12 | SPEC §§2–3 | Different raw IDs, cache counts, or inline-tree implementations project to the same canonical CSP when final runs/marks/scalars/facts/state match. |
| P35-13 | SPEC §§2–3 | CJK/emoji, soft breaks, literal newlines, and mark ranges use Unicode-scalar coordinates, never raw Ruby byte offsets. |
| P35-14 | SPEC §§2–3 | Missing required observation or Core final state cannot be filled by a second parser or HTML inference; the compatibility case remains unpassed rather than being removed from scope. |
| N36-01 | SPEC §§2–3 | Quote/verse attribution and citetitle are canonical properties; changing Alice to Bob or changing the cited title changes CSP even if body text is unchanged. |
| N36-02 | SPEC §§2–3 | [source,ruby] and [source,python] differ by language; listing context/style/source-language inheritance and fenced-source paths project into the same canonical slot. |
| N36-03 | SPEC §§2–3 | Numeric positional keys, named rekey results, and style/id/role/option physical aliases produce one canonical slot; native precedence uses the real result and is not re-decided by the projector. |
| N36-04 | SPEC §§2–3 | Real authored named attributes such as ticket=OPS-7 and custom=007 remain; 007 remains text and a native role cannot collide with an authored custom roles attribute. |
| N36-05 | SPEC §§2–3 | Internal cloaked-context, caches, reader/column objects, and temporary root-option do not enter CSP; a real authored same-name property is not deleted by name alone and unknown objects cannot be stringified generically. |
| N36-06 | SPEC §§2–3 | Every CoreBlockKind and mark/atom maps to exactly one profile row; table column/span/alignment, section numbering, image parameters, and callout associations all have positive/negative changes and cannot be omitted as “unlisted public fields.” |
| N36-07 | SPEC §§2–3 | All eight v3.5 expected projections remain exact; ordinary paragraphs gain no default subs/style/empty options, and footnote/link body text is not duplicated. |
| N36-08 | SPEC §§2–3 | Ruby without column and Rust with an extra column produce equal diagnostics when logicalFile/line/code match; code uses the same canonical namespace. |
| N36-09 | SPEC §§2–3 | No location has exactly one representation null; file-only and line-only locations remain expressible; no guessed location/second parser is allowed and duplicate diagnostics are not deduplicated. |
| N36-10 | SPEC §§2–3 | links/images/includes canonicalize as equal multisets regardless of input order while preserving multiplicity; removing one duplicate changes CSP. |
| N36-11 | SPEC §§2–3 | Property, attributeChanges, and counters sort uniquely by name and reject conflicting duplicate keys; ordered arrays such as roles/menu/columns/cells/children are not globally sorted. |
| N36-12 | SPEC §§2–3 | Footnote index 2 sorts before 10; CoreSite numeric paths and index-occurrence multiplicity are correct, and inner properties are canonicalized before an outer CJ sorting key is computed. |
| M37-01 | SPEC §§2–3 | Witness/7 is the only current witness; missing modelPropertyObservations or pretending Witness6 has an empty one fails the full-property Gate, while all retained nested types remain. |
| M37-02 | SPEC §§2–3 | nil/absent/false/Integer/Symbol/Float/String remain distinct; :chapter cannot stringify to true and Float preserves its actual numeric encoding. |
| M37-03 | SPEC §§2–3 | An ordinary book chapter and a book special section record actual true vs :chapter numbered states; the projector never reruns initialization logic. |
| M37-04 | SPEC §§2–3 | Section numeral/caption uses the result after assign_numeral/assign_caption; constructor/initialize_section intermediate values cannot impersonate final state. |
| M37-05 | SPEC §§2–3 | For cols=1,2, final widths use the actual post-balance result; the first 66.6666 snapshot for C2 cannot be selected instead of final 66.6667. |
| M37-06 | SPEC §§2–3 | Auto-created columns, absolute width, autowidth, and final-column correction all have real typed evidence and are not recomputed from source. |
| M37-07 | SPEC §§2–3 | A header Cell's @style is distinct from inherited attributes style; final halign/valign/span are correct. |
| M37-08 | SPEC §§2–3 | When reinitialize returns a new Cell, the final row binds the new subject and the old temporary Cell does not also enter CSP. |
| M37-09 | SPEC §§2–3 | ListItem marker/checklist/id/roles/options are complete; after fold_first, the folded paragraph does not become a second body node. |
| M37-10 | SPEC §§2–3 | coids are observed after the actual parse_callout_list assignment and cannot stop at the earlier parse_list_item return. |
| M37-11 | SPEC §§2–3 | Description-list multiple terms, nil description, and block-only description follow the real tuple relationships rather than text-based object guessing. |
| M37-12 | SPEC §§2–3 | Named/positional/rekey/copy/inherit provenance is fully traceable; a real overwrite with the same value still updates provenance and cannot be collapsed by value equality. |
| M37-13 | SPEC §§2–3 | Explicit foo-option=bar projects both option membership and the named value; bar→baz changes CSP. |
| M37-14 | SPEC §§2–3 | Explicit empty foo-option= keeps a named empty value; %foo/options=foo/opts=foo produce membership only. |
| M37-15 | SPEC §§2–3 | When named assignment and option expansion overwrite each other, the actual final writer wins; an overwritten bar is not resurrected. |
| M37-16 | SPEC §§2–3 | Authored root-option and converter temporary root-option are distinguished by provenance; temporary writes/deletes neither create nor erase authored property facts. |
| M37-17 | SPEC §§2–3 | Model snapshots do not add calls to content/text/title/reftext/alt/parse/reinitialize/width/counter; output and side-effect counts satisfy the existing neutrality Gate. |
| M37-18 | SPEC §§2–3 | Repeated observation/cache consumption of one object forms one subject chain; distinct objects with identical content are not merged and bind cannot be reconstructed by string or location search. |
| M37-19 | SPEC §§2–3 | Every PropertyProfile input group has either a retained concrete member or a new producer site; removing a required fact fails the evidence Gate rather than shrinking valid-language scope. |
| M37-20 | SPEC §§2–3 | Missing/duplicate/conflicting evidence, invalid provenance chains, and non-finalized intermediate values are gated; complete evidence still yields the original eight projections without unrelated default properties. |
| M38-01 | SPEC §§2–3 | Observation IDs in modelPropertyObservations are strictly increasing and unique; duplicates or reversal reject. |
| M38-02 | SPEC §§2–3 | The same real Ruby object keeps one subjectId across snapshots; distinct objects cannot reuse it, and old/new reinitialized Cells have different IDs. |
| M38-03 | SPEC §§2–3 | snapshotObservationId=x means exactly snapshot.observationId=x; there is no implicit ordinal namespace. |
| M38-04 | SPEC §§2–3 | The first snapshot per subject has previous=null and every later snapshot points exactly to the direct predecessor; skip/branch/cross-subject/forward links reject. |
| M38-05 | SPEC §§2–3 | A bind references an existing same-subject strictly earlier snapshot that is exactly latest at bind time; stale snapshot binds reject. |
| M38-06 | SPEC §§2–3 | Cut heads sort numerically by subjectId and contain exactly one head per subject; duplicates reject. |
| M38-07 | SPEC §§2–3 | If a treeprocessor returns a new Document, model_ready.documentSubjectId comes from the latest document/0 bind and the old root is not automatically live. |
| M38-08 | SPEC §§2–3 | All live header/child/List/ListItem/Column/final Cell/inner Document subjects reachable by forward roles are in the closure; omitting one rejects. |
| M38-09 | SPEC §§2–3 | parent/cell_column backedges cannot expand closure; attribute_buffer/table_parser_context do not become heads merely because snapshots exist. |
| M38-10 | SPEC §§2–3 | When first-width and post-balance Column snapshots both exist, cut selects only the latter. |
| M38-11 | SPEC §§2–3 | If Cell#reinitialize returns a new object and the final row points to it, the new Cell has the head and the old Cell is not an extra head. |
| M38-12 | SPEC §§2–3 | Snapshots after late coids and assign_numeral/caption become the subject heads; earlier snapshots reject. |
| M38-13 | SPEC §§2–3 | A legitimate semantic mutation after model_ready forces evaluation_complete to reference its later snapshot, never the old model_ready head. |
| M38-14 | SPEC §§2–3 | After a real temporary write followed by real restore/closure, evaluation_complete selects the latest snapshot while preserving the authored winner; an unclosed temporary state prevents a valid complete cut. |
| M38-15 | SPEC §§2–3 | Dangling/forward refs, missing live subjects, extra unreachable heads, and stale intermediate heads explicitly reject rather than becoming “unsupported syntax.” |
| M38-16 | SPEC §§2–3 | All previous/provenance/operation/string/inline evidence reachable from a head exists with the right type; internal supporting evidence may exist without becoming a head, and no parser/getter is called to fill evidence. |
| M39-01 | SPEC §§2–3 | A large operationId in another namespace is not rejected merely because it is numerically greater than the model observationId; existence/type/original operation DAG decide validity. |
| M39-02 | SPEC §§2–3 | A smaller inlineEventId does not prove it happened earlier than a model observation; the event must actually exist, have the right type, and satisfy its own reference contract. |
| M39-03 | SPEC §§2–3 | ModelPropertyInput.model_slot.observationId is still a same-namespace snapshot reference and must be strictly earlier; forward/self/non-snapshot targets reject. |
| M39-04 | SPEC §§2–3 | A carrier entry is an array index, not an event ID, and is never compared numerically with model observationId. PASS simultaneously requires final stream/index/type validity, producer-conformance evidence that entry < targetStream.lengthAtBind at the real bind callback, and that the callback held the exact same Ruby object. Appending the carrier later cannot make an earlier invalid bind valid. |
| M39-05 | SPEC §§2–3 | A root catalog record stores parent -> the actual top-level Document#register receiver; a nonexistent owner relation rejects. |
| M39-06 | SPEC §§2–3 | A catalog record created by an inner Document stores parent -> that inner Document, which is live through cell_inner_document; it is not reassigned to the root. |
| M39-07 | SPEC §§2–3 | Real DocBook cleanup for authored root-option follows authored bar -> temporary empty -> deleted/absent physical state; evaluation_complete selects the delete snapshot while CSP can still retain the authored semantic winner, and no fake restore is logged. |
| M39-08 | SPEC §§2–3 | Temporary cleanup cannot revive an older authored value: A -> authored B -> temporary empty -> cleanup delete yields physical absent and semantic authored winner B; a later authored delete makes the semantic winner absent. |
| M39-09 | SPEC §§2–3 | Future carrier reject: snapshot at T1, bind inlineObservations[7] at T2, actual append at T3 is producer-conformance failure even if final entry 7 exists; later completion cannot cure the invalid run. |
| M39-10 | SPEC §§2–3 | document/0 positive: the actual returned top-level Document carrier is published first and the later bind callback still holds the exact same Ruby object. |
| M39-11 | SPEC §§2–3 | document/0 negative: binding before publication, or later re-finding a Document by title/source/text, fails even if final entry0 is correct. |

## Actual-owner coordination — 212

| ID | Spec area | Requirement |
|---|---|---|
| FC4R-PROD-01 | SPEC §3.4 | Two fixed-2.0.26 source blocks differing only in declared language, e.g. ruby vs java, produce distinct D2DelimitedSemantics/1 language values and remain distinguishable to D8/D9 without reparsing exact source. |
| FC4R-PROD-02 | SPEC §3.4 | A source/listing block with linenums/start/indent/tabsize/highlight/line-comment facts round-trips those actual fixed-derived semantics through D2DocumentSnapshot/3; omitting one required present fact or inventing a host default fails the product projection. |
| FC4R-PROD-03 | SPEC §3.4 | [quote,Alice,Book] and [quote,Carol,Book] with the same body produce different attribution while retaining citetitle; D9 rendering consumes these product facts rather than recovering them from raw source. |
| FC4R-PROD-04 | SPEC §3.4 | A fixed-Ruby appendix/special/numbered section differs from an ordinary section with equal visible heading text; sectname/special/numbered are preserved and affect outline/TOC/render semantics. |
| FC4R-PROD-05 | SPEC §3.4 | Visible and concealed indexterms with equal terms remain distinguishable, and see/see-also values are preserved exactly; DocBook-style output need not reparse the source to recover them. |
| FC4R-PROD-06 | SPEC §3.4 | Quoted inline text carrying #id, .role, or #id.role retains id and ordered roles in the product projection; CSS/anchor/accessibility consumers do not lose those fixed-Ruby facts. |
| FC4R-PROD-07 | SPEC §3.4 | Audio/video product projection preserves the actual start/end/options and applicable poster/width/height/preload/list/playlist/theme/lang/control facts; equal target URL with different media semantics cannot collapse to one product value. |
| FC4R-PROD-08 | SPEC §3.4 | A legal arbitrary native attribute name remains representable through D2NativeAttributeSet/1 using only the closed D2NativeSemanticValue/1 union; an unknown property name is not itself syntax-invalid and arbitrary JSON values are rejected. |
| FC4R-PROD-09 | SPEC §3.4 | Ordered/unordered/description/callout/checklist list semantics preserve style/start/reversed/checklist/interactive/coids where applicable, while item order and duplicate visible text remain unchanged. |
| FC4R-PROD-10 | SPEC §§3.4,8.3 | Native table format/grid/frame/stripes and physical columnStart/colspan/rowspan plus TOC levels are available to product consumers; a valid complex table or TOC cannot require a second parser merely to render/export correctly. |
| FC4R-ORIGIN-01 | SPEC §3.4 | For :who: Alice followed by Hello {who}, the resulting Alice value records authored definition/reference/substitution provenance and derives a unique writable authored range only when all retained origin paths actually converge there. |
| FC4R-ORIGIN-02 | SPEC §3.4 | A generated or multi-origin semantic value with no unique authored writable range remains readable and exact-Source-saveable, while structured write is unavailable rather than guessed to one range. |
| FC4R-ORIGIN-03 | SPEC §§3.4,8.1–8.3 | Root R containing include::child.adoc[] with child V1=Alpha then V2=Beta invalidates the V1 D2 snapshot for D7/D8/D9 even if R bytes and document_format stay equal; managed include SourceObservation equality is required. |
| FC4R-ORIGIN-04 | SPEC §3.4 | Changing an artifact/network include pin or network request digest makes the old evaluation binding stale; equal logicalPath or resulting text cannot prove currentness. |
| FC4R-ORIGIN-05 | SPEC §3.4 | Changing ordered API attributes, locale/provider profile, sourceDateEpoch or another processor-environment input changes the environment digest and invalidates an old product evaluation even when final visible text happens to match. |
| FC4R-ORIGIN-06 | SPEC §3.4 | An unavailable global fact that is not in the product evaluation's closed actual dependency set does not block current product consumption; scope_dependencies proves only the dependencies actually bound by that evaluation. |
| FC4R-LEVEL-01 | SPEC §§3.4,8.1 | A legal fixed-Ruby effective level greater than 2^63-1, such as from a huge leveloffset, is encoded as the D7 integer canonical decimal value and is not rejected or truncated as int64/Counter. |
| FC4R-LEVEL-02 | SPEC §§3.4,8.1 | Ordinary effective levels 1–9 preserve the existing heading/query/outline/render path with no new compatibility branch or display change merely because the current level type is arbitrary precision. |
| FC4R-LEVEL-03 | SPEC §3.4 | If processing an extremely large legal effective level exhausts bounded work/memory, the operation returns budget_exceeded/unavailable under the resource contract; it must not reclassify the source as syntax-invalid or numeric_overflow. |
| FC4R-RUN-01 | SPEC §8.2 | Explicit .separate renders Separate even when the Workspace presentation-policy record is missing, corrupt, or conflicted; no policy read/pin is required for that presentation decision. |
| FC4R-RUN-02 | SPEC §8.2 | Explicit .run-in with an eligible semantic-adjacent body may render RunIn while Workspace policy has multiple heads; the explicit source role does not depend on choosing a policy branch. |
| FC4R-RUN-03 | SPEC §8.2 | With neither role and a body that actually needs the implicit Workspace default, missing/corrupt/multiple policy heads makes default-dependent presentation unavailable; no ambient host/device default substitutes. |
| FC4R-RUN-04 | SPEC §8.2 | No eligible body yields the closed no_eligible_body→Separate decision and does not read Workspace policy, regardless of the current policy state. |
| FC4R-RUN-05 | SPEC §8.2 | Two offline writers from revision1 may produce distinct revision2 policy records; sync retains both record hashes as maximal heads and neither revision equality, arrival order nor LWW chooses current. |
| FC4R-RUN-06 | SPEC §8.2 | An authorized conflict resolution over multiple presentation heads names the complete expected head set and creates one checked max(parent.revision)+1 multi-parent successor in one D6 planning CAS/P decision; a partial head set is stale/invalid. |
| FC4R-RUN-07 | SPEC §8.2 | Saved/planned/unknown presentation-policy mutation restores its exact original head set, proposal, pins and P responsibility; it never resamples newer heads or retries as another decision. |
| FC4R-RUN-08 | SPEC §8.2 | A policy-head change invalidates only render-cache entries whose D8PresentationDecision is workspace_default; explicit/no-body/conflict-fallback entries do not acquire a policy dependency they never consumed. |
| FC4R-RUN-09 | SPEC §§8.2–8.3 | A D9 plan staged with workspace_default policy head P remains byte-frozen if policy later changes; a plan with an explicit source-role decision contains no policy binding at all, and fresh preparation alone consumes newer policy state. |
| FC4R-EXP-01 | SPEC §8.3 | asciidoc_source produces a unique Plan3 with generationPolicy=none and null template/route/render bindings even when all renderer/template/provider registries are unavailable. |
| FC4R-EXP-02 | SPEC §8.3 | resource_exact and query_json likewise have a unique policy-independent positive encoding and cannot be disabled by unavailable Office generation configuration. |
| FC4R-EXP-03 | SPEC §8.3 | missingPolicy=empty may apply only to an existing exact template path whose authorized projection is none; unknown path, unreadable data, type error or unavailable schema still fails and cannot be converted to empty. |
| FC4R-EXP-04 | SPEC §8.3 | imageSizes is ResourceRef-key unique with positive micrometre dimensions; required layout choice is bound to that same authorized resource, and missing metadata/layout loss follows fixed Templates rules rather than a filename/digest guess. |
| FC4R-EXP-05 | SPEC §8.3 | D9ExportTemplateBinding/1.inputIndex selects exactly one catalog template item, its pin is byte-equal to that item pin, and profileId/profileVersion identify the accepted decoder for those exact bytes. |
| FC4R-EXP-06 | SPEC §8.3 | Every nonnull route has a continuous provider/profile chain whose terminal profile matches the export target; a target/terminal mismatch fails instead of silently selecting another route. |
| FC4R-EXP-07 | SPEC §8.3 | For the same semantic Plan3, evidencePins is exactly the recursive typed PinRef union plus recoveryPins. Omitting a reachable template/style/route/snapshot/dependency/staged pin or adding an unrelated pin fails, so two conforming encoders produce identical Plan bytes. |
| FC4R-EXP-08 | SPEC §8.3 | Fresh Plan3/Receipt3 tokens use d9_export_plan/3 and d9_publication/3; inspect/confirmation/recovery dispatch token tag before record version, while genuine Plan/Receipt1-/2 retain original tags/bytes/pins and are never repinned as /3. |
| FC4R-EXP-09 | SPEC §8.3 | Once Plan3 has frozen source/include/render/presentation/generation inputs and staged bytes, later source/include/policy/template/route change cannot rerender an unknown/saved publication; current reprepare is a new plan under the original authorization/error order. |
| FC4R-TABLE-01 | SPEC §8.3 | A native table with one uniquely named lowercase-ASCII leaf column amount may expose COLUMN=amount and SET=native_table with no author export metadata. |
| FC4R-TABLE-02 | SPEC §8.3 | In a multi-row header where two leaf columns are Amount under distinct parents Q1 and Q2, leaf alone is ambiguous and the shortest unique header suffix selects the intended physical column deterministically. |
| FC4R-TABLE-03 | SPEC §8.3 | Two tables containing the same header suffix are disambiguated next by exact table title; the resolver does not merge their SET namespaces or choose the first table. |
| FC4R-TABLE-04 | SPEC §8.3 | When table title and full header path are still byte-equal, the closed zero-based table/column occurrence qualifier is required; the ordinal is computed from the frozen product projection, not UI/memory order. |
| FC4R-TABLE-05 | SPEC §8.3 | CJK, RTL, combining-character and normalization-distinct header text compare by exact Unicode scalar sequence with normalization=none/case-sensitive/whitespace-preserving semantics; non-ASCII qualified names use the fixed ASCII digest token. |
| FC4R-TABLE-06 | SPEC §8.3 | Empty header cells, repeated labels and merged colspan/rowspan heads construct deterministic headerPath segments from D2 table-grid facts; rowspan does not duplicate one source cell into later path levels. |
| FC4R-TABLE-07 | SPEC §8.3 | Adding a later colliding table/column can make a previously short fresh selector ambiguous and require a qualified template token; an already prepared plan remains frozen to its original projection/selector/staged bytes. |
| FC4R-TABLE-08 | SPEC §8.3 | Every data.SET.COLUMN token in one repeat SET must resolve to the same selected table/rowset; a token that resolves another SET/table, an unknown column or irreducible collision is template_invalid/ambiguous rather than an implicit join. |
| FC4R-WIRE-01 | SPEC §§7–8 | A legal replica_local create_node/move_node/reorder_node/trash current wire13 request omits expectedAuthority, workspaceProposal and preparationBinding and strict-decodes successfully under the inherited mode matrix. |
| FC4R-WIRE-02 | SPEC §§7–8 | The same replica_local request with expectedAuthority present, workspaceProposal present, preparationBinding present, or any of those encoded as JSON null fails closed decode/mode validation. |
| FC4R-WIRE-03 | SPEC §§7–8 | managed_atomic create/fork requires expectedAuthority=create plus the inherited required proposal, continue requires continue, and other managed_atomic modes require existing; descriptor/request values match exactly where present. |
| FC4R-WIRE-04 | SPEC §§7–8 | Current D7/D8/D9 paths that create a wire13 D3 request choose a mode with an actually encodable inherited optional-member shape; routing to wire13 can never force a forbidden member merely because the /13 schema exists. |
| FC4B-VAL-01 | SPEC §§9.6–9.9 | Current interactive Annotation creation uses D7CreateAnnotationIntent/2 with destinationOwnerRef + AnnotationEditableValue/1 only; after normal owner/create authorization Core injects creator/authoredAt/lastEditor/editedAt, constructs the wire13 create_annotation graph and stores one PortableAnnotationRecord/4. Caller-supplied actor/time/trusted flags or a generic d3_operation impersonating trusted creation fail. |
| FC4B-VAL-02 | SPEC §§9.6–9.7 | A reviewState-only open→resolved mutation changes complete Value/4, allocates exactly one fresh annotationRevisionToken, advances the managed Annotation SourceVersion through the existing SourceRevisionPlan path, preserves creator/authoredAt, and Core rewrites lastEditor/editedAt. A concurrent edit prepared from the old token loses CAS. |
| FC4B-VAL-03 | SPEC §§9.6–9.7 | Changing only appearance or labels is a real Value/4 mutation with the same one-token/one-managed-after rule; it cannot reuse the old token merely because target/body/reply are unchanged. |
| FC4B-VAL-04 | SPEC §9.6 | A proposal whose complete editable Annotation value equals current state is a true no-op: attribution, annotationRevisionToken, SourceVersion and H are unchanged, with no SourceRevisionPlan or fabricated source_change. |
| FC4B-VAL-05 | SPEC §§9.6–9.7 | Current annotation_value payloadBindings, proposed pins, effects and result materialization all bind the exact D3-CJ/3 bytes of complete Value/4 and its SHA-256; PortableAnnotationRecord envelope/ref/token are not part of that value hash, and equal digest cannot substitute for identity/currentness/CAS. |
| FC4B-VAL-06 | SPEC §9.7 | For one existing Annotation changing both target@0 and reply@1, target keeps its full reference evidence while the identity-preserving reply change has exactly one S/annotation_reply_change and no duplicate reply reference result; target/reply toSource addresses, receipt source version and toAnnotationRevisionToken all use the same one final Annotation revision. |
| FC4B-VAL-07 | SPEC §9.7 | Restoring trashed Annotation A while changing reply P→Q uses non_live_source prestate for target@0 and nonnull reply@1; if old reply was null only reply uses absent. Typed target/reply evidence cannot be replaced with lifecycle-only, and there is no separate per-slot revision increment. |
| FC4B-VAL-08 | SPEC §§9.6,16–17 | Corrupt/unknown current Portable Annotation JSON never yields a partially trusted Value/4 projection. Authorized repair/backup may expose exact raw portable bytes; genuine historical D2 Annotation-v2/Value3 records recover only through their recorded decoder rather than being guessed/migrated into Value/4. |
| FC4B-D8-01 | SPEC §9.8 | Current d8_edit_prepare annotation intent is D8EditIntent/3 with exact expectedAnnotationRevisionToken, AnnotationEditableValue/1 and targetPolicy; PreparedEditBinding/3.intent is that closed type, and proposedInputs pins the complete Core-constructed D3-CJ/3(Value/4), not the editable subset. |
| FC4B-D8-02 | SPEC §§9.6,9.8 | D8 Annotation prepare rejects caller actor/time/authentication/trusted-origin fields. Core constructs attribution after current authorization/CAS and freezes it in the same immutable plan; replay does not resample it. |
| FC4B-D8-03 | SPEC §§9.3,9.8 | An Annotation inline Draft that is invalid under the single R6 AnnotationInlineProfile retains exact source plus diagnostics; visual rendering and prepare are unavailable, and the product neither falls back to plain_text nor invokes a second parser. |
| FC4B-D8-04 | SPEC §9.8 | A principal with annotation_read but without annotation_write may receive the authorized current Value/4 as a read-only Annotation Draft/projection but cannot prepare a write merely because rendering or target resolution succeeded. |
| FC4B-D8-05 | SPEC §§9.8,10 | Pure synchronization or newly successful stable-locator requalification that leaves Value/4 bytes unchanged does not advance annotationRevisionToken, SourceVersion or lastEditor/editedAt and cannot revive an old PreparedEditBinding/PAB/ActionEvidence. |
| FC4B-SUG-01 | SPEC §§9.9,15 | Current apply_suggestion on kind=replace fresh-reads the pending+confirmed Suggestion/3 and exact target, verifies expectedText, maps to the one SourceTransform replace using stored replacementSource, and commits target after-image plus Annotation accepted/not_applicable state atomically in one DecisionKey/planning CAS/P seal. |
| FC4B-SUG-02 | SPEC §§9.9,15 | Current apply_suggestion on kind=delete fresh-verifies expectedText and maps to one SourceTransform replacement with zero replacement bytes; accepted state cannot commit without that target deletion, nor vice versa. |
| FC4B-SUG-03 | SPEC §§9.9,15 | Current apply_suggestion on kind=insert freshly validates the stored zero-width point/basis and pointAffinity and maps to one SourceTransform insert using stored replacementSource; caller targetLocator/patch bytes cannot override the stored suggestion. |
| FC4B-SUG-04 | SPEC §§9.9,15 | reject_suggestion succeeds with Annotation state disclosure plus annotation_read/write and exact current Annotation token even when the target source is hidden/unreadable; it performs no target read/write and changes only Value/4 to rejected/not_applicable with normal revision/actor-time rules. |
| FC4B-SUG-05 | SPEC §§9.6,9.9,15 | Accept versus reject, body edit, reviewState edit, labels/appearance edit, reply edit or another suggestion edit all contend on the same Annotation revision token; only one old-token preparation can win and losers must fresh-read/reprepare. |
| FC4B-SUG-06 | SPEC §§9.8–10,15 | mapped/candidate geometry alone never authorizes suggestion acceptance or editing. Manual reattach selects one exact same-owner target through a fresh Value/4 mutation, moves pending to needs_reconfirmation, then reconfirmation/fresh prepare recomputes basis/expected bytes; an old PAB/PreparedIntent is never revived. |
| FC4B-LIFE-01 | SPEC §§9.7,16 | A fresh/mapped fresh Annotation's initial nonnull reply is represented by the inherited reference plan/result slot, never structural S. Only an admitted destination-owner existing Annotation may use the inherited existing-reply S path, with target@0 evidence retained and no reply double-recording. |
| FC4B-LIFE-02 | SPEC §§9.7,16 | Node/copy_annotation copy allocates fresh AnnotationRefs, rewrites target and complete same-owner reply graph through the real identityMap/candidate map, materializes one final Value/4/revision per result, and never copies old locators or guesses positions from text/equal hashes. |
| FC4B-LIFE-03 | SPEC §§9.6–9.7,16 | Ordinary import creates only fresh Annotation identity under the inherited owner rules; imported attribution is explicitly imported_unverified, and a fresh imported Annotation cannot be used as an extra existing-Annotation same-owner structural reply mutation when the D3 mode matrix forbids it. |
| FC4B-LIFE-04 | SPEC §§9.6–9.7,16 | Independent Annotation Trash/restore with byte-identical Value/4 changes lifecycle only and does not mint an Annotation revision. Restore plus a real Value/reply change follows the typed preimage/S/non_live_source rules and uses one final revision. Permanent purge follows inherited D3 tombstone/no-reuse and creates no replacement Value/token or reusable AnnotationRef. |
| FC4B-LIFE-05 | SPEC §§10,16 | Portable backup/export preserves Annotation identity/value and exact resource-region/source-origin facts only under their disclosure rules; it grants no current permission, SourceObservation, ActionEvidence or execution authority, and target SourceOrigin/Annotation identity/reply structure remain distinct. |
| FC4B-ALIAS-01 | SPEC §9.3; SCHEMAS §7 | AnnotationInlineBody/1 is the sole current canonical type. AsciiDocInlineBody/1 is an alias with canonicalOf=AnnotationInlineBody/1 and replaces=null; a registry consumer must not infer a successor migration or second wire/version. |
| FC4A-PROD-01 | SPEC §§3.4,8 | A current managed Document containing an authored level-6 section strict-decodes as D2DocumentSnapshot/3 with D2Heading/3 authoredLevel=6 and the native effectiveLevel; D8 read, D7 headings scan, and D9 document rendering consume that same occurrence rather than rejecting it through historical document_snapshot wire2. |
| FC4A-PROD-02 | SPEC §§3.4,8 | A legal fixed-2.0.26 open/example/sidebar/admonition/list/table/pass/STEM/native-inline combination remains parseable/readable and exact-Source savable even when the rich editor lacks a structural control; no product consumer may drop an unhandled legal D2ProductBlock/3 or D2ProductInline/3 arm. |
| FC4A-PROD-03 | SPEC §3.4 | Authorized invalid AsciiDoc returns exact source plus ordered D2 diagnostics with product projection unavailable and commit eligibility reject; it never returns a partial semantic tree, and a physical source-envelope failure remains the original source_unavailable class. |
| FC4A-PROD-04 | SPEC §8.1 | D7 headings scan over D2DocumentSnapshot/3 emits the existing owner/title/level object using D2Heading/3.effectiveLevel and retains the exact current locator internally; an invalid projection fails the applicable complete scan instead of silently skipping the heading. |
| FC4A-PROD-05 | SPEC §8.1 | D7 body_text recursively consumes the complete D2DocumentBody/3 family, including lists/tables/literal-source payload and explicit inline labels; it is never exact source, never writable, and a legal arm with no defined text mapping makes that adapter unavailable instead of shrinking D2 syntax. |
| FC4A-PROD-06 | SPEC §§1.4,3.4 | A native link/xref/image/citation is parsed before D2IdentityAdapter/1 attaches stable identity; a managed include retains its own source owner/ranges, so path/title/hash cannot infer identity and inclusion cannot grant the root Document write authority over the included source. |
| FC4A-D8-01 | SPEC §8.2 | Current outer D8 wireVersion2 d8_document returns D2DocumentSnapshot/3 at the same SourceObservation and current valid Draft projection preserves every legal /3 body arm; a genuine saved old D2 wire2 snapshot remains historical recovery only. |
| FC4A-RUN-01 | SPEC §8.2 | Every active Workspace has exactly one D8WorkspacePresentationPolicy/1 starting revision1/separate; policy_admin + expected revision is required to change it, and a successful change checked-increments the protected policy without changing Document source or SourceVersion. |
| FC4A-RUN-02 | SPEC §8.2 | Explicit run-in/separate roles override the Workspace policy while Enable/Disable/Use Default mutate only source roles; Use Default removes both and resumes the current policy, and none of the three commands writes D8WorkspacePresentationPolicy/1. |
| FC4A-RUN-03 | SPEC §8.2 | Changing only presentation-policy revision invalidates a D8 render-cache binding while preserving heading/body identities, source bytes, SourceVersion and authored roles; unavailable policy never silently falls back to an ambient host default. |
| FC4A-RUN-04 | SPEC §8.3 | A D9 plan prepared under presentation-policy revision P keeps exact binding P and identical staged bytes if the Workspace later changes to P+1; a fresh plan consumes P+1. |
| FC4A-EXP-01 | SPEC §8.3 | Authorized target asciidoc_source exports the exact selected Document bytes with routeBinding/templateBinding/documentRenderBinding null; unavailable HTML/PDF/DOCX providers do not make the source invalid or block this exact-source path. |
| FC4A-EXP-02 | SPEC §8.3 | HTML document export requires the exact D2DocumentSnapshot/3, format qualification, presentation-policy binding, accepted route/profile and staged bytes; deep heading/run-in presentation is derived from that frozen product projection rather than a second parser. |
| FC4A-EXP-03 | SPEC §8.3 | DOCX/ODT export preserves explicit effective Heading1–Heading9 when the selected accepted profile supports them; deeper/unsupported target structure produces an explicit ExportLossReport item or target unavailability, never D2 source rejection. |
| FC4A-EXP-04 | SPEC §8.3 | PDF export with an unavailable route/provider returns export unavailable while the same current D2 snapshot remains valid and exact-source export remains eligible; an accepted route freezes its target-specific layout/font/accessibility losses. |
| FC4A-EXP-05 | SPEC §8.3 | Two implementations encoding the same ExportPlan/3 must agree byte-for-byte on inputDomain/catalog/selection/projection, document render binding, template, route steps and profile versions, styles, generation policy, target, destination, proof/evidence pins, loss report and staged outputs; a free-form or omitted replacement for any named member fails strict decode. |
| FC4A-EXP-06 | SPEC §8.3 | D9ExportConfirmation/1 exactly covers the retained requires_choice/blocking loss set and cannot rewrite route/target/destination/staged bytes; inspect/confirm/publish/delivery recheck original authorization, and external publish/unknown recovery retains the original create-only and saved-intent rules. |
| FC4A-ROUTE-01 | SPEC §8.4 | A new D7 Definition Transfer current submission uses D3IdentityOperationRequest/13 while complete definitionTransfers/Result9 semantics are unchanged; a genuine saved/planned wire12 transfer continues with its original decoder, fingerprint, effects and recovery. |
| FC4A-ROUTE-02 | SPEC §8.4 | Current D9 Import IR preparation binds new author submission through D3IdentityOperationRequest/13 while ImportIR/Mapping/ConversionInput/file-safety/loss rules remain unchanged; historical wire12 import jobs are not migrated in place. |
| FC4A-ROUTE-03 | SPEC §8.4 | The former D9 S12 premise “D2 has only five levels” is not current: WeftextManaged authored H6–H9 pass through D2/D7/D8/D9, and a target-specific depth limit is reported as loss/degradation or provider unavailability rather than mandatory source loss/rejection. |
| FC4A-IMPACT-01 | SPEC §§3.4,8.4 | The routed D2 implementation-impact companion no longer makes open/native AsciiDoc, include, passthrough or fixed-baseline extensions categorically unsupported; current implementation must parse/read/Source-save every legal fixed-baseline construct while retaining the old single-authority, exact-source, authorization and invalid-repair safeguards. |
| FC34-FMT-01 | SPEC §§5–8 | A fresh managed Document installs source plus ManagedDocumentFormatBinding/1 in the same P, with bindingRevision=1 and the exact pinned Ruby baseline + weftext_managed/1 profile. |
| FC34-FMT-02 | SPEC §§5–8 | The old PortableComponentKey/1 decoder rejects {kind:"document_format"}; CP3 cannot treat it as a legal component. |
| FC34-FMT-03 | SPEC §§5–8 | Legacy/unbound source remains available for authorized raw read/repair; missing format binding does not delete or hide the original bytes. |
| FC34-FMT-04 | SPEC §§5–8 | Legacy/unbound source containing [weftext-attributes] does not automatically become WeftextManaged. |
| FC34-FMT-05 | SPEC §§5–8 | A future managed weftext_managed/1 -> /2 profile-only migration keeps source unchanged; the same Notice3/CP4 component set carries exactly the document_format transition with sourceChanges=[], creates no SourceRevisionPlan/SourceVersion/H advance, and absent/Baseline-to-managed is not mislabeled as profile migration. |
| FC34-FMT-06 | SPEC §§5–8 | A managed profile-only migration does not create a SourceVersion or advance H(D,E). |
| FC34-FMT-07 | SPEC §§5–8 | If SourceVersion is unchanged but the format stamp changes, an old InputDescriptor/PAB/EditBinding/ExportPlan cannot continue. |
| FC34-FMT-08 | SPEC §§5–8 | After a format change, dirty Draft bytes remain while projection/map/preview are invalidated and requalified. |
| FC34-FMT-09 | SPEC §§5–8 | A format change must not delete a dirty Draft. |
| FC34-FMT-10 | SPEC §§5–8 | Exact-source export consumes only its actual source path and is not forced to obtain an unrelated complete Query merely because no semantic parse is needed. |
| FC34-FMT-11 | SPEC §§5–8 | Rendered export lists the document_format key; a format change resets an unpublished ExportPlan3. |
| FC34-FMT-12 | SPEC §§5–8 | D7 narrow-field qualification may not add full source_read merely to consume the profile. |
| FC34-FMT-13 | SPEC §§5–8 | A narrow field uses its original source_envelope/Field qualification plus the internal format dependency. |
| FC34-FMT-14 | SPEC §§5–8 | Query query_scan cannot substitute for document_format proof. |
| FC34-FMT-15 | SPEC §§5–8 | Trash->restore preserves the exact profile; purge physically removes the format component in the same P. |
| FC34-FMT-16 | SPEC §§5–8 | A missing format row in Derived Index cannot prove that the component is absent. |
| FC34-FMT-17 | SPEC §§5–8 | The current D9 wire1 probe union cannot silently gain an adoc arm without a D9 owner-version change. |
| FC34-ST-01 | SPEC §§11–13 | With representable PortableTransformCompilation/1 and exact transform trust, planning freezes CoreSourceEditPlan/2 with Event3[] and emission=required; the same P creates Evidence/2, the artifact, and exactly one outbox item. |
| FC34-ST-02 | SPEC §§11–13 | When the transform profile is unavailable, TransformEmissionPlan/1 freezes disabled{transform_profile_unavailable}; an otherwise legal ordinary source save can commit and produces no transform outbox item. |
| FC34-ST-03 | SPEC §§11–13 | After source commit, post-hoc before/after diffing and signing a transform artifact is rejected. |
| FC34-ST-04 | SPEC §§11–13 | If a frozen required{profile,expectedTrustRevision,expectedTrustKeyId} no longer matches at seal, that plan cannot downgrade to a transform-less save and follows the original pause/reprepare rules. |
| FC34-ST-05 | SPEC §§11–13 | An insertion exactly at point p yields the distinct canonical left- and right-affinity results. |
| FC34-ST-06 | SPEC §§11–13 | Claiming exact mapping across a true edit/target overlap is rejected. |
| FC34-ST-07 | SPEC §§11–13 | A remote missing an intermediate transform artifact cannot reconstruct mapping from identical final text. |
| FC34-ST-08 | SPEC §§11–13 | A public key authorized only for the revision-token profile cannot validate a transform artifact. |
| FC34-ST-09 | SPEC §§11–13 | Even if the same raw public key is separately root-authorized for both profiles later, verification still requires the exact profile and artifact domain. |
| FC34-ST-10 | SPEC §§11–13 | A CAS loser's staging signature cannot enter the portable outbox. |
| FC34-ST-11 | SPEC §§11–13 | SourceTransform signs exactly ASCII D6-Source-Transform-Seal/1 || NUL || D3-CJ/3(the complete artifact with only signature removed); English and Chinese state the same message, and after seal publication failure republishes exact canonical pinned artifact bytes and never resigns. |
| FC34-ST-12 | SPEC §§11–13 | A SourceTransform artifact is not a PortableComponent and cannot appear in CP4 components. |
| FC34-TR-01 | SPEC §13 | Bootstrap4 keeps D3 proposalId as the canonical lowercase UUID and uses closed Profile4/creator/Registry/series/period helper types; it has one root and Genesis2 has exactly two declarations, rev1 revision-token then rev2 transform, with the correct predecessor chain. |
| FC34-TR-02 | SPEC §13 | Genesis2 with one or three declarations is rejected. |
| FC34-TR-03 | SPEC §13 | Two otherwise correct profile declarations both using revision=1 are rejected. |
| FC34-TR-04 | SPEC §13 | The two bootstrap declarations share one DecisionKey/activation ChangeId and a history cut includes both or neither. |
| FC34-TR-05 | SPEC §13 | Making either DomainSealKeyHandle usable before bootstrap commit is rejected. |
| FC34-TR-06 | SPEC §13 | Planned crash recovery restores the same staged handle associations and does not regenerate keys. |
| FC34-TR-07 | SPEC §13 | When a real Bundle1 Workspace first authorizes transform later, it creates a Bundle2 successor while retaining the exact historical Declaration1 prefix. |
| FC34-TR-08 | SPEC §13 | Changing an old Declaration1 to version2 and claiming the old signature remains valid is rejected. |
| FC34-TR-09 | SPEC §13 | A policy conflict containing CP3+Bundle1 and CP4+Bundle2 heads strict-dispatches every head by proof/bundle version, validates exact address/root/history evidence, folds both profile states, and has a legal current policy_bundle_choice resolution path; decoder fallback or permanent refusal of every legal mixed conflict fails. Current source_merge/choose_source_head keep ownerKind/intentKind d6_conflict_resolution/2 with Input2/Plan1/Preview1, while current policy_bundle_choice uses d6_conflict_resolution/3 with Input3/Plan2/Preview2; all three use the current outer InputDescriptor3/DependencyProof3/PreparedIntent3 family, and any arm/version mismatch fails. |
| FC34-TR-10 | SPEC §13 | A transform compromise from an unselected branch remains in the effective Carry1/Carry2 union at its original activation cut; selecting another branch cannot revive that compromised key, and an eligible affected transform pair may use FreshDomainAuthorizationSpec2 to produce a fresh transform Outcome2/PoP2. A current policy prepare also binds OwnerInputBinding2.canonicalDescriptorBytes to exact D3-CJ/3(Input3); pinRefs is exactly the sorted/unique union of branch ChangeRecord/CP/Bundle pins, selected/result bundle pins, and every retained Carry origin/carrier ChangeRecord/CP/Bundle pin; Preview2 binds the complete Plan2 and branchEvidenceDigest, and PreparedIntent3.pinDirectory also retains DependencyProof evidence plus the exact preview and installation pins. Omitting a losing-branch Carry hop, result pin, or preview pin fails planned/recovery closure. |
| FC34-TR-11 | SPEC §13 | Current d6_replica_register_prepare keeps its wire2 request, mints one ReplicaEpoch plus exactly two staged Handle2 values in revision-token then source-transform order, appends two Declaration2 authorizations under one DecisionKey, and makes both usable together only through one CP4/ChangeRecord P seal. The D6 Storage §9.1 current consumer must accept that same one-ReplicaEpoch/two-staged-Handle2/two-Declaration2 transition, validate both authorizations at receiver admission, and admit both profiles simultaneously rather than partially. |
| FC34-TR-12 | SPEC §13 | Generic dual-profile trust management cannot substitute for replica_register authority or produce a partially admitted replica profile; planned replica recovery restores the exact staged pair/declarations/component pins and never regenerates keys. Historical single-profile registration keeps its recorded decoder/bytes, and current retire blocks future signing for both profiles without becoming execution takeover. |
| FC34-TR-13 | SPEC §13 | D10 PublisherIdentity/package signatures cannot serve as SourceTransform trust. |
| FC34-H-01 | SPEC §17 | An old D3 wire12 saved decision replays its original receipt on a new runtime without requiring InputDescriptor3. |
| FC34-H-02 | SPEC §17 | An old planned PreparedActionBinding3 continues its original recovery and is not upgraded in place to PAB4. |
| FC34-H-03 | SPEC §17 | An old CP3 carrying a document_format component must be rejected by its historical decoder. |
| FC34-H-04 | SPEC §17 | Relabeling CP4 as CP3 for an old consumer is rejected. |
| FC34-H-05 | SPEC §17 | Old D7 effect transport continues decoding historical Plan1/Plan3; current Plan4 uses only EffectManifest3/EffectBytes3. |
| FC34-H-06 | SPEC §17 | Missing SourceTransform evidence cannot revive an old Draft selector/PAB/ActionEvidence. |
| FC34-H-07 | SPEC §17 | When current format or transform evidence becomes available again, only a new current qualification may be created; an old saved record is not modified. |
| FC34-FMT-18 | SPEC §§5–8 | BaselineOnly ordinary analysis can succeed with no portable component; no ManagedDocumentFormatBinding(profile=baseline) is created. |
| FC34-FMT-19 | SPEC §§5–8 | DependencyKey/3 rank is source, document_format, lifecycle, ..., execution_resource; appending document_format at the end fails. |
| FC34-FMT-20 | SPEC §§5–8 | PortableComponentKey/2 rank is document, document_format, resource, ..., conflict; placing document_format after conflict fails. |
| FC34-ST-13 | SPEC §§11–13 | A current plan/evidence using SourceTransformEdit/1 or SourceTransformPortableEdit/2 is rejected. |
| FC34-ST-14 | SPEC §§11–13 | A required plan with unequal D3-CJ/3(plan.edits) and D3-CJ/3(evidence.edits) cannot seal. |
| FC34-ST-15 | SPEC §§11–13 | Event3 first validates UTF-8 boundaries, replace/insert ranges, removedByteLength=end-start and removedSha256 against the exact before slice; SourceTransformEvidence2.beforeSourceSha256 must independently equal SHA-256 of the complete exact before source selected by evidence.before, and a wrong whole-before hash fails even when all Event slices/replay still match. |
| FC34-ST-16 | SPEC §§11–13 | generatedOutputSpan is derived only by the normative before/after replay cursor and delta(R)=replacementByteLength-(endByte-startByte); receiver validation reacquires the historical exact-before bytes rather than substituting current-file bytes, slice hashes, or equal SourceVersion fields. |
| FC34-ST-17 | SPEC §§11–13 | replace [5,10)->X + insert@10->Y retains two events and canonical boundary order to yield X\|Y; merging into one replacement that yields XY\| fails. |
| FC34-ST-18 | SPEC §§11–13 | A required seal rechecks profile, expectedTrustRevision, expectedTrustKeyId, and a usable handle; checking only that “some transform key exists” fails. |
| FC34-ST-19 | SPEC §§11–13 | If a cross-generated-anchor edit cannot preserve boundary slots, compilation returns unavailable; ordinary save may continue but no empty-event CoreSourceEditPlan or artifact is fabricated. |
| FC34B-D10-01 | SPEC §14 | A current unseen Workspace-control prepare uses D10WorkspaceReadDependencies/2 + DependencyProof/3 and includes document_format when parsing a managed Document; genuine saved/planned historical records recover through their recorded owner versions first. |
| FC34B-D10-02 | SPEC §14 | Using D10WorkspaceReadDependencies/1 + proof2 to establish the same current unseen prepare fails the current owner gate; this does not reinterpret a genuine historical saved/planned dependency record. |
| FC34B-D10-03 | SPEC §14 | For a current unseen control operation, D10WorkspaceReadDependencies/2 -> ControlDependencies/3 -> D10ControlInput/2 -> ControlPrepareBinding/3 matches the D6 Descriptor3/Proof3 inputs member-for-member at one cut; current external consent keeps the original ExternalConfirmationRequirement/Record semantics inside Binding3, while a proven saved Binding2 remains exact historical recovery. |
| FC34B-D10-04 | SPEC §14 | A deployment-only body cannot smuggle workspaceReads=some into ControlDependencies3. |
| FC34B-D10-05 | SPEC §14 | For recurrence source V unchanged but document_format M1->M2, the current Storage producer records ScheduleContinuityInvalidation/2 with binding_changed; it cannot advance Witness2 as continuous merely because recurrence output compares equal. |
| FC34B-D10-06 | SPEC §14 | If source/profile bytes are equal but document_format proof continuity has a gap, the current producer records ScheduleContinuityInvalidation/2 with gap and retains the last valid checkpoint; it never fabricates Evidence2 or a continuous Step2. |
| FC34B-D10-07 | SPEC §14 | With no format/rule discontinuity and a complete current ChangeRecord1 + Notice3 + CP4 chain proving a genuinely unrelated body edit, ScheduleContinuityStep2 may atomically advance Witness2; the retained step pin is artifact/recovery over UTF8 D6-Schedule-Step/2 + NUL + canonical complete Step2 bytes, and the resulting witness pin uses D6-Schedule-Continuity/2 with byteLength/SHA-256 covering the complete prefixed bytes. |
| FC34B-D10-08 | SPEC §14 | A current ScheduleContinuityStep2 attempting to decode a new current portable transition as Notice2/CP3 fails. Schedule continuity decoder dispatch is exact: a Step2 or Witness2 under a /1 artifact domain fails, a historical Step1/Witness1 under a /2 domain fails, and an unknown/mismatched domain has no fallback decoder; retained historical transitions continue only through their actual /1 bytes and decoder. |
| FC34B-D10-09 | SPEC §14 | An old Subscription1 remains a valid historical retention owner and can explicitly continue to a same-generation Subscription2 only with complete retained history proving no intervening format/rule/business discontinuity and establishing the current Evidence2/Proof3 cut; its existing Witness1/Step1/Invalidation1 pins retain exact /1 bytes while only newly produced Witness2/Step2/Invalidation2 use /2 domains, so a real bridge may retain a version-mixed exact typed chain without repinning. |
| FC34B-D10-10 | SPEC §14 | An old Subscription1 whose retained history contains a format/rule/business discontinuity cannot continue the generation; explicit replace is required, and background migration or resetting an invalid generation fails. |
| FC34B-D10-11 | SPEC §14 | A fresh automatic author preparation atomically saves D10AuthorPreparationLink2 + the exact PAB4 + required Effect3 evidence/pins before return, then constructs ApprovalUse2 from the complete current preparation; a proven saved/planned Link1/PAB3/Effect2/ApprovalUse1 responsibility retains its original decoder, bytes, pins, request and OperationId. |
| FC34B-D10-12 | SPEC §14 | A fresh interactive author responsibility must declare the exact current PAB4 or EditBinding3 selected by its owner contract; a mismatched current type fails, while a genuine saved/planned PAB3/EditBinding2 remains historical recovery rather than being upgraded. |
| FC34B-TR-01 | SPEC §13 | Declaration1 keeps its original CP3 activation rule. |
| FC34B-TR-02 | SPEC §13 | Declaration2 activates through exact CP4 and the Bundle2 policy after-image that first appends the sequence. |
| FC34B-TR-03 | SPEC §13 | A Declaration2 with only same-DecisionKey CP3 and no CP4 is invalid. |
| FC34B-TR-04 | SPEC §13 | A mixed /1,/2 trust history folds /1 activation through CP3 and /2 activation through CP4. |
| FC34B-TR-05 | SPEC §13 | Carry2 factId uses D6-Trust-Compromise-Fact/2 over the exact nine-field body; direct revoke compromise maps trustKeyId, direct rotate compromise maps oldTrustKeyId, originDeclarationDigest hashes the complete Declaration2, and originActivationChangeId rederives through the original CP4/ChangeRecord rather than resolver time. TrustConflictCarryValidationEvidence additionally pins the original Declaration2 activation ChangeRecord1/CP4/Bundle2 plus every traversed resolver carrier; resolver time cannot replace the origin cut. |
| FC34B-TR-06 | SPEC §13 | Current wireVersion3 transform-profile rotate uses mode=ordinary, the exact PoP/rotate/root signature domains and bodies, and a usable root-authorized producer; after K1 signs a transform and rotates normally to K2, a receiver still validates K1 at the producing CP4.frontierBefore. |
| FC34B-TR-07 | SPEC §13 | For transform KT1 branching to ordinary KT2 and compromise KT3, choosing the ordinary branch still retains the losing branch’s original KT1 compromise fact in the canonical recursive union plus its exact per-head origin/carrier validation evidence; same factId with different bytes is integrity_conflict, while a safe selected key or eligible fresh transform recovery follows the complete Outcome2 rules. Planned recovery restores the frozen Input3/Plan2/Preview2/pins rather than recomputing from current history. |
| FC34B-TR-08 | SPEC §13 | A normal transform receiver using current Bundle state instead of producing CP4.frontierBefore fails. |
| FC34B-CONT-01 | SPEC §13 | In Bundle2, revision current K still authorized by Declaration1 with usable Handle1 may sign new revision tokens, while transform signing with that handle fails. |
| FC34B-CONT-02 | SPEC §13 | Current unseen add/rotate/revoke uses the closed wireVersion3 dual-profile prepare family with no caller key material; after a Declaration2 ordinary rotation of the revision profile to K2, only Handle2(K2) can perform new signing, while saved/planned wireVersion2 records keep their original recovery. |
| FC34B-CONT-03 | SPEC §13 | Old states revision=current, transform=none produce revoke revision + authorize B2 revision + authorize B2 transform in one CP4. |
| FC34B-CONT-04 | SPEC §13 | Old states revision=none, transform=current produce revoke transform + authorize both B2 profiles in one CP4. |
| FC34B-CONT-05 | SPEC §13 | If either profile is conflicted/gapped/unproved, the clean profile's B2 handle cannot become usable early. |
| FC34B-CONT-06 | SPEC §13 | Continuation fails if declarations are not in fixed profile-rank order or expose an observable intermediate prefix. |
| FC34B-HOLDER-01 | SPEC §§7–8 | Wire13 conflict resolution stores D3ResolutionInputUse/2 + InputDescriptor3. |
| FC34B-HOLDER-02 | SPEC §§7–8 | Wire13 retaining an InputDescriptor2 guard fails. |
| FC34B-HOLDER-03 | SPEC §§7–8 | For a D4 strong relation operation, unchanged source with changed format stamp invalidates the old proof and requires stale/reprepare. |
| FC34B-HOLDER-04 | SPEC §§7–8 | D5 native table edit also consumes source+format; a format change stales the old preparation even when table locator/SourceVersion happen to match. |
| FC34B-HOLDER-05 | SPEC §§7–8 | Mechanical version bumps of D4/D5 inner RelationReadContext, table locator, occurrenceKey, or numeric sourceRevision merely to match /3 are rejected. |
| FC34B-CR-01 | SPEC §6.3 | A current portable decision freezes ChangeRecord1, Notice3, and CP4 in the same P; CP4 component keys equal Notice3 exactly in canonical order, each after is the actual installed/sealed owner-versioned image, and DecisionKey/ChangeId/frontiers/digests match. |
| FC34B-CR-02 | SPEC §6.3 | ChangeRecord1 pointing to the correct CP4 but the wrong Notice3 digest fails. |
| FC34B-CR-03 | SPEC §6.3 | A mismatch between CP4 frontierBefore/After and ChangeRecord fails; frontierAfter is exactly frontierBefore plus this ChangeId with no other-domain regression, and scope_dependencies requires the complete continuous ChangeRecord/completion chain plus retained unrelatedness proof. |
| FC34B-CR-04 | SPEC §6.3 | Pre-FC bytes without a proven decoder cannot be interpreted as ChangeRecord1. |
| FC34B-CR-05 | SPEC §6.3 | Receiver admission requires exact Notice3/CP4 bytes, every actual component byte and owner version, complete sourceChanges and production SourceVersions, restored-arm exclusions, and the continuous chain; an unknown historical decoder or missing proof stops the strong path with gap/proof_unavailable while ordinary source read/save retains its independent qualification. |
| FC34D-MIX-01 | SPEC §14 | ControlPrepareBinding/1(K1) and ControlPrepareBinding/3(K2), K1!=K2, may coexist in one Claims2/Inventory2/Record3; each uses its exact decoder, sorts by StableControlKey, and retains its own bytes/pins. |
| FC34D-MIX-02 | SPEC §14 | ControlPrepareBinding/1(K) plus ControlPrepareBinding/3(K) in one Claims2 is a duplicate StableControlKey responsibility and fails even when canonicalIntentBytes/originalCommitRequest match; no LWW, new-version preference, or silent drop is allowed. |
| FC34C-MIX-01 | SPEC §14 | One ControlDependencies3 may contain an old stop Pin1 and a new automation Pin2 when each binding is complete at the same cut. |
| FC34C-MIX-02 | SPEC §14 | An external_effect current record may remain Image1/Pin1 and remain fully reachable from current Dependencies3/Inventory2. |
| FC34C-MIX-03 | SPEC §14 | Encoding stop/external_effect/reservation as Image2 fails closed decode. |
| FC34C-MIX-04 | SPEC §14 | One EffectPlan2 can express Automation before=Image1+Subscription1 and after=Image2+Subscription2 without reencoding the before image; when the scheduling owner qualifies continue, the old subscription may retain the same generation rather than being forced to replace. |
| FC34C-MIX-05 | SPEC §14 | Downgrading an Image2 automation/run back to Image1 fails. |
| FC34C-MIX-06 | SPEC §14 | Pin1 authenticates the exact D10-Control-Record/1 prefixed Image1 payload and Pin2 the exact /2 prefixed Image2 payload with matching artifact byteLength/SHA-256; a schema tag/pin-domain mismatch or repinning Image1 as Pin2 fails. |
| FC34C-MIX-07 | SPEC §14 | Mixed ranges use rank records=0,cost_lineage=1,occurrences=2 and one cross-version logical range identity; a Range2 occurrence array may contain distinct old K1/new K2 records ordered by AutomationOccurrenceKey. |
| FC34C-MIX-08 | SPEC §14 | Occurrence1(K) plus Occurrence2(K) with the same key fails. |
| FC34C-MIX-09 | SPEC §14 | For record pins, one same-cut identity (binding.ref,binding.revision,usageRevision) may occur once across versions: a byte-equal duplicate is rejected and different image bytes are integrity_conflict. |
| FC34C-MIX-10 | SPEC §14 | ControlDependencies3 whose range/barrier comes from cut B while record pin, binding, usage revision, stop pin, or Workspace evidence comes from cut A fails same-cut qualification; all are captured at one real Authority Store barrier. |
| FC34C-EXEC-01 | SPEC §14 | Claims2 may simultaneously contain Binding1/2/3, Step1/2, Subscription1/2, Occurrence1/2, and Pin1/2 when their semantic keys differ; prepareBindings are globally unique by inner StableControlKey across all three binding versions. |
| FC34C-EXEC-02 | SPEC §14 | MoneyResponsibility2 may require old Pin1/new Pin2 and Range1/Range2 together; reservations are unique by complete reservation Binding, layers by CostLayerKey, evidence pins by pinToken, and mixed pins/ranges use their canonical cross-version orders. |
| FC34C-EXEC-03 | SPEC §14 | Inventory2 completely carries mixed ApprovalUse, Claims2, Money2, ExternalResponsibility1 and StopResponsibility1, orders each by its cross-version logical identity, is captured at one real barrier, and carries the actual authoritative StopCapacity1 from that same safety store. |
| FC34C-EXEC-04 | SPEC §14 | ApprovalUse1 and ApprovalUse2 with the same DecisionKey are duplicate responsibility and fail; schema version does not split the logical identity. |
| FC34C-EXEC-05 | SPEC §14 | Old Subscription1 and new Subscription2 for the same Automation may coexist when generations differ; the same Automation+generation fails, while a legal configure continue may keep the old generation. |
| FC34C-EXEC-06 | SPEC §14 | Inventory2 that omits a started/outcome_unknown external responsibility, an already stopped latch, a still-referenced completed attempt, an old recovery pin, or the actual StopCapacity fails completeness. |
| FC34C-D6-01 | SPEC §14 | Record3 Proof2 inventoryPin strict-decodes exact D6-Execution-Inventory/2; workspaceRef and the five payloads approvalUses/claims/moneyLineage/externalUnknowns/stopState are byte-equal to Inventory2, Inventory2.storeIncarnation equals Proof2.storeIncarnation, and Inventory2.stopCapacity equals the actual same-barrier safety-store value. |
| FC34C-D6-02 | SPEC §14 | Execution custody may hand off Record2 to Record3 only from one complete mixed Inventory2/barrier with actual old-holder fencing; all old responsibility is retained, revision is old+1, executionDomainId and store continuity are preserved, and shared StopCapacity is not copied into a second active store. |
| FC34C-D6-03 | SPEC §14 | Without handoff, checkpoint, or a real responsibility mutation, merely supporting the current FC schemas does not background-migrate Record2/Proof1/Inventory1 or repin their historical bytes into Record3/Proof2/Inventory2. |
| FC34C-D6-04 | SPEC §14 | If handoff cannot obtain an original decoder/bytes/pins for any old Pin1/PAB3/Binding1/Subscription1/stop/external responsibility or cannot prove the authoritative store/capacity barrier, takeover pauses/unavailable instead of constructing a partial Inventory2; unrelated ordinary source operations remain available. |

## PR4 schema repair coverage (no new acceptance IDs)

The earlier dangling-schema repair added, removed, and renumbered none of the then-554 obligations. This environment repair adds only AD2-41, bringing the cumulative inventory to 555; M39-04 is the accepted v3.10 correction of that existing ID rather than a new scenario. All other existing row obligations remain cumulative. Direct coverage is: Stage4A adds only FC4A-PROD-01..06, FC4A-D8-01, FC4A-RUN-01..04, FC4A-EXP-01..06, FC4A-ROUTE-01..03, and FC4A-IMPACT-01, bringing the cumulative inventory to 576; no prior ID body is replaced. Stage4B adds only FC4B-VAL-01..08, FC4B-D8-01..05, FC4B-SUG-01..06, FC4B-LIFE-01..05, and FC4B-ALIAS-01, bringing the cumulative inventory to 601; every prior row body remains byte-for-byte unchanged. The Stage4A-residual repair adds only FC4R-PROD-01..10, FC4R-ORIGIN-01..06, FC4R-LEVEL-01..03, FC4R-RUN-01..09, FC4R-EXP-01..09, FC4R-TABLE-01..08, and FC4R-WIRE-01..04, bringing the cumulative inventory to 650; it creates no new finding ID and replaces no prior row body.

- retained Witness/observer/document/block/collection/catalog/diagnostic/string/call/operation/inline/content evidence: AD2-37, AD2-38, O34-01–O34-16, P35-01–P35-14, N36-04–N36-09, M37-01–M37-20, M38-01–M38-16, M39-01–M39-11;
- M37 producer sites, snapshot/cut, namespace, and producer-time carrier rules: M37-01, M37-17–M37-20, M38-01–M38-16, M39-01–M39-11;
- AnnotationInlineBody/1 / AsciiDocInlineBody/1 schema alias and the single AnnotationInlineProfile/1: AN2-08, still under the AD2 complete-2.0.26 language gate.

These remain unexecuted design requirements. Closing the schema references does not turn them into passed implementation tests.

The second bounded D10 repair adds no acceptance IDs; the existing FC34B/C/D rows above now also cover fresh/current owner routing, Binding1/2/3 historical dispatch, mixed canonical ordering, the one-barrier rule, Record3↔Inventory2 five-payload equality, Proof2.storeIncarnation, and actual StopCapacity.

## Not executed

This PR did not execute these 650 scenarios or product Ruby/Rust parsers, providers, SQLite/storage code, replica/crypto/crash paths, Automation scheduling, or execution-custody handoff. Author-side checks are limited to ID/count/JSON/router/reference consistency.