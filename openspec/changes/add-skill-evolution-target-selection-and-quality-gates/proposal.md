## Why

The evidence pipeline can identify reproducible improvement patterns, but it deliberately does not decide which Skill is relevant or whether a pattern is safe and useful enough to advance. VaneHub needs a separate, auditable assessment stage so uncertain attribution, low-quality lessons, duplicates, privacy risks, and executable-content risks are stopped before any governance or mutation workflow is introduced.

## What Changes

- Add a deterministic-first target selector that ranks eligible Skill revisions using attribution, scope, type, capability metadata, lexical relevance, and evidence compatibility.
- Add an optional constrained LLM target consultation for genuinely ambiguous rankings; unavailable or invalid model output falls back to deterministic results.
- Add exactly nine deterministic quality checks covering privacy residue, insufficient evidence, duplicate knowledge, transient incidents, vague guidance, contradictory evidence, target incompatibility, executable-content risk, and immutable lifecycle state.
- Add an optional structured LLM judge that evaluates evidence support, specificity, durability, actionability, and risk without seeing raw runtime content or gaining mutation authority.
- Produce versioned, reproducible assessment records with ranked targets, check-level evidence, confidence, risk, and one of `advance`, `drop`, `record_memory_only`, `merge_duplicate`, or `needs_human_review`.
- Reassess safely when evidence, effective Skill revisions, selector rules, sanitizer versions, or evaluator versions change, while preserving prior decisions for audit.
- Expose read-only assessment summaries and explanations through the Skill service boundary and both desktop/Tauri and Web/mock adapters.
- Extend the Skill Evolution UI with target rankings, quality checks, LLM/fallback provenance, risk, confidence, and assessment history; do not expose approve, Overlay, apply, or automatic-mutation actions.
- Keep all Agent execution paths fail-open: assessment runs asynchronously and failure never changes source Agent, CLI, verification, or delegated Utility outcomes.

## Capabilities

### New Capabilities

- `skill-evolution-assessment`: Deterministic-first target selection, nine-check quality review, constrained LLM consultation and judging, reproducible assessment lifecycle, and safe routing recommendations.

### Modified Capabilities

- `skill-management`: Adds scoped read-only assessment queries and reassessment requests through the existing service and runtime adapter boundary.
- `settings-skill-management-ui`: Adds evidence-assessment explanations, history, provenance, and safe reassessment controls to the Skill Evolution area.

## Impact

- Desktop/runtime: adds a Rust assessment domain, deterministic selector and gates, constrained model-evaluation adapter, SQLite assessment persistence, asynchronous worker integration, and Tauri commands.
- Web runtime: adds matching mock assessment queries and deterministic reassessment behavior through the existing Web adapter.
- Frontend: extends agent-service models and the Skill Evolution evidence views without direct Tauri invocation.
- Data: stores sanitized assessment inputs, ranked candidates, check results, evaluator provenance, version witnesses, supersession links, and routing recommendations; it does not copy raw prompts, terminal output, tool arguments, source files, or secrets.
- Dependencies: requires effective Skill revisions and candidate seeds from the preceding Skill runtime and evidence-pipeline changes. It does not depend on Overlay application, Curator approval, scheduling, or autonomous mutation.
- Logging: assessment diagnostics use unified logging with sanitization and never introduce feature-local log files.
