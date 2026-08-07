## 1. Environment Preflight

- [x] 1.1 Run `npm ci` and confirm `node_modules/.pnpm` is absent and `node_modules/rehype-katex/node_modules/katex` exists, so build verification is trustworthy.
- [x] 1.2 Record a baseline: `npm run lint`, `npm run test`, `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml` all green before any deletion.

## 2. Frontend Removal

- [x] 2.1 Remove `startCoordination`, `listCoordinationRuns`, `getCoordinationRun`, and `cancelCoordinationRun` from the `AgentService` interface in `src/services/agent-service.ts`, along with the coordination type imports.
- [x] 2.2 Remove the four coordination method implementations from `src/services/tauri-agent-client.ts`.
- [x] 2.3 Remove the four coordination method implementations, the simulated executor, timers, and `resetWebLoopsForTest` coordination state from `src/services/web-agent-client.ts`.
- [x] 2.4 Delete `src/services/coordination-runtime.ts`, `src/services/coordination-runtime.test.ts`, and `src/services/web-coordination-runtime.test.ts`.
- [x] 2.5 Delete `src/types/coordination.ts` and confirm no remaining import references it.
- [x] 2.6 Confirm `npm run lint`, `npm run test`, and `npm run build` pass with the frontend surface gone.

## 3. Native Command and API Removal

- [x] 3.1 Delete `src-tauri/src/commands/agent_runtime/coordination/` (all five files) and its `mod` declaration in `src-tauri/src/commands/agent_runtime/mod.rs`.
- [x] 3.2 Remove the four coordination command registrations from `src-tauri/src/commands/registry.rs`.
- [x] 3.3 Remove the coordination DTOs from `src-tauri/src/commands/agent_runtime/dto.rs` and their conversions from `mapper.rs`.
- [x] 3.4 Remove coordination variants from `src-tauri/src/commands/error.rs`.
- [x] 3.5 Remove the coordination methods from `src-tauri/src/contexts/agent_runtime/api.rs`.

## 4. Native Runtime Removal

- [x] 4.1 Delete `src-tauri/src/contexts/agent_runtime/application/coordination.rs` and its `mod`/re-export lines in `application/mod.rs`.
- [x] 4.2 Delete `src-tauri/src/contexts/agent_runtime/domain/coordination.rs` and its `mod`/re-export lines in `domain/mod.rs`.
- [x] 4.3 Delete `coordination_executor.rs`, `coordination_scheduler.rs`, and `coordination_repository.rs` plus their `mod`/re-export lines in `infrastructure/mod.rs`.
- [x] 4.4 Remove coordination wiring from `src-tauri/src/bootstrap/agent_runtime.rs` and `src-tauri/src/bootstrap/runtime.rs`.
- [x] 4.5 Remove coordination-specific members from `infrastructure/composite_process_gateway.rs` and `infrastructure/runtime_support.rs` without deleting the files, which serve non-coordination paths.
- [x] 4.6 Remove coordination variants from `application/error.rs` and `domain/error.rs`, then resolve every exhaustive `match` that `cargo check` reports.
- [x] 4.7 Confirm `cargo check` and `cargo clippy --manifest-path src-tauri/Cargo.toml` are clean.

## 5. Database Retirement

- [x] 5.1 Add migration `43` named `remove-multi-agent-coordination` executing `DROP TABLE IF EXISTS coordination_runs;` in `src-tauri/src/platform/database/migrations.rs`.
- [x] 5.2 Keep the migration `27` slot as a documented no-op and delete `src-tauri/src/contexts/agent_runtime/infrastructure/coordination_schema.rs` with its re-export, leaving versions 28-41 untouched.
- [x] 5.3 Add a native test proving migration 43 is idempotent and that a database carrying a pre-existing `coordination_runs` table ends with the table absent.
- [x] 5.4 Run `cargo test --manifest-path src-tauri/Cargo.toml` and confirm the full native suite passes.

## 6. Observability Trim

- [x] 6.1 Remove coordination-node and failover span/event emission from the execution observability instrumentation.
- [x] 6.2 Remove the `candidate role` metric dimension and coordination/failover metric names so the code matches the narrowed spec.
- [x] 6.3 Update or remove observability tests that assert coordination correlation.

## 7. Specs and Documentation

- [x] 7.1 Delete `openspec/specs/multi-agent-coordination/spec.md` and its now-empty capability directory.
- [x] 7.2 Apply the observability delta so `openspec/specs/agent-execution-observability/spec.md` no longer promises coordination-node or failover coverage.
- [x] 7.3 Delete `docs/developer-guide/src/multi-agent-coordination.md` and remove its entry from `docs/developer-guide/src/SUMMARY.md`.
- [x] 7.4 Grep `openspec/specs/`, `docs/`, and `README*.md` for remaining coordination references and clear any that describe the retired capability as current.
- [x] 7.5 Leave `openspec/changes/archive/2026-07-23-add-multi-agent-coordination/` untouched as immutable history.

## 8. Verification

- [x] 8.1 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 8.2 Run `cargo test`, `cargo check`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 8.3 Run `openspec validate remove-multi-agent-coordination --strict` and `openspec validate --specs --strict`.
- [x] 8.4 Run the Playwright suite against a dev server you started yourself with `PLAYWRIGHT_PORT` pinned, since `reuseExistingServer` will otherwise latch onto another worktree's server on port 5174 and test the wrong code.
- [x] 8.5 Launch the desktop client, confirm it starts cleanly, and verify `coordination_runs` is gone from the SQLite database.
