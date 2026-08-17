## 1. Runtime Lifecycle and Error Integrity

- [x] 1.1 Add generation-aware workspace PTY natural-exit reclamation and failure-path cleanup tests.
- [x] 1.2 Make synchronous managed-child drop schedule kill-and-wait reaping and add process lifecycle coverage.
- [x] 1.3 Make Shell event subscription cleanup-safe across asynchronous unmount and add a deferred-listener regression test.
- [x] 1.4 Make manual native-tool admission fail when its initial operation cannot be persisted and diagnose later persistence failures.
- [x] 1.5 Surface skill-evidence initialization and feedback lookup failures with safe error classifications and diagnostics.

## 2. Bounded Runtime Performance

- [x] 2.1 Batch Agent registry rows, modes, and tags with a query-count-independent repository path and equivalence tests.
- [x] 2.2 Batch message feedback current state and revision reads with bounded SQLite parameter chunks and failure tests.
- [x] 2.3 Snapshot communication connector handles before awaiting individual health state.
- [x] 2.4 Batch Agent Terminal output rendering and bound replay storage globally with deterministic frontend tests.

## 3. Architecture and Observability

- [x] 3.1 Extend native architecture analysis to infrastructure and command scopes.
- [x] 3.2 Replace CLI delegation and MCP relay concrete cross-context repository dependencies with published ports assembled at the composition root.
- [x] 3.3 Record bounded redacted diagnostics for Agent execution telemetry transition failures without changing task outcomes.

## 4. Verification and Delivery

- [x] 4.1 Run frontend lint, tests, build, coverage policy, contracts, and Playwright checks required by the affected UI behavior.
- [x] 4.2 Run Rust fmt, clippy, tests, check, desktop tests, and strict OpenSpec validation.
- [ ] 4.3 Commit proposal and fixes in focused Conventional Commit groups with English messages.
