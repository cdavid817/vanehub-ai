## 1. Domain contracts and safety

- [x] 1.1 Add evaluation domain models under `execution_observability` for manifests, arenas, attempts, Agent/config snapshots, verification, metric provenance, classifications, ranking versions, and exports.
- [x] 1.2 Implement bounded manifest parsing/validation and ship 3–5 deterministic fixture tasks using allowlisted verifier profile ids.
- [x] 1.3 Add domain tests for valid manifests plus unsafe command, traversal, symlink, version/category, timeout, and size negative cases.

## 2. Isolation, verification, and metrics

- [x] 2.1 Extend the published `workspaces` API with bounded evaluation fixture prepare/reset/cleanup behavior and deterministic port doubles.
- [x] 2.2 Implement deterministic fake-Agent execution plus existing Agent-runtime dispatch adapters for OnePiece and eligible managed CLI Agents.
- [x] 2.3 Implement allowlisted acceptance, static assertion, diff-rule, flaky detection, and non-authoritative structured-judge aggregation.
- [x] 2.4 Aggregate outcome, efficiency, context, reliability, token/cost provenance, retries/replans/recovery/intervention, and transparent versioned ranking without inventing missing values.
- [x] 2.5 Add domain/application tests covering clean reset, isolation between Agents, timeout/cancel/stuck cleanup, deterministic precedence, harness-vs-task failure, metrics, pricing snapshots, ranking, and Context Engine evidence links.

## 3. Persistence and native orchestration

- [x] 3.1 Add additive SQLite migrations and repositories for bounded catalog/run/snapshot/metric/verification metadata and artifact references, with atomic terminal writes and retention.
- [x] 3.2 Add migration, round-trip, pagination, retention, redaction, and persistence-failure tests.
- [x] 3.3 Assemble evaluation application services in bootstrap using published operations, agent-runtime, workspaces, observability, artifact, and unified-log contracts.
- [x] 3.4 Add asynchronous start/cancel/query/compare/detail/timeline/export application flows correlated to canonical Agent Runs and stable operation ids.
- [x] 3.5 Add one Tauri command per evaluation operation, command-safe DTO/error mapping, registry entries, serialization/contract tests, and architecture-fitness coverage.

## 4. Frontend service and Web parity

- [x] 4.1 Add typed evaluation contracts and catalog/start/cancel/status/results/comparison/detail/timeline/export methods to `AgentService`.
- [x] 4.2 Implement matching Tauri adapter invokes and contract checks.
- [x] 4.3 Implement deterministic Web/mock catalog, clean arena attempts, lifecycle transitions, comparison, details, timeline, cancellation, and JSON export with unit tests.

## 5. Eval workspace UI

- [x] 5.1 Add the translated Eval route/navigation entry and compact responsive catalog/configuration/run-status workspace using shared semantic tokens and UI primitives.
- [x] 5.2 Add results filtering/comparison, task/attempt detail, deterministic verification, bounded diff, context/tool timeline, missing-metric provenance, and JSON export interactions.
- [x] 5.3 Add component tests for filtering, configuration, running/terminal/error states, comparison, details, cancellation, unavailable metrics, and export across all registered locales.
- [x] 5.4 Add Playwright complete mock benchmark coverage and stable visual snapshots for futuristic/minimal themes at desktop/narrow widths.

## 6. Framework, security, desktop, and performance verification

- [x] 6.1 Add deterministic end-to-end fake-Agent benchmark tests covering manifest through export without network credentials or paid models.
- [x] 6.2 Add security negative tests for command injection, path escape/symlink, secret redaction, output bounds, cancellation/timeout, and artifact/SQLite content safety.
- [x] 6.3 Add repeatable structural performance benchmarks for maximum MVP arena/result-page bounds, persistence query plans/counts, and bounded allocations.
- [x] 6.4 Extend desktop automation with one minimal real installed-Agent benchmark and explicit PASSED/FAILED/BLOCKED evidence for the current native platform.

## 7. Required quality gates and archive readiness

- [x] 7.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 7.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.3 Run `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 7.4 Run `npx playwright test`, verify futuristic/minimal desktop/narrow visual evidence, run the evaluation performance benchmark, and record results.
- [x] 7.5 Run `npm run desktop:unit:test` and `npm run test:desktop`; report Windows/macOS/Linux only from actually executed native jobs.
- [x] 7.6 Run `npx --yes @fission-ai/openspec@1.6.0 validate --specs --strict` and `npx --yes @fission-ai/openspec@1.6.0 validate add-agent-evaluation-platform --strict`, then complete implementation verification before archive.
