---
source_language: zh-CN
translation_of: REQUIREMENTS.zh-CN.md
translation_status: synced
---

[简体中文](REQUIREMENTS.zh-CN.md)

# Requirements and technical choices

Status: requirements-first reassessment has started; the technical solution has not been accepted again. This document records the current direction, not a completed requirements inventory or architecture rewrite. Independent review of the prior Typst revision retains its fixed scope and cannot become a reason to retain old technology.

## 1. Design basis

Choose technology as for a new project, prioritizing confirmed user requirements. Sunk cost, existing code, document volume, protocol names and prior review acceptance create no obligation to inherit a mechanism. The user has selected Typst and native-first authoring; this is a current explicit constraint. Prefer official capabilities where native syntax can express the content, without adding another authoring language.

Distinguish what the user wants to accomplish, the reliability/privacy/experience conditions that the result must satisfy, and the proposed implementation. The first two define observable acceptance; the third can be replaced. Simplification must not delete required failure scenarios, or merely rename the old complexity.

## 2. Classification examples

| User outcome or acceptance condition | Replaceable technical means |
| --- | --- |
| Stable ID links retain their target after rename/move; broken path links are reported rather than misdirected; copying creates an independent note | Specific ID encoding, locator structures, allocation stages and receipt fields |
| Same-level standalone and run-in headings share numbering, outline and references | Organization of native show rules, editor adapters and default spacing |
| Simple contact, organization, task and paper properties can be edited and queried | Internal schema identifiers, indexes and projection protocols; do not restore the complex fact model |
| Saving, crash recovery and retries neither lose drafts nor repeat completed effects | Journal formats, commit stages, CAS organization and proof objects |
| Revocation prevents disclosure through caches, search, compilation and export | Authorization versions, dependency tracking and cache invalidation algorithms |
| Local use, synchronization and hosted multi-user non-realtime editing have explicit conflict and permission behavior; realtime collaboration retains its prior deferred boundary | Replica protocols, persistent version encoding, transport objects and service deployment |
| Independent annotations retain targets/history and explicitly report uncertain location | Anchors, source maps, indexes and version comparison |
| Templates produce editable Word documents or spreadsheets with intranet download | Conversion representations, packaging, download implementation and Office mappings |

This table distinguishes outcomes from mechanisms; it is not the complete feature inventory. Record approved features, discussed proposals, research candidates and deferred items separately. Appearing in an old document or competitor survey does not automatically make something a requirement. An existing algorithm may be retained when current requirements justify it, based on suitability and verification.

## 3. History and compatibility

Retain historical inputs, reviews and experiments as traceable design evidence, not runtime dependencies. Old candidate CP, Notice, Proof, Policy, receipt and decoder versions must not enter the new implementation merely because they were documented. Earlier requirements to preserve protocol shapes or exact error ordering undergo technical reassessment rather than automatically constraining the new project.

Define compatibility requirements only for evidenced real data, deployed interfaces or user-confirmed compatibility targets. A repository prototype or unimplemented design does not establish that these exist. Preserve originals if real data is found, and explicitly scope import, conversion or compatibility. Do not delete user data or invent migration layers for undeployed candidates.

Translate the original 21 findings and subsequent findings into complete failure scenarios and user harm. Applicable problems must be solved by the new design, potentially using different mechanisms. Record why a scenario becomes inapplicable when its old mechanism is removed. Neither inherit the old fix mechanically nor transfer every historical CLOSED status to the new design.

## 4. Next deliverables

First establish sourced requirements and acceptance scenarios, then a concise technical solution that satisfies them. Explain necessary complexity, material tradeoffs and risks requiring prototypes, and connect requirements, mechanisms and evidence. Independent review checks omitted requirements, sufficient mechanisms and historical compatibility without a real purpose separately.

Titles, subtitles, same-level mixed heading layout, Word output and paper data remain in this work. Adding them to old protocols is insufficient; they must fit the reselected architecture. Existing technical clauses remain available as a reviewable candidate, without mixing two protocols before concrete replacement. A new global final review follows requirements and technical review; implementation and product acceptance are recorded separately.
