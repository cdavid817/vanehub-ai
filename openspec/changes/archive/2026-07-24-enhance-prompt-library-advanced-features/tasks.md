## 1. Shared Contracts and Template Variables

- [x] 1.1 Extend Prompt Hook TypeScript and Rust models with variable definitions, draft state, immutable version metadata, publication results, and evaluation summaries.
- [x] 1.2 Implement the Rust allowlisted variable parser and renderer for canonical snake-case variables, legacy aliases, unknown-variable validation, inert replacement, and one clock snapshot.
- [x] 1.3 Add domain tests for variable discovery, rendering, aliases, unknown variables, time/session context, and non-execution of script-like text.

## 2. Native Version Persistence and Application Lifecycle

- [x] 2.1 Add an additive SQLite migration for user Hook drafts, immutable versions, selected published versions, and idempotent execution observations, including backfill of existing Hook rows.
- [x] 2.2 Add repository ports and SQLite implementations for draft save/load, atomic publish, immutable history, rollback publication, idempotent observation writes, and bounded aggregates.
- [x] 2.3 Implement application services for draft save, publish with optimistic revision checks, history/detail queries, rollback while preserving drafts, built-in mutation rejection, and variable catalog queries.
- [x] 2.4 Add native commands, DTO/error mapping, registry entries, published API methods, bootstrap wiring, and Rust tests for migration preservation and lifecycle invariants.

## 3. Runtime Evaluation Attribution

- [x] 3.1 Extend the agent-runtime effective-prompt contract to retain safe fired Hook id/version references and define a consuming evaluation gateway.
- [x] 3.2 Report idempotent succeeded, failed, and cancelled outcomes with elapsed milliseconds from terminal CLI generation paths without including sensitive execution content.
- [x] 3.3 Add deterministic application/infrastructure tests for multi-Hook attribution, retry idempotency, cancellation exclusion, safe records, and version aggregates.

## 4. Frontend Service and Runtime Adapters

- [x] 4.1 Extend `AgentService` with variable-catalog, draft, publish, history/detail, rollback, and evaluation-summary methods.
- [x] 4.2 Implement Tauri adapter command mappings and contract tests for every advanced Prompt Hook operation.
- [x] 4.3 Implement deterministic Web/mock draft, immutable-version, rollback, variable-rendering, and evaluation behavior with adapter parity tests.

## 5. Settings UI and Localization

- [x] 5.1 Add synchronized zh-CN/en resources for variables, draft/publication status, version history, rollback confirmation, evaluation metrics, attribution guidance, empty states, and errors.
- [x] 5.2 Add a compact service-backed Prompt Hook lifecycle panel with variable insertion, draft save, publish, history, and rollback controls while keeping files under 300 lines.
- [x] 5.3 Add per-version operational outcome summaries with localized duration formatting, active-version state, cancelled counts, and no-data handling.
- [x] 5.4 Add focused frontend tests for draft isolation, publication, rollback with preserved drafts, built-in protection, variable validation, metric display, adapter usage, and i18n parity.
- [x] 5.5 Inspect the Prompt Hooks page in `futuristic` and `minimal` styles at desktop and narrow widths for clipping, overlap, contrast, and stable control states.

## 6. Validation

- [x] 6.1 Run `npm run lint`.
- [x] 6.2 Run `npm run test`.
- [x] 6.3 Run `npm run build`.
- [x] 6.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 6.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.6 Run `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 6.7 Run `openspec validate enhance-prompt-library-advanced-features --strict`.
- [x] 6.8 Run `openspec validate --specs --strict`.
