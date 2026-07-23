## 1. Coordination Domain

- [x] 1.1 Add coordination plan/node value objects with stable id, Agent candidate, instruction, dependency, size, and uniqueness invariants.
- [x] 1.2 Implement deterministic DAG validation/topological ordering and dependency-failure propagation with domain tests.
- [x] 1.3 Add run, node, output, attempt, failure-classification, cancellation, and recovery transition models with domain tests.

## 2. Native Application Runtime

- [x] 2.1 Define coordination repository, executor, scheduler/lease, clock/id, operation, observability, and logging ports plus query projections.
- [x] 2.2 Implement coordination start/list/get/cancel use cases and deterministic sequential scheduling.
- [x] 2.3 Implement bounded prerequisite context assembly and ordered primary-to-fallback execution with application port-double tests.
- [x] 2.4 Implement idempotent terminal callbacks, cancellation, independent-branch continuation, blocked-dependent skipping, and startup recovery tests.

## 3. Native Persistence and Integration

- [x] 3.1 Add versioned SQLite coordination schema/migration with clean-database and upgrade compatibility tests.
- [x] 3.2 Implement atomic SQLite plan/run/node/attempt persistence and projections with repository tests.
- [x] 3.3 Implement the native Agent execution adapter and background scheduler by reusing Agent registry/provider generation boundaries.
- [x] 3.4 Wire coordination operation, execution-observability, unified-log, bootstrap, and startup recovery behavior.
- [x] 3.5 Add Tauri coordination DTOs, mappers, commands, command-safe errors, registration, and serialization contract tests.

## 4. Frontend Service Parity

- [x] 4.1 Add strict TypeScript coordination input/run/node/attempt/output contracts and extend `AgentService`.
- [x] 4.2 Implement Tauri adapter methods for start/list/get/cancel without adding `invoke()` calls to React.
- [x] 4.3 Implement deterministic Web/mock graph validation, output propagation, fallback fixtures, lifecycle, cancellation, and adapter tests.

## 5. Verification

- [x] 5.1 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 5.2 Run `cargo test`, `cargo check`, and `cargo clippy` with `src-tauri/Cargo.toml`.
- [x] 5.3 Run `openspec validate add-multi-agent-coordination --strict` and `openspec validate --specs --strict`.
- [x] 5.4 Review architecture boundaries, logging redaction, migration behavior, adapter parity, and completed task evidence.

## 6. Verification Remediation

- [x] 6.1 Classify native execution failures so policy, permission, validation, and configuration failures do not trigger fallback.
- [x] 6.2 Make scheduler execution failures observable and persist a deterministic terminal run state instead of dropping errors.
- [x] 6.3 Bound native streaming output accumulation before persistence while retaining original byte count and UTF-8-safe truncation metadata.
- [x] 6.4 Add focused regression tests and rerun strict OpenSpec plus Rust coordination validation.
- [x] 6.5 Add an active-run cancellation regression test that proves the current attempt stops, fallback does not start, and remaining nodes reach terminal states.
- [x] 6.6 Add a combined prerequisite-context overflow regression test that proves the dependent executor is not invoked and the failure is non-retryable.
- [x] 6.7 Advance Web/mock coordination asynchronously by node and attempt, with an abortable active simulated attempt.
- [x] 6.8 Add Web active-cancellation regression coverage and rerun frontend plus strict OpenSpec validation.
