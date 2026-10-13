---
source_language: zh-CN
translation_of: AGENTS.zh-CN.md
translation_status: synced
---

[简体中文](AGENTS.zh-CN.md)

# Repository collaboration rules

1. Read README, current requirements and relevant technical owners first. Requirements define outcomes; technology may be reselected. Old code and undeployed protocols do not create compatibility obligations.
2. Maintain one native Typst design. Prefer official syntax without parallel authoring languages, competing data authorities or migrations unsupported by real data. Current Core contracts own persistence, authorization and failure guarantees.
3. Public human-facing documents have a Chinese source and synchronized English. Schemas, code, machine fixtures and original evidence are not localized. Private conversations, working logs, real user data and secrets stay outside the public repository.
4. Algorithm, persistent-format and authorization changes update affected consumers and meaningful positive/negative cases. Document organization preserves semantics; counts and green CI do not replace semantic review.
5. Run `python scripts/check.py`; add `--models` for changes to schemas, models or their contracts. When Rust implementation lands, add relevant fmt/clippy/test entry points inside this independent product repository.
6. Distinguish design acceptance, bounded experiments, product acceptance and release. Probes are not production implementations; never claim unrun platforms or cases passed.
7. Preserve real originals before migration. Establish ownership and inactivity before cleanup; bulk cleanup uses recycling, without permanent-delete fallback, emptying the recycle bin or bypassing safety rejection. Workspace cleanup rules live outside this product repository.
8. Do not automatically push, merge, rewrite remote history or publish. Follow current user authorization without adding unnecessary approval for reversible local edits.
