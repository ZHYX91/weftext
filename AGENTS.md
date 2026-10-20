---
source_language: zh-CN
translation_of: AGENTS.zh-CN.md
translation_status: synced
---

[简体中文](AGENTS.zh-CN.md)

# Repository collaboration rules

1. Read README, current requirements and relevant technical owners first. Requirements define outcomes; technology may be reselected.
2. The product is in development, with no backward-compatibility obligation by default. Prioritize sound design and long-term maintainability: refactor existing designs and implementations as needed, including direct replacement, removal and breaking changes. Do not retain compatibility layers, parallel implementations or obsolete branches because of old code, interfaces, protocols, formats, past reviews or sunk cost. Within the authorized task scope, complete changes to affected implementations, consumers, tests and both documentation languages; do not create technical debt by deferring refactoring behind compatibility work. Preserve compatibility only for objects and scope explicitly required by the user. When old data must be carried forward, preserve originals and use explicit import or one-time conversion rather than retaining the old implementation indefinitely.
3. Maintain one native Typst design. Prefer official syntax without parallel authoring languages, competing data authorities or migrations unsupported by real data. Current Core contracts own persistence, authorization and failure guarantees.
4. Public human-facing documents have a Chinese source and synchronized English. Schemas, code, machine fixtures and original evidence are not localized. Private conversations, working logs, real user data and secrets stay outside the public repository.
5. Algorithm, persistent-format and authorization changes update affected consumers and meaningful positive/negative cases. Document organization preserves semantics; counts and green CI do not replace semantic review.
6. Run `python scripts/check.py`; add `--models` for changes to schemas, models or their contracts. For Rust/frontend implementation changes, run the product repository's existing formatting, static-analysis and test entry points according to their actual impact. Do not depend on private workspace scripts or use documentation checks as implementation acceptance.
7. Distinguish design acceptance, bounded experiments, product acceptance and release. Probes are not production implementations; never claim unrun platforms or cases passed.
8. Preserve real originals before migration. Establish ownership and inactivity before cleanup; bulk cleanup uses recycling, without permanent-delete fallback, emptying the recycle bin or bypassing safety rejection. Workspace cleanup rules live outside this product repository.
9. Do not automatically push, merge, rewrite remote history or publish. Follow current user authorization without adding unnecessary approval for reversible local edits.
