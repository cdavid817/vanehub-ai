## 0. Baseline and change validation

- [ ] 0.1 Read `AGENTS.md`, `openspec/project.md`, the current `session-shell`, `remote-terminal-runtime`, and `runtime-performance-governance` specs, and every active change touching Shell state, PTY, SSH, operations, session deletion, or runtime shutdown.
- [ ] 0.2 Produce a current fact map covering local open/close, remote open/close, routed runtime, store/replay registration, capacity checks, natural exit, idle sweep, archive/delete, shutdown, frontend service/adapters, and desktop tests.
- [x] 0.3 Run `openspec validate fix-retained-shell-lifecycle-and-bounded-reaping --strict`; correct the change artifacts before production code if repository evolution created a conflict.
- [ ] 0.4 Add characterization tests that demonstrate the current failure modes without weakening existing behavior: partial-start ownership, fast output/exit before store registration, concurrent capacity over-admission, remove-before-close, route loss after remote close failure, and unbounded fake wait/join.
- [x] 0.5 Record current configured total/per-session/local/remote limits and close/idle/shutdown timing constants. Do not raise them as part of the fix unless a separate requirement and evidence justify it.

## 1. Domain lifecycle and typed contracts

- [x] 1.1 Introduce generation-qualified Shell identity or an equivalent opaque lifecycle token and require it in runtime events, route entries, retained handles, capacity leases, close attempts, and reaper completions.
- [x] 1.2 Extend the Shell lifecycle model with `Opening`, `Closing`, `Reaping`, `CloseFailed`, and terminal causes while preserving compatibility for existing Running/Exited/Closed consumers.
- [x] 1.3 Centralize legal transitions in the workspaces domain/application layer and add tests preventing Running after early terminal exit, terminal-to-running regression, duplicate terminal publication, and stale-generation mutation.
- [x] 1.4 Define typed `SessionShellCloseResult`, reason codes, retryability, attempt metadata, cleanup-deadline state, and `SessionShellCleanupReport` without leaking platform error text across the service boundary.
- [x] 1.5 Define application-owned ports for capacity reservation, retained runtime open/close, Reaper handoff/status, and generation-safe terminal finalization. Infrastructure implements these ports; commands do not orchestrate them.
- [x] 1.6 Add serialization/compatibility tests for native DTOs and frontend TypeScript types, including unknown/new reason-code fallback.

## 2. Atomic capacity admission and pre-registration

- [x] 2.1 Replace total and per-session count-then-open checks with one synchronized `ShellCapacityController` that atomically reserves all applicable limits or none.
- [x] 2.2 Implement a move-only/RAII capacity lease associated with `(shell_id, generation)` and prove release is exactly once across successful terminal close, startup rollback, Reaper completion, duplicate close, and stale completion.
- [x] 2.3 Reserve capacity before any local spawn or remote channel open. Add fakes that assert the runtime adapter was never called when capacity is exhausted.
- [x] 2.4 Insert the Shell/store/replay lifecycle as `Opening` before invoking the runtime and reserve the route identity before a remote worker can emit events.
- [x] 2.5 Make input, resize, title/prompt operations return a stable not-running/closing error for non-Running phases while preserving read/diagnostic access where safe.
- [x] 2.6 Add 100-way deterministic concurrent create tests for the last global slot and last per-session slot; assert no over-admission, no permit leak, and no spawned loser.

## 3. Local startup transaction and event ordering

- [x] 3.1 Implement a local launch guard that owns child/killer, PTY handles, reader/writer, worker controls, event gate/buffer, and capacity handoff from the first successful acquisition.
- [x] 3.2 Refactor every local startup `?`/early-return path so the guard either confirms cleanup or transfers the complete resource set to the Reaper; no branch may merely drop or forget a live child/worker.
- [x] 3.3 Gate worker publication until the `Opening` Shell has an installed sink, or implement a bounded ordered startup buffer. Document which pattern is chosen and why.
- [ ] 3.4 Make early output, early natural exit, child spawn failure, reader acquisition failure, writer acquisition failure, worker creation failure, and registry commit failure deterministic in fakes.
- [x] 3.5 Add tests proving an `echo-and-exit` equivalent retains first output and one terminal event and is never overwritten by a later Running transition.
- [x] 3.6 Add startup cleanup timeout tests proving a non-terminating partially launched child becomes a retained `Reaping` Shell and continues consuming its capacity lease.
- [x] 3.7 Ensure startup guards, workers, and fake resources contain no production `unwrap()`/`expect()` and emit only redacted unified diagnostics.

## 4. Remote startup transaction and route ownership

- [x] 4.1 Implement a remote launch guard that distinguishes the Shell-owned channel from the shared transport lease and owns route reservation, streams, and workers until commit/handoff.
- [x] 4.2 Pre-register a generation-qualified remote route before workers publish and make route insertion/replacement reject stale or conflicting generations.
- [x] 4.3 Refactor all remote startup failures to close/reap only the new channel and retain unrelated pooled transport users.
- [x] 4.4 Add tests for channel-open failure, stream acquisition failure, worker setup failure, route commit failure, early output/exit, and cleanup timeout.
- [x] 4.5 Add a two-channel shared-transport test proving failure/rollback of one startup does not disconnect or mutate the other Shell.

## 5. Bounded local close

- [x] 5.1 Add an injected monotonic `ShellCloseBudget` and platform termination abstraction with non-blocking/bounded exit observation; define finite graceful, terminate, force/reap, worker-completion, and total command-path limits.
- [x] 5.2 Remove command-path unbounded `child.wait()` and unconditional blocking worker `join()` from the retained local runtime.
- [x] 5.3 Implement staged local close: stop input, close PTY input, observe, terminate, observe, force when supported, observe/reap, then complete or hand off workers.
- [x] 5.4 Preserve and map kill, try-wait, reap, PTY-close, and worker-completion failures into typed outcomes instead of discarding them.
- [x] 5.5 Keep the runtime entry in the retained ownership map while Closing/Reaping/CloseFailed; remove it only through generation-safe terminal finalization.
- [x] 5.6 Add deterministic fake-child tests for already exited, graceful exit, terminate-needed, force-needed, kill failure, `try_wait` error, never-exits, worker-never-completes, and duplicate close.
- [ ] 5.7 Add platform-focused tests for the actual portable-pty implementation on every available CI OS and report unsupported descendant-tree guarantees explicitly.

## 6. Bounded remote close and routed runtime

- [x] 6.1 Implement a finite remote close budget covering input stop, EOF/channel close, worker cancellation/drain, and completion observation.
- [x] 6.2 Remove unconditional routed-runtime route deletion. Delete a route only through terminal finalization for the matching generation.
- [x] 6.3 Make repeated close of Closing/Reaping/CloseFailed remote Shells return/reconcile the existing operation and continue routing to the same remote runtime.
- [x] 6.4 Add remote fake tests for close success, channel timeout, close error, blocked reader/writer, transport failure, stale route completion, and retry after CloseFailed.
- [x] 6.5 Add shared-transport tests proving one channel close/reap does not close another and transport-wide failure produces independent generation-safe outcomes.

## 7. Retained bounded Reaper and terminal finalization

- [x] 7.1 Implement a bounded Reaper queue/worker model with explicit queue capacity, active-attempt limit, per-attempt deadline, backoff, maximum automatic attempts, and structural metrics.
- [x] 7.2 Ensure each Reaper item retains every handle/control and the capacity lease required to continue cleanup. Queue-full behavior must keep ownership in registry/runtime and return a typed failure.
- [x] 7.3 Implement generation-safe compare-and-finalize as one application operation: terminal aggregate state, runtime detach, route removal, replay/store finalization, lease release, and exactly-one event.
- [x] 7.4 Make stale Reaper completion a no-op with diagnostic evidence; it must not release capacity or remove a current generation.
- [x] 7.5 Add virtual-time tests for success on later attempt, retry exhaustion, queue full, duplicate completion, shutdown during reaping, and manual retry after automatic exhaustion.
- [x] 7.6 Add metrics/log tests for bounded redacted fields and verify no command text, terminal output, credential, host secret, or unrestricted path is emitted.

## 8. Archive, delete, idle sweep, and shutdown integration

- [x] 8.1 Replace ignored close results in session archive/delete with `SessionShellCleanupReport`; default to strict finalization while any Shell is unconfirmed.
- [x] 8.2 Preserve enough session/Shell identity after `session_shell_cleanup_incomplete` for pull, retry, UI diagnosis, and eventual deletion completion.
- [x] 8.3 Refactor idle sweep so it counts only confirmed terminal Shells as closed and reports reaping/failed dispositions without blocking indefinitely.
- [x] 8.4 Refactor application shutdown to use one global finite budget, advance existing close/reaper work, and record residual resources without an unbounded wait.
- [x] 8.5 Search for and eliminate lifecycle `let _ = close...`, ignored kill/wait/join results, remove-before-close, and success events published before confirmation; add a guard test or static check for regressions where practical.
- [x] 8.6 Add application tests for multiple Shell cleanup with mixed closed/already-terminal/reaping/failed results and for a successful retry that completes the original archive/delete request.

## 9. Frontend service, Web/mock, UI, and i18n

- [x] 9.1 Extend the frontend Shell service contract and domain types for Opening/Closing/Reaping/CloseFailed, typed close result, and aggregate cleanup report.
- [x] 9.2 Update the Tauri adapter/event mapping without adding component-level `invoke()` calls or exposing native error strings.
- [x] 9.3 Implement deterministic Web/mock capacity, startup, fast-exit, reaping, failure, retry, terminal finalization, and stale-generation behavior with no native process claims.
- [x] 9.4 Update Shell/terminal UI so intermediate cleanup keeps the Shell identifiable, blocks unsafe Running-only operations, retains output, and reconciles event/pull state.
- [x] 9.5 Update archive/delete UI to display cleanup-in-progress/failure and avoid removing the session from UI before strict finalization succeeds.
- [x] 9.6 Add every new lifecycle/reason/cleanup string to all registered locales and pass key/interpolation parity tests.
- [x] 9.7 Add frontend unit/component tests for fast terminal event reconciliation, close Reaping → Closed, CloseFailed retry, duplicate event, stale generation, Web capacity rejection, and incomplete session delete.

## 10. Architecture, documentation, and verification

- [x] 10.1 Add/update architecture fitness tests proving Tauri commands do not own lifecycle orchestration, workspaces application owns ports, and remote/local infrastructure does not mutate frontend/application stores directly.
- [x] 10.2 Update Shell and SSH developer documentation with startup ownership, capacity reservation, lifecycle phases, bounded close, Reaper, route retention, shared-transport isolation, and strict session cleanup.
- [x] 10.3 Run focused Rust domain/application/infrastructure tests, frontend Shell tests, Web/mock tests, and deterministic lifecycle/performance tests; record exact counts and results.
- [x] 10.4 Run `npm run architecture:check` and resolve violations without blanket allowlists.
- [ ] 10.5 Run `npm run desktop:unit:test`, `npm run test:desktop:build`, and the current desktop session-Shell suite with fixed fixtures. Add/adjust the repository's current remote-Shell desktop coverage if available.
- [x] 10.6 Run the complete validation set from `AGENTS.md`: lint, frontend tests, build, Cargo fmt/check/clippy/panic-check/test, and `openspec validate --specs --strict`.
- [x] 10.7 Run `openspec validate fix-retained-shell-lifecycle-and-bounded-reaping --strict` after all task/spec edits.
- [x] 10.8 Report Windows, macOS, and Linux individually as PASSED/FAILED/BLOCKED/NOT RUN. Do not infer success for an unexecuted platform.
- [x] 10.9 Compare implementation against every requirement/scenario, leave unmet tasks unchecked, document residual process-tree/platform limitations, and remove legacy paths only after parity is proven.
