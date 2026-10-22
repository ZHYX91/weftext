# Weftext Engine

Native Typst compilation and conservative author-source projections. Product explanations use the sole generated `UiMessage` catalog; original Typst, operating-system and dependency diagnostics retain their provenance and exact text. Display language does not enter author source or machine admission decisions.

`Diagnostic` separates an optional product explanation from original diagnostics and an exact source origin. The pinned native HTML adapter emits closed warning facts with compiler spans; author HTML attributes cannot forge them. Each `LocalWorld` records its own dependency-admission facts, independently from compiler diagnostics. Paged and Flow outputs preserve both channels.

`EngineError` has an explicit display-tree budget variant and a typed primary explanation plus full parallel product diagnostics. `into_parts` preserves those diagnostics across Core, API, worker and Data bridges; they are not packed into the bounded cause chain. The display-tree budget keeps its machine identity when additional evidence is attached.

Tests cover real native compilation, original diagnostic/source round trips, typed native warnings, per-compilation dependency budgets, budget-plus-evidence preservation and source editing. Use the product repository check entry points with the authorized shared build target.
