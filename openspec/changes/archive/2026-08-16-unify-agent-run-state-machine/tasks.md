## 1. Specification gate

- [x] 1.1 Run `openspec validate unify-agent-run-state-machine --strict` and resolve every proposal, design, task, and delta-spec error before editing business code.

## 2. Canonical Run domain

- [x] 2.1 Add canonical Run identity, owner/link, state, trigger, reason, retry/recovery policy, event, and snapshot types inside the existing `operations` domain.
- [x] 2.2 Implement the closed transition table, terminal monotonicity, waiting exit rules, retry bounds, verification paths, timestamps, and bounded safe metadata invariants.
- [x] 2.3 Add table-driven and property-style domain tests for every allowed/forbidden transition, all terminal states, retry exhaustion, invalid metadata, and duplicate terminal witnesses.

## 3. Application lifecycle and hierarchy

- [x] 3.1 Add narrow Run repository, clock/id, lifecycle-event, owner-recovery, and cooperative-cancellation ports plus application-safe errors.
- [x] 3.2 Implement create/get/list/transition/cancel/resume application services with optimistic versions and atomic snapshot/event writes.
- [x] 3.3 Implement parent cancellation propagation, cancellation/completion race handling, late-effect rejection, and idempotent terminal delivery.
- [x] 3.4 Add deterministic application tests using port doubles for hierarchy, retries, waits, verification, cancellation races, resume policy, privacy, and bounded pagination.

## 4. Persistence and recovery

- [x] 4.1 Add a transactional forward-only migration for indexed `agent_runs` and `agent_run_events` tables without modifying legacy owner data.
- [x] 4.2 Implement the SQLite Run repository with explicit fallible row/domain mapping, optimistic atomic writes, owner/parent/status queries, and bounded ordered events.
- [x] 4.3 Implement startup reconciliation that preserves terminal Runs, delegates verified resumability to owners, invalidates ephemeral waits, and records interrupted outcomes without replay.
- [x] 4.4 Add migration upgrade/rollback-compatibility/failure tests, repository round trips, pagination/query-plan evidence, restart idempotency, and non-replay security negative tests.

## 5. Native API and observability integration

- [x] 5.1 Publish narrow Run contracts through the `operations` API and assemble dependencies only in bootstrap without exposing infrastructure across contexts.
- [x] 5.2 Add compatible native get/list/cancel/resume commands and register them without changing existing command names or serialized responses.
- [x] 5.3 Correlate safe canonical lifecycle events with `execution_observability` while keeping telemetry failure non-blocking and routing diagnostics through unified logging with redaction.
- [x] 5.4 Add command DTO/error compatibility, architecture dependency, observability privacy/failure-isolation, and real operation lifecycle integration tests.

## 6. Existing runtime projections

- [x] 6.1 Project Session Agent generations through created/preparing/running/verifying/terminal Runs while preserving messages, streams, existing cancellation, and provider resume metadata.
- [x] 6.2 Project permission approval and user-question waits distinctly and add late-decision/answer, cancellation, expiry, and restart negative tests.
- [x] 6.3 Project PlanRun/SubTask/Attempt hierarchy, retry, verification, pause, recovery, timeout, and cancellation through canonical Runs without replacing Plan state.
- [x] 6.4 Project Loop execution, verification, retry/no-progress, pause, stuck, acceptance, recovery, and cancellation without replacing Loop phases.
- [x] 6.5 Link Goal progress evidence and group-chat delegated child execution while preserving Goal acceptance and Seat/turn routing semantics.
- [x] 6.6 Add cross-runtime acceptance tests for normal API execution, both waiting modes, transient retry, verification, user/parent cancellation, restart interruption, and illegal transitions.

## 7. Frontend service and adapter parity

- [x] 7.1 Add typed canonical Run views, events, filters, pagination, and action contracts to `agent-service.ts` and shared frontend types.
- [x] 7.2 Implement Tauri adapter calls only in `tauri-agent-client.ts` and deterministic Web/mock lifecycle/query/cancel/resume behavior in `web-agent-client.ts`.
- [x] 7.3 Add adapter contract and Vitest tests for state/reason/action parity, bounded results, idempotency, stale versions, failures, recovery claims, and cancellation races.

## 8. Minimal Run status UI

- [x] 8.1 Add localized Run status, waiting reason, elapsed, retry, cancel, and resume resources for every registered locale.
- [x] 8.2 Build a reusable semantic-token `AgentRunStatus` component and integrate it into the existing chat/Plan/Loop execution summaries without adding Mission Control.
- [x] 8.3 Add component accessibility/interaction tests and Playwright waiting/approval/cancel/resume coverage.
- [x] 8.4 Add stable visual coverage for futuristic/minimal styles at desktop and narrow widths and inspect screenshots for clipping, overlap, contrast, blank panels, and non-color status meaning.

## 9. Security, compatibility, and performance verification

- [x] 9.1 Run negative tests proving terminal Runs reject tool starts/late decisions, restart never replays destructive work, forged owner/parent/version data fails closed, and events/logs exclude sensitive payloads.
- [x] 9.2 Run deterministic Run transition/persistence benchmarks or structural budgets for constant transition lookup, bounded atomic writes, indexed owner/history queries, and bounded payload sizes; record evidence.
- [x] 9.3 Run `npm run test:coverage`, `npm run contracts:check`, `npx playwright test`, `npm run desktop:unit:test`, and `npm run test:desktop`; verify at least one real desktop Agent operation emits canonical states.

## 10. Repository quality gates

- [x] 10.1 Run `npm run lint:ci` and fix all findings without adding exemptions.
- [x] 10.2 Run `npm run test` and `npm run build`.
- [x] 10.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 10.4 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 10.5 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 10.6 Run `openspec validate --specs --strict` and `openspec validate unify-agent-run-state-machine --strict`.

## 11. Archive and final verification

- [x] 11.1 Confirm every requirement and task is complete, archive with `openspec archive unify-agent-run-state-machine`, and preserve synced main specs plus the full Markdown archive.
- [x] 11.2 Run `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`.
- [x] 11.3 Re-run `openspec validate --specs --strict`, verify the archive index entry, and record Linux actual plus Windows/macOS `NOT RUN` unless native evidence exists.
