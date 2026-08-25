---
source_language: zh-CN
translation_of: 10-links-and-potential-mentions.zh-CN.md
translation_status: synced
---
[简体中文](10-links-and-potential-mentions.zh-CN.md)

# Links and potential mentions

This specification defines candidate discovery and deterministic ordering. Canonical managed-node link syntax is the UUID `node:` macro in [`15-weftext-asciidoc-profile.md`](15-weftext-asciidoc-profile.md); potential-mention matching and bulk-link actions are available only when they preserve the contracts below.

## Terms and authority

An outgoing link is an explicit link already present in a node document. A potential mention is unlinked human-authored text that can refer to another node by its canonical name or a user-authored alias.

Managed-node identity links use `node:<uuid>[<authored label>]`. The UUID is resolution authority and the optional label is authored display text. A link produced by checklist-to-task-node promotion follows the same rule; it is not a task-state mirror or a second task occurrence.

The canonical name is derived from the node directory basename. Alternative names come from `weftext.aliases` in the operational envelope. Aliases affect Weftext lookup but are not identity; changing them never changes `weftext.id`.

Outgoing-link, backlink, alias, and potential-mention indexes are derived. They can be discarded and rebuilt from canonical AsciiDoc and node identity. Discovery never inserts or rewrites a link by itself.

Only Core-classified managed-node AsciiDoc participates. Visible unmanaged files, including Markdown and loose AsciiDoc, are neither node-link sources nor targets and are never rewritten during a structural transaction; ignored content is not inspected or disclosed.

## Outgoing-link order

Explicit outgoing-link occurrences are retained in source order. A UI may group repeated targets and show a count, but it retains the occurrence positions and uses the first occurrence as the group's default position. Moving or renaming a node cannot reorder links merely because its path changed.

## Potential-mention ordering

Candidates use a deterministic tuple in this order:

1. matched text length, descending, measured on the normalized Unicode scalar sequence rather than UTF-8 bytes;
2. match quality: exact canonical name, exact alias, normalized canonical name, then normalized alias;
3. source start position, ascending;
4. target canonical name in natural order;
5. target node ID as the final stable tie-breaker.

Length has priority over match kind. If the same source text contains overlapping `ABC` and `AB` candidates, `ABC` is listed first even when `ABC` is an alias and `AB` is another node's canonical name.

## Overlap and ambiguity

All valid overlapping candidates remain available for inspection, but the longest candidate is primary. A shorter candidate fully contained by the primary span is collapsed under other candidates for that occurrence. Creating a link for the primary span removes overlapping candidates at that occurrence from the current revision.

If one matched spelling resolves to several nodes, the UI presents one ambiguous candidate group with every target. It does not pick a target from path, recency, or index iteration order. Automatic or bulk linking skips ambiguity unless the user resolves it in the preview.

## Actions and revisions

Creating, removing, or rewriting links is a Core document action with a base revision, preview, exact source ranges, selected target IDs, and a deterministic patch. The action revalidates the candidate against the current document and alias index before commit. A stale, moved, deleted, newly ambiguous, or no-longer-matching candidate returns a structured conflict instead of editing a nearby string.

Bulk link creation is never an implicit indexing side effect. It shows included occurrences, overlaps, ambiguities, exclusions, target IDs, and the exact AsciiDoc changes before commit.

## Presentation exclusions

The reserved `weftext` envelope is not human document content and never participates in word counts, search snippets, aliases, or potential mentions. Existing explicit-link display text is not reported again as an unlinked mention for the same span.

When a `node:` target carries a valid task-node profile, Weftext may enrich the resolved link with current state, priority, or due-date presentation. Those values are read from the authorized target projection and never written into the referring source as cached authority. An unavailable/hidden/invalid target degrades through ordinary non-disclosing link behavior rather than exposing task diagnostics or stale metadata.

The initial syntax decision must separately define matching boundaries and treatment of code fences, inline code, raw HTML, headings, tables, and very short names before bulk creation is enabled. These remaining choices do not change the accepted longest-match-first rule.
