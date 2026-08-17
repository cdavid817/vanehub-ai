## 1. Baseline and Contracts

- [x] 1.1 Record the audited canonical Run, evaluation, review, approval, observability, logging, Goal, Plan, Loop, Session, navigation, and adapter reuse map in implementation notes without changing their ownership.
- [x] 1.2 Add mirrored TypeScript Mission Control contracts for summaries, counts, filters, cursors, detail-facet availability, navigation targets, action receipts, and safe typed errors; extend contract conformance tests.
- [x] 1.3 Extend `AgentService` with bounded Mission Control query/detail/action methods and add compile-time parity for Tauri and Web adapters.

## 2. Native Read Model and Migration

- [x] 2.1 Add Rust application projection models and query/action ports inside the existing `operations` canonical Run owning context, consuming owner-specific behavior only through published APIs or immutable contracts assembled by bootstrap.
- [x] 2.2 Add an additive transactional SQLite migration and selective indexes only where the audited query plan requires them, preserving legacy rows and rollback compatibility. (Audit retained the existing additive Run schema and `idx_agent_runs_state`; no new migration was required.)
- [x] 2.3 Implement the constant-count bounded overview repository query with status/Agent/project/runner filters, cursor validation, deterministic newest/oldest/attention sorting, summary counts, and safe optional metadata.
- [x] 2.4 Implement bounded detail availability and lazy correlation contracts without loading logs, diffs, artifact bodies, prompts, responses, or tool payloads in overview/detail manifests.
- [x] 2.5 Add migration compatibility, rollback, restart reconciliation, pagination, 100+ history, invalid cursor, redaction, oversized data, and no-N+1 query-count/query-plan tests.

## 3. Native Actions and Commands

- [x] 3.1 Implement state/version/owner-policy revalidation for Cancel, Resume, Retry, and Run Verification by delegating to existing canonical Run and owning workflow services.
- [x] 3.2 Add negative tests for illegal terminal reversal, stale version, cancel/complete race, unsupported retry, unauthorized or stale approval target, invalid verification, and duplicate terminal delivery.
- [x] 3.3 Add command DTOs, mappers, one-command-per-file handlers, command-safe errors, bootstrap composition, invoke registration, and serialized compatibility tests.
- [x] 3.4 Associate Mission Control action diagnostics with existing operations and unified redacted logging without feature-local logs.

## 4. Runtime Adapters and Event Reconciliation

- [x] 4.1 Implement the Tauri adapter methods using only declared Mission Control and existing owning-service commands.
- [x] 4.2 Implement deterministic Web/mock fixtures for concurrent, waiting approval, waiting user, retrying, stuck, failed, completed, review-requested, unavailable-evidence, pagination, filters, sorting, and supported/unsupported actions.
- [x] 4.3 Add adapter parity and deterministic fixture tests proving Web mode performs no native persistence, filesystem, process, credential, approval, review, or verification side effects.
- [x] 4.4 Implement a version-aware event reducer that immediately flushes state/attention/terminal events, coalesces progress/usage events, rejects stale events, and reconciles on mount, reconnect, and app focus.
- [x] 4.5 Add deterministic event burst, missed-event, terminal-flush, stale-sequence, unmount, reconnect, and bounded-render-batch tests.

## 5. Mission Control UI

- [x] 5.1 Add the localized lazy Mission Control workspace destination, route parsing, activity-bar entry, keep-alive behavior, accessible labels/tooltips, and navigation tests.
- [x] 5.2 Implement the compact summary strip, attention inbox, filter/sort controls, bounded active/recent Run presentation, pagination, loading/stale/error/empty states, fixed terminal elapsed time, and unavailable usage/cost handling.
- [x] 5.3 Implement Run detail with Overview, Plan/Tasks, Timeline, Tools, Files/Artifacts, Review, Tests/Verification, Context, Usage, and Logs navigation plus explicit unavailable/restricted states and lazy bounded loading.
- [x] 5.4 Implement Open, Cancel, Resume, Retry, approval navigation, Review Changes, and Run Verification controls from service-provided policy, with accessible confirmation/error/reconciliation behavior.
- [x] 5.5 Route approval, Session, Code Review Center, Plan/Loop/Goal, Evaluation, and logs to existing owning surfaces; verify Mission Control contains no duplicate chat, editor, diff, approval decision, or runtime-specific branch.
- [x] 5.6 Add semantically aligned translations for every registered locale and pass resource parity and hard-coded visible-text guardrails.
- [x] 5.7 Add Vitest coverage for multi-Run state, attention priority, filter/sort reset, completed timer, retry/stuck reason, actions, unavailable facets, lazy detail, accessibility, desktop, and narrow layouts.

## 6. End-to-End, Visual, Security, and Performance Acceptance

- [x] 6.1 Add Playwright flows for concurrent Runs, waiting approval priority/navigation, waiting user, retry/stuck reason, failure, completed timer, filtering/sorting/pagination, Review Center navigation, actions, and post-focus reconciliation.
- [x] 6.2 Add stable visual coverage for futuristic desktop, futuristic narrow, minimal desktop, and minimal narrow Mission Control overview/detail states and inspect artifacts for overlap, clipping, contrast, layout shifts, and blank panels.
- [x] 6.3 Add desktop smoke coverage proving a real local operation enters Mission Control and reaches terminal state through the Tauri boundary; report Windows, macOS, and Linux only as actually executed.
- [x] 6.4 Add deterministic structural performance benchmarks for maximum history, bounded query count and item counts, indexed query plans, lazy facet loading, aggregation allocation, and event coalescing batches.
- [x] 6.5 Run focused Rust, Vitest, contract, Playwright, visual, desktop, security-negative, migration, and performance suites; fix every failure before proceeding.

## 7. Repository Quality Gates

- [x] 7.1 Run `npm run lint:ci`.
- [x] 7.2 Run `npm run test` and `npm run test:coverage`.
- [x] 7.3 Run `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 7.4 Run `npm run build` and `npx playwright test`.
- [x] 7.5 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 7.6 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 7.7 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.8 Run `npm run desktop:unit:test` and `npm run test:desktop` on the current platform.
- [x] 7.9 Run `openspec validate --specs --strict` and `openspec validate add-agent-mission-control --strict`, then record all test, visual, benchmark, migration, and platform evidence in the change artifacts.

## 8. Verification and Archive

- [x] 8.1 Verify every delta requirement and scenario maps to implementation and automated or documented platform evidence, with no critical or warning divergence.
- [x] 8.2 Confirm no roadmap 08-or-later feature, duplicate bounded context, test bypass, architecture allowlist, or unrelated user file was introduced.
- [ ] 8.3 Archive with `openspec archive add-agent-mission-control`, update the archive index using the mandated PowerShell script, and rerun main-spec and archived-change strict validation.
